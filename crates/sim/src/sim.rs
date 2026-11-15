//! The simulation state and its fixed-tick update.

use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Move targets farther than this from the origin (in meters, per axis) are
/// ignored. Keeps all position arithmetic far away from `Fixed` overflow, even
/// for forged commands from a remote peer.
pub const WORLD_LIMIT_M: i32 = 1_000_000;

/// A unit placed at simulation start.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spawn {
    pub owner: PlayerId,
    /// Name of a [`UnitType`] in [`SimConfig::unit_types`].
    pub unit_type: String,
    pub position: Vec2,
}

/// Everything needed to start a match. Identical on all peers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimConfig {
    /// Simulation ticks per second.
    pub tick_hz: u32,
    pub unit_types: Vec<UnitType>,
    pub spawns: Vec<Spawn>,
}

impl SimConfig {
    pub const DEFAULT_TICK_HZ: u32 = 20;

    /// All players that own a unit, sorted and deduplicated.
    pub fn players(&self) -> Vec<PlayerId> {
        let mut players: Vec<PlayerId> = self.spawns.iter().map(|s| s.owner).collect();
        players.sort_unstable();
        players.dedup();
        players
    }
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
    owner: PlayerId,
    unit_type: u32,
    position: Vec2,
    target: Option<Vec2>,
    /// Meters per tick.
    speed: Fixed,
    /// Meters, see [`UnitType::footprint_m`].
    footprint: Fixed,
}

/// Presentation data of one unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitSnapshot {
    pub id: UnitId,
    pub owner: PlayerId,
    /// Index into [`SimConfig::unit_types`].
    pub unit_type: u32,
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

        let mut units = Vec::with_capacity(config.spawns.len());
        for (index, spawn) in (0u32..).zip(&config.spawns) {
            let (type_index, unit_type) = (0u32..)
                .zip(&config.unit_types)
                .find(|(_, t)| t.name == spawn.unit_type)
                .ok_or_else(|| ConfigError::UnknownUnitType(spawn.unit_type.clone()))?;
            units.push(Unit {
                id: UnitId(index),
                owner: spawn.owner,
                unit_type: type_index,
                position: spawn.position,
                target: None,
                speed: unit_type.speed_m_per_s / ticks_per_second,
                footprint: unit_type.footprint_m,
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
    /// Commands are applied in the given order before movement (the lockstep
    /// layer sorts them by player, then issue order).
    pub fn step(&mut self, commands: &[PlayerCommand]) {
        for command in commands {
            self.apply(command);
        }
        for unit in &mut self.units {
            move_unit(unit);
        }
        self.tick += 1;
    }

    fn apply(&mut self, command: &PlayerCommand) {
        match &command.command {
            Command::MoveTo { units, target } => {
                if !within_world(*target) {
                    return;
                }
                let mut group = self.owned_indices(command.player, units);
                if group.is_empty() {
                    return;
                }
                let footprint = group
                    .iter()
                    .map(|&i| self.units[i].footprint)
                    .max()
                    .unwrap_or(Fixed::ZERO);
                let slots = formation_slots(*target, group.len(), formation_spacing(footprint));
                // Slots are ordered row by row (y, then x). Assigning units in
                // the same order of their current position keeps paths from
                // crossing in common cases and is independent of the order of
                // ids in the command.
                group.sort_by_key(|&i| {
                    let u = &self.units[i];
                    (u.position.y, u.position.x, u.id)
                });
                for (index, slot) in group.into_iter().zip(slots) {
                    self.units[index].target = Some(slot);
                }
            }
            Command::Stop { units } => {
                for index in self.owned_indices(command.player, units) {
                    self.units[index].target = None;
                }
            }
        }
    }

    /// Indices into `self.units` of the listed units that exist and belong to
    /// `player`, sorted and without duplicates.
    fn owned_indices(&self, player: PlayerId, ids: &[UnitId]) -> Vec<usize> {
        let mut indices: Vec<usize> = ids
            .iter()
            .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
            .filter(|&index| self.units[index].owner == player)
            .collect();
        indices.sort_unstable();
        indices.dedup();
        indices
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
            h.write_u8(unit.owner.0);
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
            h.write_i64(unit.footprint.to_bits());
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
                    owner: u.owner,
                    unit_type: u.unit_type,
                    position: u.position,
                    target: u.target,
                })
                .collect(),
        }
    }
}

