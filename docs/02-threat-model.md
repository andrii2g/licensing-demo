# Threat model and security rules

## Protect
Prevent an ordinary user from inventing signed entitlements, changing product/features/expiry, casually copying a license to a different host, consuming extra activation slots through races, or activating with a replayed request.
Prevent malformed inputs from crashing the worker through avoidable parser errors, leaking credentials or invoking arbitrary code.
Preserve service availability through a bounded licensing-server outage while a valid lease exists.

## Explicit limits
A root administrator can patch .NET, replace native code, emulate identifiers and restore a complete VM snapshot. Rust native code raises some reverse-engineering effort but is not a trusted execution boundary.
Offline validity depends on local UTC. A process monotonic deadline bounds one running process; restart or snapshot rollback bypasses software-only historical state. A last-seen timestamp is operational evidence, not secure time.
All client inventory is self-reported. MACs, DMI UUID and Linux machine ID are identifiers, not secrets or attestation.
Activation limits constrain server-issued installations, not simultaneous execution of exact clones using one device identity.
Revocation cannot invalidate a previously issued offline file instantly.

## Key management
Issuer Ed25519 private keys are server-side secrets in protected storage, never checked into Git, embedded in the installer, supplied to the native client, or printed.
The server signs only after authorization and a successful transactional decision. Device Ed25519 private keys authenticate device requests, not entitlement issuance.
Use an OS CSPRNG. Generate activation tokens from 32 random bytes, encoded base64url without padding. Store only SHA-256 of their ASCII token value; high entropy is required and human-chosen license numbers are not activation credentials.
Rotate issuer keys by first shipping a trusted public-key bundle containing old and new kids, then signing with the new key. Remove old trust only after policy review and old-lease migration.
Unknown kid fails. A response cannot supply its own trusted public key. Production rejects fixture keys.

## Transport
Production HTTPS only, normal CA validation, no certificate bypass switch. Do not follow cross-origin redirects or forward credentials on redirects; easiest default is no redirects.
The development HTTP allowance is compiled only into dev tools and limited to loopback. A URL containing localhost as a suffix is not sufficient; parse host correctly.
Set connect/total timeouts, cap response sizes and limit retries. Never log Authorization, private key material, full token, raw signed request or full inventory by default.

## Filesystem
Root-owned directories and library search paths; service has read access only. Reject symlink destinations and non-regular files. Open relative to a trusted directory descriptor, use no-follow flags, verify ownership/mode with fstat on the opened object, and cap reads.
An adversarial root is outside scope; do not pretend arbitrary native pointers can be safely validated against malicious in-process code either.

## Parsing
Allowlisted envelope versions/types/algorithm. Reject duplicate JSON keys at every depth, unknown v1 fields, non-integer times, noncanonical base64url, invalid lengths, oversized data and trailing junk.
Strict Ed25519 verification with maintained library. Enforce maximum envelope 65536 bytes; maximum decoded payload 49152 bytes, header 1024 bytes, JSON nesting 16. HTTP request body limit 131072 bytes.
No unsigned feature, local config or environment variable can grant validity.

## Incident controls
On suspected issuer compromise, stop issuing, rotate keys and deliver an updated trust bundle; existing offline clients cannot learn revocation magically.
On device key compromise, retire that installation, retain its slot until last issued expiry, and issue replacement according to policy.
Use administrative audit events for issue, renew, retire, rebind and key rotation; sanitize identifiers in operational logs.
