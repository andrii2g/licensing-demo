# Included .NET 10 Native AOT background-service sample

Actual sample source is under samples/dotnet/NativeAotWorker/. It implements the ABI wrapper, typed source-generated JSON, gate, watchdog, bounded simulated jobs and process exit behavior. It requires the Rust ABI implementation to run successfully. This kit environment did not have dotnet or cargo; compile and runtime checks are mandatory implementation gates, not claimed completed.

## Build
Use an SDK with .NET 10 and the Linux Native AOT prerequisites for the target distribution (native compiler/linker and zlib development library).
Run samples/dotnet/publish-linux.sh. Default RID linux-x64; publish output is artifacts/native-worker/linux-x64.
The sample project sets PublishAot=true, InvariantGlobalization=true and uses Microsoft.Extensions.Hosting 10.0.0 as a reproducible starting version. Review/update to the current supported .NET 10 servicing version and lock packages during implementation.
Place the correct Rust library at the configured absolute path; it remains an external native dependency, even though the worker itself is a native executable.

## Startup
Program reads trusted environment/service configuration for license path, public identity path, native library path, expected product and required feature. Example defaults match the deployment templates.
NativeLicenseChecker loads the absolute library, checks ABI version, sends a bounded request and parses a source-generated response.
Any invalid result, malformed native response, ABI failure or missing library exits 78 before building/starting the host. No network call is made.
Only after a valid result is accepted into LicenseGate are hosted services started. Constructors are inert.

## Runtime
LicenseGate guards every job admission. It stores the accepted sequence/digest/identity and a monotonic deadline as well as UTC expiry. Revalidating the same lease may shorten but never extend its deadline.
Watchdog revalidates no later than 60 seconds and schedules an earlier wake for known expiry. A new higher sequence can extend permission; lower sequence, changed same-sequence digest or changed installation fails.
A failed runtime check closes admission, emits a structured reason and calls StopApplication. The process returns 78 after shutdown.
Demo jobs use an independent drain cancellation token. StopAsync closes admission immediately and cancels active work after 30 seconds. Host shutdown timeout is 35 seconds. Ordinary SIGTERM exits 0, drains work and does not mark license failure.
The demo prints authorization readiness transitions. It has no HTTP server or health endpoint; production adapters should connect the same gate to readiness.

## Production adapters
Wrap dequeue/admission in a gate lease. Do not acknowledge an unprocessed message when authorization closes. For Kafka, pause/stop consumption, finish admitted messages where possible and commit only completed work according to the existing delivery model. No Kafka package is required by the sample.
Check all required features per host capability. Do not inspect license JSON directly elsewhere or introduce a global bool without deadline checks.
The core check uses live CPU facts, including hotplug changes. Inventory and binding are not collected independently in C#.

## Required tests
- No accepted-job log/counter on missing/expired/wrong-product/bad-signature startup.
- Valid published native worker logs JOB_STARTED.
- Actual native checker receives exact UTF-8 request and returns denial even when ABI return is zero.
- Replace file with a higher-sequence lease; process stays running and deadline advances.
- Same lease with backward wall clock does not extend its monotonic lifetime (unit fake clock).
- Invalid replacement closes admission; current job drains; exit 78.
- SIGTERM while valid drains; exit 0.
- Missing .so, wrong architecture, bad ABI, truncated response all fail closed.
- Published worker starts on a supported Linux machine without any .NET runtime installed.
