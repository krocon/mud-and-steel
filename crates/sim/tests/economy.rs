//! Base building, resources and production (prompt 023).

use sim::scenario::{builtin_map, economy_commands, run_economy, skirmish_config};
use sim::{Command, FireMode, Fixed, PlaceError, PlayerCommand, PlayerId, Sim, SimConfig, Vec2};

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
    assert_eq!(sim.account(P0), Some([500, 600, 500]));
    assert_eq!(sim.faction(P0), Some("de"));
    assert_eq!(sim.faction(P1), Some("su"));
    assert_eq!(sim.income(P0), [40, 100, 90]);
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
    assert_eq!(sim.account(P0), Some([540, 700, 590]));
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
    assert_eq!(sim.account(P0), Some([500, 600, 350]));
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
    assert!(
        !sim.supply_points()[2].contested,
        "nobody contests the ford"
    );
    // Headquarters 40/100/90 and the point 50/90/40 (prompt 045).
    assert_eq!(sim.income(P0), [90, 190, 130]);
    // The read functions of the presentation (prompt 107) give the data.
    assert_eq!(sim.supply_income(), [50, 90, 40]);
    assert_eq!(sim.supply_radius(), Fixed::from_num(25));
    // An enemy unit in the area contests the point; it stays the owner's.
    let enemy = sim
        .snapshot()
        .units
        .iter()
        .find(|u| u.owner == P1 && !sim.is_structure(u.unit_type))
        .unwrap()
        .id;
    step(
        &mut sim,
        P1,
        Command::MoveTo {
            units: vec![enemy],
            target: point,
        },
    );
    let mut contested = false;
    for _ in 0..6000 {
        sim.step(&[]);
        if sim.supply_points()[2].contested {
            contested = true;
            break;
        }
    }
    assert!(contested, "the ford is contested");
    let ford = &sim.supply_points()[2];
    assert_eq!(ford.owner, Some(P0));
    assert_eq!(ford.taker, None);
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

