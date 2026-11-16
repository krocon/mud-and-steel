//! `cli balance`: AI tournaments over faction pairings, maps and modes
//! with the post-match statistics (prompt 045, docs/BALANCE.md).
//!
//! Every pairing is played in both orders (faction X as player 0, then as
//! player 1, half the games each), so start positions do not favor a
//! faction. With the same difficulty on both sides the win rates measure
//! the factions, not the AIs. Prints Markdown tables: wins per faction
//! pairing, kills and losses per unit type, and the economy per faction.

use crate::ai_tournament::{self, Balance, EconomySums, Options};
use ai::Difficulty;
use std::collections::BTreeMap;
use std::fmt::Write as _;

pub struct BalanceOptions {
    pub maps: Vec<String>,
    pub modes: Vec<String>,
    /// Faction pairings such as `de-su`.
    pub pairs: Vec<String>,
    pub a: Difficulty,
    pub b: Difficulty,
    pub games: u64,
    pub seed: u64,
    pub battle_ticks: u64,
    pub base_ticks: u64,
    pub jobs: usize,
}

struct Row {
    mode: String,
    map: String,
    x: String,
    y: String,
    games: u64,
    x_wins: u64,
    y_wins: u64,
    undecided: u64,
    mean_s: u64,
}

/// Plays everything and returns the report as Markdown.
pub fn run(options: &BalanceOptions) -> String {
    let mut rows: Vec<Row> = Vec::new();
    let mut total = Balance::default();
    let mut by_mode: BTreeMap<String, Balance> = BTreeMap::new();
    for mode in &options.modes {
        let scenario = match mode.as_str() {
            "battle" => sim::scenario::DEMO_SCENARIO,
            "base" => sim::scenario::SKIRMISH_SCENARIO,
            other => crate::fail(&format!("unknown mode '{other}' (battle, base)")),
        };
        let max_ticks = if mode == "base" {
            options.base_ticks
        } else {
            options.battle_ticks
        };
        for map_name in &options.maps {
            let map = crate::find_map(map_name);
            for pair in &options.pairs {
                let (x, y) = pair
                    .split_once('-')
                    .unwrap_or_else(|| crate::fail(&format!("pairing '{pair}' is not like de-su")));
                let mut row = Row {
                    mode: mode.clone(),
                    map: map_name.clone(),
                    x: x.to_string(),
                    y: y.to_string(),
                    games: 0,
                    x_wins: 0,
                    y_wins: 0,
                    undecided: 0,
                    mean_s: 0,
                };
                let mut decided_ticks = 0u64;
                let mut decided = 0u64;
                for (i, order) in [[x, y], [y, x]].iter().enumerate() {
                    let factions: Vec<String> = order.iter().map(|f| (*f).to_string()).collect();
                    let config = crate::tournament_config(&map, &factions, scenario)
                        .unwrap_or_else(|e| crate::fail(&e));
                    let half = options.games / 2 + u64::from(i == 0) * (options.games % 2);
                    let tournament = Options {
                        games: half,
                        seed: options.seed + i as u64 * 1000,
                        max_ticks,
                        jobs: options.jobs,
                        verbose: false,
                        trace: 0,
                    };
                    let s = ai_tournament::series(&config, &map, options.a, options.b, &tournament);
                    row.games += s.games;
                    row.undecided += s.timeouts + s.draws;
                    let wins = |f: &str| s.balance.factions.get(f).map_or(0, |&(_, w)| w);
                    if x == y {
                        // Mirror: AI A's wins are "X", B's "Y".
                        row.x_wins += s.a_wins;
                        row.y_wins += s.b_wins;
                    } else {
                        row.x_wins += wins(x);
                        row.y_wins += wins(y);
                    }
                    let n = s.a_wins + s.b_wins + s.draws;
                    decided_ticks += s.mean_tick * n;
                    decided += n;
                    merge(&mut total, &s.balance);
                    merge(by_mode.entry(mode.clone()).or_default(), &s.balance);
                }
                row.mean_s = decided_ticks.checked_div(decided * 20).unwrap_or(0);
                eprintln!(
                    "{} {} {}-{}: {} / {} / {} of {}",
                    row.mode,
                    row.map,
                    row.x,
                    row.y,
                    row.x_wins,
                    row.y_wins,
                    row.undecided,
                    row.games
                );
                rows.push(row);
            }
        }
    }
    report(options, &rows, &total, &by_mode)
}

