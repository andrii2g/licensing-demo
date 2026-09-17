using System.Text.Json;

namespace NativeAotWorker;

internal sealed class NativeLicenseChecker
{
    private readonly byte[] _request;

    public NativeLicenseChecker(SampleOptions options)
    {
        NativeMethods.Configure(options.NativeLibraryPath);
        if (NativeMethods.AbiVersion() != 1)
            throw new InvalidOperationException("NATIVE_ABI_MISMATCH");

        _request = JsonSerializer.SerializeToUtf8Bytes(
            new ValidationRequest
            {
                LicensePath = options.LicensePath,
                IdentityPath = options.IdentityPath,
                Product = options.Product,
                RequiredFeatures = [options.RequiredFeature]
            }, LicenseJsonContext.Default.ValidationRequest);
        if (_request.Length > 8192)
            throw new InvalidOperationException("NATIVE_REQUEST_TOO_LARGE");
    }

    public unsafe ValidationResult Check()
    {
        byte[] output = new byte[16384];
        nuint written = 0;
        int status;
        fixed (byte* requestPtr = _request)
        fixed (byte* outputPtr = output)
        {
            status = NativeMethods.Validate(
                requestPtr, (nuint)_request.Length,
                outputPtr, (nuint)output.Length, &written);
        }
        if (status != 0 || written == 0 || written > (nuint)output.Length)
            throw new InvalidOperationException("NATIVE_CALL_FAILED");

        var result = JsonSerializer.Deserialize(
            output.AsSpan(0, checked((int)written)),
            LicenseJsonContext.Default.ValidationResult)
            ?? throw new InvalidOperationException("NATIVE_RESULT_EMPTY");

        if (result.SchemaVersion != 1 || string.IsNullOrEmpty(result.Code) ||
            result.Features is null || result.CheckedAt <= 0 ||
            result.Valid != (result.Code == "VALID"))
            throw new InvalidOperationException("NATIVE_RESULT_INVALID");

        if (result.Valid &&
            (string.IsNullOrEmpty(result.LicenseId) ||
             !Guid.TryParseExact(result.InstallationId, "D", out _) ||
             result.Sequence is not > 0 ||
             result.LeaseValidUntil is not > 0 ||
             result.EntitlementExpiresAt is not > 0 ||
             result.LeaseValidUntil > result.EntitlementExpiresAt ||
             result.LeaseValidUntil > 4102444800L ||
             result.LeaseDigest is not { Length: 64 } ||
             !result.LeaseDigest.All(char.IsAsciiHexDigit)))
            throw new InvalidOperationException("NATIVE_CLAIMS_INVALID");

        return result;
    }
}

