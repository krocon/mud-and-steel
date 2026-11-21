//! Fire tests (prompt 041): woods burn and spread, roads stop the fire,
//! buildings burn down, dumps explode, hits start fires, and all of it is
//! deterministic.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireEventKind, FireMode, FireSource, Map,
    ObjectCause, ObjectStage, PlayerCommand, PlayerId, Remains, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const ROAD: u8 = 2;

/// 400 m × 400 m flat map (2 m cells) with the given objects (CSV rows
/// without header); `road(x_m)` says whether the column at `x_m` is road.
fn map_with(objects: &str, road: impl Fn(i32) -> bool) -> Arc<Map> {
    let n: u32 = 201;
    let layout = MapLayout {
        name: "fire_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let terrain = (0..size)
        .map(|i| {
            let x = (i as u32 % n) as i32 * 2 - 200;
            if road(x) { ROAD } else { 0 }
        })
        .collect();
    let mut map = Map::from_rasters(layout, vec![0; size], terrain).unwrap();
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
                fire_mode: FireMode::HoldFire,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).unwrap()
}

/// Two far-away units so the match is not decided at once.
fn bystanders() -> Vec<(PlayerId, &'static str, i32, i32)> {
    vec![
        (P0, "de_pz4_h", -180, 180),
        (P1, "su_t34_76_1943", 180, 180),
    ]
}

/// Conifers on a 5 m grid with x in `xs` and y from -20 to 20 m.
fn wood(xs: impl Iterator<Item = i32>) -> Vec<String> {
    let mut rows = Vec::new();
    for x in xs {
        for y in (-20..=20).step_by(5) {
            rows.push(format!(
                "env_tree_conifer,Spruce,{},{},0",
                x * 1000,
                y * 1000
            ));
        }
    }
    rows
}

/// Two woods: x = -60..-5 m and x = 5..60 m (columns every 5 m), 10 m
/// apart between the outlines of the trunks minus 0.8 m.
fn two_woods() -> String {
    let mut rows = wood((-60..=-5).step_by(5));
    rows.extend(wood((5..=60).step_by(5)));
    rows.join("\n")
}

fn run(sim: &mut Sim, ticks: u64) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

/// Object indices of the wood west (x < 0) and east of the gap.
fn sides(sim: &Sim) -> (Vec<u32>, Vec<u32>) {
    let (mut west, mut east) = (Vec::new(), Vec::new());
    for (i, o) in (0u32..).zip(sim.map().objects()) {
        if o.position.x < sim::Fixed::ZERO {
            west.push(i);
        } else {
            east.push(i);
        }
    }
    (west, east)
}

fn touched(sim: &Sim, objects: &[u32]) -> usize {
    objects
        .iter()
        .filter(|&&i| sim.object_burning(i) || sim.object_burnt(i))
        .count()
}

/// Ignites the westernmost middle tree and runs 6000 ticks (5 minutes).
fn burn_west(map: Arc<Map>) -> Sim {
    let mut sim = sim_with(map, &bystanders(), 3);
    let (west, _) = sides(&sim);
    assert!(sim.ignite_object(west[4], UnitId(0)));
    assert!(!sim.ignite_object(west[4], UnitId(0)), "burns already");
    run(&mut sim, 6000);
    sim
}

#[test]
fn fire_spreads_through_a_wood_and_across_grass() {
    // The 10 m gap is grass with nothing in it: fire jumps it only where
    // the trunks are within reach (7 m), which they are not.
    let sim = burn_west(map_with(&two_woods(), |_| false));
    let (west, east) = sides(&sim);
    assert!(
        touched(&sim, &west) > west.len() * 3 / 4,
        "{} of {}",
        touched(&sim, &west),
        west.len()
    );
    assert_eq!(touched(&sim, &east), 0, "jumped a 10 m gap");
    // Burnt trees are felled and no forest any more.
    let burnt: Vec<u32> = sim.burnt_objects().collect();
    assert!(!burnt.is_empty());
    for &i in &burnt {
        assert_eq!(sim.object_stage(i), Some(ObjectStage::Destroyed));
        assert_eq!(sim.object_remains(i), Some(Remains::Fallen));
    }

    // Closer woods (4.2 m between the trunk outlines) catch each other.
    let mut rows = wood((-60..=-5).step_by(5));
    rows.extend(wood((0..=60).step_by(5)));
    let sim = burn_west(map_with(&rows.join("\n"), |_| false));
    let (_, east) = sides(&sim);
    assert!(touched(&sim, &east) > 0, "did not cross a 4 m grass gap");
}