fn within_world(point: Vec2) -> bool {
    let limit = Fixed::from_num(WORLD_LIMIT_M);
    point.x.abs() <= limit && point.y.abs() <= limit
}

/// Distance between the centers of neighboring formation slots: the
/// footprint plus half of it as gap.
pub fn formation_spacing(footprint: Fixed) -> Fixed {
    footprint + footprint / 2
}

/// Target points of a square grid formation for `count` units around
/// `center`, row by row (ascending y, then ascending x).
///
/// The grid has `ceil(sqrt(count))` columns; every row, including a shorter
/// last one, is centered on `center`.
pub fn formation_slots(center: Vec2, count: usize, spacing: Fixed) -> Vec<Vec2> {
    let mut columns = 1;
    while columns * columns < count {
        columns += 1;
    }
    let rows = count.div_ceil(columns);
    let half = spacing / 2;
    // Offset of slot `index` in a line of `len` slots, in units of `half`:
    // 2 * index - (len - 1), symmetric around zero.
    let offset = |index: usize, len: usize| -> Fixed {
        let steps =
            i64::try_from(2 * index).unwrap_or(i64::MAX) - i64::try_from(len).unwrap_or(0) + 1;
        half * Fixed::from_num(steps)
    };
    let mut slots = Vec::with_capacity(count);
    for row in 0..rows {
        let in_row = columns.min(count - row * columns);
        let y = center.y + offset(row, rows);
        for column in 0..in_row {
            slots.push(Vec2::new(center.x + offset(column, in_row), y));
        }
    }
    slots
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

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    fn tank() -> UnitType {
        UnitType {
            name: "tank".into(),
            speed_m_per_s: Fixed::from_num(4),
            footprint_m: Fixed::from_num(6),
        }
    }

    /// Units 0..=2 belong to player 0, unit 3 to player 1.
    fn config() -> SimConfig {
        let spawn = |owner, x, y| Spawn {
            owner,
            unit_type: "tank".into(),
            position: Vec2::from_meters(x, y),
        };
        SimConfig {
            tick_hz: 20,
            unit_types: vec![tank()],
            spawns: vec![
                spawn(P0, 0, 0),
                spawn(P0, 10, 0),
                spawn(P0, 20, 0),
                spawn(P1, 0, 50),
            ],
        }
    }

    fn move_to(player: PlayerId, units: &[u32], x: i32, y: i32) -> PlayerCommand {
        PlayerCommand::new(
            player,
            Command::MoveTo {
                units: units.iter().copied().map(UnitId).collect(),
                target: Vec2::from_meters(x, y),
            },
        )
    }

    fn run(sim: &mut Sim, ticks: u32) {
        for _ in 0..ticks {
            sim.step(&[]);
        }
    }

    #[test]
    fn unit_reaches_target_and_stops() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[move_to(P0, &[0], 3, 4)]);
        // 5 m at 4 m/s and 20 Hz takes 25 ticks.
        run(&mut sim, 30);
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.position, Vec2::from_meters(3, 4));
        assert_eq!(unit.target, None);
    }

    #[test]
    fn unit_moves_by_speed_per_tick() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[move_to(P0, &[0], 10, 0)]);
        let x = sim.snapshot().units[0].position.x;
        let expected = Fixed::from_num(4) / Fixed::from_num(20);
        assert!((x - expected).abs() <= Fixed::DELTA * 4, "x = {x}");
    }

    #[test]
    fn unknown_unit_is_ignored() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        let before = sim.state_hash();
        let mut reference = Sim::new(&config(), 1).unwrap();
        sim.step(&[move_to(P0, &[99], 1, 1)]);
        reference.step(&[]);
        assert_ne!(before, sim.state_hash());
        assert_eq!(sim.state_hash(), reference.state_hash());
    }

    #[test]
    fn commands_for_foreign_units_are_ignored() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        let mut reference = Sim::new(&config(), 1).unwrap();
        sim.step(&[
            move_to(P1, &[0, 1, 2], 100, 100),
            PlayerCommand::new(P0, Command::Stop { units: vec![] }),
        ]);
        reference.step(&[]);
        assert_eq!(sim.state_hash(), reference.state_hash());

        // Mixed list: only the own unit moves.
        sim.step(&[move_to(P1, &[2, 3], 0, 100)]);
        let units = sim.snapshot().units;
        assert_eq!(units[2].target, None);
        assert_eq!(units[3].target, Some(Vec2::from_meters(0, 100)));
    }

    #[test]
    fn group_gets_distinct_slots_with_spacing() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[move_to(P0, &[0, 1, 2], 100, 100)]);
        let targets: Vec<Vec2> = sim.snapshot().units[..3]
            .iter()
            .map(|u| u.target.unwrap())
            .collect();
        // Three units: 2 columns, 2 rows, spacing 6 m * 1.5 = 9 m.
        assert_eq!(
            targets,
            vec![
                Vec2::new(Fixed::from_num(95.5), Fixed::from_num(95.5)),
                Vec2::new(Fixed::from_num(104.5), Fixed::from_num(95.5)),
                Vec2::new(Fixed::from_num(100), Fixed::from_num(104.5)),
            ]
        );
    }

    #[test]
    fn formation_is_independent_of_command_order() {
        let mut a = Sim::new(&config(), 1).unwrap();
        let mut b = Sim::new(&config(), 1).unwrap();
        a.step(&[move_to(P0, &[0, 1, 2], -40, 70)]);
        b.step(&[move_to(P0, &[2, 0, 1, 2, 0], -40, 70)]);
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn stop_clears_the_target() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[move_to(P0, &[0, 1], 0, 100)]);
        run(&mut sim, 10);
        sim.step(&[PlayerCommand::new(
            P0,
            Command::Stop {
                units: vec![UnitId(0)],
            },
        )]);
        let stopped = sim.snapshot().units[0];
        assert_eq!(stopped.target, None);
        run(&mut sim, 10);
        assert_eq!(sim.snapshot().units[0].position, stopped.position);
        assert!(sim.snapshot().units[1].target.is_some());
    }

    #[test]
    fn targets_outside_the_world_are_ignored() {
        let mut sim = Sim::new(&config(), 1).unwrap();
        sim.step(&[PlayerCommand::new(
            P0,
            Command::MoveTo {
                units: vec![UnitId(0)],
                target: Vec2::new(Fixed::MAX, Fixed::ZERO),
            },
        )]);
        assert_eq!(sim.snapshot().units[0].target, None);
    }

    #[test]
    fn formation_slots_center_every_row() {
        let slots = formation_slots(Vec2::ZERO, 5, Fixed::from_num(2));
        let meters: Vec<(i32, i32)> = slots.iter().map(|s| (s.x.to_num(), s.y.to_num())).collect();
        assert_eq!(meters, vec![(-2, -1), (0, -1), (2, -1), (-1, 1), (1, 1)]);
        assert_eq!(
            formation_slots(Vec2::ZERO, 1, Fixed::from_num(2)),
            vec![Vec2::ZERO]
        );
        assert!(formation_slots(Vec2::ZERO, 0, Fixed::ONE).is_empty());
    }

    #[test]
    fn players_are_collected_from_spawns() {
        assert_eq!(config().players(), vec![P0, P1]);
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
