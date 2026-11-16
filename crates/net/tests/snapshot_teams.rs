//! Prompt 037: the teams are part of the saved state; a restored team
//! match continues exactly like the original.

use net::snapshot;
use sim::scenario::{DEMO_SCENARIO, SlotSetup, parse_scenario_for_slots};
use sim::{Command, Map, PlayerCommand, PlayerId, Sim, Vec2};
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

#[test]
fn a_restored_team_match_continues_identically() {
    let map = twin_meadow();
    let slots: Vec<SlotSetup> = (0..4)
        .map(|p| SlotSetup {
            player: PlayerId(p),
            team: p % 2,
            faction: None,
        })
        .collect();
    let config = parse_scenario_for_slots(
        DEMO_SCENARIO,
        &slots,
        true,
        &sim::Catalog::builtin().unwrap(),
        &sim::Scale::builtin().unwrap(),
        &sim::TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    let mut original = Sim::new(&config, map.clone(), 5).unwrap();
    // Player 0 surrenders and player 1 drives to the middle: the standings
    // and the teams are in the saved state.
    original.step(&[PlayerCommand::new(PlayerId(0), Command::Surrender)]);
    let units: Vec<_> = original
        .snapshot()
        .units
        .iter()
        .filter(|u| u.owner == PlayerId(1))
        .map(|u| u.id)
        .collect();
    original.step(&[PlayerCommand::new(
        PlayerId(1),
        Command::MoveTo {
            units,
            target: Vec2::from_meters(0, 0),
        },
    )]);
    for _ in 0..200 {
        original.step(&[]);
    }
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.teams(), original.teams());
    assert!(restored.allied(PlayerId(0), PlayerId(2)));
    assert_eq!(restored.state_hash(), original.state_hash());
    for _ in 0..300 {
        original.step(&[]);
        restored.step(&[]);
        assert_eq!(restored.state_hash(), original.state_hash());
    }
    assert_eq!(restored.outcome(), original.outcome());
}
