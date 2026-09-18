# Validation and test coverage

## Run locally

Use the Linux prerequisites in the [README](../README.md) and run these commands from the repository root. Recorded results and remaining deployment checks are listed below.

```bash
python3 scripts/validate_assets.py
python3 scripts/verify_fixtures.py
cargo test --locked --workspace
cargo test --locked --workspace --all-features
dotnet run --project samples/dotnet/NativeAotWorker.Tests -c Release -r linux-x64 -p:PublishAot=false -p:RestoreLockedMode=true
bash scripts/check.sh
```

The first command checks schemas/examples, C header compilation, managed project XML, shell/Python syntax, the deployed SQLite migration and required integrity hashes. The second independently verifies the signed fixtures using Python cryptography. `check.sh` runs formatting, clippy, Rust/managed tests, the extended local demo, C ABI and actual AOT lifecycle checks, isolated runtime/production tests, and dependency audits. Docker and cargo-audit are required for the complete suite.

For separate release gates, run `python3 scripts/test-clean-checkout.py` and `bash scripts/test-packaging.sh`. The first creates a temporary local Git repository and source-only clone to test the default demo; it does not alter the workspace's Git history. The second runs the complete suite and produces explicitly labeled development-trust test archives. Production packaging uses `bash scripts/build-release.sh` with the approved public `LICENSE_GUARD_TRUST_FILE`.

## Test locations

| Area | Source or harness |
|---|---|
| Strict parsing, policy and signed vectors | [core tests](../crates/license-core/src/tests.rs) |
| Production trust rejection | [production trust test](../crates/license-core/tests/production_trust.rs), [production harness](../scripts/test-production.sh) |
| HTTP, replay, transaction races and rollback | [server tests](../crates/license-server/src/tests.rs) |
| Linux collection and fingerprint adapters | [Linux collector](../crates/license-host/src/linux.rs), [fingerprints](../crates/license-host/src/fingerprint.rs) |
| Protected filesystem, identity and installation failures | [store modules](../crates/license-store/src/) |
| Deterministic managed admission and draining | [managed harness](../samples/dotnet/NativeAotWorker.Tests/Program.cs) |
| Actual process activation, renewal and shutdown | [local flow](../scripts/local_flow.py), [AOT harness](../scripts/test-aot.sh) |
| Native header, buffers and concurrent calls | [FFI harness](../scripts/test-ffi.sh), [C client](../tests/ffi_client.c) |
| Runtime without .NET and release packages | [runtime image](../scripts/test-runtime-image.sh), [packaging harness](../scripts/test-packaging.sh) |

## Asset integrity

[tests/asset-integrity.json](../tests/asset-integrity.json) records byte lengths and SHA-256 hashes for all wire contracts, deterministic JSON/lease fixtures and the [deployed migration](../crates/license-server/migrations/001_initial.sql). The validator requires the manifest, exact coverage and unique entries; a missing file or changed byte fails validation. Documentation can change without regenerating cryptographic fixtures.

Signed envelopes are verified using their supplied signing bytes. Regenerating fixtures or changing the integrity baseline requires deliberate contract review, not an automatic test-repair step. See the [fixture guide](../fixtures/README.md).

## Deterministic core
Inject test Clock, HostProvider and trust into internal core constructors only. Fixed fixtures use UTC now=1800144000 (2027-01-17T00:00:00Z); read fixtures/manifest.json for exact values. Tests must use fixture-defined time, not the current calendar date.
Cover valid, exact not_before, one second early, one second before expiry, exact expiry, after expiry, future issued_at, malformed chronology, wrong issuer/product/feature, installation/key mismatch, machine/DMI mismatch, capacity exceeded and unknown required data.
Verify known Ed25519 vectors and supplied signed fixtures independently. Mutate protected/payload/signature bytes; malformed and validly signed but unsupported payloads must both fail.
Reject duplicate keys recursively, unknown fields, bad base64url pad bits, enormous numbers, floats for times, excessive depth, invalid UTF-8 and oversized files.
Check mode cannot be changed without signature failure. Test key rejection in a production-trust build.

