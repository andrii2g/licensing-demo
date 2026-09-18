# License Guard

Installation-scoped licensing for Linux services: a Rust verifier and installer, an Axum/SQLite licensing API, a stable C ABI, and a .NET 10 worker that fails closed before starting work.

The local activation → signed lease → native verification → managed/Native AOT worker path is implemented. The default policy is a seven-day lease, daily renewal, validation at least every 60 seconds, and a 30-second drain timeout. Entitlements can select finite `manual_offline` authorization instead. Several worker processes share one installation; activation slots are not floating seats.

## Run from a clean checkout

The validated build environment is **Ubuntu 24.04, Linux x86_64, glibc 2.39**, with Rust **1.97.1**, .NET SDK **10.0.401**, clang, a C compiler, zlib development headers, and Python 3.12+. Docker is used only for isolated runtime tests, not as the licensing deployment model.

Install the SDK using [Microsoft's .NET 10 instructions](https://learn.microsoft.com/en-us/dotnet/core/install/linux-ubuntu-install). On Ubuntu, native prerequisites are:

```bash
sudo apt-get update
sudo apt-get install -y build-essential clang zlib1g-dev python3-cryptography
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
git clone https://github.com/andrii2g/licensing-demo.git
cd licensing-demo
bash scripts/demo-local.sh
```

The demo builds explicit **development artifacts** under `target/dev`, generates fresh keys in a mode-0700 temporary directory, starts a loopback server, activates synthetic VMware inventory, runs two workers, renews, tests denials, retires the installation, and removes local authorization. It deliberately drops one committed activation response to verify exact retry behavior. It neither contacts a production licensing endpoint nor enrolls your host. `--keep` preserves the temporary development state for inspection; it contains private development keys. `--real-host` is a separate explicit staging opt-in.

```bash
bash scripts/demo-local.sh --extended  # live renewal, outage, expiry/drain, invalid update
bash scripts/test-aot.sh              # publish and run the real Native AOT executable
bash scripts/test-runtime-image.sh    # AOT on stock Ubuntu without .NET or network
bash scripts/test-production.sh       # production rejects fixture/dev trust overrides
cargo install cargo-audit --version 0.22.2 --locked
bash scripts/check.sh                 # complete automated gate suite
python3 scripts/test-clean-checkout.py # source-only local Git clone/demo
bash scripts/test-packaging.sh        # full gates + DEV-TRUST-TEST archive smoke
```

Unit tests use fixed clocks and fixtures. Process tests use bounded waits for observed events. See [IMPLEMENTATION_STATUS.md](IMPLEMENTATION_STATUS.md) for actual evidence and staging limitations; [VALIDATION.md](VALIDATION.md) is the original kit report.

## Components and commands

| Component | Implemented responsibility |
|---|---|
| `license-core` | Strict bounded JSON/base64url, exact-byte Ed25519 verification, typed claims/results, time/product/features/identity/binding/capacity |
| `license-host` | Direct Linux collection, product-specific HMAC fingerprints, online CPU count distinct from affinity |
| `license-store` | Descriptor-relative no-follow reads, ownership/mode checks, locking, recoverable identity and atomic lease/high-water writes |
| `licensectl` | `inspect`, `activate`, `renew`, `status`, `verify`, explicit `retire`, local `remove` |
| `license-server` | Scoped challenges, device proof, authenticated exact retries, SQLite transactions, slot reservations, signed leases, bounded HTTP/rate/concurrency handling |
| `license-admin` | Server-only key generation, entitlement create/revoke, installation retire/list |
| `liblicense_guard.so` | `lg_abi_version`, `lg_validate_v1` from [the supplied C header](contracts/license_guard.h) |
| `LicenseGuard.Managed` | Reusable .NET 10 source-generated P/Invoke and JSON wrapper |
| `NativeAotWorker` | Startup gate, per-job admission, monotonic deadline, watchdog, readiness logs, bounded drain |

Examples for an installed client:

```bash
licensectl inspect --config /etc/license-guard/client.toml --json
licensectl activate --config /etc/license-guard/client.toml  # hidden token prompt
licensectl status --config /etc/license-guard/client.toml --json
licensectl verify --config /etc/license-guard/client.toml --file /protected/candidate.lic
licensectl renew --config /etc/license-guard/client.toml
licensectl renew --scheduled --config /etc/license-guard/client.toml
licensectl retire --config /etc/license-guard/client.toml
licensectl remove --config /etc/license-guard/client.toml
```

For automation, `activate --token-stdin` accepts one bounded token line; no token command-line argument is supported. Mutations hold an exclusive lock. Retrying an interrupted mutation reuses its durable operation and identity. Resume a pending command before attempting a different online mutation. A failed renewal preserves the installed authorization. `remove` preserves identity/high-water metadata and does not free a server slot. A corrupted local lease may be explicitly removed before renewing; do not delete the identity or high-water record to bypass recovery checks.

Exit codes: 0 success, 64 usage/configuration, 70 internal failure, 75 retryable resource/network failure, 78 licensing denial. Native return 0 only means a complete JSON result: callers must also require `valid=true`.

## Production trust and packages

Production builds have **empty, fail-closed trust by default**. No production private key is supplied. Development HTTP, trust-file and host-fixture environment overrides are compiled out of production client/native artifacts. Known fixture issuer and device keys are rejected even when renamed.

Build the server administration binary with `cargo build --locked --release -p license-admin` and install it on the trusted server. This bootstrap build needs no client trust registry. Generate and protect your issuer key **on that server**, in a directory owned by the dedicated server account:

```bash
license-admin keygen --directory /var/lib/license-guard-server/keys --kid issuer-2026-01
```

This writes `issuer.key` (0600, server only) and `trust.json` (public key registry). The registry format is an array of `[kid, canonical_base64url_public_key]` pairs. Transfer only the public registry to the release builder, then:

```bash
export LICENSE_GUARD_TRUST_FILE=/absolute/protected/public-trust.json
bash scripts/build-release.sh
```

The script runs security, lifecycle, C ABI, managed, and AOT gates before creating separate client and server archives, a CycloneDX component inventory, and SHA-256 checksums under `artifacts/`. `test-packaging.sh` exercises this path with a temporary development issuer and labels its archives `DEV-TRUST-TEST`; those archives are test evidence only. The client archive excludes the server/admin executables and all private issuer/device material. Release binaries embed the public registry; editing client configuration cannot grant trust. Build on the oldest distribution you intend to support and test that target. Only the stated Ubuntu x64/glibc baseline is validated here.

Configure the API using `examples/server.toml`. The API binds loopback behind an administrator-managed HTTPS reverse proxy, uses its own protected SQLite directory, and ignores forwarded-address headers. IP limits therefore apply to the socket peer, including a shared proxy. Run administrative commands as the server account against the same protected configuration/database:

```bash
license-admin --config /etc/license-guard-server/server.toml entitlement create --input /protected/entitlement.json
license-admin --config /etc/license-guard-server/server.toml entitlement revoke --id LIC-2026-001234
license-admin --config /etc/license-guard-server/server.toml installations list --license-id LIC-2026-001234
license-admin --config /etc/license-guard-server/server.toml installation retire --id INSTALLATION_UUID
license-server --config /etc/license-guard-server/server.toml
```

The create command prints a random activation token once; only its SHA-256 is stored. Edit the entitlement example before creation: product, features, expiry, binding, slot/capacity limits and mode are server-owned policy. The wire contract remains in `contracts/openapi.yaml` and `contracts/*.schema.json`; another server can implement it.

For rotation, distribute a registry with old and new public keys first, then configure the server to sign with the new key. Retain old verification keys until old leases have been migrated or expired.

## Service integration and deployment

Reference `samples/dotnet/LicenseGuard.Managed/LicenseGuard.Managed.csproj`, construct `NativeLicenseChecker` from trusted `LicenseOptions`, and validate **before** `Host.StartAsync/RunAsync`. Registering a hosted checker is insufficient. Use the sample gate around every job admission; constructors must remain inert.

The sample loads the configured absolute native-library path and verifies ABI version 1. Missing/wrong-architecture libraries, bad ABI, malformed responses and licensing denials exit 78 before `JOB_STARTED`. The worker uses no network licensing calls. At runtime, failed validation closes admission, logs readiness false, drains admitted work for up to 30 seconds and exits 78. SIGTERM drains and exits 0. A new valid lease can extend the deadline only with a higher sequence; equal-sequence validation cannot extend a clock rollback.

Review [deploy/README.md](deploy/README.md) and [Linux identity requirements](docs/03-machine-identity.md) before installing the systemd/tmpfiles examples. Workers remain unprivileged; root owns client configuration, identity, authorization and native library. Device keys and pending operations are updater-only. `linux-host-v1` requires live DMI UUID access: provision and verify a narrow distribution-specific permission grant, including after reboot. Where that is unavailable, the issuer must explicitly select `linux-machine-v1`; there is no fallback.

The Native AOT worker requires no .NET runtime. On the tested image its native dependencies are glibc/libm; the Rust library also needs libgcc_s. Physical-host/VMware behavior, reboot persistence, real TLS/reverse-proxy deployment and real systemd installation remain operator staging checks. ARM64, musl, other glibc baselines, Kubernetes binding, and multi-node databases are not validated targets.

Root administrators can replace code and identifiers. Exact VM snapshots can copy an installation and device key. Offline authorization relies on local UTC; a monotonic deadline helps within a running process but does not prevent restart/snapshot rollback. Inventory is self-reported, not hardware attestation, and already-issued offline leases cannot be revoked instantly.
