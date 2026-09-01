//! Decision briefing: cross player, team, and objective data into Spanish
//! sentences without stat acronyms.

use tui_lol::intel;
use tui_lol::model::live_data::LiveData;
use tui_lol::model::snapshot::Snapshot;

fn snapshot_from_fixture(name: &str) -> Snapshot {
    let raw = std::fs::read_to_string(format!("tests/fixtures/allgamedata/{name}.json"))
        .expect("fixture file");
    let data: LiveData = serde_json::from_str(&raw).expect("valid fixture JSON");
    Snapshot::from_live(&data)
}

#[test]
fn full_fixture_briefing_crosses_lane_objectives_and_death_window() {
    let lines = intel::briefing(&snapshot_from_fixture("full"));
    assert!(!lines.is_empty());
    assert!(lines.len() <= intel::MAX_LINES);

    let text = lines.join(" ");
    assert!(text.contains("subditos"), "lane farm gap in words: {text}");
    assert!(
        text.contains("dragon") || text.contains("heraldo") || text.contains("torre"),
        "objective control in words: {text}"
    );
    assert!(
        text.contains("muerto") || text.contains("Muerto") || text.contains("ventana"),
        "death window for a decision: {text}"
    );
    for banned in [
        "CS", "KP", "KDA", "DRG", "BRN", "TWR", "JGL", "ADC", "SUP", "Lv", "DEAD",
    ] {
        assert!(
            !text.split_whitespace().any(|word| word == banned),
            "briefing must not use {banned:?}: {text}"
        );
    }
}

#[test]
fn empty_roster_still_returns_a_sentence() {
    let lines = intel::briefing(&Snapshot::default());
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("Todavia no hay cruce"));
}
