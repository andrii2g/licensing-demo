namespace LicenseGuard.Managed;

public sealed record LicenseOptions(
    string NativeLibraryPath, string LicensePath,
    string IdentityPath, string Product, string RequiredFeature);
