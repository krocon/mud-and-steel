//! Infantry ambush (prompt 056) [provisional]: squads standing silent in
//! woods are seen only by enemies close by, until they fire.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
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

fn sim_with(
    map: Arc<Map>,
    vision: VisionRules,
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
) -> Sim {
    let config = config(&map, CombatRules::builtin().unwrap(), vision, spawns);
    Sim::new(&config, map, 7).unwrap()
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

const RIFLES: &str = "de_inf_rifle_squad";
const ENEMY_RIFLES: &str = "su_inf_rifle_squad";

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

fn no_ambush() -> VisionRules {
    let mut rules = VisionRules::builtin().unwrap();
    rules.ambush.spot_real_m = 0;
    rules
}

/// A P0 squad at the forest edge (0, 18) and a P1 squad in the open at
/// `(0, y)`, both holding fire; whether P1 sees the P0 squad after 2 s.
fn ambush_seen(vision: VisionRules, y: i32) -> bool {
    let mut sim = sim_with(
        map_with(flat, &forest()),
        vision,
        &[(P0, RIFLES, 0, 18, HOLD), (P1, ENEMY_RIFLES, 0, y, HOLD)],
    );
    run(&mut sim, 40);
    sim.visible_to(P1, UnitId(0))
}

#[test]
fn a_squad_at_the_forest_edge_is_hidden_from_afar() {
    // Without the rule the forest edge is seen from 100 m …
    assert!(ambush_seen(no_ambush(), 120));
    // … with it the silent squad stays hidden …
    assert!(!ambush_seen(VisionRules::builtin().unwrap(), 120));
    // … until an enemy comes within 50 m (200 m real).
    assert!(ambush_seen(VisionRules::builtin().unwrap(), 60));
}

#[test]
fn the_first_shot_reveals_the_ambush() {
    let mut sim = sim_with(
        map_with(flat, &forest()),
        VisionRules::builtin().unwrap(),
        &[(P0, RIFLES, 0, 18, HOLD), (P1, ENEMY_RIFLES, 0, 120, HOLD)],
    );
    run(&mut sim, 40);
    assert!(!sim.visible_to(P1, UnitId(0)));
    sim.step(&[PlayerCommand::new(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 110),
        },
    )]);
    let mut seen_at = None;
    for t in 0..20 * 30 {
        sim.step(&[]);
        if sim.visible_to(P1, UnitId(0)) {
            seen_at = Some(t);
            break;
        }
    }
    assert!(seen_at.is_some(), "firing did not reveal the squad");
}

#[test]
fn a_squad_in_the_open_is_no_ambush() {
    // 30 m in front of the forest: seen as before.
    let sim = |vision| {
        let mut sim = sim_with(
            map_with(flat, &forest()),
            vision,
            &[(P0, RIFLES, 0, 52, HOLD), (P1, ENEMY_RIFLES, 0, 160, HOLD)],
        );
        run(&mut sim, 40);
        sim.visible_to(P1, UnitId(0))
    };
    assert!(sim(VisionRules::builtin().unwrap()));
}
