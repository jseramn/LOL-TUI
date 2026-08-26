//! Team-column widgets: per-player creep-score bars, fixed-scale level
//! bars, tri-color K/D/A mini-bars, and the inventory fill strip.
//!
//! Owning decision: design **D3** maps each family onto ratatui native
//! primitives (or inline glyph strips) fed exclusively through the glyph
//! whitelist module; one shared-maximum rule governs all counters.
//!
//! Created as a Phase-0 skeleton (task 0.2); implemented in Phase 4
//! (tasks 4.2–4.5).
