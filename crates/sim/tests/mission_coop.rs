//! Cooperative missions (prompt 190): human seats (`[[seat]]`), free seats
//! merged into the mission's player, the side as one team, and the units
//! of a human who gives up passed to the ally who plays on.

use sim::map::MapLayout;
use sim::mission::{Condition, parse_mission, parse_mission_with_seats};
use sim::{Catalog, Command, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, TerrainRules};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const P2: PlayerId = PlayerId(2);

fn flat_map() -> Arc<Map> {
    let n: usize = 201;
    let layout = MapLayout {
        name: "coop_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, vec![0; n * n], vec![0; n * n]).unwrap())
}

const SCRIPT: &str = r#"
[mission]
title = "Coop"
elimination = false

[[computer]]
player = 1
difficulty = "passive"

[[seat]]
player = 2
name = "Flank"
briefing = "Hold the flank."

[[group]]
name = "main"
owner = 0
unit = "de_pz4_h"
count = 2
position_m = [-100, 100]

[[group]]
name = "flank"
owner = 2
unit = "de_pz4_h"
count = 1
position_m = [100, 100]

[[group]]
name = "enemy"
owner = 1
unit = "su_t34_76_1943"
position_m = [0, -150]
hold_fire = true

[[area]]
name = "east"
center_m = [100, 0]
radius_m = 20

[[objective]]
id = "hold"
text = "Hold"
shown = true

[[trigger]]
id = "flank_there"
when = "area"
area = "east"
player = 2
actions = [{ do = "spawn", group = "help", owner = 2, unit = "de_pz4_h", count = 1, at = "east" }]
"#;

fn parse(text: &str, humans: Option<&[PlayerId]>) -> Result<SimConfig, String> {
    let (c, s, t, m) = (
        Catalog::builtin().unwrap(),
        Scale::builtin().unwrap(),
        TerrainRules::builtin().unwrap(),
        flat_map(),
    );
    match humans {
        None => parse_mission("coop", text, None, &c, &s, &t, &m),
        Some(h) => parse_mission_with_seats(
            "coop",
            text,
            None,
            &c,
            &s,
            &t,
            &m,
            sim::mission::Level::Normal,
            h,
        ),
    }
    .map_err(|e| e.to_string())
}

fn owners(config: &SimConfig) -> Vec<PlayerId> {
    config.spawns.iter().map(|s| s.owner).collect()
}

#[test]
fn a_free_seat_is_merged_into_the_leader() {
    let config = parse(SCRIPT, None).unwrap();
    assert_eq!(owners(&config), [P0, P0, P0, P1]);
    let m = config.mission.as_ref().unwrap();
    assert_eq!(m.seats.len(), 1);
    assert!(!m.seats[0].taken);
    // Conditions and spawns of the seat became the leader's.
    let t = &m.triggers[0];
    assert!(matches!(t.condition, Condition::Area { player, .. } if player == P0));
    assert!(matches!(
        t.actions[0],
        sim::mission::Action::Spawn { owner, .. } if owner == P0
    ));
    // The leader plays every unit of the side.
    let sim = Sim::new(&config, flat_map(), 1).unwrap();
    assert!(sim.alive_units(P2) == 0 && sim.alive_units(P0) == 3);
}

#[test]
fn a_taken_seat_plays_its_units_on_the_leaders_team() {
    let config = parse(SCRIPT, Some(&[P0, P2])).unwrap();
    assert_eq!(owners(&config), [P0, P0, P2, P1]);
    assert!(config.mission.as_ref().unwrap().seats[0].taken);
    assert!(config.teams.allied(P0, P2));
    assert!(!config.teams.allied(P0, P1));
    let sim = Sim::new(&config, flat_map(), 1).unwrap();
    assert_eq!(sim.alive_units(P2), 1);
}

#[test]
fn a_human_who_gives_up_hands_the_units_to_the_ally() {
    let config = parse(SCRIPT, Some(&[P0, P2])).unwrap();
    let mut sim = Sim::new(&config, flat_map(), 1).unwrap();
    sim.step(&[PlayerCommand::new(P2, Command::Surrender)]);
    sim.step(&[]);
    assert_eq!(sim.alive_units(P2), 0);
    assert_eq!(sim.alive_units(P0), 3);
    assert!(sim.outcome().is_none(), "the leader plays on");
    // The last human gives up: the computer wins.
    sim.step(&[PlayerCommand::new(P0, Command::Surrender)]);
    sim.step(&[]);
    assert_eq!(sim.outcome().map(|o| o.winner), Some(Some(P1)));
}

#[test]
fn a_mission_without_seats_is_unchanged_and_bad_seats_are_errors() {
    let without = SCRIPT
        .replace(
            "[[seat]]\nplayer = 2\nname = \"Flank\"\nbriefing = \"Hold the flank.\"\n",
            "",
        )
        .replace("owner = 2", "owner = 0")
        .replace("player = 2\n", "");
    let config = parse(&without, None).unwrap();
    assert_eq!(config.teams, sim::Teams::default());
    let computer = SCRIPT.replace("[[seat]]\nplayer = 2", "[[seat]]\nplayer = 1");
    assert!(parse(&computer, None).is_err());
    let nobody = SCRIPT.replace("[[seat]]\nplayer = 2", "[[seat]]\nplayer = 3");
    assert!(parse(&nobody, None).is_err());
}
