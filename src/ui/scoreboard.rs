//! Match header: mode, exposed clock, team kill totals, objective counts.
//!
//! Kill totals sum *exposed* per-player kill counters (ui spec forbids
//! inventing values; a team whose roster exposes no kills at all shows `?`).
//! Objective counts summarize events already in the snapshot ticker — they
//! do not invent timers or unexposed state.

use super::format;
use crate::glyphs::Glyph;
use crate::intel;
use crate::model::snapshot::{GameEvent, Snapshot, Team};

/// Single-row live headline: clock as `mm:ss`, Spanish words, no acronyms.
pub fn header_line(snapshot: &Snapshot) -> String {
    let mut line = String::from("En partida");
    if let Some(game) = &snapshot.game {
        if let Some(time) = game.game_time {
            line.push_str("  ");
            line.push_str(&format::clock(time));
        }
        if let Some(mode) = &game.game_mode {
            line.push_str("  ");
            line.push_str(&format::game_mode_name(mode));
        }
    }

    let order = team_kills(snapshot, Team::Order);
    let chaos = team_kills(snapshot, Team::Chaos);
    line.push_str(" | Orden ");
    line.push_str(&order);
    line.push_str(" - ");
    line.push_str(&chaos);
    line.push_str(" Caos");

    let objectives = objectives_summary(snapshot);
    if !objectives.is_empty() {
        line.push_str(" | ");
        line.push_str(&objectives);
    }
    line
}

/// Scoreboard plus up to three decision sentences. A one-row header clips
/// to the scoreboard; a four-row header shows the briefing underneath.
pub fn header_band(snapshot: &Snapshot) -> Vec<String> {
    let mut lines = vec![header_line(snapshot)];
    lines.extend(intel::briefing(snapshot));
    lines
}

/// Sum of exposed kills for one team. `?` only when every member omitted
/// the counter (or the roster is empty).
pub fn team_kills(snapshot: &Snapshot, team: Team) -> String {
    let members: Vec<_> = snapshot
        .players
        .iter()
        .filter(|p| p.team == Some(team))
        .collect();
    if members.is_empty() {
        return UNKNOWN.to_owned();
    }
    if members.iter().all(|p| p.kills.is_none()) {
        return UNKNOWN.to_owned();
    }
    let total: u32 = members.iter().filter_map(|p| p.kills).sum();
    total.to_string()
}

const UNKNOWN: &str = "?";

/// Coloured column caption: `Equipo Orden  23` / `Equipo Caos  20`.
pub fn team_title(team: Team, snapshot: &Snapshot) -> String {
    let kills = team_kills(snapshot, team);
    match team {
        Team::Order => format!("Equipo Orden  {kills}"),
        Team::Chaos => format!("{} Equipo Caos  {kills}", Glyph::Vertical.symbol()),
    }
}

/// Objective readout from the event list (counts only, full Spanish nouns).
pub fn objectives_summary(snapshot: &Snapshot) -> String {
    let mut dragons = 0u32;
    let mut barons = 0u32;
    let mut heralds = 0u32;
    let mut hordes = 0u32;
    let mut turrets = 0u32;
    let mut inhibs = 0u32;
    for event in &snapshot.events {
        match event {
            GameEvent::DragonKill { .. } => dragons += 1,
            GameEvent::BaronKill { .. } => barons += 1,
            GameEvent::HeraldKill { .. } => heralds += 1,
            GameEvent::HordeKill { .. } => hordes += 1,
            GameEvent::TurretKilled { .. } => turrets += 1,
            GameEvent::InhibKilled { .. } => inhibs += 1,
            _ => {}
        }
    }
    let mut parts = Vec::new();
    push_counted(&mut parts, dragons, "dragon", "dragones");
    push_counted(&mut parts, barons, "baron", "barones");
    push_counted(&mut parts, heralds, "heraldo", "heraldos");
    push_counted(&mut parts, hordes, "gusarapo", "gusarapos");
    push_counted(&mut parts, turrets, "torre", "torres");
    push_counted(&mut parts, inhibs, "inhibidor", "inhibidores");
    parts.join("  ")
}

fn push_counted(parts: &mut Vec<String>, n: u32, one: &str, many: &str) {
    if n == 0 {
        return;
    }
    if n == 1 {
        parts.push(format!("1 {one}"));
    } else {
        parts.push(format!("{n} {many}"));
    }
}
