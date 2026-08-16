using System;
using System.IO;
using System.Linq;
using System.Text;

namespace DeepSeekHarnessLauncher
{
    internal sealed class FileLogger : IDisposable
    {
        private const int MaximumLogFiles = 10;
        private readonly object syncRoot = new object();
        private readonly StreamWriter writer;

        public FileLogger()
        {
            Directory.CreateDirectory(AppPaths.LogsDirectory);
            CurrentLogFile = Path.Combine(
                AppPaths.LogsDirectory,
                "launcher-" + DateTime.Now.ToString("yyyyMMdd-HHmmss-fff") + ".log");
            writer = new StreamWriter(CurrentLogFile, false, new UTF8Encoding(false)) { AutoFlush = true };
            DeleteOldLogs();
        }

        public string CurrentLogFile { get; }

        public void Info(string message)
        {
            Write("INFO", message);
        }

        public void Error(string message)
        {
            Write("ERROR", message);
        }

        public void Harness(string message)
        {
            Write("HARNESS", message);
        }

        public void Dispose()
        {
            lock (syncRoot)
            {
                writer.Dispose();
            }
        }

        private void Write(string level, string message)
        {
            lock (syncRoot)
            {
                writer.WriteLine("{0:yyyy-MM-dd HH:mm:ss.fff} [{1}] {2}", DateTime.Now, level, message);
            }
        }

        private static void DeleteOldLogs()
        {
            var oldLogs = new DirectoryInfo(AppPaths.LogsDirectory)
                .GetFiles("launcher-*.log")
                .OrderByDescending(file => file.CreationTimeUtc)
                .Skip(MaximumLogFiles);

            foreach (var oldLog in oldLogs)
            {
                try
                {
                    oldLog.Delete();
                }
                catch (IOException)
                {
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }
    }
}
