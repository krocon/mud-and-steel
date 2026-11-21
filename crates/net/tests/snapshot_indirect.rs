//! Prompt 038 with 032: a snapshot taken while mortar shells fly and smoke
//! clouds drift restores to a simulation that continues tick for tick like
//! the original.

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
        name: "snapshot_indirect".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn battle(map: Arc<Map>) -> Sim {
    let spawns = [
        (P0, "de_inf_mortar_medium", 0, 200),
        (P0, "de_inf_mortar_medium", 30, 200),
        (P0, "de_pz4_h", 0, 150),
        (P1, "su_inf_rifle_squad", 0, 0),
        (P1, "su_t34_76_1943", 40, -100),
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
        variant: None,
    };
    Sim::new(&config, map, 5).unwrap()
}

fn barrage(unit: u32, x: i32, y: i32, salvos: u16, smoke: bool) -> PlayerCommand {
    PlayerCommand::new(
        P0,
        Command::Barrage {
            units: vec![UnitId(unit)],
            target: Vec2::from_meters(x, y),
            radius_m: 0,
            salvos,
            smoke,
        },
    )
}

#[test]
fn snapshot_with_shells_and_smoke_in_flight() {
    let map = flat_map();
    let mut original = battle(map.clone());
    original.step(&[barrage(1, 40, -60, 3, true)]);
    original.step(&[barrage(0, 0, 0, 12, false)]);
    // Until shells fly and smoke drifts at the same time.
    let mut ticks = 0;
    while ticks < 20 * 60 {
        original.step(&[]);
        ticks += 1;
        let snap = original.snapshot();
        if !snap.shells.is_empty() && !snap.smoke.is_empty() {
            break;
        }
    }
    let state = original.snapshot();
    assert!(
        !state.shells.is_empty() && !state.smoke.is_empty(),
        "no moment with shells and smoke at once"
    );

    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.snapshot().shells.len(), state.shells.len());
    assert_eq!(restored.snapshot().smoke.len(), state.smoke.len());
    for tick in 0..20 * 90 {
        // A new barrage after the restore, too.
        let commands = if tick == 100 {
            vec![barrage(1, 0, 0, 4, false)]
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
