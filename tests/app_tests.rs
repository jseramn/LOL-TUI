//! App-shell contracts (task 3.1, design D4): the lifecycle FSM driven by
//! [`PollMsg`]s, transient-degradation semantics, and the per-frame
//! latest-wins mpsc drain.
//!
//! Spec anchors:
//! - poller:R3/S1 — first successful poll ⇒ InGame.
//! - ui:R1 — q/Esc handled elsewhere; this file proves state plumbing only.
//! - design error-taxonomy table — only NotBound ends a game.

use std::sync::mpsc;

use tui_lol::api::error::TransientReason;
use tui_lol::api::poller::{Clock, Lifecycle, PollMsg};
use tui_lol::app::{App, Health, Phase};
use tui_lol::model::snapshot::{PlayerSnapshot, Snapshot};

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
    assert_eq!(app.phase(), Phase::InGame { health: Health::Healthy });
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
    assert_eq!(app.phase(), Phase::InGame { health: Health::Healthy });
}

// --- Transient degradation --------------------------------------------------

#[test]
fn transient_failure_degrades_but_retains_last_snapshot() {
    // R3/S3 — Timeout/TLS/HTTP never end the game; last good data is kept.
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(snap(&["A"])));
    app.on_msg(PollMsg::Transient(TransientReason::Tls));

    assert_eq!(
        app.phase(),
        Phase::InGame { health: Health::Degraded(TransientReason::Tls) }
    );
    assert_eq!(app.snapshot(), Some(&snap(&["A"])));
}

#[test]
fn fresh_snapshot_after_degradation_restores_health() {
    let mut app = live_app();
    app.on_msg(PollMsg::Snapshot(snap(&["A"])));
    app.on_msg(PollMsg::Transient(TransientReason::Timeout));
    app.on_msg(PollMsg::Snapshot(snap(&["B"])));

    assert_eq!(app.phase(), Phase::InGame { health: Health::Healthy });
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
    tx.send(PollMsg::Snapshot(snap(&["A"]))).unwrap();
    tx.send(PollMsg::Snapshot(snap(&["B"]))).unwrap();

    let consumed = app.drain(&rx);

    assert_eq!(consumed, 2);
    assert_eq!(app.snapshot(), Some(&snap(&["B"])));
}

#[test]
fn drain_preserves_lifecycle_messages_between_snapshots() {
    let mut app = live_app();
    let (tx, rx) = mpsc::channel();
    tx.send(PollMsg::Snapshot(snap(&["A"]))).unwrap();
    tx.send(PollMsg::Lifecycle(Lifecycle::NotInGame)).unwrap();
    tx.send(PollMsg::Snapshot(snap(&["B"]))).unwrap();

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
    assert_eq!(app.phase(), Phase::InGame { health: Health::Healthy });
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
    assert!(app.last_update_millis().is_none(), "lifecycle-only contact is not an update");

    app.on_msg(PollMsg::Snapshot(snap(&["A"])));
    assert_eq!(app.last_update_millis(), Some(1_724_592_000_123));
}
