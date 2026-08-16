using System;
using System.Threading;
using System.Windows.Forms;

namespace DeepSeekHarnessLauncher
{
    internal static class Program
    {
        [STAThread]
        private static void Main()
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);

            const string mutexName = @"Local\DeepSeekHarnessLauncher.SingleInstance";
            const string signalName = @"Local\DeepSeekHarnessLauncher.OpenWebUi";

            using (var openWebUiSignal = new EventWaitHandle(false, EventResetMode.AutoReset, signalName))
            using (var mutex = new Mutex(true, mutexName, out var isFirstInstance))
            {
                if (!isFirstInstance)
                {
                    openWebUiSignal.Set();
                    return;
                }

                Application.Run(new LauncherApplicationContext(openWebUiSignal));
            }
        }
    }
}
