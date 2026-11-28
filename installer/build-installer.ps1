# Build the per-user Windows installer with NSIS.
#
# Example:
#   .\installer\build-installer.ps1 -Version 0.2.0 -NsisDir C:\Tools\nsis-3.11
#
# The release executable is taken from target\release\softprojector.exe unless
# -SourceExe is supplied.  The installer and its SHA-256 file are written to
# dist\ by default.

param(
    [string]$Version = "0.2.0",
    [string]$SourceExe = "",
    [string]$NsisDir = $env:NSIS_DIR,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if (-not $SourceExe) {
    $SourceExe = Join-Path $root "target\release\softprojector.exe"
}
if (-not (Test-Path -LiteralPath $SourceExe)) {
    throw "Release executable not found: $SourceExe. Run 'cargo build --release' first."
}
$SourceExe = (Resolve-Path -LiteralPath $SourceExe).Path

if (-not $NsisDir) {
    $command = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($command) {
        $NsisDir = Split-Path -Parent $command.Source
    }
}
if (-not $NsisDir) {
    throw "NSIS not found. Pass -NsisDir or set the NSIS_DIR environment variable."
}
$makensis = Join-Path $NsisDir "makensis.exe"
if (-not (Test-Path -LiteralPath $makensis)) {
    throw "makensis.exe not found in $NsisDir"
}

if (-not $OutputDir) {
    $OutputDir = Join-Path $root "dist"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path
$installer = Join-Path $OutputDir "RustProjector-v$Version-windows-x64-setup.exe"

Push-Location (Join-Path $root "installer")
try {
    & $makensis /V2 `
        "/DAPP_VERSION=$Version" `
        "/DSOURCE_EXE=$SourceExe" `
        "/DPROJECT_ROOT=$root" `
        "/DOUTPUT_EXE=$installer" `
        "RustProjector.nsi"
    if ($LASTEXITCODE -ne 0) {
        throw "makensis failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

if (-not (Test-Path -LiteralPath $installer)) {
    throw "Installer was not created: $installer"
}

$hash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
$checksumFile = "$installer.sha256"
"$hash  $(Split-Path -Leaf $installer)" | Set-Content -LiteralPath $checksumFile -Encoding ASCII

Write-Host ""
Write-Host "Installer: $installer"
Write-Host "Size:      $((Get-Item -LiteralPath $installer).Length) bytes"
Write-Host "SHA-256:   $hash"
Write-Host "Checksum:  $checksumFile"
