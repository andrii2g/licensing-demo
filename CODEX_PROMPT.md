# Paste this prompt into Codex

Implement **License Guard** from this kit as a working repository.

First read AGENTS.md, docs/00-decisions.md, FILE_MAP.md, IMPLEMENTATION_PLAN.md, and ACCEPTANCE_CRITERIA.md. Treat contracts/ and the numbered docs as the normative implementation contract. The kit contains specifications and reference assets; the Rust applications still need implementation; the included C# worker source needs compilation, integration and verification.

Build:
1. A shared Rust verifier with strict signed-envelope parsing, Ed25519 verification, time/product/feature/binding/capacity rules, and typed results.
2. A Linux inventory and binding collector.
3. A Rust licensectl CLI for inspect, activate, renew, status, verify, and local removal, plus explicit server retirement.
4. A Rust licensing API using Axum and SQLite, transactional activation limits, replay protection, idempotency, signed leases, and protected administrative CLI commands.
5. A small stable C ABI exported as liblicense_guard.so, with contracts/license_guard.h as its ABI contract.
6. A .NET 10 wrapper, source-generated JSON serialization, sample worker, startup authorization gate, runtime watchdog and graceful draining.
7. Linux packaging/systemd examples, deterministic tests, a local demo, and a README that someone can follow from a clean checkout.

Use the selected defaults: renewable seven-day leases, daily renewal, 60-second runtime validation, 30-second drain timeout, direct Linux deployment on physical hosts or VMware. Server entitlements can instead choose finite manual_offline mode. Linux x86_64 glibc is the required initial release target; architecture extensions are separate tested deliverables. Online licensing is per installation, not per worker process, and an activation-slot limit does not imply floating-seat enforcement.

Keep runtime dependencies modest but use maintained libraries for security-sensitive primitives. Pin compatible versions and lock dependencies after confirming APIs. The reference server is Rust for an independently runnable repository; keep its wire protocol usable by another server implementation.

Do not ask for routine implementation choices already settled in the kit. Record reasonable local choices. Ask only if a real environment dependency or contradictory business requirement prevents a correct implementation. Never contact production hosts or enroll this workspace. Do not invent production keys or claim resistance to root/VM snapshot cloning.

Work milestone by milestone. Maintain IMPLEMENTATION_STATUS.md with completed work, checks run and remaining issues. Include actual build commands and test evidence. First deliver a functioning local activation-to-worker slice, then complete all remaining acceptance gates. Keep fixtures deterministic. Preserve the supplied cryptographic signing bytes; do not reserialize data before verifying signatures.

When finished, report:
- implemented components and commands;
- test results including FFI, worker gating and Native AOT;
- supported deployment targets;
- limitations and any unrun checks;
- how to run the local demo and replace dev trust with production trust.

Do not stop at a plan or present scaffold files as completed implementation.
