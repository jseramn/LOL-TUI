//! Normalized Snapshot: the api↔ui seam (design D5).
//!
//! Converts tolerant [`crate::model::live_data`] DTOs into a stable shape
//! the UI renders directly. Absence is preserved end-to-end:
//! - Missing/null/omitted fields stay `None`.
//! - An absent player list normalizes to an empty roster (list semantics).
//! - An absent event list normalizes to an empty list; an explicit `[]`
//!   stays an empty list.
//!
//! Compliance (design): respawn timers, gold, and event times are carried
//! verbatim from the payload. Nothing here derives a countdown or fabricates
//! values; unknown team strings normalize to absent rather than guessed.

use crate::model::live_data::{ActivePlayer, LiveData, PlayerData, RawEvent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Two-team normalization of the raw `ORDER`/`CHAOS` strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Team {
    Order,
    Chaos,
}

impl Team {
    /// Case-insensitive parse; unknown or absent teams stay absent.
    pub fn from_raw(raw: &str) -> Option<Team> {
        match raw.to_ascii_uppercase().as_str() {
            "ORDER" => Some(Team::Order),
            "CHAOS" => Some(Team::Chaos),
            _ => None,
        }
    }
}

/// Normalized per-player metrics for the dashboard panels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub summoner_name: Option<String>,
    pub champion: Option<String>,
    pub team: Option<Team>,
    /// Raw role string as exposed (`TOP`, `MIDDLE`, ...).
    pub position: Option<String>,
    pub level: Option<u32>,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub creep_score: Option<f64>,
    /// Inventory as exposed; `None` when the key was absent.
    pub items: Option<Vec<ItemSnapshot>>,
    /// Summoner spell display names as exposed (internal ids such as
    /// `SummonerFlash`). Cooldowns are not modeled — the API exposes none.
    pub spell_one: Option<String>,
    pub spell_two: Option<String>,
    pub is_dead: Option<bool>,
    /// Verbatim exposed value (`0.0` alive, positive dead, absent unknown).
    /// Never computed locally.
    pub respawn_timer: Option<f64>,
}

/// One normalized inventory slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSnapshot {
    pub display_name: Option<String>,
    pub item_id: Option<u32>,
    pub count: Option<u32>,
    /// 0–5 inventory, 6 trinket.
    pub slot: Option<u8>,
}

/// Local-player-only detail. Gold lives exclusively here.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LocalPlayerSnapshot {
    pub champion: Option<String>,
    pub level: Option<u32>,
    pub current_gold: Option<f64>,
    pub stats: Option<ActivePlayerStats>,
    /// Ability ranks Q/W/E/R as exposed; `None` when the block is absent.
    pub abilities: Option<AbilityRanks>,
}

/// Exposed ability ranks for the local player. Absence per-slot stays `None`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AbilityRanks {
    pub q: Option<u32>,
    pub w: Option<u32>,
    pub e: Option<u32>,
    pub r: Option<u32>,
}

/// Alias kept explicit at the seam so UI code never imports raw DTO types
/// just to read stat detail.
pub type ActivePlayerStats = crate::model::live_data::Statistics;

/// Game-level info surfaced on the status line / header.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GameInfo {
    pub game_mode: Option<String>,
    pub game_time: Option<f64>,
    pub map_name: Option<String>,
    pub game_id: Option<u64>,
}

/// Supported event taxonomy for the ticker, plus a lossless fallback.
/// Times are the exposed `EventTime` values verbatim — never recomputed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    GameStart {
        time: Option<f64>,
    },
    MinionsSpawning {
        time: Option<f64>,
    },
    FirstBrick {
        time: Option<f64>,
    },
    FirstBlood {
        recipient: Option<String>,
        time: Option<f64>,
    },
    ChampionKill {
        killer: Option<String>,
        victim: Option<String>,
        /// Absent participant lists normalize to empty (rendering-only
        /// semantics; no value fabrication involved).
        assisters: Vec<String>,
        time: Option<f64>,
    },
    Multikill {
        kill_streak: Option<u32>,
        time: Option<f64>,
    },
    TurretKilled {
        killer: Option<String>,
        turret: Option<String>,
        assisters: Vec<String>,
        time: Option<f64>,
    },
    DragonKill {
        dragon_type: Option<String>,
        killer: Option<String>,
        stolen: Option<bool>,
        time: Option<f64>,
    },
    HeraldKill {
        killer: Option<String>,
        stolen: Option<bool>,
        time: Option<f64>,
    },
    BaronKill {
        killer: Option<String>,
        stolen: Option<bool>,
        time: Option<f64>,
    },
    InhibKilled {
        killer: Option<String>,
        time: Option<f64>,
    },
    Ace {
        acing_team: Option<String>,
        time: Option<f64>,
    },
    GameEnd {
        result: Option<String>,
        time: Option<f64>,
    },
    /// Any event type not in the supported set, preserved with its name.
    Other {
        name: Option<String>,
        time: Option<f64>,
    },
}

