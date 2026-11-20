//! Post-match statistics (prompt 045): losses and kills per unit type,
//! resource curves and supply points per player.
//!
//! [`MatchStats`] only *observes* the simulation: it reads a [`Sim`] after
//! every tick and keeps its own counters, outside the simulation state. It
//! is therefore not part of the state hash, of snapshots or of save games
//! (a loaded game starts its statistics at the tick it was loaded), and the
//! golden hash does not depend on it. The game (`bridge`) and the headless
//! tournaments (`ai::game`, `cli`) feed it the same way, so the numbers of
//! the result screen and of the balance reports are the same.
//!
//! Kills come from [`CombatEvent::Destroyed`]: the unit that fired the last
//! hit gets the kill. Units that die without such an event (heavy strike,
//! fire, mines, aircraft crashes) count as losses with an unknown killer.

use crate::combat::CombatEvent;
use crate::command::{PlayerId, UnitId};
use crate::economy::Resources;
use crate::sim::{Sim, SimConfig};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Statistics are sampled every this many seconds of game time.
pub const SAMPLE_INTERVAL_S: u64 = 30;

/// Counters of one unit type of one player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TypeCounts {
    /// Units of this type that appeared after the start (produced, built,
    /// bailed-out crews).
    pub produced: u32,
    /// Units of this type that were destroyed.
    pub lost: u32,
    /// Enemy units destroyed by units of this type.
    pub kills: u32,
    /// Price (sum of the three resources) of the enemy units they destroyed.
    pub kill_value: u64,
}

/// One sample of a player's economy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub tick: u64,
    /// Account (fuel, ammunition, manpower).
    pub account: Resources,
    /// Income per minute at that moment.
    pub income: Resources,
    /// Supply points held.
    pub points: u32,
    /// Living units (bailed-out crews and buildings included).
    pub units: u32,
}

/// Everything recorded about one player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerStats {
    pub player: PlayerId,
    /// Faction id (`de`, `su`, `us`) if known.
    pub faction: Option<&'static str>,
    /// By unit type index (`SimConfig::unit_types`).
    pub types: BTreeMap<u32, TypeCounts>,
    /// Every [`SAMPLE_INTERVAL_S`] seconds and at the end.
    pub samples: Vec<Sample>,
    /// Supply points taken (from neutral) during the match.
    pub points_captured: u32,
    /// Supply points lost (neutralized by an enemy).
    pub points_lost: u32,
    /// Price of the units that appeared after the start (spent on the army).
    pub produced_value: u64,
    /// Price of the own units destroyed.
    pub lost_value: u64,
}

impl PlayerStats {
    pub fn lost(&self) -> u32 {
        self.types.values().map(|c| c.lost).sum()
    }

    pub fn kills(&self) -> u32 {
        self.types.values().map(|c| c.kills).sum()
    }

    pub fn kill_value(&self) -> u64 {
        self.types.values().map(|c| c.kill_value).sum()
    }
}

/// Statistics of a whole match; see the module documentation.
#[derive(Clone, Debug)]
pub struct MatchStats {
    names: Vec<String>,
    players: Vec<PlayerStats>,
    /// Living units seen at the last observation: owner and type.
    alive: BTreeMap<UnitId, (PlayerId, u32)>,
    /// Owner of every supply point at the last observation.
    point_owners: Vec<Option<PlayerId>>,
    /// Ticks between samples.
    interval: u64,
    started: bool,
    last_tick: u64,
}

impl MatchStats {
    pub fn new(config: &SimConfig) -> Self {
        Self {
            names: config.unit_types.iter().map(|t| t.name.clone()).collect(),
            players: config
                .players()
                .into_iter()
                .map(|player| PlayerStats {
                    player,
                    faction: None,
                    types: BTreeMap::new(),
                    samples: Vec::new(),
                    points_captured: 0,
                    points_lost: 0,
                    produced_value: 0,
                    lost_value: 0,
                })
                .collect(),
            alive: BTreeMap::new(),
            point_owners: Vec::new(),
            interval: SAMPLE_INTERVAL_S * u64::from(config.tick_hz.max(1)),
            started: false,
            last_tick: 0,
        }
    }

