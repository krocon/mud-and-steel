//! The ground tooltip's lookup (prompt 100): known points of a hand-made
//! map give the terrain, movement, concealment and cover the rules use.

use sim::map::MapLayout;
use sim::vision::Sight;
use sim::{
    Catalog, CombatRules, FireMode, Map, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules,
    Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const N: u32 = 201;

/// 400 m × 400 m around the origin, terrain by x in meters: road up to
/// -100, grass to 0, mud to 50, deep water from 150; objects as CSV.
fn map(objects: &[&str]) -> Arc<Map> {
    let layout = MapLayout {
        name: "ground_info_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let mut ids = Vec::new();
    for _ in 0..N as i32 {
        for i in 0..N as i32 {
            let x = (i - half) * 2;
            ids.push(match x {
                ..-100 => 2,
                -100..0 => 0,
                0..50 => 4,
                50..150 => 0,
                _ => 7,
            });
        }
    }
    let mut m = Map::from_rasters(layout, vec![0; (N * N) as usize], ids).unwrap();
    m.add_objects(&format!(
        "{}\n{}\n",
        sim::objects::CSV_HEADER,
        objects.join("\n")
    ))
    .unwrap();
    Arc::new(m)
}

fn sim(map: Arc<Map>) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let names = ["de_pz4_h", "su_t34_76_1943"];
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: vec![
            Spawn {
                owner: P0,
                unit_type: "de_pz4_h".into(),
                position: Vec2::from_meters(-60, -150),
                fire_mode: FireMode::HoldFire,
            },
            Spawn {
                owner: P1,
                unit_type: "su_t34_76_1943".into(),
                position: Vec2::from_meters(100, 150),
                fire_mode: FireMode::HoldFire,
            },
        ],
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
    Sim::new(&config, map, 1).unwrap()
}

#[test]
fn known_points_give_the_rules_values() {
    // A wood of six trees around (-40, 40), a farm house at (-60, -40), a
    // rock at (100, 0).
    let mut objects = vec![
        "env_house_farm,Intact,-60000,-40000,0".to_string(),
        "env_rock,Medium,100000,0,0".to_string(),
    ];
    for k in 0..6 {
        objects.push(format!(
            "env_tree_deciduous,Oak,{},{},0",
            -42000 + (k % 3) * 2500,
            38000 + (k / 3) * 2500
        ));
    }
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    let mut s = sim(map(&refs));
    s.step(&[]);
    let at = |x: i32, y: i32| s.ground_info(P0, Vec2::from_meters(x, y)).unwrap();

    let road = at(-150, 0);
    assert_eq!(road.terrain_name, "road");
    assert!(road.road && road.passable_small && road.passable_large);
    assert_eq!(road.object, None);
    assert_eq!(road.sight, Sight::Clear);
    assert!(
        road.gears.iter().all(|g| g.1 == 100),
        "clear weather: firm ground"
    );

    let mud = at(25, -100);
    assert_eq!((mud.terrain_name.as_str(), mud.speed_pct), ("mud", 40));
    assert!(!mud.road);

    let water = at(170, 0);
    assert_eq!(water.terrain_name, "deep_water");
    assert!(!water.passable_small && !water.passable_large);

    let house = at(-60, -40);
    assert_eq!(house.object.as_deref(), Some("env_house_farm"));
    assert!(house.building && house.hard_cover && !house.passable_large);
    assert_eq!(house.sight, Sight::Solid);

    let wood = at(-40, 40);
    assert_eq!(wood.object.as_deref(), Some("env_tree_deciduous"));
    assert!(wood.hard_cover, "a tree trunk stops bullets");
    assert_eq!(wood.sight, Sight::Foliage);

    let rock = at(100, 0);
    assert_eq!(rock.object.as_deref(), Some("env_rock"));
    assert!(!rock.hard_cover);
    assert_eq!(rock.object_cover_pct, 60, "data/infantry.toml env_rock");

    assert_eq!(at(-150, 0).grade_pct, 0);
    assert_eq!(at(-150, 0).slope_pct, Some(100));
    assert!(s.ground_info(P0, Vec2::from_meters(500, 0)).is_none());
}
