# Copies the cargo release binary into the matching npm platform package.
# Run from the repo root:  .\npm\build-npm.ps1
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$build = Join-Path $root "target/release"

if ($IsWindows -or ($env:OS -match "Windows")) {
    $exe = Join-Path $build "tordln.exe"
    if (-not (Test-Path $exe)) { Write-Error "missing $exe — run: cargo build --release"; exit 1 }
    Copy-Item $exe (Join-Path $root "npm/tordln-win32-x64/tordln.exe") -Force
    Write-Output "copied tordln.exe -> npm/tordln-win32-x64/"
} elseif ($IsLinux) {
    $bin = Join-Path $build "tordln"
    if (-not (Test-Path $bin)) { Write-Error "missing $bin — run: cargo build --release"; exit 1 }
    Copy-Item $bin (Join-Path $root "npm/tordln-linux-x64/tordln") -Force
    Write-Output "copied tordln -> npm/tordln-linux-x64/"
} else {
    Write-Error "unsupported platform for npm packaging: $($env:OS)"
    exit 1
}
