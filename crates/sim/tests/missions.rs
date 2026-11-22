//! Mission scripts (prompt 039): parsing, triggers and actions.

use sim::Fixed;
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

/// Prompt 082: `barrage`, `air` and `camouflage` issue the normal commands
/// of the units' owners.
#[test]
fn barrage_air_and_camouflage_are_commands_of_the_owner() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[group]]
name = "rockets"
owner = 0
unit = "de_pzwerfer42"
position_m = [0, 150]

[[trigger]]
id = "go"
when = "time"
actions = [
  {{ do = "camouflage", group = "far" }},
  {{ do = "barrage", group = "rockets", at = "goal", radius_m = 30 }},
  {{ do = "air", player = 1, mission = "bomb", at = "range" }},
]
"#
    ));
    let far = sim.mission_group("far")[0];
    assert_eq!(sim.camouflage_of(far), None);
    run(&mut sim, 3, &[]);
    assert!(sim.camouflage_of(far).is_some(), "no camouflage order");
    assert_eq!(sim.snapshot().sorties.len(), 1, "no air call");
    // The launcher sets up, then its salvo is in the air.
    let mut fired = false;
    for _ in 0..1200 {
        sim.step(&[]);
        fired |= !sim.snapshot().shells.is_empty();
    }
    assert!(fired, "no rocket was fired");
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
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"air\", mission = \"zeppelin\", at = \"goal\" }]",
            "unknown air mission",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"captured\"\npoint = \"nope\"",
            "no supply point",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\n[[trigger]]\nid = \"a\"\nwhen = \"time\"",
            "twice",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"spawn\", group = \"g\", unit = \"de_pz4_h\", at = \"goal\", arrive = \"fly\" }]",
            "arrive must be appear or drive",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"spawn\", group = \"g\", unit = \"de_pz4_h\", at = \"goal\", from = \"range\" }]",
            "`from` needs",
        ),
        (
            "[[trigger]]\nid = \"a\"\nwhen = \"time\"\nactions = [{ do = \"spawn\", group = \"g\", unit = \"de_pz4_h\", at = \"goal\", arrive = \"drive\", from = \"nowhere\" }]",
            "unknown place 'nowhere'",
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

// --- Prompt 108: reinforcements drive in ------------------------------------------

fn unit(snapshot: &sim::Snapshot, id: UnitId) -> &sim::UnitSnapshot {
    snapshot.units.iter().find(|u| u.id == id).unwrap()
}

/// `arrive = "drive"`: the units start in a column on the map edge nearest
/// to the place (the south edge for (0, 40) on the 400 m map), the leader
/// first, one column spacing apart, and drive to the place.
#[test]
fn a_driving_spawn_starts_as_a_column_at_the_edge_and_drives_in() {
    let text = format!(
        r#"{UNITS}
[[trigger]]
id = "come"
when = "time"
actions = [{{ do = "spawn", group = "relief", owner = 0, unit = "de_pz4_h", count = 3, at = "range", arrive = "drive" }}]
"#
    );
    let map = flat_map();
    let config = parse(&text, &map).unwrap();
    let footprint = config
        .unit_types
        .iter()
        .find(|t| t.name == "de_pz4_h")
        .unwrap()
        .footprint_m;
    let mut sim = Sim::new(&config, map, 3).unwrap();
    run(&mut sim, 1, &[]);
    let ids = sim.mission_group("relief");
    assert_eq!(ids.len(), 3);
    let snap = sim.snapshot();
    let ys: Vec<f64> = ids
        .iter()
        .map(|&id| unit(&snap, id).position.y.to_num::<f64>())
        .collect();
    let spacing = sim::column_spacing(footprint).to_num::<f64>();
    // Rearmost half a spacing inside the edge line (196 m), the others
    // ahead of it towards the place, one spacing apart.
    assert!((ys[2] - (196.0 - spacing / 2.0)).abs() < 0.5, "{ys:?}");
    assert!((ys[1] - (ys[2] - spacing)).abs() < 0.5, "{ys:?}");
    assert!((ys[0] - (ys[1] - spacing)).abs() < 0.5, "{ys:?}");
    for &id in &ids {
        let u = unit(&snap, id);
        assert!(u.position.x.abs() < Fixed::from_num(1), "{:?}", u.position);
        assert!(u.facing.y < Fixed::ZERO, "faces north, to the place");
        assert!(u.target.is_some(), "has a move order from the first tick");
    }
    let arrivals = sim.mission_arrivals();
    assert_eq!(arrivals.len(), 1);
    assert_eq!(arrivals[0].owner, P0);
    assert_eq!(arrivals[0].entry, Vec2::from_meters(0, 196));
    assert_eq!(arrivals[0].to, Vec2::from_meters(0, 40));
    // About 150 m at road march speed: there well within two minutes.
    run(&mut sim, 2400, &[]);
    let snap = sim.snapshot();
    for &id in &ids {
        let p = unit(&snap, id).position;
        assert!(
            (p - Vec2::from_meters(0, 40)).length() < Fixed::from_num(25),
            "{p:?}"
        );
    }
}

/// Spawns of one trigger with the same entry and place form one column in
/// file order (the tank first, the squads behind); an `order` after them
/// replaces the move order; squads with `ride` mount the carriers (seats
/// inside before a tank's hull).
#[test]
fn driving_spawns_of_a_trigger_queue_up_and_passengers_mount() {
    let mut sim = start(&format!(
        r#"{UNITS}
[[trigger]]
id = "come"
when = "time"
actions = [
  {{ do = "spawn", group = "relief", owner = 1, unit = "su_t34_76_1943", count = 1, at = "range", arrive = "drive" }},
  {{ do = "spawn", group = "relief", owner = 1, unit = "su_inf_rifle_squad", count = 2, at = "range", arrive = "drive" }},
  {{ do = "order", group = "relief", to = "goal", order = "move" }},
]

[[trigger]]
id = "lift"
when = "time"
delay_s = 1
actions = [
  {{ do = "spawn", group = "trucks", owner = 0, unit = "de_pz4_h", count = 1, at = "goal", arrive = "drive", from = "range" }},
  {{ do = "spawn", group = "trucks", owner = 0, unit = "us_m3_halftrack", count = 1, at = "goal", arrive = "drive", from = "range" }},
  {{ do = "spawn", group = "riders", owner = 0, unit = "us_inf_rifle_squad", count = 1, at = "goal", arrive = "drive", from = "range", ride = "trucks" }},
]
"#
    ));
    run(&mut sim, 1, &[]);
    let ids = sim.mission_group("relief");
    assert_eq!(ids.len(), 3);
    let snap = sim.snapshot();
    let y = |id: UnitId| unit(&snap, id).position.y;
    assert!(
        y(ids[0]) < y(ids[1]) && y(ids[1]) < y(ids[2]),
        "tank ahead of the squads"
    );
    // The order of the same trigger replaced the move to `range`.
    for &id in &ids {
        let target = unit(&snap, id).target.expect("moving");
        assert!(target.y.abs() < Fixed::from_num(20), "{target:?}");
    }
    run(&mut sim, 25, &[]);
    // The halftrack's seats come before riding on the tank's hull.
    let truck = sim.mission_group("trucks")[1];
    let rider = sim.mission_group("riders")[0];
    let snap = sim.snapshot();
    assert_eq!(unit(&snap, rider).carrier, Some(truck), "mounted at once");
    assert!(unit(&snap, truck).target.is_some());
    // The column started at the place `from`.
    let entry = sim.mission_arrivals().last().unwrap().entry;
    assert_eq!(entry, Vec2::from_meters(0, 40));
}

/// The entry moves along the edge to the nearest road.
#[test]
fn the_entry_is_the_nearest_road_on_the_nearest_edge() {
    let n: usize = 201;
    let mut terrain = vec![0u8; n * n];
    // A road (terrain 2) along the column x = 60 m (sample 130).
    for row in 0..n {
        terrain[row * n + 130] = 2;
    }
    let layout = MapLayout {
        name: "road_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let map = Map::from_rasters(layout, vec![0; n * n], terrain).unwrap();
    let rules = TerrainRules::builtin().unwrap();
    let entry = sim::mission::edge_entry(&map, &rules, Vec2::from_meters(10, -150));
    assert_eq!(entry, Vec2::from_meters(60, -196));
    // No road within reach: straight out to the edge.
    let entry = sim::mission::edge_entry(&map, &rules, Vec2::from_meters(-150, 20));
    assert_eq!(entry, Vec2::from_meters(-196, 20));
}

/// Driving reinforcements play the same twice (deterministic).
#[test]
fn driving_in_is_deterministic() {
    let text = format!(
        r#"{UNITS}
[[trigger]]
id = "come"
when = "time"
actions = [{{ do = "spawn", group = "relief", owner = 1, unit = "su_t34_76_1943", count = 4, at = "goal", arrive = "drive" }}]
"#
    );
    let mut a = start(&text);
    let mut b = start(&text);
    run(&mut a, 600, &[]);
    run(&mut b, 600, &[]);
    assert_eq!(a.state_hash(), b.state_hash());
}

/// `shell`: the group's units shell the buildings near a place, one each,
/// nearest first, as the owner's `AttackObject` orders.
#[test]
fn shell_sends_the_group_against_the_nearest_buildings() {
    let map = sim::scenario::builtin_map();
    let text = r#"
[mission]
title = "shell"
player = 0
elimination = false

[[spawn]]
name = "church"
position_m = [-106, 140]

[[group]]
name = "guns"
owner = 0
unit = "de_pz4_h"
position_m = [-106, 90]

[[group]]
name = "enemy"
owner = 1
unit = "su_t34_76_1943"
position_m = [100, -100]

[[trigger]]
id = "go"
when = "time"
actions = [{ do = "shell", group = "guns", at = "church", radius_m = 20 }]
"#;
    let config = parse(text, &map).unwrap();
    let mut sim = Sim::new(&config, map.clone(), 3).unwrap();
    let target = Vec2::from_meters(-106, 140);
    let church = map
        .objects()
        .iter()
        .enumerate()
        .min_by_key(|(_, o)| (o.position - target).length())
        .map(|(i, _)| i as u32)
        .unwrap();
    let (before, _) = sim.object_hp(church).unwrap();
    run(&mut sim, 1200, &[]);
    let (after, _) = sim.object_hp(church).unwrap();
    assert!(
        after < before,
        "the church was not shelled: {before} -> {after}"
    );
}
