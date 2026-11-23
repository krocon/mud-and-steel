//! Boats on the water (prompt 119): an LCVP runs onto a beach and lands a
//! platoon, an LCM lands a tank, a sunk boat leaves its men swimming (or
//! drowning) and its tank lost, an assault boat ferries a squad over a
//! river, boats never leave the water, determinism.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Spawn, TerrainRules, UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const GRASS: u8 = 0;
const SAND: u8 = 3;
const SHALLOW: u8 = 6;
const DEEP: u8 = 7;
const LCVP: &str = "us_lcvp";
const LCM: &str = "us_lcm3";
const M2: &str = "us_m2_assault_boat";
const RIFLES: &str = "us_inf_rifle_squad";
const SHERMAN: &str = "us_m4a3_75";
const PAK: &str = "de_pak40";

/// A coast, 200 m × 300 m around the origin: the sea in the
/// north (y below 0 m), a sand beach rising from 0 to 1 m between y = 0 and
/// 20 m, grass behind it. The sea bed falls 6 cm per meter from the
/// waterline, 26 m of shallows (to 1.56 m), then deep water, at most 6 m
/// deep.
fn coast() -> Arc<Map> {
    let (sx, sy) = (101u32, 151u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for _ in 0..sx {
            let y = 2 * j as i32 - (sy as i32 - 1);
            let (h, t) = if y >= 20 {
                (1000, GRASS)
            } else if y >= 0 {
                (50 * y, SAND)
            } else if y >= -26 {
                (60 * y, SHALLOW)
            } else {
                ((60 * y).max(-6000), DEEP)
            };
            heights.push(h);
            terrain.push(t);
        }
    }
    let layout = MapLayout {
        name: "boat_coast".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
}

/// 200 m × 120 m with a river 40 m wide at |x| <= 20 m, 2 m below the
/// banks.
fn river() -> Arc<Map> {
    let (sx, sy) = (101u32, 61u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for _ in 0..sy {
        for i in 0..sx {
            let x = 2 * i as i32 - 100;
            let water = x.abs() <= 20;
            heights.push(if water { -2000 } else { 0 });
            terrain.push(if water { DEEP } else { GRASS });
        }
    }
    let layout = MapLayout {
        name: "boat_river".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
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

fn cmd(command: Command) -> Vec<PlayerCommand> {
    vec![PlayerCommand::new(P0, command)]
}

fn land(units: &[u32], x: i32, y: i32) -> Vec<PlayerCommand> {
    cmd(Command::Land {
        units: ids(units),
        target: Vec2::from_meters(x, y),
    })
}

fn disembark(units: &[u32]) -> Vec<PlayerCommand> {
    cmd(Command::Disembark { units: ids(units) })
}

fn embark(units: &[u32], carrier: u32) -> Vec<PlayerCommand> {
    cmd(Command::Embark {
        units: ids(units),
        carrier: UnitId(carrier),
    })
}

fn run_until(sim: &mut Sim, limit: u32, done: impl Fn(&Sim) -> bool) -> u32 {
    for t in 0..limit {
        if done(sim) {
            return t;
        }
        sim.step(&[]);
    }
    panic!("condition not met within {limit} ticks");
}

fn position(sim: &Sim, id: u32) -> Vec2 {
    sim.unit_position(UnitId(id)).unwrap()
}

fn phase(sim: &Sim, id: u32) -> &'static str {
    sim.boat_phase(UnitId(id)).unwrap()
}

fn alive(sim: &Sim, id: u32) -> bool {
    sim.unit_hp(UnitId(id)).is_some_and(|hp| hp > 0)
}

fn m(v: Fixed) -> i32 {
    v.to_num::<i32>()
}

/// Boats keep to water deep enough for their draft: every tick of a
/// drive along the coast, the cell under a boat is open to it.
fn assert_afloat(sim: &Sim, id: u32) {
    let p = position(sim, id);
    let depth = sim.path_grid().water_depth_at(p);
    assert!(depth > 0, "boat {id} on land at {p:?}");
}

#[test]
fn lcvp_lands_a_platoon_on_the_beach() {
    // An LCVP 80 m out at sea with three rifle squads (36 men) aboard.
    let mut sim = sim_on(
        coast(),
        &[
            (P0, LCVP, 0, -90),
            (P0, RIFLES, 0, 60),
            (P0, RIFLES, 4, 60),
            (P0, RIFLES, 8, 60),
            (P1, RIFLES, 90, 90),
        ],
        7,
    );
    for squad in 1..=3 {
        assert!(sim.put_aboard(UnitId(squad), UnitId(0)), "squad {squad}");
    }
    assert!(sim.is_aboard(UnitId(3)));
    assert_eq!(sim.transport_load(UnitId(0)).0, 36);
    sim.step(&land(&[0], 0, 5));
    let t = run_until(&mut sim, 2400, |s| phase(s, 0) == "beached");
    // It grounded in the shallows, where the water under its bow is less
    // than its draft (0.9 m: 15 m out on this beach), not on the sand.
    let p = position(&sim, 0);
    assert!(
        m(p.y) > -25 && m(p.y) < 0,
        "beached at {p:?} after {t} ticks"
    );
    sim.step(&disembark(&[0]));
    run_until(&mut sim, 1200, |s| !(1..=3).any(|q| s.is_aboard(UnitId(q))));
    // The platoon wades ashore and nobody drowned.
    for q in 1..=3 {
        assert!(alive(&sim, q));
        assert!(
            m(position(&sim, q).y) > m(p.y),
            "squad {q} in front of the bow"
        );
    }
    assert_eq!(sim.transport_load(UnitId(0)).0, 0);
    // Backing off: afloat again in open water.
    sim.step(&cmd(Command::Retract { units: ids(&[0]) }));
    run_until(&mut sim, 1200, |s| phase(s, 0) == "afloat");
    assert_afloat(&sim, 0);
}

#[test]
fn lcm_lands_a_sherman() {
    let mut sim = sim_on(
        coast(),
        &[
            (P0, LCM, 20, -100),
            (P0, SHERMAN, 0, 60),
            (P1, RIFLES, 90, 90),
        ],
        3,
    );
    assert!(sim.put_aboard(UnitId(1), UnitId(0)), "the Sherman fits");
    assert_eq!(sim.boat_cargo(UnitId(0)), vec![UnitId(1)]);
    // Aboard it rides with the boat and the enemy does not see it.
    let snap = sim.snapshot();
    let tank = snap.units.iter().find(|u| u.id == UnitId(1)).unwrap();
    assert_eq!(tank.carrier, Some(UnitId(0)));
    sim.step(&[]);
    assert!(sim.snapshot_for(P1).units.iter().all(|u| u.id != UnitId(1)));
    sim.step(&land(&[0], 20, 10));
    // On the way it never leaves the water.
    let mut ticks = 0;
    while phase(&sim, 0) != "beached" {
        sim.step(&[]);
        if matches!(phase(&sim, 0), "afloat" | "landing") {
            assert_afloat(&sim, 0);
        }
        ticks += 1;
        assert!(ticks < 3000, "never beached");
    }
    sim.step(&disembark(&[0]));
    run_until(&mut sim, 1200, |s| !s.is_aboard(UnitId(1)));
    assert!(alive(&sim, 1), "the tank drove off the ramp");
    // It drives up the beach out of the water.
    run_until(&mut sim, 1200, |s| m(position(s, 1).y) >= -8);
}

#[test]
fn a_second_tank_does_not_fit_and_a_gun_is_too_much_for_an_assault_boat() {
    let mut sim = sim_on(
        coast(),
        &[
            (P0, LCM, 20, -100),
            (P0, SHERMAN, 0, 60),
            (P0, SHERMAN, 10, 60),
            (P0, M2, -40, -60),
            (P0, PAK, 40, 60),
        ],
        3,
    );
    assert!(sim.put_aboard(UnitId(1), UnitId(0)));
    assert!(!sim.put_aboard(UnitId(2), UnitId(0)), "one tank only");
    assert!(
        !sim.put_aboard(UnitId(4), UnitId(3)),
        "no vehicles on an M2"
    );
}

#[test]
fn sunk_boats_lose_their_tank_and_their_men_swim_or_drown() {
    // An LCM with a Sherman and an LCVP with a squad, 40 m out; two enemy
    // Shermans on the beach shoot them to pieces.
    let mut sim = sim_on(
        coast(),
        &[
            (P0, LCM, -15, -45),
            (P0, LCVP, 15, -45),
            (P0, SHERMAN, 0, 100),
            (P0, RIFLES, 10, 100),
            (P1, SHERMAN, -10, 40),
            (P1, SHERMAN, 10, 40),
        ],
        11,
    );
    assert!(sim.put_aboard(UnitId(2), UnitId(0)));
    assert!(sim.put_aboard(UnitId(3), UnitId(1)));
    sim.step(&[
        PlayerCommand::new(
            P1,
            Command::Attack {
                units: ids(&[4]),
                target: UnitId(0),
            },
        ),
        PlayerCommand::new(
            P1,
            Command::Attack {
                units: ids(&[5]),
                target: UnitId(1),
            },
        ),
    ]);
    run_until(&mut sim, 6000, |s| !alive(s, 0) && !alive(s, 1));
    sim.step(&[]);
    assert!(!alive(&sim, 2), "the tank went down with the boat");
    assert!(!sim.is_aboard(UnitId(3)));
    // Survivors are in deep water 40 m from the beach: they swim, and
    // some drown on the way.
    let before = sim.squad_alive(UnitId(3)).unwrap_or(0);
    let swimming = sim.swimmers().iter().any(|s| s.unit == UnitId(3));
    assert!(swimming || before == 0, "{before} men, not swimming");
    assert!(before < 12, "the sinking cost men");
    for _ in 0..2400 {
        sim.step(&[]);
    }
    assert!(sim.squad_alive(UnitId(3)).unwrap_or(0) <= before);
    // No wreck blocks the deep water there: a boat still finds its way.
    let grid = sim.path_grid();
    let lcm = 1; // profile of us_lcm3 (types sorted by name)
    assert_eq!(grid.class_of(lcm), sim::path::SizeClass::Boat);
    assert!(grid.passable_for(lcm, Vec2::from_meters(-15, -45)));
}

#[test]
fn assault_boat_ferries_a_squad_over_the_river() {
    // The M2 lies in the river; a squad waits on the west bank.
    let mut sim = sim_on(
        river(),
        &[(P0, M2, 0, 0), (P0, RIFLES, -40, 0), (P1, RIFLES, 90, 50)],
        5,
    );
    sim.step(&land(&[0], -25, 0));
    // Paddled by its crew alone it is slow (a third of its speed).
    run_until(&mut sim, 4000, |s| phase(s, 0) == "beached");
    sim.step(&embark(&[1], 0));
    run_until(&mut sim, 4000, |s| s.is_aboard(UnitId(1)));
    sim.step(&land(&[0], 25, 0));
    run_until(&mut sim, 6000, |s| {
        phase(s, 0) == "beached" && m(position(s, 0).x) > 0
    });
    sim.step(&disembark(&[1]));
    run_until(&mut sim, 1200, |s| !s.is_aboard(UnitId(1)));
    assert!(m(position(&sim, 1).x) > 20, "on the east bank");
    assert_eq!(sim.squad_alive(UnitId(1)), Some(12));
}

#[test]
fn paddled_boat_slows_down_with_fewer_paddlers() {
    let mut full = sim_on(river(), &[(P0, M2, 0, -40), (P0, RIFLES, -40, 0)], 1);
    let mut empty = sim_on(river(), &[(P0, M2, 0, -40), (P0, RIFLES, -40, 0)], 1);
    assert!(full.put_aboard(UnitId(1), UnitId(0)));
    for sim in [&mut full, &mut empty] {
        sim.step(&cmd(Command::MoveTo {
            units: ids(&[0]),
            target: Vec2::from_meters(0, 40),
        }));
        for _ in 0..200 {
            sim.step(&[]);
        }
    }
    let (a, b) = (position(&full, 0).y, position(&empty, 0).y);
    assert!(a > b, "full {a} faster than crew only {b}");
}

#[test]
fn landings_are_deterministic() {
    let run = || {
        let mut sim = sim_on(
            coast(),
            &[
                (P0, LCVP, 0, -90),
                (P0, RIFLES, 0, 60),
                (P1, RIFLES, 90, 90),
            ],
            9,
        );
        sim.put_aboard(UnitId(1), UnitId(0));
        sim.step(&land(&[0], 0, 5));
        for t in 0..1500 {
            if t == 900 {
                sim.step(&disembark(&[0]));
            } else {
                sim.step(&[]);
            }
        }
        sim.state_hash()
    };
    assert_eq!(run(), run());
}
