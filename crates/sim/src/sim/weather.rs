//! Weather in the running match (prompt 040). The rules and public types:
//! `crate::weather`.
//!
//! - Sight: [`Sim::new`] scales every sight range once
//!   ([`crate::SimConfig::effective_vision`]).
//! - Ground: the soft-ground speeds of every unit type go into the path grid
//!   ([`crate::path::PathGrid::set_ground_pct`]); movement reads them there.
//!   In the rain the ground gets softer in steps; after a step the path costs
//!   are computed again, [`RECOST_ROWS_PER_TICK`] rows per tick (a 4 km map
//!   takes 16 ticks). Routes already planned stay as they are.
//! - Muzzle flashes (night): a unit that fired is revealed for a moment to
//!   every player with a unit nearby ([`Sim::reveal_flashes`], after the
//!   vision update).
//! - Wind and fire: the smoke drift is scaled once at the start, the fire
//!   spread asks [`Sim::weather_catch_pct`].
//!
//! Clear weather by day leaves all of this out: the ground is dry, nothing
//! is scaled, and the state is not hashed.

use super::Sim;
use crate::combat::CombatEvent;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::terrain::TerrainRules;
use crate::weather::{Ground, WeatherRules, WeatherSetup};
use serde::{Deserialize, Serialize};

/// Path rows whose costs are computed again per tick after the ground got
/// softer (a row of a 4 km map: 500 cells per profile).
pub(super) const RECOST_ROWS_PER_TICK: u32 = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct WeatherState {
    rules: WeatherRules,
    setup: WeatherSetup,
    ground: Ground,
    /// Softness of the ground, 0..=100 %.
    level: u32,
    step_pct: u32,
    /// Ticks per ground step; 0: the ground stays as it is.
    step_ticks: u64,
    /// Tick of the next ground step.
    next_step: u64,
    /// First path row whose costs are not computed for `level` yet.
    recost_row: Option<u32>,
    /// Running gear per unit type (index into the gear rules).
    gears: Vec<Option<usize>>,
    /// Softness (wet, snow) per terrain id.
    softness: Vec<(u32, u32)>,
    /// Shots reveal the shooter (night).
    flash: bool,
    flash_ticks: u64,
    flash_range: Fixed,
    /// Units seen by their muzzle flash: unit index and the last tick.
    flashes: Vec<(u32, u64)>,
    wind_pct: u32,
    fire_catch_pct: u32,
}

impl WeatherState {
    pub(super) fn new(
        rules: WeatherRules,
        setup: WeatherSetup,
        unit_types: &[UnitType],
        terrain: &TerrainRules,
        tick_hz: u32,
        sight_ratio: (u32, u32),
    ) -> Self {
        let w = rules.entry(setup.weather).clone();
        let d = rules.daytime_entry(setup.daytime).clone();
        let hz = u64::from(tick_hz.max(1));
        let step_ticks = if w.ground_step_pct > 0 {
            u64::from(w.ground_step_s) * hz
        } else {
            0
        };
        let (num, den) = sight_ratio;
        let flash_range = Fixed::from_num(rules.flash.range_real_m) * Fixed::from_num(num)
            / Fixed::from_num(den.max(1));
        Self {
            gears: unit_types.iter().map(|t| rules.gear_of(t)).collect(),
            softness: rules.softness(terrain),
            setup,
            ground: w.ground,
            level: w.ground_start_pct,
            step_pct: w.ground_step_pct,
            step_ticks,
            next_step: step_ticks,
            recost_row: None,
            flash: d.muzzle_flash,
            flash_ticks: u64::from(rules.flash.reveal_s) * hz,
            flash_range,
            flashes: Vec::new(),
            wind_pct: w.wind_pct,
            fire_catch_pct: w.fire_catch_pct,
            rules,
        }
    }

    pub(super) fn is_default(&self) -> bool {
        self.setup.is_default()
    }

    pub(super) const fn setup(&self) -> WeatherSetup {
        self.setup
    }

    /// Softness of the ground now, 0..=100 %.
    pub(super) const fn level(&self) -> u32 {
        self.level
    }

    pub(super) const fn ground(&self) -> Ground {
        self.ground
    }

    pub(super) const fn wind_pct(&self) -> u32 {
        self.wind_pct
    }

    /// Soft-ground speeds per unit type and terrain id for the path grid;
    /// `None` on dry ground (nothing to change).
    pub(super) fn ground_table(&self) -> Option<Vec<Vec<u32>>> {
        if self.ground == Ground::Dry || self.level == 0 {
            return None;
        }
        Some(
            self.gears
                .iter()
                .map(|gear| match gear {
                    Some(g) => self
                        .softness
                        .iter()
                        .map(|&soft| self.rules.speed_pct(*g, self.ground, self.level, soft))
                        .collect(),
                    None => Vec::new(),
                })
                .collect(),
        )
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u8(self.setup.weather as u8);
        h.write_u8(self.setup.daytime as u8);
        h.write_u32(self.level);
        h.write_u64(self.next_step);
        h.write_u32(self.recost_row.unwrap_or(u32::MAX));
        h.write_u64(self.flashes.len() as u64);
        for &(i, until) in &self.flashes {
            h.write_u32(i);
            h.write_u64(until);
        }
    }
}

