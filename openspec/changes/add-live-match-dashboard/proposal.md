# Proposal: League of Legends Live Match Dashboard (TUI)

**Change**: `add-live-match-dashboard` · **Date**: 2026-08-25 · **Basis**: exploration.md + user scope pivot 2026-08-25 (live-only amendment)

## Intent

The product is only what can be seen in real time while playing: a terminal operational dashboard fed solely by the Live Client Data API (`https://127.0.0.1:2999`) during the user's own game. Per explicit user decision, post-game analytics are out entirely; no Riot API key exists or will be requested — port 2999 is keyless.

## Scope

### In Scope

- App shell + ratatui/crossterm event loop.
- Resilient single-source poller: ~250–1000 ms, strictly non-overlapping; server binds only mid-game, so connection refused = game-lifecycle detector (idle/standby panel ↔ live view).
- Live panels for all 10 players: champion, level, KDA, CS, items, spells (+ summoner-spell cooldowns as exposed), respawn timers.
- Local-player extra detail: gold, KDA/stat detail as exposed by `/allgamedata` + `/playerlist`.
- Objective & kill event ticker (kills, first blood, turrets, dragon/herald/baron, inhibs).
- Mandatory "not endorsed by Riot Games" notice; client-exposed data only.

### Out of Scope

- Post-game analytics / Match-V5 / timeline (user-declared out of scope).
- Spectator-V5 anything; LCU integration (v2 candidate).
- Enemy live gold (API-impossible); hidden-timer derivation (Game Integrity violation).
- Any Riot API-key handling; production distribution.
- Settings file deferred: zero-config MVP. A future optional config stays open WITHOUT resolving its storage location — prior C:/E: conflict dissolved, flagged as non-issue.

## Capabilities

> CONTRACT for sdd-spec. Greenfield: `openspec/specs/` does not exist yet.

### New Capabilities

- `live-client-poller`: port-2999 client (embedded `riotgames.pem` TLS trust), non-overlapping poll scheduler, game-lifecycle detection, serde models proven against recorded JSON fixtures.
- `live-dashboard-ui`: shell/event loop, all-10 player panels, local-player pane, event ticker, idle/live states, status line + Riot boilerplate.

### Modified Capabilities

None — greenfield. Pre-amendment `post-game-analytics` / `credential-configuration` capabilities are cancelled with this pivot.

## Approach

Strict-TDD re-slicing; each slice fits one session and needs no live game to develop:

| Slice | Content |
|---|---|
| 1 | Poller client + fixture-driven tests against recorded port-2999 JSON payloads |
| 2 | Normalized metrics model + player panels rendering |
| 3 | Event ticker + lifecycle/idle states |
| 4 | Polish/layout |

Design-phase decisions (non-blocking): aggregate `/allgamedata` vs per-endpoint calls; hand-rolled reqwest+serde vs `lol-game-client-api` models.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `Cargo.toml`, `src/` | Modified | Bootstrap extends into poller/metrics/ui modules |
| `tests/fixtures/` | New | Recorded live-client JSON payloads |

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| Self-signed TLS friction | Med | Embedded `riotgames.pem`; localhost-only insecure fallback behind explicit flag |
| Poll cadence vs game performance | Low | Non-overlapping schedule, next poll after resolve; ≤1 Hz default |
| Windows console host quirks | Med | Canonical env-prefixed `cargo test -j 1` from config.yaml, verbatim |
| ToS Game Integrity boundary | Low | Exposed data only; no derived timers; visible boilerplate |

## Rollback Plan

Greenfield single-crate binary with feature-isolated module seams; each slice lands as independent conventional commits — revert that slice's commits. No migrations, no external state; fixtures are plain files. Overall risk: low.

## Dependencies

None external. Manual verification needs a running LoL game client; automated tests need only recorded fixtures.

## Success Criteria

- [ ] In own game: all-10 panels refresh within one poll cycle (≤1 s default).
- [ ] Connection refused → clean idle/standby panel, never an error loop.
- [ ] Fixture-driven suite green via canonical env-prefixed command; strict TDD observed.
- [ ] Zero credentials anywhere; Riot boilerplate visible; no derived hidden info.
