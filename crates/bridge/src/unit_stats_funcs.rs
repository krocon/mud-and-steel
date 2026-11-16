//! `SimHost.unit_stats`: the game values of a unit type (prompt 066), the
//! card of `sim::unit_stats` as a dictionary. No calculation here: the card
//! is computed by the simulation's own rule functions and only converted.
//! Static and read-only like `catalog_card`. Presentation only.

use crate::SimHost;
use godot::prelude::*;
use sim::unit_stats::{StatRules, Value};
use std::sync::OnceLock;

/// The rules compiled into the simulation, parsed once.
fn rules() -> Option<&'static StatRules> {
    static RULES: OnceLock<Option<StatRules>> = OnceLock::new();
    RULES.get_or_init(|| StatRules::builtin().ok()).as_ref()
}

fn variant(value: &Value) -> Variant {
    match value {
        Value::Int(v) => v.to_variant(),
        Value::Bool(v) => v.to_variant(),
        Value::Text(v) => v.as_str().to_variant(),
        Value::List(items) => items.iter().map(variant).collect::<VarArray>().to_variant(),
        Value::Map(_) => dictionary(value).to_variant(),
    }
}

fn dictionary(value: &Value) -> VarDictionary {
    let mut out = VarDictionary::new();
    if let Value::Map(entries) = value {
        for (key, v) in entries {
            out.set(key.as_str(), &variant(v));
        }
    }
    out
}

#[godot_api(secondary)]
impl SimHost {
    /// The game values of the catalog id `type_id` (empty if unknown): the
    /// card of `cli unit-stats` with `ratings` (`speed`, `protection`,
    /// `anti_vehicle`, `anti_infantry`, `anti_structure`, each 0–100 and
    /// only where it applies), `movement`, `protection`, `weapons`,
    /// `indirect`, `against_vehicles`, `against_infantry`,
    /// `against_structures`, `other`, `object`. Integers with the unit in
    /// the key; see docs/ARCHITECTURE.md "Unit stats". Computed on every
    /// call: cache it per type.
    #[func]
    fn unit_stats(type_id: GString) -> VarDictionary {
        rules()
            .and_then(|r| r.card_of(&type_id.to_string()))
            .map_or_else(VarDictionary::new, |card| dictionary(&card))
    }

    /// What the cards compare with: `distances_real_m`,
    /// `duel_distance_real_m`, `weapons`, `targets`, `objects`.
    #[func]
    fn unit_stats_reference() -> VarDictionary {
        dictionary(&sim::unit_stats::reference())
    }
}
