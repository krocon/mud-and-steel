//! Thin GDExtension that hosts the simulation inside Godot.
//!
//! This is the only place where simulation values are converted to and from
//! floating point. Godot never decides anything game-relevant: it sends
//! commands in and draws snapshots that come out.

use godot::classes::{INode, Node};
use godot::prelude::*;
use net::{LocalTransport, LockstepConfig, LockstepSession, NetError, Replay, ReplayRecorder};
use sim::scenario::{DEMO_SCENARIO, parse_scenario};
use sim::{Catalog, Command, Fixed, PlayerId, Scale, Sim, SimConfig, Snapshot, UnitId, Vec2};

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

/// A running match: simulation plus lockstep session and replay recorder.
struct Game {
    sim: Sim,
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
/// Positions use Godot coordinates: sim `(x, y)` becomes `(x, 0, y)`.
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
    /// Starts the demo scenario (two players, eight tanks each) with the local
    /// player in slot 0. Returns `false` (and logs an error) if the data is
    /// invalid.
    #[func]
    fn start(&mut self, seed: i64) -> bool {
        let config = match demo_config() {
            Ok(config) => config,
            Err(e) => {
                godot_error!("{e}");
                return false;
            }
        };
        // The seed is a bit pattern; reinterpreting the sign is intended.
        let seed = seed.cast_unsigned();
        let sim = match Sim::new(&config, seed) {
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
            config,
            session,
            transport: LocalTransport,
        });
        true
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
        let result = Replay::decode(bytes.as_slice())
            .and_then(|replay| replay.verify())
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

/// The demo scenario from the catalog and scale compiled into `sim`.
fn demo_config() -> Result<SimConfig, Box<dyn std::error::Error>> {
    let catalog = Catalog::builtin()?;
    let scale = Scale::builtin()?;
    Ok(parse_scenario(DEMO_SCENARIO, &catalog, &scale)?)
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
        .map(|u| ground_to_godot(u.position))
        .collect()
}

fn fixed_to_f32(value: Fixed) -> f32 {
    value.to_num::<f32>()
}

fn ground_to_godot(position: Vec2) -> Vector3 {
    Vector3::new(fixed_to_f32(position.x), 0.0, fixed_to_f32(position.y))
}

fn meters_to_millimeters(meters: f32) -> Option<i32> {
    let millimeters = (f64::from(meters) * 1000.0).round();
    // i32 millimeters cover ±2147 km, far beyond any map.
    (millimeters.is_finite() && millimeters.abs() <= f64::from(i32::MAX))
        .then_some(millimeters as i32)
}
