#!/usr/bin/env python3
"""HTTP bridge + identity dump for the League Live Client Data API.

The game client only serves https://127.0.0.1:2999 while you are IN a
match (not lobby / champ select), with a Riot-pinned TLS cert. Cloud
agents cannot reach that port, so this process:

  1. Prints the live game identity (and LCU gameId when the lockfile exists)
  2. Serves a plain-HTTP reverse proxy on 127.0.0.1:18789
  3. Optionally launches `cloudflared tunnel --url http://127.0.0.1:18789`
     and prints the trycloudflare URL to paste back to the cloud agent

Usage (PowerShell, from the repo root, with LoL already in a game):

    python scripts/live_bridge.py
    python scripts/live_bridge.py --tunnel

Then tell the agent:

    TUI_LOL_LIVE_URL=https://<trycloudflare-host>
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import re
import ssl
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LIVE_ORIGIN = "https://127.0.0.1:2999"
DEFAULT_LISTEN = "127.0.0.1:18789"
INSECURE_CTX = ssl._create_unverified_context()


def live_get(path: str, timeout: float = 2.0) -> tuple[int, bytes]:
    req = urllib.request.Request(LIVE_ORIGIN + path, method="GET")
    try:
        with urllib.request.urlopen(req, context=INSECURE_CTX, timeout=timeout) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as err:
        return err.code, err.read() or b""
    except Exception as err:
        raise ConnectionError(str(err)) from err


def find_lockfile() -> str | None:
    candidates = [
        os.path.join(os.environ.get("PROGRAMFILES", r"C:\Program Files"), "Riot Games", "League of Legends", "lockfile"),
        r"C:\Riot Games\League of Legends\lockfile",
        os.path.join(os.environ.get("PROGRAMFILES(X86)", r"C:\Program Files (x86)"), "Riot Games", "League of Legends", "lockfile"),
        os.path.expandvars(r"%LOCALAPPDATA%\Riot Games\League of Legends\lockfile"),
    ]
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    return None


def lcu_game_id() -> str | None:
    lock = find_lockfile()
    if not lock:
        return None
    try:
        raw = open(lock, encoding="utf-8").read().strip()
        # LeagueClient:pid:port:password:https
        parts = raw.split(":")
        if len(parts) < 5:
            return None
        port, password, protocol = parts[2], parts[3], parts[4]
        token = base64.b64encode(f"riot:{password}".encode()).decode()
        url = f"{protocol}://127.0.0.1:{port}/lol-gameflow/v1/session"
        req = urllib.request.Request(url, headers={"Authorization": f"Basic {token}"})
        with urllib.request.urlopen(req, context=INSECURE_CTX, timeout=2.0) as resp:
            data = json.loads(resp.read().decode("utf-8", errors="replace"))
        game = data.get("gameData") or {}
        gid = game.get("gameId")
        return str(gid) if gid not in (None, "", 0, "0") else None
    except Exception:
        return None


def identity_payload() -> dict:
    status, body = live_get("/liveclientdata/allgamedata")
    if status != 200:
        raise ConnectionError(f"live client HTTP {status}")
    data = json.loads(body.decode("utf-8", errors="replace"))
    game = data.get("gameData") or data.get("gameStats") or {}
    active = data.get("activePlayer") or {}
    players = data.get("allPlayers") or []
    return {
        "ok": True,
        "gameIdLive": game.get("gameId"),
        "gameIdLcu": lcu_game_id(),
        "gameMode": game.get("gameMode"),
        "gameTime": game.get("gameTime"),
        "mapName": game.get("mapName"),
        "localChampion": active.get("championName"),
        "playerCount": len(players),
    }


def print_identity() -> None:
    try:
        ident = identity_payload()
    except Exception as err:
        print(f"identity: not in game ({err})", flush=True)
        return
    gid = ident.get("gameIdLcu") or ident.get("gameIdLive") or "n/a"
    print(
        "LIVE"
        f" id={gid}"
        f" mode={ident.get('gameMode')}"
        f" time={ident.get('gameTime')}"
        f" map={ident.get('mapName')}"
        f" local={ident.get('localChampion')}"
        f" players={ident.get('playerCount')}",
        flush=True,
    )


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args) -> None:
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))

    def do_GET(self) -> None:
        path = self.path.split("?", 1)[0]
        if path == "/health":
            return self._send(200, b"ok\n", "text/plain")
        if path in ("/identity", "/identity.json"):
            try:
                payload = json.dumps(identity_payload()).encode()
                return self._send(200, payload, "application/json")
            except ConnectionError:
                return self._send(503, b'{"ok":false,"reason":"not_in_game"}', "application/json")
        if path.startswith("/liveclientdata/"):
            try:
                status, body = live_get(path)
            except ConnectionError:
                return self._send(503, b'{"ok":false,"reason":"not_in_game"}', "application/json")
            return self._send(status, body, "application/json")
        self._send(404, b"not found\n", "text/plain")

    def _send(self, status: int, body: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)


def serve(host: str, port: int) -> ThreadingHTTPServer:
    httpd = ThreadingHTTPServer((host, port), Handler)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    return httpd


def run_cloudflared(url: str) -> None:
    cmd = ["cloudflared", "tunnel", "--url", url]
    print("starting:", " ".join(cmd), flush=True)
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    assert proc.stdout is not None
    pattern = re.compile(r"https://[a-z0-9-]+\.trycloudflare\.com")
    announced = False
    for line in proc.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()
        match = pattern.search(line)
        if match and not announced:
            public = match.group(0)
            print("\n===== PEGA ESTO AL AGENTE CLOUD =====", flush=True)
            print(f"TUI_LOL_LIVE_URL={public}", flush=True)
            gid = None
            try:
                ident = identity_payload()
                gid = ident.get("gameIdLcu") or ident.get("gameIdLive")
            except Exception:
                pass
            if gid:
                print(f"GAME_ID={gid}", flush=True)
            print("====================================\n", flush=True)
            announced = True
    proc.wait()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--listen", default=DEFAULT_LISTEN, help="bind address (default 127.0.0.1:18789)")
    parser.add_argument("--tunnel", action="store_true", help="also run cloudflared quick tunnel")
    parser.add_argument("--id-only", action="store_true", help="print identity once and exit (no server)")
    args = parser.parse_args()

    if args.id_only:
        print_identity()
        return 0

    host, _, port_s = args.listen.partition(":")
    port = int(port_s)
    httpd = serve(host or "127.0.0.1", port)
    print(f"bridge on http://{host}:{port}", flush=True)
    print("  cloudflared tunnel --url http://%s:%s" % (host, port), flush=True)
    print("  tailscale serve --bg %s" % port, flush=True)
    print_identity()
    threading.Thread(target=lambda: _identity_loop(), daemon=True).start()

    if args.tunnel:
        try:
            run_cloudflared(f"http://{host}:{port}")
        except FileNotFoundError:
            print("cloudflared not on PATH. Install it, then re-run with --tunnel.", file=sys.stderr)
            print("  winget install --id Cloudflare.cloudflared", flush=True)
            httpd.shutdown()
            return 1
    else:
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            pass
    httpd.shutdown()
    return 0


def _identity_loop() -> None:
    while True:
        time.sleep(5)
        print_identity()


if __name__ == "__main__":
    sys.exit(main())
