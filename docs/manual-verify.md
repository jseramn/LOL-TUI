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

## 10. Known schema drift pitfall (post-verification discovery)

After U6 verify passed on fixtures, the first real-game check on this machine
surfaced a schema drift between the hand-authored fixtures and the live Live
Client Data API:

- `events` is wrapped in an object in the real payload:
  `{"events": {"Events": [...]}}` — the bare-array form only exists in the
  legacy fixtures. The `EventsShape` untagged enum in
  `src/model/live_data.rs` accepts both shapes.
- `gameStats` was renamed to `gameData` in the live payload. The field is
  annotated `#[serde(rename = "gameData", alias = "gameStats")]` so legacy
  fixtures still deserialize.

If you ever observe the app stuck on `state=standby | updated ?` *during a
real game*:

- Run `cargo run --example live_probe -j 1` to check whether the TLS/fetch
  path itself works against the live port.
- Run `cargo run --example parse_captured -- <path-to-json>` to isolate a
  DTO mismatch from a transport error.

Both examples are checked in under `examples/` as reusable debugging aids.

`tests/fixtures/allgamedata/captured-*.json` is gitignored because real
payloads contain summoner names (PII).

---

## 11. Live dashboard visualizations (add-live-visualizations)

Offline tests prove buffer purity, scaling math, tier boundaries, and
placeholder behavior against fixtures; what they cannot prove is how the six
chart families LOOK and DEGRADE on real console hosts with real fonts. Run
every step here in **both** hosts (Windows Terminal AND classic conhost),
during a live custom game, at a window of at least 80×24 unless the step says
otherwise.

### 11.1 All six widget families render

- [ ] **CS bars** — each player panel has one horizontal bar row beneath it;
      the highest creep score on screen fills its whole track while lower
      scores render proportionally shorter (shared maximum, viz:R3/S1); a
      player whose creep score is absent shows a `CS ?` row instead of an
      empty bar (viz:R3/S2).
- [ ] **Level bars** — fixed 1–18 scale: a level 1 champion draws a
      near-empty track and level 18 draws full; an out-of-range value (e.g.
      25) clamps to a full track without any glitch or crash; an absent
      level renders `Lv?` (viz:R4/S1–S2).
- [ ] **K/D/A mini-bars** — three short segments per player: kills green,
      deaths red, assists blue, each scaled by its own metric's shared
      maximum; a player with 0 deaths still shows all three segments and NO
      derived ratio (e.g. (K+A)/D) appears anywhere in the frame
      (viz:R5/S2–S3).
- [ ] **Inventory fill bar** — a 6-cell strip of dark-shade (filled) and
      light-shade (empty) cells per player; a full build with trinket shows
      fewer than 6 filled cells because the trinket slot is excluded;
      null slots render empty; a wholly absent items list renders `?`
      (viz:R6/S1–S2).
- [ ] **Local HP / Power gauges** — two blocked-segment gauges labeled
      `LOCAL HP <pct>%` and `LOCAL Power <pct>%` whose percentages match
      your current/max values in the game HUD (e.g. 2100/3000 ⇒ 70%);
      non-local players never show gauges; if a stat is missing, THAT gauge
      alone shows `LOCAL … ?` while the other renders normally
      (viz:R7/S1–S2).
- [ ] **Gold sparkline** — a `GOLD` trend row draws once at least two real
      samples exist; before that it explicitly reads
      `GOLD warming up (n/120)` instead of a flat line; failed polls leave
      visible light-shade gaps inside the trend rather than invented points
      (viz:R8/S1).

### 11.2 Conhost glyph smoke (classic conhost)

The offline suite proves every drawn codepoint sits in box-drawing/block
ranges that classic conhost can render; this step proves the HOST FONT
actually does:

- [ ] In a plain conhost window, every chart symbol renders as a real block
      or box character: solid bars, the shaded inventory cells, and the
      eight-level sparkline ramp — none of them appear as hollow boxes,
      question-mark-in-a-box replacement glyphs, or blanks (viz:R10/S2).
- [ ] No Braille dots or sextant/octant shapes appear anywhere — the app
      never draws outside its whitelisted ranges by construction.
- [ ] On a 16-color host (conhost default palette), the K/D/A segments may
      degrade to plainer colors but all three remain visible
      (viz:R5/S3).

### 11.3 Minimum viewport and resize walk-down across tiers

Start at ≥ 80×24 with all charts visible, then shrink the window HEIGHT one
band at a time and confirm the degradation matrix (viz:R9/S1):

- [ ] Below 24 rows the gold sparkline hides FIRST; below 20 the K/D/A
      mini-bars go; below 18 the inventory strips; below 15 the level bars;
      below 13 the CS bars — always in that strict order.
- [ ] The `LOCAL HP` / `LOCAL Power` gauges NEVER hide at any height (they
      belong to no tier).
- [ ] The events ticker compresses before anything else loses space, and the
      Riot notice stays on the LAST row at every size (ui:R6).
- [ ] Shrinking WIDTH below roughly 40 columns hides ALL chart families at
      any height; the gauges and status row remain.
- [ ] Growing back up restores families in the reverse order with no stale
      artifacts or garbled rows.
- [ ] At the 80×12 minimum viewport the status row is still last, fully
      visible, and unoverlapped (viz:R2/S2).

### 11.4 Reconnect same-game trend continuation

- [ ] With the gold trend drawn, drop and restore the connection path while
      staying in the SAME game: the trend CONTINUES across the pre-disconnect
      samples (failed polls show as gaps, never a reset) (viz:R8/S2).
- [ ] Leave and enter a DIFFERENT game: the gold trend resets — the row
      returns to `GOLD warming up (n/120)` until two fresh samples arrive
      (viz:R8/S3).

---

**Result**: record pass/fail per host (Windows Terminal / conhost). Any fail
is a defect against the linked spec scenario — file it with the host name,
step number, and what diverged.
