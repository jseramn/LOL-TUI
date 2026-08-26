//! Viewport degradation matrix contracts (task 5.1, viz spec R9).
//!
//! [`tui_lol::ui::select_layout`] must be a PURE function of `(width,
//! height)`: no real terminal, no app state — the same rectangle always
//! computes the same disjoint regions and the same chart-family visibility
//! set. The tiers follow design D9 exactly:
//!
//! ```text
//! height ≤ 12 : no chart families
//! 13–14       : + CS bars                        (CS hides LAST)
//! 15–17       : + level bars
//! 18–19       : + inventory bar
//! 20–23       : + K/D/A mini-bars
//! ≥ 24        : all five (+ gold sparkline)      (sparkline hides FIRST)
//! ```
//!
//! A width below 40 hides EVERY chart family. Gauges belong to no tier
//! (the spec omits them from the matrix ⇒ always-on), so they appear in no
//! visibility flag at all. The status row belongs to no tier either and
//! NEVER hides: the sweep pins it to the frame's final row for every
//! viewport from 1×1 through 200×60 (viz:R9/S2), with all regions pairwise
//! disjoint so no widget can ever overwrite the notice.

use ratatui::layout::Rect;
use tui_lol::ui::{ChartSet, LiveLayout, select_layout};

/// One chart-family visibility set with exactly the given members.
fn visible(cs: bool, level: bool, inventory: bool, kda: bool, sparkline: bool) -> ChartSet {
    ChartSet {
        cs,
        level,
        inventory,
        kda,
        sparkline,
    }
}

/// True when every family visible in `small` is also visible in `big`.
fn is_subset(small: &ChartSet, big: &ChartSet) -> bool {
    (!small.cs || big.cs)
        && (!small.level || big.level)
        && (!small.inventory || big.inventory)
        && (!small.kda || big.kda)
        && (!small.sparkline || big.sparkline)
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width
        && b.x < a.x + a.width
        && a.y < b.y + b.height
        && b.y < a.y + a.height
}

/// viz:R9/S1 — each height band of the D9 table computes exactly its set.
#[test]
fn tier_boundaries_follow_the_documented_height_table() {
    let at = |height: u16| select_layout(Rect::new(0, 0, 80, height)).visible;

    assert_eq!(at(12), visible(false, false, false, false, false), "≤12: none");
    assert_eq!(at(1), visible(false, false, false, false, false));

    assert_eq!(at(13), visible(true, false, false, false, false), "13: +CS");
    assert_eq!(at(14), visible(true, false, false, false, false), "14: +CS");

    assert_eq!(at(15), visible(true, true, false, false, false), "15: +level");
    assert_eq!(at(17), visible(true, true, false, false, false), "17: +level");

    assert_eq!(
        at(18),
        visible(true, true, true, false, false),
        "18: +inventory"
    );
    assert_eq!(
        at(19),
        visible(true, true, true, false, false),
        "19: +inventory"
    );

    assert_eq!(at(20), visible(true, true, true, true, false), "20: +K/D/A");
    assert_eq!(at(23), visible(true, true, true, true, false), "23: +K/D/A");

    assert_eq!(at(24), visible(true, true, true, true, true), "≥24: all");
    assert_eq!(at(60), visible(true, true, true, true, true), "≥24: all");
}

/// viz:R9/S1 — a viewport narrower than 40 columns hides EVERY chart
/// family regardless of height; the 40-column boundary restores the
/// height-tier set.
#[test]
fn widths_below_40_hide_every_chart_family() {
    let narrow = select_layout(Rect::new(0, 0, 39, 60)).visible;
    assert_eq!(narrow, visible(false, false, false, false, false));

    let boundary_wide = select_layout(Rect::new(0, 0, 40, 24)).visible;
    assert_eq!(boundary_wide, visible(true, true, true, true, true));

    // The width override cannot resurrect charts the height gate hid.
    let both_gates = select_layout(Rect::new(0, 0, 40, 12)).visible;
    assert_eq!(both_gates, visible(false, false, false, false, false));
}

