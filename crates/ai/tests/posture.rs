//! AI stances and positions (prompt 178): two AI rifle squads attack the
//! village of `test_small`, held by three passive (kneeling) rifle squads.
//! Numbers with `--nocapture`.

use ai::Difficulty;
use ai::game::Game;
use sim::scenario::builtin_map;
use sim::{FireMode, PlayerId, SimConfig, Spawn, Vec2};

/// Result: winner, deciding tick, soldiers the attacker lost, soldiers the
/// defender lost.
fn village_attack(difficulty: Difficulty, seed: u64) -> (Option<PlayerId>, Option<u64>, u32, u32) {
    let map = builtin_map();
    let village = map.marker("village").expect("test_small has a village");
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    let names = ["su_inf_rifle_squad", "de_inf_rifle_squad"];
    let unit_types = sim::unit_types_from_catalog(&catalog, &scale, &names).unwrap();
    let at = |dx: i32, dy: i32| village + Vec2::from_meters(dx, dy);
    let spawn = |owner: u8, unit: &str, p: Vec2| Spawn {
        owner: PlayerId(owner),
        unit_type: unit.into(),
        position: p,
        fire_mode: FireMode::FireAtWill,
    };
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
        spawns: vec![
            spawn(0, "su_inf_rifle_squad", at(200, -20)),
            spawn(0, "su_inf_rifle_squad", at(200, 20)),
            spawn(1, "de_inf_rifle_squad", at(-25, 0)),
            spawn(1, "de_inf_rifle_squad", at(0, 10)),
            spawn(1, "de_inf_rifle_squad", at(25, 0)),
        ],
        map: map.reference(),
        terrain: sim::TerrainRules::builtin().unwrap(),
        combat: sim::CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    };
    let mut game = Game::new(&config, map, seed, &[(PlayerId(0), difficulty)]).unwrap();
    let result = game.run(20 * 600);
    let snap = game.sim().snapshot();
    let dead = |own: bool| {
        snap.soldiers
            .iter()
            .filter(|s| (s.unit.0 <= 1) == own && s.hp == 0)
            .count() as u32
    };
    (result.winner, result.decided_tick, dead(true), dead(false))
}

#[test]
fn village_attack_numbers() {
    for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
        let runs: Vec<_> = (1..=4).map(|seed| village_attack(d, seed)).collect();
        let wins = runs.iter().filter(|r| r.0 == Some(PlayerId(0))).count();
        let lost: u32 = runs.iter().map(|r| r.2).sum();
        let killed: u32 = runs.iter().map(|r| r.3).sum();
        println!(
            "{d}: attacker wins {wins}/4, lost {lost} of 72 soldiers, killed {killed} of 108, {runs:?}"
        );
    }
}

/// In a base match the squads that wait at their own buildings take to the
/// houses or kneel and lie down when the enemy shows up (Normal and Hard
/// only); Easy gives no such orders.
#[test]
fn defenders_use_houses_and_stances() {
    let map = builtin_map();
    let config = sim::scenario::parse_scenario(
        sim::scenario::SKIRMISH_SCENARIO,
        &sim::Catalog::builtin().unwrap(),
        &sim::Scale::builtin().unwrap(),
        &sim::TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    let count = |d: Difficulty| {
        let mut game = Game::new(
            &config,
            map.clone(),
            3,
            &[(PlayerId(0), d), (PlayerId(1), Difficulty::Hard)],
        )
        .unwrap();
        game.run(20 * 600);
        game.replay()
            .frames
            .iter()
            .flat_map(|f| &f.commands)
            .filter(|c| c.player == PlayerId(0))
            .filter(|c| {
                matches!(
                    c.command,
                    sim::Command::Garrison { .. } | sim::Command::SetStance { .. }
                )
            })
            .count()
    };
    let (easy, normal) = (count(Difficulty::Easy), count(Difficulty::Normal));
    println!("posture orders in a base match: easy {easy}, normal {normal}");
    assert_eq!(easy, 0);
    assert!(normal > 0, "the Normal AI never took a house or a stance");
}
