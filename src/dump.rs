//! Headless snapshot printer for cloud/CI and `--dump`.
//!
//! Renders the same identity lines the TUI uses, without a terminal.

use crate::model::snapshot::{Snapshot, Team};
use crate::ui::format::{local_line, player_line};
use crate::ui::scoreboard::{header_band, team_title};
use crate::ui::ticker;

/// Multi-line text dump of one live snapshot.
pub fn dump_snapshot(snapshot: &Snapshot) -> String {
    let mut out = String::new();
    for line in header_band(snapshot) {
        out.push_str(&line);
        out.push('\n');
    }
    for team in [Team::Order, Team::Chaos] {
        out.push_str(&team_title(team, snapshot));
        out.push('\n');
        let members: Vec<_> = snapshot
            .players
            .iter()
            .filter(|p| p.team == Some(team))
            .collect();
        if members.is_empty() {
            out.push_str("  (sin jugadores)\n");
            continue;
        }
        for player in members {
            out.push_str("  ");
            out.push_str(&player_line(player));
            out.push('\n');
        }
    }
    if let Some(local) = &snapshot.local {
        out.push_str(&local_line(local));
        out.push('\n');
    }
    out.push_str(&ticker::dump_events(snapshot));
    out
}
