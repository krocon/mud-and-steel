//! Computer players in team games (prompt 037): they know their allies,
//! never order an attack on them and fight the other team together.

use ai::game::Game;
use ai::{Difficulty, PlayerView};
use sim::scenario::{DEMO_SCENARIO, SKIRMISH_SCENARIO, SlotSetup, parse_scenario_for_slots};
use sim::{Command, Map, PlayerId, SimConfig, UnitId};
use std::path::Path;
use std::sync::Arc;

fn twin_meadow() -> Arc<Map> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/twin_meadow");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    Arc::new(map)
}

/// Players 0 and 2 against 1 and 3, all Soviet (a mirror match).
fn two_against_two(scenario: &str, map: &Map) -> SimConfig {
    let slots: Vec<SlotSetup> = (0..4)
        .map(|p| SlotSetup {
            player: PlayerId(p),
            team: p % 2,
            faction: Some("su".into()),
        })
        .collect();
    parse_scenario_for_slots(
        scenario,
        &slots,
        true,
        &sim::Catalog::builtin().unwrap(),
        &sim::Scale::builtin().unwrap(),
        &sim::TerrainRules::builtin().unwrap(),
        map,
    )
    .unwrap()
}

#[test]
fn the_view_tells_allies_from_enemies() {
    let map = twin_meadow();
    let config = two_against_two(DEMO_SCENARIO, &map);
    let sim = sim::Sim::new(&config, map, 1).unwrap();
    let view = PlayerView::new(&sim, PlayerId(0));
    assert!(!view.allies.is_empty());
    assert!(view.allies.iter().all(|u| u.owner == PlayerId(2)));
    assert!(
        view.enemies
            .iter()
            .all(|u| u.owner == PlayerId(1) || u.owner == PlayerId(3))
    );
    assert_eq!(
        view.enemy_players,
        vec![(PlayerId(1), false), (PlayerId(3), false)]
    );
}

#[test]
fn computer_teams_fight_the_other_team_only() {
    let map = twin_meadow();
    let config = two_against_two(DEMO_SCENARIO, &map);
    let owner = |id: UnitId| config.spawns[id.0 as usize].owner;
    for seed in [1, 2] {
        let levels: Vec<(PlayerId, Difficulty)> = (0..4)
            .map(|p| {
                let level = if p % 2 == seed as u8 % 2 {
                    Difficulty::Hard
                } else {
                    Difficulty::Passive
                };
                (PlayerId(p), level)
            })
            .collect();
        let mut game = Game::new(&config, map.clone(), seed, &levels).unwrap();
        let mut attacks = 0;
        for _ in 0..2500 {
            let decisions = game.think();
            for (ai, commands) in game.ais().iter().zip(&decisions) {
                for command in commands {
                    if let Command::Attack { target, .. } = command {
                        attacks += 1;
                        assert!(
                            !config.teams.allied(ai.player(), owner(*target)),
                            "player {} attacks an ally",
                            ai.player().0
                        );
                    }
                }
            }
            game.step(decisions);
        }
        assert!(attacks > 0);
        let result = game.result();
        let side = |team: u8| -> usize {
            result
                .alive
                .iter()
                .filter(|(p, _)| p.0 % 2 == team)
                .map(|&(_, n)| n)
                .sum()
        };
        // Both hard players fought: the passive team lost units. (Who
        // wins on this map is a matter of balance, see docs/AI.md.)
        let passive = 1 - seed as u8 % 2;
        let start = |team: u8| {
            config
                .spawns
                .iter()
                .filter(|s| s.owner.0 % 2 == team)
                .count()
        };
        assert!(side(passive) < start(passive), "seed {seed}: {result:?}");
    }
}

#[test]
fn every_computer_player_builds_in_a_team_base_match() {
    let map = twin_meadow();
    let config = two_against_two(SKIRMISH_SCENARIO, &map);
    let levels: Vec<(PlayerId, Difficulty)> =
        (0..4).map(|p| (PlayerId(p), Difficulty::Normal)).collect();
    let mut game = Game::new(&config, map, 5, &levels).unwrap();
    game.run(3000);
    for p in 0..4 {
        let built = game
            .sim()
            .structures()
            .iter()
            .filter(|s| s.owner == PlayerId(p))
            .count();
        assert!(built > 1, "player {p} built nothing");
    }
}
