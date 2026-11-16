//! Crater benchmark (prompt 021): many craters in a short time on a large
//! map with moving units of two players, timed per tick.

use sim::{
    Catalog, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2,
};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    /// Units per player.
    pub units: u32,
    /// Craters in all.
    pub craters: u32,
    /// Craters per tick (the rules allow at most 16).
    pub per_tick: u32,
    /// Craters land within this distance of the map center per axis (m).
    pub range_m: i32,
    pub seed: u64,
}

const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];
const SPACING_M: i32 = 12;
const KWK_MM_X10: u32 = 750;
/// Ticks of driving before the first crater (paths planned).
const WARM_UP: u64 = 60;
/// Ticks after the last crater (vision queue drains).
const COOL_DOWN: u64 = 60;

pub fn run(map: Arc<Map>, options: &Options) -> Result<String, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let unit_types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;
    let columns = (1..).find(|c| c * c >= options.units).unwrap_or(1);
    let mut spawns = Vec::new();
    for (player, offset) in [(PlayerId(0), -150), (PlayerId(1), 150)] {
        for i in 0..options.units {
            let (col, row) = (
                i32::try_from(i % columns).unwrap_or(0),
                i32::try_from(i / columns).unwrap_or(0),
            );
            spawns.push(Spawn {
                owner: player,
                unit_type: TYPES[(i % 2) as usize].into(),
                position: Vec2::from_meters(offset + col * SPACING_M, row * SPACING_M - 60),
                fire_mode: FireMode::HoldFire,
            });
        }
    }
    let config = SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: TerrainRules::builtin().map_err(|e| e.to_string())?,
        combat: sim::CombatRules::builtin().map_err(|e| e.to_string())?,
        vision: sim::VisionRules::builtin().map_err(|e| e.to_string())?,
        infantry: sim::InfantryRules::builtin().map_err(|e| e.to_string())?,
        teams: sim::Teams::default(),
    };
    let mut sim = Sim::new(&config, map, options.seed).map_err(|e| e.to_string())?;

    // Both players drive across the crater field.
    let total = options.units * 2;
    let orders: Vec<PlayerCommand> = (0..total)
        .step_by(10)
        .map(|first| {
            let player = PlayerId(u8::from(first >= options.units));
            let x = if player.0 == 0 { 300 } else { -300 };
            PlayerCommand::new(
                player,
                Command::MoveTo {
                    units: (first..(first + 10).min(total)).map(UnitId).collect(),
                    target: Vec2::from_meters(x, i32::try_from(first).unwrap_or(0) - 100),
                },
            )
        })
        .collect();
    sim.step(&orders);
    let mut baseline = Vec::new();
    for _ in 0..WARM_UP {
        let t = Instant::now();
        sim.step(&[]);
        baseline.push(t.elapsed());
    }

    let mut rng = sim::Rng::new(options.seed);
    let range = options.range_m * 1000;
    let per_tick = options.per_tick.max(1);
    let mut left = options.craters;
    let mut with = Vec::new();
    let mut after = Vec::new();
    let replans = sim.path_stats().requests;
    let start = sim.crater_count();
    while left > 0 || after.len() < COOL_DOWN as usize {
        let n = left.min(per_tick);
        for _ in 0..n {
            let p =
                Vec2::from_millimeters(rng.range_i32(-range, range), rng.range_i32(-range, range));
            sim.queue_crater(p, KWK_MM_X10);
        }
        left -= n;
        let t = Instant::now();
        sim.step(&[]);
        if n > 0 {
            with.push(t.elapsed());
        } else {
            after.push(t.elapsed());
        }
    }
    let made = sim.crater_count() - start;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{} units, {made} of {} craters at {per_tick} per tick ({} ticks, {:.2} s game time), within ±{} m",
        total,
        options.craters,
        with.len(),
        with.len() as f64 / f64::from(SimConfig::DEFAULT_TICK_HZ),
        options.range_m,
    );
    let _ = writeln!(out, "ticks before:      {}", stats(&baseline));
    let _ = writeln!(
        out,
        "first crater tick: {:.2} ms (copies the map once)",
        ms(with[0])
    );
    let _ = writeln!(
        out,
        "ticks with craters (without the first): {}",
        stats(&with[1..])
    );
    let _ = writeln!(out, "ticks after:       {}", stats(&after));
    let _ = writeln!(
        out,
        "path requests during and after: {}",
        sim.path_stats().requests - replans
    );
    let _ = writeln!(out, "state hash {:#018x}", sim.state_hash());
    Ok(out)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn stats(samples: &[Duration]) -> String {
    if samples.is_empty() {
        return "-".into();
    }
    let mut v: Vec<f64> = samples.iter().map(|&d| ms(d)).collect();
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let p99 = v[(v.len() * 99 / 100).min(v.len() - 1)];
    format!(
        "mean {mean:.2} ms, median {:.2} ms, p99 {p99:.2} ms, max {:.2} ms",
        v[v.len() / 2],
        v[v.len() - 1]
    )
}
