```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:49e4fe59b2411f967e2c4a79376a90df54ff084d421ce92cc8eb58126db3ee1e
verdict: pass_with_warnings
blockers: 0
critical_findings: 0
requirements: 10/10
scenarios: 23/23
test_command: '$env:CARGO_HOME=''E:\rust\cargo''; $env:RUSTUP_HOME=''E:\rust\rustup''; $env:Path=''E:\w64devkit\w64devkit\bin;E:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;E:\rust\cargo\bin;''+$env:Path; $env:RUSTFLAGS=''-Clink-self-contained=yes''; $env:TMP=''E:\dev\TUI-LOL\target\tmp''; $env:TEMP=$env:TMP; cargo test -j 1'
test_exit_code: 0
test_output_hash: sha256:481eafbb9341656c7351e5e86bb5b29e2d8c04b8adf5634ca971a5ae322b8cba
build_command: '$env:CARGO_HOME=''E:\rust\cargo''; $env:RUSTUP_HOME=''E:\rust\rustup''; $env:Path=''E:\w64devkit\w64devkit\bin;E:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained;E:\rust\cargo\bin;''+$env:Path; $env:RUSTFLAGS=''-Clink-self-contained=yes''; $env:TMP=''E:\dev\TUI-LOL\target\tmp''; $env:TEMP=$env:TMP; cargo build -j 1'
build_exit_code: 0
build_output_hash: sha256:14129421b0c703ddddf321a9e79d4cf2fbeb338e1a81b5874831d5235d17dc2d
```

## Verification Report

**Change**: add-live-visualizations
**Version**: spec @ `openspec/changes/add-live-visualizations/specs/live-dashboard-visualizations/spec.md` (10 req / 23 scenarios)
**Mode**: Strict TDD
**Commit under test**: main @ `96d63f8` (clean working tree, verified)
**Store**: hybrid — this file + Engram `sdd/add-live-visualizations/verify-report`
**Phase**: U7 remediation follow-up re-verification — supersedes this file's post-U6 state (W-1 adjudicated there is closed by this report)

### Completeness
| Metric | Value |
|--------|-------|
| Tasks total | 25 |
| Tasks complete | 25 |
| Tasks incomplete | 0 |

### Build & Tests Execution
**Build**: ✅ Passed — CB exit 0.
**Tests**: ✅ **142 passed / 0 failed / 0 skipped**, CT exit 0 — api 19 · app 28 · history 19 · model 8 · ui 68 (+2 vs the apply-phase final report's 140: U7's two side-by-side pins; all prior suites unchanged).
**Extra gates (re-run this phase)**: `cargo clippy -j 1 --all-targets -- -D warnings` exit 0 · `cargo fmt --all -- --check` **exit 1 → FMT-1** (see Issues).
**Output digests**: CT `sha256:481eafbb9341656c7351e5e86bb5b29e2d8c04b8adf5634ca971a5ae322b8cba` · CB `sha256:14129421b0c703ddddf321a9e79d4cf2fbeb338e1a81b5874831d5235d17dc2d` (exact captured outputs under `target/tmp/ct-output.txt` / `cb-output.txt`; clippy/fmt under the same directory).
**Evidence revision**: recomputed for this phase as SHA-256 of the HEAD tree object id (`b3e9ae98…`, main @ `96d63f8`) = `sha256:49e4fe59b2411f967e2c4a79376a90df54ff084d421ce92cc8eb58126db3ee1e`.

**Coverage**: ➖ Not available — no coverage tool configured (`coverage_threshold: 0`). Skipped, informational only.

### U7 Remediation Follow-up
**What U7 did** — commit `26c4e7f` ("fix(ui): render ORDER and CHAOS team columns side by side per viz:R2"), 5 files, +279/−33:
- `src/ui/mod.rs`: new `TeamColumns { order, chaos }` struct; `LiveLayout` gains `columns`; pure `split_team_columns(body)` splits the body band with `Layout::horizontal([Percentage(50), Percentage(50)])`, with an explicit deterministic contained split below two cells of width (`select_layout` doc updated to name viz spec R2).
- `src/ui/dashboard.rs`: the visualization band (body minus legacy text-panel rows) now threads per-team rects into `team::render` — columns keep the band's clip semantics; text panels untouched.
- `src/ui/team.rs`: `render` delegates to a new per-team `render_team(frame, snapshot, team, column_rect, …)` filling its column top-down in roster order; CS max and K/D/A per-metric maxima remain GLOBAL across both teams (viz:R3/R5 scaling unchanged — "columns change WHERE rows draw, never HOW bars scale").
- `tests/ui/degradation_tests.rs` (+68): exhaustive invariant sweep `team_columns_sit_side_by_side_wherever_the_body_region_is_visible` over every viewport 1×1→200×60 (12 000 sizes).
- `tests/ui/team_widget_tests.rs` (+86): render-level pin `order_and_chaos_columns_render_side_by_side` proving ORDER rows at the body's left edge, CHAOS rows at the right, both on one physical row band.

