//! The replay viewer (prompt 038): `ReplayPlayer` holds a decoded replay and
//! the snapshots taken while it plays; `SimHost.start_replay` plays it from
//! any tick, seen by one player (with that player's fog of war) or with
//! everything visible.
//!
//! A jump starts the match anew from the nearest earlier snapshot and plays
//! the rest headless; `main.gd` rebuilds the scene for it, because terrain,
//! craters and map objects are built from the simulation at the start.

use crate::{Game, SimHost};
use godot::prelude::*;
use net::{LocalTransport, LockstepConfig, Replay, ReplayInfo, SaveGame};
use sim::scenario::{BUILTIN_MAP_NAME, builtin_map};
use sim::{Map, PlayerId, Sim};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

/// A snapshot is kept every this many ticks of playback (30 s at 20 Hz).
pub(crate) const KEYFRAME_TICKS: u64 = 600;

/// Ticks per frame while a replay plays: 8× at 20 Hz is 160 ticks per
/// second, which 5 ticks per frame reach only above 32 FPS.
pub(crate) const MAX_REPLAY_TICKS_PER_FRAME: u32 = 20;

/// The map `name` (built in, or `maps_dir/<name>/`).
pub(crate) fn load_named_map(name: &str, maps_dir: &str) -> Result<Arc<Map>, String> {
    if name == BUILTIN_MAP_NAME {
        return Ok(builtin_map());
    }
    crate::load_map_dir(&Path::new(maps_dir).join(name))
        .map(Arc::new)
        .map_err(|e| format!("map '{name}' is not available: {e}"))
}

/// A replay opened for viewing, with the snapshots taken so far. Kept by
/// the menu across scene changes (jumps rebuild the scene).
#[derive(GodotClass)]
#[class(init, base = RefCounted)]
pub struct ReplayPlayer {
    replay: Option<Arc<Replay>>,
    map: Option<Arc<Map>>,
    /// Encoded snapshots (`net::snapshot`) by tick.
    keyframes: BTreeMap<u64, Vec<u8>>,
}

#[godot_api]
impl ReplayPlayer {
    /// Decodes replay file contents and loads its map from
    /// `maps_dir/<name>/` (or built in). Returns "" or why it cannot be
    /// played.
    #[func]
    fn open(&mut self, bytes: PackedByteArray, maps_dir: GString) -> GString {
        let result = Replay::decode(bytes.as_slice())
            .map_err(|e| e.to_string())
            .and_then(|replay| {
                let map = load_named_map(&replay.config.map.name, &maps_dir.to_string())?;
                if map.reference() != replay.config.map {
                    return Err(format!(
                        "map '{}' has changed since the replay was recorded",
                        replay.config.map.name
                    ));
                }
                Ok((replay, map))
            });
        match result {
            Ok((replay, map)) => {
                self.replay = Some(Arc::new(replay));
                self.map = Some(map);
                self.keyframes.clear();
                GString::new()
            }
            Err(e) => GString::from(e.as_str()),
        }
    }

    /// Info of the open replay (see `read_replay_info`).
    #[func]
    fn info(&self) -> VarDictionary {
        self.replay
            .as_ref()
            .map_or_else(VarDictionary::new, |r| info_dict(&r.info))
    }

    /// Ticks of the open replay (0 if none).
    #[func]
    fn ticks(&self) -> i64 {
        self.replay.as_ref().map_or(0, |r| tick_i64(r.ticks))
    }

    /// State hash at the end of the open replay, as hex string (compare with
    /// `SimHost.state_hash` at the end of playback).
    #[func]
    fn end_hash(&self) -> GString {
        self.replay.as_ref().map_or_else(GString::new, |r| {
            GString::from(format!("{:#018x}", r.end_hash).as_str())
        })
    }

    /// Ticks of the snapshots taken so far.
    #[func]
    fn keyframe_ticks(&self) -> PackedInt64Array {
        self.keyframes.keys().map(|&t| tick_i64(t)).collect()
    }

