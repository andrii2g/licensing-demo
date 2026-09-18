# CLI and Linux storage

## Commands to implement
| Command | Behavior |
|---|---|
| licensectl inspect --config PATH [--json] | Collect inventory; no activation and no secrets |
| licensectl activate --config PATH [--token-stdin] | Interactive hidden prompt by default; online challenge + activation |
| licensectl renew --config PATH | Device-key authenticated renewal; no token needed |
| licensectl status --config PATH [--json] | Local validation and human-readable reasons; no network |
| licensectl verify --config PATH --file PATH [--json] | Validate a candidate, without installing |
| licensectl retire --config PATH | Explicit online retirement; retains historical expiry information |
| licensectl remove --config PATH | Remove local authorization after explicit local operation; does not free server slot |
| license-admin entitlement create --input PATH | Trusted-side create; print new random token once |
| license-admin entitlement revoke --id ID | Deny new issuance; existing leases remain valid |
| license-admin installation retire --id ID | Trusted-side retirement for lost machines |
| license-admin installations list --license-id ID | Support inventory/status without exposing secrets |

All mutation commands take an exclusive lock. Administrative tools are a different binary, never shipped in client packages. No activation token command-line argument. --token-stdin consumes a single bounded line and strips one terminal newline; never logs it. Production CLI uses a protected config path and HTTPS only.
status exits 0 only if valid, 78 on policy denial, 70 internal failure, 75 retryable resource error, 64 usage/config error. activate/renew return 0 after installation; 75 on retryable network/server outage while preserving an existing license. JSON goes to stdout, sanitized progress/errors to stderr.

## Files and permissions
Paths are for the sample product only; parameterize at packaging time:
| Path | Owner/mode | Purpose |
|---|---|---|
| /etc/license-guard/client.toml | root:licenseguard 0640 | Product, endpoint, state path |
| /var/lib/license-guard/ | root:licenseguard 0750 | Installation state |
| installation.json | root:licenseguard 0640 | Public ID and public device key |
| license.lic | root:licenseguard 0640 | Signed current authorization |
| device.key | root:root 0600 | Device signing seed |
| pending-operation.json | root:root 0600 | Signed request for exact retries, no activation token |
| install-state.json | root:root 0600 | Highest accepted sequence/digest |
| mutation.lock | root:root 0600 | Installer/renew lock |
| /usr/lib/license-guard/liblicense_guard.so | root:root 0644 | Production verifier with trusted public keys |
| /opt/license-guard/worker/ | root:root, no group write | Published worker executable |

Activation token need not persist. If activation process restarts before success, prompt for it again but reuse pending operation/identity.
The service account is in licenseguard and has no write permission here. Do not let it read device.key.
Multiple products use separate directories and installation IDs in v1.

## Durable identity creation
Create private key and public identity in a temporary root-only directory on the same filesystem, fsync files, and publish the identity as a recoverable transaction under lock. A crash between files must be detected and repaired from the private key without changing a previously published ID/key.
Do not regenerate on partial state or lost server response; fail with recovery guidance where consistency cannot be proved.
For every candidate identity ensure private key derives the public key before use.

## License installation
Read candidate with size limit; verify issuer, claims, product, installation, binding, time and capacity before replacing any current license.
Compare sequence and digest with current license and install-state high-water mark; same sequence identical digest -> no-op, lower/different -> reject.
Write a sibling temporary file with O_EXCL and mode 0640, set owner/group, fsync, atomically rename over license.lic, fsync parent.
Record high-water metadata atomically. Since file+metadata replacement is not one filesystem transaction, define crash recovery using a verified current license: advance stale metadata to its higher sequence; never downgrade license to match stale metadata. If metadata is ahead of installed license, request renewal/recovery, do not accept rollback.
Readers open one bounded regular file and observe either old or new complete bytes. They never need the mutation lock.
Do not overwrite a valid license with an error response or delete it on DNS/HTTP failure.

## Renewal
Timer invocation is root or a tightly scoped dedicated updater account with state write access; sample uses root to avoid pretending workers can write their authorization.
If renewable license has not reached renew_after policy interval, command may no-op; timer cadence remains daily.
manual_offline: timer logs a no-op while valid; explicit renew command may obtain replacement before expiry, subject to current entitlement.
Send fresh inventory and require original binding. Do not update binding baseline implicitly.
Retry transient failures at short bounded intervals; daily scheduling is separate. Stop on permanent denial and report actionable status.

## Removal versus retirement
Removing local files cannot revoke copies or free a server reservation. retire records a server decision and prevents renewal. Keep enough public metadata to explain reserved_until.
Never perform retire merely because a package is upgraded or a transient validation fails.
