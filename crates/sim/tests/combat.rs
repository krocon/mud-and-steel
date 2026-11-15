//! Combat tests (prompt 017): duels on hand-made maps with catalog units.

use sim::map::MapLayout;
use sim::path::SizeClass;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, Face, FireMode, HitKind, Map, PlayerCommand,
    PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// 800 m × 800 m around the origin, 2 m samples; `height(x, y)` in mm from
/// meters.
fn map_with(height: impl Fn(i32, i32) -> i32) -> Arc<Map> {
    let n = 401;
    let heights = (0..n)
        .flat_map(|j| (0..n).map(move |i| (i, j)))
        .map(|(i, j): (i32, i32)| height(i * 2 - 400, j * 2 - 400))
        .collect();
    let layout = MapLayout {
        name: "combat_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, vec![0; (n * n) as usize]).unwrap())
}

/// Rules without randomness: no dispersion, no penetration spread.
fn exact_rules() -> CombatRules {
    let mut rules = CombatRules::builtin().unwrap();
    rules.dispersion.mrad_x10 = 0;
    rules.penetration.spread_pct = 0;
    rules
}

/// Units start facing the map center (the origin); a unit at the origin
/// faces north (-y).
fn sim_with(
    map: Arc<Map>,
    rules: CombatRules,
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
) -> Sim {
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
        combat: rules,
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
    };
    Sim::new(&config, map, 7).unwrap()
}

const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;

/// Runs until `id` is destroyed (or `max` ticks), collecting all events.
fn run_until_destroyed(sim: &mut Sim, id: UnitId, max: u64) -> Vec<(u64, CombatEvent)> {
    let mut events = Vec::new();
    while sim.tick() < max && sim.unit_hp(id) != Some(0) {
        let tick = sim.tick();
        sim.step(&[]);
        events.extend(sim.events().iter().map(|&e| (tick, e)));
    }
    events
}

#[test]
fn panzer_destroys_t34_after_the_expected_tick_count() {
    // Panzer IV 150 m east of a T-34 that faces north and holds fire: the
    // Panzer IV faces the origin, so its gun is on target at once, and the
    // rounds strike the T-34's flank.
    let mut sim = sim_with(
        map_with(|_, _| 0),
        exact_rules(),
        &[
            (P0, "de_pz4_h", 150, 0, FIRE),
            (P1, "su_t34_76_1943", 0, 0, HOLD),
        ],
    );
    let events = run_until_destroyed(&mut sim, UnitId(1), 2000);

    // Expected from catalog and rules: KwK 40 at 10 rpm real, time factor
    // 2 → 3 s → 60 ticks between rounds; 750 m/s × 2 = 75 m per tick; the
    // muzzle sits half a hull length (2.96 m) ahead, the T-34's side 1.5 m
    // off its center: 145.54 m of flight → the round hits during the second
    // tick after the shot. A penetration costs 75² × 10 / 100 = 562 of 1000
    // hit points, so the second hit destroys it.
    let rules = exact_rules();
    let reload = 60;
    let flight_ticks = 2;
    let hits_needed = rules.damage.vehicle_hp.div_ceil(562);
    assert_eq!(hits_needed, 2);
    let expected = u64::from(hits_needed - 1) * reload + flight_ticks - 1;

    let shots: Vec<u64> = events
        .iter()
        .filter(|(_, e)| matches!(e, CombatEvent::Shot { .. }))
        .map(|&(t, _)| t)
        .collect();
    assert_eq!(shots, vec![0, reload]);
    let hits: Vec<(u64, Face, bool, u32)> = events
        .iter()
        .filter_map(|&(t, e)| match e {
            CombatEvent::Hit {
                kind: HitKind::Unit(UnitId(1)),
                face,
                penetrated,
                damage,
                ..
            } => Some((t, face.unwrap(), penetrated, damage)),
            _ => None,
        })
        .collect();
    assert_eq!(
        hits,
        vec![
            (1, Face::Side, true, 562),
            (expected, Face::Side, true, 562)
        ]
    );
    let destroyed: Vec<(u64, CombatEvent)> = events
        .iter()
        .filter(|(_, e)| matches!(e, CombatEvent::Destroyed { .. }))
        .copied()
        .collect();
    assert_eq!(destroyed.len(), 1);
    assert_eq!(destroyed[0].0, expected);
    assert!(matches!(
        destroyed[0].1,
        CombatEvent::Destroyed {
            unit: UnitId(1),
            by: UnitId(0),
            ..
        }
    ));
    assert_eq!(sim.tick(), expected + 1);
    assert_eq!(sim.alive_units(P1), 0);
    assert_eq!(sim.alive_units(P0), 1);
    // The wreck blocks the way for tanks.
    assert!(!sim.path_grid().passable_at(SizeClass::Large, Vec2::ZERO));
    // And the Panzer IV stops firing.
    for _ in 0..200 {
        sim.step(&[]);
        assert!(sim.events().is_empty(), "{:?}", sim.events());
    }
}

