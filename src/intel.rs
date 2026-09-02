//! Game-intelligence briefing: cross player, team, and objective data
//! into short Spanish sentences for in-game decisions.
//!
//! Sources (Riot Live Client Data, `/allgamedata` only):
//! - farm per minute from exposed creepScore and gameTime (laning benchmark
//!   ~8 subditos/minuto for solo lanes on Summoner's Rift — never on Abismo)
//! - kill participation as counts (`eliminaciones` / `participas en`), not a
//!   hidden ratio acronym and never the word `muertes`
//! - lane gaps by pairing the same role on Orden vs Caos (Rift only)
//!
//! Map branching: CLASSIC / multi-lane queues keep carril gaps, farm-8, and
//! an enemy death window ("presiona ese carril"). Howling Abyss (ARAM /
//! KIWI / KINGPORO / map 12) talks about pelea (alive vs alive) and
//! eliminaciones — never calle, farm-8, or carril.
//!
//! Each live line starts with a 5-char ASCII tag (`CALLE`, `AHORA`, `TU`,
//! `MAPA`, `MARCA`). Objective totals stay on the scoreboard header, not
//! repeated in the briefing. Never advise pressing the local player's own
//! death window.
//!
//! Never invents enemy gold, damage share, or unexposed timers.

use crate::model::snapshot::{PlayerSnapshot, Snapshot, Team};
use crate::ui::format::{self, pretty_int, pretty_secs};

/// At most three decision lines — overlays recommend 3–4 live stats, not a wall.
pub const MAX_LINES: usize = 3;

/// Seconds of game time before farm-per-minute is meaningful.
const FARM_RATE_MIN_SECONDS: f64 = 60.0;

/// CS gap worth calling out as a lane swing.
const LANE_CS_GAP: f64 = 15.0;

/// Public briefing for the header band and `--dump`.
pub fn briefing(snapshot: &Snapshot) -> Vec<String> {
    if single_lane(snapshot) {
        briefing_single_lane(snapshot)
    } else {
        briefing_rift(snapshot)
    }
}

fn tagged(tag: &str, body: &str) -> String {
    format!("{tag:<5} {body}")
}

fn briefing_rift(snapshot: &Snapshot) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(line) = biggest_lane_gap(snapshot) {
        lines.push(line);
    }
    if let Some(line) = death_pressure(snapshot) {
        lines.push(line);
    }
    if lines.len() < MAX_LINES {
        if let Some(line) = local_impact(snapshot) {
            lines.push(line);
        }
    }
    finish_briefing(lines)
}

fn briefing_single_lane(snapshot: &Snapshot) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(line) = fight_window(snapshot) {
        lines.push(line);
    }
    if let Some(line) = team_score_gap(snapshot).or_else(|| fed_threat(snapshot)) {
        lines.push(line);
    }
    if lines.len() < MAX_LINES {
        if let Some(line) = local_impact(snapshot) {
            lines.push(line);
        }
    }
    finish_briefing(lines)
}

fn finish_briefing(mut lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        lines.push(
            "Todavia no hay cruce de datos suficiente para un consejo de partida.".to_owned(),
        );
    }
    lines.truncate(MAX_LINES);
    lines
}

fn single_lane(snapshot: &Snapshot) -> bool {
    let game = snapshot.game.as_ref();
    format::is_single_lane(
        game.and_then(|g| g.game_mode.as_deref()),
        game.and_then(|g| g.map_name.as_deref()),
        game.and_then(|g| g.map_number),
    )
}

fn champ(player: &PlayerSnapshot) -> &str {
    player
        .champion
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(format::UNKNOWN)
}

fn role_of(player: &PlayerSnapshot) -> Option<&'static str> {
    format::role_name(player.position.as_deref())
}

fn team_word(team: Team) -> &'static str {
    match team {
        Team::Order => "Orden",
        Team::Chaos => "Caos",
    }
}

fn other_team(team: Team) -> Team {
    match team {
        Team::Order => Team::Chaos,
        Team::Chaos => Team::Order,
    }
}

