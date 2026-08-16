using System;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Threading;

namespace DeepSeekHarnessLauncher
{
    internal sealed class HarnessProcessManager : IDisposable
    {
        private readonly object syncRoot = new object();
        private Process process;
        private bool stopping;
        private bool ready;

        public event EventHandler UnexpectedlyExited;

        public event Action<string> OutputReceived;

        public Uri WebUiUri { get; private set; }

        public bool IsRunning
        {
            get
            {
                lock (syncRoot)
                {
                    return process != null && !process.HasExited;
                }
            }
        }

        public Uri StartAndWait(TimeSpan timeout)
        {
            lock (syncRoot)
            {
                if (process != null && !process.HasExited)
                {
                    return WebUiUri;
                }

                stopping = false;
                ready = false;
                var port = FindAvailablePort();
                var startInfo = new ProcessStartInfo
                {
                    FileName = ResolveVoltaExecutable(),
                    Arguments = "run dsh web --port " + port,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    WindowStyle = ProcessWindowStyle.Hidden,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true
                };

                process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };
                process.OutputDataReceived += OnOutputDataReceived;
                process.ErrorDataReceived += OnOutputDataReceived;
                process.Exited += OnProcessExited;

                if (!process.Start())
                {
                    throw new InvalidOperationException("无法启动 DeepSeek Harness");
                }

                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
                WebUiUri = new Uri("http://127.0.0.1:" + port + "/");
            }

            WaitUntilReady(timeout);
            lock (syncRoot)
            {
                ready = true;
            }
            return WebUiUri;
        }

        public void Stop()
        {
            Process processToStop;
            lock (syncRoot)
            {
                stopping = true;
                ready = false;
                processToStop = process;
            }

            if (processToStop == null || processToStop.HasExited)
            {
                return;
            }

            try
            {
                using (var taskKill = Process.Start(new ProcessStartInfo
                {
                    FileName = "taskkill.exe",
                    Arguments = "/PID " + processToStop.Id + " /T /F",
                    UseShellExecute = false,
                    CreateNoWindow = true,
                    WindowStyle = ProcessWindowStyle.Hidden
                }))
                {
                    taskKill?.WaitForExit(5000);
                }

                processToStop.WaitForExit(5000);
            }
            catch (InvalidOperationException)
            {
            }
        }

        public void Dispose()
        {
            Stop();
            lock (syncRoot)
            {
                process?.Dispose();
                process = null;
            }
        }

        private static int FindAvailablePort()
        {
            if (CanBind(3080))
            {
                return 3080;
            }

            var listener = new TcpListener(IPAddress.Loopback, 0);
            listener.Start();
            try
            {
                return ((IPEndPoint)listener.LocalEndpoint).Port;
            }
            finally
            {
                listener.Stop();
            }
        }

        private static bool CanBind(int port)
        {
            var listener = new TcpListener(IPAddress.Loopback, port);
            try
            {
                listener.Start();
                return true;
            }
            catch (SocketException)
            {
                return false;
            }
            finally
            {
                listener.Stop();
            }
        }

        private static string ResolveVoltaExecutable()
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

        private void WaitUntilReady(TimeSpan timeout)
        {
            var stopwatch = Stopwatch.StartNew();
            while (stopwatch.Elapsed < timeout)
            {
                Process currentProcess;
                lock (syncRoot)
                {
                    currentProcess = process;
                }

                if (currentProcess == null || currentProcess.HasExited)
                {
                    throw new InvalidOperationException("DeepSeek Harness 在服务就绪前退出");
                }

                try
                {
                    using (var client = new TcpClient())
                    {
                        var result = client.BeginConnect(IPAddress.Loopback, WebUiUri.Port, null, null);
                        if (result.AsyncWaitHandle.WaitOne(250) && client.Connected)
                        {
                            client.EndConnect(result);
                            return;
                        }
                    }
                }
                catch (SocketException)
                {
                }

                Thread.Sleep(150);
            }

            Stop();
            throw new TimeoutException("等待 DeepSeek Harness 启动超时");
        }

        private void OnOutputDataReceived(object sender, DataReceivedEventArgs eventArgs)
        {
            if (!string.IsNullOrEmpty(eventArgs.Data))
            {
                OutputReceived?.Invoke(eventArgs.Data);
            }
        }

        private void OnProcessExited(object sender, EventArgs eventArgs)
        {
            bool wasStopping;
            bool wasReady;
            lock (syncRoot)
            {
                wasStopping = stopping;
                wasReady = ready;
                ready = false;
            }

            if (!wasStopping && wasReady)
            {
                UnexpectedlyExited?.Invoke(this, EventArgs.Empty);
            }
        }
    }
}
