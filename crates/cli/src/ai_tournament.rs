//! `cli ai-tournament`: computer players against each other over many
//! seeds, headless (prompt 024). Every game runs the AIs through a lockstep
//! session exactly like the game; sides (and with them factions) alternate
//! from game to game. Games run in parallel threads; the results do not
//! depend on that (every game is its own deterministic simulation).

use ai::Difficulty;
use ai::game::{Game, GameResult};
use sim::{PlayerId, SimConfig, StableHasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Options {
    pub games: u64,
    pub seed: u64,
    pub max_ticks: u64,
    pub jobs: usize,
    /// Print every game.
    pub verbose: bool,
    /// Print the living units every this many ticks (0: never; for tuning
    /// with `--games 1`).
    pub trace: u64,
}

/// One game with the time the AIs spent thinking.
struct Played {
    result: GameResult,
    /// Player of AI `a`.
    a_player: PlayerId,
    /// Time in `Game::think` (both AIs), and its maximum per tick.
    think: Duration,
    think_max: Duration,
    /// Living units at the end of a timeout (`--verbose`).
    survivors: Vec<String>,
    /// Post-match statistics (prompt 045).
    stats: sim::stats::MatchStats,
}

/// Summary of a series of `a` against `b`.
pub struct Summary {
    pub a: Difficulty,
    pub b: Difficulty,
    pub games: u64,
    pub a_wins: u64,
    pub b_wins: u64,
    pub draws: u64,
    pub timeouts: u64,
    /// Mean tick of decided games.
    pub mean_tick: u64,
    /// Mean units left to the winner.
    pub mean_left_x100: u64,
    pub think_us_per_tick: u64,
    pub think_max_us: u64,
    pub hash: u64,
    /// Prompt 045: statistics summed over all games.
    pub balance: Balance,
}

/// Statistics of a series by faction and unit type (prompt 045).
#[derive(Default)]
pub struct Balance {
    /// Faction id -> (games played, games won). A mirror match counts
    /// both players.
    pub factions: std::collections::BTreeMap<String, (u64, u64)>,
    /// Unit type -> counts summed over all games and players.
    pub types: std::collections::BTreeMap<String, sim::stats::TypeCounts>,
    /// Faction -> sum over games of the account at the end and of the
    /// income at the end, and supply points taken.
    pub economy: std::collections::BTreeMap<String, EconomySums>,
}

#[derive(Default, Clone, Copy)]
pub struct EconomySums {
    pub players: u64,
    pub account: [u64; 3],
    pub income: [u64; 3],
    pub points_captured: u64,
    pub produced_value: u64,
    pub lost_value: u64,
}

impl Balance {
    fn add(
        &mut self,
        stats: &sim::stats::MatchStats,
        winner: Option<PlayerId>,
        config: &SimConfig,
    ) {
        for p in stats.players() {
            let faction = p.faction.unwrap_or("?").to_string();
            let entry = self.factions.entry(faction.clone()).or_default();
            entry.0 += 1;
            if winner.is_some_and(|w| config.teams.allied(w, p.player)) {
                entry.1 += 1;
            }
            for (&t, c) in &p.types {
                let sum = self
                    .types
                    .entry(stats.type_name(t).to_string())
                    .or_default();
                sum.produced += c.produced;
                sum.lost += c.lost;
                sum.kills += c.kills;
                sum.kill_value += c.kill_value;
            }
            let e = self.economy.entry(faction).or_default();
            e.players += 1;
            if let Some(last) = p.samples.last() {
                for i in 0..3 {
                    e.account[i] += u64::from(last.account[i]);
                    e.income[i] += u64::from(last.income[i]);
                }
            }
            e.points_captured += u64::from(p.points_captured);
            e.produced_value += p.produced_value;
            e.lost_value += p.lost_value;
        }
    }
}

fn play(
    config: &SimConfig,
    map: &Arc<sim::Map>,
    seed: u64,
    a: Difficulty,
    b: Difficulty,
    max_ticks: u64,
    trace: u64,
) -> Played {
    // A plays player 0 (and its allies, prompt 037) in even games, player
    // 1 (and its allies) in odd ones.
    let a_player = PlayerId(u8::try_from(seed % 2).unwrap_or(0));
    let levels: Vec<(PlayerId, Difficulty)> = config
        .players()
        .into_iter()
        .map(|p| {
            (
                p,
                if config.teams.allied(p, a_player) {
                    a
                } else {
                    b
                },
            )
        })
        .collect();
    let mut game =
        Game::new(config, map.clone(), seed, &levels).unwrap_or_else(|e| crate::fail(&e));
    let mut think = Duration::ZERO;
    let mut think_max = Duration::ZERO;
    while game.sim().outcome().is_none() && game.sim().tick() < max_ticks {
        let start = Instant::now();
        let decisions = game.think();
        let spent = start.elapsed();
        think += spent;
        think_max = think_max.max(spent);
        game.step(decisions);
        if trace > 0 && game.sim().tick().is_multiple_of(trace) {
            println!("tick {}:", game.sim().tick());
            for ai in game.ais() {
                if ai.difficulty() != Difficulty::Passive {
                    println!("  {}", ai.status());
                }
                let sim = game.sim();
                let buildings: Vec<String> = sim
                    .structures()
                    .iter()
                    .filter(|s| s.owner == ai.player())
                    .map(|s| {
                        format!(
                            "{}{} {:?}",
                            config.unit_types[s.unit_type as usize].name,
                            if s.completed() { "" } else { "(site)" },
                            s.queue
                        )
                    })
                    .collect();
                if !buildings.is_empty() {
                    println!(
                        "  p{} account {:?} buildings {buildings:?}",
                        ai.player().0,
                        sim.account(ai.player()).unwrap_or_default()
                    );
                }
            }
            for line in survivors(game.sim(), config) {
                println!("  {line}");
            }
        }
    }
    game.finish_stats();
    let result = game.result();
    let survivors = if result.decided_tick.is_none() {
        survivors(game.sim(), config)
    } else {
        Vec::new()
    };
    Played {
        result,
        a_player,
        think,
        think_max,
        survivors,
        stats: game.stats().clone(),
    }
}

/// Living mobile units with type, position, hit points and stocks: why a
/// game did not end.
fn survivors(sim: &sim::Sim, config: &SimConfig) -> Vec<String> {
    sim.snapshot()
        .units
        .iter()
        .filter(|u| u.hp > 0)
        .filter(|u| {
            config
                .unit_types
                .get(u.unit_type as usize)
                .is_some_and(sim::UnitType::mobile)
        })
        .map(|u| {
            format!(
                "p{} {} at ({}, {}) hp {}/{} ammo {:?} fuel {:?}{}",
                u.owner.0,
                config.unit_types[u.unit_type as usize].name,
                u.position.x.to_num::<i32>(),
                u.position.y.to_num::<i32>(),
                u.hp,
                u.max_hp,
                u.stock.ammo_pct,
                u.stock.fuel_pct,
                if u.stock.immobile { " immobile" } else { "" },
            )
        })
        .collect()
}

/// Plays a series and prints it.
pub fn series(
    config: &SimConfig,
    map: &Arc<sim::Map>,
    a: Difficulty,
    b: Difficulty,
    options: &Options,
) -> Summary {
    let seeds: Vec<u64> = (0..options.games).map(|i| options.seed + i).collect();
    let jobs = options.jobs.max(1);
    let mut played: Vec<Played> = Vec::new();
    for chunk in seeds.chunks(jobs) {
        let results: Vec<Played> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&seed| {
                    scope.spawn(move || {
                        play(config, map, seed, a, b, options.max_ticks, options.trace)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("game thread"))
                .collect()
        });
        played.extend(results);
    }

    let mut summary = Summary {
        a,
        b,
        games: options.games,
        a_wins: 0,
        b_wins: 0,
        draws: 0,
        timeouts: 0,
        mean_tick: 0,
        mean_left_x100: 0,
        think_us_per_tick: 0,
        think_max_us: 0,
        hash: 0,
        balance: Balance::default(),
    };
    let mut hasher = StableHasher::default();
    let mut ticks_sum = 0u64;
    let mut decided = 0u64;
    let mut left_sum = 0u64;
    let mut think = Duration::ZERO;
    let mut all_ticks = 0u64;
    for p in &played {
        let r = &p.result;
        r.hash_into(&mut hasher);
        summary.balance.add(&p.stats, r.winner, config);
        let label = match (r.decided_tick, r.winner) {
            (None, _) => {
                summary.timeouts += 1;
                "timeout".to_string()
            }
            (Some(_), None) => {
                summary.draws += 1;
                "draw".to_string()
            }
            (Some(_), Some(w)) => {
                // Units left to the winning team.
                let left: usize = r
                    .alive
                    .iter()
                    .filter(|&&(q, _)| config.teams.allied(q, w))
                    .map(|&(_, n)| n)
                    .sum();
                left_sum += left as u64;
                if config.teams.allied(w, p.a_player) {
                    summary.a_wins += 1;
                    format!("{a} wins")
                } else {
                    summary.b_wins += 1;
                    format!("{b} wins")
                }
            }
        };
        if let Some(t) = r.decided_tick {
            ticks_sum += t;
            decided += 1;
        }
        think += p.think;
        all_ticks += r.ticks;
        summary.think_max_us = summary.think_max_us.max(p.think_max.as_micros() as u64);
        if options.verbose {
            println!(
                "  seed {:>4}: {a} as player {} | {label:<12} tick {:>5} | units left {:?} | commands {:?}",
                r.seed,
                p.a_player.0,
                r.ticks,
                r.alive.iter().map(|&(_, n)| n).collect::<Vec<_>>(),
                r.commands.iter().map(|&(_, n)| n).collect::<Vec<_>>(),
            );
            for line in &p.survivors {
                println!("      {line}");
            }
        }
    }
    let wins = summary.a_wins + summary.b_wins;
    summary.mean_tick = ticks_sum.checked_div(decided).unwrap_or(0);
    summary.mean_left_x100 = (left_sum * 100).checked_div(wins).unwrap_or(0);
    summary.think_us_per_tick = (think.as_micros() as u64)
        .checked_div(all_ticks)
        .unwrap_or(0);
    summary.hash = hasher.finish();
    summary
}

pub fn print_header() {
    println!(
        "{:<18} {:>5} {:>7} {:>7} {:>5} {:>7} {:>9} {:>8} {:>13}  hash",
        "pairing",
        "games",
        "A wins",
        "B wins",
        "draw",
        "timeout",
        "mean time",
        "left",
        "AI µs/tick"
    );
}

pub fn print(s: &Summary) {
    let pct = |n: u64| (n * 100).checked_div(s.games).unwrap_or(0);
    println!(
        "{:<18} {:>5} {:>6}% {:>6}% {:>5} {:>7} {:>8}s {:>4}.{:02} {:>6} (max {})  {:#018x}",
        format!("{} vs {}", s.a, s.b),
        s.games,
        pct(s.a_wins),
        pct(s.b_wins),
        s.draws,
        s.timeouts,
        s.mean_tick / 20,
        s.mean_left_x100 / 100,
        s.mean_left_x100 % 100,
        s.think_us_per_tick,
        s.think_max_us,
        s.hash,
    );
}

/// Every difficulty against every lower one (and passive against itself is
/// left out: it never ends).
pub fn matrix(config: &SimConfig, map: &Arc<sim::Map>, options: &Options) -> u64 {
    let mut h = StableHasher::default();
    print_header();
    let all = Difficulty::ALL;
    for (i, &a) in all.iter().enumerate().rev() {
        for &b in all[..i].iter().rev() {
            let s = series(config, map, a, b, options);
            print(&s);
            h.write_u64(s.hash);
        }
    }
    let hash = h.finish();
    println!("tournament hash {hash:#018x}");
    hash
}
