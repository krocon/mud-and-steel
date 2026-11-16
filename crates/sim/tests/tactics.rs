//! Infantry tactics (prompt 047): fire teams bound alternately from cover
//! to cover, squads return fire on the march, suppression pins them down,
//! close firefights lying down, blind fire, mortar teams engaging spotted
//! targets, determinism and snapshots. Hand-made maps, catalog units, fixed
//! seeds.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, Command, Engage, FireMode, Fixed, Formation, Map,
    PlayerCommand, PlayerId, Scale, Sim, SimConfig, Spawn, Stance, TerrainRules, UnitId, Vec2,
    soldier_flags, unit_types_from_catalog,
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
        name: "tactics_test".into(),
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

fn m(v: i32) -> Fixed {
    Fixed::from_num(v)
}

fn position(sim: &Sim, id: u32) -> Vec2 {
    sim.snapshot()
        .units
        .iter()
        .find(|u| u.id == UnitId(id))
        .unwrap()
        .position
}

/// Soldier positions of squad `id` by index.
fn soldiers(sim: &Sim, id: u32) -> Vec<sim::SoldierSnapshot> {
    sim.snapshot()
        .soldiers
        .into_iter()
        .filter(|s| s.unit == UnitId(id))
        .collect()
}

#[test]
fn fire_teams_bound_alternately_on_attack_move() {
    let mut sim = sim_with(map_with(&[]), 1, &[(P0, SQUAD, 0, 0, FIRE)]);
    sim.step(&[attack_move(&[0], 0, -120)]);
    let mut phases = Vec::new();
    // Ticks in which one team moved while the other stood.
    let mut split = [0u32; 2];
    let mut before = soldiers(&sim, 0);
    for _ in 0..20 * 120 {
        sim.step(&[]);
        let now = soldiers(&sim, 0);
        let t = sim.squad_tactics(UnitId(0)).unwrap();
        if let Some(team) = t.bounding
            && phases.last() != Some(&team)
        {
            phases.push(team);
        }
        if let Some(team) = t.bounding {
            let moved = |k: usize| (now[k].position - before[k].position).length() > m(1) / 50;
            let team_moved = |first: usize, last: usize| (first..last).any(moved);
            let (base, maneuver) = (team_moved(0, 4), team_moved(4, 9));
            if team == 1 && maneuver && !base {
                split[1] += 1;
            }
            if team == 0 && base && !maneuver {
                split[0] += 1;
            }
        }
        before = now;
        if (position(&sim, 0) - Vec2::from_meters(0, -120)).length() < m(2)
            && sim.squad_tactics(UnitId(0)).unwrap().bounding.is_none()
        {
            break;
        }
    }
    println!("phases {phases:?}, split ticks {split:?}");
    // The base of fire takes position, the maneuver team makes the first
    // bound, then the teams alternate.
    assert!(phases.len() >= 5, "phases {phases:?}");
    assert_eq!(phases[..2], [0, 1]);
    assert!(phases.windows(2).all(|w| w[0] != w[1]));
    assert!(
        split[0] > 20 && split[1] > 20,
        "one team moved while the other held: {split:?}"
    );
    assert!(
        (position(&sim, 0) - Vec2::from_meters(0, -120)).length() < m(2),
        "the squad reached its target: {:?}",
        position(&sim, 0)
    );
    // All soldiers stay within the squad's reach on the way.
    for s in soldiers(&sim, 0) {
        assert!((s.position - position(&sim, 0)).length() < m(15));
    }
}

#[test]
fn bounds_end_in_cover() {
    // Rocks across the route about 22 m and 45 m ahead; the moving team
    // halts behind them.
    let rocks: Vec<String> = [-22, -45]
        .iter()
        .flat_map(|&y| {
            (-3..=3).map(move |k| format!("env_rock,Medium,{},{},0", k * 2500, y * 1000))
        })
        .collect();
    let mut sim = sim_with(map_with(&rocks), 2, &[(P0, SQUAD, 0, 0, FIRE)]);
    sim.step(&[attack_move(&[0], 0, -80)]);
    let mut covered_halts = 0;
    let mut last_bound = None;
    for _ in 0..20 * 90 {
        sim.step(&[]);
        let t = sim.squad_tactics(UnitId(0)).unwrap();
        // At the end of every bound: soldiers of the team that just moved
        // with a cover object.
        if let Some(team) = last_bound
            && t.bounding != last_bound
        {
            let flags = sim.soldier_tactics(UnitId(0));
            let covered = soldiers(&sim, 0)
                .iter()
                .enumerate()
                .filter(|(k, s)| {
                    let maneuver = flags[*k] & soldier_flags::MANEUVER != 0;
                    (u8::from(maneuver) == team) && s.cover.is_some()
                })
                .count();
            if covered >= 3 {
                covered_halts += 1;
            }
        }
        last_bound = t.bounding;
    }
    assert!(
        covered_halts >= 2,
        "bounds ended in cover {covered_halts} times"
    );
}

