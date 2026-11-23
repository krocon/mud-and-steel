//! A saved game in the middle of a landing (prompt 119): a boat running
//! onto the beach with a squad and a tank aboard restores with the same
//! hash and plays on the same.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);

/// The sea in the north, a beach rising from y = 0 (like
/// `crates/sim/tests/boats.rs`).
fn coast() -> Arc<Map> {
    let (sx, sy) = (101u32, 151u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for _ in 0..sx {
            let y = 2 * j as i32 - (sy as i32 - 1);
            let (h, t) = if y >= 20 {
                (1000, 0)
            } else if y >= 0 {
                (50 * y, 3)
            } else if y >= -26 {
                (60 * y, 6)
            } else {
                ((60 * y).max(-6000), 7)
            };
            heights.push(h);
            terrain.push(t);
        }
    }
    let layout = MapLayout {
        name: "snapshot_coast".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
}

fn landing(map: Arc<Map>) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let spawns = [
        (P0, "us_lcm3", 0, -90),
        (P0, "us_m4a3_75", 0, 60),
        (PlayerId(1), "us_inf_rifle_squad", 90, 90),
    ];
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: names
            .iter()
            .map(|id| UnitType::from_catalog(catalog.get(id).unwrap(), &scale).unwrap())
            .collect(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode: FireMode::HoldFire,
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
    Sim::new(&config, map, 5).unwrap()
}

#[test]
fn snapshot_in_the_middle_of_a_landing() {
    let map = coast();
    let mut original = landing(map.clone());
    assert!(original.put_aboard(UnitId(1), UnitId(0)));
    original.step(&[
        PlayerCommand::new(
            P0,
            Command::Land {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(0, 10),
            },
        ),
        PlayerCommand::new(
            P0,
            Command::Disembark {
                units: vec![UnitId(0)],
            },
        ),
    ]);
    let mut ticks = 0;
    while original.boat_phase(UnitId(0)) != Some("ramp") {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 120, "the boat never grounded");
    }
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    for _ in 0..20 * 60 {
        original.step(&[]);
        restored.step(&[]);
    }
    assert!(!original.is_aboard(UnitId(1)), "the tank drove off");
    assert_eq!(restored.state_hash(), original.state_hash());
}