#[test]
fn fire_does_not_cross_a_road() {
    // The same close woods with a road between them: no tree east of the
    // road catches fire.
    let mut rows = wood((-60..=-5).step_by(5));
    rows.extend(wood((0..=60).step_by(5)));
    let sim = burn_west(map_with(&rows.join("\n"), |x| (-4..=-2).contains(&x)));
    let (west, east) = sides(&sim);
    assert!(touched(&sim, &west) > west.len() / 2);
    assert_eq!(touched(&sim, &east), 0, "fire crossed the road");
}

#[test]
fn fires_are_deterministic() {
    let a = burn_west(map_with(&two_woods(), |_| false));
    let b = burn_west(map_with(&two_woods(), |_| false));
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.burnt_objects().count(), b.burnt_objects().count());
    // Another seed burns differently (but just as deterministically).
    let mut c = sim_with(map_with(&two_woods(), |_| false), &bystanders(), 4);
    assert!(c.ignite_object(4, UnitId(0)));
    run(&mut c, 6000);
    assert_ne!(a.state_hash(), c.state_hash());
}

#[test]
fn a_barn_burns_down_to_a_ruin_and_neighbors_catch() {
    let rows =
        "env_barn,Intact,0,0,0\nenv_house_farm,Intact,0,16000,0\nenv_house_mud,Intact,60000,0,0";
    let mut sim = sim_with(map_with(rows, |_| false), &bystanders(), 5);
    assert!(sim.ignite_object(0, UnitId(0)));
    let mut events = Vec::new();
    let mut fire_events = Vec::new();
    for _ in 0..12_000 {
        sim.step(&[]);
        events.extend_from_slice(sim.object_events());
        fire_events.extend_from_slice(sim.fire_events());
    }
    assert_eq!(sim.object_stage(0), Some(ObjectStage::Destroyed));
    assert!(sim.object_burnt(0));
    assert!(events.iter().any(|e| e.object == 0
        && e.stage == ObjectStage::Destroyed
        && e.cause == ObjectCause::Fire(UnitId(0))));
    // The farmhouse 7 m away caught fire; the mud house 50 m away did not.
    assert!(
        sim.object_burnt(1) || sim.object_burning(1),
        "{fire_events:?}"
    );
    assert!(!sim.object_burnt(2) && !sim.object_burning(2));
    assert!(
        fire_events
            .iter()
            .any(|e| e.source == FireSource::Object(0) && e.kind == FireEventKind::BurntOut)
    );
}

#[test]
fn stone_houses_keep_their_walls() {
    let mut sim = sim_with(
        map_with("env_house_stone,Intact,0,0,0", |_| false),
        &bystanders(),
        5,
    );
    assert!(sim.ignite_object(0, UnitId(0)));
    run(&mut sim, 12_000);
    assert!(sim.object_burnt(0));
    assert_eq!(sim.object_stage(0), Some(ObjectStage::Damaged));
}

