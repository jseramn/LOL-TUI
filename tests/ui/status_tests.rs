//! Status line contracts (task 5.4, ui spec R6): every view — idle or live
//! — persistently carries the mandatory non-endorsement notice, the
//! lifecycle state, and the last-update time; degraded FSM health surfaces
//! as a status-line annotation (design D4/W3), never a new subsystem.

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;
use tui_lol::ui::status::RIOT_NOTICE;

use crate::FixedClock;

const WIDTH: u16 = 240;
const HEIGHT: u16 = 32;

/// 12:34:56 UTC expressed in epoch millis — a stamp we can assert exactly.
const STAMP_MILLIS: u64 = (12 * 3600 + 34 * 60 + 56) * 1000;

fn snapshot_from_fixture(name: &str) -> Snapshot {
    let raw = std::fs::read_to_string(format!("tests/fixtures/allgamedata/{name}.json"))
        .expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

fn draw(app: &App<FixedClock>) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend");
    let frame = terminal.draw(|f| ui::render(f, app)).expect("frame");
    frame.buffer.clone()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|&y| row_text(buffer, y).contains(needle))
}

// --- Task 5.4 / ui spec R6/S1: notice persists in BOTH views ---

#[test]
fn riot_notice_constant_carries_the_mandated_wording() {
    assert!(
        RIOT_NOTICE.contains("not endorsed by Riot Games"),
        "RIOT_NOTICE must contain the exact mandated phrase: {RIOT_NOTICE}"
    );
}

#[test]
fn notice_and_status_are_visible_in_standby_and_live_views() {
    // Standby: fresh app, nothing received yet.
    let standby = App::with_clock(FixedClock { millis: STAMP_MILLIS });
    let buffer = draw(&standby);
    let notice_y = find_row(&buffer, RIOT_NOTICE).expect("notice visible while standby");
    let status = row_text(&buffer, notice_y);
    assert!(status.contains("state=standby"), "lifecycle state shown: {status}");
    assert!(status.contains("updated ?"), "no fabricated update time: {status}");

    // Live: same frame contract after entering a game with fresh data.
    let mut live = App::with_clock(FixedClock { millis: STAMP_MILLIS });
    live.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    live.on_msg(PollMsg::Snapshot(snapshot_from_fixture("full")));
    let buffer = draw(&live);
    let notice_y = find_row(&buffer, RIOT_NOTICE).expect("notice visible while live");
    let status = row_text(&buffer, notice_y);
    assert!(status.contains("state=in-game ok"), "lifecycle state shown: {status}");
    assert!(
        status.contains("updated 12:34:56 UTC"),
        "last-update stamp from the injected clock: {status}"
    );
}

// --- Task 5.4 / design D4 + W3: degradation renders on the status line ---

/// A transient failure keeps InGame but flips health to Degraded(reason);
/// the status line annotates that existing FSM health — stale-data signal,
/// not a new subsystem.
#[test]
fn degraded_health_annotates_the_status_line() {
    let mut app = App::with_clock(FixedClock { millis: STAMP_MILLIS });
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(snapshot_from_fixture("full")));
    app.on_msg(PollMsg::Transient(tui_lol::api::error::TransientReason::Timeout));

    let buffer = draw(&app);
    let notice_y = find_row(&buffer, RIOT_NOTICE).expect("notice still visible");
    let status = row_text(&buffer, notice_y);
    assert!(
        status.contains("state=in-game DEGRADED(timeout)"),
        "degraded reason surfaced on the status line: {status}"
    );
}
