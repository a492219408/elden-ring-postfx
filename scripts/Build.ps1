# Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Push-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)
try {
    function Invoke-Cargo([string[]]$Arguments) {
        if (Get-Command mise -ErrorAction SilentlyContinue) {
            & mise @(@('exec','--','cargo') + $Arguments)
        } else { & cargo @Arguments }
        if ($LASTEXITCODE -ne 0) { throw "cargo failed: $($Arguments -join ' ')" }
    }
    Invoke-Cargo @('fmt','--all','--','--check')
    Invoke-Cargo @('clippy','--locked','--all-targets','--target','x86_64-pc-windows-msvc','--','-D','warnings')
    Invoke-Cargo @('test','--locked','--all-targets','--target','x86_64-pc-windows-msvc')
    Invoke-Cargo @('build','--locked','--release','--target','x86_64-pc-windows-msvc')
} finally { Pop-Location }
