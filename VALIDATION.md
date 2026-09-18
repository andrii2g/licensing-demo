# Kit validation report

This report concerns the implementation kit and included sample source, not a completed Rust licensing product.

## Checks performed successfully
- Parsed all supplied JSON and license envelopes; rejected duplicate keys in strict parser checks.
- Checked all 14 JSON Schema local references and representative positive examples using the bundled limited-keyword checker. This is not a general JSON Schema meta-schema validation.
- Parsed OpenAPI 3.1 YAML and checked local schema references for all six paths.
- Parsed both TOML configuration examples and systemd unit INI syntax.
- Independently verified 20 Ed25519 lease vectors using Python cryptography, including expected signature and policy failures.
- Verified the device-signed activation request and product-specific fingerprint vectors.
- Executed the SQLite reference migration against an in-memory database; integrity check passed.
- Compiled a C translation unit including the ABI header with warnings as errors, syntax-only.
- Parsed the .NET project XML and confirmed net10.0 and PublishAot=true.
- Checked the Linux publish helper with bash -n and parsed all Python scripts.
- Reviewed sample startup ordering, validity versus ABI status, monotonic deadline handling, graceful drain and ordinary-stop classification.
- Created a manifest with SHA-256 and sizes for every deliverable file, excluding the manifest itself; verified the archive and manifest.

## Not executed in this environment
Rust and .NET SDKs were not installed. Therefore cargo builds/tests, C# compilation, package restore, Native AOT publishing, real ABI invocation, end-to-end activation and actual systemd execution were not run. No claim of passing those checks is made.
No real physical host or VMware enrollment was performed. DMI permission persistence, UUID changes on migrations/clones and distribution compatibility need staging tests.
Systemd template syntax parsing does not prove the referenced paths/users exist or the service runs.
The Rust implementations are intentionally work for Codex; the C# sample is supplied as source to compile and verify against that implementation.

## Reproduce available checks
~~~bash
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_kit.py
PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify_fixtures.py
~~~
The second command needs Python cryptography. These checks supplement the required product tests in docs/09-testing.md and ACCEPTANCE_CRITERIA.md.
