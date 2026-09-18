use crate::{ApiError, config::Config, issuer::Issuer, repository as repo, service};
use ed25519_dalek::SigningKey;
use license_core::*;
use rusqlite::Connection;
fn config() -> Config {
    toml::from_str(include_str!("../../../examples/server.toml")).unwrap()
}
fn db() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    c
}
fn entitlement(slots: u32) -> Entitlement {
    let mut e: Entitlement =
        serde_json::from_str(include_str!("../../../examples/entitlement.json")).unwrap();
    e.max_installations = slots;
    e.expires_at = 1800145000;
    e.lease_seconds = 100;
    e
}
fn inventory() -> Inventory {
    serde_json::from_str(include_str!("../../../examples/inventory-vmware.json")).unwrap()
}
fn identity(n: u8) -> (Identity, SigningKey) {
    let k = SigningKey::from_bytes(&[n; 32]);
    (
        Identity {
            schema_version: 1,
            installation_id: uuid::Uuid::new_v4().to_string(),
            installation_public_key: envelope::encode(k.verifying_key().as_bytes()),
        },
        k,
    )
}
fn request(
    c: &mut Connection,
    i: &Identity,
    k: &SigningKey,
    a: Action,
    t: Option<&str>,
    now: i64,
) -> Vec<u8> {
    let ch = service::challenge(
        c,
        &ChallengeRequest {
            schema_version: 1,
            action: a,
            installation_id: i.installation_id.clone(),
            product: "worker-suite".into(),
        },
        t,
        now,
        &config(),
    )
    .unwrap();
    let r = DeviceRequest {
        schema_version: 1,
        action: a,
        operation_id: uuid::Uuid::new_v4().to_string(),
        challenge_id: ch.challenge_id,
        nonce: ch.nonce,
        installation_id: i.installation_id.clone(),
        product: "worker-suite".into(),
        installation_public_key: i.installation_public_key.clone(),
        inventory: if a == Action::Retire {
            None
        } else {
            Some(inventory())
        },
    };
    serde_json::to_vec(&crypto::sign(&r, REQUEST_TYPE, &i.installation_id, k).unwrap()).unwrap()
}
fn issue(
    c: &mut Connection,
    issuer: &Issuer,
    r: &[u8],
    a: Action,
    t: Option<&str>,
    now: i64,
) -> std::result::Result<String, ApiError> {
    let e = envelope::Envelope::parse(r).unwrap();
    let req = DeviceRequest::parse(&e).unwrap();
    service::mutate(
        c,
        issuer,
        r,
        service::Mutation {
            action: a,
            route_id: if a == Action::Activate {
                None
            } else {
                Some(&req.installation_id)
            },
            token: t,
            now,
            cache_seconds: 86400,
        },
    )
}
#[test]
fn activation_renew_replay_retirement_boundary() {
    let now = 1800144000;
    let mut c = db();
    let issuer = Issuer::new(
        "dev-generated".into(),
        SigningKey::from_bytes(&crypto::random::<32>().unwrap()),
    )
    .unwrap();
    let token = repo::create(&mut c, &entitlement(1), now).unwrap();
    let (i, k) = identity(70);
    let request1 = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    let response = issue(
        &mut c,
        &issuer,
        &request1,
        Action::Activate,
        Some(&token),
        now,
    )
    .unwrap();
    assert_eq!(
        response,
        issue(
            &mut c,
            &issuer,
            &request1,
            Action::Activate,
            Some(&token),
            now
        )
        .unwrap()
    );
    let inv = inventory();
    let context = Context {
        now,
        product: "worker-suite".into(),
        required_features: vec!["messaging".into()],
        identity: i.clone(),
        machine_id_hash: Some(inv.machine_id_hash),
        system_uuid_hash: inv.system_uuid_hash,
        logical_processors: inv.cpu.logical_processors,
    };
    let trust = Trust::new(vec![(issuer.kid.clone(), issuer.public())]).unwrap();
    assert_eq!(
        verify(response.as_bytes(), &trust, &context)
            .unwrap()
            .claims
            .sequence,
        1
    );
    let e = envelope::Envelope::parse(&request1).unwrap();
    let mut mutated = DeviceRequest::parse(&e).unwrap();
    mutated.inventory.as_mut().unwrap().hostname = "changed".into();
    let mutated =
        serde_json::to_vec(&crypto::sign(&mutated, REQUEST_TYPE, &i.installation_id, &k).unwrap())
            .unwrap();
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &mutated,
            Action::Activate,
            Some(&token),
            now
        )
        .unwrap_err()
        .code,
        "IDEMPOTENCY_CONFLICT"
    );
    let mut replay = DeviceRequest::parse(&e).unwrap();
    replay.operation_id = uuid::Uuid::new_v4().to_string();
    let replay =
        serde_json::to_vec(&crypto::sign(&replay, REQUEST_TYPE, &i.installation_id, &k).unwrap())
            .unwrap();
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &replay,
            Action::Activate,
            Some(&token),
            now
        )
        .unwrap_err()
        .code,
        "CHALLENGE_INVALID"
    );
    let renew = request(&mut c, &i, &k, Action::Renew, None, now + 10);
    let response = issue(&mut c, &issuer, &renew, Action::Renew, None, now + 10).unwrap();
    assert_eq!(
        policy::authenticate(response.as_bytes(), &trust)
            .unwrap()
            .claims
            .sequence,
        2
    );
    let retire = request(&mut c, &i, &k, Action::Retire, None, now + 20);
    let retired = issue(&mut c, &issuer, &retire, Action::Retire, None, now + 20).unwrap();
    assert_eq!(
        retired,
        issue(&mut c, &issuer, &retire, Action::Retire, None, now + 20).unwrap()
    );
    assert_eq!(
        issue(&mut c, &issuer, &renew, Action::Renew, None, now + 20)
            .unwrap_err()
            .status,
        410
    );
    let (j, l) = identity(71);
    let replacement = request(&mut c, &j, &l, Action::Activate, Some(&token), now + 20);
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &replacement,
            Action::Activate,
            Some(&token),
            now + 20
        )
        .unwrap_err()
        .code,
        "ACTIVATION_LIMIT"
    );
    assert!(
        issue(
            &mut c,
            &issuer,
            &replacement,
            Action::Activate,
            Some(&token),
            now + 110
        )
        .is_ok()
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM installations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        2
    );
}
#[test]
fn revoke_expire_and_scope_after_challenge() {
    let now = 1800144000;
    let mut c = db();
    let issuer = Issuer::new("issuer".into(), SigningKey::from_bytes(&[72; 32])).unwrap();
    let token = repo::create(&mut c, &entitlement(1), now).unwrap();
    let (i, k) = identity(73);
    let r = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    assert_eq!(
        issue(&mut c, &issuer, &r, Action::Renew, None, now)
            .unwrap_err()
            .status,
        400
    );
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &r,
            Action::Activate,
            Some(&token),
            now + 300
        )
        .unwrap_err()
        .code,
        "CHALLENGE_INVALID"
    );
    repo::revoke(&mut c, "LIC-2026-001234", now).unwrap();
    assert_eq!(
        issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now)
            .unwrap_err()
            .status,
        401
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM installations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn parallel_one_slot() {
    let d = tempfile::tempdir().unwrap();
    let path = d.path().join("race.db");
    let mut c = Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    let now = 1800144000;
    let token = repo::create(&mut c, &entitlement(1), now).unwrap();
    let requests = (80..82)
        .map(|n| {
            let (i, k) = identity(n);
            request(&mut c, &i, &k, Action::Activate, Some(&token), now)
        })
        .collect::<Vec<_>>();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = requests
        .into_iter()
        .map(|r| {
            let p = path.clone();
            let t = token.clone();
            let b = barrier.clone();
            std::thread::spawn(move || {
                let mut c = Connection::open(p).unwrap();
                c.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
                let issuer =
                    Issuer::new("issuer".into(), SigningKey::from_bytes(&[83; 32])).unwrap();
                b.wait();
                issue(&mut c, &issuer, &r, Action::Activate, Some(&t), now)
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .next()
            .unwrap()
            .code,
        "ACTIVATION_LIMIT"
    );
}
#[tokio::test]
async fn http_limits_and_shapes() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let app = crate::routes::App::new(
        db(),
        Issuer::new("issuer".into(), SigningKey::from_bytes(&[84; 32])).unwrap(),
        config(),
    );
    let router = crate::routes::router(app);
    let health = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), 200);
    for body in [
        "{\"schema_version\":1,\"schema_version\":1}".to_string(),
        "x".repeat(131073),
    ] {
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/challenges")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), 400);
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema_version"],
            1
        );
    }
}

