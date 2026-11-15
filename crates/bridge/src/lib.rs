//! Thin GDExtension that hosts the simulation inside Godot.
//!
//! This is the only place where simulation values are converted to and from
//! floating point. Godot never decides anything game-relevant: it sends
//! commands in and draws snapshots that come out.

use godot::classes::{INode, Node};
use godot::prelude::*;
use net::{LocalTransport, LockstepConfig, LockstepSession, NetError, Replay, ReplayRecorder};
use sim::scenario::{DEMO_SCENARIO, builtin_map, parse_scenario};
use sim::{
    Catalog, Command, Fixed, Map, PlayerId, RouteView, Scale, Sim, SimConfig, Snapshot,
    TerrainRules, UnitId, Vec2,
};
use std::path::Path;
use std::sync::Arc;

struct MudAndSteel;

#[gdextension]
unsafe impl ExtensionLibrary for MudAndSteel {}

/// Upper bound of ticks per rendered frame. Prevents a death spiral after a
/// long hitch; the simulation then runs slower than real time instead.
const MAX_TICKS_PER_FRAME: u32 = 5;

/// The player controlled on this machine. Player 1 of the demo scenario has
/// no input yet (the AI of prompt 024 will feed it through its own frames).
const LOCAL_PLAYER: PlayerId = PlayerId(0);

/// Hash exchange interval of the lockstep session. Single player has nobody
/// to compare with, but uses the same protocol.
const HASH_INTERVAL: u64 = 20;

/// Rays longer than this (meters) do not hit the ground in `ray_pick`.
const MAX_PICK_DISTANCE: f32 = 20_000.0;

/// A running match: simulation plus lockstep session and replay recorder.
struct Game {
    sim: Sim,
    map: Arc<Map>,
    /// Highest point of the map in meters, for ray picking.
    map_top: f32,
    config: SimConfig,
    session: LockstepSession,
    transport: LocalTransport,
    recorder: ReplayRecorder,
}

impl Game {
    /// Runs one round of the lockstep protocol. Returns whether a tick was
    /// simulated (always true with a single local player).
    fn step(&mut self, local: Vec<Command>) -> Result<bool, NetError> {
        if self.session.needs_local_input() {
            self.session.submit_local(local, &mut self.transport)?;
        }
        self.session.receive(&mut self.transport)?;
        let Some(commands) = self.session.next_commands() else {
            return Ok(false);
        };
        self.recorder.record(&commands);
        self.sim.step(&commands);
        self.session
            .record_hash(self.sim.tick(), self.sim.state_hash(), &mut self.transport)?;
        Ok(true)
    }
}

/// Runs the simulation at a fixed tick rate and provides snapshots for drawing.
///
/// All input goes through a `net::LockstepSession`, exactly like in
/// multiplayer: commands are scheduled `input_delay` ticks ahead.
///
/// Positions use Godot coordinates: sim `(x, y)` at ground height `h`
/// becomes `(x, h, y)`.
#[derive(GodotClass)]
#[class(base = Node)]
pub struct SimHost {
    /// Ticks between issuing a command and its execution. Read by `start`.
    #[export]
    input_delay: u32,
    game: Option<Game>,
    tick_hz: u32,
    /// Commands collected since the last tick; submitted with the next one.
    pending: Vec<Command>,
    /// Real time not yet consumed by ticks, in seconds.
    accumulator: f64,
    previous: Snapshot,
    current: Snapshot,
    base: Base<Node>,
}

#[godot_api]
impl INode for SimHost {
    fn init(base: Base<Node>) -> Self {
        Self {
            input_delay: 1,
            game: None,
            tick_hz: SimConfig::DEFAULT_TICK_HZ,
            pending: Vec::new(),
            accumulator: 0.0,
            previous: Snapshot::default(),
            current: Snapshot::default(),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let tick_seconds = 1.0 / f64::from(self.tick_hz);
        self.accumulator += delta;
        let mut ticks = 0;
        let mut failed = false;
        while self.accumulator >= tick_seconds {
            if ticks == MAX_TICKS_PER_FRAME {
                self.accumulator = 0.0;
                break;
            }
            match game.step(std::mem::take(&mut self.pending)) {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    godot_error!("lockstep failed, stopping the match: {e}");
                    failed = true;
                    break;
                }
            }
            self.previous = std::mem::replace(&mut self.current, game.sim.snapshot());
            self.accumulator -= tick_seconds;
            ticks += 1;
        }
        if failed {
            self.game = None;
        }
    }
}

