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
mod math;
mod rng;
pub mod scale;
pub mod scenario;
mod sim;

pub use catalog::Catalog;
pub use command::{Command, UnitId};
pub use data::{DataError, UnitType, unit_types_from_catalog};
pub use hash::StableHasher;
pub use math::{Fixed, Vec2};
pub use rng::Rng;
pub use scale::Scale;
pub use sim::{ConfigError, Sim, SimConfig, Snapshot, Spawn, UnitSnapshot};
