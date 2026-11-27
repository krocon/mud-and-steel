//! Tunnel rules and behavior (prompt 148): driving through mountains at sole
//! elevation, visibility isolation, protection from indirect fire & bombs,
//! direct fire along the bore, single-lane oncoming resolution, wreck blocking
//! for vehicles while infantry passes, clearance gauge rejection, engineer
//! portal demolition & clearing, determinism and snapshot roundtrip.

use sim::combat::{CombatEvent, HitKind};
use sim::map::MapLayout;
use sim::path::{PathGrid, SizeClass};
use sim::snapshot::MapDelta;
use sim::{
    Catalog, CombatRules, Command, FireMode, Fixed, Map, PlayerCommand, PlayerId, Scale, Sim,
    SimConfig, Snapshot, Spawn, TerrainRules, TunnelEntry, TunnelKind, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

/// Builds a 400 × 300 m mountain-ridge test map:
/// - Flat valley on west (`x <= -60`) and east (`x >= 60`) at `z = 0 m`.
/// - Sharp knife-edge rock ridge across `x in [-54, 54]` rising to `z = 44 m`
///   (`~82%` slope on both flanks -> impassable even at the crest!) except a
///   northern mountain pass road around `y = -110` (`8%` gentle ramps).
/// - Road tunnel straight through the ridge at `y = 0` from `x = -56` to `x = 56`,
///   with configurable clearance gauge.
fn ridge_map(clearance_height_mm: Option<u32>, clearance_width_mm: Option<u32>) -> Map {
    let sx = 201u32; // 400 m at 2 m cell size
    let sy = 151u32; // 300 m at 2 m cell size
    let mut heights = vec![0i32; (sx * sy) as usize];
    let mut terrain = vec![0u8; (sx * sy) as usize]; // grass
    for j in 0..sy {
        let y = -150 + i64::from(j) * 2;
        for i in 0..sx {
            let x = -200 + i64::from(i) * 2;
            let k = (j * sx + i) as usize;
            let pass_dist = (y + 110).abs();
            let ax = x.abs();
            // Triangular knife-edge profile across x in [-54, 54]: 44 m / 54 m = 81.5% slope everywhere!
            let ridge_h = if ax <= 54 { 44_000 * (54 - ax) / 54 } else { 0 };
            let pass_h = if ax <= 100 {
                8_000 * (100 - ax) / 100
            } else {
                0
            };
            let h = if pass_dist <= 16 {
                pass_h
            } else if pass_dist >= 30 {
                ridge_h
            } else {
                pass_h + (ridge_h - pass_h) * (pass_dist - 16) / 14
            };
            heights[k] = h as i32;
            if pass_dist <= 5 || (y.abs() <= 5 && ax >= 54) {
                terrain[k] = 2; // road
            }
        }
    }
    let layout = MapLayout {
        name: "ridge_test".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, terrain).expect("map");
    map.set_tunnels(vec![TunnelEntry {
        kind: TunnelKind::Road,
        points_m: vec![[-56, 0], [56, 0]],
        width_mm: Some(6000),
        clearance_height_mm,
        clearance_width_mm,
        portal_heights_mm: Some([0, 0]),
    }])
    .expect("set_tunnels");
    map
}

fn sim_on(map: Arc<Map>, spawns: &[(u8, &str, i32, i32, FireMode)], seed: u64) -> Sim {
    let catalog = Catalog::builtin().expect("catalog");
    let scale = Scale::builtin().expect("scale");
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).expect("types"),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner: PlayerId(owner),
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: map.reference(),
        terrain: TerrainRules::builtin().expect("terrain"),
        combat: CombatRules::builtin().expect("combat"),
        vision: sim::VisionRules::builtin().expect("vision"),
        infantry: sim::InfantryRules::builtin().expect("infantry"),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).expect("sim")
}

fn cmd(player: u8, command: Command) -> PlayerCommand {
    PlayerCommand::new(PlayerId(player), command)
}

