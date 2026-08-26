//! Tolerant serde DTOs mirroring the Live Client Data API `allgamedata`
//! payload (`https://127.0.0.1:2999/liveclientdata/allgamedata`).
//!
//! Tolerance rules (spec `live-client-poller` R4):
//! - Every field is `Option<T>`: absent, null, and omitted JSON keys are all
//!   represented as [`Option::None`] — never defaulted to a fabricated value.
//! - Unknown JSON keys are ignored.
//! - A malformed payload yields a parse error from [`parse_all_game_data`];
//!   callers retain their last good snapshot and keep polling.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Deserializes an `/allgamedata` response body.
///
/// Malformed JSON is reported as a [`serde_json::Error`] — this is the
/// "Transient(Parse)" input of the poller error taxonomy (Unit 2).
///
/// # Errors
/// Returns the underlying `serde_json::Error` when `json` is not valid JSON
/// or a value has an incompatible type.
pub fn parse_all_game_data(json: &str) -> Result<LiveData, serde_json::Error> {
    serde_json::from_str(json)
}

/// Root of the `allgamedata` aggregate (design D1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveData {
    /// Local player block; absent in some client states (e.g. spectator).
    pub active_player: Option<ActivePlayer>,
    /// The 10 players; `None` when the key is absent or null.
    pub all_players: Option<Vec<PlayerData>>,
    /// Top-level event list. The real client wraps the array in an object
    /// (`{"Events": [...]}`); hand-authored fixtures use the bare array. This
    /// accepts either shape via [`EventsShape`] so the model stays correct
    /// against the real API without breaking existing tests.
    pub events: Option<EventsShape>,
    /// Game-level stats. The real client names this `gameData`; legacy
    /// fixtures named it `gameStats`. Both accepted via `alias`.
    #[serde(rename = "gameData", alias = "gameStats")]
    pub game_data: Option<GameStats>,
}

/// Accepts the Live Client Data API's `{"Events": [...]}` wrapper OR a bare
/// array (legacy/fixture shape). Derefs to the inner `[RawEvent]` so callers
/// can treat it transparently like `&[RawEvent]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventsShape {
    Direct(Vec<RawEvent>),
    Wrapped {
        #[serde(rename = "Events")]
        events: Vec<RawEvent>,
    },
}

impl EventsShape {
    /// Number of events regardless of source shape.
    pub fn len(&self) -> usize {
        match self {
            EventsShape::Direct(v) => v.len(),
            EventsShape::Wrapped { events } => events.len(),
        }
    }

    /// True when neither shape contains any events.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::ops::Deref for EventsShape {
    type Target = [RawEvent];

    fn deref(&self) -> &Self::Target {
        match self {
            EventsShape::Direct(v) => v,
            EventsShape::Wrapped { events } => events,
        }
    }
}

/// Local (active) player: identity plus gold and exposed stat detail.
///
/// Compliance note: `currentGold` exists ONLY here. Downstream code must
/// never attach gold to other players' panels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePlayer {
    pub champion_name: Option<String>,
    /// Exposed only for the local player; rendered verbatim.
    pub current_gold: Option<f64>,
    pub level: Option<u32>,
    /// Exposed stat detail block; unknown stats are ignored.
    pub statistics: Option<Statistics>,
}

/// Numeric stat detail exposed by the client. All optional; extras dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    pub ability_power: Option<f64>,
    pub armor: Option<f64>,
    pub attack_damage: Option<f64>,
    pub attack_range: Option<f64>,
    pub attack_speed: Option<f64>,
    pub crit_chance: Option<f64>,
    pub current_health: Option<f64>,
    pub health_regen_rate: Option<f64>,
    pub lifesteal: Option<f64>,
    pub magic_resist: Option<f64>,
    pub max_health: Option<f64>,
    pub movement_speed: Option<f64>,
    pub omnivamp: Option<f64>,
    pub physical_lethality: Option<f64>,
    pub physical_vamp: Option<f64>,
    pub power: Option<f64>,
    pub power_max: Option<f64>,
    pub power_regen_rate: Option<f64>,
}

/// One player of `allPlayers`.
///
/// Hard rule (design Compliance): `respawnTimer` is carried verbatim as
/// exposed. No code anywhere in this crate may derive a countdown from it
/// or from `gameTime`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerData {
    pub champion_name: Option<String>,
    pub is_bot: Option<bool>,
    pub level: Option<u32>,
    pub is_dead: Option<bool>,
    /// Seconds until respawn as exposed by the client (`0.0` while alive,
    /// positive while dead, absent when unknown). Never computed locally.
    pub respawn_timer: Option<f64>,
    /// Role position as exposed (`TOP`, `JUNGLE`, `MIDDLE`, `BOTTOM`,
    /// `UTILITY`, ...); kept raw.
    pub position: Option<String>,
    pub raw_champion_name: Option<String>,
    pub scores: Option<Scores>,
    pub items: Option<Vec<Item>>,
    pub summoner_spells: Option<SummonerSpells>,
    /// `ORDER` / `CHAOS` as exposed; kept raw for normalization upstream.
    pub team: Option<String>,
    pub riot_id_game_name: Option<String>,
    pub riot_id_tag_line: Option<String>,
    pub summoner_name: Option<String>,
}

/// Kill/participation scores. `creepScore`/`wardScore` are floats in real
/// captures; kills/deaths/assists arrive as integers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scores {
    pub assists: Option<u32>,
    pub creep_score: Option<f64>,
    pub deaths: Option<u32>,
    pub kills: Option<u32>,
    pub ward_score: Option<f64>,
}

/// One inventory slot as exposed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub can_use: Option<bool>,
    pub consumable: Option<bool>,
    pub count: Option<u32>,
    pub display_name: Option<String>,
    #[serde(rename = "itemID")]
    pub item_id: Option<u32>,
    pub price: Option<u32>,
    pub raw_display_name: Option<String>,
    /// 0–5 inventory, 6 trinket.
    pub slot: Option<u8>,
}

/// Pair of summoner spells as exposed (display names carry internal ids,
/// e.g. `SummonerFlash`, `SummonerDot`). Cooldowns are NOT exposed by this
/// API; none are modeled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummonerSpells {
    pub summoner_spell_one: Option<SummonerSpell>,
    pub summoner_spell_two: Option<SummonerSpell>,
}

/// Single summoner spell entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummonerSpell {
    pub display_name: Option<String>,
}

/// Game-level stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStats {
    pub game_mode: Option<String>,
    /// Elapsed game time in seconds, verbatim. Never used to derive timers.
    pub game_time: Option<f64>,
    pub map_name: Option<String>,
    pub map_number: Option<u32>,
    pub map_terrain: Option<String>,
}

/// Untyped-but-shaped event record. Known fields are lifted; every other
/// payload-specific field lands in [`RawEvent::extra`] so normalization can
/// interpret supported event types without losing data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawEvent {
    #[serde(rename = "EventID")]
    pub event_id: Option<u64>,
    #[serde(rename = "EventName")]
    pub event_name: Option<String>,
    #[serde(rename = "EventTime")]
    pub event_time: Option<f64>,
    /// Remaining type-specific fields (Recipient, KillerName, Stolen, ...),
    /// preserved verbatim.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
