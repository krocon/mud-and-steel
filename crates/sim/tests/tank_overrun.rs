//! Tanks against walls, infantry and buildings (prompt 093): breaking
//! through mud and dry-stone walls, running over enemy soldiers, machine
//! guns with their own infantry targets, shelling a building until it is
//! a ruin.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, ObjectCause, ObjectEvent, ObjectStage,
    PlayerCommand, PlayerId, Remains, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2,
    VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
/// M4A3(75)W, 31.6 t.
const SHERMAN: &str = "us_m4a3_75";
/// M5A1 light tank, 15.2 t.
const STUART: &str = "us_m5a1";
/// GMC CCKW truck, 7.4 t.
const TRUCK: &str = "us_gmc_cckw";

/// 800 m × 800 m flat map around the origin with the given objects (CSV
/// rows without header).
fn map_with(objects: &str) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "tank_overrun_test".into(),
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
        combat: exact_rules(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

fn command(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn move_to(player: PlayerId, unit: u32, x: i32, y: i32) -> PlayerCommand {
    command(
        player,
        Command::MoveTo {
            units: vec![UnitId(unit)],
            target: Vec2::from_meters(x, y),
        },
    )
}

fn position(sim: &Sim, unit: u32) -> Vec2 {
    sim.unit_position(UnitId(unit)).unwrap()
}

/// Steps until unit `unit` stands at `(x, y)` (within 2 m); returns the
/// object events on the way and the distance it drove.
fn drive(sim: &mut Sim, unit: u32, x: i32, y: i32) -> (Vec<ObjectEvent>, i32) {
    let goal = Vec2::from_meters(x, y);
    let mut events = Vec::new();
    let mut driven = sim::Fixed::ZERO;
    let mut last = position(sim, unit);
    for _ in 0..6000 {
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

/// A wall of 3 m segments along y = 0 from x = `from` × 3 m to `to` × 3 m.
fn wall(kind: &str, from: i32, to: i32) -> String {
    (from..=to)
        .map(|k| format!("{kind},Segment,{},0,90", k * 3000))
        .collect::<Vec<_>>()
        .join("\n")
}

const HOLD: FireMode = FireMode::HoldFire;
const GERMANS: &str = "de_inf_rifle_squad";
const AMERICANS: &str = "us_inf_rifle_squad";

/// Living soldiers of squad `unit`.
fn living(sim: &Sim, unit: u32) -> usize {
    sim.snapshot()
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(unit) && s.hp > 0)
        .count()
}

fn prone(player: PlayerId, unit: u32) -> PlayerCommand {
    command(
        player,
        Command::SetStance {
            units: vec![UnitId(unit)],
            stance: sim::StanceOrder::Fixed(sim::Stance::Prone),
        },
    )
}

/// The Sherman (unit 0, holding its fire) drives 120 m north along x = 0,
/// over the squad (unit 1) at the origin, which lay down first if `lie`.
fn drive_over(squad_owner: PlayerId, squad: &str, lie: bool) -> (Sim, usize) {
    let mut sim = sim_with(
        map_with(""),
        &[(P0, SHERMAN, 0, 60, HOLD), (squad_owner, squad, 0, 0, HOLD)],
    );
    let before = living(&sim, 1);
    if lie {
        sim.step(&[prone(squad_owner, 1)]);
        for _ in 0..100 {
            sim.step(&[]);
        }
    }
    sim.step(&[move_to(P0, 0, 0, -60)]);
    drive(&mut sim, 0, 0, -60);
    (sim, before)
}

#[test]
fn a_sherman_breaks_through_a_mud_wall_and_a_truck_drives_around() {
    // A mud wall 300 m long; both vehicles are sent from 40 m south of it
    // to 40 m north of it.
    let map = map_with(&wall("env_wall_mud", -50, 50));
    let mut sim = sim_with(map, &[(P0, SHERMAN, 0, 40, HOLD), (P0, TRUCK, 9, 40, HOLD)]);
    sim.step(&[move_to(P0, 0, 0, -40)]);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(
        driven < 100,
        "the tank drove {driven} m: not straight through"
    );
    assert!(!events.is_empty(), "no wall segment was flattened");
    for e in &events {
        assert_eq!(e.stage, ObjectStage::Destroyed);
        assert_eq!(e.remains, Remains::Gone);
        assert_eq!(e.cause, ObjectCause::Crushed(UnitId(0)));
    }
    // The gap: the segment at x = 0 is gone, those 30 m away stand.
    assert_eq!(sim.object_stage(50), Some(ObjectStage::Destroyed));
    assert_eq!(sim.object_stage(40), Some(ObjectStage::Intact));
    assert_eq!(sim.object_stage(60), Some(ObjectStage::Intact));
    assert!(events.len() <= 4, "a narrow gap: {} segments", events.len());

    // The truck is too light. With the gap of the tank closed to it (a
    // fresh wall), it drives around the end of the wall.
    let map = map_with(&wall("env_wall_mud", -50, 50));
    let mut sim = sim_with(map, &[(P0, TRUCK, 0, 40, HOLD)]);
    sim.step(&[move_to(P0, 0, 0, -40)]);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(events.is_empty(), "the truck broke the wall: {events:?}");
    assert!(driven > 300, "the truck drove only {driven} m");
}

#[test]
fn a_tank_slows_down_while_it_breaks_through() {
    let run = |objects: &str| {
        let mut sim = sim_with(map_with(objects), &[(P0, SHERMAN, 0, 40, HOLD)]);
        sim.step(&[move_to(P0, 0, 0, -40)]);
        let start = sim.tick();
        drive(&mut sim, 0, 0, -40);
        sim.tick() - start
    };
    let free = run("");
    let through = run(&wall("env_wall_mud", -50, 50));
    assert!(
        through >= free + 10,
        "through the wall in {through} ticks, without a wall in {free}"
    );
    assert!(through <= free + 80, "{through} ticks against {free}");
}

#[test]
fn a_tank_takes_a_near_gate_instead_of_the_wall() {
    // The wall has a gate 12 m wide 15 m to the side: the way through it
    // is shorter than the detour a wall costs.
    let objects = format!(
        "{}\n{}",
        wall("env_wall_mud", -50, 2),
        wall("env_wall_mud", 8, 50)
    );
    let mut sim = sim_with(map_with(&objects), &[(P0, SHERMAN, 0, 40, HOLD)]);
    sim.step(&[move_to(P0, 0, 0, -40)]);
    let (events, _) = drive(&mut sim, 0, 0, -40);
    assert!(events.is_empty(), "the tank broke the wall: {events:?}");
}

#[test]
fn a_light_tank_does_not_break_a_dry_stone_wall() {
    let map = map_with(&wall("env_wall_stone", -50, 50));
    let mut sim = sim_with(
        map,
        &[(P0, STUART, 0, 40, HOLD), (P0, SHERMAN, 60, 40, HOLD)],
    );
    sim.step(&[move_to(P0, 0, 0, -40)]);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(
        events.is_empty(),
        "the light tank broke the wall: {events:?}"
    );
    assert!(driven > 300, "the light tank drove only {driven} m");
    // The medium tank goes through.
    sim.step(&[move_to(P0, 1, 60, -40)]);
    let (events, driven) = drive(&mut sim, 1, 60, -40);
    assert!(!events.is_empty() && driven < 100, "{driven} m, {events:?}");
    // A light tank does break a mud wall.
    let map = map_with(&wall("env_wall_mud", -50, 50));
    let mut sim = sim_with(map, &[(P0, STUART, 0, 40, HOLD)]);
    sim.step(&[move_to(P0, 0, 0, -40)]);
    let (events, driven) = drive(&mut sim, 0, 0, -40);
    assert!(!events.is_empty() && driven < 100, "{driven} m, {events:?}");
}

#[test]
fn enemy_soldiers_in_the_lane_of_a_tank_die() {
    // Men lying prone do not get out of the way.
    let (sim, before) = drive_over(P1, GERMANS, true);
    let after = living(&sim, 1);
    assert!(after < before, "{after} of {before} soldiers left");
    // Only those under the hull (2.6 m wide), not the whole squad.
    assert!(after > 0, "the whole squad died");
}

#[test]
fn a_wiped_out_squad_is_a_kill_of_the_tank() {
    // A sniper team of two lying at x = -1.5 m and 1.5 m: the tank drives
    // north over the first man and back south over the second.
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, SHERMAN, -2, 60, HOLD),
            (P1, "de_inf_sniper", 0, 0, HOLD),
        ],
    );
    sim.step(&[prone(P1, 1)]);
    for _ in 0..100 {
        sim.step(&[]);
    }
    let mut killers = Vec::new();
    for (x, y) in [(-2, -60), (2, -60), (2, 60)] {
        sim.step(&[move_to(P0, 0, x, y)]);
        for _ in 0..6000 {
            sim.step(&[]);
            for e in sim.events() {
                if let sim::CombatEvent::Destroyed { unit, by, .. } = *e {
                    killers.push((unit, by));
                }
            }
            if position(&sim, 0) == Vec2::from_meters(x, y) {
                break;
            }
        }
    }
    assert_eq!(killers, [(UnitId(1), UnitId(0))]);
    assert_eq!(sim.unit_experience(UnitId(0)).unwrap().kills, 1);
}

