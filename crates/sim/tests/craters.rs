//! Terrain deformation tests (prompt 021): exact crater shapes on a flat
//! hand-made map, limits, water, path re-planning, infantry cover and
//! craters from real ground hits.

use sim::map::MapLayout;
use sim::path::SizeClass;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, RouteView, Scale, Sim,
    SimConfig, Spawn, TerrainRules, UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const TANK: &str = "de_pz4_h";
const SQUAD: &str = "de_inf_rifle_squad";
/// 7.5 cm: bowl radius 2 m (the minimum), depth 375 mm, rim 93 mm out to 3 m.
const KWK: u32 = 750;
const MUD: u8 = 4;
const DEEP_WATER: u8 = 7;
/// Samples per axis: 800 m × 800 m around the origin, 2 m apart.
const N: u32 = 401;
/// Sample index of the origin on both axes.
const C: u32 = 200;

/// Flat map at height 0; `terrain(i, j)` gives the terrain ids.
fn map_with(terrain: impl Fn(u32, u32) -> u8) -> Arc<Map> {
    let layout = MapLayout {
        name: "crater_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let ids = (0..N)
        .flat_map(|j| (0..N).map(move |i| (i, j)))
        .map(|(i, j)| terrain(i, j))
        .collect();
    Arc::new(Map::from_rasters(layout, vec![0; (N * N) as usize], ids).unwrap())
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let mut combat = CombatRules::builtin().unwrap();
    combat.dispersion.mrad_x10 = 0;
    combat.penetration.spread_pct = 0;
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: FireMode::HoldFire,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat,
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, 7).unwrap()
}

/// A tank far away from everything the tests look at.
fn quiet_sim(map: Arc<Map>) -> Sim {
    sim_with(map, &[(P0, TANK, -300, -300), (P1, TANK, 300, 300)])
}

fn height(sim: &Sim, i: u32, j: u32) -> i32 {
    sim.map().sample_height_mm(i, j)
}

fn crater(sim: &mut Sim, x_mm: i32, y_mm: i32, caliber: u32, times: u32) {
    for _ in 0..times {
        sim.queue_crater(Vec2::from_millimeters(x_mm, y_mm), caliber);
    }
    sim.step(&[]);
}

#[test]
fn a_crater_changes_the_expected_samples_exactly() {
    let map = map_with(|_, _| 0);
    let map_hash = map.hash();
    let mut sim = quiet_sim(Arc::clone(&map));
    let untouched = quiet_sim(map);
    crater(&mut sim, 0, 0, KWK, 1);

    assert_eq!(sim.crater_count(), 1);
    let events = sim.crater_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].samples, (C - 1, C - 1, C + 1, C + 1));
    assert_eq!(events[0].caliber_mm_x10, KWK);
    // Center: the full depth; 2 m away: the rim at the bowl's edge;
    // diagonal (2.828 m): 93 × (3000 - 2828) / 1000; 4 m away: nothing.
    assert_eq!(height(&sim, C, C), -375);
    for (i, j) in [(C + 1, C), (C - 1, C), (C, C + 1), (C, C - 1)] {
        assert_eq!(height(&sim, i, j), 93, "sample {i},{j}");
    }
    for (i, j) in [(C + 1, C + 1), (C - 1, C - 1), (C + 1, C - 1)] {
        assert_eq!(height(&sim, i, j), 15, "sample {i},{j}");
    }
    for (i, j) in [(C + 2, C), (C, C - 2), (C + 2, C + 2), (0, 0)] {
        assert_eq!(height(&sim, i, j), 0, "sample {i},{j}");
    }
    assert_eq!(sim.deformation_mm(C, C), -375);
    assert_eq!(sim.deformation_mm(C + 1, C), 93);
    // Only the bowl turns to mud.
    assert_eq!(sim.map().sample_terrain(C, C), MUD);
    assert_eq!(sim.map().sample_terrain(C + 1, C), 0);
    // The map keeps its identity; the state does not.
    assert_eq!(sim.map().hash(), map_hash);
    let mut idle = untouched;
    idle.step(&[]);
    assert_ne!(sim.state_hash(), idle.state_hash());
    assert_eq!(idle.crater_count(), 0);
    assert_eq!(idle.map().sample_height_mm(C, C), 0);
}

#[test]
fn a_crater_between_samples_is_symmetric() {
    // 1 m east of a sample: two samples 1 m away share the bowl.
    let mut sim = quiet_sim(map_with(|_, _| 0));
    crater(&mut sim, 1000, 0, KWK, 1);
    // -375 + (375 + 93) × 1/4 (d² = 1 m², r² = 4 m²).
    assert_eq!(height(&sim, C, C), -258);
    assert_eq!(height(&sim, C + 1, C), -258);
    // (±1 m, ±2 m): d = 2.236 m on the rim: 93 × 764 / 1000.
    for (i, j) in [(C, C + 1), (C + 1, C + 1), (C, C - 1), (C + 1, C - 1)] {
        assert_eq!(height(&sim, i, j), 71, "sample {i},{j}");
    }
    assert_eq!(sim.map().sample_terrain(C, C), MUD);
    assert_eq!(sim.map().sample_terrain(C + 1, C), MUD);
}

#[test]
fn repeated_hits_stop_at_the_depth_limit() {
    let mut sim = quiet_sim(map_with(|_, _| 0));
    crater(&mut sim, 0, 0, KWK, 10);
    assert_eq!(sim.crater_count(), 10);
    assert_eq!(height(&sim, C, C), -1500);
    assert_eq!(height(&sim, C + 1, C), 400);
    // More than the limit per tick: the rest makes no crater.
    crater(&mut sim, 40_000, 0, KWK, 20);
    assert_eq!(sim.crater_count(), 10 + 16);
    assert_eq!(sim.crater_events().len(), 16);
}

