//! The simulation state and its fixed-tick update.

use crate::combat::{CombatEvent, CombatRules, FireMode, ProjectileSnapshot};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::map::{Map, MapRef};
use crate::math::{Fixed, Vec2};
use crate::outcome::{Outcome, PlayerStanding, Standings};
use crate::path::{NODE_BUDGET_PER_TICK, Path, PathGrid, PathStats, Pathfinder, Request};
use crate::rng::Rng;
use crate::terrain::TerrainRules;
use crate::vision::VisionRules;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

mod air;
mod alert;
mod boats;
mod care;
mod combat;
mod craters;
mod crew;
mod crossings;
mod destruction;
mod discipline;
mod drill;
mod economy;
mod escort;
mod experience;
mod feedback;
mod fire;
mod grenades;
mod ground_info;
mod indirect;
mod infantry;
mod knockdown;
mod logistics;
mod march;
mod mission;
mod orders;
mod overrun;
mod positions;
mod prepared;
mod progress;
mod spatial;
mod strike;
mod tactics;
mod transport;
mod vision;
mod weather;
mod works;

pub use vision::MAX_PLAYERS;

use combat::{Attack, CombatState, Projectile, TypeCombat};
use infantry::{InfantryRt, Squad};

pub use boats::BoatView;
pub use care::{AidView, CareEvent, WoundView};
pub use crew::CrewView;
pub use discipline::FireLimits;
pub use drill::DrillView;
pub use economy::{BuyOption, PlaceError, StructureView, SupplyView};
pub use experience::ExperienceView;
pub use feedback::{Activity, FireStatus, OrderEvent, RejectReason};
pub use grenades::GrenadeSnapshot;
pub use ground_info::GroundInfo;
pub use infantry::BuildingCondition;
pub use logistics::{StockView, SupplySource};
pub use mission::{ArrivalView, ObjectiveView, RatingView, column_spacing};
pub use orders::MAX_QUEUED;
pub use progress::MoveEvent;
pub use tactics::{SquadTactics, soldier_flags};

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
    /// Fire mode at the start (default: fire at will).
    pub fire_mode: FireMode,
}

/// Everything needed to start a match. Identical on all peers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimConfig {
    /// Simulation ticks per second.
    pub tick_hz: u32,
    pub unit_types: Vec<UnitType>,
    pub spawns: Vec<Spawn>,
    /// The map this match is played on. [`Sim::new`] gets the map itself and
    /// checks it against this reference.
    pub map: MapRef,
    pub terrain: TerrainRules,
    /// Combat rules (`data/combat.toml`).
    pub combat: CombatRules,
    /// Vision rules (`data/vision.toml`, prompt 018).
    pub vision: VisionRules,
    /// Infantry rules (`data/infantry.toml`, prompt 022).
    pub infantry: crate::infantry::InfantryRules,
    /// Teams and shared vision (prompt 037); default: every player for
    /// themselves.
    #[serde(default)]
    pub teams: crate::teams::Teams,
    /// Weather and time of day (prompt 040); default: clear by day, which
    /// changes nothing.
    #[serde(default)]
    pub weather: crate::weather::WeatherSetup,
    /// The mission script (prompt 039, `crate::mission`); `None` for
    /// skirmishes.
    #[serde(default)]
    pub mission: Option<crate::mission::Mission>,
    /// The start layout of the map (prompt 182, `[[variant]]` in
    /// `map.toml`); `None`: the standard one. An input of the scenario:
    /// the spawns already stand at its starts; the simulation only reads
    /// which supply points count.
    #[serde(default)]
    pub variant: Option<VariantSetup>,
}

/// A chosen start layout (prompt 182), see [`SimConfig::variant`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantSetup {
    pub name: String,
    /// Supply markers that count; `None`: all of the map's.
    pub supply: Option<Vec<String>>,
}

impl VariantSetup {
    /// The setup of a map's start variant.
    pub fn of(variant: &crate::map::MapVariant) -> Self {
        Self {
            name: variant.name.clone(),
            supply: variant.supply.clone(),
        }
    }
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

    /// The vision rules with the sight ranges of this match's weather and
    /// time of day (prompt 040): `self.vision` for clear weather by day.
    pub fn effective_vision(&self) -> VisionRules {
        let mut vision = self.vision.clone();
        if !self.weather.is_default()
            && let Ok(rules) = crate::weather::WeatherRules::builtin()
        {
            let pct = rules.sight_pct(self.weather);
            if pct != 100 {
                vision.sight_ratio.0 = vision.sight_ratio.0.saturating_mul(pct);
                vision.sight_ratio.1 = vision.sight_ratio.1.saturating_mul(100);
            }
        }
        vision
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    ZeroTickRate,
    UnknownUnitType(String),
    /// The given map is not the one the config references.
    MapMismatch {
        expected: MapRef,
        actual: MapRef,
    },
    /// The map uses a terrain id without a [`TerrainRules`] entry.
    UnknownTerrain(u8),
    /// Spawn with this index lies outside the map.
    SpawnOutsideMap(usize),
    /// More players than [`MAX_PLAYERS`].
    TooManyPlayers(usize),
    /// The built-in destruction rules do not parse (prompt 020).
    Destruction(String),
    /// The crater rules are invalid (prompt 021).
    Craters(String),
    /// The built-in economy rules do not parse (prompt 023).
    Economy(String),
    /// The built-in indirect fire rules do not parse (prompt 032).
    Indirect(String),
    /// The built-in fire rules do not parse (prompt 041).
    Fire(String),
    /// The built-in air rules do not parse (prompt 033).
    Air(String),
    /// The built-in strike rules do not parse (prompt 054).
    Strike(String),
    /// The built-in transport rules do not parse (prompt 034).
    Transport(String),
    /// The built-in logistics rules do not parse (prompt 036).
    Logistics(String),
    /// The built-in weather rules do not parse (prompt 040).
    Weather(String),
    /// The built-in field work rules do not parse (prompt 035).
    Works(String),
    /// The built-in tactics rules do not parse (prompt 047).
    Tactics(String),
    /// The built-in experience rules do not parse (prompt 050).
    Experience(String),
    /// The built-in crossing rules do not parse (prompt 055).
    Crossings(String),
    /// The built-in care rules do not parse (prompt 049).
    Care(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTickRate => write!(f, "tick_hz must be positive"),
            Self::UnknownUnitType(name) => write!(f, "unknown unit type '{name}'"),
            Self::MapMismatch { expected, actual } => write!(
                f,
                "map '{}' ({:#018x}) does not match the configured map '{}' ({:#018x})",
                actual.name, actual.hash, expected.name, expected.hash
            ),
            Self::UnknownTerrain(id) => write!(f, "map uses unknown terrain id {id}"),
            Self::SpawnOutsideMap(index) => write!(f, "spawn {index} lies outside the map"),
            Self::TooManyPlayers(n) => write!(f, "{n} players, at most {MAX_PLAYERS} allowed"),
            Self::Destruction(e) | Self::Economy(e) => write!(f, "{e}"),
            Self::Craters(e)
            | Self::Indirect(e)
            | Self::Fire(e)
            | Self::Air(e)
            | Self::Strike(e)
            | Self::Transport(e)
            | Self::Logistics(e)
            | Self::Weather(e)
            | Self::Works(e)
            | Self::Tactics(e)
            | Self::Experience(e)
            | Self::Crossings(e)
            | Self::Care(e) => {
                write!(f, "{e}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Unit {
    id: UnitId,
    owner: PlayerId,
    unit_type: u32,
    position: Vec2,
    /// Ground height at `position` in meters.
    height: Fixed,
    /// Final destination (formation slot), also while the path is planned.
    target: Option<Vec2>,
    route: Route,
    /// Paths planned again from the current position (because the way to
    /// the next waypoint was blocked) since the last waypoint was reached.
    replans: u32,
    /// Cross-country speed in meters per tick.
    speed_offroad: Fixed,
    /// Road speed in meters per tick.
    speed_road: Fixed,
    /// Meters, see [`UnitType::footprint_m`].
    footprint: Fixed,
    /// Hull direction (unit length): the last driving direction.
    facing: Vec2,
    /// Hit points; 0 = destroyed (a wreck, or a dead squad). For squads
    /// the sum of their soldiers' hit points.
    hp: u32,
    combat: CombatState,
    /// Soldiers, stance and building of an infantry squad (prompt 022).
    squad: Option<Box<Squad>>,
    /// Share of the terrain speed in percent (stance; 100 for vehicles).
    speed_pct: u32,
    /// Width for formations and separation: the footprint, or the width of
    /// a squad's formation.
    spread: Fixed,
    /// Eye height over the ground in mm when higher than the type's
    /// (observers in a building); 0 otherwise.
    elevation_mm: i32,
    /// The viewshed must be computed again (entered or left a building).
    vision_dirty: bool,
    /// Queued waypoints and patrol (prompt 031).
    orders: orders::OrderQueue,
    /// Riding in or on a vehicle, or towed by one (prompt 034).
    #[serde(default)]
    link: Option<transport::Link>,
    /// Ammunition and fuel (prompt 036).
    #[serde(default)]
    stock: logistics::Stock,
    /// Formation facing, attack-move and securing (prompt 046).
    #[serde(default)]
    drill: drill::Drill,
    /// Experience points and kills (prompt 050).
    #[serde(default)]
    veterancy: experience::Veterancy,
    /// Ticks the vehicle still drives slowly after breaking through a wall
    /// (prompt 093, `[crush]` in `data/destruction.toml`).
    #[serde(default)]
    crush_slow: u32,
    /// Where it was last fired at from, ordered facing hold (prompt 110).
    #[serde(default)]
    alert: alert::Alert,
    /// Progress along its route, for the last resort against stuck units
    /// (prompt 173).
    #[serde(default)]
    progress: progress::Progress,
    /// Who fired at it lately (return fire) and the retreat cooldown of an
    /// unarmed vehicle (prompt 176).
    #[serde(default)]
    under_fire: discipline::UnderFire,
    /// The men serving a gun (prompt 177); empty for other units.
    #[serde(default)]
    crew: crew::Crew,
}

impl Unit {
    const fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// How a unit gets to its target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Route {
    /// No target.
    None,
    /// Waiting for the path request with this id.
    Waiting(u32),
    /// Driving along waypoints; the last one is the target.
    Following {
        waypoints: Vec<Vec2>,
        next: usize,
        /// Smallest distance to `waypoints[next]` so far.
        best: Fixed,
        /// Ticks since `best` last improved.
        stalled: u32,
    },
}

/// Route state of a unit, for presentation and tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteView<'a> {
    Idle,
    /// The path is still being planned.
    Waiting,
    /// Remaining waypoints, the target last.
    Following(&'a [Vec2]),
}

/// Presentation data of one unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitSnapshot {
    pub id: UnitId,
    pub owner: PlayerId,
    /// Index into [`SimConfig::unit_types`].
    pub unit_type: u32,
    pub position: Vec2,
    /// Ground height at `position` in meters.
    pub height: Fixed,
    pub target: Option<Vec2>,
    /// Hull direction (unit length).
    pub facing: Vec2,
    /// Direction of the turret / main weapon (unit length).
    pub aim: Vec2,
    /// Hit points left; 0 = destroyed.
    pub hp: u32,
    pub max_hp: u32,
    pub fire_mode: FireMode,
    /// Limits of automatic fire (prompt 176).
    pub fire_limits: FireLimits,
    /// Stance of an infantry squad; `None` for vehicles (prompt 022).
    pub stance: Option<crate::infantry::Stance>,
    /// The building (object index) a squad is in.
    pub garrison: Option<u32>,
    /// The squad is changing its stance.
    pub changing_stance: bool,
    /// Width for formations: the footprint, or a squad's formation width.
    pub spread: Fixed,
    /// The vehicle a passenger rides in, or the tractor of a towed gun
    /// (prompt 034).
    pub carrier: Option<UnitId>,
    /// Soldiers riding in or on this vehicle.
    pub passengers: u16,
    /// The gun this vehicle tows.
    pub towing: Option<UnitId>,
    /// Mounting, dismounting, limbering or unlimbering is under way.
    pub loading: bool,
    /// Ammunition, fuel and mobility (prompt 036).
    pub stock: StockView,
    /// Experience points, rank and kills (prompt 050).
    pub experience: ExperienceView,
    /// Where the unit remembers being fired at from (prompt 110): only for
    /// the player's own units in a filtered snapshot.
    pub alert: Option<Vec2>,
}

/// Presentation data of one soldier of a squad (prompt 022).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoldierSnapshot {
    pub unit: UnitId,
    /// Index of the soldier within the squad.
    pub index: u16,
    pub position: Vec2,
    /// Ground height at `position` in meters.
    pub height: Fixed,
    /// Hit points left; 0 = dead.
    pub hp: u32,
    /// The soldier uses cover (stands at a cover spot or in a building).
    pub in_cover: bool,
    /// Ticks left knocked down by a blast (prompt 048); 0 = not.
    pub knocked: u32,
    /// Where the blast that knocked him down (last) went off.
    pub knock_from: Vec2,
    /// Prompt 194: his last throw by a blast: start point, the tick it
    /// began and the flight time in ticks (0: never thrown). `position` is
    /// already the landing point.
    pub throw_from: Vec2,
    pub throw_tick: u64,
    pub flight: u32,
    /// Map object whose cover spot he stands at (prompt 048: cover poses).
    pub cover: Option<u32>,
    /// Wound state (prompt 049).
    pub wound: WoundView,
    /// What he does for a comrade, or what is done with him (prompt 049).
    pub aid: AidView,
}

impl UnitSnapshot {
    pub const fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// Presentation data of the whole simulation after a tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Number of ticks simulated so far.
    pub tick: u64,
    /// Sorted by unit id.
    pub units: Vec<UnitSnapshot>,
    /// Projectiles in flight, by id.
    pub projectiles: Vec<ProjectileSnapshot>,
    /// What happened during the last tick, in order.
    pub events: Vec<CombatEvent>,
    /// Soldiers of all squads, by unit id and index (prompt 022).
    pub soldiers: Vec<SoldierSnapshot>,
    /// Shells of indirect fire in flight, by id (prompt 032).
    pub shells: Vec<crate::indirect::ShellSnapshot>,
    /// Smoke clouds (prompt 032).
    pub smoke: Vec<crate::indirect::SmokeSnapshot>,
    /// Aircraft in the air, by id (prompt 033; filtered by vision).
    pub aircraft: Vec<crate::air::AircraftSnapshot>,
    /// Falling bombs, by id.
    pub bombs: Vec<crate::air::BombSnapshot>,
    /// Called missions (filtered: only the player's own).
    pub sorties: Vec<crate::air::SortieSnapshot>,
}

/// Phases of a tick in [`Sim::step_observed`], in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Players' commands (orders, formation, path requests).
    Commands,
    /// Path search within the node budget, routes for waiting units.
    Paths,
    /// Infantry stances, targets, aiming and firing.
    Combat,
    /// Unit movement with separation.
    Movement,
    /// Soldiers of infantry squads (formation, cover, buildings).
    Squads,
    /// Crushing map objects, projectile flight and hits.
    Projectiles,
    /// Destroyed map objects and craters: path grid and vision cells.
    Destruction,
    /// Accounts, structures, production, supply (prompt 023).
    Economy,
    /// Viewsheds and player vision.
    Vision,
    /// Victory conditions.
    Standings,
}

impl Phase {
    pub const ALL: [Self; 10] = [
        Self::Commands,
        Self::Paths,
        Self::Combat,
        Self::Movement,
        Self::Squads,
        Self::Projectiles,
        Self::Destruction,
        Self::Economy,
        Self::Vision,
        Self::Standings,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Commands => "commands",
            Self::Paths => "paths",
            Self::Combat => "combat",
            Self::Movement => "movement",
            Self::Squads => "squads",
            Self::Projectiles => "projectiles",
            Self::Destruction => "destruction",
            Self::Economy => "economy",
            Self::Vision => "vision",
            Self::Standings => "standings",
        }
    }
}

