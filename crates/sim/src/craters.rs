//! Terrain deformation (prompt 021): the crater rules and the public types.
//!
//! `data/craters.toml` says which ground hits leave a crater, how large it
//! is and how much cover it gives. Applying craters to the map, the path
//! grid and the vision grid: `crate::sim::craters`.

use crate::hash::StableHasher;
use crate::infantry::Stance;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_CRATERS: &str = include_str!("../../../data/craters.toml");

#[derive(Debug, PartialEq, Eq)]
pub struct CraterError(pub String);

impl fmt::Display for CraterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid crater rules: {}", self.0)
    }
}

impl std::error::Error for CraterError {}

/// Size of craters and what they do to the ground.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CraterShape {
    pub min_caliber_mm_x10: u32,
    pub radius_mm_per_mm: u32,
    pub min_radius_mm: u32,
    pub depth_mm_per_mm: u32,
    pub rim_pct: u32,
    pub rim_width_pct: u32,
    pub max_depth_mm: u32,
    pub max_rise_mm: u32,
    /// Terrain type name of the bowl.
    pub terrain: String,
    /// Terrain type names that never change (and take no craters).
    pub keep: Vec<String>,
    pub max_per_tick: u32,
}

/// Cover of soldiers in craters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CraterCover {
    pub min_depth_mm: u32,
    pub stand_pct: u32,
    pub kneel_pct: u32,
    pub prone_pct: u32,
}

impl CraterCover {
    /// Chance in percent that the crater catches a hit on a soldier in
    /// `stance`.
    pub const fn pct(&self, stance: Stance) -> u32 {
        match stance {
            Stance::Stand => self.stand_pct,
            Stance::Kneel => self.kneel_pct,
            Stance::Prone => self.prone_pct,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    crater: CraterShape,
    cover: CraterCover,
}

/// Rules of `data/craters.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraterRules {
    pub crater: CraterShape,
    pub cover: CraterCover,
}

/// Bowl radius, depth and outer rim radius of one crater, millimeters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraterSize {
    pub radius_mm: i64,
    pub depth_mm: i64,
    pub rim_mm: i64,
    /// Where the rim ends (from the center).
    pub outer_mm: i64,
}

impl CraterSize {
    /// Height change in millimeters at the squared distance `d2` (mm²)
    /// from the center: a parabolic bowl from `-depth` in the center up to
    /// `+rim` at its edge, then the rim falling linearly to 0 at
    /// `outer_mm`. Integer only (floor division towards zero).
    pub fn profile(&self, d2: i64) -> i64 {
        let r2 = self.radius_mm * self.radius_mm;
        if d2 < r2 {
            -self.depth_mm + (self.depth_mm + self.rim_mm) * d2 / r2
        } else if d2 < self.outer_mm * self.outer_mm && self.outer_mm > self.radius_mm {
            let d = i64::try_from(d2.unsigned_abs().isqrt()).unwrap_or(i64::MAX);
            self.rim_mm * (self.outer_mm - d) / (self.outer_mm - self.radius_mm)
        } else {
            0
        }
    }
}

impl CraterRules {
    pub fn parse(text: &str) -> Result<Self, CraterError> {
        let file: File = toml::from_str(text).map_err(|e| CraterError(e.to_string()))?;
        let c = &file.crater;
        let err = |what: &str| Err(CraterError(what.to_owned()));
        if c.min_radius_mm == 0 || c.min_radius_mm > 50_000 {
            return err("min_radius_mm must be 1..=50000");
        }
        if c.radius_mm_per_mm > 1000 || c.depth_mm_per_mm > 1000 {
            return err("radius / depth per mm above 1000");
        }
        if c.rim_pct > 100 || c.rim_width_pct > 200 {
            return err("rim_pct above 100 or rim_width_pct above 200");
        }
        if c.max_depth_mm > 20_000 || c.max_rise_mm > 20_000 {
            return err("max_depth_mm / max_rise_mm above 20000");
        }
        if c.max_per_tick == 0 {
            return err("max_per_tick must be at least 1");
        }
        let v = &file.cover;
        if v.stand_pct > 100 || v.kneel_pct > 100 || v.prone_pct > 100 {
            return err("cover percentages above 100");
        }
        Ok(Self {
            crater: file.crater,
            cover: file.cover,
        })
    }

