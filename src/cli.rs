//! Argument parsing for the binary: TUI (default), `dump`, and `bridge`.
//!
//! No extra crate: the flag surface is small enough for a hand-rolled
//! parser. `--live-url` and `TUI_LOL_LIVE_URL` are the only ways off
//! loopback; production `tui-lol` with no flags stays local.

use crate::api::client::LIVE_URL_ENV;

/// Default HTTP bind for the cloudflared/Tailscale reverse proxy.
pub const DEFAULT_BRIDGE_LISTEN: &str = "127.0.0.1:18789";

/// Parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Interactive dashboard.
    Tui { live_url: Option<String> },
    /// One-shot stdout dump of the current snapshot (no alternate screen).
    Dump {
        live_url: Option<String>,
        /// Local JSON file (fixture or captured payload). When set, the
        /// network is not contacted.
        file: Option<String>,
    },
    /// Interactive TUI fed from a local JSON file (no live client needed).
    Replay { path: String },
    /// HTTP reverse proxy in front of `https://127.0.0.1:2999`.
    Bridge { listen: String },
}

/// Parses argv after the binary name.
///
/// # Errors
/// Unknown flags, missing values, or `--help` (returned as a dedicated
/// error whose message is the help text).
pub fn parse_args<I, S>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    parse_args_with_env(args, std::env::var(LIVE_URL_ENV).ok())
}

/// Like [`parse_args`] with an explicit env-var value (tests inject `None`).
pub fn parse_args_with_env<I, S>(args: I, env_live_url: Option<String>) -> Result<Command, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut live_url = env_live_url
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty());
    let mut listen = DEFAULT_BRIDGE_LISTEN.to_owned();
    let mut positional: Option<String> = None;
    let mut file_arg: Option<String> = None;

    let mut iter = args.into_iter();
    while let Some(raw) = iter.next() {
        let arg = raw.as_ref();
        match arg {
            "-h" | "--help" => return Err(help_text()),
            "--live-url" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "missing value for --live-url".to_owned())?;
                live_url = Some(value.as_ref().to_owned());
            }
            arg if let Some(value) = arg.strip_prefix("--live-url=") => {
                live_url = Some(value.to_owned());
            }
            "--listen" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "missing value for --listen".to_owned())?;
                listen = value.as_ref().to_owned();
            }
            arg if let Some(value) = arg.strip_prefix("--listen=") => {
                listen = value.to_owned();
            }
            "dump" | "bridge" | "tui" | "replay" => {
                if positional.is_some() {
                    return Err(format!("unexpected extra command {arg}"));
                }
                positional = Some(arg.to_owned());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag {other}\n{}", help_text()));
            }
            other => {
                if matches!(positional.as_deref(), Some("dump") | Some("replay"))
                    && file_arg.is_none()
                {
                    file_arg = Some(other.to_owned());
                } else {
                    return Err(format!("unexpected argument {other}\n{}", help_text()));
                }
            }
        }
    }

    match positional.as_deref() {
        None | Some("tui") => Ok(Command::Tui { live_url }),
        Some("dump") => Ok(Command::Dump {
            live_url,
            file: file_arg,
        }),
        Some("replay") => {
            let path = file_arg.ok_or_else(|| {
                format!(
                    "replay requires a JSON path (fixture or capture)\n{}",
                    help_text()
                )
            })?;
            Ok(Command::Replay { path })
        }
        Some("bridge") => Ok(Command::Bridge { listen }),
        Some(other) => Err(format!("unknown command {other}")),
    }
}

/// User-facing help (also the `--help` payload).
pub fn help_text() -> String {
    format!(
        "\
tui-lol — live League of Legends terminal dashboard

Usage:
  tui-lol                         local TUI (https://127.0.0.1:2999)
  tui-lol --live-url URL          TUI against a tunnel origin
  tui-lol replay FILE.json       TUI from a fixture/capture (no LoL needed)
  tui-lol dump [FILE] [--live-url URL]
                                 print one snapshot to stdout
  tui-lol bridge [--listen ADDR] HTTP proxy for cloudflared/tailscale

Options:
  --live-url URL     base URL of a Live Client Data origin
                     (default: env {LIVE_URL_ENV}, else loopback)
  --listen ADDR      bridge bind address (default {DEFAULT_BRIDGE_LISTEN})
  -h, --help         this text

During a live game, expose 2999 through the HTTP bridge then a short-lived
tunnel — see scripts/live_bridge.py — and pass the public URL as --live-url.
"
    )
}