/// A rifle squad at the origin under fire from an MG team 150 m north for
/// 8 s; returns the highest suppression, whether it was pinned and lay
/// prone, and the suppression and pinning 30 s after the MG stopped.
fn under_mg_fire(seed: u64) -> (u32, bool, bool, u32, bool) {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[(P0, SQUAD, 0, 0, HOLD), (P1, MG, 0, -150, FIRE)],
    );
    let (mut highest, mut pinned, mut prone) = (0, false, false);
    for _ in 0..20 * 8 {
        sim.step(&[]);
        let t = sim.squad_tactics(UnitId(0)).unwrap();
        highest = highest.max(t.suppression);
        pinned |= t.pinned;
        let stance = sim.snapshot().units[0].stance;
        prone |= t.pinned && stance == Some(Stance::Prone);
    }
    sim.step(&[cmd(
        P1,
        Command::SetFireMode {
            units: vec![UnitId(1)],
            mode: HOLD,
        },
    )]);
    for _ in 0..20 * 30 {
        sim.step(&[]);
    }
    assert!(sim.unit_hp(UnitId(0)).unwrap() > 0, "the squad survived");
    let after = sim.squad_tactics(UnitId(0)).unwrap();
    (highest, pinned, prone, after.suppression, after.pinned)
}

#[test]
fn machine_gun_fire_pins_a_squad_down_and_it_recovers() {
    let (highest, pinned, prone, after, still_pinned) = under_mg_fire(3);
    println!("highest {highest}, pinned {pinned}, prone {prone}, after {after}");
    assert!(highest >= 60, "suppression {highest}");
    assert!(pinned && prone, "the squad went to ground");
    assert_eq!(after, 0, "suppression wears off");
    assert!(!still_pinned);
}

/// How far a squad holding fire gets in 30 s marching west past an MG team
/// (or without one).
fn march_distance(with_mg: bool, seed: u64) -> Fixed {
    let mut spawns = vec![(P0, SQUAD, 100, 0, HOLD)];
    if with_mg {
        spawns.push((P1, MG, 0, -120, FIRE));
    }
    let mut sim = sim_with(map_with(&[]), seed, &spawns);
    sim.step(&[move_to(&[0], -300, 0)]);
    for _ in 0..20 * 30 {
        sim.step(&[]);
    }
    (position(&sim, 0) - Vec2::from_meters(100, 0)).length()
}

#[test]
fn suppression_slows_a_squad_down() {
    let free = march_distance(false, 4);
    let under_fire = march_distance(true, 4);
    println!("free {free}, under fire {under_fire}");
    assert!(
        under_fire * 2 < free,
        "free {free}, under fire {under_fire}"
    );
}

/// Rounds and wounding hits of a rifle squad (fire at will) on an enemy
/// squad 120 m away within 10 s, with or without an MG team suppressing
/// the shooters from the side.
fn hits_of_shooters(suppressed: bool, seed: u64) -> (u32, u32) {
    let mut spawns = vec![(P0, SQUAD, 0, 0, FIRE), (P1, SQUAD, 0, -120, HOLD)];
    if suppressed {
        spawns.push((P1, MG, 150, 0, FIRE));
    }
    let mut sim = sim_with(map_with(&[]), seed, &spawns);
    let (mut shots, mut hits) = (0, 0);
    for _ in 0..20 * 10 {
        sim.step(&[]);
        for e in sim.events() {
            match e {
                CombatEvent::Shot { unit, .. } if *unit == UnitId(0) => shots += 1,
                CombatEvent::Hit {
                    kind: sim::HitKind::Unit(u),
                    shooter,
                    damage,
                    ..
                } if *u == UnitId(1) && *shooter == UnitId(0) && *damage > 0 => hits += 1,
                _ => {}
            }
        }
    }
    (shots, hits)
}

