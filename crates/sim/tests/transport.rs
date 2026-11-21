//! Transport and towing (prompt 034): mounting and dismounting, seats,
//! riders on tanks, losses when the carrier is destroyed, towing and
//! unlimbering guns, guns pushed by their crew, determinism.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, UnitSnapshot, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const HALFTRACK: &str = "de_sdkfz251_1";
const TRUCK: &str = "de_opel_blitz";
const SQUAD: &str = "de_inf_rifle_squad";
const PAK: &str = "de_pak40";

/// Flat 800 m × 800 m around the origin.
fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "transport_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn sim_with(spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
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

fn embark(units: &[u32], carrier: u32) -> PlayerCommand {
    cmd(
        P0,
        Command::Embark {
            units: ids(units),
            carrier: UnitId(carrier),
        },
    )
}

fn move_to(units: &[u32], x: i32, y: i32) -> PlayerCommand {
    cmd(
        P0,
        Command::MoveTo {
            units: ids(units),
            target: Vec2::from_meters(x, y),
        },
    )
}

fn unit(sim: &Sim, id: u32) -> UnitSnapshot {
    *sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
}

/// Steps until `done` holds (at most `limit` ticks); the ticks it took.
fn run_until(sim: &mut Sim, limit: u32, done: impl Fn(&Sim) -> bool) -> u32 {
    for tick in 0..limit {
        if done(sim) {
            return tick;
        }
        sim.step(&[]);
    }
    panic!("condition not met within {limit} ticks");
}

fn distance(a: Vec2, b: Vec2) -> Fixed {
    (a - b).length()
}

#[test]
fn squad_mounts_rides_along_and_dismounts() {
    let mut sim = sim_with(&[(P0, HALFTRACK, 0, 0), (P0, SQUAD, 30, 0)], 1);
    sim.step(&[embark(&[1], 0)]);
    // The squad stops 15 m farther out than the mounting reach, its men
    // run up (catch-up speed, upright) and climb through the rear doors one
    // after another, 0.5 s real (5 ticks) each (prompt 163; before: the
    // squad walked up and all mounted after 12 s real). Nine men need at
    // least 45 ticks of climbing; the run of ~25 m about 6 s.
    let mut up_before_aboard = 0;
    let mut ticks = 0;
    while unit(&sim, 1).carrier != Some(UnitId(0)) {
        assert!(ticks < 20 * 60, "not aboard after a minute");
        let up = sim
            .snapshot()
            .soldiers
            .iter()
            .filter(|s| s.height > Fixed::ONE / 2)
            .count();
        up_before_aboard = up_before_aboard.max(up);
        sim.step(&[]);
        ticks += 1;
    }
    assert!((100..=240).contains(&ticks), "mounted in {ticks} ticks");
    // Men stood on the halftrack before the last one was up.
    assert!(
        up_before_aboard >= 4,
        "only {up_before_aboard} men up before the squad boarded"
    );
    assert_eq!(unit(&sim, 0).passengers, 9);
    assert!(
        sim.snapshot().soldiers.is_empty(),
        "soldiers aboard are not drawn"
    );
    // Passengers take no orders.
    sim.step(&[move_to(&[1], -100, 0)]);
    sim.step(&[move_to(&[0], 0, 150)]);
    run_until(&mut sim, 20 * 60, |s| unit(s, 0).target.is_none());
    let (halftrack, squad) = (unit(&sim, 0), unit(&sim, 1));
    assert!(distance(halftrack.position, Vec2::from_meters(0, 150)) < Fixed::from_num(3));
    assert_eq!(squad.position, halftrack.position);
    assert_eq!(squad.carrier, Some(UnitId(0)));

    sim.step(&[cmd(P0, Command::Disembark { units: ids(&[0]) })]);
    assert!(unit(&sim, 1).loading);
    run_until(&mut sim, 20 * 30, |s| unit(s, 1).carrier.is_none());
    let squad = unit(&sim, 1);
    let away = distance(squad.position, halftrack.position);
    assert!(away < Fixed::from_num(15), "dismounted {away} m away");
    assert_eq!(unit(&sim, 0).passengers, 0);
    assert_eq!(sim.snapshot().soldiers.len(), 9);
    // Back on foot, it takes orders again.
    sim.step(&[move_to(&[1], 0, 100)]);
    assert!(unit(&sim, 1).target.is_some());
}

