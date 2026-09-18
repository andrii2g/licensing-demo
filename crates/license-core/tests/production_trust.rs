#[cfg(not(feature = "dev"))]
#[test]
fn fixture_keys_cannot_be_trusted_even_renamed() {
    for key in [
        license_core::trust::FIXTURE_ISSUER,
        license_core::trust::FIXTURE_DEVICE,
    ] {
        assert!(license_core::Trust::new(vec![("renamed-key".into(), key.into())]).is_err());
    }
}
