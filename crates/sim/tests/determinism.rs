//! Determinism tests. The golden hash is the cross-platform proof: CI runs it on
//! macOS (ARM), Linux and Windows, and it must pass unchanged everywhere.

use sim::scenario::run_demo;

#[test]
fn same_seed_and_commands_give_same_hash() {
    let a = run_demo(42, 1000);
    let b = run_demo(42, 1000);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.snapshot(), b.snapshot());
}

#[test]
fn different_seed_gives_different_hash() {
    assert_ne!(
        run_demo(1, 1000).state_hash(),
        run_demo(2, 1000).state_hash()
    );
}

/// Expected hash of the demo scenario with seed 42 after 1000 ticks.
///
/// Last changed by prompt 012: units drive along A* paths on the 4 m path
/// grid (one path per group, formation offsets, separation, re-planning),
/// path requests are queued with a node budget per tick, unreachable
/// targets snap to the nearest reachable cell; routes, the path queue and
/// the grid version are in the state hash. `test_small` was regenerated
/// with wider fords (terrain only).
/// Before: prompt 010 (map `test_small`, ground height, terrain- and
/// slope-dependent speed, map hash and terrain rules in the hash), prompt
/// 009 (two players with eight tanks each, group commands with grid
/// formation, `Stop`, owner/footprint in the hash).
///
/// If this test fails, the simulation is no longer deterministic across
/// platforms, or its rules changed. Only update the value for an intentional
/// rule change, and say so in the commit message.
const GOLDEN_HASH_SEED_42_TICKS_1000: u64 = 0xc569_89d7_3505_efba;

#[test]
fn golden_hash() {
    let hash = run_demo(42, 1000).state_hash();
    assert_eq!(
        hash, GOLDEN_HASH_SEED_42_TICKS_1000,
        "golden hash mismatch: got {hash:#018x}"
    );
}

#[test]
fn demo_moves_units_of_both_players() {
    let start = sim::scenario::demo_config();
    let end = run_demo(42, 1000).snapshot();
    for player in start.players() {
        let moved = start
            .spawns
            .iter()
            .zip(&end.units)
            .filter(|(spawn, unit)| unit.owner == player && spawn.position != unit.position)
            .count();
        assert!(moved > 0, "no unit of player {} moved", player.0);
    }
}
