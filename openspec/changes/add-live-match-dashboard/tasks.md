# Tasks: League of Legends Live Match Dashboard (live-only)

**Change**: `add-live-match-dashboard` · Basis: proposal.md, specs `live-client-poller` + `live-dashboard-ui`, design.md D1–D5 · Store: hybrid (this file + Engram `sdd/add-live-match-dashboard/tasks`)
**TDD**: strict red-green; fixture corpus authored BEFORE model code · Threat matrix: N/A per design (loopback construction guard covered in 2.1–2.2)

## Guard Lines

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main (resolved by orchestrator at U1 apply start)
400-line budget risk: High
```

## Spec Requirement ID Map (used in acceptance links)

- poller:R1 loopback-only TLS w/ Riot cert · R2 non-overlapping scheduling · R3 lifecycle detection · R4 normalized snapshot deserialization · R5 offline fixture testability
- ui:R1 shell/event loop · R2 standby view · R3 all-player panels · R4 local-player panel · R5 event ticker · R6 notice + status line
- `Rx/S#` = #th Scenario under requirement Rx.

## Canonical Commands (verbatim from openspec/config.yaml — W4)

Every task step marked **Test:** or **Build:** executes these exact strings.

- **CT** (apply.test_command):
  `$env:CARGO_HOME='E:\rust\cargo'; $env:RUSTUP_HOME='E:\rust\rustup'; $env:Path='E:\w64devkit\w64devkit\bin;E:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;E:\rust\cargo\bin;'+$env:Path; $env:RUSTFLAGS='-Clink-self-contained=yes'; cargo test -j 1`
- **CB** (verify.build_command): identical prefix ending `cargo build -j 1`.

## Suggested Work Units

| Unit | Goal | Likely PR | Focused test | Runtime harness | Rollback boundary |
|---|---|---|---|---|---|
| 1 | Deps, pem, stubs, fixtures, tolerant models (P0+P1) | PR 1 | CT (model filters) | Offline fixtures only (poller:R5 forbids needing :2999) | revert slice commits; stubs self-contained |
| 2 | Poller core (P2) | PR 2 | CT (api filters) | N/A offline-by-design; positive TLS accept deferred to manual checklist (design limitation) | revert `src/api/*`; model untouched |
| 3 | Shell/event loop (P3) | PR 3 | CT (app/shell filters) | ratatui TestBackend buffers | revert app/events/main |
| 4 | Views (P4+P5) | PR 4 | CT (ui filters) | TestBackend buffers from fixture snapshots | revert `src/ui/*` |
| 5 | Verification + checklist (P6) | PR 5 | CT full + clippy/fmt | Manual WT/conhost checklist (docs) | docs + lint fixes only |

## Phase 0: Dependencies & Skeleton

- [x] 0.1 Extend `Cargo.toml`: serde(+derive), serde_json, thiserror, reqwest `{default-features=false, features=["blocking","rustls-tls"]}`; dev-dep `rcgen` (untrusted-chain test). Build: CB green.
- [x] 0.2 Create `assets/riotgames.pem` (vendored Riot root; resolves design open-question default) with provenance header: source URL + SHA-256.
- [x] 0.3 Create module stubs per design Module Layout: `src/api/{mod,client,poller,source,error}.rs`, `src/model/{mod,live_data,snapshot}.rs`, `src/app.rs`, `src/events.rs`, `src/ui/{mod,standby,dashboard,ticker,status}.rs`; wire mod tree from `main.rs`. Build: CB green.

## Phase 1: Fixture Corpus & Tolerant Models (RED before GREEN)

- [x] 1.1 Author `tests/fixtures/allgamedata/`: `full.json` (10 players complete), `partial_player.json` (one player omits items/respawnTimer/parts of scores), `empty_events.json`, `malformed.json` (invalid JSON); provenance comments. NO model code exists yet (strict-TDD ordering).
- [x] 1.2 RED `tests/model/live_data_tests.rs`: full parses with every listed field Some = fixture values (poller:R4/S1); partial_player → those fields absent, all other players intact (R4/S2); null == omitted == absent; malformed → reported parse error, no panic (R4/S3). Run CT → compile failure = RED confirmed.
- [x] 1.3 GREEN `src/model/live_data.rs`: Option-everywhere serde DTOs (playerlist, scores, items, summoner spells + cooldowns as exposed, gameStats, events); no field defaults to a fabricated value.
- [x] 1.4 GREEN `src/model/snapshot.rs`: normalized Snapshot — per player champion/team/position/level/KDA/CS/items/spells/isDead/respawnTimer-as-exposed; local currentGold + exposed stat detail; gameTime/gameMode/mapName; event list. Test: CT green (accepts poller:R4, R5/S1).

## Phase 2: Poller Core

