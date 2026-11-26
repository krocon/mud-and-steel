//! Experience and ranks (prompt 050): the rules of `data/experience.toml`.
//! The update itself (points from the tick's combat events, bonuses where
//! units fire and get suppressed): `crate::sim::experience`. See
//! docs/GAME_DESIGN.md "Experience and ranks".

use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_EXPERIENCE: &str = include_str!("../../../data/experience.toml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperienceError(pub String);

impl fmt::Display for ExperienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid experience rules: {}", self.0)
    }
}

impl std::error::Error for ExperienceError {}

/// How units earn experience points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GainRules {
    /// Points for all the damage it takes to destroy one enemy unit.
    pub damage_xp: u32,
    /// Extra points for the hit that destroys an enemy unit.
    pub kill_xp: u32,
    /// Points for surviving a tick with a damaging enemy hit.
    pub survived_hit_xp: u32,
    /// Points per game second a squad is suppressed and alive.
    pub suppressed_xp_per_s: u32,
}

/// One rank and its bonuses (percent of the recruit value).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RankRules {
    pub name: String,
    /// Experience points from which a unit holds this rank.
    pub xp: u32,
    pub dispersion_pct: u32,
    pub reload_pct: u32,
    pub suppression_pct: u32,
}

/// Rules of `data/experience.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperienceRules {
    pub gain: GainRules,
    #[serde(rename = "rank")]
    pub ranks: Vec<RankRules>,
}

impl ExperienceRules {
    pub fn parse(text: &str) -> Result<Self, ExperienceError> {
        let rules: Self = toml::from_str(text).map_err(|e| ExperienceError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(ExperienceError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let g = &rules.gain;
        check("gain.damage_xp", g.damage_xp, 0, 100_000)?;
        check("gain.kill_xp", g.kill_xp, 0, 100_000)?;
        check("gain.survived_hit_xp", g.survived_hit_xp, 0, 100_000)?;
        check(
            "gain.suppressed_xp_per_s",
            g.suppressed_xp_per_s,
            0,
            100_000,
        )?;
        if rules.ranks.is_empty() || rules.ranks.len() > usize::from(u8::MAX) {
            return Err(ExperienceError("needs 1..=255 ranks".into()));
        }
        let first = &rules.ranks[0];
        if first.xp != 0
            || first.dispersion_pct != 100
            || first.reload_pct != 100
            || first.suppression_pct != 100
        {
            return Err(ExperienceError(
                "the first rank starts at 0 XP without bonuses".into(),
            ));
        }
        for pair in rules.ranks.windows(2) {
            if pair[1].xp <= pair[0].xp {
                return Err(ExperienceError(format!(
                    "rank '{}' needs more XP than '{}'",
                    pair[1].name, pair[0].name
                )));
            }
        }
        for r in &rules.ranks {
            if r.name.trim().is_empty() {
                return Err(ExperienceError("a rank needs a name".into()));
            }
            // Bonuses stay moderate: at most halving a value.
            check("rank.dispersion_pct", r.dispersion_pct, 50, 100)?;
            check("rank.reload_pct", r.reload_pct, 50, 100)?;
            check("rank.suppression_pct", r.suppression_pct, 25, 100)?;
        }
        Ok(rules)
    }

    /// The rules from `data/experience.toml`, compiled in.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static ExperienceError> {
        static CACHE: std::sync::OnceLock<Result<ExperienceRules, ExperienceError>> =
            std::sync::OnceLock::new();
        CACHE
            .get_or_init(|| Self::parse(BUILTIN_EXPERIENCE))
            .as_ref()
    }

    pub fn builtin() -> Result<Self, ExperienceError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    /// Rank index of a unit with `xp` points (0 = the first rank).
    pub fn rank_of(&self, xp: u32) -> u8 {
        let n = self.ranks.iter().take_while(|r| r.xp <= xp).count();
        u8::try_from(n.saturating_sub(1)).unwrap_or(u8::MAX)
    }

    /// The rules of rank `rank` (the highest one beyond the last).
    pub fn rank(&self, rank: u8) -> &RankRules {
        let last = self.ranks.len() - 1;
        &self.ranks[usize::from(rank).min(last)]
    }

    /// Points needed for the rank after `rank`; `None` at the highest.
    pub fn next_xp(&self, rank: u8) -> Option<u32> {
        self.ranks.get(usize::from(rank) + 1).map(|r| r.xp)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0x50);
        let g = &self.gain;
        for v in [
            g.damage_xp,
            g.kill_xp,
            g.survived_hit_xp,
            g.suppressed_xp_per_s,
        ] {
            h.write_u32(v);
        }
        h.write_u64(self.ranks.len() as u64);
        for r in &self.ranks {
            h.write_bytes(r.name.as_bytes());
            for v in [r.xp, r.dispersion_pct, r.reload_pct, r.suppression_pct] {
                h.write_u32(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = ExperienceRules::builtin().unwrap();
        assert_eq!(rules.ranks.len(), 4);
        assert_eq!(rules.rank_of(0), 0);
        assert_eq!(rules.rank_of(99), 0);
        assert_eq!(rules.rank_of(100), 1);
        assert_eq!(rules.rank_of(300), 2);
        assert_eq!(rules.rank_of(u32::MAX), 3);
        assert_eq!(rules.next_xp(0), Some(100));
        assert_eq!(rules.next_xp(3), None);
        assert_eq!(rules.rank(9).name, "Elite");
    }

    #[test]
    fn bad_rules_fail() {
        let bad = |from: &str, to: &str| {
            assert!(
                ExperienceRules::parse(&BUILTIN_EXPERIENCE.replacen(from, to, 1)).is_err(),
                "{from} -> {to} accepted"
            );
        };
        bad("xp = 0", "xp = 5");
        bad("xp = 300", "xp = 50");
        bad("dispersion_pct = 76", "dispersion_pct = 20");
        bad("[gain]", "[gain]\nbonus = 1");
    }
}
