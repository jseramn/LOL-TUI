//! Team-column roster: identity lines only (no per-player viz bars).
//!
//! Each player is one Spanish identity row (role, champion, level, KDA,
//! farm, death tag). Chart helpers stay unit-tested for the local strip
//! / degradation math; they are not drawn on the roster.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tui_lol::api::poller::{Lifecycle, PollMsg};
use tui_lol::app::App;
use tui_lol::model::snapshot::{ItemSnapshot, PlayerSnapshot, Snapshot, Team};
use tui_lol::ui::select_layout;
use tui_lol::ui::team;

fn ps(team: Option<Team>, creep_score: Option<f64>) -> PlayerSnapshot {
    PlayerSnapshot {
        summoner_name: None,
        champion: None,
        team,
        position: None,
        level: None,
        kills: None,
        deaths: None,
        assists: None,
        creep_score,
        ward_score: None,
        items: None,
        spell_one: None,
        spell_two: None,
        is_dead: None,
        respawn_timer: None,
    }
}

fn named(team: Team, champion: &str, position: &str, cs: f64) -> PlayerSnapshot {
    let mut player = ps(Some(team), Some(cs));
    player.champion = Some(champion.to_owned());
    player.position = Some(position.to_owned());
    player.level = Some(10);
    player.kills = Some(1);
    player.deaths = Some(0);
    player.assists = Some(2);
    player
}

fn live_app_with(players: Vec<PlayerSnapshot>) -> App {
    let mut app = App::new();
    app.on_msg(PollMsg::Lifecycle(Lifecycle::InGame));
    app.on_msg(PollMsg::Snapshot(Box::new(Snapshot {
        players,
        ..Snapshot::default()
    })));
    app
}