    /// Reads `sim` after a tick. The first call only takes the starting
    /// state (units on the map at the start are not "produced").
    pub fn observe(&mut self, sim: &Sim) {
        let tick = sim.tick();
        if !self.started {
            self.started = true;
            for p in &mut self.players {
                p.faction = sim.faction(p.player);
            }
            for (id, owner, unit_type, alive) in sim.unit_roster() {
                if alive {
                    self.alive.insert(id, (owner, unit_type));
                }
            }
            self.point_owners = sim.supply_points().iter().map(|p| p.owner).collect();
            self.sample(sim);
            self.last_tick = tick;
            return;
        }
        if tick == self.last_tick {
            return;
        }
        self.last_tick = tick;
        let mut killers: BTreeMap<UnitId, UnitId> = BTreeMap::new();
        for e in sim.events() {
            if let CombatEvent::Destroyed { unit, by, .. } = *e {
                killers.insert(unit, by);
            }
        }
        let mut died: Vec<(UnitId, PlayerId, u32)> = Vec::new();
        for (id, owner, unit_type, alive) in sim.unit_roster() {
            match (alive, self.alive.contains_key(&id)) {
                (true, false) => {
                    self.alive.insert(id, (owner, unit_type));
                    let value = price(sim, unit_type);
                    if let Some(p) = self.player_mut(owner) {
                        p.types.entry(unit_type).or_default().produced += 1;
                        p.produced_value += value;
                    }
                }
                (false, true) => {
                    self.alive.remove(&id);
                    died.push((id, owner, unit_type));
                }
                _ => {}
            }
        }
        for (id, owner, unit_type) in died {
            let value = price(sim, unit_type);
            if let Some(p) = self.player_mut(owner) {
                p.types.entry(unit_type).or_default().lost += 1;
                p.lost_value += value;
            }
            let Some(by) = killers.get(&id) else {
                continue;
            };
            let Some((killer, killer_type)) = sim
                .unit_roster()
                .find_map(|(u, o, t, _)| (u == *by).then_some((o, t)))
            else {
                continue;
            };
            if sim.allied(killer, owner) {
                continue;
            }
            if let Some(p) = self.player_mut(killer) {
                let c = p.types.entry(killer_type).or_default();
                c.kills += 1;
                c.kill_value += value;
            }
        }
        let points = sim.supply_points();
        for (i, point) in points.iter().enumerate() {
            let before = self.point_owners.get(i).copied().flatten();
            if before == point.owner {
                continue;
            }
            if let Some(old) = before
                && let Some(p) = self.player_mut(old)
            {
                p.points_lost += 1;
            }
            if let Some(new) = point.owner
                && let Some(p) = self.player_mut(new)
            {
                p.points_captured += 1;
            }
        }
        self.point_owners = points.iter().map(|p| p.owner).collect();
        if tick.is_multiple_of(self.interval) || sim.outcome().is_some_and(|o| o.tick == tick) {
            self.sample(sim);
        }
    }

    /// Adds a last sample at the current tick (when a match stops without
    /// a decision, e.g. a tournament timeout or leaving the match).
    pub fn finish(&mut self, sim: &Sim) {
        if self
            .players
            .first()
            .and_then(|p| p.samples.last())
            .is_some_and(|s| s.tick == sim.tick())
        {
            return;
        }
        self.sample(sim);
    }

    fn sample(&mut self, sim: &Sim) {
        let tick = sim.tick();
        let points = sim.supply_points();
        let mut units: BTreeMap<PlayerId, u32> = BTreeMap::new();
        for &(owner, _) in self.alive.values() {
            *units.entry(owner).or_default() += 1;
        }
        for p in &mut self.players {
            p.samples.push(Sample {
                tick,
                account: sim.account(p.player).unwrap_or_default(),
                income: sim.income(p.player),
                points: u32::try_from(points.iter().filter(|q| q.owner == Some(p.player)).count())
                    .unwrap_or(u32::MAX),
                units: units.get(&p.player).copied().unwrap_or(0),
            });
        }
    }

    fn player_mut(&mut self, player: PlayerId) -> Option<&mut PlayerStats> {
        self.players.iter_mut().find(|p| p.player == player)
    }

    pub fn players(&self) -> &[PlayerStats] {
        &self.players
    }

    /// Name (catalog id) of a unit type.
    pub fn type_name(&self, unit_type: u32) -> &str {
        self.names
            .get(unit_type as usize)
            .map_or("?", String::as_str)
    }

