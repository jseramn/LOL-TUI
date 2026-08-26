//! Local-player strip widgets: HP and power gauges plus the gold sparkline
//! with its warm-up placeholder.
//!
//! Owning decisions: design **D3** assigns gauges/sparkline to the local
//! strip only (gold never leaves it); **D6** requires fewer than two real
//! samples to render explicit warm-up text instead of a flat line.
//!
//! Created as a Phase-0 skeleton (task 0.2); implemented in Phase 4
//! (tasks 4.6–4.7).
