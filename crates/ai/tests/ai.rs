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

/// Rifle squads (prompt 022) are an AI group of their own: squads 320 m
/// apart see each other, and the AI's squads close in and attack. Against
/// enemy tanks they attack only infantry (see `ai_moves_and_attacks...`).
#[test]
fn ai_moves_and_attacks_with_infantry() {
    let mut config = demo_config();
    config.spawns.retain(|s| s.unit_type.contains("_inf_"));
    for s in &mut config.spawns {
        if s.owner == PlayerId(1) {
            s.position.y -= sim::Fixed::from_num(150);
        }
    }
    let squads = [UnitId(0), UnitId(1)];
    assert!(config.spawns[..2].iter().all(|s| s.owner == PlayerId(0)));
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 5, &players).unwrap();
    let start: Vec<Vec2> = config.spawns[..2].iter().map(|s| s.position).collect();
    let mut attacked = false;
    let mut moved = false;
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        attacked |= decisions.iter().flatten().any(|c| {
            matches!(c, Command::Attack { units, .. } if units.iter().any(|u| squads.contains(u)))
        });
        game.step(decisions);
        let now = game.sim().snapshot();
        moved |=
            (0..2).any(|k| (now.units[k].position - start[k]).length() > sim::Fixed::from_num(20));
    }
    assert!(attacked && moved, "attacked {attacked}, moved {moved}");
}

/// With enemy tanks around, the AI's squads attack only enemy infantry and
/// leave the tanks to its own tanks.
#[test]
fn ai_squads_attack_only_what_they_can_hurt() {
    let config = demo_config();
    let squads: Vec<UnitId> = (0u32..)
        .zip(&config.spawns)
        .filter(|(_, s)| s.unit_type.contains("_inf_") && s.owner == PlayerId(0))
        .map(|(i, _)| UnitId(i))
        .collect();
    assert_eq!(squads.len(), 2);
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 5, &players).unwrap();
    let mut targets = Vec::new();
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        for command in decisions.iter().flatten() {
            if let Command::Attack { units, target } = command
                && units.iter().any(|u| squads.contains(u))
            {
                targets.push(*target);
            }
        }
        game.step(decisions);
    }
    assert!(!targets.is_empty(), "the squads got no attack order");
    for target in &targets {
        let name = &config.spawns[target.0 as usize].unit_type;
        assert!(name.contains("_inf_"), "squads attacked {name}");
    }
}

/// Hard (prompt 032): a mortar team behind the army fires barrages on the
/// enemy and lays smoke when the enemy comes into sight.
#[test]
fn hard_uses_artillery_and_smoke() {
    let mut config = demo_config();
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    config
        .unit_types
        .extend(sim::unit_types_from_catalog(&catalog, &scale, &["de_inf_mortar_medium"]).unwrap());
    let mortar = UnitId(u32::try_from(config.spawns.len()).unwrap());
    config.spawns.push(sim::Spawn {
        owner: PlayerId(0),
        unit_type: "de_inf_mortar_medium".into(),
        position: Vec2::from_meters(0, 130),
        fire_mode: FireMode::FireAtWill,
    });
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 4, &players).unwrap();
    let (mut barrage, mut smoke, mut shells) = (0, 0, 0);
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        for command in decisions.iter().flatten() {
            if let Command::Barrage {
                units, smoke: s, ..
            } = command
                && units.contains(&mortar)
            {
                if *s {
                    smoke += 1;
                } else {
                    barrage += 1;
                }
            }
        }
        game.step(decisions);
        shells += game
            .sim()
            .events()
            .iter()
            .filter(|e| {
                matches!(e, sim::CombatEvent::Shot { unit, weapon, .. }
                    if *unit == mortar && *weapon == sim::indirect::INDIRECT_WEAPON)
            })
            .count();
    }
    assert!(
        barrage > 0 && smoke > 0,
        "barrages {barrage}, smoke {smoke}"
    );
    assert!(shells > 0, "the mortar never fired");
}

