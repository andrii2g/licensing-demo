#![forbid(unsafe_code)]
pub mod claims;
pub mod crypto;
pub mod envelope;
pub mod error;
pub mod policy;
pub mod trust;
pub use claims::*;
pub use error::{Code, Result};
pub use policy::{Context, ValidationResult, VerifiedLease, verify};
pub use trust::Trust;
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
#[cfg(test)]
mod tests;
