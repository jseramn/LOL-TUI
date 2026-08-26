# Live Dashboard Visualizations Specification

**Change**: `add-live-visualizations` · **Phase**: spec · **Date**: 2026-08-25 · **Store**: hybrid (this file + Engram `sdd/add-live-visualizations/spec`)

## Purpose

TUI-native charts layered on the live dashboard. Purely additive over `live-dashboard-ui`: every existing requirement there — including the R6 Riot non-endorsement notice guarantee — remains unchanged and MUST survive any layout this capability introduces. Consumes normalized snapshots from `live-client-poller`; renders only during IN_GAME; feeds only from accepted snapshots (parse errors retain the last good snapshot). All glyphs are classic-conhost-safe; absence stays explicit — no fabricated values, no derived timers, gold only for the local player.

## Requirements

### Requirement: Glyph whitelist sole source

The system SHALL export every chart glyph from one whitelist module restricted to box drawing (U+2500–U+257F) and Block Elements/shades (U+2580–U+259F). Braille (U+2800–U+28FF) and sextant/octant codepoints MUST NOT be exported or drawn. Enforcement SHALL be static: widgets reference exported constants only, runtime glyph construction MUST NOT occur. Static is chosen over start-of-render bail: zero per-frame cost, violations surface in offline tests.

#### Scenario: Forbidden ranges never exported

- GIVEN the whitelist module's exported glyph set
- WHEN a unit test enumerates it
- THEN every codepoint lies in U+2500–U+257F or U+2580–U+259F

#### Scenario: Widget output stays whitelisted

- GIVEN any single widget rendered to an offline TestBackend
- WHEN the buffer is scanned
- THEN every symbol cell holds a whitelisted codepoint or printable ASCII label text

### Requirement: Region-based layout preserving R6

During IN_GAME the live view SHALL split vertically: header; ORDER column and CHAOS column side by side; local strip; event ticker; status row LAST. Full layout targets ≥ 80×24 cells; minimum acceptable viewport is 80×12. This refactor is presentation-only: `live-dashboard-ui` R6 MUST hold unchanged — the notice-bearing status row occupies the last row in every view.

#### Scenario: Canonical region order

- GIVEN an 80×24 viewport with a complete snapshot
- WHEN a frame renders
- THEN regions appear top-to-bottom in declared order and the Riot notice fills the final row

#### Scenario: Notice survives minimum viewport

- GIVEN an 80×12 viewport
- WHEN a frame renders
- THEN the status row remains last, visible, unoverlapped

### Requirement: Per-team creep score bars

Each team column SHALL render one horizontal CS bar per player — ORDER left, CHAOS right — length proportional to creepScore versus the highest CS among visible players, converted via the shared truncation policy. Absent creepScore SHALL render a `?` placeholder, never an empty bar.

#### Scenario: Shared-maximum scaling

- GIVEN visible players with CS 40, 120, 300
- WHEN a frame renders
- THEN the 300 bar spans full width; 40 and 120 render proportionally shorter

#### Scenario: Missing CS placeholder

- GIVEN one player with absent creepScore
- WHEN a frame renders
- THEN that row shows `?`; other bars render normally

### Requirement: Level bars on fixed scale

Each player SHALL show a level bar scaled to fixed 1–18 (no inter-frame rescaling). Out-of-range values clamp; absent level renders `?`.

#### Scenario: Fixed endpoints

- GIVEN players at level 1 and level 18
- WHEN a frame renders
- THEN level 1 draws near-empty, level 18 full

#### Scenario: Overrange clamps safely

- GIVEN a snapshot reporting level 25
- WHEN a frame renders
- THEN the bar draws full and no panic occurs

### Requirement: K/D/A mini-bars

Each player SHALL show three mini-bars — kills green, deaths red, assists blue — each derived only from its own counter. All f64→u64 chart conversions in this capability (bar widths, CS, sparkline samples) SHALL use one shared policy: truncate toward zero, saturate at zero, non-finite maps to absent. Derived ratios such as (K+A)/D MUST NOT be computed or displayed anywhere.

#### Scenario: Single truncation policy

- GIVEN inputs 195.7 and 195.2 requiring conversion
- WHEN routed through the shared policy
- THEN both yield 195 deterministically

