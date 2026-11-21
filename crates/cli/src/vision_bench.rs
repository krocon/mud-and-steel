//! Vision benchmark (prompt 018): many units of two players drive across a
//! large map at once; the time per tick is measured with the vision rules
//! and with a sight range of one cell (almost no vision work) as baseline.

use sim::{
    Catalog, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2, VisionRules,
};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    pub units: u32,
    pub ticks: u64,
    pub seed: u64,
}

/// Half tanks, half infantry squads.
const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];
const SPACING_M: i32 = 12;
/// Units per move order.
const GROUP: u32 = 10;
/// Targets lie within this distance of the map center per axis (m).
const RANGE_M: i32 = 1500;

struct Run {
    build: Duration,
    ticks: Vec<Duration>,
    steps: Vec<u32>,
    max_queue: usize,
    cells: usize,
}

fn run_once(map: &Arc<Map>, options: &Options, vision: VisionRules) -> Result<Run, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let unit_types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;
    // Two blocks 400 m apart on both sides of the map center, holding fire
    // (combat is not measured here).
    let per_player = options.units.div_ceil(2);
    let columns = (1..).find(|c| c * c >= per_player).unwrap_or(1);
    let spawns = (0..options.units)
        .map(|i| {
            let player = i % 2;
            let k = i / 2;
            let (col, row) = (k % columns, k / columns);
            let at = |v: u32| i32::try_from(v).unwrap_or(0) * SPACING_M;
            let y0 = if player == 0 { 200 } else { -200 - at(columns) };
            Spawn {
                owner: PlayerId(player as u8),
                unit_type: TYPES[(k % 2) as usize].into(),
                position: Vec2::from_meters(at(col) - at(columns) / 2, y0 + at(row)),
                fire_mode: FireMode::HoldFire,
            }
        })
        .collect();
    let config = SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: TerrainRules::builtin().map_err(|e| e.to_string())?,
        combat: sim::CombatRules::builtin().map_err(|e| e.to_string())?,
        vision,
        infantry: sim::InfantryRules::builtin().map_err(|e| e.to_string())?,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let build = Instant::now();
    let mut sim = Sim::new(&config, Arc::clone(map), options.seed).map_err(|e| e.to_string())?;
    let build = build.elapsed();

    let mut rng = sim::Rng::new(options.seed);
    let range_mm = RANGE_M * 1000;
    let mut orders: Vec<PlayerCommand> = Vec::new();
    for player in 0..2u8 {
        let mine: Vec<u32> = (0..options.units)
            .filter(|i| i % 2 == u32::from(player))
            .collect();
        for chunk in mine.chunks(GROUP as usize) {
            let target = Vec2::from_millimeters(
                rng.range_i32(-range_mm, range_mm),
                rng.range_i32(-range_mm, range_mm),
            );
            orders.push(PlayerCommand::new(
                PlayerId(player),
                Command::MoveTo {
                    units: chunk.iter().copied().map(UnitId).collect(),
                    target,
                },
            ));
        }
    }
    let mut run = Run {
        build,
        ticks: Vec::new(),
        steps: Vec::new(),
        max_queue: 0,
        cells: sim.vision_grid().cells(),
    };
    for tick in 0..options.ticks {
        let commands = if tick == 0 { orders.as_slice() } else { &[] };
        let start = Instant::now();
        sim.step(commands);
        run.ticks.push(start.elapsed());
        run.steps.push(sim.vision_steps());
        run.max_queue = run.max_queue.max(sim.vision_queue_len());
    }
    Ok(run)
}

fn summary(label: &str, ticks: &[Duration]) -> String {
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let mut sorted = ticks.to_vec();
    sorted.sort();
    let total: Duration = sorted.iter().sum();
    let pick = |q: usize| sorted[(sorted.len() - 1) * q / 100];
    format!(
        "  {label}: mean {:.3} ms, median {:.3} ms, p99 {:.3} ms, max {:.3} ms\n",
        ms(total) / sorted.len().max(1) as f64,
        ms(pick(50)),
        ms(pick(99)),
        ms(sorted[sorted.len() - 1]),
    )
}

pub fn run(map: Arc<Map>, options: &Options) -> Result<String, String> {
    let rules = VisionRules::builtin().map_err(|e| e.to_string())?;
    let mut blind = rules.clone();
    // 40 m real = 10 m game: one cell.
    blind.sight.range_real_m = 40;
    blind.sight.categories.clear();
    let with = run_once(&map, options, rules.clone())?;
    let without = run_once(&map, options, blind)?;

    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "vision-bench: map '{}', {} units of 2 players (orders of {GROUP}, targets within ±{RANGE_M} m), {} ticks, seed {}",
        map.name(),
        options.units,
        options.ticks,
        options.seed
    );
    let _ = writeln!(
        out,
        "  vision grid {} cells of {} m, sight {} m (cells {}), budget {} ray steps per tick",
        with.cells,
        rules.cell_m(),
        rules.range_m(None),
        rules.range_cells(None),
        rules.grid.budget_steps_per_tick
    );
    let _ = writeln!(
        out,
        "  Sim::new with all viewsheds {:.1} ms (without: {:.1} ms)",
        ms(with.build),
        ms(without.build)
    );
    out.push_str(&summary("with vision   ", &with.ticks));
    out.push_str(&summary("one-cell sight", &without.ticks));
    let total_steps: u64 = with.steps.iter().map(|&s| u64::from(s)).sum();
    let busy = with.steps.iter().filter(|&&s| s > 0).count();
    let _ = write!(
        out,
        "  ray steps: mean {:.0} per tick, max {}, ticks with updates {} of {}, longest queue {}",
        total_steps as f64 / with.steps.len().max(1) as f64,
        with.steps.iter().max().copied().unwrap_or(0),
        busy,
        with.steps.len(),
        with.max_queue
    );
    Ok(out)
}
