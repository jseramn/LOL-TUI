//! Local-strip visualization contracts (tasks 4.6–4.7; viz spec R7–R8).
//!
//! The local strip is the ONLY surface that ever renders gauges or the gold
//! trend: two blocked-segment HP/power gauges for the local player (absent
//! operands degrade to an explicit `?` per gauge), then the gold sparkline
//! fed from the app-side 120-sample ring buffer through `App::gold_window`
//! (design D3/D4). Snapshots are synthesized in-test (network-free) and
//! asserted through ratatui [`TestBackend`] buffers.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use tui_lol::api::error::TransientReason;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::glyphs::SPARKLINE_LEVELS;
use tui_lol::model::live_data::Statistics;
use tui_lol::model::snapshot::{GameInfo, LocalPlayerSnapshot, PlayerSnapshot, Snapshot, Team};
use tui_lol::ui::local_strip::gauge_ratio;

/// Wide enough that both gauge tracks carry enough cells for ±1-cell
/// proportional assertions.
const WIDTH: u16 = 120;
const HEIGHT: u16 = 24;

/// A player with every field absent except the team.
fn ps(team: Option<Team>) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: None,
        champion: None,
        team,
        position: None,
        level: None,
        kills: None,
        deaths: None,
        assists: None,
        creep_score: None,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: None,
        respawn_timer: None,
    }
}

/// Stat detail exposing ONLY the four gauge operands; everything else stays
/// absent so a leaking field would be visible immediately.
fn stats(
    current_health: Option<f64>,
    max_health: Option<f64>,
    power: Option<f64>,
    power_max: Option<f64>,
) -> Statistics {
    Statistics {
        ability_power: None,
        armor: None,
        attack_damage: None,
        attack_range: None,
        attack_speed: None,
        crit_chance: None,
        current_health,
        health_regen_rate: None,
        lifesteal: None,
        magic_resist: None,
        max_health,
        movement_speed: None,
        omnivamp: None,
        physical_lethality: None,
        physical_vamp: None,
        power,
        power_max,
        power_regen_rate: None,
    }
}

fn local_player(stats: Option<Statistics>, current_gold: Option<f64>) -> LocalPlayerSnapshot {
    LocalPlayerSnapshot {
        champion: Some("Ahri".to_owned()),
        level: Some(12),
        current_gold,
        stats,
    }
}

/// An in-game app holding one snapshot with a small roster and the given
/// local-player detail.
fn live_app_with_local(players: Vec<PlayerSnapshot>, local: Option<LocalPlayerSnapshot>) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(Snapshot {
        players,
        local,
        ..Snapshot::default()
    })));
    app
}

fn draw(app: &App) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("test backend");
    let frame = terminal
        .draw(|f| tui_lol::ui::render(f, app))
        .expect("frame");
    frame.buffer.clone()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn find_row(buffer: &Buffer, needle: &str) -> Option<u16> {
    (0..buffer.area.height).find(|&y| row_text(buffer, y).contains(needle))
}

/// Gauge rows lead with their explicit label at column zero — the legacy
/// LOCAL text line starts with the champion instead, so the prefixes cannot
/// collide. Asserts uniqueness: gauges belong to the local strip ONLY.
fn unique_gauge_row(buffer: &Buffer, label: &str) -> String {
    let matches: Vec<String> = (0..buffer.area.height)
        .map(|y| row_text(buffer, y))
        .filter(|row| row.trim_end().starts_with(label))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exactly one {label:?} gauge row may exist (non-local players never show gauges)"
    );
    matches.into_iter().next().expect("exactly one")
}

/// Filled-to-total block proportion of a rendered gauge row, ±1 cell of
/// rounding tolerated (LineGauge floors its fill width).
fn assert_gauge_fill(row: &str, expected_ratio: f64, context: &str) {
    let filled = row.chars().filter(|c| *c == '\u{2588}').count() as f64;
    let total = (row.chars().filter(|c| *c == '\u{2588}').count()
        + row.chars().filter(|c| *c == '\u{2591}').count()) as f64;
    assert!(
        total > 0.0,
        "{context}: gauge must draw a blocked track: {row:?}"
    );
    assert!(
        (filled - expected_ratio * total).abs() <= 1.0,
        "{context}: expected ~{expected_ratio:.2} fill over {total} cells: {row:?}"
    );
}

// --- Task 4.6 / viz spec R7: local HP and power gauges -----------------------

