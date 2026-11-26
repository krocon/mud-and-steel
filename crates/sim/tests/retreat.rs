//! Infantry retreat order (prompt 234): `Command::Retreat` and `[retreat]`
//! in `data/tactics.toml`.

use sim::CombatEvent;
use sim::snapshot::SimSnapshotOwned;
use sim::{
    Command, FireMode, Fixed, InfantryRules, PlayerCommand, PlayerId, RejectReason, Sim, Spawn,
    Stance, UnitId, Vec2,
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
fn pinned_squad_retreats_to_hq_without_firing_and_arrives_cleanly() {
    // Player 0 has an HQ at (-160, 0) and a rifle squad forward at (0, 0);
    // Player 1 has an HMG team at (55, 0) that pins the squad.
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "base_hq".into(),
                position: Vec2::from_meters(-160, 0),
                fire_mode: FireMode::FireAtWill,
            },
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: Vec2::from_meters(0, 0),
                fire_mode: FireMode::FireAtWill,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_hmg_team".into(),
                position: Vec2::from_meters(95, 0),
                fire_mode: FireMode::FireAtWill,
            },
        ],
        7,
    );
    let squad_id = UnitId(1);
    let hq_pos = Vec2::from_meters(-160, 0);

    // Hold fire on squad so the HMG pins it reliably.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::SetFireMode {
            units: vec![squad_id],
            mode: FireMode::HoldFire,
        },
    )]);
    let mut pinned = false;
    for _ in 0..20 * 20 {
        sim.step(&[]);
        if sim.squad_tactics(squad_id).is_some_and(|t| t.pinned) {
            pinned = true;
            break;
        }
    }
    assert!(pinned, "expected HMG fire to pin the rifle squad");

    // Re-enable fire at will, then order Retreat: squad must stand up,
    // sprint to HQ without firing a single shot, and end retreat on arrival.
    sim.step(&[
        PlayerCommand::new(
            P0,
            Command::SetFireMode {
                units: vec![squad_id],
                mode: FireMode::FireAtWill,
            },
        ),
        PlayerCommand::new(
            P0,
            Command::Retreat {
                units: vec![squad_id],
            },
        ),
    ]);
    assert!(
        sim.squad_tactics(squad_id).unwrap().retreating,
        "squad must immediately enter retreating state"
    );

    let mut fired_while_retreating = 0u32;
    let mut arrived_tick = None;
    let mut saw_running_upright = false;
    for tick in 0..20 * 60 {
        sim.step(&[]);
        let t = sim.squad_tactics(squad_id).unwrap();
        let u = find_unit(&sim, squad_id);
        if t.retreating {
            assert_eq!(u.stance, Some(Stance::Stand), "retreating squad stands up");
            saw_running_upright = true;
            for ev in sim.events() {
                if matches!(ev, CombatEvent::Shot { unit, .. } if *unit == squad_id) {
                    fired_while_retreating += 1;
                }
            }
        } else {
            arrived_tick = Some(tick);
            break;
        }
    }
    assert!(
        saw_running_upright,
        "squad never ran upright during retreat"
    );
    assert_eq!(
        fired_while_retreating, 0,
        "retreating squad must not fire back"
    );
    assert!(
        arrived_tick.is_some(),
        "retreating squad did not reach HQ within 60 s: end={:?}, tactics={:?}, route={:?}",
        find_unit(&sim, squad_id),
        sim.squad_tactics(squad_id),
        sim.route_of(squad_id),
    );
    let end = find_unit(&sim, squad_id);
    assert!(end.alive(), "retreating squad should survive to reach HQ");
    // `base_hq` has ~7 m footprint half-extent plus `arrive_mm = 15 m` from the
    // nearest reachable point beside the building.
    assert!(
        (end.position - hq_pos).length() <= Fixed::from_num(25),
        "expected squad near HQ (-160, 0), got {:?}",
        end.position
    );
    assert!(!sim.squad_tactics(squad_id).unwrap().retreating);
}

