//! Interface language of names that come from the data files (prompt 043).
//! Presentation only: the simulation keeps its ids and never sees a
//! language.
//!
//! Catalog entries carry `name_de`/`name_en`; building names
//! (`data/economy.toml`) and rank names (`data/experience.toml`) are
//! translated through the Godot translation keys `NAME_<ID>` and
//! `RANK_<NAME>` (`godot/locale/names.csv`), falling back to the English
//! name of the data file.

use crate::SimHost;
use godot::classes::TranslationServer;
use godot::prelude::*;
use sim::catalog::Entry;
use std::sync::atomic::{AtomicBool, Ordering};

/// True while the interface is German. English is the default, so tests
/// without a Godot runtime never reach the translation server.
static GERMAN: AtomicBool = AtomicBool::new(false);

fn german() -> bool {
    GERMAN.load(Ordering::Relaxed)
}

/// The catalog name of `e` in the interface language.
pub(crate) fn entry_name(e: &Entry) -> &str {
    if german() { &e.name_de } else { &e.name_en }
}

/// The translation of `key` (`<PREFIX>_<UPPER CASE TEXT>`) or `english`.
pub(crate) fn translated(prefix: &str, code: &str, english: &str) -> String {
    if !german() {
        return english.to_owned();
    }
    let key: String = format!("{prefix}{code}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    let text = TranslationServer::singleton()
        .translate(&StringName::from(key.as_str()))
        .to_string();
    if text == key {
        english.to_owned()
    } else {
        text
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Sets the language of unit, building and rank names (`en`, `de`;
    /// anything else is English). Called by `language.gd`.
    #[func]
    fn set_language(code: GString) {
        GERMAN.store(code == "de", Ordering::Relaxed);
    }
}