/// viz:R9/S1 — hiding is MONOTONIC: shrink either dimension and the
/// smaller viewport's visible set is a subset of the larger one's.
#[test]
fn hiding_is_monotonic_between_smaller_and_larger_viewports() {
    let heights = [1u16, 12, 13, 14, 15, 17, 18, 19, 20, 23, 24, 30, 60];
    let widths = [1u16, 39, 40, 41, 79, 80, 120, 200];

    for &small_h in &heights {
        for &big_h in &heights {
            if big_h < small_h {
                continue;
            }
            for &small_w in &widths {
                for &big_w in &widths {
                    if big_w < small_w {
                        continue;
                    }
                    let small = select_layout(Rect::new(0, 0, small_w, small_h)).visible;
                    let big = select_layout(Rect::new(0, 0, big_w, big_h)).visible;
                    assert!(
                        is_subset(&small, &big),
                        "{small_w}x{small_h} set must be a subset of {big_w}x{big_h}: {small:?} ⊄ {big:?}"
                    );
                }
            }
        }
    }
}

/// viz:R9/S1 — chart families hide STRICTLY in the documented priority
/// order as height descends: gold sparkline first, then K/D/A, then
/// inventory, then level bars; CS bars hide last. Walking every height at
/// a chart-friendly width may switch off at most the next family in line.
#[test]
fn families_hide_strictly_in_the_documented_priority_order() {
    let priority: [(&str, fn(&ChartSet) -> bool); 5] = [
        ("sparkline", |c| c.sparkline),
        ("kda", |c| c.kda),
        ("inventory", |c| c.inventory),
        ("level", |c| c.level),
        ("cs", |c| c.cs),
    ];

    let mut hidden_so_far = 0usize;
    for height in (1u16..=60).rev() {
        let set = select_layout(Rect::new(0, 0, 80, height)).visible;
        while hidden_so_far < priority.len() && !(priority[hidden_so_far].1)(&set) {
            hidden_so_far += 1;
        }
        for (index, (name, is_on)) in priority.iter().enumerate() {
            if index < hidden_so_far {
                assert!(
                    !is_on(&set),
                    "height {height}: {name} must stay hidden once earlier families hid"
                );
            } else {
                assert!(
                    is_on(&set),
                    "height {height}: {name} must not hide before earlier families in the priority order"
                );
            }
        }
    }
    assert_eq!(hidden_so_far, priority.len(), "every family hides by 1 row");
}

/// viz:R9/S2 — sweeping EVERY viewport from 1×1 through 200×60, the status
/// region is present in every result: exactly one cell tall, full width,
/// pinned to the frame's final row, and disjoint from every other region
/// (nothing can overwrite the notice — ui spec R6 made structural).
#[test]
fn status_row_is_present_for_every_viewport_from_1x1_to_200x60() {
    for width in 1u16..=200 {
        for height in 1u16..=60 {
            let area = Rect::new(0, 0, width, height);
            let LiveLayout { areas, .. } = select_layout(area);

            assert_eq!(
                areas.status,
                Rect {
                    x: 0,
                    y: height - 1,
                    width,
                    height: 1
                },
                "{width}x{height}: status must own the final row"
            );

            let others = [
                ("header", areas.header),
                ("body", areas.body),
                ("local", areas.local),
                ("ticker", areas.ticker),
            ];
            for (name, region) in others {
                assert!(
                    !overlaps(region, areas.status),
                    "{width}x{height}: {region:?} ({name}) overlaps the status row"
                );
            }
            for (i, (a_name, a)) in others.iter().enumerate() {
                for (b_name, b) in others.iter().skip(i + 1) {
                    assert!(
                        !overlaps(*a, *b),
                        "{width}x{height}: {a_name} and {b_name} overlap ({a:?} vs {b:?})"
                    );
                }
            }
        }
    }
}