#[godot_api]
impl SimHost {
    /// Starts the demo scenario (two players, eight tanks each) on the
    /// built-in map `test_small` with the local player in slot 0. Returns
    /// `false` (and logs an error) if the data is invalid.
    #[func]
    fn start(&mut self, seed: i64) -> bool {
        self.start_with(seed, builtin_map())
    }

    /// Like `start`, but on the map in the folder `map_dir` (absolute path
    /// of a `data/maps/<name>/` folder).
    #[func]
    fn start_on_map(&mut self, seed: i64, map_dir: GString) -> bool {
        match load_map_dir(Path::new(&map_dir.to_string())) {
            Ok(map) => self.start_with(seed, Arc::new(map)),
            Err(e) => {
                godot_error!("{e}");
                false
            }
        }
    }

    fn start_with(&mut self, seed: i64, map: Arc<Map>) -> bool {
        let config = match demo_config(&map) {
            Ok(config) => config,
            Err(e) => {
                godot_error!("{e}");
                return false;
            }
        };
        // The seed is a bit pattern; reinterpreting the sign is intended.
        let seed = seed.cast_unsigned();
        let sim = match Sim::new(&config, map.clone(), seed) {
            Ok(sim) => sim,
            Err(e) => {
                godot_error!("cannot start simulation: {e}");
                return false;
            }
        };
        let players = vec![LOCAL_PLAYER];
        let session = LockstepSession::new(
            LOCAL_PLAYER,
            &LockstepConfig {
                players: players.clone(),
                input_delay: self.input_delay,
                hash_interval: HASH_INTERVAL,
            },
        );
        self.tick_hz = config.tick_hz;
        self.current = sim.snapshot();
        self.previous = self.current.clone();
        self.pending.clear();
        self.accumulator = 0.0;
        self.game = Some(Game {
            recorder: ReplayRecorder::new(seed, config.clone(), &players),
            sim,
            map_top: map
                .heights_mm()
                .iter()
                .max()
                .map_or(0.0, |&h| h as f32 / 1000.0),
            map,
            config,
            session,
            transport: LocalTransport,
        });
        true
    }

    /// Name of the loaded map ("" before `start`).
    #[func]
    fn map_name(&self) -> GString {
        self.game
            .as_ref()
            .map_or_else(GString::new, |g| GString::from(g.map.name()))
    }

    /// Width (x) and depth (z) of the map in meters. The map is centered on
    /// the origin. Zero before `start`.
    #[func]
    fn map_size(&self) -> Vector2 {
        self.game.as_ref().map_or(Vector2::ZERO, |g| {
            let size = g.map.size();
            Vector2::new(fixed_to_f32(size.x), fixed_to_f32(size.y))
        })
    }

    /// Ground height at `(x, z)` in meters, exactly as the simulation
    /// computes it (input quantized to millimeters).
    #[func]
    fn height_at(&self, x: f32, z: f32) -> f32 {
        let (Some(game), Some(x), Some(z)) = (
            self.game.as_ref(),
            meters_to_millimeters(x),
            meters_to_millimeters(z),
        ) else {
            return 0.0;
        };
        fixed_to_f32(game.map.height_at(Vec2::from_millimeters(x, z)))
    }

    /// Number of samples per axis of `height_grid(stride)` and
    /// `terrain_grid(stride)`.
    #[func]
    fn grid_samples(&self, stride: i64) -> Vector2i {
        self.game.as_ref().map_or(Vector2i::ZERO, |g| {
            let (sx, sy) = g.map.samples();
            let stride = clamp_stride(stride);
            let count = |n: u32| i32::try_from((n - 1).div_ceil(stride) + 1).unwrap_or(0);
            Vector2i::new(count(sx), count(sy))
        })
    }

    /// Heights in meters of every `stride`-th sample, row by row (z), the
    /// last row and column always included. Sample spacing is `stride`
    /// cells; the map's cell size is `cell_size()`.
    #[func]
    fn height_grid(&self, stride: i64) -> PackedFloat32Array {
        self.game
            .as_ref()
            .map_or_else(PackedFloat32Array::new, |g| {
                grid_indices(&g.map, clamp_stride(stride))
                    .map(|(i, j)| g.map.sample_height_mm(i, j) as f32 / 1000.0)
                    .collect()
            })
    }

