//! Field works of engineers (prompt 035): mines go off by mass, the enemy
//! does not see them before he finds them, clearing removes them, known
//! fields are avoided, digging gives cover, wire and obstacles block and
//! wire is crushed, bridges are demolished and repaired, determinism.

use sim::map::MapLayout;
use sim::path::SizeClass;
use sim::{
    Catalog, CombatRules, Command, DigKind, FireMode, Fixed, LayKind, Map, PlayerCommand, PlayerId,
    RouteView, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const ENGINEERS: &str = "de_inf_engineer";
const ENEMY_ENGINEERS: &str = "su_inf_engineer";
const RIFLES: &str = "su_inf_rifle_squad";
const TANK: &str = "su_t34_76_1943";

/// Flat 400 m × 400 m around the origin with `objects` (CSV rows).
fn flat_map(objects: &str) -> Arc<Map> {
    let n = 201;
    let layout = MapLayout {
        name: "works_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!("kind,variant,x_mm,y_mm,heading_deg\n{objects}"))
            .unwrap();
    }
    Arc::new(map)
}

fn sim_on(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32)], seed: u64) -> Sim {
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
    Sim::new(&config, map, seed).unwrap()
}

fn sim_with(spawns: &[(PlayerId, &str, i32, i32)]) -> Sim {
    sim_on(flat_map(""), spawns, 7)
}

