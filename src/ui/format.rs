//! Shared identity-line formatters for player panels and the local strip.
//!
//! Display rounding is presentation-only: the snapshot still holds the
//! exposed f64. No countdowns are derived.

use crate::model::snapshot::{LocalPlayerSnapshot, PlayerSnapshot};

/// Explicit marker for a field the API did not expose.
pub const UNKNOWN: &str = "?";

/// Clock from exposed `gameTime` / `EventTime` seconds: `mm:ss`.
pub fn clock(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return UNKNOWN.to_owned();
    }
    let total = seconds.trunc() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Whole number for gold / HP / CS / MS. Non-finite → absent.
pub fn pretty_int(value: f64) -> String {
    if !value.is_finite() {
        return UNKNOWN.to_owned();
    }
    format!("{}", value.round() as i64)
}

fn pretty_opt_f64(value: Option<f64>) -> String {
    value.map(pretty_int).unwrap_or_else(|| UNKNOWN.to_owned())
}

/// Local strip: `LOCAL {champ} Lv{n}  Gold {g}  HP a/b  mana a/b  QWER`
///
/// Gold only here (ui spec R4). Integers so the line is readable in-game.
pub fn local_line(local: &LocalPlayerSnapshot) -> String {
    let mut line = String::from("LOCAL");
    push_part(&mut line, local.champion.as_deref());

    line.push_str(" Lv");
    line.push_str(
        local
            .level
            .map(|l| l.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    line.push_str("  Gold ");
    line.push_str(&pretty_opt_f64(local.current_gold));

    if let Some(stats) = &local.stats {
        line.push_str("  HP ");
        line.push_str(&pretty_opt_f64(stats.current_health));
        line.push('/');
        line.push_str(&pretty_opt_f64(stats.max_health));
        line.push_str("  mana ");
        line.push_str(&pretty_opt_f64(stats.power));
        line.push('/');
        line.push_str(&pretty_opt_f64(stats.power_max));
    }

    if let Some(abilities) = &local.abilities {
        line.push_str("  ");
        push_rank(&mut line, 'Q', abilities.q);
        line.push(' ');
        push_rank(&mut line, 'W', abilities.w);
        line.push(' ');
        push_rank(&mut line, 'E', abilities.e);
        line.push(' ');
        push_rank(&mut line, 'R', abilities.r);
    }
    line
}

fn push_rank(line: &mut String, letter: char, rank: Option<u32>) {
    line.push(letter);
    match rank {
        Some(level) => line.push_str(&level.to_string()),
        None => line.push_str(UNKNOWN),
    }
}

/// TUI roster line. Tightens spacing and drops spells (never Lv/CS/DEAD)
/// until the card fits the team column — 80×24 halves are ~40 cells.
pub fn player_line_for_width(p: &PlayerSnapshot, column_width: u16) -> String {
    let max = column_width as usize;
    for line in [
        player_card(p, true, true, true, "  "),
        player_card(p, true, true, true, " "),
        player_card(p, true, true, false, "  "),
        player_card(p, true, true, false, " "),
        player_card(p, false, true, false, " "),
        player_card(p, false, false, false, " "),
    ] {
        if max == 0 || line.chars().count() <= max {
            return line;
        }
    }
    let fallback = player_card(p, false, false, false, " ");
    if max == 0 {
        return fallback;
    }
    fallback.chars().take(max).collect()
}

/// Whole seconds (trunc) for exposed respawn timers — never a countdown.
pub fn pretty_secs(value: f64) -> String {
    if !value.is_finite() {
        return UNKNOWN.to_owned();
    }
    format!("{}", value.trunc() as i64)
}

/// `TOP  Mundo  Lv6  1/0/0  CS42  F+H` plus `DEAD 12s` when dead.
pub fn player_line_compact(p: &PlayerSnapshot) -> String {
    player_card(p, true, true, true, "  ")
}

fn player_card(
    p: &PlayerSnapshot,
    show_level: bool,
    show_cs: bool,
    show_spells: bool,
    gap: &str,
) -> String {
    let mut line = String::with_capacity(48);
    if let Some(role) = compact_role(p.position.as_deref()) {
        line.push_str(role);
        line.push_str(gap);
    }
    match p.champion.as_deref() {
        Some(champ) if !champ.is_empty() => line.push_str(champ),
        _ => line.push_str(UNKNOWN),
    }

    if show_level {
        line.push_str(gap);
        line.push_str("Lv");
        line.push_str(&num_marker(p.level));
    }

    line.push_str(gap);
    line.push_str(&num_marker(p.kills));
    line.push('/');
    line.push_str(&num_marker(p.deaths));
    line.push('/');
    line.push_str(&num_marker(p.assists));

    if show_cs {
        line.push_str(gap);
        line.push_str("CS");
        line.push_str(&pretty_opt_f64(p.creep_score));
    }

    if show_spells {
        let s1 = short_spell(p.spell_one.as_deref());
        let s2 = short_spell(p.spell_two.as_deref());
        if s1 != UNKNOWN || s2 != UNKNOWN {
            line.push_str(gap);
            line.push_str(s1);
            line.push('+');
            line.push_str(s2);
        }
    }

    if p.is_dead == Some(true) {
        line.push_str(gap);
        line.push_str("DEAD ");
        match p.respawn_timer {
            Some(timer) if timer.is_finite() => {
                line.push_str(&pretty_secs(timer));
                line.push('s');
            }
            _ => line.push_str(UNKNOWN),
        }
    }
    line
}

/// Dump / headless: same compact card, no item laundry list.
pub fn player_line(p: &PlayerSnapshot) -> String {
    player_line_compact(p)
}

/// Flash/Heal/etc. → one letter so spells fit a 40-cell column.
pub fn short_spell(raw: Option<&str>) -> &'static str {
    let Some(raw) = raw else {
        return UNKNOWN;
    };
    let u = raw.to_ascii_lowercase();
    if u.contains("flash") || u.contains("destello") {
        "F"
    } else if u.contains("heal") || u.contains("curaci") {
        "H"
    } else if u.contains("teleport") || u.contains("teleportaci") {
        "TP"
    } else if u.contains("dot") || u.contains("ignite") || u.contains("ignici") {
        "I"
    } else if u.contains("smite") || u.contains("castigo") {
        "D"
    } else if u.contains("exhaust") || u.contains("agotar") || u.contains("agotamiento") {
        "E"
    } else if u.contains("ghost") || u.contains("fantasmal") {
        "G"
    } else if u.contains("barrier") || u.contains("barrera") {
        "B"
    } else if u.contains("cleanse") || u.contains("aclarar") || u.contains("purificar") {
        "Q"
    } else if u.contains("clarity") {
        "C"
    } else {
        UNKNOWN
    }
}

fn compact_role(raw: Option<&str>) -> Option<&'static str> {
    match raw?.trim().to_ascii_uppercase().as_str() {
        "TOP" => Some("TOP"),
        "JUNGLE" => Some("JGL"),
        "MIDDLE" | "MID" => Some("MID"),
        "BOTTOM" | "BOT" | "ADC" => Some("ADC"),
        "UTILITY" | "SUPPORT" | "SUP" => Some("SUP"),
        _ => None,
    }
}

fn push_part(line: &mut String, part: Option<&str>) {
    if let Some(part) = part {
        if part.is_empty() {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(UNKNOWN);
            return;
        }
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
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| UNKNOWN.to_owned())
}
