//! Waypoint chains and patrols (prompt 031).

use sim::scenario::{builtin_map, demo_config};
use sim::{Command, FireMode, Fixed, MAX_QUEUED, PlayerCommand, PlayerId, Sim, UnitId, Vec2};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
/// Panzer IV H of player 0 (ids 0..8 in the demo).
const TANK: UnitId = UnitId(0);

/// The demo with everyone holding fire, so nothing but the orders moves.
fn quiet() -> Sim {
    let mut config = demo_config();
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    Sim::new(&config, builtin_map(), 42).unwrap()
}

fn position(sim: &Sim, id: UnitId) -> Vec2 {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == id)
        .expect("unit exists")
        .position
}

fn queue(units: &[UnitId], target: Vec2) -> PlayerCommand {
    PlayerCommand::new(
        P0,
        Command::QueueMove {
            units: units.to_vec(),
            target,
        },
    )
}

fn distance(a: Vec2, b: Vec2) -> Fixed {
    (a - b).length()
}

/// Runs until `id` stands still (no target) or `limit` ticks passed;
/// returns the closest distance to each of `points` seen on the way.
fn drive(sim: &mut Sim, id: UnitId, points: &[Vec2], limit: u32) -> Vec<Fixed> {
    let mut closest = vec![Fixed::MAX; points.len()];
    for _ in 0..limit {
        sim.step(&[]);
        let p = position(sim, id);
        for (c, &w) in closest.iter_mut().zip(points) {
            *c = (*c).min(distance(p, w));
        }
        let unit = sim
            .snapshot()
            .units
            .into_iter()
            .find(|u| u.id == id)
            .unwrap();
        let (queued, _) = sim.queued_waypoints(id).unwrap();
        if unit.target.is_none() && queued.is_empty() {
            break;
        }
    }
    closest
}

/// Three waypoints around the tank, on the open meadow of `test_small`.
fn waypoints(start: Vec2) -> [Vec2; 3] {
    [
        start + Vec2::from_meters(30, 0),
        start + Vec2::from_meters(30, 30),
        start + Vec2::from_meters(0, 30),
    ]
}

#[test]
fn a_chain_visits_every_waypoint_in_order() {
    let mut sim = quiet();
    let start = position(&sim, TANK);
    let [a, b, c] = waypoints(start);
    // The first one starts at once (the tank is idle), the others queue.
    sim.step(&[queue(&[TANK], a), queue(&[TANK], b), queue(&[TANK], c)]);
    let (queued, patrol) = sim.queued_waypoints(TANK).unwrap();
    assert_eq!(queued, &[b, c]);
    assert!(!patrol);

    let mut order = Vec::new();
    for _ in 0..2000 {
        sim.step(&[]);
        let p = position(&sim, TANK);
        for (k, &w) in [a, b, c].iter().enumerate() {
            if distance(p, w) < Fixed::from_num(1) && order.last() != Some(&k) {
                order.push(k);
            }
        }
        if order.last() == Some(&2) {
            break;
        }
    }
    assert_eq!(order, vec![0, 1, 2], "waypoints reached in this order");
    assert!(sim.queued_waypoints(TANK).unwrap().0.is_empty());
}

#[test]
fn a_new_order_drops_the_chain() {
    let mut sim = quiet();
    let start = position(&sim, TANK);
    let [a, b, c] = waypoints(start);
    sim.step(&[queue(&[TANK], a), queue(&[TANK], b), queue(&[TANK], c)]);
    sim.step(&[PlayerCommand::new(P0, Command::Stop { units: vec![TANK] })]);
    assert!(sim.queued_waypoints(TANK).unwrap().0.is_empty());
    let stopped = position(&sim, TANK);
    for _ in 0..100 {
        sim.step(&[]);
    }
    assert_eq!(position(&sim, TANK), stopped, "a stopped tank stays");

    // A plain move replaces the chain as well.
    sim.step(&[queue(&[TANK], a), queue(&[TANK], b)]);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![TANK],
            target: c,
        },
    )]);
    assert!(sim.queued_waypoints(TANK).unwrap().0.is_empty());
    let closest = drive(&mut sim, TANK, &[b, c], 2000);
    assert!(closest[1] < Fixed::from_num(1), "drove to the new target");
}

#[test]
fn a_group_keeps_its_formation_along_the_chain() {
    let mut sim = quiet();
    let group: Vec<UnitId> = (0..4).map(UnitId).collect();
    let center = position(&sim, TANK);
    let a = center + Vec2::from_meters(0, 40);
    let b = center + Vec2::from_meters(40, 40);
    sim.step(&[queue(&group, a), queue(&group, b)]);
    for _ in 0..3000 {
        sim.step(&[]);
    }
    let spacing = sim::formation_spacing(Fixed::from_num(7));
    let slots = sim::formation_slots(b, group.len(), spacing);
    for id in &group {
        let p = position(&sim, *id);
        let nearest = slots.iter().map(|&s| distance(p, s)).min().unwrap();
        assert!(
            nearest < Fixed::from_num(2),
            "unit {id:?} at {p:?} is not on a slot around {b:?}"
        );
    }
}

