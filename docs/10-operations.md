# Operations and support

## First installation
Install release binaries and root-owned verifier trust. Create licenseguard group and productworker service user. Create directories with supplied tmpfiles rules.
Choose binding policy on the entitlement. If using linux-host-v1, provision narrow DMI read permission and test as productworker; otherwise explicitly approve linux-machine-v1 on the server.
Configure product, HTTPS endpoint and state path in /etc/license-guard/client.toml. Generate the entitlement with license-admin in the trusted server environment.
Run licensectl inspect, then licensectl activate. Verify status as the service user. Start sample/product services only after status succeeds. Enable daily renewal timer for renewable mode.
Run inspect before sending data if the customer needs to review collected inventory; activation's purpose includes transmitting the documented fields.

## systemd
Supplied service templates use Type=simple. The worker itself gates startup. Do not depend solely on ExecStartPre; launching the executable directly must still validate.
Worker exits 78 for licensing failure. RestartPreventExitStatus=78 suppresses automatic retry loops; Restart=on-failure still covers other failures.
After renewal fixes an expired installation, an operator explicitly starts the stopped worker. A successful renew command does not silently restart production jobs.
No WatchdogSec or Type=notify is configured because the sample does not implement sd_notify.
Renew runs as an isolated oneshot with outbound networking. The worker needs no licensing-server connectivity.

## Monitoring
Expose status code, authorization expiry, last successful renewal, renewal latency and inventory-change summaries.
Alert seven days/one day ahead of entitlement expiry and on repeated renewal failures; short leases need immediate outage visibility.
Log request_id/license_id/installation_id only where useful and access-controlled. Do not log tokens, private seeds, raw machine IDs or complete signed requests.
Retain inventory only as required by the installation-management purpose; define retention and authorized access in deployment documentation. MAC/hostname are identifying operational data.

## Troubleshooting
| Code | Operator action |
|---|---|
| LICENSE_MISSING | Activate or correct trusted path |
| INVALID_SIGNATURE / UNKNOWN_KEY | Check file integrity and matching release trust |
| PRODUCT_MISMATCH / FEATURE_MISSING | Correct entitlement or service configuration |
| IDENTITY_UNAVAILABLE | Check machine-id/DMI and service-user permissions |
| MACHINE_MISMATCH | Investigate reinstall/clone/hardware change; request rebind |
| CAPACITY_EXCEEDED | Review guest/host logical CPU allowance |
| NOT_YET_VALID / CLOCK_SUSPECT | Check time synchronization; do not grant unsigned grace |
| LICENSE_EXPIRED | Renew; investigate entitlement or server outage |
| INSTALLATION_MISMATCH | Restore matching public/device identity or reactivate |
| LEASE_ROLLBACK | Investigate old backup; obtain new lease |
| ABI/loader error | Install matching architecture, libc and ABI version |

## Replacement/rebind
Approve a new installation through an administrator-controlled workflow. Retire the old one; keep reservation until all issued authorization expires. If immediate overlap is required, explicitly increase allowed slots for the transition and audit it.
Lost local identity must not produce an automatic free replacement. Support needs entitlement and machine-change context.
Hardware maintenance is expected. Do not require unchanged hostname/MAC/CPU model.

## Backup/recovery
Server: consistent SQLite backup, protected issuer keys, activation-token hashes, audit and installation sequence state. Never copy a live SQLite main file alone without its WAL using naive file copy.
Client: treat identity backup as sensitive; restoring it may duplicate a device identity. Reissue after reviewed disaster recovery.
If server database is restored behind issued sequences, do not issue lower-sequence leases. Recover issuance high-water history or migrate affected installations via an explicit recovery procedure.
Monitor server clock. A server clock jump can produce bad leases for all clients.

## Distribution
Build client packages separate from server/admin packages. Declare architecture/libc minimum and system dependencies. Root-owned absolute native library path.
Include SBOM/dependency versions and checksums. Sign software releases using your release process independently of license signing.
Never publish keys from a real deployment. Public fixtures are test-only and production cannot trust them.
