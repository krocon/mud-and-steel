//! A replay of a match decided by surrender re-simulates to the same result.

use net::{Replay, ReplayRecorder};
use sim::scenario::{builtin_map, demo_config};
use sim::{Command, PlayerCommand, PlayerId, Sim, UnitId, Vec2};

#[test]
fn surrendered_match_replays_to_the_same_result() {
    let config = demo_config();
    let players = config.players();
    let mut sim = Sim::new(&config, builtin_map(), 9).unwrap();
    let mut recorder = ReplayRecorder::new(9, config, &players);
    for tick in 0..200u32 {
        let commands = match tick {
            20 => vec![PlayerCommand::new(
                PlayerId(0),
                Command::MoveTo {
                    units: vec![UnitId(0), UnitId(1)],
                    target: Vec2::from_meters(20, 20),
                },
            )],
            60 => vec![PlayerCommand::new(
                PlayerId(1),
                Command::RemoveUnits {
                    units: vec![UnitId(8), UnitId(9)],
                },
            )],
            150 => vec![PlayerCommand::new(PlayerId(1), Command::Surrender)],
            _ => Vec::new(),
        };
        recorder.record(&commands);
        sim.step(&commands);
    }
    assert_eq!(sim.outcome().unwrap().winner, Some(PlayerId(0)));

    let replay = Replay::decode(&recorder.finish(&sim).encode()).unwrap();
    let report = replay.verify(builtin_map()).unwrap();
    assert!(report.matches(), "{report:?}");
    let replayed = replay.play(builtin_map()).unwrap();
    assert_eq!(replayed.outcome(), sim.outcome());
    assert_eq!(replayed.living_units(PlayerId(1)), 6);
}
