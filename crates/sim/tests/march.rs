//! Groups drive together (prompt 187): eight mixed units (tanks, squads,
//! half-tracks, trucks) in a 4 x 2 block get a formation move over open
//! ground, over a stone bridge and along a lane through a dense wood.
//! Measured: the spread of their arrivals (first to last), the time until
//! all stand, the largest distance between two members on the way, and
//! units the simulation reports stuck. Numbers with `--nocapture`.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, Engage, FireMode, Formation, Map, MoveEvent, PlayerCommand,
    PlayerId, RouteView, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const HOLD: FireMode = FireMode::HoldFire;
const N: u32 = 301;
const DEEP: u8 = 7;
const LIMIT: u64 = 20 * 400;

/// 600 m × 600 m around the origin; `terrain` gives terrain id and height
/// (mm) of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> (u8, i32), objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "march_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let (mut ids, mut heights) = (Vec::new(), Vec::new());
    for j in 0..N as i32 {
        for i in 0..N as i32 {
            let (id, h) = terrain((i - half) * 2, (j - half) * 2);
            ids.push(id);
            heights.push(h);
        }
    }
    let mut map = Map::from_rasters(layout, heights, ids).unwrap();
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

fn sim_with(map: Arc<Map>, spawns: &[(&str, i32, i32)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.0).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: spawns
            .iter()
            .map(|&(unit, x, y)| Spawn {
                owner: P0,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: HOLD,
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
        variant: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

/// Two rows of four, 12 m apart, around `(x, y)`: tanks, squads,
/// half-tracks and trucks, so every speed class is in the group.
fn block(x: i32, y: i32) -> Vec<(&'static str, i32, i32)> {
    let kinds = [
        "de_pz4_h",
        "de_inf_rifle_squad",
        "de_sdkfz251_1",
        "de_opel_blitz",
    ];
    let mut spawns = Vec::new();
    for row in 0..2i32 {
        for col in 0..4i32 {
            let kind = kinds[((row + col) % 4) as usize];
            spawns.push((kind, x - 18 + col * 12, y - 6 + row * 12));
        }
    }
    spawns
}

/// Flat open ground.
fn open() -> Arc<Map> {
    map_with(|_, _| (0, 0), &[])
}

/// A river north-south (deep water for |x| <= 5 m, bed 2 m down) with a
/// stone bridge across it at y = 1 m.
fn river() -> Arc<Map> {
    map_with(
        |x, _| {
            if x.abs() <= 5 { (DEEP, -2000) } else { (0, 0) }
        },
        &["env_bridge_stone,Arch1,0,1000,90".to_string()],
    )
}

/// A dense oak wood for 40 <= y <= 160 m and |x| <= 80 m (a trunk every
/// 4 m), cut by a lane 12 m wide along x = 0.
fn wood() -> Arc<Map> {
    let mut trees = Vec::new();
    for ty in 10..=40i32 {
        for tx in -20..=20i32 {
            if (tx * 4).abs() <= 6 {
                continue;
            }
            trees.push(format!(
                "env_tree_deciduous,Oak,{},{},0",
                tx * 4000,
                ty * 4000
            ));
        }
    }
    map_with(|_, _| (0, 0), &trees)
}

/// Seconds between the first and the last arrival, seconds until all
/// stand (both in tenths), largest distance between two members on the
/// way (m), units reported stuck.
#[derive(Debug)]
#[allow(dead_code)]
struct Run {
    spread_ds: u64,
    all_stand_ds: u64,
    widest_m: i32,
    stuck: usize,
}

type Scenario = (&'static str, Arc<Map>, Vec<(&'static str, i32, i32)>, Vec2);

/// Moves the group from `spawns` to `target` with a formation move
/// (`engage`: `Infantry` like a right click, `Never` like M).
fn run(map: Arc<Map>, spawns: &[(&str, i32, i32)], target: Vec2, engage: Engage) -> Run {
    let mut sim = sim_with(map, spawns);
    let n = spawns.len();
    let units: Vec<UnitId> = (0..n as u32).map(UnitId).collect();
    sim.step(&[PlayerCommand::new(
        P0,
        Command::FormationMove {
            units: units.clone(),
            target,
            formation: Formation::Auto,
            facing: None,
            engage,
        },
    )]);
    let mut arrived: Vec<Option<u64>> = vec![None; n];
    let mut widest = sim::Fixed::ZERO;
    let mut stuck = 0;
    for tick in 1..LIMIT {
        sim.step(&[]);
        stuck += sim
            .move_events()
            .iter()
            .filter(|e| matches!(e, MoveEvent::Stuck { .. }))
            .count();
        for (k, at) in arrived.iter_mut().enumerate() {
            if at.is_none() && sim.route_of(UnitId(k as u32)) == Some(RouteView::Idle) {
                *at = Some(tick);
            }
        }
        if arrived.iter().all(Option::is_none) {
            let p: Vec<Vec2> = sim.snapshot().units.iter().map(|u| u.position).collect();
            for a in 0..n {
                for b in a + 1..n {
                    widest = widest.max((p[a] - p[b]).length());
                }
            }
        }
        if arrived.iter().all(Option::is_some) {
            break;
        }
    }
    let ticks: Vec<u64> = arrived.iter().map(|t| t.unwrap_or(LIMIT)).collect();
    let (first, last) = (*ticks.iter().min().unwrap(), *ticks.iter().max().unwrap());
    Run {
        spread_ds: (last - first) / 2,
        all_stand_ds: last / 2,
        widest_m: widest.to_num::<i32>(),
        stuck,
    }
}

fn scenarios() -> Vec<Scenario> {
    vec![
        (
            "open",
            open(),
            block(-150, -150),
            Vec2::from_meters(150, 150),
        ),
        ("bridge", river(), block(-150, 1), Vec2::from_meters(150, 1)),
        ("wood", wood(), block(0, -60), Vec2::from_meters(0, 240)),
    ]
}

/// Before prompt 187 (every member at its own speed, as the plain move
/// still does): arrivals 148 / 106 / 118 s apart, the group up to 373 /
/// 267 / 230 m long, all standing after 200 / 141 / 153 s.
#[test]
fn attack_moves_arrive_together_and_plain_moves_do_not() {
    for (name, map, spawns, target) in scenarios() {
        let together = run(map.clone(), &spawns, target, Engage::Infantry);
        let plain = run(map, &spawns, target, Engage::Never);
        println!("{name} together: {together:?}");
        println!("{name} plain: {plain:?}");
        assert!(together.spread_ds <= 150, "{name}: {together:?}");
        assert!(together.widest_m <= 120, "{name}: {together:?}");
        // Driving together costs (almost) no time: the slowest sets it.
        assert!(
            together.all_stand_ds * 10 <= plain.all_stand_ds * 11,
            "{name}: {together:?} / {plain:?}"
        );
        // Prompt 231: faster infantry narrows the plain-move gap over 300 m,
        // still well above the 15 s spread limit of driving together.
        assert!(plain.spread_ds >= 400, "{name}: {plain:?}");
        assert_eq!(together.stuck + plain.stuck, 0, "{name}");
    }
}

/// `Secure` drives together like the attack-move.
#[test]
fn securing_groups_drive_together() {
    let mut sim = sim_with(open(), &block(-150, -150));
    let units: Vec<UnitId> = (0..8).map(UnitId).collect();
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Secure {
            units,
            center: Vec2::from_meters(150, 150),
            radius_m: 60,
            formation: Formation::Auto,
            facing: None,
        },
    )]);
    let mut arrived: Vec<Option<u64>> = vec![None; 8];
    for tick in 1..LIMIT {
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
    let ticks: Vec<u64> = arrived.iter().map(|t| t.unwrap()).collect();
    let spread = ticks.iter().max().unwrap() - ticks.iter().min().unwrap();
    assert!(spread <= 20 * 15, "arrivals {ticks:?}");
}

/// The same order gives the same hash twice (the caps are derived from
/// state only).
#[test]
fn driving_together_is_deterministic() {
    let hash = || {
        let mut sim = sim_with(river(), &block(-150, 1));
        sim.step(&[PlayerCommand::new(
            P0,
            Command::FormationMove {
                units: (0..8).map(UnitId).collect(),
                target: Vec2::from_meters(150, 1),
                formation: Formation::Auto,
                facing: None,
                engage: Engage::All,
            },
        )]);
        for _ in 0..20 * 60 {
            sim.step(&[]);
        }
        sim.state_hash()
    };
    assert_eq!(hash(), hash());
}
