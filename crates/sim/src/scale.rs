//! Conversion of real catalog values into game values (`data/scale.toml`).
//!
//! Sizes are 1:1. Time runs faster by the time factor, ranges are compressed
//! and aircraft use their own speed factor. Every factor is an exact fraction,
//! applied in fixed point. See `docs/SCALE.md`.

use crate::catalog::{Category, Group};
use crate::math::Fixed;
use serde::Deserialize;
use std::fmt;

/// The scale file compiled into the simulation.
pub const BUILTIN_SCALE: &str = include_str!("../../../data/scale.toml");

/// A positive fraction `num / den`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ratio {
    pub num: u32,
    pub den: u32,
}

impl Ratio {
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// `value * num / den`, multiplying first to keep precision.
    pub fn apply(self, value: Fixed) -> Fixed {
        value * Fixed::from_num(self.num) / Fixed::from_num(self.den)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeOverride {
    pub category: Category,
    pub factor: Ratio,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeSection {
    factor: Ratio,
    #[serde(default, rename = "override")]
    overrides: Vec<TimeOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RangeSection {
    direct_fire: Ratio,
    sight: Ratio,
    indirect_fire: Ratio,
    /// Fuel range (prompt 036).
    fuel: Ratio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct AircraftSection {
    speed: Ratio,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScaleFile {
    time: TimeSection,
    range: RangeSection,
    aircraft: AircraftSection,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ScaleError(pub String);

impl fmt::Display for ScaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid scale: {}", self.0)
    }
}

impl std::error::Error for ScaleError {}

/// The parsed and validated scale rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scale {
    pub time_factor: Ratio,
    pub time_overrides: Vec<TimeOverride>,
    pub direct_fire_range: Ratio,
    pub sight_range: Ratio,
    pub indirect_fire_range: Ratio,
    pub aircraft_speed: Ratio,
    /// Real road range to the distance a full tank drives (prompt 036).
    pub fuel_range: Ratio,
}

impl Scale {
    pub fn builtin() -> Result<Self, ScaleError> {
        Self::parse(BUILTIN_SCALE)
    }

    pub fn parse(source: &str) -> Result<Self, ScaleError> {
        let file: ScaleFile = toml::from_str(source).map_err(|e| ScaleError(e.to_string()))?;
        let scale = Self {
            time_factor: file.time.factor,
            time_overrides: file.time.overrides,
            direct_fire_range: file.range.direct_fire,
            sight_range: file.range.sight,
            indirect_fire_range: file.range.indirect_fire,
            aircraft_speed: file.aircraft.speed,
            fuel_range: file.range.fuel,
        };
        let mut ratios = vec![
            scale.time_factor,
            scale.direct_fire_range,
            scale.sight_range,
            scale.indirect_fire_range,
            scale.aircraft_speed,
            scale.fuel_range,
        ];
        ratios.extend(scale.time_overrides.iter().map(|o| o.factor));
        // Bounded so that no conversion can overflow Fixed.
        if ratios
            .iter()
            .any(|r| r.num == 0 || r.den == 0 || r.num > 1000 || r.den > 1000)
        {
            return Err(ScaleError("factors must be fractions of 1..=1000".into()));
        }
        Ok(scale)
    }

    /// The time factor of a category: an override if present, else the global one.
    pub fn time_factor_for(&self, category: Category) -> Ratio {
        self.time_overrides
            .iter()
            .find(|o| o.category == category)
            .map_or(self.time_factor, |o| o.factor)
    }

    /// The factor applied to real speeds: the aircraft factor for aircraft,
    /// otherwise the time factor of the category.
    pub fn speed_factor_for(&self, category: Category) -> Ratio {
        if category.group() == Group::Aircraft {
            self.aircraft_speed
        } else {
            self.time_factor_for(category)
        }
    }

    /// Converts a real speed in tenths of km/h into game meters per second.
    pub fn game_speed(&self, category: Category, speed_kmh_x10: u32) -> Fixed {
        // km/h * 10 / 36 = m/s; the factor is applied before the division.
        let factor = self.speed_factor_for(category);
        Fixed::from_num(u64::from(speed_kmh_x10) * u64::from(factor.num))
            / Fixed::from_num(36 * u64::from(factor.den))
    }

    /// Converts a real duration in milliseconds into game seconds.
    pub fn game_duration(&self, category: Category, real_ms: u32) -> Fixed {
        let factor = self.time_factor_for(category);
        Fixed::from_num(u64::from(real_ms) * u64::from(factor.den))
            / Fixed::from_num(1000 * u64::from(factor.num))
    }

    /// Game range in meters of a real direct-fire range.
    pub fn direct_fire_range_m(&self, real_m: u32) -> Fixed {
        self.direct_fire_range.apply(Fixed::from_num(real_m))
    }

    /// Game range in meters of a real sight range.
    pub fn sight_range_m(&self, real_m: u32) -> Fixed {
        self.sight_range.apply(Fixed::from_num(real_m))
    }

    /// Meters a vehicle with a real road range of `road_km` drives on a
    /// full tank (prompt 036).
    pub fn fuel_range_m(&self, road_km: u32) -> u32 {
        let m = u64::from(road_km) * 1000 * u64::from(self.fuel_range.num)
            / u64::from(self.fuel_range.den);
        u32::try_from(m).unwrap_or(u32::MAX)
    }

    /// Game range in meters of a real indirect-fire range.
    pub fn indirect_fire_range_m(&self, real_m: u32) -> Fixed {
        self.indirect_fire_range.apply(Fixed::from_num(real_m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_scale_parses() {
        let scale = Scale::builtin().unwrap();
        assert_eq!(scale.time_factor, Ratio { num: 2, den: 1 });
    }

    #[test]
    fn speeds_are_doubled_exactly() {
        let scale = Scale::builtin().unwrap();
        // 36 km/h real = 10 m/s real = 20 m/s in game.
        assert_eq!(
            scale.game_speed(Category::TankMedium, 360),
            Fixed::from_num(20)
        );
        // Aircraft: 720 km/h real = 200 m/s, halved = 100 m/s.
        assert_eq!(
            scale.game_speed(Category::AircraftFighter, 7200),
            Fixed::from_num(100)
        );
    }

    #[test]
    fn durations_are_halved_and_ranges_compressed() {
        let scale = Scale::builtin().unwrap();
        assert_eq!(
            scale.game_duration(Category::GunAt, 6000),
            Fixed::from_num(3)
        );
        assert_eq!(scale.direct_fire_range_m(1000), Fixed::from_num(250));
        assert_eq!(scale.indirect_fire_range_m(8000), Fixed::from_num(1000));
    }

    #[test]
    fn overrides_win_and_bad_factors_fail() {
        let src = BUILTIN_SCALE.replace(
            "[range]",
            "[[time.override]]\ncategory = \"inf_rifle\"\nfactor = { num = 3, den = 2 }\n\n[range]",
        );
        let scale = Scale::parse(&src).unwrap();
        assert_eq!(
            scale.time_factor_for(Category::InfRifle),
            Ratio { num: 3, den: 2 }
        );
        assert_eq!(scale.time_factor_for(Category::InfLmg), scale.time_factor);

        let bad = BUILTIN_SCALE.replace("num = 2, den = 1", "num = 2, den = 0");
        assert!(Scale::parse(&bad).is_err());
    }
}
