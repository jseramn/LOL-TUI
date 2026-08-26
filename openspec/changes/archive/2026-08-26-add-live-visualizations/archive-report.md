# Archive Report: add-live-visualizations

**Change**: `add-live-visualizations` · **Archived**: 2026-08-26 · **Store**: hybrid (this file + Engram `sdd/add-live-visualizations/archive-report`)
**Verify snapshot**: main @ `96d63f8` (post-U7 re-verification) · **Closure HEAD**: main @ `1207859`
**Final verdict**: PASS WITH SUGGESTIONS — verify reported PASS WITH WARNINGS at `96d63f8`; the sole remaining WARNING (FMT-1) was resolved post-verify by `1207859`, leaving suggestions only.

## Summary

Greenfield capability `live-dashboard-visualizations`: TUI-native charts rendered exclusively through a conhost-safe glyph whitelist, a region-based layout whose disjoint Rects make the `live-dashboard-ui` R6 notice guarantee structural, six High-tier widgets (per-team CS bars, level bars, K/D/A mini-bars, inventory fill bar, local HP/power gauges, local gold sparkline over a preallocated 120-sample ring buffer), one shared f64→u64 truncation policy (`chart_u64`), and a pure-function viewport degradation matrix swept exhaustively over 1×1→200×60. All 25 tasks shipped under strict TDD across six planned units plus a remediation unit (U7), stacked-to-main. Independent verification proved 23/23 scenarios and all 10 requirements compliant with 142/142 tests green; the one material finding (viz:R2's side-by-side ORDER/CHAOS column clause, WARN-1) was resolved by implementing the spec as written instead of amending it, and the residual hygiene finding FMT-1 was closed by follow-up commit `1207859`. Specs are synced to `openspec/specs/live-dashboard-visualizations/spec.md`; this change folder is archived as the cycle's audit trail.

## Outcome

| Gate | At verify snapshot `96d63f8` | At closure `1207859` |
|---|---|---|
| Verdict | PASS WITH WARNINGS (0 CRITICAL / 1 WARNING FMT-1 / 2 SUGGESTIONS) | **PASS WITH SUGGESTIONS** (FMT-1 retired post-verify) |
| `cargo test -j 1` (canonical env prefix) | ✅ 142 passed / 0 failed, exit 0 (api 19 · app 28 · history 19 · model 8 · ui 68) | 142/142 stands as final state; only formatting-only commits followed (`1207859` rustfmt cleanup) |
| `cargo build -j 1` | ✅ exit 0 | unchanged |
| `cargo clippy -j 1 --all-targets -- -D warnings` | ✅ exit 0 | unchanged |
| `cargo fmt --all -- --check` | ❌ exit 1 → FMT-1 (two cosmetic spots in U7-introduced lines) | ✅ exit 0 — **independently re-executed during this archive phase** under the canonical env prefix |

Evidence revision: `sha256:49e4fe59b2411f967e2c4a79376a90df54ff084d421ce92cc8eb58126db3ee1e` (SHA-256 of HEAD tree id `b3e9ae98…`, main @ `96d63f8`). Working tree at closure: clean (porcelain-empty, verified at archive time).

Per Final-State Authority: the verify report's FMT-1 WARNING describes the tree at `96d63f8` and is superseded by closure facts — the resolution landed in `1207859` ("style: rustfmt clean-up of U7 side-by-side code; reflect fmt gate green") and was corroborated at archive time by re-running the fmt gate directly (exit 0). No CRITICAL findings existed at any point; no override was requested or required.

## Implementation Trace

Stacked-to-main chain, strict TDD, every unit merged `--no-ff`. Test totals are cumulative full-suite counts.

| Unit | Scope | Key commits | Merge | Suite at close |
|---|---|---|---|---|
| U1 foundation (P0–P2) | Zero new crates; module skeletons; 17-codepoint glyph whitelist + closed `Glyph` enum with export-range/purity nets; array-backed `RingBuffer<T,120>` (1936 B ≪ 32 KiB); `chart_u64` truncate/saturate/non-finite⇒absent; `classify` game identity (gameTime decrease > 5 s ⇒ DifferentGame, else mode change, else SameGame); app-fold pushes with lifecycle immunity | ecc38a6 · 2a6f481 · 3ab5144 · 1dac009 · f0e3e66 | `d4a4e5e` | 101/0 |
| U2 region shell (P3) | Behavior-preserving: `Pen` deleted; disjoint Regions{header, body, local, ticker, status} via status-row reservation + 4-constraint Layout + greedy fallback < 9 rows; legacy text byte-for-byte, standing tests UNWEAKENED | a66f39f | `69e5995` | 104/0 |
| U3 team widgets (P4.1–4.5) | ratatui 0.30.2 API surface verified against vendored source (task 4.1 record); CS bars global shared max; level bars fixed `(level−1)/17`; K/D/A tri-color own-metric maxima; inventory 6-cell ▓/░ trinket-excluded strip; frozen row contract PREFIX_CELLS=45 | 92dcbe4 · bb966d4 · 27e8e62 · 3bb0f89 | `cd0ce0a` | 120/0 |
| U4 local strip (P4.6–4.7) | `gauge_ratio` clamps BEFORE `LineGauge::ratio` (which panics outside [0,1]); blocked-segment gauges █/░ with isolated `?` degradation; gold Sparkline over `Vec<Option<u64>>` with ░ gap breaks and `warming up (n/120)` warm-up; local band Length(2–3)→Length(4) disclosed | 8d91ec4 · 7e1aff5 | `3d5ff21` | 130/0 |
| U5 degradation matrix (P5) | Pure `select_layout(area) -> LiveLayout { areas, visible }`; tiers CS≥13 / level≥15 / inventory≥18 / KDA≥20 / sparkline≥24; width<40 ⇒ ChartSet::NONE; gauges structurally untierable; monotonic-subset property ~10k combos + exhaustive 12k-size sweep pinning status == exact final row with pairwise-disjoint regions; ghost-path lesson (vacuous probe relocated 39×24→39×60) | 28b92bf · 657a23f | `b0141c5` | 140/0 |
| housekeeping | Orchestrator reconciliation of working tree (untracked openspec dirs, stray deletions) before U6 | a74474d · cb91f48 | — | — |
| U6 verify & docs (P6) | fmt drift applied (+31/−19 formatting-only); clippy `-D warnings` satisfied (let-chains, `PriorityEntry<'a>` alias — behavior-preserving); compliance grep audit; `docs/manual-verify.md` §11 four-subsection checklist covering all six families, conhost glyph smoke, tier walk-down, reconnect | 40e3b03 · e8508de · edfb3bf | `37e37e2` | 140/0 |
| U7 remediation (post-U6) | viz:R2 side-by-side ORDER/CHAOS columns implemented exactly per spec/design letter: `TeamColumns { order, chaos }` from `Layout::horizontal([Percentage(50), Percentage(50)])` over the body band; deterministic contained split below width 2; CS max and K/D/A maxima stay GLOBAL cross-team; +68-line 12k-size adjacency invariant sweep and +86-line render-level pin; +2 tests ⇒ 142 | d54f4e9 · `26c4e7f` | `96d63f8` | 142/0 |
| post-verify closure | FMT-1 resolved: rustfmt-canonicalized the two U7-introduced spots (`src/ui/mod.rs:191` Layout chain, `tests/ui/team_widget_tests.rs:668` boolean line); zero semantic delta; fmt gate green | `1207859` | — | 142/142, fmt 0 |

## Spec Coverage Matrix

Capability `live-dashboard-visualizations` — 10 requirements / 23 scenarios, all COMPLIANT at runtime (evidence = committed tests named in the verify compliance matrix; all executions offline via `Terminal<TestBackend>` + bundled fixtures).

| Requirement | Scenarios | Committed evidence (selected named tests) |
|---|---|---|
| viz:R1 glyph whitelist sole source | S1–S2 | `every_exported_codepoint_is_conhost_safe`, `no_braille_or_sextant_octant_codepoints_are_exported`, anti-vacuity `exports_at_least_one_glyph`, completeness `glyph_enum_symbols_stay_inside_the_exported_set`; `glyph_driven_widget_output_is_whitelist_pure` with detector control `purity_scanner_flags_injected_braille` |
| viz:R2 region layout preserving ui:R6 | S1–S2 | `canonical_region_order_at_80x24_with_notice_filling_final_row`; ➕ U7 adjacency SHALL clause: `order_and_chaos_columns_render_side_by_side` + `team_columns_sit_side_by_side_wherever_the_body_region_is_visible` (12 000-size invariant sweep: edge ownership, disjointness/no-gap, right-edge closure, non-vanishing, containment); `minimum_viewport_keeps_regions_ordered_above_an_intact_status_row_80x12` |
| viz:R3 per-team CS bars | S1–S2 | `cs_bars_scale_to_the_shared_maximum_across_both_teams` (vectors 40/120/300, cross-team max preserved post-U7); `missing_creep_score_renders_placeholder_while_other_bars_render`; true-zero pin for all-zero max |
| viz:R4 level bars fixed 1–18 | S1–S2 | `level_bars_pin_fixed_scale_endpoints` + unit pin `level_fill_cells_follows_the_fixed_mapping_exactly` (1→0 cells, 18→10 cells); `overrange_and_underrange_levels_clamp_without_panicking` (25→full, 0→empty, u32::MAX saturates) |
| viz:R5 K/D/A mini-bars + truncation policy | S1–S3 | `fractional_values_truncate_toward_zero` (195.7 & 195.2 ⇒ 195), `non_finite_inputs_map_to_absent`, `negative_inputs_saturate_at_zero`, app-side `gold_samples_route_through_the_shared_truncation_policy`; `zero_deaths_keep_all_three_segments_and_display_no_numbers` (zero-digit assertion bans ratios); `kda_segments_bind_to_green_red_and_blue` |
| viz:R6 inventory fill bar | S1–S2 | `inventory_strip_shows_partial_fill_and_skips_null_slots` (exact 3 filled / 3 empty); `absent_items_list_renders_placeholder_without_touching_other_strips` + trinket-exclusion and saturation pins |
| viz:R7 local HP/power gauges | S1–S2 | `gauges_fill_70_and_80_percent_for_the_local_player_only` (2100/3000⇒70%, 400/500⇒80% ±1 cell; row-uniqueness proves gauges never attach to non-local players); `missing_power_renders_placeholder_while_the_hp_gauge_renders_normally` + panic-free clamp vectors |
| viz:R8 gold sparkline ring buffer | S1–S4 | `warm_up_placeholder_shows_until_two_real_samples_exist` (gaps/absent gold never inflate n); `notbound_round_trip_preserves_the_same_game_trend` + `same_game_snapshots_append_oldest_to_newest` (D7 fold); `different_game_resets_then_pushes_only_the_new_sample` + `game_mode_change_resets_the_trend` + `identity_tests` truth table (±5.0 s edges, degraded payloads); `wrapping_push_drops_oldest_and_pins_length_to_capacity` + `gold_history_storage_stays_within_the_32kib_bound` (1936 B ≤ 32 KiB) |
| viz:R9 viewport degradation matrix | S1–S2 | `hiding_is_monotonic_between_smaller_and_larger_viewports` (~10k sampled pairs), `families_hide_strictly_in_the_documented_priority_order` (h=60→1 walk), `tier_boundaries_follow_the_documented_height_table`, `widths_below_40_hide_every_chart_family`; `status_row_is_present_for_every_viewport_from_1x1_to_200x60` (exhaustive 12k sizes, extended by U7 with column invariants) |
| viz:R10 offline glyph-safety verification | S1–S2 | `full_frame_purity_scan_at_80x24_with_complete_data` (re-proven post-U7 with both columns drawing); static doc evidence: `docs/manual-verify.md` §11.1–§11.4 exercising all six families (execution = pending human item) |

Pinned guarantees held: `live-dashboard-ui` R6 notice/status unchanged (exhaustive final-row sweep); `live-client-poller` spec untouched (history feeds app-side per D4; api suite 19/19 throughout).

## Deviations Adjudication

The four disclosed U2–U7 deviations, adjudicated at verify and closed here (per verify-report S-1, this archive record serves as the sanctioned errata home):

| # | Deviation | Adjudication | Severity |
|---|---|---|---|
| a | U2/D2: status row reserved pre-solver instead of fifth `Layout` constraint | ACCEPTED — strengthens the guarantee: cassowary deficit distribution below the tuple minimum is unspecified, so reservation makes ui:R6/viz:R2 deterministic; exhaustive 12k-size sweep proves it | SUGGESTION (design-letter divergence, intent honored) |
| b | U3/D3: side-by-side team columns deferred to P5 | **RESOLVED by U7** — `26c4e7f` shipped the horizontal `Percentage(50)/(50)` split exactly per spec viz:R2 and design D3 letter; adjacency, disjointness, edge closure, non-vanishing, containment runtime-proven. No spec amendment made or needed | Resolved (was WARNING W-1) |
| c | U4/D2: local band Length(2–3) → Length(4) | ACCEPTED — legacy LOCAL text line kept byte-for-byte (ui-spec continuity pin) + HP + Power + GOLD rows; MIN_REGIONS_HEIGHT 9→11 and greedy fallback reconcile it; no spec clause pins band height; 80×12 scenario passes | SUGGESTION (disclosed, documented) |
| d | U5/D2 reconciliation | VERIFIED CONSISTENT — tier math over the Length(4) band; width<40 override cannot resurrect height-hidden charts; gauges untierable by construction (no `ChartSet` flag); hidden sparkline blanks the gold row without moving bands | Note only |

Verified-conformant micro-decisions (disclosed in apply-progress): warm-up n counts REAL (`Some`) samples; trim-to-newest window slice (trend tracks present, gaps preserved — tested at 130 polls); K/D/A fixed 8-cell tracks (frozen geometry); inventory occupancy requires `item_id` (null entry ≡ empty slot, per spec "absent/null slots render empty"). Design D2 letters should be read through the errata above, not literally.

## Compliance Re-Audit

Four protocol audits, last executed on main @ `96d63f8` (verify phase) with closure-context attestation at `1207859`; none touched by the formatting-only closure commit:

1. **No derived timers** ✅ — respawn rendered verbatim (`DEAD(respawn {t})` or `?` marker; standing test `dead_player_without_exposed_timer_shows_unknown_marker_never_a_countdown`); status timestamps derive from stamped receipt millis, never gameTime; `Instant::` grep over src = 0 matches.
2. **Gold exclusively local** ✅ — zero gold references in `src/ui/team.rs` (unchanged by U7's geometry-only column split); `current_gold` consumed solely via `snapshot.local`; uniqueness net `unique_gold_row` + `missing_local_gold_degrades_and_no_enemy_panel_shows_gold`.
3. **Glyph whitelist sole source (closed enum)** ✅ — zero `from_u32`/`char::from`/`Glyph::from` in src; dual offline nets + detector controls green; full-frame purity re-proven post-U7 with both columns drawing.
4. **Riot notice preserved (ui:R6)** ✅ — mandated-wording `RIOT_NOTICE` test; drawn LAST in both views; exhaustive 1×1→200×60 sweep pins final-row ownership with pairwise-disjoint regions (re-run green with U7 column invariants added).

## Risk Retirement

Disposition of every verify finding against final state:

1. **WARN-1 / W-1 — viz:R2 side-by-side ORDER/CHAOS clause unimplemented: RETIRED BY IMPLEMENTATION.** Maintainer chose implementing the spec as written over amending it; `26c4e7f` (+279/−33, 5 files) delivered the `Percentage(50)/(50)` column split with runtime proof (render-level adjacency pin + 12k-size invariant sweep). Adjudicated RESOLVED (deviation b above).
2. **WARNING FMT-1 — `cargo fmt --check` exit 1 at `96d63f8`: RETIRED POST-VERIFY.** Resolved mechanically by `1207859` (rustfmt-canonicalize the two U7-added spots, zero semantic delta); corroborated at archive time by independent re-execution of the fmt gate under the canonical env prefix → exit 0. Task 6.1's "fmt clean" claim holds again at closure HEAD.
3. **SUGGESTION S-1 — consolidate accepted deviations into an errata note: DISCHARGED BY THIS RECORD.** The verify report sanctioned either a design.md errata or the archive record; the Deviations Adjudication section above is that record. A pointer edit to design.md remains optional and is listed as a follow-up.
4. **SUGGESTION S-2 — align design.md D3 if the W-1 presentation diverged: RESOLVED BY U7.** U7 shipped exactly D3's documented `Percentage(50)/(50)` split, so design.md is accurate as written; recorded for traceability.

Carried (non-blocking, cannot be proven offline): the `docs/manual-verify.md` §11 human execution set — real-game rendering of all six families on Windows Terminal AND classic conhost, conhost glyph/font smoke, 16-color K/D/A palette degradation visual, resize walk-down across tiers (now visually exercising the side-by-side columns), reconnect same-game continuation / different-game reset on a real connection.

## Rollback Plan

Single-crate binary, feature-isolated modules, no migrations or external state:

- Per-unit reverts: `git revert -m 1 <merge>` for any integration merge (`d4a4e5e`, `69e5995`, `cd0ce0a`, `3d5ff21`, `b0141c5`, `37e37e2`, `96d63f8`); deleting `src/ui/team.rs` / `src/ui/local_strip.rs` restores the text dashboard for those bands.
- U7 alone: `git revert -m 1 96d63f8` (or revert `26c4e7f`) removes only the column split; prior layout semantics return.
- Closure commit `1207859` is formatting-only and reverts standalone.
- Archive bookkeeping is additive documentation: restore by moving `openspec/changes/archive/2026-08-26-add-live-visualizations/` back to `openspec/changes/add-live-visualizations/` and deleting `openspec/specs/live-dashboard-visualizations/`.

## Follow-ups

1. Execute `docs/manual-verify.md` §11 on a real game connection (both hosts) and log the results — the only substantive risk class still resting on offline proof.
2. Optional: add a one-line errata pointer on design.md D2/D3 referencing this report's Deviations Adjudication (S-1 home is already discharged here).
3. All work is local on main (NOT pushed) — push/remote setup is an orchestrator/user decision.
4. Engram housekeeping (outside this phase's scope): resolve the pending contested judgment attached to proposal observation #25 (`obs-d85cb86766153dc4`), and note that spec observation #26's prose says "22 scenarios" where the authoritative file, its requirement-ID map, and the verify report all say 23 (R5×3 + R8×4) — treat the persisted spec file as canonical.

## Traceability — sources read at archive time

Full reads: Engram #29 apply-progress · #31 verify-report (post-U7) · #26 spec observation. Surfaced via search previews: #24 exploration · #25 proposal · #27 design · #28 tasks · #30 session summary. Files read directly (hybrid store): `proposal.md`, `specs/live-dashboard-visualizations/spec.md`, `design.md`, `tasks.md` (25/25 `[x]`, 0 unchecked — Task Completion Gate passed), `verify-report.md`, predecessor `archive/2026-08-25-add-live-match-dashboard/archive-report.md`, `openspec/config.yaml` (`rules.archive`: warn-before-destructive-deltas — not triggered; greenfield additive sync). Repository evidence gathered at archive time: `git log`/`status` (clean tree @ `1207859`) and an independent `cargo fmt --all -- --check` run (exit 0). Native Review Receipt Gate: no review artifact existed for this candidate (`reviewGate` structurally absent) — archived under ordinary repository policy. This report persisted as topic_key `sdd/add-live-visualizations/archive-report`.