#[test]
fn retreat_leaves_garrison_and_cannot_be_abused_as_forward_sprint() {
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
        .expect("builtin map has a building");
    let hq_pos = Vec2::from_meters(-180, -180);
    let mut sim = make_sim(
        vec![
            Spawn {
                owner: P0,
                unit_type: "base_hq".into(),
                position: hq_pos,
                fire_mode: FireMode::FireAtWill,
            },
            Spawn {
                owner: P0,
                unit_type: "de_inf_rifle_squad".into(),
                position: building.1 + Vec2::from_meters(10, 0),
                fire_mode: FireMode::FireAtWill,
            },
            Spawn {
                owner: P1,
                unit_type: "su_inf_rifle_squad".into(),
                position: Vec2::from_meters(180, 180),
                fire_mode: FireMode::HoldFire,
            },
        ],
        11,
    );
    let squad_id = UnitId(1);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Garrison {
            units: vec![squad_id],
            building: building.0,
        },
    )]);
    for _ in 0..20 * 15 {
        sim.step(&[]);
        if find_unit(&sim, squad_id).garrison == Some(building.0) {
            break;
        }
    }
    assert_eq!(find_unit(&sim, squad_id).garrison, Some(building.0));

    // Issue Retreat: must immediately leave the building and head toward HQ.
    let start_dist = (find_unit(&sim, squad_id).position - hq_pos).length();
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Retreat {
            units: vec![squad_id],
        },
    )]);
    assert_eq!(
        find_unit(&sim, squad_id).garrison,
        None,
        "retreat must immediately leave garrison"
    );
    assert!(sim.squad_tactics(squad_id).unwrap().retreating);

    // Immediately try to abuse Retreat by issuing a forward MoveTo toward the
    // enemy: lock must reject the forward move so the squad keeps retreating
    // toward HQ.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![squad_id],
            target: Vec2::from_meters(180, 180),
        },
    )]);
    for _ in 0..20 * 2 {
        sim.step(&[]);
    }
    assert!(
        sim.squad_tactics(squad_id).unwrap().retreating,
        "move order during lock must not cancel retreat"
    );
    let mid_dist = (find_unit(&sim, squad_id).position - hq_pos).length();
    assert!(
        mid_dist < start_dist - Fixed::from_num(4),
        "squad must move toward HQ, not forward (start {start_dist}, mid {mid_dist})"
    );
}

#[test]
fn retreat_rejects_vehicles_and_is_deterministic_across_snapshots() {
    let build = || {
        make_sim(
            vec![
                Spawn {
                    owner: P0,
                    unit_type: "de_pz4_h".into(),
                    position: Vec2::from_meters(-150, 0),
                    fire_mode: FireMode::FireAtWill,
                },
                Spawn {
                    owner: P0,
                    unit_type: "de_inf_rifle_squad".into(),
                    position: Vec2::from_meters(20, 0),
                    fire_mode: FireMode::FireAtWill,
                },
                Spawn {
                    owner: P1,
                    unit_type: "su_inf_rifle_squad".into(),
                    position: Vec2::from_meters(70, 0),
                    fire_mode: FireMode::FireAtWill,
                },
            ],
            42,
        )
    };
    let mut a = build();
    let mut b = build();
    let tank = UnitId(0);
    let squad = UnitId(1);

    // Vehicle-only retreat is rejected with NoSquads.
    a.step(&[PlayerCommand::new(
        P0,
        Command::Retreat { units: vec![tank] },
    )]);
    assert!(a.order_events().iter().any(|e| matches!(
        e,
        sim::OrderEvent::Rejected {
            player: P0,
            reason: RejectReason::NoSquads,
            ..
        }
    )));
    b.step(&[PlayerCommand::new(
        P0,
        Command::Retreat { units: vec![tank] },
    )]);

    // Order squad retreat; save & restore mid-retreat and verify exact state hash match.
    let cmd = [PlayerCommand::new(
        P0,
        Command::Retreat { units: vec![squad] },
    )];
    a.step(&cmd);
    b.step(&cmd);
    for _ in 0..20 {
        a.step(&[]);
        b.step(&[]);
    }
    assert!(a.squad_tactics(squad).unwrap().retreating);
    let map = sim::scenario::builtin_map();
    let owned = SimSnapshotOwned {
        map: a.save_state(&map).map,
        sim: a.clone(),
    };
    let mut restored = Sim::restore(owned, map).unwrap();
    for _ in 0..80 {
        a.step(&[]);
        b.step(&[]);
        restored.step(&[]);
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.state_hash(), restored.state_hash());
}
