//! Pathfinding benchmark: many units with simultaneous path requests on a
//! large map, timed per tick.

use sim::{
    Catalog, Command, Fixed, Map, PlayerCommand, PlayerId, RouteView, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2,
};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    pub units: u32,
    /// Units per move order (1 = every unit plans its own path).
    pub group: u32,
    pub ticks: u64,
    pub seed: u64,
    /// Targets lie within this distance of the map center per axis (m).
    pub range_m: i32,
}

/// Half tanks, half infantry squads (both size classes).
const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];
const SPACING_M: i32 = 10;

pub fn run(map: Arc<Map>, options: &Options) -> Result<String, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let unit_types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;

    // Units in a square grid around the map center (the start meadow).
    let columns = (1..).find(|c| c * c >= options.units).unwrap_or(1);
    let half = i32::try_from(columns).unwrap_or(1) * SPACING_M / 2;
    let spawns = (0..options.units)
        .map(|i| {
            let (col, row) = (i % columns, i / columns);
            let at = |k: u32| i32::try_from(k).unwrap_or(0) * SPACING_M - half + SPACING_M / 2;
            Spawn {
                owner: PlayerId(0),
                unit_type: TYPES[(i % 2) as usize].into(),
                position: Vec2::from_meters(at(col), at(row)),
                fire_mode: sim::FireMode::default(),
            }
        })
        .collect();
    let config = SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns,
        map: map.reference(),
        terrain,
        combat: sim::CombatRules::builtin().map_err(|e| e.to_string())?,
        vision: sim::VisionRules::builtin().map_err(|e| e.to_string())?,
        infantry: sim::InfantryRules::builtin().map_err(|e| e.to_string())?,
        teams: sim::Teams::default(),
    };

    let build = Instant::now();
    let mut sim = Sim::new(&config, map, options.seed).map_err(|e| e.to_string())?;
    let build = build.elapsed();
    let (cells_x, cells_y) = sim.path_grid().size();

    // One order per group of consecutive units, to random targets.
    let mut rng = sim::Rng::new(options.seed);
    let range_mm = options.range_m * 1000;
    let group = options.group.max(1);
    let orders: Vec<PlayerCommand> = (0..options.units)
        .step_by(group as usize)
        .map(|first| {
            let units = (first..(first + group).min(options.units))
                .map(UnitId)
                .collect();
            let target = Vec2::from_millimeters(
                rng.range_i32(-range_mm, range_mm),
                rng.range_i32(-range_mm, range_mm),
            );
            PlayerCommand::new(PlayerId(0), Command::MoveTo { units, target })
        })
        .collect();

    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let mut ticks: Vec<Duration> = Vec::new();
    let mut planned_at = None;
    let mut arrived_at = None;
    for tick in 0..options.ticks {
        let commands = if tick == 0 { orders.as_slice() } else { &[] };
        let start = Instant::now();
        sim.step(commands);
        let took = start.elapsed();
        if std::env::var_os("PATH_BENCH_VERBOSE").is_some() && took > Duration::from_millis(15) {
            eprintln!(
                "slow tick {tick}: {:.1} ms, stats {:?}",
                ms(took),
                sim.path_stats()
            );
        }
        ticks.push(took);
        let ids = (0..options.units).map(UnitId);
        let waiting = ids
            .clone()
            .filter(|&id| sim.route_of(id) == Some(RouteView::Waiting))
            .count();
        if planned_at.is_none() && waiting == 0 && sim.path_queue_len() == 0 {
            planned_at = Some(tick + 1);
        }
        if ids
            .clone()
            .all(|id| sim.route_of(id) == Some(RouteView::Idle))
        {
            arrived_at = Some(tick + 1);
            break;
        }
    }

    let stats = sim.path_stats();
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let summary = |label: &str, slice: &[Duration]| {
        if slice.is_empty() {
            return format!("  {label}: -\n");
        }
        let mut sorted = slice.to_vec();
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
    };
    let planning = planned_at.map_or(ticks.len(), |t| usize::try_from(t).unwrap_or(0));
    let snapshot = sim.snapshot();
    let start_of = |i: usize| config.spawns[i].position;
    let moved = snapshot
        .units
        .iter()
        .enumerate()
        .filter(|(i, u)| (u.position - start_of(*i)).length() > Fixed::from_num(SPACING_M))
        .count();

    let mut out = String::new();
    let _ = writeln!(
        out,
        "path-bench: map '{}', {} units ({} orders of {group}), seed {}, targets within ±{} m",
        sim.map().name(),
        options.units,
        orders.len(),
        options.seed,
        options.range_m
    );
    let _ = writeln!(
        out,
        "  grid {cells_x} x {cells_y} cells of {} m, {} profiles, built in {:.1} ms (Sim::new total)",
        sim.path_grid().cell_size(),
        TYPES.len(),
        ms(build)
    );
    let _ = writeln!(
        out,
        "  requests {} (found {}, failed {}, dropped {}), nodes expanded {} ({:.0} per request), budget {} per tick",
        stats.requests,
        stats.found,
        stats.failed,
        stats.dropped,
        stats.expanded,
        stats.expanded as f64 / stats.requests.max(1) as f64,
        sim::path::NODE_BUDGET_PER_TICK
    );
    let _ = writeln!(
        out,
        "  all paths planned after {} ticks ({:.1} s game time); all units idle after {}",
        planned_at.map_or_else(|| "-".into(), |t| t.to_string()),
        planned_at.map_or(f64::NAN, |t| t as f64 / 20.0),
        arrived_at.map_or_else(
            || format!("- (stopped at {} ticks)", ticks.len()),
            |t| format!("{t} ticks ({:.0} s)", t as f64 / 20.0)
        )
    );
    out.push_str(&summary(
        "planning ticks",
        &ticks[..planning.min(ticks.len())],
    ));
    out.push_str(&summary("all ticks", &ticks));
    let _ = write!(
        out,
        "  units moved more than {SPACING_M} m: {moved} of {}",
        options.units
    );
    if std::env::var_os("PATH_BENCH_VERBOSE").is_some() {
        for u in &snapshot.units {
            let route = sim.route_of(u.id);
            if route != Some(RouteView::Idle) {
                let _ = write!(
                    out,
                    "\n    unit {} type {} at ({}, {}) target {:?}: {:?}",
                    u.id.0,
                    u.unit_type,
                    u.position.x,
                    u.position.y,
                    u.target.map(|t| (t.x.to_num::<f32>(), t.y.to_num::<f32>())),
                    route.map(|r| match r {
                        RouteView::Following(w) => format!(
                            "following, {} left, next {:?}",
                            w.len(),
                            (w[0].x.to_num::<f32>(), w[0].y.to_num::<f32>())
                        ),
                        other => format!("{other:?}"),
                    })
                );
            }
        }
    }
    Ok(out)
}