/// Hits of a T-34 (holding still at `shooter`) on a Tiger at the origin.
fn t34_hits_on_tiger(x: i32, y: i32) -> Vec<(Face, bool)> {
    let mut sim = sim_with(
        map_with(|_, _| 0),
        exact_rules(),
        &[
            (P0, "su_t34_76_1943", x, y, FIRE),
            (P1, "de_tiger1_e", 0, 0, HOLD),
        ],
    );
    let events = run_until_destroyed(&mut sim, UnitId(1), 400);
    events
        .iter()
        .filter_map(|&(_, e)| match e {
            CombatEvent::Hit {
                kind: HitKind::Unit(_),
                face,
                penetrated,
                ..
            } => Some((face.unwrap(), penetrated)),
            _ => None,
        })
        .collect()
}

#[test]
fn front_armor_holds_where_the_flank_does_not() {
    // F-34 at 150 m (600 m real): 109 × (1 - 0.12) = 96 mm. The Tiger's
    // front is 100 mm at 9°, its side 80 mm.
    let front = t34_hits_on_tiger(0, -150);
    assert!(!front.is_empty());
    assert!(
        front.iter().all(|&h| h == (Face::Front, false)),
        "{front:?}"
    );
    let side = t34_hits_on_tiger(150, 0);
    assert!(side.iter().all(|&h| h == (Face::Side, true)), "{side:?}");
    // Two penetrations of 76.2 mm (580 each) destroy it.
    assert_eq!(side.len(), 2);
}

#[test]
fn an_oblique_hit_on_the_flank_bounces() {
    // 45° to the side plate: 80 mm / cos 45° = 113 mm > 96 mm.
    let hits = t34_hits_on_tiger(106, -106);
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|&h| h == (Face::Side, false)), "{hits:?}");
}

#[test]
fn a_ridge_stops_the_rounds() {
    // A 20 m high bank between the tanks.
    let map = map_with(|_, y| if y.abs() <= 10 { 20_000 } else { 0 });
    let mut sim = sim_with(
        map,
        exact_rules(),
        &[
            (P0, "de_pz4_h", 0, 150, FIRE),
            (P1, "su_t34_76_1943", 0, -150, HOLD),
        ],
    );
    // Since prompt 018 a unit does not fire at targets it cannot see, so
    // the hidden T-34 draws no fire by itself …
    let events = run_until_destroyed(&mut sim, UnitId(1), 100);
    assert!(events.is_empty(), "{events:?}");
    // … and a ground attack at its position sends the rounds into the bank.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::AttackGround {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, -150),
        },
    )]);
    let events = run_until_destroyed(&mut sim, UnitId(1), 300);
    assert!(
        events
            .iter()
            .any(|(_, e)| matches!(e, CombatEvent::Shot { .. }))
    );
    assert!(events.iter().all(|(_, e)| !matches!(
        e,
        CombatEvent::Hit {
            kind: HitKind::Unit(_),
            ..
        }
    )));
    assert!(events.iter().any(|(_, e)| matches!(
        e,
        CombatEvent::Hit {
            kind: HitKind::Ground,
            ..
        }
    )));
    assert_eq!(sim.unit_hp(UnitId(1)), Some(1000));
}