fn ids(units: &[u32]) -> Vec<UnitId> {
    units.iter().map(|&u| UnitId(u)).collect()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn lay(
    player: PlayerId,
    units: &[u32],
    kind: LayKind,
    from: (i32, i32),
    to: (i32, i32),
) -> PlayerCommand {
    cmd(
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
    cmd(
        player,
        Command::MoveTo {
            units: ids(units),
            target: Vec2::from_meters(x, y),
        },
    )
}

fn run_until(sim: &mut Sim, limit: u32, done: impl Fn(&Sim) -> bool) -> u32 {
    for tick in 0..limit {
        if done(sim) {
            return tick;
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

fn position(sim: &Sim, id: u32) -> Vec2 {
    sim.unit_position(UnitId(id)).unwrap()
}

fn hp(sim: &Sim, id: u32) -> u32 {
    sim.unit_hp(UnitId(id)).unwrap()
}

/// A neutral anti-tank section from the map editor at the origin.
const AT_FIELD: &str = "fort_mine_at,Mine,0,0,0\n";

#[test]
fn map_minefields_are_neutral_hidden_sections() {
    let sim = sim_on(flat_map(AT_FIELD), &[(P0, RIFLES, -60, 0)], 1);
    let mines = sim.all_mines();
    assert_eq!(mines.len(), 20);
    assert!(mines.iter().all(|m| m.owner.is_none()));
    assert!(
        mines
            .iter()
            .all(|m| m.position.x.abs() <= Fixed::from_num(12)
                && m.position.y.abs() <= Fixed::from_num(12))
    );
    assert!(sim.mines_for(P0).is_empty(), "nobody knows map mines");
}

#[test]
fn anti_tank_mines_ignore_infantry_and_stop_tanks() {
    let mut sim = sim_on(
        flat_map(AT_FIELD),
        &[(P1, RIFLES, -60, 0), (P1, TANK, -80, 3)],
        1,
    );
    // The squad walks through the field: too light.
    sim.step(&[move_to(P1, &[0], 60, 0)]);
    run_until(&mut sim, 20 * 120, |s| {
        position(s, 0).x > Fixed::from_num(40)
    });
    assert_eq!(
        sim.all_mines().len(),
        20,
        "infantry set off an anti-tank mine"
    );
    let full = hp(&sim, 1);
    // The tank drives through it: a mine goes off, the track is gone.
    sim.step(&[move_to(P1, &[1], 50, 3)]);
    run_until(&mut sim, 20 * 120, |s| !s.tracks_off().is_empty());
    assert!(sim.all_mines().len() < 20);
    assert!(hp(&sim, 1) < full, "the tank took damage");
    let stuck = position(&sim, 1);
    // It cannot drive until the crew fitted the track (90 s).
    sim.step(&[move_to(P1, &[1], 60, 60)]);
    run(&mut sim, 20 * 10);
    assert!((position(&sim, 1) - stuck).length() < Fixed::from_num(1));
    // The explosion showed the field to its player.
    assert!(!sim.mines_for(P1).is_empty());
    run_until(&mut sim, 20 * 100, |s| s.tracks_off().is_empty());
    sim.step(&[move_to(P1, &[1], -60, 40)]);
    run(&mut sim, 20 * 5);
    assert!(
        (position(&sim, 1) - stuck).length() > Fixed::from_num(2),
        "fixed track"
    );
}

#[test]
fn laid_mines_are_hidden_from_the_enemy_until_found() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -30), (P1, ENEMY_ENGINEERS, 0, 120)]);
    let ammo = sim.account(P0).unwrap()[1];
    sim.step(&[lay(P0, &[0], LayKind::MinesAp, (0, 0), (0, 0))]);
    assert!(sim.account(P0).unwrap()[1] < ammo, "the section was paid");
    assert_eq!(sim.engineer_job(UnitId(0)), Some("build"));
    run_until(&mut sim, 20 * 60, |s| !s.all_mines().is_empty());
    // German AP section: 40 mines (catalog).
    assert_eq!(sim.mines_for(P0).len(), 40);
    assert!(sim.mines_for(P0).iter().all(|m| m.friendly));
    assert!(sim.mines_for(P1).is_empty(), "the enemy sees laid mines");
    // Own engineers walk through their own field unharmed.
    sim.step(&[move_to(P0, &[0], 0, 30)]);
    run(&mut sim, 20 * 30);
    assert_eq!(sim.all_mines().len(), 40);
    // Enemy engineers find them when they come close.
    sim.step(&[move_to(P1, &[1], 0, 18)]);
    run_until(&mut sim, 20 * 90, |s| !s.mines_for(P1).is_empty());
    assert!(sim.mines_for(P1).iter().all(|m| !m.friendly));
}

#[test]
fn clearing_removes_known_mines() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -30), (P1, ENEMY_ENGINEERS, 0, 40)]);
    sim.step(&[lay(P0, &[0], LayKind::MinesAt, (0, 0), (0, 0))]);
    run_until(&mut sim, 20 * 80, |s| !s.all_mines().is_empty());
    let laid = sim.all_mines().len();
    assert_eq!(laid, 20);
    sim.step(&[move_to(P0, &[0], 0, -80)]);
    sim.step(&[cmd(
        P1,
        Command::Clear {
            units: ids(&[1]),
            target: Vec2::from_meters(0, 0),
        },
    )]);
    run_until(&mut sim, 20 * 200, |s| s.all_mines().is_empty());
    run_until(&mut sim, 20, |s| s.engineer_job(UnitId(1)).is_none());
}

