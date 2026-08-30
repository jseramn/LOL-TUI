# tui-lol

Dashboard de terminal para una partida **en curso** de League of Legends (Live Client Data API).

## Requisitos

- **League of Legends** con una partida **ya cargada o en juego** (no lobby / selección de campeón).
- **Rust stable** (`rustup` + `cargo`).
- **Windows Terminal** recomendado (también funciona en conhost clásico).

> El puerto `https://127.0.0.1:2999` solo responde **durante la partida**.  
> El oro solo aparece en la franja **LOCAL** (tu jugador); el resto de paneles no muestran oro.

## Clonar y usar esta rama

```powershell
git clone https://github.com/jseramn/LOL-TUI.git
cd LOL-TUI
git checkout cursor/live-visual-terminal-7f59
```

## En vivo (tu PC)

Con la partida ya en curso, desde la raíz del repo:

```powershell
cargo run
```

Atajo en Windows (comprueba el puerto `2999` y lanza el TUI, o indica cómo usar el replay offline):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run-live.ps1
```

Salir: **`q`** o **`Esc`**.

### Comprobar identidad / reloj de partida

```powershell
curl.exe -k https://127.0.0.1:2999/liveclientdata/gamestats
python scripts/live_bridge.py --id-only
```

## Demo sin partida (offline)

```powershell
cargo run -- replay tests/fixtures/allgamedata/full.json
```

Un snapshot en texto (sin TUI):

```powershell
cargo run -- dump tests/fixtures/allgamedata/full.json
```

### Capturas de layout (revisión sin LoL)

El ejemplo `dump_frame` renderiza el dashboard live desde `tests/fixtures/allgamedata/full.json` con ratatui `TestBackend` y escribe UTF-8 en `docs/frames/`:

```powershell
cargo run --example dump_frame
```

Archivos generados (80×24 canónico y 120×32 ancho):

- [`docs/frames/live-80x24.txt`](docs/frames/live-80x24.txt)
- [`docs/frames/live-120x32.txt`](docs/frames/live-120x32.txt)

## Ayuda CLI

```powershell
cargo run -- --help
```

Comandos: TUI por defecto, `replay`, `dump`, `bridge` (proxy HTTP para túneles).

---

## Apéndice: túnel opcional (agente remoto)

**No hace falta** para jugar en local. Solo si un agente en la nube debe leer tu partida:

El puerto del juego (`https://127.0.0.1:2999`) solo existe **durante la partida** y usa un certificado Riot. No lo expongas directamente. En su lugar, levanta el **puente HTTP en loopback** (`127.0.0.1:18789`) y un túnel corto sobre ese puente. **Nunca** tunneles el LCU (cliente de Riot en otro puerto).

### Identidad de la partida

El reloj/modo/mapa salen del Live Client (`gamestats`). El **id numérico** suele faltar en esa API; la fuente fiable es el LCU (lockfile o línea de comandos de `LeagueClientUx` en Windows):

```powershell
curl.exe -k https://127.0.0.1:2999/liveclientdata/gamestats
python scripts/live_bridge.py --id-only
```

### Cloudflare quick tunnel (URL pública temporal)

`cloudflared tunnel --url` crea un enlace **trycloudflare.com** de un solo uso — no es `cloudflared serve` ni un hostname permanente.

```powershell
python scripts/live_bridge.py --tunnel
# o en dos terminales:
cargo run -- bridge
cloudflared tunnel --url http://127.0.0.1:18789
```

### Tailscale (solo tailnet o público con Funnel)

- **`tailscale serve`** — solo máquinas de tu tailnet (no Internet público).
- **`tailscale funnel`** — si necesitas una URL pública (equivalente conceptual al quick tunnel).

```powershell
cargo run -- bridge
tailscale serve --bg 18789
# o, para URL pública:
tailscale funnel 18789
```

El agente remoto usa `cargo run -- --live-url https://<host-del-tunel>` o la variable `TUI_LOL_LIVE_URL`.

Verificación manual detallada: [`docs/manual-verify.md`](docs/manual-verify.md) (§12).