fn farm_per_minute(creep_score: f64, game_time: Option<f64>) -> Option<f64> {
    let time = game_time?;
    if !creep_score.is_finite() || !time.is_finite() || time < FARM_RATE_MIN_SECONDS {
        return None;
    }
    Some(creep_score / (time / 60.0))
}

fn team_kills(snapshot: &Snapshot, team: Team) -> Option<u32> {
    let members: Vec<_> = snapshot
        .players
        .iter()
        .filter(|player| player.team == Some(team))
        .collect();
    if members.is_empty() || members.iter().all(|player| player.kills.is_none()) {
        return None;
    }
    Some(members.iter().filter_map(|player| player.kills).sum())
}

fn participation(player: &PlayerSnapshot, team_total: u32) -> Option<(u32, u32)> {
    let involved = player.kills? + player.assists?;
    Some((involved, team_total))
}

fn local_roster_player(snapshot: &Snapshot) -> Option<&PlayerSnapshot> {
    let champ_name = snapshot
        .local
        .as_ref()?
        .champion
        .as_deref()
        .filter(|name| !name.is_empty())?;
    snapshot
        .players
        .iter()
        .find(|player| player.champion.as_deref() == Some(champ_name))
}

fn biggest_lane_gap(snapshot: &Snapshot) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    for role in ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"] {
        let Some(order) = snapshot.players.iter().find(|player| {
            player.team == Some(Team::Order)
                && format::role_key(player.position.as_deref()) == Some(role)
        }) else {
            continue;
        };
        let Some(chaos) = snapshot.players.iter().find(|player| {
            player.team == Some(Team::Chaos)
                && format::role_key(player.position.as_deref()) == Some(role)
        }) else {
            continue;
        };
        let Some(cs_a) = order.creep_score.filter(|v| v.is_finite()) else {
            continue;
        };
        let Some(cs_b) = chaos.creep_score.filter(|v| v.is_finite()) else {
            continue;
        };
        let gap = (cs_a - cs_b).abs();
        if gap < LANE_CS_GAP && order.level == chaos.level {
            continue;
        }
        let (ahead, behind, delta) = if cs_a >= cs_b {
            (order, chaos, cs_a - cs_b)
        } else {
            (chaos, order, cs_b - cs_a)
        };
        let role_es = role_of(ahead).unwrap_or("carril");
        let mut line = format!(
            "{} ({role_es}) adelanta a {}: {} subditos",
            champ(ahead),
            champ(behind),
            pretty_int(delta)
        );
        if let (Some(la), Some(lb)) = (ahead.level, behind.level) {
            if la != lb {
                let levels = la.abs_diff(lb);
                line.push_str(&format!(
                    " y {levels} {}",
                    if levels == 1 { "nivel" } else { "niveles" }
                ));
            }
        }
        if let Some(rate) = farm_per_minute(
            ahead.creep_score.unwrap_or(0.0),
            snapshot.game.as_ref().and_then(|g| g.game_time),
        ) {
            line.push_str(&format!(" ({} por minuto)", pretty_int(rate)));
        }
        line.push('.');
        match &best {
            Some((best_gap, _)) if *best_gap >= gap => {}
            _ => best = Some((gap, tagged("CALLE", &line))),
        }
    }
    best.map(|(_, line)| line)
}

fn death_pressure(snapshot: &Snapshot) -> Option<String> {
    if let Some(me) = local_roster_player(snapshot) {
        if me.is_dead == Some(true) {
            let timer = match me.respawn_timer {
                Some(t) if t.is_finite() => format!("{}s", pretty_secs(t)),
                _ => "?s".to_owned(),
            };
            return Some(tagged(
                "AHORA",
                &format!("Tu muerto {timer} - espera a revivir."),
            ));
        }
    }

    let local_team = local_roster_player(snapshot).and_then(|player| player.team);
    let mut enemies: Vec<&PlayerSnapshot> = snapshot
        .players
        .iter()
        .filter(|player| player.is_dead == Some(true))
        .filter(|player| !is_local_player(snapshot, player))
        .filter(|player| match (player.team, local_team) {
            (Some(team), Some(mine)) => team != mine,
            _ => true,
        })
        .collect();
    if enemies.is_empty() {
        return None;
    }
    enemies.sort_by(|a, b| {
        let jung = |player: &PlayerSnapshot| u8::from(role_of(player) == Some("jungla"));
        jung(b)
            .cmp(&jung(a))
            .then_with(|| b.kills.unwrap_or(0).cmp(&a.kills.unwrap_or(0)))
            .then_with(|| timer_millis(b).cmp(&timer_millis(a)))
    });
    let pick = enemies.into_iter().next()?;
    let team = pick.team.map(team_word).unwrap_or("su equipo");
    let role = role_of(pick).unwrap_or("campeon");
    let timer = match pick.respawn_timer {
        Some(t) if t.is_finite() => format!("{}s", pretty_secs(t)),
        _ => "?s".to_owned(),
    };
    Some(tagged(
        "AHORA",
        &format!(
            "{} ({role} de {team}) muerto {timer} - presiona ese carril.",
            champ(pick)
        ),
    ))
}

