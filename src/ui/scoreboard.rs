//! Match header: mode, exposed clock, team kill totals, objective counts.
//!
//! Kill totals sum *exposed* per-player kill counters (ui spec forbids
//! inventing values; a team whose roster exposes no kills at all shows `?`).
//! Objective counts summarize events already in the snapshot ticker — they
//! do not invent timers or unexposed state.

use crate::model::snapshot::{GameEvent, Snapshot, Team};

/// Single-row live headline for the header band.
pub fn header_line(snapshot: &Snapshot) -> String {
    let mut line = String::from("LIVE");
    if let Some(game) = &snapshot.game {
        if let Some(mode) = &game.game_mode {
            line.push(' ');
            line.push_str(mode);
        }
        if let Some(time) = game.game_time {
            line.push(' ');
            line.push_str(&time.to_string());
            line.push('s');
        }
        if let Some(map) = &game.map_name {
            line.push(' ');
            line.push_str(map);
        }
        if let Some(id) = game.game_id {
            line.push_str(" id ");
            line.push_str(&id.to_string());
        }
    }

    let order = team_kills(snapshot, Team::Order);
    let chaos = team_kills(snapshot, Team::Chaos);
    line.push_str(" | ORDER ");
    line.push_str(&order);
    line.push_str(" - ");
    line.push_str(&chaos);
    line.push_str(" CHAOS");

    let objectives = objectives_summary(snapshot);
    if !objectives.is_empty() {
        line.push_str(" | ");
        line.push_str(&objectives);
    }
    line
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

/// Compact objective readout from the event list (counts only).
pub fn objectives_summary(snapshot: &Snapshot) -> String {
    let mut dragons = 0u32;
    let mut barons = 0u32;
    let mut heralds = 0u32;
    let mut turrets = 0u32;
    let mut inhibs = 0u32;
    for event in &snapshot.events {
        match event {
            GameEvent::DragonKill { .. } => dragons += 1,
            GameEvent::BaronKill { .. } => barons += 1,
            GameEvent::HeraldKill { .. } => heralds += 1,
            GameEvent::TurretKilled { .. } => turrets += 1,
            GameEvent::InhibKilled { .. } => inhibs += 1,
            _ => {}
        }
    }
    let mut parts = Vec::new();
    if dragons > 0 {
        parts.push(format!("DRG {dragons}"));
    }
    if barons > 0 {
        parts.push(format!("BRN {barons}"));
    }
    if heralds > 0 {
        parts.push(format!("HERALD {heralds}"));
    }
    if turrets > 0 {
        parts.push(format!("TWR {turrets}"));
    }
    if inhibs > 0 {
        parts.push(format!("INH {inhibs}"));
    }
    parts.join(" ")
}