#[test]
fn small_calibers_and_water_leave_no_crater() {
    let water = |i: u32, _| if i >= C { DEEP_WATER } else { 0 };
    let mut sim = quiet_sim(map_with(water));
    crater(&mut sim, 10_000, 0, KWK, 1);
    crater(&mut sim, -10_000, 0, 200, 1);
    assert_eq!(sim.crater_count(), 0);
    assert_eq!(height(&sim, C + 5, C), 0);
    // On land next to the water: the bowl reaches the water, which keeps
    // its type (but not its height).
    crater(&mut sim, -1200, 0, KWK, 1);
    assert_eq!(sim.crater_count(), 1);
    assert_eq!(sim.map().sample_terrain(C - 1, C), MUD);
    assert_eq!(sim.map().sample_terrain(C, C), DEEP_WATER);
    // d = 0.8 m and 1.2 m: -375 + 468 × d² / 4 m².
    assert_eq!(height(&sim, C - 1, C), -301);
    assert_eq!(height(&sim, C, C), -207);
}

#[test]
fn a_steep_crater_blocks_cells_and_routes_plan_again() {
    let mut sim = quiet_sim(map_with(|_, _| 0));
    let tank = UnitId(0);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![tank],
            target: Vec2::from_meters(-300, 0),
        },
    )]);
    // Let the path arrive: the tank drives north along x = -300.
    for _ in 0..5 {
        sim.step(&[]);
    }
    assert!(matches!(sim.route_of(tank), Some(RouteView::Following(_))));
    let grid = sim.path_grid();
    let point = Vec2::from_meters(-300, -200);
    let (x, y) = grid.cell_of(point);
    assert!(grid.passable(SizeClass::Large, x, y));
    let requests = sim.path_stats().requests;
    // Ten 7.5 cm rounds in one spot: 1.9 m between the bottom and the rim
    // over 2 m, far steeper than 60 %.
    crater(&mut sim, -300_000, -200_000, KWK, 10);
    let grid = sim.path_grid();
    assert!(!grid.passable(SizeClass::Large, x, y));
    assert_eq!(sim.route_of(tank), Some(RouteView::Waiting));
    for _ in 0..3 {
        sim.step(&[]);
    }
    assert!(sim.path_stats().requests > requests);
    let Some(RouteView::Following(waypoints)) = sim.route_of(tank) else {
        panic!("no new route");
    };
    // The new route passes the crater beside it.
    assert!(waypoints.iter().all(|w| (*w - point).length() > 2));
}

#[test]
fn a_shallow_crater_changes_costs_but_no_route() {
    let mut sim = quiet_sim(map_with(|_, _| 0));
    let tank = UnitId(0);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![tank],
            target: Vec2::from_meters(-300, 0),
        },
    )]);
    for _ in 0..5 {
        sim.step(&[]);
    }
    let grid = sim.path_grid();
    let (x, y) = grid.cell_of(Vec2::from_meters(-300, -200));
    let cost = grid.costs(0)[(y * grid.size().0 + x) as usize];
    crater(&mut sim, -300_000, -200_000, KWK, 1);
    assert!(matches!(sim.route_of(tank), Some(RouteView::Following(_))));
    let grid = sim.path_grid();
    assert!(
        grid.costs(0)[(y * grid.size().0 + x) as usize] > cost,
        "mud costs more"
    );
}

#[test]
fn soldiers_take_cover_in_craters() {
    // A standing squad 6 m from where a 15 cm round struck, on open
    // ground (no object spots): four samples of the bowl are deep enough.
    let mut sim = sim_with(
        map_with(|_, _| 0),
        &[(P0, SQUAD, 0, 6), (P1, TANK, 300, 300)],
    );
    for _ in 0..20 {
        sim.step(&[]);
    }
    let covered = |sim: &Sim| {
        sim.snapshot()
            .soldiers
            .iter()
            .filter(|s| s.in_cover)
            .count()
    };
    assert_eq!(covered(&sim), 0);
    crater(&mut sim, 1000, 1000, 1500, 1);
    assert_eq!(sim.deformation_mm(C, C), -282);
    for _ in 0..200 {
        sim.step(&[]);
    }
    assert_eq!(covered(&sim), 4, "{:?}", sim.snapshot().soldiers);
    for s in sim.snapshot().soldiers.iter().filter(|s| s.in_cover) {
        let d = s.position - Vec2::from_meters(1, 1);
        assert!(d.length() < 2, "{:?}", s.position);
        // The soldiers stand on the lowered ground.
        assert!(s.height < 0);
    }
}

#[test]
fn ground_hits_of_heavy_rounds_make_craters() {
    let mut sim = sim_with(
        map_with(|_, _| 0),
        &[(P0, TANK, 0, 60), (P1, TANK, 300, 300)],
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, -60),
        },
    )]);
    let mut events = Vec::new();
    for _ in 0..400 {
        sim.step(&[]);
        events.extend_from_slice(sim.crater_events());
    }
    assert!(sim.crater_count() >= 2, "{}", sim.crater_count());
    assert_eq!(events.len() as u64, sim.crater_count());
    let first = events[0];
    assert_eq!(first.caliber_mm_x10, KWK);
    assert!(
        (first.at - Vec2::from_meters(0, -60)).length() < 2,
        "{:?}",
        first.at
    );
    // The ground there is lower now.
    let (i, j) = (C, C - 30);
    assert!(sim.deformation_mm(i, j) < 0);
}
