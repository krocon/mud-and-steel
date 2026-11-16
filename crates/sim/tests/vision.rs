//! Vision and fog of war (prompt 018): sight lines over hills, into forests
//! and past buildings, cell states, combat targets and the update budget.

use sim::map::MapLayout;
use sim::{
    Catalog, CellState, CombatEvent, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand,
    PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, VisionRules,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;

/// 800 m × 800 m around the origin, 2 m samples; `height(x, y)` in mm from
/// meters; optional objects (CSV rows without header).
fn map_with(height: impl Fn(i32, i32) -> i32, objects: &str) -> Arc<Map> {
    let n = 401;
    let heights = (0..n)
        .flat_map(|j| (0..n).map(move |i| (i, j)))
        .map(|(i, j): (i32, i32)| height(i * 2 - 400, j * 2 - 400))
        .collect();
    let layout = MapLayout {
        name: "vision_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, vec![0; (n * n) as usize]).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!("{}\n{objects}", sim::objects::CSV_HEADER))
            .unwrap();
    }
    Arc::new(map)
}

/// A hill along the x axis: 10 m high at y = 0, flat beyond |y| = 100 m
/// (10 % grade, drivable).
fn hill(_: i32, y: i32) -> i32 {
    (10_000 - y.abs() * 100).max(0)
}

fn flat(_: i32, _: i32) -> i32 {
    0
}

fn config(
    map: &Map,
    combat: CombatRules,
    vision: VisionRules,
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
) -> SimConfig {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat,
        vision,
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    }
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
    let config = config(
        &map,
        CombatRules::builtin().unwrap(),
        VisionRules::builtin().unwrap(),
        spawns,
    );
    Sim::new(&config, map, 7).unwrap()
}

const TANK: &str = "de_pz4_h";
const ENEMY: &str = "su_t34_76_1943";

/// Whether P0 (a tank at `(0, 200)`) sees a P1 tank at `(x, y)`.
fn p0_sees(map: Arc<Map>, x: i32, y: i32) -> bool {
    let sim = sim_with(map, &[(P0, TANK, 0, 200, HOLD), (P1, ENEMY, x, y, HOLD)]);
    let seen = sim.visible_to(P0, UnitId(1));
    // Vision is mutual here: both are tanks of similar height.
    assert_eq!(seen, sim.visible_to(P1, UnitId(0)));
    // Own units are always visible.
    assert!(sim.visible_to(P0, UnitId(0)) && sim.visible_to(P1, UnitId(1)));
    seen
}

#[test]
fn a_unit_behind_the_hill_is_hidden() {
    assert!(!p0_sees(map_with(hill, ""), 0, -150));
    assert!(!p0_sees(map_with(hill, ""), 40, -200));
}

#[test]
fn a_unit_on_the_hill_is_seen() {
    assert!(p0_sees(map_with(hill, ""), 0, 0));
    assert!(p0_sees(map_with(hill, ""), 0, 40));
    // Just behind the crest the hill still hides a tank from below.
    assert!(!p0_sees(map_with(hill, ""), 0, -24));
}

#[test]
fn flat_ground_hides_nothing_within_range_but_everything_beyond() {
    assert!(p0_sees(map_with(flat, ""), 0, -150));
    assert!(p0_sees(map_with(flat, ""), 250, -150));
    // 500 m sight (2 km real × 1/4): 560 m is out of range.
    assert!(!p0_sees(map_with(flat, ""), 0, -360));
}

/// A slope falling at 100 % from `top` meters at y = 200 to the plain
/// at 0 (prompt 060).
fn slope_from(top: i32) -> impl Fn(i32, i32) -> i32 {
    move |_, y| (y - 200 + top).clamp(0, top) * 1000
}

/// Seeing downhill (prompt 060): a tank on a 100 m height at `(0, 200)`
/// sees a tank on the plain beyond its 500 m sight range (up to 25 % more),
/// but not the other way round, and a height of 15 m gives no bonus.
#[test]
fn a_height_sees_farther_downhill() {
    let seen = |top: i32, x: i32, y: i32| {
        let sim = sim_with(
            map_with(slope_from(top), ""),
            &[(P0, TANK, 0, 200, HOLD), (P1, ENEMY, x, y, HOLD)],
        );
        (sim.visible_to(P0, UnitId(1)), sim.visible_to(P1, UnitId(0)))
    };
    // 560 m and 609 m away, 100 m below: seen from above only.
    assert_eq!(seen(100, 0, -360), (true, false));
    assert_eq!(seen(100, 300, -330), (true, false));
    // 15 m below: no bonus, out of range both ways.
    assert_eq!(seen(15, 0, -360), (false, false));
    // Within the sight range nothing changes.
    assert_eq!(seen(100, 0, -200), (true, true));
}

