//! Computer players (prompt 024): fog of war, results against the passive
//! baseline, reproducibility, ordinary commands only.

use ai::game::{Game, GameResult};
use ai::{Ai, Difficulty, PlayerView};
use sim::scenario::{builtin_map, demo_config};
use sim::{Command, FireMode, PlayerId, Sim, StableHasher, UnitId, Vec2};

const MAX_TICKS: u64 = 6000;

/// Plays `games` matches of `a` against `b` on the demo scenario, sides
/// alternating; returns the results and how often `a` won.
fn series(a: Difficulty, b: Difficulty, games: u64, first_seed: u64) -> (Vec<GameResult>, u64) {
    let config = demo_config();
    let mut results = Vec::new();
    let mut wins = 0;
    for seed in first_seed..first_seed + games {
        let a_player = PlayerId(u8::try_from(seed % 2).unwrap());
        let b_player = PlayerId(1 - a_player.0);
        let mut game = Game::new(
            &config,
            builtin_map(),
            seed,
            &[(a_player, a), (b_player, b)],
        )
        .unwrap();
        let result = game.run(MAX_TICKS);
        if result.winner == Some(a_player) {
            wins += 1;
        }
        results.push(result);
    }
    (results, wins)
}

#[test]
fn normal_and_hard_beat_passive_from_both_sides() {
    for d in [Difficulty::Normal, Difficulty::Hard] {
        let (results, wins) = series(d, Difficulty::Passive, 6, 100);
        assert_eq!(wins, 6, "{d} against passive: {results:#?}");
        for r in &results {
            let passive = r.commands.iter().find(|&&(p, _)| r.winner != Some(p));
            assert_eq!(passive.map(|&(_, n)| n), Some(0), "passive gave commands");
            assert!(r.decided_tick.unwrap() < 3000, "{r:?}");
        }
    }
}

#[test]
fn easy_beats_passive() {
    let (results, wins) = series(Difficulty::Easy, Difficulty::Passive, 4, 200);
    assert_eq!(wins, 4, "{results:#?}");
}

#[test]
fn tournaments_are_reproducible() {
    let hash = |results: &[GameResult]| {
        let mut h = StableHasher::default();
        for r in results {
            r.hash_into(&mut h);
        }
        h.finish()
    };
    let (first, _) = series(Difficulty::Hard, Difficulty::Normal, 2, 7);
    let (second, _) = series(Difficulty::Hard, Difficulty::Normal, 2, 7);
    assert_eq!(first, second);
    assert_eq!(hash(&first), hash(&second));
    let (other, _) = series(Difficulty::Hard, Difficulty::Normal, 2, 9);
    assert_ne!(hash(&first), hash(&other), "seeds matter");
}

#[test]
fn ai_commands_only_own_living_units() {
    let config = demo_config();
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Easy),
    ];
    let mut game = Game::new(&config, builtin_map(), 3, &players).unwrap();
    let mut issued = 0;
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        let units = game.sim().snapshot().units;
        for (ai, commands) in game.ais().iter().zip(&decisions) {
            for command in commands {
                issued += 1;
                assert!(!matches!(command, Command::Surrender));
                for id in command.units() {
                    let unit = units.iter().find(|u| u.id == *id).unwrap();
                    assert_eq!(unit.owner, ai.player(), "{command:?}");
                    assert!(unit.alive(), "{command:?}");
                }
            }
        }
        game.step(decisions);
    }
    assert!(issued > 5, "only {issued} commands");
}

#[test]
fn replay_of_an_ai_match_plays_without_the_ai() {
    let config = demo_config();
    let players = [
        (PlayerId(0), Difficulty::Normal),
        (PlayerId(1), Difficulty::Hard),
    ];
    let mut game = Game::new(&config, builtin_map(), 11, &players).unwrap();
    let result = game.run(MAX_TICKS);
    assert!(result.winner.is_some());
    let replay = net::Replay::decode(&game.replay().encode()).unwrap();
    let report = replay.verify(builtin_map()).unwrap();
    assert_eq!(report.actual_hash, result.end_hash);
    assert_eq!(report.expected_hash, result.end_hash);
}

/// Positions where a player-1 unit is hidden from player 0 at the start.
fn hidden_spots() -> Vec<Vec2> {
    let mut spots = Vec::new();
    for y in (-230..=-80).step_by(30) {
        for x in (-230..=230).step_by(46) {
            let p = Vec2::from_meters(x, y);
            let mut config = demo_config();
            let last = config.spawns.len() - 1;
            config.spawns[last].position = p;
            config.spawns[last].fire_mode = FireMode::HoldFire;
            let sim = Sim::new(&config, builtin_map(), 1).unwrap();
            if !sim.visible_to(PlayerId(0), UnitId(u32::try_from(last).unwrap())) {
                spots.push(p);
            }
        }
    }
    spots
}

#[test]
fn ai_sees_only_what_its_player_sees() {
    let spots = hidden_spots();
    assert!(spots.len() >= 2, "no hidden spots on test_small: {spots:?}");
    let views: Vec<(PlayerView, Vec<Command>)> = spots[..2]
        .iter()
        .map(|&p| {
            let mut config = demo_config();
            let last = config.spawns.len() - 1;
            config.spawns[last].position = p;
            config.spawns[last].fire_mode = FireMode::HoldFire;
            let map = builtin_map();
            let sim = Sim::new(&config, map.clone(), 5).unwrap();
            let view = PlayerView::new(&sim, PlayerId(0));
            let mut ai = Ai::new(PlayerId(0), Difficulty::Hard, &config, &map, 5);
            let commands = ai.think(&view);
            (view, commands)
        })
        .collect();
    let hidden = UnitId(u32::try_from(demo_config().spawns.len() - 1).unwrap());
    assert!(views[0].0.enemies.iter().all(|e| e.id != hidden));
    assert_eq!(views[0].0, views[1].0, "the view leaks hidden units");
    assert_eq!(views[0].1, views[1].1, "decisions depend on hidden units");
    assert!(!views[0].1.is_empty(), "the AI acts at the start");
}

#[test]
fn difficulties_parse() {
    for d in Difficulty::ALL {
        assert_eq!(d.name().parse::<Difficulty>(), Ok(d));
    }
    assert_eq!(" Hard ".parse::<Difficulty>(), Ok(Difficulty::Hard));
    assert!("brutal".parse::<Difficulty>().is_err());
}
