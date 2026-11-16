//! Supply and repair (prompt 036): ammunition and fuel consumption,
//! resupply near trucks (paid, also by allies, not by enemies), immobilized
//! vehicles and their recovery by repair, building repair, selling and
//! cancelling, determinism.

use sim::map::MapLayout;
use sim::scenario::{builtin_map, skirmish_config};
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, UnitSnapshot, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);
const PZ4: &str = "de_pz4_h";
const TIGER: &str = "de_tiger1_e";
const TRUCK: &str = "de_opel_blitz";

/// Flat 800 m × 800 m around the origin.
fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "logistics_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn sim_with(spawns: &[(PlayerId, &str, i32, i32)], teams: sim::Teams) -> Sim {
    let map = flat_map();
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
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams,
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, 3).unwrap()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn unit(sim: &Sim, id: u32) -> UnitSnapshot {
    *sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
}

fn ammo(sim: &Sim, id: u32) -> u8 {
    unit(sim, id).stock.ammo_pct.unwrap()
}

fn fuel(sim: &Sim, id: u32) -> u8 {
    unit(sim, id).stock.fuel_pct.unwrap()
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

/// Shots of unit `id` during `ticks` ticks, by weapon 0 and the others.
fn count_shots(sim: &mut Sim, id: u32, ticks: u32) -> (u32, u32) {
    let mut shots = (0, 0);
    for _ in 0..ticks {
        sim.step(&[]);
        for e in &sim.snapshot().events {
            if let CombatEvent::Shot { unit, weapon, .. } = e
                && *unit == UnitId(id)
            {
                if *weapon == 0 {
                    shots.0 += 1;
                } else {
                    shots.1 += 1;
                }
            }
        }
    }
    shots
}

/// Unit 0 (a Panzer IV of player 0) fires at the ground until its
/// ammunition is gone.
fn empty_tank(extra: &[(PlayerId, &str, i32, i32)]) -> Sim {
    let mut spawns = vec![(P0, PZ4, 0, 0)];
    spawns.extend_from_slice(extra);
    let mut sim = sim_with(&spawns, sim::Teams::default());
    sim.step(&[cmd(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 150),
        },
    )]);
    let (main, _) = count_shots(&mut sim, 0, 20 * 400);
    // 87 rounds of the 7.5 cm KwK 40 (catalog `ammo_carried`).
    assert_eq!(main, 87);
    assert_eq!(ammo(&sim, 0), 0);
    sim.step(&[cmd(
        P0,
        Command::Stop {
            units: vec![UnitId(0)],
        },
    )]);
    sim
}

#[test]
fn main_weapon_uses_up_its_stock_then_nothing_fires() {
    let mut sim = empty_tank(&[]);
    // Without ammunition no weapon fires, the machine guns neither.
    sim.step(&[cmd(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 150),
        },
    )]);
    assert_eq!(count_shots(&mut sim, 0, 20 * 30), (0, 0));
}

#[test]
fn trucks_resupply_own_and_allied_units_for_a_price() {
    // Player 0's truck is 20 m away, player 1 (an ally) has an empty tank
    // next to it, player 2 (an enemy) too.
    let mut sim = empty_tank(&[]);
    let before = sim.account(P0).unwrap();
    sim.step(&[]);
    assert_eq!(sim.account(P0), Some(before), "nothing without a supplier");

    let teams = sim::Teams::new(&[(P0, 0), (P1, 0), (P2, 1)], true);
    let mut sim = sim_with(
        &[
            (P0, TRUCK, 0, -300),
            (P1, PZ4, 20, 0),
            (P2, PZ4, -20, 0),
            (P1, PZ4, 200, 0),
        ],
        teams,
    );
    // Empty both tanks first: fire at the ground far away.
    for (player, id) in [(P1, 1), (P2, 2), (P1, 3)] {
        sim.step(&[cmd(
            player,
            Command::AttackGround {
                units: vec![UnitId(id)],
                target: Vec2::from_meters(-300 + 300 * i32::from(player.0 == P1.0), 300),
            },
        )]);
    }
    run(&mut sim, 20 * 30);
    let (ally, enemy, far) = (ammo(&sim, 1), ammo(&sim, 2), ammo(&sim, 3));
    assert!(ally < 100 && enemy < 100 && far < 100);
    for (player, id) in [(P1, 1), (P2, 2), (P1, 3)] {
        sim.step(&[cmd(
            player,
            Command::Stop {
                units: vec![UnitId(id)],
            },
        )]);
    }
    // The truck drives up.
    sim.step(&[cmd(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 0),
        },
    )]);
    let mut ticks = 0;
    while (unit(&sim, 0).position - Vec2::ZERO).length() > Fixed::from_num(30) {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 120, "truck did not arrive");
    }
    let (ally, enemy, far) = (ammo(&sim, 1), ammo(&sim, 2), ammo(&sim, 3));
    let account = sim.account(P1).unwrap();
    run(&mut sim, 20 * 5);
    // 5 % per second for five seconds.
    assert!(ammo(&sim, 1) >= (ally + 24).min(100), "ally resupplied");
    assert_eq!(ammo(&sim, 2), enemy, "enemies are not supplied");
    assert_eq!(ammo(&sim, 3), far, "out of reach");
    // The receiver pays (half the tank's ammunition price per full load).
    let paid = account[1] - sim.account(P1).unwrap()[1];
    let price = sim
        .cost_of(sim.unit_type_index(PZ4).unwrap())
        .unwrap()
        .resources[1];
    assert!(
        paid > 0 && paid <= price / 2 / 4 + 1,
        "paid {paid} of {price}"
    );
    run(&mut sim, 20 * 30);
    assert_eq!(ammo(&sim, 1), 100);
}