/// A forest band between y = -20 and y = 20 m: trees every 4 m.
fn forest() -> String {
    let mut rows = Vec::new();
    for x in (-200..=200).step_by(4) {
        for y in (-20..=20).step_by(4) {
            rows.push(format!(
                "env_tree_deciduous,Oak,{},{},0",
                x * 1000 + 1000,
                y * 1000 + 1000
            ));
        }
    }
    rows.join("\n")
}

#[test]
fn the_forest_edge_is_seen_but_not_its_depth() {
    let map = map_with(flat, &forest());
    // The first two cells (16 m) into the forest are visible, both ways …
    assert!(p0_sees(Arc::clone(&map), 0, 18));
    assert!(p0_sees(Arc::clone(&map), 0, 12));
    // … deeper in and behind it nothing is.
    assert!(!p0_sees(Arc::clone(&map), 0, -12));
    assert!(!p0_sees(map, 0, -100));
}

#[test]
fn a_hill_looks_over_the_canopy() {
    // The observer stands on a 50 % slope rising north of the forest, 85 m
    // above it; the target stands 230 m behind the forest.
    let map = map_with(|_, y| (y - 30).max(0) * 500, &forest());
    assert!(p0_sees(Arc::clone(&map), 0, -250));
    // From the plain the forest hides it.
    assert!(!p0_sees(map_with(flat, &forest()), 0, -250));
}

#[test]
fn a_house_hides_what_stands_behind_it() {
    // A farmhouse (14 m × 9 m × 8 m) 100 m in front of the observer.
    let map = map_with(flat, "env_house_farm,Intact,0,100000,90");
    assert!(!p0_sees(Arc::clone(&map), 0, 60));
    assert!(p0_sees(map, 60, 60));
}

#[test]
fn cells_go_from_unexplored_to_visible_to_explored() {
    let map = map_with(flat, "");
    let mut sim = sim_with(
        Arc::clone(&map),
        &[(P0, TANK, -300, 300, HOLD), (P1, ENEMY, 300, -300, HOLD)],
    );
    let grid = sim.vision_grid().clone();
    let far = grid.index_of(Vec2::from_meters(300, -300)) as usize;
    let home = grid.index_of(Vec2::from_meters(-300, 300)) as usize;
    let mut states = Vec::new();
    sim.vision_states(P0, &mut states);
    assert_eq!(states[far], CellState::Unexplored);
    assert_eq!(states[home], CellState::Visible);
    let version = sim.vision_version(P0);

    // Drive to the other corner.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(250, -250),
        },
    )]);
    for _ in 0..3000 {
        sim.step(&[]);
    }
    sim.vision_states(P0, &mut states);
    assert_eq!(states[far], CellState::Visible);
    assert_eq!(states[home], CellState::Explored);
    assert!(sim.vision_version(P0) > version);
    // The other player is now in sight; its own vision is separate.
    assert!(sim.visible_to(P0, UnitId(1)));
    sim.vision_states(P1, &mut states);
    assert_eq!(states[far], CellState::Visible);
}

#[test]
fn hidden_targets_draw_no_fire_and_seen_ones_do() {
    let shots = |y: i32| {
        let mut sim = sim_with(
            map_with(hill, ""),
            &[(P0, TANK, 0, 200, FIRE), (P1, ENEMY, 0, y, HOLD)],
        );
        let mut shots = 0;
        for _ in 0..200 {
            sim.step(&[]);
            shots += sim
                .events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::Shot { .. }))
                .count();
        }
        shots
    };
    // Within weapon range in both cases.
    assert_eq!(shots(-120), 0);
    assert!(shots(0) > 0);
}

#[test]
fn snapshots_show_only_what_the_player_sees() {
    let sim = sim_with(
        map_with(hill, ""),
        &[
            (P0, TANK, 0, 200, HOLD),
            (P1, ENEMY, 0, -150, HOLD),
            (P1, ENEMY, 0, 0, HOLD),
        ],
    );
    let ids = |player| -> Vec<u32> {
        sim.snapshot_for(player)
            .units
            .iter()
            .map(|u| u.id.0)
            .collect()
    };
    assert_eq!(ids(P0), vec![0, 2]);
    assert_eq!(ids(P1), vec![0, 1, 2]);
    assert_eq!(sim.snapshot().units.len(), 3);
    // Reused buffers (the bridge, prompt 016) give the same snapshots.
    let mut reused = sim.snapshot();
    sim.snapshot_into(Some(P0), &mut reused);
    assert_eq!(reused, sim.snapshot_for(P0));
    sim.snapshot_into(None, &mut reused);
    assert_eq!(reused, sim.snapshot());
}

