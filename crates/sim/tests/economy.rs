//! Base building, resources and production (prompt 023).

use sim::scenario::{builtin_map, economy_commands, run_economy, skirmish_config};
use sim::{Command, FireMode, PlaceError, PlayerCommand, PlayerId, Sim, SimConfig, Vec2};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

/// The skirmish with every unit holding fire, so nothing is shot while a
/// test builds.
fn peaceful() -> SimConfig {
    let mut config = skirmish_config(&[]);
    for spawn in &mut config.spawns {
        spawn.fire_mode = FireMode::HoldFire;
    }
    config
}

fn start(config: &SimConfig) -> Sim {
    Sim::new(config, builtin_map(), 7).unwrap()
}

fn t(sim: &Sim, name: &str) -> u32 {
    sim.unit_type_index(name).unwrap()
}

fn hq(sim: &Sim, player: PlayerId) -> (sim::UnitId, Vec2) {
    let s = sim
        .structures()
        .into_iter()
        .find(|s| s.owner == player && s.unit_type == t(sim, "base_hq"))
        .unwrap();
    (s.unit, sim.unit_position(s.unit).unwrap())
}

fn step(sim: &mut Sim, player: PlayerId, command: Command) {
    sim.step(&[PlayerCommand::new(player, command)]);
}

fn run(sim: &mut Sim, ticks: u64) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

#[test]
fn skirmish_starts_with_completed_headquarters_and_resources() {
    let sim = start(&peaceful());
    let structures = sim.structures();
    assert_eq!(structures.len(), 2);
    assert!(structures.iter().all(|s| s.completed()));
    assert_eq!(sim.account(P0), Some([600, 600, 400]));
    assert_eq!(sim.faction(P0), Some("de"));
    assert_eq!(sim.faction(P1), Some("su"));
    assert_eq!(sim.income(P0), [40, 70, 80]);
    // The headquarters faces the map center along the y axis.
    let (_, p0) = hq(&sim, P0);
    assert!(p0.y > Vec2::ZERO.y);
    // Supply points from the map markers.
    let names: Vec<String> = sim.supply_points().into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["supply_village", "supply_hill", "supply_ford"]);
}

#[test]
fn income_is_paid_every_second() {
    let mut sim = start(&peaceful());
    // One minute: exactly the income per minute.
    run(&mut sim, 60 * 20 + 1);
    assert_eq!(sim.account(P0), Some([640, 670, 480]));
}

#[test]
fn placement_rules() {
    let sim = start(&peaceful());
    let barracks = t(&sim, "base_barracks");
    let factory = t(&sim, "base_factory");
    let bunker = t(&sim, "de_bunker_tobruk");
    let (_, home) = hq(&sim, P0);
    let spot = sim.find_site(P0, barracks, home).unwrap();
    assert_eq!(sim.placement(P0, barracks, spot), Ok(()));
    // Other players' radius does not count, and the radius has an end.
    assert_eq!(
        sim.placement(P1, barracks, spot),
        Err(PlaceError::OutOfRange)
    );
    let far = home + Vec2::from_meters(0, -200);
    assert_eq!(
        sim.placement(P0, barracks, far),
        Err(PlaceError::OutOfRange)
    );
    // On the headquarters.
    assert_eq!(sim.placement(P0, barracks, home), Err(PlaceError::Blocked));
    // Requirements and factions.
    assert_eq!(sim.placement(P0, factory, spot), Err(PlaceError::Requires));
    assert_eq!(sim.placement(P0, bunker, spot), Err(PlaceError::Requires));
    assert_eq!(
        sim.placement(P0, t(&sim, "su_bunker_dzot"), spot),
        Err(PlaceError::NotBuildable)
    );
    // Units cannot be placed.
    assert_eq!(
        sim.placement(P0, t(&sim, "de_pz4_h"), spot),
        Err(PlaceError::NotBuildable)
    );
}

#[test]
fn placement_keeps_clear_of_units() {
    let mut sim = start(&peaceful());
    let barracks = t(&sim, "base_barracks");
    let (_, home) = hq(&sim, P0);
    let spot = sim.find_site(P0, barracks, home).unwrap();
    let squad = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.owner == P0 && u.stance.is_some())
        .unwrap()
        .id;
    step(
        &mut sim,
        P0,
        Command::MoveTo {
            units: vec![squad],
            target: spot,
        },
    );
    run(&mut sim, 40 * 20);
    assert_eq!(sim.placement(P0, barracks, spot), Err(PlaceError::Occupied));
}

