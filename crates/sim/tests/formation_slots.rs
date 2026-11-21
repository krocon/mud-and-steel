//! Formation slots along the direction of travel (prompt 174): eight mixed
//! units in a 4 x 2 block get a move order at 45°, 135° and across their
//! lines over open ground. Measured: crossings of the straight lines from
//! each unit to its slot, the time until all stand, the spread of their
//! arrivals. Numbers with `--nocapture`.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, Engage, FireMode, Formation, Map, PlayerCommand, PlayerId,
    RouteView, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const HOLD: FireMode = FireMode::HoldFire;
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "slots_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let mut ids = Vec::with_capacity((N * N) as usize);
    for j in 0..N as i32 {
        for i in 0..N as i32 {
            ids.push(terrain((i - half) * 2, (j - half) * 2));
        }
    }
    let size = (N * N) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], ids).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!(
            "{}\n{}\n",
            sim::objects::CSV_HEADER,
            objects.join("\n")
        ))
        .unwrap();
    }
    Arc::new(map)
}

fn flat() -> Arc<Map> {
    map_with(|_, _| 0, &[])
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

/// Two rows of four, a tank, a squad, a half-track and a truck in each,
/// 12 m apart, around (0, 0).
fn block() -> Sim {
    let kinds = [
        "de_pz4_h",
        "de_inf_rifle_squad",
        "de_sdkfz251_1",
        "de_opel_blitz",
    ];
    let mut spawns = Vec::new();
    for row in 0..2 {
        for col in 0..4 {
            let kind = kinds[(row + col) % 4];
            spawns.push((P0, kind, -18 + col as i32 * 12, -6 + row as i32 * 12, HOLD));
        }
    }
    sim_with(flat(), &spawns)
}

/// Crossings, ticks until all stand, ticks between the first and the last
/// arrival (20 ticks per second).
#[derive(Debug)]
#[allow(dead_code)]
struct Run {
    crossings: usize,
    all_stand_ticks: u64,
    spread_ticks: u64,
}

fn crosses(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> bool {
    let side = |p: Vec2, q: Vec2, r: Vec2| {
        let v = (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
        v.signum()
    };
    let (d1, d2) = (side(b.0, b.1, a.0), side(b.0, b.1, a.1));
    let (d3, d4) = (side(a.0, a.1, b.0), side(a.0, a.1, b.1));
    d1 * d2 < 0 && d3 * d4 < 0
}

/// Moves the block to `target` (from the block's center) with `command`
/// and measures.
fn run(target: Vec2, formation: bool) -> Run {
    let mut sim = block();
    let units: Vec<UnitId> = (0..8).map(UnitId).collect();
    let starts: Vec<Vec2> = sim.snapshot().units.iter().map(|u| u.position).collect();
    let command = if formation {
        Command::FormationMove {
            units: units.clone(),
            target,
            formation: Formation::Line,
            facing: None,
            engage: Engage::Never,
        }
    } else {
        Command::MoveTo {
            units: units.clone(),
            target,
        }
    };
    sim.step(&[PlayerCommand::new(P0, command)]);
    let slots: Vec<Vec2> = sim
        .snapshot()
        .units
        .iter()
        .map(|u| u.target.unwrap_or(u.position))
        .collect();
    let mut arrived: [Option<u64>; 8] = [None; 8];
    for tick in 1..20 * 300 {
        sim.step(&[]);
        for (k, at) in arrived.iter_mut().enumerate() {
            if at.is_none() && sim.route_of(UnitId(k as u32)) == Some(RouteView::Idle) {
                *at = Some(tick);
            }
        }
        if arrived.iter().all(Option::is_some) {
            break;
        }
    }
    let mut crossings = 0;
    for a in 0..8 {
        for b in a + 1..8 {
            if crosses((starts[a], slots[a]), (starts[b], slots[b])) {
                crossings += 1;
            }
        }
    }
    let ticks: Vec<u64> = arrived.iter().map(|t| t.unwrap_or(20 * 300)).collect();
    let (first, last) = (*ticks.iter().min().unwrap(), *ticks.iter().max().unwrap());
    Run {
        crossings,
        all_stand_ticks: last,
        spread_ticks: last - first,
    }
}

const TARGETS: [(&str, i32, i32); 4] = [
    ("45°", 140, 140),
    ("135°", -140, 140),
    ("across", 200, 0),
    ("ahead", 0, 200),
];

#[test]
fn move_orders_do_not_cross() {
    let runs: Vec<(&str, Run)> = TARGETS
        .iter()
        .map(|&(name, x, y)| (name, run(Vec2::from_meters(x, y), false)))
        .collect();
    for (name, r) in &runs {
        println!("MoveTo {name}: {r:?}");
    }
    for (name, r) in &runs {
        assert_eq!(r.crossings, 0, "MoveTo {name}: {r:?}");
    }
}

/// `FormationMove` (the player's moves, prompt 046) assigns rank by rank
/// along the way and is not untangled: in a mission self-test any change of
/// its slots changed the outcome (prompt 174, ARCHITECTURE.md "Open").
/// Measured so it does not get worse: crossings 1 / 1 / 6 / 2.
#[test]
fn formation_moves_cross_at_most_as_before() {
    let runs: Vec<(&str, Run)> = TARGETS
        .iter()
        .map(|&(name, x, y)| (name, run(Vec2::from_meters(x, y), true)))
        .collect();
    for (name, r) in &runs {
        println!("FormationMove {name}: {r:?}");
    }
    let before = [1, 1, 6, 2];
    for ((name, r), most) in runs.iter().zip(before) {
        assert!(r.crossings <= most, "FormationMove {name}: {r:?}");
    }
}
