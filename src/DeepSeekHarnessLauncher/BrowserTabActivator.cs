using System;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Automation;

namespace DeepSeekHarnessLauncher
{
    internal static class BrowserTabActivator
    {
        private const string WebUiTitle = "DeepSeek Harness";
        private const int RestoreWindow = 9;

        public static bool TryActivate(Uri webUiUri)
        {
            try
            {
                var windows = AutomationElement.RootElement.FindAll(TreeScope.Children, Condition.TrueCondition);
                foreach (AutomationElement window in windows)
                {
                    if (!IsBrowserWindow(window.Current.ClassName))
                    {
                        continue;
                    }

                    if (IsWebUiTitle(window.Current.Name))
                    {
                        ActivateWindow(window);
                        return true;
                    }

                    var tabs = window.FindAll(
                        TreeScope.Descendants,
                        new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.TabItem));
                    foreach (AutomationElement tab in tabs)
                    {
                        if (!IsWebUiTitle(tab.Current.Name))
                        {
                            continue;
                        }

                        var previouslySelectedTab = FindSelectedTab(tabs);
                        if (!SelectTab(tab) || !HasMatchingAddress(window, webUiUri))
                        {
                            SelectTab(previouslySelectedTab);
                            continue;
                        }

                        ActivateWindow(window);
                        return true;
                    }
                }
            }
            catch (ElementNotAvailableException)
            {
            }
            catch (InvalidOperationException)
            {
            }
            catch (COMException)
            {
            }

            return false;
        }

        private static bool IsBrowserWindow(string className)
        {
            return string.Equals(className, "Chrome_WidgetWin_1", StringComparison.Ordinal) ||
                   string.Equals(className, "MozillaWindowClass", StringComparison.Ordinal);
        }

        private static bool IsWebUiTitle(string title)
        {
            return !string.IsNullOrEmpty(title) &&
                   title.IndexOf(WebUiTitle, StringComparison.OrdinalIgnoreCase) >= 0;
        }

        private static AutomationElement FindSelectedTab(AutomationElementCollection tabs)
        {
            foreach (AutomationElement tab in tabs)
            {
                if (tab.TryGetCurrentPattern(SelectionItemPattern.Pattern, out var pattern) &&
                    ((SelectionItemPattern)pattern).Current.IsSelected)
                {
                    return tab;
                }
            }

            return null;
        }

        private static bool SelectTab(AutomationElement tab)
        {
            if (tab == null || !tab.TryGetCurrentPattern(SelectionItemPattern.Pattern, out var pattern))
            {
                return false;
            }

            ((SelectionItemPattern)pattern).Select();
            return true;
        }

        private static bool HasMatchingAddress(AutomationElement window, Uri webUiUri)
        {
            for (var attempt = 0; attempt < 3; attempt++)
            {
                var editControls = window.FindAll(
                    TreeScope.Descendants,
                    new PropertyCondition(AutomationElement.ControlTypeProperty, ControlType.Edit));
                foreach (AutomationElement editControl in editControls)
                {
                    if (!editControl.TryGetCurrentPattern(ValuePattern.Pattern, out var pattern))
                    {
                        continue;
                    }

                    var address = ((ValuePattern)pattern).Current.Value;
                    if (IsSameOrigin(address, webUiUri))
                    {
                        return true;
                    }
                }

                Thread.Sleep(50);
            }

            return false;
        }

        private static bool IsSameOrigin(string address, Uri webUiUri)
        {
            if (string.IsNullOrWhiteSpace(address))
            {
                return false;
            }

            if (!Uri.TryCreate(address, UriKind.Absolute, out var addressUri))
            {
                Uri.TryCreate(webUiUri.Scheme + "://" + address, UriKind.Absolute, out addressUri);
            }

            return addressUri != null &&
                   string.Equals(addressUri.Scheme, webUiUri.Scheme, StringComparison.OrdinalIgnoreCase) &&
                   string.Equals(addressUri.Host, webUiUri.Host, StringComparison.OrdinalIgnoreCase) &&
                   addressUri.Port == webUiUri.Port;
        }

        private static void ActivateWindow(AutomationElement window)
        {
            var windowHandle = new IntPtr(window.Current.NativeWindowHandle);
            if (IsIconic(windowHandle))
            {
                ShowWindow(windowHandle, RestoreWindow);
            }

            window.SetFocus();
            SetForegroundWindow(windowHandle);
        }

        [DllImport("user32.dll")]
        private static extern bool IsIconic(IntPtr windowHandle);

        [DllImport("user32.dll")]
        private static extern bool SetForegroundWindow(IntPtr windowHandle);

        [DllImport("user32.dll")]
        private static extern bool ShowWindow(IntPtr windowHandle, int command);
    }
}
