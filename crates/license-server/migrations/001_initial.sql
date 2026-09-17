PRAGMA foreign_keys = ON;
BEGIN IMMEDIATE;

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);

CREATE TABLE entitlements (
    license_id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL,
    product TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'revoked')),
    mode TEXT NOT NULL CHECK(mode IN ('renewable', 'manual_offline')),
    binding_policy TEXT NOT NULL CHECK(binding_policy IN ('linux-host-v1', 'linux-machine-v1')),
    expires_at INTEGER NOT NULL CHECK(expires_at > 0),
    max_installations INTEGER NOT NULL CHECK(max_installations > 0),
    lease_seconds INTEGER NOT NULL DEFAULT 604800 CHECK(lease_seconds > 0),
    max_logical_processors INTEGER CHECK(max_logical_processors > 0),
    features_json TEXT NOT NULL CHECK(json_valid(features_json)),
    created_at INTEGER NOT NULL
);

CREATE TABLE activation_tokens (
    token_sha256 TEXT PRIMARY KEY CHECK(length(token_sha256) = 64),
    license_id TEXT NOT NULL REFERENCES entitlements(license_id),
    status TEXT NOT NULL CHECK(status IN ('active', 'revoked')),
    created_at INTEGER NOT NULL
);

CREATE TABLE installations (
    installation_id TEXT PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES entitlements(license_id),
    public_key BLOB NOT NULL CHECK(length(public_key) = 32),
    status TEXT NOT NULL CHECK(status IN ('active', 'retired')),
    binding_json TEXT NOT NULL CHECK(json_valid(binding_json)),
    inventory_json TEXT NOT NULL CHECK(json_valid(inventory_json)),
    sequence INTEGER NOT NULL CHECK(sequence > 0),
    last_lease_valid_until INTEGER NOT NULL,
    reserved_until INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    retired_at INTEGER
);
CREATE INDEX installations_license_status ON installations(license_id, status, reserved_until);

CREATE TABLE challenges (
    challenge_id TEXT PRIMARY KEY,
    installation_id TEXT NOT NULL,
    license_id TEXT NOT NULL REFERENCES entitlements(license_id),
    action TEXT NOT NULL CHECK(action IN ('activate', 'renew', 'retire')),
    product TEXT NOT NULL,
    nonce_sha256 TEXT NOT NULL CHECK(length(nonce_sha256) = 64),
    token_sha256 TEXT,
    expires_at INTEGER NOT NULL,
    consumed_at INTEGER,
    created_at INTEGER NOT NULL
);
CREATE INDEX challenges_installation_expiry ON challenges(installation_id, expires_at);

CREATE TABLE operations (
    installation_id TEXT NOT NULL REFERENCES installations(installation_id),
    action TEXT NOT NULL CHECK(action IN ('activate', 'renew', 'retire')),
    operation_id TEXT NOT NULL,
    request_digest TEXT NOT NULL CHECK(length(request_digest) = 64),
    response_json TEXT NOT NULL CHECK(json_valid(response_json)),
    created_at INTEGER NOT NULL,
    cache_expires_at INTEGER NOT NULL,
    PRIMARY KEY(installation_id, action, operation_id)
);
CREATE INDEX operations_cleanup ON operations(cache_expires_at);

CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    occurred_at INTEGER NOT NULL,
    request_id TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    action TEXT NOT NULL,
    license_id TEXT,
    installation_id TEXT,
    details_json TEXT NOT NULL CHECK(json_valid(details_json))
);
CREATE INDEX audit_events_license ON audit_events(license_id, occurred_at);

INSERT INTO schema_migrations(version, applied_at) VALUES (1, unixepoch());
COMMIT;

