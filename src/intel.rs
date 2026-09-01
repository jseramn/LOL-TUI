//! Game-intelligence briefing: cross player, team, and objective data
//! into short Spanish sentences for in-game decisions.
//!
//! Sources (Riot Live Client Data, `/allgamedata` only):
//! - farm per minute from exposed creepScore and gameTime (laning benchmark
//!   ~8 subditos/minuto for solo lanes — Dignitas / rft.gg / lolnow.gg)
//! - kill participation as counts, not a hidden ratio acronym
//! - objective control by mapping event killers onto the roster team
//! - lane gaps by pairing the same role on Orden vs Caos
//!
//! Never invents enemy gold, damage share, or unexposed timers.

use crate::model::snapshot::{GameEvent, PlayerSnapshot, Snapshot, Team};
use crate::ui::format::{self, pretty_int, pretty_secs};

/// At most three decision lines — overlays recommend 3–4 live stats, not a wall.
pub const MAX_LINES: usize = 3;

/// Seconds of game time before farm-per-minute is meaningful.
const FARM_RATE_MIN_SECONDS: f64 = 60.0;

/// CS gap worth calling out as a lane swing.
const LANE_CS_GAP: f64 = 15.0;

/// Public briefing for the header band and `--dump`.
pub fn briefing(snapshot: &Snapshot) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(line) = biggest_lane_gap(snapshot) {
        lines.push(line);
    }
    if let Some(line) = objective_control(snapshot) {
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
    if lines.is_empty() {
        lines.push(
            "Todavia no hay cruce de datos suficiente para un consejo de partida.".to_owned(),
        );
    }
    lines.truncate(MAX_LINES);
    lines
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

fn lookup_team(snapshot: &Snapshot, name: Option<&str>) -> Option<Team> {
    let name = name?;
    snapshot.players.iter().find_map(|player| {
        let hit = player
            .summoner_name
            .as_deref()
            .is_some_and(|summoner| summoner.eq_ignore_ascii_case(name))
            || player
                .champion
                .as_deref()
                .is_some_and(|champion| champion.eq_ignore_ascii_case(name));
        if hit { player.team } else { None }
    })
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
            _ => best = Some((gap, line)),
        }
    }
    best.map(|(_, line)| line)
}

fn objective_control(snapshot: &Snapshot) -> Option<String> {
    let mut order_drag = 0u32;
    let mut chaos_drag = 0u32;
    let mut order_herald = 0u32;
    let mut chaos_herald = 0u32;
    let mut order_baron = 0u32;
    let mut chaos_baron = 0u32;
    let mut order_tower = 0u32;
    let mut chaos_tower = 0u32;
    let mut order_horde = 0u32;
    let mut chaos_horde = 0u32;
    let mut any = false;

    let bump = |team: Option<Team>, order: &mut u32, chaos: &mut u32| match team {
        Some(Team::Order) => {
            *order += 1;
            true
        }
        Some(Team::Chaos) => {
            *chaos += 1;
            true
        }
        None => false,
    };

    for event in &snapshot.events {
        match event {
            GameEvent::DragonKill { killer, .. } => {
                any |= bump(
                    lookup_team(snapshot, killer.as_deref()),
                    &mut order_drag,
                    &mut chaos_drag,
                );
            }
            GameEvent::HeraldKill { killer, .. } => {
                any |= bump(
                    lookup_team(snapshot, killer.as_deref()),
                    &mut order_herald,
                    &mut chaos_herald,
                );
            }
            GameEvent::BaronKill { killer, .. } => {
                any |= bump(
                    lookup_team(snapshot, killer.as_deref()),
                    &mut order_baron,
                    &mut chaos_baron,
                );
            }
            GameEvent::TurretKilled { killer, .. } => {
                any |= bump(
                    lookup_team(snapshot, killer.as_deref()),
                    &mut order_tower,
                    &mut chaos_tower,
                );
            }
            GameEvent::HordeKill { killer, .. } => {
                any |= bump(
                    lookup_team(snapshot, killer.as_deref()),
                    &mut order_horde,
                    &mut chaos_horde,
                );
            }
            _ => {}
        }
    }

    if !any && order_drag + chaos_drag + order_tower + chaos_tower == 0 {
        return None;
    }

    let side = |order: u32, chaos: u32, one: &str, many: &str| -> Option<String> {
        if order == 0 && chaos == 0 {
            return None;
        }
        let noun = |n: u32| if n == 1 { one } else { many };
        if order == chaos {
            return Some(format!("empate en {} ({order} cada uno)", noun(order)));
        }
        let (team, n) = if order > chaos {
            ("Orden", order)
        } else {
            ("Caos", chaos)
        };
        Some(format!("{team} lleva {n} {}", noun(n)))
    };

    let mut parts = Vec::new();
    if let Some(part) = side(order_drag, chaos_drag, "dragon", "dragones") {
        parts.push(part);
    }
    if let Some(part) = side(order_herald, chaos_herald, "heraldo", "heraldos") {
        parts.push(part);
    }
    if let Some(part) = side(order_baron, chaos_baron, "baron", "barones") {
        parts.push(part);
    }
    if let Some(part) = side(order_tower, chaos_tower, "torre", "torres") {
        parts.push(part);
    }
    if let Some(part) = side(order_horde, chaos_horde, "gusarapo", "gusarapos") {
        parts.push(part);
    }
    if parts.is_empty() {
        return None;
    }
    let mut line = parts.join(". ");
    line.push('.');
    Some(line)
}

fn death_pressure(snapshot: &Snapshot) -> Option<String> {
    let dead: Vec<&PlayerSnapshot> = snapshot
        .players
        .iter()
        .filter(|player| player.is_dead == Some(true))
        .collect();
    if dead.is_empty() {
        return None;
    }
    let pick = dead.iter().max_by_key(|player| {
        let jungle = u32::from(role_of(player) == Some("jungla"));
        let kills = player.kills.unwrap_or(0);
        jungle * 10 + kills
    })?;
    let team = pick.team.map(team_word).unwrap_or("su equipo");
    let role = role_of(pick).unwrap_or("campeon");
    let timer = match pick.respawn_timer {
        Some(t) if t.is_finite() => format!("{} segundos", pretty_secs(t)),
        _ => "tiempo desconocido".to_owned(),
    };
    Some(format!(
        "{} ({role} de {team}) muerto {timer}: ventana para presionar ese carril.",
        champ(pick)
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
        "Tu ({champ_name}): {involved} de {total} muertes de {}.",
        team_word(team)
    );
    if let Some(rate) = farm_per_minute(
        me.creep_score?,
        snapshot.game.as_ref().and_then(|g| g.game_time),
    ) {
        line.push_str(&format!(" {} subditos por minuto", pretty_int(rate)));
        if rate + 0.5 < 8.0 && role_of(me) != Some("soporte") && role_of(me) != Some("jungla") {
            line.push_str(" (por debajo de 8, el ritmo de calle se queda corto)");
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
                line.push_str(&format!(
                    " Vision {verb} del soporte rival: {} frente a {}",
                    pretty_int(mine),
                    pretty_int(theirs)
                ));
            }
        }
    }
    line.push('.');
    Some(line)
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
