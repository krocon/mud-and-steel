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
    /// and `weapons`, in the order of the `weapon` index of shot events:
    /// `{name, caliber_mm, rate_rpm}`. Empty without a match or for an
    /// unknown index.
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
        for w in &t.combat.weapons {
            let entry = vdict! {
                "name" => w.name.as_str(),
                "caliber_mm" => w.caliber_mm_x10 as f32 / 10.0,
                "rate_rpm" => i64::from(w.rate_rpm),
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
}