    /// Info at the start of replay file contents, for the replay list:
    /// `ok` (readable), `playable` (this build can play it), `error`, and
    /// when readable `created`, `map`, `game_version`, `ticks`, `seconds`,
    /// `players` (array of `id`, `faction`, `kind`), `decided`, `winner`
    /// (-1: draw or undecided), `multiplayer`.
    #[func]
    fn read_replay_info(bytes: PackedByteArray) -> VarDictionary {
        file_info(
            Replay::read_info(bytes.as_slice()).map_err(|e| e.to_string()),
            || {
                Replay::decode(bytes.as_slice())
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            },
        )
    }

    /// The same for saved-game file contents (`ticks` is the save tick).
    #[func]
    fn read_save_info(bytes: PackedByteArray) -> VarDictionary {
        file_info(
            SaveGame::read_info(bytes.as_slice()).map_err(|e| e.to_string()),
            || {
                SaveGame::read_info(bytes.as_slice())
                    .map_err(|e| e.to_string())
                    .and_then(|info| info.check_rules().map_err(|e| e.to_string()))
            },
        )
    }
}

impl ReplayPlayer {
    fn add_keyframe(&mut self, sim: &Sim) {
        if let Some(map) = &self.map
            && !self.keyframes.contains_key(&sim.tick())
        {
            self.keyframes
                .insert(sim.tick(), net::snapshot::encode(sim, map));
        }
    }

    /// The simulation at `tick` (clamped to the replay): from the nearest
    /// earlier snapshot, the rest played headless.
    fn sim_at(&mut self, tick: u64) -> Result<Sim, String> {
        let (Some(replay), Some(map)) = (self.replay.clone(), self.map.clone()) else {
            return Err("no replay open".into());
        };
        let tick = tick.min(replay.ticks);
        let mut sim = match self.keyframes.range(..=tick).next_back() {
            Some((_, bytes)) => {
                net::snapshot::decode(bytes, map.clone()).map_err(|e| e.to_string())?
            }
            None => Sim::new(&replay.config, map, replay.seed).map_err(|e| e.to_string())?,
        };
        let mut next = first_frame_at(&replay, sim.tick());
        while sim.tick() < tick {
            next = step(&replay, next, &mut sim);
            if sim.tick().is_multiple_of(KEYFRAME_TICKS) {
                self.add_keyframe(&sim);
            }
        }
        Ok(sim)
    }
}

/// Index of the first frame at or after `tick`.
fn first_frame_at(replay: &Replay, tick: u64) -> usize {
    replay.frames.partition_point(|f| f.tick < tick)
}

/// Steps `sim` by one tick with the replay's commands; returns the index of
/// the next frame.
fn step(replay: &Replay, next: usize, sim: &mut Sim) -> usize {
    match replay.frames.get(next).filter(|f| f.tick == sim.tick()) {
        Some(frame) => {
            sim.step(&frame.commands);
            next + 1
        }
        None => {
            sim.step(&[]);
            next
        }
    }
}

/// A replay playing in the `SimHost`.
pub(crate) struct Playback {
    player: Gd<ReplayPlayer>,
    replay: Arc<Replay>,
    next: usize,
    /// `None`: everything visible.
    view: Option<PlayerId>,
}

/// One tick of a playing replay (instead of the lockstep round). Returns
/// false at the end of the replay.
pub(crate) fn step_playback(game: &mut Game) -> bool {
    let Some(playback) = game.playback.as_mut() else {
        return false;
    };
    if game.sim.tick() >= playback.replay.ticks {
        return false;
    }
    playback.next = step(&playback.replay, playback.next, &mut game.sim);
    if game.sim.tick().is_multiple_of(KEYFRAME_TICKS) {
        playback.player.bind_mut().add_keyframe(&game.sim);
    }
    true
}

