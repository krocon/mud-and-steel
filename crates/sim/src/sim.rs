//! The simulation state and its fixed-tick update.

use crate::command::{Command, UnitId};
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::rng::Rng;
use std::fmt;

/// A unit placed at simulation start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spawn {
    /// Name of a [`UnitType`] in [`SimConfig::unit_types`].
    pub unit_type: String,
    pub position: Vec2,
}

/// Everything needed to start a match. Identical on all peers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimConfig {
    /// Simulation ticks per second.
    pub tick_hz: u32,
    pub unit_types: Vec<UnitType>,
    pub spawns: Vec<Spawn>,
}

impl SimConfig {
    pub const DEFAULT_TICK_HZ: u32 = 20;
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    ZeroTickRate,
    UnknownUnitType(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTickRate => write!(f, "tick_hz must be positive"),
            Self::UnknownUnitType(name) => write!(f, "unknown unit type '{name}'"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Debug)]
struct Unit {
    id: UnitId,
    unit_type: u32,
    position: Vec2,
    target: Option<Vec2>,
    /// Meters per tick.
    speed: Fixed,
}

/// Presentation data of one unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitSnapshot {
    pub id: UnitId,
    pub position: Vec2,
    pub target: Option<Vec2>,
}

/// Presentation data of the whole simulation after a tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Number of ticks simulated so far.
    pub tick: u64,
    /// Sorted by unit id.
    pub units: Vec<UnitSnapshot>,
}

/// The deterministic simulation.
#[derive(Clone, Debug)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    /// Sorted by id; ids are assigned in increasing order.
    units: Vec<Unit>,
}

impl Sim {
    pub fn new(config: &SimConfig, seed: u64) -> Result<Self, ConfigError> {
        if config.tick_hz == 0 {
            return Err(ConfigError::ZeroTickRate);
        }
        let ticks_per_second = Fixed::from_num(config.tick_hz);
        let mm_per_m = Fixed::from_num(1000);

        let mut units = Vec::with_capacity(config.spawns.len());
        for (index, spawn) in (0u32..).zip(&config.spawns) {
            let (type_index, unit_type) = (0u32..)
                .zip(&config.unit_types)
                .find(|(_, t)| t.name == spawn.unit_type)
                .ok_or_else(|| ConfigError::UnknownUnitType(spawn.unit_type.clone()))?;
            units.push(Unit {
                id: UnitId(index),
                unit_type: type_index,
                position: spawn.position,
                target: None,
                speed: Fixed::from_num(unit_type.speed_mm_per_s) / mm_per_m / ticks_per_second,
            });
        }

        Ok(Self {
            tick: 0,
            rng: Rng::new(seed),
            units,
        })
    }

    /// Number of ticks simulated so far.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Advances the simulation by exactly one tick.
    ///
    /// Commands are applied in the given order before movement. Commands that
    /// refer to unknown units are ignored.
    pub fn step(&mut self, commands: &[Command]) {
        for command in commands {
            self.apply(command);
        }
        for unit in &mut self.units {
            move_unit(unit);
        }
        self.tick += 1;
    }

    fn apply(&mut self, command: &Command) {
        match *command {
            Command::MoveTo { unit, target } => {
                if let Some(unit) = self.unit_mut(unit) {
                    unit.target = Some(target);
                }
            }
        }
    }

    fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units
            .binary_search_by_key(&id, |u| u.id)
            .ok()
            .map(|index| &mut self.units[index])
    }

    /// Stable hash over the complete simulation state.
    ///
    /// Equal on all peers if and only if they stayed in sync (with overwhelming
    /// probability). The byte layout fed into the hasher is part of the
    /// lockstep protocol.
    pub fn state_hash(&self) -> u64 {
        let mut h = StableHasher::new();
        h.write_u64(self.tick);
        h.write_u64(self.rng.state());
        h.write_u64(self.units.len() as u64);
        for unit in &self.units {
            h.write_u32(unit.id.0);
            h.write_u32(unit.unit_type);
            h.write_i64(unit.position.x.to_bits());
            h.write_i64(unit.position.y.to_bits());
            match unit.target {
                None => h.write_u8(0),
                Some(target) => {
                    h.write_u8(1);
                    h.write_i64(target.x.to_bits());
                    h.write_i64(target.y.to_bits());
                }
            }
            h.write_i64(unit.speed.to_bits());
        }
        h.finish()
    }

    /// Data needed to draw the current state.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            tick: self.tick,
            units: self
                .units
                .iter()
                .map(|u| UnitSnapshot {
                    id: u.id,
                    position: u.position,
                    target: u.target,
                })
                .collect(),
        }
    }
}

/// Moves a unit one tick along the straight line to its target.
fn move_unit(unit: &mut Unit) {
    let Some(target) = unit.target else {
        return;
    };
    let delta = target - unit.position;
    let distance = delta.length();
    if distance <= unit.speed {
        unit.position = target;
        unit.target = None;
    } else {
        // delta / distance has components in [-1, 1], so this cannot overflow.
        let direction = Vec2::new(delta.x / distance, delta.y / distance);
        unit.position = unit.position + direction.scale(unit.speed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SimConfig {
        SimConfig {
            tick_hz: 20,
            unit_types: vec![UnitType {
                name: "tank".into(),
                speed_mm_per_s: 4000,
            }],
            spawns: vec![Spawn {
                unit_type: "tank".into(),
                position: Vec2::ZERO,
            }],
        }
    }

    #[test]
    fn unit_reaches_target_and_stops() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        let target = Vec2::from_meters(3, 4);
        sim.step(&[Command::MoveTo {
            unit: UnitId(0),
            target,
        }]);
        // 5 m at 4 m/s and 20 Hz takes 25 ticks.
        for _ in 0..30 {
            sim.step(&[]);
        }
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.position, target);
        assert_eq!(unit.target, None);
    }

    #[test]
    fn unit_moves_by_speed_per_tick() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[Command::MoveTo {
            unit: UnitId(0),
            target: Vec2::from_meters(10, 0),
        }]);
        let x = sim.snapshot().units[0].position.x;
        let expected = Fixed::from_num(4) / Fixed::from_num(20);
        assert!((x - expected).abs() <= Fixed::DELTA * 4, "x = {x}");
    }

    #[test]
    fn unknown_unit_is_ignored() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        let before = sim.state_hash();
        let mut reference = Sim::new(&config(), 1).unwrap();
        sim.step(&[Command::MoveTo {
            unit: UnitId(99),
            target: Vec2::from_meters(1, 1),
        }]);
        reference.step(&[]);
        assert_ne!(before, sim.state_hash());
        assert_eq!(sim.state_hash(), reference.state_hash());
    }

    #[test]
    fn rejects_unknown_unit_type() {
        let mut cfg = config();
        cfg.spawns[0].unit_type = "nope".into();
        assert_eq!(
            Sim::new(&cfg, 0).unwrap_err(),
            ConfigError::UnknownUnitType("nope".into())
        );
    }
}
