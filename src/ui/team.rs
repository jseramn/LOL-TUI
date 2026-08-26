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

use super::ChartSet;
use crate::glyphs::Glyph;
use crate::history::chart_u64;
use crate::model::snapshot::{ItemSnapshot, PlayerSnapshot, Snapshot, Team};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Bar, BarChart, Paragraph};

/// Fixed track width of a level bar in cells (task 4.3 fills it).
const LEVEL_TRACK_CELLS: usize = 10;
/// Fixed track width of each K/D/A mini-bar in cells (task 4.4).
const KDA_TRACK_CELLS: usize = 8;
/// Fixed cell count of the inventory fill strip (slots 0–5; task 4.5).
const INVENTORY_CELLS: usize = 6;

/// Prefix width in cells for a given visibility set: each visible family
/// contributes its fixed section, plus single-space separators between
/// adjacent sections. The full tier (`ChartSet::ALL` team families) must
/// keep reproducing the frozen U3 row contract: `Lv<10> K<8> D<8> A<8> CS`
/// = 45 cells.
fn prefix_cells(visible: ChartSet) -> u16 {
    let section_cells = visible.level as u16 * (2 + LEVEL_TRACK_CELLS as u16)
        + visible.kda as u16 * 3 * (1 + KDA_TRACK_CELLS as u16)
        + visible.cs as u16 * 2;
    let section_count = visible.level as u16 + visible.kda as u16 * 3 + visible.cs as u16;
    section_cells.saturating_add(section_count.saturating_sub(1))
}

/// Separator cell between the CS chart and the inventory strip.
const INVENTORY_GAP: u16 = 1;

/// The trinket inventory slot: it can never be PROVEN to hold an item, so
/// it is excluded from the fill count (design D3).
const TRINKET_SLOT: u8 = 6;

/// Renders the team-column widget families the degradation matrix marked
/// visible into `area`, one row per visible player in roster order (ORDER
/// block first, mirroring the legacy text panels). Hidden families omit
/// their whole section — track, label, chart or strip — from every row;
/// with all four families hidden the band draws nothing at all. Extra
/// players clip silently; an empty area is a no-op.
///
/// The CS maximum is GLOBAL — the highest converted score among ALL visible
/// players of both teams (viz:R3/S1) — computed through the sole f64→u64
/// conversion path (design D5).
pub(super) fn render(frame: &mut Frame, snapshot: &Snapshot, area: Rect, visible: ChartSet) {
    if area.is_empty() || !(visible.cs || visible.level || visible.kda || visible.inventory) {
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

    // Each K/D/A metric shares its own maximum across ALL visible players
    // (viz:R5). These are integer counters — they bypass `chart_u64` by
    // design (D5), so no truncation policy can drift their values.
    let counter_max = |pick: fn(&PlayerSnapshot) -> Option<u32>| {
        roster
            .iter()
            .filter_map(|p| pick(p).map(u64::from))
            .max()
            .unwrap_or(0)
    };
    let kda_max = (
        counter_max(|p| p.kills),
        counter_max(|p| p.deaths),
        counter_max(|p| p.assists),
    );

    let rows = area.height.min(roster.len() as u16) as usize;
    for (i, player) in roster.iter().take(rows).enumerate() {
        let row = Rect {
            y: area.y + i as u16,
            height: 1,
            ..area
        };
        render_row(frame, row, player, cs_max, kda_max, visible);
    }
}

/// Draws one player's visualization row: only the sections the degradation
/// matrix left visible — fixed-prefix tracks first, then the
/// shared-maximum CS chart (or the explicit placeholder), then the
/// inventory strip.
fn render_row(
    frame: &mut Frame,
    row: Rect,
    player: &PlayerSnapshot,
    cs_max: u64,
    kda_max: (u64, u64, u64),
    visible: ChartSet,
) {
    let prefix_width = prefix_cells(visible).min(row.width);
    let prefix = Rect {
        width: prefix_width,
        ..row
    };
    frame.render_widget(
        Paragraph::new(prefix_spans(player, kda_max, visible)),
        prefix,
    );

    if !visible.cs {
        return;
    }
    let chart_area = Rect {
        x: row.x + prefix_width,
        width: row.width - prefix_width,
        ..row
    };
    if chart_area.is_empty() {
        return;
    }

    // Reserve the trailing inventory strip for the CS chart's right edge
    // when the tier keeps it visible and the row is wide enough; otherwise
    // the strip clips away with the rest of the row.
    let inv_room = INVENTORY_GAP + INVENTORY_CELLS as u16;
    let (chart_area, inv_area) = if visible.inventory && chart_area.width > inv_room {
        let chart = Rect {
            width: chart_area.width - inv_room,
            ..chart_area
        };
        let inv = Rect {
            x: chart_area.x + chart_area.width - INVENTORY_CELLS as u16,
            width: INVENTORY_CELLS as u16,
            ..chart_area
        };
        (chart, Some(inv))
    } else {
        (chart_area, None)
    };

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

    if let Some(inv_area) = inv_area {
        frame.render_widget(inventory_paragraph(player.items.as_deref()), inv_area);
    }
}

/// Counts occupied inventory slots for the fill strip (viz:R6, design D3):
/// an entry occupies a cell only when it carries an item identity AND does
/// not sit in the trinket slot — a trinket's presence can never be proven.
/// Slotless entries count (same unprovability); `None` items (absent list)
/// yield `None`, distinct from a present-but-empty list (`Some(0)`).
pub fn inventory_occupied(items: Option<&[ItemSnapshot]>) -> Option<usize> {
    items.map(|list| {
        list.iter()
            .filter(|entry| entry.item_id.is_some() && entry.slot != Some(TRINKET_SLOT))
            .count()
            .min(INVENTORY_CELLS)
    })
}

/// The six-cell fill strip: dark shade per occupied slot, light shade per
/// empty one — or the explicit placeholder for an absent list. Glyphs flow
/// exclusively through the whitelist module (design D8).
fn inventory_paragraph(items: Option<&[ItemSnapshot]>) -> Paragraph<'static> {
    match inventory_occupied(items) {
        Some(occupied) => Paragraph::new(Line::from(Span::from(format!(
            "{}{}",
            Glyph::DarkShade.symbol().repeat(occupied),
            Glyph::LightShade
                .symbol()
                .repeat(INVENTORY_CELLS - occupied),
        )))),
        None => paragraph_of("?     "),
    }
}