#[test]
fn standing_soldiers_step_out_of_the_way_of_a_slow_tank() {
    // The squad stands and sees the tank coming: nobody is caught.
    let (sim, before) = drive_over(P1, GERMANS, false);
    assert_eq!(living(&sim, 1), before);
}

#[test]
fn own_soldiers_are_never_run_over() {
    for lie in [false, true] {
        let (sim, before) = drive_over(P0, AMERICANS, lie);
        assert_eq!(living(&sim, 1), before, "lying: {lie}");
    }
}

#[test]
fn a_standing_tank_that_turns_kills_nobody() {
    // The squad lies right beside the tank; the tank only shoots at a
    // target behind it (its hull and turret turn). 40 m: well inside the
    // 87.5 m gun range (prompt 097), so the tank does not creep forward.
    let mut sim = sim_with(
        map_with(""),
        &[(P0, SHERMAN, 0, 0, HOLD), (P1, GERMANS, 2, 0, HOLD)],
    );
    let before = living(&sim, 1);
    sim.step(&[prone(P1, 1)]);
    sim.step(&[command(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 40),
        },
    )]);
    for _ in 0..300 {
        sim.step(&[]);
    }
    assert_eq!(living(&sim, 1), before);
}

#[test]
fn soldiers_in_a_trench_are_not_run_over() {
    // Foxholes under the men of a squad lying in the lane.
    let mut sim = sim_with(
        map_with(""),
        &[(P0, SHERMAN, 0, 60, HOLD), (P1, GERMANS, 0, 0, HOLD)],
    );
    let before = living(&sim, 1);
    sim.step(&[prone(P1, 1)]);
    for _ in 0..100 {
        sim.step(&[]);
    }
    let men: Vec<Vec2> = sim
        .snapshot()
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(1))
        .map(|s| s.position)
        .collect();
    for &p in &men {
        sim.place_finished_work(sim::WorkKind::Foxhole, P1, p, p);
    }
    sim.step(&[move_to(P0, 0, 0, -60)]);
    let mut crossed = false;
    for _ in 0..1200 {
        sim.step(&[]);
        let tank = position(&sim, 0);
        crossed |= men
            .iter()
            .any(|&p| (p - tank).length() < sim::Fixed::from_num(1));
    }
    assert!(crossed, "the tank never drove over a foxhole");
    assert_eq!(living(&sim, 1), before);
}

