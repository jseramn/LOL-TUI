//! tui-lol library root.
//!
//! The mod tree lives in the library target so integration tests under
//! `tests/` can exercise model/api/app/ui code directly; `main.rs` is a thin
//! binary shell on top of this crate.
pub mod api;
pub mod app;
pub mod events;
pub mod model;
pub mod ui;
