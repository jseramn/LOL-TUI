//! Offline frame dump: renders the live dashboard from `full.json` through
//! ratatui's `TestBackend` and writes UTF-8 text snapshots under
//! `docs/frames/`. Run from the repo root:
//!
//! ```text
//! cargo run --example dump_frame
//! ```

use std::fs;
use std::path::Path;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;
use tui_lol::ui;

const FIXTURE: &str = "tests/fixtures/allgamedata/full.json";
const OUT_DIR: &str = "docs/frames";

/// Canonical and wide review sizes (viz spec R2/R9).
const SIZES: &[(u16, u16, &str)] = &[(80, 24, "live-80x24.txt"), (120, 32, "live-120x32.txt")];

fn snapshot_from_fixture(path: &str) -> Snapshot {
    let raw = fs::read_to_string(path).expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

fn live_app_with(snapshot: Snapshot) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot)));
    app
}

fn buffer_to_text(buffer: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        if y + 1 < buffer.area.height {
            out.push('\n');
        }
    }
    out
}

fn draw_at(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    let frame = terminal.draw(|f| ui::render(f, app)).expect("frame");
    frame.buffer.clone()
}

fn main() {
    let snapshot = snapshot_from_fixture(FIXTURE);
    let app = live_app_with(snapshot);

    fs::create_dir_all(OUT_DIR).expect("create docs/frames");

    for &(width, height, filename) in SIZES {
        let buffer = draw_at(&app, width, height);
        let text = buffer_to_text(&buffer);
        let path = Path::new(OUT_DIR).join(filename);
        fs::write(&path, text).expect("write frame dump");
        println!("wrote {} ({}x{})", path.display(), width, height);
    }
}