#[test]
fn vehicles_burn_fuel_by_distance_and_stop_when_dry() {
    let mut sim = sim_with(&[(P0, TIGER, -350, 0)], sim::Teams::default());
    assert_eq!(fuel(&sim, 0), 100);
    sim.step(&[cmd(
        P0,
        Command::Patrol {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(350, 0),
        },
    )]);
    // 195 km road range / 20 = 9750 m: 700 m are 7 %.
    let start = unit(&sim, 0).position;
    let mut ticks = 0;
    while (unit(&sim, 0).position - start).length() < Fixed::from_num(690) {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 300);
    }
    let left = fuel(&sim, 0);
    assert!((91..=93).contains(&left), "fuel {left} %");
    // Patrolling on, the Tiger runs dry after ~9.75 km and stands.
    let mut ticks = 0;
    while fuel(&sim, 0) > 0 {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 3000, "never ran dry");
    }
    // The last percent (97 m).
    run(&mut sim, 20 * 30);
    let parked = unit(&sim, 0).position;
    run(&mut sim, 20 * 10);
    assert_eq!(unit(&sim, 0).position, parked);
    assert!(unit(&sim, 0).target.is_none());
}

#[test]
fn hit_tanks_are_immobilized() {
    // An enemy Tiger hits the Panzer IV once (8.8 cm: 774 of 1000 points).
    let mut sim = sim_with(
        &[(P0, PZ4, 0, 0), (P1, TIGER, 0, 200)],
        sim::Teams::default(),
    );
    sim.step(&[cmd(
        P1,
        Command::Attack {
            units: vec![UnitId(1)],
            target: UnitId(0),
        },
    )]);
    let mut ticks = 0;
    while unit(&sim, 0).hp == unit(&sim, 0).max_hp {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 60, "no hit");
    }
    sim.step(&[cmd(
        P1,
        Command::Stop {
            units: vec![UnitId(1)],
        },
    )]);
    let hit = unit(&sim, 0);
    assert!(hit.alive() && hit.hp <= hit.max_hp * 35 / 100);
    assert!(hit.stock.immobile);
    // It cannot drive.
    sim.step(&[cmd(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(-100, 0),
        },
    )]);
    run(&mut sim, 20 * 10);
    assert_eq!(unit(&sim, 0).position, hit.position);
}

#[test]
fn trucks_recover_immobilized_vehicles_by_repair() {
    let mut sim = sim_with(
        &[(P0, PZ4, 0, 0), (P1, TIGER, 0, 200), (P0, TRUCK, -100, 0)],
        sim::Teams::default(),
    );
    sim.step(&[cmd(
        P1,
        Command::Attack {
            units: vec![UnitId(1)],
            target: UnitId(0),
        },
    )]);
    while unit(&sim, 0).hp == unit(&sim, 0).max_hp {
        sim.step(&[]);
    }
    sim.step(&[cmd(
        P1,
        Command::Stop {
            units: vec![UnitId(1)],
        },
    )]);
    assert!(unit(&sim, 0).stock.immobile);
    let hp = unit(&sim, 0).hp;
    // Out of reach (100 m): nothing happens.
    run(&mut sim, 20 * 5);
    assert_eq!(unit(&sim, 0).hp, hp);
    sim.step(&[cmd(
        P0,
        Command::MoveTo {
            units: vec![UnitId(2)],
            target: Vec2::from_meters(-15, 0),
        },
    )]);
    let mut ticks = 0;
    while unit(&sim, 0).stock.immobile {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 120, "never recovered");
    }
    assert!(unit(&sim, 0).hp > hp);
    // Mobile again: it drives.
    let at = unit(&sim, 0).position;
    sim.step(&[cmd(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, -100),
        },
    )]);
    run(&mut sim, 20 * 10);
    assert!((unit(&sim, 0).position - at).length() > Fixed::from_num(20));
}

