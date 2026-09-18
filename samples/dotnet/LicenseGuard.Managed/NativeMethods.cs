using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace LicenseGuard.Managed;

internal static unsafe partial class NativeMethods
{
    // Deliberately only one native library/version per process.
    public static void Configure(string path)
    {
        if (!Path.IsPathFullyQualified(path))
            throw new ArgumentException("Native library path must be absolute.");
        NativeLibrary.SetDllImportResolver(
            typeof(NativeMethods).Assembly,
            (string name, Assembly assembly, DllImportSearchPath? searchPath) =>
                name == "license_guard"
                    ? NativeLibrary.Load(path)
                    : IntPtr.Zero);
    }

    [LibraryImport("license_guard", EntryPoint = "lg_abi_version")]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial uint AbiVersion();

    [LibraryImport("license_guard", EntryPoint = "lg_validate_v1")]
    [UnmanagedCallConv(CallConvs = new[] { typeof(CallConvCdecl) })]
    internal static partial int Validate(
        byte* request, nuint requestLength,
        byte* output, nuint outputCapacity,
        nuint* outputLength);
}

