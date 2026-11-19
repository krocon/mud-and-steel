//! `SimHost` functions for sound (prompt 029): the catalog facts Godot
//! needs to pick a weapon and engine sound per unit type. Read only; sound
//! is presentation and never reaches the simulation.

use crate::{SimHost, fixed_to_f32};
use godot::prelude::*;

#[godot_api(secondary)]
impl SimHost {
    /// Sound facts of a unit type index (`unit_types()`): `id` (catalog id),
    /// `category` (catalog category, e.g. `"tank_medium"`, `""` for
    /// hand-made types), `mass_kg` (0 if unknown), `road_speed` (game m/s)
    /// and `weapons`, in the order of the `weapon` index of shot events
    /// (the secondary machine guns of vehicles last, prompt 093):
    /// `{name, caliber_mm, rate_rpm, mount}`; `mount` is "coax", "bow",
    /// "roof" or "mixed" for a machine gun that picks its own targets, ""
    /// otherwise. Empty without a match or for an unknown index.
    #[func]
    fn unit_type_sound(&self, unit_type: i64) -> VarDictionary {
        let Some(game) = &self.game else {
            return VarDictionary::new();
        };
        let Some(t) = usize::try_from(unit_type)
            .ok()
            .and_then(|i| game.config.unit_types.get(i))
        else {
            return VarDictionary::new();
        };
        let mut weapons = VarArray::new();
        let rules = &game.config.combat;
        for (index, w) in t.combat.armed_with(rules).iter().enumerate() {
            let entry = vdict! {
                "name" => w.name.as_str(),
                "caliber_mm" => w.caliber_mm_x10 as f32 / 10.0,
                "rate_rpm" => i64::from(w.rate_rpm),
                "mount" => t.combat.mg_mount(rules, index).map_or("", sim::MgMount::name),
            };
            weapons.push(&entry.to_variant());
        }
        let mut out = vdict! {
            "id" => t.name.as_str(),
            "category" => t.combat.category.map_or("", |c| c.code()),
            "mass_kg" => i64::from(t.mass_kg),
            "road_speed" => fixed_to_f32(t.road_speed_m_per_s),
        };
        out.set("weapons", &weapons);
        out
    }

    /// Number of buildings (box-shaped map objects from `BUILDING_HEIGHT_M`
    /// high, ruins included) within `radius` meters of `(x, z)`. The
    /// acoustics (prompt 053) pick the reverb of a village from it; read
    /// only, a few times per second.
    #[func]
    fn buildings_near(&self, x: f32, z: f32, radius: f32) -> i64 {
        const BUILDING_HEIGHT_M: f32 = 3.0;
        let Some(game) = &self.game else {
            return 0;
        };
        let map = game.map();
        let kinds = map.object_kinds();
        let r2 = radius * radius;
        let count = map
            .objects()
            .iter()
            .filter(|o| {
                let kind = &kinds[usize::from(o.kind)];
                if !matches!(kind.shape, sim::objects::Shape::Box { .. })
                    || fixed_to_f32(kind.height) < BUILDING_HEIGHT_M
                {
                    return false;
                }
                let dx = fixed_to_f32(o.position.x) - x;
                let dz = fixed_to_f32(o.position.y) - z;
                dx * dx + dz * dz <= r2
            })
            .count();
        i64::try_from(count).unwrap_or(i64::MAX)
    }
}
