//! `SimHost` functions for saved games (prompt 038): saving a single-player
//! match and continuing it later. Format and checks: `net::save`.

use crate::{Game, LOCAL_PLAYER, SimHost};
use godot::prelude::*;
use net::{HostedState, LocalTransport, LockstepConfig, LockstepSession, ReplayRecorder, SaveGame};
use sim::PlayerId;

/// Hash exchange interval of the restored session (as in `start_config`).
const HASH_INTERVAL: u64 = 20;

/// The replay of `game` so far with the info of a list: who played whom,
/// when it was written.
pub(crate) fn replay_of(game: &Game, created: &str) -> net::Replay {
    let mut replay = game.recorder.finish(&game.sim);
    replay.info.created = created.into();
    replay.info.multiplayer = game.networked;
    for player in &mut replay.info.players {
        player.kind = if let Some(ai) = game.ais.iter().find(|a| a.player() == player.id) {
            format!("ai:{}", ai.difficulty())
        } else if game.session.local_player() == player.id
            || (game.networked && replay.players.contains(&player.id))
        {
            "human".into()
        } else {
            String::new()
        };
    }
    replay
}

#[godot_api(secondary)]
impl SimHost {
    /// Whether the running match can be saved: single player (also against
    /// computer players), running, not a replay.
    #[func]
    fn can_save(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|g| !g.networked && g.halted.is_none() && g.playback.is_none())
    }

    /// The running match as saved-game file contents (`net::save`), or empty
    /// if it cannot be saved. `created` is the local date and time for the
    /// load list; `settings` (string keys and values, e.g. `biome`) come
    /// back from `load_game` through `loaded_settings`.
    #[func]
    fn save_game_bytes(&self, created: GString, settings: VarDictionary) -> PackedByteArray {
        let Some(game) = self.game.as_ref().filter(|_| self.can_save()) else {
            return PackedByteArray::new();
        };
        let replay = replay_of(game, &created.to_string());
        let save = SaveGame {
            info: replay.info.clone(),
            snapshot: net::snapshot::encode(&game.sim, &game.loaded_map),
            pending: game.session.pending_frames(),
            input_delay: game.session.input_delay(),
            hosted: game
                .ais
                .iter()
                .map(|ai| HostedState {
                    player: ai.player(),
                    state: ai.save_state(),
                })
                .collect(),
            settings: settings
                .iter_shared()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            replay,
        };
        PackedByteArray::from(save.encode())
    }

    /// Continues a saved game from file contents; the map is loaded from
    /// `maps_dir/<name>/` (or built in). Returns false and keeps the reason
    /// for `load_error` on failure.
    #[func]
    fn load_game(&mut self, bytes: PackedByteArray, maps_dir: GString) -> bool {
        match self.try_load_game(bytes.as_slice(), &maps_dir.to_string()) {
            Ok(()) => {
                self.load_error.clear();
                true
            }
            Err(e) => {
                godot_error!("cannot load the saved game: {e}");
                self.load_error = e;
                false
            }
        }
    }

    /// Why the last `load_game` or `start_replay` failed ("" if it did not).
    #[func]
    fn load_error(&self) -> GString {
        GString::from(self.load_error.as_str())
    }

    /// Tick and state hash (hex) right after the last `load_game`, before
    /// any further tick: `tick`, `hash` (empty without a loaded game).
    #[func]
    fn loaded_state(&self) -> VarDictionary {
        self.loaded_state
            .as_ref()
            .map_or_else(VarDictionary::new, |(tick, hash)| {
                vdict! {
                    "tick" => i64::try_from(*tick).unwrap_or(i64::MAX),
                    "hash" => format!("{hash:#018x}"),
                }
            })
    }

    /// The `settings` saved with the loaded game (empty otherwise).
    #[func]
    fn loaded_settings(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        for (key, value) in &self.loaded_settings {
            dict.set(key.as_str(), value.as_str());
        }
        dict
    }
}

impl SimHost {
    fn try_load_game(&mut self, bytes: &[u8], maps_dir: &str) -> Result<(), String> {
        let save = SaveGame::decode(bytes).map_err(|e| e.to_string())?;
        let map = crate::replay_funcs::load_named_map(&save.replay.config.map.name, maps_dir)?;
        let sim = save.restore(map.clone()).map_err(|e| e.to_string())?;
        let ais = save
            .hosted
            .iter()
            .map(|h| ai::Ai::load_state(&h.state))
            .collect::<Result<Vec<_>, _>>()?;
        let mut players: Vec<PlayerId> = ais.iter().map(ai::Ai::player).collect();
        players.push(LOCAL_PLAYER);
        players.sort_unstable();
        players.dedup();
        let lockstep = LockstepConfig {
            players,
            input_delay: save.input_delay,
            hash_interval: HASH_INTERVAL,
        };
        self.input_delay = save.input_delay;
        self.install(
            sim,
            map,
            save.replay.config.clone(),
            save.replay.seed,
            LOCAL_PLAYER,
            &lockstep,
            Box::new(LocalTransport),
        );
        let game = self.game.as_mut().expect("installed");
        game.session = LockstepSession::resume(LOCAL_PLAYER, &lockstep, save.pending.clone());
        for ai in &ais {
            game.session.host_player(ai.player());
        }
        game.recorder = ReplayRecorder::resume(&save.replay);
        game.networked = false;
        if let Some(ai) = ais.first() {
            self.ai_opponent = GString::from(ai.difficulty().to_string().as_str());
        }
        game.ais = ais;
        self.loaded_settings = save.settings;
        self.loaded_state = Some((game.sim.tick(), game.sim.state_hash()));
        godot_print!(
            "saved game loaded: map '{}', tick {}, {} computer player(s)",
            save.info.map,
            save.info.ticks,
            game.ais.len()
        );
        Ok(())
    }
}
