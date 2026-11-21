//! Prompt 038 with 052: a snapshot taken with a map strongpoint, a dug-in
//! tank, a camouflage net and a trench under way restores to a simulation
//! that continues tick for tick like the original.

use net::snapshot;
use sim::map::{MapLayout, Marker};
use sim::{
    Catalog, CombatRules, Command, DigKind, FireMode, LayKind, Map, PlayerCommand, PlayerId, Scale,
    Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn flat_map() -> Arc<Map> {
    let n = 201;
    let layout = MapLayout {
        name: "snapshot_positions".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: vec![Marker {
            name: "strongpoint_test".into(),
            position: Vec2::from_meters(-60, 60),
        }],
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn sim(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "de_inf_engineer", 0, -10),
        (P0, "de_pz4_h", -20, -20),
        (P1, "su_t34_76_1943", 0, 150),
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
                fire_mode: FireMode::FireAtWill,
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
    Sim::new(&config, map, 9).unwrap()
}

fn command(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

#[test]
fn snapshot_with_a_strongpoint_a_dug_in_tank_and_a_net() {
    let map = flat_map();
    let mut original = sim(map.clone());
    let dig = |kind: DigKind, from: (i32, i32), to: (i32, i32)| {
        command(
            P0,
            Command::Dig {
                units: vec![UnitId(0)],
                kind,
                from: Vec2::from_meters(from.0, from.1),
                to: Vec2::from_meters(to.0, to.1),
            },
        )
    };
    original.step(&[dig(DigKind::VehiclePit, (0, 0), (0, 0))]);
    for _ in 0..20 * 80 {
        original.step(&[]);
    }
    original.step(&[
        command(
            P0,
            Command::Lay {
                units: vec![UnitId(0)],
                kind: LayKind::CamoNet,
                from: Vec2::from_meters(0, 0),
                to: Vec2::from_meters(0, 0),
            },
        ),
        command(
            P0,
            Command::MoveTo {
                units: vec![UnitId(1)],
                target: Vec2::from_meters(0, 0),
            },
        ),
    ]);
    for _ in 0..20 * 25 {
        original.step(&[]);
    }
    original.step(&[dig(DigKind::Trench, (-10, -14), (10, -14))]);
    for _ in 0..20 * 10 {
        original.step(&[]);
    }
    assert!(!original.dug_in().is_empty(), "the tank is not dug in");
    assert!(original.works_for(P0).iter().any(|w| !w.built));
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.dug_in(), original.dug_in());
    for tick in 0..20 * 60 {
        original.step(&[]);
        restored.step(&[]);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "diverged {} ticks after the restore",
            tick + 1
        );
    }
}
