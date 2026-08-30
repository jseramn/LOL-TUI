# One-shot live TUI launcher (Windows, repo root).
#
# If the Live Client port answers, starts `cargo run` (127.0.0.1:2999).
# Otherwise prints that you must be in-game and how to run the offline replay.
#
#   powershell -ExecutionPolicy Bypass -File scripts/run-live.ps1

$ErrorActionPreference = "Stop"

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

if (Test-LiveClientPort) {
    Write-Host "Live Client detected on 127.0.0.1:2999 — starting TUI (q or Esc to quit)."
    cargo run
    exit $LASTEXITCODE
}

Write-Host "Live Client is not reachable on 127.0.0.1:2999."
Write-Host ""
Write-Host "Start a League match first (loading screen or in-game). The port is closed in lobby and champion select."
Write-Host ""
Write-Host "Offline demo (no game required):"
Write-Host "  cargo run -- replay tests/fixtures/allgamedata/full.json"
Write-Host ""
Write-Host "Preview the fixed layout without a game:"
Write-Host "  cargo run --example dump_frame"
Write-Host "  type docs\frames\live-80x24.txt"
exit 1
