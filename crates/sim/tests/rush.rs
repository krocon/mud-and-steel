//! Running and hitting the dirt (prompt 196): a squad charging its target
//! runs upright and does not fire, bounding teams run, a long run exhausts
//! the squad (slower, worse aim) until it has rested; a mortar shell or a
//! machine-gun burst close by throws soldiers in the open to the ground at
//! once, soldiers behind a wall stay at their cover spots, fixed stances
//! and charging squads are left alone. Hand-made maps, catalog units, fixed
//! seeds; numbers with `--nocapture`.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, Engage, FireMode, Fixed, Formation, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, Stance, StanceOrder, TerrainRules,
    UnitId, Vec2, soldier_flags, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const SQUAD: &str = "de_inf_rifle_squad";
const ENEMY: &str = "su_inf_rifle_squad";
const HMG: &str = "su_inf_hmg_team";
const MORTAR: &str = "de_inf_mortar_medium";

/// 800 m × 800 m flat map around the origin with the given objects.
fn map_with(objects: &[String]) -> Arc<Map> {
    let n = 401;
    let layout = MapLayout {
        name: "rush_test".into(),
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
        mission: None,
        variant: None,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
    };
    Sim::new(&config, map, seed).unwrap()
}

fn cmd(player: PlayerId, command: Command) -> PlayerCommand {
    PlayerCommand::new(player, command)
}

fn attack_move(units: &[u32], x: i32, y: i32) -> PlayerCommand {
    cmd(
        P0,
        Command::FormationMove {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            target: Vec2::from_meters(x, y),
            formation: Formation::Line,
            facing: None,
            engage: Engage::All,
        },
    )
}

fn move_to(units: &[u32], x: i32, y: i32) -> PlayerCommand {
    cmd(
        P0,
        Command::MoveTo {
            units: units.iter().map(|&u| UnitId(u)).collect(),
            target: Vec2::from_meters(x, y),
        },
    )
}

fn set_stance(player: PlayerId, unit: u32, stance: StanceOrder) -> PlayerCommand {
    cmd(
        player,
        Command::SetStance {
            units: vec![UnitId(unit)],
            stance,
        },
    )
}

fn position(sim: &Sim, id: u32) -> Vec2 {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .position
}

fn stance(sim: &Sim, id: u32) -> Option<Stance> {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .stance
}

/// Soldiers of squad `id` down under fire.
fn down(sim: &Sim, id: u32) -> usize {
    sim.soldier_tactics(UnitId(id))
        .iter()
        .filter(|&&f| f & soldier_flags::DOWN != 0)
        .count()
}

fn m(v: i32) -> Fixed {
    Fixed::from_num(v)
}

/// Rounds squad `id` fired this tick.
fn shots(sim: &Sim, id: u32) -> usize {
    sim.events()
        .iter()
        .filter(|e| matches!(e, CombatEvent::Shot { unit, .. } if *unit == UnitId(id)))
        .count()
}

/// A rifle squad 28 m from an enemy squad that an own heavy MG suppresses
/// opens fire and attack-moves through the enemy's position. Returns
/// whether it charged, its fastest step while charging (meters per tick),
/// the rounds it fired while running and whether it lay down or soldiers
/// threw themselves down while charging.
fn charge(seed: u64) -> (bool, Fixed, usize, bool) {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[
            (P0, SQUAD, 0, -32, HOLD),
            (P0, HMG, 95, -60, FIRE),
            (P1, ENEMY, 0, -60, HOLD),
        ],
    );
    // The MG suppresses the enemy first.
    for _ in 0..40 {
        sim.step(&[]);
    }
    sim.step(&[
        cmd(
            P0,
            Command::SetFireMode {
                units: vec![UnitId(0)],
                mode: FIRE,
            },
        ),
        attack_move(&[0], 0, -90),
    ]);
    let (mut charged, mut fastest, mut fired, mut lay) = (false, Fixed::ZERO, 0, false);
    let mut last = position(&sim, 0);
    for _ in 0..20 * 15 {
        sim.step(&[]);
        let now = position(&sim, 0);
        let step = (now - last).length();
        last = now;
        let t = sim.squad_tactics(UnitId(0)).unwrap();
        if t.assault {
            charged = true;
            assert!(t.bounding.is_none(), "the whole squad charges");
            lay |= stance(&sim, 0) == Some(Stance::Prone) || down(&sim, 0) > 0;
            if t.running {
                fastest = fastest.max(step);
                fired += shots(&sim, 0);
            }
        }
    }
    (charged, fastest, fired, lay)
}

