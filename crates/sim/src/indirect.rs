//! Indirect fire and smoke (prompt 032): the rules of `data/indirect.toml`,
//! the indirect weapon of a unit type, ballistics and the public snapshot
//! types. The update itself (orders, firing, shells, fragmentation, smoke
//! clouds): `crate::sim::indirect`. See docs/ARCHITECTURE.md "Indirect fire
//! and smoke".

use crate::catalog::{Category, Entry};
use crate::combat::fixed_sqrt;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::sin_cos_deg;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_INDIRECT: &str = include_str!("../../../data/indirect.toml");

/// `weapon` of the [`crate::CombatEvent::Shot`] events of indirect weapons.
pub const INDIRECT_WEAPON: u8 = u8::MAX;

/// Whether units of `category` fire indirectly (their first catalog weapon).
pub const fn is_indirect_category(category: Category) -> bool {
    matches!(
        category,
        Category::InfMortarLight
            | Category::InfMortarMedium
            | Category::MortarHeavy
            | Category::GunHowitzer
            | Category::SelfPropelled
    )
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndirectError(pub String);

impl fmt::Display for IndirectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid indirect fire rules: {}", self.0)
    }
}

impl std::error::Error for IndirectError {}

/// Rules of one indirect-fire category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryRules {
    pub category: Category,
    pub min_range_pct: u32,
    pub setup_ms: u32,
    pub elevation_deg: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndirectBallistics {
    pub gravity_mm_per_s2: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndirectDispersion {
    pub mrad_x10: u32,
    pub min_mm: u32,
    pub blind_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlastRules {
    pub radius_mm_per_mm: u32,
    pub infantry_pct: u32,
    pub cover_pct: u32,
    pub soldier_damage: u32,
    pub open_armor_mm: u32,
    pub open_damage_per_mm: u32,
    pub direct_hit_pen_pct: u32,
    pub roof_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeRules {
    pub radius_mm_per_mm: u32,
    pub lifetime_ms: u32,
    pub height_mm: u32,
    pub wind_x_mm_per_s: i32,
    pub wind_y_mm_per_s: i32,
    pub max_clouds: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BarrageRules {
    pub max_salvos: u32,
    pub max_radius_m: u32,
}

/// Rules of `data/indirect.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndirectRules {
    #[serde(rename = "category")]
    pub categories: Vec<CategoryRules>,
    pub ballistics: IndirectBallistics,
    pub dispersion: IndirectDispersion,
    pub blast: BlastRules,
    pub smoke: SmokeRules,
    pub barrage: BarrageRules,
}

impl IndirectRules {
    pub fn parse(text: &str) -> Result<Self, IndirectError> {
        let rules: Self = toml::from_str(text).map_err(|e| IndirectError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(IndirectError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        for c in &rules.categories {
            if !is_indirect_category(c.category) {
                return Err(IndirectError(format!(
                    "category '{}' does not fire indirectly",
                    c.category.code()
                )));
            }
            check("category.min_range_pct", c.min_range_pct, 0, 90)?;
            check("category.setup_ms", c.setup_ms, 0, 600_000)?;
            check("category.elevation_deg", c.elevation_deg, 5, 85)?;
        }
        let (b, d, x, s, r) = (
            &rules.ballistics,
            &rules.dispersion,
            &rules.blast,
            &rules.smoke,
            &rules.barrage,
        );
        check(
            "ballistics.gravity_mm_per_s2",
            b.gravity_mm_per_s2,
            1,
            100_000,
        )?;
        check("dispersion.mrad_x10", d.mrad_x10, 0, 1000)?;
        check("dispersion.min_mm", d.min_mm, 0, 100_000)?;
        check("dispersion.blind_pct", d.blind_pct, 100, 10_000)?;
        check("blast.radius_mm_per_mm", x.radius_mm_per_mm, 1, 2000)?;
        check("blast.infantry_pct", x.infantry_pct, 0, 100)?;
        check("blast.cover_pct", x.cover_pct, 0, 100)?;
        check("blast.soldier_damage", x.soldier_damage, 1, 100_000)?;
        check("blast.open_armor_mm", x.open_armor_mm, 0, 1000)?;
        check("blast.open_damage_per_mm", x.open_damage_per_mm, 0, 1000)?;
        check("blast.direct_hit_pen_pct", x.direct_hit_pen_pct, 0, 1000)?;
        check("blast.roof_pct", x.roof_pct, 0, 100)?;
        check("smoke.radius_mm_per_mm", s.radius_mm_per_mm, 1, 2000)?;
        check("smoke.lifetime_ms", s.lifetime_ms, 1000, 3_600_000)?;
        check("smoke.height_mm", s.height_mm, 1, 1_000_000)?;
        check(
            "smoke.wind_x_mm_per_s",
            s.wind_x_mm_per_s.unsigned_abs(),
            0,
            50_000,
        )?;
        check(
            "smoke.wind_y_mm_per_s",
            s.wind_y_mm_per_s.unsigned_abs(),
            0,
            50_000,
        )?;
        check("smoke.max_clouds", s.max_clouds, 1, 10_000)?;
        check("barrage.max_salvos", r.max_salvos, 1, 1000)?;
        check("barrage.max_radius_m", r.max_radius_m, 0, 10_000)?;
        Ok(rules)
    }

    /// The rules from `data/indirect.toml`, compiled in.
    pub fn builtin() -> Result<Self, IndirectError> {
        Self::parse(BUILTIN_INDIRECT)
    }

    /// Rules of a category (defaults for categories without an entry).
    pub fn category(&self, category: Option<Category>) -> CategoryRules {
        self.categories
            .iter()
            .find(|c| Some(c.category) == category)
            .copied()
            .unwrap_or(CategoryRules {
                category: Category::GunHowitzer,
                min_range_pct: 10,
                setup_ms: 0,
                elevation_deg: 45,
            })
    }

    /// Fragmentation radius of a shell in meters.
    pub fn blast_radius(&self, caliber_mm_x10: u32) -> Fixed {
        Fixed::from_num(u64::from(caliber_mm_x10) * u64::from(self.blast.radius_mm_per_mm)) / 10_000
    }

    /// Radius of the smoke cloud of a shell in meters.
    pub fn smoke_radius(&self, caliber_mm_x10: u32) -> Fixed {
        Fixed::from_num(u64::from(caliber_mm_x10) * u64::from(self.smoke.radius_mm_per_mm)) / 10_000
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.categories.len() as u64);
        for c in &self.categories {
            h.write_bytes(c.category.code().as_bytes());
            h.write_u32(c.min_range_pct);
            h.write_u32(c.setup_ms);
            h.write_u32(c.elevation_deg);
        }
        let (b, d, x, s, r) = (
            &self.ballistics,
            &self.dispersion,
            &self.blast,
            &self.smoke,
            &self.barrage,
        );
        for v in [
            b.gravity_mm_per_s2,
            d.mrad_x10,
            d.min_mm,
            d.blind_pct,
            x.radius_mm_per_mm,
            x.infantry_pct,
            x.cover_pct,
            x.soldier_damage,
            x.open_armor_mm,
            x.open_damage_per_mm,
            x.direct_hit_pen_pct,
            x.roof_pct,
            s.radius_mm_per_mm,
            s.lifetime_ms,
            s.height_mm,
            s.wind_x_mm_per_s.cast_unsigned(),
            s.wind_y_mm_per_s.cast_unsigned(),
            s.max_clouds,
            r.max_salvos,
            r.max_radius_m,
        ] {
            h.write_u32(v);
        }
    }
}

/// The indirect weapon of a unit type (the first catalog weapon of a
/// mortar, howitzer or self-propelled gun), converted by [`Scale`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndirectWeapon {
    pub name: String,
    pub caliber_mm_x10: u32,
    /// Catalog rate of fire, real.
    pub rate_rpm: u32,
    /// Catalog muzzle velocity, real; if the catalog has none, the least
    /// velocity that reaches the maximum range in vacuum (`sqrt(R·g)`).
    pub muzzle_velocity_mps: u32,
    /// Catalog maximum range, real meters.
    pub real_range_m: u32,
    /// Game range in meters (real × indirect-fire factor).
    pub range_m: Fixed,
    /// Game seconds between two rounds.
    pub interval_s: Fixed,
    /// The catalog ammunition includes smoke (or white phosphorus).
    pub smoke: bool,
    /// Indirect-fire range factor `(num, den)`: real distance = game
    /// distance × den / num.
    pub range_ratio: (u32, u32),
    /// Time factor of the category `(num, den)`.
    pub time_ratio: (u32, u32),
}

impl IndirectWeapon {
    /// The indirect weapon of a catalog entry, if its category fires
    /// indirectly and its first weapon has caliber, rate and range.
    pub fn from_catalog(e: &Entry, scale: &Scale) -> Option<Self> {
        if !is_indirect_category(e.category) {
            return None;
        }
        let w = e.weapon.first()?;
        let (caliber, rate, range) = (w.caliber_mm_x10?, w.rate_rpm?, w.range_effective_m?);
        if caliber == 0 || rate == 0 || range == 0 {
            return None;
        }
        let velocity = w.muzzle_velocity_mps.filter(|&v| v > 0).unwrap_or_else(|| {
            // sqrt(R · 9.807) in m/s, rounded up.
            let r = u64::from(range) * 9807 / 1000;
            u32::try_from(r.isqrt() + 1).unwrap_or(u32::MAX)
        });
        let ammo = w.ammo.to_ascii_lowercase();
        let smoke = ammo.contains("smoke") || w.ammo.contains("WP");
        let factor = scale.time_factor_for(e.category);
        Some(Self {
            name: w.name.clone(),
            caliber_mm_x10: caliber,
            rate_rpm: rate,
            muzzle_velocity_mps: velocity,
            real_range_m: range,
            range_m: scale.indirect_fire_range_m(range),
            interval_s: scale.game_duration(e.category, 60_000 / rate.max(1)),
            smoke,
            range_ratio: (scale.indirect_fire_range.num, scale.indirect_fire_range.den),
            time_ratio: (factor.num, factor.den),
        })
    }

    /// Real meters per game meter.
    pub fn real_per_game(&self) -> Fixed {
        let (num, den) = self.range_ratio;
        if num == 0 || den == 0 {
            Fixed::ONE
        } else {
            Fixed::from_num(den) / Fixed::from_num(num)
        }
    }
}

/// Flight of a shell: game seconds and the height of the apex over the
/// straight line from muzzle to impact, in game meters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flight {
    pub seconds: Fixed,
    pub apex_m: Fixed,
}

/// Flight time and apex of a shell to a target `game_m` game meters away.
///
/// Ballistics run at real distance (game distance × the inverse of the
/// indirect-fire factor) in vacuum: the charge is chosen so the shell
/// reaches the target at `elevation_deg`; if the muzzle velocity is not
/// enough, it flies at full charge at the matching angle (high for mortars,
/// whose elevation is above 45°, low for guns), at most to the 45° range.
/// The real flight time is divided by the time factor; the apex is scaled
/// down like the distance, so the arc keeps its shape on the map.
pub fn flight(
    weapon: &IndirectWeapon,
    game_m: Fixed,
    elevation_deg: u32,
    gravity_mm_per_s2: u32,
) -> Flight {
    let g = Fixed::from_num(gravity_mm_per_s2.max(1)) / 1000;
    let real = game_m * weapon.real_per_game();
    let v_max = Fixed::from_num(weapon.muzzle_velocity_mps.max(1));
    let elevation = u16::try_from(elevation_deg.clamp(5, 85)).unwrap_or(45);
    let sin_2e = sin_cos_deg(elevation * 2).0;
    // Velocity needed at the preferred elevation: v² = R·g / sin 2θ.
    let v2_needed = real * g / sin_2e.max(Fixed::ONE / 100);
    let (v, sin_theta) = if v2_needed <= v_max * v_max {
        (fixed_sqrt(v2_needed), sin_cos_deg(elevation).0)
    } else {
        // Full charge: sin 2θ = R·g / v², the angle on the side of the
        // preferred elevation.
        let s = (real * g / (v_max * v_max)).min(Fixed::ONE);
        let c = fixed_sqrt(Fixed::ONE - s * s);
        let high = elevation > 45;
        let half = (if high { Fixed::ONE + c } else { Fixed::ONE - c }) / 2;
        (v_max, fixed_sqrt(half))
    };
    let vertical = v * sin_theta;
    let real_seconds = vertical * 2 / g;
    let (num, den) = weapon.time_ratio;
    let time = if num == 0 || den == 0 {
        Fixed::ONE
    } else {
        Fixed::from_num(num) / Fixed::from_num(den)
    };
    let apex_real = vertical * vertical / (g * 2);
    Flight {
        seconds: real_seconds / time,
        apex_m: apex_real / weapon.real_per_game(),
    }
}

/// A shell in flight, for drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellSnapshot {
    /// Unique among projectiles and shells (shared counter).
    pub id: u32,
    pub shooter: UnitId,
    pub owner: PlayerId,
    pub position: crate::combat::Vec3,
    /// Meters per second.
    pub velocity: crate::combat::Vec3,
    /// Where it will land.
    pub target: crate::combat::Vec3,
    /// Ticks until the impact.
    pub ticks_left: u32,
    pub caliber_mm_x10: u32,
    pub smoke: bool,
}

/// A smoke cloud, for drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmokeSnapshot {
    pub id: u32,
    pub center: Vec2,
    pub radius: Fixed,
    /// Ticks since the cloud appeared and until it is gone.
    pub age_ticks: u32,
    pub ticks_left: u32,
    /// Dark smoke of a fire (prompt 041), not of a smoke shell.
    pub dark: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    fn weapon(id: &str) -> IndirectWeapon {
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        IndirectWeapon::from_catalog(catalog.get(id).unwrap(), &scale).unwrap()
    }

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = IndirectRules::builtin().unwrap();
        assert_eq!(rules.categories.len(), 5);
        assert_eq!(
            rules.blast_radius(814).round().to_num::<i32>(),
            16,
            "8 cm: about 16 m"
        );
        assert!(
            IndirectRules::parse(&BUILTIN_INDIRECT.replace("blind_pct = 300", "blind_pct = 50"))
                .is_err()
        );
        assert!(
            IndirectRules::parse(
                &BUILTIN_INDIRECT
                    .replace("category = \"self_propelled\"", "category = \"tank_light\"")
            )
            .is_err()
        );
    }

    #[test]
    fn catalog_weapons_are_converted() {
        let grw = weapon("de_inf_mortar_medium");
        // 2400 m real × 1/8 = 300 m; 15 rpm = 4 s real = 2 s game.
        assert_eq!(grw.range_m, Fixed::from_num(300));
        assert_eq!(grw.interval_s, Fixed::from_num(2));
        assert!(grw.smoke);
        let lefh = weapon("de_lefh18");
        assert!(lefh.smoke);
        assert!(!weapon("de_grw42").smoke);
        assert!(weapon("us_inf_mortar_medium").smoke, "WP counts as smoke");
        // No muzzle velocity in the catalog: sqrt(800 m × g) ≈ 89 m/s.
        assert_eq!(weapon("su_inf_mortar_light").muzzle_velocity_mps, 89);
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        assert!(IndirectWeapon::from_catalog(catalog.get("de_pz4_h").unwrap(), &scale).is_none());
    }

    #[test]
    fn flight_times_follow_real_ballistics() {
        let s = |f: Flight| f.seconds.round().to_num::<i32>();
        let grw = weapon("de_inf_mortar_medium");
        // 300 m game = 2400 m real at 60°: v ≈ 165 m/s (< 174), t ≈ 29 s
        // real, 14.6 s game; apex ≈ 1040 m real, 130 m game.
        let full = flight(&grw, Fixed::from_num(300), 60, 9807);
        assert_eq!(s(full), 15);
        assert_eq!(full.apex_m.round().to_num::<i32>(), 130);
        // Short range: a small charge, shorter flight.
        assert_eq!(s(flight(&grw, Fixed::from_num(50), 60, 9807)), 6);
        let lefh = weapon("de_lefh18");
        // 1540 m game = 12.3 km real at 25°: v ≈ 397 m/s, t ≈ 34 s real.
        assert_eq!(s(flight(&lefh, Fixed::from_num(1540), 25, 9807)), 17);
        // A mortar without enough velocity flies at full charge, high.
        let small = weapon("su_inf_mortar_light");
        let f = flight(&small, Fixed::from_num(100), 60, 9807);
        assert!(f.seconds > Fixed::from_num(4) && f.seconds < Fixed::from_num(8));
    }
}
