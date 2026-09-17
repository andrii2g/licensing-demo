use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Code {
    Valid, LicenseMissing, LicenseMalformed, UnsupportedFormat, UnknownKey,
    InvalidSignature, ProductMismatch, FeatureMissing, InstallationMismatch,
    NotYetValid, ClockSuspect, LicenseExpired, IdentityUnavailable,
    MachineMismatch, CapacityExceeded, CapacityUnavailable, IoError, LeaseRollback, InternalError,
}
impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;
        f.write_str(s.trim_matches('"'))
    }
}
impl std::error::Error for Code {}
pub type Result<T> = std::result::Result<T, Code>;
pub fn require(ok: bool) -> Result<()> { if ok { Ok(()) } else { Err(Code::LicenseMalformed) } }
