//! `cli bench` (prompt 016): the overall performance benchmark. A scenario
//! (default `data/scenarios/bench_large.toml`: 504 units, 2250 soldiers on
//! the 4 km map) is played by two computer players through a lockstep
//! session, exactly like `ai-tournament`; every tick times
//!
//! - `Sim::step` (path planning, combat, movement, infantry, projectiles,
//!   destruction, vision),
//! - the AIs' think (including their fog-of-war views),
//! - the snapshot the bridge builds for the local player (`snapshot_for`).
//!
//! The end hash makes the run a behavior check too: an optimization must
//! not change it (docs/PERFORMANCE.md).

use ai::Difficulty;
use ai::game::{Game, GameResult};
use sim::{Map, Phase, PlayerId, Sim, SimConfig};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    pub ticks: u64,
    pub seed: u64,
    pub ai: Difficulty,
    /// Number of tick windows reported separately.
    pub windows: u64,
    /// Play-throughs; the report takes the minimum per tick.
    pub runs: u32,
}

/// Timings of one tick.
#[derive(Clone, Copy, Default)]
struct Sample {
    step: Duration,
    think: Duration,
    snapshot: Duration,
    alive: usize,
    projectiles: usize,
    phases: [Duration; PHASES],
}

const PHASES: usize = Phase::ALL.len();

/// Mean, median, p99 and max of a series, in milliseconds.
struct Stats {
    mean: f64,
    median: f64,
    p99: f64,
    max: f64,
}

impl Stats {
    fn of(values: impl Iterator<Item = Duration>) -> Self {
        let mut sorted: Vec<Duration> = values.collect();
        if sorted.is_empty() {
            return Self {
                mean: 0.0,
                median: 0.0,
                p99: 0.0,
                max: 0.0,
            };
        }
        sorted.sort();
        let ms = |d: Duration| d.as_secs_f64() * 1e3;
        let total: Duration = sorted.iter().sum();
        let pick = |q: usize| ms(sorted[(sorted.len() - 1) * q / 100]);
        Self {
            mean: ms(total) / sorted.len() as f64,
            median: pick(50),
            p99: pick(99),
            max: ms(sorted[sorted.len() - 1]),
        }
    }

    fn cells(&self) -> String {
        format!(
            "{:7.3} | {:7.3} | {:7.3} | {:7.3}",
            self.mean, self.median, self.p99, self.max
        )
    }
}

/// One play-through of the scenario.
struct Run {
    build: Duration,
    total: Duration,
    samples: Vec<Sample>,
    result: GameResult,
    /// Path requests of the run (prompt 040: soft ground makes searches
    /// longer).
    paths: sim::path::PathStats,
}

fn play(config: &SimConfig, map: &Arc<Map>, options: &Options) -> Result<Run, String> {
    let players = config.players();
    let ais: Vec<(PlayerId, Difficulty)> = players.iter().map(|&p| (p, options.ai)).collect();
    let local = players.first().copied().unwrap_or(PlayerId(0));

    let build = Instant::now();
    let mut game = Game::new(config, Arc::clone(map), options.seed, &ais)?;
    let build = build.elapsed();

    let mut samples: Vec<Sample> = Vec::with_capacity(options.ticks as usize);
    let total = Instant::now();
    while game.sim().tick() < options.ticks {
        let start = Instant::now();
        let decisions = game.think();
        let think = start.elapsed();
        let start = Instant::now();
        let mut phases = [Duration::ZERO; PHASES];
        let mut last = start;
        game.step_observed(decisions, |phase| {
            let now = Instant::now();
            phases[phase as usize] = now - last;
            last = now;
        });
        let step = start.elapsed();
        let start = Instant::now();
        let snapshot = game.sim().snapshot_for(local);
        let snapshot_time = start.elapsed();
        let alive = players.iter().map(|&p| game.sim().alive_units(p)).sum();
        samples.push(Sample {
            step,
            think,
            snapshot: snapshot_time,
            phases,
            alive,
            projectiles: snapshot.projectiles.len(),
        });
        if game.sim().outcome().is_some() {
            break;
        }
    }
    Ok(Run {
        build,
        total: total.elapsed(),
        samples,
        paths: game.sim().path_stats(),
        result: game.result(),
    })
}

