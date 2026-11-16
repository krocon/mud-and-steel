//! `SimHost` functions for weather and time of day (prompt 040): the
//! choice for the next start and what the running match has. Godot draws
//! rain, snow, fog, wet ground and the light of the time of day from this;
//! the simulation decides speeds and sight.

use crate::SimHost;
use godot::prelude::*;
use sim::{Daytime, SimConfig, Weather};

/// Applies the `weather` and `daytime` properties (empty: the map's
/// default, already in `config`).
pub(crate) fn apply_choice(host: &SimHost, config: &mut SimConfig) {
    let weather = host.weather.to_string();
    if !weather.is_empty() {
        match Weather::from_name(&weather) {
            Some(w) => config.weather.weather = w,
            None => {
                godot_warn!("unknown weather '{weather}', using the map's");
            }
        }
    }
    let daytime = host.daytime.to_string();
    if !daytime.is_empty() {
        match Daytime::from_name(&daytime) {
            Some(d) => config.weather.daytime = d,
            None => {
                godot_warn!("unknown time of day '{daytime}', using the map's");
            }
        }
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Weather names for menus, in the order of the rules.
    #[func]
    fn weather_names() -> PackedStringArray {
        Weather::ALL
            .iter()
            .map(|w| GString::from(w.name()))
            .collect()
    }

    /// Time of day names for menus.
    #[func]
    fn daytime_names() -> PackedStringArray {
        Daytime::ALL
            .iter()
            .map(|d| GString::from(d.name()))
            .collect()
    }

    /// The weather of the running match: `weather`, `daytime` (names),
    /// `ground` (`dry`, `wet`, `snow`), `ground_pct` (softness 0..100),
    /// `sight_pct` (percent of the sight ranges), `wind` (Vector2 x/z in
    /// meters per second of game time). Empty without a match.
    #[func]
    fn weather_info(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(game) = self.game.as_ref() else {
            return d;
        };
        let setup = game.sim.weather_setup();
        let (ground, level) = game.sim.ground_state();
        let wind = game.sim.wind_m_per_s();
        let sight = sim::WeatherRules::builtin().map_or(100, |r| r.sight_pct(setup));
        d.set("weather", setup.weather.name());
        d.set("daytime", setup.daytime.name());
        d.set("ground", ground.name());
        d.set("ground_pct", i64::from(level));
        d.set("sight_pct", i64::from(sight));
        d.set(
            "wind",
            Vector2::new(crate::fixed_to_f32(wind.x), crate::fixed_to_f32(wind.y)),
        );
        d
    }
}