    /// Terrain ids of the same samples as `height_grid(stride)`.
    #[func]
    fn terrain_grid(&self, stride: i64) -> PackedByteArray {
        self.game.as_ref().map_or_else(PackedByteArray::new, |g| {
            let (sx, _) = g.map.samples();
            let ids = g.map.terrain_ids();
            grid_indices(&g.map, clamp_stride(stride))
                .map(|(i, j)| ids[j as usize * sx as usize + i as usize])
                .collect()
        })
    }

    /// Terrain id at `(x, z)` (nearest sample, like the simulation).
    #[func]
    fn terrain_at(&self, x: f32, z: f32) -> i64 {
        let (Some(game), Some(x), Some(z)) = (
            self.game.as_ref(),
            meters_to_millimeters(x),
            meters_to_millimeters(z),
        ) else {
            return -1;
        };
        i64::from(game.map.terrain_at(Vec2::from_millimeters(x, z)))
    }

    /// Speed of a terrain type in percent of its basis speed (-1 if unknown).
    #[func]
    fn terrain_speed_pct(&self, id: i64) -> i64 {
        self.terrain_type(id).map_or(-1, |t| i64::from(t.speed_pct))
    }

    /// Whether units use their road speed (instead of cross-country) on a
    /// terrain type.
    #[func]
    fn terrain_uses_road(&self, id: i64) -> bool {
        self.terrain_type(id)
            .is_some_and(|t| t.speed == sim::terrain::SpeedBasis::Road)
    }

    /// Distance between neighboring samples in meters.
    #[func]
    fn cell_size(&self) -> f32 {
        self.game
            .as_ref()
            .map_or(0.0, |g| g.map.cell_size_mm() as f32 / 1000.0)
    }

    /// Debug color of every terrain type, indexed by terrain id.
    #[func]
    fn terrain_colors(&self) -> PackedColorArray {
        self.game.as_ref().map_or_else(PackedColorArray::new, |g| {
            g.config
                .terrain
                .types
                .iter()
                .map(|t| {
                    let [_, r, g, b] = t.debug_color.to_be_bytes();
                    Color::from_rgba8(r, g, b, 255)
                })
                .collect()
        })
    }

    /// Ground point (with height) of a marker from `map.toml`, e.g.
    /// "hill_top", or null if the map has no such marker.
    #[func]
    fn map_marker(&self, name: GString) -> Variant {
        self.game
            .as_ref()
            .and_then(|g| {
                let p = g.map.marker(&name.to_string())?;
                Some(ground_to_godot(p, g.map.height_at(p)).to_variant())
            })
            .unwrap_or_default()
    }

    /// First point where the ray from `origin` along `direction` hits the
    /// height field, or null if it misses (sky, beyond 20 km).
    ///
    /// Presentation only (floats): marches along the ray in steps that grow
    /// with the distance, then refines the hit by bisection. Outside the map
    /// the edge heights continue, like in the simulation.
    #[func]
    fn ray_pick(&self, origin: Vector3, direction: Vector3) -> Variant {
        let Some(game) = &self.game else {
            return Variant::nil();
        };
        ray_pick(&game.map, game.map_top, origin, direction)
            .map_or_else(Variant::nil, |p| p.to_variant())
    }

    fn terrain_type(&self, id: i64) -> Option<&sim::terrain::TerrainType> {
        let game = self.game.as_ref()?;
        game.config.terrain.types.get(usize::try_from(id).ok()?)
    }

    /// Id of the player controlled on this machine.
    #[func]
    fn local_player(&self) -> i64 {
        i64::from(LOCAL_PLAYER.0)
    }

    /// Queues a move of the given units to a formation around the ground
    /// point `(x, z)` in meters. The target is quantized to millimeters before
    /// it enters the simulation. Units of other players are ignored by the
    /// simulation.
    #[func]
    fn move_units(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) {
        let (Some(x_mm), Some(z_mm)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            godot_warn!("move_units ignored: invalid target ({x}, {z})");
            return;
        };
        let units = to_unit_ids(&unit_ids);
        if units.is_empty() {
            return;
        }
        self.pending.push(Command::MoveTo {
            units,
            target: Vec2::from_millimeters(x_mm, z_mm),
        });
    }