/// Maps a level onto the FIXED 1–18 scale (viz:R4): `(level − 1) / 17`
/// clamped into [0, 1], expressed in whole track cells. The mapping never
/// rescales between frames and never panics — absurd values saturate.
pub fn level_fill_cells(level: u32) -> usize {
    let ratio = ((f64::from(level) - 1.0) / 17.0).clamp(0.0, 1.0);
    (ratio * f64::from(LEVEL_TRACK_CELLS as u16)).round() as usize
}

/// Maps a counter onto a shared maximum, expressed in whole track cells:
/// rounded half-up, never exceeding the track. `max == 0` means every
/// visible value is zero — every bar is then a true zero (empty track).
/// Display-only scaling; the D5 conversion policy does not apply here.
pub fn scaled_cells(value: u64, max: u64, track: usize) -> usize {
    if max == 0 {
        return 0;
    }
    let track = track as u64;
    (((value * track) + (max / 2)) / max).min(track) as usize
}

/// The fixed-prefix label spans for one player: only the sections the
/// degradation matrix keeps visible, joined by single spaces. With every
/// team family visible the emitted bytes are IDENTICAL to the pre-tier
/// frozen contract (`Lv<10> K<8> D<8> A<8> CS`).
fn prefix_spans(
    player: &PlayerSnapshot,
    kda_max: (u64, u64, u64),
    visible: ChartSet,
) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(5);
    if visible.level {
        // Label and track are ONE section — fusing them keeps the emitted
        // element count equal to `prefix_cells`'s section count, so the
        // full-tier prefix stays exactly 45 cells.
        spans.push(Span::from(format!("Lv{}", level_track(player.level))));
    }
    if visible.kda {
        spans.push(metric_span("K", player.kills, kda_max.0, Color::Green));
        spans.push(metric_span("D", player.deaths, kda_max.1, Color::Red));
        spans.push(metric_span("A", player.assists, kda_max.2, Color::Blue));
    }
    if visible.cs {
        spans.push(Span::from("CS"));
    }
    join_with_spaces(spans)
}

/// Collapses a span list into one line, inserting single-space separators
/// between adjacent spans (the frozen row-contract separator).
fn join_with_spaces(spans: Vec<Span<'static>>) -> Line<'static> {
    let mut line = Line::default();
    for (i, span) in spans.into_iter().enumerate() {
        if i > 0 {
            line.push_span(Span::from(" "));
        }
        line.push_span(span);
    }
    line
}

/// One labeled K/D/A mini-bar: the metric letter followed by its fixed
/// track, scaled by that metric's own shared maximum. A zero renders a
/// truly empty track; an absent counter renders `?`. The whole span binds
/// to the spec's color (kills green, deaths red, assists blue) so the
/// binding survives palette degradation — only colors degrade, never a
/// bar itself (viz:R5/S2, S3).
fn metric_span(letter: &str, value: Option<u32>, max: u64, color: Color) -> Span<'static> {
    let track = match value {
        Some(value) => {
            let filled = scaled_cells(u64::from(value), max, KDA_TRACK_CELLS);
            format!(
                "{letter}{}{}",
                Glyph::FullBlock.symbol().repeat(filled),
                " ".repeat(KDA_TRACK_CELLS - filled),
            )
        }
        None => format!("{letter}?{}", " ".repeat(KDA_TRACK_CELLS - 1)),
    };
    Span::styled(track, Style::default().fg(color))
}

/// The level bar: `filled` full blocks followed by light-shade empty track,
/// or the explicit `?` placeholder when the payload omitted the level.
/// Glyphs flow exclusively through the whitelist module (design D8).
fn level_track(level: Option<u32>) -> String {
    match level {
        Some(level) => {
            let filled = level_fill_cells(level);
            // The ratio is clamped into [0, 1] first, so `filled` can never
            // exceed the track width.
            let blocks = Glyph::FullBlock.symbol().repeat(filled);
            let shades = Glyph::LightShade
                .symbol()
                .repeat(LEVEL_TRACK_CELLS - filled);
            format!("{blocks}{shades}")
        }
        None => format!("?{}", " ".repeat(LEVEL_TRACK_CELLS.saturating_sub(1))),
    }
}

/// One-line paragraph holding `text`, clipped to whatever rect it gets.
fn paragraph_of(text: &'static str) -> Paragraph<'static> {
    Paragraph::new(Line::from(Span::from(text)))
}