#[test]
fn suppressed_squads_hit_less() {
    let sum = |suppressed: bool| {
        (1..=4)
            .map(|s| hits_of_shooters(suppressed, s))
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
    };
    let (calm_shots, calm_hits) = sum(false);
    let (shots, hits) = sum(true);
    println!("calm {calm_hits}/{calm_shots}, suppressed {hits}/{shots}");
    assert!(calm_hits >= 10 && shots >= 20, "too few rounds");
    // Hits per round fall to less than half, and pinned soldiers fire less.
    assert!(
        hits * calm_shots * 2 < calm_hits * shots,
        "calm {calm_hits}/{calm_shots}, suppressed {hits}/{shots}"
    );
}

/// A squad marching past an enemy truck 60 m away (it can hurt it, slowly);
/// returns the ticks in which it stood (halted with a route) and the rounds
/// it fired.
fn march_past(mode: FireMode, seed: u64) -> (u32, u32) {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[
            (P0, SQUAD, 150, 0, mode),
            (P1, "de_opel_blitz", 0, -60, HOLD),
        ],
    );
    sim.step(&[move_to(&[0], -150, 0)]);
    let (mut halted, mut shots) = (0, 0);
    let mut last = position(&sim, 0);
    for _ in 0..20 * 40 {
        sim.step(&[]);
        let p = position(&sim, 0);
        let moving_order = sim.snapshot().units[0].target.is_some();
        if moving_order && (p - last).length() == Fixed::ZERO {
            halted += 1;
        }
        last = p;
        shots += sim
            .events()
            .iter()
            .filter(|e| matches!(e, CombatEvent::Shot { unit, .. } if *unit == UnitId(0)))
            .count() as u32;
    }
    (halted, shots)
}

#[test]
fn squads_return_fire_on_the_march() {
    let (halted, shots) = march_past(FIRE, 5);
    let (halted_hold, shots_hold) = march_past(HOLD, 5);
    println!("fire at will: halted {halted}, shots {shots}; hold: {halted_hold}, {shots_hold}");
    assert!(shots > 20, "returned fire: {shots}");
    assert!(halted > 40, "halted to fight: {halted}");
    assert_eq!(shots_hold, 0);
    assert!(halted_hold < halted / 2);
}

#[test]
fn close_firefights_are_fought_lying_down() {
    let mut sim = sim_with(
        map_with(&[]),
        6,
        &[(P0, SQUAD, 0, 0, FIRE), (P1, SQUAD, 0, -40, FIRE)],
    );
    let mut prone = false;
    for _ in 0..20 * 5 {
        sim.step(&[]);
        let t = sim.squad_tactics(UnitId(0)).unwrap();
        prone |= t.close && sim.snapshot().units[0].stance == Some(Stance::Prone);
    }
    assert!(prone, "the squads lay down");
}

#[test]
fn squads_fire_blind_into_smoke() {
    // P0: a rifle squad (holding fire) and a mortar with smoke rounds;
    // P1: a squad 100 m north of the riflemen. The mortar lays smoke
    // between them; just before it lands the riflemen open fire, then
    // keep firing blindly into the smoke where they last saw the enemy.
    let mut sim = sim_with(
        map_with(&[]),
        7,
        &[
            (P0, SQUAD, 0, 0, HOLD),
            (P0, "de_inf_mortar_medium", 0, 150, HOLD),
            (P1, SQUAD, 0, -100, HOLD),
        ],
    );
    sim.step(&[cmd(
        P0,
        Command::Barrage {
            units: vec![UnitId(1)],
            target: Vec2::from_meters(0, -60),
            radius_m: 0,
            salvos: 3,
            smoke: true,
        },
    )]);
    let mut opened = false;
    let (mut blind_shots, mut hidden) = (0, false);
    for _ in 0..20 * 60 {
        let landing = sim
            .snapshot()
            .shells
            .iter()
            .any(|s| s.smoke && s.ticks_left <= 15);
        if landing && !opened {
            opened = true;
            sim.step(&[cmd(
                P0,
                Command::SetFireMode {
                    units: vec![UnitId(0)],
                    mode: FIRE,
                },
            )]);
            continue;
        }
        sim.step(&[]);
        hidden |= !sim.snapshot().smoke.is_empty() && !sim.visible_to(P0, UnitId(2));
        if sim.squad_tactics(UnitId(0)).unwrap().blind_fire {
            blind_shots += sim
                .events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::Shot { unit, .. } if *unit == UnitId(0)))
                .count();
        }
    }
    println!("opened {opened}, hidden {hidden}, blind shots {blind_shots}");
    assert!(opened && hidden, "the smoke hid the enemy");
    assert!(blind_shots > 0, "no blind fire into the smoke");
}

