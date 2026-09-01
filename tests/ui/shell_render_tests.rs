//! Shell rendering contracts (task 3.4, ui spec R1/S2): after a shrink
//! resize, the next frame is drawn within the new bounds and no element
//! panics. Rendered through ratatui's [`TestBackend`] so assertions read the
//! exact cells that would reach the terminal.
//!
//! Task 3.1 (viz spec R2/S1, R2/S2) extends this file with the region-layout
//! contracts: the live view splits vertically into header / body / local /
//! ticker / status, the Riot notice filling the FINAL row at every size —
//! including the 80×12 minimum acceptable viewport.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::TestBackend};
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;
use tui_lol::ui::status::RIOT_NOTICE;

fn live_app() -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app
}

fn snapshot_from_fixture(name: &str) -> Snapshot {
    let raw = std::fs::read_to_string(format!("tests/fixtures/allgamedata/{name}.json"))
        .expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

fn live_app_with_snapshot(fixture: &str) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_from_fixture(fixture))));
    app
}

/// Draws one frame at `width × height` and returns the buffer.
fn draw_at(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    let frame = terminal.draw(|f| ui::render(f, app)).expect("frame");
    frame.buffer.clone()
}

/// Flattens one buffer row into a string for whole-line assertions.
fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn shrink_resize_redraws_within_new_bounds_without_panicking() {
    let app = live_app();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test backend");
    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("initial frame");
    // Simulate the user shrinking the terminal window…
    terminal.backend_mut().resize(60, 20);

    // …the NEXT drawn frame must exist entirely within the new bounds.
    let frame = terminal
        .draw(|f| ui::render(f, &app))
        .expect("post-resize frame");
    assert_eq!(frame.area, Rect::new(0, 0, 60, 20));

    let buffer = frame.buffer;
    assert_eq!((buffer.area.width, buffer.area.height), (60, 20));
    // Real content landed at the origin — not an empty cleared buffer.
    assert!(
        !row_text(buffer, 0).trim_end().is_empty(),
        "headline must be present"
    );
}

#[test]
fn shrunk_standby_view_keeps_its_text_inside_the_visible_area() {
    let app = App::new(); // NotInGame → standby placeholder
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");

    terminal.backend_mut().resize(40, 10);
    let frame = terminal
        .draw(|f| ui::render(f, &app))
        .expect("standby frame");
    let buffer = frame.buffer;

    let headline = row_text(buffer, 0).trim_end().to_owned();
    assert_eq!(headline, "Waiting for a live game...");
    assert!(
        headline.chars().count() <= usize::from(buffer.area.width),
        "text must not exceed the shrunken width"
    );
}

#[test]
fn extreme_small_viewport_still_renders_both_phases_without_panics() {
    // Degenerate 8x2 terminal: nothing may panic and something must draw.
    let standby = App::new();
    let mut terminal = Terminal::new(TestBackend::new(8, 2)).expect("test backend");

    let frame = terminal
        .draw(|f| ui::render(f, &standby))
        .expect("tiny standby frame");
    assert_eq!(row_text(frame.buffer, 0).chars().next(), Some('W'));

    let live = live_app();
    let frame = terminal
        .draw(|f| ui::render(f, &live))
        .expect("tiny live frame");
    assert_eq!(row_text(frame.buffer, 0).chars().next(), Some('L'));
}

// --- Task 3.1 / viz spec R2: region-based layout preserving R6 ---

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|&y| row_text(buffer, y).contains(needle))
}

/// viz spec R2/S2: at the minimum acceptable viewport the split still
/// exists — team panels, LOCAL strip, and ticker each own their band in
/// top-to-bottom order above an intact status row. The roster may clip;
/// the REGIONS may not.
#[test]
fn minimum_viewport_keeps_regions_ordered_above_an_intact_status_row_80x12() {
    let app = live_app_with_snapshot("full");
    let buffer = draw_at(&app, 80, 12);

    let live_y = find_row(&buffer, "LIVE").expect("header region must render");
    assert_eq!(live_y, 0, "header owns the first row");

    let order_y = find_row(&buffer, "Team ORDER").expect("ORDER column header visible");
    let local_y = find_row(&buffer, "LOCAL").expect("LOCAL strip visible at 80x12");
    let events_y = find_row(&buffer, "EVENTS").expect("ticker section visible at 80x12");

    assert!(order_y < local_y, "LOCAL strip sits below the team columns");
    assert!(local_y < events_y, "ticker sits below the local strip");
    assert!(events_y < 11, "ticker stays above the status row");

    let status = row_text(&buffer, 11);
    assert!(
        status.starts_with(RIOT_NOTICE),
        "notice fills the final row unoverlapped: {status:?}"
    );
}

