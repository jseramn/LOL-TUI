//! Dashboard rendering contracts (tasks 4.1–4.2, ui spec R3/S1): a complete
//! snapshot renders one panel per player — all 10 — grouped by team, each
//! showing champion, level, KDA, creep score, items, summoner spells as
//! exposed, and death state with the exposed respawn timer. The latest
//! received snapshot is reflected on the next rendered frame.
//!
//! Snapshots are built directly from the offline fixture corpus (design D5:
//! UI tests are network-free); rendering is asserted through ratatui
//! [`TestBackend`] buffers.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;
use tui_lol::ui::select_layout;

/// Wide enough that a fully itemized player line never clips inside one
/// side-by-side team column (~200 cells each).
const WIDTH: u16 = 400;
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

fn body_range(buffer: &Buffer) -> std::ops::Range<u16> {
    let layout = select_layout(Rect::new(0, 0, buffer.area.width, buffer.area.height));
    layout.areas.body.y..layout.areas.body.bottom()
}

/// Rows matching `needle` inside the team columns only. The briefing and
/// ticker also repeat champion names.
fn team_rows_containing(buffer: &Buffer, needle: &str) -> Vec<u16> {
    body_range(buffer)
        .filter(|&y| row_text(buffer, y).contains(needle))
        .collect()
}

fn player_row(buffer: &Buffer, name: &str) -> String {
    let y = team_rows_containing(buffer, name)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("row for {name} must exist"));
    row_text(buffer, y)
}

const ORDER_CHAMPS: [&str; 5] = ["Aatrox", "Lee Sin", "Ahri", "Jinx", "Lulu"];
const CHAOS_CHAMPS: [&str; 5] = ["Darius", "Graves", "Syndra", "Kai'Sa", "Thresh"];

#[test]
fn full_snapshot_renders_all_ten_panels_exactly_once() {
    let buffer = draw(&live_app_with("full"));
    // Panel rows end where the event ticker begins: the ticker (ui spec R5)
    // legitimately repeats participant NAMES, so the once-only roster scan
    // is scoped to the rows above the EVENTS header.
    for name in ORDER_CHAMPS.into_iter().chain(CHAOS_CHAMPS) {
        let rows = team_rows_containing(&buffer, name);
        assert_eq!(rows.len(), 1, "{name} must appear on exactly one panel row");
    }
}

#[test]
fn panels_are_grouped_by_team_side_by_side() {
    let buffer = draw(&live_app_with("full"));

    let order_header = find_row(&buffer, "Equipo Orden").expect("Orden team header");
    let chaos_header = find_row(&buffer, "Equipo Caos").expect("Caos team header");
    assert_eq!(
        order_header, chaos_header,
        "ORDER and CHAOS headers share the top body row (side-by-side columns)"
    );

    let mid = WIDTH / 2;
    let name_x = |name: &str| -> u16 {
        let y = team_rows_containing(&buffer, name)
            .into_iter()
            .next()
            .expect(name);
        let text = row_text(&buffer, y);
        text.find(name)
            .expect("name in row")
            .try_into()
            .expect("x fits u16")
    };

    for name in ORDER_CHAMPS {
        assert!(
            name_x(name) < mid,
            "{name} must sit in the left (ORDER) column"
        );
    }
    for name in CHAOS_CHAMPS {
        assert!(
            name_x(name) >= mid,
            "{name} must sit in the right (CHAOS) column"
        );
    }
}

#[test]
fn every_panel_shows_its_exposed_fields() {
    let buffer = draw(&live_app_with("full"));

    let aatrox = player_row(&buffer, "Aatrox");
    for token in ["Aatrox", "nivel 13", "5/2/4", "212 subditos"] {
        assert!(
            aatrox.contains(token),
            "Aatrox panel missing {token:?}: {aatrox}"
        );
    }

    let ahri = player_row(&buffer, "Ahri");
    for token in ["Ahri", "nivel 12", "6/1/7", "196 subditos"] {
        assert!(ahri.contains(token), "Ahri panel missing {token:?}: {ahri}");
    }

    // Champions render exactly once across team rows (roster only).
    let champions = [
        "Aatrox", "Lee Sin", "Ahri", "Jinx", "Lulu", "Darius", "Graves", "Syndra", "Kai'Sa",
        "Thresh",
    ];
    for champion in champions {
        let rows = team_rows_containing(&buffer, champion);
        assert_eq!(
            rows.len(),
            1,
            "{champion} must appear on exactly one team row"
        );
    }
}

