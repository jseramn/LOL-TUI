//! Shared identity-line formatters for player panels and the local strip.
//!
//! Display rounding is presentation-only: the snapshot still holds the
//! exposed f64. No countdowns are derived. Visible copy uses full Spanish
//! words — no role/stat acronyms.

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

/// Whole number for gold / health / farm. Non-finite → absent.
pub fn pretty_int(value: f64) -> String {
    if !value.is_finite() {
        return UNKNOWN.to_owned();
    }
    format!("{}", value.round() as i64)
}

fn pretty_opt_f64(value: Option<f64>) -> String {
    value.map(pretty_int).unwrap_or_else(|| UNKNOWN.to_owned())
}

/// Canonical role key used to pair the same lane on both teams.
pub fn role_key(raw: Option<&str>) -> Option<&'static str> {
    match raw?.trim().to_ascii_uppercase().as_str() {
        "TOP" => Some("TOP"),
        "JUNGLE" => Some("JUNGLE"),
        "MIDDLE" | "MID" => Some("MIDDLE"),
        "BOTTOM" | "BOT" | "ADC" => Some("BOTTOM"),
        "UTILITY" | "SUPPORT" | "SUP" => Some("UTILITY"),
        _ => None,
    }
}

/// Lowercase Spanish role for sentences (`jungla`, `central`, …).
pub fn role_name(raw: Option<&str>) -> Option<&'static str> {
    match role_key(raw)? {
        "TOP" => Some("superior"),
        "JUNGLE" => Some("jungla"),
        "MIDDLE" => Some("central"),
        "BOTTOM" => Some("tirador"),
        "UTILITY" => Some("soporte"),
        _ => None,
    }
}

/// Capitalized Spanish role for roster cards.
pub fn role_label(raw: Option<&str>) -> Option<&'static str> {
    match role_key(raw)? {
        "TOP" => Some("Superior"),
        "JUNGLE" => Some("Jungla"),
        "MIDDLE" => Some("Central"),
        "BOTTOM" => Some("Tirador"),
        "UTILITY" => Some("Soporte"),
        _ => None,
    }
}

/// Queue / map mode in Spanish. Unknown values stay as the client sent them.
pub fn game_mode_name(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "CLASSIC" => "Clasica".to_owned(),
        "ARAM" | "KIWI" | "KINGPORO" => "Abismo".to_owned(),
        "URF" => "Ultra rapido".to_owned(),
        "ONEFORALL" => "Uno para todos".to_owned(),
        "NEXUSBLITZ" => "Asalto al nexo".to_owned(),
        "PRACTICETOOL" => "Herramienta de practica".to_owned(),
        "TUTORIAL" => "Tutorial".to_owned(),
        "CHERRY" => "Arena".to_owned(),
        "SWIFTPLAY" => "Rapida".to_owned(),
        _ => raw.to_owned(),
    }
}

/// Howling Abyss / single-lane queues: no calle, no farm-8, no roles TOP/JNG.
pub fn is_single_lane(
    game_mode: Option<&str>,
    map_name: Option<&str>,
    map_number: Option<u32>,
) -> bool {
    if let Some(mode) = game_mode {
        match mode.trim().to_ascii_uppercase().as_str() {
            "ARAM" | "KIWI" | "KINGPORO" | "ASCENSION" => return true,
            _ => {}
        }
    }
    if map_number == Some(12) {
        return true;
    }
    if let Some(name) = map_name {
        let upper = name.trim().to_ascii_uppercase();
        if upper == "MAP12" || upper.contains("HOWLING") || upper.contains("ABYSS") {
            return true;
        }
    }
    false
}

/// Local strip: `Tu {champ}  nivel {n}  oro {g}  QWER`
///
/// Gold only here (ui spec R4). Integers so the line is readable in-game.
/// HP / mana numbers live on the gauges, not this line. Q/W/E/R are the
/// ability keys on the keyboard, not stat acronyms.
pub fn local_line(local: &LocalPlayerSnapshot) -> String {
    let mut line = String::from("Tu");
    push_part(&mut line, local.champion.as_deref());

    line.push_str("  nivel ");
    line.push_str(
        local
            .level
            .map(|l| l.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    line.push_str("  oro ");
    line.push_str(&pretty_opt_f64(local.current_gold));

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

/// TUI roster line. Tightens spacing and drops combat score first so the
/// death window still fits a ~40-cell team column.
pub fn player_line_for_width(p: &PlayerSnapshot, column_width: u16) -> String {
    let max = column_width as usize;
    let numbered = |line: String| line.replace(" subditos", "");
    for line in [
        player_card(p, true, true, true, true, "  "),
        player_card(p, true, true, true, true, " "),
        numbered(player_card(p, true, true, true, true, " ")),
        player_card(p, true, true, false, true, " "),
        numbered(player_card(p, true, true, false, true, " ")),
        player_card(p, false, true, true, true, " "),
        numbered(player_card(p, false, true, true, true, " ")),
        player_card(p, false, true, false, true, " "),
        numbered(player_card(p, false, true, false, true, " ")),
        player_card(p, true, false, false, true, " "),
        player_card(p, false, false, false, false, " "),
    ] {
        if max == 0 || line.chars().count() <= max {
            return line;
        }
    }
    let fallback = player_card(p, false, false, false, false, " ");
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

/// Full roster card in Spanish (dump / wide columns).
pub fn player_line_compact(p: &PlayerSnapshot) -> String {
    player_card(p, true, true, true, true, "  ")
}

fn player_card(
    p: &PlayerSnapshot,
    show_role: bool,
    show_level: bool,
    show_kda: bool,
    show_cs: bool,
    gap: &str,
) -> String {
    let mut line = String::with_capacity(64);
    if show_role {
        if let Some(role) = role_label(p.position.as_deref()) {
            line.push_str(role);
            line.push_str(gap);
        }
    }
    match p.champion.as_deref() {
        Some(champ) if !champ.is_empty() => line.push_str(champ),
        _ => line.push_str(UNKNOWN),
    }

    if show_level {
        line.push_str(gap);
        line.push_str("nivel ");
        line.push_str(&num_marker(p.level));
    }

    if show_kda {
        line.push_str(gap);
        line.push_str(&num_marker(p.kills));
        line.push('/');
        line.push_str(&num_marker(p.deaths));
        line.push('/');
        line.push_str(&num_marker(p.assists));
    }

    if show_cs {
        line.push_str(gap);
        line.push_str(&pretty_opt_f64(p.creep_score));
        line.push_str(" subditos");
    }

    if p.is_dead == Some(true) {
        line.push_str(gap);
        line.push_str("Muerto");
        match p.respawn_timer {
            Some(timer) if timer.is_finite() => {
                line.push(' ');
                line.push_str(&pretty_secs(timer));
                line.push('s');
            }
            _ => {
                line.push(' ');
                line.push_str(UNKNOWN);
            }
        }
    }
    line
}

/// Dump / headless: same full card, no item laundry list.
pub fn player_line(p: &PlayerSnapshot) -> String {
    player_line_compact(p)
}

/// Flash/Heal/etc. → one letter. Kept for tests; roster cards no longer
/// print summoner-spell abbreviations.
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
