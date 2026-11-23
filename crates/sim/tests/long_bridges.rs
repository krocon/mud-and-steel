//! Long bridges over a wide river (prompt 115): a 244 m steel arch between
//! two 60 m approach spans forms one roadway 18 m above the water, reached
//! over earth dams. Tanks and infantry cross it; demolishing one approach
//! span parts the banks and units plan again without hanging; nobody drives
//! off the hanging end of the arch; the deck hides nothing; there is no
//! underpass (traffic goes round the dam); runs are deterministic. A
//! mission script blows up pieces and loses the mission when one is down
//! (prompt 116).

use sim::map::MapLayout;
use sim::{
    Catalog, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId, RouteView, Scale,
    Sim, SimConfig, Spawn, TerrainRules, UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const DEEP: u8 = 7;
const HEAVY_TANK: &str = "su_is2";
const RIFLES: &str = "de_inf_rifle_squad";
const TRUCK: &str = "de_opel_blitz";
const ENGINEERS: &str = "de_inf_engineer";

/// Axis of the bridge (a path cell center line on this map).
const AXIS_Y: i32 = 1;
/// Deep water for |x| <= this (m), 3 m below the banks.
const RIVER_HALF: i32 = 140;
/// Outer ends of the bridge (|x|, m): 122 m of arch + 60 m of approach.
const BRIDGE_HALF: i32 = 182;
/// Deck height above the banks (m).
const DECK_M: i32 = 18;
/// Ramp grade of the dams (per mille).
const RAMP_PML: i32 = 120;
/// Half width of the dam crest (m).
const CREST: i32 = 10;

/// Ground height (mm) of the test map at `(x, y)` meters: a river, banks
/// at 0 m and earth dams from the bridge ends down to the banks (crest
/// 20 m wide, sides at 100 %, too steep to climb; 8 m of abutment under
/// each deck end).
fn ground_mm(x: i32, y: i32) -> i32 {
    if x.abs() <= RIVER_HALF {
        return -3000;
    }
    // The dam reaches 8 m under the deck end (the abutment), then falls.
    let run = x.abs() - BRIDGE_HALF;
    if run < -8 {
        return 0;
    }
    let crest = DECK_M * 1000 - run.max(0) * RAMP_PML;
    let side = ((y - AXIS_Y).abs() - CREST).max(0) * 1000;
    (crest - side).max(0)
}

/// 1600 m × 400 m, 2 m samples.
fn river_map() -> Map {
    let (sx, sy) = (801u32, 201u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for j in 0..sy {
        for i in 0..sx {
            let (x, y) = (2 * i as i32 - 800, 2 * j as i32 - 200);
            heights.push(ground_mm(x, y));
            terrain.push(if x.abs() <= RIVER_HALF { DEEP } else { 0 });
        }
    }
    let layout = MapLayout {
        name: "long_bridge".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, terrain).unwrap();
    // West approach, main arch, east approach (bridges 0, 1, 2).
    map.add_objects(&format!(
        "kind,variant,x_mm,y_mm,heading_deg\n\
         env_bridge_steel_approach,Span60,-152000,{y},90\n\
         env_bridge_steel_arch,Arch244,0,{y},90\n\
         env_bridge_steel_approach,Span60,152000,{y},270\n",
        y = AXIS_Y * 1000
    ))
    .unwrap();
    map
}

fn sim_on(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let unit_types: Vec<UnitType> = names
        .iter()
        .map(|id| UnitType::from_catalog(catalog.get(id).unwrap(), &scale).unwrap())
        .collect();
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
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
    Sim::new(&config, map, seed).unwrap()
}

fn move_to(player: PlayerId, units: &[u32], x: i32, y: i32) -> PlayerCommand {
    PlayerCommand::new(
        player,
        Command::MoveTo {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            target: Vec2::from_meters(x, y),
        },
    )
}

fn idle(sim: &Sim, id: u32) -> bool {
    matches!(sim.route_of(UnitId(id)), Some(RouteView::Idle))
}

/// Steps until every listed unit is idle (at most `limit` ticks); checks on
/// every tick that no unit stands in deep water without a deck or below
/// a deck, and returns the highest height every unit reached.
fn run_until_idle(sim: &mut Sim, units: &[u32], limit: u32) -> Vec<Fixed> {
    let map = Arc::clone(sim.map());
    let mut top = vec![Fixed::from_num(-100); units.len()];
    for tick in 0..limit {
        sim.step(&[]);
        let map_now = Arc::clone(sim.map());
        let snapshot = sim.snapshot();
        for (k, &id) in units.iter().enumerate() {
            let u = snapshot.units.iter().find(|u| u.id == UnitId(id)).unwrap();
            if u.hp == 0 {
                continue;
            }
            assert_ne!(
                map_now.terrain_at(u.position),
                DEEP,
                "unit {id} in deep water at {:?}",
                u.position
            );
            assert_eq!(
                u.height,
                map_now.height_at(u.position),
                "unit {id} off the surface"
            );
            top[k] = top[k].max(u.height);
        }
        if tick > 2 && units.iter().all(|&id| idle(sim, id)) {
            return top;
        }
    }
    let _ = map;
    panic!("units not idle within {limit} ticks");
}

fn x_of(sim: &Sim, id: u32) -> Fixed {
    sim.unit_position(UnitId(id)).unwrap().x
}

#[test]
fn pieces_end_to_end_form_one_level_roadway() {
    let map = river_map();
    let bridges = map.bridges();
    assert_eq!(bridges.len(), 3);
    let deck = Fixed::from_num(DECK_M);
    for (k, b) in bridges.iter().enumerate() {
        assert_eq!(b.height_start, deck, "bridge {k} start");
        assert_eq!(b.height_end, deck, "bridge {k} end");
        assert!(b.see_through);
        assert_eq!(b.railing_cover_pct, 50);
    }
    // Over the middle of the river the deck lies 21 m above the bed.
    let middle = Vec2::from_meters(0, AXIS_Y);
    assert_eq!(map.height_at(middle), deck);
    assert_eq!(map.ground_height_at(middle), Fixed::from_num(-3));
    // Only the outer ends meet the ground (the dams); the joints hang.
    // The east approach is turned (heading 270): its start lies east.
    assert_eq!(bridges[0].hangs, [false, true]);
    assert_eq!(bridges[1].hangs, [true, true]);
    assert_eq!(bridges[2].hangs, [false, true]);
}

#[test]
fn tanks_and_infantry_cross_the_long_bridge() {
    let map = Arc::new(river_map());
    let mut sim = sim_on(
        map,
        &[
            (P0, HEAVY_TANK, -500, AXIS_Y),
            (P0, RIFLES, -500, AXIS_Y + 30),
        ],
        7,
    );
    sim.step(&[
        move_to(P0, &[0], 500, AXIS_Y),
        move_to(P0, &[1], 500, AXIS_Y + 30),
    ]);
    let top = run_until_idle(&mut sim, &[0, 1], 20 * 600);
    for (k, &h) in top.iter().enumerate() {
        assert!(
            h >= Fixed::from_num(DECK_M - 1),
            "unit {k} never was on the deck ({h})"
        );
    }
    assert!(
        x_of(&sim, 0) > Fixed::from_num(450),
        "tank at {}",
        x_of(&sim, 0)
    );
    assert!(
        x_of(&sim, 1) > Fixed::from_num(450),
        "squad at {}",
        x_of(&sim, 1)
    );
}

#[test]
fn a_demolished_approach_parts_the_banks_and_units_plan_again() {
    let run = |seed: u64| {
        let map = Arc::new(river_map());
        let mut sim = sim_on(
            map,
            &[
                (P0, HEAVY_TANK, -500, AXIS_Y),
                (P0, RIFLES, -480, AXIS_Y + 30),
            ],
            seed,
        );
        sim.step(&[move_to(P0, &[0, 1], 500, AXIS_Y)]);
        for _ in 0..20 * 10 {
            sim.step(&[]);
        }
        assert!(
            x_of(&sim, 0) < Fixed::from_num(-200),
            "the tank should still be on the west bank"
        );
        // The east approach falls: the banks are parted.
        sim.set_bridge_intact(2, false);
        let top = run_until_idle(&mut sim, &[0, 1], 20 * 600);
        // They drove as far as the arch goes and stopped at its hanging
        // end, on the deck: nobody fell off or kept trying.
        for (k, id) in [0u32, 1].into_iter().enumerate() {
            let x = x_of(&sim, id);
            assert!(x < Fixed::from_num(123), "unit {id} at x = {x}");
            assert!(
                x > Fixed::from_num(60),
                "unit {id} at x = {x}, expected near the gap"
            );
            assert!(top[k] >= Fixed::from_num(DECK_M - 1));
        }
        for _ in 0..20 * 30 {
            sim.step(&[]);
        }
        assert!(idle(&sim, 0) && idle(&sim, 1), "units keep planning");
        // Restored, they go on to the target.
        sim.set_bridge_intact(2, true);
        sim.step(&[move_to(P0, &[0, 1], 500, AXIS_Y)]);
        run_until_idle(&mut sim, &[0, 1], 20 * 600);
        assert!(x_of(&sim, 0) > Fixed::from_num(450));
        sim.state_hash()
    };
    assert_eq!(run(3), run(3));
}

#[test]
fn engineers_blow_up_one_piece_only() {
    let map = Arc::new(river_map());
    let mut sim = sim_on(
        map,
        &[(P0, ENGINEERS, -400, AXIS_Y), (P0, TRUCK, 150, AXIS_Y)],
        5,
    );
    sim.step(&[PlayerCommand::new(
        P0,
        Command::DemolishBridge {
            units: vec![UnitId(0)],
            bridge: 2,
        },
    )]);
    for _ in 0..20 * 400 {
        if !sim.map().bridges()[2].intact {
            break;
        }
        sim.step(&[]);
    }
    let bridges = sim.map().bridges();
    assert!(!bridges[2].intact, "the east approach is down");
    assert!(
        bridges[0].intact && bridges[1].intact,
        "the other pieces stand"
    );
    // The truck stood on the fallen piece: it is lost.
    assert_eq!(sim.unit_hp(UnitId(1)), Some(0));
    // The engineers worked from the arch beside the joint.
    let at = sim.unit_position(UnitId(0)).unwrap();
    assert!(
        at.x < Fixed::from_num(123) && at.x > Fixed::from_num(100),
        "engineers at {at:?}"
    );
}

/// Prompt 116: the mission action `demolish` brings down the pieces
/// within its radius (the arch, not the approaches), what stands on them
/// falls, and `bridge_down` decides the mission.
#[test]
fn a_mission_blows_up_the_arch_and_is_lost() {
    let script = r#"
[mission]
title = "Bridge"
player = 0
elimination = false

[[area]]
name = "arch"
center_m = [0, 1]
radius_m = 20

[[group]]
name = "tank"
owner = 0
unit = "de_opel_blitz"
position_m = [60, 1]

[[group]]
name = "enemy"
owner = 1
unit = "de_inf_rifle_squad"
position_m = [-500, 100]
hold_fire = true

[[objective]]
id = "hold"
text = "Hold."
shown = true

[[trigger]]
id = "fuse"
when = "time"
delay_s = 2
actions = [{ do = "demolish", at = "arch", radius_m = 60 }]

[[trigger]]
id = "lost"
when = "bridge_down"
area = "arch"
actions = [{ do = "defeat" }]
"#;
    let run = || {
        let map = Arc::new(river_map());
        let config = sim::mission::parse_mission(
            "test",
            script,
            None,
            &Catalog::builtin().unwrap(),
            &Scale::builtin().unwrap(),
            &TerrainRules::builtin().unwrap(),
            &map,
        )
        .unwrap();
        let mut sim = Sim::new(&config, map, 7).unwrap();
        for _ in 0..20 {
            sim.step(&[]);
        }
        assert!(sim.map().bridges()[1].intact, "not before the delay");
        assert_eq!(sim.outcome(), None);
        for _ in 0..40 {
            sim.step(&[]);
        }
        let bridges = sim.map().bridges();
        assert!(!bridges[1].intact, "the arch is down");
        assert!(
            bridges[0].intact && bridges[2].intact,
            "the approaches stand"
        );
        // The truck stood on the arch: it fell with it.
        assert_eq!(sim.unit_hp(UnitId(0)), Some(0));
        let outcome = sim.outcome().expect("lost");
        assert_eq!(outcome.winner, Some(P1));
        sim.state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn no_underpass_traffic_goes_round_the_dam() {
    // A truck under the land part of the east approach (the deck is 18 m
    // up) wants to the other side of the bridge line.
    let map = Arc::new(river_map());
    let mut sim = sim_on(map, &[(P0, TRUCK, 160, -100)], 1);
    sim.step(&[move_to(P0, &[0], 160, 100)]);
    let mut crossed_at = None;
    for _ in 0..20 * 600 {
        sim.step(&[]);
        let p = sim.unit_position(UnitId(0)).unwrap();
        if (p.y - Fixed::from_num(AXIS_Y)).abs() < Fixed::from_num(7) && crossed_at.is_none() {
            crossed_at = Some(p.x);
        }
        if idle(&sim, 0) {
            break;
        }
    }
    let p = sim.unit_position(UnitId(0)).unwrap();
    assert!(
        (p - Vec2::from_meters(160, 100)).length() < Fixed::from_num(10),
        "truck at {p:?}"
    );
    let x = crossed_at.expect("the truck crossed the bridge line");
    assert!(
        x > Fixed::from_num(BRIDGE_HALF + DECK_M * 1000 / RAMP_PML - 10),
        "crossed the bridge line at x = {x}, under the deck or over the dam"
    );
}

#[test]
fn the_deck_hides_nothing() {
    // Two squads on the east bank, either side of the approach span (on
    // land, deck 18 m above them): they see each other under the deck.
    let map = Arc::new(river_map());
    let mut sim = sim_on(
        map,
        &[
            (P0, RIFLES, 160, AXIS_Y - 60),
            (P1, RIFLES, 160, AXIS_Y + 60),
        ],
        2,
    );
    for _ in 0..40 {
        sim.step(&[]);
    }
    // The vision grid has the deck as the height where units stand ...
    let grid = sim.vision_grid();
    let under = grid.index_of(Vec2::from_meters(160, AXIS_Y));
    assert_eq!(grid.ground_mm(under), DECK_M * 1000);
    // ... but sight passes under it.
    assert!(sim.visible_to(P0, UnitId(1)));
    assert!(sim.visible_to(P1, UnitId(0)));
}

/// 600 m × 200 m of land: two 60 m approach spans end to end over a
/// street (x from -60 m to 60 m, joint at 0) between two dams.
fn viaduct_map() -> Map {
    let (sx, sy) = (301u32, 101u32);
    let mut heights = Vec::new();
    for j in 0..sy {
        for i in 0..sx {
            let (x, y) = (2 * i as i32 - 300, 2 * j as i32 - 100);
            let run = x.abs() - 60;
            let h = if run < -8 {
                0
            } else {
                let crest = DECK_M * 1000 - run.max(0) * RAMP_PML;
                (crest - ((y - AXIS_Y).abs() - CREST).max(0) * 1000).max(0)
            };
            heights.push(h);
        }
    }
    let layout = MapLayout {
        name: "viaduct".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, vec![0; (sx * sy) as usize]).unwrap();
    map.add_objects(&format!(
        "kind,variant,x_mm,y_mm,heading_deg\n\
         env_bridge_steel_approach,Span60,-30000,{y},90\n\
         env_bridge_steel_approach,Span60,30000,{y},90\n",
        y = AXIS_Y * 1000
    ))
    .unwrap();
    map
}

#[test]
fn nobody_drives_off_a_hanging_joint() {
    let map = Arc::new(viaduct_map());
    assert_eq!(map.bridges()[0].hangs, [false, true]);
    assert_eq!(map.bridges()[1].hangs, [true, false]);
    // A tank on the west span; the east span falls; the tank wants to the
    // street where the east span stood, 18 m below the joint.
    let mut sim = sim_on(map, &[(P0, HEAVY_TANK, -20, AXIS_Y)], 4);
    sim.set_bridge_intact(1, false);
    sim.step(&[move_to(P0, &[0], 30, AXIS_Y)]);
    // It goes back over the west dam and round its end instead
    // (`run_until_idle` fails if it ever leaves the surface).
    run_until_idle(&mut sim, &[0], 20 * 600);
    let p = sim.unit_position(UnitId(0)).unwrap();
    assert!(
        (p - Vec2::from_meters(30, AXIS_Y)).length() < Fixed::from_num(8),
        "tank at {p:?}"
    );
}

#[test]
fn heavy_hits_bring_down_one_piece() {
    // A 120 mm mortar on the west bank shells the west approach span,
    // where a truck stands, until it falls; the arch and the east span
    // stand.
    let run = |seed: u64| {
        let map = Arc::new(river_map());
        let mut sim = sim_on(
            map,
            &[(P0, "su_pm38", -300, 80), (P0, TRUCK, -172, AXIS_Y + 4)],
            seed,
        );
        let full = sim.bridge_hp(0).unwrap();
        let arch = sim.bridge_hp(1).unwrap();
        for _ in 0..40 {
            sim.step(&[]);
        }
        let barrage = PlayerCommand::new(
            P0,
            Command::Barrage {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(-150, AXIS_Y),
                radius_m: 0,
                salvos: 60,
                smoke: false,
            },
        );
        let mut hit = false;
        for tick in 0..20 * 900 {
            // One barrage after the other.
            if tick % (20 * 300) == 0 {
                sim.step(std::slice::from_ref(&barrage));
            } else {
                sim.step(&[]);
            }
            hit |= sim.bridge_hp(0).is_some_and(|hp| hp < full);
            if !sim.map().bridges()[0].intact {
                break;
            }
        }
        assert!(hit, "the bridge took hits");
        let bridges = sim.map().bridges();
        assert!(!bridges[0].intact, "the west approach was shot down");
        assert!(bridges[1].intact && bridges[2].intact);
        assert_eq!(sim.bridge_hp(1), Some(arch), "the arch was not hit");
        assert_eq!(
            sim.unit_hp(UnitId(1)),
            Some(0),
            "the truck fell with the span"
        );
        sim.state_hash()
    };
    assert_eq!(run(9), run(9));
}

/// Path planning over the long bridge (docs/PERFORMANCE.md, "Long
/// bridges"): 100 units (half heavy tanks, half squads) on the west bank,
/// every one ordered on its own to a point on the east bank; ticks until
/// every path is there and the time per tick meanwhile. Then the same with
/// the east approach cut (targets unreachable: nearest reachable cell).
/// `cargo test --release -p sim --test long_bridges bench -- --ignored --nocapture`
#[test]
#[ignore]
// Wall-clock timing of the test itself, never seen by the simulation.
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
fn bench_paths_over_the_long_bridge() {
    for cut in [false, true] {
        let mut map = river_map();
        if cut {
            map.set_bridge_intact(2, false);
        }
        let mut spawns = Vec::new();
        for k in 0..100 {
            let unit = if k % 2 == 0 { HEAVY_TANK } else { RIFLES };
            spawns.push((P0, unit, -560 + (k % 10) * 12, -150 + (k / 10) * 30));
        }
        let mut sim = sim_on(Arc::new(map), &spawns, 1);
        let orders: Vec<PlayerCommand> = (0..100u32)
            .map(|k| {
                move_to(
                    P0,
                    &[k],
                    450 + (k as i32 % 10) * 10,
                    -150 + (k as i32 / 10) * 30,
                )
            })
            .collect();
        let mut times = Vec::new();
        let start = std::time::Instant::now();
        sim.step(&orders);
        times.push(start.elapsed());
        let mut ticks = 1;
        while sim.path_queue_len() > 0
            || (0..100).any(|k| matches!(sim.route_of(UnitId(k)), Some(RouteView::Waiting)))
        {
            let t = std::time::Instant::now();
            sim.step(&[]);
            times.push(t.elapsed());
            ticks += 1;
            assert!(ticks < 20 * 600);
        }
        let stats = sim.path_stats();
        times.sort();
        let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
        let mean = times.iter().map(|&d| ms(d)).sum::<f64>() / times.len() as f64;
        println!(
            "cut {cut}: all paths after {ticks} ticks, {stats:?}, per tick mean {mean:.2} ms, max {:.2} ms",
            ms(*times.last().unwrap())
        );
    }
}