#[test]
fn dead_players_show_their_exposed_respawn_timer() {
    let buffer = draw(&live_app_with("full"));
    let lee_sin = player_row(&buffer, "Lee Sin");
    assert!(
        lee_sin.contains("Muerto 12"),
        "exposed timer as whole seconds: {lee_sin}"
    );

    let graves = player_row(&buffer, "Graves");
    assert!(
        graves.contains("Muerto 34"),
        "exposed timer as whole seconds: {graves}"
    );

    // Alive players carry no death tag.
    let lulu = player_row(&buffer, "Lulu");
    assert!(
        !lulu.contains("Muerto"),
        "alive panel must not be marked dead: {lulu}"
    );
}

#[test]
fn latest_snapshot_is_reflected_on_the_next_frame() {
    let mut app = live_app_with("full");
    let first = draw(&app);
    assert!(player_row(&first, "Aatrox").contains("nivel 13"));

    // A fresh snapshot with changed data must win on the very next frame.
    let mut updated = snapshot_from_fixture("full");
    updated.players[0].level = Some(18);
    app.on_msg(PollMsg::Snapshot(Box::new(updated)));

    let second = draw(&app);
    let aatrox = player_row(&second, "Aatrox");
    assert!(
        aatrox.contains("nivel 18"),
        "latest snapshot must render: {aatrox}"
    );
}

// --- Task 4.3: per-field degradation (ui spec R3/S2, R3/S3) ---

/// The degraded player in `partial_player.json` omits items, respawnTimer,
/// and parts of scores (assists, creepScore). Placeholders must appear ONLY
/// on those absent fields; every other panel stays fully populated.
#[test]
fn partial_fields_degrade_explicitly_and_only_where_absent() {
    let buffer = draw(&live_app_with("partial_player"));

    let kaisa = player_row(&buffer, "Kai'Sa");
    for token in ["Kai'Sa", "nivel 11", "4/5/?", "? subditos"] {
        assert!(
            kaisa.contains(token),
            "degraded panel missing {token:?}: {kaisa}"
        );
    }
    // She is alive: no death tag may be invented.
    assert!(
        !kaisa.contains("Muerto"),
        "alive panel must not be marked dead: {kaisa}"
    );

    // Every other panel remains fully populated — no placeholder leaks.
    // Skip Jinx: she shares a row with Kai'Sa, whose `?` would false-fail.
    for name in ["Aatrox", "Lee Sin", "Ahri", "Thresh"] {
        let row = player_row(&buffer, name);
        assert!(
            !row.contains('?'),
            "intact panel {name} must not contain placeholders: {row}"
        );
    }
    let thresh = player_row(&buffer, "Thresh");
    assert!(
        thresh.contains("Soporte") && thresh.contains("Thresh"),
        "intact identity survives: {thresh}"
    );
}

/// isDead=true with an exposed timer prints that timer verbatim; with the
/// timer ABSENT it prints an unknown marker instead of any derived countdown.
#[test]
fn dead_player_without_exposed_timer_shows_unknown_marker_never_a_countdown() {
    // Exposed value first: full.json Lee Sin, Muerto 12 segundos (12.5 truncated).
    let buffer = draw(&live_app_with("full"));
    let exposed = player_row(&buffer, "Lee Sin");
    let tag = &exposed[exposed.find("Muerto").expect("death tag")..];
    assert!(
        tag.starts_with("Muerto 12"),
        "exposed timer as whole seconds: {tag}"
    );

    // Same player, respawnTimer absent from the payload this time…
    let mut absent_timer = snapshot_from_fixture("full");
    let jungle = absent_timer
        .players
        .iter_mut()
        .find(|p| p.summoner_name.as_deref() == Some("JungleKing"))
        .expect("JungleKing");
    assert_eq!(jungle.is_dead, Some(true));
    jungle.respawn_timer = None;

    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(absent_timer)));

    // …must yield an explicit unknown marker, never a computed number.
    let degraded = draw(&app);
    let row = player_row(&degraded, "Lee Sin");
    let tag = &row[row.find("Muerto").expect("death tag")..];
    assert!(
        tag.starts_with("Muerto ?"),
        "unknown marker required, got: {tag}"
    );
}

