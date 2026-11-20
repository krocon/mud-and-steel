//! Orders and formations, stage 2 (prompt 046): attack-move, securing an
//! area, automatic formations and facing, on hand-made maps with catalog
//! units and fixed seeds.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, DrillView, Engage, FireMode, Fixed, Formation, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const TRUCK: &str = "de_opel_blitz";
const HALFTRACK: &str = "de_sdkfz251_1";
const SQUAD: &str = "de_inf_rifle_squad";
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "formations_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let mut ids = Vec::with_capacity((N * N) as usize);
    for j in 0..N as i32 {
        for i in 0..N as i32 {
            ids.push(terrain((i - half) * 2, (j - half) * 2));
        }
    }
    let size = (N * N) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], ids).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!(
            "{}\n{}\n",
            sim::objects::CSV_HEADER,
            objects.join("\n")
        ))
        .unwrap();
    }
    Arc::new(map)
}

fn flat() -> Arc<Map> {
    map_with(|_, _| 0, &[])
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
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
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

fn ids(list: &[u32]) -> Vec<UnitId> {
    list.iter().map(|&i| UnitId(i)).collect()
}

fn formation_move(
    units: &[u32],
    target: Vec2,
    formation: Formation,
    facing: Option<Vec2>,
    engage: Engage,
) -> PlayerCommand {
    PlayerCommand::new(
        P0,
        Command::FormationMove {
            units: ids(units),
            target,
            formation,
            facing,
            engage,
        },
    )
}

fn unit(sim: &Sim, id: u32) -> sim::UnitSnapshot {
    sim.snapshot()
        .units
        .into_iter()
        .find(|u| u.id == UnitId(id))
        .expect("unit exists")
}

fn m(v: i32) -> Fixed {
    Fixed::from_num(v)
}

/// Drives a Panzer IV past an unarmed truck with the given move mode;
/// returns whether it ever halted on the way, whether the truck was
/// destroyed, and where the tank ended. The truck stands 50 m beside the
/// path, inside the 87.5 m of the 75 mm gun (700 m real x 1/8); the tank
/// drives 300 m at 16 km/h off road (4.4 m/s, about 70 s of the 150 s).
fn past_the_truck(engage: Engage) -> (bool, bool, Vec2, Option<DrillView>) {
    let mut sim = sim_with(
        flat(),
        &[(P0, TANK, 0, -150, FIRE), (P1, TRUCK, 50, 0, HOLD)],
    );
    let target = Vec2::from_meters(0, 150);
    sim.step(&[formation_move(&[0], target, Formation::Auto, None, engage)]);
    let mut halted = false;
    for _ in 0..3000 {
        sim.step(&[]);
        if let Some(DrillView::Advance(_, true)) = sim.drill_view(UnitId(0)) {
            halted = true;
        }
        if unit(&sim, 0).target.is_none() && sim.drill_view(UnitId(0)).is_none() {
            break;
        }
    }
    (
        halted,
        !unit(&sim, 1).alive(),
        unit(&sim, 0).position,
        sim.drill_view(UnitId(0)),
    )
}

#[test]
fn attack_move_halts_to_fight_then_drives_on() {
    let (halted, destroyed, end, task) = past_the_truck(Engage::All);
    assert!(halted, "the tank halted while the truck was in range");
    assert!(destroyed, "and destroyed it");
    let target = Vec2::from_meters(0, 150);
    assert!(
        (end - target).length() < m(3),
        "then reached its target: {end:?}"
    );
    assert_eq!(task, None, "the attack-move is done");
}

#[test]
fn a_plain_move_never_halts() {
    let (halted, _, end, _) = past_the_truck(Engage::Never);
    assert!(!halted);
    assert!((end - Vec2::from_meters(0, 150)).length() < m(3));
}

#[test]
fn by_default_only_infantry_attack_moves() {
    let mut sim = sim_with(
        flat(),
        &[(P0, SQUAD, 0, -100, FIRE), (P0, HALFTRACK, 20, -100, FIRE)],
    );
    let target = Vec2::from_meters(0, 100);
    sim.step(&[formation_move(
        &[0, 1],
        target,
        Formation::Line,
        None,
        Engage::Infantry,
    )]);
    assert!(matches!(
        sim.drill_view(UnitId(0)),
        Some(DrillView::Advance(..))
    ));
    assert_eq!(sim.drill_view(UnitId(1)), None);
    // Any other order ends the attack-move.
    sim.step(&[PlayerCommand::new(P0, Command::Stop { units: ids(&[0]) })]);
    assert_eq!(sim.drill_view(UnitId(0)), None);
}

#[test]
fn securing_units_chase_inside_the_circle_and_return() {
    // The squad holds a circle of 150 m; the truck stands inside it, seen
    // (250 m sight) but out of range of the squad's machine gun (50 m, 400 m
    // real x 1/8).
    let mut sim = sim_with(
        flat(),
        &[(P0, SQUAD, 0, 0, FIRE), (P1, TRUCK, 135, 0, HOLD)],
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Secure {
            units: ids(&[0]),
            center: Vec2::ZERO,
            radius_m: 150,
            formation: Formation::Auto,
            facing: None,
        },
    )]);
    let mut farthest = Fixed::ZERO;
    for _ in 0..4000 {
        sim.step(&[]);
        farthest = farthest.max(unit(&sim, 0).position.length());
        if !unit(&sim, 1).alive() && unit(&sim, 0).position.length() < m(6) {
            break;
        }
    }
    assert!(
        !unit(&sim, 1).alive(),
        "the truck in the circle was destroyed"
    );
    assert!(farthest > m(30), "the squad went after it ({farthest})");
    assert!(farthest < m(150), "but not out of the circle ({farthest})");
    for _ in 0..600 {
        sim.step(&[]);
    }
    let home = unit(&sim, 0).position.length();
    assert!(home < m(6), "and returned to its place ({home})");
    assert!(matches!(
        sim.drill_view(UnitId(0)),
        Some(DrillView::Secure(..))
    ));
}

