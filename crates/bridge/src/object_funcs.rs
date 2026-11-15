//! `SimHost` functions for map objects and decoration (prompt 013).
//! The work happens in `objects.rs`; this block converts to Godot types.

use crate::objects::{self, DecoLayer};
use crate::{SimHost, fixed_to_f32};
use godot::prelude::*;

#[godot_api(secondary)]
impl SimHost {
    /// Every object kind of the map as `"<catalog id>:<variant>"`; the index
    /// is the `kind` of `object_groups`.
    #[func]
    fn object_kinds(&self) -> PackedStringArray {
        let Some(game) = self.game.as_ref() else {
            return PackedStringArray::new();
        };
        game.map
            .object_kinds()
            .iter()
            .map(|k| GString::from(format!("{}:{}", k.id, k.variant).as_str()))
            .collect()
    }

    /// Number of placed objects.
    #[func]
    fn object_count(&self) -> i64 {
        self.game
            .as_ref()
            .map_or(0, |g| g.map.objects().len() as i64)
    }

    /// The objects as `MultiMesh` transform buffers (12 floats per
    /// instance), one entry per kind and chunk of `chunk_m` meters:
    /// `{kind: int, chunk: Vector2i, buffer: PackedFloat32Array}`. Objects
    /// stand on the ground; trees, bushes and rocks get a small size jitter
    /// (presentation only).
    #[func]
    fn object_groups(&self, chunk_m: f32) -> Array<VarDictionary> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        objects::groups(&game.map, chunk_m.max(1.0))
            .into_iter()
            .map(|g| {
                let mut d = VarDictionary::new();
                d.set("kind", i64::from(g.kind));
                d.set("chunk", Vector2i::new(g.chunk.0, g.chunk.1));
                d.set("buffer", &PackedFloat32Array::from(g.buffer.as_slice()));
                d
            })
            .collect()
    }

    /// Position of object `index` in Godot coordinates (ground height as
    /// the renderer uses it is not included: y = simulation ground height).
    #[func]
    fn object_position(&self, index: i64) -> Vector3 {
        let Some(game) = self.game.as_ref() else {
            return Vector3::ZERO;
        };
        let Some(o) = usize::try_from(index)
            .ok()
            .and_then(|i| game.map.objects().get(i))
        else {
            return Vector3::ZERO;
        };
        Vector3::new(
            fixed_to_f32(o.position.x),
            fixed_to_f32(game.map.height_at(o.position)),
            fixed_to_f32(o.position.y),
        )
    }

    /// Decoration of one layer in the square `x0..x0 + size`, `z0..z0 +
    /// size` (Godot x and z): one `MultiMesh` buffer per variant (weights in
    /// `weights`). `density` gives the probability per lattice point for
    /// every terrain id, `wood_factor` scales it in dense wood,
    /// `patchiness` (0..1) groups it into patches. No decoration on samples
    /// covered by buildings, walls, hedges or props. `shore` (meters,
    /// signed: negative in the water, positive on land) limits the layer
    /// to a band along the water's edge (reeds); `Vector2(0, 0)` = no limit.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn scatter_deco(
        &self,
        x0: f32,
        z0: f32,
        size: f32,
        spacing: f32,
        seed: i64,
        density: PackedFloat32Array,
        wood_factor: f32,
        patchiness: f32,
        weights: PackedFloat32Array,
        scale: Vector2,
        sink: f32,
        #[opt(default = Vector2::ZERO)] shore: Vector2,
    ) -> Array<PackedFloat32Array> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let rasters = game
            .deco
            .get_or_init(|| objects::DecoRasters::new(&game.map));
        let layer = DecoLayer {
            spacing,
            seed: seed.cast_unsigned(),
            density: density.as_slice(),
            wood_factor,
            patchiness,
            weights: weights.as_slice(),
            scale: (scale.x, scale.y),
            sink,
            shore: (shore.x, shore.y),
        };
        let water = (shore != Vector2::ZERO).then(|| {
            game.water
                .get_or_init(|| crate::water::WaterField::new(&game.map))
        });
        objects::scatter(&game.map, rasters, water, &layer, x0, z0, size)
            .into_iter()
            .map(|b| PackedFloat32Array::from(b.as_slice()))
            .collect()
    }
}
