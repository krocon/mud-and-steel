//! Two simulations exchanging commands over the loopback transport.

use net::{Desync, LockstepConfig, LockstepSession, LoopbackTransport, PlayerId, Transport};
use sim::scenario::{CommandScript, demo_config};
use sim::{Command, Sim, UnitId, Vec2};

const TICKS: u64 = 600;

struct Peer {
    sim: Sim,
    session: LockstepSession,
    transport: LoopbackTransport,
    script: CommandScript,
    /// State hash after every simulated tick.
    hashes: Vec<u64>,
    /// Commands applied locally only, to provoke a desync: (tick, command).
    cheat: Option<(u64, Command)>,
}

impl Peer {
    fn new(player: u8, transport: LoopbackTransport) -> Self {
        let config = LockstepConfig {
            players: vec![PlayerId(0), PlayerId(1)],
            input_delay: 3,
            hash_interval: 10,
        };
        let unit_count = u32::try_from(demo_config().spawns.len()).unwrap();
        Self {
            sim: Sim::new(&demo_config(), 42).unwrap(),
            session: LockstepSession::new(PlayerId(player), &config),
            transport,
            // Each player issues its own, different command stream.
            script: CommandScript::new(1000 + u64::from(player), unit_count),
            hashes: Vec::new(),
            cheat: None,
        }
    }

    /// One round of the per-tick protocol. Returns whether a tick was simulated.
    fn poll(&mut self) -> bool {
        if self.session.needs_local_input() {
            let commands = self.script.commands_for_tick(self.session.next_tick());
            self.session
                .submit_local(commands, &mut self.transport)
                .unwrap();
        }
        self.session.receive(&mut self.transport).unwrap();
        let Some(mut commands) = self.session.next_commands() else {
            return false;
        };
        if let Some((tick, cheat)) = self.cheat
            && tick == self.sim.tick()
        {
            commands.push(cheat);
        }
        self.sim.step(&commands);
        let hash = self.sim.state_hash();
        self.hashes.push(hash);
        self.session
            .record_hash(self.sim.tick(), hash, &mut self.transport)
            .unwrap();
        true
    }
}

/// Alternates both peers until both simulated `TICKS` ticks, then drains the
/// remaining hash reports.
fn run(a: &mut Peer, b: &mut Peer) {
    let mut rounds = 0;
    while a.sim.tick() < TICKS || b.sim.tick() < TICKS {
        if a.sim.tick() < TICKS {
            a.poll();
        }
        if b.sim.tick() < TICKS {
            b.poll();
        }
        rounds += 1;
        assert!(rounds < 10 * TICKS, "lockstep stalled");
    }
    a.session.receive(&mut a.transport).unwrap();
    b.session.receive(&mut b.transport).unwrap();
}

#[test]
fn two_peers_stay_in_sync() {
    let (ta, tb) = LoopbackTransport::pair();
    let mut a = Peer::new(0, ta);
    let mut b = Peer::new(1, tb);
    run(&mut a, &mut b);

    assert_eq!(a.hashes.len(), usize::try_from(TICKS).unwrap());
    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);

    // The exchanged commands really changed the state.
    let mut idle = Sim::new(&demo_config(), 42).unwrap();
    for _ in 0..TICKS {
        idle.step(&[]);
    }
    assert_ne!(a.sim.state_hash(), idle.state_hash());
}

#[test]
fn desync_is_detected() {
    let (ta, tb) = LoopbackTransport::pair();
    let mut a = Peer::new(0, ta);
    let mut b = Peer::new(1, tb);
    // Peer B applies a command it never sent: its state diverges in tick 55.
    b.cheat = Some((
        55,
        Command::MoveTo {
            unit: UnitId(0),
            target: Vec2::from_meters(99, 99),
        },
    ));
    run(&mut a, &mut b);

    // Hashes are exchanged every 10 ticks, so the first mismatch is at tick 60.
    let Some(Desync {
        tick,
        player,
        local_hash,
        remote_hash,
    }) = a.session.desync()
    else {
        panic!("peer A did not detect the desync");
    };
    assert_eq!((tick, player), (60, PlayerId(1)));
    assert_ne!(local_hash, remote_hash);
    assert_eq!(
        b.session.desync().map(|d| (d.tick, d.player)),
        Some((60, PlayerId(0)))
    );
}

#[test]
fn rejects_frames_from_unknown_players() {
    let (mut ta, mut tb) = LoopbackTransport::pair();
    let config = LockstepConfig {
        players: vec![PlayerId(0), PlayerId(1)],
        input_delay: 2,
        hash_interval: 10,
    };
    let mut session = LockstepSession::new(PlayerId(0), &config);
    let forged = net::Message::Frame(net::CommandFrame {
        tick: 2,
        player: PlayerId(7),
        commands: Vec::new(),
    });
    tb.send(&net::encode(&forged)).unwrap();
    assert!(matches!(
        session.receive(&mut ta),
        Err(net::NetError::UnknownPlayer(PlayerId(7)))
    ));
}
