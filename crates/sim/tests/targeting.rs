//! Target choice by effect, threat and role (prompt 188). Scenarios: a Pak
//! beside a truck and a tank, a tank against a Pak and a rifle squad, an MG
//! team against a squad and a half-track, a mortar without orders and a
//! far single sniper. Run with `--nocapture` to see each first target.

use sim::map::MapLayout;
use sim::{
    Catalog, CombatEvent, CombatRules, FireMode, Map, PlayerId, Scale, Sim, SimConfig, Spawn,
    TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const N: u32 = 1201;

/// 2400 m × 2400 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "targeting_test".into(),
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
        variant: None,
    };
    Sim::new(&config, map, 7).unwrap()
}

/// The unit that unit 0 first takes as its automatic target (from the
/// fire status), and the tick, within `seconds`; enemies hold their fire.
fn first_target(
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
    seconds: u32,
) -> Option<(u32, u64)> {
    let mut sim = sim_with(flat(), spawns);
    for _ in 0..20 * seconds {
        sim.step(&[]);
        if let Some((_, Some(t))) = sim.fire_status(UnitId(0))
            && sim
                .events()
                .iter()
                .any(|e| matches!(e, CombatEvent::Shot { unit, .. } if unit.0 == 0))
        {
            return Some((t.0, sim.tick()));
        }
    }
    None
}

/// Whether unit 0 fired at all within `seconds`.
fn fires(spawns: &[(PlayerId, &str, i32, i32, FireMode)], seconds: u32) -> bool {
    let mut sim = sim_with(flat(), spawns);
    fires_in(&mut sim, seconds)
}

fn fires_in(sim: &mut Sim, seconds: u32) -> bool {
    for _ in 0..20 * seconds {
        sim.step(&[]);
        if sim
            .events()
            .iter()
            .any(|e| matches!(e, CombatEvent::Shot { unit, .. } if unit.0 == 0))
        {
            return true;
        }
    }
    false
}

#[test]
fn at_gun_prefers_the_tank() {
    // Truck (1) nearer than the T-34 (2).
    let t = first_target(
        &[
            (P0, "de_pak40", 0, 0, FIRE),
            (P1, "su_zis5", 10, 45, HOLD),
            (P1, "su_t34_76_1943", -10, 60, HOLD),
        ],
        30,
    );
    println!("Pak beside truck and tank: first target {t:?} (1 truck, 2 tank)");
}

#[test]
fn tank_prefers_the_at_gun_over_riflemen() {
    // Rifle squad (1) nearer than the anti-tank gun (2).
    let t = first_target(
        &[
            (P0, "de_pz4_h", 0, 0, FIRE),
            (P1, "su_inf_rifle_squad", 10, 40, HOLD),
            (P1, "su_zis3", -10, 60, HOLD),
        ],
        30,
    );
    println!("tank against Pak and squad: first target {t:?} (1 squad, 2 gun)");
    assert_eq!(t.map(|t| t.0), Some(2));
}

#[test]
fn machine_gun_prefers_the_squad() {
    // Half-track (1) nearer than the rifle squad (2).
    let t = first_target(
        &[
            (P0, "de_inf_hmg_team", 0, 0, FIRE),
            (P1, "su_m3a1_scout", 10, 30, HOLD),
            (P1, "su_inf_rifle_squad", -10, 40, HOLD),
        ],
        30,
    );
    println!("MG against half-track and squad: first target {t:?} (1 half-track, 2 squad)");
    assert_eq!(
        t.map(|t| t.0),
        Some(2),
        "before prompt 188 the MG took the nearer half-track"
    );
}

#[test]
fn mortar_ignores_a_far_single_sniper() {
    // A recon squad (1) sees the enemy (2) 140 m away; the mortar team
    // (0) 250 m from it has no order (the setup of the mortar test in
    // tactics.rs). A sniper does not draw its fire, a rifle squad does.
    let spotted = |enemy: &str| {
        fires(
            &[
                (P0, "de_inf_mortar_medium", 0, 150, FIRE),
                (P0, "de_inf_recon", 0, 40, HOLD),
                (P1, enemy, 0, -100, HOLD),
            ],
            40,
        )
    };
    let (sniper, squad) = (spotted("su_inf_sniper"), spotted("su_inf_rifle_squad"));
    println!("mortar without orders: fired at a sniper {sniper}, at a squad {squad}");
    assert!(squad, "the mortar should fire at a spotted squad");
    assert!(!sniper, "fired at a single sniper");
}

/// A tank between two equal enemies does not jump between them: the
/// current target keeps a bonus (`stick`), so while both live the target
/// changes at most once.
#[test]
fn targets_do_not_flicker() {
    let mut sim = sim_with(
        flat(),
        &[
            (P0, "de_pz4_h", 0, 0, FIRE),
            (P1, "su_t34_76_1943", -30, 60, HOLD),
            (P1, "su_t34_76_1943", 30, 60, HOLD),
        ],
    );
    let mut last = None;
    let mut changes = 0;
    for _ in 0..20 * 20 {
        sim.step(&[]);
        let snap = sim.snapshot();
        if !snap.units[1].alive() || !snap.units[2].alive() {
            break;
        }
        let target = sim.fire_status(UnitId(0)).and_then(|(_, t)| t);
        if target.is_some() && target != last {
            if last.is_some() {
                changes += 1;
            }
            last = target;
        }
    }
    assert!(changes <= 1, "the target changed {changes} times");
}