const AT_WILL: FireMode = FireMode::FireAtWill;
const PANZER: &str = "de_pz4_h";

/// Rounds per weapon index fired by `unit` over `ticks`, and the hit
/// points the enemy squad `squad` lost meanwhile.
fn shots(sim: &mut Sim, unit: u32, squad: u32, ticks: u32) -> (Vec<u32>, u32) {
    let before = sim.unit_hp(UnitId(squad)).unwrap();
    let mut fired = vec![0; 8];
    for _ in 0..ticks {
        sim.step(&[]);
        for e in sim.events() {
            if let sim::CombatEvent::Shot {
                unit: u, weapon, ..
            } = *e
                && u == UnitId(unit)
            {
                fired[usize::from(weapon)] += 1;
            }
        }
    }
    (fired, before - sim.unit_hp(UnitId(squad)).unwrap())
}

#[test]
fn the_sherman_has_its_machine_guns() {
    // 75 mm gun, the roof M2HB and the coaxial and bow M1919A4 (one
    // catalog entry with two guns).
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types = unit_types_from_catalog(&catalog, &scale, &[SHERMAN]).unwrap();
    let rules = CombatRules::builtin().unwrap();
    let combat = &types[0].combat;
    let weapons = combat.armed_with(&rules);
    assert_eq!(weapons.len(), 3, "{weapons:?}");
    assert_eq!(combat.mg_mount(&rules, 0), None);
    assert_eq!(combat.mg_mount(&rules, 1), Some(sim::MgMount::Roof));
    assert_eq!(combat.mg_mount(&rules, 2), Some(sim::MgMount::Mixed));
    // 400 m real, like the same gun as an infantry weapon: 50 m in the
    // game (1/8, prompt 097).
    assert_eq!(weapons[2].range_m, sim::Fixed::from_num(50));
    for (name, mount) in [
        ("Browning M1919A4 (coaxial)", sim::MgMount::Coax),
        ("Browning M1919A4 (bow)", sim::MgMount::Bow),
        ("MG 34 (ball mount)", sim::MgMount::Bow),
        ("Browning M2HB (AA)", sim::MgMount::Roof),
        ("MG 34", sim::MgMount::Mixed),
    ] {
        assert_eq!(sim::MgMount::of_name(name), mount, "{name}");
    }
}

