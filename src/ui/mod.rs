//! View layer: standby, dashboard panels, event ticker, and status line.
//!
//! This module hosts the shell renderer ([`render`]) that dispatches per
//! lifecycle phase: the live branch routes to [`dashboard`] (Unit 4); the
//! standby branch is still a minimal placeholder until Unit 5 owns
//! [`standby`]/[`status`]/[`ticker`].

pub mod dashboard;
pub mod standby;
pub mod status;
pub mod ticker;

use crate::app::{App, Phase};
use ratatui::Frame;
use ratatui::widgets::Paragraph;

/// Shell-level phase dispatch. Both branches clip to the frame area (never
/// panic), preserving the resize guarantee exercised via `TestBackend`.
pub fn render(frame: &mut Frame, app: &App) {
    match app.phase() {
        Phase::NotInGame => standby_placeholder(frame),
        Phase::InGame { .. } => dashboard::render(frame, app),
    }
}

/// Standby placeholder: one silent line, no error output (ui spec R2).
/// Unit 5 replaces this with the real `standby` view.
fn standby_placeholder(frame: &mut Frame) {
    frame.render_widget(Paragraph::new("Waiting for a live game..."), frame.area());
}
