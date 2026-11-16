//! `SimHost` functions for fire (prompt 041): what burns now, which objects
//! burned out (charred), and which smoke clouds are fire smoke. Godot draws
//! flames, embers, smoke and light from this; the simulation decides what
//! burns.

use crate::{SimHost, fixed_to_f32};
use godot::prelude::*;
use sim::FireSource;

#[godot_api(secondary)]
impl SimHost {
    /// Fires burning now: `Vector4(x, ground height, z, flame radius)` in
    /// meters, objects first, then wrecks.
    #[func]
    fn fires(&self) -> PackedVector4Array {
        let Some(game) = self.game.as_ref() else {
            return PackedVector4Array::new();
        };
        let map = game.sim.map();
        game.sim
            .fires()
            .iter()
            .map(|f| {
                Vector4::new(
                    fixed_to_f32(f.at.x),
                    fixed_to_f32(map.height_at(f.at)),
                    fixed_to_f32(f.at.y),
                    fixed_to_f32(f.size_m),
                )
            })
            .collect()
    }

    /// Keys of `fires()`: the object index (>= 0), or `-1 - unit id` for
    /// the wreck of a vehicle.
    #[func]
    fn fire_keys(&self) -> PackedInt64Array {
        let Some(game) = self.game.as_ref() else {
            return PackedInt64Array::new();
        };
        game.sim
            .fires()
            .iter()
            .map(|f| match f.source {
                FireSource::Object(o) => i64::from(o),
                FireSource::Wreck(u) => -1 - i64::from(u.0),
            })
            .collect()
    }

    /// Share of each fire's burn time left, `0.0..=1.0` (flames die down
    /// towards the end).
    #[func]
    fn fire_life(&self) -> PackedFloat32Array {
        let Some(game) = self.game.as_ref() else {
            return PackedFloat32Array::new();
        };
        game.sim
            .fires()
            .iter()
            .map(|f| {
                let total = f.age_ticks.saturating_add(f.ticks_left).max(1);
                f.ticks_left as f32 / total as f32
            })
            .collect()
    }

    /// Map objects whose fire burned out (drawn charred), ascending.
    #[func]
    fn burnt_objects(&self) -> PackedInt64Array {
        let Some(game) = self.game.as_ref() else {
            return PackedInt64Array::new();
        };
        game.sim.burnt_objects().map(i64::from).collect()
    }

    /// Per cloud of `smoke_clouds()`: 1 for the dark smoke of a fire, 0
    /// for a smoke shell.
    #[func]
    fn smoke_dark(&self) -> PackedByteArray {
        self.current
            .smoke
            .iter()
            .map(|c| u8::from(c.dark))
            .collect()
    }
}
