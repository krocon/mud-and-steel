//! Infantry fragmentation and smoke hand grenades (prompt 232):
//! `Command::ThrowGrenade`, `Command::ThrowSmokeGrenade`, `[grenades]` in
//! `data/infantry.toml`.

use sim::snapshot::SimSnapshotOwned;
use sim::{
    Command, FireMode, Fixed, InfantryRules, PlayerCommand, PlayerId, Sim, Spawn, UnitId, Vec2,
};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn make_sim(spawns: Vec<Spawn>, seed: u64) -> Sim {
    let mut config = sim::scenario::skirmish_config(&[]);
    config.spawns = spawns;
    Sim::new(&config, sim::scenario::builtin_map(), seed).unwrap()
}

fn find_unit(sim: &Sim, id: UnitId) -> sim::UnitSnapshot {
    sim.snapshot()
        .units
        .into_iter()
        .find(|u| u.id == id)
        .unwrap()
}

#[test]
fn throw_in_range_and_multi_selection_throws_one_at_a_time() {
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(0, 0),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(-8, 0),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(16, 0),
                fire_mode: FireMode::HoldFire,
            },
        ],
        11,
    );
    let near_squad = UnitId(0);
    let far_squad = UnitId(1);
    let enemy = UnitId(2);
    let rules = InfantryRules::builtin().unwrap();

    // Initial stock from rules.
    let (frag0, frag_max, smoke0, smoke_max, cd0, _) = sim.grenade_state(near_squad).unwrap();
    assert_eq!(u32::from(frag0), rules.grenades.frag_stock);
    assert_eq!(u32::from(frag_max), rules.grenades.frag_stock);
    assert_eq!(u32::from(smoke0), rules.grenades.smoke_stock);
    assert_eq!(u32::from(smoke_max), rules.grenades.smoke_stock);
    assert_eq!(cd0, 0);

    // One click with both squads selected: only the nearest ready squad throws.
    let enemy_hp_before = find_unit(&sim, enemy).hp;
    sim.step(&[PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![near_squad, far_squad],
            target: Vec2::from_meters(16, 0),
        },
    )]);

    let mut saw_flying = false;
    let mut saw_throwing_flag = false;
    for _ in 0..20 * 4 {
        sim.step(&[]);
        if !sim.flying_grenades().is_empty() {
            saw_flying = true;
        }
        if sim
            .soldier_tactics(near_squad)
            .into_iter()
            .any(|f| (f & 32) != 0)
        {
            saw_throwing_flag = true;
        }
    }
    assert!(saw_flying, "expected a flying grenade snapshot");
    assert!(saw_throwing_flag, "expected soldier_flags::THROWING");

    let (frag_near, _, _, _, cd_near, _) = sim.grenade_state(near_squad).unwrap();
    let (frag_far, _, _, _, cd_far, _) = sim.grenade_state(far_squad).unwrap();
    assert_eq!(frag_near, frag_max - 1, "nearest squad spent 1 frag");
    assert_eq!(frag_far, frag_max, "second squad kept its stock");
    assert!(cd_near > 0, "throwing squad entered cooldown");
    assert_eq!(cd_far, 0, "second squad has no cooldown");

    let enemy_hp_after = find_unit(&sim, enemy).hp;
    assert!(
        enemy_hp_after < enemy_hp_before,
        "frag grenade should damage open infantry ({enemy_hp_before} -> {enemy_hp_after})"
    );
}

#[test]
fn auto_advance_out_of_range_and_resupply_via_truck() {
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(-40, 0),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P0,
                unit_type: "de_opel_blitz".into(),
                position: Vec2::from_meters(-40, 12),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(20, 0),
                fire_mode: FireMode::HoldFire,
            },
        ],
        23,
    );
    let squad = UnitId(0);
    let target = Vec2::from_meters(20, 0);
    let (_, frag_max, _, _, _, _) = sim.grenade_state(squad).unwrap();

    // Target is 60 m away (> 24 m throw range): squad must walk forward and throw.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![squad],
            target,
        },
    )]);

    let mut threw = false;
    for _ in 0..20 * 25 {
        sim.step(&[]);
        if sim.grenade_state(squad).unwrap().0 < frag_max {
            threw = true;
            break;
        }
    }
    assert!(
        threw,
        "squad should advance from 60 m into throw range and throw (pos={:?})",
        find_unit(&sim, squad).position
    );

    // Move supply truck beside the squad and wait for resupply.
    let squad_pos = find_unit(&sim, squad).position;
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![UnitId(1)],
            target: Vec2::new(squad_pos.x, squad_pos.y + Fixed::from_num(6)),
        },
    )]);
    let mut resupplied = false;
    for _ in 0..20 * 40 {
        sim.step(&[]);
        if sim.grenade_state(squad).unwrap().0 == frag_max {
            resupplied = true;
            break;
        }
    }
    assert!(resupplied, "supply truck should replenish spent grenades");
}

