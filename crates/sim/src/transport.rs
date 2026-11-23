//! Transport, mounting and towing (prompt 034): the rules of
//! `data/transport.toml` and what a unit type brings from the catalog
//! (seats, engine power). The update itself (orders, mounting and
//! dismounting, hitching, the towed gun following its tractor, losses):
//! `crate::sim::transport`. See docs/ARCHITECTURE.md "Transport and
//! towing".

use crate::catalog::{Category, Entry, Group};
use crate::hash::StableHasher;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

/// The rules compiled into the simulation.
pub const BUILTIN_TRANSPORT: &str = include_str!("../../../data/transport.toml");

#[derive(Debug, PartialEq, Eq)]
pub struct TransportError(pub String);

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid transport rules: {}", self.0)
    }
}

impl std::error::Error for TransportError {}

/// Rules of one carrier category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CarrierRules {
    pub category: Category,
    /// Soldiers riding on the outside of a tank; 0: catalog seats.
    pub riders: u32,
    pub open: bool,
    /// Time one soldier needs to climb aboard once he stands at the
    /// carrier (prompt 163: soldiers mount one after another as they
    /// arrive; before, the whole squad waited and mounted together).
    pub climb_ms: u32,
    pub disembark_ms: u32,
    pub hit_pct: u32,
    pub bounce_pct: u32,
    pub destroyed_pct: u32,
    pub tows: bool,
}

/// Rules of one towed-gun category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GunRules {
    pub category: Category,
    pub limber_ms: u32,
    pub unlimber_ms: u32,
    pub manhandle_kmh_x10: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TowingRules {
    pub min_hp_per_tonne: u32,
    pub hitch_gap_mm: u32,
    pub reach_mm: u32,
}

/// Rules of `data/transport.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransportRules {
    #[serde(rename = "carrier")]
    pub carriers: Vec<CarrierRules>,
    #[serde(rename = "gun")]
    pub guns: Vec<GunRules>,
    pub towing: TowingRules,
}

impl TransportRules {
    pub fn parse(text: &str) -> Result<Self, TransportError> {
        let rules: Self = toml::from_str(text).map_err(|e| TransportError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(TransportError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        for c in &rules.carriers {
            if c.category.group() != Group::Vehicle {
                return Err(TransportError(format!(
                    "carrier category '{}' is no vehicle",
                    c.category.code()
                )));
            }
            check("carrier.riders", c.riders, 0, 30)?;
            check("carrier.climb_ms", c.climb_ms, 0, 600_000)?;
            check("carrier.disembark_ms", c.disembark_ms, 0, 600_000)?;
            check("carrier.hit_pct", c.hit_pct, 0, 100)?;
            check("carrier.bounce_pct", c.bounce_pct, 0, 100)?;
            check("carrier.destroyed_pct", c.destroyed_pct, 0, 100)?;
        }
        for g in &rules.guns {
            if g.category.group() != Group::Gun {
                return Err(TransportError(format!(
                    "gun category '{}' is no gun",
                    g.category.code()
                )));
            }
            check("gun.limber_ms", g.limber_ms, 0, 600_000)?;
            check("gun.unlimber_ms", g.unlimber_ms, 0, 600_000)?;
            check("gun.manhandle_kmh_x10", g.manhandle_kmh_x10, 1, 100)?;
        }
        let mut seen: Vec<Category> = rules
            .carriers
            .iter()
            .map(|c| c.category)
            .chain(rules.guns.iter().map(|g| g.category))
            .collect();
        seen.sort_unstable();
        if seen.windows(2).any(|w| w[0] == w[1]) {
            return Err(TransportError("categories must be unique".into()));
        }
        let t = &rules.towing;
        check("towing.min_hp_per_tonne", t.min_hp_per_tonne, 1, 1000)?;
        check("towing.hitch_gap_mm", t.hitch_gap_mm, 0, 10_000)?;
        check("towing.reach_mm", t.reach_mm, 500, 50_000)?;
        Ok(rules)
    }

    /// The rules from `data/transport.toml`, compiled in.
    pub fn builtin() -> Result<Self, TransportError> {
        Self::parse(BUILTIN_TRANSPORT)
    }

    /// Rules of a carrier category.
    pub fn carrier(&self, category: Option<Category>) -> Option<&CarrierRules> {
        self.carriers.iter().find(|c| Some(c.category) == category)
    }

    /// Rules of a towed-gun category.
    pub fn gun(&self, category: Option<Category>) -> Option<&GunRules> {
        self.guns.iter().find(|g| Some(g.category) == category)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.carriers.len() as u64);
        for c in &self.carriers {
            h.write_bytes(c.category.code().as_bytes());
            h.write_u8(0);
            for v in [
                c.riders,
                u32::from(c.open),
                c.climb_ms,
                c.disembark_ms,
                c.hit_pct,
                c.bounce_pct,
                c.destroyed_pct,
                u32::from(c.tows),
            ] {
                h.write_u32(v);
            }
        }
        h.write_u64(self.guns.len() as u64);
        for g in &self.guns {
            h.write_bytes(g.category.code().as_bytes());
            h.write_u8(0);
            h.write_u32(g.limber_ms);
            h.write_u32(g.unlimber_ms);
            h.write_u32(g.manhandle_kmh_x10);
        }
        let t = &self.towing;
        h.write_u32(t.min_hp_per_tonne);
        h.write_u32(t.hitch_gap_mm);
        h.write_u32(t.reach_mm);
    }
}

/// The compiled-in rules, parsed once (for converting catalog entries).
fn builtin_rules() -> Option<&'static TransportRules> {
    static RULES: OnceLock<Option<TransportRules>> = OnceLock::new();
    RULES
        .get_or_init(|| TransportRules::builtin().ok())
        .as_ref()
}

