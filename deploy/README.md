# Direct Linux deployment

Validated binaries target Ubuntu 24.04 x86_64 / glibc 2.39. Review these templates before installation; the demo never installs services or enrolls a real machine.

## Client host

Create the `licenseguard` group and an unprivileged `productworker` service account. Review the archive paths, then install binaries as root. Create state/configuration directories using the supplied tmpfiles rules. Install client.toml as root:licenseguard 0640, license state as root:licenseguard 0750, and the native library as root:root 0644 under /usr/lib/license-guard.

Run activation using a root updater. Public identity and license files are 0640 with the state directory's group. Private device seed, journal, pending operation, lock and high-water metadata are 0600. The worker must not read private state or write any authorization file.

For linux-host-v1, verify the real product_uuid sysfs target and provision only the required read permission for licenseguard through a distribution-specific reviewed boot mechanism. Test it after reboot as productworker. No generic ACL installer, broad /sys grant, root worker, or setuid helper is provided. Where persistent narrow access is unavailable, use an explicitly approved linux-machine-v1 entitlement.

Test local status as the worker account before starting the service. The worker itself gates startup, including direct launches. Enable the daily renewal timer for renewable mode. Its --scheduled invocation is a no-op for a still-valid manual_offline license. It never restarts stopped workers implicitly.

## Trusted server

Use a dedicated `licenseissuer` account. Create /var/lib/license-guard-server and /etc/license-guard-server with that account as owner, mode 0700. Keep server.toml and issuer.key 0600. The admin keygen command can create a protected keys subdirectory. The server and admin use protected server-account storage; client/FFI ownership checks remain root-only.

Run entitlement administration as licenseissuer. SQLite, WAL and shared-memory files stay on persistent local storage in the protected directory; use SQLite's backup API or a stopped consistent backup, not a live main-file copy. The included unit runs one API instance and handles SIGINT gracefully. Terminate TLS in a separately configured local reverse proxy. Production clients require HTTPS with normal CA validation.

## Staging checklist

- Verify actual physical/VMware identifiers, expected copies/migrations/reinstalls, online CPU topology and permission denial.
- Reboot and recheck DMI and state access as the actual service account.
- Verify reverse-proxy TLS, firewall, rate limits, backups and key rotation.
- Verify systemd start/stop/restart policies on the target host.
- Confirm that no dev-trust package, fixture override, private issuer key or admin/server executable is in a client installation.

The automated Docker test is a runtime compatibility smoke test using synthetic data; it is not a supported container binding model.
