//! Standby view shown while lifecycle is `NotInGame` (ui spec R2).
//!
//! One silent waiting line and nothing else: outside a game the app MUST
//! NOT spam error output — transient failures are swallowed by the FSM
//! before any view ever sees them. The message text is a pinned contract
//! (U3 shell tests assert it verbatim at the frame origin).

use ratatui::{Frame, widgets::Paragraph};

/// Pinned silent standby message.
pub(crate) const STANDBY_MESSAGE: &str = "Waiting for a live game...";

/// Draws the full-frame standby view.
pub(crate) fn render(frame: &mut Frame) {
    frame.render_widget(Paragraph::new(STANDBY_MESSAGE), frame.area());
}
