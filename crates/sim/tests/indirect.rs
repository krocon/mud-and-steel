//! Indirect fire and smoke (prompt 032): mortars over walls, minimum range,
//! fragments, armor, smoke that blocks sight and dissolves, determinism.

use sim::indirect::INDIRECT_WEAPON;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Spawn, Stance, StanceOrder, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const HOLD: FireMode = FireMode::HoldFire;
const MORTAR: &str = "de_inf_mortar_medium";
const SQUAD: &str = "su_inf_rifle_squad";

/// Flat 800 m × 800 m around the origin, 2 m samples, optional objects
/// (CSV rows without header).
fn map_with(objects: &str) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "indirect_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!("{}\n{objects}", sim::objects::CSV_HEADER))
            .unwrap();
    }
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
                fire_mode: HOLD,
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
    Sim::new(&config, map, seed).unwrap()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn fire_at(units: &[u32], x: i32, y: i32) -> PlayerCommand {
    cmd(
        P0,
        Command::AttackGround {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            target: Vec2::from_meters(x, y),
        },
    )
}

/// One tick with `commands`; its events.
fn order(sim: &mut Sim, commands: &[PlayerCommand]) -> Vec<CombatEvent> {
    sim.step(commands);
    sim.events().to_vec()
}

/// Runs `ticks` ticks and collects the events.
fn run(sim: &mut Sim, ticks: u32) -> Vec<CombatEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        sim.step(&[]);
        events.extend_from_slice(sim.events());
    }
    events
}

fn indirect_shots(events: &[CombatEvent]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, CombatEvent::Shot { weapon, .. } if *weapon == INDIRECT_WEAPON))
        .count()
}

/// A dry-stone wall along y = 0 from x = -150 m to 150 m.
fn wall() -> String {
    (-50..=50)
        .map(|k| format!("env_wall_stone,Segment,{},0,90", k * 3000))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn mortar_hits_a_squad_behind_a_wall() {
    // The squad crouches 8 m behind the wall, the mortar stands 200 m in
    // front of it: flat fire would strike the wall, the shells fall
    // behind it.
    let mut sim = sim_with(
        map_with(&wall()),
        &[(P0, MORTAR, 0, 200), (P1, SQUAD, 0, -8)],
        7,
    );
    let full = sim.unit_hp(UnitId(1)).unwrap();
    sim.step(&[fire_at(&[0], 0, -8)]);
    let events = run(&mut sim, 20 * 60);
    assert!(indirect_shots(&events) >= 20, "{}", indirect_shots(&events));
    let behind = events
        .iter()
        .filter(|e| {
            matches!(e, CombatEvent::Hit { shooter, at, .. }
            if *shooter == UnitId(0) && at.y < sim::Fixed::ZERO)
        })
        .count();
    assert!(behind >= 5, "only {behind} impacts behind the wall");
    let hp = sim.unit_hp(UnitId(1)).unwrap();
    assert!(hp < full / 2, "squad still has {hp} of {full}");
}

#[test]
fn the_minimum_range_holds_fire() {
    // 8 cm GrW 34: 300 m range, minimum 5 % = 15 m.
    let mut sim = sim_with(map_with(""), &[(P0, MORTAR, 0, 0), (P1, SQUAD, 0, -300)], 1);
    let mut events = order(&mut sim, &[fire_at(&[0], 0, -10)]);
    events.extend(run(&mut sim, 200));
    assert_eq!(indirect_shots(&events), 0);
    sim.step(&[fire_at(&[0], 0, -40)]);
    assert!(indirect_shots(&run(&mut sim, 200)) > 0);
    // Beyond the range the team walks closer first.
    sim.step(&[fire_at(&[0], 0, -390)]);
    run(&mut sim, 40);
    assert!(
        sim.snapshot().units[0].target.is_some(),
        "drives into range"
    );
}

/// Hit points lost by the squad (unit 1) of a mortar barrage of `salvos`
/// at its center, with the squad in `stance`.
fn barrage_on_squad(stance: Stance, seed: u64) -> u32 {
    let mut sim = sim_with(
        map_with(""),
        &[(P0, MORTAR, 0, 150), (P1, SQUAD, 0, 0)],
        seed,
    );
    let full = sim.unit_hp(UnitId(1)).unwrap();
    sim.step(&[cmd(
        P1,
        Command::SetStance {
            units: vec![UnitId(1)],
            stance: StanceOrder::Fixed(stance),
        },
    )]);
    run(&mut sim, 40);
    let mut events = order(
        &mut sim,
        &[cmd(
            P0,
            Command::Barrage {
                units: vec![UnitId(0)],
                target: Vec2::ZERO,
                radius_m: 0,
                salvos: 12,
                smoke: false,
            },
        )],
    );
    events.extend(run(&mut sim, 20 * 60));
    assert_eq!(
        indirect_shots(&events),
        12,
        "the barrage ends after 12 rounds"
    );
    full - sim.unit_hp(UnitId(1)).unwrap()
}

#[test]
fn fragments_hurt_infantry_and_stance_protects() {
    let mut standing = 0;
    let mut prone = 0;
    for seed in 1..=3 {
        standing += barrage_on_squad(Stance::Stand, seed);
        prone += barrage_on_squad(Stance::Prone, seed);
    }
    assert!(standing > 0);
    assert!(prone < standing, "prone {prone} vs standing {standing}");
}

#[test]
fn fragments_stay_within_the_radius() {
    // One squad 2 m from the target, one 80 m away (radius 16 m).
    let mut sim = sim_with(
        map_with(""),
        &[(P0, MORTAR, 0, 150), (P1, SQUAD, 2, 0), (P1, SQUAD, 80, 0)],
        3,
    );
    let full = sim.unit_hp(UnitId(2)).unwrap();
    sim.step(&[fire_at(&[0], 0, 0)]);
    run(&mut sim, 20 * 45);
    assert!(sim.unit_hp(UnitId(1)).unwrap() < full);
    assert_eq!(sim.unit_hp(UnitId(2)).unwrap(), full);
}

#[test]
fn open_vehicles_suffer_and_tanks_need_a_direct_hit() {
    // A halftrack (8 mm sides) and a Tiger 10 m to either side of the
    // target: fragments cut into the halftrack; an 8 cm shell does not get
    // through the Tiger's roof even on a direct hit.
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, MORTAR, 0, 150),
            (P1, "de_sdkfz251_1", -10, 0),
            (P1, "de_tiger1_e", 10, 0),
        ],
        5,
    );
    let full = sim.unit_hp(UnitId(1)).unwrap();
    sim.step(&[fire_at(&[0], 0, 0)]);
    run(&mut sim, 20 * 60);
    assert!(
        sim.unit_hp(UnitId(1)).unwrap() < full,
        "halftrack untouched"
    );
    assert_eq!(sim.unit_hp(UnitId(2)).unwrap(), full, "the Tiger was hurt");
}