    /// The rules from `data/craters.toml`.
    pub fn builtin() -> Result<Self, CraterError> {
        Self::parse(BUILTIN_CRATERS)
    }

    /// Whether a ground hit of this caliber (tenths of a mm) leaves a crater.
    pub const fn makes_crater(&self, caliber_mm_x10: u32) -> bool {
        caliber_mm_x10 >= self.crater.min_caliber_mm_x10
    }

    /// Size of the crater of a round of `caliber_mm_x10`.
    pub fn size(&self, caliber_mm_x10: u32) -> CraterSize {
        let c = &self.crater;
        let caliber = i64::from(caliber_mm_x10);
        let radius_mm =
            (caliber * i64::from(c.radius_mm_per_mm) / 10).max(i64::from(c.min_radius_mm));
        let depth_mm = caliber * i64::from(c.depth_mm_per_mm) / 10;
        let rim_mm = depth_mm * i64::from(c.rim_pct) / 100;
        let outer_mm = radius_mm + radius_mm * i64::from(c.rim_width_pct) / 100;
        CraterSize {
            radius_mm,
            depth_mm,
            rim_mm,
            outer_mm,
        }
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let c = &self.crater;
        for v in [
            c.min_caliber_mm_x10,
            c.radius_mm_per_mm,
            c.min_radius_mm,
            c.depth_mm_per_mm,
            c.rim_pct,
            c.rim_width_pct,
            c.max_depth_mm,
            c.max_rise_mm,
            c.max_per_tick,
        ] {
            h.write_u32(v);
        }
        h.write_bytes(c.terrain.as_bytes());
        h.write_u8(0);
        h.write_u64(c.keep.len() as u64);
        for k in &c.keep {
            h.write_bytes(k.as_bytes());
            h.write_u8(0);
        }
        let v = &self.cover;
        for x in [v.min_depth_mm, v.stand_pct, v.kneel_pct, v.prone_pct] {
            h.write_u32(x);
        }
    }
}

/// A crater was made during the last tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraterEvent {
    /// Where the round struck the ground.
    pub at: Vec2,
    pub caliber_mm_x10: u32,
    /// Bowl radius and depth in meters (the rim reaches half a radius
    /// further with the built-in rules).
    pub radius: Fixed,
    pub depth: Fixed,
    /// Map samples that may have changed: `(i0, j0, i1, j1)`, inclusive.
    pub samples: (u32, u32, u32, u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = CraterRules::builtin().unwrap();
        assert!(rules.makes_crater(750));
        assert!(!rules.makes_crater(200));
        let size = rules.size(750);
        assert_eq!(size.radius_mm, 2000);
        assert_eq!(size.depth_mm, 375);
        assert_eq!(size.rim_mm, 93);
        assert_eq!(size.outer_mm, 3000);
    }

    #[test]
    fn profile_is_a_bowl_with_a_rim() {
        let size = CraterSize {
            radius_mm: 2000,
            depth_mm: 400,
            rim_mm: 100,
            outer_mm: 3000,
        };
        assert_eq!(size.profile(0), -400);
        assert_eq!(size.profile(1000 * 1000), -400 + 500 / 4);
        assert_eq!(size.profile(2000 * 2000), 100);
        assert_eq!(size.profile(2500 * 2500), 50);
        assert_eq!(size.profile(3000 * 3000), 0);
        assert_eq!(size.profile(9_000_000_000), 0);
    }

    #[test]
    fn rules_reject_bad_values() {
        let good = BUILTIN_CRATERS;
        for (from, to) in [
            ("max_per_tick = 16", "max_per_tick = 0"),
            ("min_radius_mm = 2000", "min_radius_mm = 0"),
            ("prone_pct = 50", "prone_pct = 150"),
            ("rim_pct = 25", "rim_pct = 125"),
        ] {
            let text = good.replace(from, to);
            assert_ne!(text, good);
            assert!(CraterRules::parse(&text).is_err(), "{to}");
        }
    }
}
