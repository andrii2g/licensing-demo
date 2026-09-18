using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;
using NativeAotWorker;

internal static class Program
{
    public static async Task<int> Main(string[] args)
    {
        var exitState = new ExitState();
        NativeLicenseChecker checker;
        var gate = new LicenseGate();

        try
        {
            checker = new NativeLicenseChecker(SampleOptions.Read().ToLicenseOptions());
            var initial = checker.Check();
            if (!gate.Accept(initial, out var code))
            {
                Console.Error.WriteLine($"LICENSE_DENIED Code={code} Ready=false");
                return 78;
            }
        }
        catch (Exception)
        {
            Console.Error.WriteLine("LICENSE_DENIED Code=NATIVE_STARTUP_ERROR Ready=false");
            return 78;
        }

        try
        {
            // No host or worker starts before the explicit check above.
            var builder = Host.CreateApplicationBuilder(args);
            builder.Logging.ClearProviders();
            builder.Logging.AddSimpleConsole(options => options.SingleLine = true);
            builder.Services.Configure<HostOptions>(options =>
                options.ShutdownTimeout = TimeSpan.FromSeconds(35));
            builder.Services.AddSingleton(checker);
            builder.Services.AddSingleton(gate);
            builder.Services.AddSingleton(exitState);
            builder.Services.AddSingleton<LicenseShutdown>();
            builder.Services.AddHostedService<LicenseWatchdog>();
            builder.Services.AddHostedService<DemoWorker>();
            using var host = builder.Build();

            var lifetime = host.Services.GetRequiredService<IHostApplicationLifetime>();
            lifetime.ApplicationStopping.Register(() => gate.Close());
            var logger = host.Services.GetRequiredService<ILoggerFactory>()
                .CreateLogger("LicenseStartup");
            lifetime.ApplicationStarted.Register(() =>
                logger.LogInformation("LICENSE_READY Ready=true"));

            await host.RunAsync();
            return exitState.Code;
        }
        catch (Exception)
        {
            Console.Error.WriteLine("HOST_FAILED");
            return exitState.Code != 0 ? exitState.Code : 70;
        }
    }
}

