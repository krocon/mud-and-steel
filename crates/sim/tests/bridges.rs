//! Bridges and fords (prompt 014): units cross deep water only on a deck
//! or through a ford, heavy units avoid weak bridges, destroyed bridges
//! make units plan again.

use sim::map::MapLayout;
use sim::{
    Command, Fixed, Map, MapRef, PlayerCommand, PlayerId, Sim, SimConfig, Spawn, TerrainRules,
    UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const ROAD: u8 = 2;
const SHALLOW: u8 = 6;
const DEEP: u8 = 7;

/// Axis of the timber bridge (a path cell center line).
const WOOD_Y: i32 = -39;
/// Axis of the stone bridge.
const STONE_Y: i32 = 1;
/// The ford: shallow water for these y.
const FORD_Y: (i32, i32) = (40, 56);

/// 200 m x 120 m. A river runs north-south: deep water for |x| <= 5 m, its
/// bed 2 m below the banks; a ford with a shallow bed at `FORD_Y`; a timber
/// bridge (16 t) at `WOOD_Y` and a stone bridge at `STONE_Y`.
fn river_map(objects: &str) -> Result<Map, sim::MapError> {
    let (sx, sy) = (101u32, 61u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for i in 0..sx {
            let (x, y) = (2 * i as i32 - 100, 2 * j as i32 - 60);
            let ford = (FORD_Y.0..=FORD_Y.1).contains(&y);
            if x.abs() <= 5 {
                heights.push(if ford { -300 } else { -2000 });
                terrain.push(if ford { SHALLOW } else { DEEP });
            } else {
                heights.push(0);
                terrain.push(0);
            }
        }
    }
    let layout = MapLayout {
        name: "river".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, terrain)?;
    map.add_objects(objects)?;
    Ok(map)
}

fn bridges_csv() -> String {
    format!(
        "kind,variant,x_mm,y_mm,heading_deg\n\
         env_bridge_wood,Span16,0,{},90\n\
         env_bridge_stone,Arch1,0,{},90\n",
        WOOD_Y * 1000,
        STONE_Y * 1000
    )
}

fn unit_type(name: &str, footprint_m: i32, mass_kg: u32) -> UnitType {
    UnitType {
        name: name.into(),
        speed_m_per_s: Fixed::from_num(4),
        road_speed_m_per_s: Fixed::from_num(8),
        footprint_m: Fixed::from_num(footprint_m),
        mass_kg,
    }
}

fn sim(map: &Arc<Map>, spawns: &[(&str, i32, i32)]) -> Sim {
    let config = SimConfig {
        tick_hz: 20,
        unit_types: vec![
            unit_type("infantry", 1, 0),
            unit_type("halftrack", 6, 9_000),
            unit_type("tank", 7, 25_000),
        ],
        spawns: spawns
            .iter()
            .map(|&(t, x, y)| Spawn {
                owner: P0,
                unit_type: t.into(),
                position: Vec2::from_meters(x, y),
            })
            .collect(),
        map: MapRef {
            name: map.name().into(),
            hash: map.hash(),
        },
        terrain: TerrainRules::builtin().unwrap(),
    };
    Sim::new(&config, Arc::clone(map), 7).unwrap()
}

fn move_to(sim: &mut Sim, ids: &[u32], x: i32, y: i32) {
    sim.step(&[PlayerCommand {
        player: P0,
        command: Command::MoveTo {
            units: ids.iter().map(|&i| UnitId(i)).collect(),
            target: Vec2::from_meters(x, y),
        },
    }]);
}

/// Runs until every unit stopped (at most `ticks`); returns, per unit, the
/// y coordinates at which it was in the river (|x| <= 5 m). Fails if a unit
/// ever stands on deep water without a deck, or below the deck on one.
fn run(sim: &mut Sim, ticks: u32) -> Vec<Vec<Fixed>> {
    let map = Arc::clone(sim.map());
    let mut crossings = vec![Vec::new(); sim.snapshot().units.len()];
    for _ in 0..ticks {
        sim.step(&[]);
        let snapshot = sim.snapshot();
        for (k, u) in snapshot.units.iter().enumerate() {
            assert_ne!(
                map.terrain_at(u.position),
                DEEP,
                "unit {k} in deep water at {:?}",
                u.position
            );
            assert_eq!(u.height, map.height_at(u.position));
            if u.position.x.abs() <= Fixed::from_num(5) {
                crossings[k].push(u.position.y);
            }
        }
        if snapshot.units.iter().all(|u| u.target.is_none()) {
            break;
        }
    }
    crossings
}

fn near(values: &[Fixed], y: i32, tolerance: i32) -> bool {
    !values.is_empty()
        && values
            .iter()
            .all(|v| (*v - Fixed::from_num(y)).abs() <= Fixed::from_num(tolerance))
}

#[test]
fn deck_is_the_surface_on_a_bridge() {
    let map = river_map(&bridges_csv()).unwrap();
    assert_eq!(map.bridges().len(), 2);
    let on_wood = Vec2::from_meters(0, WOOD_Y);
    assert_eq!(map.ground_height_at(on_wood), Fixed::from_num(-2));
    assert_eq!(map.height_at(on_wood), Fixed::ZERO);
    assert_eq!(map.ground_terrain_at(on_wood), DEEP);
    assert_eq!(map.terrain_at(on_wood), ROAD);
    // Beside the deck (roadway 4.1 m): water.
    let beside = Vec2::from_meters(0, WOOD_Y + 3);
    assert_eq!(map.terrain_at(beside), DEEP);
    assert_eq!(map.height_at(beside), Fixed::from_num(-2));
    // The wood deck ends 8 m from its center.
    assert_eq!(map.terrain_at(Vec2::from_meters(9, WOOD_Y)), 0);
    assert!(map.deck_at(Vec2::from_meters(8, WOOD_Y)).is_some());
    // Destroyed: no deck any more.
    let mut broken = map.clone();
    assert!(broken.set_bridge_intact(0, false));
    assert_eq!(broken.terrain_at(on_wood), DEEP);
    assert_eq!(broken.height_at(on_wood), Fixed::from_num(-2));
}

#[test]
fn deck_height_follows_the_banks() {
    let map = river_map(&bridges_csv()).unwrap();
    let b = &map.bridges()[1];
    assert_eq!((b.height_start, b.height_end), (Fixed::ZERO, Fixed::ZERO));
    assert_eq!(b.max_load_kg, 100_000);
    assert_eq!(b.half_length, Fixed::from_num(10));
}

#[test]
fn bridge_off_the_path_grid_is_rejected() {
    // Diagonal: the cells along the axis have their centers off the deck.
    let csv = "kind,variant,x_mm,y_mm,heading_deg\nenv_bridge_wood,Span12,0,-39000,45\n";
    let e = river_map(csv).unwrap_err();
    assert!(e.0.contains("align the axis"), "{e}");
    // Beyond the map edge.
    let csv = "kind,variant,x_mm,y_mm,heading_deg\nenv_bridge_stone,Arch1,95000,1000,90\n";
    assert!(river_map(csv).unwrap_err().0.contains("beyond the map"));
}

#[test]
fn light_units_take_the_nearest_bridge_heavy_ones_avoid_the_weak_one() {
    let map = Arc::new(river_map(&bridges_csv()).unwrap());
    // From west to east at the height of the timber bridge.
    let mut s = sim(
        &map,
        &[
            ("infantry", -40, WOOD_Y),
            ("halftrack", -40, WOOD_Y - 12),
            ("tank", -40, WOOD_Y + 12),
        ],
    );
    move_to(&mut s, &[0], 40, WOOD_Y);
    move_to(&mut s, &[1], 40, WOOD_Y - 12);
    move_to(&mut s, &[2], 40, WOOD_Y + 12);
    let crossed = run(&mut s, 3000);
    assert!(near(&crossed[0], WOOD_Y, 2), "infantry {:?}", crossed[0]);
    assert!(near(&crossed[1], WOOD_Y, 2), "halftrack {:?}", crossed[1]);
    // 25 t > 16 t: the tank uses the stone bridge.
    assert!(near(&crossed[2], STONE_Y, 2), "tank {:?}", crossed[2]);
    for u in s.snapshot().units {
        assert!(u.position.x > Fixed::from_num(30), "{u:?}");
    }
}

#[test]
fn groups_use_the_most_restrictive_profile() {
    let map = Arc::new(river_map(&bridges_csv()).unwrap());
    let mut s = sim(
        &map,
        &[
            ("infantry", -40, WOOD_Y),
            ("tank", -45, WOOD_Y),
            ("halftrack", -40, WOOD_Y + 10),
        ],
    );
    move_to(&mut s, &[0, 1, 2], 40, WOOD_Y);
    let crossed = run(&mut s, 3000);
    // The group path avoids the timber bridge: the tank must not use it.
    assert!(near(&crossed[1], STONE_Y, 3), "tank {:?}", crossed[1]);
    assert!(!crossed[1].is_empty());
}

#[test]
fn destroyed_bridge_sends_units_to_the_ford() {
    let map = Arc::new(river_map(&bridges_csv()).unwrap());
    let mut s = sim(&map, &[("tank", -40, STONE_Y)]);
    move_to(&mut s, &[0], 40, STONE_Y);
    for _ in 0..5 {
        s.step(&[]);
    }
    let before = s.state_hash();
    s.set_bridge_intact(1, false);
    assert_ne!(s.state_hash(), before);
    assert!(!s.map().bridges()[1].intact);
    // The tank's map is now a copy; the shared one is unchanged.
    assert!(map.bridges()[1].intact);
    let crossed = run(&mut s, 4000);
    assert!(
        crossed[0]
            .iter()
            .all(|y| (Fixed::from_num(FORD_Y.0 - 2)..=Fixed::from_num(FORD_Y.1 + 2)).contains(y)),
        "tank {:?}",
        crossed[0]
    );
    assert!(s.snapshot().units[0].position.x > Fixed::from_num(30));

    // Repaired: the way back uses the bridge again.
    s.set_bridge_intact(1, true);
    move_to(&mut s, &[0], -40, STONE_Y);
    let crossed = run(&mut s, 4000);
    assert!(near(&crossed[0], STONE_Y, 2), "tank {:?}", crossed[0]);
}

#[test]
fn bridge_runs_are_deterministic() {
    let map = Arc::new(river_map(&bridges_csv()).unwrap());
    let play = || {
        let mut s = sim(
            &map,
            &[
                ("tank", -40, 0),
                ("infantry", -40, -20),
                ("halftrack", -40, 20),
            ],
        );
        move_to(&mut s, &[0, 1, 2], 40, -10);
        for t in 0..1500 {
            if t == 200 {
                s.set_bridge_intact(0, false);
            }
            s.step(&[]);
        }
        s.state_hash()
    };
    assert_eq!(play(), play());
}

#[test]
fn unreachable_deck_targets_snap_for_heavy_units() {
    let map = Arc::new(river_map(&bridges_csv()).unwrap());
    let grid = sim(&map, &[("tank", -40, 0)]);
    let grid = grid.path_grid();
    // The middle of the timber deck: open for infantry, closed for the tank.
    let target = Vec2::from_meters(1, WOOD_Y);
    let (x, y) = grid.cell_of(target);
    assert_eq!(grid.deck_of(x, y), Some(0));
    assert!(grid.passable_for(0, target));
    assert!(!grid.passable_for(2, target));
    let path = grid
        .find_path(Vec2::from_meters(-40, WOOD_Y), target, 2)
        .unwrap();
    assert_ne!(path.goal, target);
    assert!(map.deck_at(path.goal).is_none());
    let path = grid
        .find_path(Vec2::from_meters(-40, WOOD_Y), target, 0)
        .unwrap();
    assert_eq!(path.goal, target);
}

#[test]
fn railings_keep_units_on_the_deck() {
    let map = river_map(&bridges_csv()).unwrap();
    let on = Vec2::from_meters(0, WOOD_Y);
    let edge = Vec2::new(Fixed::ZERO, Fixed::from_num(WOOD_Y) + Fixed::from_num(2));
    let beside = Vec2::new(Fixed::ZERO, Fixed::from_num(WOOD_Y) + Fixed::from_num(3));
    assert!(map.deck_step_ok(on, edge));
    // Over the side: a 2 m drop into the river.
    assert!(!map.deck_step_ok(edge, beside));
    assert!(!map.deck_step_ok(beside, edge));
    // Over the end: onto the bank at deck height.
    let end = Vec2::from_meters(8, WOOD_Y);
    assert!(map.deck_step_ok(end, Vec2::from_meters(9, WOOD_Y)));
    // Beside the end, where the bank is as high as the deck.
    assert!(map.deck_step_ok(
        Vec2::from_meters(7, WOOD_Y + 2),
        Vec2::from_meters(7, WOOD_Y + 3)
    ));
}

#[test]
fn units_do_not_drive_down_cliffs() {
    let rules = TerrainRules::builtin().unwrap();
    let max = Fixed::from_num(rules.slope.max_grade_pct) / 100;
    assert!(rules.slope.factor(-max).is_some());
    assert!(
        rules
            .slope
            .factor(-max - Fixed::from_num(1) / 100)
            .is_none()
    );
}