#[test]
fn a_charging_squad_runs_upright_and_does_not_fire() {
    let mut charges = 0;
    for seed in 1..=3 {
        let (charged, fastest, fired, lay) = charge(seed);
        println!(
            "seed {seed}: charged {charged}, fastest step {fastest} m, \
             rounds while running {fired}, lay down {lay}"
        );
        if !charged {
            continue;
        }
        charges += 1;
        // 300 % of the walking speed (2.2 m/s game: 0.11 m per tick).
        assert!(fastest > m(25) / 100, "fastest step {fastest} m");
        assert_eq!(fired, 0, "running men fired");
        assert!(!lay, "the charge went to ground");
    }
    assert!(charges >= 2, "only {charges} of 3 squads charged");
}

/// Fire superiority first: a squad charges only an enemy squad that is
/// suppressed (`fire_teams.advance_at`, 30). Here its own first volleys
/// at 28 m do that.
#[test]
fn charges_only_after_fire_superiority() {
    let mut sim = sim_with(
        map_with(&[]),
        1,
        &[(P0, SQUAD, 0, -32, FIRE), (P1, ENEMY, 0, -60, HOLD)],
    );
    let suppression = |sim: &Sim| sim.squad_tactics(UnitId(1)).map_or(0, |t| t.suppression);
    // The squad decides on the suppression before this tick's rounds and
    // recovery.
    let mut enemy = suppression(&sim);
    let mut charged = false;
    sim.step(&[attack_move(&[0], 0, -90)]);
    for _ in 0..20 * 10 {
        let assault = sim.squad_tactics(UnitId(0)).unwrap().assault;
        if !charged {
            assert!(
                !assault || enemy >= 30,
                "charged an enemy suppressed {enemy}"
            );
        }
        charged |= assault;
        enemy = suppression(&sim);
        sim.step(&[]);
    }
    assert!(charged, "the squad never charged");
}

/// On attack-move the fire teams bound from the start; out of contact and
/// not under fire their rushes do not run (that would spend the squad
/// before it meets the enemy). Running in contact and exhaustion: the unit
/// tests of `sim/tactics.rs`.
#[test]
fn bounds_out_of_contact_do_not_run() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, FIRE)]);
    sim.step(&[attack_move(&[0], 0, -150)]);
    let mut bounded = 0;
    for _ in 0..20 * 60 {
        sim.step(&[]);
        let s = sim.squad_tactics(UnitId(0)).unwrap();
        bounded += u32::from(s.bounding.is_some());
        assert!(!s.running && !s.tired);
    }
    assert!(bounded > 100, "bounded {bounded} ticks");
}

/// A squad at the origin (stance `order`, wall objects `objects`), one
/// 8 cm shell aimed 9 m in front of it, or, `moving`, at its way west.
/// Returns the soldiers down in the tick of the impact and the soldiers at
/// their cover spot then (`None` if the shell landed farther than 12 m
/// from every soldier).
fn shell_near(
    objects: &[String],
    order: StanceOrder,
    moving: bool,
    seed: u64,
) -> Option<(usize, usize)> {
    let mut sim = sim_with(
        map_with(objects),
        seed,
        &[(P1, MORTAR, 0, -150, HOLD), (P0, SQUAD, 0, 0, HOLD)],
    );
    sim.step(&[set_stance(P0, 1, order)]);
    for _ in 0..60 {
        sim.step(&[]);
    }
    if moving {
        sim.step(&[cmd(
            P0,
            Command::MoveTo {
                units: vec![UnitId(1)],
                target: Vec2::from_meters(-200, 0),
            },
        )]);
    }
    sim.step(&[cmd(
        P1,
        Command::Barrage {
            units: vec![UnitId(0)],
            target: if moving {
                // Prompt 231: aim where the moving squad arrives during mortar
                // setup + flight (~10 s) at the faster 3.89 m/s upright / 2.33 m/s
                // kneeling pace.
                if matches!(order, StanceOrder::Fixed(Stance::Kneel)) {
                    Vec2::from_meters(-23, -3)
                } else {
                    Vec2::from_meters(-38, -3)
                }
            } else {
                Vec2::from_meters(0, -9)
            },
            radius_m: 0,
            salvos: 1,
            smoke: false,
        },
    )]);
    for _ in 0..20 * 60 {
        sim.step(&[]);
        let hit = sim.events().iter().find_map(|e| match e {
            CombatEvent::Hit { shooter, at, .. } if *shooter == UnitId(0) => Some(at.ground()),
            _ => None,
        });
        if let Some(at) = hit {
            let snap = sim.snapshot();
            let near = snap
                .soldiers
                .iter()
                .filter(|s| s.unit == UnitId(1) && s.hp > 0 && s.knocked == 0)
                .filter(|s| (s.position - at).length() <= m(12))
                .count();
            // Only shells that came close count.
            if near == 0 {
                return None;
            }
            let covered = snap
                .soldiers
                .iter()
                .filter(|s| s.unit == UnitId(1) && s.cover.is_some())
                .count();
            return Some((down(&sim, 1), covered));
        }
    }
    None
}

