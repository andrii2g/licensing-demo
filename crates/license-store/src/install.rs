use crate::SecureDir;
use license_core::{envelope::strict, *};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HighWater {
    sequence: u64,
    digest: String,
}
pub fn install(
    dir: &SecureDir,
    bytes: &[u8],
    trust: &Trust,
    ctx: &Context,
) -> Result<VerifiedLease> {
    let candidate = verify(bytes, trust, ctx)?;
    let mut sequence = 0;
    let mut digest = String::new();
    match dir.read("install-state.json", 4096, true) {
        Ok(b) => {
            let high: HighWater = strict(&b, 4096)?;
            sequence = high.sequence;
            digest = high.digest;
        }
        Err(Code::LicenseMissing) => {}
        Err(e) => return Err(e),
    }
    match dir.read("license.lic", 65536, false) {
        Ok(b) => {
            // Expired current authorization still supplies an authenticated sequence.
            let current = policy::authenticate(&b, trust)?;
            if current.claims.installation_id != ctx.identity.installation_id {
                return Err(Code::InstallationMismatch);
            }
            if current.claims.sequence == sequence && current.digest != digest {
                return Err(Code::LeaseRollback);
            }
            // A new verified server lease may repair an installed file behind metadata.
            if current.claims.sequence > sequence {
                sequence = current.claims.sequence;
                digest = current.digest;
            }
        }
        Err(Code::LicenseMissing) => {}
        Err(e) => return Err(e),
    }
    if candidate.claims.sequence < sequence
        || (candidate.claims.sequence == sequence && candidate.digest != digest)
    {
        return Err(Code::LeaseRollback);
    }
    dir.atomic("license.lic", bytes, false)?;
    let high = HighWater {
        sequence: candidate.claims.sequence,
        digest: candidate.digest.clone(),
    };
    dir.atomic(
        "install-state.json",
        &serde_json::to_vec(&high).map_err(|_| Code::InternalError)?,
        true,
    )?;
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    #[test]
    fn high_water_crash_recovery_and_bad_candidate_preserves_file() {
        let d = tempfile::tempdir().unwrap();
        let dir = SecureDir::open(d.path()).unwrap();
        let _lock = dir.lock().unwrap();
        let device = crate::Device::load_or_create(&dir).unwrap();
        let issuer = SigningKey::from_bytes(&license_core::crypto::random::<32>().unwrap());
        let trust = Trust::new(vec![(
            "generated".into(),
            envelope::encode(issuer.verifying_key().as_bytes()),
        )])
        .unwrap();
        let mut c: Claims =
            serde_json::from_str(include_str!("../../../fixtures/base-claims.json")).unwrap();
        c.installation_id = device.identity.installation_id.clone();
        c.installation_public_key_sha256 = crypto::hash(device.key.verifying_key().as_bytes());
        let ctx = Context {
            now: c.issued_at,
            product: c.product.clone(),
            required_features: vec![],
            identity: device.identity,
            machine_id_hash: Some(c.binding.machine_id_hash.clone()),
            system_uuid_hash: c.binding.system_uuid_hash.clone(),
            logical_processors: Some(8),
        };
        let sign = |c: &Claims| {
            serde_json::to_vec(&crypto::sign(c, LEASE_TYPE, "generated", &issuer).unwrap()).unwrap()
        };
        c.sequence = 1;
        let one = sign(&c);
        install(&dir, &one, &trust, &ctx).unwrap();
        assert!(install(&dir, b"bad", &trust, &ctx).is_err());
        assert_eq!(dir.read("license.lic", 65536, false).unwrap(), one);
        c.sequence = 2;
        let two = sign(&c);
        // Simulate crash after license rename but before metadata update.
        dir.atomic("license.lic", &two, false).unwrap();
        assert_eq!(
            install(&dir, &one, &trust, &ctx).unwrap_err(),
            Code::LeaseRollback
        );
        install(&dir, &two, &trust, &ctx).unwrap();
        // Metadata ahead of old restored license: cannot install old, can recover new.
        dir.atomic("license.lic", &one, false).unwrap();
        assert_eq!(
            install(&dir, &one, &trust, &ctx).unwrap_err(),
            Code::LeaseRollback
        );
        c.sequence = 3;
        let three = sign(&c);
        install(&dir, &three, &trust, &ctx).unwrap();
        assert_eq!(dir.read("license.lic", 65536, false).unwrap(), three);
    }
}