/// Reusable per-tick scratch buffers for [`Sim::move_units`] (prompt 210).
/// Derived scratch only: never hashed, never serialized.
#[derive(Clone, Debug, Default)]
struct MoveScratch {
    directions: Vec<Option<Vec2>>,
    waypoints: Vec<Option<Vec2>>,
    near: Vec<u32>,
    pushes: Vec<Vec2>,
    caps: Vec<Option<Fixed>>,
    blocked: Vec<usize>,
    stalled: Vec<(usize, Vec2)>,
    driven: Vec<(usize, Fixed)>,
}

/// The deterministic simulation.
///
/// `Serialize`/`Deserialize` are for snapshots (prompt 038,
/// `crate::snapshot`): every field is saved, so new state is included as
/// soon as its type derives them. The map is saved apart, as the changes
/// against the loaded map.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    /// Sorted by id; ids are assigned in increasing order.
    units: Vec<Unit>,
    #[serde(skip, default = "crate::snapshot::placeholder_map")]
    map: Arc<Map>,
    terrain: TerrainRules,
    paths: Pathfinder,
    tick_hz: u32,
    combat_rules: CombatRules,
    /// Per unit type.
    types: Vec<TypeCombat>,
    projectiles: Vec<Projectile>,
    next_projectile: u32,
    /// Events of the last tick (kept in snapshots because `Sim::snapshot()`
    /// exposes them immediately after restore; not hashed).
    events: Vec<CombatEvent>,
    /// Units that stopped short of their target in the last tick (prompt
    /// 173; derived, not hashed).
    #[serde(skip)]
    move_events: Vec<MoveEvent>,
    /// Units that waited in front of a bridge for their group in the last
    /// movement (prompt 187; derived, not hashed): their progress clock
    /// waits.
    #[serde(skip)]
    march_waiting: Vec<bool>,
    /// Orders dropped in the last tick, with their reason (prompt 175;
    /// derived, not hashed).
    #[serde(skip)]
    order_events: Vec<OrderEvent>,
    /// Per unit index: units that fire at it, counted at the start of every
    /// combat phase (prompt 188; derived scratch, not state).
    #[serde(skip)]
    focus: Vec<u8>,
    /// Immutable CSR index of map objects; rebuilt in `set_restored_map`
    /// (prompt 210).
    #[serde(skip)]
    objects: combat::ObjectIndex,
    /// Defeated players and the result (prompt 027).
    standings: Standings,
    /// Viewsheds and player vision (prompt 018).
    vision: vision::Vision,
    /// Infantry rules and cover index (prompt 022).
    infantry: InfantryRt,
    /// Hit points and stages of map objects (prompt 020).
    destruction: destruction::Destruction,
    /// Unit positions by area (prompt 016): rebuilt within a tick where
    /// queries need it; not state.
    #[serde(skip)]
    unit_grid: spatial::UnitGrid,
    /// Per-tick scratch buffers for `move_units` (prompt 210).
    #[serde(skip)]
    move_scratch: MoveScratch,
    /// Craters and the deformation of the ground (prompt 021).
    craters: craters::Craters,
    /// The unit types of the match (`SimConfig::unit_types`), for units
    /// produced during the match (prompt 023).
    unit_types: Vec<UnitType>,
    /// Accounts, structures, production, supply points (prompt 023).
    economy: economy::Economy,
    /// Indirect fire, shells and smoke (prompt 032).
    artillery: indirect::Artillery,
    /// Burning objects and wrecks (prompt 041).
    fire: fire::Fire,
    /// Air support and anti-aircraft fire (prompt 033).
    air: air::Air,
    /// Teams of the match (prompt 037, `SimConfig::teams`).
    teams: crate::teams::Teams,
    /// Mounting, dismounting and towing (prompt 034).
    transport: transport::Transport,
    /// Supply, consumption and repair (prompt 036).
    logistics: logistics::Logistics,
    /// Weather, soft ground and time of day (prompt 040).
    weather: weather::WeatherState,
    /// Field works of engineers: mines, obstacles, positions (prompt 035).
    works: works::Works,
    /// Infantry tactics rules (prompt 047); the state is in the squads.
    tactics: tactics::TacticsRt,
    /// Experience rules and promotions (prompt 050).
    experience: experience::Experience,
    /// Collapsing decks, swimmers, bridge-laying tanks, camouflage and
    /// paratroopers (prompt 055).
    crossings: crossings::Crossings,
    /// Wounds, buddy aid, medics and bailed-out crews (prompt 049).
    care: care::Care,
    /// Heavy rocket strikes (prompt 054).
    strikes: strike::Strikes,
    /// The mission script and its progress (prompt 039).
    #[serde(default)]
    mission: Option<Box<mission::MissionRt>>,
    /// Name of the start layout of the match (prompt 182), for the
    /// presentation; follows from the config, so not hashed.
    #[serde(default)]
    variant: Option<String>,
    /// Vehicles running soldiers over (prompt 093): lists of the current
    /// tick, not state.
    #[serde(skip)]
    overrun: overrun::Overrun,
    /// Boats: landing, ramps, cargo (prompt 119); empty without boats.
    #[serde(default)]
    boats: boats::Boats,
    /// Initial spawn home per player (prompt 234, derived from `config.spawns`
    /// for retreat fallback when a match has no headquarters; not hashed).
    #[serde(default)]
    player_homes: Vec<(PlayerId, Vec2)>,
    /// Infantry hand grenades and smoke grenades (prompt 232).
    #[serde(default)]
    grenades: grenades::Grenades,
}

impl Sim {
    /// Starts a match on `map`, which must be the map `config.map` names
    /// (same content hash).
    pub fn new(config: &SimConfig, map: Arc<Map>, seed: u64) -> Result<Self, ConfigError> {
        if config.tick_hz == 0 {
            return Err(ConfigError::ZeroTickRate);
        }
        if map.reference() != config.map {
            return Err(ConfigError::MapMismatch {
                expected: config.map.clone(),
                actual: map.reference(),
            });
        }
        if let Some(&id) = map.terrain_ids().iter().max()
            && usize::from(id) >= config.terrain.types.len()
        {
            return Err(ConfigError::UnknownTerrain(id));
        }
        let players = config.players();
        if players.len() > MAX_PLAYERS {
            return Err(ConfigError::TooManyPlayers(players.len()));
        }
        // Weather and time of day shorten every sight range (prompt 040).
        let weather_config;
        let config = if config.weather.is_default() {
            config
        } else {
            weather_config = SimConfig {
                vision: config.effective_vision(),
                ..config.clone()
            };
            &weather_config
        };
        let infantry = InfantryRt::new(&config.infantry, &config.unit_types, &map, config.tick_hz);

        let mut units = Vec::with_capacity(config.spawns.len());
        for (index, spawn) in (0u32..).zip(&config.spawns) {
            let type_index = (0u32..)
                .zip(&config.unit_types)
                .find(|(_, t)| t.name == spawn.unit_type)
                .ok_or_else(|| ConfigError::UnknownUnitType(spawn.unit_type.clone()))?
                .0;
            if !map.contains(spawn.position) {
                return Err(ConfigError::SpawnOutsideMap(index as usize));
            }
            // Units start facing the map center.
            let to_center = Vec2::ZERO - spawn.position;
            let distance = to_center.length();
            let facing = if distance > Fixed::ZERO {
                Vec2::new(to_center.x / distance, to_center.y / distance)
            } else {
                Vec2::new(Fixed::ZERO, -Fixed::ONE)
            };
            let max_hp = config
                .combat
                .max_hp(&config.unit_types[type_index as usize].combat);
            units.push(new_unit(&NewUnit {
                id: UnitId(index),
                owner: spawn.owner,
                type_index,
                unit_type: &config.unit_types[type_index as usize],
                position: spawn.position,
                facing,
                fire_mode: spawn.fire_mode,
                hp: max_hp,
                soldier_hp: config.combat.damage.soldier_hp,
                infantry: &infantry,
                map: &map,
                tick_hz: config.tick_hz,
            }));
        }
        let types: Vec<TypeCombat> = config
            .unit_types
            .iter()
            .map(|t| TypeCombat::new(t, &config.combat, config.tick_hz))
            .collect();
        let vision_types: Vec<_> = types
            .iter()
            .zip(&config.unit_types)
            .map(|(c, t)| (vision::eye_mm(c.height), t.combat.category))
            .collect();
        let vision = vision::Vision::new(
            &config.vision,
            &map,
            players.clone(),
            &config.teams,
            &vision_types,
            units.len(),
        );

        let destruction = destruction::Destruction::new(
            crate::destruction::DestructionRules::builtin()
                .map_err(|e| ConfigError::Destruction(e.to_string()))?,
            &map,
            &config.vision,
            &config.unit_types,
        );
        let craters = craters::Craters::new(
            crate::craters::CraterRules::builtin()
                .map_err(|e| ConfigError::Craters(e.to_string()))?,
            &config.terrain,
        )
        .map_err(ConfigError::Craters)?;
        let mut economy_rules = crate::economy::EconomyRules::builtin()
            .map_err(|e| ConfigError::Economy(e.to_string()))?;
        economy_rules.keep_factions(&config.unit_types);
        let economy = economy::Economy::new(economy_rules, config, &map, &players);
        let artillery = indirect::Artillery::new(
            crate::indirect::IndirectRules::builtin()
                .map_err(|e| ConfigError::Indirect(e.to_string()))?,
            &config.unit_types,
            config.tick_hz,
        );
        let fire = fire::Fire::new(
            crate::fire::FireRules::builtin().map_err(|e| ConfigError::Fire(e.to_string()))?,
            &map,
            &config.terrain,
            config.tick_hz,
        )
        .map_err(ConfigError::Fire)?;
        let air = air::Air::new(
            crate::air::AirRules::builtin().map_err(|e| ConfigError::Air(e.to_string()))?,
            config,
        );
        let mut vision = vision;
        vision.ensure_radius(air.recon_radius);
        let mut strike_rules = crate::strike::StrikeRules::builtin()
            .map_err(|e| ConfigError::Strike(e.to_string()))?;
        strike_rules
            .factions
            .retain(|f| crate::economy::keeps_faction(&f.faction, &config.unit_types));
        let strikes = strike::Strikes::new(strike_rules, config);
        // Boats (prompt 119): their rules, and their passenger rules among
        // the carriers, only in matches with a boat type.
        let boat_rules = crate::boats::BoatRules::builtin()
            .map_err(|e| ConfigError::Transport(e.to_string()))?;
        let mut transport_rules = crate::transport::TransportRules::builtin()
            .map_err(|e| ConfigError::Transport(e.to_string()))?;
        if config.unit_types.iter().any(UnitType::is_boat) {
            transport_rules
                .carriers
                .extend(boat_rules.carriers.iter().copied());
        }
        let boats = boats::Boats::new(Some(boat_rules), &config.unit_types, config.tick_hz);
        let transport =
            transport::Transport::new(transport_rules, &config.unit_types, config.tick_hz);
        let logistics = logistics::Logistics::new(
            crate::logistics::LogisticsRules::builtin()
                .map_err(|e| ConfigError::Logistics(e.to_string()))?,
            &config.unit_types,
        );
        let mut works_rules =
            crate::works::WorksRules::builtin().map_err(|e| ConfigError::Works(e.to_string()))?;
        works_rules
            .positions
            .bunkers
            .retain(|id| crate::economy::keeps_id(id, &config.unit_types));
        let works = works::Works::new(works_rules, &config.unit_types, &map, config.tick_hz);
        let tactics = tactics::TacticsRt::new(
            crate::tactics::TacticsRules::builtin()
                .map_err(|e| ConfigError::Tactics(e.to_string()))?,
            &config.unit_types,
            config.tick_hz,
        );
        let crossings = crossings::Crossings::new(
            crate::crossings::CrossingRules::builtin()
                .map_err(|e| ConfigError::Crossings(e.to_string()))?,
            config,
            &config.terrain,
        );
        let grid = PathGrid::with_crushing(
            &map,
            &config.terrain,
            &config.unit_types,
            &destruction.crushing(&map),
        );
        let weather = weather::WeatherState::new(
            crate::weather::WeatherRules::builtin()
                .map_err(|e| ConfigError::Weather(e.to_string()))?,
            config.weather,
            &config.unit_types,
            &config.terrain,
            config.tick_hz,
            config.vision.sight_ratio,
        );
        let experience = experience::Experience::new(
            crate::experience::ExperienceRules::builtin()
                .map_err(|e| ConfigError::Experience(e.to_string()))?,
        );
        let care = care::Care::new(
            crate::care::CareRules::builtin().map_err(|e| ConfigError::Care(e.to_string()))?,
            &config.unit_types,
            config.tick_hz,
        );
        let player_homes: Vec<(PlayerId, Vec2)> = players
            .iter()
            .filter_map(|&player| {
                let mut count = 0i64;
                let mut sum_x = Fixed::ZERO;
                let mut sum_y = Fixed::ZERO;
                for s in config.spawns.iter().filter(|s| s.owner == player) {
                    sum_x += s.position.x;
                    sum_y += s.position.y;
                    count += 1;
                }
                (count > 0).then(|| {
                    let n = Fixed::from_num(count);
                    (player, Vec2::new(sum_x / n, sum_y / n))
                })
            })
            .collect();
        let mut sim = Self {
            tick: 0,
            rng: Rng::new(seed),
            units,
            terrain: config.terrain.clone(),
            paths: Pathfinder::new(grid),
            tick_hz: config.tick_hz,
            combat_rules: config.combat.clone(),
            types,
            projectiles: Vec::new(),
            next_projectile: 0,
            events: Vec::new(),
            move_events: Vec::new(),
            march_waiting: Vec::new(),
            order_events: Vec::new(),
            focus: Vec::new(),
            objects: combat::ObjectIndex::new(&map),
            map,
            standings: Standings::new(&players),
            vision,
            infantry,
            destruction,
            unit_grid: spatial::UnitGrid::default(),
            move_scratch: MoveScratch::default(),
            craters,
            unit_types: config.unit_types.clone(),
            economy,
            artillery,
            fire,
            air,
            teams: config.teams.clone(),
            transport,
            logistics,
            weather,
            works,
            tactics,
            experience,
            crossings,
            care,
            strikes,
            mission: config
                .mission
                .as_ref()
                .map(|m| Box::new(mission::MissionRt::new(m))),
            variant: config.variant.as_ref().map(|v| v.name.clone()),
            overrun: overrun::Overrun::default(),
            boats,
            player_homes,
            grenades: grenades::Grenades::default(),
        };
        sim.init_weather();
        sim.init_economy();
        sim.prepare_ground();
        sim.apply_destruction();
        sim.update_vision(true);
        Ok(sim)
    }

    /// Number of ticks simulated so far.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The map of this match.
    /// Puts the map back after deserializing a snapshot (prompt 038,
    /// `crate::snapshot`; the map is not part of the serialized fields) and
    /// rebuilds the derived static object index (prompt 210).
    pub(crate) fn set_restored_map(&mut self, map: Arc<Map>) {
        self.objects = combat::ObjectIndex::new(&map);
        self.paths.grid.rebuild_derived();
        self.map = map;
    }

    pub const fn map(&self) -> &Arc<Map> {
        &self.map
    }

