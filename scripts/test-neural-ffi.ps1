# SPDX-License-Identifier: AGPL-3.0-only
param([Parameter(Mandatory=$true)][string]$ModelPath)
$ErrorActionPreference = 'Stop'
$neuralModel = (Resolve-Path -LiteralPath $ModelPath).Path
$neuralOriginalTemp = $env:TEMP
$neuralOriginalTmp = $env:TMP
Push-Location (Join-Path $PSScriptRoot '..')
try {
    cargo build -p nextime-ffi --features mobile-neural --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust neural FFI build failed' }
    $neuralMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $neuralOutput = Join-Path $neuralMetadata.target_directory 'release'
    $neuralExecutable = Join-Path $neuralOutput 'luqnexa-neural-demo.exe'
    $neuralImportLibrary = Join-Path $neuralOutput 'nextime_ffi.dll.lib'
    $neuralVswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $neuralVsPath = & $neuralVswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $neuralVsPath) { throw 'Visual Studio C++ Build Tools are required' }
    Import-Module (Join-Path $neuralVsPath 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
    Enter-VsDevShell -VsInstallPath $neuralVsPath -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
    $env:TEMP = $neuralOutput
    $env:TMP = $neuralOutput
    $neuralObject = Join-Path $neuralOutput 'luqnexa-neural-demo.obj'
    cl /nologo /std:c11 /utf-8 /W4 /WX /D_CRT_SECURE_NO_WARNINGS '/Icrates/nextime-ffi/include' 'platforms/common/neural-demo.c' $neuralImportLibrary "/Fe$neuralExecutable" "/Fo$neuralObject"
    if ($LASTEXITCODE -ne 0) { throw 'C neural ABI demo compilation failed' }
    & $neuralExecutable $neuralModel
    if ($LASTEXITCODE -ne 0) { throw 'C neural ABI demo execution failed' }
} finally {
    $env:TEMP = $neuralOriginalTemp
    $env:TMP = $neuralOriginalTmp
    Pop-Location
}
