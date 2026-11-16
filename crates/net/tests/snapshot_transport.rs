//! Prompt 038 with 034: a snapshot taken while a squad rides in a truck, a
//! gun is being limbered and another squad walks up to mount restores to a
//! simulation that continues tick for tick like the original.

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
        name: "snapshot_transport".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn convoy(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "de_opel_blitz", 0, 0),
        (P0, "de_inf_rifle_squad", 15, 0),
        (P0, "de_sdkfz251_1", -20, 0),
        (P0, "de_pak40", -20, -15),
        (P0, "de_inf_mg42_team", 60, 0),
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
fn snapshot_with_passengers_and_a_gun_being_limbered() {
    let map = flat_map();
    let mut original = convoy(map.clone());
    original.step(&[order(Command::Embark {
        units: vec![UnitId(1)],
        carrier: UnitId(0),
    })]);
    let mut ticks = 0;
    while original.snapshot().units[1].carrier.is_none() {
        original.step(&[]);
        ticks += 1;
        assert!(ticks < 20 * 60, "the squad never mounted");
    }
    original.step(&[
        order(Command::Tow {
            units: vec![UnitId(2)],
            gun: UnitId(3),
        }),
        order(Command::Embark {
            units: vec![UnitId(4)],
            carrier: UnitId(0),
        }),
        order(Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 80),
        }),
    ]);
    for _ in 0..200 {
        original.step(&[]);
    }
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.snapshot(), original.snapshot());
    for tick in 0..20 * 90 {
        let commands = if tick == 600 {
            vec![
                order(Command::Disembark {
                    units: vec![UnitId(0)],
                }),
                order(Command::Unlimber {
                    units: vec![UnitId(3)],
                }),
            ]
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
