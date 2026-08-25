# Manual Verification Checklist — tui-lol

Manual/E2E verification for the live match dashboard. These checks require a
real League of Legends game client: everything the offline test suite cannot
prove lives here (design "Testing Strategy" manual row; honest TLS limitation
per design — Riot's root private key is unavailable, so positive certificate
acceptance is only provable against a live client).

Run every step in **both** console hosts:

- [ ] **Windows Terminal**
- [ ] **Classic conhost** (e.g. `conhost.exe`, or a plain `cmd.exe` window)

---

## 1. Launch

From the repository root (`E:\dev\TUI-LOL`), in PowerShell:

```powershell
$env:CARGO_HOME='E:\rust\cargo'; $env:RUSTUP_HOME='E:\rust\rustup'; $env:Path='E:\w64devkit\w64devkit\bin;E:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;E:\rust\cargo\bin;'+$env:Path; $env:RUSTFLAGS='-Clink-self-contained=yes'; cargo run -j 1
```

Notes:

- The env prefix is mandatory on this machine (cargo is not on `PATH`; see
  `openspec/config.yaml`). `-j 1` avoids the Windows parallel-link commit
  limit.
- `cargo run --release -j 1` is optional; behavior is identical, startup is
  faster after the first release build.

## 2. Idle / standby view (no game running)

With the League client closed or outside a match:

- [ ] The app starts into the standby view: `Waiting for a live game...`
- [ ] The screen is otherwise silent — no error output, no error loop, no
      flickering messages (ui spec R2/S1).
- [ ] The bottom status line reads
      `This product is not endorsed by Riot Games. | state=standby | updated ?`
      (ui spec R6).

## 3. Live dashboard (during a real game)

Enter a real match (custom game suffices) and wait for the first poll cycle
(≤ 1 s at default cadence):

- [ ] The view swaps to the live dashboard within about one second of game
      start: `LIVE` headline, `Team ORDER` / `Team CHAOS` blocks with one
      panel per player (champion, level, KDA, CS, items, summoner spells),
      and your own `LOCAL ...` strip at the end.
- [ ] Dead players show their exposed respawn value verbatim, e.g.
      `DEAD(respawn 12.5)`; players dead without an exposed timer show
      `DEAD(respawn ?)` — never a ticking countdown (design hard rule).
- [ ] Gold appears ONLY on your LOCAL strip (`Gold <value>`); no enemy or
      ally panel ever shows gold (ui spec R4).
- [ ] The `EVENTS` section lists kills/objectives as they happen, each line
      starting with the exposed event time verbatim, e.g.
      `@512.18 DragonKill Chemtech slain by JungleKing STOLEN` (ui spec R5).
- [ ] The status line shows `state=in-game ok | updated HH:MM:SS UTC`.

## 4. Mid-game disconnect / reconnect (standby ↔ live swap)

While in a game, kill the connection path between the app and the client API
(e.g. close the game client or suspend the machine briefly):

- [ ] A hard disconnect (client gone → port unbound) drops the app back to
      the silent standby view without crashing (ui spec R2/S2).
- [ ] Re-entering / restoring the game returns the live dashboard within one
      poll cycle (≤ ~1 s) (ui spec R2/S3).
- [ ] A *transient* hiccup (brief timeout) does NOT drop to standby: panels
      stay up with the last good snapshot while the status line annotates
      `state=in-game DEGRADED(timeout)`, then recovers to
      `state=in-game ok` (design D4).

## 5. Resize behavior

At any point (standby and live), drag-resize or snap the window:

- [ ] The next frame redraws fully inside the new bounds; nothing panics,
      no visual garbage (ui spec R1/S2).
- [ ] After shrinking hard, content clips at the bottom/edges instead of
      crashing; the Riot notice stays visible on the last row.

## 6. Quit restores the console

Press `q`, then relaunch and press `Esc`:

- [ ] Either key exits immediately with success status.
- [ ] After exit the console is exactly as before launch: raw mode off
      (typed characters echo normally), alternate screen left, prompt intact,
      no leftover escape-sequence artifacts (ui spec R1/S1).

## 7. Panic restore (fault injection)

The shell installs both a Drop guard AND a panic hook that restore the
terminal before resuming unwinding — a panic must never orphan a broken
console (design "Terminal Handling"):

- [ ] If you ever observe a crash (any cause): the console comes back usable
      — text renders normally, no raw-mode input weirdness, no stuck
      alternate screen. Report the panic message if reproducible.

## 8. Positive TLS acceptance — the offline-untestable check

The automated suite pins Riot's published root certificate by SHA-256 and
rejects untrusted chains, but accepting a REAL client handshake requires the
genuine endpoint (design honest limitation):

- [ ] During a real game (step 3), data actually arrives: panels populate
      with real values that update as the game progresses. This is the proof
      that the pinned-root rustls client accepted the live client's TLS
      certificate on `https://127.0.0.1:2999`.
- [ ] Cross-check one displayed value against the game itself (your level,
      your CS, your gold) to confirm the data is live, not stale.

## 9. Non-endorsement notice visibility

In every state you passed through above — idle, live, degraded, after
resize, before quitting:

- [ ] The line `This product is not endorsed by Riot Games.` was persistently
      visible on the bottom status row in BOTH the standby and live views
      (ui spec R6). It never disappears, even mid-resize.

---

**Result**: record pass/fail per host (Windows Terminal / conhost). Any fail
is a defect against the linked spec scenario — file it with the host name,
step number, and what diverged.
