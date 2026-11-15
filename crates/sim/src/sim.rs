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

mod combat;
mod destruction;
mod infantry;
mod spatial;
mod vision;

pub use vision::MAX_PLAYERS;

use combat::{Attack, CombatState, Projectile, TypeCombat};
use infantry::{InfantryRt, Squad};

pub use infantry::BuildingCondition;

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
            Self::Destruction(e) => write!(f, "{e}"),
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
}

impl Unit {
    const fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// How a unit gets to its target.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Stance of an infantry squad; `None` for vehicles (prompt 022).
    pub stance: Option<crate::infantry::Stance>,
    /// The building (object index) a squad is in.
    pub garrison: Option<u32>,
    /// The squad is changing its stance.
    pub changing_stance: bool,
    /// Width for formations: the footprint, or a squad's formation width.
    pub spread: Fixed,
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
    /// Destroyed map objects: path grid and vision cells.
    Destruction,
    /// Viewsheds and player vision.
    Vision,
    /// Victory conditions.
    Standings,
}

impl Phase {
    pub const ALL: [Self; 9] = [
        Self::Commands,
        Self::Paths,
        Self::Combat,
        Self::Movement,
        Self::Squads,
        Self::Projectiles,
        Self::Destruction,
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
            Self::Vision => "vision",
            Self::Standings => "standings",
        }
    }
}

/// The deterministic simulation.
#[derive(Clone, Debug)]
pub struct Sim {
    tick: u64,
    rng: Rng,
    /// Sorted by id; ids are assigned in increasing order.
    units: Vec<Unit>,
    map: Arc<Map>,
    terrain: TerrainRules,
    paths: Pathfinder,
    tick_hz: u32,
    combat_rules: CombatRules,
    /// Per unit type.
    types: Vec<TypeCombat>,
    projectiles: Vec<Projectile>,
    next_projectile: u32,
    /// Events of the last tick (not state: derived, not hashed).
    events: Vec<CombatEvent>,
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
    unit_grid: spatial::UnitGrid,
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
        let ticks_per_second = Fixed::from_num(config.tick_hz);
        let infantry = InfantryRt::new(&config.infantry, &config.unit_types, &map, config.tick_hz);

