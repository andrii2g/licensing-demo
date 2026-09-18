#![forbid(unsafe_code)]
use license_core::*;
use serde::Deserialize;
use std::path::Path;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub license_path: String,
    pub identity_path: String,
    pub product: String,
    pub required_features: Vec<String>,
}
impl Request {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let r: Self = envelope::object(bytes, 8192)?;
        error::require(r.schema_version == 1)?;
        for p in [&r.license_path, &r.identity_path] {
            error::require(p.len() <= 4096 && Path::new(p).is_absolute() && !p.contains('\0'))?;
        }
        identifier(&r.product)?;
        features(&r.required_features, 0, 16)?;
        Ok(r)
    }
}
fn trust() -> Result<Trust> {
    #[cfg(feature = "dev")]
    if let Some(p) = std::env::var_os("LICENSE_GUARD_DEV_TRUST") {
        return Trust::new(envelope::strict(
            &license_store::read_absolute(&std::path::PathBuf::from(p), 16384, false)?,
            16384,
        )?);
    }
    Trust::compiled()
}
pub fn validate(r: &Request) -> ValidationResult {
    let now = now();
    let result = (|| {
        let bytes = license_store::read_absolute(Path::new(&r.license_path), 65536, false)?;
        let identity: Identity = envelope::object(
            &license_store::read_absolute(Path::new(&r.identity_path), 4096, false)?,
            4096,
        )?;
        // Authenticate before collecting host facts. No unverified claims are exposed.
        let verified = policy::authenticate(&bytes, &trust()?)?;
        let host = license_host::collect(&r.product, now)?;
        let ctx = Context {
            now,
            product: r.product.clone(),
            required_features: r.required_features.clone(),
            identity,
            machine_id_hash: Some(host.machine_id_hash),
            system_uuid_hash: host.system_uuid_hash,
            logical_processors: host.cpu.logical_processors,
        };
        policy::evaluate(&verified.claims, &ctx)?;
        Ok::<_, Code>(verified)
    })();
    match result {
        Ok(v) => ValidationResult::accepted(&v, now),
        Err(c) => ValidationResult::denied(c, now),
    }
}
