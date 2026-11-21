//! Positions tier 2 (prompt 052): zigzag trenches with firing positions,
//! dug-in vehicles that can only be hit on the turret, camouflage nets,
//! lines of obstacles with their cost, MG nests and bunkers only at trench
//! edges, map strongpoints, determinism.

use sim::map::{MapLayout, Marker};
use sim::path::SizeClass;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, DigKind, FireMode, Fixed, HitKind, LayKind, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, WorkKind,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const ENGINEERS: &str = "de_inf_engineer";
const RIFLES: &str = "de_inf_rifle_squad";
const ENEMY_RIFLES: &str = "su_inf_rifle_squad";
const TANK: &str = "de_pz4_h";
const ENEMY_TANK: &str = "su_t34_76_1943";

/// Flat 400 m × 400 m around the origin with `markers`.
fn flat_map(markers: &[(&str, i32, i32)]) -> Arc<Map> {
    let n = 201;
    let layout = MapLayout {
        name: "positions_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: markers
            .iter()
            .map(|&(name, x, y)| Marker {
                name: name.into(),
                position: Vec2::from_meters(x, y),
            })
            .collect(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

/// A match on `map`; `extra` are unit types without a spawn (bunkers).
fn sim_on(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)], extra: &[&str]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let mut unit_types = unit_types_from_catalog(&catalog, &scale, &names).unwrap();
    if !extra.is_empty() {
        // Fortifications come from the economy rules (base building).
        let economy = sim::EconomyRules::builtin().unwrap();
        let more = economy
            .unit_types(&catalog, &scale, &["de"], &unit_types)
            .unwrap();
        unit_types.extend(
            more.into_iter()
                .filter(|t| extra.contains(&t.name.as_str())),
        );
    }
    let config = SimConfig {
        tick_hz: 20,
        unit_types,
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
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

fn hold(
    spawns: &[(PlayerId, &'static str, i32, i32)],
) -> Vec<(PlayerId, &'static str, i32, i32, FireMode)> {
    spawns
        .iter()
        .map(|&(p, u, x, y)| (p, u, x, y, FireMode::HoldFire))
        .collect()
}

fn ids(units: &[u32]) -> Vec<UnitId> {
    units.iter().map(|&u| UnitId(u)).collect()
}

fn dig(
    player: PlayerId,
    units: &[u32],
    kind: DigKind,
    from: (i32, i32),
    to: (i32, i32),
) -> PlayerCommand {
    PlayerCommand::new(
        player,
        Command::Dig {
            units: ids(units),
            kind,
            from: Vec2::from_meters(from.0, from.1),
            to: Vec2::from_meters(to.0, to.1),
        },
    )
}

fn lay(
    player: PlayerId,
    units: &[u32],
    kind: LayKind,
    from: (i32, i32),
    to: (i32, i32),
) -> PlayerCommand {
    PlayerCommand::new(
        player,
        Command::Lay {
            units: ids(units),
            kind,
            from: Vec2::from_meters(from.0, from.1),
            to: Vec2::from_meters(to.0, to.1),
        },
    )
}

fn move_to(player: PlayerId, units: &[u32], x: i32, y: i32) -> PlayerCommand {
    PlayerCommand::new(
        player,
        Command::MoveTo {
            units: ids(units),
            target: Vec2::from_meters(x, y),
        },
    )
}

fn run_until(sim: &mut Sim, limit: u32, done: impl Fn(&Sim) -> bool) {
    for _ in 0..limit {
        if done(sim) {
            return;
        }
        sim.step(&[]);
    }
    panic!("condition not met within {limit} ticks");
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

fn all_built(sim: &Sim) -> bool {
    let w = sim.works_for(P0);
    !w.is_empty() && w.iter().all(|w| w.built)
}

#[test]
fn trenches_zigzag_and_give_firing_positions() {
    let mut sim = sim_on(
        flat_map(&[]),
        &hold(&[(P0, ENGINEERS, 0, -20), (P0, RIFLES, 30, -20)]),
        &[],
    );
    sim.step(&[dig(P0, &[0], DigKind::Trench, (-12, 0), (12, 0))]);
    run_until(&mut sim, 20 * 300, all_built);
    let works = sim.works_for(P0);
    assert_eq!(works.len(), 6);
    let off_line = works
        .iter()
        .flat_map(|w| [w.from, w.to])
        .filter(|p| p.y.abs() > Fixed::ONE)
        .count();
    assert!(off_line >= 4, "corners beside the line: {off_line}");
    assert!(sim.deformation_mm(100, 100) <= -400, "the middle is dug");
    // A squad that stops at the trench takes its firing positions.
    sim.step(&[move_to(P0, &[1], 0, 1)]);
    run(&mut sim, 20 * 40);
    let covered = sim
        .snapshot()
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(1) && s.in_cover)
        .count();
    assert!(covered >= 5, "only {covered} soldiers at firing positions");
}

#[test]
fn a_dug_in_tank_is_hit_only_on_the_turret() {
    let mut sim = sim_on(
        flat_map(&[]),
        &[
            (P0, ENGINEERS, 0, -12, FireMode::HoldFire),
            (P0, TANK, -20, -30, FireMode::HoldFire),
            (P1, ENEMY_TANK, 0, 120, FireMode::HoldFire),
        ],
        &[],
    );
    sim.step(&[dig(P0, &[0], DigKind::VehiclePit, (0, 0), (0, 0))]);
    run_until(&mut sim, 20 * 300, all_built);
    sim.step(&[move_to(P0, &[0], 0, -40), move_to(P0, &[1], 0, 0)]);
    run_until(&mut sim, 20 * 60, |s| s.dug_in().contains(&UnitId(1)));
    // Driving off leaves the pit; back in, it digs in again.
    sim.step(&[move_to(P0, &[1], -30, -30)]);
    run(&mut sim, 20 * 3);
    assert!(!sim.dug_in().contains(&UnitId(1)));
    sim.step(&[move_to(P0, &[1], 0, 0)]);
    run_until(&mut sim, 20 * 60, |s| s.dug_in().contains(&UnitId(1)));
    let full = sim.unit_hp(UnitId(1)).unwrap();
    sim.step(&[PlayerCommand::new(
        P1,
        Command::Attack {
            units: ids(&[2]),
            target: UnitId(1),
        },
    )]);
    let (mut turret, mut hull, mut parapet) = (0, 0, 0);
    for _ in 0..20 * 60 {
        sim.step(&[]);
        for e in sim.events() {
            if let CombatEvent::Hit {
                kind,
                turret: t,
                at,
                ..
            } = e
            {
                match kind {
                    HitKind::Unit(UnitId(1)) if *t => turret += 1,
                    HitKind::Unit(UnitId(1)) => hull += 1,
                    HitKind::Ground
                        if (Vec2::new(at.x, at.y) - Vec2::from_meters(0, 0)).length()
                            < Fixed::from_num(5) =>
                    {
                        parapet += 1;
                    }
                    _ => {}
                }
            }
        }
        if sim.unit_hp(UnitId(1)) == Some(0) {
            break;
        }
    }
    assert_eq!(hull, 0, "the hull of a dug-in tank was hit");
    assert!(turret > 0, "no turret hit (parapet: {parapet})");
    assert!(sim.unit_hp(UnitId(1)) < Some(full));
}

#[test]
fn camouflage_nets_hide_standing_units_from_afar() {
    let mut sim = sim_on(
        flat_map(&[]),
        &hold(&[
            (P0, ENGINEERS, 0, -8),
            (P0, RIFLES, 0, 0),
            (P1, ENEMY_RIFLES, 0, 150),
        ]),
        &[],
    );
    run(&mut sim, 20);
    assert!(sim.visible_to(P1, UnitId(1)), "seen in the open at 150 m");
    sim.step(&[lay(P0, &[0], LayKind::CamoNet, (0, 0), (0, 0))]);
    run_until(&mut sim, 20 * 120, all_built);
    run(&mut sim, 20);
    assert!(
        !sim.visible_to(P1, UnitId(1)),
        "a standing squad under a net is hidden from 150 m"
    );
    // The enemy does not see the net either.
    assert!(
        sim.works_for(P1)
            .iter()
            .all(|w| w.kind != WorkKind::CamoNet)
    );
    // Close by, the enemy sees it.
    sim.step(&[move_to(P1, &[2], 0, 50)]);
    run_until(&mut sim, 20 * 120, |s| {
        s.unit_position(UnitId(2))
            .is_some_and(|p| p.y < Fixed::from_num(60))
    });
    run(&mut sim, 20 * 2);
    assert!(sim.visible_to(P1, UnitId(1)), "spotted within spot range");
    assert!(
        sim.works_for(P1)
            .iter()
            .any(|w| w.kind == WorkKind::CamoNet)
    );
}

#[test]
fn firing_reveals_a_camouflaged_squad() {
    let mut sim = sim_on(
        flat_map(&[]),
        &[
            (P0, ENGINEERS, 0, -8, FireMode::HoldFire),
            (P0, RIFLES, 0, 0, FireMode::HoldFire),
            (P1, ENEMY_RIFLES, 0, 150, FireMode::HoldFire),
        ],
        &[],
    );
    sim.step(&[lay(P0, &[0], LayKind::CamoNet, (0, 0), (0, 0))]);
    run_until(&mut sim, 20 * 120, all_built);
    run(&mut sim, 20);
    assert!(!sim.visible_to(P1, UnitId(1)));
    // P0 fires at what it sees (ground at the enemy): no longer hidden.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::AttackGround {
            units: ids(&[1]),
            target: Vec2::from_meters(0, 140),
        },
    )]);
    run_until(&mut sim, 20 * 30, |s| s.visible_to(P1, UnitId(1)));
}

#[test]
fn obstacle_lines_cost_per_piece_and_teeth_block_tanks_for_good() {
    let mut sim = sim_on(
        flat_map(&[]),
        &hold(&[(P0, ENGINEERS, 0, -10), (P1, ENEMY_TANK, 0, 40)]),
        &[],
    );
    let before = sim.account(P0).unwrap();
    sim.step(&[lay(P0, &[0], LayKind::Teeth, (-6, 0), (6, 0))]);
    let pieces = sim.works_for(P0).len() as u32;
    assert_eq!(pieces, 9, "a tooth every 1.5 m over 12 m");
    let cost = sim.works_rules().rule(WorkKind::Teeth).cost;
    let after = sim.account(P0).unwrap();
    for r in 0..3 {
        assert_eq!(before[r] - after[r], cost[r] * pieces);
    }
    run_until(&mut sim, 20 * 400, all_built);
    let grid = sim.path_grid();
    assert!(!grid.passable_at(SizeClass::Large, Vec2::from_meters(0, 0)));
    assert!(grid.passable_at(SizeClass::Small, Vec2::from_meters(0, 0)));
    // A tank that tries to cross does not flatten them.
    sim.step(&[move_to(P1, &[1], 0, -40)]);
    run(&mut sim, 20 * 30);
    assert_eq!(sim.works_for(P0).len() as u32, pieces);
}

#[test]
fn nests_and_bunkers_only_at_a_trench_edge() {
    let mut sim = sim_on(
        flat_map(&[]),
        &hold(&[(P0, ENGINEERS, 0, -10)]),
        &["de_bunker_tobruk"],
    );
    sim.step(&[dig(P0, &[0], DigKind::MgNest, (0, 40), (0, 40))]);
    assert!(sim.works_for(P0).is_empty(), "no nest without a trench");
    sim.step(&[dig(P0, &[0], DigKind::Trench, (-8, 0), (8, 0))]);
    run_until(&mut sim, 20 * 300, all_built);
    sim.step(&[dig(P0, &[0], DigKind::MgNest, (0, 3), (0, 3))]);
    assert!(sim.works_for(P0).iter().any(|w| w.kind == WorkKind::MgNest));
    run_until(&mut sim, 20 * 300, all_built);
    let units = sim.snapshot().units.len();
    sim.step(&[lay(P0, &[0], LayKind::Bunker, (10, 6), (10, 20))]);
    run_until(&mut sim, 20 * 300, |s| s.snapshot().units.len() > units);
    let bunker = sim.snapshot().units.last().unwrap().id;
    assert!(
        sim.structures()
            .iter()
            .any(|s| s.unit == bunker && s.owner == P0 && s.built_pml >= 1000)
    );
}

#[test]
fn map_strongpoints_are_neutral_and_dug_at_the_start() {
    let mut sim = sim_on(
        flat_map(&[("strongpoint_hill", 0, 0)]),
        &hold(&[(P0, RIFLES, -100, 0), (P1, ENEMY_RIFLES, 100, 0)]),
        &[],
    );
    sim.step(&[]);
    let works = sim.works_for(P0);
    assert!(works.len() > 20, "only {} works", works.len());
    assert!(works.iter().all(|w| w.built));
    assert_eq!(works, sim.works_for(P1), "neutral: both see the same");
    assert!(works.iter().any(|w| w.kind == WorkKind::Dugout));
    assert!(works.iter().any(|w| w.kind == WorkKind::MgNest));
    // The ring is dug (16 m east of the marker), the wire blocks infantry.
    let ring = (104..=110)
        .flat_map(|i| (96..=104).map(move |j| (i, j)))
        .map(|(i, j)| sim.deformation_mm(i, j))
        .min()
        .unwrap();
    assert!(ring <= -400, "ring not dug: {ring}");
    let grid = sim.path_grid();
    let wired = (0..360)
        .step_by(10)
        .filter(|&deg| {
            let (s, c) = sim::objects::sin_cos_deg(deg);
            let p = Vec2::new(c * 30, s * 30);
            !grid.passable_at(SizeClass::Small, p)
        })
        .count();
    assert!(wired > 20, "wire ring blocks only {wired} of 36 directions");
}

#[test]
fn positions_are_deterministic() {
    let play = || {
        let mut sim = sim_on(
            flat_map(&[("strongpoint_a", 40, 40)]),
            &[
                (P0, ENGINEERS, 0, -10, FireMode::FireAtWill),
                (P0, TANK, -20, -20, FireMode::FireAtWill),
                (P1, ENEMY_TANK, 0, 150, FireMode::FireAtWill),
            ],
            &[],
        );
        sim.step(&[
            dig(P0, &[0], DigKind::VehiclePit, (0, 0), (0, 0)),
            lay(P0, &[0], LayKind::CamoNet, (0, 0), (0, 0)),
        ]);
        run(&mut sim, 20 * 90);
        sim.step(&[move_to(P0, &[1], 0, 0), move_to(P1, &[2], 0, 60)]);
        run(&mut sim, 20 * 60);
        sim.state_hash()
    };
    assert_eq!(play(), play());
}

#[test]
fn infantry_crosses_trenches_and_reaches_a_strongpoint() {
    let mut sim = sim_on(
        flat_map(&[("strongpoint_a", 0, 60)]),
        &hold(&[(P0, ENGINEERS, 0, -20), (P0, RIFLES, 30, -30)]),
        &[],
    );
    sim.step(&[dig(P0, &[0], DigKind::Trench, (-12, 0), (12, 0))]);
    run_until(&mut sim, 20 * 300, |s| {
        s.works_for(P0)
            .iter()
            .filter(|w| w.owner == P0)
            .all(|w| w.built)
    });
    // Across the trench, then into the strongpoint's center.
    sim.step(&[move_to(P0, &[1], 0, 20)]);
    run_until(&mut sim, 20 * 120, |s| {
        s.unit_position(UnitId(1))
            .is_some_and(|p| (p - Vec2::from_meters(0, 20)).length() < Fixed::from_num(6))
    });
    sim.step(&[move_to(P0, &[1], 0, 60)]);
    run_until(&mut sim, 20 * 180, |s| {
        s.unit_position(UnitId(1))
            .is_some_and(|p| (p - Vec2::from_meters(0, 60)).length() < Fixed::from_num(6))
    });
}

#[test]
fn infantry_walks_into_a_strongpoint_placed_during_the_match() {
    let mut sim = sim_on(
        flat_map(&[]),
        &hold(&[(P0, RIFLES, 0, -35), (P0, RIFLES, 12, -35)]),
        &[],
    );
    run(&mut sim, 20);
    assert!(sim.place_strongpoint(Vec2::from_meters(0, 0)) > 20);
    // Two squads: their places lie 11 m beside the center, inside the ring.
    sim.step(&[move_to(P0, &[0, 1], 0, 0)]);
    run_until(&mut sim, 20 * 120, |s| {
        [0, 1].iter().all(|&u| {
            s.unit_position(UnitId(u))
                .is_some_and(|p| p.length() < Fixed::from_num(14))
        })
    });
}

/// Flat 800 m × 800 m (room for a reconnaissance flight, like `tests/air.rs`).
fn wide_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "positions_air_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

#[test]
fn reconnaissance_does_not_reveal_a_truck_under_a_net() {
    // As in `tests/air.rs`: the tank sees 500 m, the truck 650 m south of
    // it is hidden; a reconnaissance flight over it would reveal it.
    let play = |net: bool| {
        let mut sim = sim_on(
            wide_map(),
            &hold(&[(P0, TANK, 0, 350), (P1, "su_zis5", 0, -300)]),
            &[],
        );
        if net {
            let at = Vec2::from_meters(0, -300);
            assert_eq!(sim.place_finished_work(WorkKind::CamoNet, P1, at, at), 1);
        }
        run(&mut sim, 5);
        sim.step(&[PlayerCommand::new(
            P0,
            Command::CallAir {
                mission: sim::AirMission::Recon,
                target: Vec2::from_meters(0, -250),
            },
        )]);
        let mut seen = false;
        for _ in 0..20 * 120 {
            sim.step(&[]);
            seen |= sim.visible_to(P0, UnitId(1));
        }
        seen
    };
    assert!(play(false), "without a net the flight reveals the truck");
    assert!(!play(true), "the flight revealed the truck under its net");
}
