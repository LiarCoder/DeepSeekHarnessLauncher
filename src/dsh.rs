use crate::command::{find_command, run_capture, ResolvedCommand};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PACKAGE_NAME: &str = "@deepseek-ai/dsh";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageManagerKind {
    Volta,
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Cnpm,
}

#[derive(Clone, Debug)]
pub struct PackageManager {
    pub kind: PackageManagerKind,
    pub command: ResolvedCommand,
}

#[derive(Clone, Debug)]
pub struct DshInstallation {
    pub dsh: ResolvedCommand,
    pub manager: PackageManager,
}

impl PackageManagerKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Volta => "Volta",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "Yarn",
            Self::Bun => "Bun",
            Self::Cnpm => "cnpm",
        }
    }

    fn command_name(self) -> &'static str {
        match self {
            Self::Volta => "volta",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
            Self::Cnpm => "cnpm",
        }
    }

    pub fn update_args(self) -> Vec<String> {
        match self {
            Self::Volta => vec!["install".into(), format!("{PACKAGE_NAME}@latest")],
            Self::Npm | Self::Cnpm => vec![
                "install".into(),
                "-g".into(),
                format!("{PACKAGE_NAME}@latest"),
            ],
            Self::Pnpm | Self::Bun => {
                vec!["add".into(), "-g".into(), format!("{PACKAGE_NAME}@latest")]
            }
            Self::Yarn => vec![
                "global".into(),
                "add".into(),
                format!("{PACKAGE_NAME}@latest"),
            ],
        }
    }
}

impl PackageManager {
    pub fn display_name(&self) -> &'static str {
        self.kind.display_name()
    }

    pub fn detect(dsh: &ResolvedCommand) -> Result<Self, String> {
        let mut available = Vec::new();
        for kind in [
            PackageManagerKind::Volta,
            PackageManagerKind::Npm,
            PackageManagerKind::Pnpm,
            PackageManagerKind::Yarn,
            PackageManagerKind::Bun,
            PackageManagerKind::Cnpm,
        ] {
            let Some(command) = find_command(kind.command_name()) else {
                continue;
            };
            let manager = Self { kind, command };
            if manager.owns_dsh(dsh) {
                return Ok(manager);
            }
            available.push(kind.display_name());
        }

        let installed = if available.is_empty() {
            "未找到可用的全局包管理器".to_owned()
        } else {
            format!("已找到但无法确认归属：{}", available.join("、"))
        };
        Err(format!("无法确认 dsh 的安装来源，{installed}"))
    }

    pub fn run_update(&self, timeout: Duration) -> Result<String, String> {
        let output = run_capture(&self.command, &self.kind.update_args(), timeout)?;
        let combined = format!("{}\n{}", output.stdout.trim(), output.stderr.trim());
        if !output.status.success() {
            return Err(format!(
                "{} 更新失败：{}",
                self.display_name(),
                combined.trim()
            ));
        }
        Ok(combined.trim().to_owned())
    }

    fn owns_dsh(&self, dsh: &ResolvedCommand) -> bool {
        match self.kind {
            PackageManagerKind::Volta => self.volta_owns_dsh(),
            PackageManagerKind::Npm | PackageManagerKind::Pnpm | PackageManagerKind::Cnpm => {
                self.node_manager_owns_dsh(dsh)
            }
            PackageManagerKind::Yarn => self.yarn_owns_dsh(dsh),
            PackageManagerKind::Bun => self.bun_owns_dsh(dsh),
        }
    }

    fn volta_owns_dsh(&self) -> bool {
        let output = run_capture(
            &self.command,
            &["which".into(), "dsh".into()],
            Duration::from_secs(5),
        );
        let Ok(output) = output else {
            return false;
        };
        output.status.success()
            && output.stdout.to_ascii_lowercase().contains("@deepseek-ai")
            && output.stdout.to_ascii_lowercase().contains("dsh")
    }

    fn node_manager_owns_dsh(&self, dsh: &ResolvedCommand) -> bool {
        let root_args = match self.kind {
            PackageManagerKind::Npm => vec!["root".into(), "-g".into()],
            PackageManagerKind::Pnpm => vec!["root".into(), "-g".into()],
            PackageManagerKind::Cnpm => vec!["root".into(), "-g".into()],
            _ => return false,
        };
        let Ok(output) = run_capture(&self.command, &root_args, Duration::from_secs(5)) else {
            return false;
        };
        if !output.status.success() {
            return false;
        }

        let Some(root) = output_path(&output.stdout) else {
            return false;
        };
        let package_root = root.join("@deepseek-ai").join("dsh");
        if !package_root.exists() {
            return false;
        }

        let bin_args = match self.kind {
            PackageManagerKind::Npm | PackageManagerKind::Cnpm => {
                vec!["prefix".into(), "-g".into()]
            }
            PackageManagerKind::Pnpm => vec!["bin".into(), "-g".into()],
            _ => return false,
        };
        let Ok(output) = run_capture(&self.command, &bin_args, Duration::from_secs(5)) else {
            return false;
        };
        let bin = output_path(&output.stdout).or_else(|| {
            if self.kind != PackageManagerKind::Pnpm {
                return None;
            }
            pnpm_global_bin(&root)
        });
        let Some(bin) = bin else {
            return false;
        };

        if command_is_directly_under(dsh, &bin) {
            return true;
        }

        if self.kind != PackageManagerKind::Pnpm {
            return false;
        }

        let config_args = vec!["config".into(), "get".into(), "global-bin-dir".into()];
        let Ok(output) = run_capture(&self.command, &config_args, Duration::from_secs(5)) else {
            return false;
        };
        let Some(configured_bin) = output_path(&output.stdout) else {
            return false;
        };
        command_is_directly_under(dsh, &configured_bin)
    }

    fn yarn_owns_dsh(&self, dsh: &ResolvedCommand) -> bool {
        let Ok(output) = run_capture(
            &self.command,
            &["global".into(), "dir".into()],
            Duration::from_secs(5),
        ) else {
            return false;
        };
        if !output.status.success() {
            return false;
        }
        let Some(directory) = output_path(&output.stdout) else {
            return false;
        };
        if !directory
            .join("node_modules")
            .join("@deepseek-ai")
            .join("dsh")
            .exists()
        {
            return false;
        }

        let Ok(output) = run_capture(
            &self.command,
            &["global".into(), "bin".into()],
            Duration::from_secs(5),
        ) else {
            return false;
        };
        if !output.status.success() {
            return false;
        }

        let Some(bin) = output_path(&output.stdout) else {
            return false;
        };
        command_is_directly_under(dsh, &bin)
    }

    fn bun_owns_dsh(&self, dsh: &ResolvedCommand) -> bool {
        let Ok(output) = run_capture(
            &self.command,
            &["pm".into(), "ls".into(), "-g".into()],
            Duration::from_secs(5),
        ) else {
            return false;
        };
        if !output.status.success() || !output.stdout.contains(PACKAGE_NAME) {
            return false;
        }

        let bin = run_capture(
            &self.command,
            &["pm".into(), "bin".into(), "-g".into()],
            Duration::from_secs(5),
        )
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| output_path(&output.stdout))
        .or_else(bun_global_bin);
        let Some(bin) = bin else {
            return false;
        };
        command_is_directly_under(dsh, &bin)
    }
}