#[test]
fn known_minefields_are_avoided() {
    let mut sim = sim_on(
        flat_map(AT_FIELD),
        &[(P1, ENEMY_ENGINEERS, -30, 30), (P1, TANK, -80, 0)],
        3,
    );
    // Whether the tank's route passes within 2 m of one of `mines`.
    let crosses = |s: &Sim, mines: &[Vec2]| match s.route_of(UnitId(1)) {
        Some(RouteView::Following(points)) => {
            let mut from = position(s, 1);
            let mut hit = false;
            for &to in points {
                hit |= (0..=40).any(|k| {
                    let p = from + (to - from).scale(Fixed::from_num(k) / 40);
                    mines.iter().any(|&m| (m - p).length() < Fixed::from_num(2))
                });
                from = to;
            }
            hit
        }
        _ => panic!("no route"),
    };
    let following = |s: &Sim| matches!(s.route_of(UnitId(1)), Some(RouteView::Following(_)));
    // Without knowledge the tank plans straight through.
    sim.step(&[move_to(P1, &[1], 80, 0)]);
    run_until(&mut sim, 200, following);
    let all: Vec<Vec2> = sim.all_mines().iter().map(|m| m.position).collect();
    assert!(crosses(&sim, &all));
    sim.step(&[cmd(P1, Command::Stop { units: ids(&[1]) })]);
    // The engineers walk into the field (too light for anti-tank mines)
    // and find most of it; the next route goes round what they found.
    sim.step(&[move_to(P1, &[0], 0, 0)]);
    run_until(&mut sim, 20 * 60, |s| s.mines_for(P1).len() >= 15);
    sim.step(&[move_to(P1, &[0], -40, 40)]);
    run(&mut sim, 20 * 20);
    let known: Vec<Vec2> = sim.mines_for(P1).iter().map(|m| m.position).collect();
    sim.step(&[move_to(P1, &[1], 80, 0)]);
    run_until(&mut sim, 400, following);
    assert!(
        !crosses(&sim, &known),
        "the route crosses a known minefield"
    );
}

#[test]
fn digging_lowers_the_ground_and_gives_cover() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -20), (P0, "de_inf_rifle_squad", 30, -20)]);
    sim.step(&[cmd(
        P0,
        Command::Dig {
            units: ids(&[0]),
            kind: DigKind::Trench,
            from: Vec2::from_meters(-12, 0),
            to: Vec2::from_meters(12, 0),
        },
    )]);
    run_until(&mut sim, 20 * 200, |s| {
        let w = s.works_for(P0);
        !w.is_empty() && w.iter().all(|w| w.built)
    });
    // Sample (100, 100) is the origin on this map.
    assert!(sim.deformation_mm(100, 100) <= -400);
    assert!(sim.deformation_mm(100, 101) > 0, "a parapet beside it");
    // A squad that stands at the trench takes its spots.
    sim.step(&[move_to(P0, &[1], 0, 2)]);
    run(&mut sim, 20 * 40);
    let covered = sim
        .snapshot()
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(1) && s.in_cover)
        .count();
    assert!(covered >= 3, "only {covered} soldiers in cover");
    // The enemy sees the finished trench too.
    assert_eq!(sim.works_for(P1).len(), sim.works_for(P0).len());
}

#[test]
fn wire_blocks_infantry_and_tanks_crush_it() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -20), (P1, TANK, 0, 60)]);
    sim.step(&[lay(P0, &[0], LayKind::Wire, (-8, 0), (8, 0))]);
    run_until(&mut sim, 20 * 60, |s| {
        s.works_for(P0).iter().all(|w| w.built)
    });
    let pieces = sim.works_for(P0).len();
    assert_eq!(pieces, 4);
    let grid = sim.path_grid();
    let (x, y) = grid.cell_of(Vec2::from_meters(0, 0));
    assert!(!grid.passable(SizeClass::Small, x, y));
    assert!(grid.passable(SizeClass::Large, x, y));
    // The tank drives over the middle of the wire and flattens it.
    sim.step(&[move_to(P1, &[1], 0, -60)]);
    run_until(&mut sim, 20 * 60, |s| s.works_for(P0).len() < pieces);
    let grid = sim.path_grid();
    let free = (-8..=8)
        .step_by(4)
        .any(|x| grid.passable_at(SizeClass::Small, Vec2::from_meters(x, 0)));
    assert!(free, "a crushed piece frees its cells");
}

#[test]
fn hedgehogs_block_tanks() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -20)]);
    sim.step(&[lay(P0, &[0], LayKind::Hedgehogs, (-12, 0), (12, 0))]);
    run_until(&mut sim, 20 * 200, |s| {
        s.works_for(P0).iter().all(|w| w.built)
    });
    let grid = sim.path_grid();
    let p = Vec2::from_meters(0, 0);
    assert!(!grid.passable_at(SizeClass::Large, p));
    assert!(grid.passable_at(SizeClass::Small, p));
}

