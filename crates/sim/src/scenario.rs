//! A fixed demo scenario with a scripted command stream.
//!
//! Used by the headless CLI, the golden-hash test and the network tests, so
//! all of them exercise exactly the same input.

use crate::command::{Command, UnitId};
use crate::data::UnitType;
use crate::math::Vec2;
use crate::rng::Rng;
use crate::sim::{Sim, SimConfig, Spawn};

/// Ticks between two scripted commands.
pub const COMMAND_INTERVAL: u64 = 40;

/// Four placeholder tanks in a square around the origin.
pub fn demo_config() -> SimConfig {
    let name = "placeholder_tank";
    SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types: vec![UnitType {
            name: name.into(),
            speed_mm_per_s: 4000,
        }],
        spawns: [(-10, -10), (10, -10), (-10, 10), (10, 10)]
            .into_iter()
            .map(|(x, y)| Spawn {
                unit_type: name.into(),
                position: Vec2::from_meters(x, y),
            })
            .collect(),
    }
}

/// Deterministic command generator: every [`COMMAND_INTERVAL`] ticks it sends
/// the next unit (round robin) to a random point within ±50 m.
#[derive(Clone, Debug)]
pub struct CommandScript {
    rng: Rng,
    unit_count: u32,
}

impl CommandScript {
    /// The script uses its own generator, derived from `seed`, so it never
    /// disturbs the simulation's generator.
    pub fn new(seed: u64, unit_count: u32) -> Self {
        Self {
            rng: Rng::new(seed ^ 0xC0FF_EE00_5C21_4700),
            unit_count,
        }
    }

    pub fn commands_for_tick(&mut self, tick: u64) -> Vec<Command> {
        if self.unit_count == 0 || !tick.is_multiple_of(COMMAND_INTERVAL) {
            return Vec::new();
        }
        let round = tick / COMMAND_INTERVAL;
        let unit = UnitId(u32::try_from(round % u64::from(self.unit_count)).expect("< unit_count"));
        let target = Vec2::from_millimeters(
            self.rng.range_i32(-50_000, 50_000),
            self.rng.range_i32(-50_000, 50_000),
        );
        vec![Command::MoveTo { unit, target }]
    }
}

/// Runs the demo scenario for `ticks` ticks and returns the final simulation.
pub fn run_demo(seed: u64, ticks: u64) -> Sim {
    let config = demo_config();
    let mut sim = Sim::new(&config, seed).expect("demo config is valid");
    let unit_count = u32::try_from(config.spawns.len()).expect("few units");
    let mut script = CommandScript::new(seed, unit_count);
    for tick in 0..ticks {
        let commands = script.commands_for_tick(tick);
        sim.step(&commands);
    }
    sim
}