pub fn locate() -> Result<ResolvedCommand, String> {
    find_command("dsh").ok_or_else(|| {
        "未找到 dsh，请先通过任意包管理器全局安装 @deepseek-ai/dsh，并确保命令在 PATH 中".to_owned()
    })
}

pub fn locate_installation() -> Result<DshInstallation, String> {
    let dsh = locate()?;
    let manager = PackageManager::detect(&dsh)?;
    Ok(DshInstallation { dsh, manager })
}

pub fn version(dsh: &ResolvedCommand) -> Result<String, String> {
    let output = run_capture(dsh, &["--version".into()], Duration::from_secs(15))?;
    if !output.status.success() {
        return Err(format!("读取 dsh 版本失败：{}", output.stderr.trim()));
    }
    output
        .stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "dsh 未返回版本号".to_owned())
}

fn command_is_directly_under(command: &ResolvedCommand, directory: &Path) -> bool {
    let command = std::fs::canonicalize(&command.path).unwrap_or_else(|_| command.path.clone());
    let directory = std::fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
    command
        .parent()
        .is_some_and(|parent| paths_equal(parent, &directory))
}

fn output_path(output: &str) -> Option<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.eq_ignore_ascii_case("undefined"))
        .last()
        .map(PathBuf::from)
}

fn pnpm_global_bin(root: &Path) -> Option<PathBuf> {
    root.parent()?.parent()?.parent().map(Path::to_path_buf)
}

fn bun_global_bin() -> Option<PathBuf> {
    if let Some(install) = std::env::var_os("BUN_INSTALL") {
        return Some(PathBuf::from(install).join("bin"));
    }
    crate::paths::user_profile()
        .ok()
        .map(|profile| profile.join(".bun").join("bin"))
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{output_path, PackageManagerKind};

    #[test]
    fn manager_update_commands_use_global_installation() {
        assert_eq!(
            PackageManagerKind::Volta.update_args(),
            vec!["install", "@deepseek-ai/dsh@latest"]
        );
        assert_eq!(
            PackageManagerKind::Npm.update_args(),
            vec!["install", "-g", "@deepseek-ai/dsh@latest"]
        );
        assert_eq!(
            PackageManagerKind::Yarn.update_args(),
            vec!["global", "add", "@deepseek-ai/dsh@latest"]
        );
    }

    #[test]
    fn package_manager_output_ignores_empty_lines() {
        assert_eq!(
            output_path("\r\nC:\\Users\\Liar\\.bun\\bin\r\n"),
            Some(std::path::PathBuf::from("C:\\Users\\Liar\\.bun\\bin"))
        );
        assert_eq!(output_path("undefined\r\n"), None);
    }
}