/// Prompt 034: the AI carries its infantry to the front in a truck,
/// unloads it short of the enemy, and fetches a towed gun from the rear and
/// sets it up behind the army.
#[test]
fn ai_rides_trucks_and_tows_guns() {
    let mut config = demo_config();
    config.spawns.retain(|s| s.unit_type.contains("_inf_"));
    for s in &mut config.spawns {
        if s.owner == PlayerId(1) {
            s.position.y -= sim::Fixed::from_num(150);
        }
    }
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    config.unit_types.extend(
        sim::unit_types_from_catalog(&catalog, &scale, &["de_opel_blitz", "de_pak40"]).unwrap(),
    );
    let own = config.spawns[0].position;
    let enemy = config
        .spawns
        .iter()
        .find(|s| s.owner == PlayerId(1))
        .unwrap()
        .position;
    let away = own - enemy;
    let back = away.scale(sim::Fixed::from_num(120) / away.length());
    let truck = UnitId(u32::try_from(config.spawns.len()).unwrap());
    let gun = UnitId(truck.0 + 1);
    for (unit, position) in [
        ("de_opel_blitz", own + Vec2::from_meters(15, 0)),
        ("de_pak40", own + back),
    ] {
        config.spawns.push(sim::Spawn {
            owner: PlayerId(0),
            unit_type: unit.into(),
            position,
            fire_mode: FireMode::FireAtWill,
        });
    }
    let players = [
        (PlayerId(0), Difficulty::Normal),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 6, &players).unwrap();
    let (mut rode, mut unloaded, mut towed, mut unlimbered) = (false, false, false, false);
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        for command in decisions.iter().flatten() {
            match command {
                Command::Disembark { units } if units.contains(&truck) => unloaded = true,
                Command::Unlimber { .. } => unlimbered = true,
                _ => {}
            }
        }
        game.step(decisions);
        let now = game.sim().snapshot();
        rode |= now.units.iter().any(|u| u.carrier == Some(truck));
        towed |= now.units.iter().any(|u| u.id == gun && u.carrier.is_some());
    }
    assert!(
        rode && unloaded && towed && unlimbered,
        "rode {rode}, unloaded {unloaded}, towed {towed}, unlimbered {unlimbered}"
    );
}

/// Prompt 036: a tank that fired most of its ammunition drives to the own
/// truck in the rear and is resupplied there.
#[test]
fn ai_sends_empty_units_to_resupply() {
    let mut config = demo_config();
    let tank_spawn = config.spawns[0].clone();
    let own = tank_spawn.position;
    // One tank of player 0; the enemy holds fire.
    config.spawns.retain(|s| s.owner == PlayerId(1));
    for s in &mut config.spawns {
        s.fire_mode = FireMode::HoldFire;
    }
    config.spawns.insert(0, tank_spawn);
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    config
        .unit_types
        .extend(sim::unit_types_from_catalog(&catalog, &scale, &["de_opel_blitz"]).unwrap());
    let enemy = config.spawns[1].position;
    let away = own - enemy;
    let back = away.scale(sim::Fixed::from_num(150) / away.length());
    config.spawns.push(sim::Spawn {
        owner: PlayerId(0),
        unit_type: "de_opel_blitz".into(),
        position: own + back,
        fire_mode: FireMode::HoldFire,
    });
    let tank = UnitId(0);
    let players = [
        (PlayerId(0), Difficulty::Normal),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 4, &players).unwrap();
    let ammo = |game: &Game| game.sim().unit_stock(tank).unwrap().ammo_pct.unwrap();
    // The tank fires at the ground until a fifth of its ammunition is left;
    // the computer player is not asked meanwhile.
    let shoot = Command::AttackGround {
        units: vec![tank],
        target: own + Vec2::from_meters(40, 0),
    };
    game.step(vec![vec![shoot], Vec::new()]);
    while ammo(&game) > 20 {
        assert!(game.sim().tick() < 20 * 400, "the tank did not fire");
        game.step(vec![Vec::new(), Vec::new()]);
    }
    game.step(vec![vec![Command::Stop { units: vec![tank] }], Vec::new()]);
    let mut sent = false;
    while ammo(&game) < 90 {
        assert!(game.sim().tick() < 20 * 900, "not resupplied (sent {sent})");
        let decisions = game.think();
        sent |= decisions[0]
            .iter()
            .any(|c| matches!(c, Command::MoveTo { units, .. } if units.contains(&tank)));
        game.step(decisions);
    }
    assert!(sent, "the AI never sent the tank");
}

