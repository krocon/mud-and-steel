//! Rivers, bridges, camouflage and paratroopers (prompt 055): a demolished
//! deck takes its units down, squads swim ashore or drown, bridge-laying
//! tanks span a narrow river and take the bridge up again, camouflage hides
//! standing vehicles, paratroopers land scattered (and drown in a lake), a
//! dragged bomb run lays a carpet along the line, determinism.

use sim::map::MapLayout;
use sim::{
    AirEvent, AirMission, Catalog, CombatEvent, CombatRules, Command, CrossingEvent, FireMode,
    Fixed, LayerState, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules,
    UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const DEEP: u8 = 7;
const ENGINEERS: &str = "de_inf_engineer";
const RIFLES: &str = "de_inf_rifle_squad";
const TRUCK: &str = "de_opel_blitz";
const LAYER: &str = "su_valentine_bridgelayer";
const LIGHT_TANK: &str = "su_t70m";
const HEAVY_TANK: &str = "su_is2";

/// A map of `sx` × `sy` samples of 2 m around the origin; `deep(x, y)`
/// (meters) marks deep water, 2 m below the banks.
fn map_with(name: &str, sx: u32, sy: u32, deep: impl Fn(i32, i32) -> bool) -> Map {
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for i in 0..sx {
            let (x, y) = (
                2 * i as i32 - (sx as i32 - 1),
                2 * j as i32 - (sy as i32 - 1),
            );
            let water = deep(x, y);
            heights.push(if water { -2000 } else { 0 });
            terrain.push(if water { DEEP } else { 0 });
        }
    }
    let layout = MapLayout {
        name: name.into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Map::from_rasters(layout, heights, terrain).unwrap()
}

/// 200 m × 120 m, a river at |x| <= 5 m and a timber bridge across it at
/// y = 1 m (like `tests/works.rs`).
fn river_with_bridge() -> Arc<Map> {
    let mut map = map_with("crossing_river", 101, 61, |x, _| x.abs() <= 5);
    map.add_objects("kind,variant,x_mm,y_mm,heading_deg\nenv_bridge_wood,Span16,0,1000,90\n")
        .unwrap();
    Arc::new(map)
}

fn sim_on(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let unit_types: Vec<UnitType> = names
        .iter()
        .map(|id| UnitType::from_catalog(catalog.get(id).unwrap(), &scale).unwrap())
        .collect();
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
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
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).unwrap()
}

fn ids(units: &[u32]) -> Vec<UnitId> {
    units.iter().map(|&u| UnitId(u)).collect()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn move_to(player: PlayerId, units: &[u32], x: i32, y: i32) -> PlayerCommand {
    cmd(
        player,
        Command::MoveTo {
            units: ids(units),
            target: Vec2::from_meters(x, y),
        },
    )
}

/// Runs `ticks` ticks; the crossing events.
fn run(sim: &mut Sim, ticks: u32) -> Vec<CrossingEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        sim.step(&[]);
        events.extend_from_slice(sim.crossing_events());
    }
    events
}

fn run_until(sim: &mut Sim, limit: u32, done: impl Fn(&Sim) -> bool) -> Vec<CrossingEvent> {
    let mut events = Vec::new();
    for _ in 0..limit {
        if done(sim) {
            return events;
        }
        sim.step(&[]);
        events.extend_from_slice(sim.crossing_events());
    }
    panic!("condition not met within {limit} ticks");
}

fn position(sim: &Sim, id: u32) -> Vec2 {
    sim.unit_position(UnitId(id)).unwrap()
}

fn hp(sim: &Sim, id: u32) -> u32 {
    sim.unit_hp(UnitId(id)).unwrap()
}

