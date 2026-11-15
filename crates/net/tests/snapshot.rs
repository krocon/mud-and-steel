//! Prompt 038: a simulation restored from a snapshot continues exactly like
//! the original, tick by tick, also after craters changed the map.

use net::snapshot::{self, SnapshotError};
use net::{Replay, ReplayRecorder};
use sim::scenario::{builtin_map, demo_config, run_demo_with};
use sim::{PlayerCommand, Sim};

const SEED: u64 = 42;
const TICKS: u64 = 1200;

fn demo_replay() -> Replay {
    let config = demo_config();
    let mut recorder = ReplayRecorder::new(SEED, config.clone(), &config.players());
    let sim = run_demo_with(SEED, TICKS, |_, commands| recorder.record(commands));
    recorder.finish(&sim)
}

fn commands_at(replay: &Replay, tick: u64) -> &[PlayerCommand] {
    replay
        .frames
        .iter()
        .find(|f| f.tick == tick)
        .map_or(&[], |f| f.commands.as_slice())
}

/// Plays `replay` to `at`, snapshots, restores, and plays both to the end;
/// every tick must have the same state hash.
fn check_round_trip(replay: &Replay, at: u64) {
    let map = builtin_map();
    let mut original = Sim::new(&replay.config, map.clone(), replay.seed).unwrap();
    for tick in 0..at {
        original.step(commands_at(replay, tick));
    }
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map.clone()).expect("snapshot restores");
    assert_eq!(restored.tick(), at);
    assert_eq!(restored.state_hash(), original.state_hash(), "at tick {at}");
    assert_eq!(restored.map().heights_mm(), original.map().heights_mm());
    for tick in at..replay.ticks {
        let commands = commands_at(replay, tick);
        original.step(commands);
        restored.step(commands);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "restored from tick {at} diverged at tick {}",
            tick + 1
        );
    }
    assert_eq!(restored.state_hash(), replay.end_hash);
}

#[test]
fn restored_simulation_continues_identically() {
    let replay = demo_replay();
    for at in [0, 1, 41, 300, 700, TICKS] {
        check_round_trip(&replay, at);
    }
}

#[test]
fn craters_are_part_of_the_snapshot() {
    let replay = demo_replay();
    let sim = replay.play(builtin_map()).unwrap();
    assert!(sim.crater_count() > 0, "the demo battle made no crater");
    let bytes = snapshot::encode(&sim, &builtin_map());
    let restored = snapshot::decode(&bytes, builtin_map()).unwrap();
    assert_eq!(restored.map().heights_mm(), sim.map().heights_mm());
    assert_eq!(restored.map().terrain_ids(), sim.map().terrain_ids());
    assert_ne!(restored.map().heights_mm(), builtin_map().heights_mm());
}

#[test]
fn damaged_snapshots_are_rejected() {
    let sim = run_demo_with(SEED, 200, |_, _| {});
    let bytes = snapshot::encode(&sim, &builtin_map());
    assert!(matches!(
        snapshot::decode(b"MSSN", builtin_map()),
        Err(SnapshotError::BadMagic)
    ));
    let mut version = bytes.clone();
    version[4] = 99;
    assert!(matches!(
        snapshot::decode(&version, builtin_map()),
        Err(SnapshotError::UnsupportedVersion(99))
    ));
    let mut hash = bytes.clone();
    hash[6] ^= 1;
    assert!(matches!(
        snapshot::decode(&hash, builtin_map()),
        Err(SnapshotError::HashMismatch { .. })
    ));
    let truncated = &bytes[..bytes.len() / 2];
    assert!(snapshot::decode(truncated, builtin_map()).is_err());
}

#[test]
fn rules_hash_is_stable_within_a_build() {
    assert_eq!(snapshot::rules_hash(), snapshot::rules_hash());
    assert_ne!(snapshot::rules_hash(), 0);
}