fn is_local_player(snapshot: &Snapshot, player: &PlayerSnapshot) -> bool {
    if let Some(me) = local_roster_player(snapshot) {
        if std::ptr::eq(me, player) {
            return true;
        }
        if let (Some(a), Some(b)) = (me.summoner_name.as_deref(), player.summoner_name.as_deref()) {
            if a.eq_ignore_ascii_case(b) {
                return true;
            }
        }
        if let (Some(a), Some(b)) = (me.champion.as_deref(), player.champion.as_deref()) {
            if a.eq_ignore_ascii_case(b) {
                return true;
            }
        }
    }
    if let Some(champ_name) = snapshot
        .local
        .as_ref()
        .and_then(|local| local.champion.as_deref())
        .filter(|name| !name.is_empty())
    {
        if player
            .champion
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case(champ_name))
        {
            return true;
        }
    }
    false
}

fn alive_count(snapshot: &Snapshot, team: Team) -> u32 {
    snapshot
        .players
        .iter()
        .filter(|player| player.team == Some(team) && player.is_dead == Some(false))
        .count() as u32
}

fn fight_window(snapshot: &Snapshot) -> Option<String> {
    if !snapshot
        .players
        .iter()
        .any(|player| player.is_dead == Some(true))
    {
        return None;
    }
    let local_team = local_roster_player(snapshot).and_then(|player| player.team);
    let us = local_team.unwrap_or(Team::Order);
    let mut line = format!(
        "{} pelea {} contra {}.",
        team_word(us),
        alive_count(snapshot, us),
        alive_count(snapshot, other_team(us))
    );
    if let Some(pick) = fight_mention(snapshot, local_team) {
        if let Some(timer) = pick.respawn_timer.filter(|t| t.is_finite()) {
            let team = pick.team.map(team_word).unwrap_or("su equipo");
            line.push_str(&format!(
                " {} ({team}) revive en {} segundos.",
                champ(pick),
                pretty_secs(timer)
            ));
        }
    }
    Some(tagged("AHORA", &line))
}

fn fight_mention<'a>(
    snapshot: &'a Snapshot,
    local_team: Option<Team>,
) -> Option<&'a PlayerSnapshot> {
    let dead: Vec<&PlayerSnapshot> = snapshot
        .players
        .iter()
        .filter(|player| player.is_dead == Some(true))
        .filter(|player| match player.respawn_timer {
            Some(t) if t.is_finite() && t > 0.0 => true,
            _ => false,
        })
        .collect();
    if dead.is_empty() {
        return None;
    }
    if let Some(mine) = local_team {
        let mut enemies: Vec<&PlayerSnapshot> = dead
            .iter()
            .copied()
            .filter(|player| player.team == Some(other_team(mine)))
            .collect();
        if !enemies.is_empty() {
            enemies.sort_by_key(|player| std::cmp::Reverse(timer_millis(player)));
            return enemies.into_iter().next();
        }
        return None;
    }
    dead.into_iter()
        .max_by_key(|player| player.kills.unwrap_or(0))
}

fn timer_millis(player: &PlayerSnapshot) -> i64 {
    player
        .respawn_timer
        .filter(|t| t.is_finite())
        .map(|t| (t * 1000.0).round() as i64)
        .unwrap_or(0)
}

