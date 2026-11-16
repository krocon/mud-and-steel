//! Prompt 038 with 036: a snapshot taken while a truck resupplies a tank
//! that fired most of its ammunition and a halftrack burns fuel restores
//! to a simulation that continues tick for tick like the original.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "snapshot_logistics".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn supply(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "de_opel_blitz", 0, -300),
        (P0, "de_pz4_h", 0, 0),
        (P0, "de_sdkfz251_1", 30, 0),
        (P1, "su_t34_76_1943", 0, 300),
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
    };
    Sim::new(&config, map, 9).unwrap()
}

fn order(command: Command) -> PlayerCommand {
    PlayerCommand::new(P0, command)
}

#[test]
fn snapshot_while_resupplying_and_driving() {
    let map = flat_map();
    let mut original = supply(map.clone());
    original.step(&[
        order(Command::AttackGround {
            units: vec![UnitId(1)],
            target: Vec2::from_meters(-150, 150),
        }),
        order(Command::Patrol {
            units: vec![UnitId(2)],
            target: Vec2::from_meters(200, -200),
        }),
    ]);
    for _ in 0..20 * 60 {
        original.step(&[]);
    }
    original.step(&[order(Command::MoveTo {
        units: vec![UnitId(0)],
        target: Vec2::from_meters(10, 10),
    })]);
    // Snapshot once the resupply has begun.
    let empty = original.unit_stock(UnitId(1)).unwrap().ammo_pct.unwrap();
    assert!(empty < 100);
    let mut ticks = 0;
    while original.unit_stock(UnitId(1)).unwrap().ammo_pct.unwrap() <= empty {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 60, "no resupply");
    }
    assert!(original.unit_stock(UnitId(1)).unwrap().ammo_pct.unwrap() < 100);
    assert!(original.unit_stock(UnitId(2)).unwrap().fuel_pct.unwrap() < 100);
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.snapshot(), original.snapshot());
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
