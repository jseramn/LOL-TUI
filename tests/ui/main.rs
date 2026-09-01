//! Integration harness for UI shell tests (task 3.4).
//!
//! Cargo auto-discovery ignores loose `.rs` files under `tests/<subdir>/`;
//! this file is the required binary root declaring the modules.

mod dashboard_tests;
mod degradation_tests;
mod format_tests;
mod glyph_tests;
mod local_strip_tests;
mod shell_render_tests;
mod standby_tests;
mod status_tests;
mod team_widget_tests;
mod ticker_tests;

/// Deterministic clock seam (design D5) shared by UI tests that must pin
/// the status line's last-update stamp.
pub struct FixedClock {
    pub millis: u64,
}

impl tui_lol::api::poller::Clock for FixedClock {
    fn now_millis(&self) -> u64 {
        self.millis
    }

    fn sleep_until_millis(&self, _deadline_millis: u64) {}
}
