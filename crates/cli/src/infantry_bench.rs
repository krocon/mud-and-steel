//! Infantry benchmark (prompt 022): 56 rifle squads (504 soldiers) of two
//! players on `test_small` (`data/scenarios/infantry_bench.toml`). Phases:
//! standing (hold fire), marching towards each other, fighting. The time
//! per tick is measured per phase.

use sim::scenario::{builtin_map, parse_scenario};
use sim::{
    Catalog, Command, FireMode, PlayerCommand, PlayerId, Scale, Sim, TerrainRules, UnitId, Vec2,
};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

const SCENARIO: &str = include_str!("../../../data/scenarios/infantry_bench.toml");

pub struct Options {
    pub ticks: u64,
    pub seed: u64,
}

/// Tick at which the squads start marching, and at which they open fire.
const MARCH_AT: u64 = 100;
const FIRE_AT: u64 = 400;
/// Squads per move order.
const GROUP: usize = 4;

fn summary(label: &str, ticks: &[Duration]) -> String {
    if ticks.is_empty() {
        return format!("  {label}: no ticks\n");
    }
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let mut sorted = ticks.to_vec();
    sorted.sort();
    let total: Duration = sorted.iter().sum();
    let pick = |q: usize| sorted[(sorted.len() - 1) * q / 100];
    format!(
        "  {label}: {} ticks, mean {:.3} ms, median {:.3} ms, p99 {:.3} ms, max {:.3} ms\n",
        sorted.len(),
        ms(total) / sorted.len() as f64,
        ms(pick(50)),
        ms(pick(99)),
        ms(sorted[sorted.len() - 1]),
    )
}

pub fn run(options: &Options) -> Result<String, String> {
    let map = builtin_map();
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let mut config =
        parse_scenario(SCENARIO, &catalog, &scale, &terrain, &map).map_err(|e| e.to_string())?;
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    let build = Instant::now();
    let mut sim = Sim::new(&config, map, options.seed).map_err(|e| e.to_string())?;
    let build = build.elapsed();
    let soldiers = sim.snapshot().soldiers.len();

    let units_of = |player: u8| -> Vec<UnitId> {
        (0u32..)
            .zip(&config.spawns)
            .filter(|(_, s)| s.owner == PlayerId(player))
            .map(|(i, _)| UnitId(i))
            .collect()
    };
    let mut march = Vec::new();
    let mut fire = Vec::new();
    for player in 0..2u8 {
        let units = units_of(player);
        let y = if player == 0 { 25 } else { -25 };
        for (k, chunk) in units.chunks(GROUP).enumerate() {
            let x = (i32::try_from(k).unwrap_or(0) - 3) * 22;
            march.push(PlayerCommand::new(
                PlayerId(player),
                Command::MoveTo {
                    units: chunk.to_vec(),
                    target: Vec2::from_meters(x, y),
                },
            ));
        }
        fire.push(PlayerCommand::new(
            PlayerId(player),
            Command::SetFireMode {
                units,
                mode: FireMode::FireAtWill,
            },
        ));
    }

    let mut phases: [Vec<Duration>; 4] = Default::default();
    let mut projectiles = 0usize;
    let mut most_projectiles = 0usize;
    for tick in 0..options.ticks {
        let commands: &[PlayerCommand] = match tick {
            MARCH_AT => &march,
            FIRE_AT => &fire,
            _ => &[],
        };
        let start = Instant::now();
        sim.step(commands);
        let elapsed = start.elapsed();
        let phase = if tick < MARCH_AT {
            0
        } else if tick < FIRE_AT {
            1
        } else if tick < FIRE_AT + 200 {
            2
        } else {
            3
        };
        phases[phase].push(elapsed);
        let flying = sim.snapshot().projectiles.len();
        projectiles += flying;
        most_projectiles = most_projectiles.max(flying);
    }
    let end = sim.snapshot();
    let alive = end.soldiers.iter().filter(|s| s.hp > 0).count();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "infantry-bench: map 'test_small', {} squads, {soldiers} soldiers, {} ticks, seed {}",
        config.spawns.len(),
        options.ticks,
        options.seed
    );
    let _ = writeln!(out, "  Sim::new {:.1} ms", build.as_secs_f64() * 1e3);
    out.push_str(&summary("standing (hold fire)", &phases[0]));
    out.push_str(&summary("marching            ", &phases[1]));
    out.push_str(&summary("first 10 s of fire  ", &phases[2]));
    out.push_str(&summary("after that          ", &phases[3]));
    let _ = write!(
        out,
        "  projectiles in flight: mean {:.0}, max {most_projectiles}; soldiers alive at the end: {alive} of {soldiers}",
        projectiles as f64 / options.ticks.max(1) as f64
    );
    Ok(out)
}
