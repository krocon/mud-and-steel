//! Two peers in lockstep send their groups across the river of the built-in
//! map: the units must find the ford, never enter deep water and stay in
//! sync on both peers.

use net::{LockstepConfig, LockstepSession, LoopbackTransport, PlayerId};
use sim::scenario::{builtin_map, demo_config};
use sim::{Command, FireMode, Fixed, Sim, SimConfig, UnitId, Vec2};

const SEED: u64 = 7;
/// Ground vehicles drive at real speed since prompt 097 (half of the earlier
/// doubled speed), so both legs get twice the ticks they had before
/// (1300 + 1100 then).
const TICKS: u64 = 4800;
/// The second order (back west); by then the first one must be done.
const BACK_TICK: u64 = 2600;
const DEEP_WATER: u8 = 7;
/// The river of `test_small` runs north-south around x = 160..175 m.
const EAST_OF_RIVER_M: i32 = 190;

/// The demo with every unit holding fire: this test is about movement,
/// and the demo tanks would otherwise destroy each other (prompt 017).
fn peaceful_demo() -> SimConfig {
    let mut config = demo_config();
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    config
}

struct Peer {
    sim: Sim,
    session: LockstepSession,
    transport: LoopbackTransport,
    /// Orders of this peer's player: (tick, command).
    orders: Vec<(u64, Command)>,
    hashes: Vec<u64>,
}

impl Peer {
    fn new(player: u8, transport: LoopbackTransport) -> Self {
        let config = LockstepConfig {
            players: vec![PlayerId(0), PlayerId(1)],
            input_delay: 2,
            hash_interval: 10,
        };
        let first = u32::from(player) * 8;
        let units: Vec<UnitId> = (first..first + 8).rev().map(UnitId).collect();
        let (y_east, y_back) = if player == 0 { (40, 60) } else { (-60, -60) };
        let orders = vec![
            (
                5,
                Command::MoveTo {
                    units: units.clone(),
                    target: Vec2::from_meters(225, y_east),
                },
            ),
            (
                BACK_TICK,
                Command::MoveTo {
                    units,
                    target: Vec2::from_meters(0, y_back),
                },
            ),
        ];
        Self {
            sim: Sim::new(&peaceful_demo(), builtin_map(), SEED).unwrap(),
            session: LockstepSession::new(PlayerId(player), &config),
            transport,
            orders,
            hashes: Vec::new(),
        }
    }

    fn poll(&mut self) -> bool {
        if self.session.needs_local_input() {
            let tick = self.session.next_local_tick();
            let commands = self
                .orders
                .iter()
                .filter(|(t, _)| *t == tick)
                .map(|(_, c)| c.clone())
                .collect();
            self.session
                .submit_local(commands, &mut self.transport)
                .unwrap();
        }
        self.session.receive(&mut self.transport).unwrap();
        let Some(commands) = self.session.next_commands() else {
            return false;
        };
        self.sim.step(&commands);
        let hash = self.sim.state_hash();
        self.hashes.push(hash);
        self.session
            .record_hash(self.sim.tick(), hash, &mut self.transport)
            .unwrap();
        true
    }
}

#[test]
fn groups_cross_the_river_in_sync() {
    let (ta, tb) = LoopbackTransport::pair();
    let (mut a, mut b) = (Peer::new(0, ta), Peer::new(1, tb));
    let map = builtin_map();
    let mut crossed = [false; 16];
    let mut rounds = 0;
    while a.sim.tick() < TICKS || b.sim.tick() < TICKS {
        if a.sim.tick() < TICKS && a.poll() {
            for (i, unit) in a.sim.snapshot().units.iter().take(16).enumerate() {
                assert_ne!(
                    map.terrain_at(unit.position),
                    DEEP_WATER,
                    "unit {} in deep water at tick {}",
                    unit.id.0,
                    a.sim.tick()
                );
                crossed[i] |= unit.position.x > Fixed::from_num(EAST_OF_RIVER_M);
            }
            if a.sim.tick() == BACK_TICK {
                // The first order is done: every tank stands east of the
                // river (the rifle squads of prompt 022, ids 16..20, get no
                // orders here).
                for unit in a.sim.snapshot().units.into_iter().take(16) {
                    assert_eq!(unit.target, None, "unit {} still driving", unit.id.0);
                    assert!(
                        unit.position.x > Fixed::from_num(EAST_OF_RIVER_M),
                        "unit {} at {:?} terrain {}",
                        unit.id.0,
                        unit.position,
                        map.terrain_at(unit.position)
                    );
                }
            }
        }
        if b.sim.tick() < TICKS {
            b.poll();
        }
        rounds += 1;
        assert!(rounds < 10 * TICKS, "lockstep stalled");
    }
    a.session.receive(&mut a.transport).unwrap();
    b.session.receive(&mut b.transport).unwrap();

    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert!(
        crossed.iter().all(|&c| c),
        "not every unit crossed: {crossed:?}"
    );
    // And all came back west of the river.
    for unit in a.sim.snapshot().units.into_iter().take(16) {
        assert_eq!(unit.target, None, "unit {} still driving back", unit.id.0);
        assert!(
            unit.position.x < Fixed::from_num(100),
            "{:?}",
            unit.position
        );
    }
    // One group request per order, plus members that needed a path of
    // their own (crowded ford): still fewer than one per unit and order.
    let stats = a.sim.path_stats();
    assert!(stats.requests >= 4 && stats.requests < 16 * 2, "{stats:?}");
    assert_eq!((stats.found, stats.failed), (stats.requests, 0));
}
