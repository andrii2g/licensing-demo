using LicenseGuard.Managed;
using NativeAotWorker;

static class Program
{
    static int _assertions;
    static void Assert(bool value, string name)
    {
        _assertions++;
        if (!value) throw new InvalidOperationException(name);
    }
    static ValidationResult Lease(long sequence, long until, string digest = "a") => new()
    {
        SchemaVersion = 1, Valid = true, Code = "VALID", CheckedAt = 1800144000,
        LicenseId = "test", InstallationId = "11111111-1111-4111-8111-111111111111",
        Sequence = sequence, LeaseValidUntil = until, EntitlementExpiresAt = 1800244000,
        Features = ["messaging"], LeaseDigest = new string(digest[0], 64)
    };
    static void Main()
    {
        const long start = 1800144000;
        var clock = new FakeTime(start);
        var gate = new LicenseGate(clock);
        Assert(!gate.TryEnter(out _), "uninitialized admission");
        Assert(gate.Accept(Lease(1, start + 100), out _), "initial lease");
        Assert(gate.TryEnter(out var admission), "valid admission");
        clock.Advance(30);
        clock.Wall -= 300;
        Assert(gate.Accept(Lease(1, start + 100), out _), "same lease after clock rollback");
        clock.Advance(71);
        Assert(!gate.TryEnter(out _), "rollback must not extend monotonic lifetime");
        admission!.Dispose(); admission.Dispose();
        Assert(gate.Close() == 0, "admission dispose idempotent");

        clock = new FakeTime(start);
        gate = new LicenseGate(clock);
        Assert(gate.Accept(Lease(1, start + 10), out _), "renewal initial");
        clock.Advance(5);
        Assert(gate.Accept(Lease(2, start + 100), out _), "higher sequence extends");
        clock.Advance(10);
        Assert(gate.TryEnter(out var renewed), "renewal admission");
        renewed!.Dispose();
        Assert(!gate.Accept(Lease(1, start + 100), out var code) && code == "LEASE_ROLLBACK", "lower sequence denied");
        Assert(!gate.Accept(Lease(3, start + 200), out _), "denial terminal");

        gate = new LicenseGate(new FakeTime(start));
        Assert(gate.Accept(Lease(1, start + 100), out _), "digest setup");
        Assert(!gate.Accept(Lease(1, start + 100, "b"), out code) && code == "LEASE_ROLLBACK", "same sequence mutation denied");

        clock = new FakeTime(start);
        gate = new LicenseGate(clock);
        Assert(gate.Accept(Lease(1, start + 2), out _), "expiry setup");
        Assert(gate.NextCheckDelay() == TimeSpan.FromSeconds(2), "watchdog earlier deadline");
        clock.Advance(2);
        Assert(!gate.TryEnter(out _), "exact expiry admission denied");
        Assert(gate.NextCheckDelay() == TimeSpan.Zero, "expired immediate poll");

        gate = new LicenseGate(new FakeTime(start));
        Assert(gate.Accept(Lease(1, start + 100), out _), "close setup");
        Assert(gate.TryEnter(out var active), "active job");
        Assert(gate.Close() == 1 && !gate.TryEnter(out _), "close preserves admitted work");
        active!.Dispose();
        Assert(gate.Close() == 0, "drained");
        clock = new FakeTime(start);
        using (var drain = new DrainController(clock))
        {
            drain.Start();
            clock.Advance(29);
            Assert(!drain.IsCancellationRequested, "drain permits work before timeout");
            clock.Advance(1);
            Assert(drain.IsCancellationRequested, "drain cancels at exactly 30 seconds");
        }
        Console.WriteLine($"PASS: {_assertions} deterministic gate assertions (no sleeps); monotonic rollback, renewal, sequence/digest, exact expiry and drain admission");
    }
    sealed class FakeTime(long start) : TimeProvider
    {
        public long Wall = start;
        long _ticks;
        readonly List<FakeTimer> _timers = [];
        public override long TimestampFrequency => TimeSpan.TicksPerSecond;
        public override long GetTimestamp() => _ticks;
        public override DateTimeOffset GetUtcNow() => DateTimeOffset.FromUnixTimeSeconds(Wall);
        public void Advance(int seconds)
        {
            Wall += seconds; _ticks += seconds * TimeSpan.TicksPerSecond;
            foreach (var timer in _timers.ToArray()) timer.Fire(_ticks);
        }
        public override ITimer CreateTimer(TimerCallback callback, object? state, TimeSpan dueTime, TimeSpan period)
        {
            var timer = new FakeTimer(this, callback, state);
            _timers.Add(timer); timer.Change(dueTime, period); return timer;
        }
        sealed class FakeTimer(FakeTime clock, TimerCallback callback, object? state) : ITimer
        {
            long _due = long.MaxValue;
            public bool Change(TimeSpan dueTime, TimeSpan period)
            {
                if (period != Timeout.InfiniteTimeSpan) throw new NotSupportedException();
                _due = dueTime == Timeout.InfiniteTimeSpan ? long.MaxValue : clock._ticks + dueTime.Ticks;
                return true;
            }
            public void Fire(long ticks) { if (ticks >= _due) { _due = long.MaxValue; callback(state); } }
            public void Dispose() => _due = long.MaxValue;
            public ValueTask DisposeAsync() { Dispose(); return ValueTask.CompletedTask; }
        }
    }
}