#### Scenario: Zero deaths fabricates nothing

- GIVEN a player with 0 deaths
- WHEN the bars render
- THEN three independent bars draw and no ratio value appears in the frame

#### Scenario: Color binding degrades gracefully

- GIVEN a 16-color host
- WHEN the bars render
- THEN kills/deaths/assists map to green/red/blue, degrading palette but never omitting a bar

### Requirement: Inventory fill bar

Each player SHALL show an inventory fill bar over item slots 0–6 (trinket excluded). Present items fill cells; absent/null slots render empty; a wholly absent items list renders `?`.

#### Scenario: Partial fill

- GIVEN a player with 3 items and 3 empty slots
- WHEN a frame renders
- THEN the bar shows half-filled extent

#### Scenario: Absent list placeholder

- GIVEN a player whose items list is absent
- WHEN a frame renders
- THEN the row shows `?`; other players render normally

### Requirement: Local HP and power gauges

The local strip SHALL render two LineGauge-style blocked-segment gauges for the local player only: health (current/max) and power (current/max). Ratios clamp to [0,1]; a missing operand renders `?`. Non-local players MUST NOT show gauges.

#### Scenario: Gauge fill reflects snapshot

- GIVEN local HP 2100/3000 and power 400/500
- WHEN a frame renders
- THEN gauges fill 70% and 80%

#### Scenario: Missing stat degrades

- GIVEN absent power fields
- WHEN a frame renders
- THEN the power gauge shows `?`; the health gauge renders normally

### Requirement: Local gold sparkline ring buffer

A local-gold trend SHALL render from a ring buffer of the last 120 samples, one per accepted snapshot (~2 min at 1 Hz); storage is preallocated — capacity never grows, bounding retained history at ≤ 32 KiB. Input is live-only: no interpolation, no cross-frame derivation; failed polls insert gaps, not points. Samples convert via the shared truncation policy. Below 2 samples the widget SHALL show an explicit warm-up placeholder, never a flat line. The buffer persists across reconnect into the same game and resets only on entering a different game ("different game" identity signal defined at design).

#### Scenario: Warm-up placeholder

- GIVEN fewer than 2 buffered samples
- WHEN a frame renders
- THEN an explicit placeholder renders instead of a chart

#### Scenario: Same-game reconnect keeps trend

- GIVEN 50 samples, then IN_GAME→NOT_IN_GAME→IN_GAME within one game
- WHEN rendering resumes
- THEN the trend continues across pre-disconnect samples

#### Scenario: Different game resets

- GIVEN a populated buffer and entry into a different game
- WHEN the first new snapshot arrives
- THEN the buffer holds only that sample

#### Scenario: Capacity bounded

- GIVEN continuous polling past 120 samples
- WHEN inserts continue
- THEN capacity stays 120 within the 32 KiB bound

### Requirement: Viewport degradation matrix

Visibility SHALL be computable by a pure function of (width, height) without a real terminal. Chart families hide strictly in priority order: gold sparkline → K/D/A mini-bars → inventory bar → level bars → CS bars. All charts visible at ≥ 80×24; all charts hidden at ≤ 80×12; hiding monotonic between. The ticker MAY compress first; the status row belongs to no hide tier and NEVER hides.

#### Scenario: Monotonic ordered hiding

- GIVEN a larger and a smaller viewport
- WHEN visibility computes for both
- THEN the smaller visible set is a subset honoring the priority order

#### Scenario: Status never hides

- GIVEN any viewport from 1×1 through 200×60
- WHEN visibility computes
- THEN the status row is present in every result

### Requirement: Offline glyph-safety verification

Conhost compatibility SHALL be proven offline: a render test draws the full dashboard to a TestBackend and asserts buffer purity (whitelisted codepoints ∪ printable ASCII only). Manual host smoke checks live in `docs/manual-verify.md`, outside strict TDD.

#### Scenario: Full-frame purity

- GIVEN an 80×24 frame with complete data
- WHEN rendered to TestBackend and scanned cell by cell
- THEN zero cells fall outside whitelist ∪ printable ASCII

#### Scenario: Manual checklist covers widgets

- GIVEN `docs/manual-verify.md`
- WHEN this capability updates it
- THEN a conhost smoke item exercises all six widget families
