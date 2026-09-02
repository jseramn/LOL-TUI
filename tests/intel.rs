//! Decision briefing: cross player, team, and objective data into Spanish
//! sentences without stat acronyms.

use tui_lol::intel;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::{
    GameEvent, GameInfo, LocalPlayerSnapshot, PlayerSnapshot, Snapshot, Team,
};
use tui_lol::ui::scoreboard;

fn snapshot_from_fixture(name: &str) -> Snapshot {
    let raw = std::fs::read_to_string(format!("tests/fixtures/allgamedata/{name}.json"))
        .expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

const BANNED_ACRONYMS: [&str; 11] = [
    "CS", "KP", "KDA", "DRG", "BRN", "TWR", "JGL", "ADC", "SUP", "Lv", "DEAD",
];

fn assert_no_banned_acronyms(text: &str) {
    for banned in BANNED_ACRONYMS {
        assert!(
            !text.split_whitespace().any(|word| word == banned),
            "briefing must not use {banned:?}: {text}"
        );
    }
}

#[test]
fn full_fixture_briefing_crosses_lane_objectives_and_death_window() {
    let lines = intel::briefing(&snapshot_from_fixture("full"));
    assert!(!lines.is_empty());
    assert!(lines.len() <= intel::MAX_LINES);

    let text = lines.join(" ");
    assert!(text.contains("subditos"), "lane farm gap in words: {text}");
    assert!(
        text.contains("CALLE") && text.contains("AHORA") && text.contains("TU"),
        "briefing lines carry scan tags: {text}"
    );
    assert!(
        text.contains("muerto") || text.contains("Muerto") || text.contains("carril"),
        "death window for a decision: {text}"
    );
    assert!(
        !text.contains("lleva 1 dragon") && !text.contains("lleva 1 heraldo"),
        "header already prints objective totals: {text}"
    );
    assert_no_banned_acronyms(&text);
}

#[test]
fn classic_fixture_still_mentions_carril_in_death_pressure() {
    let lines = intel::briefing(&snapshot_from_fixture("full"));
    let text = lines.join(" ");
    assert!(
        text.contains("carril"),
        "Rift death window keeps lane pressure: {text}"
    );
    assert!(
        text.contains("presiona ese carril"),
        "Rift death_pressure wording is pinned: {text}"
    );
    assert!(
        text.contains("Graves") && text.contains("jungla de Caos"),
        "enemy jungler is the pressure target: {text}"
    );
    assert!(
        !text.contains("ventana para presionar"),
        "old untagged wording is gone: {text}"
    );
}

#[test]
fn empty_roster_still_returns_a_sentence() {
    let lines = intel::briefing(&Snapshot::default());
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Todavia no hay cruce"));
}

fn rift_player(
    champion: &str,
    team: Team,
    position: &str,
    kills: u32,
    dead: bool,
    respawn: f64,
) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: Some(champion.to_owned()),
        champion: Some(champion.to_owned()),
        team: Some(team),
        position: Some(position.to_owned()),
        level: Some(16),
        kills: Some(kills),
        deaths: Some(3),
        assists: Some(5),
        creep_score: Some(180.0),
        ward_score: None,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: Some(dead),
        respawn_timer: Some(respawn),
    }
}

#[test]
fn local_dead_yi_waits_to_revive_instead_of_pressing_own_lane() {
    let snapshot = Snapshot {
        players: vec![
            rift_player("Aatrox", Team::Order, "TOP", 4, false, 0.0),
            rift_player("Graves", Team::Order, "JUNGLE", 6, true, 12.0),
            rift_player("Ahri", Team::Order, "MIDDLE", 3, false, 0.0),
            rift_player("Jinx", Team::Order, "BOTTOM", 5, false, 0.0),
            rift_player("Lulu", Team::Order, "UTILITY", 1, false, 0.0),
            rift_player("Darius", Team::Chaos, "TOP", 2, true, 8.0),
            rift_player("Maestro Yi", Team::Chaos, "JUNGLE", 8, true, 6.0),
            rift_player("Syndra", Team::Chaos, "MIDDLE", 4, false, 0.0),
            rift_player("Kai'Sa", Team::Chaos, "BOTTOM", 7, false, 0.0),
            rift_player("Thresh", Team::Chaos, "UTILITY", 0, false, 0.0),
        ],
        local: Some(LocalPlayerSnapshot {
            champion: Some("Maestro Yi".into()),
            level: Some(16),
            current_gold: Some(800.0),
            stats: None,
            abilities: None,
        }),
        game: Some(GameInfo {
            game_mode: Some("CLASSIC".into()),
            game_time: Some(754.0),
            map_name: Some("Map11".into()),
            map_number: Some(11),
            game_id: None,
        }),
        events: vec![],
    };

    let lines = intel::briefing(&snapshot);
    let text = lines.join(" ");
    assert!(
        text.contains("Tu muerto") || (text.contains("espera") && text.contains("revivir")),
        "local death must tell you to wait: {text}"
    );
    assert!(
        !text.contains("presionar ese carril") && !text.contains("presiona ese carril"),
        "never advise pressing your own corpse: {text}"
    );
    assert!(
        !(text.contains("Maestro Yi") && text.contains("presiona")),
        "Yi must not be the pressure target: {text}"
    );
    assert_no_banned_acronyms(&text);
}

