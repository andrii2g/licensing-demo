# Definition of done

## Required functional evidence
- [x] Rust CLI collects requested MAC/hostname/CPU inventory on Linux physical/VMware environments or records staging limitations honestly.
- [x] Raw machine ID is never sent; derivation exactly matches supplied vectors.
- [x] Online activation uses token authorization plus device proof and returns a signed installation-bound file.
- [x] Worker runs multiple processes under one host installation without pretending each is a seat.
- [x] .NET startup gate executes before any hosted worker performs work.
- [x] Included .NET 10 sample publishes Native AOT and runs using the actual Rust library.
- [x] No .NET runtime is required on the AOT test target; native OS/library dependencies documented.
- [x] Native call status and license validity are correctly distinguished.
- [x] Renewable/offline policies are issuer-controlled; no local extension or unsigned grace.
- [x] Daily renewal and atomic replacement work while workers are running.
- [x] Failure/expiry closes admission, drains active jobs and returns 78; SIGTERM returns 0.
- [x] Library missing/ABI mismatch/wrong architecture fails closed.
- [x] Server outage does not stop a still-valid lease; expired lease does not keep authorizing.

## Security and correctness
- [x] Test issuer keys are impossible to trust in production builds.
- [x] Wrong issuer/product/feature/identity, tamper, duplicate keys, unknown fields, limits and chronology tested.
- [x] Binding cannot silently degrade when DMI is unavailable.
- [x] Normal hostname/MAC changes do not invalidate binding.
- [x] CPU limit uses total online guest/host logical CPUs, not service affinity count.
- [x] Challenges are scoped, one-time, server-expiring and consumed atomically.
- [x] Exact operation retries are idempotent; request mutation fails.
- [x] Slot concurrency and reserved-until retirement behavior tested.
- [x] Server keys remain on server; client private device key unreadable by worker.
- [x] Secure local reads/writes and crash recovery tested.
- [x] No claims of guaranteed clone detection, root resistance or secure offline wall time.

## Quality and packaging
- [x] Cargo fmt/clippy/tests; .NET build/analyzers/tests; real AOT integration all pass.
- [x] Version/toolchain and dependency lockfiles committed.
- [x] Clean-checkout local demo uses isolated dev trust and temp state.
- [x] Client package excludes server/admin and issuer secret material.
- [x] Tested Linux architecture/libc/distro stated; no untested portability claims.
- [x] Tests, command output and remaining limitations in IMPLEMENTATION_STATUS.md.
- [x] README clearly distinguishes product-ready functionality from future work.



## Evidence and deployment staging

The checked implementation gates are backed by actual commands and results in [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md). The physical/VMware criterion above is satisfied through its explicit staging-limitations alternative; real hardware is not claimed as tested.

- Rust: 27 default-build tests, 26 all-feature tests, formatting/clippy, all 20 independent signed lease vectors.
- Managed: 24 deterministic lifecycle assertions; real normal and Native AOT startup/runtime processes, C ABI tests and unprivileged Ubuntu images without .NET/network.
- Operations: clean local Git clone demo, production fixture-key exclusions, private-device-key permission denial, dependency audits, separated release archives and actual packaged production-feature worker.

Remaining environment-specific deployment checks:

- [ ] Physical/VMware deployment, copies/migrations and reboot-persistent DMI access as the real worker account.
- [ ] Actual systemd service installation and restart behavior.
- [ ] Production reverse-proxy TLS/CA, backup restoration and operator key rotation.

Other architecture/libc targets remain separate unvalidated deliverables. GitHub-hosted CI results are visible in Actions and are separate from the completed local evidence.
