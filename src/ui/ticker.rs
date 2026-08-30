//! Objective/kill event ticker fed by the snapshot event list (ui spec R5).
//!
//! Every supported event type renders as one line: `@{EventTime} {TYPE}
//! {participants…}`, with the exposed `EventTime` value VERBATIM (f64
//! display — never recomputed, never counted down; design hard rule).
//! Fields the payload omits degrade to the explicit [`UNKNOWN`] marker.
//! An empty event list renders an explicit empty-state message instead of
//! failing.

use super::dashboard::UNKNOWN;
use super::draw_lines;
use crate::model::snapshot::GameEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Section header introducing the ticker rows.
pub(crate) const EVENTS_HEADER: &str = "EVENTS";

/// Explicit empty-state for a snapshot with zero events (ui spec R5/S2).
pub(crate) const EMPTY_EVENTS: &str = "No match events yet.";

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
        for event in events {
            lines.push(event_line(event));
        }
    }
    lines
}

/// Formats one event line. The exposed time always leads the line so scan
/// tests can pin a row by its verbatim `@{time} {TYPE}` prefix.
fn event_line(event: &GameEvent) -> String {
    let mut line = String::with_capacity(64);
    match event {
        GameEvent::GameStart { time } => {
            push_time(&mut line, *time);
            line.push_str("GameStart");
        }
        GameEvent::MinionsSpawning { time } => {
            push_time(&mut line, *time);
            line.push_str("MinionsSpawning");
        }
        GameEvent::FirstBrick { time } => {
            push_time(&mut line, *time);
            line.push_str("FirstBrick");
        }
        GameEvent::FirstBlood { recipient, time } => {
            push_time(&mut line, *time);
            line.push_str("FirstBlood recipient ");
            push_opt(&mut line, recipient.as_deref());
        }
        GameEvent::ChampionKill {
            killer,
            victim,
            assisters,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("ChampionKill ");
            push_opt(&mut line, killer.as_deref());
            line.push_str(" killed ");
            push_opt(&mut line, victim.as_deref());
            push_assisters(&mut line, assisters);
        }
        GameEvent::Multikill { kill_streak, time } => {
            push_time(&mut line, *time);
            line.push_str("Multikill streak ");
            match kill_streak {
                Some(streak) => line.push_str(&streak.to_string()),
                None => line.push_str(UNKNOWN),
            }
        }
        GameEvent::TurretKilled {
            killer,
            turret,
            assisters,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("TurretKilled ");
            push_opt(&mut line, killer.as_deref());
            line.push_str(" destroyed ");
            push_opt(&mut line, turret.as_deref());
            push_assisters(&mut line, assisters);
        }
        GameEvent::DragonKill {
            dragon_type,
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("DragonKill ");
            push_opt(&mut line, dragon_type.as_deref());
            line.push_str(" slain by ");
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::HeraldKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("HeraldKill slain by ");
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::BaronKill {
            killer,
            stolen,
            time,
        } => {
            push_time(&mut line, *time);
            line.push_str("BaronKill slain by ");
            push_opt(&mut line, killer.as_deref());
            push_stolen(&mut line, *stolen);
        }
        GameEvent::InhibKilled { killer, time } => {
            push_time(&mut line, *time);
            line.push_str("InhibKilled destroyed by ");
            push_opt(&mut line, killer.as_deref());
        }
        GameEvent::Ace { acing_team, time } => {
            push_time(&mut line, *time);
            line.push_str("Ace by ");
            push_opt(&mut line, acing_team.as_deref());
        }
        GameEvent::GameEnd { result, time } => {
            push_time(&mut line, *time);
            line.push_str("GameEnd ");
            push_opt(&mut line, result.as_deref());
        }
        GameEvent::Other { name, time } => {
            push_time(&mut line, *time);
            match name {
                Some(name) => line.push_str(name),
                None => line.push_str("Unknown event"),
            }
        }
    }
    line
}

/// Exposed `EventTime` verbatim (`?` when absent). f64 Display keeps value
/// fidelity exactly like the respawn timers — no rounding, no recompute.
fn push_time(line: &mut String, time: Option<f64>) {
    line.push('@');
    match time {
        Some(time) => line.push_str(&time.to_string()),
        None => line.push_str(UNKNOWN),
    }
    line.push(' ');
}

fn push_opt(line: &mut String, value: Option<&str>) {
    line.push_str(value.unwrap_or(UNKNOWN));
}

/// Assister list suffix, only when participants were actually exposed.
fn push_assisters(line: &mut String, assisters: &[String]) {
    if assisters.is_empty() {
        return;
    }
    line.push_str(" (assists: ");
    line.push_str(&assisters.join(", "));
    line.push(')');
}

/// Stolen flag suffix — printed only on an explicit `true`; absence and
/// `false` stay silent rather than inventing a negation.
fn push_stolen(line: &mut String, stolen: Option<bool>) {
    if stolen == Some(true) {
        line.push_str(" STOLEN");
    }
}
