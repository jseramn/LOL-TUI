//! App state, lifecycle FSM, and message pump (design D4 + Data Flow).
//!
//! The poller thread produces [`PollMsg`]s on an mpsc channel; the render
//! loop drains the channel **latest-wins each frame** and folds the messages
//! through [`App::on_msg`]. The FSM is exactly design D4:
//!
//! ```text
//! Phase::NotInGame ── Lifecycle(InGame) ──► Phase::InGame { Healthy }
//! Phase::InGame    ── Lifecycle(NotInGame) ► Phase::NotInGame      (only NotBound)
//! Phase::InGame    ── Transient(r) ───────► InGame { Degraded(r) }  (snapshot retained)
//! Phase::InGame    ── Snapshot(s) ────────► InGame { Healthy }     (fresh data)
//! ```
//!
//! Transient failures while standby are swallowed: outside a game the app
//! stays silent (ui spec R2 forbids an error loop).

use crate::api::error::TransientReason;
use crate::api::poller::{Clock, Lifecycle, PollMsg, SystemClock};
use crate::model::snapshot::Snapshot;
use std::sync::mpsc::{Receiver, TryRecvError};

/// Health annotation inside [`Phase::InGame`] (design D4 — degradation is
/// never a standby transition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// Latest poll cycle succeeded.
    Healthy,
    /// Last cycle failed transiently; the retained snapshot may be stale.
    Degraded(TransientReason),
}

/// Game-lifecycle phase of the dashboard (design D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// No live game detected — render the standby view.
    NotInGame,
    /// Live game in progress; `health` annotates data freshness.
    InGame { health: Health },
}

/// Application state folded from poller messages.
///
/// Generic over the [`Clock`] seam (design D5); production uses
/// [`SystemClock`] via [`App::new`].
#[derive(Debug)]
pub struct App<C: Clock = SystemClock> {
    clock: C,
    phase: Phase,
    snapshot: Option<Snapshot>,
    last_update_millis: Option<u64>,
}

impl App<SystemClock> {
    /// Production constructor: standby phase, no data, wall clock.
    pub fn new() -> Self {
        Self::with_clock(SystemClock)
    }
}

impl<C: Clock> App<C> {
    /// Test/alternative-clock constructor.
    pub fn with_clock(clock: C) -> Self {
        Self { clock, phase: Phase::NotInGame, snapshot: None, last_update_millis: None }
    }

    /// Current lifecycle phase for view dispatch.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Last good snapshot, retained across transient failures (R3/S3).
    pub fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    /// Millis timestamp of the most recent fresh snapshot, if any. Feeds the
    /// status line's "last update" readout (ui spec R6).
    pub fn last_update_millis(&self) -> Option<u64> {
        self.last_update_millis
    }

    /// Folds one poller message into the FSM.
    ///
    /// Classification mirrors the design error-taxonomy table; see the
    /// module diagram for the transition set.
    pub fn on_msg(&mut self, msg: PollMsg) {
        match msg {
            PollMsg::Lifecycle(Lifecycle::InGame) => {
                self.phase = Phase::InGame { health: Health::Healthy };
            }
            // Connection refused / port unbound is the ONLY game-ender.
            PollMsg::Lifecycle(Lifecycle::NotInGame) => {
                self.phase = Phase::NotInGame;
            }
            PollMsg::Snapshot(snapshot) => {
                self.snapshot = Some(snapshot);
                self.last_update_millis = Some(self.clock.now_millis());
                // Fresh data clears staleness, but never resurrects a game
                // that NotBound already ended.
                if let Phase::InGame { health } = &mut self.phase {
                    *health = Health::Healthy;
                }
            }
            PollMsg::Transient(reason) => {
                // Outside a game a transient failure is meaningless noise —
                // keep standby silent (ui spec R2).
                if let Phase::InGame { health } = &mut self.phase {
                    *health = Health::Degraded(reason);
                }
            }
        }
    }

    /// Drains every message currently buffered on `rx` (latest-wins per
    /// frame), collapsing consecutive snapshots so at most one snapshot per
    /// run is folded. Returns the number of raw messages consumed.
    ///
    /// A disconnected channel (poller thread gone) drains whatever remains
    /// and then simply yields zero forever — the shell keeps rendering.
    pub fn drain(&mut self, rx: &Receiver<PollMsg>) -> usize {
        let mut buffered = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(msg) => buffered.push(msg),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        let consumed = buffered.len();
        for msg in coalesce(buffered) {
            self.on_msg(msg);
        }
        consumed
    }
}

/// Collapses runs of consecutive [`PollMsg::Snapshot`]s down to the last one,
/// preserving interleaved lifecycle/transient ordering. Pure function.
fn coalesce(messages: Vec<PollMsg>) -> Vec<PollMsg> {
    let mut kept: Vec<PollMsg> = Vec::with_capacity(messages.len());
    for msg in messages {
        let replaces_snapshot =
            matches!(msg, PollMsg::Snapshot(_)) && matches!(kept.last(), Some(PollMsg::Snapshot(_)));
        if replaces_snapshot {
            *kept.last_mut().expect("just matched Some(_)") = msg;
        } else {
            kept.push(msg);
        }
    }
    kept
}
