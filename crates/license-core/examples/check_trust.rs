fn main() {
    let trust = license_core::Trust::compiled().expect("invalid production trust");
    assert!(
        !trust.is_empty(),
        "release requires at least one trusted issuer key"
    );
    println!("Production trust parsed and validated");
}
