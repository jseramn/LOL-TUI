# Exploration: add-live-visualizations

**Change**: `add-live-visualizations` · **Phase**: explore · **Date**: 2026-08-25 · **Store**: hybrid (this file + Engram `sdd/add-live-visualizations/explore`)
**Scope**: Research-only investigation into replacing/augmenting the text rows of the live dashboard with TUI-native visualizations (bars, sparklines, mini-charts), grounded in the real Live Client Data API surface, the archived `add-live-match-dashboard` design constraints, and ratatui 0.30.2 / crossterm 0.29.0 rendering capabilities. No code was modified.

---

## Executive Summary

1. **The API already exposes everything needed for per-player bars and local-player trends — except history and enemy gold.** Per player: level (1–18), K/D/A, creep score, items (7 slots incl. trinket), isDead/respawnTimer, team. Local-only: `currentGold` plus a full stat block (HP, power, MS…). There are **no frame-to-frame deltas, no team gold totals, no positions** — anything time-series must come from an in-process rolling buffer.
2. **No-history reality confirmed**: `/allgamedata` returns one current snapshot per fetch; there is no live timeline endpoint (Match-V5 timeline is post-game only). One structural gift: **the event list in every snapshot is cumulative since GameStart**, so event histograms/heatmaps need no buffer at all — only numeric series do.
3. **A 120-frame ring buffer (~2 min @ 1 Hz) is trivially cheap**: ≈ 30 KB total for 10 players + local gold; insertion at 1 Hz; render-time conversion of ≤120 samples per chart is negligible at the existing ~20 Hz frame loop (`FRAME_POLL = 50 ms`).
4. **Terminal-host compatibility dictates marker choice**: classic Windows conhost has **no font fallback** (pre-Win11 22H2) and default fonts (Consolas, Lucida Console) lack Braille glyphs entirely; sextant/octant characters are so new that even Windows Terminal only gained built-in glyphs in 2026. **Block Elements (U+2580–U+259F: ▁▂▃▄▅▆▇█ ░▒▓) are safe on both hosts** → Sparkline, BarChart, Gauge, LineGauge, and Canvas with `Marker::Block`/`Marker::HalfBlock` are the compatible set; Braille/Sextant/Octant markers must be avoided.
5. **ratatui 0.30.2 ships every primitive required, zero new dependencies**: Sparkline (per-bar styling + `Option` absent values — a perfect match for our Option-everywhere model), BarChart (horizontal + grouped + multi-color bars), Gauge, LineGauge (custom symbols since 0.30), Canvas with new `FilledLine` area rendering (0.30.1+). Sparkline/BarChart received scaling-overflow and empty-chart panic fixes exactly at 0.30.1/0.30.2.

---

## Current State

The live view (`src/ui/dashboard.rs`) renders through a one-line vertical cursor (`Pen`, `src/ui/mod.rs`): `LIVE` headline → "Team ORDER"/"Team CHAOS" headers → one formatted text line per player (`{name} {champ} Lv{n} k/d/a CS{n} spell1+spell2 [DEAD(respawn t)] | Items: …`) → LOCAL strip (gold + HP/power/MS) → EVENTS ticker rows (`@{EventTime} {TYPE} …`) → bottom-row status line with `RIOT_NOTICE` drawn last in every view. Data arrives as normalized `Snapshot` (`src/model/snapshot.rs`) from tolerant DTOs (`src/model/live_data.rs`); absence is preserved as `None` end-to-end. Render loop ticks at up to 20 Hz (`FRAME_POLL = 50 ms`); poller cadence clamps to 250–1000 ms (default 1000 ms). Everything visual today is plain `Paragraph` text.

## Affected Areas

- `src/ui/dashboard.rs` — primary rewrite target: from Pen text lines to region-based layout hosting widgets.
- `src/ui/mod.rs` — `Pen` abstraction insufficient for sub-areas; needs `Layout`-based regions while preserving "status line drawn last" guarantee.
- `src/model/snapshot.rs` — likely additions for windowed/event-bucket derivations kept pure (no timers).
- New module (candidate: `src/history.rs` or `src/app/history.rs`) — rolling frame buffer fed by each accepted snapshot; cleared on lifecycle transitions.
- `tests/ui/*` — buffer assertions extend to widget output; extreme-small-viewport tests must cover charts.
- `docs/manual-verify.md` — add conhost visual-pass checklist items for block-glyph rendering.

