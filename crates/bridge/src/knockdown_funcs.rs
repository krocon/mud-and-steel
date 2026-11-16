//! `SimHost` functions for prompt 048 (soldier animations and ragdolls):
//! knocked-down soldiers and the cover object of every soldier. The
//! simulation decides who is knocked down, where he lies and when he gets
//! up; `scenes/world/infantry_view.gd` and `ragdolls.gd` only show it.

use crate::{SimHost, fixed_to_f32, ground_to_godot};
use godot::prelude::*;
use sim::objects::Shape;

#[godot_api(secondary)]
impl SimHost {
    /// Ticks every soldier stays knocked down (0 = not), in the order of
    /// `soldier_units`.
    #[func]
    fn soldier_knocked(&self) -> PackedInt32Array {
        self.current
            .soldiers
            .iter()
            .map(|s| i32::try_from(s.knocked).unwrap_or(i32::MAX))
            .collect()
    }

    /// Where the blast that knocked every soldier down (last) went off, on
    /// the ground (Godot coordinates; the origin if never).
    #[func]
    fn soldier_knock_from(&self) -> PackedVector3Array {
        let Some(game) = self.game.as_ref() else {
            return PackedVector3Array::new();
        };
        self.current
            .soldiers
            .iter()
            .map(|s| ground_to_godot(s.knock_from, game.map().height_at(s.knock_from)))
            .collect()
    }

    /// Map object whose cover spot every soldier stands at (-1: none, also
    /// in craters and buildings).
    #[func]
    fn soldier_cover_objects(&self) -> PackedInt32Array {
        self.current
            .soldiers
            .iter()
            .map(|s| s.cover.map_or(-1, |o| i32::try_from(o).unwrap_or(-1)))
            .collect()
    }

    /// A map object as cover: `{id, height, position, forward, half_length,
    /// half_width}` (Godot coordinates; round objects have both halves equal
    /// to the radius). Empty for an unknown index.
    #[func]
    fn cover_object_info(&self, index: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(game) = self.game.as_ref() else {
            return d;
        };
        let map = game.map();
        let Some(o) = usize::try_from(index)
            .ok()
            .and_then(|i| map.objects().get(i))
        else {
            return d;
        };
        let kind = &map.object_kinds()[usize::from(o.kind)];
        let (half_length, half_width) = match kind.shape {
            Shape::Circle { radius } => (radius, radius),
            Shape::Box {
                half_length,
                half_width,
            } => (half_length, half_width),
        };
        let (forward, _) = o.axes();
        d.set("id", &GString::from(kind.id.as_str()));
        d.set("height", fixed_to_f32(kind.height));
        d.set(
            "position",
            ground_to_godot(o.position, map.height_at(o.position)),
        );
        d.set(
            "forward",
            Vector3::new(fixed_to_f32(forward.x), 0.0, fixed_to_f32(forward.y)),
        );
        d.set("half_length", fixed_to_f32(half_length));
        d.set("half_width", fixed_to_f32(half_width));
        d
    }
}
