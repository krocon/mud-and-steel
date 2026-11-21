//! `cli duel`: one unit against one unit on open, flat ground (prompt 045).
//!
//! Unit B stands at the center of a flat, empty 1.2 km map, its hull facing
//! north. Unit A stands `distance` meters away at `angle` degrees off B's
//! hull direction (0: in front of B, 90: beside it, 180: behind it) and
//! faces B. Both fire at will and never move; game *i* uses seed
//! `seed + i` (dispersion, penetration rolls). A duel ends when one of them
//! is destroyed (a bailed-out crew does not count as alive) or after
//! `max_s` game seconds. The table shows how real catalog values play out
//! after the scale and combat rules, so a rule change can be measured
//! without the noise of AI decisions.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, FireMode, Map, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules,
    UnitType, Vec2,
};
use std::fmt::Write as _;
use std::sync::Arc;

pub struct Options {
    pub a: String,
    pub b: String,
    pub distances: Vec<u32>,
    pub angles: Vec<u32>,
    pub runs: u64,
    pub seed: u64,
    pub max_s: u64,
}

/// Results of one distance and angle.
pub struct Cell {
    pub distance: u32,
    pub angle: u32,
    pub a_wins: u64,
    pub b_wins: u64,
    pub undecided: u64,
    /// Mean game seconds of the decided duels (tenths).
    pub mean_ds: u64,
}

/// Flat 1.2 km × 1.2 km meadow around the origin, 4 m samples.
fn flat_map() -> Arc<Map> {
    let n = 301;
    let layout = MapLayout {
        name: "duel".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 4000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).expect("flat map"))
}

fn unit_type(catalog: &Catalog, scale: &Scale, id: &str) -> Result<UnitType, String> {
    let entry = catalog
        .get(id)
        .ok_or_else(|| format!("unknown catalog id '{id}'"))?;
    Ok(UnitType::from_catalog(entry, scale)
        .unwrap_or_else(|| UnitType::fixed_from_catalog(entry, scale, 0)))
}

/// Position of A for `distance` and `angle` (B at the origin, facing north,
/// i.e. towards negative y).
fn a_position(distance: u32, angle: u32) -> Vec2 {
    // Set-up geometry only (the CLI may use floats; the simulation gets
    // whole millimeters).
    let rad = f64::from(angle).to_radians();
    let d = f64::from(distance) * 1000.0;
    Vec2::from_millimeters(
        (d * rad.sin()).round() as i32,
        (-d * rad.cos()).round() as i32,
    )
}

fn duel(
    a: &str,
    b: &str,
    distance: u32,
    angle: u32,
    seed: u64,
    max_ticks: u64,
) -> Result<Option<(bool, u64)>, String> {
    let map = flat_map();
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let mut names = vec![a.to_string(), b.to_string()];
    names.dedup();
    let unit_types = names
        .iter()
        .map(|id| unit_type(&catalog, &scale, id))
        .collect::<Result<Vec<_>, _>>()?;
    let spawn = |owner: u8, unit: &str, position: Vec2| Spawn {
        owner: PlayerId(owner),
        unit_type: unit.into(),
        position,
        fire_mode: FireMode::FireAtWill,
    };
    let config = SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns: vec![
            spawn(0, a, a_position(distance, angle)),
            spawn(1, b, Vec2::ZERO),
        ],
        map: map.reference(),
        terrain: TerrainRules::builtin().map_err(|e| e.to_string())?,
        combat: CombatRules::builtin().map_err(|e| e.to_string())?,
        vision: sim::VisionRules::builtin().map_err(|e| e.to_string())?,
        infantry: sim::InfantryRules::builtin().map_err(|e| e.to_string())?,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let mut sim = Sim::new(&config, map, seed).map_err(|e| e.to_string())?;
    while sim.tick() < max_ticks {
        sim.step(&[]);
        let a_alive = sim.alive_units(PlayerId(0)) > 0;
        let b_alive = sim.alive_units(PlayerId(1)) > 0;
        match (a_alive, b_alive) {
            (true, false) => return Ok(Some((true, sim.tick()))),
            (false, true) => return Ok(Some((false, sim.tick()))),
            (false, false) => return Ok(None),
            (true, true) => {}
        }
    }
    Ok(None)
}

/// Plays every distance and angle `runs` times.
pub fn run(options: &Options) -> Result<Vec<Cell>, String> {
    let max_ticks = options.max_s * u64::from(SimConfig::DEFAULT_TICK_HZ);
    let mut jobs: Vec<(u32, u32)> = Vec::new();
    for &d in &options.distances {
        for &angle in &options.angles {
            jobs.push((d, angle));
        }
    }
    let results: Vec<Result<Cell, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|&(distance, angle)| {
                scope.spawn(move || {
                    let mut cell = Cell {
                        distance,
                        angle,
                        a_wins: 0,
                        b_wins: 0,
                        undecided: 0,
                        mean_ds: 0,
                    };
                    let mut ticks = 0u64;
                    for i in 0..options.runs {
                        match duel(
                            &options.a,
                            &options.b,
                            distance,
                            angle,
                            options.seed + i,
                            max_ticks,
                        )? {
                            Some((true, t)) => {
                                cell.a_wins += 1;
                                ticks += t;
                            }
                            Some((false, t)) => {
                                cell.b_wins += 1;
                                ticks += t;
                            }
                            None => cell.undecided += 1,
                        }
                    }
                    let decided = cell.a_wins + cell.b_wins;
                    cell.mean_ds = (ticks * 10)
                        .checked_div(decided * u64::from(SimConfig::DEFAULT_TICK_HZ))
                        .unwrap_or(0);
                    Ok(cell)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("duel thread"))
            .collect()
    });
    results.into_iter().collect()
}

/// The cells as a Markdown table.
pub fn table(options: &Options, cells: &[Cell]) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} (A) against {} (B), {} runs each; angle: where A stands relative to B's hull (0 front, 90 side, 180 rear)\n",
        options.a, options.b, options.runs
    );
    s.push_str("| Distance | Angle | A wins | B wins | undecided | mean time |\n");
    s.push_str("|---|---|---|---|---|---|\n");
    for c in cells {
        let pct = |n: u64| (n * 100).checked_div(options.runs).unwrap_or(0);
        let _ = writeln!(
            s,
            "| {} m | {}° | {} % | {} % | {} % | {}.{} s |",
            c.distance,
            c.angle,
            pct(c.a_wins),
            pct(c.b_wins),
            pct(c.undecided),
            c.mean_ds / 10,
            c.mean_ds % 10
        );
    }
    s
}
