//! App-shell contracts (task 3.1, design D4): the lifecycle FSM driven by
//! [`PollMsg`]s, transient-degradation semantics, and the per-frame
//! latest-wins mpsc drain.
//!
//! Spec anchors:
//! - poller:R3/S1 — first successful poll ⇒ InGame.
//! - ui:R1 — q/Esc handled elsewhere; this file proves state plumbing only.
//! - design error-taxonomy table — only NotBound ends a game.

use std::sync::mpsc;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseEvent, MouseEventKind,
};
use tui_lol::api::error::TransientReason;
use tui_lol::api::poller::{Clock, Lifecycle, PollMsg};
use tui_lol::app::{App, Health, Phase};
use tui_lol::model::snapshot::{GameInfo, LocalPlayerSnapshot, PlayerSnapshot, Snapshot};

/// Hand-built player row; every field except the name stays absent so tests
/// can distinguish snapshots purely by roster names.
fn player(name: &str) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: Some(name.to_owned()),
        champion: None,
        team: None,
        position: None,
        level: None,
        kills: None,
        deaths: None,
        assists: None,
        creep_score: None,
        ward_score: None,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: None,
        respawn_timer: None,
    }
}

/// Snapshot whose identity is its roster (`[]` ⇒ empty roster).
fn snap(roster: &[&str]) -> Snapshot {
    Snapshot {
        players: roster.iter().map(|name| player(name)).collect(),
        local: None,
        game: None,
        events: Vec::new(),
    }
}

fn live_app() -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app
}

// --- Lifecycle FSM (design D4) ---------------------------------------------

#[test]
fn starts_in_standby_before_any_poll() {
    let app = App::new();
    assert_eq!(app.phase(), Phase::NotInGame);
    assert!(app.snapshot().is_none());
}

#[test]
fn first_contact_transitions_to_in_game_healthy() {
    // poller:R3/S1 — first successful poll announces InGame.
    let app = live_app();
    assert_eq!(
        app.phase(),
        Phase::InGame {
            health: Health::Healthy
        }
    );
}

#[test]
fn refusal_ends_the_game_back_to_standby() {
    // Design taxonomy: conn refused / port unbound is the ONLY path out of InGame.
    let mut app = live_app();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::NotInGame));
    assert_eq!(app.phase(), Phase::NotInGame);
}

#[test]
fn repeated_ingame_announcements_change_nothing() {
    let mut app = live_app();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    assert_eq!(
        app.phase(),
        Phase::InGame {
            health: Health::Healthy
        }
    );
}

// --- Transient degradation --------------------------------------------------

#[test]
fn transient_failure_degrades_but_retains_last_snapshot() {
    // R3/S3 — Timeout/TLS/HTTP never end the game; last good data is kept.
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(snap(&["A"]))));
    app.on_msg(PollMsg::Transient(TransientReason::Tls));

    assert_eq!(
        app.phase(),
        Phase::InGame {
            health: Health::Degraded(TransientReason::Tls)
        }
    );
    assert_eq!(app.snapshot(), Some(&snap(&["A"])));
}

#[test]
fn fresh_snapshot_after_degradation_restores_health() {
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(snap(&["A"]))));
    app.on_msg(PollMsg::Transient(TransientReason::Timeout));
    app.on_msg(PollMsg::Snapshot(Box::new(snap(&["B"]))));

    assert_eq!(
        app.phase(),
        Phase::InGame {
            health: Health::Healthy
        }
    );
    assert_eq!(app.snapshot(), Some(&snap(&["B"])));
}

#[test]
fn transient_while_standby_leaves_silent_standby() {
    // ui:R2 — no error loop outside a game: Transient must not fabricate state.
    let mut app = App::new();
    app.on_msg(PollMsg::Transient(TransientReason::Http));
    assert_eq!(app.phase(), Phase::NotInGame);
    assert!(app.snapshot().is_none());
}

// --- Per-frame latest-wins drain -------------------------------------------

#[test]
fn drain_applies_only_the_latest_snapshot_per_frame() {
    let mut app = live_app();
    let (tx, rx) = mpsc::channel();
    tx.send(PollMsg::Snapshot(Box::new(snap(&["A"])))).unwrap();
    tx.send(PollMsg::Snapshot(Box::new(snap(&["B"])))).unwrap();

    let consumed = app.drain(&rx);

    assert_eq!(consumed, 2);
    assert_eq!(app.snapshot(), Some(&snap(&["B"])));
}

