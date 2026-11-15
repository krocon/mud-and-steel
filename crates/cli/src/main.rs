//! Headless runner: simulates the demo scenario without graphics.

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
    }
}