fn eliminaciones(n: u32) -> &'static str {
    if n == 1 {
        "eliminacion"
    } else {
        "eliminaciones"
    }
}

fn team_score_gap(snapshot: &Snapshot) -> Option<String> {
    let order = team_kills(snapshot, Team::Order)?;
    let chaos = team_kills(snapshot, Team::Chaos)?;
    if order == chaos {
        return None;
    }
    let (ahead, gap) = if order > chaos {
        ("Orden", order - chaos)
    } else {
        ("Caos", chaos - order)
    };
    Some(tagged(
        "MARCA",
        &format!("{ahead} va {gap} {} por delante.", eliminaciones(gap)),
    ))
}

fn fed_threat(snapshot: &Snapshot) -> Option<String> {
    let local_team = local_roster_player(snapshot).and_then(|player| player.team);
    let local_champ = snapshot
        .local
        .as_ref()
        .and_then(|local| local.champion.as_deref());
    let threat = snapshot
        .players
        .iter()
        .filter(|player| {
            if player.kills.is_none() {
                return false;
            }
            if let Some(mine) = local_team {
                return player.team == Some(other_team(mine));
            }
            local_champ.is_none_or(|champ| player.champion.as_deref() != Some(champ))
        })
        .max_by_key(|player| player.kills.unwrap_or(0))?;
    let kills = threat.kills?;
    if kills == 0 {
        return None;
    }
    let team = threat.team.map(team_word)?;
    Some(tagged(
        "MARCA",
        &format!(
            "{} ({team}) va por delante ({kills} {}).",
            champ(threat),
            eliminaciones(kills)
        ),
    ))
}

fn local_impact(snapshot: &Snapshot) -> Option<String> {
    let local = snapshot.local.as_ref()?;
    let champ_name = local
        .champion
        .as_deref()
        .filter(|name| !name.is_empty())
        .unwrap_or(format::UNKNOWN);
    let me = snapshot
        .players
        .iter()
        .find(|player| player.champion.as_deref() == Some(champ_name))?;
    let team = me.team?;
    let team_total = team_kills(snapshot, team)?;
    let (involved, total) = participation(me, team_total)?;
    let mut line = format!(
        "{champ_name}: participas en {involved} de las {total} eliminaciones de {}",
        team_word(team)
    );
    let mut extra = String::new();
    let aram = single_lane(snapshot);
    if !aram {
        let laner = role_of(me) != Some("soporte") && role_of(me) != Some("jungla");
        if laner {
            if let Some(rate) = me.creep_score.and_then(|cs| {
                farm_per_minute(cs, snapshot.game.as_ref().and_then(|g| g.game_time))
            }) {
                if rate + 0.5 < 8.0 {
                    extra.push_str(&format!(
                        " {} subditos por minuto (por debajo de 8, el ritmo de calle se queda corto)",
                        pretty_int(rate)
                    ));
                }
            }
        }
        if role_of(me) == Some("soporte") {
            if let (Some(mine), Some(theirs)) = (me.ward_score, opposite_ward(snapshot, me)) {
                if mine.is_finite() && theirs.is_finite() && (mine - theirs).abs() >= 5.0 {
                    let verb = if mine >= theirs {
                        "por delante"
                    } else {
                        "por detras"
                    };
                    extra.push_str(&format!(
                        " Vision {verb} del soporte rival: {} frente a {}",
                        pretty_int(mine),
                        pretty_int(theirs)
                    ));
                }
            }
        }
    }
    line.push('.');
    if !extra.is_empty() {
        line.push_str(&extra);
        line.push('.');
    }
    Some(tagged("TU", &line))
}

fn opposite_ward(snapshot: &Snapshot, me: &PlayerSnapshot) -> Option<f64> {
    let my_team = me.team?;
    let role = format::role_key(me.position.as_deref())?;
    snapshot.players.iter().find_map(|player| {
        if player.team == Some(my_team) {
            return None;
        }
        if format::role_key(player.position.as_deref()) != Some(role) {
            return None;
        }
        player.ward_score.filter(|v| v.is_finite())
    })
}