#[test]
fn drain_preserves_lifecycle_messages_between_snapshots() {
    let mut app = live_app();
    let (tx, rx) = mpsc::channel();
    tx.send(PollMsg::Snapshot(Box::new(snap(&["A"])))).unwrap();
    tx.send(PollMsg::Lifecycle(Lifecycle::NotInGame)).unwrap();
    tx.send(PollMsg::Snapshot(Box::new(snap(&["B"])))).unwrap();

    let consumed = app.drain(&rx);

    assert_eq!(consumed, 3);
    assert_eq!(app.phase(), Phase::NotInGame);
}

#[test]
fn drain_of_empty_and_disconnected_channels_is_safe() {
    let mut app = live_app();

    let (tx, rx) = mpsc::channel::<PollMsg>();
    assert_eq!(app.drain(&rx), 0);

    drop(tx); // poller thread gone: must not hang or panic
    assert_eq!(app.drain(&rx), 0);
    assert_eq!(
        app.phase(),
        Phase::InGame {
            health: Health::Healthy
        }
    );
}

// --- Last-update stamping (status-line input, ui spec R6) -------------------

/// Deterministic clock so the stamp assertion is exact, not wall-time-flaky.
struct FixedClock(u64);

impl Clock for FixedClock {
    fn now_millis(&self) -> u64 {
        self.0
    }

    fn sleep_until_millis(&self, _deadline_millis: u64) {}
}

#[test]
fn fresh_snapshot_stamps_last_update_from_the_clock() {
    let mut app = App::with_clock(FixedClock(1_724_592_000_123));
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    assert!(
        app.last_update_millis().is_none(),
        "lifecycle-only contact is not an update"
    );

    app.on_msg(PollMsg::Snapshot(Box::new(snap(&["A"]))));
    assert_eq!(app.last_update_millis(), Some(1_724_592_000_123));
}

// --- Input classification (task 3.2, events.rs; ui spec R1) -----------------

use tui_lol::events::{ShellAction, classify_event};

/// Builds a key event with an explicit kind — Windows terminals deliver both
/// Press and Release, and only Press may act.
fn key(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind,
        state: KeyEventState::empty(),
    })
}

#[test]
fn q_press_quits() {
    assert_eq!(
        classify_event(&key(KeyCode::Char('q'), KeyEventKind::Press)),
        Some(ShellAction::Quit)
    );
}

#[test]
fn escape_press_quits() {
    assert_eq!(
        classify_event(&key(KeyCode::Esc, KeyEventKind::Press)),
        Some(ShellAction::Quit)
    );
}

#[test]
fn key_release_events_are_ignored() {
    assert_eq!(
        classify_event(&key(KeyCode::Char('q'), KeyEventKind::Release)),
        None
    );
    assert_eq!(
        classify_event(&key(KeyCode::Esc, KeyEventKind::Release)),
        None
    );
}

#[test]
fn keys_other_than_the_quit_keys_do_not_quit() {
    assert_eq!(
        classify_event(&key(KeyCode::Char('a'), KeyEventKind::Press)),
        None
    );
    // The spec contract is lowercase `q` only.
    assert_eq!(
        classify_event(&key(KeyCode::Char('Q'), KeyEventKind::Press)),
        None
    );
}

#[test]
fn resize_events_carry_the_new_bounds() {
    assert_eq!(
        classify_event(&Event::Resize(80, 24)),
        Some(ShellAction::Resize {
            width: 80,
            height: 24
        })
    );
}

#[test]
fn unrelated_events_are_ignored() {
    let click = Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    });
    assert_eq!(classify_event(&Event::FocusGained), None);
    assert_eq!(classify_event(&click), None);
}

// --- Gold-history fold (task 2.5, viz:R8/S2+S3, design D4/D7) ----------------

/// Snapshot with only identity + local gold set; roster/events irrelevant
/// to the fold contract.
fn gold_snap(mode: Option<&str>, time: Option<f64>, gold: Option<f64>) -> Snapshot {
    Snapshot {
        players: Vec::new(),
        local: Some(LocalPlayerSnapshot {
            champion: None,
            level: None,
            current_gold: gold,
            stats: None,
            abilities: None,
        }),
        game: mode.map(|mode| GameInfo {
            game_mode: Some(mode.to_owned()),
            game_time: time,
            map_name: None,
            game_id: None,
        }),
        events: Vec::new(),
    }
}

fn window_of(app: &App) -> Vec<Option<u64>> {
    app.gold_window().collect()
}

