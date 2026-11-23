//! The computer player and a long bridge (prompt 115): across a 280 m
//! river with a 364 m steel bridge on earth dams it finds the way over the
//! bridge to the enemy; with the bridge cut it stays on its bank without
//! hanging (no endless re-planning).

use ai::Difficulty;
use ai::game::Game;
use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, FireMode, Fixed, Map, PlayerId, Scale, SimConfig, Spawn, TerrainRules,
    Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const AXIS_Y: i32 = 1;

/// 1600 m × 400 m: deep water for |x| <= 140 m, banks at 0 m, earth dams
/// from the bridge ends (|x| = 182 m, 18 m high, 8 m of abutment under
/// the deck) falling at 12 %, crest 20 m wide, sides too steep to climb.
/// The bridge: west approach, 244 m arch, east approach (bridges 0-2).
fn river_map(cut: bool) -> Arc<Map> {
    let (sx, sy) = (801u32, 201u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for i in 0..sx {
            let (x, y) = (2 * i as i32 - 800, 2 * j as i32 - 200);
            let water = x.abs() <= 140;
            let run = x.abs() - 182;
            let h = if water {
                -3000
            } else if run < -8 {
                0
            } else {
                let crest = 18_000 - run.max(0) * 120;
                (crest - ((y - AXIS_Y).abs() - 10).max(0) * 1000).max(0)
            };
            heights.push(h);
            terrain.push(if water { 7 } else { 0 });
        }
    }
    let layout = MapLayout {
        name: "ai_long_bridge".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, terrain).unwrap();
    map.add_objects(&format!(
        "kind,variant,x_mm,y_mm,heading_deg\n\
         env_bridge_steel_approach,Span60,-152000,{y},90\n\
         env_bridge_steel_arch,Arch244,0,{y},90\n\
         env_bridge_steel_approach,Span60,152000,{y},270\n",
        y = AXIS_Y * 1000
    ))
    .unwrap();
    if cut {
        map.set_bridge_intact(2, false);
    }
    Arc::new(map)
}

fn game(cut: bool) -> Game {
    let map = river_map(cut);
    let spawns = [
        (PlayerId(0), "su_t34_76_1943", -560, -40),
        (PlayerId(0), "su_t34_76_1943", -560, 40),
        (PlayerId(0), "su_inf_rifle_squad", -600, 0),
        (PlayerId(1), "de_inf_rifle_squad", 560, 0),
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
    Game::new(
        &config,
        map,
        5,
        &[
            (PlayerId(0), Difficulty::Normal),
            (PlayerId(1), Difficulty::Passive),
        ],
    )
    .unwrap()
}

/// Easternmost x of player 0's units.
fn front(game: &Game) -> Fixed {
    game.sim()
        .snapshot()
        .units
        .iter()
        .filter(|u| u.owner == PlayerId(0) && u.hp > 0)
        .map(|u| u.position.x)
        .max()
        .unwrap_or(Fixed::from_num(-1000))
}

#[test]
fn the_ai_crosses_the_long_bridge() {
    let mut game = game(false);
    let mut top = Fixed::from_num(-100);
    while front(&game) < Fixed::from_num(250) {
        assert!(
            game.sim().tick() < 20 * 600,
            "the AI did not cross the bridge (front {})",
            front(&game)
        );
        let d = game.think();
        game.step(d);
        for u in game
            .sim()
            .snapshot()
            .units
            .iter()
            .filter(|u| u.owner == PlayerId(0))
        {
            top = top.max(u.height);
        }
    }
    assert!(top >= Fixed::from_num(17), "it crossed on the deck");
}

#[test]
fn with_the_bridge_cut_the_ai_stays_without_hanging() {
    let mut game = game(true);
    let mut early = 0;
    while game.sim().tick() < 20 * 400 {
        let d = game.think();
        game.step(d);
        assert!(front(&game) < Fixed::from_num(140), "crossed a cut bridge");
        if game.sim().tick() == 20 * 200 {
            early = game.sim().path_stats().requests;
        }
    }
    // No endless re-planning: few requests in the last 200 s, and the
    // queue is empty.
    let late = game.sim().path_stats().requests - early;
    eprintln!("path requests: {early} in the first 200 s, {late} in the next 200 s");
    assert!(late <= 80, "{late} path requests in 200 s");
    assert!(
        game.sim().path_queue_len() <= 2,
        "queue {}",
        game.sim().path_queue_len()
    );
}
