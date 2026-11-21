//! Mission difficulty and the "hardly winnable" warning (prompt 181):
//! `[difficulty.<level>]` values, `levels` of groups, the condition
//! `fewer`, the trigger field `open` and the action `hopeless`.

use sim::map::MapLayout;
use sim::mission::{Action, Level, parse_mission, parse_mission_at};
use sim::{Catalog, Map, PlayerId, Scale, Sim, SimConfig, TerrainRules};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);

fn flat_map() -> Arc<Map> {
    let n: usize = 201;
    let layout = MapLayout {
        name: "mission_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, vec![0; n * n], vec![0; n * n]).unwrap())
}

fn parse_at(text: &str, level: Level) -> Result<SimConfig, String> {
    parse_mission_at(
        "test",
        text,
        None,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &flat_map(),
        level,
    )
    .map_err(|e| e.to_string())
}

const SCRIPT: &str = r#"
[mission]
title = "Test"
elimination = false

[[computer]]
player = 1
difficulty = "normal"

[[group]]
name = "tanks"
owner = 0
unit = "de_pz4_h"
count = 2
position_m = [0, 100]

[[group]]
name = "guard"
owner = 1
unit = "su_t34_76_1943"
position_m = [-150, -150]
hold_fire = true

[[group]]
name = "extra"
owner = 1
unit = "su_t34_76_1943"
position_m = [150, -150]
hold_fire = true
levels = ["hard"]

[[area]]
name = "east"
center_m = [150, 0]
radius_m = 20

[[objective]]
id = "hold"
text = "Hold"
shown = true

[[trigger]]
id = "enemy"
when = "time"
delay_s = 60
actions = [{ do = "spawn", group = "wave", owner = 1, unit = "su_t34_76_1943", count = 4, at = "east", hold_fire = true }]

[[trigger]]
id = "help"
when = "time"
delay_s = 30
actions = [{ do = "spawn", group = "help", owner = 0, unit = "de_pz4_h", count = 2, at = "east" }]

[[trigger]]
id = "limit"
when = "time"
delay_s = 600
actions = [{ do = "objective", objective = "hold", state = "failed" }, { do = "defeat" }]

[[trigger]]
id = "lost"
when = "fewer"
unit = "de_pz4"
count = 2
open = ["hold"]
actions = [{ do = "hopeless", text = "Too few tanks left." }]

[difficulty.easy]
computer = "easy"
enemy_count_pct = 50
enemy_delay_pct = 200
own_count_pct = 150
time_pct = 150

[difficulty.hard]
computer = "hard"
enemy_start_pct = 200
enemy_count_pct = 150
enemy_delay_pct = 50
own_count_pct = 50
time_pct = 75
"#;

fn spawn_count(config: &SimConfig, trigger: &str) -> u32 {
    let m = config.mission.as_ref().unwrap();
    let t = m.triggers.iter().find(|t| t.id == trigger).unwrap();
    t.actions
        .iter()
        .find_map(|a| match a {
            Action::Spawn { count, .. } => Some(*count),
            _ => None,
        })
        .unwrap()
}

fn delay_s(config: &SimConfig, trigger: &str) -> u32 {
    let m = config.mission.as_ref().unwrap();
    m.triggers
        .iter()
        .find(|t| t.id == trigger)
        .unwrap()
        .delay_ticks
        / 20
}

#[test]
fn levels_override_the_script() {
    let easy = parse_at(SCRIPT, Level::Easy).unwrap();
    let normal = parse_at(SCRIPT, Level::Normal).unwrap();
    let hard = parse_at(SCRIPT, Level::Hard).unwrap();
    for (name, c) in [("easy", &easy), ("normal", &normal), ("hard", &hard)] {
        println!(
            "{name}: computer {:?}, enemy wave {} after {} s, own help {}, limit {} s, start units {}",
            c.mission.as_ref().unwrap().computers,
            spawn_count(c, "enemy"),
            delay_s(c, "enemy"),
            spawn_count(c, "help"),
            delay_s(c, "limit"),
            c.spawns.len()
        );
    }
    // Normal has no section: the script as written.
    assert_eq!(spawn_count(&normal, "enemy"), 4);
    assert_eq!(delay_s(&normal, "enemy"), 60);
    assert_eq!(normal.mission.as_ref().unwrap().computers[0].1, "normal");
    assert_eq!(normal.spawns.len(), 3, "the hard-only group is left out");
    assert_eq!(normal.mission.as_ref().unwrap().level, Level::Normal);
    // Easy.
    assert_eq!(easy.mission.as_ref().unwrap().computers[0].1, "easy");
    assert_eq!(spawn_count(&easy, "enemy"), 2);
    assert_eq!(delay_s(&easy, "enemy"), 120);
    assert_eq!(spawn_count(&easy, "help"), 3);
    assert_eq!(easy.spawns.len(), 3, "own start groups keep their count");
    assert_eq!(
        delay_s(&easy, "help"),
        30,
        "own reinforcements keep their time"
    );
    assert_eq!(delay_s(&easy, "limit"), 900);
    // Hard.
    assert_eq!(hard.mission.as_ref().unwrap().computers[0].1, "hard");
    assert_eq!(spawn_count(&hard, "enemy"), 6);
    assert_eq!(delay_s(&hard, "enemy"), 30);
    assert_eq!(spawn_count(&hard, "help"), 1);
    assert_eq!(delay_s(&hard, "limit"), 450);
    assert_eq!(hard.spawns.len(), 6, "two of each enemy start group");
    // Without `[difficulty]` every level is the script.
    let plain = SCRIPT.split("[difficulty.easy]").next().unwrap();
    let a = parse_at(plain, Level::Easy).unwrap();
    let b = parse_mission(
        "test",
        plain,
        None,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &flat_map(),
    )
    .unwrap();
    assert_eq!(a.spawns, b.spawns);
    assert_eq!(
        a.mission.as_ref().unwrap().triggers,
        b.mission.as_ref().unwrap().triggers
    );
}

#[test]
fn level_errors_name_what_is_wrong() {
    let bad = SCRIPT.replace("levels = [\"hard\"]", "levels = [\"brutal\"]");
    assert!(
        parse_at(&bad, Level::Normal)
            .unwrap_err()
            .contains("brutal")
    );
    let typo = SCRIPT.replace("time_pct = 75", "time_percent = 75");
    assert!(parse_at(&typo, Level::Normal).is_err());
    // The group exists only on Hard: a `destroyed` trigger on it would
    // never fire on the other levels.
    let watched =
        format!("{SCRIPT}\n[[trigger]]\nid = \"x\"\nwhen = \"destroyed\"\ngroup = \"extra\"\n");
    let e = parse_at(&watched, Level::Normal).unwrap_err();
    assert!(e.contains("extra"), "{e}");
    assert!(parse_at(&watched, Level::Hard).is_ok());
}

/// Two tanks of the player, an objective shown from the start, and
/// `extra` triggers before the warning `fewer than <count> tanks`.
fn warning_sim(count: u32, extra: &str) -> Sim {
    let text = format!(
        r#"
[mission]
title = "Test"
elimination = false

[[group]]
owner = 0
unit = "de_pz4_h"
count = 2
position_m = [0, 100]

[[group]]
owner = 1
unit = "su_t34_76_1943"
position_m = [-150, -150]
hold_fire = true

[[objective]]
id = "hold"
text = "Hold"
shown = true
{extra}
[[trigger]]
id = "lost"
when = "fewer"
unit = "de_pz4"
count = {count}
open = ["hold"]
actions = [{{ do = "hopeless", text = "Too few tanks left." }}]
"#
    );
    let config = parse_at(&text, Level::Normal).unwrap();
    let mut sim = Sim::new(&config, flat_map(), 3).unwrap();
    for _ in 0..40 {
        sim.step(&[]);
    }
    sim
}

#[test]
fn the_warning_comes_when_too_few_tanks_are_left_and_the_goal_is_open() {
    let sim = warning_sim(3, "");
    assert_eq!(sim.mission_hopeless(), Some(("Too few tanks left.", 1)));
    assert_eq!(sim.mission_message(), Some(("Too few tanks left.", 1)));
    // Two tanks are not fewer than two.
    assert_eq!(warning_sim(2, "").mission_hopeless(), None);
}

#[test]
fn the_warning_waits_for_an_open_goal_and_a_running_match() {
    let done = r#"
[[trigger]]
id = "done"
when = "time"
actions = [{ do = "objective", objective = "hold", state = "done" }]
"#;
    assert_eq!(warning_sim(3, done).mission_hopeless(), None);
    let won = r#"
[[trigger]]
id = "won"
when = "time"
actions = [{ do = "victory" }]
"#;
    assert_eq!(warning_sim(3, won).mission_hopeless(), None);
}

#[test]
fn easy_gives_more_start_resources() {
    let text = r#"
[mission]
title = "Test"
economy = true

[[group]]
owner = 0
unit = "de_pz4_h"
position_m = [0, 100]

[[group]]
owner = 1
unit = "su_t34_76_1943"
position_m = [-150, -150]

[difficulty.easy]
resources_pct = 150
"#;
    let account = |level| {
        let config = parse_at(text, level).unwrap();
        Sim::new(&config, flat_map(), 3)
            .unwrap()
            .account(P0)
            .unwrap()
    };
    let (easy, normal) = (account(Level::Easy), account(Level::Normal));
    println!("start resources: easy {easy:?}, normal {normal:?}");
    for k in 0..3 {
        assert_eq!(easy[k], normal[k] * 3 / 2);
    }
    // The enemy keeps the rules' start.
    let config = parse_at(text, Level::Easy).unwrap();
    let sim = Sim::new(&config, flat_map(), 3).unwrap();
    assert_eq!(sim.account(PlayerId(1)), Some(normal));
}