---

## RQ1 — Real-API Data Exposure vs Visualization Potential

Cross-checked against the official Live Client Data API docs (developer.riotgames.com, Game Client APIs section + official sample JSON), the `lol-game-client-api` crate models (v0.1.8), our recorded fixture `tests/fixtures/allgamedata/full.json`, and our DTOs.

| Field | In payload? | Where | Parsed by us? | Visualization potential |
|---|---|---|---|---|
| Per-player `level` | Yes | `allPlayers[].level` | ✅ `PlayerSnapshot.level` | Bars/gauges scaled 1–18 |
| Per-player K/D/A | Yes | `allPlayers[].scores.{kills,deaths,assists}` | ✅ | Multi-color mini-bars, ratio bar |
| Per-player CS (`creepScore`) | Yes (f64 in real captures) | `allPlayers[].scores.creepScore` | ✅ | Horizontal bars; ring-buffer trend |
| Per-player items | Yes (slots 0–5 + 6 trinket) | `allPlayers[].items[]` | ✅ | Inventory fill-ratio bar |
| Per-player `team` | Yes (`ORDER`/`CHAOS`) | `allPlayers[].team` | ✅ (normalized enum) | Team grouping/colors |
| Per-player death state / respawn | Yes | `isDead`, `respawnTimer` | ✅ | Status glyph; **no countdown bars** (derived-timer ban) |
| Per-player gold | **NO — never** | — | n/a | Structurally impossible for all 9 non-local players |
| Ally/enemy team gold totals | **NO** | Only `activePlayer.currentGold` exists | ✅ local only | Team gold charts impossible; sum would fabricate |
| Local stat block | Yes | `activePlayer.statistics` (+championStats in docs) | ✅ subset (`Statistics`) | HP & power Gauges (local panel only) |
| Timestamps | `gameData.gameTime` per fetch; `events[].EventTime` per event; `respawnTimer` | — | ✅ verbatim | Window filtering (x-axis anchors), never countdowns |
| Frame-to-frame deltas | **NO** — each fetch is a standalone current-state snapshot; no sequence ids, no diffs | — | n/a | Must be computed in-process from retained frames (presentation arithmetic over exposed values) |
| Positions / minimap coords | **NO** | — | n/a | Map-style visuals impossible |

Completeness note vs docs: our DTOs deliberately drop fields irrelevant to visualization (runes, abilities beyond local, `skinID`, `wardScore` is parsed). Two doc-vs-model deltas worth recording: official sample shows `championStats` naming where fixtures use `statistics`, and richer stats keys (`armorPenetration*`, `critDamage`, `tenacity`, `resourceValue/resourceMax/resourceType`) that we don't model — none block any candidate below; extending `Statistics` is additive if HP/power prove popular.

## RQ2 — No-History Reality & Ring-Buffer Feasibility

