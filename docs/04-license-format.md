# Signed license format, v1

## Envelope
UTF-8 JSON object with exactly protected, payload, signature. Each is canonical base64url without padding (RFC 4648 URL-safe alphabet, reject nonzero pad bits by decode+reencode equality).
protected decodes to JSON:
~~~json
{"typ":"license-guard-lease-v1","alg":"Ed25519","kid":"issuer-2026-01"}
~~~
kid is a bounded identifier selecting a pretrusted public key. No algorithm negotiation.
Signing input is exactly:
~~~text
UTF8("license-guard/v1\n") || ASCII(protected) || ASCII(".") || ASCII(payload)
~~~
Here \n means ONE byte 0x0a, not two backslash/n characters. Use ordinary Ed25519 over those bytes, not Ed25519ph and not a separately hashed message. Signature decodes to exactly 64 bytes.
Do not parse/reserialize protected or payload to reconstruct the signing input. Parsing before verification is only for bounded structural checks and selecting a trusted kid; no claims become authorized until signature succeeds.
Producer may serialize JSON however it likes within strict rules; verification uses transmitted bytes. Fixtures use compact sorted JSON for reproducibility.

## Payload fields
See contracts/license-claims.schema.json. Fields are:
- schema_version=1; issuer="license-guard"; product and license_id/customer_id;
- installation_id and installation_public_key_sha256;
- sequence (positive integer, monotonic per installation);
- issued_at, not_before, lease_valid_until, entitlement_expires_at (UTC Unix seconds);
- mode renewable or manual_offline;
- features (unique case-sensitive ASCII IDs, at least one);
- binding (policy, machine_id_hash, system_uuid_hash);
- max_logical_processors (positive integer or null).
Every listed field is required; nullable fields must be explicitly null. Unknown fields fail in v1. Never embed an activation token or device private key.

## Time and policy semantics
Valid exactly when not_before <= now < lease_valid_until <= entitlement_expires_at.
Also require not_before <= issued_at < lease_valid_until and issued_at <= now + 120.
Issuer normally uses issued_at=server_now and not_before=server_now-120 to tolerate small client lag. Expiry gets no implicit grace.
renewable: lease_valid_until = min(server_now+configured_lease_seconds, entitlement_expires_at).
manual_offline: lease_valid_until = entitlement_expires_at.
Issuer must reject already expired entitlements and zero-duration leases.
Product must equal the caller's compiled/configured expected product; all required_features must be present. Empty required_features is allowed for diagnostic inspection only, never for sample worker startup.
max_logical_processors compares with total online guest/host logical processors. Unknown CPU count fails if there is a limit.
Binding and public installation identity must match. Missing required facts are errors.

## Validation order
1. Bounded file read and envelope shape, strict base64/header decoding.
2. Known type/version/algorithm/kid and strict signature verification.
3. Strict claims schema plus semantic invariants.
4. Product/features, public installation identity, time, live machine binding, optional capacity.
5. Return full typed decision; only valid authorizes work.
A verification failure must not expose unverified customer/features as trusted claims.

## Runtime deadline
Once a lease is accepted, calculate a monotonic deadline from its remaining UTC duration. A repeated check of the same sequence must not extend that deadline after clock rollback. A newly signed higher sequence may establish a new deadline.
Worker admission checks the cached monotonic deadline on every job; file/identity revalidation runs at least every 60 seconds. Forward wall-clock jumps can fail early. Backward jumps cannot extend a running process's accepted deadline.
Per-process highest sequence blocks downgrade during that process. Installer persists highest installed sequence and rejects downgrades, but root/snapshot rollback across restarts remains outside the guarantee.
At exact expiry, no new admission; already admitted work has the configured drain timeout.

## Renewal/retirement
A renewal increments sequence and issues a fresh lease. Installing an older or same-sequence different envelope is an error. Exact identical envelope replay is safe and a no-op.
Retirement denies future renewal. Existing leases remain valid until expiry, with at most local poll delay for other policy changes and the allowed drain time for in-flight work.
Never describe a server-side flag as instantly invalidating an offline signature.

## Test trust
Fixtures use kid TEST-ONLY-issuer-v1 and public, deterministic seeds. Production builds must not include that key, and must reject it even if a trust bundle contains it. Tests inject trust into core only; a separately named development package may explicitly embed dev trust.
