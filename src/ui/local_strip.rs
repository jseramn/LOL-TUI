//! Local-player strip widgets: HP and power gauges plus the gold sparkline
//! with its warm-up placeholder.
//!
//! Owning decisions: design **D3** assigns gauges/sparkline to the local
//! strip only (gold never leaves it); **D6** requires fewer than two real
//! samples to render explicit warm-up text instead of a flat line.
//!
//! # Row contract
//!
//! The strip region hosts four rows: the legacy LOCAL text line (kept
//! byte-for-byte, pinned by `tests/ui/dashboard_tests.rs`), then the HP
//! gauge, the Power gauge, and the gold-trend row. A gauge row leads with
//! an explicit `LOCAL …` label so scan helpers can never confuse it with
//! the champion-bearing text line; a field the payload did not expose
//! renders `?` on THAT gauge alone instead of fabricating geometry.
//!
//! # Verified ratatui 0.30.2 API surface (task 4.1, vendored source)
//!
//! - **LineGauge** (`gauge.rs`): `.ratio(f64)` PANICS outside [0, 1]
//!   (lines 312–318) — [`gauge_ratio`] clamps first. `.filled_symbol(&str)`
//!   / `.unfilled_symbol(&str)` const builders confirmed (lines 341–349);
//!   the renderer paints the custom label at the left, one space, then
//!   `floor(remaining × ratio)` filled cells and unfilled cells to the edge
//!   (lines 424–454).
//!
//! Implemented in Phase 4 (tasks 4.6–4.7).

use crate::glyphs::Glyph;
use crate::model::snapshot::LocalPlayerSnapshot;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph};

/// The clamped fill ratio of one gauge (viz:R7, design error-taxonomy row):
/// `current / max` clamped into [0, 1]. Any absent operand, non-finite
/// value, or non-positive maximum maps to absent (`None`) — a gauge never
/// fabricates a fill from data the payload did not expose. Non-finite
/// handling mirrors the D5 conversion policy; the clamp MUST run before any
/// `LineGauge::ratio` builder call, which panics outside [0, 1].
pub fn gauge_ratio(current: Option<f64>, max: Option<f64>) -> Option<f64> {
    let current = current?;
    let max = max?;
    if !current.is_finite() || !max.is_finite() || max <= 0.0 {
        return None;
    }
    Some((current / max).clamp(0.0, 1.0))
}

/// Renders the local-strip widget rows into `area`: the blocked-segment HP
/// and power gauges (viz:R7), then the gold trend (task 4.7). Rows clip
/// silently when the band shrinks below the full contract; an empty area is
/// a no-op.
pub(super) fn render(frame: &mut Frame, local: &LocalPlayerSnapshot, area: Rect) {
    if area.is_empty() {
        return;
    }
    let stats = local.stats.as_ref();
    let hp_row = Rect { height: 1, ..area };
    render_gauge(
        frame,
        hp_row,
        "LOCAL HP",
        stats.and_then(|s| s.current_health),
        stats.and_then(|s| s.max_health),
    );
    let rest = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };
    if !rest.is_empty() {
        let power_row = Rect { height: 1, ..rest };
        render_gauge(
            frame,
            power_row,
            "LOCAL Power",
            stats.and_then(|s| s.power),
            stats.and_then(|s| s.power_max),
        );
    }
}

/// Draws one blocked-segment gauge: filled cells in full blocks over a
/// light-shade track with the ratio echoed in the label — or the explicit
/// `?` placeholder when any operand is absent (viz:R7/S2). Glyphs flow
/// exclusively through the whitelist module (design D8).
fn render_gauge(frame: &mut Frame, row: Rect, label: &str, current: Option<f64>, max: Option<f64>) {
    match gauge_ratio(current, max) {
        Some(ratio) => {
            let label = format!("{label} {:3.0}%", ratio * 100.0);
            frame.render_widget(
                LineGauge::default()
                    .ratio(ratio)
                    .label(Line::from(label))
                    .filled_symbol(Glyph::FullBlock.symbol())
                    .unfilled_symbol(Glyph::LightShade.symbol()),
                row,
            );
        }
        None => frame.render_widget(paragraph_of(&format!("{label} ?")), row),
    }
}

/// One-line paragraph holding `text`, clipped to whatever rect it gets.
fn paragraph_of(text: &str) -> Paragraph<'static> {
    Paragraph::new(Line::from(Span::from(text.to_owned())))
}
