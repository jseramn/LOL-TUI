//! Shell rendering contracts (task 3.4, ui spec R1/S2): after a shrink
//! resize, the next frame is drawn within the new bounds and no element
//! panics. Rendered through ratatui's [`TestBackend`] so assertions read the
//! exact cells that would reach the terminal.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::TestBackend};
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::ui;

fn live_app() -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app
}

/// Flattens one buffer row into a string for whole-line assertions.
fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect()
}

#[test]
fn shrink_resize_redraws_within_new_bounds_without_panicking() {
    let mut app = live_app();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test backend");
    terminal.draw(|frame| ui::render(frame, &app)).expect("initial frame");

    // Simulate the user shrinking the terminal window…
    terminal.backend_mut().resize(60, 20);

    // …the NEXT drawn frame must exist entirely within the new bounds.
    let frame = terminal.draw(|f| ui::render(f, &app)).expect("post-resize frame");
    assert_eq!(frame.area, Rect::new(0, 0, 60, 20));

    let buffer = frame.buffer;
    assert_eq!((buffer.area.width, buffer.area.height), (60, 20));
    // Real content landed at the origin — not an empty cleared buffer.
    assert!(row_text(buffer, 0).trim_end().len() > 0, "headline must be present");
}

#[test]
fn shrunk_standby_view_keeps_its_text_inside_the_visible_area() {
    let app = App::new(); // NotInGame → standby placeholder
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");

    terminal.backend_mut().resize(40, 10);
    let frame = terminal.draw(|f| ui::render(f, &app)).expect("standby frame");
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

    let frame = terminal.draw(|f| ui::render(f, &standby)).expect("tiny standby frame");
    assert_eq!(row_text(frame.buffer, 0).chars().next(), Some('W'));

    let mut live = live_app();
    let frame = terminal.draw(|f| ui::render(f, &live)).expect("tiny live frame");
    assert_eq!(row_text(frame.buffer, 0).chars().next(), Some('L'));

    // Silence the unused-mut lint while keeping one variable per phase.
    drop(live);
    drop(standby);
}
