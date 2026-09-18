# License Guard implementation status

Updated 2026-09-18. M0–M8 are implemented. All automated acceptance gates passed locally, including real C/.NET interop, Native AOT, clean-checkout execution and separated release archives. Real deployment staging remains listed below; no production endpoint or real host enrollment was used.

## Milestones

| Milestone | Completed implementation and evidence |
|---|---|
| M0 — repository | Seven-crate Cargo workspace; Rust 1.97.1, edition 2024/MSRV 1.97; .NET SDK 10.0.401 and Hosting 10.0.12. Cargo and separate normal/AOT NuGet dependencies are locked. |
| M1 — verifier | Bounded object-only JSON, recursive duplicate/unknown-field rejection, canonical base64url, exact transmitted signing bytes, strict Ed25519 and typed policy results. All 20 supplied lease vectors pass. |
| M2 — host/store | Direct Linux inventory and HMAC binding, online CPUs distinct from affinity; no-follow descriptor-relative storage, ownership/mode enforcement, atomic replacement, high-water recovery and recoverable installation identity. |
| M3 — server | Independently runnable Axum/SQLite API and protected local admin; scoped challenges, device proof, transactional slots, signed issuance, authenticated response caching and retirement reservations. |
| M4 — CLI | inspect, activate, renew, status, verify, retire and local remove; durable exact pending requests, bounded HTTPS transport and verified atomic installation. A deliberately lost committed response causes an exact retry and only one issuance. |
| M5 — ABI/managed | Supplied C header implemented by liblicense_guard.so; reusable source-generated LicenseGuard.Managed wrapper; actual C/C++ and managed-process tests. Startup denial precedes all job work. |
| M6 — lifecycle | Issuer-controlled renewable/finite manual-offline policy; live higher-sequence renewal, outage tolerance until expiry, terminal invalid updates, admission closure, 30-second drain and distinct 78/0 exits. |
| M7 — Native AOT | Published linux-x64 executable calls the actual Rust library and passes startup/runtime failures. Stock Ubuntu runs it without .NET or network. |
| M8 — operations | README, systemd/tmpfiles examples, CI workflow, dependency audits, clean-clone demo, production-trust exclusions and separate client/server archives with CycloneDX inventory and SHA-256 checksums. |

The functioning local activation-to-worker slice was verified before the remaining AOT/release gates. No command is a permissive checker or an implementation placeholder.

## Environment and target

- Linux execution: WSL2 Ubuntu 24.04 x86_64, glibc 2.39; permission tests ran as uid 1000.
- Rust 1.97.1; .NET SDK 10.0.401 / runtime 10.0.12; clang 18.1.3; Python 3.12.
- Docker runtime image: Ubuntu 24.04, digest sha256:69cecf4bbf72d2d44a9eef1b71fb98c7fb973d78af11399deccef19beb008ad9.
- Validated initial target: Linux x86_64 glibc on Ubuntu 24.04. Docker is a compatibility test environment, not a supported licensing/binding deployment model.
- The AOT worker needs glibc/libm; the Rust native library also needs libgcc_s. No .NET installation is needed on the runtime image.

## Actual commands and results

Commands run in the Linux repository directory:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --locked --workspace` | PASS: 27 Rust tests |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked --workspace --all-features` | PASS: 26 Rust tests; production-only trust integration test is intentionally excluded under dev |
| `python3 scripts/validate_kit.py` | PASS: schemas/examples, header, migration and immutable contracts/fixtures/reference manifest hashes |
| `python3 scripts/verify_fixtures.py` | PASS: independent verification of 20 lease vectors, device proof and fingerprint vectors |
| `dotnet run --project samples/dotnet/NativeAotWorker.Tests -c Release -r linux-x64 -p:PublishAot=false -p:RestoreLockedMode=true` | PASS: 24 deterministic assertions, no sleeps |
| `bash scripts/demo-local.sh --extended` | PASS: normal managed process, two workers/one installation, exact response-loss retry, live renewal, denials, outage/expiry, invalid update, retire/remove |
| `bash scripts/test-ffi.sh` | PASS: C/C++ header use, ABI version, pointer/length checks, sizing, canaries, concurrent calls, native status 0 with denied license |
| `bash scripts/test-aot.sh` | PASS: locked Native AOT publish and the same real-process lifecycle/denial tests |
| `bash scripts/test-runtime-image.sh` | PASS: AOT with actual .so on stock Ubuntu, no .NET runtime or network; SIGTERM exit 0 |
| `bash scripts/test-production.sh` | PASS: production ignores dev overrides, rejects fixture/renamed fixture keys, public authorization readable by unprivileged service while private device key is unreadable |
| `cargo audit` | PASS: 222 crate dependencies checked against 1,247 loaded advisories; no known vulnerabilities reported |
| `dotnet list samples/dotnet/NativeAotWorker package --no-restore --vulnerable --include-transitive` | PASS: no vulnerable packages reported |
| `python3 scripts/test-clean-checkout.py` | PASS: source-only fresh local Git clone, locked restore, full default demo; no generated files or SDK directories copied |
| `bash scripts/test-packaging.sh` | PASS: complete check.sh suite, optimized production-feature build with temporary development public trust, separated archives and actual unprivileged packaged AOT worker using live synthetic Linux machine-id binding |