- [x] 2.1 RED `tests/api/client_tests.rs`: constructing client with host ≠ 127.0.0.1 fails before any packet (poller:R1/S3). Run CT → RED.
- [x] 2.2 GREEN `src/api/error.rs` (NotBound | Transient{Timeout,Tls,Http,Parse} taxonomy per design) + `src/api/client.rs`: loopback guard + endpoint path constants for the four allowed paths; fetch implemented ONLY for `/liveclientdata/allgamedata` (D1 aggregate; W2 — no speculative fetchers).
- [x] 2.3 Trust store: `include_bytes!` pem → rustls root store; test asserts embedded pem digest == provenance SHA-256 and rejects an rcgen-generated untrusted chain (poller:R1/S2). Honest limitation: positive TLS acceptance is manual-checklist only (6.3).
- [x] 2.4 GREEN `src/api/source.rs`: `trait HttpSource { fn fetch(&self, path) -> Result<String, PollError> }` + reqwest-blocking impl with pinned roots, loopback-only URL (poller:R1/S1 via trust-wiring test; live accept manual).
- [x] 2.5 RED `tests/api/poller_tests.rs` with hand-rolled FakeClock: cadence clamped into 250–1000 ms; fast responses → ≤1 poll per window, never concurrent (poller:R2/S1); mocked 900 ms resolve → next poll starts only after resolve (R2/S2). Run CT → RED.
- [x] 2.6 GREEN `src/api/poller.rs`: strictly sequential cycle fetch→emit→`Clock::sleep_until` remaining cadence; clamp helper. Test: CT green.
- [x] 2.7 RED+GREEN classification: conn-refused/port-unbound → Lifecycle(NotInGame) (poller:R3/S1,S2); success → Snapshot; timeout/TLS/HTTP≥500/malformed → Transient(reason), lifecycle stays InGame·Degraded, last good snapshot retained (R3/S3, R4/S3). Test: CT green.

## Phase 3: App Shell & Event Loop

- [x] 3.1 RED `tests/app_tests.rs`: `App::on_msg(PollMsg)` FSM per D4 — first contact NotInGame→InGame (poller:R3/S1); refusal → NotInGame; Transient keeps InGame+Degraded(reason) with retained snapshot; mpsc drain is latest-wins per frame. Run CT → RED.
- [x] 3.2 GREEN `src/app.rs` (AppState, FSM, channel consumer) + `src/events.rs` (crossterm `q`/Esc keys, Resize).
- [x] 3.3 GREEN `src/main.rs`: spawn poller thread; enable_raw_mode + alternate screen; Drop guard AND panic hook (restore terminal, resume unwind); `q`/Esc → exit 0 (ui:R1/S1).
- [x] 3.4 RED/GREEN `tests/ui/shell_render_tests.rs` (ratatui TestBackend): shrink-resize → next frame drawn within new bounds, no element panics (ui:R1/S2). Test: CT green.

## Phase 4: Live Panels

- [ ] 4.1 RED `tests/ui/dashboard_tests.rs`: full-fixture Snapshot → buffer renders 10 panels grouped by team with champion, level, KDA, CS, items, spells(+exposed cooldowns) (ui:R3/S1). Run CT → RED.
- [ ] 4.2 GREEN `src/ui/dashboard.rs`: team-grouped grid; latest snapshot reflected next frame.
- [ ] 4.3 RED/GREEN degradation: partial_player → explicit placeholders ONLY on absent fields of that panel, others intact (ui:R3/S2); dead player shows exposed respawnTimer; isDead=true without timer → unknown marker, NEVER a derived countdown (R3/S3).
- [ ] 4.4 RED/GREEN local panel: currentGold 4350 renders on local pane (ui:R4/S1); absent gold → placeholder; gold value appears in NO non-local panel (R4/S2). Test: CT green.

## Phase 5: Ticker, Standby, Status Line

- [ ] 5.1 RED `tests/ui/ticker_tests.rs`: FirstBlood, TurretKilled, DragonKill{type Chemtech, stolen} listed with participants + exposed EventTime verbatim (ui:R5/S1); zero events → empty-state message, no failure (R5/S2). Run CT → RED.
- [ ] 5.2 GREEN `src/ui/ticker.rs` rendering the supported event types from the snapshot event list.
- [ ] 5.3 GREEN `src/ui/standby.rs`: NotInGame → standby view, silent, no error-loop output (ui:R2/S1,S2); NotInGame→InGame message swaps to live view on next drained frame (R2/S3).
- [ ] 5.4 RED/GREEN `src/ui/status.rs`: `RIOT_NOTICE` constant containing "not endorsed by Riot Games" + lifecycle state + last-update time, persistent in BOTH views (ui:R6/S1); Degraded(reason)/stale-snapshot surfaces as status-line/stale-banner annotation per D4 — rendering of existing FSM health, not a new subsystem (W3). Test: CT green.

## Phase 6: Verification & Compliance

- [ ] 6.1 Full pass: CT green; `cargo fmt` clean; `cargo clippy -j 1 -- -D warnings` under the canonical env prefix.
- [ ] 6.2 Compliance audit: grep model/api for computed timers/countdowns (must find none — design hard rule); gold local-only; ticker times verbatim; RIOT_NOTICE reachable from every view path. Record findings in PR description.
- [ ] 6.3 Write `docs/manual-verify.md`: Windows Terminal + classic conhost checklist — live-game TLS accept, mid-game disconnect/reconnect standby↔live swap, quit restores console (design manual/E2E row).

---

## Review Workload Forecast

- Estimated total changed lines: ~2400
- Per-phase line estimates: P0 ~120 · P1 ~700 (fixtures ~250, models+tests ~450) · P2 ~450 · P3 ~380 · P4 ~310 · P5 ~340 · P6 ~100
- Chained PRs recommended: Yes
- 400-line budget risk: High
- Decision needed before apply: No (delivery strategy auto-chain; proceed with Unit 1)
- Chain strategy: stacked-to-main (resolved by orchestrator at U1 apply start; greenfield, slices independent)

## Next Step

Ready for sdd-apply, Unit 1 (Phase 0+1) first.