Merged to main via `96d63f8` ("chore: integrate unit 7 (side-by-side columns + housekeeping)", parents `37e37e2` + `d54f4e9`). Commit log tail: `96d63f8` merge/integration ← `d54f4e9` ops ← `26c4e7f` fix.

**How W-1 was resolved** — `select_layout` now returns disjoint ORDER/CHAOS rects whose adjacency is runtime-proven by the new degradation_tests invariants at every one of the 12 000 swept sizes: ORDER owns the body's left edge (`order.x == body.x`); CHAOS starts exactly where ORDER ends (`order.x + order.width == chaos.x` — disjoint, no gap); the pair closes the body's right edge exactly (combined x-range covers body width with zero remainder); both columns share the body's `y` and full height; BOTH rects are non-empty whenever the body is visible (`!body.is_empty() && body.width >= 2`), with even widths splitting into equal halves; degenerate bodies keep the split contained (`order.width + chaos.width == body.width`). The spec viz:R2 SHALL clause "ORDER column and CHAOS column side by side" is implemented as written — no spec amendment was needed.

**Gates re-run this phase**: CT 142/0 exit 0 · CB exit 0 · clippy `-D warnings` exit 0 · fmt --check exit 1 (new FMT-1, introduced by U7 itself — two cosmetic rustfmt diffs on U7-added lines; see Issues).

### Spec Compliance Matrix
All executions offline via `Terminal<TestBackend>` + bundled fixtures; test names below passed at runtime in this phase's CT run. U7 added runtime coverage for the previously unproven viz:R2 adjacency SHALL clause (rows marked ➕).

