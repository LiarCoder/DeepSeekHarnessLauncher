using System;
using System.IO;

namespace DeepSeekHarnessLauncher
{
    internal static class VoltaLocator
    {
        public static string ResolveExecutable()
        {
            var voltaHome = Environment.GetEnvironmentVariable("VOLTA_HOME");
            if (!string.IsNullOrWhiteSpace(voltaHome))
            {
                var candidate = Path.Combine(voltaHome, "volta.exe");
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }

            var path = Environment.GetEnvironmentVariable("PATH") ?? string.Empty;
            foreach (var directory in path.Split(Path.PathSeparator))
            {
                var trimmedDirectory = directory.Trim().Trim('"');
                if (trimmedDirectory.Length == 0)
                {
                    continue;
                }

                var candidate = Path.Combine(trimmedDirectory, "volta.exe");
                if (File.Exists(candidate))
                {
                    return candidate;
                }
            }

            throw new FileNotFoundException("未找到 volta.exe，请确认 Volta 已安装并已加入 PATH");
        }
    }
}
