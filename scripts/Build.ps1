# Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $projectRoot
try {
    function Invoke-DevelopmentTool([string[]]$Arguments) {
        if (Get-Command mise -ErrorAction SilentlyContinue) {
            $miseArguments = @('exec', '--') + $Arguments
            & mise @miseArguments
        } else {
            $program = $Arguments[0]
            & $program @($Arguments | Select-Object -Skip 1)
        }
        if ($LASTEXITCODE -ne 0) { throw "工具执行失败: $($Arguments -join ' ')" }
    }
    $commit = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw '打包需要 Git 提交。' }
    $status = & git status --porcelain
    if ($LASTEXITCODE -ne 0 -or $status) { throw '请先提交源码改动，确保二进制与源码包对应同一提交。' }
    Invoke-DevelopmentTool -Arguments @('cargo', 'fmt', '--all', '--', '--check')
    Invoke-DevelopmentTool -Arguments @('cargo', 'clippy', '--all-targets', '--', '-D', 'warnings')
    Invoke-DevelopmentTool -Arguments @('cargo', 'test', '--all-targets')
    Invoke-DevelopmentTool -Arguments @('cargo', 'build', '--release', '--target', 'x86_64-pc-windows-msvc')
    $versionMatch = Select-String -LiteralPath 'Cargo.toml' -Pattern '^version = "([^"]+)"$'
    $version = $versionMatch.Matches[0].Groups[1].Value
    $name = "EldenRingPostFX-$version-preview-$($commit.Substring(0, 7))"
    $dist = Join-Path $projectRoot 'dist'
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    $package = Join-Path $dist $name
    if (Test-Path -LiteralPath $package) { throw "输出目录已存在，请保留或手动移走后再打包: $package" }
    New-Item -ItemType Directory -Path (Join-Path $package 'natives') | Out-Null
    Copy-Item -LiteralPath 'target/x86_64-pc-windows-msvc/release/elden_ring_postfx.dll' -Destination (Join-Path $package 'natives/EldenRingPostFX.dll')
    Copy-Item -LiteralPath 'assets/EldenRingPostFX.ini' -Destination (Join-Path $package 'natives/EldenRingPostFX.ini')
    Copy-Item -LiteralPath 'assets/EldenRingPostFX.me3' -Destination (Join-Path $package 'EldenRingPostFX.me3')
    foreach ($file in @('README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md')) {
        Copy-Item -LiteralPath $file -Destination (Join-Path $package $file)
    }
    $dllHash = (Get-FileHash -LiteralPath (Join-Path $package 'natives/EldenRingPostFX.dll') -Algorithm SHA256).Hash
    @("version=$version", "source_commit=$commit", 'target=x86_64-pc-windows-msvc', "dll_sha256=$dllHash", 'status=preview; game visual validation pending') | Set-Content -LiteralPath (Join-Path $package 'BUILD_INFO.txt') -Encoding utf8
    $sourceZip = Join-Path $dist "$name-source.zip"
    & git archive --format=zip "--prefix=$name-source/" "--output=$sourceZip" HEAD
    if ($LASTEXITCODE -ne 0) { throw '源码打包失败。' }
    $runtimeZip = Join-Path $dist "$name.zip"
    Compress-Archive -LiteralPath $package -DestinationPath $runtimeZip
    Get-FileHash -LiteralPath $runtimeZip, $sourceZip -Algorithm SHA256 | Format-Table -AutoSize
    Write-Output "运行包: $runtimeZip"
    Write-Output "源码包: $sourceZip"
} finally {
    Pop-Location
}
