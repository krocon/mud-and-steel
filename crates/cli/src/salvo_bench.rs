//! Salvo benchmark (prompt 085): a rocket or howitzer salvo into the
//! densest wood of a map among moving units of two players, timed per
//! tick; counts the trees it fells and strips.

use sim::{
    Catalog, Command, FireMode, Map, ObjectStage, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, Vec2,
};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    /// Units per player.
    pub units: u32,
    /// Rounds of the salvo and their caliber (mm × 10).
    pub rounds: u32,
    pub caliber_mm_x10: u32,
    /// Rounds land within this distance of the wood's center (m).
    pub radius_m: i32,
    /// Rounds fired per tick.
    pub per_tick: u32,
    pub seed: u64,
}

const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];
const SPACING_M: i32 = 12;
/// Edge of the squares the densest wood is searched in (m).
const WOOD_CELL_M: i32 = 64;
const WARM_UP: u64 = 60;
/// Ticks timed after the first round is fired (flight and impacts).
const TIMED: usize = 300;

/// Center of the `WOOD_CELL_M` square with the most trees, and their count.
fn densest_wood(map: &Map) -> (Vec2, usize) {
    let size = map.size();
    let cell = sim::Fixed::from_num(WOOD_CELL_M);
    let columns = (size.x / cell).ceil().to_num::<usize>().max(1);
    let rows = (size.y / cell).ceil().to_num::<usize>().max(1);
    let mut counts = vec![0usize; columns * rows];
    let kinds = map.object_kinds();
    for o in map.objects() {
        if !kinds[usize::from(o.kind)].id.starts_with("env_tree_") {
            continue;
        }
        let c = ((o.position.x + size.x / 2) / cell)
            .to_num::<usize>()
            .min(columns - 1);
        let r = ((o.position.y + size.y / 2) / cell)
            .to_num::<usize>()
            .min(rows - 1);
        counts[r * columns + c] += 1;
    }
    let (best, &n) = counts
        .iter()
        .enumerate()
        .max_by_key(|&(i, &n)| (n, std::cmp::Reverse(i)))
        .unwrap_or((0, &0));
    let half = cell / 2;
    let x = cell * sim::Fixed::from_num(best % columns) - size.x / 2 + half;
    let y = cell * sim::Fixed::from_num(best / columns) - size.y / 2 + half;
    (Vec2::new(x, y), n)
}

fn stages(sim: &Sim) -> (usize, usize) {
    let mut out = (0, 0);
    for (_, s) in sim.changed_objects() {
        match s.stage {
            ObjectStage::Damaged => out.0 += 1,
            ObjectStage::Destroyed => out.1 += 1,
            ObjectStage::Intact => {}
        }
    }
    out
}

pub fn run(map: Arc<Map>, options: &Options) -> Result<String, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let unit_types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;
    let (wood, trees) = densest_wood(&map);
    // The firing unit first (id 0), 600 m from the wood towards the center.
    let toward = if wood.y > sim::Fixed::ZERO { -600 } else { 600 };
    let mut spawns = vec![Spawn {
        owner: PlayerId(0),
        unit_type: TYPES[0].into(),
        position: Vec2::new(wood.x, wood.y + sim::Fixed::from_num(toward)),
        fire_mode: FireMode::HoldFire,
    }];
    let columns = (1..).find(|c| c * c >= options.units).unwrap_or(1);
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
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let mut sim = Sim::new(&config, map, options.seed).map_err(|e| e.to_string())?;
    let total = options.units * 2;
    let orders: Vec<PlayerCommand> = (1..=total)
        .step_by(10)
        .map(|first| {
            let player = PlayerId(u8::from(first > options.units));
            let x = if player.0 == 0 { 300 } else { -300 };
            PlayerCommand::new(
                player,
                Command::MoveTo {
                    units: (first..(first + 10).min(total + 1)).map(UnitId).collect(),
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
    let r = options.radius_m * 1000;
    let mut left = options.rounds;
    let mut timed = Vec::new();
    let mut worst = (Duration::ZERO, 0usize);
    while timed.len() < TIMED {
        for _ in 0..left.min(options.per_tick.max(1)) {
            let aim = wood + Vec2::from_millimeters(rng.range_i32(-r, r), rng.range_i32(-r, r));
            sim.debug_rocket(UnitId(0), aim, options.caliber_mm_x10);
            left -= 1;
        }
        let before = stages(&sim);
        let t = Instant::now();
        sim.step(&[]);
        let took = t.elapsed();
        let after = stages(&sim);
        let changed = after.0 + after.1 - before.0 - before.1;
        if took > worst.0 {
            worst = (took, changed);
        }
        timed.push(took);
    }
    let (bare, fallen) = stages(&sim);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{total} units; {} rounds of {} mm, {} per tick, within ±{} m of the densest wood ({trees} trees in {WOOD_CELL_M} m at {:.0}, {:.0})",
        options.rounds,
        f64::from(options.caliber_mm_x10) / 10.0,
        options.per_tick,
        options.radius_m,
        wood.x.to_num::<f64>(),
        wood.y.to_num::<f64>(),
    );
    let _ = writeln!(
        out,
        "objects changed: {fallen} destroyed (trees felled), {bare} damaged (bare)"
    );
    let _ = writeln!(out, "ticks before: {}", stats(&baseline));
    let _ = writeln!(out, "ticks during: {}", stats(&timed));
    let _ = writeln!(
        out,
        "worst tick: {:.2} ms ({} objects changed in it)",
        ms(worst.0),
        worst.1
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
