//! Strictly non-overlapping poll scheduler (spec `live-client-poller` R2)
//! with game-lifecycle classification (R3).
//!
//! Cycle shape (design Data Flow): fetch → normalize/classify → emit →
//! `Clock::sleep_until(start + cadence)`. Because every step is sequential,
//! requests can never overlap; a slow resolve simply eats into the window
//! and the overdue sleep collapses to a no-op jump.
//!
//! Testability seams (design D5): [`HttpSource`] supplies bodies,
//! [`Clock`] supplies time. Production wiring is [`SystemClock`]; tests use
//! virtual clocks so scheduling proofs run offline and instantly.

use crate::api::client::ENDPOINT_ALL_GAME_DATA;
use crate::api::error::{PollError, TransientReason};
use crate::api::source::HttpSource;
use crate::model::live_data::parse_all_game_data;
use crate::model::snapshot::Snapshot;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Minimum allowed poll cadence.
pub const MIN_CADENCE_MS: u64 = 250;

/// Maximum allowed poll cadence.
pub const MAX_CADENCE_MS: u64 = 1000;

/// Cadence used when none is configured (default 1000 ms per spec R2).
pub const DEFAULT_CADENCE: Duration = Duration::from_millis(MAX_CADENCE_MS);

/// Clamps a requested cadence into the spec-mandated 250–1000 ms range.
///
/// Pure function: same input, same output, no side effects.
pub fn clamp_cadence(cadence: Duration) -> Duration {
    let millis = cadence.as_millis();
    let clamped = millis.clamp(u128::from(MIN_CADENCE_MS), u128::from(MAX_CADENCE_MS));
    Duration::from_millis(clamped as u64)
}

/// Time source abstraction over "now" and "sleep until".
pub trait Clock {
    fn now_millis(&self) -> u64;

    /// Sleeps until the wall/virtual deadline. Implementations MUST treat an
    /// already-past deadline as immediate (overdue slow cycles).
    fn sleep_until_millis(&self, deadline_millis: u64);
}

impl<T: Clock + ?Sized> Clock for std::rc::Rc<T> {
    fn now_millis(&self) -> u64 {
        (**self).now_millis()
    }

    fn sleep_until_millis(&self, deadline_millis: u64) {
        (**self).sleep_until_millis(deadline_millis);
    }
}

impl<T: Clock + ?Sized + Send + Sync> Clock for std::sync::Arc<T> {
    fn now_millis(&self) -> u64 {
        (**self).now_millis()
    }

    fn sleep_until_millis(&self, deadline_millis: u64) {
        (**self).sleep_until_millis(deadline_millis);
    }
}

/// Production wall clock backed by [`SystemTime`] and thread sleeps.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as u64)
            .unwrap_or(0)
    }

    fn sleep_until_millis(&self, deadline_millis: u64) {
        loop {
            let now = self.now_millis();
            if now >= deadline_millis {
                return;
            }
            let remaining_ms = deadline_millis - now;
            let chunk = Duration::from_millis(remaining_ms.min(50));
            std::thread::sleep(chunk);
        }
    }
}

/// Game-lifecycle state observed by the poller (FSM input of design D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    NotInGame,
    InGame,
}

/// One completed cycle's outcomes, drained by the app layer (Unit 3).
#[derive(Debug, Clone, PartialEq)]
pub enum PollMsg {
    /// Emitted ONLY when the lifecycle actually changes (spec R3).
    Lifecycle(Lifecycle),
    /// A fresh normalized snapshot from a successful poll.
    Snapshot(Snapshot),
    /// Transient failure reason; lifecycle and last good snapshot retained.
    Transient(TransientReason),
}

/// Sequential poll driver over any [`HttpSource`] + [`Clock`] pair.
#[derive(Debug)]
pub struct Poller<S, C> {
    source: S,
    clock: C,
    cadence: Duration,
    lifecycle: Lifecycle,
}

impl<S: HttpSource, C: Clock> Poller<S, C> {
    /// Creates a poller whose cadence is clamped into 250–1000 ms.
    pub fn new(source: S, clock: C, requested_cadence: Duration) -> Self {
        Self {
            source,
            clock,
            cadence: clamp_cadence(requested_cadence),
            lifecycle: Lifecycle::NotInGame,
        }
    }

    /// The clamped cadence actually in effect.
    pub fn cadence(&self) -> Duration {
        self.cadence
    }

    /// Runs exactly one non-overlapping cycle and returns its messages.
    ///
    /// Classification (design error taxonomy / spec R3):
    /// - success ⇒ `Snapshot` (+ `Lifecycle(InGame)` on change);
    ///   malformed JSON ⇒ `Transient(Parse)` with lifecycle retained;
    /// - connection refused / port unbound ⇒ `Lifecycle(NotInGame)` on change;
    /// - timeout / TLS / HTTP ⇒ `Transient(reason)`, lifecycle retained.
    ///
    /// After emitting, sleeps out the remaining window relative to this
    /// cycle's start — overdue cycles collapse the sleep to a no-op jump,
    /// which is what keeps polls strictly non-overlapping.
    pub fn run_once(&mut self) -> Vec<PollMsg> {
        let started_at = self.clock.now_millis();
        let mut messages = Vec::with_capacity(2);

        match self.source.fetch(ENDPOINT_ALL_GAME_DATA) {
            Ok(body) => match parse_all_game_data(&body) {
                Ok(data) => {
                    self.transition_to(Lifecycle::InGame, &mut messages);
                    messages.push(PollMsg::Snapshot(Snapshot::from_live(&data)));
                }
                Err(_) => {
                    // Malformed payload: reported, previous snapshot retained,
                    // polling continues (poller:R4/S3).
                    messages.push(PollMsg::Transient(TransientReason::Parse));
                }
            },
            Err(PollError::NotBound(_)) => {
                // The ONLY failure class that ends a game session (R3/S1,S2).
                self.transition_to(Lifecycle::NotInGame, &mut messages);
            }
            Err(PollError::Transient(reason)) => {
                // Timeout/TLS/HTTP never end the game (R3/S3).
                messages.push(PollMsg::Transient(reason));
            }
        }

        let deadline = started_at.saturating_add(self.cadence.as_millis() as u64);
        self.clock.sleep_until_millis(deadline);
        messages
    }

    /// Records a lifecycle change; repeated states emit nothing (R3).
    fn transition_to(&mut self, next: Lifecycle, messages: &mut Vec<PollMsg>) {
        if next != self.lifecycle {
            self.lifecycle = next;
            messages.push(PollMsg::Lifecycle(next));
        }
    }
}
