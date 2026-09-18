use super::*;
use crate::envelope::{self, Envelope};
use serde_json::{Value, json};
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
    .unwrap()
}
fn setup() -> (Trust, Context) {
    let v: Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    let c = &v["base_context"];
    (
        Trust::new(vec![(
            "TEST-ONLY-issuer-v1".into(),
            trust::FIXTURE_ISSUER.into(),
        )])
        .unwrap(),
        Context {
            now: c["now"].as_i64().unwrap(),
            product: c["product"].as_str().unwrap().into(),
            required_features: vec!["messaging".into()],
            identity: serde_json::from_value(c["identity"].clone()).unwrap(),
            machine_id_hash: Some(c["machine_id_hash"].as_str().unwrap().into()),
            system_uuid_hash: Some(c["system_uuid_hash"].as_str().unwrap().into()),
            logical_processors: Some(8),
        },
    )
}
#[test]
fn all_supplied_vectors() {
    let manifest: Value = serde_json::from_slice(&fixture("manifest.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let (t, mut c) = setup();
        let changes = &case["context_changes"];
        if let Some(v) = changes.get("now") {
            c.now = v.as_i64().unwrap();
        }
        if let Some(v) = changes.get("logical_processors") {
            c.logical_processors = v.as_u64().map(|x| x as u32);
        }
        if let Some(v) = changes.get("system_uuid_hash") {
            c.system_uuid_hash = v.as_str().map(str::to_string);
        }
        let actual = verify(&fixture(case["file"].as_str().unwrap()), &t, &c)
            .map_or_else(|c| c, |_| Code::Valid);
        assert_eq!(
            actual.to_string(),
            case["expected"].as_str().unwrap(),
            "{}",
            case["file"]
        );
    }
}
#[test]
fn strict_json_and_base64() {
    for bytes in [
        br#"{"a":{"b":1,"b":2}}"#.as_slice(),
        b"{\"x\":1.0}",
        b"{} trailing",
        b"\xff",
        b"{\"n\":18446744073709551616}",
    ] {
        assert!(envelope::strict::<Value>(bytes, 65536).is_err());
    }
    let nested = format!("{}0{}", "[".repeat(18), "]".repeat(18));
    assert!(envelope::strict::<Value>(nested.as_bytes(), 65536).is_err());
    assert!(envelope::decode("Zh", 1).is_err());
    assert!(envelope::decode("Zg=", 1).is_err());
    assert!(Envelope::parse(&vec![b' '; 65537]).is_err());
}
#[test]
fn missing_nullable_and_unknown_nested_fail() {
    let mut c: Value = serde_json::from_slice(&fixture("base-claims.json")).unwrap();
    c.as_object_mut().unwrap().remove("max_logical_processors");
    assert!(serde_json::from_value::<Claims>(c).is_err());
    let mut c: Value = serde_json::from_slice(&fixture("base-claims.json")).unwrap();
    c["binding"]["extra"] = json!(1);
    assert!(serde_json::from_value::<Claims>(c).is_err());
}
#[test]
fn device_fixture_and_domain_separation() {
    let e = Envelope::parse(&fixture("activate-request.json")).unwrap();
    e.header(REQUEST_TYPE).unwrap();
    let req = DeviceRequest::parse(&e).unwrap();
    crypto::verify_signature(
        &e,
        &envelope::fixed::<32>(&req.installation_public_key).unwrap(),
    )
    .unwrap();
    assert_eq!(e.header(LEASE_TYPE).err(), Some(Code::UnsupportedFormat));
}
#[test]
fn exact_times_identity_and_capacity() {
    let (t, mut c) = setup();
    let bytes = fixture("valid.lic");
    let v = policy::authenticate(&bytes, &t).unwrap();
    c.now = v.claims.not_before;
    assert!(verify(&bytes, &t, &c).is_ok());
    c.now -= 1;
    assert_eq!(verify(&bytes, &t, &c).unwrap_err(), Code::NotYetValid);
    c.now = v.claims.lease_valid_until - 1;
    assert!(verify(&bytes, &t, &c).is_ok());
    c.now += 1;
    assert_eq!(verify(&bytes, &t, &c).unwrap_err(), Code::LicenseExpired);
    assert_eq!(
        policy::capacity(Some(4), Some(8)),
        Err(Code::CapacityExceeded)
    );
    assert_eq!(policy::capacity(None, None), Ok(()));
}
#[test]
fn signatures_use_transmitted_bytes() {
    use ed25519_dalek::{Signer, SigningKey};
    let seed: [u8; 32] = std::array::from_fn(|i| (i + 1) as u8);
    let key = SigningKey::from_bytes(&seed);
    let (t, c) = setup();
    let mut e = Envelope::parse(&fixture("valid.lic")).unwrap();
    let claims: Value = e.payload().unwrap();
    e.payload = envelope::encode(serde_json::to_string_pretty(&claims).unwrap().as_bytes());
    e.signature = envelope::encode(&key.sign(&e.signing_bytes()).to_bytes());
    assert!(verify(&serde_json::to_vec(&e).unwrap(), &t, &c).is_ok());
    e.payload = envelope::encode(serde_json::to_string(&claims).unwrap().as_bytes());
    assert_eq!(
        verify(&serde_json::to_vec(&e).unwrap(), &t, &c).unwrap_err(),
        Code::InvalidSignature
    );
}

#[test]
fn positional_arrays_are_not_v1_objects() {
    use ed25519_dalek::SigningKey;
    let key = SigningKey::from_bytes(&std::array::from_fn(|i| (i + 1) as u8));
    let (t, ctx) = setup();
    let base: Claims = serde_json::from_slice(&fixture("base-claims.json")).unwrap();
    let array = json!([
        base.schema_version,
        base.issuer,
        base.license_id,
        base.customer_id,
        base.product,
        base.installation_id,
        base.installation_public_key_sha256,
        base.sequence,
        base.issued_at,
        base.not_before,
        base.lease_valid_until,
        base.entitlement_expires_at,
        base.mode,
        base.features,
        base.binding,
        base.max_logical_processors
    ]);
    let signed = crypto::sign(&array, LEASE_TYPE, "TEST-ONLY-issuer-v1", &key).unwrap();
    assert_eq!(
        verify(&serde_json::to_vec(&signed).unwrap(), &t, &ctx).unwrap_err(),
        Code::LicenseMalformed
    );
    let e = Envelope::parse(&fixture("valid.lic")).unwrap();
    assert!(
        Envelope::parse(
            &serde_json::to_vec(&json!([e.protected, e.payload, e.signature])).unwrap()
        )
        .is_err()
    );
    let mut nested: Value = serde_json::from_slice(&fixture("base-claims.json")).unwrap();
    nested["binding"] = json!([
        "linux-host-v1",
        base.binding.machine_id_hash,
        base.binding.system_uuid_hash
    ]);
    let signed = crypto::sign(&nested, LEASE_TYPE, "TEST-ONLY-issuer-v1", &key).unwrap();
    assert_eq!(
        verify(&serde_json::to_vec(&signed).unwrap(), &t, &ctx).unwrap_err(),
        Code::LicenseMalformed
    );
    let mut i: Value =
        serde_json::from_str(include_str!("../../../examples/inventory-vmware.json")).unwrap();
    i["cpu"] = json!([null, 1, 4, 8, 8]);
    assert!(serde_json::from_value::<Inventory>(i).is_err());
}