fn wall() -> Vec<String> {
    (-5..=5)
        .map(|k| format!("env_wall_stone,Segment,{},-4000,90", k * 3000))
        .collect()
}

#[test]
fn a_mortar_shell_in_the_open_throws_soldiers_down_behind_a_wall_they_stay() {
    let (mut open, mut walled, mut fixed) = (Vec::new(), Vec::new(), Vec::new());
    for seed in 1..=6 {
        // On the move in the open; standing behind the wall at their
        // cover spots; on the move with a fixed stance.
        open.extend(shell_near(&[], StanceOrder::Auto, true, seed));
        walled.extend(shell_near(&wall(), StanceOrder::Auto, false, seed));
        fixed.extend(shell_near(
            &[],
            StanceOrder::Fixed(Stance::Kneel),
            true,
            seed,
        ));
    }
    println!("(down, at cover) open {open:?}, behind a wall {walled:?}, fixed kneel {fixed:?}");
    assert!(open.len() >= 3 && walled.len() >= 3 && fixed.len() >= 3);
    assert!(open.iter().all(|&(d, _)| d >= 1), "open {open:?}");
    // Behind the wall: everybody at his spot, nobody thrown down.
    assert!(walled.iter().all(|&(d, c)| d == 0 && c >= 5), "{walled:?}");
    // A fixed stance is the player's: nobody throws himself down.
    assert!(fixed.iter().all(|&(d, _)| d == 0), "fixed {fixed:?}");
}

/// A squad marching west past a heavy MG 70 m south (holding fire, so it
/// does not stop to fight). Returns the most soldiers down at once and
/// whether all were up again 10 s after the MG fell silent.
fn mg_burst(order: StanceOrder, seed: u64) -> (usize, bool) {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[(P0, SQUAD, 40, 0, HOLD), (P1, HMG, 0, 70, FIRE)],
    );
    sim.step(&[set_stance(P0, 0, order), move_to(&[0], -200, 0)]);
    let mut most = 0;
    for _ in 0..20 * 10 {
        sim.step(&[]);
        most = most.max(down(&sim, 0));
    }
    sim.step(&[cmd(
        P1,
        Command::SetFireMode {
            units: vec![UnitId(1)],
            mode: HOLD,
        },
    )]);
    for _ in 0..20 * 10 {
        sim.step(&[]);
    }
    (most, down(&sim, 0) == 0)
}

#[test]
fn machine_gun_bursts_throw_marching_soldiers_down_for_a_short_time() {
    for seed in 1..=3 {
        let (most, up) = mg_burst(StanceOrder::Auto, seed);
        let (fixed, _) = mg_burst(StanceOrder::Fixed(Stance::Stand), seed);
        println!("seed {seed}: most down {most}, up again {up}, fixed stance {fixed}");
        assert!(most >= 2, "seed {seed}: {most} down");
        assert!(up, "seed {seed}: still down");
        assert_eq!(fixed, 0, "seed {seed}: a fixed stance threw men down");
    }
}

#[test]
fn rush_and_drop_are_deterministic() {
    let run = || {
        let mut sim = sim_with(
            map_with(&[]),
            9,
            &[
                (P0, SQUAD, 0, 0, FIRE),
                (P0, HMG, 60, -20, FIRE),
                (P1, ENEMY, 0, -60, FIRE),
                (P1, MORTAR, -40, -120, FIRE),
            ],
        );
        sim.step(&[attack_move(&[0], 0, -90)]);
        for _ in 0..20 * 40 {
            sim.step(&[]);
        }
        sim.state_hash()
    };
    assert_eq!(run(), run());
}