#[test]
fn a_demolished_deck_drops_its_units_into_the_river_and_squads_swim_ashore() {
    // Engineers west of the bridge; a squad and a truck on the deck.
    let mut sim = sim_on(
        river_with_bridge(),
        &[
            (P0, ENGINEERS, -30, 1),
            (P0, RIFLES, 0, 1),
            (P0, TRUCK, 3, 1),
        ],
        3,
    );
    let before = sim.state_hash();
    sim.step(&[cmd(
        P0,
        Command::DemolishBridge {
            units: ids(&[0]),
            bridge: 0,
        },
    )]);
    let mut events = run_until(&mut sim, 20 * 120, |s| !s.map().bridges()[0].intact);
    events.extend(run(&mut sim, 1));
    assert_ne!(before, sim.state_hash());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CrossingEvent::Collapsed { bridge: 0, .. }))
    );
    for unit in [1, 2] {
        assert!(
            events
                .iter()
                .any(|e| matches!(e, CrossingEvent::Fell { unit: u, .. } if *u == UnitId(unit))),
            "unit {unit} fell"
        );
    }
    assert_eq!(hp(&sim, 2), 0, "the truck is lost in the river");
    assert!(
        sim.swimmers().iter().any(|s| s.unit == UnitId(1)),
        "the squad swims"
    );
    // 5-7 m to the bank at 0.7 m/s: well within their breath.
    let events = run_until(&mut sim, 20 * 60, |s| s.swimmers().is_empty());
    assert!(hp(&sim, 1) > 0, "the squad reached the bank");
    assert!(position(&sim, 1).x.abs() > Fixed::from_num(5));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CrossingEvent::Ashore { unit, .. } if *unit == UnitId(1)))
    );
    // Orders reach the squad again.
    let ashore = position(&sim, 1);
    sim.step(&[move_to(
        P0,
        &[1],
        ashore.x.to_num::<i32>().signum() * 40,
        20,
    )]);
    run(&mut sim, 20 * 30);
    assert!(
        position(&sim, 1).x.abs() > Fixed::from_num(20),
        "from {ashore:?} to {:?}, route {:?}",
        position(&sim, 1),
        sim.route_of(UnitId(1))
    );
}

/// Every unit of `sim` alive and in deep water.
fn wet_units(sim: &Sim) -> usize {
    let map = Arc::clone(sim.map());
    sim.snapshot()
        .units
        .iter()
        .filter(|u| u.alive() && map.terrain_at(u.position) == DEEP)
        .count()
}

#[test]
fn paratroopers_land_scattered_and_drown_far_from_the_bank() {
    // A lake 400 m wide east of a strip of land: the drop goes into it.
    let map = Arc::new(map_with("crossing_lake", 401, 201, |x, _| x > -300));
    let mut sim = sim_on(
        map,
        &[(P0, RIFLES, -360, 0), (P1, "su_inf_rifle_squad", -360, 150)],
        9,
    );
    let units = sim.snapshot().units.len();
    sim.step(&[cmd(
        P0,
        Command::CallAir {
            mission: AirMission::Airborne,
            target: Vec2::from_meters(100, 0),
        },
    )]);
    assert!(sim.air_events().iter().any(|e| matches!(
        e,
        AirEvent::Called {
            mission: AirMission::Airborne,
            ..
        }
    )));
    let mut events = run_until(&mut sim, 20 * 120, |s| !s.parachutes(None).is_empty());
    let chute = sim.parachutes(None)[0];
    assert!(chute.position.z > Fixed::from_num(50), "they jump high");
    events.extend(run_until(&mut sim, 20 * 120, |s| {
        s.parachutes(None).is_empty() && s.snapshot().units.len() >= units + 3
    }));
    let landed: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            CrossingEvent::Landed { unit, at, water } => Some((*unit, *at, *water)),
            _ => None,
        })
        .collect();
    assert_eq!(landed.len(), 3, "three aircraft, one squad each");
    for &(unit, at, water) in &landed {
        assert!(water, "{unit:?} came down in the lake");
        assert!((at - Vec2::from_meters(100, 0)).length() < Fixed::from_num(300));
        assert_eq!(
            sim.snapshot()
                .units
                .iter()
                .find(|u| u.id == unit)
                .unwrap()
                .owner,
            P0
        );
    }
    assert!(landed[0].1 != landed[1].1, "scattered");
    // No bank within reach: all of them drown within the longest breath.
    let events = run(&mut sim, 20 * 65);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CrossingEvent::Drowned { .. }))
    );
    for &(unit, ..) in &landed {
        assert_eq!(sim.unit_hp(unit), Some(0), "{unit:?} drowned");
    }
    assert_eq!(wet_units(&sim), 0);
}