#[test]
fn machine_guns_fire_at_infantry_while_the_gun_duels_a_tank() {
    // The Sherman faces north. A Panzer IV stands 75 m north (the 75 mm gun
    // reaches 87.5 m, prompt 097), a squad 30 m north and 4 m to the side
    // (machine guns 50 m): inside the turret cone and the bow cone.
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, SHERMAN, 0, 0, AT_WILL),
            (P1, PANZER, 0, -75, HOLD),
            (P1, GERMANS, 4, -30, HOLD),
        ],
    );
    let (fired, lost) = shots(&mut sim, 0, 2, 200);
    assert!(fired[0] > 0, "the gun did not fire at the tank: {fired:?}");
    assert!(fired[2] > 0, "coaxial and bow gun silent: {fired:?}");
    assert!(lost > 0, "the squad lost nothing");
    // The turret stays on the tank (straight north): it never turned for
    // the machine guns.
    let aim = sim.snapshot().units[0].aim;
    assert!(aim.y < sim::Fixed::from_num(-0.999), "{aim:?}");
}

#[test]
fn only_the_roof_gun_reaches_infantry_behind_the_tank() {
    // The squad stands 30 m south, behind the tank that duels north.
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, SHERMAN, 0, 0, AT_WILL),
            (P1, PANZER, 0, -75, HOLD),
            (P1, GERMANS, 0, 30, HOLD),
        ],
    );
    let (fired, _) = shots(&mut sim, 0, 2, 200);
    assert!(fired[0] > 0, "{fired:?}");
    assert!(fired[1] > 0, "the roof gun did not fire: {fired:?}");
    assert_eq!(fired[2], 0, "coaxial or bow gun fired backwards: {fired:?}");
}

#[test]
fn hold_fire_silences_the_machine_guns_and_an_attack_order_does_not() {
    let spawns = |mode| {
        [
            (P0, SHERMAN, 0, 0, mode),
            (P1, PANZER, 0, -75, HOLD),
            (P1, GERMANS, 4, -30, HOLD),
        ]
    };
    let attack = command(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    );
    // Holding fire, with the order to attack the tank: only the gun.
    let mut sim = sim_with(map_with(""), &spawns(HOLD));
    sim.step(std::slice::from_ref(&attack));
    let (fired, lost) = shots(&mut sim, 0, 2, 200);
    assert!(fired[0] > 0, "{fired:?}");
    assert_eq!((fired[1], fired[2], lost), (0, 0, 0), "{fired:?}");
    // Firing at will, with the same order: the machine guns keep shooting
    // at the soldiers.
    let mut sim = sim_with(map_with(""), &spawns(AT_WILL));
    sim.step(&[attack]);
    let (fired, lost) = shots(&mut sim, 0, 2, 200);
    assert!(fired[0] > 0 && fired[2] > 0 && lost > 0, "{fired:?}");
}