| Requirement | Scenario | Test | Result |
|-------------|----------|------|--------|
| viz:R1 glyph whitelist | S1 forbidden ranges never exported | `tests/ui/glyph_tests.rs > every_exported_codepoint_is_conhost_safe` (+ `no_braille_or_sextant_octant_codepoints_are_exported`, anti-vacuity `exports_at_least_one_glyph`, completeness `glyph_enum_symbols_stay_inside_the_exported_set`) | ✅ COMPLIANT |
| viz:R1 | S2 widget output stays whitelisted | `glyph_tests > glyph_driven_widget_output_is_whitelist_pure` (detector control `purity_scanner_flags_injected_braille`) | ✅ COMPLIANT |
| viz:R2 region layout / ui:R6 | S1 canonical region order @80×24 | `tests/ui/shell_render_tests.rs > canonical_region_order_at_80x24_with_notice_filling_final_row` ➕ `tests/ui/team_widget_tests.rs > order_and_chaos_columns_render_side_by_side` (adjacency SHALL clause) ➕ `degradation_tests > team_columns_sit_side_by_side_wherever_the_body_region_is_visible` (12k-size invariant sweep) | ✅ COMPLIANT (scenario-as-written) |
| viz:R2 | S2 notice survives 80×12 | `minimum_viewport_keeps_regions_ordered_above_an_intact_status_row_80x12` (+ `status_row_stays_final_across_degenerate_heights`, `notice_stays_on_the_final_row_across_all_tier_boundaries`) | ✅ COMPLIANT |
| viz:R3 CS bars | S1 shared-maximum scaling | `tests/ui/team_widget_tests.rs > cs_bars_scale_to_the_shared_maximum_across_both_teams` (spec vectors 40/120/300, cross-team max preserved post-U7) | ✅ COMPLIANT |
| viz:R3 | S2 missing CS placeholder | `missing_creep_score_renders_placeholder_while_other_bars_render` (+ `all_zero_creep_scores_render_true_zero_bars_without_placeholders`) | ✅ COMPLIANT |
| viz:R4 level bars | S1 fixed endpoints | `level_bars_pin_fixed_scale_endpoints` (+ unit pin `level_fill_cells_follows_the_fixed_mapping_exactly`: 1→0 cells, 18→10 cells) | ✅ COMPLIANT |
| viz:R4 | S2 overrange clamps safely | `overrange_and_underrange_levels_clamp_without_panicking` (25→full, 0→empty, `u32::MAX` saturates) | ✅ COMPLIANT |
| viz:R5 K/D/A + truncation | S1 single truncation policy | `tests/history/main.rs > conversion_tests::fractional_values_truncate_toward_zero` (195.7 & 195.2 ⇒ 195) + `non_finite_inputs_map_to_absent` + `negative_inputs_saturate_at_zero`; app-side `app_tests > gold_samples_route_through_the_shared_truncation_policy` | ✅ COMPLIANT |
| viz:R5 | S2 zero deaths fabricates nothing | `zero_deaths_keep_all_three_segments_and_display_no_numbers` (asserts zero digits on row ⇒ no ratios possible) | ✅ COMPLIANT |
| viz:R5 | S3 color binding degrades gracefully | `kda_segments_bind_to_green_red_and_blue` (Green/Red/Blue binding pinned offline; host palette degradation itself = manual §11.2) | ✅ COMPLIANT (offline scope) |
| viz:R6 inventory bar | S1 partial fill | `inventory_strip_shows_partial_fill_and_skips_null_slots` (exact (3 filled, 3 empty)) | ✅ COMPLIANT |
| viz:R6 | S2 absent list placeholder | `absent_items_list_renders_placeholder_without_touching_other_strips` (+ trinket-exclusion and saturation pins) | ✅ COMPLIANT |
| viz:R7 local gauges | S1 gauge fill reflects snapshot | `tests/ui/local_strip_tests.rs > gauges_fill_70_and_80_percent_for_the_local_player_only` (2100/3000⇒0.7, 400/500⇒0.8 ±1 cell; row-uniqueness proves non-local players never get gauges) | ✅ COMPLIANT |
| viz:R7 | S2 missing stat degrades | `missing_power_renders_placeholder_while_the_hp_gauge_renders_normally` (+ degeneracy vectors `gauge_ratio_follows_current_over_max…`, panic-free clamp `overrange_health_clamps_to_a_full_gauge_without_panicking`) | ✅ COMPLIANT |
| viz:R8 gold sparkline | S1 warm-up placeholder | `warm_up_placeholder_shows_until_two_real_samples_exist` (absent gold & gaps never inflate n) | ✅ COMPLIANT |
| viz:R8 | S2 same-game reconnect keeps trend | `tests/app_tests.rs > notbound_round_trip_preserves_the_same_game_trend` + `same_game_snapshots_append_oldest_to_newest` (fold code: Lifecycle::NotInGame deliberately preserves identity, design D7) | ✅ COMPLIANT |
| viz:R8 | S3 different game resets | `different_game_resets_then_pushes_only_the_new_sample` + `game_mode_change_resets_the_trend` (+ `identity_tests::*` truth table incl. ±5.0 s edges and degraded payloads) | ✅ COMPLIANT |
| viz:R8 | S4 capacity bounded | `ring_buffer_tests::wrapping_push_drops_oldest_and_pins_length_to_capacity` + `gold_history_storage_stays_within_the_32kib_bound` (1936 B ≤ 32 KiB) | ✅ COMPLIANT |
| viz:R9 degradation matrix | S1 monotonic ordered hiding | `degradation_tests > hiding_is_monotonic_between_smaller_and_larger_viewports` (sampled grid pairs) + `families_hide_strictly_in_the_documented_priority_order` (h=60→1 walk) + `tier_boundaries_follow_the_documented_height_table` + `widths_below_40_hide_every_chart_family` | ✅ COMPLIANT |
| viz:R9 | S2 status never hides | `status_row_is_present_for_every_viewport_from_1x1_to_200x60` (exhaustive 12 000 sizes; status == exact final row; pairwise region disjointness — sweep extended by U7 to also pin column invariants at each size) | ✅ COMPLIANT |
| viz:R10 glyph-safety proof | S1 full-frame purity | `full_frame_purity_scan_at_80x24_with_complete_data` (two same-game snapshots ⇒ all six families live incl. sparkline chart mode) | ✅ COMPLIANT |
| viz:R10 | S2 manual checklist covers widgets | `docs/manual-verify.md` §11.1–§11.4 verified present, exercising all six families + conhost smoke + tier walk-down + reconnect (static doc evidence; execution = pending manual item) | ✅ COMPLIANT |

