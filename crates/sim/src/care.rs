//! Wounds, buddy aid, medics and bailed-out crews (prompt 049): the rules
//! of `data/care.toml` and the crew unit types a match needs. The update
//! itself: `crate::sim::care`. See docs/INFANTRY.md "Wounds and buddy aid".

use crate::catalog::{Catalog, Category, Group};
use crate::data::{DataError, UnitType};
use crate::hash::StableHasher;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_CARE: &str = include_str!("../../../data/care.toml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CareError(pub String);

impl fmt::Display for CareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid care rules: {}", self.0)
    }
}

impl std::error::Error for CareError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WoundRules {
    pub heavy_pct: u32,
    pub bandage_ms: u32,
    pub bleed_ms: u32,
    pub limp_ms: u32,
    pub limp_speed_pct: u32,
    pub carry_speed_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuddyRules {
    pub drag_mm: u32,
    pub drag_speed_pct: u32,
    pub help_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MedicRules {
    pub search_mm: u32,
    pub reach_mm: u32,
    pub stretcher_men: u32,
    pub stretcher_speed_pct: u32,
    pub treat_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrewRules {
    pub survive_pct: u32,
    pub withdraw_mm: u32,
}

/// Rules of `data/care.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CareRules {
    pub wound: WoundRules,
    pub buddy: BuddyRules,
    pub medic: MedicRules,
    pub crew: CrewRules,
}

impl CareRules {
    pub fn parse(text: &str) -> Result<Self, CareError> {
        let r: Self = toml::from_str(text).map_err(|e| CareError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(CareError(format!("{name} = {value} outside {min}..={max}")))
            }
        };
        let w = &r.wound;
        check("wound.heavy_pct", w.heavy_pct, 0, 100)?;
        check("wound.bandage_ms", w.bandage_ms, 0, 600_000)?;
        check("wound.bleed_ms", w.bleed_ms, 1000, 3_600_000)?;
        check("wound.limp_ms", w.limp_ms, 0, 3_600_000)?;
        check("wound.limp_speed_pct", w.limp_speed_pct, 10, 100)?;
        check("wound.carry_speed_pct", w.carry_speed_pct, 10, 100)?;
        let b = &r.buddy;
        check("buddy.drag_mm", b.drag_mm, 0, 100_000)?;
        check("buddy.drag_speed_pct", b.drag_speed_pct, 5, 100)?;
        check("buddy.help_mm", b.help_mm, 0, 50_000)?;
        let m = &r.medic;
        check("medic.search_mm", m.search_mm, 0, 500_000)?;
        check("medic.reach_mm", m.reach_mm, 500, 20_000)?;
        check("medic.stretcher_men", m.stretcher_men, 1, 15)?;
        check("medic.stretcher_speed_pct", m.stretcher_speed_pct, 10, 100)?;
        check("medic.treat_ms", m.treat_ms, 0, 600_000)?;
        check("crew.survive_pct", r.crew.survive_pct, 0, 100)?;
        check("crew.withdraw_mm", r.crew.withdraw_mm, 0, 2_000_000)?;
        Ok(r)
    }

    /// The rules from `data/care.toml`, compiled in.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static CareError> {
        static CACHE: std::sync::OnceLock<Result<CareRules, CareError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_CARE)).as_ref()
    }

    pub fn builtin() -> Result<Self, CareError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0x49);
        let (w, b, m) = (&self.wound, &self.buddy, &self.medic);
        for v in [
            w.heavy_pct,
            w.bandage_ms,
            w.bleed_ms,
            w.limp_ms,
            w.limp_speed_pct,
            w.carry_speed_pct,
            b.drag_mm,
            b.drag_speed_pct,
            b.help_mm,
            m.search_mm,
            m.reach_mm,
            m.stretcher_men,
            m.stretcher_speed_pct,
            m.treat_ms,
            self.crew.survive_pct,
            self.crew.withdraw_mm,
        ] {
            h.write_u32(v);
        }
    }
}

/// Catalog id of the tank crew squad of `faction` (`de`, `su`, `us`).
pub fn crew_id(faction: &str) -> String {
    format!("{faction}_inf_tank_crew")
}

/// Whether a unit type's crew bails out when it is destroyed: vehicles
/// (tanks, tank destroyers, assault guns, halftracks, trucks, armored cars)
/// with a catalog crew.
pub fn bails_out(t: &UnitType) -> bool {
    t.transport.crew > 0
        && t.combat
            .category
            .is_some_and(|c| c.group() == Group::Vehicle)
}

/// Whether a unit type is a bailed-out crew squad.
pub fn is_crew(t: &UnitType) -> bool {
    t.combat.category == Some(Category::InfCrew)
}

/// The crew squad types a match with `have` needs: one per faction that
/// has a vehicle whose crew can bail out. Types already in `have` are left
/// out; factions without a crew entry in the catalog get none.
pub fn crew_types(
    catalog: &Catalog,
    scale: &Scale,
    have: &[UnitType],
) -> Result<Vec<UnitType>, DataError> {
    let mut ids: Vec<String> = Vec::new();
    for t in have.iter().filter(|t| bails_out(t)) {
        let Some(faction) = crate::economy::faction_of(&t.name) else {
            continue;
        };
        let id = crew_id(faction);
        if catalog.get(&id).is_some() && !ids.contains(&id) && !have.iter().any(|h| h.name == id) {
            ids.push(id);
        }
    }
    let names: Vec<&str> = ids.iter().map(String::as_str).collect();
    crate::data::unit_types_from_catalog(catalog, scale, &names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = CareRules::builtin().unwrap();
        assert_eq!(rules.medic.stretcher_men, 2);
        assert!(rules.wound.heavy_pct <= 100);
    }

    #[test]
    fn bad_rules_fail() {
        let bad = |from: &str, to: &str| {
            assert!(
                CareRules::parse(&BUILTIN_CARE.replacen(from, to, 1)).is_err(),
                "{from} -> {to} accepted"
            );
        };
        bad("heavy_pct = 40", "heavy_pct = 140");
        bad("[crew]", "[crew]\nextra = 1");
        bad("reach_mm = 3000", "reach_mm = 10");
    }

    #[test]
    fn every_faction_has_a_crew_squad() {
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let tanks = crate::data::unit_types_from_catalog(
            &catalog,
            &scale,
            &["de_pz4_h", "us_m4a3_75", "su_t34_76_1943"],
        )
        .unwrap();
        assert!(tanks.iter().all(bails_out));
        let crews = crew_types(&catalog, &scale, &tanks).unwrap();
        assert_eq!(crews.len(), 3);
        assert!(crews.iter().all(is_crew));
        assert!(crews.iter().all(|c| c.combat.squad_size >= 4));
    }
}
