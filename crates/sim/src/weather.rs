//! Weather, soft ground and time of day (prompt 040): the rules of
//! `data/weather.toml` and the public types. The update:
//! `crate::sim::weather`.
//!
//! A match has one [`Weather`] and one [`Daytime`] ([`WeatherSetup`], part
//! of [`crate::SimConfig`]). They shorten sight ranges, soften the ground
//! (speeds and path costs by running gear), change the wind and the fire
//! spread, and at night let muzzle flashes reveal the shooter. The default,
//! clear by day, changes nothing.

use crate::catalog::Category;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_WEATHER: &str = include_str!("../../../data/weather.toml");

/// Weather of a match.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Weather {
    #[default]
    Clear,
    Overcast,
    Rain,
    Snow,
    Fog,
}

impl Weather {
    pub const ALL: [Self; 5] = [
        Self::Clear,
        Self::Overcast,
        Self::Rain,
        Self::Snow,
        Self::Fog,
    ];

    /// The name used in `map.toml`, `data/weather.toml` and Godot.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Overcast => "overcast",
            Self::Rain => "rain",
            Self::Snow => "snow",
            Self::Fog => "fog",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|w| w.name() == name)
    }
}

/// Time of day of a match.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Daytime {
    Morning,
    #[default]
    Day,
    Evening,
    Night,
}

impl Daytime {
    pub const ALL: [Self; 4] = [Self::Morning, Self::Day, Self::Evening, Self::Night];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Day => "day",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }
}

/// Weather and time of day of a match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeatherSetup {
    pub weather: Weather,
    pub daytime: Daytime,
}

impl WeatherSetup {
    /// Clear by day: no effect at all.
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}

/// What the weather does to the ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ground {
    Dry,
    Wet,
    Snow,
}

