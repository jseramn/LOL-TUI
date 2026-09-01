//! Team-column visualization contracts (tasks 4.2–4.5; viz spec R3–R6).
//!
//! Each player gets ONE additive visualization row rendered inside the body
//! region BELOW the legacy text panels (which remain byte-identical, pinned
//! by `dashboard_tests.rs`). Rows follow the fixed-geometry contract built
//! in team.rs: `Lv<track> K<track> D<track> A<track> CS<chart> <inventory>`.
//!
//! Snapshots are synthesized in-test (network-free, design D5) and asserted
//! through ratatui [`TestBackend`] buffers.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::snapshot::{PlayerSnapshot, Snapshot, Team};
use tui_lol::ui::select_layout;

/// Wide enough that every fixed section plus a usable CS chart fits INSIDE
/// ONE team column: the body splits side by side (viz spec R2), so each
/// column here is 120 cells.
const WIDTH: u16 = 240;
const HEIGHT: u16 = 24;

/// A player with every field absent except the ones the caller sets.
fn ps(team: Option<Team>, creep_score: Option<f64>) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: None,
        champion: None,
        team,
        position: None,
        level: None,
        kills: None,
        deaths: None,
        assists: None,
        creep_score,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: None,
        respawn_timer: None,
    }
}

fn live_app_with(players: Vec<PlayerSnapshot>) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(Snapshot {
        players,
        ..Snapshot::default()
    })));
    app
}

fn draw(app: &App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend");
    let frame = terminal
        .draw(|f| tui_lol::ui::render(f, app))
        .expect("frame");
    frame.buffer.clone()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

/// Visualization rows, extracted PER TEAM COLUMN (viz spec R2: ORDER left,
/// CHAOS right): a physical body row can carry one row from EACH column, so
/// each column's cells are sliced out separately and concatenated in roster
/// order — ORDER rows first, then CHAOS rows — each starting with the `Lv`
/// section label at its own column origin. No legacy text row can produce a
/// match (those begin with a name/champion token).
fn viz_rows(buffer: &Buffer) -> Vec<String> {
    let columns = select_layout(Rect::new(0, 0, WIDTH, HEIGHT)).columns;
    let segment = |column: ratatui::layout::Rect, y: u16| -> String {
        (column.x..column.x + column.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    };
    let mut rows = Vec::new();
    for y in 0..buffer.area.height {
        let order_row = segment(columns.order, y);
        if order_row.starts_with("Lv") {
            rows.push(order_row);
        }
        let chaos_row = segment(columns.chaos, y);
        if chaos_row.starts_with("Lv") {
            rows.push(chaos_row);
        }
    }
    rows
}

/// The shared-max CS chart area of a visualization row: everything after
/// the `CS` label UP TO the trailing separator + six inventory cells.
fn cs_segment(row: &str) -> String {
    let chars: Vec<char> = row.chars().collect();
    let label = chars
        .windows(2)
        .position(|w| w == ['C', 'S'])
        .expect("CS section label on visualization row");
    let end = chars.len().saturating_sub(7).max(label + 2);
    chars[label + 2..end].iter().collect()
}

// --- Task 4.2 / viz spec R3: per-team creep score bars ----------------------

/// viz:R3/S1 — bar length is proportional to creepScore versus the highest
/// CS among ALL visible players: the maximum spans the full chart width and
/// shorter values render proportionally (±1 cell rounding tolerance),
/// including across teams (the max here belongs to CHAOS).
#[test]
fn cs_bars_scale_to_the_shared_maximum_across_both_teams() {
    let app = live_app_with(vec![
        ps(Some(Team::Order), Some(40.0)),
        ps(Some(Team::Chaos), Some(120.0)),
        ps(Some(Team::Chaos), Some(300.0)),
    ]);
    let buffer = draw(&app);

    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 3, "one visualization row per visible player");

    let widths: Vec<usize> = rows
        .iter()
        .map(|row| cs_segment(row).chars().filter(|c| *c == '\u{2588}').count())
        .collect();
    let full = widths[2]; // roster order preserved: the 300-CS player is last
    assert!(full >= 20, "shared-max bar must span the chart: {widths:?}");
    let expect = |cs: f64| (full as f64 * cs / 300.0).round() as isize;
    assert!(
        (widths[0] as isize - expect(40.0)).abs() <= 1,
        "CS 40 must render proportionally: {widths:?}"
    );
    assert!(
        (widths[1] as isize - expect(120.0)).abs() <= 1,
        "CS 120 must render proportionally: {widths:?}"
    );
}

/// viz:R3/S2 — an absent creepScore renders the `?` placeholder in the CS
/// section, NEVER an empty (zero-length) bar masquerading as data; other
/// players' bars render normally.
#[test]
fn missing_creep_score_renders_placeholder_while_other_bars_render() {
    let app = live_app_with(vec![
        ps(Some(Team::Order), Some(50.0)),
        ps(Some(Team::Chaos), None),
    ]);
    let buffer = draw(&app);

    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    let intact = cs_segment(&rows[0]);
    assert!(
        intact.chars().filter(|c| *c == '\u{2588}').count() > 0,
        "present CS must draw a bar: {intact:?}"
    );
    let absent = cs_segment(&rows[1]);
    assert!(
        absent.trim_start().starts_with('?'),
        "absent CS must show the placeholder: {absent:?}"
    );
    assert_eq!(
        absent.chars().filter(|c| *c == '\u{2588}').count(),
        0,
        "absent CS must never fabricate a bar: {absent:?}"
    );
}

/// viz:R3/S1 — a maximum of 0 leaves every PRESENT score at a true zero
/// bar (empty chart, no placeholder): absence (`?`) is reserved for fields
/// the payload did not expose.
#[test]
fn all_zero_creep_scores_render_true_zero_bars_without_placeholders() {
    let app = live_app_with(vec![
        ps(Some(Team::Order), Some(0.0)),
        ps(Some(Team::Chaos), Some(0.0)),
    ]);
    let buffer = draw(&app);

    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2, "one visualization row per visible player");
    for row in rows {
        let segment = cs_segment(&row);
        assert_eq!(
            segment.chars().filter(|c| *c == '\u{2588}').count(),
            0,
            "zero CS must be a true zero bar: {segment:?}"
        );
        assert!(
            !segment.contains('?'),
            "a present-but-zero CS is not absent: {segment:?}"
        );
    }
}

