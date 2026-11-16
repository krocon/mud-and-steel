//! `cli save-bench` (prompt 038): what loading a saved game costs. Two
//! computer players play a scenario; at the end the match is
//!
//! - re-simulated from its replay (loading by replay alone),
//! - written as a snapshot and restored from it (loading by snapshot),
//! - written as a whole saved game (replay, snapshot, AI state).
//!
//! The numbers decided how saved games load (docs/ARCHITECTURE.md, "Saved
//! games and replay viewer").

use ai::Difficulty;
use ai::game::Game;
use net::SaveGame;
use sim::{Map, PlayerId, SimConfig};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Instant;

pub struct Options {
    pub ticks: u64,
    pub seed: u64,
    pub ai: Difficulty,
}

pub fn run(config: &SimConfig, map: Arc<Map>, options: &Options) -> Result<String, String> {
    let players: Vec<(PlayerId, Difficulty)> = config
        .players()
        .into_iter()
        .map(|p| (p, options.ai))
        .collect();
    // First, while nothing has computed it yet.
    let began = Instant::now();
    let rules = net::snapshot::rules_hash();
    let rules_time = began.elapsed();
    let mut game = Game::new(config, Arc::clone(&map), options.seed, &players)?;
    let began = Instant::now();
    game.run(options.ticks);
    let played = began.elapsed();
    let ticks = game.sim().tick();

    let replay = game.replay();
    let replay_bytes = replay.encode();
    let began = Instant::now();
    let report = replay.verify(Arc::clone(&map)).map_err(|e| e.to_string())?;
    let resim = began.elapsed();
    if !report.matches() {
        return Err(format!("replay does not play back: {report:?}"));
    }

    let began = Instant::now();
    let snapshot = net::snapshot::encode(game.sim(), &map);
    let encode = began.elapsed();
    let began = Instant::now();
    let restored = net::snapshot::decode(&snapshot, Arc::clone(&map)).map_err(|e| e.to_string())?;
    let decode = began.elapsed();
    assert_eq!(restored.state_hash(), game.sim().state_hash());

    let began = Instant::now();
    let save = game.save(&map).encode();
    let save_time = began.elapsed();
    let began = Instant::now();
    let loaded = Game::load(
        &SaveGame::decode(&save).map_err(|e| e.to_string())?,
        Arc::clone(&map),
    )?;
    let load_time = began.elapsed();
    assert_eq!(loaded.sim().state_hash(), game.sim().state_hash());

    let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
    let kb = |n: usize| n as f64 / 1024.0;
    let seconds = ticks as f64 / f64::from(config.tick_hz.max(1));
    let mut out = String::new();
    let _ = writeln!(
        out,
        "map {} ({} units at the start), {} ticks = {:.0} s game time, seed {}, AI {}",
        config.map.name,
        config.spawns.len(),
        ticks,
        seconds,
        options.seed,
        options.ai
    );
    let _ = writeln!(out, "played with AIs:         {:9.1} ms", ms(played));
    let _ = writeln!(
        out,
        "re-simulate the replay:  {:9.1} ms ({:.3} ms per tick; replay {:.1} KB)",
        ms(resim),
        ms(resim) / ticks.max(1) as f64,
        kb(replay_bytes.len())
    );
    let _ = writeln!(
        out,
        "snapshot write:          {:9.1} ms ({:.1} KB compressed)",
        ms(encode),
        kb(snapshot.len())
    );
    let _ = writeln!(out, "snapshot restore:        {:9.1} ms", ms(decode));
    let _ = writeln!(
        out,
        "saved game write / load: {:9.1} ms / {:.1} ms ({:.1} KB)",
        ms(save_time),
        ms(load_time),
        kb(save.len())
    );
    let _ = writeln!(
        out,
        "rules hash:              {:9.1} ms ({rules:#018x}, once per process)",
        ms(rules_time)
    );
    let _ = writeln!(
        out,
        "an hour of this match re-simulates in about {:.0} s",
        ms(resim) / ticks.max(1) as f64 * 72_000.0 / 1e3
    );
    Ok(out)
}
