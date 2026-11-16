//! The computer player and its bridge-laying tank (prompt 055): with a
//! narrow river between it and the enemy, Normal lays the bridge.

use ai::Difficulty;
use ai::game::Game;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, FireMode, LayerState, Map, PlayerId, Scale, SimConfig, Spawn,
    TerrainRules, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

/// 300 m × 160 m, a narrow river (|x| <= 3 m, deep).
fn river_map() -> Arc<Map> {
    let (sx, sy) = (151u32, 81u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for _ in 0..sy {
        for i in 0..sx {
            let x = 2 * i as i32 - 150;
            let water = x.abs() <= 3;
            heights.push(if water { -2000 } else { 0 });
            terrain.push(if water { 7 } else { 0 });
        }
    }
    let layout = MapLayout {
        name: "ai_bridgelayer".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(Map::from_rasters(layout, heights, terrain).unwrap())
}

#[test]
fn normal_lays_its_bridge_across_the_river_to_the_enemy() {
    let map = river_map();
    let spawns = [
        (PlayerId(0), "su_valentine_bridgelayer", -100, 10),
        (PlayerId(0), "su_t70m", -110, -10),
        (PlayerId(1), "de_inf_rifle_squad", 120, 0),
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
    let mut game = Game::new(
        &config,
        map,
        3,
        &[
            (PlayerId(0), Difficulty::Normal),
            (PlayerId(1), Difficulty::Passive),
        ],
    )
    .unwrap();
    while !matches!(game.sim().layers()[0].state, LayerState::Laid(_)) {
        assert!(game.sim().tick() < 20 * 300, "no bridge laid");
        let d = game.think();
        game.step(d);
    }
    let bridge = &game.sim().map().bridges()[0];
    assert!(bridge.center.x.abs() < sim::Fixed::from_num(8));
}
