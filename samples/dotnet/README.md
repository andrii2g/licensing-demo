# Native AOT worker sample

This is actual C# sample source with a native ABI dependency, not a compiled binary. Implement liblicense_guard.so from contracts/license_guard.h first.

Build on Linux with the .NET 10 SDK and Native AOT native prerequisites:
~~~bash
bash samples/dotnet/publish-linux.sh linux-x64
~~~
Publish output: artifacts/native-worker/linux-x64/NativeAotWorker.

Trusted service environment:
~~~text
LICENSE_NATIVE_PATH=/usr/lib/license-guard/liblicense_guard.so
LICENSE_FILE=/var/lib/license-guard/license.lic
LICENSE_IDENTITY_FILE=/var/lib/license-guard/installation.json
LICENSE_PRODUCT=worker-suite
LICENSE_FEATURE=messaging
~~~

The executable starts only with a valid license. Native call success alone does not authorize the service: the returned validity, code and claims are checked. Missing library/license exits 78. There is deliberately no demo bypass or fake-license flag.
See docs/08-dotnet-native-aot.md for lifecycle tests and publish acceptance. A future test build of the Rust library may contain test trust; production must reject fixture keys.
The sample reads environment variables because systemd controls them in this deployment. Do not expose these settings as untrusted per-request inputs.
Package restore/build/publish and real interop tests were not run in the kit-creation environment; run them before treating the sample as verified.

