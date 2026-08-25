//! Strictly non-overlapping poll scheduling acceptance criteria
//! (spec `live-client-poller` R2) plus lifecycle classification contracts
//! (R3) driving tasks 2.5–2.7.
//!
//! RED phase Unit 2: authored against `api::poller` BEFORE it existed.
//! All timing is virtual (`FakeClock`), so the suite stays offline and
//! deterministic (poller:R5).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use tui_lol::api::error::PollError;
use tui_lol::api::poller::{clamp_cadence, Clock, Poller};
use tui_lol::api::source::HttpSource;

/// Virtual millisecond clock: sleeps jump instantly to the deadline.
#[derive(Default)]
struct FakeClock(Cell<u64>);

impl FakeClock {
    fn advance(&self, millis: u64) {
        self.0.set(self.0.get() + millis);
    }
}

impl Clock for FakeClock {
    fn now_millis(&self) -> u64 {
        self.0.get()
    }

    fn sleep_until_millis(&self, deadline_millis: u64) {
        self.0.set(self.0.get().max(deadline_millis));
    }
}

/// One fetch's (start, end) virtual timestamps, in call order.
type Log = Rc<RefCell<Vec<(u64, u64)>>>;

/// Scripted source: costs `cost_ms` of virtual time, returns `outcome`.
struct FakeSource {
    clock: Rc<FakeClock>,
    cost_ms: u64,
    outcome: Result<String, PollError>,
    log: Log,
}

impl FakeSource {
    fn fast(clock: &Rc<FakeClock>, log: &Log) -> Self {
        Self {
            clock: Rc::clone(clock),
            cost_ms: 30,
            outcome: Ok(String::from("{}")),
            log: Rc::clone(log),
        }
    }

    fn slow(clock: &Rc<FakeClock>, log: &Log, cost_ms: u64) -> Self {
        Self {
            clock: Rc::clone(clock),
            cost_ms,
            outcome: Ok(String::from("{}")),
            log: Rc::clone(log),
        }
    }
}

impl HttpSource for FakeSource {
    fn fetch(&self, _path: &str) -> Result<String, PollError> {
        let start = self.clock.now_millis();
        self.clock.advance(self.cost_ms);
        let end = self.clock.now_millis();
        self.log.borrow_mut().push((start, end));
        self.outcome.clone()
    }
}

fn drive<S: HttpSource, C: Clock>(poller: &mut Poller<S, C>, cycles: usize) {
    for _ in 0..cycles {
        let _emitted = poller.run_once();
    }
}

// ---------------------------------------------------------------------------
// Cadence clamping (task 2.5)
// ---------------------------------------------------------------------------

/// Out-of-range cadence values are clamped into the allowed range.
#[test]
fn cadence_is_clamped_into_allowed_range() {
    let cases = [
        (Duration::from_millis(50), Duration::from_millis(250)),
        (Duration::from_millis(249), Duration::from_millis(250)),
        (Duration::from_millis(250), Duration::from_millis(250)),
        (Duration::from_millis(640), Duration::from_millis(640)),
        (Duration::from_millis(1000), Duration::from_millis(1000)),
        (Duration::from_millis(1001), Duration::from_millis(1000)),
        (Duration::from_millis(9_999), Duration::from_millis(1000)),
    ];
    for (requested, expected) in cases {
        assert_eq!(
            clamp_cadence(requested),
            expected,
            "clamp({requested:?}) must be {expected:?}"
        );
    }
}

/// The poller stores the clamped value, not the requested one.
#[test]
fn poller_uses_clamped_cadence() {
    let clock = Rc::new(FakeClock::default());
    let log = Log::default();
    let mut poller = Poller::new(
        FakeSource::fast(&clock, &log),
        Rc::clone(&clock),
        Duration::from_millis(42),
    );
    assert_eq!(poller.cadence(), Duration::from_millis(250));
    drive(&mut poller, 1);
    // First cycle runs immediately; the post-cycle sleep targets start+250.
    let entries = log.borrow();
    let (start0, end0) = entries[0];
    assert_eq!((start0, end0), (0, 30));
    assert_eq!(clock.now_millis(), 250, "sleep_until(start+cadence) jumps there");
}

// ---------------------------------------------------------------------------
// Non-overlap (task 2.5)
// ---------------------------------------------------------------------------

/// poller:R2/S1 — fast responses: polls fire at most once per cadence window
/// and never concurrently (starts are strictly spaced >= cadence).
#[test]
fn fast_responses_fire_at_most_once_per_window() {
    let clock = Rc::new(FakeClock::default());
    let log = Log::default();
    let mut poller = Poller::new(
        FakeSource::fast(&clock, &log),
        Rc::clone(&clock),
        Duration::from_millis(250),
    );
    drive(&mut poller, 4);

    let entries = log.borrow();
    assert_eq!(entries.len(), 4);
    for window in entries.windows(2) {
        let (prev_start, prev_end) = window[0];
        let (next_start, _) = window[1];
        assert!(
            next_start >= prev_end,
            "cycle {next_start} started before previous resolved ({prev_end})"
        );
        assert!(
            next_start - prev_start >= 250,
            "next poll fired {} ms after the previous start; window is 250 ms",
            next_start - prev_start
        );
    }
}

/// poller:R2/S2 — a mocked 900 ms resolve delays the next poll until AFTER
/// the resolve: exactly one request in flight, next starts only once the
/// previous completed (start[1] >= end[0], regardless of the 250 ms cadence).
#[test]
fn slow_response_never_overlaps_next_cycle() {
    let clock = Rc::new(FakeClock::default());
    let log = Log::default();
    let mut poller = Poller::new(
        FakeSource::slow(&clock, &log, 900),
        Rc::clone(&clock),
        Duration::from_millis(250),
    );
    drive(&mut poller, 3);

    let entries = log.borrow();
    assert_eq!(entries.len(), 3);
    for window in entries.windows(2) {
        let (prev_start, prev_end) = window[0];
        let (next_start, _) = window[1];
        assert!(
            next_start >= prev_end,
            "overlapping polls: {next_start} < resolve {prev_end}"
        );
        // Slow path: the overdue sleep target is skipped entirely.
        assert!(
            next_start - prev_start >= 900,
            "cycle spacing {} shorter than the slow resolve",
            next_start - prev_start
        );
        assert!(prev_end - prev_start == 900 || prev_end - prev_start >= 900);
    }
    // First cycle: fetch 0..900, then the overdue sleep is a no-op jump.
    assert_eq!(entries[0], (0, 900));
    assert_eq!(entries[1].0, 900, "second cycle starts exactly at first resolve");
}