/// 200 m x 120 m, a river at |x| <= 5 m and a timber bridge across it at
/// y = 1 m (like `tests/bridges.rs`).
fn river_map() -> Arc<Map> {
    let (sx, sy) = (101u32, 61u32);
    let mut heights = Vec::new();
    let mut terrain = Vec::new();
    for _ in 0..sy {
        for i in 0..sx {
            let x = 2 * i as i32 - 100;
            let river = x.abs() <= 5;
            heights.push(if river { -2000 } else { 0 });
            terrain.push(if river { 7 } else { 0 });
        }
    }
    let layout = MapLayout {
        name: "works_river".into(),
        samples_x: sx,
        samples_y: sy,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let mut map = Map::from_rasters(layout, heights, terrain).unwrap();
    map.add_objects("kind,variant,x_mm,y_mm,heading_deg\nenv_bridge_wood,Span16,0,1000,90\n")
        .unwrap();
    Arc::new(map)
}

#[test]
fn bridges_are_demolished_and_repaired() {
    let mut sim = sim_on(river_map(), &[(P0, ENGINEERS, -30, 1)], 5);
    let deck = Vec2::from_meters(0, 1);
    assert!(sim.path_grid().passable_at(SizeClass::Small, deck));
    sim.step(&[cmd(
        P0,
        Command::DemolishBridge {
            units: ids(&[0]),
            bridge: 0,
        },
    )]);
    run_until(&mut sim, 20 * 120, |s| !s.map().bridges()[0].intact);
    assert!(!sim.path_grid().passable_at(SizeClass::Small, deck));
    let fuel = sim.account(P0).unwrap()[0];
    sim.step(&[cmd(
        P0,
        Command::RepairBridge {
            units: ids(&[0]),
            bridge: 0,
        },
    )]);
    run_until(&mut sim, 20 * 200, |s| s.map().bridges()[0].intact);
    assert!(sim.path_grid().passable_at(SizeClass::Small, deck));
    assert!(sim.account(P0).unwrap()[0] < fuel, "the repair was paid");
}

#[test]
fn other_orders_end_the_job_and_idle_sites_are_refunded() {
    let mut sim = sim_with(&[(P0, ENGINEERS, 0, -60)]);
    let before = sim.account(P0).unwrap();
    sim.step(&[lay(P0, &[0], LayKind::MinesAt, (0, 0), (0, 0))]);
    assert_ne!(sim.account(P0).unwrap(), before);
    sim.step(&[move_to(P0, &[0], 0, -80)]);
    assert_eq!(sim.engineer_job(UnitId(0)), None);
    run(&mut sim, 20 * 31);
    assert!(sim.works_for(P0).is_empty(), "the idle site is given up");
    assert_eq!(sim.account(P0).unwrap(), before, "and refunded");
}

#[test]
fn only_engineers_take_engineer_orders() {
    let mut sim = sim_with(&[(P0, "de_inf_rifle_squad", 0, -20)]);
    sim.step(&[lay(P0, &[0], LayKind::Wire, (-8, 0), (8, 0))]);
    assert!(sim.works_for(P0).is_empty());
}

#[test]
fn works_are_deterministic() {
    let play = || {
        let mut sim = sim_on(
            flat_map(AT_FIELD),
            &[
                (P0, ENGINEERS, 0, -40),
                (P1, TANK, -60, 2),
                (P1, RIFLES, -60, 30),
            ],
            11,
        );
        sim.step(&[
            lay(P0, &[0], LayKind::MinesAp, (-20, 30), (20, 30)),
            move_to(P1, &[1], 60, 2),
            move_to(P1, &[2], 60, 30),
        ]);
        run(&mut sim, 20 * 90);
        (sim.state_hash(), sim.all_mines().len())
    };
    let (a, mines) = play();
    assert_eq!(a, play().0);
    assert!(mines > 0);
}