#[test]
fn an_ammunition_dump_explodes() {
    // A truck 8 m and a squad 10 m from the dump; a fuel dump 14 m away (out of reach of the flames, within the blast).
    let rows = "env_dump_ammo,Intact,0,0,0\nenv_dump_fuel,Intact,0,-14000,0";
    let mut spawns = bystanders();
    spawns.push((P1, "de_opel_blitz", 8, 0));
    spawns.push((P1, "su_inf_rifle_squad", -10, 0));
    let mut sim = sim_with(map_with(rows, |_| false), &spawns, 7);
    let truck = UnitId(2);
    let squad = UnitId(3);
    let truck_hp = sim.unit_hp(truck).unwrap();
    let squad_hp = sim.unit_hp(squad).unwrap();
    assert!(sim.ignite_object(0, UnitId(0)));
    let mut exploded = Vec::new();
    let mut blasts = 0;
    for _ in 0..200 {
        sim.step(&[]);
        for e in sim.fire_events() {
            if e.kind == FireEventKind::Exploded {
                exploded.push((sim.tick(), e.source));
            }
        }
        blasts += sim
            .events()
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::Hit {
                        caliber_mm_x10: 1500,
                        ..
                    }
                )
            })
            .count();
    }
    // The fuse: 6 s real = 3 s game = 60 ticks.
    assert_eq!(
        exploded.first(),
        Some(&(61, FireSource::Object(0))),
        "{exploded:?}"
    );
    assert!(blasts >= 1);
    assert_eq!(sim.object_stage(0), Some(ObjectStage::Destroyed));
    assert!(sim.unit_hp(truck).unwrap() < truck_hp, "truck unharmed");
    assert!(sim.unit_hp(squad).unwrap() < squad_hp, "squad unharmed");
    // The blast set the fuel dump on fire; it burst 2 s real later.
    assert!(sim.object_burning(1), "{exploded:?} {:?}", sim.fires());
    assert!(
        exploded.iter().any(|&(_, s)| s == FireSource::Object(1)),
        "{exploded:?}"
    );
}

#[test]
fn occupants_of_a_burning_house_get_hurt() {
    let mut spawns = bystanders();
    spawns.push((P0, "de_inf_rifle_squad", 0, 12));
    let mut sim = sim_with(
        map_with("env_house_farm,Intact,0,0,0", |_| false),
        &spawns,
        8,
    );
    let squad = UnitId(2);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Garrison {
            units: vec![squad],
            building: 0,
        },
    )]);
    run(&mut sim, 400);
    let inside = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.id == squad)
        .and_then(|u| u.garrison);
    assert_eq!(inside, Some(0), "the squad did not enter");
    let before = sim.unit_hp(squad).unwrap();
    assert!(sim.ignite_object(0, UnitId(1)));
    run(&mut sim, 400);
    assert!(sim.unit_hp(squad).unwrap() < before);
}

#[test]
fn shells_bursting_next_to_hay_set_it_on_fire() {
    // A Panzer IV fires high explosive at the ground next to a row of hay
    // ricks (hit_pct 60 within 4 m of the burst).
    let rows: Vec<String> = (0..10)
        .map(|k| format!("env_hay_bale,Rick,{},-100000,0", k * 12000 - 54000))
        .collect();
    let mut sim = sim_with(map_with(&rows.join("\n"), |_| false), &bystanders(), 9);
    let mut ignited = 0;
    for k in 0..10 {
        sim.step(&[PlayerCommand::new(
            P0,
            Command::AttackGround {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(k * 12 - 54, -102),
            },
        )]);
        for _ in 0..200 {
            sim.step(&[]);
            ignited += sim
                .fire_events()
                .iter()
                .filter(|e| {
                    e.kind == FireEventKind::Ignited && matches!(e.source, FireSource::Object(_))
                })
                .count();
        }
    }
    assert!(ignited > 0, "no hay rick caught fire");
}

#[test]
fn destroyed_vehicles_burn_as_wrecks() {
    // The truck stands 127 m from the Panzer IV (before 254 m at the origin,
    // halved with the direct-fire and sight factors 1/4 -> 1/8): seen
    // (250 m sight), the tank closes in to its 87.5 m and fires.
    let mut spawns = bystanders();
    spawns.push((P1, "de_opel_blitz", -90, 90));
    let mut sim = sim_with(map_with("", |_| false), &spawns, 10);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(2),
        },
    )]);
    let mut wreck = None;
    for _ in 0..2000 {
        sim.step(&[]);
        if let Some(e) = sim
            .fire_events()
            .iter()
            .find(|e| e.source == FireSource::Wreck(UnitId(2)))
        {
            wreck = Some(e.kind);
            break;
        }
    }
    assert_eq!(wreck, Some(FireEventKind::Ignited));
    assert!(
        sim.fires()
            .iter()
            .any(|f| f.source == FireSource::Wreck(UnitId(2)))
    );
}