**Compliance summary**: 23/23 scenarios compliant at runtime; all 10 requirements fully implemented with no open spec warnings — W-1 is resolved (U7); the only open finding is hygiene-gate FMT-1 (see Issues).

### Correctness (Static Evidence)
| Requirement | Status | Notes |
|------------|--------|-------|
| viz:R1 whitelist sole source | ✅ Implemented | 17 codepoints (U+2500–2518 ∪ U+2581–2588 ∪ U+2591–2593), closed 10-variant `Glyph` enum, `const fn symbol()`; zero runtime construction (grep `from_u32\|char::from\|Glyph::from` over src = 0 hits) |
| viz:R2 region layout | ✅ Implemented (post-U7) | Vertical regions + status-LAST structural reservation AND side-by-side ORDER/CHAOS columns via `split_team_columns` (`Percentage(50)/(50)`); disjointness/edge-closure/non-vanishing swept exhaustively 1×1→200×60; degenerate width<2 handled deterministically and contained |
| viz:R3 CS bars | ✅ Implemented | Global shared max via `chart_u64`; absent ⇒ `?`; max 0 ⇒ true zeros; global-across-columns property explicitly preserved by U7 |
| viz:R4 level bars | ✅ Implemented | `(level−1)/17` clamp [0,1], fixed 10-cell track |
| viz:R5 K/D/A + policy | ✅ Implemented | Per-metric shared maxima (still cross-team after U7); Green/Red/Blue styled spans; ratios never computed (zero-digit assertion); `chart_u64` sole f64→u64 path |
| viz:R6 inventory bar | ✅ Implemented | 6-cell ▓/░ strip; trinket slot excluded (unprovable presence); absent list ⇒ `?` |
| viz:R7 local gauges | ✅ Implemented | `gauge_ratio` clamps BEFORE `LineGauge::ratio` (which panics outside [0,1]); isolated per-row `?` degradation |
| viz:R8 gold sparkline | ✅ Implemented | `RingBuffer<u64,120>` preallocated (1936 B); app-side fold; gaps as `None` → visible ░ breaks; warm-up `(n/120)` on Some-count < 2 |
| viz:R9 degradation matrix | ✅ Implemented | Pure `select_layout`; tiers 13/15/18/20/24; width<40 override; gauges untiered structurally (not `ChartSet` members); now also emits the side-by-side columns purely |
| viz:R10 offline verification | ✅ Implemented | Dual nets (export range + full-frame purity scan); manual checklist §11 delivered |

### Coherence (Design)
| Decision | Followed? | Notes |
|----------|-----------|-------|
| D1 game identity | ✅ Yes | 5.0 s tolerance + mode change; degraded payloads conservative (`identity_tests` truth table) |
| D2 region layout | ✅ Yes (letter deviation, disclosed) | Status row RESERVED pre-solver instead of 5th constraint; local band Length(4) vs 2–3 — see Deviations (a)/(c) |
| D3 widget mapping | ✅ Yes (post-U7) | Native BarChart/Sparkline/LineGauge confirmed against vendored ratatui 0.30.2 (task 4.1 record in team.rs docs); **`Percentage(50)/(50)` team columns landed exactly per design letter in U7** — design.md required no edit (S-2 resolved) |
| D4 ring buffer app-side | ✅ Yes | Poller thread untouched (poller spec intact, api 19 green); lifecycle never touches history |
| D5 truncation policy | ✅ Yes | Sole path `chart_u64`; integer counters bypass by design |
| D6 warm-up | ✅ Yes | `< 2 Some points` ⇒ text, never flat line |
| D7 reconnect fold | ✅ Yes | classify→clear→push; NotBound round-trip continuity proven |
| D8 glyph enforcement | ✅ Yes | Static constants-only; two offline nets + sensitivity controls |
| D9 degradation matrix | ✅ Yes | Tier table matches exactly; monotonicity + priority order property-proven; column emission folded into the same pure function |
| D10 no new traits | ✅ Yes | Zero new crates (task 0.1); pure seams only |

