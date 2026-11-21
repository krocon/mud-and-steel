//! Mission conditions of prompt 180: `stance`, `garrison`, `spotted`,
//! `unseen` and `air`. (`pinned` and `smoke` are played through by the
//! tutorial chapters in `crates/ai/tests/missions.rs`.)

use sim::map::MapLayout;
use sim::mission::parse_mission;
use sim::{
    AirMission, Catalog, Command, Map, ObjectiveState, PlayerCommand, PlayerId, Scale, Sim, Stance,
    StanceOrder, TerrainRules, UnitId, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);

/// 400 m × 400 m, flat, with a farm house at (0, -60).
fn map() -> Arc<Map> {
    let n: usize = 201;
    let layout = MapLayout {
        name: "trigger_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, vec![0; n * n], vec![0; n * n]).unwrap();
    map.add_objects(&format!(
        "{}\nenv_house_farm,Intact,0,-60000,0\n",
        sim::objects::CSV_HEADER
    ))
    .unwrap();
    Arc::new(map)
}

/// A rifle squad of the player at (0, 0), an enemy squad at `enemy`, and
/// one trigger `extra` that completes the objective `goal`.
fn start(enemy: [i32; 2], trigger: &str) -> Sim {
    let text = format!(
        r#"
[mission]
title = "Test"
elimination = false

[[group]]
name = "rifles"
owner = 0
unit = "de_inf_rifle_squad"
position_m = [0, 0]
hold_fire = true

[[group]]
name = "enemy"
owner = 1
unit = "su_inf_rifle_squad"
position_m = [{}, {}]
hold_fire = true

[[area]]
name = "house"
center_m = [0, -60]
radius_m = 20

[[objective]]
id = "goal"
text = "Goal"
shown = true

[[trigger]]
id = "goal"
{trigger}
actions = [{{ do = "objective", objective = "goal", state = "done" }}]
"#,
        enemy[0], enemy[1]
    );
    let map = map();
    let config = parse_mission(
        "test",
        &text,
        None,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        &map,
    )
    .unwrap();
    Sim::new(&config, map, 3).unwrap()
}

fn done(sim: &Sim) -> bool {
    sim.mission_objectives()[0].state == ObjectiveState::Done
}

fn run(sim: &mut Sim, seconds: u32, commands: &[Command]) {
    let commands: Vec<PlayerCommand> = commands
        .iter()
        .map(|c| PlayerCommand::new(P0, c.clone()))
        .collect();
    sim.step(&commands);
    for _ in 1..20 * seconds {
        sim.step(&[]);
    }
}

#[test]
fn stance_counts_once_the_squad_lies() {
    let mut sim = start(
        [180, 180],
        "when = \"stance\"\nstance = \"prone\"\nunit = \"de_inf\"",
    );
    run(&mut sim, 2, &[]);
    assert!(!done(&sim));
    run(
        &mut sim,
        10,
        &[Command::SetStance {
            units: vec![UnitId(0)],
            stance: StanceOrder::Fixed(Stance::Prone),
        }],
    );
    assert!(done(&sim), "prone squad not counted");
    // A wrong stance name is an error.
    let bad = std::panic::catch_unwind(|| start([180, 180], "when = \"stance\"\nstance = \"sit\""));
    assert!(bad.is_err());
}

#[test]
fn garrison_counts_a_squad_in_a_house_of_the_area() {
    let mut sim = start([180, 180], "when = \"garrison\"\narea = \"house\"");
    run(&mut sim, 2, &[]);
    assert!(!done(&sim));
    let building = sim
        .map()
        .objects()
        .iter()
        .position(|o| o.position == Vec2::from_meters(0, -60))
        .unwrap() as u32;
    run(
        &mut sim,
        60,
        &[Command::Garrison {
            units: vec![UnitId(0)],
            building,
        }],
    );
    assert!(done(&sim), "garrisoned squad not counted");
}

#[test]
fn spotted_and_unseen_follow_vision() {
    // Near: the player sees the enemy, and the enemy sees the player.
    let mut near = start([0, 40], "when = \"spotted\"\ngroup = \"enemy\"");
    run(&mut near, 3, &[]);
    assert!(done(&near), "a squad 40 m away not spotted");
    let mut seen = start([0, 40], "when = \"unseen\"\nby = 1");
    run(&mut seen, 3, &[]);
    assert!(
        !done(&seen),
        "a squad 40 m from the enemy counted as unseen"
    );
    // Far beyond sight.
    let mut far = start([190, 190], "when = \"spotted\"\ngroup = \"enemy\"");
    run(&mut far, 3, &[]);
    assert!(!done(&far));
    let mut unseen = start([190, 190], "when = \"unseen\"\nby = 1");
    run(&mut unseen, 3, &[]);
    assert!(done(&unseen));
}

#[test]
fn air_counts_a_call_of_the_player() {
    let mut sim = start([180, 180], "when = \"air\"");
    run(&mut sim, 2, &[]);
    assert!(!done(&sim));
    let calls: Vec<Command> = AirMission::ALL
        .into_iter()
        .map(|mission| Command::CallAir {
            mission,
            target: Vec2::from_meters(180, 180),
        })
        .collect();
    run(&mut sim, 1, &calls);
    assert!(done(&sim), "no air call counted");
}
