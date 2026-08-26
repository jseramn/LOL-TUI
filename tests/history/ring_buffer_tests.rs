//! Ring-buffer contracts (viz:R8/S4, design D4): preallocated capacity of
//! 120 gold samples, O(1) push, oldest-to-newest iteration, explicit gaps,
//! and a hard memory bound well below 32 KiB.

use std::mem::size_of;

use tui_lol::history::{GoldHistory, RingBuffer};

#[test]
fn new_buffer_is_empty_with_zero_length() {
    let buffer = GoldHistory::new();
    assert_eq!(buffer.len(), 0);
    assert_eq!(buffer.iter().count(), 0);
}

#[test]
fn push_preserves_samples_oldest_to_newest() {
    let mut buffer = GoldHistory::new();
    for gold in [10u64, 20, 30] {
        buffer.push(Some(gold));
    }
    let window: Vec<Option<u64>> = buffer.iter().collect();
    assert_eq!(window, vec![Some(10), Some(20), Some(30)]);
}

#[test]
fn gaps_surface_as_none_samples_in_place() {
    // A failed poll inserts a gap, not a point (viz:R8 spec): None must
    // occupy its own slot between real samples.
    let mut buffer = GoldHistory::new();
    buffer.push(Some(50));
    buffer.push(None);
    buffer.push(Some(75));
    let window: Vec<Option<u64>> = buffer.iter().collect();
    assert_eq!(window, vec![Some(50), None, Some(75)]);
}

#[test]
fn wrapping_push_drops_oldest_and_pins_length_to_capacity() {
    let mut buffer = GoldHistory::new();
    for tick in 1u64..=130 {
        buffer.push(Some(tick));
    }
    assert_eq!(buffer.len(), GoldHistory::CAPACITY);
    assert_eq!(GoldHistory::CAPACITY, 120);

    let window: Vec<Option<u64>> = buffer.iter().collect();
    assert_eq!(window.len(), 120);
    // Samples 1..=10 were evicted; the oldest survivor is sample 11.
    assert_eq!(window[0], Some(11));
    assert_eq!(window[119], Some(130));
    // Continuity across the wrap point proves eviction order.
    for (offset, sample) in window.iter().enumerate() {
        assert_eq!(*sample, Some(11 + offset as u64));
    }
}

#[test]
fn clear_empties_the_buffer_so_a_new_game_starts_clean() {
    // viz:R8/S3 support: classify(DifferentGame) ⇒ clear() then push.
    let mut buffer = GoldHistory::new();
    for gold in [1u64, 2, 3, 4] {
        buffer.push(Some(gold));
    }
    buffer.clear();
    assert_eq!(buffer.len(), 0);
    buffer.push(Some(900));
    let window: Vec<Option<u64>> = buffer.iter().collect();
    assert_eq!(window, vec![Some(900)]);
}

#[test]
fn gold_history_storage_stays_within_the_32kib_bound() {
    // viz:R8/S4 — preallocated storage bound. 120 × Option<u64> ≈ 1.9 KiB;
    // the assertion pins the SPEC bound while the equality pins the design
    // figure so accidental growth (e.g. a Vec swap) cannot slip through.
    let byte_size = size_of::<GoldHistory>();
    assert!(
        byte_size <= 32 * 1024,
        "GoldHistory occupies {byte_size} bytes, over the 32 KiB budget"
    );
    // Fixed layout: the sample array plus exactly two usize cursors
    // (head, len). A swap to heap growth (e.g. VecDeque = 24 B + heap)
    // would break this equality.
    assert_eq!(
        byte_size,
        120 * size_of::<Option<u64>>() + 2 * size_of::<usize>()
    );
}

#[test]
fn generic_buffer_holds_capacity_for_any_instantiation() {
    // The const-generic primitive must behave identically beyond the gold
    // alias; a tiny instantiation wraps fast and keeps the proof cheap.
    let mut buffer: RingBuffer<u8, 2> = RingBuffer::new();
    for tick in 1u8..=3 {
        buffer.push(Some(tick));
    }
    assert_eq!(buffer.len(), 2);
    let window: Vec<Option<u8>> = buffer.iter().collect();
    assert_eq!(window, vec![Some(2), Some(3)]);
}
