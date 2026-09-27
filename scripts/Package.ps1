# Copyright (C) 2026 Luna(a492219408). SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$script = Join-Path (Split-Path $PSScriptRoot -Parent) 'tools\release.py'
if (Get-Command mise -ErrorAction SilentlyContinue) {
    & mise @('exec','--','python',$script,'pack')
} else { & python $script pack }
if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed' }
