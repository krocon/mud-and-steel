//! Mission building blocks of prompt 189: the condition `reach` (units
//! that reached an area count even after they left it), timers and counts
//! shown with an objective, optional objectives and the rating of a won
//! mission (`[rating]`).

use sim::map::MapLayout;
use sim::mission::parse_mission;
use sim::{
    Catalog, Command, Map, ObjectiveState, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    TerrainRules, UnitId, Vec2,
};
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

fn parse(text: &str) -> Result<SimConfig, String> {
    parse_mission(
        "test",
        text,
        None,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &flat_map(),
    )
    .map_err(|e| e.to_string())
}

/// Three trucks drive through the area `exit` (one by one, to a point
/// behind it); two must have reached it. A timer of 60 s; an optional
/// objective done when the trucks are out, one failed on the timer.
const SCRIPT: &str = r#"
[mission]
title = "Test"
elimination = false

[[group]]
name = "trucks"
owner = 0
unit = "de_opel_blitz"
count = 3
position_m = [0, 120]

[[group]]
name = "enemy"
owner = 1
unit = "su_t34_76_1943"
position_m = [-150, -150]
hold_fire = true

[[area]]
name = "exit"
center_m = [0, 0]
radius_m = 15

[[objective]]
id = "out"
text = "Bring two trucks out"
area = "exit"
shown = true
progress = "escaped"

[[objective]]
id = "clock"
text = "Hold out"
shown = true
timer = "clock"

[[objective]]
id = "side"
text = "All trucks out"
shown = true
optional = true

[[objective]]
id = "side2"
text = "Something else"
shown = true
optional = true

[[trigger]]
id = "escaped"
when = "reach"
area = "exit"
unit = "de_opel"
count = 2
actions = [{ do = "objective", objective = "out", state = "done" }]

[[trigger]]
id = "all_out"
when = "reach"
area = "exit"
count = 3
actions = [{ do = "objective", objective = "side", state = "done" }]

[[trigger]]
id = "clock"
when = "time"
delay_s = 60
actions = [{ do = "objective", objective = "clock", state = "done" }, { do = "objective", objective = "side2", state = "failed" }]

[[trigger]]
id = "won"
after = ["escaped", "clock"]
when = "time"
actions = [{ do = "victory" }]

[rating]
par_s = 300
losses_pct = 0
"#;

fn objective(sim: &Sim, id: &str) -> sim::ObjectiveView {
    sim.mission_objectives()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
}

#[test]
fn units_that_reached_an_area_count_and_the_mission_is_rated() {
    let config = parse(SCRIPT).unwrap();
    let mut sim = Sim::new(&config, flat_map(), 1).unwrap();
    sim.step(&[]);
    assert_eq!(objective(&sim, "out").progress, Some((0, 2)));
    let timer = objective(&sim, "clock").timer_ticks.unwrap();
    assert!((1100..=1200).contains(&timer), "timer {timer}");
    assert!(objective(&sim, "side").optional);
    assert_eq!(sim.mission_rating(), None);
    // Every truck drives through the area to a point behind it.
    let to = Vec2::from_meters(0, -120);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: (0..3).map(UnitId).collect(),
            target: to,
        },
    )]);
    let mut seen = 0;
    for _ in 0..20 * 120 {
        sim.step(&[]);
        if let Some((n, _)) = objective(&sim, "out").progress {
            assert!(n >= seen, "the count never drops");
            seen = n;
        }
        if sim.outcome().is_some() {
            break;
        }
    }
    assert_eq!(objective(&sim, "out").state, ObjectiveState::Done);
    assert_eq!(objective(&sim, "out").progress, Some((2, 2)));
    assert_eq!(objective(&sim, "side").state, ObjectiveState::Done);
    assert_eq!(objective(&sim, "clock").timer_ticks, None);
    // Nobody stands in the area any more: `reach` remembers.
    let snapshot = sim.snapshot();
    assert!(
        snapshot
            .units
            .iter()
            .filter(|u| u.owner == P0)
            .all(|u| (u.position - Vec2::ZERO).length() > sim::Fixed::from_num(15))
    );
    let outcome = sim.outcome().expect("won");
    assert_eq!(outcome.winner, Some(P0));
    let r = sim.mission_rating().unwrap();
    assert!(r.in_time && r.few_losses);
    assert!(!r.optional_done, "side2 failed");
    assert_eq!(r.optional, (1, 2));
    assert_eq!((r.lost, r.units), (0, 3));
    assert_eq!(r.stars, 2);
}

#[test]
fn the_same_mission_hashes_the_same_twice() {
    let run = || {
        let config = parse(SCRIPT).unwrap();
        let mut sim = Sim::new(&config, flat_map(), 1).unwrap();
        sim.step(&[PlayerCommand::new(
            P0,
            Command::MoveTo {
                units: (0..3).map(UnitId).collect(),
                target: Vec2::from_meters(0, -120),
            },
        )]);
        for _ in 0..20 * 40 {
            sim.step(&[]);
        }
        sim.state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn bad_timers_and_counts_are_errors() {
    let bad = SCRIPT.replace("timer = \"clock\"", "timer = \"nope\"");
    assert!(parse(&bad).unwrap_err().contains("unknown trigger 'nope'"));
    let bad = SCRIPT.replace("progress = \"escaped\"", "progress = \"nope\"");
    assert!(parse(&bad).is_err());
    let bad = SCRIPT.replace("par_s = 300", "par = 300");
    assert!(parse(&bad).is_err());
    let bad = SCRIPT.replace(
        "when = \"reach\"\narea = \"exit\"\nunit",
        "when = \"reach\"\nunit",
    );
    assert!(parse(&bad).is_err(), "reach needs an area");
}
