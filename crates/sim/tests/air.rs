//! Air support and anti-aircraft fire (prompt 033): approach from the map
//! edge, cooldown and cost, bombs on infantry and trucks, flak shooting an
//! aircraft down, reconnaissance revealing hidden units, determinism.

use sim::air::AIR_SHOOTER;
use sim::map::MapLayout;
use sim::{
    AirBlock, AirEvent, AirMission, Catalog, CombatEvent, CombatRules, Command, FireMode, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, TerrainRules, UnitId, UnitType, Vec2,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// Flat 800 m × 800 m around the origin, 2 m samples.
fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "air_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

/// A match on the flat map; catalog ids of guns (`gun_*`) become
/// stationary units like fortifications.
fn sim_with(spawns: &[(PlayerId, &str, i32, i32, FireMode)], seed: u64) -> Sim {
    let map = flat_map();
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let mut names: Vec<&str> = spawns.iter().map(|s| s.1).collect();
    names.sort_unstable();
    names.dedup();
    let unit_types: Vec<UnitType> = names
        .iter()
        .map(|id| {
            let entry = catalog.get(id).unwrap();
            UnitType::from_catalog(entry, &scale)
                .unwrap_or_else(|| UnitType::fixed_from_catalog(entry, &scale, 0))
        })
        .collect();
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
    };
    Sim::new(&config, map, seed).unwrap()
}

fn call(player: PlayerId, mission: AirMission, x: i32, y: i32) -> PlayerCommand {
    PlayerCommand::new(
        player,
        Command::CallAir {
            mission,
            target: Vec2::from_meters(x, y),
        },
    )
}

/// Runs `ticks` ticks; the combat and air events.
fn run(sim: &mut Sim, ticks: u32) -> (Vec<CombatEvent>, Vec<AirEvent>) {
    let mut combat = Vec::new();
    let mut air = Vec::new();
    for _ in 0..ticks {
        sim.step(&[]);
        combat.extend_from_slice(sim.events());
        air.extend_from_slice(sim.air_events());
    }
    (combat, air)
}

const HOLD: FireMode = FireMode::HoldFire;
const FREE: FireMode = FireMode::FireAtWill;

#[test]
fn aircraft_fly_in_from_the_edge_over_the_target_and_out() {
    // Player 0 (German) starts in the north (+y), calls fighters on a
    // point in the south.
    let mut sim = sim_with(
        &[
            (P0, "de_pz4_h", 0, 300, HOLD),
            (P1, "su_t34_76_1943", 0, -300, HOLD),
        ],
        1,
    );
    let before = sim.account(P0).unwrap();
    sim.step(&[call(P0, AirMission::Strafe, 0, -100)]);
    assert!(sim.air_events().iter().any(|e| matches!(
        e,
        AirEvent::Called {
            mission: AirMission::Strafe,
            ..
        }
    )));
    let after = sim.account(P0).unwrap();
    assert_eq!(
        [
            before[0] - after[0],
            before[1] - after[1],
            before[2] - after[2]
        ],
        [60, 60, 10]
    );
    let snapshot = sim.snapshot();
    assert_eq!(snapshot.aircraft.len(), 2);
    assert_eq!(snapshot.sorties.len(), 1);
    let first = snapshot.aircraft[0];
    assert_eq!(sim.air_types()[usize::from(first.kind)].name, "de_fw190_a8");
    // Outside the map, north of it (the map ends at y = 400).
    assert!(first.position.y > sim::Fixed::from_num(400), "{first:?}");
    // A second call while the mission cools down does nothing.
    sim.step(&[call(P0, AirMission::Strafe, 0, -100)]);
    assert!(sim.air_events().is_empty());
    assert_eq!(sim.snapshot().sorties.len(), 1);
    let option = sim
        .air_options(P0)
        .into_iter()
        .find(|o| o.mission == AirMission::Strafe)
        .unwrap();
    assert_eq!(option.blocked, Some(AirBlock::Cooldown));
    // The lead aircraft passes over the target, low.
    let mut closest = sim::Fixed::MAX;
    let mut low = sim::Fixed::MAX;
    let mut strafed = false;
    for _ in 0..600 {
        sim.step(&[]);
        strafed |= sim
            .air_events()
            .iter()
            .any(|e| matches!(e, AirEvent::Strafe { .. }));
        if let Some(a) = sim.snapshot().aircraft.iter().find(|a| a.id == first.id) {
            let d = (a.position.ground() - Vec2::from_meters(0, -100)).length();
            if d < closest {
                closest = d;
                low = a.position.z;
            }
        }
    }
    assert!(closest < sim::Fixed::from_num(5), "closest {closest}");
    assert!(
        low < sim::Fixed::from_num(60),
        "altitude over the target {low}"
    );
    assert!(strafed, "the fighters never fired");
    // Gone again.
    let snapshot = sim.snapshot();
    assert!(snapshot.aircraft.is_empty(), "{:?}", snapshot.aircraft);
    assert!(snapshot.sorties.is_empty());
    // The cooldown (90 s = 1800 ticks) is over.
    run(&mut sim, 1200);
    let option = sim
        .air_options(P0)
        .into_iter()
        .find(|o| o.mission == AirMission::Strafe)
        .unwrap();
    assert_eq!(option.blocked, None);
}

