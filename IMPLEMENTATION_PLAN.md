# Ordered implementation plan

Each milestone ends with runnable evidence. Keep IMPLEMENTATION_STATUS.md updated. The full production-quality scope spans multiple sessions; do not compress it into an unreviewed single-file demo.

## M0 — Repository and trust boundaries
Create the Cargo workspace described in FILE_MAP.md; select/pin a stable Rust toolchain and compatible maintained crates. Inspect and compile the included .NET sample, update supported .NET 10 servicing versions, lock restore.
Copy reference migration into server migrations; preserve schemas and header.
Gate: cargo check and dotnet build succeed with honest temporary "not implemented" commands returning errors, no permissive checker.

## M1 — Core verification
Implement strict envelope/header/payload parser, maintained Ed25519 verification, bounded decoding, typed errors, pure policy evaluation and fixed fixture tests.
Gate: every fixture expected outcome passes; wrong signatures/types/duplicate keys fail; exact expiry denies. Production trust rejects fixtures.

## M2 — Host collector and store
Implement direct Linux facts, normalization/HMAC vectors, missing-data semantics, installation key creation, public identity, secure reads and atomic writes.
Gate: bare-metal/VMware simulated filesystem cases, affinity vs online CPUs, symlink/permission/crash cases pass. No raw machine ID in reports.
Prove unprivileged DMI behavior in tests and make unsupported permission configurations explicit.

## M3 — Reference server vertical slice
Implement SQLite repositories, local admin entitlement creation, activation credentials, nonce challenge, signed activation, transactional slot limit, response idempotency and issuer signing.
Gate: local activation returns a license that core verifies. Parallel one-slot test permits exactly one new installation.
Local demo uses fresh development keys and an explicitly dev-trust package.

## M4 — Online CLI
Implement inspect/activate/status/verify and durable pending operations. HTTPS production policy, loopback dev mode, bounded transport and exact retries.
Gate: intentional lost response still leaves one installation and a usable local license; failed response never replaces a good license.

## M5 — C ABI and real managed integration
Implement header exactly; add production native trust generation. Check source-generated LibraryImport with included sample; fix any compile/analyzer issues without weakening contracts.
Gate: C client and normal .NET process call actual .so. Invalid license prevents JOB_STARTED. Native return=0 with valid=false denies.

## M6 — Renewal and runtime lifecycle
Implement renew/retire API+CLI, sequence checks, retained slot reservations, timer behavior, clock/deadline handling and graceful shutdown.
Gate: higher sequence renews live worker; same sequence cannot extend a rollback clock; expiry stops admission and drains; offline policy runs without contact.

## M7 — Native AOT sample/release
Publish included sample for linux-x64; package matching Rust .so; implement integration process tests and local demo automation.
Gate: published executable runs on a supported Linux image with no .NET runtime, accepts valid activation and rejects missing/expired/tampered data. Verify exit 78 and SIGTERM exit 0.

## M8 — Operational/release completion
Finalize docs, systemd templates, dependency audit, tests, package separation and production trust exclusions.
Gate: clean-checkout demo, all acceptance criteria, tested supported distro/libc, no fixture keys in production trust, documented unrun VMware/hardware tests if no environment exists.

## Suggested dependency roles
serde/serde_json with strict visitor handling; ed25519-dalek; sha2 + hmac; base64; OS randomness and uuid; zeroize where appropriate; clap; reqwest with rustls; axum/tokio; rusqlite; tracing.
Do not assume serde Value parsing rejects duplicate fields; implement the recursive duplicate check explicitly.
Use std/system APIs or narrowly scoped maintained Unix wrappers for secure file operations and CPU affinity.
No hand-written TLS, Ed25519, JWT algorithm negotiation, dependency on a cloud SDK, or hardware driver is required.

## Final demo command target
Implement scripts/demo-local.sh to create temp state and dev keys, start loopback API, create one entitlement, activate, publish/start worker, renew and demonstrate tamper denial. Its default must use fixture host data in isolated DEV artifacts, not enroll the real machine. A separate explicit --real-host mode is for operator-owned staging tests.
Production artifacts must have neither dev HTTP nor fixture host overrides.
