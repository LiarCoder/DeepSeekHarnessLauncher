use crate::paths;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MAXIMUM_LOG_FILES: usize = 10;

pub struct Logger {
    writer: Mutex<File>,
    current_log_file: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self, String> {
        let directory = paths::logs_directory()?;
        fs::create_dir_all(&directory).map_err(|error| format!("创建日志目录失败：{error}"))?;

        let current_log_file = directory.join(format!("launcher-{}.log", log_stamp()));
        let writer = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&current_log_file)
            .map_err(|error| format!("创建日志文件失败：{error}"))?;

        delete_old_logs(&directory);

        Ok(Self {
            writer: Mutex::new(writer),
            current_log_file,
        })
    }

    pub fn current_log_file(&self) -> &PathBuf {
        &self.current_log_file
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.write("INFO", message.as_ref());
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.write("ERROR", message.as_ref());
    }

    pub fn harness(&self, message: impl AsRef<str>) {
        self.write("HARNESS", message.as_ref());
    }

    fn write(&self, level: &str, message: &str) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writeln!(writer, "{} [{}] {}", log_timestamp(), level, message);
            let _ = writer.flush();
        }
    }
}

fn delete_old_logs(directory: &PathBuf) {
    let mut files = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with("launcher-")
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "log")
            })
            .filter_map(|entry| {
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok()?;
                Some((modified, entry.path()))
            })
            .collect::<Vec<_>>(),
        Err(_) => return,
    };

    files.sort_by(|left, right| right.0.cmp(&left.0));
    for (_, path) in files.into_iter().skip(MAXIMUM_LOG_FILES) {
        let _ = fs::remove_file(path);
    }
}

fn log_stamp() -> String {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    milliseconds.to_string()
}

fn log_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", duration.as_secs(), duration.subsec_millis())
}
