//! View layer: standby, dashboard panels, event ticker, and status line.
//!
//! This module hosts the shell renderer ([`render`]) that dispatches per
//! lifecycle phase — live branch routes to [`dashboard`] (which appends the
//! [`ticker`] section), standby branch still a minimal placeholder until
//! Unit 5 owns [`standby`]/[`status`] — plus the shared clipping cursor
//! ([`Pen`]) every text view draws through.

pub mod dashboard;
pub mod standby;
pub mod status;
pub mod ticker;

use crate::app::{App, Phase};
use ratatui::Frame;
use ratatui::layout::Rect;
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

/// One-line vertical cursor shared by the view modules: it advances row by
/// row and clips at the frame bottom instead of panicking, so degenerate
/// viewports stay safe by construction.
pub(crate) struct Pen {
    x: u16,
    y: u16,
    width: u16,
    bottom: u16,
}

impl Pen {
    pub(crate) fn new(area: Rect) -> Self {
        Self { x: area.x, y: area.y, width: area.width, bottom: area.bottom() }
    }

    pub(crate) fn line(&mut self, text: String, frame: &mut Frame) {
        if self.y >= self.bottom || self.width == 0 {
            return;
        }
        let area = Rect { x: self.x, y: self.y, width: self.width, height: 1 };
        frame.render_widget(Paragraph::new(text), area);
        self.y += 1;
    }
}