    /// Queues a stop of the given units.
    #[func]
    fn stop_units(&mut self, unit_ids: PackedInt64Array) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::Stop { units });
        }
    }

    #[func]
    fn tick_rate(&self) -> i64 {
        i64::from(self.tick_hz)
    }

    /// Number of simulated ticks.
    #[func]
    fn current_tick(&self) -> i64 {
        i64::try_from(self.current.tick).unwrap_or(i64::MAX)
    }

    /// State hash as hex string, for debugging and desync reports.
    #[func]
    fn state_hash(&self) -> GString {
        let hash = self
            .game
            .as_ref()
            .map(|game| format!("{:#018x}", game.sim.state_hash()))
            .unwrap_or_default();
        GString::from(hash.as_str())
    }

    /// Unit ids of the latest snapshot. All per-unit arrays use this order.
    #[func]
    fn unit_ids(&self) -> PackedInt64Array {
        self.current
            .units
            .iter()
            .map(|u| i64::from(u.id.0))
            .collect()
    }

    /// Owning player of every unit.
    #[func]
    fn unit_owners(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| i32::from(u.owner.0))
            .collect()
    }

    /// Unit type index of every unit; equal values mean the same type.
    #[func]
    fn unit_types(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| i32::try_from(u.unit_type).unwrap_or(-1))
            .collect()
    }

    /// Catalog id of every unit's type (e.g. `de_pz4_h`), for picking models.
    #[func]
    fn unit_type_ids(&self) -> PackedStringArray {
        let Some(game) = &self.game else {
            return PackedStringArray::new();
        };
        self.current
            .units
            .iter()
            .map(|u| {
                let name = usize::try_from(u.unit_type)
                    .ok()
                    .and_then(|i| game.config.unit_types.get(i))
                    .map_or("", |t| t.name.as_str());
                GString::from(name)
            })
            .collect()
    }

    /// Larger of length and width of every unit, in meters.
    #[func]
    fn unit_footprints(&self) -> PackedFloat32Array {
        let Some(game) = &self.game else {
            return PackedFloat32Array::new();
        };
        self.current
            .units
            .iter()
            .map(|u| {
                usize::try_from(u.unit_type)
                    .ok()
                    .and_then(|i| game.config.unit_types.get(i))
                    .map_or(0.0, |t| fixed_to_f32(t.footprint_m))
            })
            .collect()
    }

    /// Whether every unit is moving (has a target).
    #[func]
    fn unit_moving(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.target.is_some()))
            .collect()
    }

    /// Unit positions of the latest snapshot.
    #[func]
    fn current_positions(&self) -> PackedVector3Array {
        positions(&self.current)
    }

    /// Unit positions of the snapshot before the latest one.
    #[func]
    fn previous_positions(&self) -> PackedVector3Array {
        positions(&self.previous)
    }

    /// How far real time has progressed from the previous towards the latest
    /// snapshot, in `0.0..=1.0`. Used to interpolate positions between ticks.
    #[func]
    fn interpolation_alpha(&self) -> f32 {
        let alpha = self.accumulator * f64::from(self.tick_hz);
        alpha.clamp(0.0, 1.0) as f32
    }

    // --- Pathfinding debug view (prompt 012) ---------------------------

    /// Number of path cells per axis (x, z). Zero before `start`.
    #[func]
    fn path_grid_size(&self) -> Vector2i {
        self.game.as_ref().map_or(Vector2i::ZERO, |g| {
            let (w, h) = g.sim.path_grid().size();
            Vector2i::new(i32::try_from(w).unwrap_or(0), i32::try_from(h).unwrap_or(0))
        })
    }

    /// Edge length of a path cell in meters.
    #[func]
    fn path_cell_size(&self) -> f32 {
        self.game
            .as_ref()
            .map_or(0.0, |g| fixed_to_f32(g.sim.path_grid().cell_size()))
    }

    /// Corner (smallest x, z) of path cell (0, 0) in meters.
    #[func]
    fn path_grid_origin(&self) -> Vector2 {
        self.game.as_ref().map_or(Vector2::ZERO, |g| {
            let o = g.sim.path_grid().origin();
            Vector2::new(fixed_to_f32(o.x), fixed_to_f32(o.y))
        })
    }

    /// The path cost grid of a unit type (index as in `unit_types()`) as
    /// RGBA8 pixels, one per cell, row by row (z): blocked cells red,
    /// cheap cells green, expensive ones yellow to purple. Empty if the
    /// type is unknown.
    #[func]
    fn path_cost_image(&self, unit_type: i64) -> PackedByteArray {
        let Some(game) = &self.game else {
            return PackedByteArray::new();
        };
        let grid = game.sim.path_grid();
        let Some(profile) = usize::try_from(unit_type)
            .ok()
            .filter(|&p| p < grid.profiles())
        else {
            return PackedByteArray::new();
        };
        let min = f32::from(sim::path::MIN_CELL_COST);
        grid.costs(profile)
            .iter()
            .flat_map(|&cost| {
                if cost == 0 {
                    return [200, 30, 30, 170];
                }
                // 0 at the cheapest cell, 1 at ten times its cost.
                let t = ((f32::from(cost) / min).log10()).clamp(0.0, 1.0);
                let color = if t < 0.5 {
                    let u = t * 2.0;
                    Color::from_rgb(0.2 + 0.8 * u, 0.85, 0.2 * (1.0 - u))
                } else {
                    let u = (t - 0.5) * 2.0;
                    Color::from_rgb(1.0 - 0.4 * u, 0.85 * (1.0 - u), 0.6 * u)
                };
                let alpha = (40.0 + 140.0 * t) as u8;
                [color.r8(), color.g8(), color.b8(), alpha]
            })
            .collect()
    }

    /// Remaining waypoints of all units with a route, in the order of
    /// `unit_ids()`, as ground points; `unit_path_counts()` says how many
    /// belong to each unit.
    #[func]
    fn unit_path_points(&self) -> PackedVector3Array {
        let Some(game) = &self.game else {
            return PackedVector3Array::new();
        };
        let mut points = PackedVector3Array::new();
        for unit in &game.sim.snapshot().units {
            if let Some(RouteView::Following(waypoints)) = game.sim.route_of(unit.id) {
                for &w in waypoints {
                    points.push(ground_to_godot(w, game.map.height_at(w)));
                }
            }
        }
        points
    }

    /// Number of waypoints per unit in `unit_path_points()`, in the order of
    /// `unit_ids()`; -1 for units whose path is still being planned.
    #[func]
    fn unit_path_counts(&self) -> PackedInt32Array {
        let Some(game) = &self.game else {
            return PackedInt32Array::new();
        };
        game.sim
            .snapshot()
            .units
            .iter()
            .map(|u| match game.sim.route_of(u.id) {
                Some(RouteView::Following(w)) => i32::try_from(w.len()).unwrap_or(i32::MAX),
                Some(RouteView::Waiting) => -1,
                _ => 0,
            })
            .collect()
    }

    /// Path requests not finished yet.
    #[func]
    fn path_queue_len(&self) -> i64 {
        self.game.as_ref().map_or(0, |g| {
            i64::try_from(g.sim.path_queue_len()).unwrap_or(i64::MAX)
        })
    }

    /// The match so far as replay file contents (see `net::replay`).
    /// Empty if no match is running.
    #[func]
    fn replay_bytes(&self) -> PackedByteArray {
        self.game
            .as_ref()
            .map_or_else(PackedByteArray::new, |game| {
                PackedByteArray::from(game.recorder.finish(&game.sim).encode())
            })
    }

    /// Plays replay file contents headless. Returns an empty string if the
    /// end hash matches, otherwise the reason.
    #[func]
    fn verify_replay(&self, bytes: PackedByteArray) -> GString {
        // The replay names its map by content hash. The running match's map
        // is the usual case; the built-in map covers replays of the demo.
        let running = self.game.as_ref().map(|g| g.map.clone());
        let result = Replay::decode(bytes.as_slice())
            .and_then(|replay| {
                let map = running
                    .filter(|m| m.reference() == replay.config.map)
                    .unwrap_or_else(builtin_map);
                replay.verify(map)
            })
            .map_err(|e| e.to_string())
            .and_then(|report| {
                if report.matches() {
                    Ok(())
                } else {
                    Err(format!(
                        "end hash mismatch after {} ticks: expected {:#018x}, got {:#018x}",
                        report.ticks, report.expected_hash, report.actual_hash
                    ))
                }
            });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

/// The demo scenario from the catalog, scale and terrain rules compiled
/// into `sim`, on `map`.
fn demo_config(map: &Map) -> Result<SimConfig, Box<dyn std::error::Error>> {
    let catalog = Catalog::builtin()?;
    let scale = Scale::builtin()?;
    let terrain = TerrainRules::builtin()?;
    Ok(parse_scenario(
        DEMO_SCENARIO,
        &catalog,
        &scale,
        &terrain,
        map,
    )?)
}

/// Reads and decodes `dir/map.toml`, `dir/height.png` and `dir/terrain.png`.
fn load_map_dir(dir: &Path) -> Result<Map, String> {
    let read = |file: &str| {
        let path = dir.join(file);
        std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let toml = String::from_utf8(read("map.toml")?).map_err(|e| e.to_string())?;
    Map::load(&toml, &read("height.png")?, &read("terrain.png")?)
        .map_err(|e| format!("{}: {e}", dir.display()))
}

fn clamp_stride(stride: i64) -> u32 {
    u32::try_from(stride.clamp(1, 1024)).unwrap_or(1)
}

/// Sample indices `(i, j)` of every `stride`-th sample per axis, row by row,
/// always including the last row and column.
fn grid_indices(map: &Map, stride: u32) -> impl Iterator<Item = (u32, u32)> {
    let (sx, sy) = map.samples();
    let axis =
        move |n: u32| (0..(n - 1).div_ceil(stride) + 1).map(move |k| (k * stride).min(n - 1));
    axis(sy).flat_map(move |j| axis(sx).map(move |i| (i, j)))
}

/// Bilinear height in float, for ray picking. Matches the simulation to
/// well below a millimeter, which is plenty for the mouse.
fn height_f32(map: &Map, x: f32, z: f32) -> f32 {
    let (sx, sy) = map.samples();
    let size = map.size();
    let cell = map.cell_size_mm() as f32 / 1000.0;
    let grid = |v: f32, half: f32, n: u32| ((v + half) / cell).clamp(0.0, (n - 1) as f32);
    let gx = grid(x, fixed_to_f32(size.x) / 2.0, sx);
    let gz = grid(z, fixed_to_f32(size.y) / 2.0, sy);
    let i = (gx as u32).min(sx - 2);
    let j = (gz as u32).min(sy - 2);
    let (fx, fz) = (gx - i as f32, gz - j as f32);
    let h = |i: u32, j: u32| map.sample_height_mm(i, j) as f32;
    let top = h(i, j) + (h(i + 1, j) - h(i, j)) * fx;
    let bottom = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * fx;
    (top + (bottom - top) * fz) / 1000.0
}

fn ray_pick(map: &Map, top: f32, origin: Vector3, direction: Vector3) -> Option<Vector3> {
    let direction = direction.try_normalized()?;
    let above = |t: f32| {
        let p = origin + direction * t;
        p.y - height_f32(map, p.x, p.z)
    };
    // Skip the part of the ray above the highest point of the map.
    let mut t = 0.0;
    if origin.y > top {
        if direction.y >= 0.0 {
            return None;
        }
        t = (origin.y - top) / -direction.y;
    }
    if above(t) <= 0.0 {
        let p = origin + direction * t;
        return Some(Vector3::new(p.x, height_f32(map, p.x, p.z), p.z));
    }
    // March: steps of 0.5 m near the camera, growing to 20 m far away.
    let mut hit = None;
    while t < MAX_PICK_DISTANCE {
        let step = (0.5 + t * 0.01).min(20.0);
        if above(t + step) <= 0.0 {
            hit = Some((t, t + step));
            break;
        }
        t += step;
        let p = origin + direction * t;
        if p.y > top && direction.y >= 0.0 {
            return None;
        }
    }
    let (mut low, mut high) = hit?;
    for _ in 0..24 {
        let mid = 0.5 * (low + high);
        if above(mid) > 0.0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    let p = origin + direction * high;
    Some(Vector3::new(p.x, height_f32(map, p.x, p.z), p.z))
}

fn to_unit_ids(ids: &PackedInt64Array) -> Vec<UnitId> {
    ids.as_slice()
        .iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .map(UnitId)
        .collect()
}

fn positions(snapshot: &Snapshot) -> PackedVector3Array {
    snapshot
        .units
        .iter()
        .map(|u| ground_to_godot(u.position, u.height))
        .collect()
}

fn fixed_to_f32(value: Fixed) -> f32 {
    value.to_num::<f32>()
}

fn ground_to_godot(position: Vec2, height: Fixed) -> Vector3 {
    Vector3::new(
        fixed_to_f32(position.x),
        fixed_to_f32(height),
        fixed_to_f32(position.y),
    )
}

fn meters_to_millimeters(meters: f32) -> Option<i32> {
    let millimeters = (f64::from(meters) * 1000.0).round();
    // i32 millimeters cover ±2147 km, far beyond any map.
    (millimeters.is_finite() && millimeters.abs() <= f64::from(i32::MAX))
        .then_some(millimeters as i32)
}
