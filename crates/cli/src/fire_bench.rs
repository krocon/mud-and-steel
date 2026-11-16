//! Fire benchmark (prompt 041): the town map with units of two players,
//! once without fire and once with the west wood, a row of town houses and
//! both supply dumps set on fire, timed per tick.

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
    pub ticks: u64,
    pub seed: u64,
}

const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];
const SPACING_M: i32 = 12;

fn new_sim(map: &Arc<Map>, options: &Options) -> Result<Sim, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let unit_types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;
    let mut spawns = Vec::new();
    for (player, y) in [(PlayerId(0), 240_i32), (PlayerId(1), -240)] {
        for i in 0..options.units {
            let i = i32::try_from(i).unwrap_or(0);
            spawns.push(Spawn {
                owner: player,
                unit_type: TYPES[(i % 2) as usize].into(),
                position: Vec2::from_meters(
                    (i % 10 - 5) * SPACING_M,
                    y - (i / 10) * SPACING_M * y.signum(),
                ),
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
    };
    Sim::new(&config, Arc::clone(map), options.seed).map_err(|e| e.to_string())
}

/// Objects to set on fire: every tenth conifer of the west wood, the town
/// houses of the main street's north-west terrace row, all dumps.
fn fire_starts(map: &Map) -> Vec<u32> {
    let kinds = map.object_kinds();
    let mut conifers = 0;
    let mut out = Vec::new();
    for (i, o) in (0u32..).zip(map.objects()) {
        let id = kinds[usize::from(o.kind)].id.as_str();
        let (x, y) = (o.position.x.to_num::<i32>(), o.position.y.to_num::<i32>());
        let pick = match id {
            "env_tree_conifer" => {
                conifers += 1;
                conifers % 10 == 0
            }
            "env_house_town" => x < -30 && (-20..0).contains(&y),
            "env_dump_ammo" | "env_dump_fuel" => true,
            _ => false,
        };
        if pick {
            out.push(i);
        }
    }
    out
}

/// Runs both variants; returns the report.
pub fn run(map: Arc<Map>, options: &Options) -> Result<String, String> {
    let mut report = String::new();
    let _ = writeln!(
        report,
        "fire bench: map '{}', {} objects, {} units per player, {} ticks",
        map.reference().name,
        map.objects().len(),
        options.units,
        options.ticks
    );
    for burn in [false, true] {
        let mut sim = new_sim(&map, options)?;
        // Both sides advance on the cross street into the town.
        let orders: Vec<PlayerCommand> =
            [(PlayerId(0), 0u32, 60), (PlayerId(1), options.units, -60)]
                .into_iter()
                .map(|(player, first, y)| {
                    PlayerCommand::new(
                        player,
                        Command::MoveTo {
                            units: (first..first + options.units).map(UnitId).collect(),
                            target: Vec2::from_meters(0, y),
                        },
                    )
                })
                .collect();
        let starts = if burn { fire_starts(&map) } else { Vec::new() };
        for &o in &starts {
            sim.ignite_object(o, UnitId(0));
        }
        let mut total = Duration::ZERO;
        let mut worst = Duration::ZERO;
        let mut peak_fires = 0;
        for tick in 0..options.ticks {
            let commands = if tick == 0 { orders.as_slice() } else { &[] };
            let began = Instant::now();
            sim.step(commands);
            let took = began.elapsed();
            total += took;
            worst = worst.max(took);
            if tick % 20 == 0 {
                peak_fires = peak_fires.max(sim.fires().len());
            }
        }
        let mean = total.as_secs_f64() * 1000.0 / options.ticks.max(1) as f64;
        let _ = writeln!(
            report,
            "  {}: {} fires started, peak {} burning, {} burnt out; tick mean {mean:.3} ms, max {:.3} ms (budget 50 ms); hash {:#018x}",
            if burn { "with fire   " } else { "without fire" },
            starts.len(),
            peak_fires,
            sim.burnt_objects().count(),
            worst.as_secs_f64() * 1000.0,
            sim.state_hash()
        );
    }
    Ok(report)
}
