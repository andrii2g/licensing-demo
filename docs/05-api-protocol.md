# Central API protocol and transaction rules

OpenAPI lives in contracts/openapi.yaml. JSON Schemas define request and response shapes; this document specifies authentication, authorization and transaction semantics beyond OpenAPI.

## Endpoints
| Method/path | Authentication | Purpose |
|---|---|---|
| POST /v1/challenges | Activation token for action activate; no token for renew/retire | Fresh nonce scoped to action/installation |
| POST /v1/activations | Activation token plus device signature | Approve installation and issue first lease |
| POST /v1/installations/{id}/renew | Stored device-key signature | Refresh inventory and issue next lease |
| POST /v1/installations/{id}/retire | Stored device-key signature | Deny future renewals; hold slot until last expiry |
| GET /health/live | None | Process health, no sensitive details |
| GET /health/ready | None | Database/signing-key readiness, no private details |

No public entitlement creation/rebind/admin endpoints in v1. Use license-admin locally on the trusted server.

## Challenge
Request contains schema_version=1, action (activate/renew/retire), installation_id, product.
For activate require the Authorization header with the Bearer scheme and the activation token; resolve its entitlement. For other actions look up installation. Use generic not-found/unauthorized response and rate limits to avoid unlimited identity probing.
Generate challenge_id UUIDv4 plus random 32-byte nonce (base64url), expiry server_now+300. Persist action, installation, product, entitlement and optional activation token digest. Max 10 outstanding challenges per installation; configurable server rate limits. Purge expired records.
Response: schema_version, challenge_id, nonce, expires_at.
Challenge creation is not authorization to issue a license. Recheck entitlement during the mutation transaction.

## Signed device requests
Same three-field envelope and exact signing input as docs/04-license-format.md. Protected typ="license-guard-request-v1", alg="Ed25519", kid=installation_id.
Activation payload fields: schema_version, action="activate", operation_id UUIDv4, challenge_id, nonce, installation_id, product, installation_public_key (raw 32 bytes base64url), inventory.
Renew payload has same fields with action="renew"; public key must equal the stored one.
Retire payload action="retire" has the same identity fields and omits inventory. Each action has its own closed schema.
Activate verifies proof of possession with the submitted key AND validates the activation token. Never accept device self-signature as entitlement authorization.
Renew/retire verify with the DATABASE key, not the submitted key. Route ID, protected kid, payload ID, challenge scope, product and action must agree.
Reject weak device keys and use strict signature verification.
No client timestamp is needed for nonce expiry; server time determines freshness.

## Atomic mutation algorithm
1. Bound/read/parse envelope; verify token where required and signature.
2. Enter a short SQLite BEGIN IMMEDIATE transaction (or equivalent serialized repository operation).
3. Check current entitlement and installation status. No lease issuance for revoked/expired/retired state.
4. Compute request_digest = SHA256(ASCII(protected)+"."+ASCII(payload)+"."+ASCII(signature)).
5. Look up (installation_id, action, operation_id). Same digest returns the stored exact response if still permitted by step 3; different digest -> 409 IDEMPOTENCY_CONFLICT. A successful retire replay may return its cached acknowledgment even though the installation is now retired.
6. For a new operation require unconsumed, unexpired challenge with all scope fields matching; constant-time compare nonce digests.
7. Recheck activation-slot allowance, binding/capacity, and entitlement policy.
8. Apply installation/sequence updates, sign final claims, serialize exact response, consume challenge, persist idempotency response and audit event.
9. Commit before responding. Any failure including signing rolls back all mutation.
Signing uses an in-process loaded issuer key in the reference backend and must not await an external KMS while holding SQLite write lock. Future KMS integration needs a separate issuance design.

Do not hold a DB transaction while waiting for HTTP reads. Use a dedicated DB actor or bounded spawn_blocking work for rusqlite, not blocking Tokio executor threads.
Configure busy timeout and map lock contention to retryable 503. WAL is useful for readers but does not allow multiple writers.

## Retrying
Installer persists operation envelope before sending. If response is lost, retry identical bytes and operation ID. A consumed nonce may succeed only through this exact authenticated cached operation.
Idempotency cache retention: at least 24h. Expired cached lease is never installed as valid. If a successful response can no longer be recovered, request a new challenge/operation using the SAME installation identity.
Activation is also naturally unique on installation_id: existing active ID with same entitlement/key/binding consumes no extra slot; issue a next-sequence lease. Different key/entitlement for the same ID -> conflict.
Client never resends the same operation ID with a new nonce/payload.

## Slot accounting and retirement
Count active installations (even if their last lease expired), plus retired installations for which reserved_until > now. Active installs require explicit retirement to free capacity.
On retire set status=retired, reserved_until=last_lease_valid_until and deny future renewal. A device cannot be silently reactivated under the same retired ID.
This prevents immediate reissue from exceeding the allowed unexpired authorizations; it does not prevent exact clones from sharing one authorization.
A replacement needing immediate overlap requires an explicit audited extra slot or administrator exception. Preserve the old lease expiry during rebind; do not pretend deactivation proves all copies stopped.

## Responses/errors
Issue endpoints return the signed lease envelope directly. Retire returns schema_version, installation_id, retired_at, reserved_until.
Error shape: schema_version=1, code, message (sanitized), retryable, request_id.
400 malformed; 401 invalid credential/signature; 403 entitlement denied; 404 unknown installation; 409 conflict/slot limit/binding change; 410 retired; 422 inventory/policy mismatch; 429 throttled; 503 transient service/DB unavailable.
Do not return a full inventory or token in errors. No wildcard acceptance of unknown modes or versions.
Retry only network failures, 429 and 503 with bounded jitter/backoff; honor bounded Retry-After. Never erase a valid local lease on a failed renewal.
