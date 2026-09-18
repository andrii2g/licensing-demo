namespace NativeAotWorker;

internal sealed record SampleOptions(
    string NativeLibraryPath, string LicensePath,
    string IdentityPath, string Product, string RequiredFeature)
{
    public LicenseOptions ToLicenseOptions() => new(NativeLibraryPath, LicensePath, IdentityPath, Product, RequiredFeature);

    public static SampleOptions Read()
    {
        static string Value(string name, string fallback) =>
            Environment.GetEnvironmentVariable(name) is { Length: > 0 } value
                ? value : fallback;

        var options = new SampleOptions(
            Value("LICENSE_NATIVE_PATH", "/usr/lib/license-guard/liblicense_guard.so"),
            Value("LICENSE_FILE", "/var/lib/license-guard/license.lic"),
            Value("LICENSE_IDENTITY_FILE", "/var/lib/license-guard/installation.json"),
            Value("LICENSE_PRODUCT", "worker-suite"),
            Value("LICENSE_FEATURE", "messaging"));

        if (!Path.IsPathFullyQualified(options.LicensePath) ||
            !Path.IsPathFullyQualified(options.IdentityPath))
            throw new InvalidOperationException("License paths must be absolute.");
        if (string.IsNullOrWhiteSpace(options.RequiredFeature))
            throw new InvalidOperationException("Required feature is empty.");
        return options;
    }

    public static readonly TimeSpan ValidationInterval = TimeSpan.FromSeconds(60);
    public static readonly TimeSpan DrainTimeout = TimeSpan.FromSeconds(30);
}

