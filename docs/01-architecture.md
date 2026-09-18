# Architecture and boundaries

## Components
license-core owns signed envelopes, license claim types, strict decoding, crypto and pure policy evaluation. It does not read files, call a network or log identifiers.

license-host reads Linux facts and computes a HostSnapshot. It separates inventory from identity, can represent missing/unknown facts, and never shells out to dmidecode, hostnamectl, ip, or lscpu.

license-store owns secure local paths, locking, durable writes, installation identity and the currently installed envelope.

licensectl composes the above with an HTTP client. It collects inventory, authenticates activation, signs device requests, validates every response before installation and reports status.

license-ffi composes the core, host and read-only store for production local checks. It exports only the C ABI. It has no network client or issuer private key.

license-server implements the public activation protocol and persistent entitlement/installation state. license-admin is a separate trusted-side CLI using the same database/domain modules. The customer distribution excludes both issuer tools and issuer credentials.

LicenseGuard.Managed calls the native ABI, converts results to managed typed data and owns no cryptographic policy. Sample.Worker demonstrates lifecycle behavior without coupling the library to Kafka, Redis or a particular product.

## Data flow
Admin creates entitlement and high-entropy activation token on trusted server.
Installer creates device identity -> requests challenge -> sends signed activation plus inventory.
Server approves and signs a lease -> installer verifies and atomically stores it.
Worker loads native checker -> checker verifies current file and live identity -> worker starts.
A timer renews; workers re-read authorization. Network availability does not affect still-valid local authorization.

## Trusted versus untrusted
Trusted issuer keys live only on the server. Trusted verifier public keys are embedded at build time or in a root-owned release trust bundle distributed with the software; the FFI request cannot override trust.
User-supplied license files, JSON, responses, clock, inventory and paths are untrusted inputs.
Root ownership prevents an unprivileged service from altering files; it is not protection against the machine administrator.
The product code decides the required product and feature. A caller cannot demand fewer features through an untrusted HTTP/job field.

## Core interfaces to implement
Clock -> UTC seconds; HostProvider -> HostSnapshot; LicenseVerifier -> VerifiedClaims; PolicyEvaluator -> ValidationDecision.
Store -> bounded read, locked identity creation, durable replacement; Issuer -> sign authorized claims; Repository -> transactional domain operations.
These interfaces allow deterministic tests without production bypass flags.

## Deployment
One reference API process, SQLite on persistent local storage, TLS at a reverse proxy, no public administrative API. Use a dedicated server user. Admin tools need protected local access and never run on a customer machine.
Many worker processes on one host can read the same signed license. No per-process seat counting is claimed.
Use ordinary Linux package installation to distribute the native library and service wrapper. licensectl activates a license; it does not install arbitrary software or restart customer services without an explicit command.

## Evolvability
The API contract is backend-independent. A future .NET API can implement it without changing installed workers.
PostgreSQL and concurrent multi-node API deployments require a new transaction/locking validation gate.
TPM device keys, authenticated time, clone sessions and floating seats are later features with distinct requirements.
