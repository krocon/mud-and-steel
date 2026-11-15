//! Terrain types and slope rules (`data/terrain.toml`).

use crate::hash::StableHasher;
use crate::math::Fixed;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The terrain file compiled into the simulation.
pub const BUILTIN_TERRAIN: &str = include_str!("../../../data/terrain.toml");

/// Which catalog speed a unit uses on a terrain type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeedBasis {
    Road,
    Offroad,
}

/// Properties of one terrain type. Its index in [`TerrainRules::types`] is
/// the pixel value in a map's `terrain.png`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainType {
    pub name: String,
    pub speed: SpeedBasis,
    /// Percent of the basis speed, `1..=200`.
    pub speed_pct: u32,
    pub passable: bool,
    /// `0xRRGGBB`, presentation only.
    pub debug_color: u32,
}

/// How the grade along the driving direction changes the speed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlopeRules {
    /// Speed loss in percent per 100 % uphill grade.
    pub uphill_penalty_pct: u32,
    /// Lower bound of the slope factor, in percent (`1..=100`).
    pub min_speed_pct: u32,
    /// Steeper uphill grades cannot be climbed, in percent.
    pub max_grade_pct: u32,
}

impl SlopeRules {
    /// Speed factor for driving at `grade` (rise per meter, negative
    /// downhill), or `None` if the grade is too steep to climb.
    pub fn factor(&self, grade: Fixed) -> Option<Fixed> {
        let hundred = Fixed::from_num(100);
        if grade > Fixed::from_num(self.max_grade_pct) / hundred {
            return None;
        }
        if grade <= Fixed::ZERO {
            return Some(Fixed::ONE);
        }
        let loss = grade * Fixed::from_num(self.uphill_penalty_pct) / hundred;
        let min = Fixed::from_num(self.min_speed_pct) / hundred;
        Some((Fixed::ONE - loss).max(min))
    }
}

/// All terrain types and the slope rules. Part of [`crate::SimConfig`], so
/// replays carry the rules they were played with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainRules {
    pub slope: SlopeRules,
    pub types: Vec<TerrainType>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TerrainError(pub String);

impl fmt::Display for TerrainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid terrain rules: {}", self.0)
    }
}

impl std::error::Error for TerrainError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainEntry {
    id: u32,
    name: String,
    speed: SpeedBasis,
    speed_pct: u32,
    passable: bool,
    debug_color: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TerrainFile {
    slope: SlopeRules,
    terrain: Vec<TerrainEntry>,
}

impl TerrainRules {
    pub fn parse(text: &str) -> Result<Self, TerrainError> {
        let file: TerrainFile = toml::from_str(text).map_err(|e| TerrainError(e.to_string()))?;
        let slope = file.slope;
        if !(1..=100).contains(&slope.min_speed_pct) {
            return Err(TerrainError("slope.min_speed_pct must be 1..=100".into()));
        }
        if file.terrain.is_empty() || file.terrain.len() > 256 {
            return Err(TerrainError("expected 1 to 256 terrain types".into()));
        }
        let mut types = Vec::with_capacity(file.terrain.len());
        for (index, entry) in (0u32..).zip(file.terrain) {
            if entry.id != index {
                return Err(TerrainError(format!(
                    "terrain '{}' has id {}, expected {index} (ids follow file order)",
                    entry.name, entry.id
                )));
            }
            if !(1..=200).contains(&entry.speed_pct) {
                return Err(TerrainError(format!(
                    "terrain '{}': speed_pct must be 1..=200",
                    entry.name
                )));
            }
            if entry.debug_color > 0xFF_FFFF {
                return Err(TerrainError(format!(
                    "terrain '{}': debug_color must be 0xRRGGBB",
                    entry.name
                )));
            }
            if types.iter().any(|t: &TerrainType| t.name == entry.name) {
                return Err(TerrainError(format!("duplicate terrain '{}'", entry.name)));
            }
            types.push(TerrainType {
                name: entry.name,
                speed: entry.speed,
                speed_pct: entry.speed_pct,
                passable: entry.passable,
                debug_color: entry.debug_color,
            });
        }
        Ok(Self { slope, types })
    }

    /// The rules from `data/terrain.toml`, compiled in.
    pub fn builtin() -> Result<Self, TerrainError> {
        Self::parse(BUILTIN_TERRAIN)
    }

    /// Feeds the game-relevant part of the rules into a state hash.
    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.slope.uphill_penalty_pct);
        h.write_u32(self.slope.min_speed_pct);
        h.write_u32(self.slope.max_grade_pct);
        h.write_u64(self.types.len() as u64);
        for t in &self.types {
            h.write_u8(u8::from(t.speed == SpeedBasis::Road));
            h.write_u32(t.speed_pct);
            h.write_u8(u8::from(t.passable));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct(value: i32) -> Fixed {
        Fixed::from_num(value) / Fixed::from_num(100)
    }

    #[test]
    fn builtin_rules_parse() {
        let rules = TerrainRules::builtin().unwrap();
        assert_eq!(rules.types.len(), 8);
        assert_eq!(rules.types[0].name, "grass");
        assert!(!rules.types[7].passable);
    }

    #[test]
    fn slope_factor_follows_the_rules() {
        let slope = TerrainRules::builtin().unwrap().slope;
        assert_eq!(slope.factor(pct(-30)), Some(Fixed::ONE));
        assert_eq!(slope.factor(Fixed::ZERO), Some(Fixed::ONE));
        // 25 % uphill: 1 - 0.25 * 2 = 0.5.
        assert_eq!(slope.factor(Fixed::ONE / 4), Some(Fixed::ONE / 2));
        // 50 %: clamped to 20 %.
        assert_eq!(slope.factor(Fixed::ONE / 2), Some(pct(20)));
        assert_eq!(slope.factor(pct(61)), None);
    }

    #[test]
    fn ids_must_follow_file_order() {
        let text = BUILTIN_TERRAIN.replace("id = 1\n", "id = 9\n");
        assert!(TerrainRules::parse(&text).is_err());
    }
}
