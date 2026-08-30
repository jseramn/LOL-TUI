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
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;

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

fn rows_containing(buffer: &Buffer, needle: &str) -> Vec<u16> {
    (0..buffer.area.height)
        .filter(|&y| row_text(buffer, y).contains(needle))
        .collect()
}

/// Rows matching `needle` excluding the local-player strip (whose label is
/// `LOCAL`) and the event-ticker section: the strip repeats the local
/// champion, and the ticker legitimately repeats participant names that can
/// embed champion substrings (e.g. `Syndra` inside victim `SyndraGod`), so
/// roster scans stay scoped to the panel rows above the EVENTS header.
fn team_rows_containing(buffer: &Buffer, needle: &str) -> Vec<u16> {
    let events_y = find_row(buffer, "EVENTS").unwrap_or(buffer.area.height);
    rows_containing(buffer, needle)
        .into_iter()
        .take_while(|&y| y < events_y)
        .filter(|&y| !row_text(buffer, y).contains("LOCAL"))
        .collect()
}

fn player_row(buffer: &Buffer, name: &str) -> String {
    let y = find_row(buffer, name).unwrap_or_else(|| panic!("row for {name} must exist"));
    row_text(buffer, y)
}

const ORDER_NAMES: [&str; 5] = [
    "TopLaneTitan",
    "JungleKing",
    "MidMage",
    "ADCarryMain",
    "SupportSage",
];
const CHAOS_NAMES: [&str; 5] = [
    "TopGap",
    "GravesMain",
    "SyndraGod",
    "KaiSaFan",
    "HookMaster",
];

#[test]
fn full_snapshot_renders_all_ten_panels_exactly_once() {
    let buffer = draw(&live_app_with("full"));
    // Panel rows end where the event ticker begins: the ticker (ui spec R5)
    // legitimately repeats participant NAMES, so the once-only roster scan
    // is scoped to the rows above the EVENTS header.
    let events_y = find_row(&buffer, "EVENTS").expect("EVENTS header");
    for name in ORDER_NAMES.into_iter().chain(CHAOS_NAMES) {
        let rows: Vec<u16> = (0..events_y)
            .filter(|&y| row_text(&buffer, y).contains(name))
            .collect();
        assert_eq!(rows.len(), 1, "{name} must appear on exactly one panel row");
    }
}

#[test]
fn panels_are_grouped_by_team_side_by_side() {
    let buffer = draw(&live_app_with("full"));

    let order_header = find_row(&buffer, "Team ORDER").expect("ORDER team header");
    let chaos_header = find_row(&buffer, "Team CHAOS").expect("CHAOS team header");
    assert_eq!(
        order_header, chaos_header,
        "ORDER and CHAOS headers share the top body row (side-by-side columns)"
    );

    let mid = WIDTH / 2;
    let name_x = |name: &str| -> u16 {
        let y = find_row(&buffer, name).expect(name);
        let text = row_text(&buffer, y);
        text.find(name)
            .expect("name in row")
            .try_into()
            .expect("x fits u16")
    };

    for name in ORDER_NAMES {
        assert!(
            name_x(name) < mid,
            "{name} must sit in the left (ORDER) column"
        );
    }
    for name in CHAOS_NAMES {
        assert!(
            name_x(name) >= mid,
            "{name} must sit in the right (CHAOS) column"
        );
    }
}

