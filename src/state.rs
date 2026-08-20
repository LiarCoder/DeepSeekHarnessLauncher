use crate::paths;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct LauncherState {
    #[serde(rename = "harnessPid")]
    pub harness_pid: u32,
    #[serde(rename = "webUiUrl")]
    pub web_ui_url: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

pub fn save(process_id: u32, web_ui_url: &str) -> Result<(), String> {
    let root = paths::launcher_root()?;
    fs::create_dir_all(&root).map_err(|error| format!("创建状态目录失败：{error}"))?;
    let file =
        File::create(paths::state_file()?).map_err(|error| format!("创建状态文件失败：{error}"))?;
    let state = LauncherState {
        harness_pid: process_id,
        web_ui_url: web_ui_url.to_owned(),
        updated_at: format!("{}", chrono_stamp()),
    };
    serde_json::to_writer(file, &state).map_err(|error| format!("写入状态文件失败：{error}"))
}

pub fn clear() {
    if let Ok(path) = paths::state_file() {
        remove_file_quietly(&path);
    }
}

fn remove_file_quietly(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

fn chrono_stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