#[test]
fn lifecycle_messages_never_touch_history() {
    // D7: lifecycle carries no identity — it must not clear OR push.
    let mut app = live_app();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Lifecycle(Lifecycle::NotInGame));
    assert_eq!(window_of(&app), Vec::<Option<u64>>::new());
}

#[test]
fn first_in_game_snapshot_initializes_history_with_one_sample() {
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(100.0),
        Some(120.0),
    ))));
    assert_eq!(window_of(&app), vec![Some(120)]);
}

#[test]
fn same_game_snapshots_append_oldest_to_newest() {
    let mut app = live_app();
    for time in [100.0f64, 101.0, 102.0] {
        app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
            Some("CLASSIC"),
            Some(time),
            Some(time * 10.0),
        ))));
    }
    assert_eq!(window_of(&app), vec![Some(1000), Some(1010), Some(1020)]);
}

#[test]
fn gold_samples_route_through_the_shared_truncation_policy() {
    // D5 sole-path rule exercised end to end: exposed 195.7 must land as 195.
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(1.0),
        Some(195.7),
    ))));
    assert_eq!(window_of(&app), vec![Some(195)]);
}

#[test]
fn absent_local_gold_pushes_a_gap_not_a_value() {
    // D6: absent-gold unifies with failed-poll gaps as "no observable value".
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(1.0),
        Some(50.0),
    ))));
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(2.0),
        None,
    ))));
    assert_eq!(window_of(&app), vec![Some(50), None]);
}

#[test]
fn transient_failure_in_game_pushes_a_gap() {
    // viz:R8 spec — failed polls insert gaps, not points (D4).
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(1.0),
        Some(50.0),
    ))));
    app.on_msg(PollMsg::Transient(TransientReason::Timeout));
    assert_eq!(window_of(&app), vec![Some(50), None]);
}

#[test]
fn snapshots_while_standby_do_not_feed_history() {
    // History feeds the IN_GAME view only; a snapshot that arrives before
    // any lifecycle announcement must not seed a phantom trend.
    let mut app = App::new();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(100.0),
        Some(42.0),
    ))));
    assert_eq!(window_of(&app), Vec::<Option<u64>>::new());
}

#[test]
fn notbound_round_trip_preserves_the_same_game_trend() {
    // viz:R8/S2 — IN_GAME→NOT_IN_GAME→IN_GAME inside ONE game continues
    // the pre-disconnect samples (identity survives; lifecycle never wipes).
    let mut app = live_app();
    for time in [100.0f64, 101.0] {
        app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
            Some("CLASSIC"),
            Some(time),
            Some(time),
        ))));
    }
    app.on_msg(PollMsg::Lifecycle(Lifecycle::NotInGame));
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(105.0),
        Some(105.0),
    ))));
    assert_eq!(window_of(&app), vec![Some(100), Some(101), Some(105)]);
}

#[test]
fn different_game_resets_then_pushes_only_the_new_sample() {
    // viz:R8/S3 — first snapshot of a NEW game leaves exactly one sample.
    let mut app = live_app();
    for time in [600.0f64, 601.0] {
        app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
            Some("CLASSIC"),
            Some(time),
            Some(time),
        ))));
    }
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(12.0),
        Some(77.0),
    ))));
    assert_eq!(window_of(&app), vec![Some(77)]);
}

#[test]
fn game_mode_change_resets_the_trend() {
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("CLASSIC"),
        Some(600.0),
        Some(500.0),
    ))));
    app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
        Some("ARAM"),
        Some(601.0),
        Some(9.0),
    ))));
    assert_eq!(window_of(&app), vec![Some(9)]);
}

#[test]
fn degraded_snapshot_without_game_info_appends_instead_of_resetting() {
    // D1 conservative default: a payload missing gameStats entirely is
    // inconclusive — it must NEVER wipe an existing trend.
    let mut app = live_app();
    for time in [100.0f64, 101.0] {
        app.on_msg(PollMsg::Snapshot(Box::new(gold_snap(
            Some("CLASSIC"),
            Some(time),
            Some(time),
        ))));
    }
    let degraded = Snapshot {
        players: Vec::new(),
        local: Some(LocalPlayerSnapshot {
            champion: None,
            level: None,
            current_gold: Some(999.0),
            stats: None,
            abilities: None,
        }),
        game: None,
        events: Vec::new(),
    };
    app.on_msg(PollMsg::Snapshot(Box::new(degraded)));
    assert_eq!(window_of(&app), vec![Some(100), Some(101), Some(999)]);
}
