using Microsoft.Extensions.Hosting;

namespace NativeAotWorker;

internal sealed class LicenseWatchdog(
    NativeLicenseChecker checker, LicenseGate gate,
    LicenseShutdown shutdown) : BackgroundService
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

