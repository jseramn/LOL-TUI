//! tui-lol library root.
//!
//! The mod tree lives in the library target so integration tests under
//! `tests/` can exercise model/api/app/ui code directly; `main.rs` is a thin
//! binary shell on top of this crate.
pub mod api;
pub mod app;
pub mod bridge;
pub mod cli;
pub mod dump;
pub mod events;
pub mod glyphs;
pub mod history;
pub mod intel;
pub mod model;
pub mod ui;