/// Prompt 202, group D: prickly pear, agave and vine rows extend the hashed
/// object and cover rules (before `0xfae8_4ad0_7626_2954`).
/// Prompt 202, group C: seven buildings extend the hashed rules (before
/// `0x87e7_ef9a_cd69_6272`).
/// Prompt 202, group B: coast defences and the barracks hut extend the hashed
/// object rules (before `0xb126_94fd_cada_50b5`).
/// Prompt 202: the US vehicle list gained the jeep `us_jeep` (before
/// `0xa1ed_bc94_c531_1283`).
/// Golden hash of the scripted skirmish (prompt 023): construction,
/// production, rally points, supply points and the fight of the starting
/// units. Recorded on macOS ARM64. Last changed by prompt 127 (castle
/// pieces extend the hashed object, destruction, vision and infantry rules;
/// the skirmish uses none of them, the demo with the same rule change ends
/// with every unit where it did). Before (`0xdf8e_836e_557b_e127`), prompt 120
/// (Mediterranean biome: new map objects extend the hashed object,
/// destruction, fire, vision and infantry rules; the skirmish on
/// `test_small` uses none of them, the demo with the same rule change ends
/// with every unit where it did). Before (`0x3813_5701_663f_8ad8`), prompt 198 (infantry
/// behind tanks: the hashed `[escort]` tactics rules; the skirmish plays
/// as before: with the section off, or on without hashing its rules, the
/// old hash, checked). Before (`0x9341_d487_d364_8990`), prompt 196 (running and
/// hitting the dirt: the hashed `[rush]` and `[drop]` tactics rules, the
/// running state of squads and the time soldiers lie; the squads of the
/// skirmish now run in their bounds and throw themselves down under
/// blasts and bursts; with both sections off the old hash, checked). Before (`0x7d91_9d45_5e9c_4d0f`), prompt 194 (big blasts
/// throw soldiers: three new hashed knock-down rules and every soldier's
/// last throw; hashing without them gives the old value, the skirmish
/// plays as before). Before (`0x7004_c6d5_5878_f778`), the circling recon
/// (`recon_loiter`: a sixth air mission in the hashed air rules, its
/// aircraft types and cooldowns; hashing without them gives the old value,
/// the skirmish plays as before). Before (`0x1a99_554f_9420_4ad3`), the
/// Sturmtiger (`de_sturmtiger` at the end of the German vehicle works' list
/// in the hashed economy rules; without it the old value comes out, the
/// skirmish plays as before). Before (`0xafb0_2c7e_529f_7d66`), prompt 187 (groups
/// drive together: hashed `[march]` tactics rules, no fire-team bounds at
/// bridges; see `determinism.rs`). Before (`0x4be5_2419_b382_c8e1`),
/// prompt 177 (gun crews:
/// hashed `[gun_crew]` rules; see `determinism.rs`). Before (`0x25d6_7bfb_0e17_ff83`), prompt
/// 188 (target
/// choice by score: hashed `[targeting]` rules; see `determinism.rs`).
/// Before (`0x2a15_922b_d831_f01f`), prompt 176 (fire
/// discipline: hashed `[discipline]` tactics rules; the skirmish's trucks
/// drive away when fired at; see `determinism.rs`). Before
/// (`0xab39_28dc_8af4_b40c`), prompt 173 (movement
/// rules against stuck units and the progress of every unit in the hash;
/// see `determinism.rs`). Before (`0x2c11_092d_3344_0e3a`), prompt 110 (guns and
/// standing tanks react to fire from the flank: hashed `[flank]` tactics
/// rules and alert state, standing units turn towards hits; see
/// `determinism.rs`). Before (`0xb7e4_8df4_b2bc_e43b`), prompt 106 (heavy
/// vehicles push trees over: hashed per-variant crush limits and the
/// dense-stand check of the tree class; checked: without them in the hash
/// it is the one before, no tree falls; see `determinism.rs`). Before
/// (`0x4353_2085_ed39_76de`), prompt 163 (faster
/// mounting and dismounting: hashed transport rules with `climb_ms` per
/// soldier and shorter `disembark_ms`; see `determinism.rs`). Before
/// (`0x3e60_5854_3f4c_5793`), prompt 097 (vehicles
/// at real speed, direct fire and sight 1/8 with a 15 m minimum, the price
/// range reference 87 m, the starts of `test_small` 60 m apart; see
/// `determinism.rs`). Before (`0xa7db_2e45_cf79_5aff`), prompt 093 (tanks
/// against walls, infantry and buildings: the hashed destruction and combat
/// rules gained `[crush]`, `[overrun]` and `[vehicle_mg]`, units carry
/// their slow-down after a wall and their machine gun targets, and the
/// tanks of the skirmish fire their machine guns at infantry and run it
/// over; see `determinism.rs`). Before (`0x9038_1c53_98b6_6b5e`), prompt
/// 088 (fields: the
/// hashed terrain and weather rules gained three field types with the rules
/// of grass; with the old data files the old value comes out). Before
/// (`0xb4ab_0840_6daa_d29c`), prompt 085 (trees under
/// heavy fire: the hashed destruction rules gained `[blast]` and per class
/// `blast`, `damaged_clear`, `damaged_height_pct`, trees a bare stage; with
/// the old `data/destruction.toml` and without the new hashed fields the old
/// value comes out). Before (`0x7cd2_7c4e_1028_5c40`), prompt 078 (apartment
/// blocks: the hashed object, vision, destruction, infantry and fire rules
/// gained the `env_block_*` entries; with the old data files the old value
/// comes out, the skirmish plays as before). Before (`0xfd0e_b7ad_02e8_3f84`),
/// prompt 074 (rocks: the
/// hashed vision rules list `env_rock_outcrop` and `env_rock_massif` as
/// solid; with the old `data/vision.toml` the old value comes out, the
/// skirmish plays as before). Before (`0x5b04_4ce3_de45_ae63`), prompt 073 (tall grass:
/// the hashed terrain and weather rules gained `tall_grass` with the rules
/// of grass; with both entries left out of the hash the old value comes
/// out, the skirmish plays as before). Before (`0x8fec_1754_e6ee_3bc1`),
/// prompt 062 (rocket
/// launchers: the hashed indirect-fire rules gained the `rocket_launcher`
/// category, salvo and dispersion values and the salvo state of guns; the
/// vehicle works' lists end with the new units). Before
/// (`0x57e7_5da9_31e6_20dd`), prompt 067 (typical
/// combat distances: weapon ranges from `range_typical_m`, the range
/// reference and the velocity factor of unit prices). Before
/// (`0xe007_c7b9_cb19_a7b3`), the decisions of prompt 045
/// (2026-11-16): the
/// rate-of-fire factor changes the prices of the produced units, and the
/// `[defeat]` rules and the players who have had a base are hashed. Before
/// (`0x88ec_4656_950c_0751`): prompt 045 (economy values) and the merge of prompt 049
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
/// Prompt 224: four Dutch/Flemish stepped-gable buildings extend the hashed
/// object, destruction, fire, vision and infantry rules (`determinism.rs`;
/// before `0x59ce_fae2_076d_9d78`).
/// Prompt 241: five rural village detail objects (`env_tree_blossom`,
/// `env_sunflowers`, `env_well_shadoof`, `env_shed_wood`, `env_fence_picket`)
/// extend the hashed object, destruction, fire and vision rules
/// (`determinism.rs`; before `0xbd2b_2f9d_1484_eade`).
/// Prompt 228: Soviet 152 mm gun-howitzer `su_ml20` appended to
/// `[produce.vehicles] su` in hashed `data/economy.toml` (before
/// `0x5c7c_594e_0721_4657`).
/// Prompt 231: faster infantry movement speed (`[speed] infantry = 7/2`,
/// `infantry_heavy = 5/2`, `[rush] speed_pct = 155`, `[stance.prone] = 15`;
/// before `0xa12e_0f6d_30e1_34c6`).
#[test]
fn economy_golden_hash() {
    let sim = run_economy(42, 5000);
    assert_eq!(
        sim.state_hash(),
        0x8a9a_8e7b_7c21_427c,
        "economy golden hash mismatch: got {:#018x}",
        sim.state_hash()
    );
}
