# .NET 10 integration and Native AOT

## Projects and build

[LicenseGuard.Managed](../samples/dotnet/LicenseGuard.Managed/) is the reusable wrapper. It uses source-generated `LibraryImport` and `System.Text.Json`, loads an absolute native-library path and checks ABI version 1. Configure one checker per process and reuse it; the native library resolver is process-wide for this assembly.

[NativeAotWorker](../samples/dotnet/NativeAotWorker/) is the runnable sample. [NativeAotWorker.Tests](../samples/dotnet/NativeAotWorker.Tests/) tests its gate and drain controller with controlled time.

The repository pins .NET SDK 10.0.401 and Microsoft.Extensions.Hosting 10.0.12. Normal and AOT dependency graphs have separate lockfiles selected by `samples/dotnet/Directory.Build.targets`. Linux Native AOT requires the compiler/linker and zlib development prerequisites in the [README](../README.md).

Run from the repository root on Ubuntu/WSL:

```bash
dotnet run --project samples/dotnet/NativeAotWorker.Tests -c Release -r linux-x64 -p:PublishAot=false -p:RestoreLockedMode=true
bash scripts/demo-local.sh --extended
bash scripts/test-aot.sh
bash scripts/test-runtime-image.sh
```

The AOT test builds the matching dev Rust library, publishes the worker and exercises actual processes. The runtime-image test uses those built artifacts on stock Ubuntu without .NET or network. To publish only the managed worker, run `bash samples/dotnet/publish-linux.sh linux-x64`; output goes to `artifacts/native-worker/linux-x64/`.

Only linux-x64 on the documented Ubuntu/glibc baseline is validated. An AOT executable still needs the external Rust `.so` and native OS libraries.

## Trusted configuration

| Environment variable | Default |
|---|---|
| LICENSE_NATIVE_PATH | /usr/lib/license-guard/liblicense_guard.so |
| LICENSE_FILE | /var/lib/license-guard/license.lic |
| LICENSE_IDENTITY_FILE | /var/lib/license-guard/installation.json |
| LICENSE_PRODUCT | worker-suite |
| LICENSE_FEATURE | messaging |

These values are service configuration; they do not provide trust keys, host facts, a clock or a validity override. The sample requires a nonempty feature. Additional features can be requested through the native ABI by an application-specific adapter.

## Startup and runtime

`Program.Main` constructs the checker and validates before building or starting the host. Missing or wrong-architecture libraries, ABI mismatch, malformed native responses and licensing denials exit 78 before any job starts. Native return 0 only indicates a complete response; the wrapper also requires a valid typed result.

`LicenseGate` checks each admission against UTC expiry and a monotonic deadline. Revalidating the same sequence may shorten but never extend its lifetime. A higher sequence can renew authorization; lower sequences, a changed same-sequence digest or a changed installation close the gate.

`LicenseWatchdog` schedules the next check within 60 seconds or sooner at the accepted expiry. Failed validation closes admission, logs readiness false and stops the host with exit 78. Invalid runtime state is terminal for that process.

Admitted jobs use an independent drain token. `StopAsync` closes admission and gives active work up to 30 seconds; the host timeout is 35 seconds. Ordinary SIGTERM drains and returns 0. The drain timer is synchronized with disposal so an already queued callback cannot use a disposed cancellation source.

The sample logs readiness transitions but exposes no HTTP health endpoint. Production services should connect gate state to their own readiness checks.

## Adapting a service

Keep constructors and dependency registration free of work. Validate before `Host.StartAsync/RunAsync`, then put every dequeue/admission behind the gate. On closure, stop consumption and acknowledge only completed work according to the service's delivery model.

For Kafka or another queue, pause new consumption, drain already admitted messages within the deadline and retain existing retry/commit semantics. The sample deliberately has no queue-client dependency. C# does not separately collect binding facts or interpret signed license claims.

## Verification

The [validation guide](09-testing.md) maps deterministic tests, actual C ABI calls, normal managed processes, AOT processes and clean-image/package checks. It also records executed results and remaining real-host staging checks.
