//! Mission scripts (prompt 039): parsing, triggers and actions.

use sim::map::{MapLayout, Marker};
use sim::mission::{MissionPoints, parse_mission};
use sim::{
    Catalog, Command, DefeatReason, Map, ObjectiveState, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, TerrainRules, UnitId, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// 400 m × 400 m, flat, with a supply point at (100, 0).
fn flat_map() -> Arc<Map> {
    let n: usize = 201;
    let layout = MapLayout {
        name: "mission_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: vec![Marker {
            name: "supply_a".into(),
            position: Vec2::from_meters(100, 0),
        }],
    };
    Arc::new(Map::from_rasters(layout, vec![0; n * n], vec![0; n * n]).unwrap())
}

fn parse(text: &str, map: &Map) -> Result<SimConfig, String> {
    parse_mission(
        "test",
        text,
        None,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        map,
    )
    .map_err(|e| e.to_string())
}

fn start(text: &str) -> Sim {
    let map = flat_map();
    let config = parse(text, &map).unwrap();
    Sim::new(&config, map, 3).unwrap()
}

fn run(sim: &mut Sim, ticks: u32, commands: &[PlayerCommand]) {
    sim.step(commands);
    for _ in 1..ticks {
        sim.step(&[]);
    }
}

fn objective(sim: &Sim, id: &str) -> ObjectiveState {
    sim.mission_objectives()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
        .state
}

/// Two tanks of the player at (0, 100), one enemy tank far away holding
/// fire.
const UNITS: &str = r#"
[mission]
title = "Test"
elimination = false

[[group]]
name = "tanks"
owner = 0
unit = "de_pz4_h"
count = 2
position_m = [0, 100]

[[group]]
name = "far"
owner = 1
unit = "su_t34_76_1943"
position_m = [-180, -180]
hold_fire = true

[[area]]
name = "goal"
center_m = [0, 0]
radius_m = 20

[[spawn]]
name = "range"
position_m = [0, 40]
"#;

#[test]
fn a_timer_shows_its_text_after_the_delay() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "hello"
when = "time"
delay_s = 2
actions = [{{ do = "text", text = "Hello" }}]
"#
    ));
    run(&mut sim, 40, &[]);
    assert_eq!(sim.mission_message(), None);
    run(&mut sim, 1, &[]);
    assert_eq!(sim.mission_message(), Some(("Hello", 41)));
    assert_eq!(sim.mission_progress(), (1, 1));
}

#[test]
fn entering_an_area_completes_the_objective_and_wins() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[objective]]
id = "go"
text = "Go"
area = "goal"
shown = true

[[trigger]]
id = "arrived"
when = "area"
area = "goal"
count = 2
actions = [
  {{ do = "objective", objective = "go", state = "done" }},
  {{ do = "victory" }},
]
"#
    ));
    assert_eq!(objective(&sim, "go"), ObjectiveState::Active);
    let go = PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0), UnitId(1)],
            target: Vec2::from_meters(0, 0),
        },
    );
    run(&mut sim, 30, &[go]);
    assert_eq!(sim.outcome(), None, "not there yet");
    run(&mut sim, 600, &[]);
    assert_eq!(objective(&sim, "go"), ObjectiveState::Done);
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, Some(P0));
    assert_eq!(
        sim.standings()[1].defeat.map(|d| d.0),
        Some(DefeatReason::Mission)
    );
}

#[test]
fn signals_count_only_from_the_mission_player_and_while_armed() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "first"
when = "time"
delay_s = 1

[[trigger]]
id = "looked"
after = ["first"]
when = "signal"
signal = "camera"
actions = [{{ do = "defeat" }}]
"#
    ));
    let signal = |player| {
        PlayerCommand::new(
            player,
            Command::MissionSignal {
                name: "camera".into(),
            },
        )
    };
    // Not armed yet: the signal is lost.
    run(&mut sim, 1, &[signal(P0)]);
    assert!(sim.mission_wanted_signals().is_empty());
    run(&mut sim, 25, &[]);
    assert_eq!(sim.mission_wanted_signals(), vec!["camera"]);
    run(&mut sim, 5, &[signal(P1)]);
    assert_eq!(sim.outcome(), None, "the enemy cannot send signals");
    run(&mut sim, 1, &[signal(P0)]);
    let outcome = sim.outcome().expect("decided");
    assert_eq!(outcome.winner, Some(P1));
    assert_eq!(
        sim.standings()[0].defeat.map(|d| d.0),
        Some(DefeatReason::Mission)
    );
}