/// Plays the scenario `options.runs` times and returns the report. Every
/// run simulates exactly the same ticks (the simulation and the AIs are
/// deterministic), so the minimum per tick over the runs removes most of
/// the noise of a busy machine (other processes preempting the bench);
/// the runs must end with the same hash.
pub fn run(config: &SimConfig, map: Arc<Map>, options: &Options) -> Result<String, String> {
    let players = config.players();
    let mut runs = Vec::new();
    for _ in 0..options.runs.max(1) {
        runs.push(play(config, &map, options)?);
    }
    let first = &runs[0];
    if let Some(other) = runs.iter().find(|r| r.result != first.result) {
        return Err(format!(
            "runs differ: end hash {:#018x} vs {:#018x} (nondeterminism)",
            first.result.end_hash, other.result.end_hash
        ));
    }
    let mut samples = first.samples.clone();
    for run in &runs[1..] {
        for (s, o) in samples.iter_mut().zip(&run.samples) {
            s.step = s.step.min(o.step);
            s.think = s.think.min(o.think);
            s.snapshot = s.snapshot.min(o.snapshot);
            for (p, q) in s.phases.iter_mut().zip(&o.phases) {
                *p = (*p).min(*q);
            }
        }
    }
    let build = runs.iter().map(|r| r.build).min().unwrap_or_default();
    let total = runs.iter().map(|r| r.total).min().unwrap_or_default();
    let result = &first.result;
    let units = config.spawns.len();
    let soldiers = Game::new(config, Arc::clone(&map), options.seed, &[])?
        .sim()
        .snapshot()
        .soldiers
        .len();
    let stuck = stuck_spawns(
        &Sim::new(config, Arc::clone(&map), options.seed).map_err(|e| e.to_string())?,
        config,
    );

    let mut out = String::new();
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let _ = writeln!(
        out,
        "bench: map '{}', {units} units ({soldiers} soldiers) of {} players, AI {}, seed {}, {} ticks ({:.0} s game time)",
        map.name(),
        players.len(),
        options.ai.name(),
        options.seed,
        samples.len(),
        samples.len() as f64 / f64::from(config.tick_hz),
    );
    let _ = writeln!(
        out,
        "  setup (Sim::new, viewsheds, AIs) {:.0} ms; run {:.1} s; minimum per tick of {} runs; spawns on cells blocked for their class: {stuck}",
        ms(build),
        total.as_secs_f64(),
        runs.len()
    );
    let _ = writeln!(
        out,
        "  end: tick {}, units alive {:?}, winner {:?}, hash {:#018x}",
        result.ticks,
        result.alive,
        result.winner.map(|p| p.0),
        result.end_hash
    );
    let p = &first.paths;
    let _ = writeln!(
        out,
        "  paths: {} requests, {} found, {} failed, {} dropped, {} nodes expanded",
        p.requests, p.found, p.failed, p.dropped, p.expanded
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| ms per tick | mean | median | p99 | max |\n|---|---|---|---|---|"
    );
    let row = |label: &str, f: &dyn Fn(&Sample) -> Duration| {
        format!(
            "| {label} | {} |\n",
            Stats::of(samples.iter().map(f)).cells()
        )
    };
    out.push_str(&row("sim step", &|s| s.step));
    out.push_str(&row("AI think (all)", &|s| s.think));
    out.push_str(&row("snapshot", &|s| s.snapshot));
    out.push_str(&row("total", &|s| s.step + s.think + s.snapshot));
    for phase in Phase::ALL {
        out.push_str(&row(&format!("· {}", phase.name()), &|s| {
            s.phases[phase as usize]
        }));
    }

    // Where the slowest ticks spend their time.
    let mut slow: Vec<usize> = (0..samples.len()).collect();
    slow.sort_by_key(|&i| std::cmp::Reverse(samples[i].step));
    let _ = writeln!(out);
    let _ = write!(out, "| slowest ticks | step |");
    for phase in Phase::ALL {
        let _ = write!(out, " {} |", phase.name());
    }
    let _ = writeln!(out, "\n|{}", "---|".repeat(PHASES + 2));
    for &i in slow.iter().take(5) {
        let _ = write!(out, "| {} | {:.2} |", i, ms(samples[i].step));
        for phase in Phase::ALL {
            let _ = write!(out, " {:.2} |", ms(samples[i].phases[phase as usize]));
        }
        let _ = writeln!(out);
    }

    // The match changes over time (approach, battle, mopping up): every
    // window separately.
    let windows = options.windows.max(1) as usize;
    let per = samples.len().div_ceil(windows).max(1);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| ticks | alive | projectiles (max) | step mean | step p99 | step max | think mean | snapshot mean |\n|---|---|---|---|---|---|---|---|"
    );
    for (k, chunk) in samples.chunks(per).enumerate() {
        let step = Stats::of(chunk.iter().map(|s| s.step));
        let think = Stats::of(chunk.iter().map(|s| s.think));
        let snap = Stats::of(chunk.iter().map(|s| s.snapshot));
        let last = chunk.last().copied().unwrap_or_default();
        let _ = writeln!(
            out,
            "| {}–{} | {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |",
            k * per,
            k * per + chunk.len(),
            last.alive,
            chunk.iter().map(|s| s.projectiles).max().unwrap_or(0),
            step.mean,
            step.p99,
            step.max,
            think.mean,
            snap.mean,
        );
    }
    Ok(out)
}

/// Spawns that stand on a path cell their unit cannot drive on (a badly
/// placed scenario would measure stuck units).
fn stuck_spawns(sim: &Sim, config: &SimConfig) -> usize {
    config
        .spawns
        .iter()
        .filter(|s| {
            config
                .unit_types
                .iter()
                .position(|t| t.name == s.unit_type)
                .is_some_and(|profile| !sim.path_grid().passable_for(profile, s.position))
        })
        .count()
}