    /// The pathfinding grid of this match.
    pub const fn path_grid(&self) -> &PathGrid {
        &self.paths.grid
    }

    /// Counters of the path request queue.
    pub const fn path_stats(&self) -> PathStats {
        self.paths.stats
    }

    /// Number of path requests not finished yet.
    pub fn path_queue_len(&self) -> usize {
        self.paths.queued()
    }

    /// Route state of a unit (`None` if the unit does not exist).
    pub fn route_of(&self, id: UnitId) -> Option<RouteView<'_>> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        Some(match &self.units[index].route {
            Route::None => RouteView::Idle,
            Route::Waiting(_) => RouteView::Waiting,
            Route::Following {
                waypoints, next, ..
            } => RouteView::Following(&waypoints[*next..]),
        })
    }

    /// Advances the simulation by exactly one tick.
    ///
    /// Commands are applied in the given order (the lockstep layer sorts
    /// them by player, then issue order), then queued path requests get
    /// their node budget, then units move.
    pub fn step(&mut self, commands: &[PlayerCommand]) {
        self.step_observed(commands, |_| {});
    }

    /// [`Self::step`] that calls `done` after every phase of the tick, for
    /// benchmarks that time the phases (`cli bench`, prompt 016). The
    /// simulation itself never reads a clock; `done` cannot change the
    /// state.
    pub fn step_observed(&mut self, commands: &[PlayerCommand], mut done: impl FnMut(Phase)) {
        self.events.clear();
        self.air.events.clear();
        self.crossings.events.clear();
        self.strikes.events.clear();
        self.move_events.clear();
        self.order_events.clear();
        for command in commands {
            self.apply(command);
        }
        self.advance_queues();
        done(Phase::Commands);
        self.plan_paths();
        done(Phase::Paths);
        self.update_stances();
        self.care_speed();
        self.rebuild_unit_grid();
        self.update_alerts();
        self.aim_and_fire();
        self.update_drills();
        self.update_tactics();
        self.update_grenades();
        self.fire_indirect();
        self.update_strikes();
        done(Phase::Combat);
        self.halt_stranded();
        self.update_swimmers();
        self.move_units();
        self.check_progress();
        self.update_boats();
        self.update_transport();
        done(Phase::Movement);
        self.plan_overrun();
        self.update_squads();
        self.overrun_soldiers();
        done(Phase::Squads);
        self.crush_objects();
        self.rebuild_unit_grid();
        self.update_works();
        self.fly_projectiles();
        self.fly_shells();
        self.fly_grenades();
        self.update_air();
        self.update_crossings();
        self.update_fire();
        self.transport_losses();
        done(Phase::Projectiles);
        self.apply_destruction();
        self.apply_craters();
        self.apply_digs();
        self.update_weather();
        done(Phase::Destruction);
        self.update_economy();
        self.update_logistics();
        self.update_experience();
        self.update_care();
        done(Phase::Economy);
        self.update_smoke();
        self.update_vision(false);
        self.hide_camouflaged();
        self.reveal_flashes();
        self.reveal_strikes();
        done(Phase::Vision);
        self.tick += 1;
        // The mission script (prompt 039) may decide the match first.
        self.update_mission();
        let mut exempt_mask = 0u64;
        for s in self.standings.players() {
            if !self.eliminable(s.player) && usize::from(s.player.0) < MAX_PLAYERS {
                exempt_mask |= 1u64 << s.player.0;
            }
        }
        let is_exempt =
            |p: PlayerId| usize::from(p.0) < MAX_PLAYERS && (exempt_mask & (1u64 << p.0)) != 0;
        let units = &self.units;
        let teams = &self.teams;
        let care = &self.care;
        let economy = &self.economy;
        let crews = |u: &Unit| care.is_crew(u);
        let alive = |id: UnitId| {
            units
                .binary_search_by_key(&id, |u| u.id)
                .is_ok_and(|i| units[i].alive())
        };
        self.standings.update_with_bases(
            self.tick,
            |player| {
                if is_exempt(player) {
                    1
                } else {
                    alive_units(units, player, crews)
                }
            },
            // Decision 6 of prompt 045: a base match is also lost with the base.
            |player| !is_exempt(player) && economy.base_lost(player, alive),
            |a, b| teams.allied(a, b),
        );
        done(Phase::Standings);
    }

    /// Every player of the match (ascending) and whether they are defeated.
    pub fn standings(&self) -> &[PlayerStanding] {
        self.standings.players()
    }

    /// The result, once the match is decided. It never changes afterwards.
    /// With teams, `winner` is the lowest undefeated player of the winning
    /// team; [`Self::winners`] lists the whole team.
    pub const fn outcome(&self) -> Option<Outcome> {
        self.standings.outcome()
    }

    /// Teams of the match (prompt 037).
    pub const fn teams(&self) -> &crate::teams::Teams {
        &self.teams
    }

    /// Whether `a` and `b` are on the same team (a player is its own ally).
    pub fn allied(&self, a: PlayerId, b: PlayerId) -> bool {
        self.teams.allied(a, b)
    }

    /// Every player of the winning team (empty before the match is decided
    /// and for a draw), ascending; also its defeated members.
    pub fn winners(&self) -> Vec<PlayerId> {
        let Some(winner) = self.outcome().and_then(|o| o.winner) else {
            return Vec::new();
        };
        self.standings()
            .iter()
            .map(|s| s.player)
            .filter(|&p| self.teams.allied(p, winner))
            .collect()
    }

    /// Number of units of `player` that are not destroyed (for victory
    /// conditions).
    pub fn alive_units(&self, player: PlayerId) -> usize {
        alive_units(&self.units, player, |u| self.is_crew_unit(u))
    }

    /// Hit points of a unit (`None` if it does not exist).
    pub fn unit_hp(&self, id: UnitId) -> Option<u32> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        Some(self.units[index].hp)
    }

    /// Every unit (buildings and crews included) with owner, type index
    /// and whether it is alive, in id order. Read-only, for the match
    /// statistics (prompt 045, `stats`).
    pub fn unit_roster(&self) -> impl Iterator<Item = (UnitId, PlayerId, u32, bool)> + '_ {
        self.units
            .iter()
            .map(|u| (u.id, u.owner, u.unit_type, u.alive()))
    }

    /// Ticks per second.
    /// The unit types of the match (`SimConfig::unit_types`).
    pub fn unit_types(&self) -> &[UnitType] {
        &self.unit_types
    }

    /// The start layout the match was set up in (prompt 182), `None` for
    /// the standard one.
    pub fn config_variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    pub const fn tick_hz(&self) -> u32 {
        self.tick_hz
    }

    /// Events of the last tick.
    pub fn events(&self) -> &[CombatEvent] {
        &self.events
    }

    /// Blocks (or frees) the area between the corners `a` and `b` for
    /// pathfinding, as a building, a wreck or a wall would. Called by game
    /// systems (destruction, prompt 020), never by player commands. Units
    /// whose remaining route touches the changed cells plan again.
    pub fn set_path_obstacle(&mut self, a: Vec2, b: Vec2, blocked: bool) {
        let grid = &mut self.paths.grid;
        let rect = grid.cells_in(a, b);
        let changed = grid.set_obstacle(&self.map, &self.terrain, rect, blocked);
        self.replan_through(changed);
    }

    /// Destroys bridge `bridge` (index into [`Map::bridges`]) or restores
    /// it: without its deck the water below is impassable again. Called by
    /// game systems (destruction, prompt 020), never by player commands.
    /// The map is copied on the first change (copy-on-write); units whose
    /// remaining route touches the deck plan again. Units standing on a
    /// destroyed deck stay where they are (prompt 020 decides their fate).
    pub fn set_bridge_intact(&mut self, bridge: usize, intact: bool) {
        let Some(rect) = self.paths.grid.bridge_cells(bridge) else {
            return;
        };
        if self.map.bridges()[bridge].intact == intact {
            return;
        }
        Arc::make_mut(&mut self.map).set_bridge_intact(bridge, intact);
        let changed = self.paths.grid.rebuild(&self.map, &self.terrain, rect);
        self.replan_through(changed);
        // See-through decks (prompt 115): the vision cells under the deck
        // get the ground (or the deck) as their height again.
        let b = &self.map.bridges()[bridge];
        if b.see_through {
            let (lo, hi) = b.bounds();
            let grid = &self.vision.grid;
            let (c0, r0) = grid.cell_of(lo);
            let (c1, r1) = grid.cell_of(hi);
            let cells = (
                c0.max(0) as u32,
                r0.max(0) as u32,
                c1.max(0) as u32,
                r1.max(0) as u32,
            );
            let map = Arc::clone(&self.map);
            if Arc::make_mut(&mut self.vision.grid).refresh_decks(cells, &map) {
                self.requeue_viewsheds(cells);
            }
        }
    }

    /// After the path grid changed in `changed`: restarts the running search
    /// and lets every unit whose remaining route touches the area plan again.
    fn replan_through(&mut self, changed: crate::path::CellRect) {
        // The running search used the old grid.
        self.paths.restart_active();
        for unit in &mut self.units {
            let Route::Following {
                waypoints, next, ..
            } = &unit.route
            else {
                continue;
            };
            let grid = &self.paths.grid;
            let mut from = unit.position;
            let touched = waypoints[*next..].iter().any(|&to| {
                let cells = grid.cells_in(from, to);
                from = to;
                cells.x0 <= changed.x1
                    && changed.x0 <= cells.x1
                    && cells.y0 <= changed.y1
                    && changed.y0 <= cells.y1
            });
            if let (true, Some(target)) = (touched, unit.target) {
                let id = self.paths.request(
                    (&self.map, &self.terrain),
                    unit.position,
                    target,
                    unit.unit_type as usize,
                    vec![(unit.id, Vec2::ZERO)],
                );
                unit.route = Route::Waiting(id);
            }
        }
    }

    /// Gives the path queue its budget and hands finished paths to the
    /// units waiting for them.
    fn plan_paths(&mut self) {
        if self.paths.queued() == 0 {
            self.paths.stats.expanded_last_tick = 0;
            return;
        }
        // Known minefields of every player (prompt 035).
        let avoid_by = self.avoid_by_player();
        let units = &self.units;
        // Precompute active waiting request ids once instead of scanning all
        // units per queued request (prompt 211).
        let mut waiting: Vec<u32> = units
            .iter()
            .filter_map(|u| match u.route {
                Route::Waiting(id) => Some(id),
                _ => None,
            })
            .collect();
        waiting.sort_unstable();
        waiting.dedup();
        let wanted = |id: u32| waiting.binary_search(&id).is_ok();
        let avoid = |request: &Request| -> &[u32] {
            request
                .units
                .first()
                .and_then(|&(id, _)| units.binary_search_by_key(&id, |u| u.id).ok())
                .and_then(|i| avoid_by.iter().find(|(p, _)| *p == units[i].owner))
                .map_or(&[], |(_, cells)| cells.as_slice())
        };
        let finished = self.paths.run_avoiding(NODE_BUDGET_PER_TICK, wanted, avoid);
        for (request, path) in finished {
            self.take_path(&request, path.as_ref());
        }
    }

    /// Turns a group path into the route of every unit still waiting for it.
    ///
    /// A unit follows the group path shifted by its formation offset. Where
    /// a shifted leg is blocked it uses the unshifted waypoint (the group
    /// squeezes through); if that is blocked too, it gets a path of its own
    /// to its slot.
    fn take_path(&mut self, request: &Request, path: Option<&Path>) {
        let single = request.units.len() == 1;
        for &(id, offset) in &request.units {
            let Ok(index) = self.units.binary_search_by_key(&id, |u| u.id) else {
                continue;
            };
            // A squad behind a tank first walks into its track (prompt 198).
            let join = path.and_then(|p| self.escort_join(index, p));
            let unit = &mut self.units[index];
            if unit.route != Route::Waiting(request.id) {
                continue;
            }
            let Some(path) = path else {
                // Nowhere to go from here (prompt 173: reported).
                let target = unit.target.take();
                unit.route = Route::None;
                self.report_short(index, target);
                continue;
            };
            let grid = &self.paths.grid;
            let waypoints = if single {
                // The path starts at the unit, or at its lead-in point
                // ([`PathGrid::lead_in`]), which it drives to first.
                let from = usize::from(request.start == unit.position);
                Some(path.waypoints[from..].to_vec())
            } else {
                let profile = unit.unit_type as usize;
                formation_route(grid, &self.map, profile, unit.position, path, offset).map(
                    |mut route| {
                        let clear = |a: Vec2, b: Vec2| {
                            self.map.contains(b) && grid.line_of_sight_for(profile, a, b)
                        };
                        if let Some(join) = join
                            && clear(unit.position, join)
                            && route.first().is_some_and(|&next| clear(join, next))
                        {
                            route.insert(0, join);
                        }
                        route
                    },
                )
            };
            match waypoints {
                Some(waypoints) => {
                    unit.target = waypoints.last().copied();
                    unit.route = following(waypoints);
                }
                None => {
                    let slot = unit.target.unwrap_or(path.goal + offset);
                    let own = self.paths.request(
                        (&self.map, &self.terrain),
                        unit.position,
                        slot,
                        unit.unit_type as usize,
                        vec![(unit.id, Vec2::ZERO)],
                    );
                    unit.route = Route::Waiting(own);
                }
            }
        }
    }

    /// Sorts the units into [`spatial::UnitGrid`] by their current
    /// positions.
    fn rebuild_unit_grid(&mut self) {
        let size = self.map.size();
        let types = &self.types;
        self.unit_grid.rebuild(&self.units, size, |u| {
            combat::reach_of(u, &types[u.unit_type as usize])
        });
    }

    /// Moves every unit that follows a route by one tick, with separation
    /// from nearby units computed from the positions at the start of the
    /// movement (so the result does not depend on the update order).
    fn move_units(&mut self) {
        let mut scratch = std::mem::take(&mut self.move_scratch);
        // Units arriving at their final waypoint ignore separation, so a
        // parked unit next to the target cannot keep them from arriving.
        scratch.directions.clear();
        scratch.directions.extend(
            self.units
                .iter()
                .map(|u| heading(u).filter(|_| !arriving(u))),
        );
        // Current waypoints: units squeezing through the same one queue.
        scratch.waypoints.clear();
        scratch
            .waypoints
            .extend(self.units.iter().map(current_waypoint));
        // The unit grid holds the positions of this phase (built before
        // combat, which moves nobody).
        let overrun_kg = self.combat_rules.overrun.min_mass_kg;
        scratch.pushes.clear();
        for i in 0..self.units.len() {
            let push = scratch.directions[i].map_or(Vec2::ZERO, |d| {
                // Separation looks at most (own + other spread) far.
                let me = &self.units[i];
                let r = me.spread + self.unit_grid.max_spread;
                let r = Vec2::new(r, r);
                self.unit_grid
                    .query(me.position - r, me.position + r, &mut scratch.near);
                // Heavy vehicles do not swerve around enemy infantry:
                // they run it over (prompt 093).
                let runs_over = overrun_kg > 0
                    && me.squad.is_none()
                    && self.unit_types[me.unit_type as usize].mass_kg >= overrun_kg;
                let ignore = |other: &Unit| {
                    runs_over && other.squad.is_some() && !self.teams.allied(me.owner, other.owner)
                };
                // At bridges a column queues instead of pushing its
                // members off the deck's line (prompt 173).
                let narrow = near_bridge(&self.map, me.position);
                let lane = (&scratch.directions[..], &scratch.waypoints[..], narrow);
                separation(&self.units, lane, i, d, &scratch.near, ignore)
            });
            scratch.pushes.push(push);
        }
        // Groups drive together (prompt 187).
        self.march_caps_into(&mut scratch.caps);
        // Squads behind tanks (prompt 198).
        self.escort_caps(&mut scratch.caps);
        self.march_waiting.clear();
        self.march_waiting
            .extend(scratch.caps.iter().map(|c| *c == Some(Fixed::ZERO)));
        let grid = &self.paths.grid;
        let crush_slow_pct = self.destruction.crush_slow_pct();
        scratch.blocked.clear();
        scratch.stalled.clear();
        scratch.driven.clear();
        for (index, (unit, &push)) in self.units.iter_mut().zip(scratch.pushes.iter()).enumerate() {
            let profile = unit.unit_type as usize;
            let keep_clear = |p: Vec2| grid.passable_for(profile, p);
            let from = unit.position;
            let ground = (&self.terrain, grid.ground_pct(profile));
            // Slower for a moment after breaking through a wall (prompt 093).
            let speed_pct = unit.speed_pct;
            if unit.crush_slow > 0 {
                unit.crush_slow -= 1;
                if speed_pct > 0 {
                    unit.speed_pct = (speed_pct * crush_slow_pct / 100).max(1);
                }
            }
            let cap = scratch.caps.get(index).copied().flatten();
            let moved = if grid.class_of(profile) == crate::path::SizeClass::Boat {
                // Boats (prompt 119) drive on the water, not on the ground.
                boats::move_boat(unit, push, (&self.map, grid), profile)
            } else {
                move_unit(unit, (push, cap), &self.map, ground, keep_clear)
            };
            unit.speed_pct = speed_pct;
            if unit.route == Route::None {
                unit.drill.march = 0;
            }
            match moved {
                Moved::Blocked => scratch.blocked.push(index),
                Moved::Stalled(at) => scratch.stalled.push((index, at)),
                Moved::Ok => {}
            }
            // The hull points where the unit drove.
            let step = unit.position - from;
            let length = step.length();
            if length > Fixed::ZERO {
                unit.facing = Vec2::new(step.x / length, step.y / length);
                scratch.driven.push((index, length));
            }
        }
        self.overrun.driven.clear();
        for &(index, length) in &scratch.driven {
            self.burn_fuel(index, length);
            if self.units[index].squad.is_none() {
                self.overrun.driven.push((index as u32, length));
            }
        }
        // Pushed off the planned line (or the way is blocked): plan again
        // from here, a limited number of times.
        for &index in &scratch.blocked {
            let unit = &mut self.units[index];
            unit.replans += 1;
            match unit.target {
                Some(target) if unit.replans <= MAX_REPLANS => {
                    let id = self.paths.request(
                        (&self.map, &self.terrain),
                        unit.position,
                        target,
                        unit.unit_type as usize,
                        vec![(unit.id, Vec2::ZERO)],
                    );
                    unit.route = Route::Waiting(id);
                }
                target => {
                    stop(unit);
                    self.report_short(index, target);
                }
            }
        }
        for &(index, at) in &scratch.stalled {
            self.report_short(index, Some(at));
        }
        self.move_scratch = scratch;
    }

    fn apply(&mut self, command: &PlayerCommand) {
        let Some(command) = self.filter_retreat_command(command) else {
            return;
        };
        let command = &command;
        self.drop_retreat_on_order(command);
        self.drop_grenade_orders(command);
        self.drop_replaced_queues(command);
        self.drop_transport_tasks(command);
        self.drop_work_jobs(command);
        self.drop_drills(command);
        self.drop_crossing_jobs(command);
        self.drop_boat_orders(command);
        self.apply_command(command);
        self.boats_after_command(command);
    }

    /// Strips squads that are still locked into a retreat (`retreat_lock > 0`
    /// or under fire) from orders that would cancel their retreat (prompt 234).
    /// Returns `None` if no units remain for the command.
    fn filter_retreat_command(&self, command: &PlayerCommand) -> Option<PlayerCommand> {
        let replaces = matches!(
            command.command,
            Command::MoveTo { .. }
                | Command::Stop { .. }
                | Command::Attack { .. }
                | Command::AttackGround { .. }
                | Command::AttackObject { .. }
                | Command::SetStance { .. }
                | Command::Garrison { .. }
                | Command::Barrage { .. }
                | Command::QueueMove { .. }
                | Command::Patrol { .. }
                | Command::Embark { .. }
                | Command::Disembark { .. }
                | Command::FormationMove { .. }
                | Command::Secure { .. }
                | Command::Lay { .. }
                | Command::Dig { .. }
                | Command::Clear { .. }
                | Command::DemolishBridge { .. }
                | Command::RepairBridge { .. }
                | Command::ThrowGrenade { .. }
                | Command::ThrowSmokeGrenade { .. }
        );
        if !replaces {
            return Some(command.clone());
        }
        let ids = command.command.units();
        let locked = |id: &UnitId| {
            self.units
                .binary_search_by_key(id, |u| u.id)
                .ok()
                .is_some_and(|i| self.units[i].owner == command.player && self.retreat_locked(i))
        };
        if !ids.iter().any(locked) {
            return Some(command.clone());
        }
        let mut filtered = command.clone();
        let list = match &mut filtered.command {
            Command::MoveTo { units, .. }
            | Command::Stop { units }
            | Command::Attack { units, .. }
            | Command::AttackGround { units, .. }
            | Command::AttackObject { units, .. }
            | Command::SetStance { units, .. }
            | Command::Garrison { units, .. }
            | Command::Barrage { units, .. }
            | Command::QueueMove { units, .. }
            | Command::Patrol { units, .. }
            | Command::Embark { units, .. }
            | Command::Disembark { units }
            | Command::FormationMove { units, .. }
            | Command::Secure { units, .. }
            | Command::Lay { units, .. }
            | Command::Dig { units, .. }
            | Command::Clear { units, .. }
            | Command::DemolishBridge { units, .. }
            | Command::RepairBridge { units, .. }
            | Command::ThrowGrenade { units, .. }
            | Command::ThrowSmokeGrenade { units, .. } => units,
            _ => return Some(filtered),
        };
        list.retain(|id| !locked(id));
        (!list.is_empty()).then_some(filtered)
    }

    /// Clears the retreat state of unlocked retreating squads when a new
    /// order takes over (prompt 234).
    fn drop_retreat_on_order(&mut self, command: &PlayerCommand) {
        if !matches!(
            command.command,
            Command::MoveTo { .. }
                | Command::Stop { .. }
                | Command::Attack { .. }
                | Command::AttackGround { .. }
                | Command::AttackObject { .. }
                | Command::SetStance { .. }
                | Command::Garrison { .. }
                | Command::Barrage { .. }
                | Command::QueueMove { .. }
                | Command::Patrol { .. }
                | Command::Embark { .. }
                | Command::FormationMove { .. }
                | Command::Secure { .. }
                | Command::Lay { .. }
                | Command::Dig { .. }
                | Command::Clear { .. }
                | Command::DemolishBridge { .. }
                | Command::RepairBridge { .. }
                | Command::ThrowGrenade { .. }
                | Command::ThrowSmokeGrenade { .. }
        ) {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            self.clear_retreat(i);
        }
    }

    fn apply_command(&mut self, command: &PlayerCommand) {
        match &command.command {
            Command::MoveTo { units, target } => {
                if self.reject_movement(command.player, units, *target) {
                    return;
                }
                let mut group = self.mobile_indices(command.player, units);
                for &i in &group {
                    self.leave_building(i);
                }
                let footprint = group
                    .iter()
                    .map(|&i| self.units[i].spread)
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
                let target = *target;
                // Prompt 174: then by rows along the way to the target, so a
                // unit in front takes a slot in front.
                let slots = travel_order(&mut group, slots, &self.units, target);
                for &i in &group {
                    self.units[i].combat.attack = None;
                    self.cancel_indirect(i);
                }
                let mut assigned: Vec<(usize, Vec2)> = group.into_iter().zip(slots).collect();
                // Prompt 174: no unit crosses another's way to its slot.
                let starts: Vec<Vec2> = assigned
                    .iter()
                    .map(|&(i, _)| self.units[i].position)
                    .collect();
                crate::formation::untangle(&mut assigned, &starts);
                self.request_group_path(&assigned, target);
            }
            Command::Stop { units } => {
                for index in self.owned_indices(command.player, units) {
                    stop(&mut self.units[index]);
                    self.units[index].combat.attack = None;
                    self.cancel_enter(index);
                    self.cancel_indirect(index);
                }
            }
            Command::Attack { units, target } => {
                let Ok(t) = self.units.binary_search_by_key(target, |u| u.id) else {
                    self.reject(command.player, RejectReason::TargetGone, Some(*target));
                    return;
                };
                if !self.units[t].alive() {
                    self.reject(command.player, RejectReason::TargetGone, Some(*target));
                    return;
                }
                if self.teams.allied(self.units[t].owner, command.player) {
                    self.reject(command.player, RejectReason::TargetFriendly, Some(*target));
                    return;
                }
                self.reject_attack(command.player, units, t);
                let guns =
                    self.order_indirect(command.player, units, indirect::Order::Unit(*target));
                for index in self.mobile_indices(command.player, units) {
                    if !guns.contains(&index) {
                        self.start_attack(index, Attack::Unit(*target));
                    }
                }
            }
            Command::AttackGround { units, target } => {
                if !within_world(*target) {
                    self.reject(command.player, RejectReason::OutsideMap, None);
                    return;
                }
                let guns =
                    self.order_indirect(command.player, units, indirect::Order::Ground(*target));
                for index in self.mobile_indices(command.player, units) {
                    if !guns.contains(&index) {
                        self.start_attack(index, Attack::Ground(*target));
                    }
                }
            }
            Command::AttackObject { units, object } => {
                if !self.shellable(*object) {
                    self.reject(command.player, RejectReason::NotShellable, None);
                    return;
                }
                let shooters: Vec<usize> = self
                    .mobile_indices(command.player, units)
                    .into_iter()
                    .filter(|&i| self.shells_objects(i))
                    .collect();
                if shooters.is_empty() && !self.owned_indices(command.player, units).is_empty() {
                    self.reject(command.player, RejectReason::NoObjectWeapon, None);
                }
                for index in shooters {
                    self.start_attack(index, Attack::Object(*object));
                }
            }
            Command::SetFireMode { units, mode } => {
                for index in self.owned_indices(command.player, units) {
                    self.units[index].combat.fire_mode = *mode;
                    if *mode != FireMode::FireAtWill {
                        self.units[index].combat.auto_target = None;
                    }
                }
            }
            Command::Land { units, target } => self.order_land(command.player, units, *target),
            Command::Retract { units } => self.order_retract(command.player, units),
            Command::SetFireLimits {
                units,
                close_only,
                vehicles_only,
            } => self.order_fire_limits(
                command.player,
                units,
                FireLimits {
                    close_only: *close_only,
                    vehicles_only: *vehicles_only,
                },
            ),
            Command::Surrender => {
                // Prompt 190: in a cooperative mission the units go to the
                // ally who plays on.
                self.hand_over_on_surrender(command.player);
                self.standings.surrender(command.player, self.tick + 1);
            }
            Command::SetStance { units, stance } => {
                for index in self.owned_indices(command.player, units) {
                    self.set_stance(index, *stance);
                }
            }
            Command::Garrison { units, building } => {
                if self.reject_immobile(command.player, units) {
                    return;
                }
                let indices = self.mobile_indices(command.player, units);
                if indices.iter().all(|&i| self.units[i].squad.is_none()) {
                    self.reject(command.player, RejectReason::NoSquads, None);
                    return;
                }
                if self.infantry.building(&self.map, *building).is_none() {
                    self.reject(command.player, RejectReason::NotABuilding, None);
                    return;
                }
                self.order_garrison(&indices, *building);
            }
            Command::Build {
                unit_type,
                position,
            } => self.order_build(command.player, *unit_type, *position),
            Command::Produce {
                building,
                unit_type,
            } => self.order_produce(command.player, *building, *unit_type),
            Command::CancelProduction { building } => self.order_cancel(command.player, *building),
            Command::SetRallyPoint { building, point } => {
                self.order_rally(command.player, *building, *point);
            }
            Command::Barrage {
                units,
                target,
                radius_m,
                salvos,
                smoke,
            } => self.order_barrage(command.player, units, *target, (*radius_m, *salvos, *smoke)),
            Command::QueueMove { units, target } => {
                self.order_queue_move(command.player, units, *target);
            }
            Command::Patrol { units, target } => self.order_patrol(command.player, units, *target),
            Command::CallAir { mission, target } => {
                self.order_air(command.player, *mission, *target);
            }
            // Presentation only (prompt 037): the games of the allies show it.
            Command::Ping { .. } => {}
            Command::Embark { units, carrier } => {
                self.order_embark(command.player, units, *carrier);
            }
            Command::Disembark { units } => self.order_disembark(command.player, units),
            Command::Tow { units, gun } => self.order_tow(command.player, units, *gun),
            Command::Unlimber { units } => self.order_unlimber(command.player, units),
            Command::Repair { building } => self.order_repair(command.player, *building),
            Command::Sell { building } => self.order_sell(command.player, *building),
            Command::Lay {
                units,
                kind,
                from,
                to,
            } => self.order_works(command.player, units, (*kind).into(), (*from, *to)),
            Command::Dig {
                units,
                kind,
                from,
                to,
            } => self.order_works(command.player, units, (*kind).into(), (*from, *to)),
            Command::Clear { units, target } => self.order_clear(command.player, units, *target),
            Command::DemolishBridge { units, bridge } => {
                self.order_bridge(command.player, units, *bridge, false);
            }
            Command::RepairBridge { units, bridge } => {
                self.order_bridge(command.player, units, *bridge, true);
            }
            Command::FormationMove {
                units,
                target,
                formation,
                facing,
                engage,
            } => {
                self.order_formation_move(
                    command.player,
                    units,
                    *target,
                    *formation,
                    *facing,
                    *engage,
                );
            }
            Command::Secure {
                units,
                center,
                radius_m,
                formation,
                facing,
            } => self.order_secure(
                command.player,
                units,
                *center,
                *radius_m,
                *formation,
                *facing,
            ),
            Command::LayBridge { units, target } => {
                self.order_lay_bridge(command.player, units, *target);
            }
            Command::PickUpBridge { units } => self.order_pick_up_bridge(command.player, units),
            Command::SetCamouflage { units, on } => {
                self.order_camouflage(command.player, units, *on);
            }
            Command::CallAirLine { mission, from, to } => {
                self.order_air_line(command.player, *mission, (*from, *to));
            }
            Command::HeavyStrike { target } => self.order_strike(command.player, *target),
            Command::MissionSignal { name } => self.mission_signal(command.player, name),
            Command::Retreat { units } => self.order_retreat(command.player, units),
            Command::ThrowGrenade { units, target } => {
                self.order_throw_grenade(command.player, units, *target, false);
            }
            Command::ThrowSmokeGrenade { units, target } => {
                self.order_throw_grenade(command.player, units, *target, true);
            }
        }
    }

    /// [`Self::owned_indices`] without units that cannot move (buildings,
    /// prompt 023; towed guns, prompt 034).
    fn mobile_indices(&self, player: PlayerId, ids: &[UnitId]) -> Vec<usize> {
        let mut indices = self.owned_indices(player, ids);
        indices
            .retain(|&i| self.units[i].speed_offroad > Fixed::ZERO && self.units[i].link.is_none());
        indices
    }

    /// Queues one path for a group (unit indices with their formation slots)
    /// to `target`. It starts at the unit nearest to the group's center and
    /// uses the profile of the most restrictive unit (size class, then the
    /// bridges it may not cross, then the largest), so it fits every member.
    fn request_group_path(&mut self, assigned: &[(usize, Vec2)], target: Vec2) {
        // Boats and land units never share a path (prompt 119).
        let boat = |i: usize| self.unit_types[self.units[i].unit_type as usize].is_boat();
        if assigned.iter().any(|&(i, _)| boat(i)) && !assigned.iter().all(|&(i, _)| boat(i)) {
            let (water, land): (Vec<_>, Vec<_>) = assigned.iter().partition(|&&(i, _)| boat(i));
            self.request_group_path(&water, target);
            self.request_group_path(&land, target);
            return;
        }
        let count = Fixed::from_num(assigned.len());
        let center = assigned.iter().fold(Vec2::ZERO, |sum, &(i, _)| {
            let p = self.units[i].position;
            sum + Vec2::new(p.x / count, p.y / count)
        });
        let distance_bits = |p: Vec2| {
            let d = p - center;
            let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
            x * x + y * y
        };
        let anchor = assigned
            .iter()
            .map(|&(i, _)| &self.units[i])
            .min_by_key(|u| (distance_bits(u.position), u.id))
            .map_or(target, |u| u.position);
        let grid = &self.paths.grid;
        let profile = assigned
            .iter()
            .map(|&(i, _)| &self.units[i])
            .max_by_key(|u| {
                let restriction = grid.restriction(u.unit_type as usize);
                (restriction, u.footprint, std::cmp::Reverse(u.id))
            })
            .map_or(0, |u| u.unit_type as usize);
        let members = assigned
            .iter()
            .map(|&(i, slot)| (self.units[i].id, slot - target))
            .collect();
        let id = self
            .paths
            .request((&self.map, &self.terrain), anchor, target, profile, members);
        for &(i, slot) in assigned {
            self.units[i].target = Some(slot);
            self.units[i].route = Route::Waiting(id);
            self.units[i].replans = 0;
        }
    }

    /// Indices into `self.units` of the listed units that exist and belong to
    /// `player`, sorted and without duplicates. Passengers (prompt 034) take
    /// no orders but `Disembark`.
    fn owned_indices(&self, player: PlayerId, ids: &[UnitId]) -> Vec<usize> {
        let mut indices: Vec<usize> = ids
            .iter()
            .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
            .filter(|&index| {
                let u = &self.units[index];
                u.owner == player && u.alive() && !transport::aboard(u)
            })
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
        h.write_u64(self.map.hash());
        self.map.hash_bridge_state(&mut h);
        self.terrain.hash_into(&mut h);
        self.paths.hash_into(&mut h);
        h.write_u64(self.units.len() as u64);
        for unit in &self.units {
            h.write_u32(unit.id.0);
            h.write_u8(unit.owner.0);
            h.write_u32(unit.unit_type);
            h.write_i64(unit.position.x.to_bits());
            h.write_i64(unit.position.y.to_bits());
            h.write_i64(unit.height.to_bits());
            match unit.target {
                None => h.write_u8(0),
                Some(target) => {
                    h.write_u8(1);
                    h.write_i64(target.x.to_bits());
                    h.write_i64(target.y.to_bits());
                }
            }
            match &unit.route {
                Route::None => h.write_u8(0),
                Route::Waiting(id) => {
                    h.write_u8(1);
                    h.write_u32(*id);
                }
                Route::Following {
                    waypoints,
                    next,
                    best,
                    stalled,
                } => {
                    h.write_u8(2);
                    h.write_u64(*next as u64);
                    h.write_i64(best.to_bits());
                    h.write_u32(*stalled);
                    h.write_u64(waypoints.len() as u64);
                    for w in waypoints {
                        h.write_i64(w.x.to_bits());
                        h.write_i64(w.y.to_bits());
                    }
                }
            }
            h.write_u32(unit.replans);
            h.write_i64(unit.speed_offroad.to_bits());
            h.write_i64(unit.speed_road.to_bits());
            h.write_i64(unit.footprint.to_bits());
            h.write_i64(unit.facing.x.to_bits());
            h.write_i64(unit.facing.y.to_bits());
            h.write_u32(unit.hp);
            unit.combat.hash_into(&mut h);
            h.write_u32(unit.speed_pct);
            h.write_i64(unit.spread.to_bits());
            h.write_u32(unit.elevation_mm.cast_unsigned());
            h.write_u8(u8::from(unit.vision_dirty));
            h.write_u32(unit.crush_slow);
            unit.progress.hash_into(&mut h);
            unit.under_fire.hash_into(&mut h);
            unit.crew.hash_into(&mut h);
            match &unit.squad {
                None => h.write_u8(0),
                Some(squad) => {
                    h.write_u8(1);
                    squad.hash_into(&mut h);
                }
            }
        }
        self.combat_rules.hash_into(&mut h);
        self.infantry.rules.hash_into(&mut h);
        h.write_u32(self.next_projectile);
        h.write_u64(self.projectiles.len() as u64);
        for p in &self.projectiles {
            p.hash_into(&mut h);
        }
        self.standings.hash_into(&mut h);
        self.vision.hash_into(&mut h);
        self.destruction.hash_into(&mut h);
        self.craters.hash_into(&mut h);
        self.economy.hash_into(&mut h);
        self.artillery.hash_into(&mut h);
        self.fire.hash_into(&mut h);
        self.hash_queues(&mut h);
        self.air.hash_into(&mut h, &self.paratrooper_types());
        self.teams.hash_into(&mut h);
        self.hash_transport(&mut h);
        self.hash_logistics(&mut h);
        self.hash_drills(&mut h);
        self.hash_alerts(&mut h);
        // Clear weather by day changes nothing and is left out, so matches
        // without weather keep their hashes (prompt 040).
        if !self.weather.is_default() {
            self.weather.hash_into(&mut h);
        }
        self.works.hash_into(&mut h);
        self.tactics.hash_into(&mut h);
        self.hash_experience(&mut h);
        self.crossings.hash_into(&mut h);
        self.hash_care(&mut h);
        // Only matches with a boat type (prompt 119).
        self.boats.hash_into(&mut h);
        // Only matches with grenade activity (prompt 232).
        self.grenades.hash_into(&mut h);
        // Left out until the first strike, so other matches keep their
        // hashes (prompt 054).
        if self.strikes.used() {
            self.strikes.hash_into(&mut h);
        }
        // Only matches with a mission (prompt 039): the others keep their hash.
        if let Some(m) = &self.mission {
            m.hash_into(&mut h);
        }
        h.finish()
    }

    /// Data needed to draw the current state.
    pub fn snapshot(&self) -> Snapshot {
        let mut out = Snapshot::default();
        self.snapshot_into(None, &mut out);
        out
    }

    /// Writes the snapshot into `out`, reusing its buffers (prompt 016: the
    /// bridge builds one per tick). With `player`, enemy units (and their
    /// soldiers) that the player does not see are left out, like
    /// [`Self::snapshot_for`].
    pub fn snapshot_into(&self, player: Option<PlayerId>, out: &mut Snapshot) {
        self.snapshot_into_parts(player, true, out);
    }

    /// Writes only the unit and aircraft parts of `player`'s snapshot into
    /// `out` (reusing its buffers) for the AI's [`PlayerView`], skipping
    /// soldier, projectile, event, shell, smoke, bomb and sortie snapshots.
    pub fn snapshot_for_ai_into(&self, player: PlayerId, out: &mut Snapshot) {
        self.snapshot_into_parts(Some(player), false, out);
    }

    fn snapshot_into_parts(&self, player: Option<PlayerId>, full: bool, out: &mut Snapshot) {
        let shown = |u: &Unit| !self.sold(u.id) && player.is_none_or(|p| self.visible_to(p, u.id));
        let loads = self.transport_loads();
        out.tick = self.tick;
        out.units.clear();
        out.units
            .extend(self.units.iter().filter(|u| shown(u)).map(|u| {
                UnitSnapshot {
                    id: u.id,
                    owner: u.owner,
                    unit_type: u.unit_type,
                    position: u.position,
                    height: u.height,
                    target: u.target,
                    facing: u.facing,
                    aim: u.combat.aim,
                    hp: u.hp,
                    max_hp: self.types[u.unit_type as usize].max_hp,
                    fire_mode: u.combat.fire_mode,
                    fire_limits: u.combat.limits,
                    stance: u.squad.as_ref().map(|s| s.stance),
                    garrison: u.squad.as_ref().and_then(|s| s.garrison),
                    changing_stance: u.squad.as_ref().is_some_and(|s| s.changing > 0),
                    spread: u.spread,
                    carrier: transport::carrier_of(u),
                    passengers: transport::load_of(&loads, u.id).0,
                    towing: transport::load_of(&loads, u.id).1,
                    loading: self.transport_busy(u.id),
                    stock: self.stock_view(u),
                    experience: self.experience.view(u.veterancy),
                    alert: u
                        .alert
                        .direction()
                        .filter(|_| player.is_none_or(|p| p == u.owner)),
                }
            }));
        out.projectiles.clear();
        out.events.clear();
        out.soldiers.clear();
        out.shells.clear();
        out.smoke.clear();
        if full {
            out.projectiles
                .extend(self.projectiles.iter().map(|p| p.snapshot(self.tick_hz)));
            out.events.extend_from_slice(&self.events);
            out.soldiers.extend(
                self.units
                    .iter()
                    .filter(|u| shown(u) && !transport::aboard(u))
                    .filter_map(|u| u.squad.as_ref().map(|s| (u, s)))
                    .flat_map(|(u, s)| {
                        let inside = s.garrison.is_some();
                        (0u16..).zip(&s.soldiers).map(move |(index, x)| {
                            let (wound, aid) = self.care_view(u, usize::from(index), x);
                            SoldierSnapshot {
                                unit: u.id,
                                index,
                                position: x.position,
                                height: x.height,
                                hp: x.hp,
                                in_cover: inside || x.at_spot(),
                                knocked: x.knocked,
                                knock_from: x.knock_from,
                                throw_from: x.throw_from,
                                throw_tick: x.throw_tick,
                                flight: x.flight,
                                cover: x.cover.filter(|_| !inside && x.at_spot()),
                                wound,
                                aid,
                            }
                        })
                    }),
            );
            self.artillery
                .shell_snapshots(self.tick_hz, &mut out.shells);
            self.artillery.smoke_snapshots(self.tick, &mut out.smoke);
            self.air_snapshots(player, &mut out.aircraft, &mut out.bombs, &mut out.sorties);
        } else {
            out.bombs.clear();
            out.sorties.clear();
            self.air_snapshots(player, &mut out.aircraft, &mut out.bombs, &mut out.sorties);
            out.bombs.clear();
            out.sorties.clear();
        }
    }
}

