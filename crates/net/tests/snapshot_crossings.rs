//! Prompt 038 with 055: a snapshot taken while a bridge lies laid across a
//! river, a squad swims, a vehicle stands camouflaged and paratroopers hang
//! under their canopies restores to a simulation that continues tick for
//! tick like the original (the laid bridge comes back with the map).

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    AirMission, Catalog, CombatRules, Command, FireMode, LayerState, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// 400 m × 200 m, a narrow river (|x| <= 3 m) and a wide one (60 m..100 m).
fn river_map() -> Arc<Map> {
    let (sx, sy) = (201u32, 101u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for _ in 0..sy {
        for i in 0..sx {
            let x = 2 * i as i32 - 200;
            let water = x.abs() <= 3 || (60..=100).contains(&x);
            heights.push(if water { -2000 } else { 0 });
            terrain.push(if water { 7 } else { 0 });
        }
    }
    let layout = MapLayout {
        name: "snapshot_crossings".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
}

fn sim(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "su_valentine_bridgelayer", -40, 0),
        (P0, "su_t70m", -40, 30),
        (P0, "su_inf_rifle_squad", -150, 60),
        (P1, "de_pz4_h", 150, -60),
    ];
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
    };
    Sim::new(&config, map, 8).unwrap()
}

#[test]
fn snapshot_with_a_laid_bridge_swimmers_camouflage_and_paratroopers() {
    let map = river_map();
    let mut original = sim(map.clone());
    original.step(&[
        PlayerCommand::new(
            P0,
            Command::LayBridge {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(0, 0),
            },
        ),
        PlayerCommand::new(
            P1,
            Command::SetCamouflage {
                units: vec![UnitId(3)],
                on: true,
            },
        ),
    ]);
    let mut ticks = 0;
    while !matches!(original.layers()[0].state, LayerState::Laid(_)) {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 180, "the bridge was not laid");
    }
    // Paratroopers into the wide river.
    original.step(&[PlayerCommand::new(
        P0,
        Command::CallAir {
            mission: AirMission::Airborne,
            target: Vec2::from_meters(80, 0),
        },
    )]);
    let mut ticks = 0;
    while original.swimmers().is_empty() || original.parachutes(None).is_empty() {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 120, "no swimmers or no parachutes");
    }
    assert_eq!(original.camouflage_of(UnitId(3)), Some(true));
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.map().bridges(), original.map().bridges());
    assert_eq!(restored.map().bridges()[0].laid_by, Some(UnitId(0)));
    for tick in 0..20 * 60 {
        let commands = if tick == 20 {
            vec![PlayerCommand::new(
                P0,
                Command::MoveTo {
                    units: vec![UnitId(1)],
                    target: Vec2::from_meters(40, 30),
                },
            )]
        } else {
            Vec::new()
        };
        original.step(&commands);
        restored.step(&commands);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "diverged {} ticks after the restore",
            tick + 1
        );
    }
}
