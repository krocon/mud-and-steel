//! Saved games (prompt 038): a match between computer players is saved,
//! loaded and continued; it must go on exactly like the match that was never
//! saved, and its replay must play back from tick 0.

use ai::Difficulty;
use ai::game::Game;
use net::SaveGame;
use sim::scenario::{builtin_map, demo_config, skirmish_config};
use sim::{PlayerId, SimConfig};

fn check_save_and_load(config: &SimConfig, seed: u64, save_at: u64, then: u64) {
    let map = builtin_map();
    let players = [
        (PlayerId(0), Difficulty::Hard),
        (PlayerId(1), Difficulty::Normal),
    ];
    let mut original = Game::new(config, map.clone(), seed, &players).unwrap();
    original.run(save_at);
    assert_eq!(original.sim().tick(), save_at, "decided before the save");

    let bytes = original.save(&map).encode();
    let save = SaveGame::decode(&bytes).expect("the saved game decodes");
    assert_eq!(save.info.ticks, save_at);
    // Loading equals playing through: the replay re-simulated from tick 0
    // ends in the snapshot's state.
    save.verify(map.clone()).expect("replay and snapshot agree");

    let mut loaded = Game::load(&save, map.clone()).expect("the saved game loads");
    assert_eq!(loaded.sim().state_hash(), original.sim().state_hash());
    for _ in 0..then {
        let a = original.think();
        original.step(a);
        let b = loaded.think();
        loaded.step(b);
        assert_eq!(
            loaded.sim().state_hash(),
            original.sim().state_hash(),
            "loaded game diverged at tick {}",
            original.sim().tick()
        );
    }
    let replay = loaded.replay();
    assert_eq!(replay, original.replay());
    let report = replay.verify(map).unwrap();
    assert!(report.matches(), "{report:?}");
    assert_eq!(report.ticks, save_at + then);
}

#[test]
fn saved_skirmish_continues_identically() {
    // Base building: accounts, production queues, the AIs' economy plans.
    check_save_and_load(&skirmish_config(&[]), 3, 2400, 1200);
}

#[test]
fn saved_battle_continues_identically() {
    // Saved in the middle of the fighting (projectiles, wrecks, craters).
    check_save_and_load(&demo_config(), 11, 380, 400);
}

#[test]
fn saves_of_other_rules_are_rejected() {
    let map = builtin_map();
    let mut game = Game::new(&demo_config(), map.clone(), 1, &[]).unwrap();
    game.run(50);
    let mut save = game.save(&map);
    save.info.rules_hash ^= 1;
    let error = SaveGame::decode(&save.encode()).unwrap_err();
    assert!(error.to_string().contains("other rules"), "{error}");
    // The list still shows it.
    assert_eq!(SaveGame::read_info(&save.encode()).unwrap().ticks, 50);
}
