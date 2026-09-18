# Decisions, defaults, and scope

## Confirmed requirements
Rust implements the license installer and checker. Existing C# services run on Linux. Activation is online and sends installation details to a central server, including MAC addresses, hostname and CPU counts. Deployments include physical hardware and VMware. A failed license check prevents .NET workers from starting.

## Selected defaults
| Decision | Default | Reason |
|---|---|---|
| Product | License Guard | Product-independent reusable component |
| Repository | andrii2g/licensing-demo | Public reference implementation |
| Reference backend | Rust + Axum + SQLite | Local runnable demo; shared protocol types |
| Managed sample | .NET 10 | Modern source-generated interop and AOT test |
| License scope | One installation per activated host | Several worker processes may share one license |
| Renewal | Every day | Tolerates short outages |
| Authorization duration | 604800 seconds | Seven days, capped by entitlement end |
| Runtime checks | At most 60 seconds between checks | Bound local reaction delay |
| Shutdown drain | 30 seconds | Finish work when possible without unbounded shutdown |
| Binding | linux-host-v1 or explicitly authorized linux-machine-v1 | Hardware/VM identifier plus OS identity where usable |
| CPU licensing | Disabled unless entitlement has a limit | Inventory should not become an accidental restriction |
| Platform | Linux x86_64 glibc first | Concrete initial compatibility target |

The implementation request explicitly selects these defaults and the independently runnable Rust reference backend. All business policies remain server-controlled.

## Offline alternative
An entitlement may have mode manual_offline. Activation still occurs online, but the issued license lasts until the finite entitlement expiry. Renew it manually online before that time. No periodic connectivity is required. There is no client flag that converts renewable to offline. Perpetual licenses and unsigned emergency grace are outside v1.

## Business definitions
- Entitlement: customer/product/features/expiry/policy and installation-slot allowance.
- Installation: persisted UUID plus device key and approved binding baseline.
- Lease: signed current authorization for that installation.
- Slot: one active installation, or a retired installation whose last lease has not yet expired.
- Inventory: customer-reported machine facts, useful for support and capacity checks, not attestation.
- Feature: a case-sensitive ASCII identifier required by a particular service.
- Capacity: optional maximum guest/host online logical processors, not a limit on concurrent services.

## Non-goals
No automatic payment integration, dashboard, multi-region service, floating licenses, fleet discovery, mobile/Windows client, Docker/Kubernetes binding, TPM, obfuscation/anti-debugging, or custom cryptography. Do not promise secure expiry against system-clock manipulation or exact clone detection.

## Change record
Before changing signed claims or fingerprint normalization, create a new version or explicitly compatible migration. Record business changes with the effective date, affected contracts and new tests.


## Implementation choices (2026-09-18)
- Rust 1.97.1; .NET SDK 10.0.401 / Hosting 10.0.12; dependency versions locked after successful API compilation.
- Empty production trust is fail-closed. Release packaging requires an explicit public trust file. Dev trust/host inputs are compiled only with the dev feature, in separate target/dev artifacts.
- The reference API listens on loopback behind a TLS reverse proxy. It ignores forwarded-address headers; rate limiting uses the socket peer (a conservative shared proxy limit) plus installation ID.
- Dedicated server-account storage is accepted only by server/admin APIs. Production client/FFI store reads still require root ownership.
- A protected identity journal makes first identity publication recoverable. High-water metadata remains after local removal; a new verified lease can repair an older installed file without lowering the high-water mark.
- Managed wrapper is a reusable LicenseGuard.Managed project; sample lifecycle uses an internal TimeProvider for deterministic tests.
- GitHub Actions workflows are omitted at the user's request. Validation remains local through scripts/check.sh and the existing demo, AOT and release scripts.
- Repository maintenance uses one deployed SQLite migration and a required asset-integrity manifest for byte-exact contracts and cryptographic fixtures. Source layout and validation commands live in the architecture and testing guides.