// --- Task 4.3 / viz spec R4: level bars on a fixed scale --------------------

/// Everything between the `Lv` label and the kills section: a 10-cell track
/// of filled (`█`) and empty (`░`) cells — or the absence placeholder.
fn level_track(row: &str) -> &str {
    let start = row.find("Lv").expect("level label on visualization row");
    let end = row[start..]
        .find(" K")
        .map(|i| start + i)
        .unwrap_or(row.len());
    &row[start + 2..end]
}

/// viz:R4/S1 — the scale NEVER rescales between frames: level 1 draws
/// near-empty (exactly zero filled cells) and level 18 draws full on the
/// same fixed 1–18 mapping.
#[test]
fn level_bars_pin_fixed_scale_endpoints() {
    let mut low = ps(Some(Team::Order), Some(9.0));
    low.level = Some(1);
    let mut maxed = ps(Some(Team::Chaos), Some(9.0));
    maxed.level = Some(18);

    let buffer = draw(&live_app_with(vec![low, maxed]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    assert_eq!(
        level_track(&rows[0])
            .chars()
            .filter(|c| *c == '\u{2588}')
            .count(),
        0,
        "level 1 must sit at the bottom of the fixed scale: {0:?}",
        rows[0]
    );
    let maxed_track = level_track(&rows[1]);
    assert_eq!(
        maxed_track.chars().filter(|c| *c == '\u{2588}').count(),
        10,
        "level 18 must fill the whole fixed track: {maxed_track:?}"
    );
}

/// viz:R4/S2 — out-of-range levels clamp into [0, 1] and never panic:
/// 25 renders full; 0 clamps to the empty end.
#[test]
fn overrange_and_underrange_levels_clamp_without_panicking() {
    let mut over = ps(Some(Team::Order), Some(9.0));
    over.level = Some(25);
    let mut under = ps(Some(Team::Chaos), Some(9.0));
    under.level = Some(0);

    let buffer = draw(&live_app_with(vec![over, under]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2, "both players render");

    assert_eq!(
        level_track(&rows[0])
            .chars()
            .filter(|c| *c == '\u{2588}')
            .count(),
        10,
        "level 25 must clamp to a full bar: {0:?}",
        rows[0]
    );
    assert_eq!(
        level_track(&rows[1])
            .chars()
            .filter(|c| *c == '\u{2588}')
            .count(),
        0,
        "level 0 must clamp to an empty bar: {0:?}",
        rows[1]
    );
}

// --- Task 4.4 / viz spec R5: K/D/A mini-bars --------------------------------

/// Frozen row-contract width of one K/D/A sub-track (mirrors team.rs).
const METRIC_TRACK_CELLS: usize = 8;

/// One labeled K/D/A sub-track (`K`, `D`, or `A`) of a visualization row:
/// the letter plus its FIXED-width track. Operates in CHAR space — block
/// glyphs are multi-byte — and must not stop at the first blank cell (an
/// empty track is blank by definition).
fn metric_track(row: &str, label: char) -> String {
    let chars: Vec<char> = row.chars().collect();
    let start = chars
        .windows(2)
        .position(|w| w == [' ', label])
        .unwrap_or_else(|| panic!("missing {label} section"));
    let end = (start + 2 + METRIC_TRACK_CELLS).min(chars.len());
    chars[start + 1..end].iter().collect()
}

fn filled_cells(track: impl AsRef<str>) -> usize {
    track.as_ref().chars().filter(|c| *c == '\u{2588}').count()
}

/// viz:R5 — each mini-bar scales by ITS OWN metric's shared maximum across
/// visible players (not by any global counter max): with kills 2/4,
/// deaths 1/2 and assists 4/8 across two players, every second player bar
/// fills its whole 8-cell track and every first-player bar fills exactly
/// half of it — independently per metric.
#[test]
fn kda_mini_bars_scale_by_each_metrics_own_shared_max() {
    let mut alpha = ps(Some(Team::Order), Some(9.0));
    (alpha.kills, alpha.deaths, alpha.assists) = (Some(2), Some(1), Some(4));
    let mut beta = ps(Some(Team::Chaos), Some(9.0));
    (beta.kills, beta.deaths, beta.assists) = (Some(4), Some(2), Some(8));

    let buffer = draw(&live_app_with(vec![alpha, beta]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    let first_k = metric_track(&rows[0], 'K');
    let second_k = metric_track(&rows[1], 'K');
    assert_eq!(filled_cells(&first_k), 4, "kills half-of-max: {first_k}");
    assert_eq!(filled_cells(&second_k), 8, "kills at-max: {second_k}");

    let first_d = metric_track(&rows[0], 'D');
    let second_d = metric_track(&rows[1], 'D');
    assert_eq!(
        filled_cells(&first_d),
        4,
        "deaths scale independently: {first_d}"
    );
    assert_eq!(filled_cells(&second_d), 8, "deaths at-max: {second_d}");

    let first_a = metric_track(&rows[0], 'A');
    let second_a = metric_track(&rows[1], 'A');
    assert_eq!(
        filled_cells(&first_a),
        4,
        "assists scale independently: {first_a}"
    );
    assert_eq!(filled_cells(&second_a), 8, "assists at-max: {second_a}");
}

/// viz:R5/S3 — the three segments bind to green/red/blue respectively; the
/// palette may degrade on 16-color hosts, but the binding itself is fixed.
#[test]
fn kda_segments_bind_to_green_red_and_blue() {
    let mut player = ps(Some(Team::Order), Some(9.0));
    (player.kills, player.deaths, player.assists) = (Some(1), Some(1), Some(1));

    let buffer = draw(&live_app_with(vec![player]));
    let row_y = (0..buffer.area.height)
        .find(|&y| row_text(&buffer, y).starts_with("Lv"))
        .expect("visualization row");

    let mut seen = std::collections::HashSet::new();
    for x in 0..buffer.area.width {
        let cell = &buffer[(x, row_y)];
        if cell.symbol() != " " {
            seen.insert(cell.fg);
        }
    }
    for expected in [
        ratatui::style::Color::Green,
        ratatui::style::Color::Red,
        ratatui::style::Color::Blue,
    ] {
        assert!(
            seen.contains(&expected),
            "segment color binding missing {expected:?}: {seen:?}"
        );
    }
}

/// viz:R5/S2 — a zero death keeps ALL THREE segments drawn (letters never
/// vanish) while the death track stays truly empty, and no numeric value —
/// raw or derived, e.g. a (K+A)/D ratio — ever appears on a visualization
/// row: they carry glyphs and letters only.
#[test]
fn zero_deaths_keep_all_three_segments_and_display_no_numbers() {
    let mut player = ps(Some(Team::Order), Some(9.0));
    (player.kills, player.deaths, player.assists) = (Some(3), Some(0), Some(2));

    let buffer = draw(&live_app_with(vec![player]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    for label in ['K', 'D', 'A'] {
        assert!(
            metric_track(row, label).starts_with(label),
            "all three labeled segments must stay drawn: {row:?}"
        );
    }
    assert_eq!(
        filled_cells(metric_track(row, 'D')),
        0,
        "zero deaths must be a true zero bar: {row:?}"
    );
    assert!(
        !row.chars().any(|c| c.is_ascii_digit()),
        "no counters or derived ratios belong on visualization rows: {row:?}"
    );
}

/// Absent counters degrade per section: an omitted kills value shows `?`
/// while sibling metrics keep scaling normally.
#[test]
fn absent_counter_renders_placeholder_without_touching_sibling_metrics() {
    let mut absent_kills = ps(Some(Team::Order), Some(9.0));
    (
        absent_kills.kills,
        absent_kills.deaths,
        absent_kills.assists,
    ) = (None, Some(2), Some(2));
    let mut intact = ps(Some(Team::Chaos), Some(9.0));
    (intact.kills, intact.deaths, intact.assists) = (Some(4), Some(4), Some(4));

    let buffer = draw(&live_app_with(vec![absent_kills, intact]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    let kills = metric_track(&rows[0], 'K');
    assert!(
        kills.starts_with("K?"),
        "absent kills must show the placeholder: {kills:?}"
    );
    assert_eq!(filled_cells(kills), 0, "placeholder fabricates no bar");
    assert_eq!(
        filled_cells(metric_track(&rows[0], 'D')),
        4,
        "deaths still scale by their own shared max"
    );
    assert_eq!(
        filled_cells(metric_track(&rows[1], 'K')),
        8,
        "other players' bars unaffected"
    );
}

// --- Task 4.5 / viz spec R6: inventory fill bar -----------------------------

use tui_lol::model::snapshot::ItemSnapshot;

fn item(slot: Option<u8>, item_id: Option<u32>) -> ItemSnapshot {
    ItemSnapshot {
        display_name: item_id.map(|_| "Item".to_owned()),
        item_id,
        count: Some(1),
        slot,
    }
}

/// The strip is the FINAL six cells of a visualization row: dark-shade
/// filled cells followed by light-shade empty ones.
fn inventory_strip(row: &str) -> String {
    row.chars().skip(row.chars().count() - 6).collect()
}

fn player_with_items(team: Option<Team>, items: Option<Vec<ItemSnapshot>>) -> PlayerSnapshot {
    let mut p = ps(team, Some(9.0));
    p.items = items;
    p
}

/// viz:R6/S1 — three occupied slots and three empty ones draw the exact
/// half-filled extent; null slots (entries without an item id) stay empty.
#[test]
fn inventory_strip_shows_partial_fill_and_skips_null_slots() {
    let player = player_with_items(
        Some(Team::Order),
        Some(vec![
            item(Some(0), Some(3006)),
            item(Some(1), Some(6672)),
            item(Some(2), Some(3153)),
            item(Some(3), None), // null slot exposed as an entry
            item(Some(4), None),
        ]),
    );

    let buffer = draw(&live_app_with(vec![player]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 1);

    let strip = inventory_strip(&rows[0]);
    let filled = strip.chars().filter(|c| *c == '\u{2593}').count();
    let empty = strip.chars().filter(|c| *c == '\u{2591}').count();
    assert_eq!(
        (filled, empty),
        (3, 3),
        "three real items must fill exactly half the strip: {strip}"
    );
}

/// Trinkets can never be proven present, so slot 6 NEVER fills a cell;
/// a slotless entry counts (it cannot be proven to be the trinket either).
#[test]
fn inventory_excludes_trinket_slot_but_counts_slotless_items() {
    let player = player_with_items(
        Some(Team::Order),
        Some(vec![
            item(Some(0), Some(3006)),
            item(Some(6), Some(2055)), // trinket slot: excluded per spec
            item(None, Some(1039)),    // slotless: cannot prove trinket
        ]),
    );

    let buffer = draw(&live_app_with(vec![player]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 1);

    let strip = inventory_strip(&rows[0]);
    let filled = strip.chars().filter(|c| *c == '\u{2593}').count();
    assert_eq!(filled, 2, "trinket excluded, slotless counted: {strip}");
}

/// A wholly absent items list shows the placeholder while other players'
/// strips render normally (viz:R6/S2).
#[test]
fn absent_items_list_renders_placeholder_without_touching_other_strips() {
    let mut intact = player_with_items(
        Some(Team::Order),
        Some(vec![
            item(Some(0), Some(3006)),
            item(Some(1), Some(6672)),
            item(Some(2), Some(3153)),
            item(Some(3), Some(3026)),
            item(Some(4), Some(3153)),
            item(Some(5), Some(1056)),
        ]),
    );
    intact.items = Some(Vec::new()); // present-but-empty: six empty cells
    let absent = player_with_items(Some(Team::Chaos), None);

    let buffer = draw(&live_app_with(vec![intact, absent]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    let empty_strip = inventory_strip(&rows[0]);
    assert_eq!(
        empty_strip.chars().filter(|c| *c == '\u{2591}').count(),
        6,
        "an empty list is six truly empty cells: {empty_strip}"
    );
    assert!(!empty_strip.contains('\u{2593}'));

    let absent_strip = inventory_strip(&rows[1]);
    assert!(
        absent_strip.starts_with('?'),
        "wholly absent list must show the placeholder: {absent_strip}"
    );
}

/// Degenerate payloads listing more than six non-trinket items saturate at
/// the strip width — never panic, never overflow the section.
#[test]
fn oversized_inventories_saturate_at_six_cells() {
    let player = player_with_items(
        Some(Team::Order),
        Some((0..=7).map(|slot| item(Some(slot), Some(3001))).collect()),
    );

    let buffer = draw(&live_app_with(vec![player]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 1);

    let strip = inventory_strip(&rows[0]);
    assert_eq!(
        strip.chars().filter(|c| *c == '\u{2593}').count(),
        6,
        "eight items saturate at six filled cells: {strip}"
    );
}

/// Unit-layer pin of the fixed mapping itself, including the absurd-input
/// saturations the rendered tests cannot reach through a payload.
#[test]
fn level_fill_cells_follows_the_fixed_mapping_exactly() {
    assert_eq!(tui_lol::ui::team::level_fill_cells(1), 0);
    assert_eq!(tui_lol::ui::team::level_fill_cells(18), 10);
    assert_eq!(tui_lol::ui::team::level_fill_cells(0), 0);
    assert_eq!(tui_lol::ui::team::level_fill_cells(25), 10);
    assert_eq!(tui_lol::ui::team::level_fill_cells(u32::MAX), 10);
}

/// Absent level renders the explicit placeholder in its own track only;
/// another player's bar is unaffected.
#[test]
fn absent_level_renders_placeholder_while_other_bars_render() {
    let mut present = ps(Some(Team::Order), Some(9.0));
    present.level = Some(18);
    let absent = ps(Some(Team::Chaos), Some(9.0));

    let buffer = draw(&live_app_with(vec![present, absent]));
    let rows = viz_rows(&buffer);
    assert_eq!(rows.len(), 2);

    assert_eq!(
        level_track(&rows[0])
            .chars()
            .filter(|c| *c == '\u{2588}')
            .count(),
        10,
        "populated level keeps its bar: {0:?}",
        rows[0]
    );
    let absent_track = level_track(&rows[1]);
    assert!(
        absent_track.trim_start().starts_with('?'),
        "absent level must show the placeholder: {absent_track:?}"
    );
}

/// The visualization block is additive and regional: each row sits BELOW
/// the legacy text panel of its team, inside the body band, and never
/// touches the status row.
#[test]
fn team_visualizations_stay_between_text_panels_and_status_row() {
    let app = live_app_with(vec![ps(Some(Team::Order), Some(12.0))]);
    let buffer = draw(&app);
    let height = buffer.area.height;

    let team_header_y = (0..height)
        .find(|&y| row_text(&buffer, y).contains("Team ORDER"))
        .expect("text panels must keep rendering");

    let ys: Vec<u16> = (0..height)
        .filter(|&y| row_text(&buffer, y).starts_with("Lv"))
        .collect();
    assert_eq!(ys.len(), 1, "exactly one visualization row");
    assert!(
        ys[0] > team_header_y,
        "visualization must sit below the text panels"
    );
    assert!(
        ys[0] < height - 1,
        "visualization must never leak into the status row"
    );
}

/// viz:R2 (remediation W-1) — ORDER and CHAOS columns sit SIDE BY SIDE:
/// with one player per team both visualization rows share ONE physical
/// body row, ORDER starting at the body's left edge and CHAOS beginning at
/// the body's horizontal midpoint. Stacked rendering fails this: the CHAOS
/// row occupies its own lower row and leaves the right half blank.
#[test]
fn order_and_chaos_columns_render_side_by_side() {
    let app = live_app_with(vec![
        ps(Some(Team::Order), Some(9.0)),
        ps(Some(Team::Chaos), Some(9.0)),
    ]);
    let buffer = draw(&app);

    // Cell index == char index: every buffer symbol is one cell wide.
    let row_chars = |y: u16| -> Vec<char> { row_text(&buffer, y).chars().collect() };

    let mid = {
        let body = select_layout(Rect::new(0, 0, WIDTH, HEIGHT)).areas.body;
        usize::from(body.x + body.width / 2)
    };
    let starts_with_lv = |cells: &[char]| cells.starts_with(&['L', 'v']);

    // Existence controls (anti-vacuity) — true whether teams stack or sit
    // side by side, so the contract assertion below cannot pass vacuously.
    let left_rows: Vec<u16> = (0..buffer.area.height)
        .filter(|&y| starts_with_lv(&row_chars(y)))
        .collect();
    assert!(
        !left_rows.is_empty(),
        "the ORDER visualization row must render at the body's left edge"
    );
    let chaos_renders = left_rows.len() > 1
        || (0..buffer.area.height).any(|y| row_chars(y).get(mid..).is_some_and(starts_with_lv));
    assert!(chaos_renders, "the CHAOS visualization row must render");

    // THE side-by-side contract: one physical row hosts both columns.
    let side_by_side = (0..buffer.area.height).any(|y| {
        let cells = row_chars(y);
        starts_with_lv(&cells) && cells.get(mid..).is_some_and(starts_with_lv)
    });
    assert!(
        side_by_side,
        "ORDER and CHAOS visualization rows must share a physical row, \
         CHAOS beginning at column {mid}"
    );
}

/// viz spec R2 + R9: at the canonical 80×24 viewport each team column is
/// ~40 cells. Charts must still draw (compact tracks), not clip away
/// behind a 45-cell full-width prefix.
#[test]
fn charts_fit_inside_canonical_80x24_team_columns() {
    let mut order = ps(Some(Team::Order), Some(200.0));
    order.level = Some(18);
    (order.kills, order.deaths, order.assists) = (Some(5), Some(2), Some(4));
    order.items = Some(vec![
        item(Some(0), Some(3006)),
        item(Some(1), Some(6672)),
        item(Some(2), Some(3153)),
    ]);
    let mut chaos = ps(Some(Team::Chaos), Some(80.0));
    chaos.level = Some(1);
    (chaos.kills, chaos.deaths, chaos.assists) = (Some(1), Some(1), Some(1));
    chaos.items = Some(vec![item(Some(0), Some(1055))]);

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");
    let app = live_app_with(vec![order, chaos]);
    let frame = terminal
        .draw(|f| tui_lol::ui::render(f, &app))
        .expect("frame");
    let buffer = frame.buffer.clone();
    let columns = select_layout(Rect::new(0, 0, 80, 24)).columns;

    let segment = |column: ratatui::layout::Rect, y: u16| -> String {
        (column.x..column.x + column.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    };
    let viz_in = |column: ratatui::layout::Rect| -> Vec<String> {
        (0..buffer.area.height)
            .map(|y| segment(column, y))
            .filter(|row| row.starts_with("Lv"))
            .collect()
    };

    let order_rows = viz_in(columns.order);
    let chaos_rows = viz_in(columns.chaos);
    assert_eq!(order_rows.len(), 1, "ORDER viz row: {order_rows:?}");
    assert_eq!(chaos_rows.len(), 1, "CHAOS viz row: {chaos_rows:?}");

    let has_block = |row: &str| row.chars().any(|c| c == '\u{2588}' || c == '\u{2591}');
    assert!(
        has_block(&order_rows[0]),
        "ORDER charts must render at 80x24: {}",
        order_rows[0]
    );
    assert!(
        has_block(&chaos_rows[0]),
        "CHAOS charts must render at 80x24: {}",
        chaos_rows[0]
    );
    assert!(
        order_rows[0].contains("CS"),
        "CS section must remain: {}",
        order_rows[0]
    );
    let inv = inventory_strip(&order_rows[0]);
    assert_eq!(
        inv.chars().filter(|c| c == &'\u{2593}').count(),
        3,
        "inventory strip must occupy the last six cells: {inv}"
    );
}
