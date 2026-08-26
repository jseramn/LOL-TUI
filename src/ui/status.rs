//! Persistent status line (ui spec R6): Riot notice + lifecycle state +
//! last-update time on the frame's bottom row of EVERY view.
//!
//! Design D4/W3: degradation is an annotation of the existing FSM health
//! inside [`Phase::InGame`] — `DEGRADED(reason)` text on this line — never
//! a separate subsystem or a standby transition. The last-update readout
//! formats the STAMPED receipt time (`App::last_update_millis`) as UTC
//! wall-clock text; nothing is derived from `gameTime` and no countdown
//! exists here (design hard rule).

use super::dashboard::UNKNOWN;
use crate::api::poller::Clock;
use crate::app::{App, Health, Phase};
use ratatui::layout::Rect;
use ratatui::{Frame, widgets::Paragraph};

/// The mandatory non-endorsement notice, rendered verbatim in both views.
pub const RIOT_NOTICE: &str = "This product is not endorsed by Riot Games.";

/// Draws the bottom-row status line, computing the row itself. This is the
/// standby path's entry point (the idle view keeps its full-frame
/// paragraph and the status line claims the last row afterwards); the live
/// view calls [`render_into`] with the shell's reserved region instead.
pub(crate) fn render<C: Clock>(frame: &mut Frame, app: &App<C>) {
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    let row = Rect {
        x: area.x,
        y: area.bottom() - 1,
        width: area.width,
        height: 1,
    };
    render_into(frame, app, row);
}

/// Draws the status line into an explicitly assigned single-row rect — the
/// shell's reserved final band ([`super::Regions.status`]) during IN_GAME.
/// Drawn LAST by the shell dispatcher; clips safely at any viewport size.
pub(crate) fn render_into<C: Clock>(frame: &mut Frame, app: &App<C>, row: Rect) {
    if row.is_empty() {
        return;
    }
    frame.render_widget(Paragraph::new(status_text(app)), row);
}

/// Composes the single-line status readout:
/// `{RIOT_NOTICE} | state={standby|in-game ok|in-game DEGRADED(reason)} | updated {HH:MM:SS UTC|?}`
fn status_text<C: Clock>(app: &App<C>) -> String {
    let mut line = String::from(RIOT_NOTICE);
    line.push_str(" | state=");
    match app.phase() {
        Phase::NotInGame => line.push_str("standby"),
        Phase::InGame { health } => match health {
            Health::Healthy => line.push_str("in-game ok"),
            Health::Degraded(reason) => {
                line.push_str("in-game DEGRADED(");
                line.push_str(&reason.to_string());
                line.push(')');
            }
        },
    }
    line.push_str(" | updated ");
    match app.last_update_millis() {
        Some(millis) => line.push_str(&format_utc_hms(millis)),
        None => line.push_str(UNKNOWN),
    }
    line.push_str(" UTC");
    line
}

/// Pure arithmetic formatting of an epoch-millis stamp into `HH:MM:SS`
/// UTC. Deterministic and offline-testable via the injected clock seam.
fn format_utc_hms(millis: u64) -> String {
    let total_seconds = millis / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        (total_seconds / 3600) % 24,
        (total_seconds / 60) % 60,
        total_seconds % 60
    )
}