// --- Task 4.4: local-player enhanced panel (ui spec R4/S1, R4/S2) ---

/// The local strip is distinguished by its `Tu` label and carries the
/// exposed currentGold — here the spec scenario value 4350.
#[test]
fn local_gold_renders_on_the_local_strip() {
    let mut snapshot = snapshot_from_fixture("full");
    let local = snapshot.local.as_mut().expect("local player in fixture");
    local.current_gold = Some(4350.0);

    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot)));

    let buffer = draw(&app);

    let strip_y = find_row(&buffer, "Tu").expect("local strip");
    let strip = row_text(&buffer, strip_y);
    assert!(strip.contains("Ahri"), "local champion identified: {strip}");
    assert!(strip.contains("oro 4350"), "exposed gold verbatim: {strip}");

    // Exclusivity: that gold value appears NOWHERE else in the frame.
    for y in 0..buffer.area.height {
        if y != strip_y {
            assert!(
                !row_text(&buffer, y).contains("4350"),
                "gold value must stay on the local strip, found on row {y}"
            );
        }
    }
}

/// Absent local gold degrades to the explicit marker; no other panel ever
/// displays a gold value.
#[test]
fn missing_local_gold_degrades_and_no_enemy_panel_shows_gold() {
    let mut snapshot = snapshot_from_fixture("full");
    let local = snapshot.local.as_mut().expect("local player in fixture");
    local.current_gold = None;

    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot)));

    let buffer = draw(&app);

    let strip_y = find_row(&buffer, "Tu").expect("local strip");
    let strip = row_text(&buffer, strip_y);
    assert!(strip.contains("oro ?"), "explicit placeholder: {strip}");

    for y in 0..buffer.area.height {
        assert!(!row_text(&buffer, y).contains("oro 4350"));
        let text = row_text(&buffer, y);
        if !text.contains("Tu") && !text.trim_start().starts_with("oro") {
            assert!(
                !text.contains("oro"),
                "no gold outside the local strip, row {y}: {text}"
            );
        }
    }
}

#[test]
fn scoreboard_header_shows_mode_clock_and_team_kills() {
    let buffer = draw(&live_app_with("full"));
    let header = row_text(&buffer, 0);
    for token in [
        "En partida",
        "Clasica",
        "12:34",
        "Orden 23",
        "20 Caos",
        "1 dragon",
        "1 heraldo",
    ] {
        assert!(
            header.contains(token),
            "scoreboard header missing {token:?}: {header}"
        );
    }
    assert!(
        !header.contains("oro") && !header.contains("Gold"),
        "gold must never appear on the header: {header}"
    );
}

#[test]
fn header_band_shows_decision_briefing_under_the_scoreboard() {
    let buffer = draw(&live_app_with("full"));
    let band: String = (0..4).map(|y| row_text(&buffer, y)).collect();
    assert!(
        band.contains("En partida") && band.contains("12:34"),
        "scoreboard stays on the first header rows: {band}"
    );
    assert!(
        band.contains("subditos") || band.contains("carril") || band.contains("participas"),
        "briefing must cross live data into a decision: {band}"
    );
    assert!(
        band.contains("CALLE") || band.contains("AHORA") || band.contains("TU"),
        "briefing lines are tagged: {band}"
    );
    assert!(
        !band.contains("CS") && !band.contains("DRG") && !band.contains("KP"),
        "briefing stays in full words: {band}"
    );
}
