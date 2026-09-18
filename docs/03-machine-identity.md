# Linux identity and inventory

## Collection
Use bounded direct reads from /etc/machine-id, /sys/class/dmi/id/product_uuid, /sys/class/dmi/id/sys_vendor, /sys/class/dmi/id/product_name, /sys/devices/system/cpu/, /sys/class/net/, /etc/os-release, and uname/gethostname APIs.
Parse CPU online ranges, intersect with sched_getaffinity for available logical CPUs. Do not use available CPU count for a host capacity entitlement: service affinity could otherwise reduce the apparent capacity.
Enumerate package/core tuples for online CPUs. Report topology counts as null if data is missing or inconsistent. A core ID is unique only within a package.
In a VM, report guest-visible sockets/cores/vCPUs. Do not infer ESXi hardware from guest topology.
Collect interface name, normalized MAC and kind physical/virtual/unknown. Ignore loopback, all-zero and multicast MACs. Locally administered MACs are valid VMware inventory and must not be discarded. Sort by interface name then MAC. Max 64 entries.
Report hostname as inventory, bounded to 253 characters. CPU model and OS strings max 256. Parse os-release as data; never source it as shell.
A VMware hint from DMI is advisory. If uncertain, virtualization=unknown. No hidden probing of the hypervisor management network.

## Machine-ID normalization and privacy
Trim ASCII whitespace, require exactly 32 hexadecimal characters, reject all zeros, lowercase. Missing, empty, uninitialized and malformed values are errors for activation and validation.
Do not send the raw machine ID in inventory, logs or activation requests.
Define:
- binding namespace: exact UTF-8 product identifier, e.g. worker-suite.
- K = SHA256(UTF8("license-guard/fingerprint/v1/" + product)).
- machine_id_hash = lowercase hex HMAC-SHA256(K, UTF8("machine-id:" + normalized_machine_id)).
- Normalize DMI UUID by trimming ASCII whitespace, lowercasing, and parsing strict 8-4-4-4-12 hex groups; reject all-zero and all-f UUIDs.
- system_uuid_hash = lowercase hex HMAC-SHA256(K, UTF8("system-uuid:" + normalized_uuid)).
The key K is public domain separation for identifier privacy, not an authentication secret. Keep the product byte string identical on installer/server/checker. See fixtures/fingerprint-vector.json.

## Binding policies
linux-host-v1: machine_id_hash AND system_uuid_hash both required and must equal signed claims.
linux-machine-v1: machine_id_hash required; signed system_uuid_hash is null. Server must explicitly allow this weaker policy for that entitlement.
No scored voting, substring matches, opportunistic fallback, or MAC matching in v1.
Installer proposes collected evidence. Server chooses the policy from entitlement settings and records the exact baseline. Renewal must match that baseline. Missing data is not equal to a wildcard.
If host policy is configured but DMI is absent or unreadable, activation fails with guidance. Do not silently choose machine-only.
The checker must freshly collect the required facts. A cached root-generated snapshot is inventory evidence, not live binding.

## DMI permissions: required deployment decision
DMI product_uuid may be root-readable only. Workers remain unprivileged.
For linux-host-v1, the deployment must provide read access to exactly the required DMI node for the licenseguard group. Verify ownership and real file target before applying a narrow read ACL. Reapply as needed on reboot using an administrator-reviewed boot-time mechanism for the target distribution; sysfs permissions/ACL support differ.
Do not grant broad /sys access, run the worker as root, or install a setuid verifier.
If a narrow permission grant cannot be maintained on that platform, select the server-approved linux-machine-v1 policy, or defer to a future host helper design. Packaging must test permissions after reboot as the real service user.
The sample unit documents the DMI prerequisite. The operator must supply the narrow permission grant and verify it after reboot; the unit does not provision it.

## Installation identity
Generate a UUIDv4 installation_id and 32-byte Ed25519 device seed once, under an exclusive lock, before the first activation request. Persist identity durably before contacting the API.
Retry uses the same identity. Never regenerate because a request timed out.
Public installation.json contains version, installation_id, and installation_public_key (32 raw bytes base64url). Root-only device.key contains the seed, raw 32 bytes. Checker reads only installation.json.
Lease claims bind both installation_id and installation_public_key_sha256. This catches mismatched public identity; software-clone resistance is not implied.
For another product, use another configured state directory and installation identity in v1.

## Changes
Hostname/MAC/OS updates: inventory only. CPU expansion: enforce only a signed capacity limit.
OS reinstall, identity loss, changed system UUID: explicit retirement/rebind; no automatic override.
Exact clones: can remain indistinguishable; distinguish ordinary VMware copies with new UUIDs from snapshots preserving all values.
Never activate a golden image. Clone templates first, initialize OS identity correctly, then activate each intended installation.
