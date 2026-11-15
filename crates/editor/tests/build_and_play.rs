//! Prompt 028 acceptance: a map built by a script with the editor's tools
//! is saved, loaded on two peers and played in lockstep over loopback; both
//! compute the same hash every tick. The same script gives the same files.

use editor::brush::{Brush, DEEP_WATER, SHALLOW_WATER};
use editor::validate::{validate_files, validate_map};
use editor::{EditMap, MapFiles, PlacedObject};
use net::{LockstepConfig, LockstepSession, LoopbackTransport, PlayerId};
use sim::scenario::{CommandScript, DEMO_SCENARIO, parse_scenario};
use sim::{Biome, Catalog, Map, Scale, Sim, SimConfig, TerrainRules};
use std::path::PathBuf;
use std::sync::Arc;

const SEED: u64 = 28;
const TICKS: u64 = 400;
const ROAD: u8 = 2;

fn m(meters: i32) -> i32 {
    meters * 1000
}

fn r(meters: u32) -> u32 {
    meters * 1000
}

fn object(kind: &str, variant: &str, x: i32, y: i32, heading: u16) -> PlacedObject {
    PlacedObject {
        kind: kind.into(),
        variant: variant.into(),
        x_mm: m(x),
        y_mm: m(y),
        heading_deg: heading,
    }
}

