using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace NativeAotWorker;

internal sealed class ExitState
{
    private int _code;
    public int Code => Volatile.Read(ref _code);
    public void LicensingFailure() => Interlocked.Exchange(ref _code, 78);
    public void WorkerFailure() => Interlocked.CompareExchange(ref _code, 70, 0);
}

internal sealed class LicenseShutdown(
    LicenseGate gate, ExitState exitState,
    IHostApplicationLifetime lifetime, ILogger<LicenseShutdown> logger)
{
    private int _requested;

    public void Fail(string code)
    {
        // An ordinary host stop must not become a licensing failure.
        if (lifetime.ApplicationStopping.IsCancellationRequested) return;
        if (Interlocked.Exchange(ref _requested, 1) != 0) return;
        exitState.LicensingFailure();
        int active = gate.Close();
        logger.LogError(
            "LICENSE_DENIED Code={Code} Ready=false ActiveJobs={ActiveJobs}", code, active);
        lifetime.StopApplication();
    }
}