#[test]
fn smoke_grenade_blinds_bunker_and_two_frags_destroy_bunker() {
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(0, 0),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(0, 8),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_bunker_dzot".into(),
                position: Vec2::from_meters(20, 0),
                fire_mode: FireMode::FireAtWill,
            },
        ],
        37,
    );
    let s0 = UnitId(0);
    let s1 = UnitId(1);
    let bunker = UnitId(2);

    // Throw smoke between squad and bunker.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::ThrowSmokeGrenade {
            units: vec![s0],
            target: Vec2::from_meters(10, 0),
        },
    )]);
    for _ in 0..20 * 3 {
        sim.step(&[]);
    }
    assert!(
        sim.smoke_at(Vec2::from_meters(10, 0)) && !sim.snapshot().smoke.is_empty(),
        "smoke grenade must create a smoke cloud"
    );
    assert!(
        sim.snapshot_for(P1).units.iter().all(|u| u.id != s0),
        "smoke cloud between bunker and squad must block enemy vision"
    );

    // Now throw two frag grenades (one from each squad, or second after cooldown) onto the bunker.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![s1],
            target: Vec2::from_meters(20, 0),
        },
    )]);
    // Wait for s0's cooldown (8 s game) so s0 can throw the second frag.
    for _ in 0..20 * 10 {
        sim.step(&[]);
    }
    let hp_after_one = find_unit(&sim, bunker).hp;
    assert!(
        hp_after_one > 0 && hp_after_one < 2500,
        "1 frag grenade should heavily damage bunker (hp={hp_after_one})"
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![s0],
            target: Vec2::from_meters(20, 0),
        },
    )]);
    for _ in 0..20 * 4 {
        sim.step(&[]);
    }
    assert!(
        !find_unit(&sim, bunker).alive(),
        "2 frag grenades should destroy a 2500-HP bunker (hp={})",
        find_unit(&sim, bunker).hp
    );
}

#[test]
fn frag_grenade_hurts_garrison_but_not_closed_tank_and_roundtrips() {
    let map = sim::scenario::builtin_map();
    let rules = InfantryRules::builtin().unwrap();
    let kinds = map.object_kinds();
    let building = (0u32..)
        .zip(map.objects())
        .find(|(_, o)| {
            kinds
                .get(usize::from(o.kind))
                .is_some_and(|k| rules.building(&k.id).is_some())
        })
        .map(|(i, o)| (i, o.position))
        .expect("builtin map has a garrisonable building");
    let throw_from = Vec2::new(building.1.x - Fixed::from_num(14), building.1.y);
    let tank_pos = Vec2::new(
        building.1.x - Fixed::from_num(14),
        building.1.y + Fixed::from_num(16),
    );

    let mut a = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: throw_from,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::new(building.1.x + Fixed::from_num(6), building.1.y),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_t34_76_1943".into(),
                position: tank_pos,
                fire_mode: FireMode::HoldFire,
            },
        ],
        51,
    );
    let mut b = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: throw_from,
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::new(building.1.x + Fixed::from_num(6), building.1.y),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_t34_76_1943".into(),
                position: tank_pos,
                fire_mode: FireMode::HoldFire,
            },
        ],
        51,
    );
    let thrower = UnitId(0);
    let defender = UnitId(1);
    let tank = UnitId(2);

    // Defender occupies the building.
    let enter = [PlayerCommand::new(
        P1,
        Command::Garrison {
            units: vec![defender],
            building: building.0,
        },
    )];
    a.step(&enter);
    b.step(&enter);
    for _ in 0..20 * 6 {
        a.step(&[]);
        b.step(&[]);
    }
    assert_eq!(find_unit(&a, defender).garrison, Some(building.0));

    // Throw frag into building.
    let def_hp_before = find_unit(&a, defender).hp;
    let cmd_house = [PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![thrower],
            target: building.1,
        },
    )];
    a.step(&cmd_house);
    b.step(&cmd_house);

    // Mid-flight snapshot roundtrip on `a`.
    for _ in 0..10 {
        a.step(&[]);
        b.step(&[]);
    }
    let saved = SimSnapshotOwned {
        map: a.save_state(&map).map,
        sim: a.clone(),
    };
    let mut restored = Sim::restore(saved, map.clone()).unwrap();
    for _ in 0..20 * 10 {
        a.step(&[]);
        b.step(&[]);
        restored.step(&[]);
    }
    assert_eq!(
        a.state_hash(),
        b.state_hash(),
        "grenades must be deterministic"
    );
    assert_eq!(
        a.state_hash(),
        restored.state_hash(),
        "snapshot restore must preserve flying grenade state"
    );
    assert!(
        find_unit(&a, defender).hp < def_hp_before,
        "frag grenade must damage garrisoned infantry"
    );

    // Throw frag at closed T-34-85: must deal 0 hull damage.
    let tank_hp_before = find_unit(&a, tank).hp;
    a.step(&[PlayerCommand::new(
        P0,
        Command::ThrowGrenade {
            units: vec![thrower],
            target: tank_pos,
        },
    )]);
    for _ in 0..20 * 4 {
        a.step(&[]);
    }
    assert_eq!(
        find_unit(&a, tank).hp,
        tank_hp_before,
        "frag grenade must not damage closed tank armor"
    );
}