/// Units of `player` that are not destroyed (see [`Sim::alive_units`]);
/// bailed-out crews (`crew`, prompt 049) do not count.
fn alive_units(units: &[Unit], player: PlayerId, crew: impl Fn(&Unit) -> bool) -> usize {
    units
        .iter()
        .filter(|u| u.owner == player && u.alive() && !crew(u))
        .count()
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

/// How far (in path cells) a blocked formation slot may move to the
/// nearest reachable spot.
const SLOT_SEARCH_CELLS: u32 = 4;

/// Route of one unit along a group path, shifted by its formation `offset`.
///
/// Every shifted waypoint must be reachable in a straight line from the
/// previous one; otherwise the unshifted waypoint is used. The last waypoint
/// is the shifted goal (the unit's slot), or the nearest spot to it that is
/// visible from the group's end point. `None` if none of this works.
fn formation_route(
    grid: &PathGrid,
    map: &Map,
    profile: usize,
    from: Vec2,
    path: &Path,
    offset: Vec2,
) -> Option<Vec<Vec2>> {
    let clear = |a: Vec2, b: Vec2| map.contains(b) && grid.line_of_sight_for(profile, a, b);
    let points = &path.waypoints[1..];
    let last = points.len() - 1;
    let mut route = Vec::with_capacity(points.len() + 1);
    let mut previous = from;
    let start = path.waypoints[0];
    for (k, &point) in points.iter().enumerate() {
        let shifted = point + offset;
        // A group waypoint close to the group's start only re-forms the
        // group; members skip it when they see the next one.
        let near_start = (point - start).length() < grid.cell_size() * 2;
        if k < last && near_start && clear(previous, points[k + 1] + offset) {
            continue;
        }
        if clear(previous, shifted) {
            route.push(shifted);
            previous = shifted;
        } else if k == last && clear(previous, point) {
            // The slot lies behind an obstacle or in it: the nearest spot
            // that is visible from the group's end point.
            let spot = if clear(point, shifted) {
                shifted
            } else {
                grid.nearest_visible(profile, point, shifted, SLOT_SEARCH_CELLS)?
            };
            // Straight to the spot if possible: members that share the
            // group's end point as a waypoint would crowd it.
            if !clear(previous, spot) {
                route.push(point);
            }
            route.push(spot);
        } else if k != last && clear(previous, point) {
            route.push(point);
            previous = point;
        } else {
            return None;
        }
    }
    Some(route)
}

/// Unit-length direction towards the current waypoint of a unit that follows
/// a route.
fn heading(unit: &Unit) -> Option<Vec2> {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return None;
    };
    let delta = waypoints[*next] - unit.position;
    let distance = delta.length();
    // delta / distance has components in [-1, 1], so this cannot overflow.
    (distance > Fixed::ZERO).then(|| Vec2::new(delta.x / distance, delta.y / distance))
}