fn draw_at(app: &App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("test backend");
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

fn column_segment(buffer: &Buffer, column: Rect, y: u16) -> String {
    (column.x..column.x + column.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn lv_viz_rows(buffer: &Buffer, columns: tui_lol::ui::TeamColumns) -> Vec<String> {
    let mut rows = Vec::new();
    for y in 0..buffer.area.height {
        let order = column_segment(buffer, columns.order, y);
        if order.trim_start().starts_with("Lv") {
            rows.push(order);
        }
        let chaos = column_segment(buffer, columns.chaos, y);
        if chaos.trim_start().starts_with("Lv") {
            rows.push(chaos);
        }
    }
    rows
}

fn identity_rows(buffer: &Buffer, column: Rect, body: Rect) -> Vec<String> {
    (body.y..body.bottom())
        .map(|y| column_segment(buffer, column, y))
        .filter(|row| {
            let trimmed = row.trim();
            !trimmed.is_empty()
                && !trimmed.contains("Equipo ")
                && !trimmed.starts_with("Lv")
                && !trimmed.starts_with('│')
        })
        .collect()
}

fn five_a_side() -> Vec<PlayerSnapshot> {
    vec![
        named(Team::Order, "Aatrox", "TOP", 200.0),
        named(Team::Order, "Lee Sin", "JUNGLE", 150.0),
        named(Team::Order, "Ahri", "MIDDLE", 180.0),
        named(Team::Order, "Jinx", "BOTTOM", 220.0),
        named(Team::Order, "Lulu", "UTILITY", 40.0),
        named(Team::Chaos, "Darius", "TOP", 190.0),
        named(Team::Chaos, "Graves", "JUNGLE", 160.0),
        named(Team::Chaos, "Syndra", "MIDDLE", 210.0),
        named(Team::Chaos, "Kai'Sa", "BOTTOM", 200.0),
        named(Team::Chaos, "Thresh", "UTILITY", 45.0),
    ]
}

fn assert_identity_roster(width: u16, height: u16) {
    let app = live_app_with(five_a_side());
    let buffer = draw_at(&app, width, height);
    let layout = select_layout(Rect::new(0, 0, width, height));
    assert_eq!(
        layout.areas.body.height, 6,
        "{width}x{height}: body is title + five identity rows"
    );
    assert!(
        lv_viz_rows(&buffer, layout.columns).is_empty(),
        "{width}x{height}: roster must not draw Lv viz rows"
    );

    let order = identity_rows(&buffer, layout.columns.order, layout.areas.body);
    let chaos = identity_rows(&buffer, layout.columns.chaos, layout.areas.body);
    assert_eq!(
        order.len(),
        5,
        "{width}x{height} ORDER identity rows: {order:?}"
    );
    assert_eq!(
        chaos.len(),
        5,
        "{width}x{height} CHAOS identity rows: {chaos:?}"
    );
    for row in order.iter().chain(chaos.iter()) {
        assert!(
            row.contains("nivel"),
            "{width}x{height} identity missing nivel: {row:?}"
        );
        assert!(
            !row.contains("Lv█") && !row.contains("Lv░"),
            "{width}x{height} identity must not be a viz row: {row:?}"
        );
    }
}

#[test]
fn roster_is_identity_only_at_canonical_80x24() {
    assert_identity_roster(80, 24);
}

#[test]
fn roster_is_identity_only_at_wide_120x32() {
    assert_identity_roster(120, 32);
}

#[test]
fn order_and_chaos_identity_rows_share_physical_rows() {
    let app = live_app_with(vec![
        named(Team::Order, "Aatrox", "TOP", 9.0),
        named(Team::Chaos, "Darius", "TOP", 9.0),
    ]);
    let buffer = draw_at(&app, 240, 24);
    let layout = select_layout(Rect::new(0, 0, 240, 24));
    let mid = usize::from(layout.areas.body.x + layout.areas.body.width / 2);
    let side_by_side = (layout.areas.body.y..layout.areas.body.bottom()).any(|y| {
        let row: Vec<char> = row_text(&buffer, y).chars().collect();
        let left = row.iter().collect::<String>();
        let right = row.get(mid..).map(|c| c.iter().collect::<String>());
        left.contains("Aatrox") && right.is_some_and(|text| text.contains("Darius"))
    });
    assert!(
        side_by_side,
        "ORDER and CHAOS identity lines must share a physical row"
    );
}

#[test]
fn team_visualizations_do_not_draw_lv_rows() {
    let app = live_app_with(vec![ps(Some(Team::Order), Some(12.0))]);
    let buffer = draw_at(&app, 240, 24);
    let ys: Vec<u16> = (0..buffer.area.height)
        .filter(|&y| row_text(&buffer, y).starts_with("Lv"))
        .collect();
    assert!(
        ys.is_empty(),
        "no Lv visualization rows belong on the roster: {ys:?}"
    );
    let header = (0..buffer.area.height).find(|&y| row_text(&buffer, y).contains("Equipo Orden"));
    assert!(header.is_some(), "team header must still render");
}

#[test]
fn level_fill_cells_follows_the_fixed_mapping_exactly() {
    assert_eq!(team::level_fill_cells(1), 0);
    assert_eq!(team::level_fill_cells(18), 10);
    assert_eq!(team::level_fill_cells(0), 0);
    assert_eq!(team::level_fill_cells(25), 10);
    assert_eq!(team::level_fill_cells(u32::MAX), 10);
}

#[test]
fn scaled_cells_rounds_half_up_against_a_shared_max() {
    assert_eq!(team::scaled_cells(0, 10, 8), 0);
    assert_eq!(team::scaled_cells(5, 10, 8), 4);
    assert_eq!(team::scaled_cells(10, 10, 8), 8);
    assert_eq!(team::scaled_cells(1, 0, 8), 0);
    assert_eq!(team::scaled_cells(3, 4, 8), 6);
}

fn item(slot: Option<u8>, item_id: Option<u32>) -> ItemSnapshot {
    ItemSnapshot {
        display_name: item_id.map(|_| "Item".to_owned()),
        item_id,
        count: Some(1),
        slot,
    }
}

#[test]
fn inventory_occupied_skips_trinket_and_null_slots() {
    let partial = vec![
        item(Some(0), Some(3006)),
        item(Some(1), Some(6672)),
        item(Some(2), Some(3153)),
        item(Some(3), None),
        item(Some(4), None),
    ];
    assert_eq!(team::inventory_occupied(Some(&partial)), Some(3));

    let with_trinket = vec![
        item(Some(0), Some(3006)),
        item(Some(6), Some(2055)),
        item(None, Some(1039)),
    ];
    assert_eq!(team::inventory_occupied(Some(&with_trinket)), Some(2));

    assert_eq!(team::inventory_occupied(None), None);
    assert_eq!(team::inventory_occupied(Some(&[])), Some(0));

    let oversized: Vec<_> = (0..=7).map(|slot| item(Some(slot), Some(3001))).collect();
    assert_eq!(team::inventory_occupied(Some(&oversized)), Some(6));
}
