//! `SimHost` functions for air support and anti-aircraft fire (prompt 033):
//! calling a mission, what the local player can call, aircraft, bombs and
//! own sorties of the current snapshot, and the air event stream. Godot
//! only draws what comes out of here.

use crate::{SimHost, fixed_to_f32, meters_to_millimeters};
use godot::prelude::*;
use sim::{AirEvent, AirMission, Command, FlakClass, PlayerId, Sim, UnitId, Vec2, Vec3};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 4096;

/// Appends the air events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, AirEvent)>, sim: &Sim) {
    let tick = sim.tick().saturating_sub(1);
    pending.extend(sim.air_events().iter().map(|&e| (tick, e)));
    if pending.len() > MAX_PENDING_EVENTS {
        let excess = pending.len() - MAX_PENDING_EVENTS;
        pending.drain(..excess);
    }
}

fn to_godot(p: Vec3) -> Vector3 {
    Vector3::new(fixed_to_f32(p.x), fixed_to_f32(p.z), fixed_to_f32(p.y))
}

fn flat(p: Vec2) -> Vector3 {
    Vector3::new(fixed_to_f32(p.x), 0.0, fixed_to_f32(p.y))
}

fn mission_of(name: &str) -> Option<AirMission> {
    AirMission::ALL.into_iter().find(|m| m.name() == name)
}

