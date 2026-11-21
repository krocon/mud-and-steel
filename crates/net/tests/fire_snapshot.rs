//! Prompt 041: fires (burning trees, a dump with a running fuse, wrecks)
//! are part of the snapshot of prompt 038; a restored match burns on
//! exactly like the original.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, FireMode, Map, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules,
    UnitId, Vec2, VisionRules, unit_types_from_catalog,
};
use std::sync::Arc;

fn forest_map() -> Arc<Map> {
    let n: u32 = 201;
    let layout = MapLayout {
        name: "fire_snapshot".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let mut rows = vec![sim::objects::CSV_HEADER.to_string()];
    for x in (-60..=60).step_by(5) {
        for y in (-20..=20).step_by(5) {
            rows.push(format!(
                "env_tree_conifer,Spruce,{},{},0",
                x * 1000,
                y * 1000
            ));
        }
    }
    rows.push("env_dump_ammo,Intact,0,60000,0".into());
    let mut map = Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap();
    map.add_objects(&rows.join("\n")).unwrap();
    Arc::new(map)
}

fn new_sim(map: Arc<Map>) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let names = ["de_pz4_h", "su_t34_76_1943"];
    let spawn = |owner: u8, unit: &str, x: i32| Spawn {
        owner: PlayerId(owner),
        unit_type: unit.into(),
        position: Vec2::from_meters(x, 150),
        fire_mode: FireMode::HoldFire,
    };
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: vec![spawn(0, "de_pz4_h", -150), spawn(1, "su_t34_76_1943", 150)],
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, 11).unwrap()
}

#[test]
fn burning_match_restores_and_burns_on_identically() {
    let map = forest_map();
    let mut original = new_sim(map.clone());
    // Index 0 is the first tree; the dump is the last object.
    let dump = u32::try_from(map.objects().len() - 1).unwrap();
    assert!(original.ignite_object(0, UnitId(0)));
    for _ in 0..600 {
        original.step(&[]);
    }
    assert!(original.ignite_object(dump, UnitId(1)));
    original.step(&[]);
    assert!(original.fires().len() > 3, "the wood is not burning");
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map.clone()).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.fires(), original.fires());
    // The dump's fuse runs out after the restore.
    let mut exploded = false;
    for tick in 0..2000 {
        original.step(&[]);
        restored.step(&[]);
        assert_eq!(
            restored.state_hash(),
            original.state_hash(),
            "diverged {tick} ticks after the restore"
        );
        exploded |= restored
            .fire_events()
            .iter()
            .any(|e| e.kind == sim::FireEventKind::Exploded);
    }
    assert!(exploded);
    assert_eq!(
        restored.burnt_objects().collect::<Vec<_>>(),
        original.burnt_objects().collect::<Vec<_>>()
    );
}
