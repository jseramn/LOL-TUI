//! Team-grouped live dashboard panels (ui spec R3).
//!
//! The dashboard renders the [`App`]'s latest retained [`Snapshot`] every
//! frame into the shell's disjoint regions: a scoreboard headline in the
//! header band, ORDER and CHAOS **side-by-side** player cards in the body
//! (identity line + visualization row), and the local strip in its own
//! band. Gold renders only on the local-player strip (task 4.4).
//!
//! Compliance (design): the respawn value printed here is the exposed
//! snapshot value verbatim — nothing is derived or counted down locally.

use super::format::local_line;
use super::scoreboard::header_band;
use super::{LiveLayout, draw_lines};
use crate::api::poller::Clock;
use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Re-export so existing `dashboard::UNKNOWN` imports keep compiling.
pub use super::format::UNKNOWN;

/// Renders the in-game view into the shell's regions: scoreboard headline
/// plus team-column cards for the latest snapshot, then the event ticker in
/// its own band. Widget families render only when the degradation matrix
/// (`layout.visible`, design D9) keeps them visible at this viewport.
/// With no snapshot yet (lifecycle arrived first), only the headline
/// draws — never a panic, whatever the frame size.
pub(crate) fn render<C: Clock>(frame: &mut Frame, app: &App<C>, layout: &LiveLayout) {
    let Some(snapshot) = app.snapshot() else {
        draw_lines(frame, layout.areas.header, &["En partida".to_owned()]);
        return;
    };
    draw_snapshot(snapshot, app.gold_window(), layout, frame);
}

fn draw_snapshot<G>(
    snapshot: &crate::model::snapshot::Snapshot,
    gold_window: G,
    layout: &LiveLayout,
    frame: &mut Frame,
) where
    G: Iterator<Item = Option<u64>>,
{
    let regions = &layout.areas;
    let width = regions.header.width as usize;
    let mut headline = header_band(snapshot);
    if width > 0 {
        for line in &mut headline {
            if line.chars().count() > width {
                *line = line.chars().take(width).collect();
            }
        }
    }
    draw_lines(frame, regions.header, &headline);

    super::team::render(frame, snapshot, layout.columns, layout.visible);

    // Local-player strip (ui spec R4): the ONLY surface that ever renders
    // gold. `activePlayer` absent from the payload → no strip at all. The
    // band's first row keeps the legacy text line; the rows below it host
    // the gauge/sparkline widgets. Gauges are untiered; only the sparkline
    // obeys the matrix.
    if let Some(local) = &snapshot.local {
        let legacy_row = Rect {
            height: regions.local.height.min(1),
            ..regions.local
        };
        draw_lines(frame, legacy_row, &[local_line(local)]);
        if regions.local.height > 1 {
            let widget_rows = Rect {
                y: regions.local.y + 1,
                height: regions.local.height - 1,
                ..regions.local
            };
            super::local_strip::render(
                frame,
                local,
                gold_window,
                widget_rows,
                layout.visible.sparkline,
            );
        }
    }

    super::ticker::render(&snapshot.events, regions.ticker, frame);
}
