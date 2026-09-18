namespace NativeAotWorker;

// Keeps already admitted work independent from the host's immediate stop token.
internal sealed class DrainController : IDisposable
{
    private readonly CancellationTokenSource _cancellation = new();
    private readonly ITimer _timer;
    public DrainController(TimeProvider? clock = null)
    {
        _timer = (clock ?? TimeProvider.System).CreateTimer(
            _ => _cancellation.Cancel(), null, Timeout.InfiniteTimeSpan, Timeout.InfiniteTimeSpan);
    }
    public CancellationToken Token => _cancellation.Token;
    public bool IsCancellationRequested => _cancellation.IsCancellationRequested;
    public void Start() => _timer.Change(SampleOptions.DrainTimeout, Timeout.InfiniteTimeSpan);
    public void Dispose() { _timer.Dispose(); _cancellation.Dispose(); }
}
