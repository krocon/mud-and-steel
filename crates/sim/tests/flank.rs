//! Reaction to fire from the flank (prompt 110): standing vehicles and guns
//! remember where rounds came from and turn there (`[flank]` in
//! data/tactics.toml).

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, Engage, FireMode, Fixed, Formation, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TacticsRules, TerrainRules, UnitId,
    Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;

/// Flat 800 m × 800 m around the origin.
fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "flank_test".into(),
        samples_x: n as u32,
        samples_y: n as u32,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    Arc::new(
        Map::from_rasters(layout, vec![0; (n * n) as usize], vec![0; (n * n) as usize]).unwrap(),
    )
}

/// No dispersion, no penetration spread, and hit points nobody shoots
/// through in these tests: the target lives while it turns.
fn tough_rules() -> CombatRules {
    let mut rules = CombatRules::builtin().unwrap();
    rules.dispersion.mrad_x10 = 0;
    rules.penetration.spread_pct = 0;
    rules.damage.vehicle_hp = 1_000_000;
    rules
}

/// Units start facing the map center; a unit at the origin faces north
/// (-y), so west (-x) is its left.
fn sim_with(spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let map = flat_map();
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
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
        combat: tough_rules(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

fn unit(sim: &Sim, id: u32) -> sim::UnitSnapshot {
    sim.snapshot()
        .units
        .into_iter()
        .find(|u| u.id == UnitId(id))
        .expect("unit exists")
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

const WEST: Vec2 = Vec2 {
    x: Fixed::from_bits(-(1 << 32)),
    y: Fixed::ZERO,
};
const NORTH: Vec2 = Vec2 {
    x: Fixed::ZERO,
    y: Fixed::from_bits(-(1 << 32)),
};

/// Cosine of about 8°: "points there".
fn close() -> Fixed {
    Fixed::from_num(99) / 100
}

/// Ticks of `ms` real milliseconds (time factor 2, 20 Hz).
fn ticks(ms: u32) -> u64 {
    u64::from(ms) * 20 / 2000
}

/// Steps the simulation; returns the first tick a round of `shooter` hit
/// `target` and every shot `target` fired.
fn run(sim: &mut Sim, until: u64, target: UnitId) -> (Option<u64>, Vec<u64>) {
    let mut first_hit = None;
    let mut shots = Vec::new();
    while sim.tick() < until {
        let tick = sim.tick();
        sim.step(&[]);
        for e in sim.events() {
            match *e {
                CombatEvent::Hit {
                    kind: sim::HitKind::Unit(id),
                    ..
                } if id == target && first_hit.is_none() => first_hit = Some(tick),
                CombatEvent::Shot { unit, .. } if unit == target => shots.push(tick),
                _ => {}
            }
        }
    }
    (first_hit, shots)
}

#[test]
fn hit_from_the_left_turns_a_casemate_left_after_the_reaction_time() {
    // A StuG holding its fire faces north; a T-34 75 m west fires into its
    // left flank.
    let mut sim = sim_with(&[
        (P0, "de_stug3_g", 0, 0, HOLD),
        (P1, "su_t34_76_1943", -75, 0, FIRE),
    ]);
    let stug = UnitId(0);
    let rules = TacticsRules::builtin().unwrap().flank;
    let (first, _) = run(&mut sim, 5, stug);
    let first = first.expect("the T-34 hits at once");
    assert!(dot(unit(&sim, 0).facing, NORTH) > close());
    // Within the reaction time the hull stays.
    let react = ticks(rules.react_ms);
    run(&mut sim, first + react, stug);
    assert!(dot(unit(&sim, 0).facing, NORTH) > close());
    assert_eq!(
        sim.alert_direction(stug).map(|d| dot(d, WEST) > close()),
        Some(true)
    );
    // Then it turns its hull at 30°/s real (60°/s game) until the target
    // is within the gun's traverse (20°) and lays the gun on it; heavy
    // rounds (76 mm) keep the crew down longer (`shaken_ms` per round).
    let (_, shots) = run(&mut sim, first + ticks(rules.shaken_max_ms) + 60, stug);
    let stug_now = unit(&sim, 0);
    assert!(dot(stug_now.aim, WEST) > close(), "aim {:?}", stug_now.aim);
    assert!(dot(stug_now.facing, WEST) > Fixed::from_num(98) / 100);
    // Hold fire: it turned, but it did not fire.
    assert!(shots.is_empty());
}

#[test]
fn a_turret_turns_but_the_hull_stays() {
    let mut sim = sim_with(&[
        (P0, "su_t34_76_1943", 0, 0, HOLD),
        (P1, "de_pz4_h", -75, 0, FIRE),
    ]);
    run(&mut sim, 400, UnitId(0));
    let t34 = unit(&sim, 0);
    assert!(dot(t34.facing, NORTH) > close());
    assert!(dot(t34.aim, WEST) > close(), "aim {:?}", t34.aim);
}

#[test]
fn an_ordered_facing_holds_and_then_gives_way() {
    let mut sim = sim_with(&[
        (P0, "de_stug3_g", 0, 0, HOLD),
        (P1, "su_t34_76_1943", -75, 0, FIRE),
    ]);
    let rules = TacticsRules::builtin().unwrap().flank;
    // Face north (as it already does): the order holds `ordered_hold_ms`.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::FormationMove {
            units: vec![UnitId(0)],
            target: Vec2::ZERO,
            formation: Formation::Line,
            facing: Some(NORTH),
            engage: Engage::Never,
        },
    )]);
    let hold = ticks(rules.ordered_hold_ms);
    run(&mut sim, hold - 20, UnitId(0));
    assert!(dot(unit(&sim, 0).facing, NORTH) > close());
    assert!(sim.alert_direction(UnitId(0)).is_some());
    // After the hold the remembered fire wins.
    run(&mut sim, hold + 300, UnitId(0));
    assert!(dot(unit(&sim, 0).aim, WEST) > close());
}

