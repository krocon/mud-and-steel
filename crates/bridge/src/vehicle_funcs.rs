//! `SimHost.vehicle_record`: the technical data of a vehicle from the
//! vehicle database (`data/vehicles/*.json`, prompt 064). A reference work
//! next to the catalog, never input of a game rule, so it lives here and not
//! in `crates/sim`. The files are compiled into the bridge like the catalog
//! is compiled into the simulation: the exported game needs no extra file
//! and cannot run with a database that does not match its catalog.
//! Presentation only.

use crate::SimHost;
use godot::classes::Json;
use godot::prelude::*;
use std::mem::ManuallyDrop;

/// The database files by faction prefix of the catalog id.
const FILES: [(&str, &str); 3] = [
    ("de_", include_str!("../../../data/vehicles/de.json")),
    ("su_", include_str!("../../../data/vehicles/su.json")),
    ("us_", include_str!("../../../data/vehicles/us.json")),
];

thread_local! {
    /// The parsed files, in the order of `FILES`; parsed on first use. Never
    /// dropped: thread-local destructors run after the engine is gone, and
    /// dropping a `Variant` then panics.
    static PARSED: ManuallyDrop<Vec<Variant>> =
        ManuallyDrop::new(FILES.iter().map(|(_, text)| Json::parse_string(*text)).collect());
}

#[godot_api(secondary)]
impl SimHost {
    /// The database entry of the catalog id `type_id` as a dictionary in the
    /// shape of `data/vehicles/SCHEMA.md` (`id`, `name`, `type`, `family`,
    /// `source`, `general`, `firepower`, `mobility`, `armor`, `notes`), or an
    /// empty dictionary for units without an entry. Numbers arrive as floats
    /// (Godot's JSON has one number type).
    #[func]
    fn vehicle_record(type_id: GString) -> VarDictionary {
        let id = type_id.to_string();
        let Some(file) = FILES.iter().position(|(prefix, _)| id.starts_with(prefix)) else {
            return VarDictionary::new();
        };
        PARSED.with(|parsed| {
            let Ok(entries) = parsed[file].try_to::<VarArray>() else {
                return VarDictionary::new();
            };
            entries
                .iter_shared()
                .filter_map(|entry| entry.try_to::<VarDictionary>().ok())
                .find(|entry| {
                    entry
                        .get("id")
                        .is_some_and(|v| v.try_to::<GString>().is_ok_and(|v| v == type_id))
                })
                .unwrap_or_default()
        })
    }
}
