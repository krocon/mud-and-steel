//! `SimHost` functions for the build and production picker (prompt 058):
//! everything a player can buy, and the catalog values a tile's tooltip
//! shows. Static and read-only: they read the economy rules and the catalog
//! compiled into the simulation, so the thumbnail tool and the gallery check
//! can call them without a match. Presentation only.

use crate::SimHost;
use godot::prelude::*;
use sim::catalog::{Entry, Group};

fn group_code(group: Group) -> &'static str {
    match group {
        Group::Infantry => "infantry",
        Group::Vehicle => "vehicle",
        Group::Gun => "gun",
        Group::Fortification => "fortification",
        Group::Aircraft => "aircraft",
        Group::Environment => "environment",
    }
}

fn faces(values: [Option<u16>; 3]) -> PackedInt64Array {
    if values.iter().all(Option::is_none) {
        return PackedInt64Array::new();
    }
    values.iter().map(|v| i64::from(v.unwrap_or(0))).collect()
}

fn kmh(x10: Option<u32>) -> f64 {
    x10.map_or(0.0, |v| f64::from(v) / 10.0)
}

fn entry_card(e: &Entry) -> VarDictionary {
    let mut weapons = Array::<VarDictionary>::new();
    for w in &e.weapon {
        weapons.push(&vdict! {
            "name" => w.name.as_str(),
            "count" => i64::from(w.count),
            "caliber_mm" => w.caliber_mm_x10.map_or(0.0, |c| f64::from(c) / 10.0),
            // The typical combat distance (prompt 067); indirect weapons
            // have only their maximum range.
            "range_m" => i64::from(w.range_typical_m.or(w.range_effective_m).unwrap_or(0)),
        });
    }
    vdict! {
        "id" => e.id.as_str(),
        "name" => crate::language_funcs::entry_name(e),
        "designation" => e.designation.as_str(),
        "faction" => e.faction.code(),
        "group" => group_code(e.category.group()),
        "category" => e.category.code(),
        "year" => i64::from(e.year_from.unwrap_or(0)),
        "year_to" => i64::from(e.year_to.unwrap_or(0)),
        "crew" => i64::from(e.crew.unwrap_or(0)),
        "squad_size" => i64::from(e.squad_size.unwrap_or(0)),
        "passengers" => i64::from(e.passengers.unwrap_or(0)),
        "mass_t" => f64::from(e.mass_kg.unwrap_or(0)) / 1000.0,
        "speed_road_kmh" => kmh(e.speed_road_kmh_x10),
        "speed_offroad_kmh" => kmh(e.speed_offroad_kmh_x10),
        "armor_hull" => &faces([e.armor_hull_front_mm, e.armor_hull_side_mm, e.armor_hull_rear_mm]),
        "armor_turret" => &faces([e.armor_turret_front_mm, e.armor_turret_side_mm, e.armor_turret_rear_mm]),
        "weapons" => &weapons,
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Every id a player of any faction can build or produce, from the
    /// economy rules (base buildings, fortifications, production lists), in
    /// that order without repeats. What gets a thumbnail (prompt 058).
    #[func]
    fn buildable_ids() -> PackedStringArray {
        let Ok(rules) = sim::EconomyRules::builtin() else {
            return PackedStringArray::new();
        };
        let fortifications = rules.fortifications.iter().flat_map(|f| &f.ids);
        let produced = rules.produce.values().flat_map(|l| l.values().flatten());
        let mut ids: Vec<&String> = Vec::new();
        for id in rules
            .buildings
            .iter()
            .map(|b| &b.id)
            .chain(fortifications)
            .chain(produced)
        {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.into_iter()
            .map(|id| GString::from(id.as_str()))
            .collect()
    }

    /// Every id the unit reference of the main menu lists (prompt 066):
    /// the base buildings of the economy rules, then every catalog entry
    /// that is not an environment object, in catalog order. Tier-3 entries
    /// (prompt 063: from the vehicle references, not buildable) are left out
    /// until they have a model.
    #[func]
    fn catalog_ids() -> PackedStringArray {
        let buildings = sim::EconomyRules::builtin()
            .map(|rules| rules.buildings.iter().map(|b| b.id.clone()).collect())
            .unwrap_or_else(|_| Vec::new());
        let entries = sim::economy::builtin_catalog().map_or_else(Vec::new, |c| {
            c.entries
                .iter()
                .filter(|e| e.category.group() != Group::Environment)
                .filter(|e| e.tier != 3 || !e.model.is_empty())
                .map(|e| e.id.clone())
                .collect()
        });
        buildings
            .iter()
            .chain(&entries)
            .map(|id| GString::from(id.as_str()))
            .collect()
    }

    /// What the picker's tooltip shows of `type_id` (empty if unknown):
    /// `id`, `name`, `group` (`building`, or the catalog group `infantry`,
    /// `vehicle`, `gun`, `fortification`, `aircraft`) and, for catalog
    /// entries, `designation`, `faction`, `category`, `year`, `year_to`, `crew`,
    /// `squad_size`, `passengers`, `mass_t`, `speed_road_kmh`,
    /// `speed_offroad_kmh`, `armor_hull` / `armor_turret` (front, side, rear
    /// mm; empty if unknown), `weapons` (`name`, `count`, `caliber_mm`,
    /// `range_m` = real effective range). Base buildings have `hp`,
    /// `armor_mm`, `income` (per minute), `cost` (fuel, ammunition,
    /// manpower), `build_s`, `produces` and `requires` (names).
    #[func]
    fn catalog_card(type_id: GString) -> VarDictionary {
        let id = type_id.to_string();
        if let Ok(rules) = sim::EconomyRules::builtin()
            && let Some(b) = rules.buildings.iter().find(|b| b.id == id)
        {
            let income: PackedInt64Array = b.income_per_min.iter().map(|&v| i64::from(v)).collect();
            let cost: PackedInt64Array = b.cost.iter().map(|&v| i64::from(v)).collect();
            let requires: PackedStringArray = b
                .requires
                .iter()
                .map(|r| GString::from(crate::economy_funcs::display_name(r).as_str()))
                .collect();
            return vdict! {
                "id" => b.id.as_str(),
                "name" => crate::economy_funcs::display_name(&b.id).as_str(),
                "group" => "building",
                "hp" => i64::from(b.hp),
                "armor_mm" => i64::from(b.armor_mm),
                "income" => &income,
                "cost" => &cost,
                "build_s" => i64::from(b.build_s),
                "produces" => b.produces.as_str(),
                "requires" => &requires,
            };
        }
        sim::economy::builtin_catalog()
            .and_then(|c| c.get(&id))
            .map_or_else(VarDictionary::new, entry_card)
    }
}
