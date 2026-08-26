# Archive Report: add-live-match-dashboard

**Change**: `add-live-match-dashboard` · **Archived**: 2026-08-25 · **Store**: hybrid (this file + Engram `sdd/add-live-match-dashboard/archive-report`)
**Verify snapshot**: main @ `0440eeb70f78741ead712fe020d98bea6b163467` · **Closure HEAD**: main @ `1d4d949d827cc7eec965f351d365a9316b9221c6`
**Final verdict**: PASS WITH WARNINGS at verify → CLOSED WITH POST-VERIFY REMEDIATION (schema drift fixed and validated live in-game)

## Summary

Greenfield live-only League of Legends TUI dashboard fed solely by the Live Client Data API (`https://127.0.0.1:2999`, keyless). All 29 tasks across 6 work units were implemented under strict TDD in a stacked-to-main chain, independently verified (65/65 tests, build, fmt, clippy green on re-execution; 11/11 requirements, 25/25 scenarios accounted), then hardened after verify when a real in-game session exposed schema drift between recorded fixtures and the actual API. The fix landed on main and the dashboard was confirmed working end-to-end during a live game. Specs are synced to `openspec/specs/{live-client-poller,live-dashboard-ui}/spec.md`; this change folder is archived as the cycle's audit trail.

## Outcome

- **Verify verdict**: PASS WITH WARNINGS — 0 CRITICAL / 5 WARNING / 5 SUGGESTION. Gates re-executed independently at main @ `0440eeb`: CT 65 passed / 0 failed (api 19, app 17, model 8, ui 21) exit 0; CB exit 0; `cargo fmt --all -- --check` exit 0; `cargo clippy -j 1 --all-targets -- -D warnings` exit 0. Compliance matrix: 23/25 scenarios COMPLIANT by covering tests; the 2 PARTIAL scenarios are exactly the design-disclosed offline-impossible manual items.
- **Post-verify remediation (final state)**: a live in-game validation surfaced schema drift the offline fixture suite could not detect:
  - Real payloads wrap events as `{"events": {"Events": [...]}}`; legacy fixtures used a bare array. Fixed by an `EventsShape` untagged enum accepting both shapes via `Deref`.
  - Real payloads renamed `gameStats` to `gameData`. Fixed with `#[serde(rename = "gameData", alias = "gameStats")]`.
  - Fix committed as `f0a1c64 fix(model): accept real Live Client Data API schema` (touches `src/model/live_data.rs`, `src/model/snapshot.rs`, `tests/model/live_data_tests.rs`, `docs/manual-verify.md`, `.gitignore`). Per Final-State Authority, the launch prompt's closure facts outrank intermediate snapshots: the end-to-end live confirmation postdates the verify report.
- **Live validation result**: manual in-game run confirmed the dashboard working end-to-end — LIVE header, ORDER/CHAOS team panels, LOCAL gold strip, EVENTS ticker, Riot non-endorsement notice all rendered from real game data.
- **Warning disposition**: W1 retired by live evidence (see Risk Retirement); W2 partially retired; W3–W5 (documentation duties) fulfilled by this report's Deviations section.

## Implementation Trace

Stacked-to-main chain, strict TDD, each unit merged `--no-ff` to main:

| Unit | Scope | Merge commit | Suite at close |
|---|---|---|---|
| U1 | Deps, vendored `riotgames.pem` w/ provenance SHA-256, module skeleton, fixture corpus (`full`/`partial_player`/`empty_events`/`malformed`), tolerant Option-everywhere DTOs, normalized Snapshot | `7501fec` | 8 passed |
| U2 | Poller core: loopback-only client (construction guard), `NotBound | Transient{Timeout,Tls,Http,Parse}` taxonomy, pinned-cert rustls trust store, `HttpSource` seam, non-overlapping clamped scheduler over `Clock` seam | `f763329` | 27 passed |
| U3 | App FSM (`NotInGame ∨ InGame{Healthy|Degraded}`), mpsc latest-wins drain, crossterm keys/resize, RAII terminal restore + panic hook, shell render tests | `124b2fb` | 47 passed |
| U4 | Live views: LIVE headline, ORDER/CHAOS panels ×10 with per-field degradation, LOCAL gold exclusivity, no derived timers | `608267d` | 56 passed |
| U5 | Event ticker (verbatim exposed times, stolen flags, empty state), dedicated standby module, persistent status line + RIOT_NOTICE drawn last in both views | `db39366` | 65 passed |
| U6 | Verification unit: fmt/clippy remediation under green (incl. `PollMsg::Snapshot(Box<Snapshot>)` refactor), compliance grep audit, `docs/manual-verify.md` checklist | `0440eeb` | 65 passed |

