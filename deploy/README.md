# Deployment templates

Templates target direct Linux service installation, not containers. They reference future Rust binaries and the included C# AOT sample.
Do not execute these as a generic installer: create users/groups, review paths and native runtime dependencies, and configure your server/entitlement first.
The worker process itself handles license denial; there is no ExecStartPre-only gate.
DMI read access is intentionally a documented deployment gate. A generic setuid helper or running the worker as root would weaken the design.
Use license-guard-renew.timer only for renewable policy. For manual_offline, renewal can be manual and the timer may be omitted.
The API's TLS reverse proxy and issuer-secret provisioning belong to the trusted server deployment and are not configured with customer credentials here.