fn kiwi_player(
    champion: &str,
    team: Team,
    kills: u32,
    deaths: u32,
    assists: u32,
    dead: bool,
    respawn: f64,
) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: Some(champion.to_owned()),
        champion: Some(champion.to_owned()),
        team: Some(team),
        position: None,
        level: Some(10),
        kills: Some(kills),
        deaths: Some(deaths),
        assists: Some(assists),
        creep_score: Some(10.0),
        ward_score: None,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: Some(dead),
        respawn_timer: Some(respawn),
    }
}

#[test]
fn howling_abyss_briefing_talks_pelea_not_calle() {
    let snapshot = Snapshot {
        players: vec![
            kiwi_player("Aatrox", Team::Order, 5, 8, 10, true, 0.0),
            kiwi_player("Volibear", Team::Order, 2, 6, 8, true, 14.0),
            kiwi_player("Twitch", Team::Order, 5, 9, 11, false, 0.0),
            kiwi_player("Galio", Team::Order, 2, 4, 10, false, 0.0),
            kiwi_player("Kindred", Team::Order, 8, 7, 10, true, 7.0),
            kiwi_player("Mel", Team::Chaos, 14, 3, 12, false, 0.0),
            kiwi_player("Viego", Team::Chaos, 12, 9, 13, true, 17.0),
            kiwi_player("Aphelios", Team::Chaos, 1, 3, 23, false, 0.0),
            kiwi_player("Rumble", Team::Chaos, 3, 1, 20, false, 0.0),
            kiwi_player("Rengar", Team::Chaos, 4, 7, 17, false, 0.0),
        ],
        local: Some(LocalPlayerSnapshot {
            champion: Some("Galio".into()),
            level: Some(10),
            current_gold: Some(446.0),
            stats: None,
            abilities: None,
        }),
        game: Some(GameInfo {
            game_mode: Some("KIWI".into()),
            game_time: Some(554.0),
            map_name: Some("Map12".into()),
            map_number: Some(12),
            game_id: None,
        }),
        events: vec![GameEvent::TurretKilled {
            killer: Some("Mel".into()),
            turret: Some("Turret_T1_C_05".into()),
            assisters: vec![],
            time: Some(400.0),
        }],
    };

    let header = scoreboard::header_line(&snapshot);
    assert!(
        header.contains("Abismo") && !header.contains("KIWI"),
        "header must name the map, not the raw queue: {header}"
    );

    let lines = intel::briefing(&snapshot);
    assert!(lines.len() <= intel::MAX_LINES);
    let text = lines.join(" ");

    assert!(!text.contains("carril"), "Abismo has no lanes: {text}");
    assert!(
        !text.contains("muertes"),
        "participation is not deaths: {text}"
    );
    assert!(
        !text.contains("ritmo de calle"),
        "farm-8 is a Rift laning cue: {text}"
    );
    assert!(
        !text.contains("por debajo de 8"),
        "CS/min 8 does not apply on Abismo: {text}"
    );
    assert!(
        !text.contains("KIWI"),
        "raw mode must not leak into briefing: {text}"
    );
    assert!(
        !text.contains("campeon de"),
        "do not invent a role on Abismo: {text}"
    );
    assert!(
        text.contains("eliminaciones") || text.contains("pelea") || text.contains("participas"),
        "Abismo briefing must talk fight/eliminaciones: {text}"
    );
    assert!(
        text.contains("pelea") && text.contains("2 contra 4"),
        "alive-vs-alive from local Orden: {text}"
    );
    assert!(
        text.contains("Viego (Caos) revive en 17 segundos"),
        "longest remaining enemy death, no invented role: {text}"
    );
    assert!(
        text.contains("participas en 12 de las 22 eliminaciones de Orden."),
        "kill participation in words: {text}"
    );
    assert!(
        !text.contains(".."),
        "participation line must not double-period: {text}"
    );
    assert_no_banned_acronyms(&text);

    let dumped = tui_lol::dump::dump_snapshot(&snapshot);
    assert!(
        dumped.contains("Abismo") && dumped.contains("Orden pelea 2 contra 4"),
        "dump must use the same Abismo briefing: {dumped}"
    );
    assert!(
        !dumped.contains("KIWI") && !dumped.contains("carril"),
        "dump must not leak raw mode or lane advice: {dumped}"
    );
}