/// The skirmish with every unit holding fire, and a Tiger of player 1 next
/// to player 0's headquarters.
fn skirmish_with_tiger() -> (Sim, UnitId, UnitId) {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let sim = Sim::new(&config, builtin_map(), 7).unwrap();
    let hq = sim
        .structures()
        .into_iter()
        .find(|s| s.owner == P0 && s.unit_type == sim.unit_type_index("base_hq").unwrap())
        .unwrap()
        .unit;
    let at = sim.unit_position(hq).unwrap();
    config.spawns.push(Spawn {
        owner: P1,
        unit_type: TIGER.into(),
        position: at + Vec2::from_meters(0, if at.y > Fixed::ZERO { -120 } else { 120 }),
        fire_mode: FireMode::HoldFire,
    });
    let tiger = UnitId(u32::try_from(config.spawns.len() - 1).unwrap());
    (Sim::new(&config, builtin_map(), 7).unwrap(), hq, tiger)
}

#[test]
fn buildings_are_repaired_for_a_price_and_sold() {
    let (mut sim, hq, tiger) = skirmish_with_tiger();
    sim.step(&[cmd(
        P1,
        Command::Attack {
            units: vec![tiger],
            target: hq,
        },
    )]);
    let full = sim.unit_hp(hq).unwrap();
    let mut ticks = 0;
    while sim.unit_hp(hq).unwrap() >= full - 1000 {
        sim.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 120, "headquarters not hit");
    }
    sim.step(&[cmd(P1, Command::Stop { units: vec![tiger] })]);
    run(&mut sim, 20);
    let damaged = sim.unit_hp(hq).unwrap();
    let account = sim.account(P0).unwrap();
    sim.step(&[cmd(P0, Command::Repair { building: hq })]);
    assert!(sim.building_repairing(hq));
    // A foreign player cannot stop it.
    sim.step(&[cmd(P1, Command::Repair { building: hq })]);
    assert!(sim.building_repairing(hq));
    run(&mut sim, 20 * 5);
    assert!(sim.unit_hp(hq).unwrap() > damaged);
    // Repair costs manpower and fuel (half the price for a full repair).
    let after = sim.account(P0).unwrap();
    let income = sim.income(P0);
    assert!(after[2] < account[2] + income[2] * 6 / 60);
    // Repaired to full, the order ends.
    run(&mut sim, 20 * 60);
    assert_eq!(sim.unit_hp(hq), Some(full));
    assert!(!sim.building_repairing(hq));

    // Selling: half the price back, the building is gone.
    let price = sim
        .cost_of(sim.unit_type_index("base_hq").unwrap())
        .unwrap()
        .resources;
    let before = sim.account(P0).unwrap();
    let count = sim.snapshot().units.len();
    sim.step(&[cmd(P1, Command::Sell { building: hq })]);
    assert_eq!(sim.unit_hp(hq), Some(full), "only the owner sells");
    sim.step(&[cmd(P0, Command::Sell { building: hq })]);
    assert_eq!(sim.unit_hp(hq), Some(0));
    let after = sim.account(P0).unwrap();
    assert!(after[2] >= before[2] + price[2] / 2);
    assert_eq!(sim.snapshot().units.len(), count - 1);
    assert!(sim.structures().iter().all(|s| s.unit != hq));
}

#[test]
fn construction_sites_can_be_cancelled() {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let mut sim = Sim::new(&config, builtin_map(), 7).unwrap();
    let barracks = sim.unit_type_index("base_barracks").unwrap();
    let hq = sim
        .structures()
        .into_iter()
        .find(|s| s.owner == P0)
        .unwrap()
        .unit;
    let home = sim.unit_position(hq).unwrap();
    let spot = sim.find_site(P0, barracks, home).unwrap();
    let before = sim.account(P0).unwrap();
    sim.step(&[cmd(
        P0,
        Command::Build {
            unit_type: barracks,
            position: spot,
        },
    )]);
    let site = sim
        .structures()
        .into_iter()
        .find(|s| s.unit_type == barracks)
        .unwrap()
        .unit;
    sim.step(&[cmd(P0, Command::Sell { building: site })]);
    assert!(sim.structures().iter().all(|s| s.unit != site));
    // 75 % of the 150 manpower back; the ground is free again.
    let after = sim.account(P0).unwrap();
    assert!(after[2] + 150 / 4 <= before[2] + 1, "{after:?} {before:?}");
    assert!(after[2] + 150 / 4 + 2 >= before[2], "{after:?} {before:?}");
    assert_eq!(sim.placement(P0, barracks, spot), Ok(()));
}

#[test]
fn supply_is_deterministic_and_survives_a_snapshot_roundtrip() {
    let play = || {
        let mut sim = sim_with(
            &[(P0, PZ4, 0, 0), (P0, TRUCK, 30, 0), (P1, TIGER, 0, 250)],
            sim::Teams::default(),
        );
        sim.step(&[cmd(
            P0,
            Command::AttackGround {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(-200, 100),
            },
        )]);
        run(&mut sim, 20 * 60);
        sim.state_hash()
    };
    assert_eq!(play(), play());
}
