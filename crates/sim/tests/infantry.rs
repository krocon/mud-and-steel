//! Infantry tests (prompt 022): soldiers in formation, stances, cover and
//! buildings, on hand-made maps with catalog units and fixed seeds.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, HitKind, Map, PlayerCommand, PlayerId,
    Scale, Sim, SimConfig, Spawn, Stance, StanceOrder, TerrainRules, UnitId, Vec2,
    unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const SQUAD: &str = "de_inf_rifle_squad";
const MG: &str = "su_inf_mg_team";

/// 800 m × 800 m flat map around the origin with the given objects
/// (`kind,variant,x_mm,y_mm,heading_deg` lines).
fn map_with(objects: &[String]) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "infantry_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let size = (n * n) as usize;
    let mut map = Map::from_rasters(layout, vec![0; size], vec![0; size]).unwrap();
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

/// Units start facing the map center; a unit at the origin faces north.
fn sim_with(map: Arc<Map>, seed: u64, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> Sim {
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
    };
    Sim::new(&config, map, seed).unwrap()
}

fn order(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn stance(player: PlayerId, unit: u32, stance: Stance) -> PlayerCommand {
    order(
        player,
        Command::SetStance {
            units: vec![UnitId(unit)],
            stance: StanceOrder::Fixed(stance),
        },
    )
}

fn run(sim: &mut Sim, ticks: u32) {
    for _ in 0..ticks {
        sim.step(&[]);
    }
}

/// Rounds fired and hits that wounded soldiers of `unit` during `ticks`
/// (or until the squad is wiped out). Every shot of the test is aimed at
/// `unit`, so `hits / shots` is the chance that a round wounds.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    shots: u32,
    hits: u32,
}

impl Tally {
    /// Wounding hits per 1000 rounds.
    fn per_mille(self) -> u32 {
        (self.hits * 1000).checked_div(self.shots).unwrap_or(0)
    }
}

impl std::ops::Add for Tally {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self {
            shots: self.shots + o.shots,
            hits: self.hits + o.hits,
        }
    }
}

fn tally(sim: &mut Sim, unit: UnitId, ticks: u32) -> Tally {
    let mut t = Tally::default();
    for _ in 0..ticks {
        if sim.unit_hp(unit) == Some(0) {
            break;
        }
        sim.step(&[]);
        for e in sim.events() {
            match e {
                CombatEvent::Shot { unit: shooter, .. } if *shooter != unit => t.shots += 1,
                CombatEvent::Hit {
                    kind: HitKind::Unit(u),
                    damage,
                    ..
                } if *u == unit && *damage > 0 => t.hits += 1,
                _ => {}
            }
        }
    }
    t
}

/// A row of sandbag positions 4 m north of the origin.
fn sandbags() -> Vec<String> {
    (-4..=4)
        .map(|k| format!("fort_sandbag_position,Position,{},-4000,90", k * 3000))
        .collect()
}

/// A rifle squad at the origin (holding fire) under fire from an MG team
/// 150 m north; rounds and wounding hits over `ticks`, summed over five
/// seeds.
fn hits_on_squad(objects: &[String], squad_stance: Stance, ticks: u32) -> Tally {
    let map = map_with(objects);
    (1..=5)
        .map(|seed| {
            let mut sim = sim_with(
                Arc::clone(&map),
                seed,
                &[(P0, SQUAD, 0, 0, HOLD), (P1, MG, 0, -150, FIRE)],
            );
            sim.step(&[stance(P0, 0, squad_stance)]);
            // Let the squad settle into its places (and cover spots).
            run(&mut sim, 60);
            tally(&mut sim, UnitId(0), ticks)
        })
        .fold(Tally::default(), |a, b| a + b)
}

#[test]
fn squads_have_soldiers_in_formation() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    run(&mut sim, 40);
    let snapshot = sim.snapshot();
    let soldiers: Vec<_> = snapshot
        .soldiers
        .iter()
        .filter(|s| s.unit == UnitId(0))
        .collect();
    assert_eq!(soldiers.len(), 9);
    assert_eq!(snapshot.units[0].hp, 900);
    assert_eq!(snapshot.units[0].stance, Some(Stance::Kneel));
    // Distinct places, all within 10 m of the squad.
    for (k, a) in soldiers.iter().enumerate() {
        assert!(a.position.length() < sim::Fixed::from_num(10));
        for b in &soldiers[k + 1..] {
            assert!((a.position - b.position).length() > sim::Fixed::from_num(2));
        }
    }
}

