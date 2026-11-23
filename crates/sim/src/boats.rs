//! Boats on the water (prompt 119): the rules of `data/boats.toml`. The
//! update (landing on a beach, the ramp, backing off, vehicles as cargo,
//! sinking): `crate::sim::boats`. See docs/ARCHITECTURE.md "Boats".

use crate::catalog::{Category, Group};
use crate::hash::StableHasher;
use crate::transport::CarrierRules;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_BOATS: &str = include_str!("../../../data/boats.toml");

#[derive(Debug, PartialEq, Eq)]
pub struct BoatError(pub String);

impl fmt::Display for BoatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid boat rules: {}", self.0)
    }
}

impl std::error::Error for BoatError {}

/// Rules of one boat category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoatKind {
    pub category: Category,
    pub ramp_ms: u32,
    pub retract_ms: u32,
    pub run_in_pct: u32,
    pub vehicle_load_ms: u32,
    pub vehicle_unload_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoatGeneral {
    pub run_in_max_m: u32,
    pub sink_depth_mm: u32,
    pub overload_pct: u32,
    pub paddle_min_pct: u32,
}

/// Rules of `data/boats.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoatRules {
    #[serde(rename = "carrier")]
    pub carriers: Vec<CarrierRules>,
    #[serde(rename = "boat")]
    pub kinds: Vec<BoatKind>,
    pub rules: BoatGeneral,
}

impl BoatRules {
    pub fn parse(text: &str) -> Result<Self, BoatError> {
        let rules: Self = toml::from_str(text).map_err(|e| BoatError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(BoatError(format!("{name} = {value} outside {min}..={max}")))
            }
        };
        let boat = |c: Category| c.group() == Group::Vehicle && c.is_boat();
        for c in &rules.carriers {
            if !boat(c.category) {
                return Err(BoatError(format!(
                    "carrier category '{}' is no boat",
                    c.category.code()
                )));
            }
            check("carrier.climb_ms", c.climb_ms, 0, 600_000)?;
            check("carrier.disembark_ms", c.disembark_ms, 0, 600_000)?;
            check("carrier.hit_pct", c.hit_pct, 0, 100)?;
            check("carrier.destroyed_pct", c.destroyed_pct, 0, 100)?;
            if c.riders != 0 || c.tows {
                return Err(BoatError("boats carry on seats and tow nothing".into()));
            }
        }
        for k in &rules.kinds {
            if !boat(k.category) {
                return Err(BoatError(format!(
                    "boat category '{}' is no boat",
                    k.category.code()
                )));
            }
            check("boat.ramp_ms", k.ramp_ms, 0, 600_000)?;
            check("boat.retract_ms", k.retract_ms, 0, 600_000)?;
            check("boat.run_in_pct", k.run_in_pct, 1, 100)?;
            check("boat.vehicle_load_ms", k.vehicle_load_ms, 0, 600_000)?;
            check("boat.vehicle_unload_ms", k.vehicle_unload_ms, 0, 600_000)?;
        }
        for list in [
            rules
                .carriers
                .iter()
                .map(|c| c.category)
                .collect::<Vec<_>>(),
            rules.kinds.iter().map(|k| k.category).collect(),
        ] {
            let mut seen = list.clone();
            seen.sort_unstable();
            seen.dedup();
            if seen.len() != list.len() {
                return Err(BoatError("categories must be unique".into()));
            }
        }
        let r = &rules.rules;
        check("rules.run_in_max_m", r.run_in_max_m, 1, 1000)?;
        check("rules.sink_depth_mm", r.sink_depth_mm, 0, 100_000)?;
        check("rules.overload_pct", r.overload_pct, 0, 100)?;
        check("rules.paddle_min_pct", r.paddle_min_pct, 1, 100)?;
        Ok(rules)
    }

    /// The rules from `data/boats.toml`, compiled in.
    pub fn builtin() -> Result<Self, BoatError> {
        Self::parse(BUILTIN_BOATS)
    }

    /// Rules of a boat category.
    pub fn kind(&self, category: Option<Category>) -> Option<&BoatKind> {
        self.kinds.iter().find(|k| Some(k.category) == category)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.kinds.len() as u64);
        for k in &self.kinds {
            h.write_bytes(k.category.code().as_bytes());
            h.write_u8(0);
            for v in [
                k.ramp_ms,
                k.retract_ms,
                k.run_in_pct,
                k.vehicle_load_ms,
                k.vehicle_unload_ms,
            ] {
                h.write_u32(v);
            }
        }
        let r = &self.rules;
        for v in [
            r.run_in_max_m,
            r.sink_depth_mm,
            r.overload_pct,
            r.paddle_min_pct,
        ] {
            h.write_u32(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = BoatRules::builtin().unwrap();
        assert!(rules.kind(Some(Category::BoatLanding)).is_some());
        assert_eq!(rules.carriers.len(), 2);
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let bad = BUILTIN_BOATS.replace("run_in_pct = 50", "run_in_pct = 0");
        assert!(BoatRules::parse(&bad).is_err());
        let wrong =
            BUILTIN_BOATS.replacen("category = \"boat_landing\"", "category = \"truck\"", 1);
        assert!(BoatRules::parse(&wrong).is_err());
    }
}
