//! Objective/kill event ticker contracts (tasks 5.1–5.2, ui spec R5):
//! the ticker lists match events with their participants and the exposed
//! `EventTime` values as `mm:ss`, and degrades to an explicit empty-state
//! message when the snapshot carries zero events.
//!
//! Snapshots come straight from the offline fixture corpus (design D5);
//! rendering is asserted through ratatui [`TestBackend`] buffers.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;

const WIDTH: u16 = 240;
const HEIGHT: u16 = 32;

fn snapshot_from_fixture(name: &str) -> Snapshot {
    let raw = std::fs::read_to_string(format!("tests/fixtures/allgamedata/{name}.json"))
        .expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

fn live_app_with(fixture: &str) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_from_fixture(fixture))));
    app
}

fn draw(app: &App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend");
    let frame = terminal.draw(|f| ui::render(f, app)).expect("frame");
    frame.buffer.clone()
}

/// Flattens one buffer row into a string for whole-line assertions.
fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|&y| row_text(buffer, y).contains(needle))
}

fn require_row(buffer: &Buffer, needle: &str) -> String {
    let y =
        find_row(buffer, needle).unwrap_or_else(|| panic!("row containing {needle:?} must exist"));
    row_text(buffer, y)
}

// --- Task 5.1 / ui spec R5/S1: supported events with participants + times ---

/// The full fixture carries nine events; the spec scenario pins three of
/// them: FirstBlood, TurretKilled, and a stolen Chemtech DragonKill — each
/// listed with participants and the exposed EventTime as `mm:ss`.
#[test]
fn ticker_lists_events_with_participants_and_exposed_times() {
    let buffer = draw(&live_app_with("full"));

    let first_blood = require_row(&buffer, "primera sangre");
    assert!(
        first_blood.contains("Order"),
        "recipient as exposed: {first_blood}"
    );
    assert!(
        first_blood.contains("03:02"),
        "EventTime 182.44 as clock: {first_blood}"
    );

    let turret = require_row(&buffer, "ADCarryMain");
    assert!(
        turret.contains("torre"),
        "participants as exposed: {turret}"
    );
    assert!(
        turret.contains("11:43"),
        "EventTime 703.42 as clock: {turret}"
    );

    let dragon = require_row(&buffer, "Chemtech");
    for token in ["JungleKing", "robado", "08:32"] {
        assert!(dragon.contains(token), "stolen dragon wording: {dragon}");
    }
}

/// Newest events lead so a short ticker still shows what just happened.
#[test]
fn ticker_lists_newest_events_first() {
    let buffer = draw(&live_app_with("full"));
    let ace_y = find_row(&buffer, "aniquilacion").expect("ace is the latest fixture event");
    let start_y = find_row(&buffer, "inicio").expect("game start still listed");
    assert!(
        ace_y < start_y,
        "newest event must sit above older ones ({ace_y} vs {start_y})"
    );
}

/// The ticker section renders below the player panels (after the LOCAL
/// strip), keeping one stable reading order: headline → teams → local → events.
#[test]
fn ticker_section_sits_below_the_local_strip() {
    let buffer = draw(&live_app_with("full"));

    let local_y = find_row(&buffer, "Tu").expect("local strip");
    let events_y = find_row(&buffer, "Sucesos").expect("Sucesos section header");
    assert!(events_y > local_y, "ticker belongs under the panels");

    let first_event_y = find_row(&buffer, "inicio").expect("game start listed");
    assert!(
        first_event_y > events_y,
        "event lines follow the section header"
    );
}

// --- Task 5.1 / ui spec R5/S2: zero events degrade to an empty state ---

/// A snapshot whose event list is empty renders an explicit empty-state
/// message instead of failing (the `empty_events` fixture omits the key).
#[test]
fn empty_event_list_shows_placeholder_instead_of_failing() {
    let buffer = draw(&live_app_with("empty_events"));

    find_row(&buffer, "Sucesos").expect("section header still present");
    let placeholder = require_row(&buffer, "sin eventos");
    assert!(
        !placeholder.contains(':') || placeholder.contains("sin eventos"),
        "empty state must not invent an event time: {placeholder}"
    );
    assert!(
        !placeholder.contains("inicio"),
        "empty state must not invent events: {placeholder}"
    );
}
