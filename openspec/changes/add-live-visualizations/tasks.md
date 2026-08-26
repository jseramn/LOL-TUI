# Tasks: Live Dashboard Visualizations

**Change**: `add-live-visualizations` · Basis: proposal.md (slices S1–S11), design.md D1–D10, spec `live-dashboard-visualizations` (10 req / 23 scenarios), config.yaml rules.tasks · Store: hybrid (this file + Engram `sdd/add-live-visualizations/tasks`)
**TDD**: strict red-green; pure fns (`chart_u64`, `RingBuffer`, `classify`, `select_layout`) tested BEFORE the widgets that compose them · Threat matrix: N/A per design

## Guard Lines

```text
Decision needed before apply: No
Chained PRs recommended: Yes
Chain strategy: stacked-to-main
400-line budget risk: High
```

## Spec Requirement ID Map (`Rx/S#` = #th scenario under requirement Rx)

- viz:R1 glyph whitelist sole source (S1 export-range, S2 widget purity)
- viz:R2 region layout preserving ui:R6 (S1 region order, S2 notice @ 80×12)
- viz:R3 per-team CS bars (S1 shared-max, S2 absent `?`)
- viz:R4 level bars fixed 1–18 (S1 endpoints, S2 clamp)
- viz:R5 K/D/A mini-bars + shared truncation policy (S1 truncate⇒195, S2 zero-deaths, S3 16-color)
- viz:R6 inventory fill bar (S1 partial, S2 absent list)
- viz:R7 local HP/power gauges (S1 70%/80%, S2 missing stat)
- viz:R8 gold sparkline ring buffer (S1 warm-up, S2 same-game continuity, S3 different-game reset, S4 capacity)
- viz:R9 degradation matrix (S1 monotonic subset, S2 status never hides)
- viz:R10 offline glyph-safety verification (S1 full-frame purity, S2 manual checklist)

Pinned guarantees: `live-dashboard-ui` R6 notice/status unchanged; `live-client-poller` spec untouched — history feeds app-side only (D4).

## Canonical Commands