/// A 400 m map: a hill, a hollow, a flattened square, a road, a river
/// (north–south) with a ford and a stone bridge, a wood, a house and moved
/// start positions. Every tool of the editor is used.
fn build() -> EditMap {
    let mut map = EditMap::new("editor_test", 400, 400, Biome::Forest).unwrap();
    // Hill in the north-west, raised over one stroke of ten dabs.
    map.begin_stroke();
    for _ in 0..10 {
        assert!(
            map.raise(Brush::new(m(-120), m(-120), r(45)), 1200)
                .is_some()
        );
    }
    map.end_stroke();
    // Hollow in the south-west, then smooth the hill.
    map.raise(Brush::new(m(-130), m(120), r(30)), -2500);
    map.smooth(Brush::new(m(-120), m(-120), r(55)), 600);
    // Village square, flattened to 1 m.
    map.flatten(Brush::new(m(-60), m(150), r(20)), 1000, 1000);
    // Road from the square to the south start, dab by dab.
    map.begin_stroke();
    for k in 0..=30 {
        map.paint(Brush::new(m(-60 + 2 * k), m(150 - 2 * k), r(4)), ROAD)
            .unwrap();
    }
    map.end_stroke();
    // River in the east, level 0, 1.8 m deep, 24 m wide.
    map.begin_stroke();
    let mut y = -200;
    while y <= 200 {
        map.river(Brush::new(m(120), m(y), r(12)), 0, 1800);
        y += 3;
    }
    map.end_stroke();
    // A ford: shallow water across the river (tanks need about 8 m).
    map.begin_stroke();
    for x in (104..=136).step_by(3) {
        map.paint(Brush::new(m(x), m(40), r(6)), SHALLOW_WATER)
            .unwrap();
    }
    map.end_stroke();
    // Stone bridge further south, snapped onto the path grid.
    let bridge = map
        .place(object("env_bridge_stone", "Arch2", 120, -105, 88))
        .unwrap();
    assert_eq!(bridge.heading_deg, 90);
    assert_eq!(
        bridge.y_mm,
        m(-103),
        "the bridge axis snaps to a path cell center line"
    );
    // A wood west of the meadow and a house on the square.
    let trees = [
        ("env_tree_deciduous".to_string(), "Oak".to_string()),
        ("env_tree_deciduous".to_string(), "Beech".to_string()),
        ("env_tree_conifer".to_string(), "Spruce".to_string()),
    ];
    map.begin_stroke();
    let mut planted = 0;
    for x in [-90, -75, -60] {
        planted += map
            .scatter(Brush::new(m(x), m(-30), r(18)), &trees, 400, 5000)
            .unwrap();
    }
    map.end_stroke();
    assert!(planted > 20, "only {planted} trees");
    map.place(object("env_house_farm", "Intact", -60, 170, 180))
        .unwrap();
    // A few trees too many: erase them again.
    let erased = map.erase(Brush::new(m(-60), m(-30), r(5)), Some("env_tree_deciduous"));
    assert!(planted > erased);
    map.set_start(0, m(10), m(70));
    map.set_start(1, m(-20), m(-80));
    map
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mud_and_steel_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn demo_on(map: &Map) -> SimConfig {
    parse_scenario(
        DEMO_SCENARIO,
        &Catalog::builtin().unwrap(),
        &Scale::builtin().unwrap(),
        &TerrainRules::builtin().unwrap(),
        map,
    )
    .unwrap()
}

struct Peer {
    sim: Sim,
    session: LockstepSession,
    transport: LoopbackTransport,
    script: CommandScript,
    hashes: Vec<u64>,
}

impl Peer {
    fn new(player: u8, transport: LoopbackTransport, map: Arc<Map>) -> Self {
        let config = demo_on(&map);
        let lockstep = LockstepConfig {
            players: vec![PlayerId(0), PlayerId(1)],
            input_delay: 3,
            hash_interval: 10,
        };
        Self {
            sim: Sim::new(&config, map, SEED).unwrap(),
            session: LockstepSession::new(PlayerId(player), &lockstep),
            transport,
            script: CommandScript::new(SEED, PlayerId(player), &config),
            hashes: Vec::new(),
        }
    }

    fn poll(&mut self) {
        while self.session.needs_local_input() {
            let tick = self.session.next_local_tick();
            let commands = self.script.commands_for_tick(tick);
            self.session
                .submit_local(commands, &mut self.transport)
                .unwrap();
        }
        self.session.receive(&mut self.transport).unwrap();
        if let Some(commands) = self.session.next_commands() {
            self.sim.step(&commands);
            let hash = self.sim.state_hash();
            self.hashes.push(hash);
            self.session
                .record_hash(self.sim.tick(), hash, &mut self.transport)
                .unwrap();
        }
    }
}

#[test]
fn built_map_plays_in_lockstep_on_two_peers() {
    let mut built = build();
    let report = validate_files(&built.files());
    assert!(report.ok(), "{:?}", report.errors);

    let dir = temp_dir("editor_lockstep");
    built.save_dir(&dir).unwrap();
    // Each peer reads and loads the folder on its own, like two machines.
    let load = || Arc::new(MapFiles::read_dir(&dir).unwrap().load().unwrap());
    let (map_a, map_b) = (load(), load());
    assert_eq!(map_a.hash(), map_b.hash());
    assert_eq!(Some(map_a.hash()), report.map_hash);
    assert!(map_a.terrain_ids().contains(&DEEP_WATER));
    assert_eq!(map_a.bridges().len(), 1);

    // The units stand at the map's start positions.
    let config = demo_on(&map_a);
    let first = config
        .spawns
        .iter()
        .find(|s| s.owner == PlayerId(1))
        .unwrap();
    assert_eq!(first.position, sim::Vec2::from_meters(-38, -86));

    let (ta, tb) = LoopbackTransport::pair();
    let mut a = Peer::new(0, ta, map_a);
    let mut b = Peer::new(1, tb, map_b);
    let mut rounds = 0;
    while a.sim.tick() < TICKS || b.sim.tick() < TICKS {
        if a.sim.tick() < TICKS {
            a.poll();
        }
        if b.sim.tick() < TICKS {
            b.poll();
        }
        rounds += 1;
        assert!(rounds < 10 * TICKS, "lockstep stalled");
    }
    a.session.receive(&mut a.transport).unwrap();
    b.session.receive(&mut b.transport).unwrap();
    assert_eq!(a.hashes.len(), TICKS as usize);
    assert_eq!(a.hashes, b.hashes, "per-tick hashes differ");
    assert_eq!(a.session.desync(), None);
    assert_eq!(b.session.desync(), None);
    assert_eq!(a.sim.snapshot(), b.sim.snapshot());
    // Units drove on the edited map.
    let start = Sim::new(&config, load(), SEED).unwrap().snapshot();
    let moved = start
        .units
        .iter()
        .zip(&a.sim.snapshot().units)
        .filter(|(s, e)| s.position != e.position)
        .count();
    assert!(moved >= 8, "only {moved} units moved");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_same_script_gives_the_same_files() {
    let a = build().files();
    let b = build().files();
    assert_eq!(a, b);
    // Saving and loading keeps everything.
    let dir = temp_dir("editor_roundtrip");
    a.write_dir(&dir).unwrap();
    let loaded = EditMap::load_dir(&dir).unwrap();
    assert_eq!(loaded.files(), a);
    assert_eq!(loaded.meta().generator.as_deref(), Some("map editor"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn undo_and_redo_restore_every_kind_of_change() {
    let mut map = build();
    let before = map.files();
    // Heights, terrain, objects and metadata in one stroke.
    map.begin_stroke();
    map.raise(Brush::new(m(50), m(-150), r(20)), 3000);
    map.paint(Brush::new(m(50), m(-150), r(10)), 4).unwrap();
    map.place(object("env_barn", "Intact", 50, -150, 0))
        .unwrap();
    map.set_start(1, m(-30), m(-90));
    map.end_stroke();
    // And a stroke reaching far beyond the first rectangle.
    map.begin_stroke();
    map.raise(Brush::new(m(-180), m(-180), r(10)), 500);
    map.raise(Brush::new(m(180), m(180), r(10)), 500);
    map.end_stroke();
    let after = map.files();
    assert_ne!(before, after);
    let steps = map.undo_steps();

    assert!(map.undo().rect.is_some());
    let changed = map.undo();
    assert!(changed.objects && changed.meta && changed.rect.is_some());
    assert_eq!(map.files(), before);
    assert_eq!(map.redo_steps(), 2);
    map.redo();
    map.redo();
    assert_eq!(map.files(), after);
    assert_eq!(map.undo_steps(), steps);
    // A new edit clears the redo steps.
    map.undo();
    map.raise(Brush::new(0, 0, r(5)), 100);
    assert_eq!(map.redo_steps(), 0);
    // Nothing changed: no undo step.
    let steps = map.undo_steps();
    map.begin_stroke();
    map.raise(Brush::new(0, 0, r(5)), 0);
    assert!(!map.end_stroke().any());
    assert_eq!(map.undo_steps(), steps);
}

#[test]
fn validation_finds_missing_starts_blocked_paths_and_skewed_bridges() {
    // A wall of deep water across the map: nobody gets through.
    let mut map = EditMap::new("blocked", 200, 200, Biome::Winter).unwrap();
    map.begin_stroke();
    for x in (-100..=100).step_by(4) {
        map.paint(Brush::new(m(x), 0, r(8)), DEEP_WATER).unwrap();
    }
    map.end_stroke();
    let report = validate_files(&map.files());
    assert!(
        report.errors.iter().any(|e| e.contains("cannot drive")),
        "{report:?}"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|e| e.contains("de_inf_rifle_squad"))
    );

    // A start in the water.
    map.undo();
    map.set_start(0, 0, 0);
    map.paint(Brush::new(0, 0, r(10)), DEEP_WATER).unwrap();
    let report = validate_files(&map.files());
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("player 0") && e.contains("cannot stand")),
        "{report:?}"
    );

    // No start for player 1.
    let mut map = EditMap::new("nostart", 200, 200, Biome::Desert).unwrap();
    assert!(validate_files(&map.files()).ok());
    map.remove_start(1);
    let report = validate_files(&map.files());
    assert_eq!(
        report.errors,
        vec!["player 1 has no start position".to_string()]
    );

    // A bridge whose axis misses the path grid (written by hand).
    let map = EditMap::new("skewed", 200, 200, Biome::Forest).unwrap();
    let mut files = map.files();
    files
        .objects_csv
        .push_str("env_bridge_wood,Span12,0,0,30\n");
    let report = validate_files(&files);
    assert!(report.errors[0].contains("align the axis"), "{report:?}");
    // The editor itself snaps it.
    let mut map = map;
    map.place(object("env_bridge_wood", "Span12", 0, 2, 90))
        .unwrap();
    assert!(validate_map(map.files().load().unwrap()).ok());
}

#[test]
fn large_height_ranges_are_saved_with_coarser_steps() {
    let mut map = EditMap::new("tall", 200, 200, Biome::Forest).unwrap();
    map.raise(Brush::new(0, 0, r(60)), 80_000);
    let dir = temp_dir("editor_tall");
    map.save_dir(&dir).unwrap();
    let loaded = EditMap::load_dir(&dir).unwrap();
    // After saving, the editor holds exactly what the file holds.
    assert_eq!(loaded.heights(), map.heights());
    assert!(map.files().map_toml.contains("height_step_mm = 2"));
    let _ = std::fs::remove_dir_all(&dir);
}
