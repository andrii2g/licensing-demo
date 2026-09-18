# Repository implementation instructions

## Goal and precedence
Implement this project completely against CODEX_PROMPT.md, contracts/, and docs/. The user's later explicit decisions take priority. Resolve a discovered contract contradiction before coding around it; record changes in docs/00-decisions.md and update examples/tests together. Never silently weaken validation to make a demo pass.

## Working method
- Implement milestones sequentially; leave a concise progress record in IMPLEMENTATION_STATUS.md.
- Inspect existing files before editing. Preserve unrelated work.
- Do not deploy, contact a real licensing service, enroll a real host, or commit secrets while building.
- Use synthetic inventory and local test servers. Generate production-like development secrets at runtime into ignored directories with restrictive permissions.
- Do not generate dozens of stubs and claim completion. Finish one end-to-end slice, then extend it.
- Keep transport, cryptography, host collection, filesystem operations, and policy evaluation separate.
- Do not add delegation requirements. Work locally unless the user chooses otherwise.

## Rust
Use a Cargo workspace, Rust 2024 edition, stable toolchain pinned at implementation time, rustfmt and clippy. Select compatible current stable dependencies, commit Cargo.lock, and record versions/MSRV. Do not assume a documentation site's latest examples match an older dependency major.
Use maintained Ed25519, SHA-256, HMAC, TLS, JSON and HTTP crates. Do not implement cryptographic primitives.
Forbid unsafe code in core, policy and API modules. Isolate documented unsafe pointer handling in the ABI module.
Use owned typed data after parsing; enforce size limits before allocation. Reject duplicate JSON keys recursively, unknown fields and unsupported versions in v1 contracts.
No unbounded HTTP bodies, shell parsing of machine details, embedded private keys, secret-bearing Debug output, or silent fallback policies.
Map errors to stable public codes; internal detail belongs in sanitized logs.

## C# and worker lifecycle
Use .NET 10 and source-generated LibraryImport and System.Text.Json metadata. Avoid runtime reflection-dependent serializers.
Startup validation must precede host.StartAsync/RunAsync. Constructors and DI registration must not connect consumers or start work.
Every job admission passes a shared authorization gate. Invalid runtime state closes admission, marks readiness false, drains admitted work within 30 seconds, and stops with exit 78.
Keep process exit codes distinct from native validation status codes.
Tests must prove no work starts during failed startup; a registration-order-only check is insufficient.

## Security invariants
Only issuer keys sign leases. Device keys sign requests. These key types and signing domains are not interchangeable.
Never trust an unsigned entitlement, local override, caller-provided inventory, or caller-supplied clock in the production FFI.
The installer may use a dev server only in a dev build; production enforces HTTPS and normal certificate validation.
Production artifacts reject all fixture keys, even if a config attempts to trust them.
The API owns entitlement mode, binding policy, activation count, capacity limit, lease duration and expiry.
A valid MAC/hostname report is not proof of hardware authenticity.
No arbitrary command execution, dynamic plugin loading, or unsafe deserialization is needed.

## Tests and completion
Use pure test clocks/host providers internally; do not expose them through production ABI or environment overrides.
Run meaningful tests from docs/09-testing.md. Use no sleeps for simulated expiry.
Generate release artifacts only after security, host lifecycle, actual C# interop and AOT tests pass.
State which checks could not run. Do not describe an untested glibc/architecture target as supported.
