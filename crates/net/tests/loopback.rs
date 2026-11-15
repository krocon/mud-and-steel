//! Two simulations playing the demo scenario in lockstep over loopback.

mod common;

use common::{SEED, pair, run};
use net::{Desync, LockstepConfig, LockstepSession, LoopbackTransport, PlayerId, Transport};
use sim::scenario::{demo_config, run_demo};
use sim::{Command, PlayerCommand, Sim, UnitId, Vec2};

const TICKS: u64 = 600;

#[test]
fn two_peers_stay_in_sync() {
    let (mut a, mut b) = pair();
    // Both also try to command the other player's tanks.
    a.forge = true;
    b.forge = true;
    run(&mut a, &mut b, TICKS);

    assert_eq!(a.hashes.len(), usize::try_from(TICKS).unwrap());
    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert_eq!(a.session.verified_tick(), TICKS);
    assert_eq!(b.session.verified_tick(), TICKS);

    // Each peer ran only its own player's script, and the forged commands were
    // ignored, so the match equals the scripted demo with both players.
    assert_eq!(a.sim.state_hash(), run_demo(SEED, TICKS).state_hash());

    // The exchanged group commands really moved units of both players.
    let mut idle = Sim::new(&demo_config(), sim::scenario::builtin_map(), SEED).unwrap();
    for _ in 0..TICKS {
        idle.step(&[]);
    }
    let start = idle.snapshot().units;
    let end = a.sim.snapshot().units;
    for player in [PlayerId(0), PlayerId(1)] {
        let moved = start
            .iter()
            .zip(&end)
            .filter(|(s, e)| e.owner == player && s.position != e.position)
            .count();
        assert!(moved >= 2, "player {} moved only {moved} units", player.0);
    }
}

#[test]
fn changing_input_delays_stay_in_sync() {
    let (mut a, mut b) = pair();
    // Each peer changes its own delay independently while the match runs.
    let delays_a = [3, 7, 1, 12, 2];
    let delays_b = [5, 1, 9, 3, 4];
    let mut rounds = 0;
    while a.sim.tick() < TICKS || b.sim.tick() < TICKS {
        for (peer, delays) in [(&mut a, &delays_a), (&mut b, &delays_b)] {
            if peer.sim.tick() < TICKS {
                let phase = usize::try_from(peer.sim.tick() / 130).unwrap();
                peer.session.set_input_delay(delays[phase % delays.len()]);
                peer.poll();
            }
        }
        rounds += 1;
        assert!(rounds < 20 * TICKS, "lockstep stalled");
    }
    a.session.receive(&mut a.transport).unwrap();
    b.session.receive(&mut b.transport).unwrap();

    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert_eq!(a.session.verified_tick(), TICKS);
    // Every frame asks the script for its own tick, so the delays do not
    // change the match.
    assert_eq!(a.sim.state_hash(), run_demo(SEED, TICKS).state_hash());
}

#[test]
fn desync_is_detected() {
    let (mut a, mut b) = pair();
    // Peer B applies a command it never sent: its state diverges in tick 55.
    b.cheat = Some((
        55,
        PlayerCommand::new(
            PlayerId(1),
            Command::MoveTo {
                units: vec![UnitId(8)],
                target: Vec2::from_meters(99, 99),
            },
        ),
    ));
    run(&mut a, &mut b, TICKS);

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

#[test]
fn single_player_runs_through_the_same_path() {
    let config = LockstepConfig {
        players: vec![PlayerId(0)],
        input_delay: 1,
        hash_interval: 10,
    };
    let mut session = LockstepSession::new(PlayerId(0), &config);
    let mut transport = net::LocalTransport;
    let mut sim = Sim::new(&demo_config(), sim::scenario::builtin_map(), SEED).unwrap();
    let order = Command::MoveTo {
        units: vec![UnitId(0), UnitId(1)],
        target: Vec2::from_meters(0, 200),
    };
    for tick in 0..100 {
        let commands = if tick == 0 {
            vec![order.clone()]
        } else {
            vec![]
        };
        assert_eq!(session.next_local_tick(), tick + 1);
        session.submit_local(commands, &mut transport).unwrap();
        session.receive(&mut transport).unwrap();
        let merged = session.next_commands().expect("one player never waits");
        if tick == 1 {
            // The input delay of one tick moved the order to tick 1.
            assert_eq!(merged, vec![PlayerCommand::new(PlayerId(0), order.clone())]);
        } else {
            assert!(merged.is_empty());
        }
        sim.step(&merged);
        session
            .record_hash(sim.tick(), sim.state_hash(), &mut transport)
            .unwrap();
    }
    assert!(sim.snapshot().units[0].target.is_some());
}
