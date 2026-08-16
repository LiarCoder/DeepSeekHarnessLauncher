using System;
using System.Diagnostics;

namespace DeepSeekHarnessLauncher
{
    internal sealed class HarnessUpdater
    {
        private readonly FileLogger logger;

        public HarnessUpdater(FileLogger logger)
        {
            this.logger = logger;
        }

        public string GetInstalledVersion()
        {
            return RunVolta("run dsh --version", TimeSpan.FromSeconds(15)).Trim();
        }

        public string GetLatestVersion()
        {
            var output = RunVolta(
                "run npm view @deepseek-ai/dsh dist-tags.latest --json",
                TimeSpan.FromSeconds(30));
            return output.Trim().Trim('"');
        }

        public void InstallLatestVersion()
        {
            RunVolta("install @deepseek-ai/dsh@latest", TimeSpan.FromMinutes(5));
        }

        private string RunVolta(string arguments, TimeSpan timeout)
        {
            logger.Info("执行 Volta 命令：volta " + arguments);
            using (var process = new Process
            {
                StartInfo = new ProcessStartInfo
                {
                    FileName = VoltaLocator.ResolveExecutable(),
                    Arguments = arguments,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    WindowStyle = ProcessWindowStyle.Hidden,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                }
            })
            {
                process.Start();
                var standardOutput = process.StandardOutput.ReadToEndAsync();
                var standardError = process.StandardError.ReadToEndAsync();

                if (!process.WaitForExit((int)timeout.TotalMilliseconds))
                {
                    process.Kill();
                    throw new TimeoutException("Volta 命令执行超时");
                }

                var output = standardOutput.GetAwaiter().GetResult();
                var error = standardError.GetAwaiter().GetResult();
                if (!string.IsNullOrWhiteSpace(output))
                {
                    logger.Info(output.Trim());
                }

                if (!string.IsNullOrWhiteSpace(error))
                {
                    logger.Error(error.Trim());
                }

                if (process.ExitCode != 0)
                {
                    throw new InvalidOperationException(
                        "Volta 命令执行失败（退出代码 " + process.ExitCode + "）：" + Environment.NewLine + error.Trim());
                }

                return output;
            }
        }
    }
}