## Linux collector
Use fixture filesystem adapters: bare metal, VMware, multiple NICs, locally administered MACs, loopback, missing DMI, zero/ff UUID, uninitialized machine ID, CPU offline ranges, repeated core IDs across sockets and restricted affinity.
Expected online logical count differs from available count in the affinity case.
No real filesystem override is exported in production FFI. Feature-gated dev CLI fixture input must not appear in release help.

## API and SQLite
Run HTTP integration tests against temporary DB and per-test generated signing keys.
Concurrency: with one slot and two new installation IDs racing, exactly one succeeds. With one identity and lost response/retries, exactly one installation exists.
Replay: exact cached request succeeds only through authenticated idempotency; changed payload/same operation ID fails; consumed nonce/new operation fails; wrong action/product/route/key fails.
Crash/fault injection: signing failure, transaction rollback, response loss after commit, DB busy, expired challenge, token revoked during challenge, entitlement expiry during mutation.
Renewal increments sequence once per operation and preserves original binding; offline mode needs no scheduler contact.
Retired slot remains reserved until last lease expiry. At the boundary it can be reused. Retired device renewal fails.
Verify response cache and nonce cleanup do not delete audit/installation records or permit extra issuance.

## Filesystem
Inject failure before temp fsync, before rename, after rename and before high-water state update. Prior authorization remains readable or recovery chooses the new valid higher sequence; never produce a partial license.
Test concurrent installers, read during renewal, symlink replacement, world-writable files, oversized FIFO/nonregular input, identity/key mismatch and insufficient permission.
No successful renewal removes existing valid authorization on HTTP failure.

## Native and managed integration
Header compiles with C and C++. Native buffer canaries remain intact. ABI validation failure return 0 must not authorize.
Build normal managed and Native AOT sample. Start real processes; assert exit codes and job logs. Exercise actual Rust library, not a managed fake.
AOT smoke: no .NET runtime on clean VM/container running published executable; correct libc and native OS dependencies still required.
Use a fixture-trust dev distribution for tests and a separate production-trust negative check.

## Lifecycle
The gate and drain controller use an internal TimeProvider. Production uses TimeProvider.System; deterministic tests control UTC, monotonic timestamps and timers without changing the production ABI.
No fixed-duration sleeps for unit tests. Process tests use bounded wait for readiness/job lines and explicit timeouts.
SIGTERM closes the gate before hosted-service cancellation; that race must still exit 0 rather than being classified as license expiry.
Expiry blocks admission immediately at cached deadline; poll revalidation happens within 60 seconds; bounded drain within 30 seconds after shutdown begins.
Renewal equal sequence must not reset countdown after clock rollback; higher sequence may renew. Invalid license update remains terminal for that process even if a later file appears.

## Operational tests
Real physical/VMware identity behavior needs staging hardware; mark unrun if unavailable. Test reboot preserves expected identity and DMI read permission as the actual service user.
Simulate server downtime longer/shorter than lease duration without modifying host wall time where avoidable.
Dependency audit, clippy, source-gen warnings, release test-key scanning and reproducible lockfiles form release gates.

## Recorded validation

The complete local suite passed on 2026-09-18 using WSL2 Ubuntu 24.04 x86_64, glibc 2.39, Rust 1.97.1, .NET SDK 10.0.401 / runtime 10.0.12, clang 18.1.3 and Python 3.12. Permission checks ran as uid 1000; runtime/package tests also exercised unprivileged workers. This validates the Ubuntu x64/glibc baseline, not every Linux distribution or physical/VMware deployment.

