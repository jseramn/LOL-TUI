# One-shot live TUI launcher (Windows, repo root).
#
# Applies the windows-gnu env this machine needs (w64devkit dlltool, rust on E:),
# then `cargo run -j 1` so rustc does not OOM the pagefile.
#
#   powershell -ExecutionPolicy Bypass -File scripts/run-live.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/run-live.ps1 -Replay

param(
    [switch]$Replay
)

$ErrorActionPreference = "Stop"

function Enable-WindowsGnuToolchain {
    if (Test-Path "E:\rust\cargo") { $env:CARGO_HOME = "E:\rust\cargo" }
    if (Test-Path "E:\rust\rustup") { $env:RUSTUP_HOME = "E:\rust\rustup" }
    $prefix = @()
    if (Test-Path "E:\w64devkit\w64devkit\bin") {
        $prefix += "E:\w64devkit\w64devkit\bin"
    }
    $dlltoolHome = "E:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"
    if (Test-Path $dlltoolHome) { $prefix += $dlltoolHome }
    if (Test-Path "E:\rust\cargo\bin") { $prefix += "E:\rust\cargo\bin" }
    if ($prefix.Count -gt 0) {
        $env:Path = ($prefix -join ";") + ";" + $env:Path
    }
    if (-not $env:RUSTFLAGS) {
        $env:RUSTFLAGS = "-Clink-self-contained=yes"
    }
    $env:CARGO_BUILD_JOBS = "1"
    $env:CARGO_INCREMENTAL = "0"
}

function Test-LiveClientPort {
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $client.Connect("127.0.0.1", 2999)
        return $true
    } catch {
        return $false
    } finally {
        if ($client.Connected) { $client.Close() }
    }
}

$repoRoot = Split-Path $PSScriptRoot -Parent
Set-Location $repoRoot
Enable-WindowsGnuToolchain

if ($Replay) {
    cargo run -j 1 -- replay tests/fixtures/allgamedata/full.json
    exit $LASTEXITCODE
}

if (Test-LiveClientPort) {
    Write-Host "Live Client on 127.0.0.1:2999 — TUI with cargo -j 1 (q / Esc to quit)."
    cargo run -j 1
    exit $LASTEXITCODE
}

Write-Host "Live Client is not reachable on 127.0.0.1:2999."
Write-Host "Enter a match first (not lobby). To compile without a game:"
Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/run-live.ps1 -Replay"
exit 1
