namespace NativeAotWorker;

// Keeps already admitted work independent from the host's immediate stop token.
internal sealed class DrainController : IDisposable
{
    private readonly object _sync = new();
    private readonly CancellationTokenSource _cancellation = new();
    private readonly ITimer _timer;
    private bool _disposed;

    public DrainController(TimeProvider? clock = null)
    {
        _timer = (clock ?? TimeProvider.System).CreateTimer(
            _ => Expire(), null, Timeout.InfiniteTimeSpan, Timeout.InfiniteTimeSpan);
    }

    public CancellationToken Token => _cancellation.Token;
    public bool IsCancellationRequested => _cancellation.IsCancellationRequested;

    private void Expire()
    {
        lock (_sync)
        {
            // Disposing a timer does not withdraw a callback already queued.
            if (!_disposed) _cancellation.Cancel();
        }
    }

    public void Start()
    {
        lock (_sync)
        {
            if (!_disposed) _timer.Change(SampleOptions.DrainTimeout, Timeout.InfiniteTimeSpan);
        }
    }

    public void Dispose()
    {
        lock (_sync)
        {
            if (_disposed) return;
            _disposed = true;
            _timer.Dispose();
            _cancellation.Dispose();
        }
    }
}