#[test]
fn tank_and_infantry_drive_through_tunnel_at_sole_elevation() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (0, "de_pz4_h", -90, 0, FireMode::HoldFire),
            (0, "de_inf_rifle_squad", -110, 0, FireMode::HoldFire),
        ],
        7,
    );
    let tank = UnitId(0);
    let squad = UnitId(1);
    sim.step(&[cmd(
        0,
        Command::MoveTo {
            units: vec![tank, squad],
            target: Vec2::from_meters(95, 0),
        },
    )]);
    let mut tank_in_tunnel = false;
    let mut max_tank_z = Fixed::from_num(-100);
    for _ in 0..800 {
        sim.step(&[]);
        let snap = sim.snapshot();
        let tu = snap.units.iter().find(|u| u.id == tank).unwrap();
        if sim.unit_in_tunnel(tank).is_some() {
            tank_in_tunnel = true;
            max_tank_z = max_tank_z.max(tu.height);
        }
    }
    let snap = sim.snapshot();
    let tu = snap.units.iter().find(|u| u.id == tank).unwrap();
    let su = snap.units.iter().find(|u| u.id == squad).unwrap();
    assert!(tank_in_tunnel, "tank should pass through the tunnel bore");
    assert!(
        max_tank_z < Fixed::from_num(2),
        "tank inside tunnel must stay on sole height (~0 m), not mountain top (44 m): got {max_tank_z}"
    );
    assert!(
        tu.position.x > Fixed::from_num(70),
        "tank should reach east valley through tunnel, got x={}",
        tu.position.x
    );
    assert!(
        su.position.x > Fixed::from_num(60),
        "infantry should reach east valley through tunnel, got x={}",
        su.position.x
    );
}

#[test]
fn tunnel_hides_units_from_outside_and_blinds_inside() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            // Unit 0 deep inside tunnel center (x=0, y=0; 56 m from portals).
            (0, "de_pz4_h", 0, 0, FireMode::HoldFire),
            // Unit 1 outside east valley off-axis (x=90, y=40): cannot see into portal.
            (1, "de_pz4_h", 90, 40, FireMode::HoldFire),
            // Unit 2 inside west side of tunnel (x=-40, y=0; 16 m from portal 0).
            (0, "de_pz4_h", -40, 0, FireMode::HoldFire),
            // Unit 3 of player 1 right outside west portal on axis (x=-72, y=0; 32 m from unit 2).
            (1, "de_pz4_h", -72, 0, FireMode::HoldFire),
        ],
        11,
    );
    sim.step(&[]);
    assert!(
        !sim.visible_to(PlayerId(1), UnitId(0)),
        "unit deep inside tunnel must be invisible to enemy outside"
    );
    assert!(
        !sim.visible_to(PlayerId(0), UnitId(1)),
        "units inside tunnel must be blind to off-axis enemies outside"
    );
    assert!(
        sim.visible_to(PlayerId(1), UnitId(2)),
        "observer aligned outside open portal should see unit near portal mouth"
    );
    assert!(
        sim.visible_to(PlayerId(0), UnitId(3)),
        "unit near open portal should see enemy directly outside the portal mouth"
    );
}

#[test]
fn artillery_above_mountain_does_not_harm_units_in_tunnel() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (0, "de_inf_rifle_squad", 0, 0, FireMode::HoldFire),
            (1, "de_lefh18", 160, 0, FireMode::HoldFire),
        ],
        19,
    );
    let gun = UnitId(1);
    let hp_before = sim.snapshot().units[0].hp;
    sim.step(&[cmd(
        1,
        Command::Barrage {
            units: vec![gun],
            target: Vec2::from_meters(0, 0),
            radius_m: 10,
            salvos: 6,
            smoke: false,
        },
    )]);
    let mut shell_impacts = 0u32;
    for _ in 0..900 {
        sim.step(&[]);
        for ev in &sim.snapshot().events {
            if let CombatEvent::Hit {
                kind: HitKind::Ground,
                at,
                ..
            } = ev
                && at.ground().length() < Fixed::from_num(35)
            {
                shell_impacts += 1;
                assert!(
                    at.z > Fixed::from_num(15),
                    "artillery shell above tunnel should detonate on mountain roof, got z={}",
                    at.z
                );
            }
        }
    }
    assert!(
        shell_impacts >= 2,
        "expected artillery shells to land on mountain above tunnel, got {shell_impacts}"
    );
    let hp_after = sim.snapshot().units[0].hp;
    assert_eq!(
        hp_after, hp_before,
        "infantry inside tunnel must take ZERO damage from artillery hitting the mountain above"
    );
}

