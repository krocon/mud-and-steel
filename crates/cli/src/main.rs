//! Headless runner: simulates the demo scenario without graphics.

mod ai_tournament;
mod bench;
mod catalog_doc;
mod crater_bench;
mod gen_map;
mod gen_textures;
mod infantry_bench;
mod lan;
mod path_bench;
mod save_bench;
#[cfg(feature = "steam")]
mod steam;
mod vision_bench;

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Directory of the map folders, relative to the repository root (where
/// `just` runs).
const MAPS_DIR: &str = "data/maps";

#[derive(Parser)]
#[command(version, about = "Mud and Steel headless simulation runner")]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Runs the scripted demo scenario and prints the final state hash.
    Run {
        /// Number of ticks to simulate.
        #[arg(long, default_value_t = 1000)]
        ticks: u64,
        /// Seed of the simulation and of the command script.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Also print the final position of every unit.
        #[arg(long)]
        verbose: bool,
        /// Write a replay of the run to this file.
        #[arg(long)]
        record: Option<std::path::PathBuf>,
    },
    /// Plays a replay headless and checks its end hash (exit code 1 on mismatch).
    Replay {
        /// Replay file (`.msreplay`).
        file: std::path::PathBuf,
    },
    /// Checks a saved game (prompt 038): restores its snapshot and
    /// re-simulates its replay from tick 0; both must agree.
    SaveCheck {
        /// Saved game (`.mssave`).
        file: std::path::PathBuf,
    },
    /// What loading costs (prompt 038): two computer players play a
    /// scenario, then the match is re-simulated from its replay and saved
    /// and restored as a snapshot; times and sizes.
    SaveBench {
        /// Scenario file; its map is loaded from data/maps/.
        #[arg(long, default_value = "data/scenarios/bench_large.toml")]
        scenario: PathBuf,
        #[arg(long, default_value_t = 6000)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Difficulty of both computer players.
        #[arg(long, default_value = "hard")]
        ai: ai::Difficulty,
    },
    /// Generates a test map (height field, terrain) from a seed.
    GenMap {
        /// Map name, written into map.toml.
        #[arg(long)]
        name: String,
        /// Width and depth in meters (multiple of 4).
        #[arg(long, default_value_t = 500)]
        size_m: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Climate look written into map.toml (presentation only).
        #[arg(long, default_value = "forest", value_parser = ["forest", "desert", "winter"])]
        biome: String,
        /// Distance between the two starts in meters (120: both in the
        /// central meadow, as on the test maps).
        #[arg(long, default_value_t = gen_map::DEFAULT_START_DISTANCE_M)]
        start_distance_m: u32,
        /// Output folder (default: data/maps/<name>).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Loads a map folder and reports load time, memory and height range.
    MapInfo {
        /// Map folder, e.g. data/maps/test_large.
        dir: PathBuf,
    },
    /// Validates a map folder like the map editor: loading, start
    /// positions, room for the units, paths between the starts (exit code 1
    /// on errors).
    MapCheck {
        /// Map folder, e.g. data/maps/test_small.
        dir: PathBuf,
    },
    /// Benchmarks pathfinding: many units with simultaneous path requests.
    PathBench {
        /// Map folder, e.g. data/maps/test_large.
        #[arg(long, default_value = "data/maps/test_large")]
        map: PathBuf,
        #[arg(long, default_value_t = 200)]
        units: u32,
        /// Units per move order (1 = every unit gets its own path).
        #[arg(long, default_value_t = 1)]
        group: u32,
        /// Upper bound of simulated ticks.
        #[arg(long, default_value_t = 20 * 60 * 20)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Targets lie within this distance of the map center per axis (m).
        #[arg(long, default_value_t = 1800)]
        range_m: i32,
    },
    /// Benchmarks craters (prompt 021): many craters in a short time among
    /// moving units of two players, time per tick.
    CraterBench {
        /// Map folder, e.g. data/maps/test_large.
        #[arg(long, default_value = "data/maps/test_large")]
        map: PathBuf,
        /// Units per player.
        #[arg(long, default_value_t = 100)]
        units: u32,
        #[arg(long, default_value_t = 100)]
        craters: u32,
        #[arg(long, default_value_t = 5)]
        per_tick: u32,
        /// Craters land within this distance of the map center per axis (m).
        #[arg(long, default_value_t = 200)]
        range_m: i32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Computer players against each other over many seeds (prompt 024):
    /// sides alternate every game; prints the results and a hash over them.
    AiTournament {
        /// Difficulty of AI A (passive, easy, normal, hard).
        #[arg(long, default_value = "hard")]
        a: ai::Difficulty,
        /// Difficulty of AI B.
        #[arg(long, default_value = "passive")]
        b: ai::Difficulty,
        /// Every difficulty against every lower one instead of A vs B.
        #[arg(long)]
        matrix: bool,
        #[arg(long, default_value_t = 40)]
        games: u64,
        /// Seed of the first game; game i uses seed + i.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Map name in data/maps/ (or the built-in test_small).
        #[arg(long, default_value = "test_small")]
        map: String,
        /// Factions of player 0 and 1, e.g. `de,su` (default: the scenario's).
        #[arg(long, value_delimiter = ',')]
        factions: Vec<String>,
        /// `battle` (the demo armies) or `base` (base building, prompt 023).
        #[arg(long, default_value = "battle")]
        mode: String,
        /// A game not decided after this many ticks counts as a timeout.
        #[arg(long, default_value_t = 6000)]
        max_ticks: u64,
        /// Games played at the same time (default: all CPU cores).
        #[arg(long)]
        jobs: Option<usize>,
        /// Print every game.
        #[arg(long)]
        verbose: bool,
    },
    /// Benchmarks vision (prompt 018): many moving units of two players.
    VisionBench {
        /// Map folder, e.g. data/maps/test_large.
        #[arg(long, default_value = "data/maps/test_large")]
        map: PathBuf,
        #[arg(long, default_value_t = 500)]
        units: u32,
        #[arg(long, default_value_t = 600)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Benchmarks infantry (prompt 022): 504 soldiers of two players stand,
    /// march and fight on `test_small`.
    InfantryBench {
        #[arg(long, default_value_t = 1200)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Overall benchmark (prompt 016): two computer players play a large
    /// scenario; time per tick of the simulation, the AIs and the snapshot.
    Bench {
        /// Scenario file; its map is loaded from data/maps/.
        #[arg(long, default_value = "data/scenarios/bench_large.toml")]
        scenario: PathBuf,
        #[arg(long, default_value_t = 6000)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Difficulty of both computer players.
        #[arg(long, default_value = "hard")]
        ai: ai::Difficulty,
        /// Tick windows reported separately.
        #[arg(long, default_value_t = 10)]
        windows: u64,
        /// Play-throughs; the minimum per tick filters out the noise of a
        /// busy machine.
        #[arg(long, default_value_t = 1)]
        runs: u32,
    },
    /// Generates the procedural ground textures of the terrain shader.
    GenTerrainTextures {
        /// Output folder.
        #[arg(long, default_value = "godot/assets/textures/terrain")]
        out: PathBuf,
        /// Pixels per layer tile (power of two).
        #[arg(long, default_value_t = 1024)]
        size: u32,
        #[arg(long, default_value_t = 1)]
        seed: u32,
        /// Also write a lit contact sheet of all layers to this PNG.
        #[arg(long)]
        preview: Option<PathBuf>,
    },
    /// Hosts a LAN game of the demo scenario and plays it headless
    /// (exit code 1 on desync).
    LanHost {
        /// Player name shown to searching clients.
        #[arg(long, default_value = "cli host")]
        name: String,
        #[arg(long, default_value_t = net::lan::DEFAULT_GAME_PORT)]
        port: u16,
        #[arg(long, default_value_t = net::lan::DEFAULT_DISCOVERY_PORT)]
        discovery_port: u16,
        /// Players including the host; the match starts when all joined.
        #[arg(long, default_value_t = 2)]
        players: usize,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Initial input delay in ticks (adapted to the round trip later).
        #[arg(long, default_value_t = 3)]
        input_delay: u32,
        #[command(flatten)]
        play: LanPlayArgs,
    },
    /// Joins a LAN game (by address or the first one found) and plays it
    /// headless (exit code 1 on desync).
    LanJoin {
        #[arg(long, default_value = "cli client")]
        name: String,
        /// Host address, e.g. 192.168.1.20:47810. Searched on the LAN if omitted.
        #[arg(long)]
        address: Option<std::net::SocketAddr>,
        #[arg(long, default_value_t = net::lan::DEFAULT_DISCOVERY_PORT)]
        discovery_port: u16,
        #[command(flatten)]
        play: LanPlayArgs,
    },
    /// Hosts an online game of the demo over Steam (needs a running Steam
    /// client) and plays it headless; prints the lobby code.
    #[cfg(feature = "steam")]
    SteamHost {
        /// Friends only instead of listed for everyone.
        #[arg(long)]
        friends_only: bool,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 3)]
        input_delay: u32,
        #[command(flatten)]
        play: LanPlayArgs,
    },
    /// Joins an online game over Steam (by lobby code, or the first one
    /// listed) and plays it headless.
    #[cfg(feature = "steam")]
    SteamJoin {
        /// Lobby code shown by the host.
        #[arg(long)]
        code: Option<u64>,
        #[command(flatten)]
        play: LanPlayArgs,
    },
    /// Writes docs/UNIT_CATALOG.md (or the given path) from the built-in catalog.
    CatalogDoc {
        #[arg(default_value = catalog_doc::DOC_PATH)]
        out: std::path::PathBuf,
    },
    /// Prints the prices and production times of everything the economy
    /// sells (prompt 023, data/economy.toml) as a Markdown table.
    EconomyTable,
    /// Validates catalog files (default: the built-in catalog) and prints all errors.
    CatalogCheck {
        /// Catalog TOML files, named `<faction>.toml`.
        files: Vec<std::path::PathBuf>,
    },
}

#[derive(clap::Args)]
struct LanPlayArgs {
    /// Number of ticks to play.
    #[arg(long, default_value_t = 600)]
    ticks: u64,
    /// Write a replay of the match to this file.
    #[arg(long)]
    record: Option<PathBuf>,
    /// Give up after this many seconds (lobby and match).
    #[arg(long, default_value_t = 60)]
    timeout_s: u64,
    /// Testing: apply an extra command locally at this tick to provoke a desync.
    #[arg(long, hide = true)]
    cheat_tick: Option<u64>,
}

impl LanPlayArgs {
    fn options(self) -> lan::MatchOptions {
        lan::MatchOptions {
            ticks: self.ticks,
            record: self.record,
            cheat_tick: self.cheat_tick,
            timeout: std::time::Duration::from_secs(self.timeout_s),
        }
    }
}

fn exit_with(result: Result<i32, String>) -> ! {
    match result {
        Ok(code) => std::process::exit(code),
        // 2, so that errors are told apart from a desync (1).
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2)
        }
    }
}

