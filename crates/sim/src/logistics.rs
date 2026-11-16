//! Supply, consumption and repair (prompt 036): the rules of
//! `data/logistics.toml` and the stocks a unit type brings from the
//! catalog. The update itself (consumption, resupply, repair, building
//! repair and selling): `crate::sim::logistics`. See docs/GAME_DESIGN.md
//! "Supply and repair".

use crate::catalog::{Category, Entry, Group};
use crate::hash::StableHasher;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

/// The rules compiled into the simulation.
pub const BUILTIN_LOGISTICS: &str = include_str!("../../../data/logistics.toml");

/// A full stock in millionths.
pub const FULL: u32 = 1_000_000;

#[derive(Debug, PartialEq, Eq)]
pub struct LogisticsError(pub String);

impl fmt::Display for LogisticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid logistics rules: {}", self.0)
    }
}

impl std::error::Error for LogisticsError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoRules {
    pub default_infantry: u32,
    pub default_vehicle: u32,
    pub default_gun: u32,
    pub min_salvos: u32,
    pub max_salvos: u32,
    pub low_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FuelRules {
    pub low_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DamageRules {
    pub immobile_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResupplyRules {
    pub pct_per_s: u32,
    pub cost_pct: u32,
}

/// A supplier or repairer: a base building or units of a category.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    #[serde(default)]
    pub building: Option<String>,
    #[serde(default)]
    pub category: Option<Category>,
    pub radius_m: u32,
    /// Repair rate (repairers only).
    #[serde(default)]
    pub pct_per_s: u32,
}

impl Source {
    /// Whether units of the type `name` / `category` are this source.
    pub fn matches(&self, name: &str, category: Option<Category>) -> bool {
        self.building.as_deref() == Some(name)
            || (self.category.is_some() && self.category == category)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRules {
    pub repair_pct_per_s: u32,
    pub repair_cost_pct: u32,
    pub sell_refund_pct: u32,
    pub cancel_refund_pct: u32,
}

/// Rules of `data/logistics.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogisticsRules {
    pub ammo: AmmoRules,
    pub fuel: FuelRules,
    pub damage: DamageRules,
    pub resupply: ResupplyRules,
    #[serde(rename = "supplier")]
    pub suppliers: Vec<Source>,
    #[serde(rename = "repairer")]
    pub repairers: Vec<Source>,
    pub buildings: BuildingRules,
}

impl LogisticsRules {
    pub fn parse(text: &str) -> Result<Self, LogisticsError> {
        let rules: Self = toml::from_str(text).map_err(|e| LogisticsError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(LogisticsError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let a = &rules.ammo;
        check("ammo.min_salvos", a.min_salvos, 1, 10_000)?;
        check("ammo.max_salvos", a.max_salvos, a.min_salvos, 100_000)?;
        for (name, v) in [
            ("ammo.default_infantry", a.default_infantry),
            ("ammo.default_vehicle", a.default_vehicle),
            ("ammo.default_gun", a.default_gun),
        ] {
            check(name, v, 1, 100_000)?;
        }
        check("ammo.low_pct", a.low_pct, 0, 100)?;
        check("fuel.low_pct", rules.fuel.low_pct, 0, 100)?;
        check("damage.immobile_pct", rules.damage.immobile_pct, 0, 99)?;
        check("resupply.pct_per_s", rules.resupply.pct_per_s, 1, 100)?;
        check("resupply.cost_pct", rules.resupply.cost_pct, 0, 1000)?;
        for s in rules.suppliers.iter().chain(&rules.repairers) {
            if s.building.is_some() == s.category.is_some() {
                return Err(LogisticsError(
                    "a source needs either `building` or `category`".into(),
                ));
            }
            check("radius_m", s.radius_m, 1, 1000)?;
        }
        for s in &rules.repairers {
            check("repairer.pct_per_s", s.pct_per_s, 1, 100)?;
        }
        let b = &rules.buildings;
        check("buildings.repair_pct_per_s", b.repair_pct_per_s, 1, 100)?;
        check("buildings.repair_cost_pct", b.repair_cost_pct, 0, 1000)?;
        check("buildings.sell_refund_pct", b.sell_refund_pct, 0, 100)?;
        check("buildings.cancel_refund_pct", b.cancel_refund_pct, 0, 100)?;
        Ok(rules)
    }

    /// The rules from `data/logistics.toml`, compiled in.
    pub fn builtin() -> Result<Self, LogisticsError> {
        Self::parse(BUILTIN_LOGISTICS)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0x36);
        let a = &self.ammo;
        for v in [
            a.default_infantry,
            a.default_vehicle,
            a.default_gun,
            a.min_salvos,
            a.max_salvos,
            a.low_pct,
            self.fuel.low_pct,
            self.damage.immobile_pct,
            self.resupply.pct_per_s,
            self.resupply.cost_pct,
            self.buildings.repair_pct_per_s,
            self.buildings.repair_cost_pct,
            self.buildings.sell_refund_pct,
            self.buildings.cancel_refund_pct,
        ] {
            h.write_u32(v);
        }
        for list in [&self.suppliers, &self.repairers] {
            h.write_u64(list.len() as u64);
            for s in list {
                h.write_bytes(s.building.as_deref().unwrap_or("").as_bytes());
                h.write_u8(0);
                h.write_bytes(s.category.map_or("", Category::code).as_bytes());
                h.write_u8(0);
                h.write_u32(s.radius_m);
                h.write_u32(s.pct_per_s);
            }
        }
    }
}

/// The compiled-in rules, parsed once (for converting catalog entries).
fn builtin_rules() -> Option<&'static LogisticsRules> {
    static RULES: OnceLock<Option<LogisticsRules>> = OnceLock::new();
    RULES
        .get_or_init(|| LogisticsRules::builtin().ok())
        .as_ref()
}

/// The stocks a unit type brings from the catalog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogisticsType {
    /// Salvos of the main weapon in a full stock; 0: no ammunition stock
    /// (unarmed units, buildings, fortifications).
    pub salvos: u32,
    /// Game meters driven on a full tank; 0: no fuel stock (everything but
    /// vehicles).
    pub fuel_m: u32,
}

impl LogisticsType {
    /// Stocks of a mobile catalog unit (fortifications have none: the base
    /// supplies them).
    pub fn from_catalog(entry: &Entry, scale: &Scale) -> Self {
        let Some(rules) = builtin_rules() else {
            return Self::default();
        };
        let group = entry.category.group();
        // The main weapon: the indirect one of mortars and howitzers, else
        // the first weapon combat fires (all values present).
        let main = if crate::indirect::is_indirect_category(entry.category) {
            entry.weapon.first()
        } else {
            entry.weapon.iter().find(|w| {
                w.caliber_mm_x10.is_some_and(|c| c > 0)
                    && w.rate_rpm.is_some_and(|r| r > 0)
                    && w.muzzle_velocity_mps.is_some_and(|v| v > 0)
                    && w.range_effective_m.is_some_and(|r| r > 0)
            })
        };
        let salvos = match (main, group) {
            (None, _) | (_, Group::Aircraft | Group::Fortification | Group::Environment) => 0,
            (Some(w), _) => {
                let a = &rules.ammo;
                let default = match group {
                    Group::Infantry => a.default_infantry,
                    Group::Vehicle => a.default_vehicle,
                    _ => a.default_gun,
                };
                w.ammo_carried
                    .map_or(default, |n| n / u32::from(w.count.max(1)))
                    .clamp(a.min_salvos, a.max_salvos)
            }
        };
        let fuel_m = match (group, entry.range_road_km) {
            (Group::Vehicle, Some(km)) => scale.fuel_range_m(km),
            _ => 0,
        };
        Self { salvos, fuel_m }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = LogisticsRules::builtin().unwrap();
        assert!(
            rules
                .suppliers
                .iter()
                .any(|s| s.matches("x", Some(Category::Truck)))
        );
        assert!(
            rules
                .repairers
                .iter()
                .any(|s| s.matches("base_factory", None))
        );
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let bad = BUILTIN_LOGISTICS.replace("immobile_pct = 35", "immobile_pct = 100");
        assert!(LogisticsRules::parse(&bad).is_err());
        let both = BUILTIN_LOGISTICS.replace(
            "building = \"base_hq\"",
            "building = \"base_hq\"\ncategory = \"truck\"",
        );
        assert!(LogisticsRules::parse(&both).is_err());
    }

    #[test]
    fn stocks_come_from_the_catalog() {
        let catalog = crate::Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let pz4 = LogisticsType::from_catalog(catalog.get("de_pz4_h").unwrap(), &scale);
        // 87 rounds of the KwK 40; 210 km road range / 20.
        assert_eq!(pz4.salvos, 87);
        assert_eq!(pz4.fuel_m, 10_500);
        let squad = LogisticsType::from_catalog(catalog.get("de_inf_rifle_squad").unwrap(), &scale);
        assert!(squad.salvos > 0);
        assert_eq!(squad.fuel_m, 0);
    }
}
