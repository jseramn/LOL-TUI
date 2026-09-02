//! Presentation helpers: clocks, compact roster cards, short spells.

use tui_lol::model::snapshot::{LocalPlayerSnapshot, PlayerSnapshot};
use tui_lol::ui::format;

#[test]
fn role_helpers_use_spanish_words() {
    assert_eq!(format::role_key(Some("JUNGLE")), Some("JUNGLE"));
    assert_eq!(format::role_name(Some("JUNGLE")), Some("jungla"));
    assert_eq!(format::role_label(Some("UTILITY")), Some("Soporte"));
    assert_eq!(format::game_mode_name("CLASSIC"), "Clasica");
    assert_eq!(format::game_mode_name("ARAM"), "Abismo");
    assert_eq!(format::game_mode_name("KIWI"), "Abismo");
    assert_eq!(format::game_mode_name("KINGPORO"), "Abismo");
    assert!(format::is_single_lane(
        Some("KIWI"),
        Some("Map12"),
        Some(12)
    ));
    assert!(!format::is_single_lane(
        Some("CLASSIC"),
        Some("Map11"),
        Some(11)
    ));
    assert!(format::is_single_lane(Some("ARAM"), None, None));
}

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
    assert_eq!(format::short_spell(Some("Curacion")), "H");
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
        ward_score: Some(0.31),
        items: Some(vec![]),
        spell_one: Some("SummonerFlash".into()),
        spell_two: Some("SummonerTeleport".into()),
        is_dead: Some(false),
        respawn_timer: Some(0.0),
    };
    let line = format::player_line(&player);
    assert_eq!(line, "Superior  Aatrox  nivel 13  5/2/4  212 subditos");
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
        ward_score: Some(0.87),
        items: None,
        spell_one: Some("SummonerFlash".into()),
        spell_two: Some("SummonerSmite".into()),
        is_dead: Some(true),
        respawn_timer: Some(12.5),
    };
    let wide = format::player_line(&player);
    assert!(wide.contains("Muerto 12 segundos"), "full card: {wide}");
    assert!(!wide.contains("F+D"));
    assert!(!wide.contains("DEAD"));

    let narrow = format::player_line_for_width(&player, 40);
    assert!(
        narrow.chars().count() <= 40,
        "must fit a 80x24 team half: {narrow}"
    );
    assert!(
        narrow.contains("Muerto 12"),
        "death tag must not clip: {narrow}"
    );
    assert!(narrow.contains("Lee Sin"));
    assert!(
        narrow.contains("nivel") && narrow.contains("subditos"),
        "canonical column still shows level and farm: {narrow}"
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
    assert!(line.starts_with("Tu Ahri  nivel 12"));
    assert!(line.contains("oro 4350"));
    assert!(!line.contains("4350.0"));
}