#[test]
fn squad_in_cover_takes_fewer_hits() {
    let open = hits_on_squad(&[], Stance::Kneel, 400);
    let covered = hits_on_squad(&sandbags(), Stance::Kneel, 400);
    println!("open {open:?}, behind sandbags {covered:?}");
    assert!(open.hits >= 20 && covered.shots >= 100, "too few rounds");
    // Sandbags catch 80 % of the rounds that would hit.
    assert!(
        covered.per_mille() * 2 < open.per_mille(),
        "open {open:?}, covered {covered:?}"
    );
}

#[test]
fn soldiers_take_cover_spots_behind_objects() {
    let mut sim = sim_with(map_with(&sandbags()), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
    run(&mut sim, 80);
    let snapshot = sim.snapshot();
    let in_cover = snapshot.soldiers.iter().filter(|s| s.in_cover).count();
    assert!(in_cover >= 7, "{in_cover} soldiers in cover");
    // The spots lie behind the sandbags (south of y = -4 m).
    for s in snapshot.soldiers.iter().filter(|s| s.in_cover) {
        assert!(s.position.y > sim::Fixed::from_num(-4), "{:?}", s.position);
    }
}

#[test]
fn stone_wall_protects_kneeling_soldiers() {
    let walls: Vec<String> = (-5..=5)
        .map(|k| format!("env_wall_stone,Segment,{},-4000,90", k * 3000))
        .collect();
    let open = hits_on_squad(&[], Stance::Kneel, 400);
    let behind = hits_on_squad(&walls, Stance::Kneel, 400);
    println!("open {open:?}, behind a wall {behind:?}");
    assert!(behind.shots >= 100, "too few rounds");
    assert!(
        behind.per_mille() * 2 < open.per_mille(),
        "open {open:?}, behind a wall {behind:?}"
    );
}

#[test]
fn prone_squad_takes_fewer_hits_than_standing() {
    let standing = hits_on_squad(&[], Stance::Stand, 400);
    let prone = hits_on_squad(&[], Stance::Prone, 400);
    println!("standing {standing:?}, prone {prone:?}");
    assert!(
        prone.per_mille() * 2 < standing.per_mille(),
        "standing {standing:?}, prone {prone:?}"
    );
}

#[test]
fn stance_changes_the_speed() {
    let ticks_to = |s: Stance| {
        let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, HOLD)]);
        sim.step(&[stance(P0, 0, s)]);
        sim.step(&[order(
            P0,
            Command::MoveTo {
                units: vec![UnitId(0)],
                target: Vec2::from_meters(20, 0),
            },
        )]);
        let mut ticks = 0;
        while sim.snapshot().units[0].target.is_some() && ticks < 5000 {
            sim.step(&[]);
            ticks += 1;
        }
        ticks
    };
    let (stand, kneel, prone) = (
        ticks_to(Stance::Stand),
        ticks_to(Stance::Kneel),
        ticks_to(Stance::Prone),
    );
    println!("ticks for 20 m: stand {stand}, kneel {kneel}, prone {prone}");
    assert!(stand < kneel && kneel < prone);
    // 20 % of the speed: about five times as long (plus the stance change).
    assert!(prone > stand * 4, "stand {stand}, prone {prone}");
}

#[test]
fn automatic_stance_goes_prone_under_fire() {
    let mut sim = sim_with(
        map_with(&[]),
        1,
        &[(P0, SQUAD, 0, 0, HOLD), (P1, MG, 0, -150, HOLD)],
    );
    run(&mut sim, 40);
    assert_eq!(sim.snapshot().units[0].stance, Some(Stance::Kneel));
    sim.step(&[order(
        P1,
        Command::SetFireMode {
            units: vec![UnitId(1)],
            mode: FIRE,
        },
    )]);
    run(&mut sim, 200);
    assert_eq!(sim.snapshot().units[0].stance, Some(Stance::Prone));
}

