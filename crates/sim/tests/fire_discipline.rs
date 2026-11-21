//! Fire discipline (prompt 176): an ambush in a wood (Pak 40 and an MG
//! team) against a column with a scout car in front and a T-34 behind, and
//! an unarmed truck under machine-gun fire. Run with `--nocapture` to see
//! the numbers of each setup.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "discipline_test".into(),
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

/// A wood west of the road (x = -60..-6 m, y = -40..40 m): trees every 6 m.
fn wood() -> Arc<Map> {
    let mut trees = Vec::new();
    for x in (-60..=-6).step_by(6) {
        for y in (-40..=40).step_by(6) {
            if (x, y) == (-12, 2) || (x, y) == (-12, -10) {
                continue;
            }
            trees.push(format!(
                "env_tree_conifer,Spruce,{},{},0",
                x * 1000 + 1500,
                y * 1000 + 1500
            ));
        }
    }
    map_with(|_, _| 0, &trees)
}

/// Result of one ambush.
#[derive(Debug, Default)]
struct Ambush {
    /// Tick and the target of the ambushers' first shot, and its distance.
    first_shot: Option<(u64, u32, i32)>,
    scout_destroyed: Option<u64>,
    tank_destroyed: Option<u64>,
    ambushers_lost: usize,
}

/// Pak 40 (0) and MG 42 team (1) in the wood; the BA-64 scout car (2) and
/// a T-34 80 m behind it (3) drive north along the road at x = 20 m.
/// `setup` are player 0's orders at tick 0.
fn ambush(setup: Vec<Command>) -> Ambush {
    let mut sim = sim_with(
        wood(),
        &[
            (P0, "de_pak40", -10, 0, FIRE),
            (P0, "de_inf_mg42_team", -10, -8, FIRE),
            (P1, "su_ba64b", 20, -260, FIRE),
            (P1, "su_t34_76_1943", 20, -340, FIRE),
        ],
    );
    let mut orders: Vec<PlayerCommand> = setup
        .into_iter()
        .map(|c| PlayerCommand::new(P0, c))
        .collect();
    orders.push(PlayerCommand::new(
        P1,
        Command::MoveTo {
            units: vec![UnitId(2)],
            target: Vec2::from_meters(20, 300),
        },
    ));
    orders.push(PlayerCommand::new(
        P1,
        Command::MoveTo {
            units: vec![UnitId(3)],
            target: Vec2::from_meters(20, 220),
        },
    ));
    let mut out = Ambush::default();
    sim.step(&orders);
    for _ in 0..20 * 120 {
        sim.step(&[]);
        let tick = sim.tick();
        let snap = sim.snapshot();
        for e in sim.events() {
            match *e {
                CombatEvent::Shot { unit, .. } if unit.0 <= 1 && out.first_shot.is_none() => {
                    let me = snap.units[unit.0 as usize].position;
                    let target = [2usize, 3]
                        .into_iter()
                        .min_by_key(|&k| (snap.units[k].position - me).length())
                        .unwrap_or(2);
                    let d = (snap.units[target].position - me).length().to_num::<i32>();
                    out.first_shot = Some((tick, target as u32, d));
                }
                CombatEvent::Destroyed { unit, .. } => match unit.0 {
                    2 => out.scout_destroyed = out.scout_destroyed.or(Some(tick)),
                    3 => out.tank_destroyed = out.tank_destroyed.or(Some(tick)),
                    _ => {}
                },
                _ => {}
            }
        }
    }
    out.ambushers_lost = sim.snapshot().units[..2]
        .iter()
        .filter(|u| !u.alive())
        .count();
    out
}

#[test]
fn ambush_fire_at_will() {
    let a = ambush(Vec::new());
    println!("fire at will: {a:?}");
    assert!(a.first_shot.is_some(), "the ambushers never fired");
}

/// An unarmed truck (0) without orders, 40 m from an enemy rifle squad
/// (1) that sees it: how far it gets from where it stood, and whether it
/// lives, after 60 s.
fn truck_under_fire() -> (i32, bool) {
    let mut sim = sim_with(
        flat(),
        &[
            (P0, "de_opel_blitz", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 0, 40, FIRE),
        ],
    );
    for _ in 0..20 * 60 {
        sim.step(&[]);
    }
    let u = sim.snapshot().units[0];
    (u.position.length().to_num::<i32>(), u.alive())
}