/// Unit-layer pin of the clamped ratio mapping itself, including the
/// degenerate inputs a fixture payload can carry: any absent operand, a
/// non-finite value, or a non-positive maximum maps to absent (`None`)
/// instead of fabricating a fill; out-of-range ratios saturate at both ends.
#[test]
fn gauge_ratio_follows_current_over_max_and_treats_degeneracy_as_absent() {
    let close = |got: Option<f64>, want: f64| matches!(got, Some(v) if (v - want).abs() < 1e-9);
    assert!(close(gauge_ratio(Some(2100.0), Some(3000.0)), 0.7));
    assert!(close(gauge_ratio(Some(400.0), Some(500.0)), 0.8));
    // Any missing operand is absent — never a guessed fraction.
    assert_eq!(gauge_ratio(None, Some(3000.0)), None);
    assert_eq!(gauge_ratio(Some(2100.0), None), None);
    // Non-positive maximum divides nothing: absent, not infinite.
    assert_eq!(gauge_ratio(Some(100.0), Some(0.0)), None);
    assert_eq!(gauge_ratio(Some(100.0), Some(-3000.0)), None);
    // Non-finite values follow the D5 spirit: absent, never fabricated.
    assert_eq!(gauge_ratio(Some(f64::NAN), Some(3000.0)), None);
    assert_eq!(gauge_ratio(Some(f64::INFINITY), Some(3000.0)), None);
    assert_eq!(gauge_ratio(Some(100.0), Some(f64::NAN)), None);
    // Out-of-range ratios clamp into [0, 1] without panicking.
    assert!(close(gauge_ratio(Some(9000.0), Some(3000.0)), 1.0));
    assert!(close(gauge_ratio(Some(-5.0), Some(3000.0)), 0.0));
}

/// viz:R7/S1 — local HP 2100/3000 and power 400/500 render gauges filled to
/// 70% and 80%. The roster is populated: non-local players must NOT gain a
/// gauge row of their own (uniqueness asserted per label).
#[test]
fn gauges_fill_70_and_80_percent_for_the_local_player_only() {
    let app = live_app_with_local(
        vec![ps(Some(Team::Order)), ps(Some(Team::Chaos))],
        Some(local_player(
            Some(stats(Some(2100.0), Some(3000.0), Some(400.0), Some(500.0))),
            None,
        )),
    );
    let buffer = draw(&app);

    let hp = unique_gauge_row(&buffer, "LOCAL HP");
    assert_gauge_fill(&hp, 0.7, "HP gauge");
    let power = unique_gauge_row(&buffer, "LOCAL Power");
    assert_gauge_fill(&power, 0.8, "Power gauge");
}

/// viz:R7/S2 — absent power fields degrade THAT gauge alone to an explicit
/// `?`; the health gauge keeps rendering normally beside it.
#[test]
fn missing_power_renders_placeholder_while_the_hp_gauge_renders_normally() {
    let app = live_app_with_local(
        vec![ps(Some(Team::Order))],
        Some(local_player(
            Some(stats(Some(2100.0), Some(3000.0), None, None)),
            None,
        )),
    );
    let buffer = draw(&app);

    let power = unique_gauge_row(&buffer, "LOCAL Power");
    assert!(
        power.trim_end().starts_with("LOCAL Power ?"),
        "absent power must show the placeholder: {power:?}"
    );
    assert_eq!(
        power.chars().filter(|c| *c == '\u{2588}').count(),
        0,
        "placeholder fabricates no fill: {power:?}"
    );
    assert_eq!(
        power.chars().filter(|c| *c == '\u{2591}').count(),
        0,
        "placeholder fabricates no track either: {power:?}"
    );

    let hp = unique_gauge_row(&buffer, "LOCAL HP");
    assert_gauge_fill(&hp, 0.7, "HP gauge must be unaffected");

    // Wholly absent stat detail degrades BOTH gauges.
    let bare = live_app_with_local(vec![], Some(local_player(None, None)));
    let bare_buffer = draw(&bare);
    let hp = unique_gauge_row(&bare_buffer, "LOCAL HP");
    assert!(
        hp.trim_end().starts_with("LOCAL HP ?"),
        "absent stats must show the placeholder on every gauge: {hp:?}"
    );
}

/// viz:R7 + design error-taxonomy row — an out-of-range ratio clamps to a
/// FULL gauge before ever reaching `LineGauge::ratio`, which panics above
/// 1.0: surviving this frame proves the clamp sits in front of the builder.
#[test]
fn overrange_health_clamps_to_a_full_gauge_without_panicking() {
    let app = live_app_with_local(
        vec![],
        Some(local_player(
            Some(stats(Some(9000.0), Some(3000.0), Some(1.0), Some(2.0))),
            None,
        )),
    );
    let buffer = draw(&app);

    let hp = unique_gauge_row(&buffer, "LOCAL HP");
    assert_eq!(
        hp.chars().filter(|c| *c == '\u{2591}').count(),
        0,
        "clamped-full gauge leaves no unfilled cells: {hp:?}"
    );
    assert_gauge_fill(&hp, 1.0, "HP gauge");
}

