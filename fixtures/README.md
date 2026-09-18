# Public deterministic test fixtures

The private seeds in `TEST-ONLY-keys.json` are deliberately public and reproduce synthetic test vectors. Production builds reject these keys even when renamed or copied into a public trust registry.

Twenty lease cases cover signatures and policy decisions. `manifest.json` defines the fixed time/context and expected result. Some negative cases have valid signatures over invalid claims; others deliberately have invalid signatures. At-expiry and missing-DMI cases change test context rather than signing bytes.

`activate-request.json` is a device-signed synthetic request. `fingerprint-vector.json` contains synthetic raw identifiers for testing product-specific HMAC derivation. Actual activation never transmits the raw machine ID. The inventory examples include locally administered MACs and different online versus affinity-available CPU counts.

From the repository root on Linux:

```bash
python3 scripts/validate_assets.py
python3 scripts/verify_fixtures.py
```

[tests/asset-integrity.json](../tests/asset-integrity.json) preserves the byte-exact contract, fixture and migration baseline. The validator requires the manifest and complete protected-file coverage. It rejects missing entries and changed bytes.

`scripts/generate_fixtures.py` is retained for deliberate fixture maintenance and requires Python cryptography. Regeneration is not part of ordinary validation. Review intentional fixture or contract changes together with their expected outcomes and integrity entries; do not reserialize signing inputs during verification.