#[test]
fn hold_fire_keeps_everyone_quiet_until_attack() {
    let mut sim = sim_with(
        map_with(|_, _| 0),
        CombatRules::builtin().unwrap(),
        &[
            (P0, "de_pz4_h", 0, 100, HOLD),
            (P1, "su_t34_76_1943", 0, -100, HOLD),
        ],
    );
    for _ in 0..100 {
        sim.step(&[]);
        assert!(sim.events().is_empty());
    }
    // Attacking an own unit or with foreign units does nothing.
    let attack = |player, unit, target| {
        PlayerCommand::new(
            player,
            Command::Attack {
                units: vec![UnitId(unit)],
                target: UnitId(target),
            },
        )
    };
    sim.step(&[attack(P0, 0, 0), attack(P1, 0, 1)]);
    assert!(sim.events().is_empty());
    sim.step(&[attack(P0, 0, 1)]);
    // Ordered tanks fire even when holding fire.
    let events = run_until_destroyed(&mut sim, UnitId(1), 2000);
    assert_eq!(sim.unit_hp(UnitId(1)), Some(0));
    assert!(events.iter().all(|(_, e)| match e {
        CombatEvent::Shot { unit, .. } => *unit == UnitId(0),
        _ => true,
    }));
    // Fire at will again: nothing left to shoot at.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::SetFireMode {
            units: vec![UnitId(0)],
            mode: FIRE,
        },
    )]);
    // The wreck takes no orders.
    let before = sim.snapshot().units[1];
    for _ in 0..40 {
        sim.step(&[PlayerCommand::new(
            P1,
            Command::MoveTo {
                units: vec![UnitId(1)],
                target: Vec2::from_meters(50, -100),
            },
        )]);
    }
    let after = sim.snapshot().units[1];
    assert_eq!(after.position, before.position);
    assert_eq!(after.target, None);
    assert!(!after.alive());
}

#[test]
fn attack_drives_into_range_and_fires() {
    // 380 m apart, the KwK 40 reaches 375 m: the Panzer IV must close in to
    // 90 % of its range, then stop and fire.
    let mut sim = sim_with(
        map_with(|_, _| 0),
        CombatRules::builtin().unwrap(),
        &[
            (P0, "de_pz4_h", 0, 190, HOLD),
            (P1, "su_t34_76_1943", 0, -190, HOLD),
        ],
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    let mut first_shot = None;
    let mut stopped_at = None;
    while sim.tick() < 3000 && sim.unit_hp(UnitId(1)) != Some(0) {
        sim.step(&[]);
        let me = sim.snapshot().units[0];
        let distance = (me.position - sim.snapshot().units[1].position).length();
        if first_shot.is_none()
            && sim
                .events()
                .iter()
                .any(|e| matches!(e, CombatEvent::Shot { .. }))
        {
            first_shot = Some(distance);
        }
        if stopped_at.is_none() && me.target.is_none() && sim.tick() > 2 {
            stopped_at = Some(distance);
        }
    }
    assert_eq!(sim.unit_hp(UnitId(1)), Some(0), "target survived");
    let first_shot = first_shot.unwrap().to_num::<i32>();
    assert!(
        (330..=375).contains(&first_shot),
        "first shot at {first_shot} m"
    );
    let stopped = stopped_at.unwrap().to_num::<i32>();
    assert!((320..=338).contains(&stopped), "stopped at {stopped} m");
}

#[test]
fn infantry_is_killed_by_machine_guns_but_cannot_hurt_tanks() {
    let mut sim = sim_with(
        map_with(|_, _| 0),
        CombatRules::builtin().unwrap(),
        &[
            (P0, "de_inf_rifle_squad", 0, 50, FIRE),
            (P1, "us_m4a3_75", 0, -50, FIRE),
        ],
    );
    let events = run_until_destroyed(&mut sim, UnitId(0), 2000);
    assert_eq!(sim.unit_hp(UnitId(0)), Some(0));
    // The squad's rifles and MG never fire at the Sherman (they cannot
    // penetrate 51 mm), so its hit points stay full.
    assert!(events.iter().all(|(_, e)| match e {
        CombatEvent::Shot { unit, .. } => *unit == UnitId(1),
        _ => true,
    }));
    assert_eq!(sim.unit_hp(UnitId(1)), Some(1000));
    // Dead infantry leaves no wreck.
    assert!(
        sim.path_grid()
            .passable_at(SizeClass::Small, Vec2::from_meters(0, 50))
    );
}

#[test]
fn same_seed_same_battle() {
    let run = || {
        let mut sim = sim_with(
            map_with(|x, y| (x * 7 + y * 3).rem_euclid(900)),
            CombatRules::builtin().unwrap(),
            &[
                (P0, "de_pz4_h", -20, 120, FIRE),
                (P0, "de_pz4_h", 20, 120, FIRE),
                (P0, "de_inf_rifle_squad", 0, 140, FIRE),
                (P1, "su_t34_76_1943", -20, -120, FIRE),
                (P1, "su_t34_76_1943", 20, -120, FIRE),
                (P1, "su_inf_rifle_squad", 0, -140, FIRE),
            ],
        );
        let mut events = Vec::new();
        for _ in 0..600 {
            sim.step(&[]);
            events.extend_from_slice(sim.events());
        }
        (sim.state_hash(), events)
    };
    let (a, events) = run();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CombatEvent::Destroyed { .. }))
    );
    assert_eq!((a, events), run());
}
