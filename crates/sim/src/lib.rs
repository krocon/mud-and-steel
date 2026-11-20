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

pub mod air;
pub mod bridges;
pub mod care;
pub mod catalog;
pub mod combat;
mod command;
pub mod craters;
pub mod crossings;
mod data;
pub mod destruction;
pub mod economy;
pub mod experience;
pub mod fire;
pub mod formation;
mod hash;
pub mod indirect;
pub mod infantry;
pub mod logistics;
pub mod map;
mod math;
pub mod mission;
pub mod objects;
mod outcome;
pub mod path;
pub mod positions;
mod rng;
pub mod scale;
pub mod scenario;
mod sim;
pub mod snapshot;
pub mod stats;
pub mod strike;
pub mod tactics;
pub mod teams;
pub mod terrain;
pub mod transport;
pub mod unit_stats;
pub mod vision;
pub mod weather;
pub mod works;

pub use air::{
    AirBlock, AirEvent, AirMission, AirOption, AirRules, AirType, AircraftSnapshot, BombSnapshot,
    FlakClass, SortieSnapshot,
};
pub use bridges::Bridge;
pub use care::CareRules;
pub use catalog::Catalog;
pub use combat::{
    CombatEvent, CombatRules, CombatType, Face, FireMode, HitKind, MgMount, ProjectileSnapshot,
    Vec3,
};
pub use command::{Command, PlayerCommand, PlayerId, UnitId};
pub use craters::{CraterEvent, CraterRules};
pub use crossings::{
    CrossingEvent, CrossingRules, Gap, LayerState, LayerView, ParachuteView, SwimmerView,
};
pub use data::{DEFAULT_FOOTPRINT_MM, DataError, UnitType, unit_types_from_catalog};
pub use destruction::{
    DestructionRules, ObjectCause, ObjectEvent, ObjectStage, ObjectState, Remains,
};
pub use economy::{Cost, EconomyRules, RESOURCE_NAMES, Resources};
pub use experience::{ExperienceRules, RankRules};
pub use fire::{FireEvent, FireEventKind, FireRules, FireSource, FireView};
pub use formation::{Engage, Formation};
pub use hash::StableHasher;
pub use indirect::{IndirectRules, ShellSnapshot, SmokeSnapshot};
pub use infantry::{InfantryRules, Stance, StanceOrder};
pub use logistics::{LogisticsRules, LogisticsType};
pub use map::{Biome, Map, MapError, MapRef};
pub use math::{Fixed, Vec2};
pub use mission::{Mission, MissionError, ObjectiveState};
pub use objects::{Blocks, MapObject, ObjectKind, ObjectRules};
pub use outcome::{DefeatReason, Outcome, PlayerStanding};
pub use positions::{LayoutPiece, PositionRules, strongpoint_layout};
pub use rng::Rng;
pub use scale::Scale;
pub use sim::{
    AidView, BuildingCondition, BuyOption, CareEvent, ConfigError, DrillView, ExperienceView,
    GroundInfo, MAX_PLAYERS, MAX_QUEUED, ObjectiveView, Phase, PlaceError, RouteView, Sim,
    SimConfig, Snapshot, SoldierSnapshot, Spawn, SquadTactics, StockView, StructureView,
    SupplySource, SupplyView, UnitSnapshot, WORLD_LIMIT_M, WoundView, formation_slots,
    formation_spacing, soldier_flags,
};
pub use strike::{StrikeBlock, StrikeEvent, StrikeOption, StrikeRules, StrikeView};
pub use tactics::TacticsRules;
pub use teams::Teams;
pub use terrain::{TerrainError, TerrainRules};
pub use transport::{TransportRules, TransportType};
pub use vision::{CellState, VisionGrid, VisionRules};
pub use weather::{Daytime, Ground, Weather, WeatherRules, WeatherSetup};
pub use works::{DigKind, LayKind, MineKind, MineView, WorkKind, WorkView, WorksRules};
