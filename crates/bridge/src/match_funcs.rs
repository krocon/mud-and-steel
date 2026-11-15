//! `SimHost` functions for the game flow (prompt 027): starting a skirmish
//! with chosen factions, surrendering, and the result of the match.

use crate::{SimHost, load_map_dir};
use godot::prelude::*;
use sim::scenario::builtin_map;
use sim::{Command, DefeatReason, PlayerId};
use std::path::Path;
use std::sync::Arc;

#[godot_api(secondary)]
impl SimHost {
    /// Starts the demo scenario as a skirmish: on the built-in map if
    /// `map_dir` is empty, else on the map in that folder (absolute path);
    /// `factions[i]` is the faction id (`"de"`, `"su"`, `"us"`) of player
    /// `i`, an empty entry keeps the scenario's units. The local player is
    /// player 0.
    #[func]
    fn start_match(&mut self, seed: i64, map_dir: GString, factions: PackedStringArray) -> bool {
        let map = if map_dir.is_empty() {
            builtin_map()
        } else {
            match load_map_dir(Path::new(&map_dir.to_string())) {
                Ok(map) => Arc::new(map),
                Err(e) => {
                    godot_error!("{e}");
                    return false;
                }
            }
        };
        let names: Vec<String> = factions.as_slice().iter().map(GString::to_string).collect();
        let overrides: Vec<(PlayerId, &str)> = (0u8..)
            .zip(&names)
            .filter(|(_, name)| !name.is_empty())
            .map(|(player, name)| (PlayerId(player), name.as_str()))
            .collect();
        self.start_with(seed, map, &overrides)
    }

    /// Queues the surrender of the local player.
    #[func]
    fn surrender(&mut self) {
        if self.game.is_some() {
            self.pending.push(Command::Surrender);
        }
    }

    /// State of the match: `decided` (bool); once decided `winner` (player
    /// id, -1 for a draw) and `tick`; always `players`, an array of
    /// dictionaries with `id`, `defeated` (bool), `reason` (`""`,
    /// `"surrendered"`, `"eliminated"`), `defeat_tick` and `units` (units not
    /// destroyed).
    #[func]
    fn match_result(&self) -> VarDictionary {
        let Some(game) = &self.game else {
            let mut empty = vdict! { "decided" => false };
            empty.set("players", &VarArray::new());
            return empty;
        };
        let mut players = VarArray::new();
        for standing in game.sim.standings() {
            let (reason, tick) = match standing.defeat {
                None => ("", -1),
                Some((DefeatReason::Surrendered, tick)) => ("surrendered", tick_i64(tick)),
                Some((DefeatReason::Eliminated, tick)) => ("eliminated", tick_i64(tick)),
            };
            let entry = vdict! {
                "id" => i64::from(standing.player.0),
                "defeated" => standing.defeat.is_some(),
                "reason" => reason,
                "defeat_tick" => tick,
                "units" => game.sim.alive_units(standing.player) as i64,
            };
            players.push(&entry.to_variant());
        }
        let mut result = vdict! { "decided" => false };
        result.set("players", &players);
        if let Some(outcome) = game.sim.outcome() {
            result.set("decided", true);
            result.set("winner", outcome.winner.map_or(-1, |p| i64::from(p.0)));
            result.set("tick", tick_i64(outcome.tick));
        }
        result
    }
}

fn tick_i64(tick: u64) -> i64 {
    i64::try_from(tick).unwrap_or(i64::MAX)
}
