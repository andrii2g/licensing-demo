# Architecture and repository layout

## Components

| Location | Responsibility |
|---|---|
| [license-core](../crates/license-core/src/) | Envelope/claim parsing, exact-byte signatures, issuer trust, typed results and pure policy evaluation |
| [license-host](../crates/license-host/src/) | Direct Linux inventory, product-specific fingerprints and live host collection |
| [license-store](../crates/license-store/src/) | Protected paths, locking, recoverable device identity, atomic license replacement and sequence history |
| [licensectl](../crates/licensectl/src/) | Local inspection/verification and device-authenticated activation, renewal and retirement |
| [license-server](../crates/license-server/src/) | Axum routes, bounded HTTP work, SQLite transactions, challenges, slot accounting, response caching and issuance |
| [license-admin](../crates/license-admin/src/) | Trusted-server key generation, entitlement administration and installation support |
| [license-ffi](../crates/license-ffi/src/) | Stable C exports and production composition of core, live host and read-only storage |
| [LicenseGuard.Managed](../samples/dotnet/LicenseGuard.Managed/) | Source-generated P/Invoke/JSON and typed native results |
| [NativeAotWorker](../samples/dotnet/NativeAotWorker/) | Startup authorization, per-job admission, runtime watchdog and bounded draining |

The public wire schemas, OpenAPI document, result codes and C header are in [contracts/](../contracts/). The API initializes SQLite from [its deployed migration](../crates/license-server/migrations/001_initial.sql); there is one maintained migration copy.

[examples/](../examples/) contains configuration and synthetic protocol examples. [deploy/](../deploy/) contains direct Linux service/permission guidance. [scripts/](../scripts/) contains runnable local checks, demos and release packaging. Test locations and commands are listed in [the validation guide](09-testing.md).

## Data flow

1. An administrator creates an entitlement and high-entropy activation token on the trusted server.
2. The installer durably creates a device identity, obtains a scoped challenge and sends device-signed inventory with token authorization.
3. The server approves the request transactionally and signs a lease. The installer verifies it against its own trust and live host before atomic installation.
4. The worker verifies the local lease through the native library before starting services. Each job admission checks the shared gate and its cached expiry deadline; the watchdog revalidates through the native library.
5. A separate daily timer renews renewable leases. Workers revalidate local state; an outage does not invalidate a still-valid lease.
6. Explicit retirement denies future renewal and retains the slot until the last issued authorization expires.

## Implemented interfaces and boundaries

`license_core::verify` takes envelope bytes, `Trust` and a `Context` containing time, required product/features, public identity and collected host facts. It returns a `VerifiedLease` or stable `Code`; `ValidationResult` is the public diagnostic result. Policy evaluation performs no filesystem or network access.

`license_host::HostProvider` returns a `HostSnapshot`; `LiveHost` uses bounded Linux reads and system APIs without shell commands. Internal fixture adapters support deterministic tests. Production FFI always uses live collection.

`license_store::SecureDir` implements descriptor-relative protected reads and writes. Device identity uses a recoverable private journal. Lease installation verifies before replacement and preserves a persistent sequence/digest high-water record.

The API runs bounded blocking database work outside Tokio executor threads and uses SQLite `BEGIN IMMEDIATE` for mutations. Its in-process `Issuer` alone holds the server signing key. The C ABI has no network client or issuer private key; unsafe pointer handling is isolated in its exported boundary.

The managed gate and drain controller use an internal `TimeProvider`. Production uses `TimeProvider.System`; tests supply controlled UTC, monotonic time and timers. Neither the production ABI nor environment configuration accepts a clock override.

## Trust and deployment

Issuer private keys remain on the server. Production verifier public keys are embedded at build time from the public registry selected by `LICENSE_GUARD_TRUST_FILE`. Empty trust fails closed. Only explicit dev builds accept separate development trust/inventory inputs.

Signed files, HTTP data and reported inventory are untrusted until validated. Product/features come from trusted service configuration, never from job payloads. Root ownership protects against an unprivileged service, not against the host administrator.

Deploy one API process with SQLite on persistent local storage, behind a TLS reverse proxy, under a dedicated server account. There is no public administrative API. Client packages exclude server/admin executables and private issuer material.

Several worker processes may share one installation; activation slots do not count worker processes. The CLI manages authorization files and does not install software or restart customer services.

## Extension boundaries

The wire protocol can be implemented by another server backend. Multi-node databases, ARM64/musl packages, TPM keys, authenticated time and floating seats require separate implementation and validation. See [recorded validation and staging limits](09-testing.md#recorded-validation).
