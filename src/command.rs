use std::ffi::OsString;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    pub path: PathBuf,
}

impl ResolvedCommand {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn is_shell_script(&self) -> bool {
        matches!(
            self.path.extension().and_then(|extension| extension.to_str()),
            Some(extension) if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
        )
    }
}

pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub fn find_command(name: &str) -> Option<ResolvedCommand> {
    let path = std::env::var_os("PATH")?;
    let candidates = command_candidates(name);

    for directory in std::env::split_paths(&path) {
        for candidate in &candidates {
            let path = directory.join(candidate);
            if path.is_file() {
                return Some(ResolvedCommand::new(path));
            }
        }
    }

    None
}

pub fn build_command(command: &ResolvedCommand, args: &[String]) -> Command {
    let mut process = if command.is_shell_script() {
        let command_line = build_shell_command_line(&command.path, args);
        let mut process = Command::new(comspec());
        process.args(["/D", "/S", "/C", &command_line]);
        process
    } else {
        let mut process = Command::new(&command.path);
        process.args(args);
        process
    };

    process.creation_flags(0x0800_0000);
    process
}

pub fn run_capture(
    command: &ResolvedCommand,
    args: &[String],
    timeout: Duration,
) -> Result<CommandOutput, String> {
    let mut child = build_command(command, args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("启动 {} 失败：{error}", command.path.display()))?;

    let stdout = child.stdout.take().map(|mut stream| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stream.read_to_end(&mut bytes);
            (result, bytes)
        })
    });
    let stderr = child.stderr.take().map(|mut stream| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = stream.read_to_end(&mut bytes);
            (result, bytes)
        })
    });

    let started_at = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started_at.elapsed() < timeout => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("命令执行超时：{}", command.path.display()));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("等待命令结束失败：{error}"));
            }
        }
    };

    let stdout = join_output(stdout)?;
    let stderr = join_output(stderr)?;
    Ok(CommandOutput {
        status,
        stdout,
        stderr,
    })
}

pub fn taskkill(process_id: u32) {
    let mut command = Command::new("taskkill.exe");
    command
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .creation_flags(0x0800_0000)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Ok(mut child) = command.spawn() {
        let _ = child.wait();
    }
}

fn join_output(
    handle: Option<thread::JoinHandle<(std::io::Result<usize>, Vec<u8>)>>,
) -> Result<String, String> {
    let Some(handle) = handle else {
        return Ok(String::new());
    };
    let (result, bytes) = handle
        .join()
        .map_err(|_| "读取命令输出线程异常退出".to_owned())?;
    result.map_err(|error| format!("读取命令输出失败：{error}"))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn command_candidates(name: &str) -> Vec<OsString> {
    if Path::new(name).extension().is_some() {
        return vec![OsString::from(name)];
    }

    [".exe", ".cmd", ".bat", ".com", ""]
        .into_iter()
        .map(|extension| OsString::from(format!("{name}{extension}")))
        .collect()
}

fn comspec() -> OsString {
    std::env::var_os("ComSpec").unwrap_or_else(|| OsString::from("cmd.exe"))
}

fn build_shell_command_line(path: &Path, args: &[String]) -> String {
    let mut command_line = format!("\"{}\"", path.display());
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&quote_shell_argument(arg));
    }
    format!("\"{command_line}\"")
}

fn quote_shell_argument(argument: &str) -> String {
    if !argument
        .chars()
        .any(|character| character.is_whitespace() || character == '"')
    {
        return argument.to_owned();
    }

    format!("\"{}\"", argument.replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::{command_candidates, quote_shell_argument};

    #[test]
    fn command_candidates_prefer_windows_launchers() {
        let candidates = command_candidates("dsh");
        assert_eq!(candidates[0], "dsh.exe");
        assert_eq!(candidates[1], "dsh.cmd");
        assert_eq!(candidates[2], "dsh.bat");
    }

    #[test]
    fn shell_arguments_are_quoted_when_needed() {
        assert_eq!(quote_shell_argument("plain"), "plain");
        assert_eq!(quote_shell_argument("with space"), "\"with space\"");
    }
}
