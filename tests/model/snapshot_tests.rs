//! Normalized Snapshot acceptance criteria (task 1.4; spec poller R4/R5).
//!
//! RED phase: authored against the fixture corpus before `snapshot.rs`
//! contained any implementation.

use tui_lol::model::live_data::parse_all_game_data;
use tui_lol::model::snapshot::{GameEvent, Snapshot};

const FULL: &str = include_str!("../fixtures/allgamedata/full.json");
const PARTIAL: &str = include_str!("../fixtures/allgamedata/partial_player.json");
const EMPTY_EVENTS: &str = include_str!("../fixtures/allgamedata/empty_events.json");

fn snapshot_of(json: &str) -> Snapshot {
    let data = parse_all_game_data(json).expect("fixture must deserialize");
    Snapshot::from_live(&data)
}

/// Full fixture normalizes into a complete snapshot: every listed per-player
/// field populated, local player detail, game info, and typed events.
#[test]
fn snapshot_normalizes_full_fixture() {
    let snap = snapshot_of(FULL);

    assert_eq!(snap.players.len(), 10);

    let ahri = snap
        .players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("MidMage"))
        .expect("local player present in roster");
    assert_eq!(ahri.champion.as_deref(), Some("Ahri"));
    assert_eq!(
        ahri.team.map(|t| t as u8),
        Some(tui_lol::model::snapshot::Team::Order as u8)
    );
    assert_eq!(ahri.position.as_deref(), Some("MIDDLE"));
    assert_eq!(ahri.level, Some(12));
    assert_eq!(ahri.kills, Some(6));
    assert_eq!(ahri.deaths, Some(1));
    assert_eq!(ahri.assists, Some(7));
    assert_eq!(ahri.creep_score, Some(195.5));
    assert_eq!(ahri.ward_score, Some(0.42));
    let items = ahri.items.as_ref().expect("items normalized");
    assert_eq!(items.len(), 7);
    assert_eq!(items[0].item_id, Some(3020));
    assert_eq!(items[0].slot, Some(0));
    assert_eq!(ahri.spell_one.as_deref(), Some("SummonerFlash"));
    assert_eq!(ahri.spell_two.as_deref(), Some("SummonerDot"));
    assert_eq!(ahri.is_dead, Some(false));
    assert_eq!(ahri.respawn_timer, Some(0.0));

    let lee = snap
        .players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("JungleKing"))
        .unwrap();
    assert_eq!(lee.is_dead, Some(true));
    assert_eq!(lee.respawn_timer, Some(12.5));
    let graves = snap
        .players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("GravesMain"))
        .unwrap();
    assert_eq!(graves.respawn_timer, Some(34.0));

    let game = snap.game.as_ref().expect("game info normalized");
    assert_eq!(game.game_mode.as_deref(), Some("CLASSIC"));
    assert_eq!(game.game_time, Some(754.19));
    assert_eq!(game.map_name.as_deref(), Some("Map11"));

    let local = snap.local.as_ref().expect("local player normalized");
    assert_eq!(local.champion.as_deref(), Some("Ahri"));
    assert_eq!(local.current_gold, Some(1234.56));
    let stats = local.stats.as_ref().expect("stat detail normalized");
    assert_eq!(stats.max_health, Some(2015.0));
    let ranks = local.abilities.as_ref().expect("ability ranks normalized");
    assert_eq!(ranks.q, Some(4));
    assert_eq!(ranks.w, Some(2));
    assert_eq!(ranks.e, Some(3));
    assert_eq!(ranks.r, Some(1));

    let dumped = tui_lol::dump::dump_snapshot(&snap);
    assert!(dumped.contains("En partida  12:34  Clasica"));
    assert!(dumped.contains("Orden 23 - 20 Caos"));
    assert!(dumped.contains("subditos"));
    assert!(dumped.contains("Aatrox"));
    assert!(dumped.contains("Q4 W2 E3 R1"));

    assert_eq!(snap.events.len(), 9);
    let has_first_blood = snap.events.iter().any(|e| {
        matches!(
            e,
            GameEvent::FirstBlood { recipient: Some(r), .. } if r == "Order"
        )
    });
    assert!(has_first_blood, "FirstBlood with Recipient expected");
    let has_dragon = snap.events.iter().any(|e| matches!(
        e,
        GameEvent::DragonKill { dragon_type: Some(dt), stolen: Some(true), .. } if dt == "Chemtech"
    ));
    assert!(has_dragon, "stolen Chemtech DragonKill expected");
    let has_turret = snap.events.iter().any(|e| {
        matches!(
            e,
            GameEvent::TurretKilled { turret: Some(t), assisters, .. }
                if t == "Turret_T1_C_03" && assisters.len() == 2
        )
    });
    assert!(has_turret, "TurretKilled with two assisters expected");
}

