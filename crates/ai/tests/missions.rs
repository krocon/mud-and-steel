//! Every mission and the tutorial (prompt 039) can be won: an autopilot
//! plays the human player with the orders the objectives ask for, the
//! computer players of the mission play as in the game. Also checks that
//! a mission plays the same twice (state hash) and survives a snapshot.

use ai::{Ai, Difficulty, PlayerView};
use sim::mission::{Level, MISSIONS_DIR, POINTS_FILE, parse_mission_at};
use sim::{
    Command, Engage, Formation, Map, ObjectiveState, PlayerCommand, PlayerId, Sim, SimConfig,
    UnitId, Vec2,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SEED: u64 = 42;
/// Game time limit per mission (20 minutes).
const LIMIT_TICKS: u64 = 20 * 60 * 20;

fn maps_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/maps")
}

fn load_map(name: &str) -> Arc<Map> {
    let dir = maps_dir().join(name);
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    Arc::new(map)
}

/// Every mission file: (map, id, path), sorted.
fn mission_files() -> Vec<(String, String, PathBuf)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(maps_dir()).unwrap() {
        let dir = entry.unwrap().path();
        let Ok(files) = std::fs::read_dir(dir.join(MISSIONS_DIR)) else {
            continue;
        };
        let map = dir.file_name().unwrap().to_string_lossy().into_owned();
        for file in files {
            let path = file.unwrap().path();
            if path.extension().is_some_and(|e| e == "toml") {
                let id = path.file_stem().unwrap().to_string_lossy().into_owned();
                out.push((map.clone(), id, path));
            }
        }
    }
    out.sort();
    out
}

fn config_of(map: &Map, map_name: &str, id: &str, path: &Path) -> SimConfig {
    config_at(map, map_name, id, path, Level::Normal)
}

fn config_at(map: &Map, map_name: &str, id: &str, path: &Path, level: Level) -> SimConfig {
    let text = std::fs::read_to_string(path).unwrap();
    let points = std::fs::read_to_string(maps_dir().join(map_name).join(POINTS_FILE)).ok();
    parse_mission_at(
        id,
        &text,
        points.as_deref(),
        &sim::Catalog::builtin().unwrap(),
        &sim::Scale::builtin().unwrap(),
        &sim::TerrainRules::builtin().unwrap(),
        map,
        level,
    )
    .unwrap_or_else(|e| panic!("{map_name}/{id}: {e}"))
}

/// The orders of a player who follows the objectives (the self-test of
/// the game does the same through the HUD).
fn autopilot(sim: &Sim, player: PlayerId, tick: u64) -> Vec<Command> {
    let mut out: Vec<Command> = sim
        .mission_wanted_signals()
        .into_iter()
        .map(|name| Command::MissionSignal { name: name.into() })
        .collect();
    if !tick.is_multiple_of(100) {
        return out;
    }
    let snapshot = sim.snapshot();
    let own: Vec<UnitId> = snapshot
        .units
        .iter()
        .filter(|u| u.owner == player && u.alive())
        .map(|u| u.id)
        .collect();
    let Some(objective) = sim
        .mission_objectives()
        .into_iter()
        .find(|o| o.state == ObjectiveState::Active)
    else {
        return out;
    };
    if let Some(unit) = &objective.unit {
        let Some(t) = sim.unit_type_index(unit) else {
            return out;
        };
        let structures = sim.structures();
        if sim.build_options(player).iter().any(|o| o.unit_type == t) {
            let pending = structures
                .iter()
                .any(|s| s.owner == player && s.unit_type == t);
            let hq = structures.iter().find(|s| s.owner == player);
            if !pending && let Some(hq) = hq {
                let at = snapshot.units.iter().find(|u| u.id == hq.unit).unwrap();
                if let Some(p) = sim.find_site(player, t, at.position) {
                    out.push(Command::Build {
                        unit_type: t,
                        position: p,
                    });
                }
            }
        } else if let Some(s) = structures.iter().find(|s| {
            s.owner == player
                && s.completed()
                && s.queue.is_empty()
                && sim
                    .production_options(s.unit)
                    .iter()
                    .any(|o| o.unit_type == t && o.blocked.is_none())
        }) {
            out.push(Command::Produce {
                building: s.unit,
                unit_type: t,
            });
        }
    }
    if let Some((center, radius)) = objective.area {
        // The nearest enemy the player sees near the objective is attacked
        // first, like a player would.
        let mut seen = sim::Snapshot::default();
        sim.snapshot_into(Some(player), &mut seen);
        let reach = radius + sim::Fixed::from_num(300);
        let distance_bits = |p: Vec2| {
            let d = p - center;
            let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
            x * x + y * y
        };
        let reach_bits = i128::from(reach.to_bits()).pow(2);
        let enemy = seen
            .units
            .iter()
            .filter(|u| u.owner != player && u.alive() && distance_bits(u.position) <= reach_bits)
            .min_by_key(|u| (distance_bits(u.position), u.id));
        if let Some(enemy) = enemy {
            out.push(Command::Attack {
                units: own,
                target: enemy.id,
            });
            return out;
        }
        out.push(Command::FormationMove {
            units: own,
            target: center,
            formation: Formation::Auto,
            facing: None,
            engage: Engage::Infantry,
        });
    }
    out
}