#[test]
fn without_a_tank_around_gun_and_machine_guns_fight_the_infantry() {
    let mut sim = sim_with(
        map_with(""),
        &[(P0, SHERMAN, 0, 0, AT_WILL), (P1, GERMANS, 4, -30, HOLD)],
    );
    let (fired, lost) = shots(&mut sim, 0, 1, 200);
    assert!(fired[0] > 0 && fired[2] > 0 && lost > 0, "{fired:?}");
    // The turret turned for the gun only: it points at the squad.
    let tank = sim.snapshot().units[0];
    assert!(tank.aim.y < sim::Fixed::from_num(-0.9), "{:?}", tank.aim);
}

fn attack_object(player: PlayerId, unit: u32, object: u32) -> PlayerCommand {
    command(
        player,
        Command::AttackObject {
            units: vec![UnitId(unit)],
            object,
        },
    )
}

#[test]
fn shelling_a_building_ends_when_it_is_a_ruin() {
    // A mud house 100 m north of the Sherman (out of range: it drives up),
    // a second one 40 m beside it.
    let map = map_with("env_house_mud,Intact,0,-100000,0\nenv_house_mud,Intact,40000,-100000,0");
    let mut sim = sim_with(map, &[(P0, SHERMAN, 0, 0, HOLD)]);
    assert!(sim.shellable(0));
    sim.step(&[attack_object(P0, 0, 0)]);
    let mut shots_fired = 0;
    let mut stages = Vec::new();
    for _ in 0..4000 {
        sim.step(&[]);
        shots_fired += sim
            .events()
            .iter()
            .filter(|e| matches!(e, sim::CombatEvent::Shot { .. }))
            .count();
        stages.extend(sim.object_events().iter().map(|e| (e.object, e.stage)));
        if sim.object_stage(0) == Some(ObjectStage::Destroyed) {
            break;
        }
    }
    assert_eq!(
        stages,
        [(0, ObjectStage::Damaged), (0, ObjectStage::Destroyed)]
    );
    assert_eq!(sim.object_remains(0), Some(Remains::Ruin));
    // It drove into range: 700 m real are 87.5 m in the game (1/8, prompt
    // 097).
    let distance = (position(&sim, 0) - Vec2::from_meters(0, -100)).length();
    assert!(distance < sim::Fixed::from_num(87.5), "{distance}");
    // 1800 hit points, 562 per 7.5 cm round: four hits. Only the gun fired.
    assert!((4..=6).contains(&shots_fired), "{shots_fired} rounds");
    // The order is done: no more rounds, the neighbor stands.
    let after: usize = (0..600)
        .map(|_| {
            sim.step(&[]);
            sim.events()
                .iter()
                .filter(|e| matches!(e, sim::CombatEvent::Shot { .. }))
                .count()
        })
        .sum();
    // Rounds already in the air may still land.
    assert_eq!(after, 0, "the tank kept firing at the ruin");
    assert_eq!(sim.object_stage(1), Some(ObjectStage::Intact));
    assert!(!sim.shellable(0));
    // A ruin is no target: the order is ignored.
    sim.step(&[attack_object(P0, 0, 0)]);
    sim.step(&[]);
    assert_eq!(sim.snapshot().units[0].target, None);
}

#[test]
fn only_guns_that_damage_objects_shell_a_building() {
    // A rifle squad ignores the order; the machine guns of a tank that
    // shells a house keep to the infantry.
    // House 50 m north (in range of the 87.5 m gun), the enemy squad 32 m
    // away (machine guns 50 m, prompt 097).
    let map = map_with("env_house_mud,Intact,0,-50000,0");
    let mut sim = sim_with(
        map,
        &[
            (P0, SHERMAN, 0, 0, AT_WILL),
            (P0, AMERICANS, 15, 0, HOLD),
            (P1, GERMANS, -25, -20, HOLD),
        ],
    );
    sim.step(&[attack_object(P0, 0, 0), attack_object(P0, 1, 0)]);
    let squad = position(&sim, 1);
    let (fired, lost) = shots(&mut sim, 0, 2, 300);
    assert_eq!(position(&sim, 1), squad, "the rifle squad walked off");
    assert!(fired[0] > 0, "{fired:?}");
    assert!(sim.object_hp(0).unwrap().0 < 1800);
    // The roof gun covers all round.
    assert!(fired[1] > 0 && lost > 0, "{fired:?}, squad lost {lost}");
}
