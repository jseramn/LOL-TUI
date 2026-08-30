//! History primitives for the gold trend, the shared f64-to-u64 chart
//! conversion policy, and game identity classification.
//!
//! Owning decisions:
//! - **D4** places history app-side: the poller stays frame-pure, and
//!   `App` folds accepted snapshots into a preallocated
//!   `RingBuffer<u64, 120>` (`GoldHistory`, about 1.9 KiB). Failed polls
//!   become explicit `None` gaps; capacity never grows.
//! - **D5** funnels every f64-to-u64 chart conversion through one helper
//!   (`chart_u64`: truncate toward zero, saturate at zero, non-finite is
//!   absent) so scaling policy cannot drift per widget.
//! - **D1** defines the different-game identity signal used to decide
//!   whether history continues across reconnects or resets into a new game.
//!
//! Implemented by Phase 2 (tasks 2.1–2.6).

use std::array;

/// Capacity-bounded, preallocated FIFO of optional samples (design D4).
///
/// Storage is a fixed `[Option<T>; N]` allocated once at construction:
/// capacity never grows, so retained history has a compile-time upper
/// bound (`GoldHistory` ≈ 1.9 KiB, spec budget 32 KiB). A push of `None`
/// records an explicit *gap* ("no observable value this cycle") rather
/// than inventing a point; iteration always yields oldest → newest.
pub struct RingBuffer<T, const N: usize> {
    entries: [Option<T>; N],
    /// Index of the oldest live sample.
    head: usize,
    /// Number of live samples (`≤ N`; equals `N` once wrapped).
    len: usize,
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    /// Fixed storage capacity; pushes beyond it evict the oldest sample.
    pub const CAPACITY: usize = N;

    pub fn new() -> Self {
        Self {
            entries: array::from_fn(|_| None),
            head: 0,
            len: 0,
        }
    }

    /// Appends one sample in O(1). Past capacity the oldest sample is
    /// overwritten in place; no allocation ever occurs.
    pub fn push(&mut self, sample: Option<T>) {
        if self.len < N {
            self.entries[(self.head + self.len) % N] = sample;
            self.len += 1;
        } else {
            self.entries[self.head] = sample;
            self.head = (self.head + 1) % N;
        }
    }

    /// Drops every sample (new-game reset path, design D7).
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    /// Live samples currently retained (`≤ CAPACITY`).
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterates the logical window oldest → newest; gap samples surface as
    /// `None` exactly where they were recorded.
    pub fn iter(&self) -> impl Iterator<Item = Option<T>> + '_ {
        (0..self.len).map(move |offset| self.entries[(self.head + offset) % N])
    }
}

impl<T: Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> std::fmt::Debug for RingBuffer<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Structural debug only: samples may be arbitrary non-Debug types.
        f.debug_struct("RingBuffer")
            .field("capacity", &N)
            .field("head", &self.head)
            .field("len", &self.len)
            .finish()
    }
}

/// The sole instantiation (design D4): 120 local-gold samples, one per
/// accepted snapshot (~2 min at 1 Hz).
pub type GoldHistory = RingBuffer<u64, 120>;

/// The shared f64→u64 chart-conversion policy (design D5) — the ONLY path
/// for converting exposed floats (CS, gold samples) into chart units:
///
/// - non-finite (`NaN`, `±inf`) ⇒ `None` (absent — never a fabricated bar),
/// - otherwise truncate toward zero and saturate at zero.
///
/// Integer counters bypass this helper; per-widget casts are forbidden so
/// scaling policy cannot drift between widgets.
pub fn chart_u64(value: f64) -> Option<u64> {
    if !value.is_finite() {
        return None;
    }
    let truncated = value.trunc();
    if truncated <= 0.0 {
        return Some(0);
    }
    // Float-to-int casts saturate in Rust (since 1.45), so even absurd
    // finite magnitudes stay deterministic and panic-free.
    Some(truncated as u64)
}

/// Jitter tolerance for the gameTime identity signal (design D1): real new
/// games are minutes apart, so only a decrease strictly beyond this bound
/// may classify as a different game.
pub const IDENTITY_TIME_TOLERANCE_SECONDS: f64 = 5.0;

/// The identity signal compared at snapshot-fold time (design D1). Both
/// fields are optional so degraded payloads stay representable; an
/// inconclusive comparison conservatively preserves history.
#[derive(Debug, Clone, PartialEq)]
pub struct GameIdentity {
    pub game_id: Option<u64>,
    pub game_time: Option<f64>,
    pub game_mode: Option<String>,
}

impl GameIdentity {
    /// Extracts the identity from a snapshot's normalized game info.
    pub fn from_game_info(info: &crate::model::snapshot::GameInfo) -> Self {
        Self {
            game_id: info.game_id,
            game_time: info.game_time,
            game_mode: info.game_mode.clone(),
        }
    }
}

/// Verdict of an identity comparison at fold time (design D7): SameGame
/// appends to the trend, DifferentGame clears it first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Continuation {
    SameGame,
    DifferentGame,
}

/// Classifies whether `next` continues the game described by `prev`
/// (design D1, extended with optional live `gameId`):
///
/// 1. both game ids present and unequal ⇒ different game; both present and
///    equal ⇒ same game (id is authoritative over time jitter);
/// 2. else both game times present and `prev - next > 5.0 s` ⇒ different;
/// 3. else both modes present and changed ⇒ different game;
/// 4. otherwise same game — absence is never evidence of a new game.
pub fn classify(prev: &GameIdentity, next: &GameIdentity) -> Continuation {
    if let (Some(prev_id), Some(next_id)) = (prev.game_id, next.game_id) {
        return if prev_id == next_id {
            Continuation::SameGame
        } else {
            Continuation::DifferentGame
        };
    }
    if let (Some(prev_time), Some(next_time)) = (prev.game_time, next.game_time)
        && prev_time - next_time > IDENTITY_TIME_TOLERANCE_SECONDS
    {
        return Continuation::DifferentGame;
    }
    if let (Some(prev_mode), Some(next_mode)) = (&prev.game_mode, &next.game_mode)
        && prev_mode != next_mode
    {
        return Continuation::DifferentGame;
    }
    Continuation::SameGame
}
