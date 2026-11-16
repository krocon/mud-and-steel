//! Prompt 040: the weather state (ground softness, path costs being
//! computed again, muzzle flashes at night) is part of the snapshot of
//! prompt 038; a restored match goes on exactly like the original.

use net::snapshot;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Daytime, FireMode, Map, PlayerId, Scale, Sim, SimConfig, Spawn,
    TerrainRules, Vec2, VisionRules, Weather, WeatherSetup, unit_types_from_catalog,
};
use std::sync::Arc;

fn track_map() -> Arc<Map> {
    let n: u32 = 201;
    let layout = MapLayout {
        name: "weather_snapshot".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    // Dirt track (terrain id 1): turns to mud in the rain.
    Arc::new(Map::from_rasters(layout, vec![0; size], vec![1; size]).unwrap())
}

fn new_sim(map: Arc<Map>) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let names = ["de_pz4_h", "su_t34_76_1943"];
    let spawn = |owner: u8, unit: &str, y: i32| Spawn {
        owner: PlayerId(owner),
        unit_type: unit.into(),
        position: Vec2::from_meters(0, y),
        fire_mode: FireMode::FireAtWill,
    };
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: vec![spawn(0, "de_pz4_h", -60), spawn(1, "su_t34_76_1943", 60)],
        map: map.reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: WeatherSetup {
            weather: Weather::Rain,
            daytime: Daytime::Night,
        },
        mission: None,
    };
    Sim::new(&config, map, 5).unwrap()
}

#[test]
fn rainy_night_restores_and_goes_on_identically() {
    let map = track_map();
    let mut original = new_sim(map.clone());
    // One tick after the first ground step: the path costs are being
    // computed again, row band by row band.
    for _ in 0..(120 * 20 + 1) {
        original.step(&[]);
    }
    let (_, level) = original.ground_state();
    assert!(level > 50, "the ground got softer: {level}");
    let bytes = snapshot::encode(&original, &map);
    let mut restored = snapshot::decode(&bytes, map.clone()).expect("snapshot restores");
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.weather_setup(), original.weather_setup());
    assert_eq!(restored.ground_state(), original.ground_state());
    for _ in 0..200 {
        original.step(&[]);
        restored.step(&[]);
    }
    assert_eq!(restored.state_hash(), original.state_hash());
    assert_eq!(restored.path_grid().costs(0), original.path_grid().costs(0));
}