/// Partial player keeps absence through normalization while everyone else
/// stays intact (poller:R4/S2 end-to-end).
#[test]
fn snapshot_preserves_absence_from_partial_fixture() {
    let snap = snapshot_of(PARTIAL);

    let kaisa = snap
        .players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("KaiSaFan"))
        .expect("partial player normalized");
    assert!(kaisa.items.is_none());
    assert!(kaisa.respawn_timer.is_none());
    assert_eq!(kaisa.kills, Some(4));
    assert_eq!(kaisa.deaths, Some(5));
    assert!(kaisa.assists.is_none());
    assert!(kaisa.creep_score.is_none());
    assert_eq!(kaisa.is_dead, Some(false));

    let jinx = snap
        .players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("ADCarryMain"))
        .expect("other players intact");
    assert_eq!(jinx.kills, Some(7));
    assert_eq!(jinx.items.as_ref().map(Vec::len), Some(7));
}

/// Explicit empty event list stays an empty list after normalization
/// (poller:R5 offline corpus; feeds ui:R5/S2 empty state later).
#[test]
fn empty_events_fixture_yields_empty_event_list() {
    let snap = snapshot_of(EMPTY_EVENTS);
    assert!(snap.events.is_empty());
    assert_eq!(snap.players.len(), 10);
    assert!(snap.game.is_some());
}

/// A bare `{}` payload normalizes to an all-absent snapshot: zero players,
/// no local player, no game info, no events — nothing fabricated.
#[test]
fn minimal_payload_normalizes_without_fabrication() {
    let data = parse_all_game_data("{}").expect("empty object parses");
    let snap = Snapshot::from_live(&data);
    assert!(snap.players.is_empty());
    assert!(snap.local.is_none());
    assert!(snap.game.is_none());
    assert!(snap.events.is_empty());
}

/// Live `activePlayer.championName` is often empty; fill from the roster
/// when summoner / riot id match. No identity match must not guess.
#[test]
fn local_champion_fills_from_roster_when_active_player_omits_it() {
    let json = r#"{
        "activePlayer": {
            "championName": "",
            "summonerName": "MidMage",
            "currentGold": 10.0,
            "level": 6
        },
        "allPlayers": [
            {"championName": "Ahri", "summonerName": "MidMage", "team": "ORDER"},
            {"championName": "Aatrox", "summonerName": "TopLaneTitan", "team": "ORDER"}
        ]
    }"#;
    let snap = snapshot_of(json);
    assert_eq!(snap.local.unwrap().champion.as_deref(), Some("Ahri"));
}

#[test]
fn local_champion_stays_absent_without_a_roster_match() {
    let json = r#"{
        "activePlayer": {
            "championName": "",
            "summonerName": "Unknown",
            "level": 1
        },
        "allPlayers": [
            {"championName": "Ahri", "summonerName": "MidMage", "team": "ORDER"}
        ]
    }"#;
    let snap = snapshot_of(json);
    let champ = snap.local.unwrap().champion;
    assert!(
        champ.as_deref().is_none() || champ.as_deref() == Some(""),
        "must not invent a champion: {champ:?}"
    );
}

#[test]
fn local_champion_is_not_overwritten_when_already_exposed() {
    let json = r#"{
        "activePlayer": {
            "championName": "Ahri",
            "summonerName": "MidMage"
        },
        "allPlayers": [
            {"championName": "Aatrox", "summonerName": "MidMage", "team": "ORDER"}
        ]
    }"#;
    let snap = snapshot_of(json);
    assert_eq!(snap.local.unwrap().champion.as_deref(), Some("Ahri"));
}
