//! Curated objective/kill ticker fed by the snapshot (ui spec R5).
//!
//! Times are the exposed `EventTime` values shown as `mm:ss` (presentation
//! of the payload clock — never recomputed, never counted down). Newest
//! events lead so a short ticker band still shows what just happened.
//! Display names prefer roster champions; summoner names stay only when
//! the roster cannot map them.

use super::dashboard::UNKNOWN;
use super::draw_lines;
use super::format;
use crate::model::snapshot::{GameEvent, Snapshot};
use ratatui::Frame;
use ratatui::layout::Rect;

/// Section header introducing the ticker rows.
pub(crate) const EVENTS_HEADER: &str = "Sucesos clave";

/// Explicit empty-state for a snapshot with zero events (ui spec R5/S2).
pub(crate) const EMPTY_EVENTS: &str = "sin eventos";

/// On-screen event lines (plus the title row in the 6-row band).
const MAX_EVENT_LINES: usize = 5;

/// Draws the ticker section into its assigned region: the header row, then
/// one line per curated event, clipped at the band boundary.
pub(crate) fn render(snapshot: &Snapshot, area: Rect, frame: &mut Frame) {
    draw_lines(frame, area, &ticker_lines(snapshot));
}

/// Headless ticker text (header + event lines or empty-state).
pub fn dump_events(snapshot: &Snapshot) -> String {
    let mut out = String::new();
    for line in ticker_lines(snapshot) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn ticker_lines(snapshot: &Snapshot) -> Vec<String> {
    let mut lines = vec![EVENTS_HEADER.to_owned()];
    if snapshot.events.is_empty() {
        lines.push(EMPTY_EVENTS.to_owned());
    } else {
        for event in curated_events(snapshot) {
            lines.push(event_line(snapshot, event));
        }
    }
    lines
}

fn curated_events(snapshot: &Snapshot) -> Vec<&GameEvent> {
    let ranked = newest_first(&snapshot.events);
    let mut chosen: Vec<&GameEvent> = Vec::new();

    push_tier(&ranked, &mut chosen, |event| is_objective_or_first(event));
    push_tier(&ranked, &mut chosen, |event| {
        matches!(
            event,
            GameEvent::ChampionKill {
                killer,
                victim,
                assisters,
                ..
            } if local_involved(snapshot, killer.as_deref(), victim.as_deref(), assisters)
        )
    });
    push_tier(&ranked, &mut chosen, |event| {
        matches!(event, GameEvent::ChampionKill { .. })
    });

    if chosen.is_empty() {
        push_tier(&ranked, &mut chosen, |event| {
            matches!(
                event,
                GameEvent::GameStart { .. } | GameEvent::MinionsSpawning { .. }
            )
        });
    }

    if chosen.is_empty() {
        chosen.extend(ranked.iter().copied().take(MAX_EVENT_LINES));
    }

    chosen.sort_by(|a, b| match (a.time(), b.time()) {
        (Some(ta), Some(tb)) => tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    chosen
}

fn push_tier<'a>(
    ranked: &[&'a GameEvent],
    chosen: &mut Vec<&'a GameEvent>,
    keep: impl Fn(&GameEvent) -> bool,
) {
    for event in ranked {
        if chosen.len() >= MAX_EVENT_LINES {
            break;
        }
        if !keep(event) {
            continue;
        }
        if chosen.iter().any(|seen| std::ptr::eq(*seen, *event)) {
            continue;
        }
        chosen.push(*event);
    }
}

fn is_objective_or_first(event: &GameEvent) -> bool {
    match event {
        GameEvent::DragonKill { .. }
        | GameEvent::HeraldKill { .. }
        | GameEvent::BaronKill { .. }
        | GameEvent::HordeKill { .. }
        | GameEvent::TurretKilled { .. }
        | GameEvent::InhibKilled { .. }
        | GameEvent::FirstBlood { .. }
        | GameEvent::FirstBrick { .. }
        | GameEvent::Ace { .. }
        | GameEvent::GameEnd { .. } => true,
        GameEvent::Multikill {
            kill_streak: Some(streak),
            ..
        } if *streak >= 3 => true,
        _ => false,
    }
}

fn local_involved(
    snapshot: &Snapshot,
    killer: Option<&str>,
    victim: Option<&str>,
    assisters: &[String],
) -> bool {
    let names = local_names(snapshot);
    if names.is_empty() {
        return false;
    }
    name_in(&names, killer)
        || name_in(&names, victim)
        || assisters
            .iter()
            .any(|assister| name_in(&names, Some(assister.as_str())))
}

fn local_names(snapshot: &Snapshot) -> Vec<String> {
    let mut names = Vec::new();
    let local_champ = snapshot
        .local
        .as_ref()
        .and_then(|local| local.champion.as_deref())
        .filter(|name| !name.is_empty());
    if let Some(champ) = local_champ {
        names.push(champ.to_owned());
    }
    if let Some(player) = snapshot.players.iter().find(|player| {
        local_champ.is_some_and(|champ| {
            player
                .champion
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(champ))
        })
    }) {
        if let Some(summoner) = player
            .summoner_name
            .as_deref()
            .filter(|name| !name.is_empty())
        {
            if !names.iter().any(|name| name.eq_ignore_ascii_case(summoner)) {
                names.push(summoner.to_owned());
            }
        }
        if let Some(champ) = player.champion.as_deref().filter(|name| !name.is_empty()) {
            if !names.iter().any(|name| name.eq_ignore_ascii_case(champ)) {
                names.push(champ.to_owned());
            }
        }
    }
    names
}

fn name_in(names: &[String], exposed: Option<&str>) -> bool {
    let Some(exposed) = exposed else {
        return false;
    };
    names.iter().any(|name| name.eq_ignore_ascii_case(exposed))
}

fn newest_first(events: &[GameEvent]) -> Vec<&GameEvent> {
    let mut rows: Vec<&GameEvent> = events.iter().collect();
    rows.sort_by(|a, b| match (a.time(), b.time()) {
        (Some(ta), Some(tb)) => tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    rows
}

/// One scan-friendly event line: clock, then a short Spanish label and
/// the participants the payload actually exposed (champions when known).
fn event_line(snapshot: &Snapshot, event: &GameEvent) -> String {
    let mut line = String::with_capacity(64);
    match event {
        GameEvent::GameStart { time } => {
            push_time(&mut line, *time);
            line.push_str("inicio");
        }
        GameEvent::MinionsSpawning { time } => {
            push_time(&mut line, *time);
            line.push_str("subditos");
        }
        GameEvent::FirstBrick { time } => {
            push_time(&mut line, *time);
            line.push_str("primera torre");
        }
        GameEvent::FirstBlood { recipient, time } => {
            push_time(&mut line, *time);
            line.push_str("primera sangre  ");
            push_named(snapshot, &mut line, recipient.as_deref());
        }
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
            time,
        } => {
            push_time(&mut line, *time);
            push_named(snapshot, &mut line, killer.as_deref());
            line.push_str("  mata  ");
            push_named(snapshot, &mut line, victim.as_deref());
            push_assisters(snapshot, &mut line, assisters);
        }
        GameEvent::Multikill { kill_streak, time } => {
            push_time(&mut line, *time);
            line.push_str("racha ");
            match kill_streak {
                Some(streak) => line.push_str(&streak.to_string()),
                None => line.push_str(UNKNOWN),
            }
        }
        GameEvent::TurretKilled {
            killer,
            turret: _,
            assisters,
            time,
        } => {
            push_time(&mut line, *time);
            push_named(snapshot, &mut line, killer.as_deref());
            line.push_str("  torre");
            push_assisters(snapshot, &mut line, assisters);
        }
        GameEvent::DragonKill {
            dragon_type,
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("dragon ");
            push_opt(&mut line, dragon_type.as_deref());
            line.push_str("  ");
            push_named(snapshot, &mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::HeraldKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("heraldo  ");
            push_named(snapshot, &mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::BaronKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("baron  ");
            push_named(snapshot, &mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::InhibKilled { killer, time } => {
            push_time(&mut line, *time);
            line.push_str("inhibidor  ");
            push_named(snapshot, &mut line, killer.as_deref());
        }
        GameEvent::HordeKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("gusarapos  ");
            push_named(snapshot, &mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::Ace { acing_team, time } => {
            push_time(&mut line, *time);
            line.push_str("aniquilacion  ");
            push_opt(&mut line, acing_team.as_deref());
        }
        GameEvent::GameEnd { result, time } => {
            push_time(&mut line, *time);
            line.push_str("fin  ");
            push_opt(&mut line, result.as_deref());
        }
        GameEvent::Other { name, time } => {
            push_time(&mut line, *time);
            line.push_str(&pretty_other(name.as_deref()));
        }
    }
    line
}

fn pretty_other(name: Option<&str>) -> String {
    let Some(name) = name else {
        return "evento".to_owned();
    };
    let lower = name.to_ascii_lowercase();
    if lower.contains("horde") {
        "horda".to_owned()
    } else if lower.contains("inhib") {
        "inhibidor".to_owned()
    } else {
        name.to_owned()
    }
}

/// Exposed `EventTime` as `mm:ss` (`?` when absent).
fn push_time(line: &mut String, time: Option<f64>) {
    match time {
        Some(time) => line.push_str(&format::clock(time)),
        None => line.push_str(UNKNOWN),
    }
    line.push_str("  ");
}

fn push_opt(line: &mut String, value: Option<&str>) {
    line.push_str(value.unwrap_or(UNKNOWN));
}

fn push_named(snapshot: &Snapshot, line: &mut String, raw: Option<&str>) {
    line.push_str(&display_name(snapshot, raw));
}

/// Roster champion when the exposed string matches a summoner or champion;
/// otherwise the payload string (or `?`).
fn display_name(snapshot: &Snapshot, raw: Option<&str>) -> String {
    let Some(name) = raw.filter(|value| !value.is_empty()) else {
        return UNKNOWN.to_owned();
    };
    if let Some(player) = snapshot.players.iter().find(|player| {
        player
            .summoner_name
            .as_deref()
            .is_some_and(|summoner| summoner.eq_ignore_ascii_case(name))
            || player
                .champion
                .as_deref()
                .is_some_and(|champion| champion.eq_ignore_ascii_case(name))
    }) {
        if let Some(champ) = player
            .champion
            .as_deref()
            .filter(|champion| !champion.is_empty())
        {
            return champ.to_owned();
        }
    }
    name.to_owned()
}

/// Assister suffix: one champion, or `+N` when two or more assisted.
fn push_assisters(snapshot: &Snapshot, line: &mut String, assisters: &[String]) {
    if assisters.is_empty() {
        return;
    }
    line.push_str("  +");
    if assisters.len() == 1 {
        line.push_str(&display_name(snapshot, Some(assisters[0].as_str())));
    } else {
        line.push_str(&assisters.len().to_string());
    }
}

/// Stolen flag suffix — printed only on an explicit `true`.
fn push_stolen(line: &mut String, stolen: Option<bool>) {
    if stolen == Some(true) {
        line.push_str("  robado");
    }
}
