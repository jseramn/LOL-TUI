//! Presentation helpers: clocks, compact roster cards, short spells.

use tui_lol::model::snapshot::{LocalPlayerSnapshot, PlayerSnapshot};
use tui_lol::ui::format;

#[test]
fn clock_renders_mm_ss_from_exposed_seconds() {
    assert_eq!(format::clock(0.046), "00:00");
    assert_eq!(format::clock(15.023), "00:15");
    assert_eq!(format::clock(182.44), "03:02");
    assert_eq!(format::clock(754.19), "12:34");
    assert_eq!(format::clock(-1.0), "?");
}

#[test]
fn pretty_int_rounds_and_pretty_secs_truncates() {
    assert_eq!(format::pretty_int(195.5), "196");
    assert_eq!(format::pretty_int(1234.56), "1235");
    assert_eq!(format::pretty_secs(12.5), "12");
    assert_eq!(format::pretty_secs(34.0), "34");
}

#[test]
fn short_spell_maps_english_and_spanish_names() {
    assert_eq!(format::short_spell(Some("SummonerFlash")), "F");
    assert_eq!(format::short_spell(Some("Destello")), "F");
    assert_eq!(format::short_spell(Some("Curación")), "H");
    assert_eq!(format::short_spell(Some("SummonerDot")), "I");
    assert_eq!(format::short_spell(Some("SummonerTeleport")), "TP");
    assert_eq!(format::short_spell(Some("Castigo")), "D");
    assert_eq!(format::short_spell(None), "?");
}

#[test]
fn compact_player_line_skips_item_laundry_lists() {
    let player = PlayerSnapshot {
        summoner_name: Some("TopLaneTitan".into()),
        champion: Some("Aatrox".into()),
        team: None,
        position: Some("TOP".into()),
        level: Some(13),
        kills: Some(5),
        deaths: Some(2),
        assists: Some(4),
        creep_score: Some(212.0),
        items: Some(vec![]),
        spell_one: Some("SummonerFlash".into()),
        spell_two: Some("SummonerTeleport".into()),
        is_dead: Some(false),
        respawn_timer: Some(0.0),
    };
    let line = format::player_line(&player);
    assert_eq!(line, "TOP  Aatrox  Lv13  5/2/4  CS212  F+TP");
    assert!(!line.contains("Items"));
    assert!(!line.contains("TopLaneTitan"));
}

#[test]
fn narrow_column_keeps_the_dead_tag() {
    let player = PlayerSnapshot {
        summoner_name: Some("JungleKing".into()),
        champion: Some("Lee Sin".into()),
        team: None,
        position: Some("JUNGLE".into()),
        level: Some(12),
        kills: Some(4),
        deaths: Some(3),
        assists: Some(9),
        creep_score: Some(168.0),
        items: None,
        spell_one: Some("SummonerFlash".into()),
        spell_two: Some("SummonerSmite".into()),
        is_dead: Some(true),
        respawn_timer: Some(12.5),
    };
    let wide = format::player_line(&player);
    assert!(wide.contains("DEAD 12s"), "full card: {wide}");
    assert!(wide.contains("F+D"));

    let narrow = format::player_line_for_width(&player, 40);
    assert!(
        narrow.chars().count() <= 40,
        "must fit a 80x24 team half: {narrow}"
    );
    assert!(
        narrow.contains("DEAD 12s"),
        "death tag must not clip: {narrow}"
    );
    assert!(narrow.contains("Lee Sin"));
    assert!(
        narrow.contains("Lv") && narrow.contains("CS"),
        "canonical column still shows level and CS: {narrow}"
    );
}

#[test]
fn local_line_uses_integer_gold_and_the_gold_token() {
    let local = LocalPlayerSnapshot {
        champion: Some("Ahri".into()),
        level: Some(12),
        current_gold: Some(4350.0),
        stats: None,
        abilities: None,
    };
    let line = format::local_line(&local);
    assert!(line.starts_with("LOCAL Ahri Lv12"));
    assert!(line.contains("Gold 4350"));
    assert!(!line.contains("4350.0"));
}
