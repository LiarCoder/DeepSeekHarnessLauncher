# AGENTS.md

## 项目说明

- 这是 Windows 原生 Rust 托盘启动器
- 启动器只依赖 PATH 中可调用的全局 `dsh`
- 更新前必须根据 `dsh` 命令的实际归属识别全局包管理器，并使用该包管理器更新
- 不要把 Volta、npm、pnpm、Yarn、Bun 或 cnpm 写成唯一运行前提

## 验证命令

```powershell
cargo fmt --all -- --check
cargo test
cargo build --release
```

发布构建必须保持单个 Windows EXE 且不超过 5 MB

## 变更约定

- 临时文件统一放在仓库根目录的 `.codex-tmp`
- 提交应按功能拆分，并遵循 Conventional Commits
- 修改 Windows API、进程树终止、包管理器识别或更新流程时，优先补充可自动执行的测试
