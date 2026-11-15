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
/// Last changed by prompt 027: victory conditions. The demo battle is
/// decided at tick 405 (player 0 loses its last tank, player 1 wins); from the
/// first defeat on, the standings and the result are part of the hash. Ticks
/// before that hash exactly as with prompt 017.
/// Before: prompt 017: combat. Units have hit points, a hull
/// direction, turrets, weapons and armor from the catalog, pick targets,
/// fire projectiles with ballistic flight and dispersion (simulation RNG),
/// penetrate armor by hit face and angle, and are destroyed (wrecks block
/// paths); combat rules (`data/combat.toml`), unit combat state and
/// projectiles are in the hash. The demo's tanks start 120 m apart and fight.
/// Before: prompt 013: `test_small` has map objects (woods,
/// hedges, a village; `objects.csv`), their shapes block path cells by size
/// class (`data/objects.toml`), and the map hash covers them.
/// Before: prompt 012 (units drive along A* paths on the 4 m path grid, one
/// path per group, formation offsets, separation, re-planning, path queue
/// with a node budget, unreachable targets snap to the nearest reachable
/// cell; routes, queue and grid version in the hash; wider fords), prompt 010 (map `test_small`, ground height, terrain- and
/// slope-dependent speed, map hash and terrain rules in the hash), prompt
/// 009 (two players with eight tanks each, group commands with grid
/// formation, `Stop`, owner/footprint in the hash).
///
/// If this test fails, the simulation is no longer deterministic across
/// platforms, or its rules changed. Only update the value for an intentional
/// rule change, and say so in the commit message.
const GOLDEN_HASH_SEED_42_TICKS_1000: u64 = 0xcb06_6ca1_b9cf_7d89;

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

#[test]
fn demo_is_a_battle() {
    let end = run_demo(42, 1000);
    let snapshot = end.snapshot();
    let destroyed = snapshot.units.iter().filter(|u| !u.alive()).count();
    assert!(destroyed >= 8, "only {destroyed} units destroyed");
    let start = sim::scenario::demo_config();
    for player in start.players() {
        let alive = snapshot
            .units
            .iter()
            .filter(|u| u.owner == player && u.alive())
            .count();
        assert_eq!(end.alive_units(player), alive);
    }
}

/// The demo battle ends by elimination (prompt 027): the result is part of
/// the golden state above.
#[test]
fn demo_battle_is_decided() {
    let end = run_demo(42, 1000);
    let outcome = end.outcome().expect("the demo battle is decided");
    assert_eq!(outcome.winner, Some(sim::PlayerId(1)));
    assert_eq!(outcome.tick, 405);
    assert_eq!(end.alive_units(sim::PlayerId(0)), 0);
    assert_eq!(
        end.standings()[0].defeat,
        Some((sim::DefeatReason::Eliminated, 405))
    );
}
