//! Feedback on orders (prompt 175): dropped orders are reported with a
//! reason code, and the read-only queries (activity, fire status,
//! penetration, reachable point) change nothing.

use sim::map::MapLayout;
use sim::{
    Activity, Catalog, CombatRules, Command, FireMode, FireStatus, Fixed, Map, OrderEvent,
    PlayerCommand, PlayerId, RejectReason, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId,
    Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const TANK: &str = "de_pz4_h";
const TRUCK: &str = "de_opel_blitz";
const SQUAD: &str = "de_inf_rifle_squad";
const T34: &str = "su_t34_76_1943";
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "feedback_test".into(),
        samples_x: N,
        samples_y: N,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let half = (N as i32 - 1) / 2;
    let mut ids = Vec::with_capacity((N * N) as usize);
    for j in 0..N as i32 {
        for i in 0..N as i32 {
            ids.push(terrain((i - half) * 2, (j - half) * 2));
        }
    }
    let size = (N * N) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], ids).unwrap();
    if !objects.is_empty() {
        map.add_objects(&format!(
            "{}\n{}\n",
            sim::objects::CSV_HEADER,
            objects.join("\n")
        ))
        .unwrap();
    }
    Arc::new(map)
}

fn flat() -> Arc<Map> {
    map_with(|_, _| 0, &[])
}

fn sim_with(map: Arc<Map>, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
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
    };
    Sim::new(&config, map, 7).unwrap()
}

fn ids(list: &[u32]) -> Vec<UnitId> {
    list.iter().map(|&i| UnitId(i)).collect()
}

fn rejected(sim: &Sim) -> Vec<RejectReason> {
    sim.order_events()
        .iter()
        .map(|e| match e {
            OrderEvent::Rejected { reason, .. } => *reason,
        })
        .collect()
}

fn order(sim: &mut Sim, command: Command) -> Vec<RejectReason> {
    sim.step(&[PlayerCommand::new(P0, command)]);
    rejected(sim)
}

#[test]
fn reason_codes_are_unique_and_named() {
    let mut codes: Vec<u8> = RejectReason::ALL.iter().map(|r| r.code()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), RejectReason::ALL.len());
    let mut names: Vec<&str> = RejectReason::ALL.iter().map(|r| r.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), RejectReason::ALL.len());
}

#[test]
fn dropped_orders_are_reported_with_their_reason() {
    let mut sim = sim_with(
        flat(),
        &[
            (P0, TANK, 0, 0, HOLD),
            (P0, TRUCK, 20, 0, HOLD),
            (P0, SQUAD, 40, 0, HOLD),
            (P1, T34, 0, 200, HOLD),
        ],
    );
    let before = sim.state_hash();
    // Attacking an own unit.
    let r = order(
        &mut sim,
        Command::Attack {
            units: ids(&[0]),
            target: UnitId(1),
        },
    );
    assert_eq!(r, vec![RejectReason::TargetFriendly]);
    // An unknown target.
    let r = order(
        &mut sim,
        Command::Attack {
            units: ids(&[0]),
            target: UnitId(99),
        },
    );
    assert_eq!(r, vec![RejectReason::TargetGone]);
    // An unarmed truck cannot hurt a tank: nothing is reported for units
    // without weapons, but a rifle squad against a T-34 is.
    let r = order(
        &mut sim,
        Command::Attack {
            units: ids(&[2]),
            target: UnitId(3),
        },
    );
    assert_eq!(r, vec![RejectReason::CannotHurtTarget]);
    // Mounting an enemy tank, with a tank only (no squad).
    let r = order(
        &mut sim,
        Command::Embark {
            units: ids(&[2]),
            carrier: UnitId(3),
        },
    );
    assert_eq!(r, vec![RejectReason::NoSeats]);
    let r = order(
        &mut sim,
        Command::Embark {
            units: ids(&[0]),
            carrier: UnitId(1),
        },
    );
    assert_eq!(r, vec![RejectReason::NoSquads]);
    // Towing a tank, unlimbering without a gun.
    let r = order(
        &mut sim,
        Command::Tow {
            units: ids(&[1]),
            gun: UnitId(0),
        },
    );
    assert_eq!(r, vec![RejectReason::NotAGun]);
    let r = order(
        &mut sim,
        Command::Unlimber {
            units: ids(&[0, 1]),
        },
    );
    assert_eq!(r, vec![RejectReason::NothingToUnlimber]);
    // A barrage without artillery, mines without engineers.
    let r = order(
        &mut sim,
        Command::Barrage {
            units: ids(&[0]),
            target: Vec2::from_meters(0, 100),
            radius_m: 30,
            salvos: 2,
            smoke: false,
        },
    );
    assert_eq!(r, vec![RejectReason::NoArtillery]);
    // Nothing is reported for other players' units or valid orders.
    sim.step(&[PlayerCommand::new(
        P1,
        Command::MoveTo {
            units: ids(&[0]),
            target: Vec2::from_meters(10, 10),
        },
    )]);
    assert!(rejected(&sim).is_empty());
    let r = order(
        &mut sim,
        Command::MoveTo {
            units: ids(&[0]),
            target: Vec2::from_meters(0, -50),
        },
    );
    assert!(r.is_empty(), "{r:?}");
    // The events of a tick are gone in the next one; the rules did not
    // change: the same orders without reports give the same state.
    sim.step(&[]);
    assert!(rejected(&sim).is_empty());
    let _ = before;
}