Post-verify remediation commits (after verify snapshot @ `0440eeb`):

- `f0a1c64 fix(model): accept real Live Client Data API schema` — EventsShape untagged enum (wrapped/bare events), `gameData` rename + `gameStats` alias; model tests adjusted; manual checklist updated (+29 lines).
- `f50e911 chore(examples): add live_probe and parse_captured debugging aids` — offline debugging aids for probing/parsing captured payloads.
- `1d4d949 chore(ops): track sdd artifacts and project config` — SDD planning docs tracked.

## Spec Coverage Matrix

Evidence = committed tests named in the verify compliance matrix plus post-archive live evidence.

### Capability `live-client-poller` (5 requirements / 12 scenarios) — synced to `openspec/specs/live-client-poller/spec.md`

| Requirement | Committed evidence |
|---|---|
| R1 Loopback-only TLS w/ Riot cert | S2/S3 ✅ `client_tests > untrusted_certificate_chain_is_rejected_as_transient_tls`, `non_loopback_hosts_are_refused_at_construction` (5 hosts incl. `localhost`, `::1`, `0.0.0.0`), pem digest/provenance tests. S1 positive acceptance: design-disclosed offline-impossible; retired by live in-game evidence (real handshake through pinned-cert client delivered live data). |
| R2 Non-overlapping scheduling | ✅ `poller_tests > fast_responses_fire_at_most_once_per_window`, `slow_response_never_overlaps_next_cycle`, `poller_uses_clamped_cadence` (7-case clamp boundaries). |
| R3 Lifecycle detection | ✅ first-contact/disconnect/reconnect scripts, `refused_connection_classifies_as_not_bound` over a real socket, transient-failure matrix keeps InGame·Degraded. |
| R4 Normalized snapshot deserialization | ✅ full/partial/malformed fixture tests (`live_data_tests`, `snapshot_tests`); EXTENDED post-verify by `f0a1c64` to accept the real wrapped-events shape and `gameData` key without breaking legacy fixtures. |
| R5 Fixture-driven offline testability | ✅ whole suite runs green offline via canonical env-prefixed CT (no port 2999; only ephemeral 127.0.0.1:0 test sockets). |

### Capability `live-dashboard-ui` (6 requirements / 13 scenarios) — synced to `openspec/specs/live-dashboard-ui/spec.md`

| Requirement | Committed evidence |
|---|---|
| R1 Shell & event loop | S2 ✅ shrink-resize reflow + extreme-small-viewport tests. S1 quit logic ✅ (`q_press_quits`, `escape_press_quits`, key-release ignored, ExitCode::SUCCESS); physical console restore remains manual-checklist (see Risks). |
| R2 Standby view | ✅ silent startup standby, mid-game disconnect fallback, reconnect restores live view next frame ≤1 cycle. |
| R3 All-player panels | ✅ 10 panels exactly once, team-grouped, every exposed field, per-field placeholders ONLY where absent, dead player shows exposed respawn or unknown marker (never a countdown). |
| R4 Local-player panel | ✅ `local_gold_renders_on_the_local_strip` (spec value Gold 4350); missing-gold placeholder + buffer-wide proof that no non-local panel shows gold; structurally impossible otherwise (no gold field on roster structs). |
| R5 Event ticker | ✅ verbatim participants/times incl. stolen Chemtech DragonKill, section-order lock, empty-state message. |
| R6 Notice + status line | ✅ single `RIOT_NOTICE` const with mandated phrase; dispatcher draws status LAST unconditionally; buffer locks prove visibility in standby, live, degraded, post-resize frames. |

## Deviations Adjudication

Carried from verify adjudication (none CRITICAL; documentation duties discharged here):

