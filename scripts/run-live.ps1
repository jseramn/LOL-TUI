# One-shot live TUI launcher (Windows). Locates the repo from this script
# path, optionally git-pulls main, applies the windows-gnu env, then
# `cargo run -j 1` so rustc does not OOM the pagefile.
#
# From ANY directory (update + start):
#   powershell -ExecutionPolicy Bypass -File E:\dev\TUI-LOL\LOL-TUI\scripts\run-live.ps1 -Pull
#
# Already in the repo, current PowerShell (PS7 or 5.1):
#   powershell -ExecutionPolicy Bypass -File scripts\run-live.ps1
#   pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\run-live.ps1
#
# If you are NOT in a match, this script prints a diagnosis and starts the
# offline demo automatically. Force demo: -Replay. Refuse demo: -LiveOnly.

param(
    [switch]$Replay,
    [switch]$Pull,
    [switch]$LiveOnly,
    [switch]$Diagnose
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

function Test-TcpPortOpen {
    param(
        [string]$HostName = "127.0.0.1",
        [int]$Port = 2999,
        [int]$TimeoutMs = 700
    )
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $iar = $client.BeginConnect($HostName, $Port, $null, $null)
        if (-not $iar.AsyncWaitHandle.WaitOne($TimeoutMs, $false)) {
            return $false
        }
        $client.EndConnect($iar)
        return $client.Connected
    } catch {
        return $false
    } finally {
        $client.Close()
    }
}

function Get-LiveStatsProbe {
    $curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if (-not $curl) { return $null }
    $saved = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $out = & curl.exe -sk --max-time 2 https://127.0.0.1:2999/liveclientdata/gamestats 2>$null
        if ($LASTEXITCODE -ne 0 -or -not $out) { return $null }
        try {
            return $out | ConvertFrom-Json
        } catch {
            return @{ gameMode = "unknown"; gameTime = $null; mapName = $null }
        }
    } finally {
        $ErrorActionPreference = $saved
    }
}

function Write-Diagnosis {
    param([string]$RepoRoot)

    Write-Host ""
    Write-Host "=== diagnostico tui-lol ==="
    Write-Host ("cwd        {0}" -f (Get-Location).Path)
    Write-Host ("repo       {0}" -f $RepoRoot)
    Write-Host ("PS         {0}" -f $PSVersionTable.PSVersion)
    Write-Host ("host       {0}" -f $env:COMPUTERNAME)

    $git = Get-Command git -ErrorAction SilentlyContinue
    if ($git) {
        $branch = (git -C $RepoRoot rev-parse --abbrev-ref HEAD 2>$null)
        $sha = (git -C $RepoRoot rev-parse --short HEAD 2>$null)
        Write-Host ("git        {0} @ {1}" -f $branch, $sha)
    } else {
        Write-Host "git        NO encontrado"
    }

    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    Write-Host ("cargo      {0}" -f $(if ($cargo) { $cargo.Source } else { "NO encontrado (hace falta E:\rust\cargo\bin)" }))
    Write-Host ("rustc      {0}" -f $(if ($rustc) { $rustc.Source } else { "NO encontrado" }))
    if (Test-Path "E:\w64devkit\w64devkit\bin\dlltool.exe") {
        Write-Host "dlltool    E:\w64devkit\w64devkit\bin\dlltool.exe"
    } else {
        Write-Host "dlltool    NO en E:\w64devkit\w64devkit\bin (el link windows-gnu lo necesita)"
    }

    $tcp = Test-TcpPortOpen
    Write-Host ("puerto 2999 TCP  {0}" -f $(if ($tcp) { "abierto" } else { "cerrado" }))
    $stats = $null
    if ($tcp) {
        $stats = Get-LiveStatsProbe
    }
    if ($stats) {
        Write-Host ("liveclient      READY  modo={0} reloj={1} mapa={2}" -f $stats.gameMode, $stats.gameTime, $stats.mapName)
    } else {
        Write-Host "liveclient      DOWN — Riot solo abre 127.0.0.1:2999 DENTRO del mapa, no en lobby ni en seleccion."
    }
    Write-Host "=== fin diagnostico ==="
    Write-Host ""
    return [bool]$stats
}

$repoRoot = Split-Path $PSScriptRoot -Parent
if (-not (Test-Path (Join-Path $repoRoot "Cargo.toml"))) {
    Write-Host "No encuentro Cargo.toml junto al script. Ruta esperada: $repoRoot"
    exit 1
}
Set-Location $repoRoot
Enable-WindowsGnuToolchain

if ($Pull) {
    Write-Host "repo $repoRoot — checkout + pull main"
    git checkout main
    if ($LASTEXITCODE -ne 0) { throw "git checkout main failed" }
    git pull origin main
    if ($LASTEXITCODE -ne 0) { throw "git pull origin main failed" }
}

$liveOk = Write-Diagnosis -RepoRoot $repoRoot

if ($Diagnose) {
    if ($liveOk) { exit 0 } else { exit 2 }
}

function Start-Tui {
    param([string[]]$CargoArgs)
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Host "cargo no esta en PATH. Instala rust en E:\rust o abre el terminal donde `cargo --version` funcione."
        exit 1
    }
    Write-Host ("arrancando: cargo run -j 1 {0}" -f ($CargoArgs -join " "))
    Write-Host "salir: q o Esc"
    & cargo run -j 1 @CargoArgs
    exit $LASTEXITCODE
}

if ($Replay) {
    Write-Host "Modo demo (fixture offline, no hace falta LoL)."
    Start-Tui -CargoArgs @("replay", "tests/fixtures/allgamedata/full.json")
}

if ($liveOk) {
    Write-Host "Partida detectada — TUI en vivo."
    Start-Tui -CargoArgs @()
}

Write-Host "No hay partida en 127.0.0.1:2999."
if ($LiveOnly) {
    Write-Host "(-LiveOnly) no arranco la demo. Entra al mapa y vuelve a ejecutar este script."
    exit 1
}

Write-Host "Arranco la DEMO para que veas la herramienta. Cuando estes EN el mapa, corre el mismo comando otra vez."
Start-Tui -CargoArgs @("replay", "tests/fixtures/allgamedata/full.json")
