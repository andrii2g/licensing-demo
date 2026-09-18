using System.Text.Json;

namespace LicenseGuard.Managed;

public sealed class NativeLicenseChecker
{
    private readonly byte[] _request;
    private readonly string _requiredFeature;

    public NativeLicenseChecker(LicenseOptions options)
    {
        _requiredFeature = options.RequiredFeature;
        if (string.IsNullOrWhiteSpace(_requiredFeature)) throw new ArgumentException("Required feature is empty.");
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

        CheckDuplicateProperties(output.AsSpan(0, checked((int)written)));
        var result = JsonSerializer.Deserialize(
            output.AsSpan(0, checked((int)written)),
            LicenseJsonContext.Default.ValidationResult)
            ?? throw new InvalidOperationException("NATIVE_RESULT_EMPTY");

        if (result.SchemaVersion != 1 || string.IsNullOrEmpty(result.Code) ||
            result.Features is null || result.CheckedAt <= 0 || result.CheckedAt > 4102444800L ||
            result.Valid != (result.Code == "VALID"))
            throw new InvalidOperationException("NATIVE_RESULT_INVALID");

        if (result.Valid &&
            (string.IsNullOrEmpty(result.LicenseId) ||
             !Guid.TryParseExact(result.InstallationId, "D", out _) ||
             result.Sequence is not > 0 || result.Sequence > 9007199254740991L ||
             !result.Features.Contains(_requiredFeature, StringComparer.Ordinal) ||
             result.LeaseValidUntil is not > 0 ||
             result.EntitlementExpiresAt is not > 0 ||
             result.LeaseValidUntil > result.EntitlementExpiresAt ||
             result.LeaseValidUntil > 4102444800L ||
             result.LeaseDigest is not { Length: 64 } ||
             !result.LeaseDigest.All(char.IsAsciiHexDigit)))
            throw new InvalidOperationException("NATIVE_CLAIMS_INVALID");

        if (!result.Valid && (result.LicenseId is not null || result.InstallationId is not null ||
            result.Sequence is not null || result.LeaseValidUntil is not null ||
            result.EntitlementExpiresAt is not null || result.LeaseDigest is not null || result.Features.Length != 0))
            throw new InvalidOperationException("NATIVE_DENIAL_INVALID");
        return result;
    }
    private static void CheckDuplicateProperties(ReadOnlySpan<byte> bytes)
    {
        var reader = new Utf8JsonReader(bytes, new JsonReaderOptions { MaxDepth = 16 });
        var objects = new Stack<HashSet<string>>();
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.StartObject) objects.Push(new(StringComparer.Ordinal));
            else if (reader.TokenType == JsonTokenType.EndObject) objects.Pop();
            else if (reader.TokenType == JsonTokenType.PropertyName && !objects.Peek().Add(reader.GetString()!))
                throw new InvalidOperationException("NATIVE_DUPLICATE_PROPERTY");
        }
    }
}

