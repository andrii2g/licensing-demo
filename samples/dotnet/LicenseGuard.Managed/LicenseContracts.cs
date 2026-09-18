using System.Text.Json.Serialization;

namespace LicenseGuard.Managed;

internal sealed class ValidationRequest
{
    [JsonPropertyName("schema_version")] public int SchemaVersion { get; init; } = 1;
    [JsonPropertyName("license_path")] public required string LicensePath { get; init; }
    [JsonPropertyName("identity_path")] public required string IdentityPath { get; init; }
    [JsonPropertyName("product")] public required string Product { get; init; }
    [JsonPropertyName("required_features")] public required string[] RequiredFeatures { get; init; }
}

public sealed class ValidationResult
{
    [JsonPropertyName("schema_version")] public required int SchemaVersion { get; init; }
    [JsonPropertyName("valid")] public required bool Valid { get; init; }
    [JsonPropertyName("code")] public required string Code { get; init; }
    [JsonPropertyName("checked_at")] public required long CheckedAt { get; init; }
    [JsonPropertyName("license_id")] public required string? LicenseId { get; init; }
    [JsonPropertyName("installation_id")] public required string? InstallationId { get; init; }
    [JsonPropertyName("sequence")] public required long? Sequence { get; init; }
    [JsonPropertyName("lease_valid_until")] public required long? LeaseValidUntil { get; init; }
    [JsonPropertyName("entitlement_expires_at")] public required long? EntitlementExpiresAt { get; init; }
    [JsonPropertyName("features")] public required string[] Features { get; init; }
    [JsonPropertyName("lease_digest")] public required string? LeaseDigest { get; init; }
}

[JsonSourceGenerationOptions(UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow)]
[JsonSerializable(typeof(ValidationRequest))]
[JsonSerializable(typeof(ValidationResult))]
internal partial class LicenseJsonContext : JsonSerializerContext { }