#[test]
fn offline_binding_keys_and_policy_are_server_owned() {
    let now = 1800144000;
    let mut c = db();
    let issuer = Issuer::new("issuer".into(), SigningKey::from_bytes(&[90; 32])).unwrap();
    let mut ent = entitlement(1);
    ent.mode = Mode::ManualOffline;
    let token = repo::create(&mut c, &ent, now).unwrap();
    let (i, k) = identity(91);
    let r = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    let out = issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now).unwrap();
    let trust = Trust::new(vec![(issuer.kid.clone(), issuer.public())]).unwrap();
    let claims = policy::authenticate(out.as_bytes(), &trust).unwrap().claims;
    assert_eq!(claims.lease_valid_until, ent.expires_at);
    assert_eq!(claims.mode, Mode::ManualOffline);
    let r = request(&mut c, &i, &k, Action::Renew, None, now + 1);
    let e = envelope::Envelope::parse(&r).unwrap();
    let mut req = DeviceRequest::parse(&e).unwrap();
    req.inventory.as_mut().unwrap().hostname = "normal-name-change".into();
    req.inventory.as_mut().unwrap().network_interfaces.clear();
    let bytes =
        serde_json::to_vec(&crypto::sign(&req, REQUEST_TYPE, &i.installation_id, &k).unwrap())
            .unwrap();
    assert!(issue(&mut c, &issuer, &bytes, Action::Renew, None, now + 1).is_ok());
    let r = request(&mut c, &i, &k, Action::Renew, None, now + 2);
    let e = envelope::Envelope::parse(&r).unwrap();
    let mut req = DeviceRequest::parse(&e).unwrap();
    req.inventory.as_mut().unwrap().machine_id_hash = "0".repeat(64);
    let bytes =
        serde_json::to_vec(&crypto::sign(&req, REQUEST_TYPE, &i.installation_id, &k).unwrap())
            .unwrap();
    assert_eq!(
        issue(&mut c, &issuer, &bytes, Action::Renew, None, now + 2)
            .unwrap_err()
            .code,
        "BINDING_CHANGED"
    );
    let other = SigningKey::from_bytes(&[92; 32]);
    req.installation_public_key = envelope::encode(other.verifying_key().as_bytes());
    let bytes =
        serde_json::to_vec(&crypto::sign(&req, REQUEST_TYPE, &i.installation_id, &other).unwrap())
            .unwrap();
    assert_eq!(
        issue(&mut c, &issuer, &bytes, Action::Renew, None, now + 2)
            .unwrap_err()
            .code,
        "INVALID_SIGNATURE"
    );
}
#[test]
fn signing_and_commit_failure_roll_back_consumption() {
    let now = 1800144000;
    let mut c = db();
    let mut issuer = Issuer::new("issuer".into(), SigningKey::from_bytes(&[93; 32])).unwrap();
    let token = repo::create(&mut c, &entitlement(1), now).unwrap();
    let (i, k) = identity(94);
    let r = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    issuer.fail_signing = true;
    assert!(issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now).is_err());
    issuer.fail_signing = false;
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM challenges WHERE consumed_at IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    c.execute_batch("CREATE TRIGGER fail_audit BEFORE INSERT ON audit_events BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now).is_err());
    assert_eq!(
        c.query_row("SELECT count(*) FROM installations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM challenges WHERE consumed_at IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    c.execute_batch("DROP TRIGGER fail_audit;").unwrap();
    assert!(issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now).is_ok());
}
#[test]
fn database_busy_is_retryable_and_no_issuance_after_entitlement_expiry() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("busy.db");
    let mut c = Connection::open(&p).unwrap();
    c.execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    let now = 1800144000;
    let token = repo::create(&mut c, &entitlement(1), now).unwrap();
    let (i, k) = identity(95);
    let r = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    let blocker = Connection::open(&p).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    c.busy_timeout(std::time::Duration::ZERO).unwrap();
    let issuer = Issuer::new("issuer".into(), SigningKey::from_bytes(&[96; 32])).unwrap();
    assert_eq!(
        issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now)
            .unwrap_err()
            .status,
        503
    );
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &r,
            Action::Activate,
            Some(&token),
            now + 1000
        )
        .unwrap_err()
        .code,
        "ENTITLEMENT_DENIED"
    );
}
#[test]
fn challenge_limits_cleanup_and_active_slots_persist() {
    let now = 1800144000;
    let mut c = db();
    let issuer = Issuer::new("issuer".into(), SigningKey::from_bytes(&[97; 32])).unwrap();
    let mut ent = entitlement(1);
    ent.expires_at = now + 200000;
    let token = repo::create(&mut c, &ent, now).unwrap();
    let (i, k) = identity(98);
    let r = request(&mut c, &i, &k, Action::Activate, Some(&token), now);
    issue(&mut c, &issuer, &r, Action::Activate, Some(&token), now).unwrap();
    let q = ChallengeRequest {
        schema_version: 1,
        action: Action::Renew,
        installation_id: i.installation_id.clone(),
        product: "worker-suite".into(),
    };
    for _ in 0..10 {
        service::challenge(&mut c, &q, None, now, &config()).unwrap();
    }
    assert_eq!(
        service::challenge(&mut c, &q, None, now, &config())
            .err()
            .unwrap()
            .status,
        429
    );
    service::challenge(&mut c, &q, None, now + 86401, &config()).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM operations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM installations", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    let (j, l) = identity(99);
    let new = request(&mut c, &j, &l, Action::Activate, Some(&token), now + 86401);
    assert_eq!(
        issue(
            &mut c,
            &issuer,
            &new,
            Action::Activate,
            Some(&token),
            now + 86401
        )
        .unwrap_err()
        .code,
        "ACTIVATION_LIMIT"
    );
}
#[tokio::test]
async fn real_http_challenge_activation_and_authenticated_retry() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let now = license_core::now();
    let mut c = db();
    let mut e = entitlement(1);
    e.expires_at = now + 1000;
    let token = repo::create(&mut c, &e, now).unwrap();
    let (i, k) = identity(100);
    let app = crate::routes::App::new(
        c,
        Issuer::new("issuer".into(), SigningKey::from_bytes(&[101; 32])).unwrap(),
        config(),
    );
    let router = crate::routes::router(app);
    let q = ChallengeRequest {
        schema_version: 1,
        action: Action::Activate,
        installation_id: i.installation_id.clone(),
        product: "worker-suite".into(),
    };
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/challenges")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&q).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let ch: Challenge = envelope::strict(&bytes, 65536).unwrap();
    let req = DeviceRequest {
        schema_version: 1,
        action: Action::Activate,
        operation_id: uuid::Uuid::new_v4().to_string(),
        challenge_id: ch.challenge_id,
        nonce: ch.nonce,
        installation_id: i.installation_id.clone(),
        product: "worker-suite".into(),
        installation_public_key: i.installation_public_key,
        inventory: Some(inventory()),
    };
    let bytes =
        serde_json::to_vec(&crypto::sign(&req, REQUEST_TYPE, &i.installation_id, &k).unwrap())
            .unwrap();
    let mut previous = None;
    for _ in 0..2 {
        let r = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/activations")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(bytes.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        let result = r.into_body().collect().await.unwrap().to_bytes();
        if let Some(p) = previous {
            assert_eq!(result, p);
        }
        previous = Some(result);
    }
    let r = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/activations")
                .header("content-type", "application/json")
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
}
