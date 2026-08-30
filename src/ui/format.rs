//! Shared identity-line formatters for player panels and the local strip.
//!
//! Kept independent of layout so the TUI, the stdout dump, and the HTTP
//! bridge identity endpoint print the same exposed values.

use crate::model::snapshot::{LocalPlayerSnapshot, PlayerSnapshot};

/// Explicit marker for a field the API did not expose. Absence is never
/// rendered as a fabricated value (ui spec: degradation is per field).
pub const UNKNOWN: &str = "?";

/// Formats the distinguished local-player line:
/// `LOCAL {champion} Lv{level} Gold {gold} | HP {cur}/{max} Power {p}/{max} MS {ms} [Q# W# E# R#]`
///
/// Gold is rendered exclusively here — roster panels never carry it
/// (ui spec R4/S2). Values are exposed verbatim; absent ones degrade to
/// [`UNKNOWN`].
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

    line.push_str(" Gold ");
    line.push_str(
        local
            .current_gold
            .map(|g| g.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    if let Some(stats) = &local.stats {
        line.push_str(" | HP ");
        line.push_str(
            stats
                .current_health
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push('/');
        line.push_str(
            stats
                .max_health
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push_str(" Power ");
        line.push_str(
            stats
                .power
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push('/');
        line.push_str(
            stats
                .power_max
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
        line.push_str(" MS ");
        line.push_str(
            stats
                .movement_speed
                .map(|v| v.to_string())
                .as_deref()
                .unwrap_or(UNKNOWN),
        );
    }

    if let Some(abilities) = &local.abilities {
        line.push_str(" | ");
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

/// Column width at or above which roster identity lines keep the full
/// formatter (summoner, spells, items). Narrower team halves (80×24 ⇒
/// ~40 cells each) use [`player_line_compact`] so text is not clipped
/// mid-field.
pub const FULL_PLAYER_LINE_MIN_WIDTH: u16 = 52;

/// Picks the full or compact roster identity formatter for a team column.
pub fn player_line_for_width(p: &PlayerSnapshot, column_width: u16) -> String {
    if column_width >= FULL_PLAYER_LINE_MIN_WIDTH {
        player_line(p)
    } else {
        player_line_compact(p)
    }
}

/// Compact roster identity for side-by-side columns (~40 cells): role,
/// champion, level, K/D/A, CS, and death marker — no summoner name,
/// spells, or item list (those stay on the visualization row / dump).
pub fn player_line_compact(p: &PlayerSnapshot) -> String {
    let mut line = String::with_capacity(48);
    if let Some(role) = compact_role(p.position.as_deref()) {
        line.push_str(role);
    }
    push_part(&mut line, p.champion.as_deref());

    line.push_str(" Lv");
    line.push_str(p.level.map(|l| l.to_string()).as_deref().unwrap_or(UNKNOWN));

    line.push(' ');
    line.push_str(&num_marker(p.kills));
    line.push('/');
    line.push_str(&num_marker(p.deaths));
    line.push('/');
    line.push_str(&num_marker(p.assists));

    line.push_str(" CS");
    line.push_str(
        p.creep_score
            .map(|cs| cs.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    if p.is_dead == Some(true) {
        line.push_str(" DEAD");
        match p.respawn_timer {
            Some(timer) => line.push_str(&timer.to_string()),
            None => line.push_str(UNKNOWN),
        }
    }
    line
}

/// Formats one player panel line:
/// `[{role} ]{name} {champion} Lv{level} {k}/{d}/{a} CS{cs} {spell1}+{spell2}[ DEAD(respawn {t}|?)] | Items: …`
///
/// Values are the exposed snapshot values verbatim; fields the payload
/// omits render as the explicit [`UNKNOWN`] marker — placeholders appear
/// ONLY on absent fields, never on populated ones.
pub fn player_line(p: &PlayerSnapshot) -> String {
    let mut line = String::with_capacity(128);
    if let Some(role) = compact_role(p.position.as_deref()) {
        line.push_str(role);
    }
    push_part(&mut line, p.summoner_name.as_deref());
    push_part(&mut line, p.champion.as_deref());

    if !line.is_empty() {
        line.push(' ');
    }
    line.push_str("Lv");
    line.push_str(p.level.map(|l| l.to_string()).as_deref().unwrap_or(UNKNOWN));

    line.push(' ');
    line.push_str(&num_marker(p.kills));
    line.push('/');
    line.push_str(&num_marker(p.deaths));
    line.push('/');
    line.push_str(&num_marker(p.assists));

    line.push_str(" CS");
    line.push_str(
        p.creep_score
            .map(|cs| cs.to_string())
            .as_deref()
            .unwrap_or(UNKNOWN),
    );

    line.push(' ');
    line.push_str(p.spell_one.as_deref().unwrap_or(UNKNOWN));
    line.push('+');
    line.push_str(p.spell_two.as_deref().unwrap_or(UNKNOWN));

    // Death state: the exposed respawn value, or an unknown marker when the
    // API exposes none. A countdown is NEVER derived locally (design hard
    // rule).
    if p.is_dead == Some(true) {
        match p.respawn_timer {
            Some(timer) => line.push_str(&format!(" DEAD(respawn {timer})")),
            None => line.push_str(&format!(" DEAD(respawn {UNKNOWN})")),
        }
    }

    line.push_str(" | Items: ");
    match &p.items {
        Some(items) => line.push_str(
            &items
                .iter()
                .map(|i| i.display_name.as_deref().unwrap_or(UNKNOWN))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        None => line.push_str(UNKNOWN),
    }
    line
}

/// Compact role label for the identity row. Unknown/empty positions stay
/// off the line rather than becoming another `?` (the rest of the line
/// already marks absence per field).
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