#[test]
fn direct_fire_along_bore_hits_target_in_tunnel() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (0, "de_pz4_h", -30, 0, FireMode::FireAtWill),
            (1, "de_pz4_h", 25, 0, FireMode::FireAtWill),
        ],
        23,
    );
    let hp0 = sim.snapshot().units[0].hp;
    let hp1 = sim.snapshot().units[1].hp;
    sim.step(&[
        cmd(
            0,
            Command::Attack {
                units: vec![UnitId(0)],
                target: UnitId(1),
            },
        ),
        cmd(
            1,
            Command::Attack {
                units: vec![UnitId(1)],
                target: UnitId(0),
            },
        ),
    ]);
    for _ in 0..300 {
        sim.step(&[]);
    }
    let snap = sim.snapshot();
    assert!(
        snap.units[0].hp < hp0 || snap.units[1].hp < hp1,
        "tanks facing each other inside the same tunnel bore must hit each other"
    );
}

#[test]
fn oncoming_vehicles_in_single_lane_tunnel_resolve_without_deadlock() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (0, "de_pz4_h", -85, 0, FireMode::HoldFire),
            (0, "de_pz4_h", 85, 0, FireMode::HoldFire),
        ],
        29,
    );
    sim.step(&[
        cmd(
            0,
            Command::MoveTo {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(95, 0),
            },
        ),
        cmd(
            0,
            Command::MoveTo {
                units: vec![UnitId(1)],
                target: Vec2::from_meters(-95, 0),
            },
        ),
    ]);
    for _ in 0..1600 {
        sim.step(&[]);
    }
    let snap = sim.snapshot();
    let u0 = snap.units.iter().find(|u| u.id == UnitId(0)).unwrap();
    let u1 = snap.units.iter().find(|u| u.id == UnitId(1)).unwrap();
    assert!(
        u0.position.x > Fixed::from_num(70) && u1.position.x < Fixed::from_num(-70),
        "both oncoming tanks must cross through the tunnel without deadlocking: u0.x={}, u1.x={}",
        u0.position.x,
        u1.position.x
    );
}

#[test]
fn wreck_in_tunnel_blocks_vehicles_while_infantry_passes() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (1, "de_pz4_h", 0, 0, FireMode::HoldFire),
            (0, "de_pz4_h", -20, 0, FireMode::FireAtWill),
            (0, "de_inf_rifle_squad", -75, 0, FireMode::HoldFire),
        ],
        31,
    );
    sim.step(&[cmd(
        0,
        Command::Attack {
            units: vec![UnitId(1)],
            target: UnitId(0),
        },
    )]);
    for _ in 0..400 {
        sim.step(&[]);
        if !sim.snapshot().units[0].alive() {
            break;
        }
    }
    assert!(
        !sim.snapshot().units[0].alive(),
        "victim tank inside tunnel should be destroyed to form a wreck"
    );
    let grid = sim.path_grid();
    let wreck_pos = sim.snapshot().units[0].position;
    assert!(
        !grid.passable_at(SizeClass::Large, wreck_pos),
        "wreck inside tunnel must block Large vehicles"
    );
    assert!(
        grid.passable_at(SizeClass::Small, wreck_pos),
        "infantry (Small) must still pass a vehicle wreck inside a tunnel"
    );
}

#[test]
fn oversized_vehicle_is_rejected_by_clearance_gauge_and_routes_over_pass() {
    let map = ridge_map(Some(2200), Some(2500));
    let catalog = Catalog::builtin().expect("catalog");
    let scale = Scale::builtin().expect("scale");
    let terrain = TerrainRules::builtin().expect("terrain");
    let types = unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .expect("types");
    let grid = PathGrid::new(&map, &terrain, &types);
    let start = Vec2::from_meters(-90, 0);
    let goal = Vec2::from_meters(90, 0);
    let tank_path = grid.find_path(start, goal, 0).expect("tank path over pass");
    let inf_path = grid
        .find_path(start, goal, 1)
        .expect("infantry path through tunnel");
    let tank_min_y = tank_path
        .waypoints
        .iter()
        .map(|p| p.y)
        .fold(Fixed::MAX, Fixed::min);
    let inf_min_y = inf_path
        .waypoints
        .iter()
        .map(|p| p.y)
        .fold(Fixed::MAX, Fixed::min);
    assert!(
        tank_min_y < Fixed::from_num(-80),
        "oversized tank must route over the northern mountain pass, got min_y={tank_min_y}"
    );
    assert!(
        inf_min_y > Fixed::from_num(-20),
        "infantry should still take the direct tunnel, got min_y={inf_min_y}"
    );
}

