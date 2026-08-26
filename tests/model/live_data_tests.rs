//! Deserialization acceptance criteria for `/allgamedata` payloads
//! (spec `live-client-poller` R4: normalized snapshot deserialization).
//!
//! RED phase of Unit 1: these tests were authored against the fixture
//! corpus BEFORE any model implementation existed.

use tui_lol::model::live_data::parse_all_game_data;

const FULL: &str = include_str!("../fixtures/allgamedata/full.json");
const PARTIAL: &str = include_str!("../fixtures/allgamedata/partial_player.json");
const EMPTY_EVENTS: &str = include_str!("../fixtures/allgamedata/empty_events.json");
const MALFORMED: &str = include_str!("../fixtures/allgamedata/malformed.json");

/// poller:R4/S1 — recorded full payload parses completely:
/// every listed field of every player present with the fixture's values.
#[test]
fn full_fixture_parses_completely() {
    let data = parse_all_game_data(FULL).expect("full fixture must deserialize");

    let players = data.all_players.expect("allPlayers array must be present");
    assert_eq!(players.len(), 10);

    let ahri = &players[2];
    assert_eq!(ahri.champion_name.as_deref(), Some("Ahri"));
    assert_eq!(ahri.summoner_name.as_deref(), Some("MidMage"));
    assert_eq!(ahri.level, Some(12));
    assert_eq!(ahri.team.as_deref(), Some("ORDER"));
    assert_eq!(ahri.position.as_deref(), Some("MIDDLE"));
    assert_eq!(ahri.is_dead, Some(false));
    // Living players expose an explicit zero timer — Some(0.0), not absence.
    assert_eq!(ahri.respawn_timer, Some(0.0));

    let scores = ahri.scores.as_ref().expect("Ahri scores present");
    assert_eq!(scores.kills, Some(6));
    assert_eq!(scores.deaths, Some(1));
    assert_eq!(scores.assists, Some(7));
    assert_eq!(scores.creep_score, Some(195.5));
    assert_eq!(scores.ward_score, Some(0.42));

    let items = ahri.items.as_ref().expect("Ahri items present");
    assert_eq!(items.len(), 7);
    assert_eq!(items[0].item_id, Some(3020));
    assert_eq!(items[0].count, Some(1));
    assert_eq!(items[0].slot, Some(0));
    assert_eq!(items[0].display_name.as_deref(), Some("Sorcerer's Shoes"));
    assert_eq!(items[6].item_id, Some(3364));

    let spells = ahri.summoner_spells.as_ref().expect("Ahri spells present");
    assert_eq!(
        spells
            .summoner_spell_one
            .as_ref()
            .and_then(|s| s.display_name.as_deref()),
        Some("SummonerFlash")
    );
    // Ignite's internal id is SummonerDot, exactly as the live client reports.
    assert_eq!(
        spells
            .summoner_spell_two
            .as_ref()
            .and_then(|s| s.display_name.as_deref()),
        Some("SummonerDot")
    );

    // Dead players expose positive timers verbatim.
    let lee_sin = &players[1];
    assert_eq!(lee_sin.champion_name.as_deref(), Some("Lee Sin"));
    assert_eq!(lee_sin.is_dead, Some(true));
    assert_eq!(lee_sin.respawn_timer, Some(12.5));
    let graves = &players[6];
    assert_eq!(graves.is_dead, Some(true));
    assert_eq!(graves.respawn_timer, Some(34.0));

    let stats = data.game_data.as_ref().expect("gameData present");
    assert_eq!(stats.game_mode.as_deref(), Some("CLASSIC"));
    assert_eq!(stats.game_time, Some(754.19));
    assert_eq!(stats.map_name.as_deref(), Some("Map11"));
    assert_eq!(stats.map_number, Some(1));
    assert_eq!(stats.map_terrain.as_deref(), Some("Default"));

    let local = data.active_player.as_ref().expect("activePlayer present");
    assert_eq!(local.champion_name.as_deref(), Some("Ahri"));
    assert_eq!(local.current_gold, Some(1234.56));
    assert_eq!(local.level, Some(12));
    let st = local.statistics.as_ref().expect("statistics block present");
    assert_eq!(st.max_health, Some(2015.0));
    assert_eq!(st.power_max, Some(1120.0));
    assert_eq!(st.movement_speed, Some(355.0));

    let events = data.events.as_ref().expect("events list present");
    assert_eq!(events.len(), 9);
    assert_eq!(events[0].event_name.as_deref(), Some("GameStart"));
    let dragon = &events[2];
    assert_eq!(dragon.event_name.as_deref(), Some("DragonKill"));
    assert_eq!(
        dragon.extra.get("DragonType").and_then(|v| v.as_str()),
        Some("Chemtech")
    );
    assert_eq!(
        dragon.extra.get("Stolen").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(dragon.event_time, Some(512.18));
}

/// poller:R4/S2 — partial player degrades gracefully: omitted items,
/// respawnTimer, and parts of scores are absent; all other players intact.
#[test]
fn partial_player_degrades_gracefully() {
    let data = parse_all_game_data(PARTIAL).expect("partial fixture must deserialize");

    let players = data.all_players.expect("allPlayers array must be present");
    assert_eq!(players.len(), 10);

    let kaisa = players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("KaiSaFan"))
        .expect("partial player still listed");

    // Omitted keys stay absent — never defaulted to fabricated values.
    assert!(kaisa.items.is_none(), "omitted items must remain absent");
    assert!(
        kaisa.respawn_timer.is_none(),
        "omitted respawnTimer must remain absent"
    );
    let scores = kaisa
        .scores
        .as_ref()
        .expect("scores object itself was kept");
    assert_eq!(scores.kills, Some(4));
    assert_eq!(scores.deaths, Some(5));
    assert!(scores.assists.is_none());
    assert!(scores.creep_score.is_none());
    assert!(scores.ward_score.is_none());

    // All other players arrive intact.
    let jinx = players
        .iter()
        .find(|p| p.summoner_name.as_deref() == Some("ADCarryMain"))
        .expect("other players intact");
    assert_eq!(jinx.champion_name.as_deref(), Some("Jinx"));
    assert_eq!(jinx.level, Some(11));
    assert_eq!(jinx.items.as_ref().map(Vec::len), Some(7));
    assert_eq!(jinx.respawn_timer, Some(0.0));

    // Events and game stats untouched by the partial player.
    assert_eq!(data.events.as_ref().map(|e| e.len()), Some(9));
    assert_eq!(data.game_data.as_ref().unwrap().game_time, Some(754.19));
}