/// viz spec R2/S1: canonical region order on the full-size viewport with a
/// complete snapshot, the Riot notice filling row 23 of 24.
#[test]
fn canonical_region_order_at_80x24_with_notice_filling_final_row() {
    let app = live_app_with_snapshot("full");
    let buffer = draw_at(&app, 80, 24);

    let live_y = find_row(&buffer, "LIVE").expect("header");
    let order_y = find_row(&buffer, "Team ORDER").expect("ORDER header");
    let chaos_y = find_row(&buffer, "Team CHAOS").expect("CHAOS header");
    let local_y = find_row(&buffer, "LOCAL").expect("LOCAL strip");
    let events_y = find_row(&buffer, "EVENTS").expect("EVENTS header");

    assert_eq!(live_y, 0, "header first");
    assert_eq!(
        order_y, chaos_y,
        "ORDER and CHAOS headers share one body row (side-by-side columns)"
    );
    assert!(order_y < local_y, "columns before the local strip");
    assert!(local_y < events_y, "local strip before the ticker");
    assert!(events_y < 23, "ticker before the status row");

    let status = row_text(&buffer, 23);
    assert!(
        status.starts_with(RIOT_NOTICE),
        "Riot notice fills the final row: {status:?}"
    );
}

/// live-dashboard-ui R3: canonical 80×24 must show all five players per team
/// (identity + viz rows) — supports appear as SUP + champion in compact columns.
#[test]
fn canonical_80x24_renders_all_five_players_per_team() {
    let app = live_app_with_snapshot("full");
    let buffer = draw_at(&app, 80, 24);
    let events_y = find_row(&buffer, "EVENTS").unwrap_or(buffer.area.height);
    let layout = tui_lol::ui::select_layout(Rect::new(0, 0, 80, 24));
    let mid = buffer.area.width / 2;

    assert_eq!(
        layout.areas.body.height, 11,
        "canonical viewport pins body to eleven rows for five 2-row cards"
    );

    for (label, champ) in [("SUP", "Lulu"), ("SUP", "Thresh")] {
        let rows = (0..events_y)
            .filter(|&y| {
                let left = row_text(&buffer, y);
                let right = (mid..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>();
                (left.contains(label) && left.contains(champ))
                    || (right.contains(label) && right.contains(champ))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rows.len(),
            1,
            "support {label} {champ} must appear once in team columns at 80x24: {rows:?}"
        );
        assert!(rows[0] < events_y, "support row must sit above EVENTS");
    }

    let viz_rows = (layout.areas.body.y..layout.areas.body.bottom())
        .filter(|&y| row_text(&buffer, y).starts_with("Lv"))
        .count();
    assert_eq!(
        viz_rows, 5,
        "five paired visualization rows (one per roster slot) at 80x24"
    );
}

/// Canonical 80×24 frame dumps must keep roster identity readable inside
/// each ~40-cell team column (no mid-field clip from the full formatter).
#[test]
fn canonical_80x24_identity_lines_fit_team_columns() {
    let app = live_app_with_snapshot("full");
    let buffer = draw_at(&app, 80, 24);
    let mid = buffer.area.width / 2;

    let segment = |y: u16, start: u16, end: u16| -> String {
        (start..end)
            .map(|x| buffer[(x, y)].symbol())
            .collect::<String>()
            .trim_end()
            .to_owned()
    };

    for y in 2..12 {
        let left = segment(y, 0, mid);
        let right = segment(y, mid, buffer.area.width);
        if left.starts_with("Lv") || right.starts_with("Lv") {
            continue;
        }
        if left.contains("Team ") || right.contains("Team ") {
            continue;
        }
        if left.is_empty() && right.is_empty() {
            continue;
        }
        assert!(
            left.chars().count() <= usize::from(mid),
            "ORDER identity must fit its column on row {y}: {left:?}"
        );
        assert!(
            right.chars().count() <= usize::from(buffer.area.width - mid),
            "CHAOS identity must fit its column on row {y}: {right:?}"
        );
        assert!(
            left.contains("Lv") && left.contains("CS"),
            "compact ORDER identity on row {y}: {left:?}"
        );
        assert!(
            right.contains("Lv") && right.contains("CS"),
            "compact CHAOS identity on row {y}: {right:?}"
        );
    }
}

/// viz spec R2 + R9/S2 discipline: whatever the height — from a 1-row
/// sliver to the full layout — the notice-bearing status row is the LAST
/// row of the frame. Nothing else may occupy it.
#[test]
fn status_row_stays_final_across_degenerate_heights() {
    let app = live_app_with_snapshot("full");
    for height in [1u16, 2, 5, 9, 10] {
        let buffer = draw_at(&app, 80, height);
        let status = row_text(&buffer, height - 1);
        assert!(
            status.starts_with(RIOT_NOTICE),
            "height {height}: notice must own the final row, got {status:?}"
        );
    }
}
