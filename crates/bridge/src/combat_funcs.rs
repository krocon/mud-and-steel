//! `SimHost` functions for combat (prompt 017): orders, hit points,
//! turret directions, projectiles and the event stream for effects and
//! sound. Godot only draws what comes out of here.

use crate::{SimHost, fixed_to_f32, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::{CombatEvent, Command, Face, FireMode, HitKind, PlayerId, Sim, UnitId, Vec2, Vec3};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 8192;

/// Appends the events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, CombatEvent)>, sim: &Sim) {
    // Read after the tick, so its events belong to tick - 1.
    let tick = sim.tick().saturating_sub(1);
    pending.extend(sim.events().iter().map(|&e| (tick, e)));
    if pending.len() > MAX_PENDING_EVENTS {
        let excess = pending.len() - MAX_PENDING_EVENTS;
        pending.drain(..excess);
    }
}

/// Sim point to Godot: `(x, height, y)`.
fn to_godot(p: Vec3) -> Vector3 {
    Vector3::new(fixed_to_f32(p.x), fixed_to_f32(p.z), fixed_to_f32(p.y))
}

/// Godot yaw (`rotation.y`) of a model facing -Z turned towards `d`.
fn yaw(d: Vec2) -> f32 {
    (-fixed_to_f32(d.x)).atan2(-fixed_to_f32(d.y))
}

