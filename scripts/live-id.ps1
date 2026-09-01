# Live Client Data: dump match identity + start the HTTP bridge/tunnel.
#
# Run from the repo root WHILE a League match is already loading/in-game
# (port 2999 is closed in lobby / champ select).
#
# Identity only:
#   powershell -ExecutionPolicy Bypass -File scripts/live-id.ps1
#
# Bridge + cloudflared (paste TUI_LOL_LIVE_URL back to the cloud agent):
#   powershell -ExecutionPolicy Bypass -File scripts/live-id.ps1 -Tunnel

param(
    [switch]$Tunnel,
    [string]$Listen = "127.0.0.1:18789"
)

$ErrorActionPreference = "Continue"

function Get-LiveStats {
    try {
        return Invoke-RestMethod -Uri "https://127.0.0.1:2999/liveclientdata/gamestats" -SkipCertificateCheck
    } catch {
        # Windows PowerShell 5: no -SkipCertificateCheck
        add-type @"
using System.Net;
using System.Security.Cryptography.X509Certificates;
public class TrustAll : ICertificatePolicy {
    public bool CheckValidationResult(ServicePoint s, X509Certificate c, WebRequest r, int p) { return true; }
}
"@ -ErrorAction SilentlyContinue
        [System.Net.ServicePointManager]::CertificatePolicy = New-Object TrustAll
        [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
        return Invoke-RestMethod -Uri "https://127.0.0.1:2999/liveclientdata/gamestats"
    }
}

Write-Host "=== tui-lol live identity ==="
try {
    $stats = Get-LiveStats
    Write-Host ("liveclient READY  mode={0} time={1} map={2}" -f $stats.gameMode, $stats.gameTime, $stats.mapName)
} catch {
    Write-Host "liveclient DOWN — enter a match (not lobby). $($_.Exception.Message)"
}

$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command python3 -ErrorAction SilentlyContinue }
$script = Join-Path $PSScriptRoot "live_bridge.py"

if ($python -and (Test-Path $script)) {
    $pyArgs = @($script, "--listen", $Listen)
    if ($Tunnel) { $pyArgs += "--tunnel" }
    Write-Host "starting $($python.Source) $($pyArgs -join ' ')"
    & $python.Source @pyArgs
    exit $LASTEXITCODE
}

Write-Host "Python not found. Fallback one-liners:"
Write-Host '  curl.exe -k https://127.0.0.1:2999/liveclientdata/gamestats'
Write-Host '  cargo run -- bridge'
Write-Host '  cloudflared tunnel --url http://127.0.0.1:18789'
if ($Tunnel) { exit 1 }
exit 0