        let mut units = Vec::with_capacity(config.spawns.len());
        for (index, spawn) in (0u32..).zip(&config.spawns) {
            let (type_index, unit_type) = (0u32..)
                .zip(&config.unit_types)
                .find(|(_, t)| t.name == spawn.unit_type)
                .ok_or_else(|| ConfigError::UnknownUnitType(spawn.unit_type.clone()))?;
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
            let max_hp = config.combat.max_hp(&unit_type.combat);
            let weapons = unit_type.combat.weapons.len();
            let squad = Self::new_squad(
                &infantry,
                type_index as usize,
                spawn.position,
                facing,
                config.combat.damage.soldier_hp,
                &map,
            );
            let spread = infantry.types[type_index as usize]
                .as_ref()
                .map_or(unit_type.footprint_m, |t| t.spread);
            units.push(Unit {
                id: UnitId(index),
                owner: spawn.owner,
                unit_type: type_index,
                position: spawn.position,
                height: map.height_at(spawn.position),
                target: None,
                route: Route::None,
                replans: 0,
                speed_offroad: unit_type.speed_m_per_s / ticks_per_second,
                speed_road: unit_type.road_speed_m_per_s / ticks_per_second,
                footprint: unit_type.footprint_m,
                facing,
                hp: max_hp,
                combat: CombatState::new(facing, weapons, spawn.fire_mode),
                squad,
                speed_pct: 100,
                spread,
                elevation_mm: 0,
                vision_dirty: false,
            });
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
        let grid = PathGrid::new(&map, &config.terrain, &config.unit_types);
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
            objects: combat::ObjectIndex::new(&map),
            map,
            standings: Standings::new(&players),
            vision,
            infantry,
            destruction,
            unit_grid: spatial::UnitGrid::default(),
        };
        sim.apply_destruction();
        sim.update_vision(true);
        Ok(sim)
    }

    /// Number of ticks simulated so far.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// The map of this match.
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
        for command in commands {
            self.apply(command);
        }
        done(Phase::Commands);
        self.plan_paths();
        done(Phase::Paths);
        self.update_stances();
        self.rebuild_unit_grid();
        self.aim_and_fire();
        done(Phase::Combat);
        self.move_units();
        done(Phase::Movement);
        self.update_squads();
        done(Phase::Squads);
        self.crush_objects();
        self.rebuild_unit_grid();
        self.fly_projectiles();
        done(Phase::Projectiles);
        self.apply_destruction();
        done(Phase::Destruction);
        self.update_vision(false);
        done(Phase::Vision);
        self.tick += 1;
        let units = &self.units;
        self.standings
            .update(self.tick, |player| alive_units(units, player));
        done(Phase::Standings);
    }

    /// Every player of the match (ascending) and whether they are defeated.
    pub fn standings(&self) -> &[PlayerStanding] {
        self.standings.players()
    }

    /// The result, once the match is decided. It never changes afterwards.
    pub const fn outcome(&self) -> Option<Outcome> {
        self.standings.outcome()
    }

    /// Number of units of `player` that are not destroyed (for victory
    /// conditions).
    pub fn alive_units(&self, player: PlayerId) -> usize {
        alive_units(&self.units, player)
    }

    /// Hit points of a unit (`None` if it does not exist).
    pub fn unit_hp(&self, id: UnitId) -> Option<u32> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        Some(self.units[index].hp)
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
        let units = &self.units;
        let wanted = |id: u32| units.iter().any(|u| u.route == Route::Waiting(id));
        let finished = self.paths.run(NODE_BUDGET_PER_TICK, wanted);
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
            let unit = &mut self.units[index];
            if unit.route != Route::Waiting(request.id) {
                continue;
            }
            let Some(path) = path else {
                unit.target = None;
                unit.route = Route::None;
                continue;
            };
            let grid = &self.paths.grid;
            let waypoints = if single {
                Some(path.waypoints[1..].to_vec())
            } else {
                let profile = unit.unit_type as usize;
                formation_route(grid, &self.map, profile, unit.position, path, offset)
            };
            match waypoints {
                Some(waypoints) => {
                    unit.target = waypoints.last().copied();
                    unit.route = following(waypoints);
                }
                None => {
                    let slot = unit.target.unwrap_or(path.goal + offset);
                    let own = self.paths.request(
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
        // Units arriving at their final waypoint ignore separation, so a
        // parked unit next to the target cannot keep them from arriving.
        let directions: Vec<Option<Vec2>> = self
            .units
            .iter()
            .map(|u| heading(u).filter(|_| !arriving(u)))
            .collect();
        // The unit grid holds the positions of this phase (built before
        // combat, which moves nobody).
        let mut near = Vec::new();
        let pushes: Vec<Vec2> = (0..self.units.len())
            .map(|i| {
                directions[i].map_or(Vec2::ZERO, |d| {
                    // Separation looks at most (own + other spread) far.
                    let me = &self.units[i];
                    let r = me.spread + self.unit_grid.max_spread;
                    let r = Vec2::new(r, r);
                    self.unit_grid
                        .query(me.position - r, me.position + r, &mut near);
                    separation(&self.units, &directions, i, d, &near)
                })
            })
            .collect();
        let grid = &self.paths.grid;
        let mut blocked = Vec::new();
        for (index, (unit, push)) in self.units.iter_mut().zip(pushes).enumerate() {
            let profile = unit.unit_type as usize;
            let keep_clear = |p: Vec2| grid.passable_for(profile, p);
            let from = unit.position;
            if move_unit(unit, push, &self.map, &self.terrain, keep_clear) == Moved::Blocked {
                blocked.push(index);
            }
            // The hull points where the unit drove.
            let step = unit.position - from;
            let length = step.length();
            if length > Fixed::ZERO {
                unit.facing = Vec2::new(step.x / length, step.y / length);
            }
        }
        // Pushed off the planned line (or the way is blocked): plan again
        // from here, a limited number of times.
        for index in blocked {
            let unit = &mut self.units[index];
            unit.replans += 1;
            match unit.target {
                Some(target) if unit.replans <= MAX_REPLANS => {
                    let id = self.paths.request(
                        unit.position,
                        target,
                        unit.unit_type as usize,
                        vec![(unit.id, Vec2::ZERO)],
                    );
                    unit.route = Route::Waiting(id);
                }
                _ => stop(unit),
            }
        }
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
                for &i in &group {
                    self.units[i].combat.attack = None;
                }
                let assigned: Vec<(usize, Vec2)> = group.into_iter().zip(slots).collect();
                self.request_group_path(&assigned, target);
            }
            Command::Stop { units } => {
                for index in self.owned_indices(command.player, units) {
                    stop(&mut self.units[index]);
                    self.units[index].combat.attack = None;
                    self.cancel_enter(index);
                }
            }
            Command::Attack { units, target } => {
                let Ok(t) = self.units.binary_search_by_key(target, |u| u.id) else {
                    return;
                };
                if self.units[t].owner == command.player || !self.units[t].alive() {
                    return;
                }
                for index in self.owned_indices(command.player, units) {
                    self.start_attack(index, Attack::Unit(*target));
                }
            }
            Command::AttackGround { units, target } => {
                if !within_world(*target) {
                    return;
                }
                for index in self.owned_indices(command.player, units) {
                    self.start_attack(index, Attack::Ground(*target));
                }
            }
            Command::SetFireMode { units, mode } => {
                for index in self.owned_indices(command.player, units) {
                    self.units[index].combat.fire_mode = *mode;
                    if *mode == FireMode::HoldFire {
                        self.units[index].combat.auto_target = None;
                    }
                }
            }
            Command::Surrender => self.standings.surrender(command.player, self.tick + 1),
            Command::SetStance { units, stance } => {
                for index in self.owned_indices(command.player, units) {
                    self.set_stance(index, *stance);
                }
            }
            Command::Garrison { units, building } => {
                let indices = self.owned_indices(command.player, units);
                self.order_garrison(&indices, *building);
            }
        }
    }

    /// Queues one path for a group (unit indices with their formation slots)
    /// to `target`. It starts at the unit nearest to the group's center and
    /// uses the profile of the most restrictive unit (size class, then the
    /// bridges it may not cross, then the largest), so it fits every member.
    fn request_group_path(&mut self, assigned: &[(usize, Vec2)], target: Vec2) {
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
        let id = self.paths.request(anchor, target, profile, members);
        for &(i, slot) in assigned {
            self.units[i].target = Some(slot);
            self.units[i].route = Route::Waiting(id);
            self.units[i].replans = 0;
        }
    }

    /// Indices into `self.units` of the listed units that exist and belong to
    /// `player`, sorted and without duplicates.
    fn owned_indices(&self, player: PlayerId, ids: &[UnitId]) -> Vec<usize> {
        let mut indices: Vec<usize> = ids
            .iter()
            .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
            .filter(|&index| self.units[index].owner == player && self.units[index].alive())
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
                    height: u.height,
                    target: u.target,
                    facing: u.facing,
                    aim: u.combat.aim,
                    hp: u.hp,
                    max_hp: self.types[u.unit_type as usize].max_hp,
                    fire_mode: u.combat.fire_mode,
                    stance: u.squad.as_ref().map(|s| s.stance),
                    garrison: u.squad.as_ref().and_then(|s| s.garrison),
                    changing_stance: u.squad.as_ref().is_some_and(|s| s.changing > 0),
                    spread: u.spread,
                })
                .collect(),
            projectiles: self
                .projectiles
                .iter()
                .map(|p| p.snapshot(self.tick_hz))
                .collect(),
            events: self.events.clone(),
            soldiers: self
                .units
                .iter()
                .filter_map(|u| u.squad.as_ref().map(|s| (u, s)))
                .flat_map(|(u, s)| {
                    let inside = s.garrison.is_some();
                    (0u16..)
                        .zip(&s.soldiers)
                        .map(move |(index, x)| SoldierSnapshot {
                            unit: u.id,
                            index,
                            position: x.position,
                            height: x.height,
                            hp: x.hp,
                            in_cover: inside || x.at_spot(),
                        })
                })
                .collect(),
        }
    }
}

