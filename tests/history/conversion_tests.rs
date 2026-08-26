//! Shared f64→u64 conversion policy (viz:R5/S1, design D5): ONE helper for
//! every chart conversion — truncate toward zero, saturate at zero,
//! non-finite maps to absent. Drift into floor/round/per-widget casts is
//! what these vectors make impossible.

use tui_lol::history::chart_u64;

#[test]
fn fractional_values_truncate_toward_zero() {
    // The spec's pinned pair: both 195.7 and 195.2 yield exactly 195.
    assert_eq!(chart_u64(195.7), Some(195));
    assert_eq!(chart_u64(195.2), Some(195));
}

#[test]
fn non_finite_inputs_map_to_absent() {
    assert_eq!(chart_u64(f64::NAN), None);
    assert_eq!(chart_u64(f64::INFINITY), None);
    assert_eq!(chart_u64(f64::NEG_INFINITY), None);
}

#[test]
fn negative_inputs_saturate_at_zero() {
    // Gold/CS can never be negative; a hostile or buggy payload must not
    // fabricate one.
    assert_eq!(chart_u64(-3.5), Some(0));
    assert_eq!(chart_u64(-0.9), Some(0));
}

#[test]
fn zero_and_exact_integers_pass_through_deterministically() {
    assert_eq!(chart_u64(0.0), Some(0));
    assert_eq!(chart_u64(300.0), Some(300));
    assert_eq!(chart_u64(-0.0), Some(0));
}

#[test]
fn large_finite_values_survive_without_panic() {
    assert_eq!(chart_u64(1.0e10), Some(10_000_000_000));
}
