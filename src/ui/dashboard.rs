//! Team-grouped live dashboard panels (ui spec R3).
//!
//! The dashboard renders the [`App`]'s latest retained [`Snapshot`] every
//! frame: a `LIVE` headline followed by an ORDER block and a CHAOS block,
//! each listing one line-panel per player with the fields exposed by the
//! API. Absent fields degrade explicitly (task 4.3); gold renders only on
//! the local-player strip (task 4.4).
//!
//! Compliance (design): the respawn value printed here is the exposed
//! snapshot value verbatim — nothing is derived or counted down locally.

use crate::app::App;
use crate::model::snapshot::{PlayerSnapshot, Snapshot, Team};
use ratatui::layout::Rect;
use ratatui::{Frame, widgets::Paragraph};

/// Explicit marker for a field the API did not expose. Absence is never
/// rendered as a fabricated value (ui spec: degradation is per field).
pub const UNKNOWN: &str = "?";

/// Renders the in-game view: headline plus team-grouped panels for the
/// latest snapshot. With no snapshot yet (lifecycle arrived first), only
/// the headline draws — never a panic, whatever the frame size.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    let mut pen = Pen { x: area.x, y: area.y, width: area.width, bottom: area.bottom() };
    pen.line("LIVE".to_owned(), frame);

    let Some(snapshot) = app.snapshot() else { return };
    draw_snapshot(&mut pen, snapshot, frame);
}

fn draw_snapshot(pen: &mut Pen, snapshot: &Snapshot, frame: &mut Frame) {
    for (header, team) in [("Team ORDER", Team::Order), ("Team CHAOS", Team::Chaos)] {
        let members: Vec<&PlayerSnapshot> =
            snapshot.players.iter().filter(|p| p.team == Some(team)).collect();
        if members.is_empty() {
            continue;
        }
        pen.line(header.to_owned(), frame);
        for player in members {
            pen.line(player_line(player), frame);
        }
    }
}

/// One-line vertical cursor that clips at the frame bottom instead of
/// panicking, so degenerate viewports stay safe by construction.
struct Pen {
    x: u16,
    y: u16,
    width: u16,
    bottom: u16,
}

impl Pen {
    fn line(&mut self, text: String, frame: &mut Frame) {
        if self.y >= self.bottom || self.width == 0 {
            return;
        }
        let area = Rect { x: self.x, y: self.y, width: self.width, height: 1 };
        frame.render_widget(Paragraph::new(text), area);
        self.y += 1;
    }
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
    line.push_str(p.creep_score.map(|cs| cs.to_string()).as_deref().unwrap_or(UNKNOWN));

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
    value.map(|v| v.to_string()).unwrap_or_else(|| UNKNOWN.to_owned())
}
