//! Heavy vehicles push trees over (prompt 106): limits by trunk thickness
//! (model variant), dense stands stay closed, felled trees fall the way the
//! vehicle drives, free the sight and the lane for everybody.

use sim::map::MapLayout;
use sim::{
    Catalog, Command, FireMode, Map, ObjectCause, ObjectEvent, ObjectStage, PlayerCommand,
    PlayerId, Remains, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, VisionRules,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
/// Tiger I Ausf. E, 57 t: fells thin and thick trunks.
const TIGER: &str = "de_tiger1_e";
/// M4A3(75)W, 31.6 t: fells thin trunks only.
const SHERMAN: &str = "us_m4a3_75";
/// M5A1 light tank, 15.2 t: fells no tree.
const STUART: &str = "us_m5a1";
/// GMC CCKW truck, 7.4 t.
const TRUCK: &str = "us_gmc_cckw";
const ENEMY: &str = "de_inf_rifle_squad";
const HOLD: FireMode = FireMode::HoldFire;

/// 800 m × 800 m flat map around the origin with the given objects (CSV
/// rows without header).
fn map_with(objects: &str) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "tree_crush_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap();
    map.add_objects(&format!("{}\n{objects}", sim::objects::CSV_HEADER))
        .unwrap();
    Arc::new(map)
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: HOLD,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: sim::CombatRules::builtin().unwrap(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).unwrap()
}

fn move_to(unit: u32, x: i32, y: i32) -> PlayerCommand {
    PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![UnitId(unit)],
            target: Vec2::from_meters(x, y),
        },
    )
}

fn position(sim: &Sim, unit: u32) -> Vec2 {
    sim.unit_position(UnitId(unit)).unwrap()
}

/// Sends unit `unit` to `(x, y)` and steps until it stands there (within
/// 2 m); returns the object events on the way and the distance it drove.
fn drive(sim: &mut Sim, unit: u32, x: i32, y: i32) -> (Vec<ObjectEvent>, i32) {
    sim.step(&[move_to(unit, x, y)]);
    let goal = Vec2::from_meters(x, y);
    let mut events = sim.object_events().to_vec();
    let mut driven = sim::Fixed::ZERO;
    let mut last = position(sim, unit);
    for _ in 0..8000 {
        sim.step(&[]);
        events.extend_from_slice(sim.object_events());
        let now = position(sim, unit);
        driven += (now - last).length();
        last = now;
        if (now - goal).length() < sim::Fixed::from_num(2) {
            return (events, driven.to_num());
        }
    }
    panic!("unit {unit} did not arrive: it stands at {last:?}");
}

