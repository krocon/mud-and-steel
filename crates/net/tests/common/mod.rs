//! A lockstep match of the demo scenario between peers, over loopback or
//! any other transport.

#![allow(dead_code)]

use net::{LockstepConfig, LockstepSession, LoopbackTransport, ReplayRecorder, Transport};
use sim::scenario::{CommandScript, demo_config};
use sim::{Command, PlayerCommand, PlayerId, Sim, UnitId, Vec2};

pub const SEED: u64 = 42;
pub const INPUT_DELAY: u32 = 3;

pub struct Peer<T = LoopbackTransport> {
    pub sim: Sim,
    pub session: LockstepSession,
    pub transport: T,
    script: CommandScript,
    /// Also send move commands for the other player's units (must be ignored).
    pub forge: bool,
    /// State hash after every simulated tick.
    pub hashes: Vec<u64>,
    pub recorder: ReplayRecorder,
    /// Commands applied locally only, to provoke a desync: (tick, command).
    pub cheat: Option<(u64, PlayerCommand)>,
}

impl<T: Transport> Peer<T> {
    pub fn new(player: u8, transport: T) -> Self {
        let players = vec![PlayerId(0), PlayerId(1)];
        let config = LockstepConfig {
            players,
            input_delay: INPUT_DELAY,
            hash_interval: 10,
        };
        Self::with_config(player, transport, &config, demo_config())
    }

    pub fn with_config(
        player: u8,
        transport: T,
        config: &LockstepConfig,
        sim_config: sim::SimConfig,
    ) -> Self {
        let players = config.players.clone();
        Self {
            sim: Sim::new(&sim_config, sim::scenario::builtin_map(), SEED).unwrap(),
            session: LockstepSession::new(PlayerId(player), config),
            transport,
            // The same scripts as `run_demo`, but each peer only runs its own.
            script: CommandScript::new(SEED, PlayerId(player), &sim_config),
            forge: false,
            hashes: Vec::new(),
            recorder: ReplayRecorder::new(SEED, sim_config, &players),
            cheat: None,
        }
    }

    /// One round of the per-tick protocol. Returns whether a tick was simulated.
    pub fn poll(&mut self) -> bool {
        // A loop, because a raised input delay asks for several frames.
        while self.session.needs_local_input() {
            // Asking for the scheduled tick makes the match equal to `run_demo`.
            let tick = self.session.next_local_tick();
            let mut commands = self.script.commands_for_tick(tick);
            if self.forge && tick.is_multiple_of(25) {
                let other = 1 - self.session.local_player().0;
                let first = u32::from(other) * 8;
                commands.push(Command::MoveTo {
                    units: (first..first + 8).map(UnitId).collect(),
                    target: Vec2::from_meters(500, 500),
                });
            }
            self.session
                .submit_local(commands, &mut self.transport)
                .unwrap();
        }
        self.session.receive(&mut self.transport).unwrap();
        let Some(mut commands) = self.session.next_commands() else {
            return false;
        };
        if let Some((tick, cheat)) = &self.cheat
            && *tick == self.sim.tick()
        {
            commands.push(cheat.clone());
        }
        self.recorder.record(&commands);
        self.sim.step(&commands);
        let hash = self.sim.state_hash();
        self.hashes.push(hash);
        self.session
            .record_hash(self.sim.tick(), hash, &mut self.transport)
            .unwrap();
        true
    }
}

/// Alternates both peers until both simulated `ticks` ticks, then drains the
/// remaining hash reports.
pub fn run<T: Transport>(a: &mut Peer<T>, b: &mut Peer<T>, ticks: u64) {
    let mut rounds = 0;
    while a.sim.tick() < ticks || b.sim.tick() < ticks {
        if a.sim.tick() < ticks {
            a.poll();
        }
        if b.sim.tick() < ticks {
            b.poll();
        }
        rounds += 1;
        assert!(rounds < 10 * ticks, "lockstep stalled");
    }
    a.session.receive(&mut a.transport).unwrap();
    b.session.receive(&mut b.transport).unwrap();
}

pub fn pair() -> (Peer, Peer) {
    let (ta, tb) = LoopbackTransport::pair();
    (Peer::new(0, ta), Peer::new(1, tb))
}
