using System;
using System.IO;

namespace DeepSeekHarnessLauncher
{
    internal static class AppPaths
    {
        public static readonly string RootDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            ".dsh",
            "dsh-launcher");

        public static readonly string LogsDirectory = Path.Combine(RootDirectory, "logs");

        public static readonly string StateFile = Path.Combine(RootDirectory, "state.json");
    }
}
