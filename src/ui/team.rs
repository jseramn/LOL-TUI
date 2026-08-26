//! Team-column widgets: per-player creep-score bars, fixed-scale level
//! bars, tri-color K/D/A mini-bars, and the inventory fill strip.
//!
//! Owning decision: design **D3** maps each family onto ratatui native
//! primitives (or inline glyph strips) fed exclusively through the glyph
//! whitelist module; one shared-maximum rule governs all counters.
//!
//! # Row contract
//!
//! Every visible player gets ONE additive visualization row inside the body
//! region, below the legacy text panels (which stay byte-identical — pinned
//! by `tests/ui/dashboard_tests.rs`). Rows carry FIXED-geometry sections so
//! columns align across players:
//!
//! ```text
//! Lv########## K#######. D#######. A#######. CS############## ######
//! \___________/\______/ separators \_________________/ \___/ \___/
//!   level (10)   kills(8) ... deaths ... assists      chart  inv(6)
//! ```
//!
//! A section the payload did not expose renders `?` in its first track cell
//! instead of fabricating geometry; zero values render true zero-width bars.
//!
//! # Verified ratatui 0.30.2 API surface (task 4.1, vendored source)
//!
//! Facade `ratatui 0.30.2` resolves to `ratatui-widgets 0.3.2` +
//! `ratatui-core 0.1.2`. Signatures verified against the vendored sources:
//!
//! - **BarChart** (`barchart.rs`): `BarChart::horizontal(impl Into<Vec<Bar>>)`
//!   (line 167), `.max(u64)` const builder (line 272; the renderer clamps the
//!   effective maximum to ≥ 1, so a global max of 0 degrades to true zero
//!   bars without division blowups), `.bar_set(symbols::bar::Set)` symbol
//!   override exists (line 334, default `NINE_LEVELS`). The HORIZONTAL path
//!   fills each cell with `bar_set.full` (`█`) or `bar_set.empty` (`" "`)
//!   only (lines 549–556) — whitelist-pure by default. Bars print their
//!   numeric value unless suppressed: pass `.text_value("")`
//!   (`barchart/bar.rs` line 212 skips empty text).
//! - **Sparkline** (`sparkline.rs`, consumed by task 4.7): `.data(impl
//!   IntoIterator<Item: Into<SparklineBar>>)` accepts `Option<u64>`
//!   natively (line 209); `None` samples render via
//!   `.absent_value_symbol(&str)` (line 143), DEFAULT `symbols::shade::EMPTY`
//!   which is a plain SPACE (not `░`). `.bar_set` default `NINE_LEVELS`
//!   (`▁▂▃▄▅▆▇█` + space) is fully whitelisted; `.max(u64)` const, auto-max
//!   when unset.
//! - **LineGauge** (`gauge.rs`, consumed by task 4.6): `.ratio(f64)` +
//!   custom-symbol overrides CONFIRMED — `.filled_symbol(&str)` /
//!   `.unfilled_symbol(&str)` const builders (lines 341–349); defaults are
//!   `symbols::line::HORIZONTAL` (`─`, U+2500, whitelisted).
//!
//! **Fallback verdict**: signatures match the D3 assumption — NO inline-strip
//! fallback is needed for the three native widgets; inline strips remain
//! confined to the families D3 maps to them (level, K/D/A, inventory).
//!
//! Implemented in Phase 4 (tasks 4.2–4.5).

use crate::history::chart_u64;
use crate::model::snapshot::{PlayerSnapshot, Snapshot, Team};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, Paragraph};

/// Fixed track width of a level bar in cells (task 4.3 fills it).
const LEVEL_TRACK_CELLS: usize = 10;
/// Fixed track width of each K/D/A mini-bar in cells (task 4.4).
const KDA_TRACK_CELLS: usize = 8;

/// Width of the fixed prefix every row carries before the CS chart:
/// `Lv` + level track + one-space separator + three labeled K/D/A tracks
/// (each letter + 8 cells) with separators + the `CS` label.
const PREFIX_CELLS: u16 = 2 + LEVEL_TRACK_CELLS as u16 + 1 + 9 + 1 + 9 + 1 + 9 + 1 + 2;

/// Renders the four team-column widget families into `area`, one row per
/// visible player in roster order (ORDER block first, mirroring the legacy
/// text panels). Extra players clip silently; an empty area is a no-op.
///
/// The CS maximum is GLOBAL — the highest converted score among ALL visible
/// players of both teams (viz:R3/S1) — computed through the sole f64→u64
/// conversion path (design D5).
pub(super) fn render(frame: &mut Frame, snapshot: &Snapshot, area: Rect) {
    if area.is_empty() {
        return;
    }
    let roster: Vec<&PlayerSnapshot> = [Team::Order, Team::Chaos]
        .into_iter()
        .flat_map(|team| {
            snapshot
                .players
                .iter()
                .filter(move |p| p.team == Some(team))
        })
        .collect();

    let cs_values: Vec<Option<u64>> = roster
        .iter()
        .map(|p| p.creep_score.and_then(chart_u64))
        .collect();
    let cs_max = cs_values.iter().flatten().copied().max().unwrap_or(0);

    let rows = area.height.min(roster.len() as u16) as usize;
    for (i, player) in roster.iter().take(rows).enumerate() {
        let row = Rect {
            y: area.y + i as u16,
            height: 1,
            ..area
        };
        render_row(frame, row, player, cs_max);
    }
}

/// Draws one player's visualization row: fixed-prefix labels plus the
/// shared-maximum CS chart (or the explicit placeholder).
fn render_row(frame: &mut Frame, row: Rect, player: &PlayerSnapshot, cs_max: u64) {
    let prefix_width = PREFIX_CELLS.min(row.width);
    let prefix = Rect {
        width: prefix_width,
        ..row
    };
    frame.render_widget(prefix_paragraph(), prefix);

    let chart_area = Rect {
        x: row.x + prefix_width,
        width: row.width - prefix_width,
        ..row
    };
    if chart_area.is_empty() {
        return;
    }
    match player.creep_score.and_then(chart_u64) {
        Some(cs) => frame.render_widget(
            BarChart::horizontal([Bar::default().value(cs).text_value("")])
                .max(cs_max)
                .bar_width(1)
                .bar_gap(0),
            chart_area,
        ),
        None => frame.render_widget(paragraph_of("?"), chart_area),
    }
}

/// The fixed-prefix label line: section letters plus BLANK tracks. Sections
/// gain their glyphs from tasks 4.3–4.5; the geometry is frozen here so the
/// column layout never shifts between tasks or players.
fn prefix_paragraph() -> Paragraph<'static> {
    let mut spans = vec![
        Span::from("Lv"),
        Span::from(" ".repeat(LEVEL_TRACK_CELLS)),
        Span::from(" K"),
        Span::from(" ".repeat(KDA_TRACK_CELLS)),
        Span::from(" D"),
        Span::from(" ".repeat(KDA_TRACK_CELLS)),
        Span::from(" A"),
        Span::from(" ".repeat(KDA_TRACK_CELLS)),
        Span::from(" CS"),
    ];
    spans.shrink_to_fit();
    Paragraph::new(Line::from(spans))
}

/// One-line paragraph holding `text`, clipped to whatever rect it gets.
fn paragraph_of(text: &'static str) -> Paragraph<'static> {
    Paragraph::new(Line::from(Span::from(text)))
}
