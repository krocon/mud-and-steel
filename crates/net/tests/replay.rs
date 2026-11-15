//! Replays recorded from a lockstep match reproduce it exactly.

mod common;

use common::{pair, run};
use net::Replay;
use sim::{Command, Vec2};

const TICKS: u64 = 800;

#[test]
fn recorded_match_replays_to_the_same_hash() {
    let (mut a, mut b) = pair();
    a.forge = true;
    run(&mut a, &mut b, TICKS);

    let replay = a.recorder.finish(&a.sim);
    // Both peers recorded the same match.
    assert_eq!(replay, b.recorder.finish(&b.sim));
    assert!(!replay.frames.is_empty());

    let bytes = replay.encode();
    let decoded = Replay::decode(&bytes).unwrap();
    assert_eq!(decoded, replay);

    let report = decoded.verify(sim::scenario::builtin_map()).unwrap();
    assert!(report.matches(), "{report:?}");
    assert_eq!(report.ticks, TICKS);
    assert_eq!(report.actual_hash, a.sim.state_hash());
    assert_eq!(
        decoded
            .play(sim::scenario::builtin_map())
            .unwrap()
            .snapshot(),
        a.sim.snapshot()
    );
}

#[test]
fn tampered_replay_is_reported() {
    let (mut a, mut b) = pair();
    run(&mut a, &mut b, TICKS);
    let mut replay = a.recorder.finish(&a.sim);

    let command = replay
        .frames
        .iter_mut()
        .flat_map(|f| f.commands.iter_mut())
        .find_map(|c| match &mut c.command {
            Command::MoveTo { target, .. } => Some(target),
            Command::Stop { .. } => None,
        })
        .expect("the demo issues move commands");
    *command = Vec2::from_meters(1, 2);

    let report = Replay::decode(&replay.encode())
        .unwrap()
        .verify(sim::scenario::builtin_map())
        .unwrap();
    assert!(!report.matches());
}