/// Reorders `group` and `slots` (prompt 174) so that the unit most
/// forward along the way from the group's center to `target` comes first
/// and gets the most forward slot: both are cut into ranks of the grid's
/// width along that way, and each rank sorted across it (ties: unit id,
/// slot order). Returns the slots in the new order.
fn travel_order(group: &mut [usize], slots: Vec<Vec2>, units: &[Unit], target: Vec2) -> Vec<Vec2> {
    let n = group.len().min(slots.len());
    if n < 2 {
        return slots;
    }
    let count = Fixed::from_num(n);
    let center = group.iter().fold(Vec2::ZERO, |sum, &i| {
        let p = units[i].position;
        sum + Vec2::new(p.x / count, p.y / count)
    });
    let Some(forward) = crate::formation::normalized(target - center) else {
        return slots;
    };
    let right = crate::formation::right_of(forward);
    let along = |p: Vec2| (p.x - center.x) * forward.x + (p.y - center.y) * forward.y;
    let across = |p: Vec2| (p.x - center.x) * right.x + (p.y - center.y) * right.y;
    let columns = (1..).find(|c| c * c >= n).unwrap_or(1);
    let mut u: Vec<(Fixed, Fixed, UnitId, usize)> = group
        .iter()
        .map(|&i| {
            (
                along(units[i].position),
                across(units[i].position),
                units[i].id,
                i,
            )
        })
        .collect();
    u.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    let mut s: Vec<(Fixed, Fixed, usize, Vec2)> = slots
        .iter()
        .enumerate()
        .map(|(k, &p)| (along(p), across(p), k, p))
        .collect();
    s.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    for chunk in u.chunks_mut(columns) {
        chunk.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
    }
    for chunk in s.chunks_mut(columns) {
        chunk.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
    }
    for (k, e) in u.iter().enumerate() {
        group[k] = e.3;
    }
    s.into_iter().map(|e| e.3).collect()
}

/// The waypoint a unit drives to now.
fn current_waypoint(unit: &Unit) -> Option<Vec2> {
    match &unit.route {
        Route::Following {
            waypoints, next, ..
        } => waypoints.get(*next).copied(),
        _ => None,
    }
}

/// Whether a unit following a route is within one footprint of its target.
fn arriving(unit: &Unit) -> bool {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return false;
    };
    *next + 1 == waypoints.len() && (waypoints[*next] - unit.position).length() < unit.footprint
}