fn main() {
    match Cli::parse().command {
        CliCommand::MapCheck { dir } => exit_with(map_check(&dir)),
        CliCommand::LanHost {
            name,
            port,
            discovery_port,
            players,
            seed,
            input_delay,
            play,
        } => {
            let options = lan::HostOptions {
                name,
                port,
                discovery_port,
                players: players.max(2),
                seed,
                input_delay,
            };
            exit_with(lan::host(&options, &play.options()));
        }
        CliCommand::LanJoin {
            name,
            address,
            discovery_port,
            play,
        } => {
            let options = lan::JoinOptions {
                name,
                address,
                discovery_port,
            };
            exit_with(lan::join(&options, &play.options()));
        }
        #[cfg(feature = "steam")]
        CliCommand::SteamHost {
            friends_only,
            seed,
            input_delay,
            play,
        } => {
            let options = steam::HostOptions {
                public: !friends_only,
                seed,
                input_delay,
            };
            exit_with(steam::host(&options, &play.options()));
        }
        #[cfg(feature = "steam")]
        CliCommand::SteamJoin { code, play } => exit_with(steam::join(code, &play.options())),
        CliCommand::Run {
            ticks,
            seed,
            verbose,
            record,
        } => {
            let config = sim::scenario::demo_config();
            let mut recorder = net::ReplayRecorder::new(seed, config.clone(), &config.players());
            let sim = sim::scenario::run_demo_with(seed, ticks, |_, commands| {
                recorder.record(commands);
            });
            if let Some(path) = record {
                std::fs::write(&path, recorder.finish(&sim).encode())
                    .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", path.display())));
                println!("wrote {}", path.display());
            }
            if verbose {
                for unit in sim.snapshot().units {
                    println!(
                        "unit {} of player {} at ({}, {}) target {:?}",
                        unit.id.0,
                        unit.owner.0,
                        unit.position.x,
                        unit.position.y,
                        unit.target.map(|t| (t.x.to_string(), t.y.to_string()))
                    );
                }
            }
            println!(
                "seed={seed} ticks={} hash={:#018x}",
                sim.tick(),
                sim.state_hash()
            );
        }
        CliCommand::Replay { file } => replay(&file),
        CliCommand::SaveCheck { file } => save_check(&file),
        CliCommand::SaveBench {
            scenario,
            ticks,
            seed,
            ai,
        } => {
            let (config, map) = bench_config(&scenario).unwrap_or_else(|e| fail(&e));
            let options = save_bench::Options { ticks, seed, ai };
            match save_bench::run(&config, map, &options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::GenMap {
            name,
            size_m,
            seed,
            biome,
            start_distance_m,
            out,
        } => {
            let out = out.unwrap_or_else(|| Path::new(MAPS_DIR).join(&name));
            let options = gen_map::Options {
                name: &name,
                size_m,
                seed,
                biome: &biome,
                start_distance_m,
                out: &out,
            };
            match gen_map::generate(&options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::MapInfo { dir } => map_info(&dir),
        CliCommand::PathBench {
            map,
            units,
            group,
            ticks,
            seed,
            range_m,
        } => {
            let map = Arc::new(load_map_dir(&map).unwrap_or_else(|e| fail(&e)));
            let options = path_bench::Options {
                units,
                group,
                ticks,
                seed,
                range_m,
            };
            match path_bench::run(map, &options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::CraterBench {
            map,
            units,
            craters,
            per_tick,
            range_m,
            seed,
        } => {
            let map = Arc::new(load_map_dir(&map).unwrap_or_else(|e| fail(&e)));
            let options = crater_bench::Options {
                units,
                craters,
                per_tick,
                range_m,
                seed,
            };
            match crater_bench::run(map, &options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::AiTournament {
            a,
            b,
            matrix,
            games,
            seed,
            map,
            factions,
            mode,
            max_ticks,
            jobs,
            verbose,
        } => {
            let map = find_map(&map);
            let scenario = match mode.as_str() {
                "battle" => sim::scenario::DEMO_SCENARIO,
                "base" => sim::scenario::SKIRMISH_SCENARIO,
                other => fail(&format!("unknown mode '{other}' (battle, base)")),
            };
            let config = tournament_config(&map, &factions, scenario).unwrap_or_else(|e| fail(&e));
            let options = ai_tournament::Options {
                games,
                seed,
                max_ticks,
                jobs: jobs.unwrap_or_else(|| {
                    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
                }),
                verbose,
            };
            let start = std::time::Instant::now();
            if matrix {
                ai_tournament::matrix(&config, &map, &options);
            } else {
                let summary = ai_tournament::series(&config, &map, a, b, &options);
                ai_tournament::print_header();
                ai_tournament::print(&summary);
            }
            println!("{:.1} s", start.elapsed().as_secs_f64());
        }
        CliCommand::VisionBench {
            map,
            units,
            ticks,
            seed,
        } => {
            let map = Arc::new(load_map_dir(&map).unwrap_or_else(|e| fail(&e)));
            let options = vision_bench::Options { units, ticks, seed };
            match vision_bench::run(map, &options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::Bench {
            scenario,
            ticks,
            seed,
            ai,
            windows,
            runs,
        } => {
            let (config, map) = bench_config(&scenario).unwrap_or_else(|e| fail(&e));
            let options = bench::Options {
                ticks,
                seed,
                ai,
                windows,
                runs,
            };
            match bench::run(&config, map, &options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::InfantryBench { ticks, seed } => {
            match infantry_bench::run(&infantry_bench::Options { ticks, seed }) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::GenTerrainTextures {
            out,
            size,
            seed,
            preview,
        } => {
            let options = gen_textures::Options {
                out: &out,
                size,
                seed,
                preview: preview.as_deref(),
            };
            match gen_textures::generate(&options) {
                Ok(report) => println!("{report}"),
                Err(e) => fail(&e),
            }
        }
        CliCommand::CatalogCheck { files } => catalog_check(&files),
        CliCommand::EconomyTable => economy_table(),
        CliCommand::CatalogDoc { out } => {
            let catalog = sim::Catalog::builtin().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(&out, catalog_doc::render(&catalog))
                .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", out.display())));
            println!("wrote {}", out.display());
        }
    }
}

/// `cli economy-table`.
fn economy_table() {
    let rules = sim::EconomyRules::builtin().unwrap_or_else(|e| fail(&e.to_string()));
    let catalog = sim::Catalog::builtin().unwrap_or_else(|e| fail(&e.to_string()));
    let scale = sim::Scale::builtin().unwrap_or_else(|e| fail(&e.to_string()));
    println!("| Built in | Id | Fuel | Ammunition | Manpower | Time (s) |");
    println!("|---|---|---|---|---|---|");
    let row = |place: &str, id: &str, cost: sim::Cost| {
        let [f, a, m] = cost.resources;
        let ms = cost.time_ms;
        println!(
            "| {place} | `{id}` | {f} | {a} | {m} | {}.{} |",
            ms / 1000,
            ms % 1000 / 100
        );
    };
    for b in &rules.buildings {
        row(
            "–",
            &b.id,
            sim::Cost {
                resources: b.cost,
                time_ms: b.build_s * 1000,
            },
        );
    }
    for f in &rules.fortifications {
        for id in &f.ids {
            let Some(entry) = catalog.get(id) else {
                continue;
            };
            let combat = sim::CombatType::from_catalog(entry, &scale);
            if let Some(cost) = rules.structure_cost(id, &catalog, &combat) {
                row("fortification", id, cost);
            }
        }
    }
    for b in rules.buildings.iter().filter(|b| !b.produces.is_empty()) {
        for faction in ["de", "su", "us"] {
            for id in rules.produced(&b.produces, faction) {
                let Some(entry) = catalog.get(id) else {
                    continue;
                };
                let combat = sim::CombatType::from_catalog(entry, &scale);
                row(&b.name, id, rules.unit_cost(entry, &combat));
            }
        }
    }
}

fn replay(path: &std::path::Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
    if let Ok(info) = net::Replay::read_info(&bytes) {
        println!(
            "recorded with game version {} on '{}' at {}",
            info.game_version, info.map, info.created
        );
    }
    let replay = net::Replay::decode(&bytes).unwrap_or_else(|e| fail(&e.to_string()));
    let commands: usize = replay.frames.iter().map(|f| f.commands.len()).sum();
    println!(
        "replay: seed={} ticks={} players={} units={} commands={}",
        replay.seed,
        replay.ticks,
        replay.players.len(),
        replay.config.spawns.len(),
        commands
    );
    let map = find_map(&replay.config.map.name);
    let report = replay.verify(map).unwrap_or_else(|e| fail(&e.to_string()));
    if report.matches() {
        println!("end hash OK: {:#018x}", report.actual_hash);
    } else {
        fail(&format!(
            "end hash MISMATCH: expected {:#018x}, got {:#018x}",
            report.expected_hash, report.actual_hash
        ));
    }
}

fn save_check(path: &std::path::Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
    let save = net::SaveGame::decode(&bytes).unwrap_or_else(|e| fail(&e.to_string()));
    println!(
        "saved game: map '{}', tick {}, {} computer player(s), game version {}",
        save.info.map,
        save.info.ticks,
        save.hosted.len(),
        save.info.game_version
    );
    let map = find_map(&save.replay.config.map.name);
    match save.verify(map) {
        Ok(()) => println!("snapshot and replay agree: {:#018x}", save.replay.end_hash),
        Err(e) => fail(&e.to_string()),
    }
}

/// Reads and decodes `dir/map.toml`, `dir/height.png`, `dir/terrain.png`
/// and, if present, `dir/objects.csv`.
fn load_map_dir(dir: &Path) -> Result<sim::Map, String> {
    let read = |file: &str| {
        let path = dir.join(file);
        std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let toml = String::from_utf8(read("map.toml")?).map_err(|e| e.to_string())?;
    let mut map = sim::Map::load(&toml, &read("height.png")?, &read("terrain.png")?)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    if dir.join("objects.csv").exists() {
        let csv = String::from_utf8(read("objects.csv")?).map_err(|e| e.to_string())?;
        map.add_objects(&csv)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(map)
}

/// A scenario file and its map from `data/maps/`.
fn bench_config(file: &Path) -> Result<(sim::SimConfig, Arc<sim::Map>), String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", file.display());
    let name = sim::scenario::scenario_map_name(&text).map_err(|e| err(&e))?;
    let map = Arc::new(load_map_dir(&Path::new(MAPS_DIR).join(&name))?);
    let catalog = sim::Catalog::builtin().map_err(|e| err(&e))?;
    let scale = sim::Scale::builtin().map_err(|e| err(&e))?;
    let terrain = sim::TerrainRules::builtin().map_err(|e| err(&e))?;
    let config = sim::scenario::parse_scenario(&text, &catalog, &scale, &terrain, &map)
        .map_err(|e| err(&e))?;
    Ok((config, map))
}

/// The demo scenario on `map`; `factions` (one per player, may be empty)
/// replaces the units like the skirmish setup.
fn tournament_config(
    map: &sim::Map,
    factions: &[String],
    scenario: &str,
) -> Result<sim::SimConfig, String> {
    let overrides: Vec<(sim::PlayerId, &str)> = (0u8..)
        .zip(factions)
        .filter(|(_, f)| !f.is_empty())
        .map(|(p, f)| (sim::PlayerId(p), f.as_str()))
        .collect();
    let err = |e: &dyn std::fmt::Display| e.to_string();
    sim::scenario::parse_scenario_with_factions(
        scenario,
        &overrides,
        &sim::Catalog::builtin().map_err(|e| err(&e))?,
        &sim::Scale::builtin().map_err(|e| err(&e))?,
        &sim::TerrainRules::builtin().map_err(|e| err(&e))?,
        map,
    )
    .map_err(|e| e.0)
}

/// The map with this name: `data/maps/<name>/` if it exists, else the
/// built-in map if the name matches.
fn find_map(name: &str) -> Arc<sim::Map> {
    let dir = Path::new(MAPS_DIR).join(name);
    if dir.join("map.toml").exists() {
        return Arc::new(load_map_dir(&dir).unwrap_or_else(|e| fail(&e)));
    }
    if name == sim::scenario::BUILTIN_MAP_NAME {
        return sim::scenario::builtin_map();
    }
    fail(&format!("map '{name}' not found in {MAPS_DIR}/"))
}

/// Prints the editor's validation report of a map folder (prompt 028).
fn map_check(dir: &Path) -> Result<i32, String> {
    let start = std::time::Instant::now();
    let report = editor::validate::validate_files(&editor::MapFiles::read_dir(dir)?);
    for error in &report.errors {
        println!("error: {error}");
    }
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
    let hash = report
        .map_hash
        .map_or_else(|| "-".to_string(), |h| format!("{h:#018x}"));
    println!(
        "{}: {} ({} errors, {} warnings), map hash {hash}, checked in {} ms",
        dir.display(),
        if report.ok() { "ok" } else { "NOT PLAYABLE" },
        report.errors.len(),
        report.warnings.len(),
        start.elapsed().as_millis()
    );
    Ok(i32::from(!report.ok()))
}

fn map_info(dir: &Path) {
    let start = std::time::Instant::now();
    let names = ["map.toml", "height.png", "terrain.png"];
    let files: Vec<Vec<u8>> = names
        .iter()
        .map(|f| std::fs::read(dir.join(f)).unwrap_or_else(|e| fail(&format!("{f}: {e}"))))
        .collect();
    let read = start.elapsed();
    let toml = String::from_utf8_lossy(&files[0]);
    let decode_start = std::time::Instant::now();
    let mut map =
        sim::Map::load(&toml, &files[1], &files[2]).unwrap_or_else(|e| fail(&e.to_string()));
    let decode = decode_start.elapsed();
    let objects_file = std::fs::read_to_string(dir.join("objects.csv")).ok();
    let objects_start = std::time::Instant::now();
    if let Some(csv) = &objects_file {
        map.add_objects(csv)
            .unwrap_or_else(|e| fail(&e.to_string()));
    }
    let objects_time = objects_start.elapsed();

    let (sx, sy) = map.samples();
    let samples = u64::from(sx) * u64::from(sy);
    let bytes = size_of_val(map.heights_mm()) + map.terrain_ids().len();
    let low = map.heights_mm().iter().min().copied().unwrap_or(0);
    let high = map.heights_mm().iter().max().copied().unwrap_or(0);
    let mut counts = [0u64; 256];
    for &id in map.terrain_ids() {
        counts[usize::from(id)] += 1;
    }
    // Speed of the height lookup the simulation uses every tick.
    let lookups = 1_000_000u32;
    let size = map.size();
    let mut rng = sim::Rng::new(1);
    let points: Vec<sim::Vec2> = (0..lookups)
        .map(|_| {
            let half_mm = |v: sim::Fixed| (v * sim::Fixed::from_num(500)).to_num::<i32>();
            sim::Vec2::from_millimeters(
                rng.range_i32(-half_mm(size.x), half_mm(size.x)),
                rng.range_i32(-half_mm(size.y), half_mm(size.y)),
            )
        })
        .collect();
    let lookup_start = std::time::Instant::now();
    let mut sum = sim::Fixed::ZERO;
    for &p in &points {
        sum = sum.wrapping_add(map.height_at(p));
    }
    let lookup = lookup_start.elapsed();
    let file_bytes: usize =
        files.iter().map(Vec::len).sum::<usize>() + objects_file.as_ref().map_or(0, String::len);
    // Steepest grade between neighboring samples.
    let mut steepest = 0i64;
    for j in 0..sy {
        for i in 0..sx {
            let h = i64::from(map.sample_height_mm(i, j));
            let right = i64::from(map.sample_height_mm(i + 1, j));
            let down = i64::from(map.sample_height_mm(i, j + 1));
            steepest = steepest.max((right - h).abs()).max((down - h).abs());
        }
    }

    println!("map '{}' ({})", map.name(), dir.display());
    println!(
        "  size {} x {} m, {sx} x {sy} samples ({samples}), cell {} mm",
        size.x,
        size.y,
        map.cell_size_mm()
    );
    println!(
        "  heights {:.3} .. {:.3} m, hash {:#018x}",
        f64::from(low) / 1000.0,
        f64::from(high) / 1000.0,
        map.hash()
    );
    println!(
        "  steepest grade {:.1} %",
        steepest as f64 * 100.0 / f64::from(map.cell_size_mm())
    );
    println!(
        "  files {:.2} MB, memory {:.2} MB (heights i32 + terrain u8)",
        file_bytes as f64 / 1e6,
        bytes as f64 / 1e6
    );
    let blocking = map
        .objects()
        .iter()
        .filter(|o| map.object_kinds()[usize::from(o.kind)].blocks != sim::Blocks::None)
        .count();
    println!(
        "  objects {} ({} kinds, {blocking} blocking), {:.2} MB in memory, parse + hash {:.1} ms",
        map.objects().len(),
        map.object_kinds().len(),
        size_of_val(map.objects()) as f64 / 1e6,
        objects_time.as_secs_f64() * 1e3
    );
    println!(
        "  read {:.1} ms, decode + validate + hash {:.1} ms",
        read.as_secs_f64() * 1e3,
        decode.as_secs_f64() * 1e3
    );
    println!(
        "  height_at: {:.1} ns per lookup (checksum {sum})",
        lookup.as_secs_f64() * 1e9 / f64::from(lookups)
    );
    let rules = sim::TerrainRules::builtin().unwrap_or_else(|e| fail(&e.to_string()));
    for (id, &count) in counts.iter().enumerate().filter(|(_, c)| **c > 0) {
        let name = rules.types.get(id).map_or("?", |t| t.name.as_str());
        println!(
            "  terrain {id} {name}: {:.1} %",
            count as f64 * 100.0 / samples as f64
        );
    }
}

fn catalog_check(files: &[std::path::PathBuf]) {
    let result = if files.is_empty() {
        sim::Catalog::builtin()
    } else {
        let loaded: Vec<(String, String)> = files
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let contents = std::fs::read_to_string(path)
                    .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
                (name, contents)
            })
            .collect();
        let borrowed: Vec<(&str, &str)> = loaded
            .iter()
            .map(|(n, c)| (n.as_str(), c.as_str()))
            .collect();
        sim::Catalog::parse(&borrowed)
    };
    match result {
        Ok(catalog) => println!("catalog ok: {} entries", catalog.entries.len()),
        Err(e) => fail(&e.to_string()),
    }
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}