const fn class_name(class: FlakClass) -> &'static str {
    match class {
        FlakClass::Mg => "mg",
        FlakClass::Light => "light",
        FlakClass::Heavy => "heavy",
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Calls the air mission `mission` ("recon", "strafe", "attack",
    /// "bomb") on the ground point `(x, z)` in meters.
    #[func]
    fn call_air(&mut self, mission: GString, x: f32, z: f32) {
        let (Some(x_mm), Some(z_mm)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            return;
        };
        let Some(mission) = mission_of(&mission.to_string()) else {
            return;
        };
        self.pending.push(Command::CallAir {
            mission,
            target: Vec2::from_millimeters(x_mm, z_mm),
        });
    }

    /// What the local player can call, one dictionary per mission:
    /// `mission`, `aircraft` (catalog id or ""), `aircraft_name`, `cost` (fuel, ammo,
    /// manpower), `cooldown` and `cooldown_total` (seconds), `blocked`
    /// ("" if it can be called now, else the reason).
    #[func]
    fn air_options(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let local = game.session.local_player();
        let hz = self.tick_hz.max(1) as f32;
        for o in game.sim.air_options(local) {
            let mut d = VarDictionary::new();
            d.set("mission", o.mission.name());
            d.set("aircraft", o.aircraft.clone().unwrap_or_default());
            d.set(
                "aircraft_name",
                o.aircraft
                    .as_deref()
                    .map(crate::economy_funcs::display_name)
                    .unwrap_or_default(),
            );
            let cost: PackedInt32Array = o
                .cost
                .iter()
                .map(|&c| i32::try_from(c).unwrap_or(i32::MAX))
                .collect();
            d.set("cost", &cost);
            d.set("cooldown", o.cooldown_left as f32 / hz);
            d.set("cooldown_total", o.cooldown_total as f32 / hz);
            d.set("blocked", o.blocked.map_or("", |b| b.name()));
            out.push(&d);
        }
        out
    }

    /// Ids of the aircraft the local player sees.
    #[func]
    fn aircraft_ids(&self) -> PackedInt64Array {
        self.current
            .aircraft
            .iter()
            .map(|a| i64::from(a.id))
            .collect()
    }

    /// Positions of `aircraft_ids` (Godot coordinates).
    #[func]
    fn aircraft_positions(&self) -> PackedVector3Array {
        self.current
            .aircraft
            .iter()
            .map(|a| to_godot(a.position))
            .collect()
    }

    /// Velocities of `aircraft_ids` in meters per second.
    #[func]
    fn aircraft_velocities(&self) -> PackedVector3Array {
        self.current
            .aircraft
            .iter()
            .map(|a| to_godot(a.velocity))
            .collect()
    }

    /// Owners of `aircraft_ids`.
    #[func]
    fn aircraft_owners(&self) -> PackedInt32Array {
        self.current
            .aircraft
            .iter()
            .map(|a| i32::from(a.owner.0))
            .collect()
    }

    /// Catalog ids of `aircraft_ids` (their models).
    #[func]
    fn aircraft_types(&self) -> PackedStringArray {
        let Some(game) = &self.game else {
            return PackedStringArray::new();
        };
        let types = game.sim.air_types();
        self.current
            .aircraft
            .iter()
            .map(|a| {
                types
                    .get(usize::from(a.kind))
                    .map_or_else(GString::new, |t| GString::from(t.name.as_str()))
            })
            .collect()
    }

    /// Hit points of `aircraft_ids` as a share of the maximum; negative
    /// while shot down and falling.
    #[func]
    fn aircraft_health(&self) -> PackedFloat32Array {
        self.current
            .aircraft
            .iter()
            .map(|a| {
                if a.crashing {
                    -1.0
                } else {
                    a.hp as f32 / a.max_hp.max(1) as f32
                }
            })
            .collect()
    }

    /// Whether the guns of `aircraft_ids` fire this tick (1) or not (0).
    #[func]
    fn aircraft_firing(&self) -> PackedByteArray {
        self.current
            .aircraft
            .iter()
            .map(|a| u8::from(a.firing))
            .collect()
    }

    /// Wingspans of the catalog ids of `aircraft_types` (meters).
    #[func]
    fn aircraft_wingspan(&self, type_id: GString) -> f32 {
        let name = type_id.to_string();
        self.game
            .as_ref()
            .and_then(|g| g.sim.air_types().iter().find(|t| t.name == name))
            .map_or(0.0, |t| fixed_to_f32(t.wingspan_m))
    }

    /// Falling bombs: positions.
    #[func]
    fn bomb_positions(&self) -> PackedVector3Array {
        self.current
            .bombs
            .iter()
            .map(|b| to_godot(b.position))
            .collect()
    }

    /// Velocities of `bomb_positions` in meters per second.
    #[func]
    fn bomb_velocities(&self) -> PackedVector3Array {
        self.current
            .bombs
            .iter()
            .map(|b| to_godot(b.velocity))
            .collect()
    }

    /// The local player's own called missions: `id`, `mission`, `target`,
    /// `entry`, `exit` (ground points, y = 0), `seconds` until the first
    /// aircraft passes the target, `aircraft` still flying.
    #[func]
    fn air_sorties(&self) -> Array<VarDictionary> {
        let hz = self.tick_hz.max(1) as f32;
        let local = self
            .game
            .as_ref()
            .map_or(PlayerId(0), |g| g.session.local_player());
        let mut out = Array::new();
        for s in self.current.sorties.iter().filter(|s| s.owner == local) {
            let mut d = VarDictionary::new();
            d.set("id", i64::from(s.id));
            d.set("mission", s.mission.name());
            d.set("target", flat(s.target));
            d.set("entry", flat(s.entry));
            d.set("exit", flat(s.exit));
            d.set("seconds", s.ticks_to_target as f32 / hz);
            d.set("aircraft", i64::from(s.aircraft));
            out.push(&d);
        }
        out
    }

    /// Range (m) of the longest anti-aircraft weapon of a unit; 0 if none.
    #[func]
    fn unit_aa_range(&self, unit_id: i64) -> f32 {
        let Ok(id) = u32::try_from(unit_id) else {
            return 0.0;
        };
        self.game
            .as_ref()
            .and_then(|g| g.sim.aa_range(UnitId(id)))
            .map_or(0.0, fixed_to_f32)
    }

    /// The air events since the last call, oldest first, as dictionaries
    /// with `type` and `tick`:
    ///
    /// - called: `owner`, `mission`, `target` (ground point)
    /// - flak: `unit`, `aircraft`, `position` (burst), `class` ("mg",
    ///   "light", "heavy"), `hit`
    /// - strafe: `aircraft`, `muzzle`, `from`, `to`, `caliber_mm`
    /// - released: `aircraft`, `bomb`, `position`
    /// - shot_down: `aircraft`, `owner`, `by`, `position`
    /// - crashed: `aircraft`, `owner`, `aircraft_type` (catalog id),
    ///   `position`, `yaw` (Godot `rotation.y` of the model)
    ///
    /// Events are not filtered by the fog of war; the view checks
    /// visibility.
    #[func]
    fn take_air_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.air_events);
        let types: Vec<String> = self.game.as_ref().map_or_else(Vec::new, |g| {
            g.sim.air_types().iter().map(|t| t.name.clone()).collect()
        });
        let mut out = Array::new();
        for (tick, event) in events {
            let mut d = VarDictionary::new();
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            match event {
                AirEvent::Called {
                    owner,
                    mission,
                    target,
                    ..
                } => {
                    d.set("type", "called");
                    d.set("owner", i64::from(owner.0));
                    d.set("mission", mission.name());
                    d.set("target", flat(target));
                }
                AirEvent::Flak {
                    unit,
                    aircraft,
                    at,
                    class,
                    hit,
                } => {
                    d.set("type", "flak");
                    d.set("unit", i64::from(unit.0));
                    d.set("aircraft", i64::from(aircraft));
                    d.set("position", to_godot(at));
                    d.set("class", class_name(class));
                    d.set("hit", hit);
                }
                AirEvent::Strafe {
                    aircraft,
                    muzzle,
                    from,
                    to,
                    caliber_mm_x10,
                } => {
                    d.set("type", "strafe");
                    d.set("aircraft", i64::from(aircraft));
                    d.set("muzzle", to_godot(muzzle));
                    d.set("from", to_godot(from));
                    d.set("to", to_godot(to));
                    d.set("caliber_mm", caliber_mm_x10 as f32 / 10.0);
                }
                AirEvent::Released { aircraft, bomb, at } => {
                    d.set("type", "released");
                    d.set("aircraft", i64::from(aircraft));
                    d.set("bomb", i64::from(bomb));
                    d.set("position", to_godot(at));
                }
                AirEvent::ShotDown {
                    aircraft,
                    owner,
                    by,
                    at,
                } => {
                    d.set("type", "shot_down");
                    d.set("aircraft", i64::from(aircraft));
                    d.set("owner", i64::from(owner.0));
                    d.set("by", i64::from(by.0));
                    d.set("position", to_godot(at));
                }
                AirEvent::Crashed {
                    aircraft,
                    owner,
                    kind,
                    at,
                    facing,
                } => {
                    d.set("type", "crashed");
                    d.set("aircraft", i64::from(aircraft));
                    d.set("owner", i64::from(owner.0));
                    d.set(
                        "aircraft_type",
                        types.get(usize::from(kind)).cloned().unwrap_or_default(),
                    );
                    d.set("position", to_godot(at));
                    d.set(
                        "yaw",
                        (-fixed_to_f32(facing.x)).atan2(-fixed_to_f32(facing.y)),
                    );
                }
            }
            out.push(&d);
        }
        out
    }
}