#[test]
fn bombers_hurt_infantry_and_trucks_and_leave_craters() {
    let mut sim = sim_with(
        &[
            (P0, "de_pz4_h", 0, 350, HOLD),
            (P1, "su_inf_rifle_squad", -10, -100, HOLD),
            (P1, "su_inf_rifle_squad", 10, -100, HOLD),
            (P1, "su_zis5", 0, -110, HOLD),
            (P1, "su_zis5", 0, -90, HOLD),
        ],
        3,
    );
    let full: Vec<u32> = (1..=4).map(|u| sim.unit_hp(UnitId(u)).unwrap()).collect();
    sim.step(&[call(P0, AirMission::Bomb, 0, -100)]);
    let (mut combat, mut air, mut craters) = (Vec::new(), Vec::new(), 0);
    for _ in 0..20 * 40 {
        sim.step(&[]);
        combat.extend_from_slice(sim.events());
        air.extend_from_slice(sim.air_events());
        craters += sim.crater_events().len();
    }
    let released = air
        .iter()
        .filter(|e| matches!(e, AirEvent::Released { .. }))
        .count();
    // Three He 111 with eight SC 250 each.
    assert_eq!(released, 24);
    let impacts = combat
        .iter()
        .filter(|e| matches!(e, CombatEvent::Hit { shooter, .. } if *shooter == AIR_SHOOTER))
        .count();
    assert!(impacts >= 24, "{impacts} bomb impacts");
    let lost: u32 = (1..=4)
        .map(|u| full[u as usize - 1] - sim.unit_hp(UnitId(u)).unwrap())
        .sum();
    assert!(lost >= 500, "only {lost} hit points lost");
    assert!(craters > 0, "no craters");
}

#[test]
fn flak_shoots_down_slow_aircraft() {
    // Four 37 mm guns around the target of two Ju 87.
    let mut downed = 0;
    let mut crashed = 0;
    for seed in 1..=3 {
        let mut sim = sim_with(
            &[
                (P0, "de_pz4_h", 0, 350, HOLD),
                (P1, "su_61k", -40, -100, FREE),
                (P1, "su_61k", 40, -100, FREE),
                (P1, "su_61k", 0, -60, FREE),
                (P1, "su_61k", 0, -140, FREE),
            ],
            seed,
        );
        sim.step(&[call(P0, AirMission::Attack, 0, -100)]);
        let (combat, air) = run(&mut sim, 20 * 50);
        assert!(
            combat
                .iter()
                .any(|e| matches!(e, CombatEvent::Shot { weapon, .. }
                if *weapon == sim::air::AA_WEAPON)),
            "the flak never fired"
        );
        downed += air
            .iter()
            .filter(|e| matches!(e, AirEvent::ShotDown { owner, .. } if *owner == P0))
            .count();
        crashed += air
            .iter()
            .filter(|e| matches!(e, AirEvent::Crashed { .. }))
            .count();
    }
    assert!(downed >= 2, "only {downed} of 6 Ju 87 shot down");
    assert_eq!(downed, crashed, "every downed aircraft crashes");
}

#[test]
fn flak_holding_fire_does_not_shoot() {
    let mut sim = sim_with(
        &[(P0, "de_pz4_h", 0, 350, HOLD), (P1, "su_61k", 0, -60, HOLD)],
        2,
    );
    sim.step(&[call(P0, AirMission::Attack, 0, -100)]);
    let (_, air) = run(&mut sim, 20 * 40);
    assert!(!air.iter().any(|e| matches!(e, AirEvent::Flak { .. })));
}

#[test]
fn reconnaissance_reveals_hidden_units_along_its_track() {
    // The tank sees 500 m; the enemy truck 650 m south of it is hidden.
    let mut sim = sim_with(
        &[
            (P0, "de_pz4_h", 0, 350, HOLD),
            (P1, "su_zis5", 0, -300, HOLD),
        ],
        4,
    );
    run(&mut sim, 5);
    assert!(!sim.visible_to(P0, UnitId(1)));
    sim.step(&[call(P0, AirMission::Recon, 0, -250)]);
    let mut seen = false;
    for _ in 0..20 * 120 {
        sim.step(&[]);
        seen |= sim.visible_to(P0, UnitId(1));
    }
    assert!(seen, "the reconnaissance flight never revealed the truck");
    assert!(sim.snapshot().aircraft.is_empty(), "the Storch left");
    assert!(
        !sim.visible_to(P0, UnitId(1)),
        "the truck stays revealed after the flight"
    );
}

#[test]
fn enemy_aircraft_are_seen_only_within_sight() {
    let mut sim = sim_with(
        &[
            (P0, "de_pz4_h", 0, 350, HOLD),
            (P1, "su_t34_76_1943", 0, -300, HOLD),
        ],
        5,
    );
    sim.step(&[call(P0, AirMission::Recon, 0, 300)]);
    // Entering from the north edge, far from the T-34: player 1 does not
    // see it yet, player 0 does.
    sim.step(&[]);
    assert_eq!(sim.snapshot_for(P0).aircraft.len(), 1);
    assert_eq!(sim.snapshot_for(P1).aircraft.len(), 0);
    assert!(
        sim.snapshot_for(P1).sorties.is_empty(),
        "sorties are secret"
    );
}

#[test]
fn air_support_is_deterministic() {
    let play = || {
        let mut sim = sim_with(
            &[
                (P0, "de_pz4_h", 0, 350, HOLD),
                (P1, "su_61k", 30, -80, FREE),
                (P1, "su_inf_rifle_squad", 0, -100, HOLD),
                (P1, "su_zis5", 0, -120, HOLD),
            ],
            9,
        );
        sim.step(&[
            call(P0, AirMission::Attack, 0, -100),
            call(P0, AirMission::Bomb, 0, -100),
        ]);
        run(&mut sim, 20 * 45);
        sim
    };
    let (a, b) = (play(), play());
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.snapshot(), b.snapshot());
}