/// Regional containment: both gauge rows live INSIDE the local band — below
/// the legacy LOCAL text line, above the event ticker — and never touch the
/// status row.
#[test]
fn gauges_render_inside_the_local_band_between_strip_and_ticker() {
    let app = live_app_with_local(
        vec![ps(Some(Team::Order))],
        Some(local_player(
            Some(stats(Some(2100.0), Some(3000.0), Some(400.0), Some(500.0))),
            None,
        )),
    );
    let buffer = draw(&app);
    let height = buffer.area.height;

    let local_y = find_row(&buffer, "LOCAL").expect("legacy LOCAL strip line");
    let events_y = find_row(&buffer, "EVENTS").expect("ticker section");
    let hp_y = (0..height).find(|&y| row_text(&buffer, y).trim_end().starts_with("LOCAL HP"));
    let power_y = (0..height).find(|&y| row_text(&buffer, y).trim_end().starts_with("LOCAL Power"));

    assert!(hp_y.is_some(), "HP gauge row must render");
    assert!(power_y.is_some(), "Power gauge row must render");
    assert!(
        hp_y.expect("checked") > local_y && power_y.expect("checked") > local_y,
        "gauges sit below the LOCAL text line"
    );
    assert!(
        hp_y.expect("checked") < events_y && power_y.expect("checked") < events_y,
        "gauges stay inside the band above the ticker"
    );
    assert!(events_y < height - 1, "status row untouched");
}

// --- Task 4.7 / viz spec R8: local gold sparkline ring buffer ----------------

/// The gold-trend row leads with its explicit `GOLD` label at column zero.
/// Uppercase by contract: roster scans and the legacy strip's lowercase
/// `Gold` token can never collide with it.
fn unique_gold_row(buffer: &Buffer) -> String {
    let matches: Vec<String> = (0..buffer.area.height)
        .map(|y| row_text(buffer, y))
        .filter(|row| row.trim_end().starts_with("GOLD"))
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "exactly one GOLD trend row may exist (gold stays on the local strip)"
    );
    matches.into_iter().next().expect("exactly one")
}

/// A fully gauged local player carrying `gold`; advancing game times keep
/// every successive fold classified SameGame (design D1).
fn snapshot_with_gold(gold: Option<f64>, game_time: f64) -> Snapshot {
    Snapshot {
        players: Vec::new(),
        local: Some(local_player(
            Some(stats(Some(2100.0), Some(3000.0), Some(400.0), Some(500.0))),
            gold,
        )),
        game: Some(GameInfo {
            game_mode: Some("classic".to_owned()),
            game_time: Some(game_time),
            map_name: None,
        }),
        events: Vec::new(),
    }
}

/// An in-game app whose gold history folded one snapshot per entry of
/// `golds`: `Some(g)` carries real gold, `None` is an absent value in the
/// payload (which folds to a gap exactly like a failed poll, design D6).
fn app_with_gold_series(golds: &[Option<f64>]) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    let mut game_time = 60.0;
    for gold in golds {
        app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
            *gold, game_time,
        ))));
        game_time += 10.0;
    }
    app
}

/// viz:R8/S1 + design D6 — below two REAL samples the widget shows an
/// explicit warm-up placeholder instead of a chart: one real sample reads
/// `warming up (1/120)`, and gaps or absent gold never count as progress.
#[test]
fn warm_up_placeholder_shows_until_two_real_samples_exist() {
    // Exactly one real sample.
    let app = app_with_gold_series(&[Some(4350.0)]);
    let row = unique_gold_row(&draw(&app));
    assert!(
        row.contains("warming up (1/120)"),
        "one real sample must warm up explicitly: {row:?}"
    );
    assert!(
        !row.chars()
            .any(|c| SPARKLINE_LEVELS.contains(&c.to_string().as_str())),
        "warm-up must never draw a chart line: {row:?}"
    );

    // Absent gold is not a sample: zero real points so far.
    let app = app_with_gold_series(&[None]);
    let row = unique_gold_row(&draw(&app));
    assert!(
        row.contains("warming up (0/120)"),
        "absent gold must count as no progress: {row:?}"
    );

    // Gaps between real samples do not inflate the count either.
    let app = app_with_gold_series(&[Some(10.0), None]);
    let row = unique_gold_row(&draw(&app));
    assert!(
        row.contains("warming up (1/120)"),
        "a gap must never count as a real sample: {row:?}"
    );
}

