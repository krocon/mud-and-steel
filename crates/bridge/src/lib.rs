//! Thin GDExtension that hosts the simulation inside Godot.
//!
//! This is the only place where simulation values are converted to and from
//! floating point. Godot never decides anything game-relevant: it sends
//! commands in and draws snapshots that come out.

use godot::classes::{INode, Node};
use godot::prelude::*;
use sim::scenario::DEMO_UNIT;
use sim::{
    Catalog, Command, Fixed, Scale, Sim, SimConfig, Snapshot, Spawn, UnitId, UnitType, Vec2,
};

struct MudAndSteel;

#[gdextension]
unsafe impl ExtensionLibrary for MudAndSteel {}

/// Upper bound of ticks per rendered frame. Prevents a death spiral after a
/// long hitch; the simulation then runs slower than real time instead.
const MAX_TICKS_PER_FRAME: u32 = 5;

/// Runs the simulation at a fixed tick rate and provides snapshots for drawing.
///
/// Positions use Godot coordinates: sim `(x, y)` becomes `(x, 0, y)`.
#[derive(GodotClass)]
#[class(base = Node)]
pub struct SimHost {
    sim: Option<Sim>,
    tick_hz: u32,
    /// Commands collected since the last tick; applied with the next tick.
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
            sim: None,
            tick_hz: SimConfig::DEFAULT_TICK_HZ,
            pending: Vec::new(),
            accumulator: 0.0,
            previous: Snapshot::default(),
            current: Snapshot::default(),
            base,
        }
    }

    fn process(&mut self, delta: f64) {
        let Some(sim) = self.sim.as_mut() else {
            return;
        };
        let tick_seconds = 1.0 / f64::from(self.tick_hz);
        self.accumulator += delta;
        let mut ticks = 0;
        while self.accumulator >= tick_seconds {
            if ticks == MAX_TICKS_PER_FRAME {
                self.accumulator = 0.0;
                break;
            }
            let commands = std::mem::take(&mut self.pending);
            sim.step(&commands);
            self.previous = std::mem::replace(&mut self.current, sim.snapshot());
            self.accumulator -= tick_seconds;
            ticks += 1;
        }
    }
}

#[godot_api]
impl SimHost {
    /// Starts a new simulation with one demo tank at the origin.
    /// Returns `false` (and logs an error) if the catalog data is invalid.
    #[func]
    fn start(&mut self, seed: i64) -> bool {
        let unit_types = match load_unit_types() {
            Ok(types) => types,
            Err(e) => {
                godot_error!("{e}");
                return false;
            }
        };
        let config = SimConfig {
            tick_hz: SimConfig::DEFAULT_TICK_HZ,
            unit_types,
            spawns: vec![Spawn {
                unit_type: DEMO_UNIT.into(),
                position: Vec2::ZERO,
            }],
        };
        // The seed is a bit pattern; reinterpreting the sign is intended.
        match Sim::new(&config, seed.cast_unsigned()) {
            Ok(sim) => {
                self.tick_hz = config.tick_hz;
                self.current = sim.snapshot();
                self.previous = self.current.clone();
                self.sim = Some(sim);
                self.pending.clear();
                self.accumulator = 0.0;
                true
            }
            Err(e) => {
                godot_error!("cannot start simulation: {e}");
                false
            }
        }
    }

    /// Queues a straight-line move to the ground point `(x, z)` in meters.
    /// The target is quantized to millimeters before it enters the simulation.
    #[func]
    fn move_to(&mut self, unit_id: i64, x: f32, z: f32) {
        let (Ok(unit), Some(x_mm), Some(z_mm)) = (
            u32::try_from(unit_id),
            meters_to_millimeters(x),
            meters_to_millimeters(z),
        ) else {
            godot_warn!("move_to ignored: invalid unit {unit_id} or target ({x}, {z})");
            return;
        };
        self.pending.push(Command::MoveTo {
            unit: UnitId(unit),
            target: Vec2::from_millimeters(x_mm, z_mm),
        });
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
            .sim
            .as_ref()
            .map(|sim| format!("{:#018x}", sim.state_hash()))
            .unwrap_or_default();
        GString::from(hash.as_str())
    }

    /// Unit ids of the latest snapshot, in the order of the position arrays.
    #[func]
    fn unit_ids(&self) -> PackedInt64Array {
        self.current
            .units
            .iter()
            .map(|u| i64::from(u.id.0))
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
}

/// Converts the demo unit from the catalog and scale compiled into `sim`.
fn load_unit_types() -> Result<Vec<UnitType>, Box<dyn std::error::Error>> {
    let catalog = Catalog::builtin()?;
    let scale = Scale::builtin()?;
    Ok(sim::unit_types_from_catalog(
        &catalog,
        &scale,
        &[DEMO_UNIT],
    )?)
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
