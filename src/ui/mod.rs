//! View layer: standby, dashboard panels, event ticker, and status line.
//!
//! This module hosts the shell renderer ([`render`]) that dispatches per
//! lifecycle phase. The idle branch keeps the full-frame [`standby`]
//! paragraph. The live branch splits the frame into disjoint region rects
//! via [`split_regions`] (design D2), routes each region to its owner view
//! ([`dashboard`] renders header/body/local and drives the [`ticker`]), and
//! draws the status line LAST so it owns the bottom row in EVERY view —
//! making the ui spec R6 notice guarantee structural rather than
//! convention: no widget ever receives a rect containing the status row.

pub mod dashboard;
pub mod local_strip;
pub mod standby;
pub mod status;
pub mod team;
pub mod ticker;

use crate::api::poller::Clock;
use crate::app::{App, Phase};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

/// Vertical constraint tuple of the live view's four upper regions (design
/// D2): one headline row, at least five body rows for the team columns, a
/// four-row local strip (legacy text line + two blocked-segment gauges +
/// the gold-trend row the sparkline fills in task 4.7), and a compressible
/// ticker band. The fifth D2 constraint — `Length(1)` status, LAST — is
/// enforced by reserving that row before the solver runs; see
/// [`split_regions`].
const REGION_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Length(1),
    Constraint::Min(5),
    Constraint::Length(4),
    Constraint::Min(1),
];

/// Smallest height the D2 tuple can satisfy exactly: 1 + 5 + 4 + 1.
const MIN_REGIONS_HEIGHT: u16 = 11;

/// The live view's disjoint vertical bands, top-to-bottom. `status` is
/// always the frame's final row; the other four never intersect it or each
/// other, which is what makes the R6 guarantee structural (viz spec R2).
pub(crate) struct Regions {
    pub(crate) header: Rect,
    pub(crate) body: Rect,
    pub(crate) local: Rect,
    pub(crate) ticker: Rect,
    pub(crate) status: Rect,
}

/// Splits `area` into the live view's regions (design D2).
///
/// The status row is RESERVED before the solver runs instead of being
/// expressed as a fifth `Layout` constraint: below the tuple's 9-row
/// minimum the solver's deficit distribution is unspecified, and the
/// final-row guarantee must not depend on tie-breaking. At every viable
/// height the result equals the plain five-way split of D2; below it, the
/// four upper regions shrink by deterministic greedy top-down allocation
/// (header first, ticker last) — mirroring the clipping order of the
/// line cursor this layout replaces. Precondition: `area.height >= 1`.
pub(crate) fn split_regions(area: Rect) -> Regions {
    let status = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let mut rest = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(1),
    };
    if rest.height >= MIN_REGIONS_HEIGHT {
        let rects = Layout::vertical(REGION_CONSTRAINTS).split(rest);
        return Regions {
            header: rects[0],
            body: rects[1],
            local: rects[2],
            ticker: rects[3],
            status,
        };
    }
    let header = take_rows(&mut rest, 1);
    let body = take_rows(&mut rest, 5);
    let local = take_rows(&mut rest, 4);
    let remaining = rest.height;
    let ticker = take_rows(&mut rest, remaining);
    Regions {
        header,
        body,
        local,
        ticker,
        status,
    }
}

/// Cuts up to `rows` off the top of `front`, advancing it in place.
fn take_rows(front: &mut Rect, rows: u16) -> Rect {
    let height = rows.min(front.height);
    let taken = Rect {
        x: front.x,
        y: front.y,
        width: front.width,
        height,
    };
    front.y = front.y.saturating_add(height);
    front.height -= height;
    taken
}

/// Shell-level phase dispatch. Both branches clip to the frame area (never
/// panic), preserving the resize guarantee exercised via `TestBackend`. In
/// the live branch each region owner renders into its own rect and the
/// status line is drawn LAST so it wins the bottom row in EVERY view (ui
/// spec R6). Generic over the clock seam (design D5) so tests can pin the
/// last-update stamp.
pub fn render<C: Clock>(frame: &mut Frame, app: &App<C>) {
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    match app.phase() {
        Phase::NotInGame => {
            standby::render(frame);
            status::render(frame, app);
        }
        Phase::InGame { .. } => {
            let regions = split_regions(area);
            dashboard::render(frame, app, &regions);
            status::render_into(frame, app, regions.status);
        }
    }
}

/// Draws pre-formatted text lines top-down inside `area`, clipping at the
/// region boundary exactly as the retired line cursor used to clip at the
/// frame edge. Shared by the views that still render plain text rows;
/// widget-backed regions (task 4.x) will replace their calls to this.
pub(crate) fn draw_lines(frame: &mut Frame, area: Rect, lines: &[String]) {
    if area.is_empty() || lines.is_empty() {
        return;
    }
    let text: Text<'static> = lines.iter().map(|line| Line::from(line.clone())).collect();
    frame.render_widget(Paragraph::new(text), area);
}