#[test]
fn spawned_targets_are_destroyed_and_elimination_waits_for_the_script() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "targets"
when = "time"
actions = [{{ do = "spawn", group = "targets", owner = 1, unit = "su_t70m", count = 2, at = "range", hold_fire = true }}]

[[trigger]]
id = "far_gone"
when = "destroyed"
group = "far"
actions = [{{ do = "text", text = "far" }}]

[[trigger]]
id = "hit"
after = ["targets"]
when = "destroyed"
group = "targets"
delay_s = 1
actions = [{{ do = "victory" }}]
"#
    ));
    run(&mut sim, 2, &[]);
    let targets = sim.mission_group("targets");
    assert_eq!(targets.len(), 2);
    assert!(
        targets
            .iter()
            .all(|id| sim.unit_hp(*id).is_some_and(|hp| hp > 0))
    );
    // The tanks fire at will at the targets 60 m away.
    run(&mut sim, 1200, &[]);
    assert!(targets.iter().all(|id| sim.unit_hp(*id) == Some(0)));
    let outcome = sim.outcome().expect("victory");
    assert_eq!(outcome.winner, Some(P0));
}

#[test]
fn an_order_moves_a_group_and_capturing_a_point_counts() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "go"
when = "time"
actions = [{{ do = "order", group = "tanks", to = "supply_a", order = "move" }}]

[[trigger]]
id = "taken"
when = "captured"
point = "supply_a"
actions = [{{ do = "text", text = "taken" }}]
"#
    ));
    run(&mut sim, 3, &[]);
    assert!(sim.snapshot().units[0].target.is_some(), "no order");
    run(&mut sim, 2000, &[]);
    assert_eq!(
        sim.mission_message().map(|m| m.0),
        Some("taken"),
        "{:?}",
        sim.supply_points()
    );
}

#[test]
fn units_count_completed_and_eliminated_conditions() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "two"
when = "units"
unit = "de_pz4"
count = 2
actions = [{{ do = "text", text = "two" }}]

[[trigger]]
id = "three"
when = "units"
unit = "de_pz4"
count = 3
actions = [{{ do = "text", text = "three" }}]

[[trigger]]
id = "none"
when = "eliminated"
player = 1
actions = [{{ do = "text", text = "eliminated" }}]
"#
    ));
    run(&mut sim, 2, &[]);
    assert_eq!(sim.mission_message().map(|m| m.0), Some("two"));
    assert_eq!(sim.mission_progress(), (1, 3));
}

#[test]
fn errors_name_what_is_wrong() {
    let map = flat_map();
    let bad = [
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"area\"\narea = \"nowhere\"",
            "unknown area 'nowhere'",
        ),
        (
            "[[trigger]]\nid = \"a\"\nafter = [\"b\"]\nwhen = \"time\"",
            "unknown trigger 'b'",
        ),
        ("[[trigger]]\nid = \"a\"\nwhen = \"often\"", "unknown `when"),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"dance\" }]",
            "unknown action",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"spawn\", group = \"g\", owner = 2, unit = \"de_pz4_h\", at = \"goal\" }]",
            "player 2 has no units",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"spawn\", group = \"g\", unit = \"base_hq\", at = \"goal\" }]",
            "buildings cannot be spawned",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"captured\"\npoint = \"nope\"",
            "no supply point",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\n[[trigger]]\nid = \"a\"\nwhen = \"time\"",
            "twice",
        ),
    ];
    for (extra, expected) in bad {
        let e = parse(&format!("{UNITS}\n{extra}\n"), &map).unwrap_err();
        assert!(e.contains(expected), "{extra}: {e}");
    }
}

#[test]
fn points_round_trip_through_their_file() {
    let points = MissionPoints::parse(
        "[[area]]\nname = \"a\"\ncenter_m = [1, -2]\nradius_m = 30\n[[spawn]]\nname = \"s\"\nposition_m = [5, 6]\n",
    )
    .unwrap();
    assert_eq!(MissionPoints::parse(&points.to_toml()).unwrap(), points);
    assert!(!points.is_empty());
}

#[test]
fn skirmishes_hash_as_before_and_missions_add_their_state() {
    // The golden hash test covers the demo; a mission changes the hash.
    let map = flat_map();
    let with = parse(UNITS, &map).unwrap();
    let mut without = with.clone();
    without.mission = None;
    let a = Sim::new(&with, map.clone(), 1).unwrap();
    let b = Sim::new(&without, map, 1).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
}
