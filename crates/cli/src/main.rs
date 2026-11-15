//! Headless runner: simulates the demo scenario without graphics.

mod catalog_doc;
mod gen_map;
mod gen_textures;
mod lan;
mod path_bench;
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
        /// Output folder (default: data/maps/<name>).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Loads a map folder and reports load time, memory and height range.
    MapInfo {
        /// Map folder, e.g. data/maps/test_large.
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
    /// Writes docs/UNIT_CATALOG.md (or the given path) from the built-in catalog.
    CatalogDoc {
        #[arg(default_value = catalog_doc::DOC_PATH)]
        out: std::path::PathBuf,
    },
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
        CliCommand::GenMap {
            name,
            size_m,
            seed,
            biome,
            out,
        } => {
            let out = out.unwrap_or_else(|| Path::new(MAPS_DIR).join(&name));
            let options = gen_map::Options {
                name: &name,
                size_m,
                seed,
                biome: &biome,
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
        CliCommand::CatalogDoc { out } => {
            let catalog = sim::Catalog::builtin().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(&out, catalog_doc::render(&catalog))
                .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", out.display())));
            println!("wrote {}", out.display());
        }
    }
}

fn replay(path: &std::path::Path) {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
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
