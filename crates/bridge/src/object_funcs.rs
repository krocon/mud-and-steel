//! `SimHost` functions for map objects and decoration (prompt 013).
//! The work happens in `objects.rs`; this block converts to Godot types.

use crate::objects::{self, DecoLayer};
use crate::{SimHost, fixed_to_f32, meters_to_millimeters};
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
        game.map()
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
            .map_or(0, |g| g.map().objects().len() as i64)
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
        objects::groups(game.map(), chunk_m.max(1.0))
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

    /// Highest top (Godot y: ground height at the object plus its catalog
    /// height) of the standing objects that block everybody (buildings,
    /// rock outcrops and ridges) whose ground box, grown by `margin` meters,
    /// contains the point `(x, z)`; `-1e9` if there is none. The camera
    /// keeps above it (prompt 078: apartment blocks are higher than the
    /// closest camera). Presentation only.
    #[func]
    fn object_top_at(&self, x: f32, z: f32, margin: f32) -> f32 {
        const NONE: f32 = -1e9;
        let Some(game) = self.game.as_ref() else {
            return NONE;
        };
        let (Some(x_mm), Some(z_mm), Some(m_mm)) = (
            meters_to_millimeters(x),
            meters_to_millimeters(z),
            meters_to_millimeters(margin.max(0.0)),
        ) else {
            return NONE;
        };
        let p = sim::Vec2::from_millimeters(x_mm, z_mm);
        let m = sim::Fixed::from_num(m_mm) / 1000;
        let map = game.map();
        let kinds = map.object_kinds();
        let mut top = NONE;
        for (i, o) in (0u32..).zip(map.objects()) {
            let kind = &kinds[usize::from(o.kind)];
            if kind.blocks != sim::Blocks::All {
                continue;
            }
            let sim::objects::Shape::Box {
                half_length,
                half_width,
            } = kind.shape
            else {
                continue;
            };
            let rel = p - o.position;
            if rel.x.abs() > half_length + half_width + m
                || rel.y.abs() > half_length + half_width + m
            {
                continue;
            }
            let (forward, right) = o.axes();
            let along = rel.x * forward.x + rel.y * forward.y;
            let across = rel.x * right.x + rel.y * right.y;
            if along.abs() > half_length + m || across.abs() > half_width + m {
                continue;
            }
            if game.sim.object_blocks(i) != Some(sim::Blocks::All) {
                continue;
            }
            let h = fixed_to_f32(map.height_at(o.position) + kind.height);
            top = top.max(h);
        }
        top
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
            .and_then(|i| game.map().objects().get(i))
        else {
            return Vector3::ZERO;
        };
        Vector3::new(
            fixed_to_f32(o.position.x),
            fixed_to_f32(game.map().height_at(o.position)),
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
    /// `fray` (meters, prompt 073) frays the edges of a stand: the density
    /// is interpolated at a point moved by noise and instances get shorter
    /// towards the edge; 0 = the nearest sample's density.
    /// `rock_edge_macro`: x (`rock`, prompt 074) adds density in a 3 m seam
    /// around rock slabs, outcrops and ridge pieces (on grass, sand, mud,
    /// snow and tall grass); y (`edge`, prompt 191) multiplies the density
    /// by `1 + edge` in a 4 m band along objects, tracks, roads and fields
    /// and at the edge of woods; z (`macro`) groups it over about 90 m
    /// (one parameter, because godot-rust takes at most 14).
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
        #[opt(default = 0.0)] fray: f32,
        #[opt(default = Vector3::ZERO)] rock_edge_macro: Vector3,
    ) -> Array<PackedFloat32Array> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let rasters = game
            .deco
            .get_or_init(|| objects::DecoRasters::new(game.map()));
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
            fray,
            rock: rock_edge_macro.x,
            edge: rock_edge_macro.y,
            macro_noise: rock_edge_macro.z,
        };
        let water = (shore != Vector2::ZERO).then(|| {
            game.water
                .get_or_init(|| crate::water::WaterField::new(game.map()))
        });
        // Nothing grows in craters (prompt 021).
        let sim = &game.sim;
        let sx = game.map().samples().0 as usize;
        let deformed = |k: usize| {
            sim.has_craters() && sim.deformation_mm((k % sx) as u32, (k / sx) as u32) != 0
        };
        objects::scatter(game.map(), rasters, water, &layer, x0, z0, size, deformed)
            .into_iter()
            .map(|b| PackedFloat32Array::from(b.as_slice()))
            .collect()
    }

    /// Wood density 0..1 at `(x, z)` (Godot x and z): trunks per 8 m cell,
    /// smoothed over the neighbors, as used for the decoration. Read by the
    /// ambience (prompt 030) for rustling leaves and birds; presentation only.
    #[func]
    fn wood_density_at(&self, x: f32, z: f32) -> f32 {
        let Some(game) = self.game.as_ref() else {
            return 0.0;
        };
        game.deco
            .get_or_init(|| objects::DecoRasters::new(game.map()))
            .wood_at(x, z)
    }
}