#[godot_api(secondary)]
impl SimHost {
    /// Plays the replay of `player` from `tick`, seen by player `view`
    /// (with its fog of war) or, for -1, with everything visible. Returns
    /// false and keeps the reason for `load_error` on failure.
    #[func]
    fn start_replay(&mut self, mut player: Gd<ReplayPlayer>, tick: i64, view: i64) -> bool {
        let result = {
            let mut bound = player.bind_mut();
            let tick = u64::try_from(tick).unwrap_or(0);
            bound.sim_at(tick).map(|sim| {
                (
                    sim,
                    bound.replay.clone().expect("open"),
                    bound.map.clone().expect("open"),
                )
            })
        };
        let (sim, replay, map) = match result {
            Ok(parts) => parts,
            Err(e) => {
                godot_error!("cannot play the replay: {e}");
                self.load_error = e;
                return false;
            }
        };
        let view = u8::try_from(view)
            .ok()
            .map(PlayerId)
            .filter(|p| replay.config.players().contains(p));
        let local = view
            .or_else(|| replay.config.players().first().copied())
            .unwrap_or(PlayerId(0));
        self.reveal_all = view.is_none();
        let lockstep = LockstepConfig {
            players: vec![local],
            input_delay: 1,
            hash_interval: 20,
        };
        let next = first_frame_at(&replay, sim.tick());
        self.install(
            sim,
            map,
            replay.config.clone(),
            replay.seed,
            local,
            &lockstep,
            Box::new(LocalTransport),
        );
        let game = self.game.as_mut().expect("installed");
        game.networked = false;
        game.playback = Some(Playback {
            player,
            replay,
            next,
            view,
        });
        self.load_error.clear();
        true
    }

    /// Whether a replay is playing (no commands, no saving).
    #[func]
    fn is_replay(&self) -> bool {
        self.game.as_ref().is_some_and(|g| g.playback.is_some())
    }

    /// State of the playing replay: `tick`, `ticks`, `ended` (bool), `view`
    /// (player id, -1 = everything visible). Empty without a replay.
    #[func]
    fn replay_status(&self) -> VarDictionary {
        let Some((game, playback)) = self
            .game
            .as_ref()
            .and_then(|g| g.playback.as_ref().map(|p| (g, p)))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "tick" => tick_i64(game.sim.tick()),
            "ticks" => tick_i64(playback.replay.ticks),
            "ended" => game.sim.tick() >= playback.replay.ticks,
            "view" => playback.view.map_or(-1, |p| i64::from(p.0)),
        }
    }

    /// The match so far as replay file contents with the info of the replay
    /// list; `created` is the local date and time. Empty while a replay
    /// plays or without a match.
    #[func]
    fn replay_file_bytes(&self, created: GString) -> PackedByteArray {
        self.game
            .as_ref()
            .filter(|g| g.playback.is_none())
            .map_or_else(PackedByteArray::new, |game| {
                PackedByteArray::from(
                    crate::save_funcs::replay_of(game, &created.to_string()).encode(),
                )
            })
    }
}

fn file_info(
    info: Result<ReplayInfo, String>,
    playable: impl FnOnce() -> Result<(), String>,
) -> VarDictionary {
    match info {
        Err(e) => vdict! { "ok" => false, "playable" => false, "error" => e },
        Ok(info) => {
            let mut dict = info_dict(&info);
            dict.set("ok", true);
            match playable() {
                Ok(()) => {
                    dict.set("playable", true);
                    dict.set("error", "");
                }
                Err(e) => {
                    dict.set("playable", false);
                    dict.set("error", e.as_str());
                }
            }
            dict
        }
    }
}

fn info_dict(info: &ReplayInfo) -> VarDictionary {
    let mut players = VarArray::new();
    for p in &info.players {
        let entry = vdict! {
            "id" => i64::from(p.id.0),
            "faction" => p.faction.as_str(),
            "kind" => p.kind.as_str(),
        };
        players.push(&entry.to_variant());
    }
    let mut dict = vdict! {
        "created" => info.created.as_str(),
        "map" => info.map.as_str(),
        "game_version" => info.game_version.as_str(),
        "ticks" => tick_i64(info.ticks),
        "seconds" => tick_i64(info.ticks / u64::from(info.tick_hz.max(1))),
        "decided" => info.decided,
        "winner" => info.winner.map_or(-1, |p| i64::from(p.0)),
        "multiplayer" => info.multiplayer,
    };
    dict.set("players", &players);
    dict
}

fn tick_i64(tick: u64) -> i64 {
    i64::try_from(tick).unwrap_or(i64::MAX)
}