fn house() -> Vec<String> {
    vec!["env_house_farm,Intact,0,0,0".into()]
}

fn garrison(player: PlayerId, units: &[u32]) -> PlayerCommand {
    order(
        player,
        Command::Garrison {
            units: units.iter().copied().map(UnitId).collect(),
            building: 0,
        },
    )
}

#[test]
fn squads_occupy_buildings_up_to_their_capacity() {
    let mut sim = sim_with(
        map_with(&house()),
        1,
        &[
            (P0, SQUAD, -40, 0, HOLD),
            (P0, MG, -40, 20, HOLD),
            (P0, SQUAD, -40, -25, HOLD),
            (P1, "de_inf_mg42_team", 40, 0, HOLD),
        ],
    );
    assert_eq!(sim.building_capacity(0), Some(12));
    sim.step(&[garrison(P0, &[0])]);
    run(&mut sim, 600);
    let garrisons = |sim: &Sim| -> Vec<Option<u32>> {
        sim.snapshot().units.iter().map(|u| u.garrison).collect()
    };
    assert_eq!(garrisons(&sim)[0], Some(0));
    assert_eq!(sim.building_occupants(0), (Some(P0), 9));
    // 9 + 2 fit, 9 + 9 do not; the enemy cannot get in.
    sim.step(&[garrison(P0, &[1, 2]), garrison(P1, &[3])]);
    run(&mut sim, 600);
    assert_eq!(
        garrisons(&sim),
        vec![Some(0), Some(0), None, None],
        "{:?}",
        sim.building_occupants(0)
    );
    assert_eq!(sim.building_occupants(0), (Some(P0), 11));
    // Leaving: a move order takes the squad out where it entered.
    sim.step(&[order(
        P0,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(-60, 0),
        },
    )]);
    run(&mut sim, 700);
    let unit = sim.snapshot().units[0];
    assert_eq!(unit.garrison, None);
    assert!((unit.position - Vec2::from_meters(-60, 0)).length() < sim::Fixed::from_num(3));
}

#[test]
fn occupants_take_fewer_hits_than_squads_in_the_open() {
    let hits = |objects: &[String], garrisoned: bool| -> Tally {
        let map = map_with(objects);
        (1..=5)
            .map(|seed| {
                let mut sim = sim_with(
                    Arc::clone(&map),
                    seed,
                    &[(P0, SQUAD, 0, 12, HOLD), (P1, MG, 0, -150, HOLD)],
                );
                if garrisoned {
                    sim.step(&[garrison(P0, &[0])]);
                    run(&mut sim, 300);
                    assert_eq!(sim.snapshot().units[0].garrison, Some(0));
                } else {
                    sim.step(&[stance(P0, 0, Stance::Stand)]);
                    run(&mut sim, 300);
                }
                sim.step(&[order(
                    P1,
                    Command::Attack {
                        units: vec![UnitId(1)],
                        target: UnitId(0),
                    },
                )]);
                tally(&mut sim, UnitId(0), 400)
            })
            .fold(Tally::default(), |a, b| a + b)
    };
    let inside = hits(&house(), true);
    // Same squad in the open, 12 m south of where the house would stand.
    let open = hits(&[], false);
    println!("open {open:?}, in the house {inside:?}");
    assert!(inside.shots >= 100, "too few rounds");
    assert!(
        inside.per_mille() * 2 < open.per_mille(),
        "open {open:?}, inside {inside:?}"
    );
}

#[test]
fn infantry_runs_are_deterministic() {
    let play = || {
        let mut sim = sim_with(
            map_with(&sandbags()),
            9,
            &[
                (P0, SQUAD, 0, 0, FIRE),
                (P1, SQUAD, 0, -120, FIRE),
                (P1, MG, 30, -120, FIRE),
            ],
        );
        sim.step(&[order(
            P1,
            Command::MoveTo {
                units: vec![UnitId(1), UnitId(2)],
                target: Vec2::from_meters(0, -60),
            },
        )]);
        run(&mut sim, 600);
        (sim.state_hash(), sim.snapshot())
    };
    let (a, b) = (play(), play());
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    // Somebody got hurt.
    assert!(a.1.units.iter().any(|u| u.hp < u.max_hp));
}