/// Before prompt 176 the truck stood and was destroyed (moved 0 m). Now it
/// drives out of the fire.
#[test]
fn unarmed_truck_under_fire() {
    let (moved, alive) = truck_under_fire();
    println!("truck under fire: moved {moved} m, alive {alive}");
    assert!(moved >= 40, "the truck moved only {moved} m");
}

/// The ambush with discipline: the Pak opens fire only at vehicles within
/// half its range, the MG team returns fire only.
#[test]
fn ambush_with_discipline() {
    let a = ambush(vec![
        Command::SetFireLimits {
            units: vec![UnitId(0)],
            close_only: true,
            vehicles_only: true,
        },
        Command::SetFireMode {
            units: vec![UnitId(1)],
            mode: FireMode::ReturnFire,
        },
    ]);
    println!("discipline: {a:?}");
    let before = ambush(Vec::new());
    let (_, _, d) = a.first_shot.expect("the ambushers never fired");
    let (_, _, d0) = before.first_shot.expect("no first shot before");
    assert!(d <= d0, "opened fire at {d} m, at will at {d0} m");
}

fn quiet(
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
    setup: Vec<Command>,
    seconds: u32,
) -> Vec<u32> {
    let mut sim = sim_with(flat(), spawns);
    let orders: Vec<PlayerCommand> = setup
        .into_iter()
        .map(|c| PlayerCommand::new(P0, c))
        .collect();
    sim.step(&orders);
    let mut shooters = Vec::new();
    for _ in 0..20 * seconds {
        sim.step(&[]);
        for e in sim.events() {
            if let CombatEvent::Shot { unit, .. } = e
                && !shooters.contains(&unit.0)
            {
                shooters.push(unit.0);
            }
        }
    }
    shooters
}

/// Return fire: a squad that sees an enemy squad holding its fire stays
/// quiet; when the enemy fires at it, it answers.
#[test]
fn return_fire_waits_until_fired_at() {
    let hold = FireMode::HoldFire;
    let shooters = quiet(
        &[
            (P0, "de_inf_rifle_squad", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 0, 40, hold),
        ],
        vec![Command::SetFireMode {
            units: vec![UnitId(0)],
            mode: FireMode::ReturnFire,
        }],
        30,
    );
    assert!(
        shooters.is_empty(),
        "fired without being fired at: {shooters:?}"
    );
    let shooters = quiet(
        &[
            (P0, "de_inf_rifle_squad", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 0, 40, FIRE),
        ],
        vec![Command::SetFireMode {
            units: vec![UnitId(0)],
            mode: FireMode::ReturnFire,
        }],
        30,
    );
    assert!(
        shooters.contains(&1),
        "the enemy did not fire: {shooters:?}"
    );
    assert!(shooters.contains(&0), "no fire returned: {shooters:?}");
}

/// Vehicles only: a Pak 40 does not open fire at a rifle squad in range;
/// close range only: a tank does not fire at a T-34 beyond half its range.
#[test]
fn limits_keep_automatic_fire_back() {
    let shooters = quiet(
        &[
            (P0, "de_pak40", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 0, 40, FireMode::HoldFire),
        ],
        vec![Command::SetFireLimits {
            units: vec![UnitId(0)],
            close_only: false,
            vehicles_only: true,
        }],
        20,
    );
    assert!(!shooters.contains(&0), "fired at infantry: {shooters:?}");
    let shooters = quiet(
        &[
            (P0, "de_pak40", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 0, 40, FireMode::HoldFire),
        ],
        Vec::new(),
        20,
    );
    assert!(
        shooters.contains(&0),
        "without limits the Pak should fire: {shooters:?}"
    );
    let shooters = quiet(
        &[
            (P0, "de_pz4_h", 0, 0, FIRE),
            (P1, "su_t34_76_1943", 0, 70, FireMode::HoldFire),
        ],
        vec![Command::SetFireLimits {
            units: vec![UnitId(0)],
            close_only: true,
            vehicles_only: false,
        }],
        20,
    );
    assert!(
        !shooters.contains(&0),
        "fired beyond half range: {shooters:?}"
    );
}
