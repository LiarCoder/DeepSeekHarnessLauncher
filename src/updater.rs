use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const UPDATE_SCRIPT: &str = r#"@echo off
setlocal
:wait_for_launcher
tasklist /FI "PID eq %DSH_LAUNCHER_UPDATE_PARENT_PID%" /NH | findstr /R /C:" %DSH_LAUNCHER_UPDATE_PARENT_PID% " >nul
if not errorlevel 1 (
    timeout /t 1 /nobreak >nul
    goto wait_for_launcher
)

set /A attempts=0
:replace_launcher
move /Y "%DSH_LAUNCHER_UPDATE_SOURCE%" "%DSH_LAUNCHER_UPDATE_TARGET%" >nul
if not errorlevel 1 goto restart_launcher
set /A attempts+=1
if %attempts% GEQ 30 goto restore_launcher
timeout /t 1 /nobreak >nul
goto replace_launcher

:restore_launcher
start "" "%DSH_LAUNCHER_UPDATE_TARGET%"
del "%DSH_LAUNCHER_UPDATE_SOURCE%" >nul 2>nul
del "%~f0" >nul 2>nul
exit /b 1

:restart_launcher
start "" "%DSH_LAUNCHER_UPDATE_TARGET%"
del "%~f0" >nul 2>nul
exit /b 0
"#;

pub fn save_download(bytes: &[u8]) -> Result<PathBuf, String> {
    validate_download(bytes)?;
    let path = unique_path(
        &env::temp_dir(),
        "deepseek-harness-launcher-download",
        "exe",
    );
    write_new_file(&path, bytes).map_err(|error| {
        let _ = fs::remove_file(&path);
        format!("保存 Launcher 更新文件失败：{error}")
    })?;
    Ok(path)
}

pub fn install_and_restart(download_path: &Path) -> Result<(), String> {
    if !download_path.is_file() {
        return Err("找不到已下载的 Launcher 更新文件".to_owned());
    }

    let target = env::current_exe().map_err(|error| format!("获取 Launcher 路径失败：{error}"))?;
    let target_directory = target
        .parent()
        .ok_or_else(|| "无法确定 Launcher 安装目录".to_owned())?;
    if !target_directory.is_dir() {
        return Err("Launcher 安装目录不存在".to_owned());
    }

    let staged_path = unique_path(target_directory, ".deepseek-harness-launcher-update", "exe");
    if let Err(error) = fs::copy(download_path, &staged_path) {
        let _ = fs::remove_file(&staged_path);
        return Err(format!("无法写入 Launcher 安装目录：{error}"));
    }

    if let Err(error) = spawn_replacement(&staged_path, &target) {
        let _ = fs::remove_file(&staged_path);
        return Err(error);
    }

    let _ = fs::remove_file(download_path);
    Ok(())
}

fn spawn_replacement(staged_path: &Path, target: &Path) -> Result<(), String> {
    let script_path = unique_path(&env::temp_dir(), "deepseek-harness-launcher-updater", "cmd");
    write_new_file(&script_path, UPDATE_SCRIPT.as_bytes()).map_err(|error| {
        let _ = fs::remove_file(&script_path);
        format!("创建 Launcher 更新程序失败：{error}")
    })?;

    let command_path = env::var_os("ComSpec").unwrap_or_else(|| "cmd.exe".into());
    let mut command = Command::new(command_path);
    command
        .args(["/D", "/C"])
        .raw_arg(format!("\"{}\"", script_path.display()))
        .env(
            "DSH_LAUNCHER_UPDATE_PARENT_PID",
            std::process::id().to_string(),
        )
        .env("DSH_LAUNCHER_UPDATE_SOURCE", staged_path)
        .env("DSH_LAUNCHER_UPDATE_TARGET", target)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    command.spawn().map_err(|error| {
        let _ = fs::remove_file(&script_path);
        format!("启动 Launcher 更新程序失败：{error}")
    })?;
    Ok(())
}

fn validate_download(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 2 || &bytes[..2] != b"MZ" {
        return Err("下载的 Launcher 文件格式无效".to_owned());
    }
    Ok(())
}

fn write_new_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    if let Err(error) = file.write_all(contents).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

fn unique_path(directory: &Path, prefix: &str, extension: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let base = format!("{prefix}-{}-{timestamp}", std::process::id());
    let mut path = directory.join(format!("{base}.{extension}"));
    let mut suffix = 0u32;
    while path.exists() {
        suffix = suffix.saturating_add(1);
        path = directory.join(format!("{base}-{suffix}.{extension}"));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::{validate_download, UPDATE_SCRIPT};

    #[test]
    fn accepts_windows_executable_signature() {
        validate_download(b"MZ").expect("MZ signature should be accepted");
    }

    #[test]
    fn rejects_non_executable_download() {
        assert_eq!(
            validate_download(b"not an executable").expect_err("invalid download should fail"),
            "下载的 Launcher 文件格式无效"
        );
    }

    #[test]
    fn update_script_waits_retries_and_restarts() {
        assert!(UPDATE_SCRIPT.contains(":wait_for_launcher"));
        assert!(UPDATE_SCRIPT.contains("GEQ 30"));
        assert!(UPDATE_SCRIPT.contains("start \"\" \"%DSH_LAUNCHER_UPDATE_TARGET%\""));
    }
}