impl Sim {
    /// Weather and time of day of this match (prompt 040).
    pub const fn weather_setup(&self) -> WeatherSetup {
        self.weather.setup()
    }

    /// Softness of the ground in percent (0: dry or firm) and what makes it
    /// soft.
    pub const fn ground_state(&self) -> (crate::weather::Ground, u32) {
        (self.weather.ground(), self.weather.level())
    }

    /// The wind of the match in meters per second of game time (smoke drift,
    /// fire spread).
    pub fn wind_m_per_s(&self) -> Vec2 {
        self.artillery.wind().scale(Fixed::from_num(self.tick_hz))
    }

    /// Soft ground for the path grid at the start of a match: sets the
    /// speeds and computes every cost at once.
    pub(super) fn init_weather(&mut self) {
        let pct = self.weather.wind_pct();
        self.artillery.scale_wind(pct);
        if let Some(table) = self.weather.ground_table() {
            let grid = &mut self.paths.grid;
            grid.set_ground_pct(table);
            grid.recost_rows(&self.map, &self.terrain, 0, u32::MAX);
        }
    }

    /// Once per tick, after the projectiles: the ground gets softer in the
    /// rain (and its path costs follow), muzzle flashes of this tick.
    pub(super) fn update_weather(&mut self) {
        if self.weather.is_default() {
            return;
        }
        let tick = self.tick + 1;
        let w = &mut self.weather;
        if w.step_ticks > 0 && w.level < 100 && tick >= w.next_step {
            w.level = (w.level + w.step_pct).min(100);
            w.next_step += w.step_ticks;
            if let Some(table) = w.ground_table() {
                self.paths.grid.set_ground_pct(table);
                w.recost_row = Some(0);
            }
        }
        if let Some(row) = w.recost_row {
            let last = row + RECOST_ROWS_PER_TICK - 1;
            self.paths
                .grid
                .recost_rows(&self.map, &self.terrain, row, last);
            w.recost_row = (last + 1 < self.paths.grid.size().1).then_some(last + 1);
        }
        w.flashes.retain(|&(_, until)| until >= tick);
        if w.flash {
            let until = tick + w.flash_ticks.max(1) - 1;
            for event in &self.events {
                let CombatEvent::Shot { unit, .. } = event else {
                    continue;
                };
                let Ok(i) = self.units.binary_search_by_key(unit, |u| u.id) else {
                    continue;
                };
                let i = i as u32;
                match w.flashes.iter_mut().find(|(f, _)| *f == i) {
                    Some(f) => f.1 = until,
                    None => w.flashes.push((i, until)),
                }
            }
            w.flashes.sort_unstable();
        }
    }

    /// After the vision update: every unit that fired lately is seen by
    /// every player with a living unit within the flash range.
    pub(super) fn reveal_flashes(&mut self) {
        if self.weather.flashes.is_empty() {
            return;
        }
        let range = self.weather.flash_range;
        let mut players = Vec::new();
        for &(i, _) in &self.weather.flashes {
            let shooter = &self.units[i as usize];
            if !shooter.alive() || super::transport::aboard(shooter) {
                continue;
            }
            players.clear();
            players.extend(
                self.units
                    .iter()
                    .filter(|u| {
                        u.alive()
                            && u.owner != shooter.owner
                            && (u.position - shooter.position).length() <= range
                    })
                    .map(|u| u.owner),
            );
            players.sort_unstable();
            players.dedup();
            for &p in &players {
                self.vision.reveal(i as usize, p);
            }
        }
    }

    /// The catch chance of a fire spreading along `way` (from the fire to
    /// the neighbor) under this weather: wet wood catches less, downwind
    /// more, upwind less.
    pub(super) fn weather_catch_pct(&self, pct: u32, way: Vec2) -> u32 {
        let w = &self.weather;
        let mut pct = pct * w.fire_catch_pct / 100;
        let bias = w.rules.fire.downwind_pct;
        let wind = self.artillery.wind();
        let (wind_len, way_len) = (wind.length(), way.length());
        if bias > 0 && pct > 0 && wind_len > Fixed::ZERO && way_len > Fixed::ZERO {
            let cos = (wind.x * way.x + wind.y * way.y) / (wind_len * way_len);
            let strength = Fixed::from_num(w.wind_pct.min(200)) / 100;
            let factor = Fixed::from_num(100) + Fixed::from_num(bias) * cos * strength;
            pct = (Fixed::from_num(pct) * factor.max(Fixed::ZERO) / 100)
                .round()
                .to_num::<u32>();
        }
        pct.min(100)
    }
}