#[test]
fn paratroopers_land_on_open_ground_as_squads() {
    let map = Arc::new(map_with("crossing_field", 401, 401, |_, _| false));
    let mut sim = sim_on(
        map,
        &[(P0, RIFLES, 0, 350), (P1, "su_inf_rifle_squad", 0, -350)],
        4,
    );
    let units = sim.snapshot().units.len();
    sim.step(&[cmd(
        P0,
        Command::CallAirLine {
            mission: AirMission::Airborne,
            from: Vec2::from_meters(-100, 0),
            to: Vec2::from_meters(100, 0),
        },
    )]);
    run_until(&mut sim, 20 * 180, |s| {
        s.snapshot().units.len() >= units + 3
    });
    run(&mut sim, 5);
    let snapshot = sim.snapshot();
    let new: Vec<_> = snapshot.units[units..].iter().collect();
    assert!(
        new.iter()
            .all(|u| u.owner == P0 && u.alive() && u.stance.is_some())
    );
    // Dropped along the dragged line, each stick at its own point.
    let xs: Vec<i32> = new.iter().map(|u| u.position.x.to_num::<i32>()).collect();
    let spread = xs.iter().max().unwrap() - xs.iter().min().unwrap();
    assert!(spread > 80, "spread along the line: {xs:?}");
    assert!(
        new.iter()
            .all(|u| u.position.y.abs() < Fixed::from_num(150))
    );
}

/// 200 m × 120 m, a narrow river (|x| <= 3 m: 6 m of deep water).
fn narrow_river() -> Arc<Map> {
    Arc::new(map_with("crossing_narrow", 101, 61, |x, _| x.abs() <= 3))
}

#[test]
fn a_bridge_layer_spans_a_narrow_river_and_takes_its_bridge_up_again() {
    let mut sim = sim_on(
        narrow_river(),
        &[
            (P0, LAYER, -40, 0),
            (P0, LIGHT_TANK, -40, 24),
            (P0, HEAVY_TANK, -40, -24),
        ],
        2,
    );
    let types: Vec<u32> = sim.snapshot().units.iter().map(|u| u.unit_type).collect();
    let gap = sim
        .find_gap(types[0], Vec2::from_meters(1, 2))
        .expect("a gap");
    assert!(gap.width_m <= 8, "{gap:?}");
    assert!(
        sim.find_gap(types[1], Vec2::ZERO).is_none(),
        "tanks lay no bridges"
    );
    // Nothing crosses before.
    sim.step(&[move_to(P0, &[1], 40, 24)]);
    run(&mut sim, 20 * 30);
    assert!(position(&sim, 1).x < Fixed::ZERO);
    sim.step(&[cmd(
        P0,
        Command::LayBridge {
            units: ids(&[0]),
            target: Vec2::from_meters(0, 0),
        },
    )]);
    assert_eq!(sim.layers()[0].state, LayerState::Driving);
    let events = run_until(&mut sim, 20 * 180, |s| {
        matches!(s.layers()[0].state, LayerState::Laid(_))
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CrossingEvent::Bridge { laid: true, .. }))
    );
    let bridge = sim.map().bridges()[0].clone();
    assert_eq!(bridge.laid_by, Some(UnitId(0)));
    assert!(bridge.intact);
    // The light tank crosses on it, the heavy one is too heavy.
    sim.step(&[move_to(P0, &[1], 40, 0), move_to(P0, &[2], 40, -24)]);
    run_until(&mut sim, 20 * 120, |s| {
        position(s, 1).x > Fixed::from_num(20)
    });
    assert!(position(&sim, 2).x < Fixed::ZERO, "the IS-2 stays behind");
    assert_eq!(wet_units(&sim), 0);
    // Taken up again: the river is closed.
    sim.step(&[cmd(P0, Command::PickUpBridge { units: ids(&[0]) })]);
    run_until(&mut sim, 20 * 240, |s| {
        s.layers()[0].state == LayerState::Carrying
    });
    assert!(!sim.map().bridges()[0].intact);
    sim.step(&[move_to(P0, &[1], -40, 24)]);
    run(&mut sim, 20 * 30);
    assert!(
        position(&sim, 1).x > Fixed::ZERO,
        "no way back without the bridge"
    );
    // And laid again in the same slot.
    sim.step(&[cmd(
        P0,
        Command::LayBridge {
            units: ids(&[0]),
            target: Vec2::from_meters(0, 30),
        },
    )]);
    run_until(&mut sim, 20 * 240, |s| {
        matches!(s.layers()[0].state, LayerState::Laid(0))
    });
    assert_eq!(sim.map().bridges().len(), 1);
}