#[test]
fn a_patrol_shuttles_between_start_and_target() {
    let mut sim = quiet();
    let start = position(&sim, TANK);
    let target = start + Vec2::from_meters(40, 0);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Patrol {
            units: vec![TANK],
            target,
        },
    )]);
    let (_, patrol) = sim.queued_waypoints(TANK).unwrap();
    assert!(patrol);
    // There, back, and there again.
    let mut visits = Vec::new();
    for _ in 0..3000 {
        sim.step(&[]);
        let p = position(&sim, TANK);
        let at = if distance(p, target) < Fixed::from_num(1) {
            Some(1)
        } else if distance(p, start) < Fixed::from_num(1) {
            Some(0)
        } else {
            None
        };
        if let Some(at) = at
            && visits.last() != Some(&at)
        {
            visits.push(at);
        }
        if visits.len() >= 4 {
            break;
        }
    }
    assert_eq!(visits[..], [0, 1, 0, 1][..visits.len().min(4)]);
    assert!(visits.len() >= 3, "patrol visits: {visits:?}");
    sim.step(&[PlayerCommand::new(P0, Command::Stop { units: vec![TANK] })]);
    assert_eq!(sim.queued_waypoints(TANK), Some((&[][..], false)));
}

#[test]
fn foreign_units_and_too_many_waypoints_are_ignored() {
    let mut sim = quiet();
    let enemy = UnitId(8);
    let start = position(&sim, enemy);
    sim.step(&[queue(&[enemy], Vec2::from_meters(0, 0))]);
    assert!(sim.queued_waypoints(enemy).unwrap().0.is_empty());
    for _ in 0..20 {
        sim.step(&[]);
    }
    assert_eq!(position(&sim, enemy), start);

    let p = position(&sim, TANK);
    let commands: Vec<PlayerCommand> = (0..MAX_QUEUED as i32 + 10)
        .map(|k| queue(&[TANK], p + Vec2::from_meters(20 + k % 5, 0)))
        .collect();
    sim.step(&commands);
    // The first one started the tank; the queue holds at most MAX_QUEUED.
    assert_eq!(sim.queued_waypoints(TANK).unwrap().0.len(), MAX_QUEUED);
    // Player 1 commanding through the queue does not reach player 0's units.
    sim.step(&[PlayerCommand::new(P1, Command::Stop { units: vec![TANK] })]);
    assert_eq!(sim.queued_waypoints(TANK).unwrap().0.len(), MAX_QUEUED);
}

#[test]
fn chains_are_deterministic_and_in_the_hash() {
    let run = |with_chain: bool| {
        let mut sim = quiet();
        let start = position(&sim, TANK);
        let [a, b, c] = waypoints(start);
        let mut commands = vec![queue(&[TANK], a)];
        if with_chain {
            commands.extend([queue(&[TANK], b), queue(&[TANK], c)]);
        }
        sim.step(&commands);
        let after_order = sim.state_hash();
        for _ in 0..500 {
            sim.step(&[]);
        }
        (after_order, sim.state_hash())
    };
    assert_eq!(run(true), run(true));
    let (with, _) = run(true);
    let (without, _) = run(false);
    assert_ne!(with, without, "the queue is part of the state hash");
}

/// Prompt 101: a standing unit without orders is idle; with a move order it
/// is not, until it arrives; with waypoints neither.
#[test]
fn idle_follows_the_orders() {
    let mut sim = quiet();
    assert!(sim.unit_idle(TANK), "a fresh tank stands idle");
    let start = position(&sim, TANK);
    let target = start + Vec2::from_meters(20, 0);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![TANK],
            target,
        },
    )]);
    assert!(!sim.unit_idle(TANK), "a moving tank is not idle");
    for _ in 0..2000 {
        sim.step(&[]);
        if sim.unit_idle(TANK) {
            break;
        }
    }
    assert!(sim.unit_idle(TANK), "the tank is idle again after arriving");
    assert!(distance(position(&sim, TANK), target) < Fixed::from_num(3));
    sim.step(&[queue(&[TANK], start), queue(&[TANK], target)]);
    assert!(!sim.unit_idle(TANK), "a tank with waypoints is not idle");
    assert!(!sim.unit_idle(UnitId(9999)), "unknown units are not idle");
}