#[test]
fn seats_limit_the_passengers() {
    // Ten seats: one squad of nine fits, the second one does not.
    let mut sim = sim_with(
        &[
            (P0, HALFTRACK, 0, 0),
            (P0, SQUAD, 20, 0),
            (P0, SQUAD, -20, 0),
        ],
        2,
    );
    assert_eq!(sim.seats(UnitId(0)), 10);
    sim.step(&[embark(&[1, 2], 0)]);
    sim.step(&[]);
    run_until(&mut sim, 20 * 60, |s| {
        s.snapshot().units.iter().all(|u| !u.loading) && unit(s, 0).passengers > 0
    });
    for _ in 0..200 {
        sim.step(&[]);
    }
    let aboard = [1, 2]
        .iter()
        .filter(|&&k| unit(&sim, k).carrier.is_some())
        .count();
    assert_eq!(aboard, 1);
    assert_eq!(unit(&sim, 0).passengers, 9);
}

#[test]
fn riders_mount_a_tank() {
    let mut sim = sim_with(
        &[
            (P0, "su_t34_76_1943", 0, 0),
            (P0, "su_inf_rifle_squad", 0, 20),
        ],
        3,
    );
    assert_eq!(sim.seats(UnitId(0)), 10);
    sim.step(&[embark(&[1], 0)]);
    run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_some());
    assert_eq!(unit(&sim, 0).passengers, 9);
}

#[test]
fn destroyed_truck_costs_passengers() {
    // A truck with a squad aboard; an enemy tank destroys it.
    let mut sim = sim_with(
        &[
            (P0, TRUCK, 0, 0),
            (P0, SQUAD, 15, 0),
            (P1, "su_t34_76_1943", 0, 200),
        ],
        4,
    );
    sim.step(&[embark(&[1], 0)]);
    run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_some());
    // The enemy does not see the squad while it is aboard.
    assert!(!sim.visible_to(P1, UnitId(1)));
    sim.step(&[cmd(
        P1,
        Command::Attack {
            units: ids(&[2]),
            target: UnitId(0),
        },
    )]);
    let mut destroyed = false;
    for _ in 0..20 * 120 {
        sim.step(&[]);
        if sim
            .events()
            .iter()
            .any(|e| matches!(e, CombatEvent::Destroyed { unit, .. } if *unit == UnitId(0)))
        {
            destroyed = true;
            break;
        }
    }
    assert!(destroyed, "the truck survived");
    let squad = unit(&sim, 1);
    assert_eq!(squad.carrier, None, "survivors jump off");
    let alive = sim
        .snapshot()
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(1) && s.hp > 0)
        .count();
    assert!(alive < 9, "nobody died with the truck");
    if alive > 0 {
        assert!(squad.alive());
    }
}

#[test]
fn truck_tows_a_gun_and_unlimbers_it() {
    let mut sim = sim_with(&[(P0, TRUCK, 0, 0), (P0, PAK, 20, 0)], 5);
    assert!(sim.can_tow(UnitId(0), UnitId(1)));
    sim.step(&[cmd(
        P0,
        Command::Tow {
            units: ids(&[0]),
            gun: UnitId(1),
        },
    )]);
    let ticks = run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier == Some(UnitId(0)));
    assert!(ticks > 300, "30 s real limbering: {ticks} ticks");
    assert_eq!(unit(&sim, 0).towing, Some(UnitId(1)));
    // The gun cannot be moved on its own while it is towed.
    sim.step(&[move_to(&[1], 100, 100)]);
    assert!(unit(&sim, 1).target.is_none());

    sim.step(&[move_to(&[0], 0, 200)]);
    run_until(&mut sim, 20 * 120, |s| unit(s, 0).target.is_none());
    let (truck, gun) = (unit(&sim, 0), unit(&sim, 1));
    // Trailing behind the truck (it drove towards +y), muzzle to the rear.
    assert!(gun.position.y < truck.position.y - Fixed::from_num(4));
    assert!(gun.facing.y < Fixed::ZERO);

    sim.step(&[cmd(P0, Command::Unlimber { units: ids(&[0]) })]);
    run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_none());
    let gun = unit(&sim, 1);
    assert!(!gun.loading);
    assert!(gun.facing.y > Fixed::ZERO, "points the way the truck drove");
    assert_eq!(unit(&sim, 0).towing, None);
}

#[test]
fn towing_slows_the_tractor() {
    let drive = |with_gun: bool| {
        let mut sim = sim_with(&[(P0, TRUCK, 0, 0), (P0, PAK, 0, -12)], 6);
        if with_gun {
            sim.step(&[cmd(
                P0,
                Command::Tow {
                    units: ids(&[0]),
                    gun: UnitId(1),
                },
            )]);
            run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_some());
        }
        sim.step(&[move_to(&[0], 0, 300)]);
        run_until(&mut sim, 20 * 300, |s| unit(s, 0).target.is_none())
    };
    let (alone, towing) = (drive(false), drive(true));
    assert!(towing > alone, "towing {towing} ticks, alone {alone}");
}