| Command | Recorded result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` and the same command with `--all-features` | Passed |
| `cargo test --locked --workspace` | 27 tests passed |
| `cargo test --locked --workspace --all-features` | 26 tests passed; the production-only trust test is excluded under dev |
| `python3 scripts/validate_assets.py` | 47 JSON/lease files, 14 schemas, C header, project XML, script syntax, deployed migration and 45 integrity hashes passed |
| `python3 scripts/verify_fixtures.py` | 20 lease vectors, device request signature and fingerprint vectors passed |
| Managed test command shown above | 24 deterministic assertions passed; zero compiler/analyzer warnings or errors |
| `bash scripts/demo-local.sh --extended` | Actual managed processes: two workers sharing one installation, exact response-loss retry, renewal, startup denials, outage/expiry, invalid update, retire/remove |
| `bash scripts/test-ffi.sh` | C/C++ header, ABI, pointer/length checks, sizing, canaries, concurrency and native status 0 with denied authorization passed |
| `bash scripts/test-aot.sh` | Locked Native AOT publish and actual startup/runtime lifecycle tests passed |
| `bash scripts/test-runtime-image.sh` | Actual AOT worker and Rust library ran on stock Ubuntu without .NET or network; SIGTERM returned 0 |
| `bash scripts/test-production.sh` | Dev overrides ignored, fixture keys rejected even when renamed, public authorization readable and private device key unreadable by the worker |
| `cargo audit` | 222 dependencies checked against 1,247 loaded advisories; no known vulnerabilities reported at the time |
| `dotnet list samples/dotnet/NativeAotWorker package --no-restore --vulnerable --include-transitive` | No vulnerable packages reported at the time |
| `python3 scripts/test-clean-checkout.py` | Source-only local Git clone, locked restore and full default demo passed |
| `bash scripts/test-packaging.sh` | Complete suite, optimized production-feature build with temporary development public trust, separate archives and actual unprivileged packaged AOT worker passed |

Actual startup denials included missing, expired or tampered authorization; wrong product/feature; missing or wrong-architecture library; ABI mismatch; and malformed native response. Each exited 78 without starting a job. Runtime tests covered higher-sequence renewal, invalid replacement, expiry/drain and SIGTERM exit 0. The packaged worker used live synthetic Linux machine-id binding.

Release commands were `cargo build --locked --release --workspace --no-default-features --target-dir target/release-build` and `bash samples/dotnet/publish-linux.sh linux-x64`. The runtime image was Ubuntu 24.04 with digest `sha256:69cecf4bbf72d2d44a9eef1b71fb98c7fb973d78af11399deccef19beb008ad9`. A Cargo global-cache cleanup permission warning did not affect builds or checks.

Local generated evidence is in ignored `artifacts/release-test.log` and `artifacts/clean-checkout.log`. Verified archives are named `DEV-TRUST-TEST-{client,server}-linux-x64-glibc.tar.gz`, with `DEV-TRUST-TEST-SHA256SUMS`; these use development trust and are not production distributions. The harness removes its temporary private issuer key.

Subsequent documentation cleanup left application code and cryptographic bytes unchanged. Asset and independent signature checks passed again; five temporary-copy checks rejected changed signed bytes, omitted/duplicate entries, an unlisted protected asset and a missing integrity manifest. The full Rust/managed/AOT suite was not rerun for documentation-only changes.

## Deployment checks still required

- Physical/VMware identifiers, copies, migrations, reinstalls and actual host CPU topology.
- Reboot-persistent DMI permission and state access as the real service account. Missing required DMI fails closed; only the issuer can select linux-machine-v1.
- Actual systemd installation/start/stop/restart, HTTPS reverse proxy/CA behavior, backup restoration and operator key rotation.
- Separate builds and tests for ARM64, musl and other distribution/glibc baselines.

No production host was contacted or enrolled during validation. Docker provides an isolated runtime test environment, not a supported licensing deployment model. Root resistance, exact VM snapshot detection and secure offline time across restarts remain outside the software-only threat model.