/// Separation of unit `i` (driving in `direction`) from all other units.
///
/// - **Push:** away from every unit closer than half of both footprints
///   (`reach`), growing linearly to one unit length at full overlap.
/// - **Swerve:** sideways from units ahead within twice the reach whose
///   center is less than `reach` off the own line, away from the side they
///   are on (straight ahead: to the left of the heading, the same rule for
///   everybody, so two units meeting head-on pass each other). Units ahead
///   that drive in roughly the same direction (`directions`, less than 60°
///   apart) are not swerved around: a column follows its leader instead of
///   zig-zagging behind it. The push still keeps them from overlapping.
/// - **Queue** (prompt 173): between units driving the same way, the push
///   acts only along the own line when both head for the same waypoint (a
///   group squeezing through a gap or onto a bridge) or the unit is near a
///   bridge (`lane.2`, [`near_bridge`]): they wait behind each other
///   instead of pushing each other aside onto a bank or beside a deck.
///   Near a bridge squads count only `FILE_WIDTH_M` wide (in file).
///
/// `lane`: directions and current waypoints of all units, and whether unit
/// `i` is near a bridge. `near` lists the candidates (indices; every unit within the reach must
/// be among them). The push is a sum, so their order does not matter.
fn separation(
    units: &[Unit],
    lane: (&[Option<Vec2>], &[Option<Vec2>], bool),
    i: usize,
    direction: Vec2,
    near: &[u32],
    ignore: impl Fn(&Unit) -> bool,
) -> Vec2 {
    let (directions, waypoints, narrow) = lane;
    let me = &units[i];
    let left = Vec2::new(direction.y, -direction.x);
    let mut push = Vec2::ZERO;
    for &j in near {
        let j = j as usize;
        let other = &units[j];
        if j == i
            || !other.alive()
            || infantry::inside(other)
            || other.link.is_some()
            || ignore(other)
        {
            continue;
        }
        // Near a bridge a squad crosses in file, not in its line
        // formation (prompt 173): oncoming squads pass each other on the
        // deck instead of swerving onto the bank beside it.
        let width = |u: &Unit| {
            if narrow && u.squad.is_some() {
                u.spread.min(Fixed::from_num(FILE_WIDTH_M))
            } else {
                u.spread
            }
        };
        let reach = (width(me) + width(other)) / 2;
        let look = reach * 2;
        // From me to the other unit.
        let d = other.position - me.position;
        if d.x.abs() >= look || d.y.abs() >= look {
            continue;
        }
        let distance = d.length();
        if distance >= look {
            continue;
        }
        let same_way =
            directions[j].is_some_and(|o| o.x * direction.x + o.y * direction.y > Fixed::ONE / 2);
        if distance < reach {
            let weight = (reach - distance) / reach;
            let away = if distance == Fixed::ZERO {
                // Same spot: the lower id gives way towards -x.
                Vec2::new(
                    if me.id < other.id {
                        -Fixed::ONE
                    } else {
                        Fixed::ONE
                    },
                    Fixed::ZERO,
                )
            } else {
                Vec2::new(-d.x / distance, -d.y / distance)
            };
            let away = away.scale(weight);
            push = push
                + if same_way && (narrow || waypoints[i] == waypoints[j]) {
                    // Only along the own line: wait behind, not beside.
                    direction.scale(away.x * direction.x + away.y * direction.y)
                } else {
                    away
                };
        }
        let ahead = d.x * direction.x + d.y * direction.y;
        // Positive: the other unit is on my left.
        let lateral = d.x * left.x + d.y * left.y;
        if ahead > Fixed::ZERO && lateral.abs() < reach && !same_way {
            let weight = (look - distance) / look;
            let side = if lateral > Fixed::ZERO {
                -Fixed::ONE
            } else {
                Fixed::ONE
            };
            push = push + left.scale(side * weight);
        }
    }
    push
}

/// Whether `p` lies on or near the deck of an intact bridge (within
/// `NEAR_BRIDGE_M` of its ends and sides), where units of a column queue
/// instead of pushing each other sideways ([`separation`]).
fn near_bridge(map: &Map, p: Vec2) -> bool {
    bridge_within(map, p, Fixed::from_num(NEAR_BRIDGE_M)).is_some()
}

/// Center of the first intact bridge whose deck lies within `margin` of
/// `p` (along its ends and sides).
fn bridge_within(map: &Map, p: Vec2, margin: Fixed) -> Option<Vec2> {
    map.bridges()
        .iter()
        .find(|b| {
            if !b.intact {
                return false;
            }
            let d = p - b.center;
            let reach = b.half_length.max(b.half_width) + margin;
            if d.x.abs() > reach || d.y.abs() > reach {
                return false;
            }
            let along = (d.x * b.forward.x + d.y * b.forward.y).abs();
            let across = (d.x * b.right.x + d.y * b.right.y).abs();
            along <= b.half_length + margin && across <= b.half_width + margin
        })
        .map(|b| b.center)
}

/// See [`near_bridge`] (meters).
const NEAR_BRIDGE_M: i32 = 25;

/// Width of a squad in file near a bridge, for [`separation`] (meters).
const FILE_WIDTH_M: i32 = 4;

/// Result of [`move_unit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Moved {
    Ok,
    /// The way to the next waypoint is blocked (impassable terrain, the map
    /// edge or a grade too steep): the unit did not move.
    Blocked,
    /// It stopped on its last waypoint (this one) without reaching it.
    Stalled(Vec2),
}

/// Moves a unit one tick towards its current waypoint, at the speed of the
/// terrain under it and the grade ahead, deflected by `push` (separation;
/// the step never gets longer than the speed).
///
/// The unit never enters impassable terrain, leaves the map, climbs over
/// the side of a bridge deck ([`Map::deck_step_ok`]) or takes a step
/// steeper than the grade limit. If neither
/// the pushed nor the direct step is possible, it slides along the obstacle
/// (only the x or only the y part of the step); if that fails too, it
/// reports [`Moved::Blocked`]. Every step except the last one onto the
/// waypoint must also stay on cells that `clear` accepts (passable for the
/// unit's size class) unless the unit already stands on a blocked cell
/// (since prompt 173 also the direct one: a unit pushed off its leg).
/// `ground`: the terrain rules and the unit's soft-ground speeds per
/// terrain id (prompt 040, [`PathGrid::ground_pct`]).
fn move_unit(
    unit: &mut Unit,
    (push, cap): (Vec2, Option<Fixed>),
    map: &Map,
    ground: (&TerrainRules, &[u32]),
    clear: impl Fn(Vec2) -> bool,
) -> Moved {
    let (rules, ground_pct) = ground;
    // Waiting in front of a bridge for its group (prompt 187): like a
    // halt, no step and no stall.
    if cap == Some(Fixed::ZERO) {
        return Moved::Ok;
    }
    // Driving with its group: no faster than the cap.
    let capped = |speed: Fixed| cap.map_or(speed, |c| speed.min(c));
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return Moved::Ok;
    };
    // Halted (stance change, a fire team's bound, prompt 047): no step and
    // no stall, the route waits.
    if unit.speed_pct == 0 {
        return Moved::Ok;
    }
    let (waypoint, last) = (waypoints[*next], *next + 1 == waypoints.len());
    if let Some(direction) = heading(unit) {
        let ahead = ground_speed(unit, direction, map, rules, ground_pct).map(capped);
        let (from, from_height) = (unit.position, unit.height);
        let max_grade = Fixed::from_num(rules.slope.max_grade_pct) / 100;
        // No step steeper than the grade limit, up or down: separation must
        // not push a unit onto a cliff (an abutment, a steep bank).
        let valid = |p: Vec2| {
            map.contains(p)
                && rules.types[usize::from(map.terrain_at(p))].passable
                && map.deck_step_ok(from, p)
                && (map.height_at(p) - from_height).abs() <= (p - from).length() * max_grade
        };
        let off_grid = !clear(unit.position);
        let detour_ok = |p: Vec2| off_grid || clear(p);
        let straight = ahead.and_then(|speed| {
            let candidates = if (waypoint - unit.position).length() <= speed {
                [Some(waypoint), None, None, None]
            } else {
                let mut velocity = direction + push;
                let length = velocity.length();
                if length > Fixed::ONE {
                    velocity = Vec2::new(velocity.x / length, velocity.y / length);
                }
                let step = direction.scale(speed);
                let pushed = unit.position + velocity.scale(speed);
                // A slide shorter than a quarter of the step is no way
                // forward: the unit would creep along the obstacle by
                // millimeters instead of planning again (prompt 173).
                let long = |d: Fixed| d.abs() * 4 >= speed;
                let slide_x =
                    long(step.x).then_some(unit.position + Vec2::new(step.x, Fixed::ZERO));
                let slide_y =
                    long(step.y).then_some(unit.position + Vec2::new(Fixed::ZERO, step.y));
                [
                    Some(pushed).filter(|&p| detour_ok(p)),
                    // Also the direct step: a unit pushed off its leg must
                    // not drive onto a cell its class cannot use (a steep
                    // bank beside a deck), from where no search starts
                    // (prompt 173).
                    Some(unit.position + step).filter(|&p| detour_ok(p)),
                    slide_x.filter(|&p| detour_ok(p)),
                    slide_y.filter(|&p| detour_ok(p)),
                ]
            };
            candidates.into_iter().flatten().find(|&p| valid(p))
        });
        // Too steep or blocked ahead: take it at an angle, like a tank that
        // climbs a bank it cannot take head-on. Without this a unit pushed
        // onto a slope steeper than the limit (the bank of a ford's deep
        // channel) could neither climb back nor go anywhere else.
        let next_position = straight.or_else(|| {
            angled(direction).into_iter().find_map(|d| {
                let speed = capped(ground_speed(unit, d, map, rules, ground_pct)?);
                let p = unit.position + d.scale(speed);
                (detour_ok(p) && valid(p)).then_some(p)
            })
        });
        let Some(next_position) = next_position else {
            return Moved::Blocked;
        };
        unit.position = next_position;
        unit.height = map.height_at(next_position);
    }
    advance_route(unit, waypoint, last)
}

/// After a step towards `waypoint` (the `last` one of the route, or not):
/// counts progress, moves on to the next waypoint or stops on the last.
fn advance_route(unit: &mut Unit, waypoint: Vec2, last: bool) -> Moved {
    let distance = (waypoint - unit.position).length();
    let Route::Following {
        next,
        best,
        stalled,
        ..
    } = &mut unit.route
    else {
        return Moved::Ok;
    };
    let reached =
        distance == Fixed::ZERO || (!last && distance <= Fixed::from_num(WAYPOINT_RADIUS_M));
    // Progress counts only in steps of `MIN_PROGRESS_MM`: a unit that
    // creeps forward by millimeters (pushed back by separation almost as
    // far as it drives) is stalled, not arriving (prompt 173).
    if distance + Fixed::from_num(MIN_PROGRESS_MM) / 1000 <= *best || *best == Fixed::MAX {
        *best = distance;
        *stalled = 0;
    } else {
        *stalled += 1;
    }
    if reached {
        unit.replans = 0;
    }
    if reached || *stalled >= STALL_TICKS {
        // A unit that makes no progress (crowded waypoint, separation
        // deadlock) gives up on the waypoint; on the last one it stays
        // where it is.
        if last {
            stop(unit);
            if !reached {
                return Moved::Stalled(waypoint);
            }
        } else {
            *next += 1;
            *best = Fixed::MAX;
            *stalled = 0;
        }
    }
    Moved::Ok
}

/// `direction` turned by 45 degrees to the left and to the right.
fn angled(direction: Vec2) -> [Vec2; 2] {
    let c = Fixed::from_num(7071) / Fixed::from_num(10_000);
    let (x, y) = (direction.x, direction.y);
    [
        Vec2::new((x - y) * c, (x + y) * c),
        Vec2::new((x + y) * c, (y - x) * c),
    ]
}

/// Paths a unit may plan again from its position before reaching the next
/// waypoint; then it gives up and stops.
const MAX_REPLANS: u32 = 3;

/// A route along `waypoints` from the first one on.
const fn following(waypoints: Vec<Vec2>) -> Route {
    Route::Following {
        waypoints,
        next: 0,
        best: Fixed::MAX,
        stalled: 0,
    }
}

/// Intermediate waypoints count as reached within this distance (meters).
const WAYPOINT_RADIUS_M: i32 = 1;

/// Ticks without getting closer to the current waypoint after which a unit
/// gives up on it (2 s).
const STALL_TICKS: u32 = 40;

/// Getting closer to the waypoint by less than this (mm) is no progress
/// (prompt 173). The slowest units (infantry crawling through mud, about
/// 0.2 m/s) cover it in a second.
const MIN_PROGRESS_MM: i32 = 200;

fn stop(unit: &mut Unit) {
    unit.target = None;
    unit.route = Route::None;
    unit.replans = 0;
}

/// Meters per tick of `unit` driving in `direction` (unit length) from its
/// current position, or `None` if the grade is too steep.
///
/// Terrain picks road or cross-country speed and scales it; the result never
/// exceeds the road speed. The grade is the height difference over the next
/// meter.
fn ground_speed(
    unit: &Unit,
    direction: Vec2,
    map: &Map,
    rules: &TerrainRules,
    ground_pct: &[u32],
) -> Option<Fixed> {
    let id = usize::from(map.terrain_at(unit.position));
    let terrain = &rules.types[id];
    let mut speed = terrain.speed(unit.speed_offroad, unit.speed_road);
    // Soft ground (prompt 040).
    if let Some(&pct) = ground_pct.get(id)
        && pct != 100
    {
        speed = speed * Fixed::from_num(pct) / 100;
    }
    if unit.speed_pct != 100 {
        speed = speed * Fixed::from_num(unit.speed_pct) / 100;
    }
    let grade = map.height_at(unit.position + direction) - unit.height;
    Some(speed * rules.slope.factor(grade)?)
}

/// Everything [`new_unit`] needs to create a unit.
struct NewUnit<'a> {
    id: UnitId,
    owner: PlayerId,
    type_index: u32,
    unit_type: &'a UnitType,
    position: Vec2,
    facing: Vec2,
    fire_mode: FireMode,
    hp: u32,
    soldier_hp: u32,
    infantry: &'a InfantryRt,
    map: &'a Map,
    tick_hz: u32,
}

