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

## Ayuda CLI

```powershell
cargo run -- --help
```

Comandos: TUI por defecto, `replay`, `dump`, `bridge` (proxy HTTP para túneles).

---

## Apéndice: túnel opcional (agente remoto)

**No hace falta** para jugar en local. Solo si un agente en la nube debe leer tu partida:

```powershell
python scripts/live_bridge.py --tunnel
# o en dos terminales:
cargo run -- bridge
cloudflared tunnel --url http://127.0.0.1:18789
```

Alternativa Tailscale: `cargo run -- bridge` y `tailscale serve --bg 18789`.

El agente remoto usa `cargo run -- --live-url https://<host-del-tunel>` o la variable `TUI_LOL_LIVE_URL`.

Verificación manual detallada: [`docs/manual-verify.md`](docs/manual-verify.md).
