# Proposal: Live Dashboard Visualizations

**Change**: `add-live-visualizations` · **Phase**: propose · **Date**: 2026-08-25 · **Store**: hybrid (this file + Engram `sdd/add-live-visualizations/proposal`)

## Intent

The shipped dashboard renders everything as plain text rows ("I want graphics… I see only data"). This change adds TUI-native visualizations rendered solely with Block Element glyphs (U+2580–U+259F) so frames render identically on Windows Terminal AND classic conhost (no font fallback pre-Win11 22H2). Grounded entirely in `exploration.md`.

## Scope

### In Scope
- **Glyph whitelist module + unit tests** — Block Elements/shades/box-drawing only; the conhost regression kill switch
- **Layout-region rewrite of `src/ui/dashboard.rs`** — header / team columns / local strip / ticker / status LAST, replacing the Pen line-cursor while preserving ui-spec R6 notice visibility
- **Six High-tier widgets**: per-team CS horizontal bars; level bars (fixed 1–18 scale); K/D/A mini-bars; inventory-fill bar (0–6 slots); local HP/power gauges; local gold sparkline over a 120-frame ring buffer (~2 min @ 1 Hz, ≈30 KB)
- **One documented f64→u64 truncation policy** for Sparkline/BarChart inputs, applied uniformly
- **Viewport degradation matrix** — charts hide/shrink before the status row ever clips; tests extended
- `docs/manual-verify.md` conhost glyph smoke items

### Out of Scope
- Enemy/team gold visuals — API-impossible (gold exists ONLY on `activePlayer.currentGold`)
- Respawn countdowns / any derived timer or cooldown — Riot Game Integrity violation
- Animation (redraw per poll cycle only); positions/minimap; cumulative-history graphs
- Windowed event histograms (Medium tier) — compliant as static aggregates but needs presentation-arithmetic policy sign-off → v2
- Canvas-based charts — revisit after the widget set proves the layout

## Capabilities

> CONTRACT between proposal and specs phases.

### New Capabilities
- `live-dashboard-visualizations`: glyph-safety rules; ring-buffer history semantics (warm-up placeholders; reconnect/new-game reset policy decided at design); per-widget data bindings; absence/degradation behavior; viewport hide thresholds

### Modified Capabilities
None — `live-dashboard-ui` requirements are unchanged; the Layout refactor is behavior-preserving and all new behavior lives in the new capability. Existing R6 notice/status scenarios remain the pinned guarantee.

## Approach

Exploration Approach B: native ratatui 0.30 widgets (Sparkline, BarChart, Gauge/LineGauge), zero new dependencies; a pure history module behind a deterministic clock seam feeds trend widgets. Delivery slices S1–S11: whitelist → layout extraction (behavior-preserving) → buffer → six widgets → degradation matrix → manual verify. Each slice is strict-TDD friendly and independently revertable.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `src/ui/dashboard.rs` | Modified | Region-hosted widgets replace text lines |
| `src/ui/mod.rs` | Modified | Layout regions replace Pen cursor |
| `src/history.rs` (new) | New | Frame-series ring buffer |
| `tests/ui/*`, `docs/manual-verify.md` | Modified | Widget assertions, conhost checklist |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| conhost glyph regression (Braille/Sextant slip) | Med | Whitelist module + glyph unit tests + manual smoke |
| Charts crowd out mandatory notice at small viewports | Med | Hide-threshold matrix; buffer-lock tests pin status row |
| Reconnect-same-game stale buffer misleads trends | Low | Design-phase reset policy (correctness, not compliance) |
| Inconsistent f64→u64 truncation across charts | Low | Single documented policy |

## Rollback Plan

Single-crate binary, feature-isolated modules, no migrations or external state. Revert any slice via `git revert -m 1 <merge>`; S2 layout extraction lands behavior-preserving and can ship alone; removing widget modules restores today's text dashboard. Spec sync is additive documentation.

## Dependencies

None new — ratatui 0.30.2/crossterm 0.29.0 already ship every required primitive. Prerequisite: archived `add-live-match-dashboard` on main (complete).

## Success Criteria

- [ ] All six High-tier widgets render identically on Windows Terminal AND classic conhost using Block Elements only
- [ ] Riot non-endorsement notice visible in every view at every tested viewport size (incl. extreme-small)
- [ ] Canonical env-prefixed `cargo test -j 1` green offline; every slice TDD'd
- [ ] Structurally zero non-local gold rendering and zero derived timers
- [ ] `docs/manual-verify.md` includes the conhost glyph regression checklist