#[test]
fn placement_needs_flat_dry_ground() {
    let sim = start(&peaceful());
    let depot = t(&sim, "base_depot");
    let map = builtin_map();
    // Every spot of the map: water and steep ground are never accepted, and
    // some spot near the headquarters is.
    let (_, home) = hq(&sim, P0);
    let mut ok = 0;
    for dy in -12..=12 {
        for dx in -12..=12 {
            let p = home + Vec2::from_meters(dx * 10, dy * 10);
            match sim.placement(P0, depot, p) {
                Ok(()) => {
                    ok += 1;
                    assert!(sim.path_grid().passable_at(sim::path::SizeClass::Large, p));
                    assert!(map.contains(p));
                }
                Err(
                    PlaceError::Blocked
                    | PlaceError::TooSteep
                    | PlaceError::OutOfRange
                    | PlaceError::Occupied
                    | PlaceError::OutsideMap,
                ) => {}
                Err(e) => panic!("unexpected {e:?}"),
            }
        }
    }
    assert!(ok > 10, "{ok} spots");
}

#[test]
fn construction_production_and_rally_point() {
    let mut sim = start(&peaceful());
    let barracks = t(&sim, "base_barracks");
    let (_, home) = hq(&sim, P0);
    let spot = sim.find_site(P0, barracks, home).unwrap();
    let units_before = sim.snapshot().units.len();
    step(
        &mut sim,
        P0,
        Command::Build {
            unit_type: barracks,
            position: spot,
        },
    );
    // Paid at once, a site appears with a tenth of its hit points.
    assert_eq!(sim.account(P0), Some([600, 600, 250]));
    let site = sim
        .structures()
        .into_iter()
        .find(|s| s.unit_type == barracks)
        .unwrap();
    assert!(!site.completed());
    assert_eq!(sim.unit_hp(site.unit), Some(300 + 3000 * 9 / 10 / 500));
    assert_eq!(sim.snapshot().units.len(), units_before + 1);
    // A site cannot produce.
    let squad = t(&sim, "de_inf_rifle_squad");
    step(
        &mut sim,
        P0,
        Command::Produce {
            building: site.unit,
            unit_type: squad,
        },
    );
    assert!(sim.structures().iter().all(|s| s.queue.is_empty()));
    // 25 s later it is completed with full hit points.
    run(&mut sim, 25 * 20);
    let built = sim
        .structures()
        .into_iter()
        .find(|s| s.unit == site.unit)
        .unwrap();
    assert!(built.completed());
    assert_eq!(sim.unit_hp(site.unit), Some(3000));
    // Production: paid when queued, the unit appears when done and drives
    // to the rally point.
    let account = sim.account(P0).unwrap();
    let rally = spot + Vec2::from_meters(-30, -30);
    step(
        &mut sim,
        P0,
        Command::SetRallyPoint {
            building: site.unit,
            point: rally,
        },
    );
    step(
        &mut sim,
        P0,
        Command::Produce {
            building: site.unit,
            unit_type: squad,
        },
    );
    let cost = sim.cost_of(squad).unwrap();
    let after = sim.account(P0).unwrap();
    assert_eq!(after[2] + cost.resources[2], account[2]);
    let ticks = u64::from(cost.time_ms) * 20 / 1000;
    let count = sim.snapshot().units.len();
    run(&mut sim, ticks + 1);
    let snap = sim.snapshot();
    assert_eq!(snap.units.len(), count + 1);
    let new = snap.units.last().unwrap();
    assert_eq!((new.owner, new.unit_type), (P0, squad));
    assert!(new.target.is_some() || sim.path_queue_len() > 0 || new.position == rally);
    run(&mut sim, 30 * 20);
    let new = sim
        .snapshot()
        .units
        .into_iter()
        .find(|u| u.id == new.id)
        .unwrap();
    // On its slot of the 3 × 3 gathering formation around the rally point.
    let spacing = sim::formation_spacing(new.spread);
    assert!((new.position - rally).length() <= spacing * 2);
}