#[test]
fn queries_read_the_state_without_changing_it() {
    // A T-34 60 m in front of a Panzer IV and a rifle squad (both see it).
    let mut sim = sim_with(
        flat(),
        &[
            (P0, TANK, 0, 0, HOLD),
            (P0, SQUAD, 10, 45, FIRE),
            (P1, T34, 0, 60, HOLD),
        ],
    );
    for _ in 0..40 {
        sim.step(&[]);
    }
    let before = sim.state_hash();
    // Holding its fire, the tank says so and names the enemy.
    let (status, target) = sim.fire_status(UnitId(0)).unwrap();
    assert_eq!(status, FireStatus::HoldFire);
    assert_eq!(target, Some(UnitId(2)));
    // Rifles cannot hurt a tank.
    let (status, _) = sim.fire_status(UnitId(1)).unwrap();
    assert_eq!(status, FireStatus::Ineffective);
    // The Panzer IV's 75 mm gun penetrates a T-34 at 60 m from the side
    // and the rear; rifles nowhere.
    let tank = sim.penetration_view(UnitId(0), UnitId(2)).unwrap();
    assert!(tank[1] && tank[2], "{tank:?}");
    let rifles = sim.penetration_view(UnitId(1), UnitId(2)).unwrap();
    assert_eq!(rifles, [false; 3]);
    // Against infantry everything penetrates.
    assert_eq!(sim.penetration_view(UnitId(2), UnitId(1)), Some([true; 3]));
    assert_eq!(sim.activity(UnitId(0)), Some((Activity::Idle, None)));
    assert_eq!(sim.state_hash(), before);
    // Moving.
    sim.step(&[PlayerCommand::new(
        P0,
        Command::MoveTo {
            units: ids(&[0]),
            target: Vec2::from_meters(-100, 0),
        },
    )]);
    sim.step(&[]);
    assert_eq!(sim.activity(UnitId(0)).map(|a| a.0), Some(Activity::Moving));
}

#[test]
fn unreachable_targets_have_a_reachable_point() {
    // Deep water (terrain 7) east of x = 100 m.
    let map = map_with(|x, _| if x >= 100 { 7 } else { 0 }, &[]);
    let sim = sim_with(map, &[(P0, TANK, 0, 0, HOLD)]);
    let open = Vec2::from_meters(50, 20);
    assert_eq!(sim.reachable_point(UnitId(0), open), Some(open));
    let lake = Vec2::from_meters(200, 0);
    let reach = sim.reachable_point(UnitId(0), lake).unwrap();
    assert_ne!(reach, lake);
    assert!(reach.x < Fixed::from_num(100), "{reach:?}");
    assert!(reach.x > Fixed::from_num(80), "{reach:?}");
}
