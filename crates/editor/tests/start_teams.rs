//! Start positions with teams (prompt 037): set, saved in `map.toml`,
//! loaded by the simulation, undone.

use editor::EditMap;
use sim::Biome;

#[test]
fn start_teams_are_saved_loaded_and_undone() {
    let mut map = EditMap::new("teams_test", 400, 400, Biome::Forest).unwrap();
    for (player, x, y) in [(2, -60, 80), (3, 60, -80)] {
        map.begin_stroke();
        map.set_start(player, x * 1000, y * 1000);
        map.set_start_team(player, Some(player % 2));
        map.end_stroke();
    }
    map.set_start_team(0, Some(0));
    map.set_start_team(1, Some(1));
    let files = map.files();
    assert!(
        files.map_toml.contains("player = 3\nteam = 1\n"),
        "{}",
        files.map_toml
    );

    let loaded = files.load().unwrap();
    let teams: Vec<(u8, Option<u8>)> = loaded.starts().iter().map(|s| (s.player, s.team)).collect();
    assert_eq!(
        teams,
        vec![(0, Some(0)), (1, Some(1)), (2, Some(0)), (3, Some(1))]
    );

    // Undo removes the team again; removing a start drops its team.
    map.undo();
    assert_eq!(map.meta().start_team(1), None);
    map.remove_start(3);
    assert_eq!(map.meta().start_team(3), None);
    assert_eq!(map.meta().start_team(2), Some(0));
}
