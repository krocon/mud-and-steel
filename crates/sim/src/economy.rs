//! Economy rules (prompt 023): resources, base buildings, fortifications,
//! production lists and the unit cost formula, from `data/economy.toml`.
//!
//! The design and its reasons are in `docs/GAME_DESIGN.md`; the state
//! (accounts, construction sites, production queues, supply points) lives in
//! `crate::sim::economy`.

use crate::catalog::{Catalog, Entry, Group};
use crate::combat::CombatType;
use crate::data::{DataError, UnitType, unit_types_from_catalog};
use crate::hash::StableHasher;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::OnceLock;

/// The rules compiled into the simulation.
pub const BUILTIN_ECONOMY: &str = include_str!("../../../data/economy.toml");

/// Amounts of the three resources, always in this order.
pub type Resources = [u32; 3];

/// Names of the resources, in the order of [`Resources`].
pub const RESOURCE_NAMES: [&str; 3] = ["fuel", "ammo", "manpower"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomyError(pub String);

impl fmt::Display for EconomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid economy rules: {}", self.0)
    }
}

impl std::error::Error for EconomyError {}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountRules {
    pub start: Resources,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructionRules {
    pub max_sites: u32,
    pub start_hp_pct: u32,
    pub max_grade_pct: u32,
    pub clearance_m: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionRules {
    pub max_queue: u32,
    pub max_units: u32,
    pub rally_m: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyRules {
    pub marker_prefix: String,
    pub radius_m: u32,
    pub capture_s: u32,
    pub income_per_min: Resources,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnitCostRules {
    pub manpower_per_soldier: u32,
    pub fuel_per_tonne: u32,
    pub ammo_base: u32,
    pub ammo_caliber_div: u32,
    pub time_base_s: u32,
    pub time_per_point_div: u32,
}

/// A base building (not in the catalog).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRule {
    pub id: String,
    pub name: String,
    pub length_mm: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub hp: u32,
    pub armor_mm: u16,
    pub cost: Resources,
    pub build_s: u32,
    pub build_radius_m: u32,
    pub income_per_min: Resources,
    pub max_count: u32,
    pub requires: Vec<String>,
    pub produces: String,
}

/// Catalog entries that can be placed like buildings.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FortificationRule {
    pub ids: Vec<String>,
    /// 0 = from the combat rules.
    pub hp: u32,
    /// 0 = from the catalog.
    pub armor_mm: u16,
    /// Absent = from the unit cost formula.
    #[serde(default)]
    pub cost: Option<Resources>,
    #[serde(default)]
    pub build_s: Option<u32>,
    pub requires: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct EconomyFile {
    account: AccountRules,
    construction: ConstructionRules,
    production: ProductionRules,
    supply: SupplyRules,
    unit_cost: UnitCostRules,
    building: Vec<BuildingRule>,
    fortification: Vec<FortificationRule>,
    produce: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

/// Price and production time of something a player can buy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cost {
    pub resources: Resources,
    /// Game milliseconds to build or produce it.
    pub time_ms: u32,
}

/// What a structure type is, as the economy sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureRule<'a> {
    Building(&'a BuildingRule),
    Fortification(&'a FortificationRule),
}

/// The validated economy rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyRules {
    pub account: AccountRules,
    pub construction: ConstructionRules,
    pub production: ProductionRules,
    pub supply: SupplyRules,
    pub unit_cost: UnitCostRules,
    pub buildings: Vec<BuildingRule>,
    pub fortifications: Vec<FortificationRule>,
    /// Production list name -> faction -> catalog ids.
    pub produce: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl EconomyRules {
    /// The rules compiled into the simulation (parsed once).
    pub fn builtin() -> Result<Self, EconomyError> {
        static RULES: OnceLock<Result<EconomyRules, EconomyError>> = OnceLock::new();
        RULES.get_or_init(|| Self::parse(BUILTIN_ECONOMY)).clone()
    }

    pub fn parse(text: &str) -> Result<Self, EconomyError> {
        let file: EconomyFile = toml::from_str(text).map_err(|e| EconomyError(e.to_string()))?;
        let rules = Self {
            account: file.account,
            construction: file.construction,
            production: file.production,
            supply: file.supply,
            unit_cost: file.unit_cost,
            buildings: file.building,
            fortifications: file.fortification,
            produce: file.produce,
        };
        rules.validate()?;
        Ok(rules)
    }

    fn validate(&self) -> Result<(), EconomyError> {
        let err = |m: String| Err(EconomyError(m));
        if self.unit_cost.ammo_caliber_div == 0 || self.unit_cost.time_per_point_div == 0 {
            return err("unit_cost divisors must be positive".into());
        }
        if self.construction.start_hp_pct == 0 || self.construction.start_hp_pct > 100 {
            return err("start_hp_pct must be 1..=100".into());
        }
        let mut ids: Vec<&str> = Vec::new();
        for b in &self.buildings {
            if !b.id.starts_with("base_") {
                return err(format!("building '{}' must start with 'base_'", b.id));
            }
            if b.length_mm == 0 || b.width_mm == 0 || b.height_mm == 0 || b.hp == 0 {
                return err(format!("building '{}' needs sizes and hit points", b.id));
            }
            if !b.produces.is_empty() && !self.produce.contains_key(&b.produces) {
                return err(format!(
                    "building '{}' produces unknown list '{}'",
                    b.id, b.produces
                ));
            }
            ids.push(&b.id);
        }
        for f in &self.fortifications {
            ids.extend(f.ids.iter().map(String::as_str));
        }
        for b in &self.buildings {
            for r in &b.requires {
                if !ids.contains(&r.as_str()) {
                    return err(format!("'{}' requires unknown '{r}'", b.id));
                }
            }
        }
        for f in &self.fortifications {
            for r in &f.requires {
                if !ids.contains(&r.as_str()) {
                    return err(format!("fortification requires unknown '{r}'"));
                }
            }
        }
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() != ids.len() {
            return err("structure ids must be unique".into());
        }
        Ok(())
    }

    /// The structure rule of a unit type name, if it is one.
    pub fn structure(&self, id: &str) -> Option<StructureRule<'_>> {
        if let Some(b) = self.buildings.iter().find(|b| b.id == id) {
            return Some(StructureRule::Building(b));
        }
        self.fortifications
            .iter()
            .find(|f| f.ids.iter().any(|i| i == id))
            .map(StructureRule::Fortification)
    }

    /// Price of a catalog unit by the formula of `[unit_cost]`.
    pub fn unit_cost(&self, entry: &Entry, combat: &CombatType) -> Cost {
        let r = &self.unit_cost;
        let soldiers = u32::from(entry.squad_size.or(entry.crew).unwrap_or(1));
        let manpower = r.manpower_per_soldier.saturating_mul(soldiers);
        let fuel = if entry.category.group() == Group::Vehicle {
            u32::try_from(
                u64::from(entry.mass_kg.unwrap_or(0)) * u64::from(r.fuel_per_tonne) / 1000,
            )
            .unwrap_or(u32::MAX)
        } else {
            0
        };
        let squares: u64 = combat
            .weapons
            .iter()
            .map(|w| (w.caliber_mm_x10, w.count))
            .chain(combat.indirect.iter().map(|w| (w.caliber_mm_x10, 1)))
            .map(|(caliber, count)| {
                let mm = u64::from(caliber);
                mm * mm * u64::from(count) / 100
            })
            .sum();
        let ammo = r.ammo_base.saturating_add(
            u32::try_from(squares / u64::from(r.ammo_caliber_div)).unwrap_or(u32::MAX),
        );
        let resources = [fuel, ammo, manpower];
        Cost {
            resources,
            time_ms: self.time_ms(resources),
        }
    }

    /// Production time of a price by the formula of `[unit_cost]`.
    fn time_ms(&self, resources: Resources) -> u32 {
        let r = &self.unit_cost;
        let total: u64 = resources.iter().map(|&v| u64::from(v)).sum();
        let ms = u64::from(r.time_base_s) * 1000 + total * 1000 / u64::from(r.time_per_point_div);
        u32::try_from(ms).unwrap_or(u32::MAX)
    }

    /// Price and build time of a structure (`entry` and `combat` for
    /// fortifications from the catalog).
    pub fn structure_cost(&self, id: &str, catalog: &Catalog, combat: &CombatType) -> Option<Cost> {
        match self.structure(id)? {
            StructureRule::Building(b) => Some(Cost {
                resources: b.cost,
                time_ms: b.build_s.saturating_mul(1000),
            }),
            StructureRule::Fortification(f) => {
                let formula = self.unit_cost(catalog.get(id)?, combat);
                let resources = f.cost.unwrap_or(formula.resources);
                Some(Cost {
                    resources,
                    time_ms: f
                        .build_s
                        .map_or(self.time_ms(resources), |s| s.saturating_mul(1000)),
                })
            }
        }
    }

    /// Catalog ids a production list makes for `faction`.
    pub fn produced(&self, list: &str, faction: &str) -> &[String] {
        self.produce
            .get(list)
            .and_then(|f| f.get(faction))
            .map_or(&[], Vec::as_slice)
    }

    /// Every unit type a match with these factions needs for the economy:
    /// the base buildings, the faction's fortifications, then what their
    /// buildings produce. Types already in `have` are left out.
    pub fn unit_types(
        &self,
        catalog: &Catalog,
        scale: &Scale,
        factions: &[&str],
        have: &[UnitType],
    ) -> Result<Vec<UnitType>, DataError> {
        let mut out: Vec<UnitType> = Vec::new();
        let missing =
            |out: &[UnitType], name: &str| !have.iter().chain(out).any(|t| t.name == name);
        for b in &self.buildings {
            if missing(&out, &b.id) {
                out.push(UnitType::structure(
                    &b.id,
                    b.length_mm,
                    b.width_mm,
                    b.height_mm,
                    b.armor_mm,
                    scale,
                ));
            }
        }
        for faction in factions {
            for f in &self.fortifications {
                for id in f.ids.iter().filter(|i| faction_of(i) == Some(faction)) {
                    if missing(&out, id) {
                        let entry = catalog
                            .get(id)
                            .ok_or_else(|| DataError(format!("unknown catalog id '{id}'")))?;
                        out.push(UnitType::fixed_from_catalog(entry, scale, f.armor_mm));
                    }
                }
            }
        }
        for faction in factions {
            for b in &self.buildings {
                let ids: Vec<&str> = self
                    .produced(&b.produces, faction)
                    .iter()
                    .map(String::as_str)
                    .filter(|id| missing(&out, id))
                    .collect();
                out.extend(unit_types_from_catalog(catalog, scale, &ids)?);
            }
        }
        Ok(out)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let res = |h: &mut StableHasher, r: &Resources| {
            for &v in r {
                h.write_u32(v);
            }
        };
        let text = |h: &mut StableHasher, s: &str| {
            h.write_bytes(s.as_bytes());
            h.write_u8(0);
        };
        res(h, &self.account.start);
        let c = &self.construction;
        for v in [c.max_sites, c.start_hp_pct, c.max_grade_pct, c.clearance_m] {
            h.write_u32(v);
        }
        let p = &self.production;
        for v in [p.max_queue, p.max_units, p.rally_m] {
            h.write_u32(v);
        }
        text(h, &self.supply.marker_prefix);
        h.write_u32(self.supply.radius_m);
        h.write_u32(self.supply.capture_s);
        res(h, &self.supply.income_per_min);
        let u = &self.unit_cost;
        for v in [
            u.manpower_per_soldier,
            u.fuel_per_tonne,
            u.ammo_base,
            u.ammo_caliber_div,
            u.time_base_s,
            u.time_per_point_div,
        ] {
            h.write_u32(v);
        }
        h.write_u64(self.buildings.len() as u64);
        for b in &self.buildings {
            text(h, &b.id);
            for v in [b.length_mm, b.width_mm, b.height_mm, b.hp] {
                h.write_u32(v);
            }
            h.write_u32(u32::from(b.armor_mm));
            res(h, &b.cost);
            h.write_u32(b.build_s);
            h.write_u32(b.build_radius_m);
            res(h, &b.income_per_min);
            h.write_u32(b.max_count);
            h.write_u64(b.requires.len() as u64);
            for r in &b.requires {
                text(h, r);
            }
            text(h, &b.produces);
        }
        h.write_u64(self.fortifications.len() as u64);
        for f in &self.fortifications {
            h.write_u64(f.ids.len() as u64);
            for id in &f.ids {
                text(h, id);
            }
            h.write_u32(f.hp);
            h.write_u32(u32::from(f.armor_mm));
            match f.cost {
                None => h.write_u8(0),
                Some(c) => {
                    h.write_u8(1);
                    res(h, &c);
                }
            }
            h.write_u32(f.build_s.unwrap_or(u32::MAX));
            h.write_u64(f.requires.len() as u64);
            for r in &f.requires {
                text(h, r);
            }
        }
        for (list, factions) in &self.produce {
            text(h, list);
            for (faction, ids) in factions {
                text(h, faction);
                h.write_u64(ids.len() as u64);
                for id in ids {
                    text(h, id);
                }
            }
        }
    }
}

/// Faction prefix of a catalog id (`de_pz4_h` -> `de`); `None` for neutral
/// and base ids.
pub fn faction_of(id: &str) -> Option<&'static str> {
    ["de", "su", "us"]
        .into_iter()
        .find(|f| id.len() > f.len() && id.starts_with(f) && id.as_bytes()[f.len()] == b'_')
}

/// The catalog compiled into the simulation, parsed once (unit costs need
/// crew and squad sizes, which [`UnitType`] does not carry).
pub fn builtin_catalog() -> Option<&'static Catalog> {
    static CATALOG: OnceLock<Option<Catalog>> = OnceLock::new();
    CATALOG.get_or_init(|| Catalog::builtin().ok()).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> EconomyRules {
        EconomyRules::builtin().expect("built-in economy rules are valid")
    }

    #[test]
    fn builtin_rules_parse() {
        let r = rules();
        assert_eq!(r.account.start, [600, 600, 400]);
        assert_eq!(r.buildings.len(), 4);
        assert!(matches!(
            r.structure("base_hq"),
            Some(StructureRule::Building(b)) if b.build_radius_m == 120
        ));
        assert!(matches!(
            r.structure("su_bunker_dzot"),
            Some(StructureRule::Fortification(_))
        ));
        assert_eq!(r.produced("vehicles", "de")[2], "de_pz4_h");
        assert!(r.produced("vehicles", "xx").is_empty());
    }

    #[test]
    fn unknown_fields_and_bad_references_are_errors() {
        let bad = BUILTIN_ECONOMY.replace("max_sites = 2", "max_sites = 2\nmax_site = 3");
        assert!(EconomyRules::parse(&bad).is_err());
        let bad = BUILTIN_ECONOMY.replace(
            "requires = [\"base_hq\", \"base_depot\"]",
            "requires = [\"base_nothing\"]",
        );
        assert!(EconomyRules::parse(&bad).is_err());
    }

    #[test]
    fn unit_costs_follow_the_formula() {
        let r = rules();
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let cost = |id: &str| {
            let e = catalog.get(id).unwrap();
            r.unit_cost(e, &CombatType::from_catalog(e, &scale))
        };
        let pz4 = cost("de_pz4_h");
        let tiger = cost("de_tiger1_e");
        let squad = cost("de_inf_rifle_squad");
        // Fuel by mass, manpower by crew.
        assert_eq!(pz4.resources[0], 25_000 * 4 / 1000);
        assert_eq!(pz4.resources[2], 6 * 5);
        assert_eq!(squad.resources[0], 0);
        // A stronger unit is never cheaper and takes longer.
        for i in 0..3 {
            assert!(tiger.resources[i] >= pz4.resources[i]);
        }
        assert!(tiger.time_ms > pz4.time_ms && pz4.time_ms > squad.time_ms);
    }

    #[test]
    fn economy_types_for_a_faction() {
        let r = rules();
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types = r.unit_types(&catalog, &scale, &["de"], &[]).unwrap();
        let names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names[..4],
            ["base_hq", "base_barracks", "base_depot", "base_factory"]
        );
        assert!(names.contains(&"de_bunker_tobruk") && names.contains(&"de_pak40"));
        assert!(names.contains(&"de_tiger1_e") && !names.contains(&"su_is2"));
        assert!(types.iter().take(6).all(|t| !t.mobile()));
        assert!(types[6..].iter().all(UnitType::mobile));
        assert_eq!(faction_of("de_pz4_h"), Some("de"));
        assert_eq!(faction_of("base_hq"), None);
    }
}