    /// The statistics as JSON (integers only; times in ticks and seconds).
    /// `tick_hz` converts ticks to seconds.
    pub fn to_json(&self, sim: &Sim) -> String {
        let hz = u64::from(sim.tick_hz().max(1));
        let mut s = String::new();
        let _ = write!(
            s,
            "{{\n  \"tick\": {},\n  \"seconds\": {},\n",
            sim.tick(),
            sim.tick() / hz
        );
        match sim.outcome() {
            Some(o) => {
                let _ = writeln!(
                    s,
                    "  \"decided\": true,\n  \"winner\": {},\n  \"decided_tick\": {},",
                    o.winner.map_or(-1, |p| i32::from(p.0)),
                    o.tick
                );
            }
            None => s.push_str("  \"decided\": false,\n  \"winner\": null,\n"),
        }
        s.push_str("  \"players\": [");
        for (i, p) in self.players.iter().enumerate() {
            s.push_str(if i == 0 { "\n" } else { ",\n" });
            let _ = write!(
                s,
                "    {{\n      \"player\": {},\n      \"faction\": {},\n      \"lost\": {},\n      \"kills\": {},\n      \"lost_value\": {},\n      \"kill_value\": {},\n      \"produced_value\": {},\n      \"points_captured\": {},\n      \"points_lost\": {},\n      \"types\": [",
                p.player.0,
                p.faction
                    .map_or_else(|| "null".to_string(), |f| format!("\"{f}\"")),
                p.lost(),
                p.kills(),
                p.lost_value,
                p.kill_value(),
                p.produced_value,
                p.points_captured,
                p.points_lost,
            );
            for (j, (&t, c)) in p.types.iter().enumerate() {
                let _ = write!(
                    s,
                    "{}\n        {{ \"type\": \"{}\", \"produced\": {}, \"lost\": {}, \"kills\": {}, \"kill_value\": {} }}",
                    if j == 0 { "" } else { "," },
                    json_escape(self.type_name(t)),
                    c.produced,
                    c.lost,
                    c.kills,
                    c.kill_value
                );
            }
            s.push_str(if p.types.is_empty() {
                "],\n"
            } else {
                "\n      ],\n"
            });
            s.push_str("      \"samples\": [");
            for (j, x) in p.samples.iter().enumerate() {
                let _ = write!(
                    s,
                    "{}\n        {{ \"tick\": {}, \"seconds\": {}, \"account\": [{}, {}, {}], \"income\": [{}, {}, {}], \"points\": {}, \"units\": {} }}",
                    if j == 0 { "" } else { "," },
                    x.tick,
                    x.tick / hz,
                    x.account[0],
                    x.account[1],
                    x.account[2],
                    x.income[0],
                    x.income[1],
                    x.income[2],
                    x.points,
                    x.units
                );
            }
            s.push_str(if p.samples.is_empty() {
                "]\n    }"
            } else {
                "\n      ]\n    }"
            });
        }
        s.push_str("\n  ]\n}\n");
        s
    }
}

/// Price of a unit type as one number (sum of the three resources; 0 if it
/// cannot be bought), the unit value the AI uses as well.
fn price(sim: &Sim, unit_type: u32) -> u64 {
    sim.cost_of(unit_type)
        .map_or(0, |c| c.resources.iter().map(|&r| u64::from(r)).sum())
}

fn json_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlayerCommand;
    use crate::scenario::{builtin_map, demo_config, demo_scripts};

    #[test]
    fn counts_losses_and_kills_of_the_demo() {
        let map = builtin_map();
        let config = demo_config();
        let mut sim = Sim::new(&config, map, 42).unwrap();
        let mut stats = MatchStats::new(&config);
        stats.observe(&sim);
        // The demo's command scripts move the armies about: since prompt
        // 097 (direct fire × 1/8, guns 87.5 m) the tanks start out of each
        // other's reach (120 m) and would not fight standing still.
        let mut scripts = demo_scripts(42, &config);
        let mut ticks = 0;
        while sim.outcome().is_none() && ticks < 6000 {
            let commands: Vec<PlayerCommand> = scripts
                .iter_mut()
                .flat_map(|(player, script)| {
                    let player = *player;
                    script
                        .commands_for_tick(ticks)
                        .into_iter()
                        .map(move |c| PlayerCommand::new(player, c))
                })
                .collect();
            sim.step(&commands);
            stats.observe(&sim);
            ticks += 1;
        }
        stats.finish(&sim);
        let lost: u32 = stats.players().iter().map(PlayerStats::lost).sum();
        let kills: u32 = stats.players().iter().map(PlayerStats::kills).sum();
        assert!(lost > 0, "the demo armies fire at will and lose units");
        assert!(kills <= lost, "every kill is a loss of someone else");
        assert!(kills > 0, "tanks are credited with kills");
        for p in stats.players() {
            assert!(p.samples.len() >= 2, "start and end are sampled");
            assert_eq!(p.samples[0].tick, 0);
        }
        let json = stats.to_json(&sim);
        assert!(json.contains("\"players\""));
        assert!(json.contains("de_pz4_h") || json.contains("su_t34_76_1943"));
        assert_eq!(json.matches('{').count(), json.matches('}').count());
    }
}
