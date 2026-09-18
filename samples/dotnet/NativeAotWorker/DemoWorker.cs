using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace NativeAotWorker;

internal sealed class DemoWorker(
    LicenseGate gate, LicenseShutdown shutdown, ExitState exitState,
    IHostApplicationLifetime lifetime, ILogger<DemoWorker> logger) : BackgroundService
{
    private readonly DrainController _drain = new();

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        long job = 0;
        try
        {
            while (!stoppingToken.IsCancellationRequested &&
                   !lifetime.ApplicationStopping.IsCancellationRequested)
            {
                if (!gate.TryEnter(out var admission))
                {
                    if (!stoppingToken.IsCancellationRequested &&
                        !lifetime.ApplicationStopping.IsCancellationRequested)
                        shutdown.Fail("LICENSE_EXPIRED");
                    return;
                }

                using (admission)
                {
                    job++;
                    logger.LogInformation("JOB_STARTED Job={Job}", job);
                    // Simulates an admitted task. Host stop cancels new admission,
                    // while this independent token gives active work time to drain.
                    await Task.Delay(TimeSpan.FromSeconds(3), _drain.Token);
                    logger.LogInformation("JOB_COMPLETED Job={Job}", job);
                }

                await Task.Delay(TimeSpan.FromSeconds(1), stoppingToken);
            }
        }
        catch (OperationCanceledException) when (
            stoppingToken.IsCancellationRequested || _drain.IsCancellationRequested)
        {
            logger.LogInformation("WORKER_STOPPED LastJob={Job}", job);
        }
        catch (Exception)
        {
            gate.Close();
            exitState.WorkerFailure();
            logger.LogError("WORKER_FAILED");
            lifetime.StopApplication();
        }
    }

    public override async Task StopAsync(CancellationToken cancellationToken)
    {
        gate.Close();
        _drain.Start();
        await base.StopAsync(cancellationToken);
    }

    public override void Dispose()
    {
        _drain.Dispose();
        base.Dispose();
    }
}

