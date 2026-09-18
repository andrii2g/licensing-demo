use crate::{ApiError, Result};
use license_core::*;
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, time::Duration};
pub fn open(path: &Path, busy_ms: u64) -> Result<Connection> {
    let parent = path.parent().ok_or(ApiError::new(400, "DATABASE_PATH"))?;
    let directory = license_store::SecureDir::open_service(parent)?;
    // Protected parent makes pathname-based SQLite opens safe against unprivileged replacement.
    if std::fs::symlink_metadata(path).is_ok() {
        directory.open_regular(
            path.file_name()
                .and_then(|s| s.to_str())
                .ok_or(ApiError::new(400, "DATABASE_PATH"))?,
            true,
        )?;
    }
    let c = Connection::open(path)?;
    c.busy_timeout(Duration::from_millis(busy_ms))?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|_| ApiError::new(500, "DATABASE_PERMISSIONS"))?;
    c.pragma_update(None, "foreign_keys", "ON")?;
    c.pragma_update(None, "journal_mode", "WAL")?;
    let exists: i64 = c.query_row(
        "SELECT count(*) FROM sqlite_master WHERE name='schema_migrations'",
        [],
        |r| r.get(0),
    )?;
    if exists == 0 {
        c.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    }
    Ok(c)
}
pub fn entitlement(c: &Connection, id: &str) -> Result<(Entitlement, String)> {
    c.query_row("SELECT license_id,customer_id,product,mode,binding_policy,expires_at,max_installations,lease_seconds,max_logical_processors,features_json,status FROM entitlements WHERE license_id=?1",[id],|r|{
        Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,i64>(5)?,r.get::<_,u32>(6)?,r.get::<_,u32>(7)?,r.get::<_,Option<u32>>(8)?,r.get::<_,String>(9)?,r.get::<_,String>(10)?))
    }).optional()?.ok_or(ApiError::new(401,"UNAUTHORIZED")).and_then(|v|Ok((Entitlement{
        license_id:v.0,customer_id:v.1,product:v.2,
        mode:serde_json::from_value(serde_json::Value::String(v.3)).map_err(|_|ApiError::new(500,"DATABASE_ERROR"))?,
        binding_policy:serde_json::from_value(serde_json::Value::String(v.4)).map_err(|_|ApiError::new(500,"DATABASE_ERROR"))?,
        expires_at:v.5,max_installations:v.6,lease_seconds:v.7,max_logical_processors:v.8,
        features:serde_json::from_str(&v.9).map_err(|_|ApiError::new(500,"DATABASE_ERROR"))?
    },v.10)))
}
pub struct Installation {
    pub license_id: String,
    pub key: Vec<u8>,
    pub status: String,
    pub binding: Binding,
    pub sequence: u64,
    pub expiry: i64,
    pub reserved: i64,
    pub retired_at: Option<i64>,
}
pub fn installation(c: &Connection, id: &str) -> Result<Option<Installation>> {
    let row=c.query_row("SELECT license_id,public_key,status,binding_json,sequence,last_lease_valid_until,reserved_until,retired_at FROM installations WHERE installation_id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Vec<u8>>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,u64>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,Option<i64>>(7)?))).optional()?;
    row.map(|v| {
        Ok(Installation {
            license_id: v.0,
            key: v.1,
            status: v.2,
            binding: license_core::envelope::strict(v.3.as_bytes(), 4096)?,
            sequence: v.4,
            expiry: v.5,
            reserved: v.6,
            retired_at: v.7,
        })
    })
    .transpose()
}
pub fn audit(
    c: &Connection,
    now: i64,
    actor: &str,
    action: &str,
    license: &str,
    id: Option<&str>,
) -> Result<()> {
    c.execute("INSERT INTO audit_events(occurred_at,request_id,actor_kind,action,license_id,installation_id,details_json) VALUES(?1,?2,?3,?4,?5,?6,'{}')",params![now,uuid::Uuid::new_v4().to_string(),actor,action,license,id])?;
    Ok(())
}
pub fn create(c: &mut Connection, e: &Entitlement, now: i64) -> Result<String> {
    e.validate()?;
    if e.expires_at <= now {
        return Err(ApiError::new(403, "ENTITLEMENT_EXPIRED"));
    }
    let token = license_core::envelope::encode(&license_core::crypto::random::<32>()?);
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO entitlements VALUES(?1,?2,?3,'active',?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            e.license_id,
            e.customer_id,
            e.product,
            crate::json(&e.mode)?.trim_matches('"'),
            crate::json(&e.binding_policy)?.trim_matches('"'),
            e.expires_at,
            e.max_installations,
            e.lease_seconds,
            e.max_logical_processors,
            crate::json(&e.features)?,
            now
        ],
    )?;
    tx.execute(
        "INSERT INTO activation_tokens VALUES(?1,?2,'active',?3)",
        params![crypto::hash(token.as_bytes()), e.license_id, now],
    )?;
    audit(&tx, now, "admin", "entitlement.create", &e.license_id, None)?;
    tx.commit()?;
    Ok(token)
}
pub fn revoke(c: &mut Connection, id: &str, now: i64) -> Result<()> {
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    if tx.execute(
        "UPDATE entitlements SET status='revoked' WHERE license_id=?1",
        [id],
    )? != 1
    {
        return Err(ApiError::new(404, "NOT_FOUND"));
    }
    tx.execute(
        "UPDATE activation_tokens SET status='revoked' WHERE license_id=?1",
        [id],
    )?;
    audit(&tx, now, "admin", "entitlement.revoke", id, None)?;
    tx.commit()?;
    Ok(())
}
pub fn retire(c: &Connection, id: &str, now: i64, actor: &str) -> Result<RetireResponse> {
    let i = installation(c, id)?.ok_or(ApiError::new(404, "NOT_FOUND"))?;
    let at = i.retired_at.unwrap_or(now);
    c.execute("UPDATE installations SET status='retired',reserved_until=last_lease_valid_until,retired_at=?2,updated_at=?2 WHERE installation_id=?1",params![id,at])?;
    audit(
        c,
        now,
        actor,
        "installation.retire",
        &i.license_id,
        Some(id),
    )?;
    Ok(RetireResponse {
        schema_version: 1,
        installation_id: id.into(),
        retired_at: at,
        reserved_until: i.expiry,
    })
}
