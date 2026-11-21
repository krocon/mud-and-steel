//! Test scenario of prompt 110: three guns in position, four T-34s attack
//! them from the front or from the flank. Prints losses and duration per
//! variant; run with
//! `cargo test --release -p ai --test flank_scenario -- --ignored --nocapture`.
//! Not an assertion test: the numbers go into docs/BALANCE.md.

use ai::{Ai, Difficulty, PlayerView};
use sim::map::MapLayout;
use sim::{
    Catalog, Command, Engage, FireMode, Formation, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Spawn, TerrainRules, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const MAX_TICKS: u64 = 2400;
const SEEDS: u64 = 10;

fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "flank_scenario".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(
        Map::from_rasters(layout, vec![0; (n * n) as usize], vec![0; (n * n) as usize]).unwrap(),
    )
}

struct Variant {
    name: &'static str,
    defenders: &'static [(&'static str, i32, i32)],
    fire: FireMode,
    ai: bool,
}

/// Defenders stand around (0, 100) facing about north; attackers start
/// 140 m north (front) or east (flank) of them.
fn play(v: &Variant, flank: bool, seed: u64) -> (usize, usize, u64) {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = v.defenders.iter().map(|d| d.0).collect();
    names.push("su_t34_76_1943");
    names.sort_unstable();
    names.dedup();
    let mut spawns: Vec<Spawn> = v
        .defenders
        .iter()
        .map(|&(unit, x, y)| Spawn {
            owner: P0,
            unit_type: unit.into(),
            position: Vec2::from_meters(x, y),
            fire_mode: v.fire,
        })
        .collect();
    let defenders = spawns.len();
    for k in 0..4 {
        let (x, y) = if flank {
            (140, 70 + 20 * k)
        } else {
            (-30 + 20 * k, -40)
        };
        spawns.push(Spawn {
            owner: P1,
            unit_type: "su_t34_76_1943".into(),
            position: Vec2::from_meters(x, y),
            fire_mode: FireMode::FireAtWill,
        });
    }
    let map = flat_map();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns,
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: sim::CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    };
    let mut sim = Sim::new(&config, map.clone(), seed).unwrap();
    let mut ai =
        v.ai.then(|| Ai::new(P0, Difficulty::Normal, &config, &map, seed));
    let n = config.spawns.len();
    let ids: Vec<sim::UnitId> = (0..n).map(|i| sim::UnitId(i as u32)).collect();
    let attackers: Vec<sim::UnitId> = ids[defenders..].to_vec();
    let guns: Vec<sim::UnitId> = v
        .defenders
        .iter()
        .zip(&ids)
        .filter(|(d, _)| d.0 != "de_opel_blitz")
        .map(|(_, &id)| id)
        .collect();
    let mut first = vec![PlayerCommand::new(
        P1,
        Command::FormationMove {
            units: attackers.clone(),
            target: Vec2::from_meters(0, 100),
            formation: Formation::Line,
            facing: None,
            engage: Engage::All,
        },
    )];
    let alive = |sim: &Sim, list: &[sim::UnitId]| {
        list.iter()
            .filter(|&&id| sim.unit_hp(id).is_some_and(|hp| hp > 0))
            .count()
    };
    while sim.tick() < MAX_TICKS {
        let mut commands = std::mem::take(&mut first);
        if let Some(ai) = &mut ai {
            let view = PlayerView::new(&sim, P0);
            commands.extend(
                ai.think(&view)
                    .into_iter()
                    .map(|c| PlayerCommand::new(P0, c)),
            );
        }
        sim.step(&commands);
        if alive(&sim, &guns) == 0 || alive(&sim, &attackers) == 0 {
            break;
        }
    }
    (
        guns.len() - alive(&sim, &guns),
        attackers.len() - alive(&sim, &attackers),
        sim.tick(),
    )
}

#[test]
#[ignore = "measurement, prints a table"]
fn flank_scenario_table() {
    let variants = [
        Variant {
            name: "3 StuG III G, fire at will",
            defenders: &[
                ("de_stug3_g", -20, 100),
                ("de_stug3_g", 0, 100),
                ("de_stug3_g", 20, 100),
            ],
            fire: FireMode::FireAtWill,
            ai: false,
        },
        Variant {
            name: "3 StuG III G, hold fire",
            defenders: &[
                ("de_stug3_g", -20, 100),
                ("de_stug3_g", 0, 100),
                ("de_stug3_g", 20, 100),
            ],
            fire: FireMode::HoldFire,
            ai: false,
        },
        Variant {
            name: "3 PaK 40 + truck, Normal AI",
            defenders: &[
                ("de_pak40", -20, 100),
                ("de_pak40", 0, 100),
                ("de_pak40", 20, 100),
                ("de_opel_blitz", 0, 140),
            ],
            fire: FireMode::FireAtWill,
            ai: true,
        },
        Variant {
            name: "3 StuG III G, Normal AI",
            defenders: &[
                ("de_stug3_g", -20, 100),
                ("de_stug3_g", 0, 100),
                ("de_stug3_g", 20, 100),
            ],
            fire: FireMode::FireAtWill,
            ai: true,
        },
    ];
    println!("| Variant | Attack | guns lost | T-34 lost | seconds |");
    println!("|---|---|---|---|---|");
    for v in &variants {
        for flank in [false, true] {
            let (mut guns, mut tanks, mut ticks) = (0, 0, 0);
            for seed in 1..=SEEDS {
                let (g, t, k) = play(v, flank, seed);
                guns += g;
                tanks += t;
                ticks += k;
            }
            let n = SEEDS as f64;
            println!(
                "| {} | {} | {:.1} | {:.1} | {:.0} |",
                v.name,
                if flank { "flank" } else { "front" },
                guns as f64 / n,
                tanks as f64 / n,
                ticks as f64 / n / 20.0
            );
        }
    }
}
