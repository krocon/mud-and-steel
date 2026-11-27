//! Grown rock (prompt 074): tanks and infantry go around rock outcrops and
//! along ridge pieces, close by, never through them, and reach targets
//! right behind them (no invisible walls); slabs stop nobody.

use sim::map::MapLayout;
use sim::objects::{MapObject, Shape};
use sim::{
    Catalog, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const SQUAD: &str = "de_inf_rifle_squad";

/// Flat 800 m map with the given objects (CSV rows without header).
fn map_with(objects: &[&str]) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "rock_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap();
    map.add_objects(&format!(
        "{}\n{}\n",
        sim::objects::CSV_HEADER,
        objects.join("\n")
    ))
    .unwrap();
    Arc::new(map)
}

fn sim_with(map: Arc<Map>, spawns: &[(&str, i32, i32)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.0).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(unit, x, y)| Spawn {
                owner: P0,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: HOLD,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: sim::CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        mission: None,
        variant: None,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, 74).unwrap()
}

fn move_to(sim: &mut Sim, id: u32, x: i32, y: i32) {
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![UnitId(id)],
            target: Vec2::from_meters(x, y),
        },
    )]);
}

/// The placed objects with their shapes.
fn rocks(sim: &Sim) -> Vec<(MapObject, Shape)> {
    let map = sim.map();
    map.objects()
        .iter()
        .map(|o| (*o, map.object_kinds()[usize::from(o.kind)].shape))
        .filter(|(o, _)| {
            map.object_kinds()[usize::from(o.kind)]
                .id
                .starts_with("env_rock_")
        })
        .collect()
}

/// Distance from `p` to the box of a rock (0 inside).
fn distance(o: &MapObject, shape: Shape, p: Vec2) -> Fixed {
    let Shape::Box {
        half_length,
        half_width,
    } = shape
    else {
        unreachable!("rocks have boxes")
    };
    let (fw, rt) = o.axes();
    let d = p - o.position;
    let along = (d.x * fw.x + d.y * fw.y).abs() - half_length;
    let across = (d.x * rt.x + d.y * rt.y).abs() - half_width;
    let (a, b) = (along.max(Fixed::ZERO), across.max(Fixed::ZERO));
    (a * a + b * b).sqrt()
}

/// Runs a unit to `target`; returns its closest approach to the solid
/// rocks (outcrops, ridges) and its distance to the target at the end.
/// Fails when the unit (or one of its soldiers) stands inside a solid rock.
fn run(sim: &mut Sim, id: u32, target: (i32, i32), ticks: u32) -> (Fixed, Fixed) {
    let solid: Vec<_> = rocks(sim)
        .into_iter()
        .filter(|(o, _)| {
            let kind = &sim.map().object_kinds()[usize::from(o.kind)];
            kind.blocks == sim::objects::Blocks::All
        })
        .collect();
    let mut closest = Fixed::from_num(1000);
    for _ in 0..ticks {
        sim.step(&[]);
        let snapshot = sim.snapshot();
        let unit = snapshot.units.iter().find(|u| u.id == UnitId(id)).unwrap();
        let mut points = vec![unit.position];
        points.extend(
            snapshot
                .soldiers
                .iter()
                .filter(|s| s.unit == UnitId(id))
                .map(|s| s.position),
        );
        for p in points {
            for (o, shape) in &solid {
                let d = distance(o, *shape, p);
                assert!(d > Fixed::ZERO, "unit {id} inside a rock at {p:?}");
                closest = closest.min(d);
            }
        }
    }
    let end = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .position;
    let left = (end - Vec2::from_meters(target.0, target.1)).length();
    println!("unit {id} to {target:?}: closest {closest:.2} m to a rock, {left:.2} m short");
    (closest, left)
}

/// An outcrop (7.8 m × 6 m) straight on the route and a ridge piece (25 m)
/// beside it; slabs on the way.
const FIELD: [&str; 4] = [
    "env_rock_outcrop,Group,0,0,0",
    "env_rock_massif,Long,30000,0,0",
    "env_rock_slab,Wide,-20000,30000,0",
    "env_rock_slab,Field,-20000,-30000,45",
];