#[test]
fn smoke_blocks_sight_and_dissolves() {
    // P0: an observer tank at (0, 150) and a mortar at (0, 200); P1: a
    // tank at (0, -100), seen across open ground.
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, "de_pz4_h", 0, 150),
            (P0, MORTAR, 0, 200),
            (P1, "su_t34_76_1943", 0, -100),
        ],
        11,
    );
    assert!(sim.visible_to(P0, UnitId(2)));
    sim.step(&[cmd(
        P0,
        Command::Barrage {
            units: vec![UnitId(1)],
            target: Vec2::from_meters(0, 20),
            radius_m: 0,
            salvos: 3,
            smoke: true,
        },
    )]);
    let mut hidden_at = None;
    let mut bursts = 0;
    for _ in 0..20 * 30 {
        sim.step(&[]);
        bursts += sim
            .events()
            .iter()
            .filter(|e| matches!(e, CombatEvent::Smoke { .. }))
            .count();
        if bursts > 0 && hidden_at.is_none() && !sim.visible_to(P0, UnitId(2)) {
            hidden_at = Some(sim.tick());
        }
    }
    assert!(hidden_at.is_some(), "the smoke never hid the tank");
    assert!(!sim.snapshot().smoke.is_empty());
    assert!(sim.smoke_at(Vec2::from_meters(0, 20)) || !sim.snapshot().smoke.is_empty());
    // Clouds last 45 s game time and drift; afterwards the view is clear.
    run(&mut sim, 20 * 60);
    assert!(sim.snapshot().smoke.is_empty());
    assert!(sim.visible_to(P0, UnitId(2)), "smoke must dissolve");
}

#[test]
fn units_without_smoke_ammunition_ignore_smoke_orders() {
    // The Soviet 82-BM-37 has no smoke rounds in the catalog.
    let mut sim = sim_with(
        map_with(""),
        &[(P0, "su_inf_mortar_medium", 0, 0), (P1, SQUAD, 0, -300)],
        2,
    );
    let mut events = order(
        &mut sim,
        &[cmd(
            P0,
            Command::Barrage {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(0, -200),
                radius_m: 20,
                salvos: 2,
                smoke: true,
            },
        )],
    );
    events.extend(run(&mut sim, 400));
    assert_eq!(indirect_shots(&events), 0);
}

