//! Infantry tactics rules (`data/tactics.toml`, prompt 047): fire teams and
//! bounds, suppression, close firefights, blind fire, mortar teams and
//! weapon jams. The update itself lives in `sim/tactics.rs`. See
//! docs/INFANTRY.md "Tactics (prompt 047)".

use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The tactics rules compiled into the simulation.
pub const BUILTIN_TACTICS: &str = include_str!("../../../data/tactics.toml");

/// Highest suppression of a squad.
pub const MAX_SUPPRESSION: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireTeamRules {
    pub min_size: u32,
    pub bound_min_mm: u32,
    pub bound_max_mm: u32,
    pub sample_mm: u32,
    pub cover_radius_mm: u32,
    pub settle_ms: u32,
    pub timeout_ms: u32,
    pub overwatch_ms: u32,
    pub contact_ms: u32,
    pub contact_range_pct: u32,
    pub advance_at: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuppressionRules {
    pub near_miss: u32,
    pub heavy_near_miss: u32,
    pub heavy_caliber_mm_x10: u32,
    pub hit: u32,
    pub blast: u32,
    pub prone_pct: u32,
    pub recover_delay_ms: u32,
    pub recover_per_s: u32,
    pub pinned_at: u32,
    pub unpinned_at: u32,
    pub pinned_fire_pct: u32,
    pub dispersion_pct: u32,
    pub speed_pct: u32,
}

impl SuppressionRules {
    /// Dispersion of a squad with `suppression` in percent of the normal
    /// one (100 without suppression).
    pub const fn dispersion_factor_pct(&self, suppression: u32) -> u32 {
        100 + self.dispersion_pct * min(suppression, MAX_SUPPRESSION) / 100
    }

    /// Speed of a squad with `suppression` in percent of its stance speed.
    pub const fn speed_factor_pct(&self, suppression: u32) -> u32 {
        100 - min(self.speed_pct * min(suppression, MAX_SUPPRESSION) / 100, 99)
    }
}

const fn min(a: u32, b: u32) -> u32 {
    if a < b { a } else { b }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloseRules {
    pub prone_within_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlindFireRules {
    pub remember_ms: u32,
    pub fire_pct: u32,
    pub dispersion_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MortarRules {
    pub spotted_targets: u32,
    pub halt_on_attack_move: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JamRules {
    pub enabled: u32,
    pub per_100k: u32,
    pub clear_ms: u32,
}

/// All tactics rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TacticsRules {
    pub fire_teams: FireTeamRules,
    pub suppression: SuppressionRules,
    pub close: CloseRules,
    pub blind_fire: BlindFireRules,
    pub mortar: MortarRules,
    pub jams: JamRules,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TacticsError(pub String);

impl fmt::Display for TacticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid tactics rules: {}", self.0)
    }
}

impl std::error::Error for TacticsError {}

impl TacticsRules {
    pub fn parse(text: &str) -> Result<Self, TacticsError> {
        let rules: Self = toml::from_str(text).map_err(|e| TacticsError(e.to_string()))?;
        let check = |name: &str, value: u32, lo: u32, hi: u32| {
            if (lo..=hi).contains(&value) {
                Ok(())
            } else {
                Err(TacticsError(format!(
                    "{name} = {value} outside {lo}..={hi}"
                )))
            }
        };
        let f = &rules.fire_teams;
        check("fire_teams.min_size", f.min_size, 2, 100)?;
        check("fire_teams.bound_min_mm", f.bound_min_mm, 1000, 200_000)?;
        check(
            "fire_teams.bound_max_mm",
            f.bound_max_mm,
            f.bound_min_mm,
            200_000,
        )?;
        check("fire_teams.sample_mm", f.sample_mm, 500, 50_000)?;
        check("fire_teams.cover_radius_mm", f.cover_radius_mm, 0, 50_000)?;
        check("fire_teams.settle_ms", f.settle_ms, 0, 60_000)?;
        check("fire_teams.timeout_ms", f.timeout_ms, 1000, 600_000)?;
        check("fire_teams.overwatch_ms", f.overwatch_ms, 0, 60_000)?;
        check("fire_teams.contact_ms", f.contact_ms, 0, 600_000)?;
        check("fire_teams.contact_range_pct", f.contact_range_pct, 0, 100)?;
        check("fire_teams.advance_at", f.advance_at, 0, MAX_SUPPRESSION)?;
        let s = &rules.suppression;
        for (name, v) in [
            ("near_miss", s.near_miss),
            ("heavy_near_miss", s.heavy_near_miss),
            ("hit", s.hit),
            ("blast", s.blast),
            ("recover_per_s", s.recover_per_s),
        ] {
            check(&format!("suppression.{name}"), v, 0, MAX_SUPPRESSION)?;
        }
        check(
            "suppression.heavy_caliber_mm_x10",
            s.heavy_caliber_mm_x10,
            1,
            10_000,
        )?;
        check(
            "suppression.recover_delay_ms",
            s.recover_delay_ms,
            0,
            600_000,
        )?;
        check("suppression.prone_pct", s.prone_pct, 0, 100)?;
        check("suppression.pinned_at", s.pinned_at, 1, MAX_SUPPRESSION)?;
        check("suppression.unpinned_at", s.unpinned_at, 0, s.pinned_at)?;
        check("suppression.pinned_fire_pct", s.pinned_fire_pct, 0, 100)?;
        check("suppression.dispersion_pct", s.dispersion_pct, 0, 1000)?;
        check("suppression.speed_pct", s.speed_pct, 0, 99)?;
        check(
            "close.prone_within_mm",
            rules.close.prone_within_mm,
            0,
            1_000_000,
        )?;
        let b = &rules.blind_fire;
        check("blind_fire.remember_ms", b.remember_ms, 0, 600_000)?;
        check("blind_fire.fire_pct", b.fire_pct, 0, 100)?;
        check("blind_fire.dispersion_pct", b.dispersion_pct, 100, 2000)?;
        let m = &rules.mortar;
        check("mortar.spotted_targets", m.spotted_targets, 0, 1)?;
        check("mortar.halt_on_attack_move", m.halt_on_attack_move, 0, 1)?;
        let j = &rules.jams;
        check("jams.enabled", j.enabled, 0, 1)?;
        check("jams.per_100k", j.per_100k, 0, 100_000)?;
        check("jams.clear_ms", j.clear_ms, 0, 600_000)?;
        Ok(rules)
    }

    /// The rules from `data/tactics.toml`, compiled in.
    pub fn builtin() -> Result<Self, TacticsError> {
        Self::parse(BUILTIN_TACTICS)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let f = &self.fire_teams;
        let s = &self.suppression;
        let b = &self.blind_fire;
        let m = &self.mortar;
        let j = &self.jams;
        for v in [
            f.min_size,
            f.bound_min_mm,
            f.bound_max_mm,
            f.sample_mm,
            f.cover_radius_mm,
            f.settle_ms,
            f.timeout_ms,
            f.overwatch_ms,
            f.contact_ms,
            f.contact_range_pct,
            f.advance_at,
            s.near_miss,
            s.heavy_near_miss,
            s.heavy_caliber_mm_x10,
            s.hit,
            s.blast,
            s.prone_pct,
            s.recover_delay_ms,
            s.recover_per_s,
            s.pinned_at,
            s.unpinned_at,
            s.pinned_fire_pct,
            s.dispersion_pct,
            s.speed_pct,
            self.close.prone_within_mm,
            b.remember_ms,
            b.fire_pct,
            b.dispersion_pct,
            m.spotted_targets,
            m.halt_on_attack_move,
            j.enabled,
            j.per_100k,
            j.clear_ms,
        ] {
            h.write_u32(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = TacticsRules::builtin().unwrap();
        assert_eq!(rules.fire_teams.min_size, 6);
        assert_eq!(rules.suppression.dispersion_factor_pct(0), 100);
        assert_eq!(rules.suppression.dispersion_factor_pct(100), 250);
        assert_eq!(rules.suppression.speed_factor_pct(100), 50);
        assert_eq!(rules.suppression.speed_factor_pct(500), 50);
        let bad = BUILTIN_TACTICS.replace("unpinned_at = 30", "unpinned_at = 70");
        assert!(TacticsRules::parse(&bad).is_err());
        let bad = BUILTIN_TACTICS.replace("enabled = 1", "enabled = 2");
        assert!(TacticsRules::parse(&bad).is_err());
        let unknown = format!("{BUILTIN_TACTICS}\n[extra]\nx = 1\n");
        assert!(TacticsRules::parse(&unknown).is_err());
    }
}
