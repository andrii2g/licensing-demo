# .NET 10 integration

`LicenseGuard.Managed` is the reusable wrapper project. It uses source-generated LibraryImport and System.Text.Json, loads one absolute configured native library per process, checks ABI version, and distinguishes completed native calls from valid authorization.

`NativeAotWorker` performs its startup check before host construction/start. Its per-job gate, watchdog and independent drain controller illustrate service integration. `NativeAotWorker.Tests` is a deterministic executable test harness using an internal TimeProvider; it does not need a test framework or real-time sleeps.

```bash
dotnet run --project samples/dotnet/NativeAotWorker.Tests -c Release
bash scripts/demo-local.sh --extended
bash samples/dotnet/publish-linux.sh linux-x64
bash scripts/test-aot.sh
bash scripts/test-runtime-image.sh
```

The worker is configured through trusted service environment values: LICENSE_NATIVE_PATH, LICENSE_FILE, LICENSE_IDENTITY_FILE, LICENSE_PRODUCT and LICENSE_FEATURE. These values do not supply a clock, inventory, keys or validity override. The dev native artifact alone accepts separate development test inputs.

The only validated publish RID is linux-x64. An AOT executable still requires the matching external Rust .so and native OS libraries. See the root README and IMPLEMENTATION_STATUS.md for prerequisites, commands and actual evidence.