#[test]
fn securing_units_do_not_chase_out_of_the_circle() {
    // The truck is seen, but farther than the circle of 50 m plus the 50 m
    // of the machine gun.
    let mut sim = sim_with(
        flat(),
        &[(P0, SQUAD, 0, 0, FIRE), (P1, TRUCK, 135, 0, HOLD)],
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Secure {
            units: ids(&[0]),
            center: Vec2::ZERO,
            radius_m: 50,
            formation: Formation::Auto,
            facing: None,
        },
    )]);
    let mut farthest = Fixed::ZERO;
    for _ in 0..1500 {
        sim.step(&[]);
        farthest = farthest.max(unit(&sim, 0).position.length());
    }
    assert!(farthest < m(10), "the squad stayed in place ({farthest})");
    assert!(unit(&sim, 1).alive());
}

fn tanks_at(map: Arc<Map>, count: i32) -> Sim {
    let spawns: Vec<(PlayerId, &str, i32, i32, FireMode)> = (0..count)
        .map(|k| (P0, TANK, -30 + 15 * k, -250, HOLD))
        .collect();
    sim_with(map, &spawns)
}

fn auto(sim: &Sim, count: u32, target: Vec2) -> Formation {
    let units: Vec<UnitId> = (0..count).map(UnitId).collect();
    sim.auto_formation_for(P0, &units, target, None).unwrap()
}

#[test]
fn auto_formation_follows_the_ground() {
    let target = Vec2::from_meters(0, 100);
    // Open field: tanks in a wedge, infantry in a line.
    assert_eq!(auto(&tanks_at(flat(), 4), 4, target), Formation::Wedge);
    let squads = sim_with(
        flat(),
        &[(P0, SQUAD, 0, -250, HOLD), (P0, SQUAD, 30, -250, HOLD)],
    );
    assert_eq!(auto(&squads, 2, target), Formation::Line);
    // A road along the way (terrain 2, 10 m wide, north-south): column.
    let road = map_with(|x, _| if x.abs() <= 5 { 2 } else { 0 }, &[]);
    assert_eq!(auto(&tanks_at(road, 4), 4, target), Formation::Column);
    // A 16 m gap between deep water (terrain 7): column.
    let gap = map_with(
        |x, y| {
            if (60..140).contains(&y) && x.abs() > 8 {
                7
            } else {
                0
            }
        },
        &[],
    );
    assert_eq!(auto(&tanks_at(gap, 4), 4, target), Formation::Column);
    // Woods for infantry: dispersed (tanks would see narrow ground).
    let mut trees = Vec::new();
    for gx in -8..=8 {
        for gy in -8..=8 {
            trees.push(format!(
                "env_tree_deciduous,Oak,{},{},0",
                gx * 5000,
                100_000 + gy * 5000
            ));
        }
    }
    let woods = sim_with(
        map_with(|_, _| 0, &trees),
        &[(P0, SQUAD, 0, -250, HOLD), (P0, SQUAD, 30, -250, HOLD)],
    );
    assert_eq!(auto(&woods, 2, target), Formation::Dispersed);
}

#[test]
fn a_dragged_facing_turns_the_line_and_the_hulls() {
    let mut sim = tanks_at(flat(), 3);
    let east = Vec2::new(Fixed::ONE, Fixed::ZERO);
    let target = Vec2::from_meters(0, 0);
    sim.step(&[formation_move(
        &[0, 1, 2],
        target,
        Formation::Line,
        Some(east.scale(m(50))),
        Engage::Never,
    )]);
    for _ in 0..2000 {
        sim.step(&[]);
        let all_done = (0..3).all(|i| unit(&sim, i).target.is_none());
        if all_done {
            break;
        }
    }
    for _ in 0..200 {
        sim.step(&[]);
    }
    for i in 0..3 {
        let u = unit(&sim, i);
        // A line facing east stands north-south through the target.
        assert!(u.position.x.abs() < m(2), "unit {i} at {:?}", u.position);
        assert!(
            u.facing.x > Fixed::from_num(0.99),
            "unit {i} faces {:?}",
            u.facing
        );
    }
}

#[test]
fn a_waypoint_waits_for_the_attack_move() {
    let mut sim = tanks_at(flat(), 1);
    let first = Vec2::from_meters(0, -150);
    let second = Vec2::from_meters(100, -150);
    sim.step(&[formation_move(
        &[0],
        first,
        Formation::Auto,
        None,
        Engage::All,
    )]);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::QueueMove {
            units: ids(&[0]),
            target: second,
        },
    )]);
    let mut reached_first = false;
    for _ in 0..2000 {
        sim.step(&[]);
        let p = unit(&sim, 0).position;
        reached_first |= (p - first).length() < m(2);
        if (p - second).length() < m(2) {
            break;
        }
    }
    assert!(reached_first, "the attack-move target came first");
    assert!((unit(&sim, 0).position - second).length() < m(2));
}
