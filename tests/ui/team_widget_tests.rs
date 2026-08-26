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
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::snapshot::{PlayerSnapshot, Snapshot, Team};

/// Wide enough that every fixed section plus a usable CS chart fits.
const WIDTH: u16 = 120;
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
    let frame = terminal.draw(|f| tui_lol::ui::render(f, app)).expect("frame");
    frame.buffer.clone()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

/// Visualization rows start with the `Lv` section label at column zero;
/// no legacy text row can (those begin with a name/champion token).
fn viz_rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| row_text(buffer, y))
        .filter(|row| row.starts_with("Lv"))
        .collect()
}

/// Everything after the `CS` label on a visualization row: the shared-max
/// chart area (and, once implemented, the trailing inventory strip).
fn cs_segment(row: &str) -> &str {
    let label = row.find("CS").expect("CS section label on visualization row");
    &row[label + 2..]
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
        level_track(&rows[0]).chars().filter(|c| *c == '\u{2588}').count(),
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
        level_track(&rows[0]).chars().filter(|c| *c == '\u{2588}').count(),
        10,
        "level 25 must clamp to a full bar: {0:?}",
        rows[0]
    );
    assert_eq!(
        level_track(&rows[1]).chars().filter(|c| *c == '\u{2588}').count(),
        0,
        "level 0 must clamp to an empty bar: {0:?}",
        rows[1]
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
        level_track(&rows[0]).chars().filter(|c| *c == '\u{2588}').count(),
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