struct Played {
    sim: Sim,
    hashes: Vec<u64>,
}

fn play(map: &Arc<Map>, config: &SimConfig, until: Option<u64>) -> Played {
    let mut sim = Sim::new(config, map.clone(), SEED).unwrap();
    let mission = config.mission.clone().unwrap();
    let mut ais: Vec<Ai> = mission
        .computers
        .iter()
        .map(|(p, d)| {
            let d: Difficulty = d.parse().unwrap();
            Ai::new(*p, d, config, map, SEED)
        })
        .collect();
    let mut hashes = Vec::new();
    while sim.outcome().is_none() && sim.tick() < until.unwrap_or(LIMIT_TICKS) {
        let mut commands: Vec<PlayerCommand> = autopilot(&sim, mission.player, sim.tick())
            .into_iter()
            .map(|c| PlayerCommand::new(mission.player, c))
            .collect();
        for ai in &mut ais {
            let view = PlayerView::new(&sim, ai.player());
            commands.extend(
                ai.think(&view)
                    .into_iter()
                    .map(|c| PlayerCommand::new(ai.player(), c)),
            );
        }
        commands.sort_by_key(|c| c.player);
        sim.step(&commands);
        if sim.tick().is_multiple_of(500) {
            hashes.push(sim.state_hash());
        }
    }
    Played { sim, hashes }
}

#[test]
fn every_mission_can_be_won() {
    let files = mission_files();
    assert!(files.len() >= 4, "tutorial and three missions: {files:?}");
    for (map_name, id, path) in &files {
        let map = load_map(map_name);
        let config = config_of(&map, map_name, id, path);
        let played = play(&map, &config, None);
        let sim = &played.sim;
        let outcome = sim.outcome();
        let objectives: Vec<String> = sim
            .mission_objectives()
            .iter()
            .map(|o| format!("{}={}", o.id, o.state.name()))
            .collect();
        assert_eq!(
            outcome.map(|o| o.winner),
            Some(Some(PlayerId(0))),
            "{map_name}/{id} not won after {} ticks: objectives {objectives:?}, message {:?}",
            sim.tick(),
            sim.mission_message()
        );
        assert!(
            sim.mission_objectives()
                .iter()
                .all(|o| o.state == ObjectiveState::Done),
            "{map_name}/{id}: {objectives:?}"
        );
        let arrivals: Vec<String> = sim
            .mission_arrivals()
            .iter()
            .map(|a| {
                format!(
                    "p{} ({}, {}) -> ({}, {}) at tick {}",
                    a.owner.0,
                    a.entry.x.to_num::<i32>(),
                    a.entry.y.to_num::<i32>(),
                    a.to.x.to_num::<i32>(),
                    a.to.y.to_num::<i32>(),
                    a.tick
                )
            })
            .collect();
        println!(
            "{map_name}/{id}: won at tick {} ({} s game time), {} units, columns driving in {arrivals:?}",
            sim.tick(),
            sim.tick() / 20,
            sim.snapshot().units.len()
        );
    }
}

