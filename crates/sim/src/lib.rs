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

pub mod bridges;
pub mod catalog;
pub mod combat;
mod command;
pub mod craters;
mod data;
pub mod destruction;
pub mod economy;
mod hash;
pub mod indirect;
pub mod infantry;
pub mod map;
mod math;
pub mod objects;
mod outcome;
pub mod path;
mod rng;
pub mod scale;
pub mod scenario;
mod sim;
pub mod snapshot;
pub mod terrain;
pub mod vision;

pub use bridges::Bridge;
pub use catalog::Catalog;
pub use combat::{
    CombatEvent, CombatRules, CombatType, Face, FireMode, HitKind, ProjectileSnapshot, Vec3,
};
pub use command::{Command, PlayerCommand, PlayerId, UnitId};
pub use craters::{CraterEvent, CraterRules};
pub use data::{DEFAULT_FOOTPRINT_MM, DataError, UnitType, unit_types_from_catalog};
pub use destruction::{
    DestructionRules, ObjectCause, ObjectEvent, ObjectStage, ObjectState, Remains,
};
pub use economy::{Cost, EconomyRules, RESOURCE_NAMES, Resources};
pub use hash::StableHasher;
pub use indirect::{IndirectRules, ShellSnapshot, SmokeSnapshot};
pub use infantry::{InfantryRules, Stance, StanceOrder};
pub use map::{Biome, Map, MapError, MapRef};
pub use math::{Fixed, Vec2};
pub use objects::{Blocks, MapObject, ObjectKind, ObjectRules};
pub use outcome::{DefeatReason, Outcome, PlayerStanding};
pub use rng::Rng;
pub use scale::Scale;
pub use sim::{
    BuildingCondition, BuyOption, ConfigError, MAX_PLAYERS, Phase, PlaceError, RouteView, Sim,
    SimConfig, Snapshot, SoldierSnapshot, Spawn, StructureView, SupplyView, UnitSnapshot,
    WORLD_LIMIT_M, formation_slots, formation_spacing,
};
pub use terrain::{TerrainError, TerrainRules};
pub use vision::{CellState, VisionGrid, VisionRules};
