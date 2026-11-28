# Package the portable Windows build and its SHA-256 checksum.
#
# The resulting zip contains softprojector.exe, README.md and LICENSE at the
# archive root.  The built-in updater looks for softprojector.exe in the zip,
# so keep that layout when changing this script.
#
# Example:
#   .\scripts\package-release.ps1 -Version 0.2.0

param(
    [string]$Version = "0.2.0",
    [string]$SourceExe = "",
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

if (-not $OutputDir) {
    $OutputDir = Join-Path $root "dist"
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$OutputDir = (Resolve-Path -LiteralPath $OutputDir).Path

$name = "RustProjector-v$Version-windows-x64"
$staging = Join-Path $OutputDir $name
$archive = Join-Path $OutputDir "$name.zip"

if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
New-Item -ItemType Directory -Path $staging | Out-Null

Copy-Item -LiteralPath $SourceExe -Destination (Join-Path $staging "softprojector.exe")
Copy-Item -LiteralPath (Join-Path $root "README.md") -Destination $staging
Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination $staging

if (Test-Path -LiteralPath $archive) {
    Remove-Item -LiteralPath $archive -Force
}
Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $archive -CompressionLevel Optimal
Remove-Item -LiteralPath $staging -Recurse -Force

$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
$checksumFile = "$archive.sha256"
"$hash  $(Split-Path -Leaf $archive)" | Set-Content -LiteralPath $checksumFile -Encoding ASCII

Write-Host ""
Write-Host "Archive:  $archive"
Write-Host "Size:     $((Get-Item -LiteralPath $archive).Length) bytes"
Write-Host "SHA-256:  $hash"
Write-Host "Checksum: $checksumFile"