#[test]
fn cancelling_refunds_and_queues_are_limited() {
    let mut sim = start(&peaceful());
    let barracks = t(&sim, "base_barracks");
    let (_, home) = hq(&sim, P0);
    let spot = sim.find_site(P0, barracks, home).unwrap();
    step(
        &mut sim,
        P0,
        Command::Build {
            unit_type: barracks,
            position: spot,
        },
    );
    run(&mut sim, 25 * 20 + 1);
    let b = sim
        .structures()
        .into_iter()
        .find(|s| s.unit_type == barracks)
        .unwrap()
        .unit;
    let before = sim.account(P0).unwrap();
    let squad = t(&sim, "de_inf_rifle_squad");
    for _ in 0..8 {
        step(
            &mut sim,
            P0,
            Command::Produce {
                building: b,
                unit_type: squad,
            },
        );
    }
    let queued = sim
        .structures()
        .into_iter()
        .find(|s| s.unit == b)
        .unwrap()
        .queue
        .len();
    // As many as the manpower pays for, at most five.
    let cost = sim.cost_of(squad).unwrap().resources[2];
    assert_eq!(queued, (before[2] / cost).min(5) as usize);
    assert!(queued >= 3);
    for _ in 0..10 {
        step(&mut sim, P0, Command::CancelProduction { building: b });
    }
    let after = sim.account(P0).unwrap();
    // Everything back, plus the income of the ticks in between.
    assert!(after[2] >= before[2]);
    assert!(sim.structures().iter().all(|s| s.queue.is_empty()));
    // Other players cannot order in someone else's building.
    let su_squad = t(&sim, "su_inf_rifle_squad");
    step(
        &mut sim,
        P1,
        Command::Produce {
            building: b,
            unit_type: su_squad,
        },
    );
    assert!(sim.structures().iter().all(|s| s.queue.is_empty()));
}

#[test]
fn buildings_cannot_move_and_are_destroyed_with_their_queue() {
    let mut config = peaceful();
    // Three T-34 of player 1 stand right in front of player 0's
    // headquarters and may shoot (the starting units cannot hurt buildings).
    let home = config
        .spawns
        .iter()
        .find(|s| s.owner == P0 && s.unit_type == "base_hq")
        .unwrap()
        .position;
    for dx in [-30, 30, 42] {
        config.spawns.push(sim::Spawn {
            owner: P1,
            unit_type: "su_t34_76_1943".into(),
            position: home + Vec2::from_meters(dx, -22),
            fire_mode: FireMode::FireAtWill,
        });
    }
    let mut sim = start(&config);
    let (hq0, _) = hq(&sim, P0);
    step(
        &mut sim,
        P0,
        Command::MoveTo {
            units: vec![hq0],
            target: Vec2::ZERO,
        },
    );
    run(&mut sim, 40);
    assert_eq!(sim.unit_position(hq0), Some(home));
    // The enemy tanks attack the headquarters until it falls.
    let tanks: Vec<_> = sim
        .snapshot()
        .units
        .iter()
        .filter(|u| u.owner == P1 && sim.unit_type_index("su_t34_76_1943") == Some(u.unit_type))
        .map(|u| u.id)
        .collect();
    step(
        &mut sim,
        P1,
        Command::Attack {
            units: tanks,
            target: hq0,
        },
    );
    for _ in 0..4000 {
        sim.step(&[]);
        if sim.unit_hp(hq0) == Some(0) {
            break;
        }
    }
    assert_eq!(sim.unit_hp(hq0), Some(0), "the headquarters fell");
    assert!(sim.structures().iter().all(|s| s.unit != hq0));
    assert_eq!(sim.income(P0), [0, 0, 0]);
}

#[test]
fn supply_points_are_taken_and_pay() {
    let mut sim = start(&peaceful());
    let point = sim.supply_points()[2].position; // the ford
    let tank = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.owner == P0 && !sim.is_structure(u.unit_type))
        .unwrap()
        .id;
    step(
        &mut sim,
        P0,
        Command::MoveTo {
            units: vec![tank],
            target: point,
        },
    );
    let mut taken = None;
    for tick in 0..4000 {
        sim.step(&[]);
        if sim.supply_points()[2].owner == Some(P0) {
            taken = Some(tick);
            break;
        }
    }
    assert!(taken.is_some(), "the ford was taken");
    assert_eq!(sim.income(P0), [100, 130, 110]);
}