The release build commands include `cargo build --locked --release --workspace --no-default-features --target-dir target/release-build` and `bash samples/dotnet/publish-linux.sh linux-x64`. Managed builds completed with zero compiler/analyzer warnings and errors. The environment emitted a Cargo global-cache cleanup permission warning; it did not affect builds or checks.

Local logs: `artifacts/release-test.log` and `artifacts/clean-checkout.log` (ignored generated evidence). The kit validator's “NOT CHECKED” line describes only that script; compilation, live ABI and AOT are covered by the separate commands above.

## Security and regression evidence

- Exact signature input is preserved; a signed pretty-printed payload verifies, while reserializing it without resigning fails. Original cryptographic assets remain byte-for-byte unchanged.
- Parser tests include signed positional arrays, missing nullable fields, unknown nested fields, recursive duplicate keys, floats/oversized numbers, UTF-8, depth, size and base64 pad-bit errors.
- Server tests cover one-slot races, replay/mutation, expired challenges, revocation/expiry during operations, binding/key changes, manual-offline policy, cache cleanup, database busy and signing/commit rollback.
- Store tests cover symlinks/FIFOs/modes, locking, stable identity and interrupted publication, atomic failure points, complete concurrent reads, rollback/high-water recovery and preserving good authorization on rejection.
- Managed tests cover UTC rollback against monotonic deadlines, equal/higher/lower sequences, changed digest, exact expiry, admission/drain and a queued timer callback after disposal. Final review fixed that callback/disposal race and reran the release suite.
- Actual managed and AOT startup failures include missing/expired/tampered authorization, wrong product/feature, missing library, ABI mismatch, malformed native response and wrong architecture; each exits 78 without JOB_STARTED.
- Runtime renewal and expired/invalid replacement tests close admission and drain; SIGTERM remains exit 0.

## Packages and production trust

The verified local archives are `artifacts/DEV-TRUST-TEST-{client,server}-linux-x64-glibc.tar.gz`, with `DEV-TRUST-TEST-SHA256SUMS`. These are test artifacts, not production trust distributions. Their fresh temporary development private issuer key is removed when the harness exits.

Production builds default to empty, fail-closed trust. To build usable releases, generate the issuer key on the trusted server, transfer only its public trust registry to the builder, set `LICENSE_GUARD_TRUST_FILE` to its absolute path and run `bash scripts/build-release.sh`. Client archives exclude server/admin binaries, issuer/device private keys and fixture files. See README.md for commands and key rotation.

## Remaining staging checks and limitations

- Physical/VMware identifier behavior, copies/migrations/reinstalls and actual host CPU topology.
- Reboot-persistent DMI permissions and state access as the real service account. Missing required DMI fails closed; only the issuer can explicitly select linux-machine-v1.
- Actual systemd installation/start/stop/restart, real HTTPS reverse proxy/CA behavior, backup restoration and operational key rotation.
- GitHub-hosted CI is supplied but is not included in the local pass claims above; its current result is visible in the repository Actions page.
- ARM64, musl and other distributions/glibc baselines are unvalidated.

No production keys were invented and no production host was contacted or enrolled. Root can replace code/identifiers; exact VM snapshots can copy installation keys; offline wall-clock rollback across restarts is not solved. Activation slots are installation limits, not floating-seat enforcement.
