use crate::{
    crypto,
    envelope::{Envelope, fixed},
    *,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug)]
pub struct Context {
    pub now: i64,
    pub product: String,
    pub required_features: Vec<String>,
    pub identity: Identity,
    pub machine_id_hash: Option<String>,
    pub system_uuid_hash: Option<String>,
    pub logical_processors: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct VerifiedLease {
    pub claims: Claims,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationResult {
    pub schema_version: u32,
    pub valid: bool,
    pub code: Code,
    pub checked_at: i64,
    pub license_id: Option<String>,
    pub installation_id: Option<String>,
    pub sequence: Option<u64>,
    pub lease_valid_until: Option<i64>,
    pub entitlement_expires_at: Option<i64>,
    pub features: Vec<String>,
    pub lease_digest: Option<String>,
}
impl ValidationResult {
    pub fn denied(code: Code, now: i64) -> Self {
        Self {
            schema_version: 1,
            valid: false,
            code,
            checked_at: now,
            license_id: None,
            installation_id: None,
            sequence: None,
            lease_valid_until: None,
            entitlement_expires_at: None,
            features: vec![],
            lease_digest: None,
        }
    }
    pub fn accepted(v: &VerifiedLease, now: i64) -> Self {
        let c = &v.claims;
        Self {
            schema_version: 1,
            valid: true,
            code: Code::Valid,
            checked_at: now,
            license_id: Some(c.license_id.clone()),
            installation_id: Some(c.installation_id.clone()),
            sequence: Some(c.sequence),
            lease_valid_until: Some(c.lease_valid_until),
            entitlement_expires_at: Some(c.entitlement_expires_at),
            features: c.features.clone(),
            lease_digest: Some(v.digest.clone()),
        }
    }
}
pub fn authenticate(bytes: &[u8], trust: &Trust) -> Result<VerifiedLease> {
    let e = Envelope::parse(bytes)?;
    let h = e.header(LEASE_TYPE)?;
    crypto::verify_signature(&e, trust.key(&h.kid)?)?;
    let claims: Claims = e.payload()?;
    claims.validate()?;
    Ok(VerifiedLease {
        claims,
        digest: e.digest(),
    })
}
pub fn evaluate(c: &Claims, ctx: &Context) -> Result<()> {
    if c.product != ctx.product {
        return Err(Code::ProductMismatch);
    }
    if !ctx.required_features.iter().all(|f| c.features.contains(f)) {
        return Err(Code::FeatureMissing);
    }
    ctx.identity
        .validate()
        .map_err(|_| Code::InstallationMismatch)?;
    if c.installation_id != ctx.identity.installation_id
        || c.installation_public_key_sha256
            != crypto::hash(&fixed::<32>(&ctx.identity.installation_public_key)?)
    {
        return Err(Code::InstallationMismatch);
    }
    if ctx.now < c.not_before {
        return Err(Code::NotYetValid);
    }
    if c.issued_at > ctx.now.saturating_add(120) {
        return Err(Code::ClockSuspect);
    }
    if ctx.now >= c.lease_valid_until {
        return Err(Code::LicenseExpired);
    }
    binding(
        &c.binding,
        ctx.machine_id_hash.as_deref(),
        ctx.system_uuid_hash.as_deref(),
    )?;
    capacity(c.max_logical_processors, ctx.logical_processors)
}
pub fn binding(b: &Binding, m: Option<&str>, s: Option<&str>) -> Result<()> {
    let m = m.ok_or(Code::IdentityUnavailable)?;
    if b.machine_id_hash != m {
        return Err(Code::MachineMismatch);
    }
    if b.policy == BindingPolicy::Host {
        let s = s.ok_or(Code::IdentityUnavailable)?;
        if b.system_uuid_hash.as_deref() != Some(s) {
            return Err(Code::MachineMismatch);
        }
    }
    Ok(())
}
pub fn capacity(limit: Option<u32>, count: Option<u32>) -> Result<()> {
    if let Some(limit) = limit {
        let count = count.ok_or(Code::CapacityUnavailable)?;
        if count > limit {
            return Err(Code::CapacityExceeded);
        }
    }
    Ok(())
}
pub fn verify(bytes: &[u8], trust: &Trust, ctx: &Context) -> Result<VerifiedLease> {
    let v = authenticate(bytes, trust)?;
    evaluate(&v.claims, ctx)?;
    Ok(v)
}
