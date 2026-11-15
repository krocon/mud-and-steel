//! `SimHost` functions for water and bridges (prompt 014). The work
//! happens in `water.rs`; this block converts to Godot types.

use crate::water::WaterField;
use crate::{SimHost, fixed_to_f32, meters_to_millimeters};
use godot::prelude::*;
use sim::Vec2;

impl SimHost {
    fn with_water<R>(&self, f: impl FnOnce(&WaterField) -> R) -> Option<R> {
        let game = self.game.as_ref()?;
        Some(f(game.water.get_or_init(|| WaterField::new(&game.map))))
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// The water surface as mesh buffers: `{vertices: PackedVector3Array,
    /// uv: PackedVector2Array (flow x, z in m/s), uv2: PackedVector2Array
    /// (shore distance m, depth m), indices: PackedInt32Array}`. Empty
    /// arrays for maps without water.
    #[func]
    fn water_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(mesh) = self.with_water(WaterField::mesh) else {
            return d;
        };
        let v: PackedVector3Array = mesh
            .vertices
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let uv: PackedVector2Array = mesh.uv.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        let uv2: PackedVector2Array = mesh.uv2.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        let indices: PackedInt32Array = mesh.indices.iter().map(|&i| i as i32).collect();
        d.set("vertices", &v);
        d.set("uv", &uv);
        d.set("uv2", &uv2);
        d.set("indices", &indices);
        d
    }

    /// Water level per map sample (row-major, meters), for the wet band of
    /// the terrain shader: the water surface on water and land near it,
    /// -10000 elsewhere.
    #[func]
    fn water_levels(&self) -> PackedFloat32Array {
        self.with_water(|w| {
            w.level
                .iter()
                .map(|&l| if l.is_nan() { -10_000.0 } else { l })
                .collect()
        })
        .unwrap_or_default()
    }

    /// Water level (m) at the sample nearest to Godot `(x, z)`, or NaN.
    #[func]
    fn water_level_at(&self, x: f32, z: f32) -> f32 {
        self.with_water(|w| w.sample(x, z).map_or(f32::NAN, |k| w.level[k]))
            .unwrap_or(f32::NAN)
    }

    /// Whether the sample nearest to Godot `(x, z)` is water (ground terrain).
    #[func]
    fn is_water_at(&self, x: f32, z: f32) -> bool {
        self.with_water(|w| w.sample(x, z).is_some_and(|k| w.is_water[k]))
            .unwrap_or(false)
    }

    /// The bridges of the map: `{object: int (index of object_position),
    /// kind: String, variant: String, center: Vector3 (y = deck height in
    /// the middle), heading_deg: int, half_length: float, half_width: float
    /// (roadway), height_start: float, height_end: float, max_load_kg: int,
    /// intact: bool}`.
    #[func]
    fn bridges(&self) -> Array<VarDictionary> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let map = game.sim.map();
        map.bridges()
            .iter()
            .map(|b| {
                let o = &map.objects()[b.object as usize];
                let kind = &map.object_kinds()[usize::from(o.kind)];
                let mut d = VarDictionary::new();
                d.set("object", i64::from(b.object));
                d.set("kind", &GString::from(kind.id.as_str()));
                d.set("variant", &GString::from(kind.variant.as_str()));
                d.set(
                    "center",
                    Vector3::new(
                        fixed_to_f32(b.center.x),
                        fixed_to_f32(b.height_at(b.center)),
                        fixed_to_f32(b.center.y),
                    ),
                );
                d.set("heading_deg", i64::from(o.heading_deg));
                d.set("half_length", fixed_to_f32(b.half_length));
                d.set("half_width", fixed_to_f32(b.half_width));
                d.set("height_start", fixed_to_f32(b.height_start));
                d.set("height_end", fixed_to_f32(b.height_end));
                d.set("max_load_kg", i64::from(b.max_load_kg));
                d.set("intact", b.intact);
                d
            })
            .collect()
    }

    /// Ground height (m) at Godot `(x, z)` without bridge decks
    /// (`height_at` reports the deck where there is one).
    #[func]
    fn ground_height_at(&self, x: f32, z: f32) -> f32 {
        let (Some(game), Some(x), Some(z)) = (
            self.game.as_ref(),
            meters_to_millimeters(x),
            meters_to_millimeters(z),
        ) else {
            return 0.0;
        };
        fixed_to_f32(
            game.sim
                .map()
                .ground_height_at(Vec2::from_millimeters(x, z)),
        )
    }

    /// Water flow direction of the map (compass degrees, `map.toml`
    /// `water_flow_deg`).
    #[func]
    fn water_flow_deg(&self) -> i64 {
        self.game
            .as_ref()
            .map_or(180, |g| i64::from(g.map.water_flow_deg()))
    }
}