#[test]
fn ruins_cannot_be_occupied_but_squads_move_into_the_rubble() {
    let ruin = vec!["env_house_farm,Ruin,0,0,0".to_string()];
    let mut sim = sim_with(map_with(&ruin), 1, &[(P0, SQUAD, -40, 0, HOLD)]);
    assert_eq!(sim.building_condition(0), sim::BuildingCondition::Ruin);
    sim.step(&[garrison(P0, &[0])]);
    run(&mut sim, 900);
    let snapshot = sim.snapshot();
    assert_eq!(snapshot.units[0].garrison, None);
    assert_eq!(sim.building_occupants(0), (None, 0));
    // All soldiers stand in the 14 m × 9 m rubble, at their spots.
    for s in &snapshot.soldiers {
        assert!(
            s.position.x.abs() < 5 && s.position.y.abs() < 7,
            "{:?}",
            s.position
        );
        assert!(s.in_cover);
    }
}

/// Wounding hits per round of an MG team 150 m north at a rifle squad at
/// the origin, in the open or ordered into a farmhouse ruin there.
fn hits_in_rubble(ruin: bool) -> Tally {
    let objects = if ruin {
        vec!["env_house_farm,Ruin,0,0,0".to_string()]
    } else {
        Vec::new()
    };
    let map = map_with(&objects);
    (1..=5)
        .map(|seed| {
            let mut sim = sim_with(
                Arc::clone(&map),
                seed,
                &[(P0, SQUAD, 0, 0, HOLD), (P1, MG, 0, -150, HOLD)],
            );
            sim.step(&[garrison(P0, &[0])]);
            run(&mut sim, 120);
            sim.step(&[order(
                P1,
                Command::Attack {
                    units: vec![UnitId(1)],
                    target: UnitId(0),
                },
            )]);
            tally(&mut sim, UnitId(0), 400)
        })
        .fold(Tally::default(), |a, b| a + b)
}

#[test]
fn rubble_gives_cover() {
    let open = hits_in_rubble(false);
    let rubble = hits_in_rubble(true);
    println!("open {open:?}, in the ruin {rubble:?}");
    assert!(rubble.shots >= 100, "too few rounds");
    assert!(
        rubble.per_mille() * 3 < open.per_mille() * 2,
        "open {open:?}, rubble {rubble:?}"
    );
}

#[test]
fn a_building_collapsing_on_its_occupants_kills_some_and_leaves_the_rest_in_the_ruin() {
    // A Panzer IV shells the farmhouse the squad holds (2500 hit points,
    // a ruin after five 7.5 cm hits, prompt 020); five seeds.
    let (mut before, mut after) = (0, 0);
    for seed in 1..=5 {
        let mut sim = sim_with(
            map_with(&house()),
            seed,
            &[(P0, SQUAD, -30, 0, HOLD), (P1, "de_pz4_h", 0, -150, HOLD)],
        );
        sim.step(&[garrison(P0, &[0])]);
        run(&mut sim, 400);
        assert_eq!(sim.snapshot().units[0].garrison, Some(0));
        sim.step(&[order(
            P1,
            Command::AttackGround {
                units: vec![UnitId(1)],
                target: Vec2::ZERO,
            },
        )]);
        let mut ticks = 0;
        while sim.building_condition(0) != sim::BuildingCondition::Ruin && ticks < 3000 {
            sim.step(&[]);
            ticks += 1;
        }
        assert_eq!(sim.building_condition(0), sim::BuildingCondition::Ruin);
        let alive = |sim: &Sim| sim.snapshot().soldiers.iter().filter(|s| s.hp > 0).count();
        before += alive(&sim);
        sim.step(&[]);
        after += alive(&sim);
        let snapshot = sim.snapshot();
        // (The shelling may have killed the whole squad before.)
        assert!(snapshot.units[0].hp == 0 || snapshot.units[0].garrison.is_none());
        // The survivors stay in the rubble.
        for s in snapshot.soldiers.iter().filter(|s| s.hp > 0) {
            assert!(
                s.position.x.abs() < 5 && s.position.y.abs() < 8,
                "{:?}",
                s.position
            );
        }
    }
    println!("soldiers alive before the collapse {before}, after {after}");
    assert!(after < before, "nobody died in the collapses");
}