| # | Deviation | Adjudication |
|---|---|---|
| a | Summoner spells rendered as display names, no cooldowns | SPEC-FAITHFUL — Live Client Data API exposes NO spell-cooldown fields ("with cooldowns as exposed" ⇒ nothing exposed ⇒ nothing rendered; zero cooldown code exists). Recorded. |
| b | Respawn/event times printed via f64 Display (34.0 → `34`) | ACCEPTABLE — value fidelity preserved; fixed-format printing would fabricate precision absent from the payload. Recorded. |
| c | `ApiClient::with_fetch_timeout` constructor knob | JUSTIFIED — hardened hosts delay loopback refusal ~2 s, which would misreport idle as `Transient(Timeout)` instead of `NotBound` at any fixed default; production default stays 800 ms. No spec requirement touched. Recorded. |
| d | `PollMsg::Snapshot(Box<Snapshot>)` after clippy `large_enum_variant` | BENIGN behavior-preserving refactor (channel payload 448 B → 16 B); suite green before/after. |
| e | Uniqueness scans scoped above EVENTS header | CORRECT — ticker participant names legitimately embed champion substrings (`SyndraGod` ⊃ `Syndra`); scoping preserves R3/S1 intent. |
| F | Post-verify: models accepted only legacy fixture shape (bare `events` array, `gameStats` key) while the real API wraps `{"events": {"Events": [...]}}` and uses `gameData` | RESOLVED — `EventsShape` untagged enum accepts both shapes via Deref; serde rename/alias bridges `gameData`/`gameStats`. Root cause: fixture corpus recorded/prepared before real-game capture; offline TDD could not observe the drift. Fix `f0a1c64`; live-validated in-game same day. |

## Risk Retirement

Disposition of each verify-report WARNING against final state:

1. **poller:R1/S1 positive TLS acceptance unproven offline — RETIRED BY LIVE EVIDENCE.** A real in-game session ran the poller against the actual game client; live data reached the dashboard (the schema-drift bug was discovered precisely because real payloads flowed through fetch→parse, and the post-fix run confirmed end-to-end rendering). Successful TLS acceptance against the self-signed cert chaining to the vendored Riot root is entailed by that data flow. Formal §8 checklist sign-off was not separately archived; the substantive risk (pinned-cert handshake fails in vivo) did not materialize.
2. **ui:R1/S1 physical console restore (raw mode off, alt screen left) — PARTIALLY RETIRED / STILL OPEN.** The closure context attests end-to-end in-game dashboard operation but does not attest the specific §6 (quit restores console on Windows Terminal AND classic conhost) and §7 (panic restore) sub-checks. Remains a pending user checklist item; non-blocking.
3. **Deviation (a) spells-without-cooldowns — DOCUMENTED** (table above).
4. **Deviation (b) f64 Display formatting — DOCUMENTED** (table above).
5. **Deviation (c) `with_fetch_timeout` — DOCUMENTED** (table above).

SUGGESTIONS carried forward (non-blocking): redundant disjunction `poller_tests.rs:199`; no dedicated `InhibRespawned` variant (lossless via `Other{name,time}`); one-line post-resize notice assert would close the last buffer-assert gap.

## Rollback Plan

Greenfield single-crate binary with feature-isolated seams; no migrations, no external state. Roll back by reverting merge commits in isolation (`git revert -m 1 <merge>`); the post-verify model fix reverts alone via `git revert f0a1c64` (model tolerance + docs only; poller/UI untouched). Archive move and spec sync are additive documentation operations — revert by moving the archive folder back and deleting `openspec/specs/{live-client-poller,live-dashboard-ui}/`.

## Follow-ups

1. Execute remaining `docs/manual-verify.md` sub-checks not yet archived — §6 quit-restores-console on BOTH Windows Terminal and classic conhost, §7 panic restore — and formally log §1–§9 execution; resolves the legacy-conhost alt-screen open question from design.md.
2. Consider refreshing the fixture corpus with real captured payloads (`examples/live_probe.rs` / `examples/parse_captured.rs` aid capture) so R4/S1 exercises the real wrapped-events shape natively rather than through the compatibility shim added in `f0a1c64`.
3. Optional polish: simplify `poller_tests.rs:199` disjunction, add post-resize notice buffer assert, add a named `InhibRespawned::GameEvent` variant if richer formatting is ever needed.
4. All work is local on main (NOT pushed) — push/remote setup is an orchestrator/user decision.
5. Deferred v2 candidates (per proposal): LCU integration, optional settings file (storage location deliberately unresolved).

## Traceability — Engram observation IDs read at archive time

Full reads: #7 proposal · #10 specs · #11 design · #13 tasks · #14 apply-progress · #20 verify-report. Surfaced via search (previews): #6 exploration · #9 scope pivot · #12 design validation · #15/#17/#18 unit session summaries. This report persisted as topic_key `sdd/add-live-match-dashboard/archive-report`.