#[test]
fn crew_pushes_a_gun_slowly() {
    let mut sim = sim_with(&[(P0, PAK, 0, 0), (P0, SQUAD, 5, 0)], 7);
    sim.step(&[move_to(&[0], 0, 20), move_to(&[1], 5, 20)]);
    for _ in 0..40 {
        sim.step(&[]);
    }
    let (gun, squad) = (unit(&sim, 0), unit(&sim, 1));
    assert!(gun.position.y > Fixed::ZERO, "the gun moves");
    assert!(gun.position.y < squad.position.y, "slower than infantry");
}

#[test]
fn towed_gun_does_not_fire_until_unlimbered() {
    let mut sim = sim_with(
        &[
            (P0, TRUCK, 0, 0),
            (P0, PAK, 0, -12),
            // Within the 87.5 m of the PaK 40 (700 m real × 1/8, prompt
            // 097) wherever the truck leaves it.
            (P1, "su_t34_76_1943", 0, 60),
        ],
        8,
    );
    sim.step(&[cmd(
        P0,
        Command::Tow {
            units: ids(&[0]),
            gun: UnitId(1),
        },
    )]);
    run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_some());
    sim.step(&[cmd(
        P0,
        Command::SetFireMode {
            units: ids(&[1]),
            mode: FireMode::FireAtWill,
        },
    )]);
    let fired = |sim: &Sim| {
        sim.events()
            .iter()
            .any(|e| matches!(e, CombatEvent::Shot { unit, .. } if *unit == UnitId(1)))
    };
    for _ in 0..200 {
        sim.step(&[]);
        assert!(!fired(&sim), "a towed gun fired");
    }
    sim.step(&[cmd(P0, Command::Unlimber { units: ids(&[1]) })]);
    let mut shots = 0;
    for _ in 0..20 * 40 {
        sim.step(&[]);
        if fired(&sim) {
            shots += 1;
        }
    }
    assert!(shots > 0, "the unlimbered gun never fired");
}

#[test]
fn too_heavy_guns_are_not_towed() {
    let mut sim = sim_with(&[(P0, TRUCK, 0, 0), (P0, "de_flak36", 20, 0)], 9);
    assert!(!sim.can_tow(UnitId(0), UnitId(1)));
    sim.step(&[cmd(
        P0,
        Command::Tow {
            units: ids(&[0]),
            gun: UnitId(1),
        },
    )]);
    for _ in 0..20 * 40 {
        sim.step(&[]);
    }
    assert_eq!(unit(&sim, 1).carrier, None);
    assert_eq!(unit(&sim, 0).target, None);
}

#[test]
fn transport_is_deterministic() {
    let play = || {
        let mut sim = sim_with(
            &[
                (P0, TRUCK, 0, 0),
                (P0, SQUAD, 15, 0),
                (P0, HALFTRACK, -20, 0),
                (P0, PAK, -20, -15),
                (P1, "su_t34_76_1943", 0, 110),
            ],
            11,
        );
        sim.step(&[
            embark(&[1], 0),
            cmd(
                P0,
                Command::Tow {
                    units: ids(&[2]),
                    gun: UnitId(3),
                },
            ),
        ]);
        for tick in 0..20 * 90 {
            let commands = match tick {
                700 => vec![move_to(&[0, 2], 0, 120)],
                1300 => vec![
                    cmd(P0, Command::Disembark { units: ids(&[0]) }),
                    cmd(P0, Command::Unlimber { units: ids(&[2]) }),
                ],
                _ => Vec::new(),
            };
            sim.step(&commands);
        }
        sim
    };
    let (a, b) = (play(), play());
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.snapshot(), b.snapshot());
}

/// Prompt 163: game seconds from the order until the squad is aboard
/// (standing beside the carrier, and 25 m away) and from the order until it
/// is off again, per carrier. Printed with `--nocapture`.
#[test]
#[ignore]
fn measure_mounting_times() {
    for carrier in [TRUCK, HALFTRACK, "de_pz4_h"] {
        for gap in [6, 25] {
            let mut sim = sim_with(&[(P0, carrier, 0, 0), (P0, SQUAD, gap, 0)], 1);
            sim.step(&[embark(&[1], 0)]);
            let on = run_until(&mut sim, 20 * 120, |s| {
                unit(s, 1).carrier == Some(UnitId(0))
            });
            sim.step(&[cmd(P0, Command::Disembark { units: ids(&[0]) })]);
            let off = run_until(&mut sim, 20 * 60, |s| unit(s, 1).carrier.is_none());
            println!(
                "MEASURE {carrier} gap {gap} m: on {:.2} s, off {:.2} s",
                f64::from(on + 1) / 20.0,
                f64::from(off + 1) / 20.0
            );
        }
    }
}
