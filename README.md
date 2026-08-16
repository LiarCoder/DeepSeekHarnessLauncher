# DeepSeek Harness Launcher

一个面向 Windows 的轻量 DeepSeek Harness 托盘启动器

## 环境要求

- Windows 10 或更高版本
- .NET Framework 4.8
- 由 Volta 管理并全局安装的 `@deepseek-ai/dsh`

## 构建

使用 Visual Studio 2022 或 MSBuild 构建：

```powershell
msbuild DeepSeekHarnessLauncher.sln -property:Configuration=Release
```

