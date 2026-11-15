//! Victory conditions (prompt 027): surrender, elimination, draw.

use sim::scenario::{
    DEMO_SCENARIO, builtin_map, demo_config, parse_scenario_with_factions, scenario_factions,
};
use sim::{
    Catalog, Command, DefeatReason, PlayerCommand, PlayerId, Scale, Sim, SimConfig, TerrainRules,
    UnitId,
};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn demo() -> Sim {
    Sim::new(&demo_config(), builtin_map(), 42).unwrap()
}

fn ids(range: std::ops::Range<u32>) -> Vec<UnitId> {
    range.map(UnitId).collect()
}

fn surrender(player: PlayerId) -> PlayerCommand {
    PlayerCommand::new(player, Command::Surrender)
}

fn remove(player: PlayerId, units: Vec<UnitId>) -> PlayerCommand {
    PlayerCommand::new(player, Command::RemoveUnits { units })
}

/// Steps `ticks` ticks without commands.
fn idle(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

#[test]
fn undecided_at_start() {
    let mut sim = demo();
    idle(&mut sim, 50);
    assert_eq!(sim.outcome(), None);
    assert_eq!(sim.standings().len(), 2);
    assert!(sim.standings().iter().all(|s| s.defeat.is_none()));
    assert_eq!(sim.living_units(P0), 8);
    assert_eq!(sim.living_units(P1), 8);
}

#[test]
fn surrender_decides_the_match() {
    let mut sim = demo();
    idle(&mut sim, 10);
    sim.step(&[surrender(P1)]);
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, Some(P0));
    assert_eq!(outcome.tick, 11);
    assert_eq!(
        sim.standings()[1].defeat,
        Some((DefeatReason::Surrendered, 11))
    );
    assert_eq!(sim.standings()[0].defeat, None);
    // Units of a surrendered player stay on the field.
    assert_eq!(sim.living_units(P1), 8);
}

#[test]
fn removing_all_own_units_eliminates() {
    let mut sim = demo();
    sim.step(&[remove(P0, ids(0..4))]);
    assert_eq!(sim.living_units(P0), 4);
    assert_eq!(sim.outcome(), None);
    sim.step(&[remove(P0, ids(4..8))]);
    assert_eq!(sim.living_units(P0), 0);
    assert_eq!(
        sim.standings()[0].defeat,
        Some((DefeatReason::Eliminated, 2))
    );
    assert_eq!(sim.outcome().map(|o| o.winner), Some(Some(P1)));
    // The remaining units still move and the ids of the snapshot are intact.
    assert_eq!(
        sim.snapshot()
            .units
            .iter()
            .map(|u| u.id)
            .collect::<Vec<_>>(),
        ids(8..16)
    );
}

#[test]
fn units_of_other_players_cannot_be_removed() {
    let mut sim = demo();
    sim.step(&[remove(P0, ids(8..16)), remove(P1, ids(0..8))]);
    assert_eq!(sim.living_units(P0), 8);
    assert_eq!(sim.living_units(P1), 8);
    assert_eq!(sim.outcome(), None);
}

#[test]
fn both_eliminated_in_one_tick_is_a_draw() {
    let mut sim = demo();
    sim.step(&[remove(P0, ids(0..8)), remove(P1, ids(8..16))]);
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, None);
    assert_eq!(outcome.tick, 1);
}

#[test]
fn outcome_is_final() {
    let mut sim = demo();
    sim.step(&[surrender(P0)]);
    let outcome = sim.outcome();
    sim.step(&[surrender(P1), remove(P1, ids(8..16))]);
    idle(&mut sim, 5);
    assert_eq!(sim.outcome(), outcome);
    assert_eq!(sim.standings()[1].defeat, None);
}

#[test]
fn surrender_twice_and_by_unknown_players_is_ignored() {
    let mut sim = demo();
    sim.step(&[surrender(PlayerId(7))]);
    assert_eq!(sim.outcome(), None);
    let before = sim.state_hash();
    let mut other = sim.clone();
    other.step(&[]);
    sim.step(&[surrender(PlayerId(7))]);
    assert_eq!(sim.state_hash(), other.state_hash());
    assert_ne!(before, sim.state_hash());
}

#[test]
fn single_player_matches_are_never_decided() {
    let mut config: SimConfig = demo_config();
    config.spawns.retain(|s| s.owner == P0);
    let mut sim = Sim::new(&config, builtin_map(), 1).unwrap();
    sim.step(&[surrender(P0)]);
    sim.step(&[remove(P0, ids(0..8))]);
    assert_eq!(sim.outcome(), None);
}

#[test]
fn standings_enter_the_hash_once_someone_lost() {
    let mut a = demo();
    let mut b = demo();
    a.step(&[surrender(P1)]);
    b.step(&[]);
    assert_ne!(a.state_hash(), b.state_hash());
    // Same commands, same result.
    let mut c = demo();
    c.step(&[surrender(P1)]);
    assert_eq!(a.state_hash(), c.state_hash());
}

#[test]
fn factions_replace_the_units_of_a_player() {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let terrain = TerrainRules::builtin().unwrap();
    assert_eq!(
        scenario_factions(DEMO_SCENARIO).unwrap(),
        ["de", "su", "us"]
    );
    let config = parse_scenario_with_factions(
        DEMO_SCENARIO,
        &[(P0, "us"), (P1, "de")],
        &catalog,
        &scale,
        &terrain,
        &builtin_map(),
    )
    .unwrap();
    let demo = demo_config();
    assert_eq!(config.spawns.len(), demo.spawns.len());
    for (spawn, original) in config.spawns.iter().zip(&demo.spawns) {
        assert_eq!(spawn.position, original.position);
        let expected = if spawn.owner == P0 {
            "us_m4a3_75"
        } else {
            "de_pz4_h"
        };
        assert_eq!(spawn.unit_type, expected);
    }
    Sim::new(&config, builtin_map(), 1).unwrap();

    // No override: exactly the demo.
    let same = parse_scenario_with_factions(
        DEMO_SCENARIO,
        &[],
        &catalog,
        &scale,
        &terrain,
        &builtin_map(),
    )
    .unwrap();
    assert_eq!(same, demo);
    let unknown = parse_scenario_with_factions(
        DEMO_SCENARIO,
        &[(P0, "xx")],
        &catalog,
        &scale,
        &terrain,
        &builtin_map(),
    );
    assert!(unknown.is_err());
}
