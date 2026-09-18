# Test strategy

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
Pure gate tests use controllable monotonic/UTC clock; sample uses Stopwatch and DateTimeOffset, refactor clock injection internally when adding tests.
No fixed-duration sleeps for unit tests. Process tests use bounded wait for readiness/job lines and explicit timeouts.
SIGTERM closes the gate before hosted-service cancellation; that race must still exit 0 rather than being classified as license expiry.
Expiry blocks admission immediately at cached deadline; poll revalidation happens within 60 seconds; bounded drain within 30 seconds after shutdown begins.
Renewal equal sequence must not reset countdown after clock rollback; higher sequence may renew. Invalid license update remains terminal for that process even if a later file appears.

## Operational tests
Real physical/VMware identity behavior needs staging hardware; mark unrun if unavailable. Test reboot preserves expected identity and DMI read permission as the actual service user.
Simulate server downtime longer/shorter than lease duration without modifying host wall time where avoidable.
Dependency audit, clippy, source-gen warnings, release test-key scanning and reproducible lockfiles form release gates.