/// Units of `player` that are not destroyed (see [`Sim::alive_units`]).
fn alive_units(units: &[Unit], player: PlayerId) -> usize {
    units
        .iter()
        .filter(|u| u.owner == player && u.alive())
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
///
/// `near` lists the candidates (indices; every unit within the reach must
/// be among them). The push is a sum, so their order does not matter.
fn separation(
    units: &[Unit],
    directions: &[Option<Vec2>],
    i: usize,
    direction: Vec2,
    near: &[u32],
) -> Vec2 {
    let me = &units[i];
    let left = Vec2::new(direction.y, -direction.x);
    let mut push = Vec2::ZERO;
    for &j in near {
        let j = j as usize;
        let other = &units[j];
        if j == i || !other.alive() || infantry::inside(other) {
            continue;
        }
        let reach = (me.spread + other.spread) / 2;
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
            push = push + away.scale(weight);
        }
        let ahead = d.x * direction.x + d.y * direction.y;
        // Positive: the other unit is on my left.
        let lateral = d.x * left.x + d.y * left.y;
        let same_way =
            directions[j].is_some_and(|o| o.x * direction.x + o.y * direction.y > Fixed::ONE / 2);
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

/// Result of [`move_unit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Moved {
    Ok,
    /// The way to the next waypoint is blocked (impassable terrain, the map
    /// edge or a grade too steep): the unit did not move.
    Blocked,
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
/// reports [`Moved::Blocked`]. Pushed and sliding steps must also stay on
/// cells that `clear` accepts (passable for the unit's size class) unless
/// the unit already stands on a blocked cell; the direct step follows the
/// planned leg and needs no such check.
fn move_unit(
    unit: &mut Unit,
    push: Vec2,
    map: &Map,
    rules: &TerrainRules,
    clear: impl Fn(Vec2) -> bool,
) -> Moved {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return Moved::Ok;
    };
    let (waypoint, last) = (waypoints[*next], *next + 1 == waypoints.len());
    if let Some(direction) = heading(unit) {
        let Some(speed) = ground_speed(unit, direction, map, rules) else {
            return Moved::Blocked;
        };
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
            let slide_x = unit.position + Vec2::new(step.x, Fixed::ZERO);
            let slide_y = unit.position + Vec2::new(Fixed::ZERO, step.y);
            [
                Some(pushed).filter(|&p| detour_ok(p)),
                Some(unit.position + step),
                Some(slide_x).filter(|&p| detour_ok(p)),
                Some(slide_y).filter(|&p| detour_ok(p)),
            ]
        };
        let Some(next_position) = candidates.into_iter().flatten().find(|&p| valid(p)) else {
            return Moved::Blocked;
        };
        unit.position = next_position;
        unit.height = map.height_at(next_position);
    }
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
    if distance < *best {
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
        } else {
            *next += 1;
            *best = Fixed::MAX;
            *stalled = 0;
        }
    }
    Moved::Ok
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
fn ground_speed(unit: &Unit, direction: Vec2, map: &Map, rules: &TerrainRules) -> Option<Fixed> {
    let terrain = &rules.types[usize::from(map.terrain_at(unit.position))];
    let mut speed = terrain.speed(unit.speed_offroad, unit.speed_road);
    if unit.speed_pct != 100 {
        speed = speed * Fixed::from_num(unit.speed_pct) / 100;
    }
    let grade = map.height_at(unit.position + direction) - unit.height;
    Some(speed * rules.slope.factor(grade)?)
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
