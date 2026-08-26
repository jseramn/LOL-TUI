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