#[test]
fn engineer_demolish_and_clear_tunnel_portals_and_snapshot_roundtrip() {
    let map = Arc::new(ridge_map(None, None));
    let mut sim = sim_on(
        Arc::clone(&map),
        &[
            (0, "de_inf_engineer", -70, 0, FireMode::HoldFire),
            (1, "de_inf_rifle_squad", -56, 0, FireMode::HoldFire),
            (0, "de_inf_engineer", 0, 0, FireMode::HoldFire),
        ],
        37,
    );
    let eng_out = UnitId(0);
    let eng_in = UnitId(2);
    let victim_hp_before = sim.snapshot().units[1].hp;

    // 1. Demolish portal 0 from outside.
    sim.step(&[cmd(
        0,
        Command::DemolishTunnel {
            units: vec![eng_out],
            tunnel: 0,
            portal: 0,
        },
    )]);
    for _ in 0..800 {
        sim.step(&[]);
        if !sim.map().tunnels()[0].portal_open(0) {
            break;
        }
    }
    assert!(
        !sim.map().tunnels()[0].portal_open(0),
        "engineers must collapse portal 0"
    );
    assert!(
        sim.snapshot().units[1].hp < victim_hp_before,
        "unit standing in portal mouth during demolition must take blast damage"
    );
    assert!(
        !sim.path_grid()
            .passable_at(SizeClass::Small, Vec2::from_meters(-56, 0)),
        "collapsed portal must block even infantry"
    );

    // 2. Collapse portal 1 too -> engineers inside are trapped until they clear a portal!
    sim.set_tunnel_portal_open(0, 1, false);
    assert!(
        !sim.path_grid()
            .reachable(1, Vec2::from_meters(0, 0), Vec2::from_meters(-90, 0)),
        "when both portals are collapsed, units inside the tunnel are trapped"
    );

    // MapDelta roundtrip preserves collapsed portals cleanly.
    let delta = MapDelta::between(&map, sim.map());
    assert_eq!(delta.tunnel_portals, vec![[false, false]]);
    let restored_map = delta.apply(Arc::clone(&map)).expect("apply map delta");
    assert_eq!(restored_map.tunnels()[0].portals_open, [false, false]);

    // 3. Trapped engineers inside the tunnel clear portal 0 from the inside!
    sim.step(&[cmd(
        0,
        Command::ClearTunnel {
            units: vec![eng_in],
            tunnel: 0,
            portal: 0,
        },
    )]);
    for _ in 0..3200 {
        sim.step(&[]);
        if sim.map().tunnels()[0].portal_open(0) {
            break;
        }
    }
    assert!(
        sim.map().tunnels()[0].portal_open(0),
        "trapped engineers inside tunnel must be able to clear collapsed portal 0"
    );
    assert!(
        sim.path_grid()
            .reachable(1, Vec2::from_meters(0, 0), Vec2::from_meters(-90, 0)),
        "once portal 0 is cleared, tunnel is reachable again"
    );
}

#[test]
fn tunnel_simulation_is_deterministic() {
    let run = || -> (u64, Snapshot) {
        let map = Arc::new(ridge_map(None, None));
        let mut sim = sim_on(
            Arc::clone(&map),
            &[
                (0, "de_pz4_h", -85, 0, FireMode::FireAtWill),
                (1, "de_pz4_h", 85, 0, FireMode::FireAtWill),
            ],
            41,
        );
        sim.step(&[
            cmd(
                0,
                Command::MoveTo {
                    units: vec![UnitId(0)],
                    target: Vec2::from_meters(85, 0),
                },
            ),
            cmd(
                1,
                Command::MoveTo {
                    units: vec![UnitId(1)],
                    target: Vec2::from_meters(-85, 0),
                },
            ),
        ]);
        for _ in 0..400 {
            sim.step(&[]);
        }
        (sim.state_hash(), sim.snapshot())
    };
    let (h1, _) = run();
    let (h2, _) = run();
    assert_eq!(h1, h2, "identical tunnel match must produce identical hash");
}
