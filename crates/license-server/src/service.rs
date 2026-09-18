use crate::Result;
use crate::{repository as repo, *};
use license_core::{
    envelope::{Envelope, encode, fixed},
    *,
};
use rusqlite::{Connection, OptionalExtension, params};
pub fn challenge(
    c: &mut Connection,
    req: &ChallengeRequest,
    token: Option<&str>,
    now: i64,
    config: &config::Config,
) -> Result<Challenge> {
    req.validate()?;
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let digest = if req.action == Action::Activate {
        Some(auth::token_digest(token)?)
    } else {
        None
    };
    let license_id = if let Some(d) = &digest {
        auth::token_entitlement(&tx, d)?
    } else {
        let i = repo::installation(&tx, &req.installation_id)?
            .ok_or(ApiError::new(401, "UNAUTHORIZED"))?;
        if i.status != "active" && req.action != Action::Retire {
            return Err(ApiError::new(410, "RETIRED"));
        }
        i.license_id
    };
    let (e, status) = repo::entitlement(&tx, &license_id)?;
    allowed(&e, &status, &req.product, now)?;
    tx.execute("DELETE FROM challenges WHERE expires_at<=?1", [now])?;
    tx.execute("DELETE FROM operations WHERE cache_expires_at<=?1", [now])?;
    let count:i64=tx.query_row("SELECT count(*) FROM challenges WHERE installation_id=?1 AND consumed_at IS NULL AND expires_at>?2",params![req.installation_id,now],|r|r.get(0))?;
    if count >= config.max_outstanding_challenges_per_installation as i64 {
        return Err(ApiError::new(429, "CHALLENGE_LIMIT"));
    }
    let out = Challenge {
        schema_version: 1,
        challenge_id: uuid::Uuid::new_v4().to_string(),
        nonce: encode(&crypto::random::<32>()?),
        expires_at: now + config.challenge_ttl_seconds as i64,
    };
    tx.execute(
        "INSERT INTO challenges VALUES(?1,?2,?3,?4,?5,?6,?7,?8,NULL,?9)",
        params![
            out.challenge_id,
            req.installation_id,
            license_id,
            req.action.as_str(),
            req.product,
            crypto::hash(out.nonce.as_bytes()),
            digest,
            out.expires_at,
            now
        ],
    )?;
    tx.commit()?;
    Ok(out)
}
fn allowed(e: &Entitlement, status: &str, product: &str, now: i64) -> Result<()> {
    if status != "active" || e.expires_at <= now || e.product != product {
        return Err(ApiError::new(403, "ENTITLEMENT_DENIED"));
    }
    Ok(())
}
pub struct Mutation<'a> {
    pub action: Action,
    pub route_id: Option<&'a str>,
    pub token: Option<&'a str>,
    pub now: i64,
    pub cache_seconds: u32,
}
pub fn mutate(
    c: &mut Connection,
    issuer: &issuer::Issuer,
    bytes: &[u8],
    operation: Mutation<'_>,
) -> Result<String> {
    let Mutation {
        action,
        route_id,
        token,
        now,
        cache_seconds,
    } = operation;
    let env = Envelope::parse(bytes)?;
    let req = DeviceRequest::parse(&env)?;
    if req.action != action || route_id.is_some_and(|id| id != req.installation_id) {
        return Err(ApiError::new(400, "SCOPE_MISMATCH"));
    }
    let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let existing = repo::installation(&tx, &req.installation_id)?;
    let token_digest = if action == Action::Activate {
        Some(auth::token_digest(token)?)
    } else {
        None
    };
    let license_id = if let Some(d) = &token_digest {
        auth::token_entitlement(&tx, d)?
    } else {
        existing
            .as_ref()
            .ok_or(ApiError::new(401, "UNAUTHORIZED"))?
            .license_id
            .clone()
    };
    let key = if action == Action::Activate {
        fixed::<32>(&req.installation_public_key)?.to_vec()
    } else {
        existing
            .as_ref()
            .ok_or(ApiError::new(401, "UNAUTHORIZED"))?
            .key
            .clone()
    };
    auth::proof(&env, &req, &key)?;
    let (ent, status) = repo::entitlement(&tx, &license_id)?;
    let retired = existing.as_ref().is_some_and(|i| i.status == "retired");
    if let Some(i) = &existing
        && (i.license_id != license_id || i.key != key)
    {
        return Err(ApiError::new(409, "INSTALLATION_CONFLICT"));
    }
    if retired && action != Action::Retire {
        return Err(ApiError::new(410, "RETIRED"));
    }
    if !(retired && action == Action::Retire) {
        allowed(&ent, &status, &req.product, now)?;
    }
    let digest = env.digest();
    let cached=tx.query_row("SELECT request_digest,response_json FROM operations WHERE installation_id=?1 AND action=?2 AND operation_id=?3 AND cache_expires_at>?4",params![req.installation_id,action.as_str(),req.operation_id,now],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional()?;
    if let Some((old, response)) = cached {
        if old != digest {
            return Err(ApiError::new(409, "IDEMPOTENCY_CONFLICT"));
        }
        tx.commit()?;
        return Ok(response);
    }
    if retired {
        return Err(ApiError::new(410, "RETIRED"));
    }
    let challenge=tx.query_row("SELECT installation_id,license_id,action,product,nonce_sha256,token_sha256,expires_at,consumed_at FROM challenges WHERE challenge_id=?1",[&req.challenge_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,i64>(6)?,r.get::<_,Option<i64>>(7)?))).optional()?.ok_or(ApiError::new(401,"CHALLENGE_INVALID"))?;
    if challenge.0 != req.installation_id
        || challenge.1 != license_id
        || challenge.2 != action.as_str()
        || challenge.3 != req.product
        || !auth::equal_digest(&challenge.4, &crypto::hash(req.nonce.as_bytes()))
        || challenge.5 != token_digest
        || challenge.6 <= now
        || challenge.7.is_some()
    {
        return Err(ApiError::new(401, "CHALLENGE_INVALID"));
    }
    let response = if action == Action::Retire {
        crate::json(&repo::retire(&tx, &req.installation_id, now, "device")?)?
    } else {
        let inventory = req
            .inventory
            .as_ref()
            .ok_or(ApiError::new(422, "INVENTORY_REQUIRED"))?;
        let binding = if let Some(i) = &existing {
            policy::binding(
                &i.binding,
                Some(&inventory.machine_id_hash),
                inventory.system_uuid_hash.as_deref(),
            )
            .map_err(|_| ApiError::new(409, "BINDING_CHANGED"))?;
            i.binding.clone()
        } else {
            let slots:i64=tx.query_row("SELECT count(*) FROM installations WHERE license_id=?1 AND (status='active' OR reserved_until>?2)",params![license_id,now],|r|r.get(0))?;
            if slots >= ent.max_installations as i64 {
                return Err(ApiError::new(409, "ACTIVATION_LIMIT"));
            }
            if ent.binding_policy == BindingPolicy::Host && inventory.system_uuid_hash.is_none() {
                return Err(ApiError::new(422, "IDENTITY_UNAVAILABLE"));
            }
            Binding {
                policy: ent.binding_policy.clone(),
                machine_id_hash: inventory.machine_id_hash.clone(),
                system_uuid_hash: if ent.binding_policy == BindingPolicy::Host {
                    inventory.system_uuid_hash.clone()
                } else {
                    None
                },
            }
        };
        policy::capacity(ent.max_logical_processors, inventory.cpu.logical_processors)
            .map_err(|_| ApiError::new(422, "CAPACITY_DENIED"))?;
        let sequence = existing.as_ref().map_or(1, |i| i.sequence + 1);
        let expiry = if ent.mode == Mode::ManualOffline {
            ent.expires_at
        } else {
            (now + ent.lease_seconds as i64).min(ent.expires_at)
        };
        let claims = Claims {
            schema_version: 1,
            issuer: "license-guard".into(),
            license_id: ent.license_id.clone(),
            customer_id: ent.customer_id.clone(),
            product: ent.product.clone(),
            installation_id: req.installation_id.clone(),
            installation_public_key_sha256: crypto::hash(&key),
            sequence,
            issued_at: now,
            not_before: (now - 120).max(0),
            lease_valid_until: expiry,
            entitlement_expires_at: ent.expires_at,
            mode: ent.mode,
            features: ent.features,
            binding: binding.clone(),
            max_logical_processors: ent.max_logical_processors,
        };
        // Signing and serialization happen before any commit; failure rolls back the entire mutation.
        let response = crate::json(&issuer.issue(&claims)?)?;
        tx.execute("INSERT INTO installations VALUES(?1,?2,?3,'active',?4,?5,?6,?7,0,?8,?8,NULL) ON CONFLICT(installation_id) DO UPDATE SET inventory_json=excluded.inventory_json,sequence=excluded.sequence,last_lease_valid_until=excluded.last_lease_valid_until,updated_at=excluded.updated_at",params![req.installation_id,license_id,key,crate::json(&binding)?,crate::json(inventory)?,sequence,expiry,now])?;
        response
    };
    tx.execute(
        "UPDATE challenges SET consumed_at=?2 WHERE challenge_id=?1",
        params![req.challenge_id, now],
    )?;
    tx.execute(
        "INSERT INTO operations VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            req.installation_id,
            action.as_str(),
            req.operation_id,
            digest,
            response,
            now,
            now + cache_seconds.max(86400) as i64
        ],
    )?;
    repo::audit(
        &tx,
        now,
        "device",
        action.as_str(),
        &license_id,
        Some(&req.installation_id),
    )?;
    tx.commit()?;
    Ok(response)
}