### TDD Compliance
| Check | Result | Details |
|-------|--------|---------|
| TDD Evidence reported | ✅ | Per-unit RED/GREEN cycle evidence in apply-progress (Engram obs #29, U1–U6); U7 remediation evidence in commit `26c4e7f` message + this phase's independent run |
| All tasks have tests | ✅ | 25/25 tasks map to named passing tests (matrix above); U7 adds two named pins beyond the authored set |
| RED confirmed (tests exist & ran red) | ✅ | U1–U5 document genuine RED states (incl. U5's ghost-path lesson: probe moved to 39×60 after a vacuous pass at 39×24); U6 was verification/docs-only — approval-style REFACTOR steps each bracketed by the standing green suite, disclosed |
| GREEN confirmed on execution | ✅ | This phase's independent CT run on main @ `96d63f8`: 142/0, exit 0 |
| Triangulation adequate | ✅ | Boundary tables (h∈{1,12,13,…,60}), ~10k-combo monotonic subset property, exhaustive 12k-size sweep now covering BOTH status-row and column invariants, truth-table vectors, ±1-cell proportional tolerance, plus a buffer-level adjacency pin |
| Safety Net for modified files | ✅ | Baseline CT before every unit; standing pins caught a real regression in U6; all standing pins green unweakened after U7's signature change (`render` now takes `TeamColumns`) |

**TDD Compliance**: 6/6 checks passed

### Test Layer Distribution
| Layer | Tests | Files | Tools |
|-------|-------|-------|-------|
| Unit (pure fns) | 19 history + ~6 in-module pins | `tests/history/*`, unit pins in team/local_strip test files | built-in `cargo test` |
| Integration (TestBackend render + fold) | 68 ui + 28 app | `tests/ui/*.rs`, `tests/app_tests.rs` | ratatui `TestBackend` |
| E2E | 0 | — | manual checklist `docs/manual-verify.md` §11 (pending human run) |
| **Total** | **142** | | |

### Changed File Coverage
Coverage analysis skipped — no coverage tool detected (`coverage_threshold: 0`). Not a failure.

### Assertion Quality
✅ All assertions verify real behavior. Audit of `team_widget_tests.rs`, `local_strip_tests.rs`, `glyph_tests.rs`, `degradation_tests.rs`, `shell_render_tests.rs`: no tautologies, no ghost loops (every filtered collection is length-pinned), anti-vacuity controls present (`exports_at_least_one_glyph`, injected-Braille detector control, U5 ghost-path lesson applied), behavioral assertions on buffer content rather than implementation internals. U7's additions hold the same bar: the 12k-size column sweep asserts returned-Rect geometry (pure output), and `order_and_chaos_columns_render_side_by_side` asserts rendered buffer content per physical row — not internals.

**Assertion quality**: 0 CRITICAL, 0 WARNING

### Quality Metrics
**Linter**: ❌ `cargo fmt --all -- --check` **exit 1 — FMT-1** (see Issues). Diff limited to two U7-introduced spots: `src/ui/mod.rs:191` (the `Layout::horizontal([...]).split(body)` chain wraps differently than rustfmt canonical form) and `tests/ui/team_widget_tests.rs:668` (boolean expression should be single-line). Zero behavioral content.
**Type/Lint Checker**: ✅ `cargo clippy -j 1 --all-targets -- -D warnings` exit 0 (independently re-run this phase).

### Deviations Adjudication (disclosed in apply-progress)
| # | Deviation | Verdict | Severity |
|---|-----------|---------|----------|
| a | U2/D2: status row reserved pre-solver instead of fifth `Layout` constraint | ACCEPTED — strengthens the guarantee: cassowary deficit distribution below the tuple minimum is unspecified, so reservation makes ui:R6/viz:R2 deterministic; exhaustive 12k-size sweep proves it | SUGGESTION (design-letter divergence, intent honored) |
| b | U3/D3: side-by-side team columns deferred to P5 | **RESOLVED by U7** — commit `26c4e7f` implemented the horizontal `Percentage(50)/(50)` column split exactly per spec viz:R2's and design D3's letter; adjacency, disjointness, edge closure, non-vanishing, and containment are runtime-proven (two new pins, 12k-size sweep). No spec amendment was made or needed | Resolved (was WARNING W-1) |
| c | U4/D2: local band Length(2–3) → Length(4) | ACCEPTED — legacy LOCAL text line kept byte-for-byte (ui-spec continuity pin) + HP + Power + GOLD rows; MIN_REGIONS_HEIGHT 9→11 and greedy fallback reconcile it; no spec clause pins band height; 80×12 scenario passes | SUGGESTION (disclosed, documented) |
| d | U5/D2 reconciliation | VERIFIED CONSISTENT — tier math over the Length(4) band, width<40 override cannot resurrect height-hidden charts, gauges untierable by construction (no `ChartSet` flag), hidden sparkline blanks the gold row without moving bands | Note only |

Additional disclosed micro-decisions verified spec-conformant: warm-up n counts REAL (`Some`) samples (matches "below 2 samples"); trim-to-newest window slice (trend tracks present, gaps preserved — tested at 130 polls); K/D/A fixed 8-cell tracks (frozen geometry); inventory occupancy requires `item_id` (null entry ≡ empty slot, per spec "absent/null slots render empty").

### Compliance Re-Audit (protocol items, re-run on main @ `96d63f8`)
1. **No derived timers** ✅ — respawn rendered verbatim (`DEAD(respawn {t})` or `?` marker; standing test `dead_player_without_exposed_timer_shows_unknown_marker_never_a_countdown`); status timestamp derives from stamped receipt millis, never gameTime; grep `Instant::` over src = 0 matches.
2. **Gold exclusively local** ✅ — `src/ui/team.rs` contains zero gold references; `current_gold` consumed solely via `snapshot.local` → LOCAL line + local-strip widgets; uniqueness assertion `unique_gold_row` (exactly one GOLD row); standing net `missing_local_gold_degrades_and_no_enemy_panel_shows_gold`. U7's column split did not move any gold surface.
3. **Glyph whitelist sole source (closed enum)** ✅ — zero `from_u32`/`char::from`/`Glyph::from` in src; widgets draw only through exported constants/enum; dual nets + detector controls green; full-frame purity re-proven post-U7 with both columns drawing.
4. **Riot notice preserved (ui:R6)** ✅ — `RIOT_NOTICE` mandated-wording test; drawn LAST in both views; exhaustive 1×1→200×60 sweep pins final-row ownership with pairwise-disjoint regions (sweep re-run green with column invariants added); `live-client-poller` spec untouched (history feeds app-side per D4; api suite 19/19 unchanged).

### Pending Manual §11 Items (cannot be proven offline)
Real-game rendering of all six families (both hosts) · conhost glyph/font smoke (no replacement glyphs) · 16-color K/D/A degradation visual · resize walk-down across tiers with reverse regrowth (now visually exercising the side-by-side columns) · reconnect same-game trend continuation / different-game reset on a real connection. Checklist: `docs/manual-verify.md` §11 (items unchecked, awaiting human execution).

### Issues Found
**CRITICAL**: None.
**WARNING**:
- **FMT-1 (hygiene gate, material but mechanical)**: `cargo fmt --all -- --check` exits 1 on main @ `96d63f8`. The entire diff is two rustfmt-canonicalization spots in U7-introduced code: `src/ui/mod.rs:191` (`Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(body)` should collapse onto fewer lines) and `tests/ui/team_widget_tests.rs:668` (the `chaos_renders` boolean expression should be single-line). No behavioral impact — CB/CT/clippy all exit 0 — but task 6.1's "cargo fmt clean" claim regressed post-U6, so the tree no longer satisfies its own documented gate. Resolve before archive: run `cargo fmt` (or apply the two-line diff), then re-run the fmt gate; expected instant green with zero semantic delta.
**SUGGESTION**:
- S-1: Consolidate the accepted deviations (status-row reservation, Length(4) band, trim-to-newest window, warm-up Some-count) into a short errata note on design.md (or the archive record) so future readers do not treat D2 letters as literal. Still open.
- S-2 (RESOLVED by U7): update design.md D3 if the presentation chosen for W-1 diverged from it — it did not: U7 shipped exactly D3's documented `Percentage(50)/(50)` split, so design.md is accurate as written and required no edit. Recorded here for traceability of the original finding.

### Verdict
**PASS WITH WARNINGS** — WARN-1 is fully resolved: viz:R2's side-by-side ORDER/CHAOS clause is implemented per spec and design letter and runtime-proven (23/23 scenarios, 142/142 tests, all requirements ✅). One remaining WARNING (FMT-1): U7 introduced a cosmetic `cargo fmt --check` regression on two of its own lines; mechanical one-command fix required before `sdd-archive`. The planned downgrade to PASS WITH SUGGESTIONS is withheld solely because of FMT-1; once the fmt gate is green, the change is archive-ready with suggestions only.
