//! View layer: standby, dashboard panels, event ticker, and status line.
//! Full views land in Units 4–5 (tasks P4/P5).
//!
//! This module currently hosts ONLY the shell renderer ([`render`]): the
//! minimal per-frame surface the app loop draws through and the resize
//! contract (ui spec R1/S2) exercises via `TestBackend`. Unit 4 replaces the
//! live-view branch, Unit 5 the standby/ticker/status branches.

pub mod dashboard;
pub mod standby;
pub mod status;
pub mod ticker;

use crate::app::{App, Phase};
use ratatui::Frame;
use ratatui::widgets::Paragraph;

/// Shell-level placeholder rendering: one phase headline drawn into the full
/// frame area. Deliberately trivial so the resize guarantee (next frame fits
/// the new bounds, nothing panics) holds by construction until the real
/// views arrive; `Paragraph` clips to the given area, never past it.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let headline = match app.phase() {
        Phase::NotInGame => "Waiting for a live game...",
        Phase::InGame { .. } => "LIVE",
    };
    frame.render_widget(Paragraph::new(headline), area);
}