/// A unit standing still at its position (at the start of a match, or
/// produced during it, prompt 023).
fn new_unit(n: &NewUnit<'_>) -> Unit {
    let ticks_per_second = Fixed::from_num(n.tick_hz);
    let squad = Sim::new_squad(
        n.infantry,
        n.type_index as usize,
        n.position,
        n.facing,
        n.soldier_hp,
        n.map,
    );
    let spread = n.infantry.types[n.type_index as usize]
        .as_ref()
        .map_or(n.unit_type.footprint_m, |t| t.spread);
    Unit {
        id: n.id,
        owner: n.owner,
        unit_type: n.type_index,
        position: n.position,
        height: n.map.height_at(n.position),
        target: None,
        route: Route::None,
        replans: 0,
        speed_offroad: n.unit_type.speed_m_per_s / ticks_per_second,
        speed_road: n.unit_type.road_speed_m_per_s / ticks_per_second,
        footprint: n.unit_type.footprint_m,
        facing: n.facing,
        hp: n.hp,
        combat: CombatState::new(
            n.facing,
            n.unit_type.combat.weapons.len() + n.unit_type.combat.machine_guns.len(),
            n.fire_mode,
        ),
        squad,
        speed_pct: 100,
        spread,
        elevation_mm: 0,
        vision_dirty: false,
        orders: orders::OrderQueue::default(),
        link: None,
        stock: logistics::Stock::default(),
        drill: drill::Drill::default(),
        veterancy: experience::Veterancy::default(),
        crush_slow: 0,
        alert: alert::Alert::default(),
        progress: progress::Progress::default(),
        under_fire: discipline::UnderFire::default(),
        crew: crew::Crew::new(n.unit_type.combat.gun_crew),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapLayout;

    const P0: PlayerId = PlayerId(0);
    const P1: PlayerId = PlayerId(1);

    fn tank() -> UnitType {
        UnitType {
            name: "tank".into(),
            speed_m_per_s: Fixed::from_num(4),
            road_speed_m_per_s: Fixed::from_num(10),
            footprint_m: Fixed::from_num(6),
            mass_kg: 0,
            combat: crate::combat::CombatType::default(),
            transport: crate::transport::TransportType::default(),
        }
    }

    /// 400 m x 400 m (201 x 201 samples of 2 m), centered on the origin.
    /// `sample(i, j)` returns height in mm and terrain id of each sample.
    fn map_with(sample: impl Fn(u32, u32) -> (i32, u8)) -> Arc<Map> {
        let n = 201;
        let (heights, terrain) = (0..n)
            .flat_map(|j| (0..n).map(move |i| (i, j)))
            .map(|(i, j)| sample(i, j))
            .unzip();
        let layout = MapLayout {
            name: "test".into(),
            samples_x: n,
            samples_y: n,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
    }

    fn flat_map() -> Arc<Map> {
        map_with(|_, _| (0, 0))
    }

    fn new_sim(config: &SimConfig) -> Result<Sim, ConfigError> {
        Sim::new(config, flat_map(), 1)
    }

    /// Only unit 0 of [`config`], on `map`: nothing in its way.
    fn solo_on(map: Arc<Map>) -> Sim {
        let mut cfg = config();
        cfg.spawns.truncate(1);
        cfg.map = map.reference();
        Sim::new(&cfg, map, 1).unwrap()
    }

    /// Same units as [`config`], on `map`.
    fn sim_on(map: Arc<Map>) -> Sim {
        let mut cfg = config();
        cfg.map = map.reference();
        Sim::new(&cfg, map, 1).unwrap()
    }

    /// Units 0..=2 belong to player 0, unit 3 to player 1.
    fn config() -> SimConfig {
        let spawn = |owner, x, y| Spawn {
            owner,
            unit_type: "tank".into(),
            position: Vec2::from_meters(x, y),
            fire_mode: crate::combat::FireMode::default(),
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
            map: flat_map().reference(),
            terrain: TerrainRules::builtin().unwrap(),
            combat: crate::combat::CombatRules::builtin().unwrap(),
            vision: crate::vision::VisionRules::builtin().unwrap(),
            infantry: crate::infantry::InfantryRules::builtin().unwrap(),
            teams: crate::teams::Teams::default(),
            weather: crate::weather::WeatherSetup::default(),
            mission: None,
            variant: None,
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
        let mut sim = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[0], 3, 4)]);
        // 5 m at 4 m/s and 20 Hz takes 25 ticks.
        run(&mut sim, 30);
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.position, Vec2::from_meters(3, 4));
        assert_eq!(unit.target, None);
    }

    #[test]
    fn unit_moves_by_speed_per_tick() {
        let mut sim = solo_on(flat_map());
        sim.step(&[move_to(P0, &[0], 10, 0)]);
        let x = sim.snapshot().units[0].position.x;
        let expected = Fixed::from_num(4) / Fixed::from_num(20);
        assert!((x - expected).abs() <= Fixed::DELTA * 4, "x = {x}");
    }

    #[test]
    fn unknown_unit_is_ignored() {
        let mut sim = new_sim(&config()).unwrap();
        let before = sim.state_hash();
        let mut reference = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[99], 1, 1)]);
        reference.step(&[]);
        assert_ne!(before, sim.state_hash());
        assert_eq!(sim.state_hash(), reference.state_hash());
    }

    #[test]
    fn commands_for_foreign_units_are_ignored() {
        let mut sim = new_sim(&config()).unwrap();
        let mut reference = new_sim(&config()).unwrap();
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
        let mut sim = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[0, 1, 2], 100, 100)]);
        let targets: Vec<Vec2> = sim.snapshot().units[..3]
            .iter()
            .map(|u| u.target.unwrap())
            .collect();
        // Three units: 2 columns, 2 rows, spacing 6 m * 1.5 = 9 m. Since
        // prompt 174 the slots go by the way to the target (north-east):
        // the left unit takes the far slot, the others the near row, so no
        // way crosses another (before: row by row, the right unit crossed).
        assert_eq!(
            targets,
            vec![
                Vec2::new(Fixed::from_num(100), Fixed::from_num(104.5)),
                Vec2::new(Fixed::from_num(95.5), Fixed::from_num(95.5)),
                Vec2::new(Fixed::from_num(104.5), Fixed::from_num(95.5)),
            ]
        );
    }

    /// Eight tanks in two rows drive straight ahead as a group. Followers
    /// must not zig-zag: the sideways part of their step may change sign
    /// only a few times (formation shuffling at the start), not every few
    /// ticks.
    #[test]
    fn group_drives_straight_without_wobbling() {
        let mut cfg = config();
        cfg.spawns = (0..8)
            .map(|i| Spawn {
                owner: P0,
                unit_type: "tank".into(),
                position: Vec2::from_meters(-30 + 20 * (i % 4), 40 + 20 * (i / 4)),
                fire_mode: crate::combat::FireMode::default(),
            })
            .collect();
        let mut sim = new_sim(&cfg).unwrap();
        let ids: Vec<u32> = (0..8).collect();
        sim.step(&[move_to(P0, &ids, 0, -150)]);
        let mut last: Vec<Vec2> = sim.snapshot().units.iter().map(|u| u.position).collect();
        let mut last_dx = [Fixed::ZERO; 8];
        let mut flips = [0u32; 8];
        let noise = Fixed::from_num(1) / Fixed::from_num(100);
        for _ in 0..800 {
            sim.step(&[]);
            let now: Vec<Vec2> = sim.snapshot().units.iter().map(|u| u.position).collect();
            for i in 0..8 {
                let dx = now[i].x - last[i].x;
                if dx.abs() > noise {
                    if last_dx[i] != Fixed::ZERO && (dx > Fixed::ZERO) != (last_dx[i] > Fixed::ZERO)
                    {
                        flips[i] += 1;
                    }
                    last_dx[i] = dx;
                }
            }
            last = now;
        }
        println!("sideways direction changes per unit: {flips:?}");
        assert!(flips.iter().all(|&f| f <= 4), "units wobble: {flips:?}");
    }

    #[test]
    fn formation_is_independent_of_command_order() {
        let mut a = new_sim(&config()).unwrap();
        let mut b = new_sim(&config()).unwrap();
        a.step(&[move_to(P0, &[0, 1, 2], -40, 70)]);
        b.step(&[move_to(P0, &[2, 0, 1, 2, 0], -40, 70)]);
        assert_eq!(a.state_hash(), b.state_hash());
    }

    #[test]
    fn stop_clears_the_target() {
        let mut sim = new_sim(&config()).unwrap();
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
        let mut sim = new_sim(&config()).unwrap();
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
            Sim::new(&cfg, flat_map(), 0).unwrap_err(),
            ConfigError::UnknownUnitType("nope".into())
        );
    }

    #[test]
    fn rejects_a_different_map() {
        let other = map_with(|i, _| (i32::try_from(i).unwrap(), 0));
        assert!(matches!(
            Sim::new(&config(), other, 0),
            Err(ConfigError::MapMismatch { .. })
        ));
    }

    #[test]
    fn rejects_spawns_outside_the_map() {
        let mut cfg = config();
        cfg.spawns[1].position = Vec2::from_meters(201, 0);
        assert_eq!(
            Sim::new(&cfg, flat_map(), 0).unwrap_err(),
            ConfigError::SpawnOutsideMap(1)
        );
    }

    /// Grade 10 % uphill towards +x: 200 mm per 2 m sample.
    fn ramp() -> Arc<Map> {
        map_with(|i, _| (i32::try_from(i).unwrap() * 200, 0))
    }

    #[test]
    fn unit_follows_the_ground_height() {
        let map = ramp();
        let mut sim = sim_on(map.clone());
        let start = sim.snapshot().units[0];
        // Origin is sample 100: 20 m.
        assert_eq!(start.height, Fixed::from_num(20));
        sim.step(&[move_to(P0, &[0], 50, 0)]);
        run(&mut sim, 40);
        let unit = sim.snapshot().units[0];
        // 41 ticks at 80 % of 0.2 m: about 6.56 m.
        assert!(
            unit.position.x > Fixed::from_num(6),
            "x = {}",
            unit.position.x
        );
        assert_eq!(unit.height, map.height_at(unit.position));
        assert!(unit.height > start.height + Fixed::from_num(6) / 10);
    }

    #[test]
    fn uphill_is_slower_than_flat_and_downhill() {
        let step_x = |map: Arc<Map>, x: i32| {
            let mut sim = solo_on(map);
            sim.step(&[move_to(P0, &[0], x, 0)]);
            sim.snapshot().units[0].position.x.abs()
        };
        let per_tick = Fixed::from_num(4) / Fixed::from_num(20);
        // 10 % uphill: 1 - 0.1 * 2 = 80 % of 0.2 m (up to rounding of the
        // 1 m lookahead height).
        let uphill = step_x(ramp(), 50);
        let expected = per_tick * Fixed::from_num(8) / Fixed::from_num(10);
        assert!(
            (uphill - expected).abs() <= Fixed::DELTA * 16,
            "uphill {uphill}"
        );
        assert_eq!(step_x(ramp(), -50), per_tick);
        assert_eq!(step_x(flat_map(), 50), per_tick);
    }

    #[test]
    fn too_steep_grades_stop_the_unit() {
        // 70 % grade: 1400 mm per sample.
        let steep = map_with(|i, _| (i32::try_from(i).unwrap() * 1400, 0));
        let mut sim = sim_on(steep);
        let start = sim.snapshot().units[0].position;
        sim.step(&[move_to(P0, &[0], 50, 0)]);
        let unit = sim.snapshot().units[0];
        assert_eq!((unit.position, unit.target), (start, None));
    }

    #[test]
    fn road_uses_the_road_speed() {
        let road = map_with(|_, _| (0, 2));
        let mut sim = solo_on(road);
        sim.step(&[move_to(P0, &[0], 50, 0)]);
        // 10 m/s at 20 Hz.
        assert_eq!(sim.snapshot().units[0].position.x, Fixed::ONE / 2);
    }

    #[test]
    fn targets_in_deep_water_end_at_the_nearest_reachable_spot() {
        // Deep water from x = 20 m (sample 110, owned by the cell 19..23 m)
        // on. The tank's margin of one sample also blocks the cell 15..19 m,
        // so the nearest reachable cell center is x = 13 m (y = 1 m: cells
        // start half a sample before the map edge).
        let lake = map_with(|i, _| (0, if i >= 110 { 7 } else { 0 }));
        let mut sim = solo_on(lake.clone());
        sim.step(&[move_to(P0, &[0], 60, 0)]);
        assert_eq!(
            sim.snapshot().units[0].target,
            Some(Vec2::from_meters(13, 1))
        );
        run(&mut sim, 200);
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.target, None);
        assert_eq!(unit.position, Vec2::from_meters(13, 1));
        assert_ne!(lake.terrain_at(unit.position), 7);
    }

    #[test]
    fn targets_beyond_the_map_edge_end_inside_with_clearance() {
        let mut sim = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[0], 0, -500)]);
        run(&mut sim, 20 * 60);
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.target, None);
        // Row 0 (-201..-197 m) touches the edge, which tanks keep a margin
        // from; center of row 1.
        assert_eq!(unit.position, Vec2::from_meters(1, -195));
    }

    /// A wall of deep water across the map at x = 40..44 m with a gap at
    /// y = 60..80 m.
    fn wall_with_gap() -> Arc<Map> {
        map_with(|i, j| {
            let wall = (120..=122).contains(&i) && !(130..=140).contains(&j);
            (0, if wall { 7 } else { 0 })
        })
    }

    #[test]
    fn units_drive_around_obstacles() {
        let map = wall_with_gap();
        let mut sim = sim_on(map.clone());
        sim.step(&[move_to(P0, &[0, 1, 2], 80, 0)]);
        let mut max_x = Fixed::ZERO;
        for _ in 0..20 * 120 {
            sim.step(&[]);
            for u in &sim.snapshot().units[..3] {
                assert_ne!(map.terrain_at(u.position), 7, "unit in the water");
                max_x = max_x.max(u.position.x);
            }
        }
        let units = sim.snapshot().units;
        for u in &units[..3] {
            assert_eq!(u.target, None);
            assert!(u.position.x > Fixed::from_num(70), "{:?}", u.position);
        }
        // All three went through the gap.
        assert!(max_x > Fixed::from_num(70));
    }

    #[test]
    fn group_shares_one_path_request() {
        let mut sim = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[0, 1, 2], 80, 60)]);
        assert_eq!(sim.path_stats().requests, 1);
        assert_eq!(sim.path_stats().found, 1);
        for id in 0..3 {
            assert!(matches!(
                sim.route_of(UnitId(id)),
                Some(RouteView::Following(_))
            ));
        }
        // Through a narrow gap a member may need a path of its own, but
        // the group still arrives.
        let mut sim = sim_on(wall_with_gap());
        sim.step(&[move_to(P0, &[0, 1, 2], 80, 0)]);
        run(&mut sim, 20 * 120);
        assert!(sim.path_stats().requests <= 4);
        assert!(sim.snapshot().units[..3].iter().all(|u| u.target.is_none()));
    }

    /// Unit 0 of a solo sim on `map`, following `waypoints`, moved by
    /// [`move_unit`] with `push` for `ticks` ticks; `clear` as in
    /// [`Sim::move_units`]. Returns the unit and the results.
    fn drive(
        map: &Arc<Map>,
        from: Vec2,
        waypoints: Vec<Vec2>,
        push: impl Fn(&Unit) -> Vec2,
        clear: impl Fn(Vec2) -> bool,
        ticks: u32,
    ) -> (Unit, Vec<Moved>) {
        let sim = solo_on(map.clone());
        let mut unit = sim.units[0].clone();
        unit.position = from;
        unit.height = map.height_at(from);
        unit.target = waypoints.last().copied();
        unit.route = following(waypoints);
        let ground = (&sim.terrain, sim.paths.grid.ground_pct(0));
        let moves = (0..ticks)
            .map(|_| {
                let p = push(&unit);
                move_unit(&mut unit, (p, None), map, ground, &clear)
            })
            .collect();
        (unit, moves)
    }

    /// Prompt 173: a unit pushed back almost as far as it drives (a crowd
    /// at the target) creeps by millimeters; that is no progress, so it
    /// gives up the waypoint after the stall time instead of creeping for
    /// minutes.
    #[test]
    fn creeping_is_no_progress() {
        let map = flat_map();
        let waypoints = vec![Vec2::from_meters(30, 0), Vec2::from_meters(60, 0)];
        // Pushed straight back by 99 % of the heading.
        let back =
            |u: &Unit| heading(u).map_or(Vec2::ZERO, |d| d.scale(-Fixed::from_num(99) / 100));
        let (unit, _) = drive(&map, Vec2::ZERO, waypoints, back, |_| true, STALL_TICKS + 2);
        let Route::Following { next, .. } = unit.route else {
            panic!("route ended: {:?}", unit.route);
        };
        assert_eq!(
            next, 1,
            "the creeping unit still heads for the first waypoint"
        );
        assert!(unit.position.x < Fixed::ONE, "{:?}", unit.position);
    }

    /// Prompt 173: blocked by water almost head-on, a slide along the bank
    /// of a few millimeters per tick is no way forward: the unit reports
    /// the way as blocked (and plans again) instead of creeping.
    #[test]
    fn tiny_slides_do_not_count() {
        // Deep water from x = 40 m on.
        let map = map_with(|i, _| (0, if i >= 120 { 7 } else { 0 }));
        let from = Vec2::new(Fixed::from_num(38), Fixed::ZERO);
        let waypoints = vec![Vec2::new(Fixed::from_num(120), Fixed::from_num(2))];
        let (unit, moves) = drive(&map, from, waypoints, |_| Vec2::ZERO, |_| true, 40);
        assert!(
            moves.contains(&Moved::Blocked),
            "never blocked: {:?}",
            unit.position
        );
        assert!(
            (unit.position.y - from.y).abs() < Fixed::ONE,
            "crept along the bank to {:?}",
            unit.position
        );
    }

    /// Prompt 173: the direct step along the leg must stay on cells the
    /// unit's class may use, like pushed and sliding steps: a unit pushed
    /// off its leg beside a deck must not drive onto the steep bank from
    /// where no search starts.
    #[test]
    fn direct_steps_stay_on_clear_cells() {
        let map = flat_map();
        let waypoints = vec![Vec2::from_meters(60, 20)];
        // Cells with y >= 2 m are closed to this unit.
        let clear = |p: Vec2| p.y < Fixed::from_num(2);
        let (unit, _) = drive(&map, Vec2::ZERO, waypoints, |_| Vec2::ZERO, clear, 200);
        assert!(unit.position.y < Fixed::from_num(2), "{:?}", unit.position);
    }

    /// Prompt 173: two units of a column heading for the same waypoint
    /// push each other only along their line (the rear one waits), not
    /// sideways onto a bank; units with different waypoints still push
    /// each other sideways.
    #[test]
    fn a_column_queues_at_a_shared_waypoint() {
        let mut sim = sim_on(flat_map());
        let ahead = Vec2::new(Fixed::from_num(4), Fixed::ONE);
        sim.units[0].position = Vec2::ZERO;
        sim.units[1].position = ahead;
        let gap = Vec2::from_meters(50, 0);
        for unit in &mut sim.units[..2] {
            unit.route = following(vec![gap]);
        }
        let east = Vec2::new(Fixed::ONE, Fixed::ZERO);
        let directions = vec![Some(east), Some(east), None, None];
        let shared = vec![Some(gap), Some(gap), None, None];
        let near = [0, 1];
        let push = separation(
            &sim.units,
            (&directions, &shared, false),
            0,
            east,
            &near,
            |_| false,
        );
        assert_eq!(push.y, Fixed::ZERO, "pushed sideways: {push:?}");
        assert!(
            push.x < Fixed::ZERO,
            "the rear unit is not held back: {push:?}"
        );
        let own = vec![Some(gap), Some(Vec2::from_meters(50, 10)), None, None];
        let push = separation(
            &sim.units,
            (&directions, &own, false),
            0,
            east,
            &near,
            |_| false,
        );
        assert!(
            push.y < Fixed::ZERO,
            "no sideways push without a shared waypoint: {push:?}"
        );
    }

    /// Prompt 173: a unit that follows a route but gets nowhere for
    /// `STUCK_SECONDS` stops where it is and is reported; a halted unit
    /// (stance change, bound) is not, and a new target starts afresh.
    #[test]
    fn units_without_progress_stop_and_are_reported() {
        let mut sim = solo_on(flat_map());
        let limit = progress::STUCK_SECONDS * sim.tick_hz;
        let hold = |sim: &mut Sim, target: Vec2, ticks: u32, speed_pct: u32| {
            for _ in 0..ticks {
                sim.move_events.clear();
                let unit = &mut sim.units[0];
                if unit.target != Some(target) {
                    unit.target = Some(target);
                    unit.route = following(vec![target]);
                }
                unit.speed_pct = speed_pct;
                sim.check_progress();
                if !sim.move_events.is_empty() {
                    return true;
                }
            }
            false
        };
        let far = Vec2::from_meters(100, 0);
        // Halted on purpose: never reported.
        assert!(!hold(&mut sim, far, limit * 2, 0));
        // A new target just before the limit starts the clock again.
        assert!(!hold(&mut sim, far, limit - 1, 100));
        assert!(!hold(&mut sim, Vec2::from_meters(0, 100), limit - 1, 100));
        assert!(hold(&mut sim, Vec2::from_meters(0, 100), 2, 100));
        let unit = &sim.units[0];
        assert_eq!(unit.route, Route::None);
        assert_eq!(unit.target, None);
        assert_eq!(
            sim.move_events,
            vec![MoveEvent::Stuck {
                unit: unit.id,
                at: unit.position
            }]
        );
    }

    /// Prompt 173: a unit that stops short of its target under the old
    /// rules (stalled on its last waypoint) is reported, one that stops
    /// close to it is not.
    #[test]
    fn units_stopping_short_are_reported() {
        let mut sim = solo_on(flat_map());
        sim.report_short(0, Some(Vec2::from_meters(5, 0)));
        assert!(sim.move_events.is_empty());
        sim.report_short(0, Some(Vec2::from_meters(40, 0)));
        assert_eq!(sim.move_events.len(), 1);
    }

    #[test]
    fn units_wait_for_their_path_and_stop_cancels_it() {
        let mut sim = sim_on(wall_with_gap());
        // Many single-unit orders in one tick exceed the node budget, so
        // later ones wait.
        let commands: Vec<PlayerCommand> = (0..3)
            .flat_map(|_| [move_to(P0, &[0], 90, 90), move_to(P0, &[1], 90, -90)])
            .collect();
        let start = sim.snapshot().units;
        sim.step(&commands);
        assert!(sim.path_stats().dropped > 0, "{:?}", sim.path_stats());
        sim.step(&[PlayerCommand::new(
            P0,
            Command::Stop {
                units: vec![UnitId(0), UnitId(1)],
            },
        )]);
        assert_eq!(sim.path_queue_len(), 0);
        run(&mut sim, 20);
        let units = sim.snapshot().units;
        assert!(units[0].target.is_none() && units[1].target.is_none());
        assert!((units[0].position - start[0].position).length() < Fixed::from_num(2));
    }

    #[test]
    fn separation_keeps_crossing_units_apart() {
        // Two tanks swap places head-on.
        let mut cfg = config();
        cfg.spawns.truncate(2);
        cfg.spawns[1].owner = P0;
        cfg.spawns[1].position = Vec2::from_meters(40, 0);
        let mut sim = new_sim(&cfg).unwrap();
        sim.step(&[move_to(P0, &[0], 40, 0), move_to(P0, &[1], 0, 0)]);
        let mut closest = Fixed::MAX;
        for _ in 0..20 * 30 {
            sim.step(&[]);
            let u = sim.snapshot().units;
            closest = closest.min((u[0].position - u[1].position).length());
        }
        let u = sim.snapshot().units;
        assert_eq!(u[0].position, Vec2::from_meters(40, 0));
        assert_eq!(u[1].position, Vec2::from_meters(0, 0));
        assert!(closest > Fixed::from_num(3), "closest {closest}");
    }

    #[test]
    fn obstacles_trigger_replanning() {
        let mut sim = new_sim(&config()).unwrap();
        sim.step(&[move_to(P0, &[0], 100, 0)]);
        // A building appears across the route.
        sim.set_path_obstacle(Vec2::from_meters(30, -20), Vec2::from_meters(40, 20), true);
        assert_eq!(sim.route_of(UnitId(0)), Some(RouteView::Waiting));
        run(&mut sim, 20 * 60);
        let unit = sim.snapshot().units[0];
        assert_eq!(unit.position, Vec2::from_meters(100, 0));
        assert_eq!(sim.path_grid().version(), 1);
    }
}

