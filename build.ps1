param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release'
)

$ErrorActionPreference = 'Stop'
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw '未找到 Visual Studio Installer，请安装 Visual Studio 2022 Build Tools'
}

$msbuild = & $vswhere -latest -products * -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' |
    Select-Object -First 1
if (-not $msbuild) {
    throw '未找到 MSBuild，请安装 Visual Studio 2022 Build Tools'
}

& $msbuild "$PSScriptRoot\DeepSeekHarnessLauncher.sln" /restore /t:Build "/p:Configuration=$Configuration" /v:minimal
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Output "输出文件：$PSScriptRoot\src\DeepSeekHarnessLauncher\bin\$Configuration\DeepSeekHarnessLauncher.exe"