/// Speed in real km/h × 10 at which the crew pushes a gun of `category`
/// (`None`: not a towed gun of the rules).
pub fn manhandle_kmh_x10(category: Category) -> Option<u32> {
    builtin_rules()?
        .gun(Some(category))
        .map(|g| g.manhandle_kmh_x10)
}

/// What a unit type brings for transport from the catalog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportType {
    /// Troop seats (catalog `passengers`), 0 if none.
    pub passengers: u16,
    /// Engine power in metric hp (towing), 0 if unknown.
    pub power_hp: u32,
    /// Crew (catalog `crew`), 0 if none: the men who may bail out of the
    /// destroyed vehicle (prompt 049).
    #[serde(default)]
    pub crew: u16,
    /// Boats (prompt 119): draft in mm; 0 for every unit on land. A unit
    /// type with a draft moves on the water ([`crate::path::SizeClass::Boat`]).
    #[serde(default)]
    pub draft_mm: u32,
    /// Boats: payload of vehicles and guns (kg) and how many at once.
    #[serde(default)]
    pub cargo_kg: u32,
    #[serde(default)]
    pub cargo_vehicles: u16,
    /// Boats paddled by the men aboard.
    #[serde(default)]
    pub paddled: bool,
}

impl TransportType {
    pub fn from_catalog(entry: &Entry) -> Self {
        Self {
            passengers: entry.passengers.unwrap_or(0),
            power_hp: entry.power_hp.unwrap_or(0),
            crew: entry.crew.unwrap_or(0),
            draft_mm: if entry.category.is_boat() {
                entry.draft_mm.unwrap_or(0).max(1)
            } else {
                0
            },
            cargo_kg: entry.cargo_kg.unwrap_or(0),
            cargo_vehicles: entry.cargo_vehicles.unwrap_or(0),
            paddled: entry.paddled.unwrap_or(false),
        }
    }
}

/// Game speed (m/s) of a gun pushed by its crew, from the rules.
pub fn manhandle_speed(entry: &Entry, scale: &Scale) -> Option<crate::math::Fixed> {
    let kmh = manhandle_kmh_x10(entry.category)?;
    Some(scale.game_speed(entry.category, kmh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = TransportRules::builtin().unwrap();
        assert!(rules.carrier(Some(Category::Truck)).is_some_and(|c| c.tows));
        assert_eq!(
            rules.carrier(Some(Category::TankMedium)).map(|c| c.riders),
            Some(10)
        );
        assert!(rules.gun(Some(Category::GunAt)).is_some());
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let bad = BUILTIN_TRANSPORT.replace("hit_pct = 50", "hit_pct = 250");
        assert!(TransportRules::parse(&bad).is_err());
        let wrong = BUILTIN_TRANSPORT.replace("category = \"gun_at\"", "category = \"truck\"");
        assert!(TransportRules::parse(&wrong).is_err());
    }
}
