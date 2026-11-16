//! Prompt 038 with 035: a snapshot taken while engineers dig a trench, a
//! minefield lies armed, a tank has lost its track and wire stands
//! restores to a simulation that continues tick for tick like the
//! original (mines, sites, jobs and the dug ground included).

use net::snapshot;
use sim::map::MapLayout;
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
        name: "snapshot_works".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap();
    map.add_objects("kind,variant,x_mm,y_mm,heading_deg\nfort_mine_at,Mine,0,40000,0\n")
        .unwrap();
    Arc::new(map)
}

fn sim(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "de_inf_engineer", 0, -20),
        (P0, "de_inf_engineer", 30, -20),
        (P1, "su_t34_76_1943", -70, 40),
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
    Sim::new(&config, map, 3).unwrap()
}

#[test]
fn snapshot_with_mines_sites_and_a_thrown_track() {
    let map = flat_map();
    let mut original = sim(map.clone());
    original.step(&[
        PlayerCommand::new(
            P0,
            Command::Lay {
                units: vec![UnitId(1)],
                kind: LayKind::Wire,
                from: Vec2::from_meters(20, 0),
                to: Vec2::from_meters(40, 0),
            },
        ),
        PlayerCommand::new(
            P1,
            Command::MoveTo {
                units: vec![UnitId(2)],
                target: Vec2::from_meters(70, 40),
            },
        ),
    ]);
    for _ in 0..20 * 30 {
        original.step(&[]);
    }
    original.step(&[PlayerCommand::new(
        P0,
        Command::Dig {
            units: vec![UnitId(0)],
            kind: DigKind::Trench,
            from: Vec2::from_meters(-10, 0),
            to: Vec2::from_meters(10, 0),
        },
    )]);
    for _ in 0..20 * 20 {
        original.step(&[]);
    }
    assert!(!original.tracks_off().is_empty(), "the tank hit no mine");
    assert!(original.works_for(P0).iter().any(|w| !w.built));
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.all_mines(), original.all_mines());
    for tick in 0..20 * 110 {
        original.step(&[]);
        restored.step(&[]);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "diverged {} ticks after the restore",
            tick + 1
        );
    }
    assert!(original.works_for(P0).iter().all(|w| w.built));
}