/// Prompt 035: Normal lays anti-tank mines with its engineers at a
/// chokepoint; the enemy's view does not show them, the own view does.
#[test]
fn ai_lays_mines_that_the_enemy_does_not_see() {
    let mut config = demo_config();
    config.spawns.retain(|s| s.unit_type.contains("_inf_"));
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    config
        .unit_types
        .extend(sim::unit_types_from_catalog(&catalog, &scale, &["de_inf_engineer"]).unwrap());
    let own = config.spawns[0].position;
    let engineers = UnitId(u32::try_from(config.spawns.len()).unwrap());
    config.spawns.push(sim::Spawn {
        owner: PlayerId(0),
        unit_type: "de_inf_engineer".into(),
        position: own + Vec2::from_meters(0, 10),
        fire_mode: FireMode::FireAtWill,
    });
    let players = [
        (PlayerId(0), Difficulty::Normal),
        (PlayerId(1), Difficulty::Passive),
    ];
    let mut game = Game::new(&config, builtin_map(), 4, &players).unwrap();
    let mut laid = false;
    while game.sim().all_mines().is_empty() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        for command in decisions.iter().flatten() {
            if let Command::Lay { units, .. } = command {
                laid |= units.contains(&engineers);
            }
        }
        game.step(decisions);
    }
    assert!(laid, "no Lay order for the engineers");
    let mines = game.sim().all_mines();
    assert!(
        !mines.is_empty(),
        "no mines after {} ticks",
        game.sim().tick()
    );
    assert!(mines.iter().all(|m| m.owner == Some(PlayerId(0))));
    assert_eq!(
        PlayerView::new(game.sim(), PlayerId(0)).mines.len(),
        mines.len()
    );
    assert!(PlayerView::new(game.sim(), PlayerId(1)).mines.is_empty());
}

/// Prompt 050: on Normal, a badly hurt veteran pulls back out of the fight
/// while a recruit with the same losses fights on.
#[test]
fn normal_pulls_back_hurt_veterans() {
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    let mut config = demo_config();
    config.unit_types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h"]).unwrap();
    let spawn = |owner: u8, x: i32| sim::Spawn {
        owner: PlayerId(owner),
        unit_type: "de_pz4_h".into(),
        position: Vec2::from_meters(x, 0),
        fire_mode: FireMode::FireAtWill,
    };
    config.spawns = vec![spawn(0, -100), spawn(0, -90), spawn(0, -80), spawn(1, 100)];
    let map = builtin_map();
    let sim = Sim::new(&config, map.clone(), 1).unwrap();
    let base = PlayerView::new(&sim, PlayerId(0));
    assert!(!base.enemies.is_empty(), "the enemy tank is not in sight");
    let pulled_back = |rank: u8| {
        let mut view = base.clone();
        let hurt = &mut view.own[0];
        hurt.hp = hurt.max_hp * 35 / 100;
        hurt.rank = rank;
        let mut ai = Ai::new(PlayerId(0), Difficulty::Normal, &config, &map, 3);
        ai.think(&view)
            .iter()
            .any(|c| matches!(c, Command::MoveTo { units, .. } if units.as_slice() == [UnitId(0)]))
    };
    assert!(!pulled_back(0), "a recruit pulled back on Normal");
    assert!(pulled_back(2), "the veteran stayed in the fight");
}

/// Prompt 046: Normal and Hard advance with attack-move in an automatic
/// formation; Easy keeps plain moves.
#[test]
fn normal_advances_with_attack_move_easy_does_not() {
    // The infantry of the demo, out of sight of each other.
    let mut config = demo_config();
    config.spawns.retain(|s| s.unit_type.contains("_inf_"));
    for s in &mut config.spawns {
        if s.owner == PlayerId(1) {
            s.position.y -= sim::Fixed::from_num(150);
        }
    }
    let players = [
        (PlayerId(0), Difficulty::Normal),
        (PlayerId(1), Difficulty::Easy),
    ];
    let mut game = Game::new(&config, builtin_map(), 5, &players).unwrap();
    let mut attack_moves = [0, 0];
    while game.sim().outcome().is_none() && game.sim().tick() < MAX_TICKS {
        let decisions = game.think();
        for (k, commands) in decisions.iter().enumerate() {
            for command in commands {
                if let Command::FormationMove {
                    engage, formation, ..
                } = command
                {
                    assert_eq!(*engage, sim::Engage::All);
                    assert_eq!(*formation, sim::Formation::Auto);
                    attack_moves[k] += 1;
                }
            }
        }
        game.step(decisions);
    }
    assert!(attack_moves[0] > 0, "Normal never attack-moved");
    assert_eq!(attack_moves[1], 0, "Easy attack-moved");
}
