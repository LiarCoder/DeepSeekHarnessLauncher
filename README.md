# DeepSeek Harness Launcher

一个面向 Windows 的轻量 DeepSeek Harness 托盘启动器，最终产物是单个原生 Rust 可执行文件

## 功能

- 无终端窗口地启动 PATH 中的全局 `dsh`
- 优先使用 `3080`，端口冲突时自动选择空闲端口
- 从托盘打开 Web UI、重启 Harness、查看日志，并分别检查 dsh 与 Launcher 更新
- 检查更新前识别 `dsh` 的实际安装来源，并使用同一个全局包管理器更新
- Launcher 更新信息从 GitHub Releases 获取，发现新版本时自动下载、替换并重启启动器
- Harness 异常退出后自动重启一次
- 退出启动器时终止 Harness 进程树
- 日志与运行状态保存在 `~\.dsh\dsh-launcher`

## 环境要求

- Windows 10 或更高版本
- 全局安装并可在 PATH 中直接调用的 `dsh`
- 如需使用自动更新，安装 `dsh` 的包管理器也必须在 PATH 中

启动器本身不要求安装 Volta，也不绑定某个特定包管理器。更新检测支持 Volta、npm、pnpm、Yarn、Bun 和 cnpm；只有能够确认 `dsh` 归属时才会执行更新，避免调用错误的包管理器

例如可以使用任意一种方式安装全局 `dsh`：

```powershell
npm install -g @deepseek-ai/dsh
pnpm add -g @deepseek-ai/dsh
yarn global add @deepseek-ai/dsh
bun add -g @deepseek-ai/dsh
volta install @deepseek-ai/dsh
```

## 构建

使用 Rust 工具链构建：

```powershell
.\build.ps1
```

也可以直接执行：

```powershell
cargo fmt --all -- --check
cargo test
cargo build --release
```

发布文件位于 `target\release\deepseek-harness-launcher.exe`，默认使用体积优化和静态 MSVC CRT，发布构建会检查单文件体积上限

本地修改代码后，可在 Git Bash 中运行以下脚本。它会关闭当前启动器及其 Harness 子进程，构建最新 Debug 产物，然后重新启动应用：

```bash
./debug-local.sh
```

## 运行时目录

- 日志：`%USERPROFILE%\.dsh\dsh-launcher\logs`
- 运行状态：`%USERPROFILE%\.dsh\dsh-launcher\state.json`

## 发布

推送 `v*` 标签后，GitHub Actions 会在 Windows runner 上执行测试、构建并创建 GitHub Release，Release 中只上传 `deepseek-harness-launcher.exe`
