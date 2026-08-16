# DeepSeek Harness Launcher

一个面向 Windows 的轻量 DeepSeek Harness 托盘启动器

## 功能

- 无终端窗口地启动由 Volta 管理的全局 `dsh`
- 优先使用 `3080`，端口冲突时自动选择空闲端口
- 从托盘打开 Web UI、重启 Harness、查看日志和检查更新
- Harness 异常退出后自动重启一次
- 退出启动器时终止 Harness 进程树
- 日志与运行状态保存在 `~\.dsh\dsh-launcher`

## 环境要求

- Windows 10 或更高版本
- .NET Framework 4.8
- 由 Volta 管理并全局安装的 `@deepseek-ai/dsh`

## 构建

使用 Visual Studio 2022 Build Tools 构建：

```powershell
.\build.ps1
```

发布文件位于 `src\DeepSeekHarnessLauncher\bin\Release\DeepSeekHarnessLauncher.exe`