#[test]
fn every_panel_shows_its_exposed_fields() {
    let buffer = draw(&live_app_with("full"));

    let aatrox = player_row(&buffer, "TopLaneTitan");
    for token in [
        "TopLaneTitan",
        "Aatrox",
        "Lv13",
        "5/2/4",
        "CS212",
        "SummonerFlash+SummonerTeleport",
        "Berserker's Greaves",
        "Kraken Slayer",
        "Blade of The Ruined King",
        "Guardian Angel",
        "Death's Dance",
        "Long Sword",
        "Warding Totem Trinket",
    ] {
        assert!(
            aatrox.contains(token),
            "Aatrox panel missing {token:?}: {aatrox}"
        );
    }

    let ahri = player_row(&buffer, "MidMage");
    for token in [
        "MidMage",
        "Ahri",
        "Lv12",
        "6/1/7",
        "CS195.5",
        "SummonerFlash+SummonerDot",
    ] {
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
    let lee_sin = player_row(&buffer, "JungleKing");
    assert!(
        lee_sin.contains("DEAD(respawn 12.5)"),
        "exposed timer verbatim: {lee_sin}"
    );

    let graves = player_row(&buffer, "GravesMain");
    assert!(
        graves.contains("DEAD(respawn 34)"),
        "exposed timer verbatim: {graves}"
    );

    // Alive players carry no death tag.
    let lulu = player_row(&buffer, "SupportSage");
    assert!(
        !lulu.contains("DEAD"),
        "alive panel must not be marked dead: {lulu}"
    );
}

#[test]
fn latest_snapshot_is_reflected_on_the_next_frame() {
    let mut app = live_app_with("full");
    let first = draw(&app);
    assert!(player_row(&first, "TopLaneTitan").contains("Lv13"));

    // A fresh snapshot with changed data must win on the very next frame.
    let mut updated = snapshot_from_fixture("full");
    updated.players[0].level = Some(18);
    app.on_msg(PollMsg::Snapshot(Box::new(updated)));

    let second = draw(&app);
    let aatrox = player_row(&second, "TopLaneTitan");
    assert!(
        aatrox.contains("Lv18"),
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

    let kaisa = player_row(&buffer, "KaiSaFan");
    for token in [
        "KaiSaFan",
        "Kai'Sa",
        "Lv11",
        "4/5/?",
        "CS?",
        "Items: ?",
        "SummonerHeal+SummonerFlash",
    ] {
        assert!(
            kaisa.contains(token),
            "degraded panel missing {token:?}: {kaisa}"
        );
    }
    // She is alive: no death tag may be invented.
    assert!(
        !kaisa.contains("DEAD"),
        "alive panel must not be marked dead: {kaisa}"
    );

    // Every other panel remains fully populated — no placeholder leaks.
    for name in ["TopLaneTitan", "JungleKing", "MidMage", "HookMaster"] {
        let row = player_row(&buffer, name);
        assert!(
            !row.contains('?'),
            "intact panel {name} must not contain placeholders: {row}"
        );
    }
    let thresh = player_row(&buffer, "HookMaster");
    assert!(
        thresh.contains("Locket of the Iron Solari"),
        "intact items survive: {thresh}"
    );
}

/// isDead=true with an exposed timer prints that timer verbatim; with the
/// timer ABSENT it prints an unknown marker instead of any derived countdown.
#[test]
fn dead_player_without_exposed_timer_shows_unknown_marker_never_a_countdown() {
    // Exposed value first: full.json Lee Sin, DEAD(respawn 12.5).
    let buffer = draw(&live_app_with("full"));
    let exposed = player_row(&buffer, "JungleKing");
    let tag = &exposed[exposed.find("DEAD").expect("death tag")..];
    assert!(
        tag.starts_with("DEAD(respawn 12.5)"),
        "verbatim exposed timer: {tag}"
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
    let row = player_row(&degraded, "JungleKing");
    let tag = &row[row.find("DEAD").expect("death tag")..];
    assert!(
        tag.starts_with("DEAD(respawn ?)"),
        "unknown marker required, got: {tag}"
    );
}

// --- Task 4.4: local-player enhanced panel (ui spec R4/S1, R4/S2) ---

/// The local strip is distinguished by its `LOCAL` label and carries the
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

    let strip_y = find_row(&buffer, "LOCAL").expect("LOCAL strip");
    let strip = row_text(&buffer, strip_y);
    assert!(strip.contains("Ahri"), "local champion identified: {strip}");
    assert!(
        strip.contains("Gold 4350"),
        "exposed gold verbatim: {strip}"
    );

    // Exclusivity: that gold value appears NOWHERE else in the frame.
    for y in 0..buffer.area.height {
        if y != strip_y {
            assert!(
                !row_text(&buffer, y).contains("Gold"),
                "gold token must stay on the LOCAL strip, found on row {y}"
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

    let strip = player_row(&buffer, "LOCAL");
    assert!(strip.contains("Gold ?"), "explicit placeholder: {strip}");

    for y in 0..buffer.area.height {
        assert!(!row_text(&buffer, y).contains("Gold 4350"));
        let text = row_text(&buffer, y);
        if !text.contains("LOCAL") {
            assert!(
                !text.contains("Gold"),
                "no gold outside the LOCAL strip, row {y}: {text}"
            );
        }
    }
}

#[test]
fn scoreboard_header_shows_mode_clock_and_team_kills() {
    let buffer = draw(&live_app_with("full"));
    let header = row_text(&buffer, 0);
    for token in [
        "LIVE", "CLASSIC", "754.19s", "Map11", "ORDER 23", "20 CHAOS", "DRG 1", "HERALD 1",
    ] {
        assert!(
            header.contains(token),
            "scoreboard header missing {token:?}: {header}"
        );
    }
    assert!(
        !header.contains("Gold"),
        "gold must never appear on the header: {header}"
    );
}
