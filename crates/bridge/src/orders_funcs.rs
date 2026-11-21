//! `SimHost` functions for formations, attack-move and securing an area
//! (prompt 046). Godot picks the formation, the facing (right-drag) and the
//! mode (`scenes/selection/formation_orders.gd`); the simulation lays the
//! formation out and decides everything else.

use crate::{SimHost, fixed_to_f32, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::{Command, DrillView, Engage, Formation, UnitId, Vec2};

fn point(x: f32, z: f32) -> Option<Vec2> {
    Some(Vec2::from_millimeters(
        meters_to_millimeters(x)?,
        meters_to_millimeters(z)?,
    ))
}

/// A facing from a Godot ground direction; `None` for a zero vector.
fn facing(dx: f32, dz: f32) -> Option<Vec2> {
    let length = (dx * dx + dz * dz).sqrt();
    if !length.is_finite() || length < 0.001 {
        return None;
    }
    // Quantized to a unit vector in millimeters; the simulation normalizes.
    point(dx / length, dz / length)
}

#[godot_api(secondary)]
impl SimHost {
    /// Sends the units in `formation` (code: index of `formation_names`)
    /// to `(x, z)`, facing the ground direction `(dx, dz)` (zero: the
    /// direction of travel). `engage`: 0 plain move, 1 infantry halts to
    /// fight (right-click default), 2 everybody (attack-move).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn formation_move(
        &mut self,
        unit_ids: PackedInt64Array,
        x: f32,
        z: f32,
        formation: i64,
        dx: f32,
        dz: f32,
        engage: i64,
    ) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(target)) = (units.is_empty(), point(x, z)) {
            self.pending.push(Command::FormationMove {
                units,
                target,
                formation: Formation::from_code(formation),
                facing: facing(dx, dz),
                engage: Engage::from_code(engage),
            });
        }
    }

    /// The units take up `formation` around `(x, z)` and hold the circle
    /// of `radius` meters, facing `(dx, dz)` (zero: the direction of
    /// travel).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn secure_area(
        &mut self,
        unit_ids: PackedInt64Array,
        x: f32,
        z: f32,
        radius: i64,
        formation: i64,
        dx: f32,
        dz: f32,
    ) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(center)) = (units.is_empty(), point(x, z)) {
            self.pending.push(Command::Secure {
                units,
                center,
                radius_m: u16::try_from(radius.clamp(0, i64::from(u16::MAX))).unwrap_or(0),
                formation: Formation::from_code(formation),
                facing: facing(dx, dz),
            });
        }
    }

    /// Names of the formations by code (`auto`, `line`, ...).
    #[func]
    fn formation_names(&self) -> PackedStringArray {
        Formation::ALL
            .iter()
            .map(|f| GString::from(f.name()))
            .collect()
    }

    /// The formation `Auto` would pick for the units moving to `(x, z)`
    /// (code), or -1.
    #[func]
    fn auto_formation(&self, unit_ids: PackedInt64Array, x: f32, z: f32) -> i64 {
        let (Some(game), Some(target)) = (&self.game, point(x, z)) else {
            return -1;
        };
        let player = game.session.local_player();
        game.sim
            .auto_formation_for(player, &to_unit_ids(&unit_ids), target, None)
            .map_or(-1, |f| i64::from(f.code()))
    }

    /// Target preview of a formation move (prompt 099): the slots the
    /// order `formation_move(unit_ids, x, z, formation, dx, dz, ..)` would
    /// give the local player's mobile units, computed by the same
    /// simulation code. Six floats per unit: id, slot x, slot z, blocked
    /// (1: the slot is outside the map or impassable for the unit; the
    /// order then stops it at the nearest reachable spot), and where the
    /// unit really ends up, x and z (prompt 175: the slot, or the nearest
    /// point it can reach when the slot lies across a river or in a lake).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn formation_preview(
        &self,
        unit_ids: PackedInt64Array,
        x: f32,
        z: f32,
        formation: i64,
        dx: f32,
        dz: f32,
    ) -> PackedFloat32Array {
        let (Some(game), Some(target)) = (&self.game, point(x, z)) else {
            return PackedFloat32Array::new();
        };
        let player = game.session.local_player();
        let slots = game.sim.formation_preview(
            player,
            &to_unit_ids(&unit_ids),
            target,
            Formation::from_code(formation),
            facing(dx, dz),
        );
        let mut out = Vec::with_capacity(slots.len() * 6);
        for (id, slot, blocked) in slots {
            // Where the unit really ends up (prompt 175): the slot, or the
            // nearest point it can reach.
            let reach = game.sim.reachable_point(id, slot).unwrap_or(slot);
            out.extend([
                id.0 as f32,
                fixed_to_f32(slot.x),
                fixed_to_f32(slot.y),
                if blocked { 1.0 } else { 0.0 },
                fixed_to_f32(reach.x),
                fixed_to_f32(reach.y),
            ]);
        }
        PackedFloat32Array::from(out)
    }

    /// Sight range of unit `id` in meters on level ground, as the fog of
    /// war uses it (weather applied); 0 for unknown units (prompt 099).
    #[func]
    fn unit_sight_range(&self, id: i64) -> f32 {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return 0.0;
        };
        game.sim.sight_range(UnitId(id)).map_or(0.0, fixed_to_f32)
    }

    /// Task of unit `id`: `(kind, x, z, radius)`, kind 0 none, 1
    /// attack-move (to x, z), 2 halted to fight, 3 securing the circle.
    #[func]
    fn unit_drill(&self, id: i64) -> Vector4 {
        let none = Vector4::ZERO;
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return none;
        };
        match game.sim.drill_view(UnitId(id)) {
            None => none,
            Some(DrillView::Advance(p, halted)) => Vector4::new(
                if halted { 2.0 } else { 1.0 },
                fixed_to_f32(p.x),
                fixed_to_f32(p.y),
                0.0,
            ),
            Some(DrillView::Secure(c, r)) => {
                Vector4::new(3.0, fixed_to_f32(c.x), fixed_to_f32(c.y), fixed_to_f32(r))
            }
        }
    }
}