impl Ground {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Dry => "dry",
            Self::Wet => "wet",
            Self::Snow => "snow",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherEntry {
    pub name: Weather,
    pub sight_pct: u32,
    pub ground: Ground,
    pub ground_start_pct: u32,
    pub ground_step_pct: u32,
    pub ground_step_s: u32,
    pub wind_pct: u32,
    pub fire_catch_pct: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaytimeEntry {
    pub name: Daytime,
    pub sight_pct: u32,
    pub muzzle_flash: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlashRules {
    pub reveal_s: u32,
    pub range_real_m: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearRules {
    pub name: String,
    pub wet_loss_pct: u32,
    pub snow_loss_pct: u32,
    pub categories: Vec<Category>,
    pub ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainSoftness {
    pub name: String,
    pub wet_pct: u32,
    pub snow_pct: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundRules {
    pub min_speed_pct: u32,
    pub gear: Vec<GearRules>,
    pub terrain: Vec<TerrainSoftness>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherFireRules {
    pub downwind_pct: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    weather: Vec<WeatherEntry>,
    daytime: Vec<DaytimeEntry>,
    flash: FlashRules,
    ground: GroundRules,
    fire: WeatherFireRules,
}

/// All weather rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherRules {
    /// One entry per [`Weather`], in [`Weather::ALL`] order.
    pub weather: Vec<WeatherEntry>,
    /// One entry per [`Daytime`], in [`Daytime::ALL`] order.
    pub daytime: Vec<DaytimeEntry>,
    pub flash: FlashRules,
    pub ground: GroundRules,
    pub fire: WeatherFireRules,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeatherError(pub String);

impl fmt::Display for WeatherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid weather rules: {}", self.0)
    }
}

impl std::error::Error for WeatherError {}

/// Longest ground step (one hour of game time).
const MAX_STEP_S: u32 = 3600;

impl WeatherRules {
    pub fn parse(text: &str) -> Result<Self, WeatherError> {
        let file: File = toml::from_str(text).map_err(|e| WeatherError(e.to_string()))?;
        let err = |what: String| Err(WeatherError(what));
        let mut weather = Vec::new();
        for w in Weather::ALL {
            let mut found = file.weather.iter().filter(|e| e.name == w);
            let (Some(e), None) = (found.next(), found.next()) else {
                return err(format!("weather '{}' must be listed once", w.name()));
            };
            if !(1..=100).contains(&e.sight_pct)
                || e.ground_start_pct > 100
                || e.ground_step_pct > 100
                || e.ground_step_s > MAX_STEP_S
                || e.wind_pct > 400
                || e.fire_catch_pct > 200
            {
                return err(format!("weather '{}': value out of range", w.name()));
            }
            weather.push(e.clone());
        }
        let mut daytime = Vec::new();
        for d in Daytime::ALL {
            let mut found = file.daytime.iter().filter(|e| e.name == d);
            let (Some(e), None) = (found.next(), found.next()) else {
                return err(format!("daytime '{}' must be listed once", d.name()));
            };
            if !(1..=100).contains(&e.sight_pct) {
                return err(format!("daytime '{}': sight_pct must be 1..=100", d.name()));
            }
            daytime.push(e.clone());
        }
        // The default must not change anything (golden hash, prompt 040).
        let clear = &weather[0];
        if clear.sight_pct != 100
            || clear.ground_start_pct != 0
            || clear.ground_step_pct != 0
            || clear.wind_pct != 100
            || clear.fire_catch_pct != 100
            || daytime[1].sight_pct != 100
            || daytime[1].muzzle_flash
        {
            return err("clear weather by day must leave every value at 100 %".into());
        }
        if file.flash.reveal_s > 60 || file.flash.range_real_m > 10_000 {
            return err("flash values out of range".into());
        }
        let g = &file.ground;
        if !(1..=100).contains(&g.min_speed_pct) {
            return err("ground.min_speed_pct must be 1..=100".into());
        }
        for (n, gear) in g.gear.iter().enumerate() {
            if gear.wet_loss_pct > 100 || gear.snow_loss_pct > 100 {
                return err(format!("gear '{}': losses must be at most 100", gear.name));
            }
            for c in &gear.categories {
                if g.gear[..n].iter().any(|o| o.categories.contains(c)) {
                    return err(format!("category '{}' has two running gears", c.code()));
                }
            }
            for id in &gear.ids {
                if g.gear[..n].iter().any(|o| o.ids.contains(id)) {
                    return err(format!("id '{id}' has two running gears"));
                }
            }
        }
        for t in &g.terrain {
            if t.wet_pct > 100 || t.snow_pct > 100 {
                return err(format!(
                    "terrain '{}': softness must be at most 100",
                    t.name
                ));
            }
        }
        if file.fire.downwind_pct > 100 {
            return err("fire.downwind_pct must be at most 100".into());
        }
        Ok(Self {
            weather,
            daytime,
            flash: file.flash,
            ground: file.ground,
            fire: file.fire,
        })
    }

    /// The rules from `data/weather.toml`.
    /// Borrowed reference to the built-in weather rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static WeatherError> {
        static CACHE: std::sync::OnceLock<Result<WeatherRules, WeatherError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_WEATHER)).as_ref()
    }

    pub fn builtin() -> Result<Self, WeatherError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    pub fn entry(&self, weather: Weather) -> &WeatherEntry {
        &self.weather[weather as usize]
    }

    pub fn daytime_entry(&self, daytime: Daytime) -> &DaytimeEntry {
        &self.daytime[daytime as usize]
    }

    /// Percent of every sight range for `setup` (at least 1).
    pub fn sight_pct(&self, setup: WeatherSetup) -> u32 {
        (self.entry(setup.weather).sight_pct * self.daytime_entry(setup.daytime).sight_pct / 100)
            .max(1)
    }

    /// Index into `ground.gear` of a unit type's running gear (`None`: not
    /// slowed by the ground).
    pub fn gear_of(&self, unit_type: &UnitType) -> Option<usize> {
        let gears = &self.ground.gear;
        gears
            .iter()
            .position(|g| g.ids.contains(&unit_type.name))
            .or_else(|| {
                let category = unit_type.combat.category?;
                gears.iter().position(|g| g.categories.contains(&category))
            })
    }

    /// Softness (wet, snow) in percent per terrain id of `terrain`.
    pub fn softness(&self, terrain: &TerrainRules) -> Vec<(u32, u32)> {
        terrain
            .types
            .iter()
            .map(|t| {
                self.ground
                    .terrain
                    .iter()
                    .find(|s| s.name == t.name)
                    .map_or((0, 0), |s| (s.wet_pct, s.snow_pct))
            })
            .collect()
    }

    /// Speed in percent of the terrain speed for running gear `gear` on
    /// ground of `kind` at softness `level` (0..=100) of a terrain whose
    /// softness is `soft` (0..=100).
    pub fn speed_pct(&self, gear: usize, kind: Ground, level: u32, soft: (u32, u32)) -> u32 {
        let g = &self.ground.gear[gear];
        let (loss, soft) = match kind {
            Ground::Dry => return 100,
            Ground::Wet => (g.wet_loss_pct, soft.0),
            Ground::Snow => (g.snow_loss_pct, soft.1),
        };
        let loss = loss * soft * level.min(100) / 10_000;
        100u32
            .saturating_sub(loss)
            .max(self.ground.min_speed_pct)
            .min(100)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        for w in &self.weather {
            for v in [
                w.sight_pct,
                w.ground as u32,
                w.ground_start_pct,
                w.ground_step_pct,
                w.ground_step_s,
                w.wind_pct,
                w.fire_catch_pct,
            ] {
                h.write_u32(v);
            }
        }
        for d in &self.daytime {
            h.write_u32(d.sight_pct);
            h.write_u8(u8::from(d.muzzle_flash));
        }
        h.write_u32(self.flash.reveal_s);
        h.write_u32(self.flash.range_real_m);
        h.write_u32(self.ground.min_speed_pct);
        h.write_u64(self.ground.gear.len() as u64);
        for g in &self.ground.gear {
            h.write_bytes(g.name.as_bytes());
            h.write_u32(g.wet_loss_pct);
            h.write_u32(g.snow_loss_pct);
            h.write_u64(g.categories.len() as u64);
            for c in &g.categories {
                h.write_bytes(c.code().as_bytes());
            }
            h.write_u64(g.ids.len() as u64);
            for id in &g.ids {
                h.write_bytes(id.as_bytes());
            }
        }
        h.write_u64(self.ground.terrain.len() as u64);
        for t in &self.ground.terrain {
            h.write_bytes(t.name.as_bytes());
            h.write_u32(t.wet_pct);
            h.write_u32(t.snow_pct);
        }
        h.write_u32(self.fire.downwind_pct);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = WeatherRules::builtin().unwrap();
        assert_eq!(rules.weather.len(), Weather::ALL.len());
        assert_eq!(rules.sight_pct(WeatherSetup::default()), 100);
        let foggy_night = WeatherSetup {
            weather: Weather::Fog,
            daytime: Daytime::Night,
        };
        assert!(rules.sight_pct(foggy_night) < 20);
    }

    #[test]
    fn mud_slows_wheels_more_than_tracks_and_spares_roads() {
        let rules = WeatherRules::builtin().unwrap();
        let terrain = TerrainRules::builtin().unwrap();
        let soft = rules.softness(&terrain);
        let gear = |name: &str| rules.ground.gear.iter().position(|g| g.name == name);
        let (wheels, tracks) = (gear("wheeled").unwrap(), gear("tracked").unwrap());
        let track = soft[1];
        let road = soft[2];
        let wet = |g, s| rules.speed_pct(g, Ground::Wet, 100, s);
        assert!(wet(wheels, track) < wet(tracks, track));
        assert!(wet(tracks, track) < 100);
        assert_eq!(wet(wheels, road), 100);
        assert_eq!(rules.speed_pct(wheels, Ground::Dry, 100, track), 100);
    }

    #[test]
    fn every_name_round_trips() {
        for w in Weather::ALL {
            assert_eq!(Weather::from_name(w.name()), Some(w));
        }
        for d in Daytime::ALL {
            assert_eq!(Daytime::from_name(d.name()), Some(d));
        }
    }

    #[test]
    fn the_default_must_stay_neutral() {
        let text = BUILTIN_WEATHER.replacen("sight_pct = 100", "sight_pct = 90", 1);
        assert!(WeatherRules::parse(&text).is_err());
    }
}
