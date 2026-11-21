//! Causes of losses in the match statistics (prompt 179): a truck shot by
//! a tank, a truck on an anti-tank mine, a squad under mortar fire, a
//! squad run over by a tank.

use sim::map::MapLayout;
use sim::stats::{Cause, MatchStats};
use sim::{
    Catalog, CombatRules, Command, FireMode, Map, PlayerCommand, PlayerId, Scale, Sim, SimConfig,
    Spawn, TerrainRules, UnitId, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);
const FIRE: FireMode = FireMode::FireAtWill;
const HOLD: FireMode = FireMode::HoldFire;
const N: u32 = 401;

/// 800 m × 800 m flat map around the origin; `terrain` gives the terrain
/// id of a sample from its position in meters, `objects` are CSV lines.
fn map_with(terrain: impl Fn(i32, i32) -> u8, objects: &[String]) -> Arc<Map> {
    let layout = MapLayout {
        name: "loss_test".into(),
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
    Sim::new(&config, map.clone(), 7).unwrap()
}

/// Runs `sim` with `orders` at the start for up to `seconds` and returns
/// player 1's losses.
fn losses(
    spawns: &[(PlayerId, &str, i32, i32, FireMode)],
    objects: &[String],
    orders: Vec<PlayerCommand>,
    seconds: u32,
) -> Vec<sim::stats::Loss> {
    let map = map_with(|_, _| 0, objects);
    let mut sim = sim_with(map, spawns);
    let mut stats = MatchStats::new(&config_of(&sim, spawns));
    stats.observe(&sim);
    sim.step(&orders);
    stats.observe(&sim);
    for _ in 0..20 * seconds {
        sim.step(&[]);
        stats.observe(&sim);
        let p1 = stats.players().iter().find(|p| p.player == P1).unwrap();
        if !p1.losses.is_empty() {
            return p1.losses.clone();
        }
    }
    Vec::new()
}

/// The config of `sim_with` again (MatchStats needs the players and types).
fn config_of(sim: &Sim, spawns: &[(PlayerId, &str, i32, i32, FireMode)]) -> SimConfig {
    SimConfig {
        tick_hz: 20,
        unit_types: sim.unit_types().to_vec(),
        spawns: spawns
            .iter()
            .map(|&(owner, unit, x, y, fire_mode)| Spawn {
                owner,
                unit_type: unit.into(),
                position: Vec2::from_meters(x, y),
                fire_mode,
            })
            .collect(),
        map: sim.map().reference(),
        terrain: TerrainRules::builtin().unwrap(),
        combat: CombatRules::builtin().unwrap(),
        vision: sim::VisionRules::builtin().unwrap(),
        infantry: sim::InfantryRules::builtin().unwrap(),
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
    }
}

#[test]
fn direct_fire_is_recorded_with_the_shooter() {
    // A second truck of player 1 keeps watching after the first is gone
    // (the player's sight is that of its living units).
    let l = losses(
        &[
            (P0, "de_pz4_h", 0, 0, FIRE),
            (P1, "su_zis5", 0, 60, HOLD),
            (P1, "su_zis5", 0, 75, HOLD),
        ],
        &[],
        Vec::new(),
        60,
    );
    let loss = l.first().expect("the truck was not destroyed");
    assert_eq!(loss.cause, Cause::Direct);
    assert!(loss.by_type.is_some());
    assert!(loss.shooter_seen, "the truck's player saw the tank");
    assert!(loss.face.is_some());
}

#[test]
fn mines_are_recorded() {
    let field = vec!["fort_mine_at,Mine,0,0,0".to_string()];
    let order = PlayerCommand::new(
        P1,
        Command::MoveTo {
            units: vec![UnitId(0)],
            target: Vec2::from_meters(0, 60),
        },
    );
    let l = losses(
        &[
            (P1, "su_zis5", 0, -60, HOLD),
            (P0, "de_inf_rifle_squad", -150, 0, HOLD),
        ],
        &field,
        vec![order],
        120,
    );
    let loss = l.first().expect("the truck did not die on the mines");
    assert_eq!(loss.cause, Cause::Mine);
}

#[test]
fn mortar_fire_is_indirect() {
    let l = losses(
        &[
            (P0, "de_inf_mortar_medium", 0, 150, FIRE),
            (P0, "de_inf_recon", 0, 40, HOLD),
            (P1, "su_zis5", 0, -100, HOLD),
        ],
        &[],
        Vec::new(),
        240,
    );
    let loss = l.first().expect("the mortar destroyed nothing");
    assert_eq!(loss.cause, Cause::Indirect);
}

/// The report groups losses and finds a hint for every cause; unseen
/// shooters and hits from the side get their own hints.
#[test]
fn report_ranks_causes_and_finds_hints() {
    use sim::combat::Face;
    use sim::stats::{Loss, report};
    let loss = |cause, by_type, value, face, seen| Loss {
        tick: 0,
        unit_type: 0,
        position: Vec2::ZERO,
        cause,
        by_type,
        face,
        shooter_seen: seen,
        value,
    };
    for cause in Cause::ALL {
        let lines = report(&[loss(cause, Some(1), 100, None, true)], 3);
        assert!(!lines[0].hint.is_empty(), "no hint for {cause:?}");
    }
    let lines = report(
        &[
            loss(Cause::Direct, Some(1), 400, Some(Face::Front), false),
            loss(Cause::Direct, Some(1), 400, Some(Face::Side), false),
            loss(Cause::Mine, None, 300, None, false),
            loss(Cause::Direct, Some(2), 100, Some(Face::Rear), true),
        ],
        3,
    );
    assert_eq!(lines.len(), 3);
    assert_eq!(
        (lines[0].cause, lines[0].count, lines[0].value),
        (Cause::Direct, 2, 800)
    );
    assert_eq!(lines[0].hint, "REPORT_HINT_UNSEEN");
    assert_eq!(lines[1].hint, "REPORT_HINT_MINE");
    assert_eq!(lines[2].hint, "REPORT_HINT_FLANK");
}
