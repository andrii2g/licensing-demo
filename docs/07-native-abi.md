# Native ABI v1

contracts/license_guard.h is authoritative. Export:
- uint32_t lg_abi_version(void), returns 1;
- int32_t lg_validate_v1(request_ptr, request_len, output_ptr, output_capacity, output_len_ptr).

Pointers/lengths use uint8_t/size_t; C# uses byte*/nuint. Explicit Cdecl. No Rust strings, Vec, enums, allocator-owned objects or exceptions cross the boundary.
Request UTF-8 JSON includes schema_version, license_path, identity_path, product, required_features. It has no clock, inventory, trusted-key, signature-bypass or validity override.
Paths must be absolute and read through secure store rules. Product/features originate from trusted service configuration. The production native library contains its own verifier trust and always collects live identity.

## Return semantics
0 = call completed: output contains a complete ValidationResult JSON, which may be valid=false. A successful native call is NOT a valid license.
Negative codes: -1 invalid arguments/request, -2 output buffer too small, -3 internal error/panic.
When output_len_ptr is nonnull initialize to 0 before work. -2 sets it to required length and writes no partial result. Bytes are UTF-8 without a terminating NUL.
Managed caller provides 16384 bytes and treats -2 as failure; no retry loop with changing data or unbounded allocation.
Null/zero mismatches are rejected. Zero-capacity output may only return -2 with required size after evaluation; it never authorizes anything.
Safety contract: nonnull pointers must refer to valid aligned memory of the promised lengths, nonoverlapping writable output, valid size_t output pointer. Rust cannot detect arbitrary invalid pointers; callers must uphold this contract.
Cap request to 8192 bytes. Never write beyond capacity. Thread-safe; no mutable global error string.
Use catch_unwind at extern boundary with panic=unwind; no panic should be expected in normal paths. OOM/abort/native memory corruption cannot be reliably caught. No unsafe code outside the small pointer-copy wrapper.

## ValidationResult
schema_version=1, valid boolean, code string, checked_at UTC seconds.
For valid=true also include license_id, installation_id, sequence, lease_valid_until, entitlement_expires_at, features and lease_digest. For valid=false those fields are null (features empty) to avoid treating partial claims as trusted.
code uses the stable names in contracts/status-codes.json. valid=true iff code=VALID.
lease_digest = lowercase hex SHA256(ASCII(protected)+"."+ASCII(payload)+"."+ASCII(signature)).
Result text is diagnostic; authorization uses typed validity/code and deadlines, not a substring search.

## Build/loading
Rust cdylib target name license_guard produces liblicense_guard.so.
.NET installs a DllImportResolver for logical name license_guard, loading an absolute configured root-owned file path. No current-directory or LD_LIBRARY_PATH fallback.
Check lg_abi_version before validation. A mismatch/load failure is a startup license-system failure (exit 78), never permission to continue.
Install separate tested libraries for each CPU/libc target. Required first target is x86_64 Linux glibc. Do not claim one Linux binary covers musl and glibc.

## Interop tests
Compile a tiny C client against the header. Test sizes/null cases with valid memory and canary buffers, denial results, non-ASCII diagnostic data, repeated calls and concurrency.
Run a real .NET managed process AND its Native AOT-published executable against the built Rust library. A mock wrapper test does not prove ABI compatibility.
Use fixture trust only in explicitly named dev test artifacts; verify production artifact refuses the test key.