/// A row of trees `variant` (of catalog id `id`) along y = `y` m, 4 m
/// apart from x = `from` × 4 m to `to` × 4 m: closed to vehicles (every
/// 4 m path cell along it holds a trunk), but only 4 neighbors within
/// 10 m each, so no dense stand.
fn row(id: &str, variant: &str, y: i32, from: i32, to: i32) -> String {
    (from..=to)
        .map(|k| format!("{id},{variant},{},{},0", k * 4000, y * 1000))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Two rows of young oaks (thick trunks) 300 m long at y = 0 and y = 12 m:
/// two forest cells of 8 m deep for sight, 12 m apart (no dense stand).
fn double_row(variant: &str) -> String {
    format!(
        "{}\n{}",
        row("env_tree_deciduous", variant, 0, -37, 37),
        row("env_tree_deciduous", variant, 12, -37, 37)
    )
}

#[test]
fn a_heavy_tank_breaks_through_thick_trees_and_others_drive_around() {
    let mut sim = sim_with(map_with(&double_row("OakYoung")), &[(P0, TIGER, 0, 60)], 7);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(
        driven < 120,
        "the Tiger drove {driven} m: not straight through"
    );
    assert!(!events.is_empty(), "no tree fell");
    assert!(events.len() <= 6, "a narrow lane: {} trees", events.len());
    for e in &events {
        assert_eq!(e.stage, ObjectStage::Destroyed);
        assert_eq!(e.remains, Remains::Fallen);
        assert_eq!(e.cause, ObjectCause::Crushed(UnitId(0)));
        // The tank drives north (-y): the trees fall north.
        assert!(
            e.fall_deg <= 20 || e.fall_deg >= 340,
            "fell to {}°",
            e.fall_deg
        );
        assert_eq!(sim.object_blocks(e.object), Some(sim::Blocks::None));
    }
    // Trees 20 m beside the lane stand.
    assert_eq!(sim.object_stage(37 + 5), Some(ObjectStage::Intact));
    assert_eq!(sim.object_stage(37 - 5), Some(ObjectStage::Intact));

    // A medium tank does not fell thick trunks, a truck no trees at all:
    // both drive around the end of the rows.
    for vehicle in [SHERMAN, TRUCK] {
        let mut sim = sim_with(
            map_with(&double_row("OakYoung")),
            &[(P0, vehicle, 0, 60)],
            7,
        );
        let (events, driven) = drive(&mut sim, 0, 0, -40);
        assert!(events.is_empty(), "{vehicle} felled trees: {events:?}");
        assert!(driven > 300, "{vehicle} drove only {driven} m");
    }
}

#[test]
fn mature_trees_stand_against_every_tank() {
    let mut sim = sim_with(map_with(&double_row("Oak")), &[(P0, TIGER, 0, 60)], 7);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(
        events.is_empty(),
        "the Tiger felled mature oaks: {events:?}"
    );
    assert!(driven > 300, "the Tiger drove only {driven} m");
}

#[test]
fn medium_tanks_fell_thin_trees_and_light_tanks_do_not() {
    let thin = row("env_tree_birch", "BirchYoung", 0, -37, 37);
    let mut sim = sim_with(map_with(&thin), &[(P0, SHERMAN, 0, 40)], 7);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(!events.is_empty() && driven < 100, "{driven} m, {events:?}");
    let mut sim = sim_with(map_with(&thin), &[(P0, STUART, 0, 40)], 7);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(events.is_empty(), "the light tank felled trees: {events:?}");
    assert!(driven > 300, "the light tank drove only {driven} m");
}

#[test]
fn the_lane_frees_the_sight_and_lets_a_truck_through() {
    // A German squad 210 m south, hidden behind the two forest rows.
    let mut sim = sim_with(
        map_with(&double_row("OakYoung")),
        &[
            (P0, TIGER, 2, 60),
            (P0, TRUCK, 30, 60),
            (P1, ENEMY, 2, -150),
        ],
        7,
    );
    sim.step(&[]);
    let grid = sim.vision_grid();
    let cells = [
        grid.index_of(Vec2::from_meters(2, 0)) as usize,
        grid.index_of(Vec2::from_meters(2, 12)) as usize,
    ];
    for &cell in &cells {
        assert_eq!(grid.blocker_at(cell), sim::vision::Sight::Foliage);
    }
    assert!(!sim.visible_to(P0, UnitId(2)), "the rows hide the squad");
    // Through the rows and back: the Tiger stands where it started.
    drive(&mut sim, 0, 2, -30);
    drive(&mut sim, 0, 2, 60);
    for &cell in &cells {
        assert_eq!(
            sim.vision_grid().blocker_at(cell),
            sim::vision::Sight::Clear,
            "the lane still counts as forest"
        );
    }
    sim.step(&[]);
    assert!(
        sim.visible_to(P0, UnitId(2)),
        "the lane does not free the sight"
    );
    // The truck takes the lane instead of the 150 m long way around.
    let (events, driven) = drive(&mut sim, 1, 2, -40);
    assert!(events.is_empty(), "the truck felled trees: {events:?}");
    assert!(
        driven < 140,
        "the truck drove {driven} m: not through the lane"
    );
}

/// A wood of young birches 5 m apart, 300 m wide and 20 m deep around
/// y = 0: every tree has 6 and more neighbors within 10 m (the corners 5).
fn dense_wood() -> String {
    let mut rows = Vec::new();
    for j in -2..=2 {
        for k in -30..=30 {
            rows.push(format!(
                "env_tree_birch,BirchYoung,{},{},0",
                k * 5000,
                j * 5000
            ));
        }
    }
    rows.join("\n")
}

#[test]
fn a_dense_wood_stays_closed_to_tanks() {
    let mut sim = sim_with(map_with(&dense_wood()), &[(P0, TIGER, 0, 40)], 7);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(driven > 300, "the Tiger drove only {driven} m");
    // Rounding the wood, the hull may clip a tree at its end (the corners
    // have fewer neighbors); none inside it falls.
    for e in &events {
        let x = e.at.x.to_num::<i32>().abs();
        assert!(x > 140, "the Tiger felled a tree in the wood: {e:?}");
    }
}

#[test]
fn a_tank_drives_around_a_short_row() {
    // A row 36 m long: going around its end is shorter than the detour a
    // felled tree costs.
    let short = row("env_tree_birch", "BirchYoung", 0, -4, 4);
    let mut sim = sim_with(map_with(&short), &[(P0, SHERMAN, 0, 40)], 7);
    let (events, _) = drive(&mut sim, 0, 0, -40);
    // At most the end tree it rounds falls, none in the middle.
    for e in &events {
        assert!(
            e.object == 0 || e.object == 8,
            "the tank broke through the row: {events:?}"
        );
    }
}

#[test]
fn a_tank_slows_down_while_it_fells_trees() {
    let run = |objects: &str| {
        let mut sim = sim_with(map_with(objects), &[(P0, TIGER, 0, 60)], 7);
        let start = sim.tick();
        drive(&mut sim, 0, 0, -40);
        sim.tick() - start
    };
    let free = run("");
    let through = run(&double_row("OakYoung"));
    assert!(
        through >= free + 10,
        "through the rows in {through} ticks, without them in {free}"
    );
    assert!(through <= free + 120, "{through} ticks against {free}");
}

#[test]
fn felling_trees_is_deterministic() {
    let run = || {
        let mut sim = sim_with(
            map_with(&double_row("OakYoung")),
            &[(P0, TIGER, 0, 60), (P0, SHERMAN, 20, 60)],
            11,
        );
        sim.step(&[move_to(0, 0, -40), move_to(1, 20, -40)]);
        for _ in 0..1500 {
            sim.step(&[]);
        }
        (sim.state_hash(), sim.changed_objects().count())
    };
    let (a, felled) = run();
    assert!(felled > 0);
    assert_eq!((a, felled), run());
}
