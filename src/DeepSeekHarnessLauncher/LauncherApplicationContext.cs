using System;
using System.Diagnostics;
using System.Drawing;
using System.Threading;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace DeepSeekHarnessLauncher
{
    internal sealed class LauncherApplicationContext : ApplicationContext
    {
        private readonly EventWaitHandle openWebUiSignal;
        private readonly HarnessProcessManager harness = new HarnessProcessManager();
        private readonly FileLogger logger = new FileLogger();
        private readonly NotifyIcon trayIcon;
        private readonly Control dispatcher;
        private readonly ToolStripMenuItem openMenuItem;
        private readonly ToolStripMenuItem restartMenuItem;
        private readonly System.Windows.Forms.Timer signalTimer;
        private bool operationInProgress;
        private bool automaticRestartUsed;
        private bool exiting;

        public LauncherApplicationContext(EventWaitHandle openWebUiSignal)
        {
            this.openWebUiSignal = openWebUiSignal;
            dispatcher = new Control();
            dispatcher.CreateControl();
            harness.UnexpectedlyExited += OnHarnessUnexpectedlyExited;
            harness.OutputReceived += logger.Harness;

            openMenuItem = new ToolStripMenuItem("打开 Web UI", null, (sender, args) => OpenWebUi());
            restartMenuItem = new ToolStripMenuItem("重启 Harness", null, async (sender, args) => await RestartHarnessAsync(false));
            var logsMenuItem = new ToolStripMenuItem("查看日志", null, (sender, args) => OpenCurrentLog());
            var versionMenuItem = new ToolStripMenuItem("v… - 检查更新") { Enabled = false };
            var exitMenuItem = new ToolStripMenuItem("退出", null, async (sender, args) => await ExitAsync());

            var menu = new ContextMenuStrip();
            menu.Items.AddRange(new ToolStripItem[]
            {
                openMenuItem,
                restartMenuItem,
                logsMenuItem,
                new ToolStripSeparator(),
                versionMenuItem,
                new ToolStripSeparator(),
                exitMenuItem
            });

            trayIcon = new NotifyIcon
            {
                ContextMenuStrip = menu,
                Icon = SystemIcons.Application,
                Text = "DeepSeek Harness Launcher",
                Visible = true
            };
            trayIcon.DoubleClick += (sender, args) => OpenWebUi();

            signalTimer = new System.Windows.Forms.Timer { Interval = 250 };
            signalTimer.Tick += (sender, args) =>
            {
                if (this.openWebUiSignal.WaitOne(0))
                {
                    OpenWebUi();
                }
            };
            signalTimer.Start();

            Application.Idle += OnFirstIdle;
            logger.Info("启动器已启动");
        }

        private async void OnFirstIdle(object sender, EventArgs eventArgs)
        {
            Application.Idle -= OnFirstIdle;
            await StartHarnessAsync(true, true);
        }

        private async Task StartHarnessAsync(bool openBrowser, bool resetRestartBudget)
        {
            if (operationInProgress || exiting)
            {
                return;
            }

            operationInProgress = true;
            UpdateMenuState();
            if (resetRestartBudget)
            {
                automaticRestartUsed = false;
            }

            try
            {
                logger.Info("正在启动 DeepSeek Harness");
                await Task.Run(() => harness.StartAndWait(TimeSpan.FromSeconds(30)));
                LauncherStateStore.Save(harness.ProcessId, harness.WebUiUri);
                logger.Info("DeepSeek Harness 已就绪：" + harness.WebUiUri.AbsoluteUri);
                if (openBrowser)
                {
                    OpenWebUi();
                }
            }
            catch (Exception exception)
            {
                LauncherStateStore.Clear();
                logger.Error("DeepSeek Harness 启动失败：" + exception);
                MessageBox.Show(
                    "DeepSeek Harness 启动失败：\r\n\r\n" + exception.Message,
                    "DeepSeek Harness Launcher",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }
            finally
            {
                operationInProgress = false;
                UpdateMenuState();
            }
        }

        private async Task RestartHarnessAsync(bool openBrowser)
        {
            if (operationInProgress || exiting)
            {
                return;
            }

            operationInProgress = true;
            UpdateMenuState();
            try
            {
                logger.Info("正在停止 DeepSeek Harness");
                await Task.Run(() => harness.Stop());
                LauncherStateStore.Clear();
            }
            finally
            {
                operationInProgress = false;
            }

            await StartHarnessAsync(openBrowser, true);
        }

        private void OpenWebUi()
        {
            if (harness.WebUiUri == null || !harness.IsRunning)
            {
                return;
            }

            try
            {
                Process.Start(new ProcessStartInfo(harness.WebUiUri.AbsoluteUri) { UseShellExecute = true });
            }
            catch (Exception exception)
            {
                MessageBox.Show(
                    "无法打开默认浏览器：\r\n\r\n" + exception.Message,
                    "DeepSeek Harness Launcher",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }
        }

        private void OnHarnessUnexpectedlyExited(object sender, EventArgs eventArgs)
        {
            dispatcher.BeginInvoke(new Action(HandleHarnessUnexpectedExit));
        }

        private async void HandleHarnessUnexpectedExit()
        {
            if (exiting)
            {
                return;
            }

            if (!automaticRestartUsed)
            {
                automaticRestartUsed = true;
                logger.Error("DeepSeek Harness 意外退出，准备自动重启一次");
                await StartHarnessAsync(false, false);
                return;
            }

            logger.Error("DeepSeek Harness 再次意外退出，停止自动重启");
            MessageBox.Show(
                "DeepSeek Harness 再次意外退出，启动器已停止自动重启。",
                "DeepSeek Harness Launcher",
                MessageBoxButtons.OK,
                MessageBoxIcon.Error);
        }

        private async Task ExitAsync()
        {
            if (exiting)
            {
                return;
            }

            exiting = true;
            signalTimer.Stop();
            trayIcon.Visible = false;
            logger.Info("正在退出启动器并停止 DeepSeek Harness");
            await Task.Run(() => harness.Dispose());
            LauncherStateStore.Clear();
            ExitThread();
        }

        private void OpenCurrentLog()
        {
            try
            {
                Process.Start(new ProcessStartInfo(logger.CurrentLogFile) { UseShellExecute = true });
            }
            catch (Exception exception)
            {
                logger.Error("无法打开日志文件：" + exception);
                MessageBox.Show(
                    "无法打开日志文件：\r\n\r\n" + exception.Message,
                    "DeepSeek Harness Launcher",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }
        }

        private void UpdateMenuState()
        {
            openMenuItem.Enabled = !operationInProgress && harness.IsRunning;
            restartMenuItem.Enabled = !operationInProgress;
        }

        protected override void Dispose(bool disposing)
        {
            if (disposing)
            {
                signalTimer.Dispose();
                trayIcon.Dispose();
                dispatcher.Dispose();
                harness.Dispose();
                logger.Dispose();
            }

            base.Dispose(disposing);
        }
    }
}
