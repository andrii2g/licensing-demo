use crate::{ApiError, Result};
use license_core::{
    envelope::{Envelope, fixed},
    *,
};
use rusqlite::{Connection, OptionalExtension};
use subtle::ConstantTimeEq;
pub fn token_digest(token: Option<&str>) -> Result<String> {
    let token = token.ok_or(ApiError::new(401, "UNAUTHORIZED"))?;
    fixed::<32>(token).map_err(|_| ApiError::new(401, "UNAUTHORIZED"))?;
    Ok(crypto::hash(token.as_bytes()))
}
pub fn token_entitlement(c: &Connection, digest: &str) -> Result<String> {
    c.query_row(
        "SELECT license_id FROM activation_tokens WHERE token_sha256=?1 AND status='active'",
        [digest],
        |r| r.get(0),
    )
    .optional()?
    .ok_or(ApiError::new(401, "UNAUTHORIZED"))
}
pub fn proof(e: &Envelope, r: &DeviceRequest, key: &[u8]) -> Result<()> {
    let h = e.header(REQUEST_TYPE)?;
    if h.kid != r.installation_id || fixed::<32>(&r.installation_public_key)?.as_slice() != key {
        return Err(ApiError::new(401, "INVALID_SIGNATURE"));
    }
    crypto::verify_signature(
        e,
        &key.try_into()
            .map_err(|_| ApiError::new(401, "INVALID_SIGNATURE"))?,
    )
    .map_err(|_| ApiError::new(401, "INVALID_SIGNATURE"))
}
pub fn equal_digest(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
