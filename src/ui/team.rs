//! Team-column roster: one Spanish identity line per player.
//!
//! Each team is a side-by-side card column (viz spec R2): a coloured
//! `Equipo Orden` / `Equipo Caos` header, then role, champion, level, KDA,
//! farm, and death tag. KDA / level / CS / inventory bars are not drawn —
//! those numbers already sit on the identity line. Level / CS / inventory
//! helpers stay exported for unit tests.

use super::TeamColumns;
use super::format;
use crate::model::snapshot::{ItemSnapshot, PlayerSnapshot, Snapshot, Team};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

/// Full-width level track (frozen U3 contract; used by [`level_fill_cells`]).
const LEVEL_TRACK_FULL: usize = 10;
/// Fixed cell count of the inventory fill strip (slots 0–5).
const INVENTORY_CELLS: usize = 6;

/// The trinket inventory slot: it can never be PROVEN to hold an item, so
/// it is excluded from the fill count (design D3).
const TRINKET_SLOT: u8 = 6;

/// Renders each team as a side-by-side card column (viz spec R2): a
/// coloured `Equipo Orden` / `Equipo Caos` header, then one identity line
/// per player. Extra players clip silently.
pub(super) fn render(frame: &mut Frame, snapshot: &Snapshot, columns: TeamColumns) {
    render_team(frame, snapshot, Team::Order, columns.order);
    render_team(frame, snapshot, Team::Chaos, columns.chaos);
}

/// Draws ONE team's header + player identity lines top-down inside its column.
fn render_team(frame: &mut Frame, snapshot: &Snapshot, team: Team, area: Rect) {
    if area.is_empty() {
        return;
    }
    let color = match team {
        Team::Order => Color::Cyan,
        Team::Chaos => Color::Red,
    };
    let title = super::scoreboard::team_title(team, snapshot);
    frame.render_widget(
        Paragraph::new(Line::from(ratatui::text::Span::styled(
            title,
            Style::default().fg(color),
        ))),
        Rect { height: 1, ..area },
    );

    let members: Vec<&PlayerSnapshot> = snapshot
        .players
        .iter()
        .filter(|p| p.team == Some(team))
        .collect();
    let mut y = 1u16;
    for player in members {
        if y >= area.height {
            break;
        }
        let ident = Rect {
            y: area.y + y,
            height: 1,
            ..area
        };
        // Leave one cell empty so 80-wide halves cannot glue `12sGraves`.
        let text = format::player_line_for_width(player, area.width.saturating_sub(1));
        let identity =
            Paragraph::new(text).style(Style::default().fg(if player.is_dead == Some(true) {
                Color::DarkGray
            } else {
                color
            }));
        frame.render_widget(identity, ident);
        y = y.saturating_add(1);
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

/// Maps a level onto the FIXED 1–18 scale (viz:R4): `(level − 1) / 17`
/// clamped into [0, 1], expressed in whole track cells of the full (10-cell)
/// contract. The mapping never rescales between frames and never panics —
/// absurd values saturate.
pub fn level_fill_cells(level: u32) -> usize {
    let ratio = ((f64::from(level) - 1.0) / 17.0).clamp(0.0, 1.0);
    (ratio * LEVEL_TRACK_FULL as f64).round() as usize
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