/// A light mortar team attack-moves north towards an enemy squad; returns
/// whether it halted for it and how many shells it fired.
fn mortar_on_attack_move(seed: u64) -> (bool, usize) {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[
            (P0, "de_inf_mortar_light", 0, 60, FIRE),
            (P1, SQUAD, 0, -60, HOLD),
        ],
    );
    sim.step(&[attack_move(&[0], 0, -40)]);
    let (mut halted, mut shells) = (false, 0);
    for _ in 0..20 * 60 {
        sim.step(&[]);
        if let Some(sim::DrillView::Advance(_, true)) = sim.drill_view(UnitId(0)) {
            halted = true;
        }
        shells += indirect_shots(&sim, 0);
    }
    (halted, shells)
}

fn indirect_shots(sim: &Sim, unit: u32) -> usize {
    sim.events()
        .iter()
        .filter(|e| {
            matches!(e, CombatEvent::Shot { unit: u, weapon, .. }
                if *u == UnitId(unit) && *weapon == sim::indirect::INDIRECT_WEAPON)
        })
        .count()
}

#[test]
fn mortar_teams_halt_and_set_up_on_attack_move() {
    let (halted, shells) = mortar_on_attack_move(8);
    println!("halted {halted}, shells {shells}");
    assert!(halted, "the mortar team halted on its attack-move");
    assert!(shells > 0, "and fired at the squad");
}

/// Shells of a medium mortar team (fire at will) hidden behind a row of
/// houses at an enemy squad 250 m away, with or without a friendly recon
/// squad that sees the enemy.
fn mortar_shells_on_spotted(with_recon: bool) -> usize {
    let houses: Vec<String> = (-3..=3)
        .map(|k| format!("env_house_farm,Intact,{},140000,0", k * 12000))
        .collect();
    let mut spawns = vec![
        (P0, "de_inf_mortar_medium", 0, 150, FIRE),
        (P1, SQUAD, 0, -100, HOLD),
    ];
    if with_recon {
        spawns.push((P0, "de_inf_recon", 0, 40, HOLD));
    }
    let mut sim = sim_with(map_with(&houses), 8, &spawns);
    let mut shells = 0;
    for _ in 0..20 * 40 {
        sim.step(&[]);
        shells += indirect_shots(&sim, 0);
    }
    shells
}

#[test]
fn mortar_teams_engage_targets_their_player_spotted() {
    let alone = mortar_shells_on_spotted(false);
    let spotted = mortar_shells_on_spotted(true);
    println!("alone {alone}, with recon {spotted}");
    assert_eq!(alone, 0, "the mortar team sees nothing behind the houses");
    assert!(spotted > 0, "it fires at what the recon squad spots");
}

/// A bounding fight of two squads and an MG team.
fn fight(seed: u64) -> Sim {
    let mut sim = sim_with(
        map_with(&[]),
        seed,
        &[
            (P0, SQUAD, 0, 100, FIRE),
            (P0, SQUAD, 30, 100, FIRE),
            (P1, SQUAD, 0, -100, FIRE),
            (P1, MG, 40, -100, FIRE),
        ],
    );
    sim.step(&[attack_move(&[0, 1], 0, -60)]);
    for _ in 0..20 * 30 {
        sim.step(&[]);
    }
    sim
}

#[test]
fn same_fight_gives_same_hash() {
    let a = fight(9);
    let b = fight(9);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_ne!(a.state_hash(), fight(10).state_hash());
}

#[test]
fn weapons_jam_now_and_then() {
    // Two lines of three squads, 150 m apart, holding their ground.
    let mut spawns = Vec::new();
    for k in 0..3 {
        spawns.push((P0, SQUAD, k * 30 - 30, 75, FIRE));
        spawns.push((P1, "su_inf_rifle_squad", k * 30 - 30, -75, FIRE));
    }
    let mut sim = sim_with(map_with(&[]), 11, &spawns);
    let (mut jams, mut shots) = (0, 0);
    let mut was = [0u8; 6 * 16];
    for _ in 0..20 * 90 {
        sim.step(&[]);
        shots += sim
            .events()
            .iter()
            .filter(|e| matches!(e, CombatEvent::Shot { .. }))
            .count();
        for id in 0..6u32 {
            for (k, f) in sim.soldier_tactics(UnitId(id)).iter().enumerate() {
                let jammed = f & soldier_flags::JAMMED;
                let slot = id as usize * 16 + k;
                if jammed != 0 && was[slot] == 0 {
                    jams += 1;
                }
                was[slot] = jammed;
            }
        }
    }
    println!("jams {jams} in {shots} rounds");
    assert!(jams > 0, "no weapon ever jammed");
    assert!(jams * 100 < shots, "jams are rare: {jams} in {shots}");
}
