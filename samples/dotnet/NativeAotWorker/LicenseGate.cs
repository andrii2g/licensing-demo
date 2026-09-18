

namespace NativeAotWorker;

// UTC and monotonic time share an internal injectable provider; production uses TimeProvider.System.
internal sealed class LicenseGate(TimeProvider? timeProvider = null)
{
    private readonly TimeProvider _time = timeProvider ?? TimeProvider.System;
    private readonly object _sync = new();
    private bool _closed;
    private string? _installationId;
    private string? _licenseId;
    private long _sequence;
    private string? _digest;
    private long _expiresAt;
    private long _startedAt;
    private TimeSpan _budget;
    private int _activeJobs;

    public bool Accept(ValidationResult result, out string failure)
    {
        lock (_sync)
        {
            failure = result.Code;
            if (_closed || !result.Valid)
                {
                    _closed = true;
                    return false;
                }

            var now = _time.GetUtcNow();
            var remaining = DateTimeOffset.FromUnixTimeSeconds(
                result.LeaseValidUntil!.Value) - now;
            if (remaining <= TimeSpan.Zero)
            {
                failure = "LICENSE_EXPIRED";
                {
                    _closed = true;
                    return false;
                }
            }

            if (_installationId is not null)
            {
                if (_installationId != result.InstallationId || _licenseId != result.LicenseId)
                {
                    failure = "INSTALLATION_MISMATCH";
                    {
                    _closed = true;
                    return false;
                }
                }
                if (result.Sequence!.Value < _sequence ||
                    (result.Sequence!.Value == _sequence && result.LeaseDigest != _digest))
                {
                    failure = "LEASE_ROLLBACK";
                    {
                    _closed = true;
                    return false;
                }
                }
                if (result.Sequence!.Value == _sequence)
                {
                    var existing = RemainingLocked();
                    if (existing < remaining) remaining = existing;
                    if (remaining <= TimeSpan.Zero)
                    {
                        failure = "LICENSE_EXPIRED";
                        {
                    _closed = true;
                    return false;
                }
                    }
                }
            }

            _installationId = result.InstallationId;
            _licenseId = result.LicenseId;
            _sequence = result.Sequence!.Value;
            _digest = result.LeaseDigest;
            _expiresAt = result.LeaseValidUntil!.Value;
            _budget = remaining;
            _startedAt = _time.GetTimestamp();
            return true;
        }
    }

    private TimeSpan RemainingLocked()
    {
        var monotonic = _budget - _time.GetElapsedTime(_startedAt);
        var wall = DateTimeOffset.FromUnixTimeSeconds(_expiresAt) - _time.GetUtcNow();
        return monotonic < wall ? monotonic : wall;
    }

    public TimeSpan NextCheckDelay()
    {
        lock (_sync)
        {
            if (_closed) return TimeSpan.Zero;
            var remaining = RemainingLocked();
            return remaining <= TimeSpan.Zero ? TimeSpan.Zero :
                remaining < SampleOptions.ValidationInterval
                    ? remaining : SampleOptions.ValidationInterval;
        }
    }

    public bool TryEnter(out IDisposable? admission)
    {
        lock (_sync)
        {
            admission = null;
            if (_closed || _installationId is null || RemainingLocked() <= TimeSpan.Zero)
                return false;
            _activeJobs++;
            admission = new Admission(this);
            return true;
        }
    }

    public int Close()
    {
        lock (_sync)
        {
            _closed = true;
            return _activeJobs;
        }
    }

    private void Leave()
    {
        lock (_sync) _activeJobs--;
    }

    private sealed class Admission(LicenseGate owner) : IDisposable
    {
        private LicenseGate? _owner = owner;
        public void Dispose() => Interlocked.Exchange(ref _owner, null)?.Leave();
    }
}