fn merge(into: &mut Balance, from: &Balance) {
    for (f, &(g, w)) in &from.factions {
        let e = into.factions.entry(f.clone()).or_default();
        e.0 += g;
        e.1 += w;
    }
    for (t, c) in &from.types {
        let e = into.types.entry(t.clone()).or_default();
        e.produced += c.produced;
        e.lost += c.lost;
        e.kills += c.kills;
        e.kill_value += c.kill_value;
    }
    for (f, x) in &from.economy {
        let e = into.economy.entry(f.clone()).or_default();
        e.players += x.players;
        for i in 0..3 {
            e.account[i] += x.account[i];
            e.income[i] += x.income[i];
        }
        e.points_captured += x.points_captured;
        e.produced_value += x.produced_value;
        e.lost_value += x.lost_value;
    }
}

fn pct(n: u64, of: u64) -> u64 {
    (n * 100).checked_div(of).unwrap_or(0)
}

fn report(
    options: &BalanceOptions,
    rows: &[Row],
    total: &Balance,
    by_mode: &BTreeMap<String, Balance>,
) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{} vs {}, {} games per row (half with each faction as player 0), seeds from {}, battle at most {} ticks, base at most {} ticks.\n",
        options.a, options.b, options.games, options.seed, options.battle_ticks, options.base_ticks
    );
    s.push_str("| Mode | Map | X – Y | X wins | Y wins | undecided | mean decided |\n");
    s.push_str("|---|---|---|---|---|---|---|\n");
    for r in rows {
        let _ = writeln!(
            s,
            "| {} | {} | {} – {} | {} % | {} % | {} % | {} s |",
            r.mode,
            r.map,
            r.x,
            r.y,
            pct(r.x_wins, r.games),
            pct(r.y_wins, r.games),
            pct(r.undecided, r.games),
            r.mean_s
        );
    }
    s.push_str(
        "\nWin rate per faction (games won / played; mirror matches count both players):\n\n",
    );
    s.push_str("| Mode | Faction | played | won | win rate |\n|---|---|---|---|---|\n");
    for (mode, b) in by_mode {
        for (f, &(g, w)) in &b.factions {
            let _ = writeln!(s, "| {mode} | {f} | {g} | {w} | {} % |", pct(w, g));
        }
    }
    s.push_str("\nUnit types over all games (kills per loss: how much a unit takes with it):\n\n");
    s.push_str("| Unit | produced | lost | kills | kills per loss | killed value |\n|---|---|---|---|---|---|\n");
    let mut types: Vec<(&String, &sim::stats::TypeCounts)> = total.types.iter().collect();
    types.sort_by(|a, b| b.1.kills.cmp(&a.1.kills).then(a.0.cmp(b.0)));
    for (t, c) in types {
        if c.kills == 0 && c.lost == 0 {
            continue;
        }
        let ratio = (u64::from(c.kills) * 100).checked_div(u64::from(c.lost));
        let _ = writeln!(
            s,
            "| `{t}` | {} | {} | {} | {} | {} |",
            c.produced,
            c.lost,
            c.kills,
            ratio.map_or_else(
                || "–".to_string(),
                |r| format!("{}.{:02}", r / 100, r % 100)
            ),
            c.kill_value
        );
    }
    if let Some(base) = by_mode.get("base") {
        s.push_str(
            "\nEconomy per faction in base matches (means per player at the end of the game):\n\n",
        );
        s.push_str("| Faction | account fuel / ammo / manpower | income per min | points taken | spent on units | value lost |\n|---|---|---|---|---|---|\n");
        for (f, e) in &base.economy {
            let n = e.players.max(1);
            let EconomySums {
                account,
                income,
                points_captured,
                produced_value,
                lost_value,
                ..
            } = *e;
            let _ = writeln!(
                s,
                "| {f} | {} / {} / {} | {} / {} / {} | {}.{} | {} | {} |",
                account[0] / n,
                account[1] / n,
                account[2] / n,
                income[0] / n,
                income[1] / n,
                income[2] / n,
                points_captured / n,
                points_captured * 10 / n % 10,
                produced_value / n,
                lost_value / n
            );
        }
    }
    s
}