/// Absent == null == omitted at every level: an explicit JSON null must land
/// on the same `None` as a missing key, and an empty payload yields all-None
/// without fabrication.
#[test]
fn null_and_omitted_fields_are_equivalent() {
    let omitted = parse_all_game_data(r#"{"allPlayers":[{"championName":"Ahri"}]}"#)
        .expect("minimal payload deserializes");
    let nulled = parse_all_game_data(
        r#"{"allPlayers":[{"championName":"Ahri","items":null,"respawnTimer":null,"scores":null,"summonerSpells":null,"level":null}]}"#,
    )
    .expect("explicit-null payload deserializes");

    let o = &omitted.all_players.as_ref().unwrap()[0];
    let n = &nulled.all_players.as_ref().unwrap()[0];
    assert_eq!(o.champion_name, n.champion_name);
    assert!(n.items.is_none() && o.items.is_none());
    assert!(n.respawn_timer.is_none() && o.respawn_timer.is_none());
    assert!(n.scores.is_none() && o.scores.is_none());
    assert!(n.summoner_spells.is_none() && o.summoner_spells.is_none());
    assert!(n.level.is_none() && o.level.is_none());

    let empty = parse_all_game_data("{}").expect("empty object deserializes");
    assert!(empty.active_player.is_none());
    assert!(empty.all_players.is_none());
    assert!(empty.events.is_none());
    assert!(empty.game_data.is_none());

    let empty_events = parse_all_game_data(EMPTY_EVENTS).expect("empty_events fixture parses");
    let events = empty_events.events.expect("events key explicitly present");
    assert!(
        events.is_empty(),
        "explicit [] must be an empty list, not None"
    );
}

/// poller:R4/S3 — malformed payload produces a reported parse error;
/// the call returns instead of panicking.
#[test]
fn malformed_json_reports_parse_error_without_panic() {
    let result = parse_all_game_data(MALFORMED);
    let err = result.expect_err("malformed fixture must be rejected");
    assert!(!err.to_string().is_empty());
}
