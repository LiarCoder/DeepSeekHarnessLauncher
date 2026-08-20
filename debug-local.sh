#!/usr/bin/env bash

set -euo pipefail

repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
binary="$repo_dir/target/debug/deepseek-harness-launcher.exe"

echo "Stopping running launcher instances..."
MSYS_NO_PATHCONV=1 taskkill.exe /F /T /IM deepseek-harness-launcher.exe >/dev/null 2>&1 || true
MSYS_NO_PATHCONV=1 taskkill.exe /F /T /IM DeepSeekHarnessLauncher.exe >/dev/null 2>&1 || true

echo "Building latest debug binary..."
cargo build --manifest-path "$repo_dir/Cargo.toml"

if [[ ! -f "$binary" ]]; then
    echo "Build succeeded but binary was not found: $binary" >&2
    exit 1
fi

binary_windows="$(cygpath -w "$binary")"
echo "Starting $binary_windows"
DSH_LAUNCHER_EXE="$binary_windows" powershell.exe -NoProfile -NonInteractive \
    -Command 'Start-Process -FilePath $env:DSH_LAUNCHER_EXE'