#[test]
fn a_tank_drives_around_an_outcrop_close_by() {
    let mut sim = sim_with(map_with(&FIELD), &[(TANK, 0, 40)]);
    move_to(&mut sim, 0, 0, -40);
    let (closest, left) = run(&mut sim, 0, (0, -40), 20 * 40);
    assert!(left < Fixed::from_num(5), "the tank stopped {left} m short");
    assert!(
        closest < Fixed::from_num(8),
        "the tank kept {closest} m from the rocks"
    );
}

#[test]
fn infantry_walks_around_an_outcrop_and_along_a_ridge() {
    let mut sim = sim_with(map_with(&FIELD), &[(SQUAD, 0, 40)]);
    move_to(&mut sim, 0, 0, -40);
    let (closest, left) = run(&mut sim, 0, (0, -40), 20 * 60);
    assert!(
        left < Fixed::from_num(5),
        "the squad stopped {left} m short"
    );
    assert!(closest < Fixed::from_num(8), "the squad kept {closest} m");
    // Along the ridge piece to a point right behind its end.
    move_to(&mut sim, 0, 30, -18);
    let (_, left) = run(&mut sim, 0, (30, -18), 20 * 60);
    assert!(
        left < Fixed::from_num(6),
        "the squad stopped {left} m short"
    );
}

#[test]
fn a_tank_reaches_the_far_side_of_a_ridge_piece() {
    // From the west of the ridge piece to just east of it: around one end.
    let mut sim = sim_with(map_with(&FIELD), &[(TANK, 18, 0)]);
    move_to(&mut sim, 0, 42, 0);
    let (closest, left) = run(&mut sim, 0, (42, 0), 20 * 60);
    assert!(left < Fixed::from_num(5), "the tank stopped {left} m short");
    assert!(closest < Fixed::from_num(8), "the tank kept {closest} m");
}

#[test]
fn a_tank_and_infantry_route_around_a_rock_cliff_wall() {
    // Prompt 243: two overlapping `env_rock_cliff` segments form a 40 m wall
    // across the route (y = 0, running along x = -18..+18 m, heading 90°).
    let wall = [
        "env_rock_cliff,Straight,-9000,0,90",
        "env_rock_cliff,High,9000,0,90",
    ];
    // Tank drives from south of the cliff to north of it, cleanly around an end.
    let mut sim = sim_with(map_with(&wall), &[(TANK, 0, 35)]);
    move_to(&mut sim, 0, 0, -35);
    let (closest, left) = run(&mut sim, 0, (0, -35), 20 * 60);
    assert!(
        left < Fixed::from_num(5),
        "the tank stopped {left} m short of the cliff target"
    );
    assert!(
        closest < Fixed::from_num(10),
        "the tank kept {closest} m from the cliff"
    );
    // Infantry also routes around the cliff wall without entering it.
    let mut sim = sim_with(map_with(&wall), &[(SQUAD, 0, 35)]);
    move_to(&mut sim, 0, 0, -35);
    let (closest, left) = run(&mut sim, 0, (0, -35), 20 * 80);
    assert!(
        left < Fixed::from_num(5),
        "the squad stopped {left} m short of the cliff target"
    );
    assert!(
        closest < Fixed::from_num(10),
        "the squad kept {closest} m from the cliff"
    );
}

#[test]
fn slabs_stop_nobody() {
    // Straight over two slabs: the tank arrives as fast as on open ground.
    let mut over = sim_with(map_with(&FIELD), &[(TANK, -20, 50)]);
    let mut open = sim_with(map_with(&[]), &[(TANK, -20, 50)]);
    for sim in [&mut over, &mut open] {
        move_to(sim, 0, -20, -50);
    }
    let mut ticks = [0u32; 2];
    for (k, sim) in [&mut over, &mut open].into_iter().enumerate() {
        for t in 0..20 * 40 {
            sim.step(&[]);
            let p = sim.snapshot().units[0].position;
            if (p - Vec2::from_meters(-20, -50)).length() < Fixed::from_num(3) {
                ticks[k] = t;
                break;
            }
        }
    }
    assert!(ticks[1] > 0, "the tank reached the target on open ground");
    assert_eq!(ticks[0], ticks[1], "slabs changed the drive");
}
