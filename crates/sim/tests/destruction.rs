//! Destruction tests (prompt 020): walls, trees, buildings and wire on
//! hand-made maps, shot at by catalog units.

use sim::map::MapLayout;
use sim::path::SizeClass;
use sim::{
    Blocks, Catalog, CombatEvent, CombatRules, Command, FireMode, HitKind, Map, ObjectCause,
    ObjectEvent, ObjectStage, PlayerCommand, PlayerId, Remains, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const ENEMY: &str = "su_t34_76_1943";
const SQUAD: &str = "de_inf_rifle_squad";

/// 800 m × 800 m flat map around the origin with the given objects (CSV
/// rows without header).
fn map_with(objects: &str) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "destruction_test".into(),
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

/// Rules without randomness: no dispersion, no penetration spread.
fn exact_rules() -> CombatRules {
    let mut rules = CombatRules::builtin().unwrap();
    rules.dispersion.mrad_x10 = 0;
    rules.penetration.spread_pct = 0;
    rules
}

/// Unit types are sorted by name: their index is the path profile.
fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)]) -> Sim {
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
        combat: exact_rules(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
    };
    Sim::new(&config, map, 7).unwrap()
}

fn command(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn attack_ground(unit: u32, x: i32, y: i32) -> PlayerCommand {
    command(
        P0,
        Command::AttackGround {
            units: vec![UnitId(unit)],
            target: Vec2::from_meters(x, y),
        },
    )
}

/// Steps until `done` holds (at most `max` ticks), collecting object events.
fn run_until(sim: &mut Sim, max: u64, done: impl Fn(&Sim) -> bool) -> Vec<ObjectEvent> {
    let mut events = Vec::new();
    let end = sim.tick() + max;
    while sim.tick() < end && !done(sim) {
        sim.step(&[]);
        events.extend_from_slice(sim.object_events());
    }
    assert!(done(sim), "not done after {max} ticks");
    events
}

fn path_length(sim: &Sim, profile: usize, from: Vec2, to: Vec2) -> i32 {
    let path = sim.path_grid().find_path(from, to, profile).unwrap();
    assert_eq!(path.goal, to, "goal not reachable");
    path.length().to_num::<i32>()
}

/// A dry-stone wall from x = -150 m to 150 m along y = 0: 3 m segments
/// centered on multiples of 3 m.
fn wall() -> String {
    (-50..=50)
        .map(|k| format!("env_wall_stone,Segment,{},0,90", k * 3000))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_destroyed_wall_opens_a_path() {
    // The Panzer IV stands 60 m south of the wall (y grows southwards) and shoots five segments
    // around x = 0 out of it (1200 hit points, 562 per 7.5 cm round).
    let mut sim = sim_with(
        map_with(&wall()),
        &[(P0, TANK, 0, 60), (P1, ENEMY, 0, -300)],
    );
    let (south, north) = (Vec2::from_meters(0, 40), Vec2::from_meters(0, -40));
    let before = path_length(&sim, 0, south, north);
    assert!(before > 300, "path around the wall: {before} m");
    let mut events = Vec::new();
    for k in -2..=2 {
        let index = (k + 50) as u32;
        sim.step(&[attack_ground(0, k * 3, 0)]);
        events.extend_from_slice(sim.object_events());
        events.extend(run_until(&mut sim, 1000, |s| {
            s.object_stage(index) == Some(ObjectStage::Destroyed)
        }));
        assert_eq!(sim.object_blocks(index), Some(Blocks::None));
    }
    assert_eq!(events.len(), 5, "{events:?}");
    for e in &events {
        assert_eq!(e.stage, ObjectStage::Destroyed);
        assert_eq!(e.remains, Remains::Gone);
        assert_eq!(e.cause, ObjectCause::Hit(UnitId(0)));
    }
    // Three hits per segment; the rounds fired after the last one hit the
    // ground behind the gap.
    assert_eq!(sim.object_hp(48), Some((0, 1200)));
    assert_eq!(sim.object_stage(47), Some(ObjectStage::Intact));
    let after = path_length(&sim, 0, south, north);
    assert!(
        after < 90,
        "path through the gap: {after} m (before {before} m)"
    );
    assert!(sim.path_grid().passable_at(SizeClass::Large, Vec2::ZERO));
}

#[test]
fn a_unit_on_its_way_takes_the_new_gap() {
    // A T-34 ordered around the wall re-plans through the gap.
    let mut sim = sim_with(map_with(&wall()), &[(P0, TANK, 0, 60), (P1, ENEMY, 20, 30)]);
    sim.step(&[command(
        P1,
        Command::MoveTo {
            units: vec![UnitId(1)],
            target: Vec2::from_meters(20, -30),
        },
    )]);
    for k in -2..=2 {
        let index = (k + 50) as u32;
        sim.step(&[attack_ground(0, k * 3, 0)]);
        run_until(&mut sim, 1000, |s| {
            s.object_stage(index) == Some(ObjectStage::Destroyed)
        });
    }
    // Well before driving around the 300 m wall it is south of it.
    run_until(&mut sim, 1200, |s| {
        s.snapshot().units[1].position.y < sim::Fixed::from_num(-20)
    });
}

/// A row of oaks along x = 4 m from y = -20 to 20 m: two per 8 m vision
/// cell, a forest strip 40 m deep between an observer at (4, 200) and a
/// target at (4, -100) (y grows southwards).
fn tree_row() -> String {
    (-5..=5)
        .map(|k| format!("env_tree_deciduous,Oak,4000,{},0", k * 4000))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn felled_trees_fall_away_from_the_gun_and_thin_the_forest() {
    let mut sim = sim_with(
        map_with(&tree_row()),
        &[(P0, TANK, 4, 200), (P1, ENEMY, 4, -100)],
    );
    assert!(
        !sim.visible_to(P0, UnitId(1)),
        "the forest hides the target"
    );
    let grid = sim.vision_grid();
    let cell = grid.index_of(Vec2::from_meters(4, 0)) as usize;
    assert_eq!(grid.blocker_at(cell), sim::vision::Sight::Foliage);
    // Rounds at the ground just behind the strip fell one tree each, from
    // the south end (nearest to the gun) on.
    sim.step(&[attack_ground(0, 4, -30)]);
    let events = run_until(&mut sim, 2000, |s| {
        (0..11).all(|i| s.object_stage(i) == Some(ObjectStage::Destroyed))
    });
    let order: Vec<u32> = events.iter().map(|e| e.object).collect();
    assert_eq!(order, (0..11).rev().collect::<Vec<_>>());
    for e in &events {
        assert_eq!(e.remains, Remains::Fallen);
        // The rounds fly north (-y): the trees fall north (0°).
        assert_eq!(e.fall_deg, 0);
        assert_eq!(sim.object_state(e.object).unwrap().fall_deg, 0);
    }
    assert_eq!(
        sim.vision_grid().blocker_at(cell),
        sim::vision::Sight::Clear
    );
    sim.step(&[command(
        P0,
        Command::Stop {
            units: vec![UnitId(0)],
        },
    )]);
    // The observer's viewshed was computed again: the target is in sight.
    sim.step(&[]);
    assert!(
        sim.visible_to(P0, UnitId(1)),
        "the felled forest still hides it"
    );
}

#[test]
fn a_shelled_house_becomes_a_ruin_that_stops_tanks_and_low_rounds() {
    // Farmhouse 14 m (along y) × 9 m at the origin; the Panzer IV 100 m
    // north (-y) fires at its center.
    let mut sim = sim_with(
        map_with("env_house_farm,Intact,0,0,0"),
        &[(P0, TANK, 0, -100), (P1, ENEMY, 0, 300)],
    );
    assert!(!sim.path_grid().passable_at(SizeClass::Small, Vec2::ZERO));
    sim.step(&[attack_ground(0, 0, 0)]);
    let events = run_until(&mut sim, 1000, |s| {
        s.object_stage(0) == Some(ObjectStage::Destroyed)
    });
    // 2500 hit points: damaged at 1250 (third hit), a ruin at the fifth.
    let stages: Vec<ObjectStage> = events.iter().map(|e| e.stage).collect();
    assert_eq!(stages, [ObjectStage::Damaged, ObjectStage::Destroyed]);
    assert_eq!(events[1].remains, Remains::Ruin);
    assert_eq!(sim.object_blocks(0), Some(Blocks::Large));
    assert!(sim.path_grid().passable_at(SizeClass::Small, Vec2::ZERO));
    assert!(!sim.path_grid().passable_at(SizeClass::Large, Vec2::ZERO));
    // The next round hits the 3.2 m ruin wall, not the ground behind it.
    let mut hit = None;
    for _ in 0..200 {
        sim.step(&[]);
        if let Some(e) = sim
            .events()
            .iter()
            .find(|e| matches!(e, CombatEvent::Hit { .. }))
        {
            hit = Some(*e);
            break;
        }
    }
    assert!(
        matches!(
            hit,
            Some(CombatEvent::Hit {
                kind: HitKind::Object(0),
                damage: 0,
                ..
            })
        ),
        "{hit:?}"
    );
}

#[test]
fn infantry_fires_out_of_a_ruin_it_stands_in() {
    // A ruin placed by the map acts as a ruin from the start: open for
    // infantry, closed for tanks.
    let mut sim = sim_with(
        map_with("env_house_farm,Ruin,0,0,0"),
        &[(P0, SQUAD, 0, -60), (P1, SQUAD, 0, 0)],
    );
    assert_eq!(sim.object_stage(0), Some(ObjectStage::Destroyed));
    assert_eq!(sim.object_hp(0), Some((0, 2500)));
    assert!(sim.path_grid().passable_at(SizeClass::Small, Vec2::ZERO));
    assert!(!sim.path_grid().passable_at(SizeClass::Large, Vec2::ZERO));
    assert!(sim.visible_to(P1, UnitId(0)), "ruins do not block sight");
    // The squad first spreads its soldiers over the rubble (prompt 022).
    for _ in 0..80 {
        sim.step(&[]);
    }
    sim.step(&[command(
        P1,
        Command::Attack {
            units: vec![UnitId(1)],
            target: UnitId(0),
        },
    )]);
    let mut hits = Vec::new();
    for _ in 0..100 {
        sim.step(&[]);
        hits.extend(sim.events().iter().filter_map(|e| match e {
            CombatEvent::Hit { kind, .. } => Some(*kind),
            _ => None,
        }));
    }
    // No round of the squad in the ruin strikes the ruin or the squad
    // itself. Since prompt 022 every soldier has a small hit box of his own,
    // so rounds that miss hit the ground behind the target.
    assert!(hits.contains(&HitKind::Unit(UnitId(0))), "{hits:?}");
    assert!(
        hits.iter()
            .all(|&k| k == HitKind::Unit(UnitId(0)) || k == HitKind::Ground),
        "{hits:?}"
    );
}

#[test]
fn a_tank_flattens_wire_and_opens_the_way_for_infantry() {
    // Barbed wire from x = -150 m to 150 m along y = 0.
    let wire: String = (-50..=50)
        .map(|k| format!("fort_barbed_wire,Segment,{},0,90", k * 3000))
        .collect::<Vec<_>>()
        .join("\n");
    // Profiles: 0 = rifle squad, 1 = Panzer IV (names sorted).
    let mut sim = sim_with(
        map_with(&wire),
        &[(P0, TANK, 0, 30), (P0, SQUAD, 30, 30), (P1, ENEMY, 0, -300)],
    );
    let (south, north) = (Vec2::from_meters(0, 20), Vec2::from_meters(0, -20));
    let before = path_length(&sim, 0, south, north);
    assert!(before > 300, "infantry path around the wire: {before} m");
    sim.step(&[command(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, -30),
        },
    )]);
    let events = run_until(&mut sim, 600, |s| {
        s.object_stage(50) == Some(ObjectStage::Destroyed)
    });
    assert!(!events.is_empty());
    for e in &events {
        assert_eq!(e.cause, ObjectCause::Crushed(UnitId(0)));
        assert_eq!(e.remains, Remains::Flat);
        // Driving north (-y).
        assert_eq!(e.fall_deg, 0);
    }
    let after = path_length(&sim, 0, south, north);
    assert!(
        after < 60,
        "infantry path through the gap: {after} m (before {before} m)"
    );
}

/// The wall battle twice with the same seed: identical hashes every tick.
#[test]
fn destruction_is_deterministic() {
    let run = || {
        let mut sim = sim_with(
            map_with(&wall()),
            &[(P0, TANK, 0, 60), (P1, ENEMY, 40, -60)],
        );
        let mut hashes = Vec::new();
        sim.step(&[attack_ground(0, 0, 0)]);
        for tick in 0..600 {
            if tick == 300 {
                sim.step(&[command(
                    P1,
                    Command::MoveTo {
                        units: vec![UnitId(1)],
                        target: Vec2::from_meters(0, 40),
                    },
                )]);
            } else {
                sim.step(&[]);
            }
            hashes.push(sim.state_hash());
        }
        (hashes, sim.changed_objects().collect::<Vec<_>>())
    };
    let (a, changed) = run();
    assert!(!changed.is_empty());
    assert_eq!(a, run().0);
}
