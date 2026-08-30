//! Game-identity truth table (viz:R8/S2+S3 support, design D1): a gameTime
//! decrease beyond 5 s means a NEW game; otherwise a gameMode change (when
//! both sides expose one) decides; anything inconclusive stays SameGame so
//! degraded payloads can never destroy a trend.

use tui_lol::history::{Continuation, GameIdentity, classify};

fn identity(time: Option<f64>, mode: Option<&str>) -> GameIdentity {
    GameIdentity {
        game_id: None,
        game_time: time,
        game_mode: mode.map(str::to_owned),
    }
}

#[test]
fn large_time_decrease_is_a_different_game() {
    let prev = identity(Some(100.0), Some("CLASSIC"));
    let next = identity(Some(94.9), Some("CLASSIC"));
    assert_eq!(classify(&prev, &next), Continuation::DifferentGame);
}

#[test]
fn time_decrease_of_exactly_the_tolerance_is_the_same_game() {
    // Strictly-greater rule: 5.0 s of jitter is tolerated, 5.0001 is not.
    let prev = identity(Some(100.0), Some("CLASSIC"));
    assert_eq!(
        classify(&prev, &identity(Some(95.0), Some("CLASSIC"))),
        Continuation::SameGame
    );
    assert_eq!(
        classify(&prev, &identity(Some(94.9999), Some("CLASSIC"))),
        Continuation::DifferentGame
    );
}

#[test]
fn normal_progression_and_upward_jitter_stay_same_game() {
    let prev = identity(Some(100.0), Some("CLASSIC"));
    assert_eq!(
        classify(&prev, &identity(Some(100.4), Some("CLASSIC"))),
        Continuation::SameGame
    );
    assert_eq!(
        classify(&prev, &identity(Some(105.0), Some("CLASSIC"))),
        Continuation::SameGame
    );
}

#[test]
fn game_mode_change_with_both_present_is_a_different_game() {
    let prev = identity(Some(100.0), Some("CLASSIC"));
    assert_eq!(
        classify(&prev, &identity(Some(101.0), Some("ARAM"))),
        Continuation::DifferentGame
    );
}

#[test]
fn mode_change_decides_even_when_time_is_absent() {
    let prev = identity(None, Some("CLASSIC"));
    assert_eq!(
        classify(&prev, &identity(None, Some("ARAM"))),
        Continuation::DifferentGame
    );
}

#[test]
fn degraded_payloads_never_wipe_history() {
    // Absent fields are inconclusive — the conservative default preserves
    // the trend rather than destroying it.
    let full = identity(Some(100.0), Some("CLASSIC"));
    let blank = identity(None, None);
    assert_eq!(classify(&full, &blank), Continuation::SameGame);
    assert_eq!(classify(&blank, &full), Continuation::SameGame);
    assert_eq!(
        classify(
            &identity(Some(100.0), None),
            &identity(Some(99.0), Some("CLASSIC"))
        ),
        Continuation::SameGame,
        "mode unknown on one side: small time drop alone cannot decide"
    );
}

#[test]
fn unchanged_identity_is_same_game() {
    let prev = identity(Some(512.25), Some("CLASSIC"));
    assert_eq!(classify(&prev, &prev.clone()), Continuation::SameGame);
}

#[test]
fn matching_game_ids_are_the_same_game_even_if_time_drops() {
    let prev = GameIdentity {
        game_id: Some(42),
        game_time: Some(600.0),
        game_mode: Some("CLASSIC".into()),
    };
    let next = GameIdentity {
        game_id: Some(42),
        game_time: Some(1.0),
        game_mode: Some("CLASSIC".into()),
    };
    assert_eq!(classify(&prev, &next), Continuation::SameGame);
}

#[test]
fn differing_game_ids_are_a_different_game() {
    let prev = GameIdentity {
        game_id: Some(1),
        game_time: Some(100.0),
        game_mode: Some("CLASSIC".into()),
    };
    let next = GameIdentity {
        game_id: Some(2),
        game_time: Some(101.0),
        game_mode: Some("CLASSIC".into()),
    };
    assert_eq!(classify(&prev, &next), Continuation::DifferentGame);
}