#[test]
fn camouflage_hides_standing_vehicles_from_afar() {
    let map = Arc::new(map_with("crossing_camo", 401, 401, |_, _| false));
    let mut sim = sim_on(
        map,
        &[(P0, "de_pz4_h", 0, 0), (P1, "su_t34_76_1943", 0, 150)],
        5,
    );
    // 150 m: inside the 250 m sight (2000 m real x 1/8), outside the 100 m
    // (40 %) that camouflage leaves.
    run(&mut sim, 20);
    assert!(sim.visible_to(P1, UnitId(0)), "seen at 150 m in the open");
    sim.step(&[cmd(
        P0,
        Command::SetCamouflage {
            units: ids(&[0, 1]),
            on: true,
        },
    )]);
    assert_eq!(sim.camouflage_of(UnitId(0)), Some(false));
    assert_eq!(sim.camouflage_of(UnitId(1)), None, "only own units");
    run(&mut sim, 20 * 34);
    assert_eq!(sim.camouflage_of(UnitId(0)), Some(true));
    assert!(sim.camouflage_hides(UnitId(0)));
    assert!(!sim.visible_to(P1, UnitId(0)), "hidden at 150 m");
    assert!(sim.visible_to(P0, UnitId(0)));
    // The enemy comes closer than 40 % of its sight range (100 m).
    sim.step(&[move_to(P1, &[1], 0, 60)]);
    run_until(&mut sim, 20 * 60, |s| s.visible_to(P1, UnitId(0)));
    // Driving: seen again even from afar.
    sim.step(&[move_to(P1, &[1], 0, 150)]);
    run(&mut sim, 20 * 40);
    assert!(!sim.visible_to(P1, UnitId(0)));
    sim.step(&[move_to(P0, &[0], 30, 0)]);
    run(&mut sim, 10);
    assert!(sim.visible_to(P1, UnitId(0)), "moving vehicles are seen");
    sim.step(&[cmd(
        P0,
        Command::SetCamouflage {
            units: ids(&[0]),
            on: false,
        },
    )]);
    assert_eq!(sim.camouflage_of(UnitId(0)), None);
}

#[test]
fn a_dragged_bomb_run_lays_a_carpet_along_the_line() {
    let map = Arc::new(map_with("crossing_carpet", 401, 401, |_, _| false));
    let mut sim = sim_on(
        map,
        &[(P0, "de_pz4_h", 0, 350), (P1, "su_t34_76_1943", 0, -350)],
        6,
    );
    sim.step(&[cmd(
        P0,
        Command::CallAirLine {
            mission: AirMission::Bomb,
            from: Vec2::from_meters(-120, -50),
            to: Vec2::from_meters(120, -50),
        },
    )]);
    let mut impacts: Vec<Vec2> = Vec::new();
    for _ in 0..20 * 120 {
        sim.step(&[]);
        for b in &sim.snapshot().bombs {
            let t = b.target.ground();
            if !impacts.contains(&t) {
                impacts.push(t);
            }
        }
        // The aircraft fly along the line (east-west).
        for a in &sim.snapshot().aircraft {
            assert!(a.velocity.x.abs() > a.velocity.y.abs() * 4);
        }
    }
    assert!(impacts.len() >= 12, "{} bombs", impacts.len());
    let min = impacts.iter().map(|p| p.x).min().unwrap();
    let max = impacts.iter().map(|p| p.x).max().unwrap();
    assert!(
        min < Fixed::from_num(-90) && max > Fixed::from_num(90),
        "{min} .. {max}"
    );
    assert!(
        impacts
            .iter()
            .all(|p| (p.y + Fixed::from_num(50)).abs() < Fixed::from_num(120))
    );
}

#[test]
fn crossings_are_deterministic() {
    let play = || {
        let mut sim = sim_on(
            river_with_bridge(),
            &[
                (P0, ENGINEERS, -30, 1),
                (P0, RIFLES, 0, 1),
                (P0, "de_pz4_h", -30, 20),
                (P1, "su_inf_rifle_squad", 40, 1),
            ],
            13,
        );
        sim.step(&[
            cmd(
                P0,
                Command::DemolishBridge {
                    units: ids(&[0]),
                    bridge: 0,
                },
            ),
            cmd(
                P0,
                Command::SetCamouflage {
                    units: ids(&[2]),
                    on: true,
                },
            ),
        ]);
        let events = run(&mut sim, 20 * 90);
        let combat: Vec<CombatEvent> = sim.events().to_vec();
        (sim.state_hash(), events.len(), combat.len())
    };
    let a = play();
    assert_eq!(a, play());
    assert!(a.1 > 0);
}
