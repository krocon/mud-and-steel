//! Blasts knock soldiers down (prompt 048): who falls, where he lies, when
//! he gets up; tank ammunition explosions; determinism.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, FireMode, InfantryRules, Map, PlayerCommand,
    PlayerId, Scale, Sim, SimConfig, SoldierSnapshot, Spawn, Stance, StanceOrder, TerrainRules,
    UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const MORTAR: &str = "de_inf_mortar_medium";
const SQUAD: &str = "su_inf_rifle_squad";

fn flat_map() -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "knockdown_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    Arc::new(Map::from_rasters(layout, vec![0; count], vec![0; count]).unwrap())
}

fn sim_with(
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
    infantry: InfantryRules,
    seed: u64,
) -> Sim {
    let map = flat_map();
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
        infantry,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    Sim::new(&config, map, seed).unwrap()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn stance(unit: u32, stance: Stance) -> PlayerCommand {
    cmd(
        P1,
        Command::SetStance {
            units: vec![UnitId(unit)],
            stance: StanceOrder::Fixed(stance),
        },
    )
}

fn soldiers(sim: &Sim, unit: u32) -> Vec<SoldierSnapshot> {
    sim.snapshot()
        .soldiers
        .into_iter()
        .filter(|s| s.unit == UnitId(unit))
        .collect()
}

/// One 8 cm mortar shell on a squad standing (or in `st`) at the origin.
/// Returns the sim right after the impact, the soldiers just before it and
/// the point of impact (`None` if the shell never landed).
fn one_shell(st: Stance, seed: u64) -> (Sim, Vec<SoldierSnapshot>, Option<Vec2>) {
    let mut sim = sim_with(
        &[
            (P0, MORTAR, 0, 150, FireMode::HoldFire),
            (P1, SQUAD, 0, 0, FireMode::HoldFire),
        ],
        InfantryRules::builtin().unwrap(),
        seed,
    );
    sim.step(&[stance(1, st)]);
    for _ in 0..60 {
        sim.step(&[]);
    }
    sim.step(&[cmd(
        P0,
        Command::Barrage {
            units: vec![UnitId(0)],
            target: Vec2::ZERO,
            radius_m: 0,
            salvos: 1,
            smoke: false,
        },
    )]);
    for _ in 0..20 * 60 {
        let before = soldiers(&sim, 1);
        sim.step(&[]);
        let hit = sim.events().iter().find_map(|e| match e {
            CombatEvent::Hit { shooter, at, .. } if *shooter == UnitId(0) => Some(at.ground()),
            _ => None,
        });
        if hit.is_some() {
            return (sim, before, hit);
        }
    }
    (sim, Vec::new(), None)
}

#[test]
fn a_blast_throws_soldiers_down_and_they_get_up_again() {
    let mut knocked_total = 0;
    for seed in 1..=6 {
        let (mut sim, before, at) = one_shell(Stance::Stand, seed);
        let at = at.expect("the shell lands");
        let after = soldiers(&sim, 1);
        let down: Vec<usize> = (0..after.len()).filter(|&k| after[k].knocked > 0).collect();
        knocked_total += down.len();
        for (k, (a, b)) in before.iter().zip(&after).enumerate() {
            let (d0, d1) = ((a.position - at).length(), (b.position - at).length());
            if b.knocked > 0 {
                // 8 cm: 4.8 m for standing soldiers in the open (the
                // positions before the tick: soldiers still settle a bit).
                assert!(
                    d0 < sim::Fixed::from_num(50) / 10,
                    "soldier {k} {d0} m away"
                );
                assert_eq!(b.knock_from, at);
                // Thrown away from the blast, at most 2.5 m.
                assert!(
                    d1 + sim::Fixed::ONE / 5 >= d0,
                    "soldier {k} thrown towards the blast"
                );
                let moved = (b.position - a.position).length();
                assert!(moved <= sim::Fixed::from_num(27) / 10, "thrown {moved} m");
            } else if a.hp > 0 && b.hp > 0 {
                assert!(
                    d0 >= sim::Fixed::from_num(46) / 10,
                    "soldier {k} {d0} m away"
                );
            }
        }
        // Down for 6 s real = 60 ticks at 20 Hz and time factor 2: lies
        // still meanwhile, then gets up and takes his place again.
        let lying: Vec<Vec2> = after.iter().map(|s| s.position).collect();
        for t in 0..59 {
            sim.step(&[]);
            let now = soldiers(&sim, 1);
            for &k in &down {
                if now[k].hp > 0 {
                    assert!(now[k].knocked > 0, "seed {seed} tick {t}: up too early");
                    assert_eq!(now[k].position, lying[k], "a knocked soldier moved");
                }
            }
        }
        for _ in 0..80 {
            sim.step(&[]);
        }
        for s in soldiers(&sim, 1) {
            assert_eq!(s.knocked, 0, "everybody is up again");
        }
    }
    assert!(
        knocked_total >= 6,
        "only {knocked_total} soldiers knocked down"
    );
}

#[test]
fn lying_soldiers_are_knocked_down_less() {
    let count = |st: Stance| -> usize {
        (1..=6)
            .map(|seed| {
                let (sim, _, _) = one_shell(st, seed);
                soldiers(&sim, 1).iter().filter(|s| s.knocked > 0).count()
            })
            .sum()
    };
    let (standing, prone) = (count(Stance::Stand), count(Stance::Prone));
    assert!(prone < standing, "prone {prone}, standing {standing}");
}

#[test]
fn knockdowns_are_deterministic() {
    let hash = |seed| {
        let (mut sim, _, _) = one_shell(Stance::Stand, seed);
        for _ in 0..30 {
            sim.step(&[]);
        }
        sim.state_hash()
    };
    assert_eq!(hash(3), hash(3));
}

/// A Tiger kills a T-34 that has a rifle squad standing next to it; returns
/// the soldiers of the squad and the wreck position after the kill.
fn tank_dies_next_to_a_squad(ammo_pct: u32, seed: u64) -> (Vec<SoldierSnapshot>, Vec2) {
    let mut rules = InfantryRules::builtin().unwrap();
    rules.knockdown.ammo_explosion_pct = ammo_pct;
    let mut sim = sim_with(
        &[
            (P0, "de_tiger1_e", 0, 250, FireMode::HoldFire),
            (P1, "su_t34_76_1943", 0, 0, FireMode::HoldFire),
            (P1, SQUAD, 8, -4, FireMode::HoldFire),
        ],
        rules,
        seed,
    );
    sim.step(&[stance(2, Stance::Stand)]);
    sim.step(&[cmd(
        P0,
        Command::Attack {
            units: vec![UnitId(0)],
            target: UnitId(1),
        },
    )]);
    for _ in 0..20 * 120 {
        sim.step(&[]);
        let killed = sim
            .events()
            .iter()
            .any(|e| matches!(e, CombatEvent::Destroyed { unit, .. } if *unit == UnitId(1)));
        if killed {
            let wreck = sim.snapshot().units[1].position;
            return (soldiers(&sim, 2), wreck);
        }
    }
    panic!("the T-34 survived");
}

#[test]
fn a_tank_ammunition_explosion_knocks_the_squad_beside_it_down() {
    let (squad, wreck) = tank_dies_next_to_a_squad(100, 5);
    let down: Vec<_> = squad.iter().filter(|s| s.knocked > 0).collect();
    assert!(!down.is_empty(), "nobody knocked down");
    for s in down {
        assert_eq!(s.knock_from, wreck);
        assert!((s.position - wreck).length() <= sim::Fixed::from_num(12));
    }
    let (squad, _) = tank_dies_next_to_a_squad(0, 5);
    assert!(
        squad.iter().all(|s| s.knocked == 0),
        "no explosion, no knockdown"
    );
}

// Prompt 194: big charges throw farther, on a flight from the simulation,
// never into blocked ground.

/// 800 m × 800 m flat map around the origin: deep water (terrain 7) from
/// `water_from_m` east on (none if `None`) and the given objects.
fn map_with(water_from_m: Option<i32>, objects: &[String]) -> Arc<Map> {
    let n = 401u32;
    let layout = MapLayout {
        name: "throw_test".into(),
        samples_x: n,
        samples_y: n,
        cell_size_mm: 2000,
        starts: Vec::new(),
        markers: Vec::new(),
    };
    let count = (n * n) as usize;
    let mut terrain = vec![0u8; count];
    if let Some(from) = water_from_m {
        for y in 0..n {
            for x in 0..n {
                // Sample x lies at (x - 200) × 2 m.
                if (i64::from(x) - 200) * 2 >= i64::from(from) {
                    terrain[(y * n + x) as usize] = 7;
                }
            }
        }
    }
    let mut map = Map::from_rasters(layout, vec![0; count], terrain).unwrap();
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

/// The soldiers thrown by one blast: (before, after) of every soldier of
/// the squad whose throw began on this tick.
struct Blast {
    at: Vec2,
    thrown: Vec<(SoldierSnapshot, SoldierSnapshot)>,
}

/// One rocket of `caliber` (mm × 10) from the mortar 150 m north onto `aim`
/// next to a rifle squad at the origin in `st`, on `map`.
fn blast_on_squad(map: Arc<Map>, caliber: u32, aim: Vec2, st: Stance, seed: u64) -> Blast {
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let names = [MORTAR, SQUAD];
    let config = SimConfig {
        tick_hz: 20,
        unit_types: unit_types_from_catalog(&catalog, &scale, &names).unwrap(),
        spawns: [(P0, MORTAR, 0, 150), (P1, SQUAD, 0, 0)]
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
        infantry: InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let mut sim = Sim::new(&config, map, seed).unwrap();
    sim.step(&[stance(1, st)]);
    for _ in 0..80 {
        sim.step(&[]);
    }
    assert!(sim.debug_rocket(UnitId(0), aim, caliber));
    for _ in 0..20 * 60 {
        let before = soldiers(&sim, 1);
        sim.step(&[]);
        let hit = sim.events().iter().find_map(|e| match e {
            CombatEvent::Hit { shooter, at, .. } if *shooter == UnitId(0) => Some(at.ground()),
            _ => None,
        });
        if let Some(at) = hit {
            let tick = sim.snapshot().tick;
            let after = soldiers(&sim, 1);
            let thrown = before
                .into_iter()
                .zip(after)
                .filter(|(_, b)| b.flight > 0 && b.throw_tick == tick)
                .collect();
            return Blast { at, thrown };
        }
    }
    panic!("the rocket never landed");
}

fn meters(f: sim::Fixed) -> f64 {
    f.to_num::<f64>()
}

#[test]
fn the_throw_grows_with_the_charge() {
    let rules = InfantryRules::builtin().unwrap().knockdown;
    let air = sim::AirRules::builtin().unwrap();
    // 8 cm stays as before prompt 194, 15 cm and more throw several meters,
    // bombs and the 38 cm rocket farther.
    assert_eq!(rules.throw_mm(810), 2500);
    assert_eq!(rules.throw_mm(500), 2500);
    let heavy = rules.throw_mm(1500);
    assert!((4000..=8000).contains(&heavy), "15 cm: {heavy} mm");
    let sc250 = rules.throw_mm(air.bomb_caliber(250));
    let sc500 = rules.throw_mm(air.bomb_caliber(500));
    let sturmtiger = rules.throw_mm(3800);
    assert!(sc250 >= heavy && sc500 > sc250, "bombs {sc250} / {sc500} mm");
    assert!(sturmtiger > sc500 && sturmtiger >= 10_000, "38 cm: {sturmtiger} mm");

    // In the game: every thrown soldier flies the share of the charge's
    // throw that his distance leaves, away from the blast, for the time of
    // a 45-degree arc.
    let map = map_with(None, &[]);
    let mut farthest = Vec::new();
    for caliber in [810, 1500, air.bomb_caliber(250), 3800] {
        let b = blast_on_squad(Arc::clone(&map), caliber, Vec2::from_meters(0, -3), Stance::Stand, 4);
        assert!(!b.thrown.is_empty(), "{caliber}: nobody thrown");
        let radius = meters(sim::Fixed::from_num(rules.radius_mm(caliber)) / 1000);
        let throw = f64::from(rules.throw_mm(caliber)) / 1000.0;
        let mut best = 0.0f64;
        for (a, b2) in &b.thrown {
            // Where he stood when the blast went off.
            let d0 = meters((b2.throw_from - b.at).length());
            let moved = meters((b2.position - b2.throw_from).length());
            let expected = throw * (radius - d0) / radius;
            assert!(
                (moved - expected).abs() < 0.35,
                "{caliber}: thrown {moved:.2} m from {d0:.2} m, expected {expected:.2} m"
            );
            assert!(
                meters((b2.position - b.at).length()) + 0.2 >= d0,
                "{caliber}: thrown towards the blast"
            );
            assert!(meters((b2.throw_from - a.position).length()) < 0.3);
            // 45-degree arc: t = sqrt(2 d / g) real, at time factor 2 and
            // 20 Hz: 10 ticks per real second.
            let ticks = (2.0 * moved / 9.81).sqrt() * 10.0;
            assert!(
                (f64::from(b2.flight) - ticks).abs() <= 1.0,
                "{caliber}: flight {} ticks for {moved:.2} m",
                b2.flight
            );
            if b2.hp > 0 {
                // He lies the knock-down time after landing.
                assert_eq!(b2.knocked, 60 + b2.flight);
            }
            best = best.max(moved);
        }
        farthest.push(best);
    }
    println!("farthest throws 8 cm / 15 cm / SC 250 / 38 cm: {farthest:?}");
    assert!(farthest[0] <= 2.5 + 0.01);
    assert!(farthest[1] >= 3.0 && farthest[1] > farthest[0] + 1.0);
    assert!(farthest[2] >= farthest[1] - 0.5);
    assert!(farthest[3] >= 8.0);
}

#[test]
fn thrown_soldiers_never_land_in_water_or_houses() {
    // Deep water 4 m east of the squad, a 38 cm rocket west of it: the
    // throws towards the water are cut short at its shore.
    let map = map_with(Some(4), &[]);
    assert_eq!(map.terrain_at(Vec2::from_meters(5, 0)), 7);
    assert_eq!(map.terrain_at(Vec2::from_meters(1, 0)), 0);
    let mut cut = 0;
    for seed in 1..=3 {
        let b = blast_on_squad(Arc::clone(&map), 3800, Vec2::from_meters(-6, 0), Stance::Stand, seed);
        assert!(!b.thrown.is_empty(), "nobody thrown");
        let rules = InfantryRules::builtin().unwrap().knockdown;
        let radius = f64::from(rules.radius_mm(3800)) / 1000.0;
        for (_, s) in &b.thrown {
            assert_ne!(map.terrain_at(s.position), 7, "landed in the water at {:?}", s.position);
            let d0 = meters((s.throw_from - b.at).length());
            let full = 11.4 * (radius - d0) / radius;
            if meters((s.position - s.throw_from).length()) + 0.6 < full {
                cut += 1;
            }
        }
    }
    assert!(cut > 0, "no throw was cut short at the shore");

    // A farmhouse east of the squad: nobody lands inside its walls.
    let map = map_with(None, &["env_house_farm,Intact,9000,0,0".to_string()]);
    let house = map.objects()[0];
    let kind = &map.object_kinds()[usize::from(house.kind)];
    let (half_length, half_width) = match kind.shape {
        sim::objects::Shape::Box {
            half_length,
            half_width,
        } => (half_length, half_width),
        sim::objects::Shape::Circle { radius } => (radius, radius),
    };
    let (forward, right) = house.axes();
    let mut thrown = 0;
    for seed in 1..=3 {
        let b = blast_on_squad(Arc::clone(&map), 3800, Vec2::from_meters(-6, 0), Stance::Stand, seed);
        for (_, s) in &b.thrown {
            thrown += 1;
            let rel = s.position - house.position;
            let along = rel.x * forward.x + rel.y * forward.y;
            let across = rel.x * right.x + rel.y * right.y;
            assert!(
                along.abs() > half_length || across.abs() > half_width,
                "landed in the house at {:?}",
                s.position
            );
        }
    }
    assert!(thrown > 0);
}

/// Sum of the throws (m) of a 15 cm shell 5 m north of a squad at the
/// origin, over three seeds.
fn total_throw(objects: &[String], st: Stance) -> (f64, usize) {
    let map = map_with(None, objects);
    let mut sum = 0.0;
    let mut n = 0;
    for seed in 1..=3 {
        let b = blast_on_squad(Arc::clone(&map), 1500, Vec2::from_meters(0, -5), st, seed);
        for (_, s) in &b.thrown {
            sum += meters((s.position - s.throw_from).length());
            n += 1;
        }
    }
    (sum, n)
}

#[test]
fn lying_down_and_cover_damp_the_throw() {
    let (open, n_open) = total_throw(&[], Stance::Kneel);
    let (prone, n_prone) = total_throw(&[], Stance::Prone);
    // Sandbags 4 m north: the soldiers kneel at cover spots behind them.
    let sandbags: Vec<String> = (-4..=4)
        .map(|k| format!("fort_sandbag_position,Position,{},-4000,90", k * 3000))
        .collect();
    let (covered, n_covered) = total_throw(&sandbags, Stance::Kneel);
    println!(
        "kneeling {open:.1} m ({n_open}), prone {prone:.1} m ({n_prone}), behind sandbags {covered:.1} m ({n_covered})"
    );
    assert!(n_open > 0 && open > 1.0);
    assert!(prone < open * 0.6, "prone {prone}, kneeling {open}");
    assert!(covered < open * 0.6, "covered {covered}, kneeling {open}");
}