impl From<&RawEvent> for GameEvent {
    fn from(raw: &RawEvent) -> GameEvent {
        let name = raw.event_name.as_deref();
        let time = raw.event_time;
        match name {
            Some("GameStart") => GameEvent::GameStart { time },
            Some("MinionsSpawning") => GameEvent::MinionsSpawning { time },
            Some("FirstBrick") => GameEvent::FirstBrick { time },
            Some("FirstBlood") => GameEvent::FirstBlood {
                recipient: str_field(&raw.extra, "Recipient"),
                time,
            },
            Some("ChampionKill") => GameEvent::ChampionKill {
                killer: str_field(&raw.extra, "KillerName"),
                victim: str_field(&raw.extra, "VictimName"),
                assisters: str_list(&raw.extra, "Assisters"),
                time,
            },
            Some("Multikill") => GameEvent::Multikill {
                kill_streak: raw.extra.get("KillStreak").and_then(ValueExt::as_u32),
                time,
            },
            Some("TurretKilled") => GameEvent::TurretKilled {
                killer: str_field(&raw.extra, "KillerName"),
                turret: str_field(&raw.extra, "TurretKilled"),
                assisters: str_list(&raw.extra, "Assisters"),
                time,
            },
            Some("DragonKill") => GameEvent::DragonKill {
                dragon_type: str_field(&raw.extra, "DragonType"),
                killer: str_field(&raw.extra, "KillerName"),
                stolen: bool_field(&raw.extra, "Stolen"),
                time,
            },
            Some("HeraldKill") => GameEvent::HeraldKill {
                killer: str_field(&raw.extra, "KillerName"),
                stolen: bool_field(&raw.extra, "Stolen"),
                time,
            },
            Some("BaronKill") => GameEvent::BaronKill {
                killer: str_field(&raw.extra, "KillerName"),
                stolen: bool_field(&raw.extra, "Stolen"),
                time,
            },
            Some("InhibKilled") => GameEvent::InhibKilled {
                killer: str_field(&raw.extra, "KillerName"),
                time,
            },
            Some("Ace") => GameEvent::Ace {
                acing_team: str_field(&raw.extra, "AcingTeam"),
                time,
            },
            Some("GameEnd") => GameEvent::GameEnd {
                result: str_field(&raw.extra, "Result"),
                time,
            },
            other => GameEvent::Other {
                name: other.map(str::to_owned),
                time,
            },
        }
    }
}

trait ValueExt {
    fn as_u32(&self) -> Option<u32>;
}

impl ValueExt for serde_json::Value {
    fn as_u32(&self) -> Option<u32> {
        self.as_u64().and_then(|v| u32::try_from(v).ok())
    }
}

fn str_field(extra: &BTreeMap<String, serde_json::Value>, key: &str) -> Option<String> {
    extra.get(key).and_then(|v| v.as_str()).map(str::to_owned)
}

fn bool_field(extra: &BTreeMap<String, serde_json::Value>, key: &str) -> Option<bool> {
    extra.get(key).and_then(serde_json::Value::as_bool)
}

/// Absent lists normalize to empty; non-string entries are skipped.
fn str_list(extra: &BTreeMap<String, serde_json::Value>, key: &str) -> Vec<String> {
    extra
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Normalized view of one `/allgamedata` sample.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// Empty when `allPlayers` was absent or null.
    pub players: Vec<PlayerSnapshot>,
    /// Local-player detail; `None` when `activePlayer` was absent or null.
    pub local: Option<LocalPlayerSnapshot>,
    /// `None` when `gameStats` was absent or null.
    pub game: Option<GameInfo>,
    /// Empty when the event list was absent; verbatim order preserved.
    pub events: Vec<GameEvent>,
}

impl Snapshot {
    /// Normalizes a parsed payload. Pure: no I/O, no clocks, no derivation.
    pub fn from_live(data: &LiveData) -> Snapshot {
        Snapshot {
            players: data
                .all_players
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(PlayerSnapshot::from_player)
                .collect(),
            local: data
                .active_player
                .as_ref()
                .map(LocalPlayerSnapshot::from_local),
            game: data.game_data.as_ref().map(|g| GameInfo {
                game_mode: g.game_mode.clone(),
                game_time: g.game_time,
                map_name: g.map_name.clone(),
                game_id: g.game_id,
            }),
            events: data
                .events
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(GameEvent::from)
                .collect(),
        }
    }
}

impl PlayerSnapshot {
    fn from_player(p: &PlayerData) -> PlayerSnapshot {
        let scores = p.scores.as_ref();
        PlayerSnapshot {
            summoner_name: p.summoner_name.clone(),
            champion: p.champion_name.clone(),
            team: p.team.as_deref().and_then(Team::from_raw),
            position: p.position.clone(),
            level: p.level,
            kills: scores.and_then(|s| s.kills),
            deaths: scores.and_then(|s| s.deaths),
            assists: scores.and_then(|s| s.assists),
            creep_score: scores.and_then(|s| s.creep_score),
            items: p.items.as_ref().map(|items| {
                items
                    .iter()
                    .map(|i| ItemSnapshot {
                        display_name: i.display_name.clone(),
                        item_id: i.item_id,
                        count: i.count,
                        slot: i.slot,
                    })
                    .collect()
            }),
            spell_one: p
                .summoner_spells
                .as_ref()
                .and_then(|s| s.summoner_spell_one.as_ref())
                .and_then(|s| s.display_name.clone()),
            spell_two: p
                .summoner_spells
                .as_ref()
                .and_then(|s| s.summoner_spell_two.as_ref())
                .and_then(|s| s.display_name.clone()),
            is_dead: p.is_dead,
            respawn_timer: p.respawn_timer,
        }
    }
}

impl LocalPlayerSnapshot {
    fn from_local(a: &ActivePlayer) -> LocalPlayerSnapshot {
        LocalPlayerSnapshot {
            champion: a.champion_name.clone(),
            level: a.level,
            current_gold: a.current_gold,
            stats: a.statistics.clone(),
            abilities: a.abilities.as_ref().map(|abilities| AbilityRanks {
                q: abilities.q.as_ref().and_then(|ab| ab.ability_level),
                w: abilities.w.as_ref().and_then(|ab| ab.ability_level),
                e: abilities.e.as_ref().and_then(|ab| ab.ability_level),
                r: abilities.r.as_ref().and_then(|ab| ab.ability_level),
            }),
        }
    }
}
