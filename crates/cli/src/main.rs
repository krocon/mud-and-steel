//! Headless runner: simulates the demo scenario without graphics.

mod catalog_doc;

use clap::{Parser, Subcommand};

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

fn main() {
    match Cli::parse().command {
        CliCommand::Run {
            ticks,
            seed,
            verbose,
        } => {
            let sim = sim::scenario::run_demo(seed, ticks);
            if verbose {
                for unit in sim.snapshot().units {
                    println!(
                        "unit {} at ({}, {}) target {:?}",
                        unit.id.0,
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
        CliCommand::CatalogCheck { files } => catalog_check(&files),
        CliCommand::CatalogDoc { out } => {
            let catalog = sim::Catalog::builtin().unwrap_or_else(|e| fail(&e.to_string()));
            std::fs::write(&out, catalog_doc::render(&catalog))
                .unwrap_or_else(|e| fail(&format!("cannot write {}: {e}", out.display())));
            println!("wrote {}", out.display());
        }
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