#[test]
fn same_barrage_gives_same_hash() {
    let play = || {
        let mut sim = sim_with(
            map_with(&wall()),
            &[
                (P0, MORTAR, 0, 200),
                (P0, "de_wespe", -40, 380),
                (P1, SQUAD, 0, -20),
            ],
            9,
        );
        sim.step(&[
            fire_at(&[0], 0, -20),
            cmd(
                P0,
                Command::Barrage {
                    units: vec![UnitId(1)],
                    target: Vec2::from_meters(0, -30),
                    radius_m: 30,
                    salvos: 4,
                    smoke: true,
                },
            ),
        ]);
        run(&mut sim, 20 * 40);
        (sim.state_hash(), sim.snapshot())
    };
    let (a, sa) = play();
    let (b, sb) = play();
    assert_eq!(a, b);
    assert_eq!(sa, sb);
}

/// Prompt 062: tick and projectile of every indirect shot of `unit`, and
/// the ground impacts of those projectiles; `orders` go with the first tick.
fn salvo_log(
    sim: &mut Sim,
    orders: &[PlayerCommand],
    ticks: u32,
    unit: UnitId,
) -> (Vec<(u32, u32)>, Vec<sim::combat::Vec3>) {
    let mut shots = Vec::new();
    let mut impacts = Vec::new();
    for t in 0..ticks {
        sim.step(if t == 0 { orders } else { &[] });
        for e in sim.events() {
            match *e {
                CombatEvent::Shot {
                    unit: u,
                    weapon,
                    projectile,
                    ..
                } if u == unit && weapon == INDIRECT_WEAPON => shots.push((t, projectile)),
                CombatEvent::Hit {
                    projectile,
                    at,
                    kind: sim::combat::HitKind::Ground,
                    ..
                } if shots.iter().any(|&(_, p)| p == projectile) => impacts.push(at),
                _ => {}
            }
        }
    }
    (shots, impacts)
}

#[test]
fn a_rocket_salvo_fires_every_tube_then_reloads() {
    // The 15 cm Panzerwerfer 42: 10 tubes, 4 rockets per minute sustained,
    // rockets 500 ms real (5 ticks) apart.
    // A squad of P0 at (0, -100) watches the aim point: since prompt 097
    // the launcher's own sight (250 m) no longer reaches 500 m, and an
    // unseen aim point would triple the dispersion ([dispersion] blind_pct).
    let mut sim = sim_with(
        map_with(""),
        &[
            (P0, "de_pzwerfer42", 0, 300),
            (P1, SQUAD, -380, -380),
            (P0, SQUAD, 0, -100),
        ],
        11,
    );
    let (shots, impacts) = salvo_log(&mut sim, &[fire_at(&[0], 0, -200)], 20 * 120, UnitId(0));
    assert!(shots.len() >= 11, "only {} rockets", shots.len());
    // The first salvo: exactly the ten tubes, 5 ticks apart.
    let first: Vec<u32> = shots.iter().take(10).map(|s| s.0).collect();
    for pair in first.windows(2) {
        assert_eq!(pair[1] - pair[0], 5, "ripple {first:?}");
    }
    // Then the reload: 10 × 7.5 s game per rocket minus the ripple
    // (1500 - 45 ticks) before the next salvo.
    let gap = shots[10].0 - shots[9].0;
    assert_eq!(gap, 1455, "reload after the salvo");
    // Every rocket lands within the dispersion of the aim point: 8 mrad
    // of 4000 m real (500 m game) is ±32 m per axis, at most 45.3 m away.
    assert!(impacts.len() >= 10, "{} impacts", impacts.len());
    let target = Vec2::from_meters(0, -200);
    let mut far = sim::Fixed::ZERO;
    for at in &impacts {
        let d = (Vec2::new(at.x, at.y) - target).length();
        assert!(
            d <= sim::Fixed::from_num(46),
            "impact {d} m from the aim point"
        );
        far = far.max(d);
    }
    // ... and they scatter (no two salvos of one point).
    assert!(
        far > sim::Fixed::from_num(8),
        "rockets scatter only {far} m"
    );
}

#[test]
fn a_barrage_counts_whole_salvos_and_same_salvo_gives_same_hash() {
    let play = || {
        let mut sim = sim_with(
            map_with(""),
            &[(P0, "su_bm13", 0, 380), (P1, SQUAD, -380, -380)],
            5,
        );
        let barrage = cmd(
            P0,
            Command::Barrage {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(0, -150),
                radius_m: 20,
                salvos: 1,
                smoke: false,
            },
        );
        let (shots, _) = salvo_log(&mut sim, &[barrage], 20 * 60, UnitId(0));
        (shots.len(), sim.state_hash(), sim.snapshot())
    };
    let (n, a, sa) = play();
    // One salvo of the BM-13-16: all 16 rails, then the order is done.
    assert_eq!(n, 16);
    let (_, b, sb) = play();
    assert_eq!(a, b);
    assert_eq!(sa, sb);
}
