//! Deterministic game simulation for Mud and Steel.
//!
//! This crate is the single source of truth for game state. It must produce
//! bit-identical results on every platform, so it follows strict rules
//! (see `docs/DETERMINISM.md`):
//!
//! - no floating-point arithmetic (fixed-point [`Fixed`] instead),
//! - no OS entropy or wall-clock time (seeded [`Rng`], tick counter),
//! - no iteration over hash-based collections,
//! - no Godot dependency.

pub mod catalog;
mod command;
mod data;
mod hash;
pub mod map;
mod math;
pub mod path;
mod rng;
pub mod scale;
pub mod scenario;
mod sim;
pub mod terrain;

pub use catalog::Catalog;
pub use command::{Command, PlayerCommand, PlayerId, UnitId};
pub use data::{DEFAULT_FOOTPRINT_MM, DataError, UnitType, unit_types_from_catalog};
pub use hash::StableHasher;
pub use map::{Map, MapError, MapRef};
pub use math::{Fixed, Vec2};
pub use rng::Rng;
pub use scale::Scale;
pub use sim::{
    ConfigError, RouteView, Sim, SimConfig, Snapshot, Spawn, UnitSnapshot, WORLD_LIMIT_M,
    formation_slots, formation_spacing,
};
pub use terrain::{TerrainError, TerrainRules};