/// viz:R8 — from two real samples on, the trend draws over the ring-buffer
/// window with auto-maximum scaling: distinct golds yield distinct ramp
/// glyphs and the placeholder text disappears.
#[test]
fn sparkline_draws_the_trend_once_two_real_samples_exist() {
    let app = app_with_gold_series(&[Some(100.0), Some(200.0), Some(300.0)]);
    let row = unique_gold_row(&draw(&app));

    assert!(
        !row.contains("warming"),
        "a drawn trend must not claim to be warming up: {row:?}"
    );
    let ramp_glyphs: Vec<char> = row
        .chars()
        .filter(|c| SPARKLINE_LEVELS.contains(&c.to_string().as_str()))
        .collect();
    assert!(
        ramp_glyphs.len() >= 2,
        "three rising golds must draw a multi-bar trend: {row:?}"
    );
    assert_ne!(
        ramp_glyphs.first(),
        ramp_glyphs.last(),
        "rising golds must scale through the ramp: {ramp_glyphs:?}"
    );
}

/// Failed polls insert GAPS, not points (design D4): the gap renders as a
/// visible light-shade break between bars — never silently bridged.
#[test]
fn failed_polls_render_visible_breaks_in_the_trend() {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
        Some(5000.0),
        60.0,
    ))));
    // The failed poll cycle records an explicit gap in the history…
    app.on_msg(PollMsg::Transient(TransientReason::Timeout));
    // …and polling resumes with real samples around it.
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
        Some(6000.0),
        70.0,
    ))));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
        Some(7000.0),
        80.0,
    ))));

    let row = unique_gold_row(&draw(&app));
    assert_eq!(
        row.chars().filter(|c| *c == '\u{2591}').count(),
        1,
        "the single failed poll must appear as exactly one visible break: {row:?}"
    );
    assert!(
        row.chars()
            .any(|c| SPARKLINE_LEVELS.contains(&c.to_string().as_str())),
        "real samples around the gap must still draw bars: {row:?}"
    );
}

/// Past the chart's on-screen width the trend must track the PRESENT: the
/// ring buffer keeps the newest 120 samples, the chart shows the newest
/// slice of those, and the most recent gold lands as a full-height bar at
/// the right edge — never silently scrolled off.
#[test]
fn sparkline_tracks_the_newest_samples_beyond_the_chart_width() {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    let mut game_time = 60.0;
    // 130 polls: more than both the buffer capacity and the 115-chart cells.
    for i in 0..130 {
        let gold = if i == 129 {
            Some(1_000_000.0)
        } else {
            Some(10.0)
        };
        app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
            gold, game_time,
        ))));
        game_time += 1.0;
    }

    let row = unique_gold_row(&draw(&app));
    let chart: Vec<char> = row.chars().skip("GOLD ".len()).collect();
    let rightmost = chart
        .iter()
        .rposition(|c| SPARKLINE_LEVELS.contains(&c.to_string().as_str()))
        .expect("the freshest sample must still be drawn");
    assert_eq!(
        chart[rightmost], '\u{2588}',
        "the latest gold must render full-height at the trend head: {row:?}"
    );
}

/// Glyph-safety net scoped to the new family (viz:R1/S2): with gauges, a
/// gapped trend, and placeholder rows all on screen, every local-strip row
/// carries whitelisted codepoints or printable ASCII only.
#[test]
fn local_strip_rows_stay_within_the_glyph_whitelist() {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(
        Some(100.0),
        60.0,
    ))));
    app.on_msg(PollMsg::Transient(TransientReason::Timeout));
    app.on_msg(PollMsg::Snapshot(Box::new(snapshot_with_gold(None, 70.0))));
    // A degraded gauge joins the frame too (power fields absent).
    app.on_msg(PollMsg::Snapshot(Box::new(Snapshot {
        players: Vec::new(),
        local: Some(local_player(
            Some(stats(Some(1500.0), Some(3000.0), None, None)),
            Some(250.0),
        )),
        game: Some(GameInfo {
            game_mode: Some("classic".to_owned()),
            game_time: Some(80.0),
            map_name: None,
        }),
        events: Vec::new(),
    })));
    let buffer = draw(&app);

    let whitelist: Vec<char> = tui_lol::glyphs::ALL_CODEPOINTS.to_vec();
    for y in 0..buffer.area.height {
        let row = row_text(&buffer, y);
        let trimmed = row.trim_end();
        if !(trimmed.starts_with("LOCAL HP")
            || trimmed.starts_with("LOCAL Power")
            || trimmed.starts_with("GOLD"))
        {
            continue;
        }
        for c in trimmed.chars() {
            assert!(
                c.is_ascii_graphic() || c == ' ' || whitelist.contains(&c),
                "local-strip glyph {c:?} (U+{:04X}) is not conhost-safe",
                c as u32
            );
        }
    }
}