#[test]
fn an_attack_order_wins_over_the_fire_from_the_flank() {
    // The StuG attacks a truck in front of it while a T-34 shoots at it
    // from the west.
    let mut sim = sim_with(&[
        (P0, "de_stug3_g", 0, 0, HOLD),
        (P1, "su_t34_76_1943", -75, 0, FIRE),
        (P1, "su_zis5", 0, -200, HOLD),
    ]);
    sim.step(&[PlayerCommand::new(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(2),
        },
    )]);
    let mut toward_truck = true;
    while sim.tick() < 300 && sim.unit_hp(UnitId(2)) != Some(0) {
        sim.step(&[]);
        let stug = unit(&sim, 0);
        let to_truck = unit(&sim, 2).position - stug.position;
        let length = to_truck.length();
        let want = Vec2::new(to_truck.x / length, to_truck.y / length);
        if stug.target.is_none() && dot(stug.facing, want) < Fixed::from_num(9) / 10 {
            toward_truck = false;
        }
    }
    assert!(sim.alert_direction(UnitId(0)).is_some());
    assert!(toward_truck);
}

#[test]
fn the_nearer_shooter_wins_without_twitching() {
    // T-34s west (60 m) and east (85 m) both fire into the StuG's flanks.
    let mut sim = sim_with(&[
        (P0, "de_stug3_g", 0, 0, HOLD),
        (P1, "su_t34_76_1943", -60, 0, FIRE),
        (P1, "su_t34_76_1943", 85, 0, FIRE),
    ]);
    let mut changes = 0;
    let mut last = None;
    for _ in 0..800 {
        sim.step(&[]);
        let side = sim.alert_direction(UnitId(0)).map(|d| d.x < Fixed::ZERO);
        if side.is_some() && last.is_some() && side != last {
            changes += 1;
        }
        if side.is_some() {
            last = side;
        }
    }
    assert_eq!(last, Some(true), "remembers the west");
    assert!(changes <= 1, "{changes} changes");
    assert!(dot(unit(&sim, 0).aim, WEST) > close());
}

#[test]
fn infantry_does_not_turn_on_its_own() {
    // Squads keep their own rules (prompt 047): no alert for them.
    let mut sim = sim_with(&[
        (P0, "de_inf_rifle_squad", 0, 0, HOLD),
        (P1, "su_t34_76_1943", -75, 0, FIRE),
    ]);
    run(&mut sim, 200, UnitId(0));
    assert_eq!(sim.alert_direction(UnitId(0)), None);
}

#[test]
fn same_seed_same_flank_fight() {
    let play = || {
        let mut sim = sim_with(&[
            (P0, "de_stug3_g", 0, 0, HOLD),
            (P0, "de_pak40", 20, 0, HOLD),
            (P1, "su_t34_76_1943", -60, 10, FIRE),
            (P1, "su_t34_76_1943", 85, -10, FIRE),
        ]);
        run(&mut sim, 600, UnitId(0));
        sim.state_hash()
    };
    assert_eq!(play(), play());
}
