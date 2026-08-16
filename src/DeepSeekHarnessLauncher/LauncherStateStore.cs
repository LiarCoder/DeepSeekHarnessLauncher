using System;
using System.IO;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;

namespace DeepSeekHarnessLauncher
{
    [DataContract]
    internal sealed class LauncherState
    {
        [DataMember(Name = "harnessPid")]
        public int HarnessProcessId { get; set; }

        [DataMember(Name = "webUiUrl")]
        public string WebUiUrl { get; set; }

        [DataMember(Name = "updatedAt")]
        public string UpdatedAt { get; set; }
    }

    internal static class LauncherStateStore
    {
        public static void Save(int processId, Uri webUiUri)
        {
            Directory.CreateDirectory(AppPaths.RootDirectory);
            var state = new LauncherState
            {
                HarnessProcessId = processId,
                WebUiUrl = webUiUri.AbsoluteUri,
                UpdatedAt = DateTimeOffset.Now.ToString("O")
            };

            using (var stream = File.Create(AppPaths.StateFile))
            {
                new DataContractJsonSerializer(typeof(LauncherState)).WriteObject(stream, state);
            }
        }

        public static void Clear()
        {
            try
            {
                if (File.Exists(AppPaths.StateFile))
                {
                    File.Delete(AppPaths.StateFile);
                }
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
