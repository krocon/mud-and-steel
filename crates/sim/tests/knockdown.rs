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
