//! `SimHost` functions for indirect fire and smoke (prompt 032): barrage
//! orders, which units can fire indirectly, shells in flight with their
//! impact points, and smoke clouds. Godot only draws what comes out of here.

use crate::helpers::{ground_point, vec3_to_godot as to_godot};
use crate::{SimHost, fixed_to_f32, to_unit_ids};
use godot::prelude::*;
use sim::{Command, UnitId};

#[godot_api(secondary)]
impl SimHost {
    /// Orders the units with an indirect weapon to fire `salvos` rounds
    /// each at random points within `radius` meters of `(x, z)`; smoke
    /// shells if `smoke` (units without smoke ammunition ignore it).
    #[func]
    fn barrage(
        &mut self,
        unit_ids: PackedInt64Array,
        x: f32,
        z: f32,
        radius: i64,
        salvos: i64,
        smoke: bool,
    ) {
        let Some(target) = ground_point(x, z) else {
            return;
        };
        let units = to_unit_ids(&unit_ids);
        if units.is_empty() {
            return;
        }
        self.pending.push(Command::Barrage {
            units,
            target,
            radius_m: u16::try_from(radius.max(0)).unwrap_or(u16::MAX),
            salvos: u16::try_from(salvos.max(0)).unwrap_or(u16::MAX),
            smoke,
        });
    }

    /// Indirect weapon of a unit: `Vector3(min range, max range, smoke)`
    /// in meters (smoke 1 or 0); `Vector3.ZERO` if it has none.
    #[func]
    fn unit_indirect(&self, unit_id: i64) -> Vector3 {
        let Ok(id) = u32::try_from(unit_id) else {
            return Vector3::ZERO;
        };
        self.game
            .as_ref()
            .and_then(|g| g.sim.indirect_weapon(UnitId(id)))
            .map_or(Vector3::ZERO, |(min, max, smoke)| {
                Vector3::new(
                    fixed_to_f32(min),
                    fixed_to_f32(max),
                    if smoke { 1.0 } else { 0.0 },
                )
            })
    }

    /// Shells in flight: positions in Godot coordinates.
    #[func]
    fn shell_positions(&self) -> PackedVector3Array {
        self.current
            .shells
            .iter()
            .map(|s| to_godot(s.position))
            .collect()
    }

    /// Velocities of `shell_positions` in meters per second.
    #[func]
    fn shell_velocities(&self) -> PackedVector3Array {
        self.current
            .shells
            .iter()
            .map(|s| to_godot(s.velocity))
            .collect()
    }

    /// Impact points of `shell_positions`.
    #[func]
    fn shell_targets(&self) -> PackedVector3Array {
        self.current
            .shells
            .iter()
            .map(|s| to_godot(s.target))
            .collect()
    }

    /// Owners of `shell_positions`.
    #[func]
    fn shell_owners(&self) -> PackedInt32Array {
        self.current
            .shells
            .iter()
            .map(|s| i32::from(s.owner.0))
            .collect()
    }

    /// Ids of `shell_positions` (the `projectile` of their events).
    #[func]
    fn shell_ids(&self) -> PackedInt64Array {
        self.current
            .shells
            .iter()
            .map(|s| i64::from(s.id))
            .collect()
    }

    /// Seconds until each shell of `shell_positions` lands.
    #[func]
    fn shell_seconds_left(&self) -> PackedFloat32Array {
        let hz = self.tick_hz.max(1) as f32;
        self.current
            .shells
            .iter()
            .map(|s| s.ticks_left as f32 / hz)
            .collect()
    }

    /// Calibers of `shell_positions` in millimeters; negative for smoke.
    #[func]
    fn shell_calibers(&self) -> PackedFloat32Array {
        self.current
            .shells
            .iter()
            .map(|s| {
                let mm = s.caliber_mm_x10 as f32 / 10.0;
                if s.smoke { -mm } else { mm }
            })
            .collect()
    }

    /// Fragmentation radius in meters of a shell of `caliber_mm` (`[blast]`
    /// in `data/indirect.toml`); 0 without a match.
    #[func]
    fn shell_blast_radius(&self, caliber_mm: f32) -> f32 {
        let x10 = (caliber_mm.abs() * 10.0).round();
        if !x10.is_finite() || x10 > u32::MAX as f32 {
            return 0.0;
        }
        self.game
            .as_ref()
            .map_or(0.0, |g| fixed_to_f32(g.sim.shell_blast_radius(x10 as u32)))
    }

    /// Smoke clouds: `Vector4(x, ground height, z, radius)` per cloud.
    #[func]
    fn smoke_clouds(&self) -> PackedVector4Array {
        let Some(game) = self.game.as_ref() else {
            return PackedVector4Array::new();
        };
        let map = game.sim.map();
        self.current
            .smoke
            .iter()
            .map(|c| {
                Vector4::new(
                    fixed_to_f32(c.center.x),
                    fixed_to_f32(map.height_at(c.center)),
                    fixed_to_f32(c.center.y),
                    fixed_to_f32(c.radius),
                )
            })
            .collect()
    }

    /// Ids of `smoke_clouds`.
    #[func]
    fn smoke_ids(&self) -> PackedInt64Array {
        self.current.smoke.iter().map(|c| i64::from(c.id)).collect()
    }

    /// Share of each cloud's life left, `0.0..=1.0`, for fading.
    #[func]
    fn smoke_life(&self) -> PackedFloat32Array {
        self.current
            .smoke
            .iter()
            .map(|c| {
                let total = c.age_ticks.saturating_add(c.ticks_left).max(1);
                c.ticks_left as f32 / total as f32
            })
            .collect()
    }
}
