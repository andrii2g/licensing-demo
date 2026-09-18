using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace NativeAotWorker;

internal sealed class LicenseWatchdog(
    NativeLicenseChecker checker, LicenseGate gate,
    LicenseShutdown shutdown, ILogger<LicenseWatchdog> logger) : BackgroundService
{
    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        try
        {
            while (!stoppingToken.IsCancellationRequested)
            {
                await Task.Delay(gate.NextCheckDelay(), stoppingToken);
                var result = checker.Check();
                if (!gate.Accept(result, out var code))
                {
                    shutdown.Fail(code);
                    return;
                }
                logger.LogInformation("LEASE_ACCEPTED Sequence={Sequence}", result.Sequence);
            }
        }
        catch (OperationCanceledException) when (stoppingToken.IsCancellationRequested) { }
        catch (Exception)
        {
            // Avoid writing exception details that could contain sensitive inputs.
            shutdown.Fail("NATIVE_CHECK_ERROR");
        }
    }
}

