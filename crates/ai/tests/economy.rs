//! The computer player in a base-building match (prompt 023).

use ai::Difficulty;
use ai::game::Game;
use sim::PlayerId;
use sim::scenario::{builtin_map, skirmish_config};

fn play(a: Difficulty, b: Difficulty, seed: u64, ticks: u64) -> Game {
    let config = skirmish_config(&[]);
    let mut game = Game::new(
        &config,
        builtin_map(),
        seed,
        &[(PlayerId(0), a), (PlayerId(1), b)],
    )
    .unwrap();
    game.run(ticks);
    game
}

#[test]
fn every_difficulty_builds_and_produces() {
    for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
        // 3.5 minutes against a passive opponent, which only fires at will.
        let game = play(d, Difficulty::Passive, 5, 4200);
        let sim = game.sim();
        let built: Vec<String> = sim
            .structures()
            .into_iter()
            .filter(|s| s.owner == PlayerId(0))
            .map(|s| sim_name(sim, s.unit_type))
            .collect();
        for name in ["base_hq", "base_barracks", "base_depot"] {
            assert!(built.iter().any(|b| b == name), "{d}: {built:?}");
        }
        let spawned = sim.snapshot().units.len() - skirmish_config(&[]).spawns.len();
        let produced = spawned - (built.len() - 1);
        assert!(produced >= 2, "{d}: {produced} units produced, {built:?}");
        // The passive player builds nothing (its headquarters may be gone).
        let passive = sim
            .structures()
            .iter()
            .filter(|s| s.owner == PlayerId(1))
            .count();
        assert!(passive <= 1, "{d}: passive player has {passive} buildings");
    }
}

fn sim_name(sim: &sim::Sim, unit_type: u32) -> String {
    ["base_hq", "base_barracks", "base_depot", "base_factory"]
        .into_iter()
        .find(|n| sim.unit_type_index(n) == Some(unit_type))
        .unwrap_or("other")
        .to_owned()
}

#[test]
fn base_matches_are_reproducible_and_both_sides_build() {
    let config = skirmish_config(&[]);
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Normal),
    ];
    let mut a = Game::new(&config, builtin_map(), 11, &players).unwrap();
    let mut b = Game::new(&config, builtin_map(), 11, &players).unwrap();
    // Most completed buildings each player had at once.
    let mut most = [0usize; 2];
    for _ in 0..12_000 {
        let d = a.think();
        a.step(d);
        let d = b.think();
        b.step(d);
        for (player, m) in most.iter_mut().enumerate() {
            let built = a
                .sim()
                .structures()
                .iter()
                .filter(|s| usize::from(s.owner.0) == player && s.completed())
                .count();
            *m = (*m).max(built);
        }
    }
    assert_eq!(a.result(), b.result());
    for (player, &m) in most.iter().enumerate() {
        assert!(m >= 4, "player {player} had at most {m} buildings");
    }
    let spawned = a.sim().snapshot().units.len() - config.spawns.len();
    assert!(spawned >= 20, "{spawned} units and buildings added");
}
