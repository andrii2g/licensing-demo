# Public, deterministic test fixtures

All private seeds in TEST-ONLY-keys.json are deliberately public. They only reproduce synthetic test vectors. A production native library must reject this trust anchor even if someone copies it into a trust file.

Twenty lease cases cover cryptographic integrity and policy decisions. manifest.json supplies the fixed clock/context and expected code. Some negative files have valid signatures over invalid claims; others deliberately have bad signatures. Do not treat every .lic file as an example of acceptable input.
At-expiry and missing-DMI cases use context overrides, not modified signed claims.
activate-request.json is a real device-signed synthetic activation request; no actual server enrollment happened.
inventory examples include locally administered VMware MACs and different online versus affinity-available CPU counts.
fingerprint-vector.json publishes raw synthetic IDs only for deterministic tests; real activation must never transmit raw machine ID.
Regenerate with scripts/generate_fixtures.py (Python cryptography required). It will invalidate BUNDLE_MANIFEST.json hashes until the bundle manifest is rebuilt.