fn face_name(face: Option<Face>) -> &'static str {
    match face {
        None => "",
        Some(Face::Front) => "front",
        Some(Face::Side) => "side",
        Some(Face::Rear) => "rear",
        Some(Face::Top) => "top",
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Orders the units to attack unit `target` (ignored for own targets).
    #[func]
    fn attack_unit(&mut self, unit_ids: PackedInt64Array, target: i64) {
        let units = to_unit_ids(&unit_ids);
        let Ok(target) = u32::try_from(target) else {
            return;
        };
        if !units.is_empty() {
            self.pending.push(Command::Attack {
                units,
                target: UnitId(target),
            });
        }
    }

    /// Orders the units to fire at the ground point `(x, z)` in meters.
    #[func]
    fn attack_ground(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) {
        let (Some(x_mm), Some(z_mm)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            return;
        };
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::AttackGround {
                units,
                target: Vec2::from_millimeters(x_mm, z_mm),
            });
        }
    }

    /// Sets the units to hold fire (`true`) or fire at will.
    #[func]
    fn set_hold_fire(&mut self, unit_ids: PackedInt64Array, hold: bool) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::SetFireMode {
                units,
                mode: if hold {
                    FireMode::HoldFire
                } else {
                    FireMode::FireAtWill
                },
            });
        }
    }

    /// Whether every unit holds fire (same order as `unit_ids`).
    #[func]
    fn unit_hold_fire(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.fire_mode == FireMode::HoldFire))
            .collect()
    }

    /// Whether every unit is alive (1) or destroyed (0).
    #[func]
    fn unit_alive(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.alive()))
            .collect()
    }

    /// Hit points left as a share of the maximum, `0.0..=1.0`.
    #[func]
    fn unit_health(&self) -> PackedFloat32Array {
        self.current
            .units
            .iter()
            .map(|u| {
                if u.max_hp == 0 {
                    0.0
                } else {
                    u.hp as f32 / u.max_hp as f32
                }
            })
            .collect()
    }

    /// Hull direction of every unit as Godot yaw (radians, models face -Z).
    #[func]
    fn unit_facing_yaws(&self) -> PackedFloat32Array {
        self.current.units.iter().map(|u| yaw(u.facing)).collect()
    }

    /// Turret direction of every unit as Godot yaw.
    #[func]
    fn unit_aim_yaws(&self) -> PackedFloat32Array {
        self.current.units.iter().map(|u| yaw(u.aim)).collect()
    }

    /// Units of `player` that are not destroyed.
    #[func]
    fn alive_unit_count(&self, player: i64) -> i64 {
        let Ok(player) = u8::try_from(player) else {
            return 0;
        };
        self.game.as_ref().map_or(0, |g| {
            i64::try_from(g.sim.alive_units(PlayerId(player))).unwrap_or(0)
        })
    }

    /// Projectiles in flight: positions in Godot coordinates.
    #[func]
    fn projectile_positions(&self) -> PackedVector3Array {
        self.current
            .projectiles
            .iter()
            .map(|p| to_godot(p.position))
            .collect()
    }

    /// Velocities of `projectile_positions` in meters per second.
    #[func]
    fn projectile_velocities(&self) -> PackedVector3Array {
        self.current
            .projectiles
            .iter()
            .map(|p| to_godot(p.velocity))
            .collect()
    }

    /// Calibers of `projectile_positions` in millimeters.
    #[func]
    fn projectile_calibers(&self) -> PackedFloat32Array {
        self.current
            .projectiles
            .iter()
            .map(|p| p.caliber_mm_x10 as f32 / 10.0)
            .collect()
    }

    /// The combat events since the last call, oldest first, as
    /// dictionaries with `type` ("shot", "hit", "destroyed", "smoke"), `tick` and:
    ///
    /// - shot: `unit`, `weapon`, `projectile`, `position` (muzzle), `caliber_mm`
    /// - hit: `projectile`, `shooter`, `position`, `target` (unit id or -1),
    ///   `surface` ("unit", "ground", "object"), `face`, `turret`,
    ///   `penetrated`, `damage`, `caliber_mm`
    /// - destroyed: `unit`, `by`, `position`
    /// - smoke: `projectile`, `shooter`, `position`, `radius` (a smoke shell
    ///   burst, prompt 032)
    ///
    /// Shells of indirect fire (prompt 032) fire `shot` events with
    /// `weapon` 255 and land with `hit` events of the same `projectile` id.
    #[func]
    fn take_combat_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.combat_events);
        let mut out = Array::new();
        for (tick, event) in events {
            let mut d = VarDictionary::new();
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            match event {
                CombatEvent::Shot {
                    unit,
                    weapon,
                    projectile,
                    muzzle,
                    caliber_mm_x10,
                } => {
                    d.set("type", "shot");
                    d.set("unit", i64::from(unit.0));
                    d.set("weapon", i64::from(weapon));
                    d.set("projectile", i64::from(projectile));
                    d.set("position", to_godot(muzzle));
                    d.set("caliber_mm", caliber_mm_x10 as f32 / 10.0);
                }
                CombatEvent::Hit {
                    projectile,
                    shooter,
                    at,
                    kind,
                    face,
                    turret,
                    penetrated,
                    damage,
                    caliber_mm_x10,
                } => {
                    d.set("type", "hit");
                    d.set("projectile", i64::from(projectile));
                    d.set("shooter", i64::from(shooter.0));
                    d.set("position", to_godot(at));
                    let (surface, target) = match kind {
                        HitKind::Unit(id) => ("unit", i64::from(id.0)),
                        HitKind::Ground => ("ground", -1),
                        HitKind::Object(_) => ("object", -1),
                    };
                    d.set("surface", surface);
                    d.set("target", target);
                    d.set("face", face_name(face));
                    d.set("turret", turret);
                    d.set("penetrated", penetrated);
                    d.set("damage", i64::from(damage));
                    d.set("caliber_mm", caliber_mm_x10 as f32 / 10.0);
                }
                CombatEvent::Smoke {
                    projectile,
                    shooter,
                    at,
                    radius,
                } => {
                    d.set("type", "smoke");
                    d.set("projectile", i64::from(projectile));
                    d.set("shooter", i64::from(shooter.0));
                    d.set("position", to_godot(at));
                    d.set("radius", fixed_to_f32(radius));
                }
                CombatEvent::Destroyed { unit, by, at } => {
                    d.set("type", "destroyed");
                    d.set("unit", i64::from(unit.0));
                    d.set("by", i64::from(by.0));
                    d.set("position", to_godot(at));
                }
            }
            out.push(&d);
        }
        out
    }
}
