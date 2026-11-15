//! A peer that sends the frames of a second player it hosts (a computer
//! player, prompt 024): the other peer receives them like any player's.

use net::{LockstepConfig, LockstepSession, LoopbackTransport, NetError};
use sim::scenario::{CommandScript, builtin_map, demo_config, run_demo};
use sim::{Command, PlayerId, Sim, UnitId, Vec2};

const SEED: u64 = 42;
const TICKS: u64 = 500;

struct Peer {
    sim: Sim,
    session: LockstepSession,
    transport: LoopbackTransport,
    /// Scripts of the local player and the hosted ones.
    scripts: Vec<(PlayerId, CommandScript)>,
    hashes: Vec<u64>,
}

impl Peer {
    fn new(local: u8, hosted: &[u8], transport: LoopbackTransport) -> Self {
        let config = demo_config();
        // Player 2 owns no units: a spectator slot, like a human whose
        // opponent is the host's AI.
        let lockstep = LockstepConfig {
            players: vec![PlayerId(0), PlayerId(1), PlayerId(2)],
            input_delay: 2,
            hash_interval: 10,
        };
        let mut session = LockstepSession::new(PlayerId(local), &lockstep);
        let mut scripts = vec![(
            PlayerId(local),
            CommandScript::new(SEED, PlayerId(local), &config),
        )];
        for &p in hosted {
            session.host_player(PlayerId(p));
            scripts.push((PlayerId(p), CommandScript::new(SEED, PlayerId(p), &config)));
        }
        Self {
            sim: Sim::new(&config, builtin_map(), SEED).unwrap(),
            session,
            transport,
            scripts,
            hashes: Vec::new(),
        }
    }

    fn poll(&mut self) {
        while self.session.needs_local_input() {
            let tick = self.session.next_local_tick();
            let mut scripts = self.scripts.iter_mut();
            let (_, local) = scripts.next().unwrap();
            self.session
                .submit_local(local.commands_for_tick(tick), &mut self.transport)
                .unwrap();
            for (player, script) in scripts {
                let t = self
                    .session
                    .submit_hosted(*player, script.commands_for_tick(tick), &mut self.transport)
                    .unwrap();
                assert_eq!(t, tick, "hosted frames share the local frame's tick");
            }
        }
        self.session.receive(&mut self.transport).unwrap();
        if let Some(commands) = self.session.next_commands() {
            self.sim.step(&commands);
            self.hashes.push(self.sim.state_hash());
            self.session
                .record_hash(self.sim.tick(), self.sim.state_hash(), &mut self.transport)
                .unwrap();
        }
    }
}

#[test]
fn hosted_player_frames_reach_the_other_peer() {
    let (ta, tb) = LoopbackTransport::pair();
    // Peer A is player 0 and hosts player 1; peer B is player 2.
    let mut a = Peer::new(0, &[1], ta);
    let mut b = Peer::new(2, &[], tb);
    assert_eq!(a.session.hosted_players(), &[PlayerId(1)]);
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

    assert_eq!(a.hashes, b.hashes);
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    // Hashes are compared with the one remote peer only.
    assert_eq!(a.session.verified_tick(), TICKS);
    assert_eq!(b.session.verified_tick(), TICKS);
    // Both scripted players acted: the match equals the scripted demo.
    assert_eq!(a.sim.state_hash(), run_demo(SEED, TICKS).state_hash());
}

#[test]
fn remote_frames_for_a_hosted_player_are_rejected() {
    let (ta, tb) = LoopbackTransport::pair();
    let mut a = Peer::new(0, &[1], ta);
    let mut b = Peer::new(1, &[], tb);
    // B claims to be player 1, which A hosts.
    b.session
        .submit_local(
            vec![Command::MoveTo {
                units: vec![UnitId(8)],
                target: Vec2::from_meters(0, 0),
            }],
            &mut b.transport,
        )
        .unwrap();
    let err = a.session.receive(&mut a.transport).unwrap_err();
    assert!(matches!(err, NetError::UnknownPlayer(PlayerId(1))), "{err}");
}

#[test]
fn single_player_with_a_hosted_ai_needs_no_peer() {
    let mut session = LockstepSession::new(
        PlayerId(0),
        &LockstepConfig {
            players: vec![PlayerId(0), PlayerId(1)],
            input_delay: 1,
            hash_interval: 1,
        },
    );
    session.host_player(PlayerId(1));
    let mut transport = net::LocalTransport;
    let mut sim = Sim::new(&demo_config(), builtin_map(), SEED).unwrap();
    for _ in 0..50 {
        while session.needs_local_input() {
            session.submit_local(Vec::new(), &mut transport).unwrap();
            session
                .submit_hosted(PlayerId(1), vec![Command::Surrender], &mut transport)
                .unwrap();
        }
        let commands = session.next_commands().expect("all frames are local");
        sim.step(&commands);
        session
            .record_hash(sim.tick(), sim.state_hash(), &mut transport)
            .unwrap();
    }
    // The hosted player's surrender arrived: player 0 won.
    assert_eq!(sim.outcome().unwrap().winner, Some(PlayerId(0)));
    assert_eq!(session.verified_tick(), 0, "nobody to compare with");
}