- Every endpoint under `/liveclientdata/*` answers with the **current** state; there is **no historical or timeline endpoint during a live game** (Match-V5 `/timeline` requires an API key and only exists post-game — out of scope per the archived proposal's live-only pivot).
- **Events are the exception**: each snapshot carries the full cumulative event list since `GameStart`. Windowed event views ("last 60 s") can filter this list per frame using two exposed values (`EventTime` ≤ `gameTime`, > `gameTime − 60`). No storage needed. (Design phase must bless this as presentation-only arithmetic — it fabricates nothing hidden.)
- **Numeric series need our own buffer.** Cap analysis at 120 frames (~2 min @ 1 Hz):
  - Per-player series (cs, kills, deaths, assists, level): 10 × 120 × ~16–24 B ≈ **≤ 29 KB**
  - Local gold series: 120 × 8 B ≈ **1 KB**
  - Event buckets: zero (cumulative list, see above)
  - Insert cost: one append per accepted snapshot (≤ 1 Hz), O(1) ring write; render reads ≤120 samples/charts/frame at 20 Hz — noise-level CPU.
  - Semantics to settle in design: buffer reset on lifecycle `InGame→NotInGame→InGame` (a reconnect may be the *same* game — stale mixing risk), warm-up behavior when < 2 samples (render explicit placeholder, never an empty-looking flat line).

## RQ3 — ratatui 0.30.2 Rendering Primitives (zero new dependencies)

| Primitive | What it gives us | 0.30.x notes | Conhost-safe? |
|---|---|---|---|
| `Sparkline` | One-row (or n-row) level-bar trend from `&[u64]`, `Vec<Option<u64>>`, or styled `SparklineBar`s; `.max()`, direction, absent-value style/symbol | Breaking change in 0.29: `data()` takes `IntoIterator<Item = SparklineBar>`; per-bar styles + `Option` absent values are available in 0.30.2; u128 scaling-overflow fix landed 0.30.2 | ✅ Default symbols are Block-Element level sets (▁▂▃▄▅▆▇█) |
| `BarChart` / `Bar` / `BarGroup` | Vertical, **horizontal** (`BarChart::horizontal`), and **grouped** (`BarChart::grouped`, added 0.30) charts; per-bar label/value/style; value text inside horizontal bars | Empty-horizontal-chart panic fixed in 0.30.2; labels accept `Into<Line>` | ✅ Fills with block chars |
| `Gauge` | Single horizontal percentage fill with centered label | Long-stable | ✅ Full-block fill |
| `LineGauge` | Thin one-row gauge with line symbols + label | Custom `filled_symbol`/`unfilled_symbol` since 0.30 (`line_set` deprecated) | ✅ Box-drawing range |
| `Canvas` | Float-space drawing: lines, rectangles, points; `FilledLine` area fills + `GraphType::Area` with `Dataset::fill_to_y` (both new 0.30.1/0.30.2) | `Marker` now non-exhaustive; new Quadrant/Sextant/Octant markers exist but see right | ⚠️ Only with `Marker::Block` / `Marker::HalfBlock` / `Marker::Dot`; **Braille/Sextant/Octant are NOT conhost-safe** (below) |
| Symbols inventory | `symbols::block::{FULL, SEVEN_LEVELS, NINE_LEVELS}` (U+2581–2588), `symbols::shade` (░▒▓█), `symbols::line::*`, half blocks ▀▄█ (U+2580/84/88) | All within Box-Drawing + Block-Elements ranges | ✅ Present in Consolas/Cascadia/Most mono fonts |
| Known crossterm-backend issues | None specific to these widgets found; the documented failure mode is font-level (glyph coverage), not backend-level | — | Font coverage is the constraint |

**Host compatibility matrix (evidence: microsoft/terminal #18042/#12314/#18024, ratatui #457, WT PR #19841):**

| Glyph class | Range | Windows Terminal | Classic conhost (Win10, Consolas/Lucida) |
|---|---|---|---|
| Block elements + shades | U+2580–U+259F | ✅ | ✅ (in base fonts; no fallback needed) |
| Box drawing | U+2500–U+257F | ✅ | ✅ |
| Braille patterns | U+2800–U+28FF | ✅ (Cascadia) | ❌ renders `?` — no font fallback pre-Win11 22H2 |
| Sextants/octants (Legacy Computing Supp.) | U+1FB00+ | ⚠️ built-in glyphs only merged 2026-02 | ❌ |

## RQ4 — Constraints Honored (carried from archived design + Riot Game Integrity policy)

1. **No derived timers/countdowns**: respawn progress bars, "time until X" gauges, and anything decrementing locally are **rejected candidates** regardless of feasibility. Window arithmetic (RQ2) uses exposed values only and produces static aggregates, never clocks.
2. **Gold exclusively local player**: gold sparklines/bars bind to `LocalPlayerSnapshot.current_gold` only; the roster type has no gold field so violations are structurally impossible (verified: buffer-lock test precedent from archived change).
3. **Notice persistence**: any layout restructure keeps `status::render` drawn last on the bottom row in every view — existing buffer-lock tests pin this; chart regions must not overlap the status row.
4. **Windows Terminal AND conhost**: restrict to Block-Elements/Shades/Box-drawing glyphs (matrix above); Canvas only with Block/HalfBlock markers; colors degrade to 16-palette gracefully (conhost VT truecolor varies) — degrade color, never features.
5. Absence semantics: charts render explicit placeholders when data is absent or buffer not warmed (< 2 samples); `Option<u64>` maps directly onto Sparkline's absent-value support.

## RQ5 — Visualization Candidate Inventory (ranked)

Feasibility tiers weigh data availability (RQ1) + history availability (RQ2) + widget fit and host safety (RQ3).

| # | Candidate | Data source | Tier | Notes |
|---|---|---|---|---|
| 1 | **Rolling gold sparkline, local player (~2 min)** | Ring buffer of `current_gold` (120 f64 → u64) | **High** | Flagship "graphics" win; Sparkline w/ absent-value support; local-only by construction |
| 2 | **Horizontal CS bars per player (team-grouped)** | `creep_score` per player | **High** | `BarChart::horizontal` or inline block-char bars; f64→u64 truncation policy needed |
| 3 | **Level bars/gauges per player** (fixed 1–18 scale) | `level` | **High** | Fixed max avoids rescale jitter between frames |
| 4 | **K/D/A tri-bars per player** (kills green, deaths red, assists blue) | scores | **High** | `BarChart::grouped` or three inline colored segments |
| 5 | **Local HP & resource gauges** | `stats.current_health/max_health`, `power/power_max` | **High** | Already parsed; Gauge pair on LOCAL strip; local-only field so compliant |
| 6 | **Inventory fill ratio per player** (filled slots/6) | `items[]` slot count (excl. trinket) | **High** | Mini-Gauge or 6-cell block strip; absent list → unknown placeholder |
| 7 | **KDA ratio bar per player** | k/d/a | Medium | Presentation arithmetic; deaths=0 policy needed (cap/convention — never fabricate) |
| 8 | **Team event histogram, last 60 s** (kill/monster/structure buckets) | Cumulative event list filtered by `EventTime` vs `gameTime` | Medium | No buffer needed; killer-name→team mapping via roster; derivation-policy sign-off required |
| 9 | **Cumulative kills-per-team bar over rolling window** | ChampionKill events + roster mapping | Medium | Same windowing caveat as #8 |
| 10 | **CS trend sparklines grid (all 10 players)** | Per-player CS ring buffers | Medium | Cheap perf-wise; layout-heavy (10 charts); consider ally-team only to halve |
| 11 | **Event heatmap/matrix** (type × time-bucket intensity via ░▒▓█) | Cumulative events bucketed by `EventTime` | Low-Medium | Visually rich but dense for small terminals; shade chars are conhost-safe |
| 12 | **Gold delta/trend indicator** (▲▼ + delta over window) | Gold ring buffer endpoints | High | Trivial once #1's buffer exists |
| — | Respawn progress bars / countdowns | `respawnTimer` | **Rejected** | Derived timer — Game Integrity violation (design hard rule) |
| — | Any enemy/team gold chart | — | **Rejected** | API-impossible; summing would fabricate |
| — | Minimap/position visuals | — | **Rejected** | No position data exposed |

## RQ6 — Performance & Layout Impact

- **CPU/render budget**: frame loop already runs at up to 20 Hz redrawing ~20 Paragraphs. Each Sparkline/Gauge/bar redraw touches its own Rect's cells once; ratatui's diff flush means only changed cells reach the terminal. Ten sparklines ≈ a few thousand cell updates worst case — well within budget. A single Canvas (gold area chart, HalfBlock) rasterizing ~80×8 cells from ≤120 float points is also fine at 20 Hz. The pathological case (multiple large Canvases with thousands of points each) has no candidate requiring it.
- **Layout restructure is the real work**: `Pen` is a strict top-down line cursor; widgets need `Rect`s. Expected shape: split the live area into header / team panels / local strip / charts row(s) / ticker / status via `Layout`, keeping status drawn last. Small viewports must degrade by hiding/shrinking chart regions before ever clipping the notice — extend existing shrink/extreme-small tests accordingly.
- **Memory**: ring buffers ≤ ~30 KB total (RQ2) — negligible.
- **Borrow/data-shape friction**: Sparkline wants owned/iterable u64 data per frame; converting ≤120-sample windows per chart per frame allocates small vecs — acceptable, but prefer reusable buffers if profiling ever disagrees.

## Approaches

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **A. Inline ASCII/block bars in Pen strings** | Hand-roll bars (█░ repeated) inside existing text lines | Zero layout refactor; minimal diff | Duplicated scaling logic everywhere; crude visuals; misses Sparkline/Gauge quality; still needs history module | Low-Med |
| **B. Native ratatui widgets + Layout restructure (recommended)** | Sparkline/BarChart/Gauge/LineGauge in Layout regions; history module feeds them; Canvas optional for gold area later | Idiomatic, library-tested primitives; absent-value support matches model; best visual payoff per LOC; zero new deps | Dashboard rewrite; viewport degradation matrix to define/test | Medium |
| **C. Canvas-first custom charts** | All charts as Canvas drawings | Max expressiveness (FilledLine areas, axes) | Marker-compatibility discipline required (no Braille/Sextant defaults); more code, more perf care; overkill for bars | High |

## Recommendation

**Approach B.** Land a `History` ring-buffer module first (pure, offline-testable), then convert the dashboard to Layout regions hosting native widgets: start with candidates #1, #2/#3, #4, #5, #6 (all High tier, no derivation-policy ambiguity), defer #7–#11 behind a design-phase decision on window-arithmetic presentation policy. Keep Canvas out of scope initially; revisit for a gold area chart only after the widget set proves the layout. This maximizes visible "graphics" payoff while structurally preserving every compliance invariant.

## Risks

- **conhost glyph regression** (highest): any accidental Braille/Sextant usage renders `?` on classic hosts; mitigate via marker whitelist + manual-verify checklist item on both hosts.
- **Derived-info boundary creep**: windowed histograms (#8/#9) are compliant as static aggregates of exposed values, but implementation drift (e.g., interpolating "next objective" or countdowns) would violate Game Integrity — needs an explicit spec scenario set fencing it.
- **Viewport degradation regressions**: charts need minimum sizes; without a hide-threshold matrix, tiny terminals could clip the mandatory notice (spec R6 violation).
- **Buffer lifecycle semantics**: reconnect-same-game vs new-game buffer reset is observable UI behavior; wrong choice shows misleading trends (not a compliance issue, a correctness one).
- **f64→u64 truncation** for Sparkline/BarChart inputs (CS 195.5, gold 1234.56): pick and document floor/round once; silent inconsistency across charts would look buggy.

## Ready for Proposal

Yes. Recommended scope statement: *augment the live dashboard with TUI-native visualizations powered solely by exposed snapshot fields plus an in-process 120-frame ring buffer; ship High-tier candidates (local gold sparkline, per-player CS/level/KDA bars, local HP/power gauges, inventory fill) with block-element-safe rendering for Windows Terminal AND conhost; explicitly reject respawn countdowns, enemy/team gold, and position visuals; defer windowed event analytics pending a derivation-policy decision.*

## Sources

- Official Live Client Data API docs + sample JSON: https://developer.riotgames.com/docs/lol#game-client-api · https://static.developer.riotgames.com/docs/lol/liveclientdata_sample.json
- Rust models cross-check: https://docs.rs/lol-game-client-api/latest/src/lol_game_client_api/model.rs.html
- ratatui 0.30 highlights (widgets/markers/LineGauge): https://ratatui.rs/highlights/v030/
- ratatui 0.30.1/0.30.2 changelog (FilledLine, Fill, BarChart empty-chart fix, scaling overflow fix): https://github.com/ratatui/ratatui/blob/main/CHANGELOG.md
- ratatui breaking changes (Sparkline IntoIterator 0.29; Marker non-exhaustive 0.30): https://github.com/ratatui/ratatui/blob/main/BREAKING-CHANGES.md
- Widget APIs (Sparkline/SparklineBar/BarChart/Bar): https://docs.rs/ratatui/0.30.0/ratatui/widgets/struct.Sparkline.html · https://docs.rs/ratatui/0.30.0/ratatui/widgets/struct.BarChart.html
- Marker/font support: https://docs.rs/ratatui/latest/ratatui/symbols/enum.Marker.html · microsoft/terminal #18042, #18024, #12314 (conhost font fallback), WT sextant glyphs PR #19841 · ratatui #457 (braille on cmd)
