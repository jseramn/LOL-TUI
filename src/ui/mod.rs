//! View layer: standby, dashboard panels, event ticker, and status line.
//!
//! This module hosts the shell renderer ([`render`]) that dispatches per
//! lifecycle phase. The idle branch keeps the full-frame [`standby`]
//! paragraph. The live branch computes a [`LiveLayout`] through
//! [`select_layout`] — a PURE function of the frame rectangle (designs
//! D9/D10): it splits the frame into disjoint region rects via
//! [`split_regions`] (design D2) and derives which chart families the
//! current viewport can still host (the D9 degradation matrix). Each
//! region routes to its owner view ([`dashboard`] renders header/body/
//! local and drives the [`ticker`]), and the status line is drawn LAST so
//! it owns the bottom row in EVERY view — making the ui spec R6 notice
//! guarantee structural rather than convention: no widget ever receives a
//! rect containing the status row.

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
pub struct Regions {
    pub header: Rect,
    pub body: Rect,
    pub local: Rect,
    pub ticker: Rect,
    pub status: Rect,
}

/// Viewport width below which EVERY chart family hides, whatever the
/// height (design D9: `width < 40` ⇒ no charts). Gauges are unaffected —
/// they belong to no tier.
pub const MIN_CHART_WIDTH: u16 = 40;

/// Smallest height that still shows the CS bars (design D9 tier table).
const CS_MIN_HEIGHT: u16 = 13;
/// Smallest height that keeps the level bars (D9).
const LEVEL_MIN_HEIGHT: u16 = 15;
/// Smallest height that keeps the inventory strip (D9).
const INVENTORY_MIN_HEIGHT: u16 = 18;
/// Smallest height that keeps the K/D/A mini-bars (D9).
const KDA_MIN_HEIGHT: u16 = 20;
/// Height at which every family — including the gold sparkline — is
/// visible (D9; viz spec R9 pins "all charts visible at ≥ 80×24").
const SPARKLINE_MIN_HEIGHT: u16 = 24;

/// Which chart families the current viewport may draw. Gauges are NOT
/// members: the degradation matrix gives them no hide tier (viz spec R9
/// omits them ⇒ always-on), so there is no flag that could ever turn them
/// off. The five flags are ordered by the spec's hide priority — the
/// sparkline hides FIRST and the CS bars hide LAST.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartSet {
    pub cs: bool,
    pub level: bool,
    pub inventory: bool,
    pub kda: bool,
    pub sparkline: bool,
}

impl ChartSet {
    /// No chart family visible (height ≤ 12, or width < [`MIN_CHART_WIDTH`]).
    pub const NONE: Self = Self {
        cs: false,
        level: false,
        inventory: false,
        kda: false,
        sparkline: false,
    };

    /// Every chart family visible (the ≥ 24-row full-layout viewport).
    pub const ALL: Self = Self {
        cs: true,
        level: true,
        inventory: true,
        kda: true,
        sparkline: true,
    };
}

/// The pure result of the D9 degradation matrix for one frame: disjoint
/// region rects plus the set of chart families the viewport can host.
pub struct LiveLayout {
    pub areas: Regions,
    pub visible: ChartSet,
}

/// Computes the live view's layout from the frame rectangle ALONE (designs
/// D9/D10): regions via [`split_regions`], chart visibility via the D9
/// tier table. No terminal, no app state — the same rectangle always
/// yields the same result, which is what makes the matrix offline-sweepable
/// (viz:R9).
///
/// Tier table by HEIGHT (families accumulate as rows grow): ≤ 12 none ·
/// 13–14 +CS · 15–17 +level · 18–19 +inventory · 20–23 +K/D/A · ≥ 24 all
/// (+ gold sparkline). A WIDTH below [`MIN_CHART_WIDTH`] hides every chart
/// family. Hiding stays monotonic in both dimensions, and strictly follows
/// the spec priority order: sparkline → K/D/A → inventory → level → CS.
/// The ticker compresses first (`Min(1)`); the status row belongs to no
/// tier and never hides.
pub fn select_layout(area: Rect) -> LiveLayout {
    let mut visible = ChartSet::ALL;
    if area.height < SPARKLINE_MIN_HEIGHT {
        visible.sparkline = false;
    }
    if area.height < KDA_MIN_HEIGHT {
        visible.kda = false;
    }
    if area.height < INVENTORY_MIN_HEIGHT {
        visible.inventory = false;
    }
    if area.height < LEVEL_MIN_HEIGHT {
        visible.level = false;
    }
    if area.height < CS_MIN_HEIGHT {
        visible.cs = false;
    }
    if area.width < MIN_CHART_WIDTH {
        visible = ChartSet::NONE;
    }
    LiveLayout {
        areas: split_regions(area),
        visible,
    }
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
            let layout = select_layout(area);
            dashboard::render(frame, app, &layout);
            status::render_into(frame, app, layout.areas.status);
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
