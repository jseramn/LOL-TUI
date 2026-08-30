//! Objective/kill event ticker fed by the snapshot event list (ui spec R5).
//!
//! Times are the exposed `EventTime` values shown as `mm:ss` (presentation
//! of the payload clock — never recomputed, never counted down). Newest
//! events lead so a short ticker band still shows what just happened.

use super::dashboard::UNKNOWN;
use super::draw_lines;
use super::format;
use crate::model::snapshot::GameEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Section header introducing the ticker rows.
pub(crate) const EVENTS_HEADER: &str = "EVENTS";

/// Explicit empty-state for a snapshot with zero events (ui spec R5/S2).
pub(crate) const EMPTY_EVENTS: &str = "sin eventos";

/// Draws the ticker section into its assigned region: the header row, then
/// one line per event, clipped at the band boundary.
pub(crate) fn render(events: &[GameEvent], area: Rect, frame: &mut Frame) {
    draw_lines(frame, area, &ticker_lines(events));
}

/// Headless ticker text (header + event lines or empty-state).
pub fn dump_events(events: &[GameEvent]) -> String {
    let mut out = String::new();
    for line in ticker_lines(events) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn ticker_lines(events: &[GameEvent]) -> Vec<String> {
    let mut lines = vec![EVENTS_HEADER.to_owned()];
    if events.is_empty() {
        lines.push(EMPTY_EVENTS.to_owned());
    } else {
        for event in newest_first(events) {
            lines.push(event_line(event));
        }
    }
    lines
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
/// the participants the payload actually exposed.
fn event_line(event: &GameEvent) -> String {
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
            push_opt(&mut line, recipient.as_deref());
        }
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
            time,
        } => {
            push_time(&mut line, *time);
            push_opt(&mut line, killer.as_deref());
            line.push_str("  mata  ");
            push_opt(&mut line, victim.as_deref());
            push_assisters(&mut line, assisters);
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
            push_opt(&mut line, killer.as_deref());
            line.push_str("  torre");
            push_assisters(&mut line, assisters);
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
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::HeraldKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("heraldo  ");
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::BaronKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("baron  ");
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::InhibKilled { killer, time } => {
            push_time(&mut line, *time);
            line.push_str("inhibidor  ");
            push_opt(&mut line, killer.as_deref());
        }
        GameEvent::Ace { acing_team, time } => {
            push_time(&mut line, *time);
            line.push_str("ace  ");
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

/// Assister suffix, only when participants were actually exposed.
fn push_assisters(line: &mut String, assisters: &[String]) {
    if assisters.is_empty() {
        return;
    }
    line.push_str("  +");
    line.push_str(&assisters.join(" +"));
}

/// Stolen flag suffix — printed only on an explicit `true`.
fn push_stolen(line: &mut String, stolen: Option<bool>) {
    if stolen == Some(true) {
        line.push_str("  robado");
    }
}
