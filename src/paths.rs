use std::env;
use std::path::PathBuf;

pub fn user_profile() -> Result<PathBuf, String> {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            Some(PathBuf::from(drive).join(path))
        })
        .ok_or_else(|| "无法确定当前用户目录".to_owned())
}

pub fn launcher_root() -> Result<PathBuf, String> {
    Ok(user_profile()?.join(".dsh").join("dsh-launcher"))
}

pub fn logs_directory() -> Result<PathBuf, String> {
    Ok(launcher_root()?.join("logs"))
}

pub fn state_file() -> Result<PathBuf, String> {
    Ok(launcher_root()?.join("state.json"))
}
