param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    throw '未找到 cargo，请安装 Rust 工具链'
}

$arguments = @('build')
if ($Configuration -eq 'Release') {
    $arguments += '--release'
}

Push-Location $PSScriptRoot
try {
    & $cargo.Source @arguments
}
finally {
    Pop-Location
}
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$outputDirectory = if ($Configuration -eq 'Release') { 'release' } else { 'debug' }
$output = Join-Path $PSScriptRoot "target\$outputDirectory\deepseek-harness-launcher.exe"
$limit = 5MB
if (-not (Test-Path -LiteralPath $output)) {
    throw "未找到构建产物：$output"
}
$size = (Get-Item -LiteralPath $output).Length
if ($size -gt $limit) {
    throw "构建产物超过 5 MB：$size bytes"
}

Write-Output "输出文件：$output"
Write-Output "文件大小：$size bytes"
