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

    /// Orders the units to shell map object `object` until it is destroyed
    /// (prompt 093; units without a gun that damages objects ignore it).
    #[func]
    fn attack_object(&mut self, unit_ids: PackedInt64Array, object: i64) {
        let units = to_unit_ids(&unit_ids);
        let Ok(object) = u32::try_from(object) else {
            return;
        };
        if !units.is_empty() {
            self.pending.push(Command::AttackObject { units, object });
        }
    }

    /// The map object that can be shelled (it has hit points and is not
    /// destroyed) at the ground point `(x, z)` in meters, with `margin`
    /// meters to spare around its ground shape; -1 if there is none.
    #[func]
    fn shell_target_at(&self, x: f32, z: f32, margin: f32) -> i64 {
        let Some(game) = &self.game else {
            return -1;
        };
        let (Some(x_mm), Some(z_mm), Some(m_mm)) = (
            meters_to_millimeters(x),
            meters_to_millimeters(z),
            meters_to_millimeters(margin.clamp(0.0, 50.0)),
        ) else {
            return -1;
        };
        game.sim
            .shellable_object_at(
                Vec2::from_millimeters(x_mm, z_mm),
                sim::Fixed::from_num(m_mm) / 1000,
            )
            .map_or(-1, i64::from)
    }

    /// Whether some of the units can shell map objects (a direct weapon of
    /// at least the caliber that damages objects).
    #[func]
    fn can_shell_objects(&self, unit_ids: PackedInt64Array) -> bool {
        let Some(game) = &self.game else {
            return false;
        };
        to_unit_ids(&unit_ids)
            .iter()
            .any(|&id| game.sim.can_shell_objects(id))
    }

    /// The box around map object `object` as it stands now: eight corners
    /// in Godot coordinates, four on the ground (clockwise) and the four
    /// above them at the object's current height; empty for an unknown
    /// index. For highlighting the target of a shelling order.
    #[func]
    fn object_outline(&self, object: i64) -> PackedVector3Array {
        let Some(game) = &self.game else {
            return PackedVector3Array::new();
        };
        let map = game.map();
        let Some((index, o)) = u32::try_from(object)
            .ok()
            .and_then(|i| Some((i, map.objects().get(i as usize)?)))
        else {
            return PackedVector3Array::new();
        };
        let kind = &map.object_kinds()[usize::from(o.kind)];
        let (half_length, half_width) = match kind.shape {
            sim::objects::Shape::Circle { radius } => (radius, radius),
            sim::objects::Shape::Box {
                half_length,
                half_width,
            } => (half_length, half_width),
        };
        let (forward, right) = o.axes();
        let ground = fixed_to_f32(map.height_at(o.position));
        // A ruin keeps a part of its height; what is gone has none.
        let ruin = game.sim.object_stage(index) == Some(sim::ObjectStage::Destroyed);
        let height = fixed_to_f32(kind.height) * if ruin { 0.4 } else { 1.0 };
        let mut corners = Vec::with_capacity(8);
        for up in [0.0, height.max(0.5)] {
            for (a, b) in [(1, 1), (1, -1), (-1, -1), (-1, 1)] {
                let p = o.position + forward.scale(half_length * a) + right.scale(half_width * b);
                corners.push(Vector3::new(
                    fixed_to_f32(p.x),
                    ground + up,
                    fixed_to_f32(p.y),
                ));
            }
        }
        PackedVector3Array::from(corners.as_slice())
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

    /// Sets the fire mode of the units (prompt 176): 0 fire at will, 1
    /// hold fire, 2 return fire.
    #[func]
    fn set_fire_mode(&mut self, unit_ids: PackedInt64Array, code: i64) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::SetFireMode {
                units,
                mode: FireMode::from_code(code),
            });
        }
    }

    /// Fire mode of every unit (same order as `unit_ids`): 0 fire at will,
    /// 1 hold fire, 2 return fire (prompt 176).
    #[func]
    fn unit_fire_modes(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u.fire_mode.code())
            .collect()
    }

    /// Sets the limits of automatic fire of the units (prompt 176).
    #[func]
    fn set_fire_limits(
        &mut self,
        unit_ids: PackedInt64Array,
        close_only: bool,
        vehicles_only: bool,
    ) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::SetFireLimits {
                units,
                close_only,
                vehicles_only,
            });
        }
    }

    /// Fire limits of every unit (same order as `unit_ids`): bit 0 close
    /// range only, bit 1 vehicles only (prompt 176).
    #[func]
    fn unit_fire_limits(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| {
                u8::from(u.fire_limits.close_only) | (u8::from(u.fire_limits.vehicles_only) << 1)
            })
            .collect()
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

    /// Ids of `projectile_positions` (the `projectile` of their `shot`
    /// events; prompt 077: tracers start at the model's muzzle).
    #[func]
    fn projectile_ids(&self) -> PackedInt64Array {
        self.current
            .projectiles
            .iter()
            .map(|p| i64::from(p.id))
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
