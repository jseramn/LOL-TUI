//! Loopback HTTP reverse proxy in front of the Live Client Data API.
//!
//! The game client serves HTTPS on 127.0.0.1:2999 with a Riot-pinned
//! certificate. Cloudflare quick tunnels and Tailscale Serve speak HTTP(S)
//! to a public CA, so this process strips that mismatch: it listens on
//! plain HTTP (default `127.0.0.1:18789`) and forwards `/liveclientdata/*`
//! to the pinned [`ApiClient`].
//!
//! Intended for short-lived cloud-dev tunnels, not a permanent public
//! hostname. Local TUI use never needs this.

use crate::api::client::{ApiClient, ENDPOINT_ALL_GAME_DATA, LIVE_CLIENT_PORT, LOOPBACK_HOST};
use crate::api::error::PollError;
use crate::model::live_data::parse_all_game_data;
use crate::model::snapshot::Snapshot;
use crate::ui::scoreboard::header_line;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Binds `listen` and proxies until the process is killed.
///
/// # Errors
/// Bind failures.
pub fn run(listen: &str) -> std::io::Result<()> {
    let client = ApiClient::new(LOOPBACK_HOST, LIVE_CLIENT_PORT)
        .map_err(|err| std::io::Error::other(format!("loopback live-client client: {err}")))?;
    let listener = TcpListener::bind(listen)?;
    let port = listen.rsplit(':').next().unwrap_or("18789");
    eprintln!("tui-lol bridge listening on http://{listen}");
    eprintln!("  cloudflared tunnel --url http://{listen}");
    eprintln!("  tailscale serve --bg {port}");
    eprintln!("GET /identity  — compact live-game summary");
    eprintln!("GET /liveclientdata/*  — proxied to the game client");

    let probe = client.clone();
    thread::Builder::new()
        .name("bridge-identity".into())
        .spawn(move || {
            loop {
                match fetch_identity(&probe) {
                    Ok(text) => eprintln!("{text}"),
                    Err(err) => eprintln!("identity: {err} (in-game only — lobby has no :2999)"),
                }
                thread::sleep(Duration::from_secs(5));
            }
        })
        .expect("identity thread");

    loop {
        let (stream, _) = listener.accept()?;
        let client = client.clone();
        thread::spawn(move || {
            if let Err(err) = handle(stream, &client) {
                eprintln!("bridge connection: {err}");
            }
        });
    }
}

fn fetch_identity(client: &ApiClient) -> Result<String, String> {
    let body = client
        .fetch(ENDPOINT_ALL_GAME_DATA)
        .map_err(|err| err.to_string())?;
    let data = parse_all_game_data(&body).map_err(|err| err.to_string())?;
    let snapshot = Snapshot::from_live(&data);
    let header = header_line(&snapshot);
    let local = snapshot
        .local
        .as_ref()
        .and_then(|l| l.champion.as_deref())
        .unwrap_or("?");
    let id = snapshot
        .game
        .as_ref()
        .and_then(|g| g.game_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "n/a (use LCU lockfile for match id)".into());
    Ok(format!(
        "LIVE id={id} local={local} players={} | {header}",
        snapshot.players.len()
    ))
}

fn handle(mut stream: TcpStream, client: &ApiClient) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    if request_line.is_empty() {
        return Ok(());
    }
    let path = parse_path(&request_line).unwrap_or("/");
    // Drain headers (and ignore a body — we only serve GET).
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        if header.trim().is_empty() {
            break;
        }
    }
    let _ = reader;

    if path == "/health" {
        return write_response(&mut stream, 200, "text/plain", b"ok\n");
    }
    if path == "/identity" || path == "/identity.json" {
        return match identity_json(client) {
            Ok(body) => write_response(&mut stream, 200, "application/json", body.as_bytes()),
            Err(PollError::NotBound(_)) => write_response(
                &mut stream,
                503,
                "application/json",
                br#"{"ok":false,"reason":"not_in_game"}"#,
            ),
            Err(_) => write_response(
                &mut stream,
                502,
                "application/json",
                br#"{"ok":false,"reason":"upstream"}"#,
            ),
        };
    }
    if path.starts_with("/liveclientdata/") {
        return match client.fetch(path) {
            Ok(body) => write_response(&mut stream, 200, "application/json", body.as_bytes()),
            Err(PollError::NotBound(_)) => write_response(
                &mut stream,
                503,
                "application/json",
                br#"{"ok":false,"reason":"not_in_game"}"#,
            ),
            Err(_) => write_response(
                &mut stream,
                502,
                "application/json",
                br#"{"ok":false,"reason":"upstream"}"#,
            ),
        };
    }
    write_response(&mut stream, 404, "text/plain", b"not found\n")
}

fn identity_json(client: &ApiClient) -> Result<String, PollError> {
    let body = client.fetch(ENDPOINT_ALL_GAME_DATA)?;
    let data = parse_all_game_data(&body)
        .map_err(|_| PollError::Transient(crate::api::error::TransientReason::Parse))?;
    let snapshot = Snapshot::from_live(&data);
    let game = snapshot.game.as_ref();
    Ok(format!(
        "{{\
\"ok\":true,\
\"gameId\":{},\
\"gameMode\":{},\
\"gameTime\":{},\
\"mapName\":{},\
\"localChampion\":{},\
\"playerCount\":{},\
\"header\":{}\
}}",
        json_opt_u64(game.and_then(|g| g.game_id)),
        json_opt_str(game.and_then(|g| g.game_mode.as_deref())),
        json_opt_f64(game.and_then(|g| g.game_time)),
        json_opt_str(game.and_then(|g| g.map_name.as_deref())),
        json_opt_str(snapshot.local.as_ref().and_then(|l| l.champion.as_deref())),
        snapshot.players.len(),
        json_str(&header_line(&snapshot)),
    ))
}

fn json_opt_u64(value: Option<u64>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "null".into(),
    }
}

fn json_opt_f64(value: Option<f64>) -> String {
    match value {
        Some(v) => v.to_string(),
        None => "null".into(),
    }
}

fn json_opt_str(value: Option<&str>) -> String {
    match value {
        Some(v) => json_str(v),
        None => "null".into(),
    }
}

fn json_str(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn parse_path(request_line: &str) -> Option<&str> {
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?;
    if method != "GET" && method != "HEAD" {
        return None;
    }
    let target = parts.next()?;
    Some(target.split('?').next().unwrap_or(target))
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "OK",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}