#[test]
fn attack_orders_follow_a_target_out_of_sight() {
    // A T-34 attacks a Tiger on the hill; the Tiger's front holds and it
    // has plenty of hit points, so the duel lasts.
    let mut combat = CombatRules::builtin().unwrap();
    combat.damage.vehicle_hp = 1_000_000;
    let map = map_with(hill, "");
    let config = config(
        &map,
        combat,
        VisionRules::builtin().unwrap(),
        &[(P0, ENEMY, 0, -200, HOLD), (P1, "de_tiger1_e", 0, 0, HOLD)],
    );
    let mut sim = Sim::new(&config, map, 7).unwrap();
    assert!(sim.visible_to(P0, UnitId(1)));
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    // The Tiger drives down behind the hill.
    sim.step(&[PlayerCommand::new(
        P1,
        Command::MoveTo {
            units: vec![UnitId(1)],
            target: Vec2::from_meters(0, 200),
        },
    )]);
    let mut hidden_at = None;
    let mut shots_after = 0;
    for _ in 0..2000 {
        sim.step(&[]);
        if hidden_at.is_none() && !sim.visible_to(P0, UnitId(1)) {
            hidden_at = Some(sim.tick());
        }
        if hidden_at.is_some() {
            shots_after += sim
                .events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::Shot { unit, .. } if *unit == UnitId(0)))
                .count();
        }
    }
    assert!(hidden_at.is_some(), "the Tiger never left the T-34's sight");
    // The T-34 drove up the hill to where it last saw the Tiger, found it
    // again and kept firing.
    let t34 = sim.snapshot().units[0];
    assert!(t34.position.y > Fixed::from_num(-100), "{:?}", t34.position);
    assert!(shots_after > 0);
}

#[test]
fn orders_against_unseen_units_are_dropped() {
    let mut sim = sim_with(
        map_with(hill, ""),
        &[(P0, TANK, 0, 200, HOLD), (P1, ENEMY, 0, -150, HOLD)],
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    for _ in 0..100 {
        sim.step(&[]);
        assert!(sim.events().is_empty());
    }
    assert_eq!(sim.snapshot().units[0].target, None);
    assert_eq!(sim.snapshot().units[0].position, Vec2::from_meters(0, 200));
}

#[test]
fn the_budget_spreads_updates_over_ticks() {
    let map = map_with(flat, "");
    let mut vision = VisionRules::builtin().unwrap();
    vision.grid.budget_steps_per_tick = 1;
    let spawns: Vec<_> = (0..10)
        .map(|i| (P0, TANK, -200 + i * 20, 0, HOLD))
        .chain([(P1, ENEMY, 0, -300, HOLD)])
        .collect();
    let config = config(&map, CombatRules::builtin().unwrap(), vision, &spawns);
    let mut sim = Sim::new(&config, map, 7).unwrap();
    // At the start every viewshed is computed at once.
    assert_eq!(sim.vision_queue_len(), 0);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: (0..10).map(UnitId).collect(),
            target: Vec2::from_meters(0, 200),
        },
    )]);
    let mut longest = 0;
    let mut largest_step = 0;
    for _ in 0..400 {
        sim.step(&[]);
        longest = longest.max(sim.vision_queue_len());
        largest_step = largest_step.max(sim.vision_steps());
    }
    // Several units waited, and no tick computed more than one viewshed
    // (62 cells radius: at most 8 × 62 × 62 ray steps).
    assert!(longest >= 2, "queue never grew: {longest}");
    assert!(largest_step <= 8 * 62 * 62, "{largest_step}");
}

#[test]
fn same_seed_gives_same_vision() {
    let run = || {
        let mut sim = sim_with(
            map_with(hill, &forest()),
            &[
                (P0, TANK, -100, 200, FIRE),
                (P0, TANK, 100, 200, FIRE),
                (P1, ENEMY, 0, -200, FIRE),
                (P1, ENEMY, 50, -150, FIRE),
            ],
        );
        for player in [P0, P1] {
            sim.step(&[PlayerCommand::new(
                player,
                Command::MoveTo {
                    units: vec![UnitId(u32::from(player.0) * 2)],
                    target: Vec2::from_meters(0, 0),
                },
            )]);
        }
        for _ in 0..600 {
            sim.step(&[]);
        }
        let mut states = Vec::new();
        sim.vision_states(P0, &mut states);
        (sim.state_hash(), states)
    };
    assert_eq!(run(), run());
}
