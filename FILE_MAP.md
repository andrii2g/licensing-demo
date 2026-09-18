# Target repository and file responsibilities

This table includes both files already supplied and files Codex must create. Existing samples/contracts are inputs to preserve and verify. Do not create empty files merely to satisfy the tree.

| Target | Responsibility |
|---|---|
| Cargo.toml, Cargo.lock, rust-toolchain.toml | Workspace, locked dependencies, pinned stable toolchain |
| crates/license-core/src/lib.rs | Public typed core API |
| crates/license-core/src/envelope.rs | Bounded strict JSON, canonical base64url, signing bytes |
| crates/license-core/src/claims.rs | Closed v1 claim/header/request types |
| crates/license-core/src/crypto.rs | Ed25519 strict verification/signing adapters |
| crates/license-core/src/policy.rs | Pure product/features/time/binding/capacity checks |
| crates/license-core/src/error.rs | Stable result/status mapping |
| crates/license-core/src/trust.rs | Pretrusted issuer public-key registry |
| crates/license-host/src/lib.rs | HostSnapshot and HostProvider |
| crates/license-host/src/linux.rs | Bounded proc/sysfs/OS collectors |
| crates/license-host/src/fingerprint.rs | Product-specific HMAC derivation |
| crates/license-store/src/lib.rs | Read-only store and installer operations |
| crates/license-store/src/secure_fs.rs | Safe paths, fd-based reads, ownership checks |
| crates/license-store/src/identity.rs | Durable device identity/key generation |
| crates/license-store/src/install.rs | Lock, fsync/rename, sequence recovery |
| crates/licensectl/src/main.rs | Command parsing and exit behavior |
| crates/licensectl/src/client.rs | Bounded authenticated HTTP |
| crates/licensectl/src/activation.rs | Challenge, request signing, exact retries |
| crates/licensectl/src/renewal.rs | Renew and retire workflow |
| crates/licensectl/src/config.rs | Strict protected config |
| crates/license-ffi/src/lib.rs | Exported C functions and only unsafe boundary |
| crates/license-ffi/src/validation.rs | Production host/store/core composition |
| crates/license-server/src/main.rs | API host configuration |
| crates/license-server/src/routes.rs | OpenAPI-matching HTTP routes |
| crates/license-server/src/service.rs | Domain authorization and issuance |
| crates/license-server/src/repository.rs | SQLite transactions and slot accounting |
| crates/license-server/src/auth.rs | Tokens, nonce scope, device proof, idempotency |
| crates/license-server/src/issuer.rs | Server-only key loading and signing |
| crates/license-server/src/limits.rs | Body/rate/concurrency limits |
| crates/license-server/migrations/001_initial.sql | Adapt supplied reference migration |
| crates/license-admin/src/main.rs | Trusted local administration, no public admin API |
| tests/core_vectors.rs | Fixed signed fixture outcomes |
| tests/server_flow.rs | Activation/renew/retire HTTP tests |
| tests/server_races.rs | Slot, retry and transaction races |
| tests/store_failures.rs | Filesystem failures/recovery |
| tests/ffi_client.c | Real C ABI caller and buffer guards |
| tests/aot-smoke/ | Published process orchestration and denial assertions |
| tests/host-fixtures/ | Bare-metal/VMware/permissions/topology cases |
| samples/dotnet/NativeAotWorker/*.cs | Included actual managed sample |
| samples/dotnet/NativeAotWorker/NativeAotWorker.csproj | Included AOT project |
| samples/dotnet/publish-linux.sh | Included Linux publish helper |
| samples/dotnet/NativeAotWorker.Tests/ | Add deterministic gate/lifecycle tests |
| contracts/* | Supplied JSON Schemas, OpenAPI, header and code registry |
| deploy/* | Supplied systemd/tmpfiles templates; adapt after verification |
| scripts/demo-local.sh | Implement isolated dev demo |
| scripts/build-release.sh | Build/package supported target, exclude dev trust |
| scripts/test-aot.sh | Compile Rust and run actual published worker integration |
| .github/workflows/ci.yml | Formatting/build/core/API/FFI/AOT gates |
| README.md | Replace kit intro with actual product usage after completion |
| IMPLEMENTATION_STATUS.md | Honest progress and test evidence |

Keep shared libraries independent of CLI/server hosting concerns. license-core may expose signing helpers, but client artifacts contain no issuer private key or administrative command.



## Implemented test locations

The implementation keeps focused Rust tests beside their modules so private fault hooks remain test-only:

| Planned responsibility | Actual location |
|---|---|
| Core vectors/strict parser | `crates/license-core/src/tests.rs` |
| Production fixture trust rejection | `crates/license-core/tests/production_trust.rs`, `scripts/test-production.sh` |
| HTTP flows, database races and rollback | `crates/license-server/src/tests.rs` |
| Host fixture adapters and permission behavior | `crates/license-host/src/linux.rs`, `fingerprint.rs` test modules |
| Store failures, identity recovery and concurrent reads | `crates/license-store/src/{secure_fs,identity,install}.rs` test modules |
| Managed deterministic gate/deadline/drain tests | `samples/dotnet/NativeAotWorker.Tests/Program.cs` |
| Actual managed/AOT process orchestration | `scripts/local_flow.py`, `scripts/test-aot.sh`, `scripts/test-runtime-image.sh` |
| Clean checkout and production-feature archive smoke | `scripts/test-clean-checkout.py`, `scripts/test-packaging.sh`, `scripts/package-smoke.py` |

`LicenseGuard.Managed` is the reusable managed wrapper extracted from the supplied sample. Normal builds and AOT builds have separate committed NuGet lockfiles selected by `samples/dotnet/Directory.Build.targets`. `IMPLEMENTATION_STATUS.md` records executed checks and unrun staging work.
