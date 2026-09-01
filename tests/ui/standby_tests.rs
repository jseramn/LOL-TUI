//! Standby view contracts (task 5.3, ui spec R2/S1–S3). The FSM
//! transitions behind these scenarios were proven RED-first in U3's
//! `app_tests`; these are their buffer-level regression locks (green on
//! arrival by design): silent waiting screen outside a game, mid-game
//! disconnect falls back without crashing, reconnect restores the live
//! view on the next drained frame.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;

use crate::FixedClock;

const WIDTH: u16 = 240;
const HEIGHT: u16 = 32;

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
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|&y| row_text(buffer, y).contains(needle))
}

/// R2/S1: startup outside a game renders the silent standby view — no
/// error output anywhere in the frame.
#[test]
fn startup_outside_a_game_shows_silent_standby() {
    let app = App::with_clock(FixedClock { millis: 0 });
    let buffer = draw(&app);

    let y = find_row(&buffer, "Aun no hay una partida en curso").expect("standby message");
    assert_eq!(y, 0, "standby message leads the view");

    for scan in 0..buffer.area.height {
        let text = row_text(&buffer, scan);
        assert!(
            !text.contains("error"),
            "no error output while idle: {text}"
        );
        assert!(
            !text.contains("DEGRADED"),
            "no degradation noise while idle: {text}"
        );
    }
}

/// R2/S2: an active game dropping to NOT_IN_GAME swaps to standby without
/// crashing or leaving live-view residue behind.
#[test]
fn mid_game_disconnect_falls_back_to_standby() {
    let mut app = App::with_clock(FixedClock { millis: 0 });
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_from_fixture("full"))));
    let live = draw(&app);
    assert!(
        find_row(&live, "Equipo Orden").is_some(),
        "live view was up"
    );

    app.on_msg(PollMsg::Lifecycle(Lifecycle::NotInGame));
    let standby = draw(&app);

    assert!(
        find_row(&standby, "Aun no hay una partida en curso").is_some(),
        "standby view restored"
    );
    assert!(
        find_row(&standby, "Equipo Orden").is_none(),
        "no live panels linger"
    );
    assert!(find_row(&standby, "Sucesos").is_none(), "no ticker lingers");
}

/// R2/S3: reconnecting (NOT_IN_GAME → IN_GAME with a fresh snapshot)
/// restores the live dashboard on the very next drained frame.
#[test]
fn reconnect_returns_to_the_live_view_on_the_next_frame() {
    let mut app = App::with_clock(FixedClock { millis: 0 });
    app.on_msg(PollMsg::Lifecycle(Lifecycle::NotInGame));

    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_from_fixture("full"))));

    let live = draw(&app);
    assert!(
        find_row(&live, "En partida").is_some(),
        "live headline back"
    );
    assert!(find_row(&live, "Equipo Orden").is_some(), "panels restored");
}
