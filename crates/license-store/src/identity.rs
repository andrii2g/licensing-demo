use crate::SecureDir;
use ed25519_dalek::SigningKey;
use license_core::{
    crypto::random,
    envelope::{encode, fixed, object as strict},
    *,
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};
pub struct Device {
    pub identity: Identity,
    pub key: SigningKey,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    #[serde(deserialize_with = "license_core::json_object")]
    identity: Identity,
    seed: String,
}
impl Drop for Journal {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}
impl Device {
    pub fn load(dir: &SecureDir) -> Result<Self> {
        let identity: Identity = strict(&dir.read("installation.json", 4096, false)?, 4096)?;
        identity.validate()?;
        let bytes = Zeroizing::new(dir.read("device.key", 32, true)?);
        let seed: Zeroizing<[u8; 32]> = Zeroizing::new(
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| Code::InstallationMismatch)?,
        );
        let key = SigningKey::from_bytes(&seed);
        if encode(key.verifying_key().as_bytes()) != identity.installation_public_key {
            return Err(Code::InstallationMismatch);
        }
        Ok(Self { identity, key })
    }
    /// Caller holds mutation.lock for the entire recovery/creation/network mutation.
    pub fn load_or_create(dir: &SecureDir) -> Result<Self> {
        match dir.read("identity.pending.json", 4096, true) {
            Ok(bytes) => {
                let bytes = Zeroizing::new(bytes);
                let j: Journal = strict(&bytes, 4096)?;
                let seed = Zeroizing::new(fixed::<32>(&j.seed)?);
                let key = SigningKey::from_bytes(&seed);
                j.identity.validate()?;
                if encode(key.verifying_key().as_bytes()) != j.identity.installation_public_key {
                    return Err(Code::InstallationMismatch);
                }
                let public = serde_json::to_vec(&j.identity).map_err(|_| Code::InternalError)?;
                for (name, expected, private) in [
                    ("device.key", seed.as_slice(), true),
                    ("installation.json", public.as_slice(), false),
                ] {
                    match dir.read(name, 4096, private) {
                        Ok(existing) if existing != expected => {
                            return Err(Code::InstallationMismatch);
                        }
                        Ok(_) => {}
                        Err(Code::LicenseMissing) => dir.atomic(name, expected, private)?,
                        Err(e) => return Err(e),
                    }
                }
                dir.remove("identity.pending.json")?;
                return Self::load(dir);
            }
            Err(Code::LicenseMissing) => {}
            Err(e) => return Err(e),
        }
        match dir.read("installation.json", 4096, false) {
            Ok(_) => return Self::load(dir),
            Err(Code::LicenseMissing) => {}
            Err(e) => return Err(e),
        }
        match dir.read("device.key", 32, true) {
            Err(Code::LicenseMissing) => {}
            _ => return Err(Code::InstallationMismatch),
        }
        let seed = Zeroizing::new(random::<32>()?);
        let key = SigningKey::from_bytes(&seed);
        let identity = Identity {
            schema_version: 1,
            installation_id: uuid::Uuid::new_v4().to_string(),
            installation_public_key: encode(key.verifying_key().as_bytes()),
        };
        let j = Journal {
            identity,
            seed: encode(seed.as_slice()),
        };
        let bytes = Zeroizing::new(serde_json::to_vec(&j).map_err(|_| Code::InternalError)?);
        dir.atomic("identity.pending.json", &bytes, true)?;
        Self::load_or_create(dir)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_identity_and_corruption() {
        let d = tempfile::tempdir().unwrap();
        let dir = SecureDir::open(d.path()).unwrap();
        let _lock = dir.lock().unwrap();
        let a = Device::load_or_create(&dir).unwrap();
        let b = Device::load_or_create(&dir).unwrap();
        assert_eq!(a.identity.installation_id, b.identity.installation_id);
        dir.atomic("device.key", &[0; 32], true).unwrap();
        assert!(Device::load_or_create(&dir).is_err());
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn interrupted_publication_preserves_identity() {
        let d = tempfile::tempdir().unwrap();
        let dir = SecureDir::open(d.path()).unwrap();
        let _lock = dir.lock().unwrap();
        let seed = [102; 32];
        let key = SigningKey::from_bytes(&seed);
        let identity = Identity {
            schema_version: 1,
            installation_id: uuid::Uuid::new_v4().to_string(),
            installation_public_key: encode(key.verifying_key().as_bytes()),
        };
        let journal = Journal {
            identity: identity.clone(),
            seed: encode(&seed),
        };
        dir.atomic(
            "identity.pending.json",
            &serde_json::to_vec(&journal).unwrap(),
            true,
        )
        .unwrap();
        dir.atomic("device.key", &seed, true).unwrap();
        let result = Device::load_or_create(&dir).unwrap();
        assert_eq!(result.identity.installation_id, identity.installation_id);
        assert_eq!(
            dir.read("identity.pending.json", 4096, true),
            Err(Code::LicenseMissing)
        );
    }
}
