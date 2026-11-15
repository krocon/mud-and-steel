//! A production chain (prompt 023) on two peers in lockstep: each peer
//! builds, produces and sets rally points for its own player only, from its
//! own copy of the simulation, and both stay identical tick by tick.

use net::{LockstepConfig, LockstepSession, LoopbackTransport, PlayerId, ReplayRecorder};
use sim::scenario::{builtin_map, economy_commands, skirmish_config};
use sim::{Command, FireMode, Sim};

const SEED: u64 = 23;
const TICKS: u64 = 3600;

struct Peer {
    sim: Sim,
    session: LockstepSession,
    transport: LoopbackTransport,
    hashes: Vec<u64>,
    recorder: ReplayRecorder,
    /// Most completed buildings each player had at once.
    most_built: [usize; 2],
}

impl Peer {
    fn new(player: u8, transport: LoopbackTransport, config: &sim::SimConfig) -> Self {
        let players = vec![PlayerId(0), PlayerId(1)];
        let lockstep = LockstepConfig {
            players: players.clone(),
            input_delay: 3,
            hash_interval: 10,
        };
        Self {
            sim: Sim::new(config, builtin_map(), SEED).unwrap(),
            session: LockstepSession::new(PlayerId(player), &lockstep),
            transport,
            hashes: Vec::new(),
            recorder: ReplayRecorder::new(SEED, config.clone(), &players),
            most_built: [0; 2],
        }
    }

    fn poll(&mut self) {
        while self.session.needs_local_input() {
            // Like a human or the AI: decide from what this peer sees now.
            let own = self.session.local_player();
            let commands: Vec<Command> = economy_commands(&self.sim)
                .into_iter()
                .filter(|c| c.player == own)
                .map(|c| c.command)
                .collect();
            self.session
                .submit_local(commands, &mut self.transport)
                .unwrap();
        }
        self.session.receive(&mut self.transport).unwrap();
        let Some(commands) = self.session.next_commands() else {
            return;
        };
        self.recorder.record(&commands);
        self.sim.step(&commands);
        let hash = self.sim.state_hash();
        self.hashes.push(hash);
        for (player, most) in self.most_built.iter_mut().enumerate() {
            let built = self
                .sim
                .structures()
                .iter()
                .filter(|s| usize::from(s.owner.0) == player && s.completed())
                .count();
            *most = (*most).max(built);
        }
        self.session
            .record_hash(self.sim.tick(), hash, &mut self.transport)
            .unwrap();
    }
}

#[test]
fn production_chain_is_identical_on_two_peers() {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let (ta, tb) = LoopbackTransport::pair();
    let mut a = Peer::new(0, ta, &config);
    let mut b = Peer::new(1, tb, &config);
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

    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert_eq!(a.session.verified_tick(), TICKS);
    assert_eq!(a.sim.snapshot(), b.sim.snapshot());

    // Both players built barracks, depot and vehicle works (their units
    // may have shot some of them down since) and produced units; both peers
    // agree on accounts and queues.
    assert_eq!(a.most_built, b.most_built);
    for player in [PlayerId(0), PlayerId(1)] {
        let built = a.most_built[usize::from(player.0)];
        assert!(
            built >= 4,
            "player {} had at most {built} buildings",
            player.0
        );
        assert_eq!(a.sim.account(player), b.sim.account(player));
    }
    assert_eq!(a.sim.structures(), b.sim.structures());
    let produced = a.sim.snapshot().units.len() - config.spawns.len() - 6;
    assert!(produced >= 4, "{produced} units produced");

    // The replay of the match plays back to the same end.
    let replay = a.recorder.finish(&a.sim);
    let bytes = replay.encode();
    let decoded = net::Replay::decode(&bytes).unwrap();
    let again = decoded.play(builtin_map()).unwrap();
    assert_eq!(again.state_hash(), a.sim.state_hash());
}
