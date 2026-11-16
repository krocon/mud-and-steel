//! The computer player's heavy rocket strike (prompt 054, Normal and Hard):
//! builds the rocket battery position shortly before the strike unlocks,
//! calls the strike on the largest enemy group or a known enemy building
//! away from own units, and answers enemy strikes: own units leave the
//! marked area, and the Hard AI sends armor against the revealed battery
//! position. Ordinary commands only.

use crate::{Ai, Difficulty, PlayerView, Role, SeenUnit, artillery::largest_group, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, UnitId, Vec2};
use std::collections::BTreeMap;

/// The battery is placed this many ticks before the strike unlocks (its
/// build time and a margin).
const BUILD_LEAD_TICKS: u64 = 120 * 20;
/// A group needs this many units to be worth the strike.
const MIN_GROUP: usize = 4;
/// Units leave the marked area this far beyond its radius (m).
const EVADE_MARGIN_M: i32 = 40;
/// The battery position (`building` of data/strike.toml).
const BATTERY: &str = "base_rocket_battery";
/// Units sent against an enemy battery position (Hard).
const RAIDERS: usize = 4;

/// What the computer player remembers about heavy strikes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct StrikeState {
    /// Own units busy with a strike (leaving the area, attacking the
    /// position) until the tick given; the groups leave them alone.
    busy: BTreeMap<UnitId, u64>,
}

impl StrikeState {
    pub(crate) fn busy(&self, unit: UnitId, tick: u64) -> bool {
        self.busy.get(&unit).is_some_and(|&until| tick < until)
    }
}

impl Ai {
    pub(crate) fn think_strike(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !matches!(self.difficulty, Difficulty::Normal | Difficulty::Hard) {
            return;
        }
        let tick = view.tick;
        self.strike.busy.retain(|_, until| tick < *until);
        self.answer_strikes(view, out);
        if !Ai::has_base(view) {
            return;
        }
        self.build_battery(view, out);
        self.call_strike(view, out);
    }

    /// Places the battery position once the vehicle works stand and the
    /// strike unlocks soon.
    fn build_battery(&self, view: &PlayerView, out: &mut Vec<Command>) {
        let Some(battery) = self.type_index_of(BATTERY) else {
            return;
        };
        if u64::from(view.strike.unlock_left) > BUILD_LEAD_TICKS
            || view.structures.iter().any(|s| s.unit_type == battery)
            || !view
                .structures
                .iter()
                .any(|s| s.completed() && self.name_of(s.unit_type) == "base_factory")
        {
            return;
        }
        let Some(&(_, position)) = view.sites.iter().find(|(t, _)| *t == battery) else {
            return;
        };
        out.push(Command::Build {
            unit_type: battery,
            position,
        });
    }

    /// Calls the strike on the largest enemy group, else on a visible enemy
    /// building, else on the enemy's base, never near own or allied units.
    fn call_strike(&self, view: &PlayerView, out: &mut Vec<Command>) {
        if view.strike.blocked.is_some() {
            return;
        }
        // Keep the base going: only spend what is left after twice the
        // price.
        if !view
            .account
            .iter()
            .zip(view.strike.cost)
            .all(|(&have, c)| have >= c.saturating_mul(2))
        {
            return;
        }
        let safe = i32::try_from(view.strike.radius_m).unwrap_or(60) + 60;
        let own: Vec<Vec2> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .map(|u| u.position)
            .chain(view.allies.iter().filter(|a| a.alive()).map(|a| a.position))
            .collect();
        let near_own = |p: Vec2| own.iter().any(|&q| dist(p, q) <= Fixed::from_num(safe));
        let enemies: Vec<&SeenUnit> = view
            .enemies
            .iter()
            .filter(|e| e.alive() && self.type_info(e.unit_type).role != Role::Structure)
            .collect();
        let group = largest_group(&enemies, &own, safe).filter(|&center| {
            let radius = Fixed::from_num(crate::artillery::CLUSTER_M * 2);
            enemies
                .iter()
                .filter(|e| dist(e.position, center) <= radius)
                .count()
                >= MIN_GROUP
        });
        let building = || {
            view.enemies
                .iter()
                .filter(|e| e.alive() && self.type_info(e.unit_type).role == Role::Structure)
                .map(|e| e.position)
                .find(|&p| !near_own(p))
        };
        // Else the enemy's base (its start is known from the map, like a
        // human player knows it).
        let base = || Some(self.enemy_home).filter(|&p| !near_own(p));
        if let Some(target) = group.or_else(building).or_else(base) {
            out.push(Command::HeavyStrike {
                target: self.clamp_to_map(target),
            });
        }
    }

    /// Enemy strikes: own units in the marked area leave it; the Hard AI
    /// attacks the revealed battery position with its nearest armor.
    fn answer_strikes(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        for s in &view.strikes {
            if !view.enemy_players.iter().any(|&(p, _)| p == s.owner) {
                continue;
            }
            let until = view.tick + u64::from(s.warning_left) + 30 * 20;
            let reach = s.radius + Fixed::from_num(EVADE_MARGIN_M);
            let mut groups: BTreeMap<(i64, i64), Vec<UnitId>> = BTreeMap::new();
            for u in view.own.iter().filter(|u| u.alive() && u.carrier.is_none()) {
                let role = self.type_info(u.unit_type).role;
                if role == Role::Structure || self.strike.busy(u.id, view.tick) {
                    continue;
                }
                let d = dist(u.position, s.target);
                if d > reach {
                    continue;
                }
                let to = self.clamp_to_map(toward(s.target, u.position, reach - d));
                // Units close together share one move.
                let key = ((to.x / 20).to_num::<i64>(), (to.y / 20).to_num::<i64>());
                groups.entry(key).or_default().push(u.id);
                self.strike.busy.insert(u.id, until);
            }
            for (key, units) in groups {
                let target = Vec2::new(Fixed::from_num(key.0 * 20), Fixed::from_num(key.1 * 20));
                out.push(Command::MoveTo { units, target });
            }
            if self.difficulty != Difficulty::Hard || s.warning_left == 0 {
                continue;
            }
            if !view.enemies.iter().any(|e| e.id == s.battery && e.alive()) {
                continue;
            }
            let mut armor: Vec<(Fixed, UnitId)> = view
                .own
                .iter()
                .filter(|u| u.alive() && self.type_info(u.unit_type).role == Role::Armor)
                .filter(|u| !self.strike.busy(u.id, view.tick))
                .map(|u| (dist(u.position, s.from), u.id))
                .collect();
            armor.sort_unstable();
            let raiders: Vec<UnitId> = armor.iter().take(RAIDERS).map(|&(_, id)| id).collect();
            if raiders.is_empty() {
                continue;
            }
            for &id in &raiders {
                self.strike.busy.insert(id, until);
            }
            out.push(Command::Attack {
                units: raiders,
                target: s.battery,
            });
        }
    }

    fn type_index_of(&self, name: &str) -> Option<u32> {
        self.names
            .iter()
            .position(|n| n == name)
            .and_then(|i| u32::try_from(i).ok())
    }
}