#[cfg(test)]
mod object_tests {
    use super::*;
    use crate::map::MapLayout;
    use crate::objects::CSV_HEADER;
    use crate::path::SizeClass;
    use std::fmt::Write as _;

    const P0: PlayerId = PlayerId(0);

    fn unit_type(name: &str, footprint_m: i32) -> UnitType {
        UnitType {
            name: name.into(),
            speed_m_per_s: Fixed::from_num(4),
            road_speed_m_per_s: Fixed::from_num(10),
            footprint_m: Fixed::from_num(footprint_m),
            mass_kg: 0,
            combat: crate::combat::CombatType::default(),
            transport: crate::transport::TransportType::default(),
        }
    }

    /// Flat 400 m map with a dry-stone wall along x = 40 m (gap at
    /// y = 60..80 m), a dense row of oaks along x = -40 m (gap at y = -10..10 m)
    /// and a farmhouse at (0, -100).
    fn walled_map() -> Arc<Map> {
        let n = 201;
        let layout = MapLayout {
            name: "walls".into(),
            samples_x: n,
            samples_y: n,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        let count = (n * n) as usize;
        let mut map = Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap();
        let mut csv = format!("{CSV_HEADER}\n");
        // Wall segments are 3 m long, heading 0 = along y.
        for k in 0..133 {
            let y = -198_500 + k * 3000;
            if !(58_000..=82_000).contains(&y) {
                let _ = writeln!(csv, "env_wall_stone,Segment,40000,{y},0");
            }
        }
        // Trees 3 m apart: no 4 m cell between two trunks stays free.
        for k in 0..133 {
            let y = -198_500 + k * 3000;
            if !(-12_000..=12_000).contains(&y) {
                let _ = writeln!(csv, "env_tree_deciduous,Oak,-40000,{y},{}", k * 37 % 360);
            }
        }
        let _ = writeln!(csv, "env_house_farm,Intact,0,-100000,90");
        map.add_objects(&csv).unwrap();
        Arc::new(map)
    }

    /// Units 0..=2: tanks at x = 0..20, unit 3: infantry at (0, 20).
    fn sim() -> (Sim, Arc<Map>) {
        let map = walled_map();
        let spawn = |unit_type: &str, x, y| Spawn {
            owner: P0,
            unit_type: unit_type.into(),
            position: Vec2::from_meters(x, y),
            fire_mode: crate::combat::FireMode::default(),
        };
        let config = SimConfig {
            tick_hz: 20,
            unit_types: vec![unit_type("tank", 7), unit_type("infantry", 1)],
            spawns: vec![
                spawn("tank", 0, 0),
                spawn("tank", 10, 0),
                spawn("tank", 20, 0),
                spawn("infantry", 0, 20),
            ],
            map: map.reference(),
            terrain: TerrainRules::builtin().unwrap(),
            combat: crate::combat::CombatRules::builtin().unwrap(),
            vision: crate::vision::VisionRules::builtin().unwrap(),
            infantry: crate::infantry::InfantryRules::builtin().unwrap(),
            teams: crate::teams::Teams::default(),
            weather: crate::weather::WeatherSetup::default(),
            mission: None,
            variant: None,
        };
        (Sim::new(&config, map.clone(), 1).unwrap(), map)
    }

    fn order(units: &[u32], x: i32, y: i32) -> PlayerCommand {
        PlayerCommand::new(
            P0,
            Command::MoveTo {
                units: units.iter().copied().map(UnitId).collect(),
                target: Vec2::from_meters(x, y),
            },
        )
    }

    #[test]
    fn objects_block_cells_by_size_class() {
        let (sim, _) = sim();
        let grid = sim.path_grid();
        let at = |x, y| grid.cell_of(Vec2::from_meters(x, y));
        let (wx, wy) = at(40, 0);
        assert!(grid.object_blocked(SizeClass::Large, wx, wy));
        assert!(!grid.object_blocked(SizeClass::Small, wx, wy));
        assert!(!grid.passable(SizeClass::Large, wx, wy));
        assert!(grid.passable(SizeClass::Small, wx, wy));
        let (gx, gy) = at(40, 70);
        assert!(grid.passable(SizeClass::Large, gx, gy));
        // The house (14 m along x, 9 m along y) blocks both classes.
        for (x, y) in [(0, -100), (6, -103), (-6, -97)] {
            let (hx, hy) = at(x, y);
            assert!(!grid.passable(SizeClass::Small, hx, hy), "({x}, {y})");
            assert!(!grid.passable(SizeClass::Large, hx, hy), "({x}, {y})");
        }
        let (ox, oy) = at(0, -110);
        assert!(grid.passable(SizeClass::Small, ox, oy));
    }

    #[test]
    fn tanks_use_the_gaps_and_infantry_climbs_the_wall() {
        let (mut sim, map) = sim();
        sim.step(&[order(&[0, 1, 2], 80, 0), order(&[3], 80, 20)]);
        let grid = sim.path_grid().clone();
        let mut tank_max_y = Fixed::MIN;
        let mut infantry_max_dy = Fixed::ZERO;
        for _ in 0..20 * 120 {
            sim.step(&[]);
            let units = sim.snapshot().units;
            for u in &units[..3] {
                let (x, y) = grid.cell_of(u.position);
                assert!(
                    !grid.object_blocked(SizeClass::Large, x, y),
                    "tank in a blocked cell at {:?}",
                    u.position
                );
                tank_max_y = tank_max_y.max(u.position.y);
            }
            infantry_max_dy =
                infantry_max_dy.max((units[3].position.y - Fixed::from_num(20)).abs());
        }
        let units = sim.snapshot().units;
        assert!(units.iter().all(|u| u.target.is_none()), "{units:?}");
        assert!(units.iter().all(|u| u.position.x > Fixed::from_num(70)));
        // The tanks went through the gap at y = 60..80 ...
        assert!(tank_max_y > Fixed::from_num(55), "{tank_max_y}");
        // ... the infantry straight across the wall.
        assert!(infantry_max_dy < Fixed::from_num(6), "{infantry_max_dy}");
        assert!(!map.objects().is_empty());
    }

    #[test]
    fn tanks_pass_the_tree_row_only_at_its_gap() {
        let (mut sim, _) = sim();
        sim.step(&[order(&[0], -80, 60)]);
        let mut crossed_at = None;
        let mut last = sim.snapshot().units[0].position;
        for _ in 0..20 * 120 {
            sim.step(&[]);
            let p = sim.snapshot().units[0].position;
            if last.x > Fixed::from_num(-40) && p.x <= Fixed::from_num(-40) {
                crossed_at = Some(p.y);
            }
            last = p;
        }
        assert!(sim.snapshot().units[0].target.is_none());
        let y = crossed_at.expect("tank crossed the tree row");
        assert!(y.abs() < Fixed::from_num(12), "crossed at y = {y}");
    }

    #[test]
    fn objects_change_the_map_hash() {
        let plain = || {
            let n = 201;
            let layout = MapLayout {
                name: "walls".into(),
                samples_x: n,
                samples_y: n,
                cell_size_mm: 2000,
                starts: Vec::new(),
                markers: Vec::new(),
            };
            let count = (n * n) as usize;
            Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap()
        };
        let with = |heading: u32| {
            let mut map = plain();
            let csv = format!("{CSV_HEADER}\nenv_house_farm,Intact,0,-100000,{heading}\n");
            map.add_objects(&csv).unwrap();
            map.hash()
        };
        assert_ne!(plain().hash(), walled_map().hash());
        assert_ne!(with(90), with(91));
        assert_eq!(with(90), with(90));
        // An empty list keeps the hash of the rasters.
        let mut empty = plain();
        empty.add_objects(&format!("{CSV_HEADER}\n")).unwrap();
        assert_eq!(empty.hash(), plain().hash());
    }
}
