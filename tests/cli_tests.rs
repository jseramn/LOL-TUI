//! CLI flag parsing for the local-default / tunnel-override surface.

use tui_lol::cli::{self, Command, DEFAULT_BRIDGE_LISTEN};

#[test]
fn no_args_is_local_tui() {
    assert_eq!(
        cli::parse_args_with_env(Vec::<String>::new(), None).unwrap(),
        Command::Tui { live_url: None }
    );
}

#[test]
fn dump_accepts_live_url_flag() {
    let cmd = cli::parse_args_with_env(
        ["dump", "--live-url", "https://abc.trycloudflare.com"],
        None,
    )
    .unwrap();
    assert_eq!(
        cmd,
        Command::Dump {
            live_url: Some("https://abc.trycloudflare.com".into()),
            file: None,
        }
    );
}

#[test]
fn live_url_env_fills_tui_when_flag_omitted() {
    let cmd = cli::parse_args_with_env(
        Vec::<String>::new(),
        Some("https://abc.trycloudflare.com".into()),
    )
    .unwrap();
    assert_eq!(
        cmd,
        Command::Tui {
            live_url: Some("https://abc.trycloudflare.com".into())
        }
    );
}

#[test]
fn flag_overrides_env_live_url() {
    let cmd = cli::parse_args_with_env(
        ["--live-url", "https://flag.example"],
        Some("https://env.example".into()),
    )
    .unwrap();
    assert_eq!(
        cmd,
        Command::Tui {
            live_url: Some("https://flag.example".into())
        }
    );
}

#[test]
fn bridge_defaults_listen_address() {
    let cmd = cli::parse_args_with_env(["bridge"], None).unwrap();
    assert_eq!(
        cmd,
        Command::Bridge {
            listen: DEFAULT_BRIDGE_LISTEN.into()
        }
    );
}

#[test]
fn help_is_an_error_carrying_the_usage_text() {
    let err = cli::parse_args_with_env(["--help"], None).unwrap_err();
    assert!(err.contains("tui-lol"));
    assert!(err.contains("--live-url"));
    assert!(err.contains("replay"));
}

#[test]
fn replay_requires_a_json_path() {
    let cmd =
        cli::parse_args_with_env(["replay", "tests/fixtures/allgamedata/full.json"], None).unwrap();
    assert_eq!(
        cmd,
        Command::Replay {
            path: "tests/fixtures/allgamedata/full.json".into()
        }
    );
}

#[test]
fn dump_file_positional_skips_the_network() {
    let cmd = cli::parse_args_with_env(["dump", "capture.json"], None).unwrap();
    assert_eq!(
        cmd,
        Command::Dump {
            live_url: None,
            file: Some("capture.json".into())
        }
    );
}