/// Every mission on every difficulty (prompt 181) against the autopilot:
/// game time, result, losses of the player and the "hopeless" warning.
/// Easy and Normal must be won; Hard is only reported.
#[test]
fn every_level_of_every_mission() {
    for (map_name, id, path) in &mission_files() {
        let map = load_map(map_name);
        for level in Level::ALL {
            let config = config_at(&map, map_name, id, path, level);
            let played = play(&map, &config, None);
            let sim = &played.sim;
            let won = sim.outcome().map(|o| o.winner) == Some(Some(PlayerId(0)));
            let snap = sim.snapshot();
            let own = |alive: bool| {
                snap.units
                    .iter()
                    .filter(|u| u.owner == PlayerId(0) && u.alive() == alive)
                    .count()
            };
            println!(
                "{map_name}/{id} {}: {} after {} s, {} own units lost, {} left, warning {:?}",
                level.name(),
                if won { "won" } else { "not won" },
                sim.tick() / 20,
                own(false),
                own(true),
                sim.mission_hopeless().map(|(_, t)| t / 20)
            );
            if level != Level::Hard {
                assert!(won, "{map_name}/{id} not won on {}", level.name());
            }
        }
    }
}

#[test]
fn a_mission_plays_the_same_twice_and_survives_a_snapshot() {
    let files = mission_files();
    let (map_name, id, path) = files
        .iter()
        .find(|(_, id, _)| id == "tutorial")
        .expect("tutorial");
    let map = load_map(map_name);
    let config = config_of(&map, map_name, id, path);
    let a = play(&map, &config, Some(3000));
    let b = play(&map, &config, Some(3000));
    assert_eq!(a.hashes, b.hashes);
    assert!(
        a.sim.mission_progress().0 >= 3,
        "{:?}",
        a.sim.mission_progress()
    );
    // The snapshot (saved games, prompt 038) keeps the mission's progress.
    let bytes = net::snapshot::encode(&a.sim, &map);
    let restored = net::snapshot::decode(&bytes, map.clone()).unwrap();
    assert_eq!(restored.state_hash(), a.sim.state_hash());
    assert_eq!(restored.mission_progress(), a.sim.mission_progress());
}

/// `cargo test -p ai --test missions debug_units -- --ignored --nocapture`
/// with `MISSION=<id>` prints the units every 400 ticks (tuning missions).
#[test]
#[ignore = "tool for tuning missions"]
fn debug_units() {
    let wanted = std::env::var("MISSION").unwrap_or_else(|_| "tutorial".into());
    let files = mission_files();
    let (map_name, id, path) = files.iter().find(|(_, id, _)| *id == wanted).unwrap();
    let map = load_map(map_name);
    let config = config_of(&map, map_name, id, path);
    let end: u64 = std::env::var("END")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4000);
    for t in (end / 10..=end).step_by((end / 10) as usize) {
        let a = play(&map, &config, Some(t));
        let objectives: Vec<String> = a
            .sim
            .mission_objectives()
            .iter()
            .map(|o| format!("{}={}", o.id, o.state.name()))
            .collect();
        println!("tick {t}: {objectives:?}");
        for u in a.sim.snapshot().units {
            let p = |v: Vec2| (v.x.to_num::<i32>(), v.y.to_num::<i32>());
            println!(
                "  unit {} p{} {} at {:?} -> {:?} hp {}",
                u.id.0,
                u.owner.0,
                sim::mission::type_name(&config.unit_types, u.unit_type),
                p(u.position),
                u.target.map(p),
                u.hp
            );
        }
    }
}

/// Prompt 108: every column that drives in (`arrive = "drive"`) finds a
/// path from its entry to its place for its unit type, the entry stands on
/// ground the type can drive on.
#[test]
fn every_driving_entry_reaches_its_place() {
    use sim::mission::{Action, Arrival};
    let mut checked = 0;
    for (map_name, id, path) in &mission_files() {
        let map = load_map(map_name);
        let config = config_of(&map, map_name, id, path);
        let grid = sim::path::PathGrid::new(&map, &config.terrain, &config.unit_types);
        for t in &config.mission.as_ref().unwrap().triggers {
            for a in &t.actions {
                let Action::Spawn {
                    unit_type,
                    at,
                    arrival: Arrival::Drive { entry },
                    ..
                } = a
                else {
                    continue;
                };
                let profile = *unit_type as usize;
                let name = &config.unit_types[profile].name;
                assert!(
                    grid.passable_for(profile, *entry),
                    "{map_name}/{id}: entry {entry:?} of {name} not passable"
                );
                let found = grid
                    .find_path(*entry, *at, profile)
                    .unwrap_or_else(|| panic!("{map_name}/{id}: no path for {name}"));
                assert!(
                    (found.goal - *at).length() < sim::Fixed::from_num(10),
                    "{map_name}/{id}: {name} only reaches {:?}",
                    found.goal
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 3, "driving spawns checked: {checked}");
}