#[test]
fn scripted_production_chain_is_deterministic() {
    let a = run_economy(42, 3500);
    let b = run_economy(42, 3500);
    assert_eq!(a.state_hash(), b.state_hash());
    // Both players built their base (before the produced armored cars
    // reach the other base) and produced units.
    for player in [P0, P1] {
        let built = a
            .structures()
            .into_iter()
            .filter(|s| s.owner == player && s.completed())
            .count();
        assert!(built >= 3, "player {} built {built}", player.0);
    }
    let produced = a.snapshot().units.len() - skirmish_config(&[]).spawns.len();
    assert!(produced >= 6 + 2, "{produced} units and buildings added");
    // The commands depend only on the state.
    assert_eq!(economy_commands(&a), economy_commands(&b));
}

/// Golden hash of the scripted skirmish (prompt 023): construction,
/// production, rally points, supply points and the fight of the starting
/// units. Recorded on macOS ARM64. Last changed by the merge of prompt 049
/// (wounds, buddy aid and bailed-out crews in the rules and the hash, see
/// `determinism.rs`; 049 alone `0x5dc4_d2fb_ab3e_f22e`) with 047, 054 and
/// 039. Before (`0x745c_45c6_528b_58ee`): the merge of prompt 047
/// (tactics rules and squad state in the hash; the squads' fight runs
/// differently; 047 before 054: `0xe06d_4822_417f_100e`) with 054. Before
/// (`0x656a_58c4_9caa_aed9`): the merge of prompt 054
/// with main (the rocket battery position is a fifth base building of the
/// hashed economy rules and shifts the unit type indices; checked: with
/// the old `data/economy.toml` the hash stays the one before;
/// 054 alone `0xd7bb_f247_09d6_e516`). Before (`0xd6cd_83ac_0539_44dd`):
/// the merge of prompt
/// 055: the air rules gained paratroopers (see `determinism.rs`) and the
/// vehicle works produce bridge-laying tanks (de, su); 055 alone
/// `0x285b_b16d_4efa_0ea0`. Before (`0x8ce5_3de4_6339_50aa`): prompt 050
/// (experience
/// rules and every unit's experience points in the hash; ranked units
/// shoot more accurately). Before (`0x81bd_862b_c792_ce79`): the merge of prompt 059
/// (desert village entries in the hashed destruction, infantry and vision
/// rule tables; the skirmish uses none of them; 059 with 048 alone
/// `0x787f_c469_8fa2_b0d1`) with 060. Before (`0x2f4e_e467_b3e8_4f64`): the
/// merge of prompt
/// 060 (downhill sight rules hashed; see `determinism.rs`; 060 alone
/// `0x69cf_fbce_474c_7b62`). Before (`0x6316_8c90_a213_4d1c`): prompt 048
/// (knockdown rules and every soldier's knock-down state in the hash). Before
/// (`0x87ed_9b47_05ad_8899`): the merge of prompt 035
/// (field work rules and state in the hash, the barracks also produce
/// engineer squads; 035 alone `0x5489_8b04_42f6_7a57`) with 036. Before
/// (`0xe435_5d73_1d25_d25a`): prompt 036 (supply
/// rules, stocks, consumption; the headquarters resupplies). Before
/// (`0xcdc0_5864_0c6f_5ac8`): the merge of prompt
/// 034 (transport rules and state, trucks, AT guns and howitzers produced
/// instead of the AT gun position) with 041 (fire) and 033 (air). Before:
/// the merge of 041 with 033 (`0x7828_5ee2_24ae_d5f3`); prompt 033 alone;
/// prompt 041 alone; prompt 034 alone; prompt 032: the indirect-fire state
/// is in the hash and the production lists (economy rules) gained mortar
/// teams and self-propelled guns.
#[test]
fn economy_golden_hash() {
    let sim = run_economy(42, 5000);
    assert_eq!(
        sim.state_hash(),
        0xa4a5_8dfc_9b57_0a72,
        "economy golden hash mismatch: got {:#018x}",
        sim.state_hash()
    );
}
