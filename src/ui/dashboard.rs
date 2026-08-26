//! Team-grouped live dashboard panels (ui spec R3).
//!
//! The dashboard renders the [`App`]'s latest retained [`Snapshot`] every
//! frame into the shell's disjoint regions: a `LIVE` headline in the header
//! band, ORDER and CHAOS blocks in the body band, and the local strip in
//! its own band — each listing one line-panel per player with the fields
//! exposed by the API, clipped at the region boundary instead of the frame
//! edge. Absent fields degrade explicitly (task 4.3); gold renders only on
//! the local-player strip (task 4.4).
//!
//! Compliance (design): the respawn value printed here is the exposed
//! snapshot value verbatim — nothing is derived or counted down locally.

use super::{LiveLayout, TeamColumns, draw_lines};
use crate::api::poller::Clock;
use crate::app::App;
use crate::model::snapshot::{LocalPlayerSnapshot, PlayerSnapshot, Snapshot, Team};
use ratatui::Frame;
use ratatui::layout::Rect;

/// Explicit marker for a field the API did not expose. Absence is never
/// rendered as a fabricated value (ui spec: degradation is per field).
pub const UNKNOWN: &str = "?";

/// Renders the in-game view into the shell's regions: headline plus
/// team-grouped panels for the latest snapshot, then the event ticker in
/// its own band. Widget families render only when the degradation matrix
/// (`layout.visible`, design D9) keeps them visible at this viewport.
/// With no snapshot yet (lifecycle arrived first), only the headline
/// draws — never a panic, whatever the frame size.
pub(crate) fn render<C: Clock>(frame: &mut Frame, app: &App<C>, layout: &LiveLayout) {
    draw_lines(frame, layout.areas.header, &["LIVE".to_owned()]);

    let Some(snapshot) = app.snapshot() else {
        return;
    };
    draw_snapshot(snapshot, app.gold_window(), layout, frame);
}

fn draw_snapshot<G>(snapshot: &Snapshot, gold_window: G, layout: &LiveLayout, frame: &mut Frame)
where
    G: Iterator<Item = Option<u64>>,
{
    let regions = &layout.areas;
    let mut panel_lines: Vec<String> = Vec::new();
    for (header, team) in [("Team ORDER", Team::Order), ("Team CHAOS", Team::Chaos)] {
        let members: Vec<&PlayerSnapshot> = snapshot
            .players
            .iter()
            .filter(|p| p.team == Some(team))
            .collect();
        if members.is_empty() {
            continue;
        }
        panel_lines.push(header.to_owned());
        for player in members {
            panel_lines.push(player_line(player));
        }
    }
    draw_lines(frame, regions.body, &panel_lines);

    // Team-column visualizations (viz spec R3–R6): additive rows BELOW the
    // legacy text panels, laid out SIDE BY SIDE in the shell's ORDER and
    // CHAOS team columns (viz spec R2 — the W-1 remediation). The columns
    // are clipped to the viz band below the text content; the text above is
    // untouched — standing panel pins hold — and the families themselves
    // obey the degradation matrix's visible set (viz:R9): below their tier
    // they draw nothing at all. Shared maxima stay global across both teams.
    let used_rows = panel_lines.len().min(regions.body.height as usize) as u16;
    if regions.body.height > used_rows {
        let viz_height = regions.body.height - used_rows;
        let columns = TeamColumns {
            order: Rect {
                y: regions.body.y + used_rows,
                height: viz_height,
                ..layout.columns.order
            },
            chaos: Rect {
                y: regions.body.y + used_rows,
                height: viz_height,
                ..layout.columns.chaos
            },
        };
        super::team::render(frame, snapshot, columns, layout.visible);
    }

    // Local-player strip (ui spec R4): the ONLY surface that ever renders
    // gold. `activePlayer` absent from the payload → no strip at all. The
    // band's first row keeps the legacy text line byte-for-byte; the rows
    // below it host the gauge/sparkline widgets (tasks 4.6–4.7). Gauges are
    // untiered; only the sparkline obeys the matrix.
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

    // Objective/kill ticker (ui spec R5) in its own compressible band.
    super::ticker::render(&snapshot.events, regions.ticker, frame);
}

/// Formats the distinguished local-player line:
/// `LOCAL {champion} Lv{level} Gold {gold} | HP {cur}/{max} Power {p}/{max} MS {ms}`
///
/// Gold is rendered exclusively here — roster panels never carry it
/// (ui spec R4/S2). Values are exposed verbatim; absent ones degrade to
/// [`UNKNOWN`].
fn local_line(local: &LocalPlayerSnapshot) -> String {
    let mut line = String::from("LOCAL");
    push_part(&mut line, local.champion.as_deref());

    line.push_str(" Lv");
    line.push_str(
        local
            .level
            .map(|l| l.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    line.push_str(" Gold ");
    line.push_str(
        local
            .current_gold
            .map(|g| g.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    if let Some(stats) = &local.stats {
        line.push_str(" | HP ");
        line.push_str(
            stats
                .current_health
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push('/');
        line.push_str(
            stats
                .max_health
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push_str(" Power ");
        line.push_str(
            stats
                .power
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push('/');
        line.push_str(
            stats
                .power_max
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push_str(" MS ");
        line.push_str(
            stats
                .movement_speed
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
    }
    line
}

/// Formats one player panel line:
/// `{name} {champion} Lv{level} {k}/{d}/{a} CS{cs} {spell1}+{spell2}[ DEAD(respawn {t}|?)] | Items: …`
///
/// Values are the exposed snapshot values verbatim; fields the payload
/// omits render as the explicit [`UNKNOWN`] marker — placeholders appear
/// ONLY on absent fields, never on populated ones.
fn player_line(p: &PlayerSnapshot) -> String {
    let mut line = String::with_capacity(128);
    push_part(&mut line, p.summoner_name.as_deref());
    push_part(&mut line, p.champion.as_deref());

    if !line.is_empty() {
        line.push(' ');
    }
    line.push_str("Lv");
    line.push_str(p.level.map(|l| l.to_string()).as_deref().unwrap_or(UNKNOWN));

    line.push(' ');
    line.push_str(&num_marker(p.kills));
    line.push('/');
    line.push_str(&num_marker(p.deaths));
    line.push('/');
    line.push_str(&num_marker(p.assists));

    line.push_str(" CS");
    line.push_str(
        p.creep_score
            .map(|cs| cs.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    line.push(' ');
    line.push_str(p.spell_one.as_deref().unwrap_or(UNKNOWN));
    line.push('+');
    line.push_str(p.spell_two.as_deref().unwrap_or(UNKNOWN));

    // Death state: the exposed respawn value, or an unknown marker when the
    // API exposes none. A countdown is NEVER derived locally (design hard
    // rule).
    if p.is_dead == Some(true) {
        match p.respawn_timer {
            Some(timer) => line.push_str(&format!(" DEAD(respawn {timer})")),
            None => line.push_str(&format!(" DEAD(respawn {UNKNOWN})")),
        }
    }

    line.push_str(" | Items: ");
    match &p.items {
        Some(items) => line.push_str(
            &items
                .iter()
                .map(|i| i.display_name.as_deref().unwrap_or(UNKNOWN))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        None => line.push_str(UNKNOWN),
    }
    line
}

fn push_part(line: &mut String, part: Option<&str>) {
    if let Some(part) = part {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(part);
    } else {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(UNKNOWN);
    }
}

fn num_marker(value: Option<u32>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}
