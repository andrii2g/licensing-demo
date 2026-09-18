# Primary references

These references support implementation choices; this project's protocol and business defaults are its own design.

- Ed25519 specification and test vectors: https://www.rfc-editor.org/rfc/rfc8032
- Strict verifier behavior and weak-key discussion: https://docs.rs/ed25519-dalek/latest/ed25519_dalek/struct.VerifyingKey.html
- Linux machine-ID privacy/derivation guidance: https://www.freedesktop.org/software/systemd/man/latest/machine-id.html
- Linux CPU topology: https://www.kernel.org/doc/html/v5.5/admin-guide/cputopology.html
- VMware moved/copied UUID behavior: https://knowledge.broadcom.com/external/article?legacyId=1541
- SQLite transactions and BEGIN IMMEDIATE: https://www.sqlite.org/lang_transaction.html
- .NET native interop best practices: https://learn.microsoft.com/en-us/dotnet/standard/native-interop/best-practices
- .NET P/Invoke source generation: https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke-source-generation
- .NET Generic Host lifecycle: https://learn.microsoft.com/en-us/dotnet/core/extensions/generic-host
- .NET Native AOT deployment prerequisites: https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/
- systemd.service exit/restart behavior: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html

Dependency versions must be selected and locked during implementation; links using latest are not version pins. systemd and DMI behavior must be verified on the target distro. No fixture or source reference changes the hostile-root/VM-clone limitation.
