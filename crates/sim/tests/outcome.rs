//! Victory conditions (prompt 027): surrender, elimination, draw.

use sim::scenario::{
    DEMO_SCENARIO, builtin_map, demo_config, parse_scenario_with_factions, scenario_factions,
};
use sim::{
    Catalog, Command, DefeatReason, FireMode, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    TerrainRules, UnitId,
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
    // Eight tanks and two rifle squads each (prompt 022).
    assert_eq!(sim.alive_units(P0), 10);
    assert_eq!(sim.alive_units(P1), 10);
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
    assert_eq!(sim.alive_units(P1), 10);
}

/// Player 1 holds fire; player 0 attacks one enemy after the other (tanks,
/// then the rifle squads) until the last one is destroyed: player 1 is
/// eliminated by real combat.
#[test]
fn destroying_all_enemy_units_eliminates() {
    let mut config = demo_config();
    for spawn in &mut config.spawns {
        if spawn.owner == P1 {
            spawn.fire_mode = FireMode::HoldFire;
        }
    }
    let mut sim = Sim::new(&config, builtin_map(), 42).unwrap();
    // Tanks 0..8 and squads 16, 17 of player 0; tanks 8..16 and squads 18,
    // 19 of player 1 (infantry groups follow the tanks in the scenario).
    let mut own = ids(0..8);
    own.extend(ids(16..18));
    let mut ticks = 0;
    while sim.outcome().is_none() && ticks < 20_000 {
        let commands = if ticks % 100 == 0 {
            // Attack the first enemy that is still alive.
            (8..16)
                .chain(18..20)
                .map(UnitId)
                .find(|&id| sim.unit_hp(id).is_some_and(|hp| hp > 0))
                .map(|target| {
                    vec![PlayerCommand::new(
                        P0,
                        Command::Attack {
                            units: own.clone(),
                            target,
                        },
                    )]
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        sim.step(&commands);
        ticks += 1;
    }
    let outcome = sim.outcome().expect("player 0 destroys every enemy tank");
    assert_eq!(outcome.winner, Some(P0));
    assert_eq!(sim.alive_units(P1), 0);
    assert!(sim.alive_units(P0) > 0);
    assert_eq!(
        sim.standings()[1].defeat,
        Some((DefeatReason::Eliminated, outcome.tick))
    );
    // Wrecks stay in the match.
    assert_eq!(sim.snapshot().units.len(), 20);
}

#[test]
fn single_player_matches_are_never_decided() {
    let mut config: SimConfig = demo_config();
    config.spawns.retain(|s| s.owner == P0);
    let mut sim = Sim::new(&config, builtin_map(), 1).unwrap();
    sim.step(&[surrender(P0)]);
    idle(&mut sim, 5);
    assert_eq!(sim.outcome(), None);
}

#[test]
fn outcome_is_final() {
    let mut sim = demo();
    sim.step(&[surrender(P0)]);
    let outcome = sim.outcome();
    sim.step(&[surrender(P1)]);
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
        let infantry = original.unit_type.contains("_inf_");
        let expected = match (spawn.owner == P0, infantry) {
            (true, false) => "us_m4a3_75",
            (false, false) => "de_pz4_h",
            (true, true) => "us_inf_rifle_squad",
            (false, true) => "de_inf_rifle_squad",
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