**CT** = openspec/config.yaml `apply.test_command`; **CB** = `verify.build_command` (identical env prefix, `-j 1`). Every **Test:**/**Build:** step runs these verbatim.

## Suggested Work Units

| Unit | Goal | Likely PR | Focused test | Runtime harness | Rollback boundary |
|---|---|---|---|---|---|
| 1 | Stubs + glyph whitelist + history primitives + app fold (P0–P2) | PR 1 | CT (`--test history`, `--test app`) | Offline fixtures + synthetic in-test snapshot sequences (fixtures hold single frames) | revert `src/glyphs.rs`, `src/history.rs`, app-fold hunks |
| 2 | Region-layout shell, behavior-preserving (P3) | PR 2 | CT (`--test ui`) | TestBackend buffers | revert `src/ui/mod.rs`/`dashboard.rs` hunks; text output identical |
| 3 | Team-column widgets: CS/level/K-D-A/inventory (P4.2–4.5) | PR 3 | CT (`--test ui`) | TestBackend from fixture snapshots | delete `src/ui/team.rs` |
| 4 | Local strip: gauges + gold sparkline (P4.6–4.7) | PR 4 | CT (`--test ui`) | TestBackend from fixture snapshots | delete `src/ui/local_strip.rs` |
| 5 | Degradation matrix `select_layout` + sweeps (P5) | PR 5 | CT (`--test ui`) | Pure-fn sweep, no real terminal | revert `LiveLayout` wiring |
| 6 | Verification + conhost smoke docs (P6) | PR 6 | CT full + clippy/fmt | Manual WT/conhost checklist | docs + lint fixes only |

## Phase 0: Dependencies & Skeleton

- [x] 0.1 Deps decision: expect NO new crates — ratatui 0.30.2 features already present, `bitflags` unnecessary for a const whitelist (D8 constants-only). If apply discovers a genuine need, stop-and-record rationale; otherwise `Cargo.toml` untouched. Build: CB green.
- [x] 0.2 Create module stubs `src/glyphs.rs`, `src/history.rs`, `src/ui/team.rs`, `src/ui/local_strip.rs`; wire `mod` tree (`lib.rs`, `ui/mod.rs`); doc-comment each stub with owning decision (D8/D4/D3). Build: CB green.

## Phase 1: Glyph Whitelist Module (slice S1)

- [x] 1.1 RED `tests/ui/glyph_tests.rs`: enumerate every exported codepoint ∈ U+2500–257F ∪ U+2580–259F; zero Braille (U+2800–28FF)/sextant-octant exports (viz:R1/S1). Register harness per `tests/<name>/main.rs` pattern. Run CT → RED confirmed.
- [x] 1.2 GREEN `src/glyphs.rs`: `pub const` codepoints + `pub enum Glyph` with `pub const fn symbol(self) -> &'static str`; enum variants are the only constructors — runtime glyph construction impossible by construction (D8).
- [x] 1.3 RED/GREEN widget-purity net: render one trivial Glyph-driven widget to TestBackend, scan every cell ⊆ whitelist ∪ printable ASCII (viz:R1/S2). Test: CT green.

## Phase 2: History Primitives & App Fold (slice S3)

- [x] 2.1 RED `tests/history/` (new dir, `main.rs` harness): wrap/clear/`len`/iter oldest→newest; pushing past 120 keeps len 120; `size_of::<GoldHistory>()` ≤ 32 KiB — expect 120 × `Option<u64>` ≈ **1.9 KiB** (corrected figure, D4) (viz:R8/S4). Run CT → RED.
- [x] 2.2 GREEN `src/history.rs`: array-backed `RingBuffer<T, 120>`, preallocated, O(1) `push(Option<T>)`, capacity never grows; `type GoldHistory = RingBuffer<u64, 120>` as sole instantiation (D4).
- [x] 2.3 RED/GREEN `chart_u64(f64) -> Option<u64>`: non-finite ⇒ `None`; else truncate toward zero, saturate at 0 (vectors: 195.7/195.2 ⇒ 195; NaN ⇒ None; negative ⇒ 0) (viz:R5/S1, D5). Sole conversion path for CS + gold samples.
- [x] 2.4 RED/GREEN `GameIdentity` + `classify`: gameTime decrease > 5.0 s ⇒ DifferentGame; else gameMode change (both present) ⇒ DifferentGame; else SameGame — truth-table vectors incl. ±5.0 s jitter edges and degraded payloads with absent fields (D1).
- [ ] 2.5 RED `tests/app_tests.rs` extensions: fold per D4/D7 — Snapshot: DifferentGame ⇒ `clear()` then push; SameGame ⇒ push (incl. NotBound round-trip continuity); Transient-in-game ⇒ push(`None`) gap; Lifecycle messages NEVER touch history (viz:R8/S2, S3). Run CT → RED.
- [ ] 2.6 GREEN `src/app.rs`: holds `GoldHistory` + last identity; fold-time pushes; `gold_window()` iterator oldest→newest. Poller thread untouched. Test: CT green.

## Phase 3: Region-Layout Shell (slice S2 — behavior-preserving)

- [ ] 3.1 RED `tests/ui/shell_render_tests.rs` extensions: 80×24 frame shows regions top-to-bottom header/body/local/ticker/status with Riot notice filling the final row (viz:R2/S1); 80×12 keeps status row last, visible, unoverlapped (viz:R2/S2). Run CT → RED.
- [ ] 3.2 GREEN `src/ui/mod.rs`: vertical `Layout` `[Length(1) header, Min(5) body, Length(2–3) local, Min(1) ticker, Length(1) status]` — status LAST; disjoint Rects make ui:R6 structural (D2). Retire `Pen` only after ticker/status migrate.
- [ ] 3.3 GREEN `src/ui/dashboard.rs`, `ticker.rs`, `status.rs`: render into assigned Rects; textual content identical to today — existing dashboard/ticker/status tests stay green UNWEAKENED. Test: CT green.

## Phase 4: Widgets — one task per family (slices S5–S9, S8′)

- [ ] 4.1 FIRST widget task: verify ratatui 0.30.2 symbol-override signatures (BarChart/Sparkline/LineGauge custom-symbol params) against the vendored source; document the actual API surface used in module docs; record fallback (inline glyph strips) if signatures diverge from design assumption (design open-question closure).
- [ ] 4.2 RED/GREEN CS bars `src/ui/team.rs`: one horizontal BarChart per player; `.max()` = highest CS among ALL visible players (global); max 0 ⇒ empty bars as true zeros (never fabricated); absent creepScore ⇒ `?` text row (viz:R3/S1, S2).
- [ ] 4.3 RED/GREEN level bars: fixed scale `(level−1)/17` clamp [0,1] — level 1 near-empty, 18 full, 25 clamps full with no panic; absent level ⇒ `?` (viz:R4/S1, S2).
- [ ] 4.4 RED/GREEN K/D/A mini-bars: three glyph spans green/red/blue, each scaled by ITS metric's shared max across visible players; derived ratios (e.g. (K+A)/D) never computed nor displayed; 16-color host degrades palette but keeps all three bars (viz:R5/S2, S3).
- [ ] 4.5 RED/GREEN inventory fill `src/ui/team.rs`: 6-cell `▓`/`░` strip counting occupied slots at index ≠ 6 (index 6 = trinket, excluded per spec — cannot prove trinket presence); null/absent slots render empty; wholly absent items list ⇒ `?` (viz:R6/S1, S2).
- [ ] 4.6 RED/GREEN local gauges `src/ui/local_strip.rs`: 2 × LineGauge-style blocked-segment gauges, HP (2100/3000 ⇒ 70%) and power (400/500 ⇒ 80%), ratio clamped [0,1]; any missing operand ⇒ `?` on that gauge only; non-local players never show gauges (viz:R7/S1, S2).
- [ ] 4.7 RED/GREEN gold sparkline: Sparkline over `Vec<Option<u64>>` from `gold_window()`; window auto-max; `None` renders as break (absent value); < 2 `Some` points ⇒ explicit `warming up (n/120)` text — never a flat line (D6) (viz:R8/S1). Test: CT green.

## Phase 5: Viewport Degradation Matrix (slice S10)

- [ ] 5.1 RED `tests/ui/degradation_tests.rs`: `select_layout` tier boundaries — height ≤12 none · 13–14 +CS · 15–17 +level · 18–19 +inventory · 20–23 +K/D/A · ≥24 all (+sparkline); width < 40 hides ALL charts; gauges have no tier (always-on); monotonic-subset property between sizes; status row present for EVERY size in a 1×1 → 200×60 sweep (viz:R9/S1, S2). Run CT → RED.
- [ ] 5.2 GREEN `select_layout(area: Rect) -> LiveLayout { areas, visible }` in `src/ui/mod.rs`; wire widgets to the visible set; ticker `Min(1)` compresses first. Full-frame purity scan at 80×24 with complete data (viz:R10/S1) + re-run notice-at-bottom sweep pinning ui:R6. Test: CT green.

## Phase 6: Verification & Docs (slice S11)

- [ ] 6.1 Full pass: CT green offline; `cargo fmt` clean; `cargo clippy -j 1 -- -D warnings` under canonical env prefix. Audit greps: zero runtime glyph construction, zero derived timers/countdowns, gold rendered local-only.
- [ ] 6.2 Extend `docs/manual-verify.md`: Windows Terminal AND classic-conhost smoke items exercising all SIX widget families, resize walk-down across tiers, and reconnect-same-game trend continuation (viz:R10/S2; proposal success criteria).

---

## Review Workload Forecast

- Estimated total changed lines: ~2350
- Per-phase estimates: P0 ~35 · P1 ~190 · P2 ~520 (history+unit tests ~430, app fold ~90) · P3 ~260 · P4 ~810 (`team.rs` ~480, `local_strip.rs` ~330, incl. widget tests) · P5 ~270 · P6 ~110
- Chained PRs recommended: Yes
- 400-line budget risk: High
- Decision needed before apply: No (delivery auto-chain cached; chain strategy stacked-to-main cached)
- Chain strategy: stacked-to-main

## Next Step

Ready for sdd-apply, Unit 1 (Phases 0–2) first.
