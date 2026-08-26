//! Parse a captured live payload through our real parse_all_game_data to
//! verify the DTOs match the real schema. Debug aid only.

use std::env;
use std::fs;
use tui_lol::model::live_data::parse_all_game_data;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "../tests/fixtures/allgamedata/captured-live.json".to_string());
    let body = match fs::read_to_string(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("READ FAILED {path}: {e}");
            std::process::exit(2);
        }
    };
    match parse_all_game_data(&body) {
        Ok(d) => {
            println!("PARSE OK");
            println!("  active_player present: {}", d.active_player.is_some());
            println!(
                "  all_players count: {:?}",
                d.all_players.as_ref().map(|v| v.len())
            );
            println!(
                "  events count: {:?}",
                d.events.as_ref().map(|v| v.len())
            );
            println!("  game_data present: {}", d.game_data.is_some());
            if let Some(aps) = &d.all_players {
                if let Some(p0) = aps.first() {
                    println!("  first player summoner_name: {:?}", p0.summoner_name);
                    println!("  first player champion_name: {:?}", p0.champion_name);
                }
            }
        }
        Err(e) => {
            println!("PARSE FAILED: {e}");
            std::process::exit(1);
        }
    }
}