//! Groups drive together (prompt 187). Rules: `[march]` in
//! `data/tactics.toml` (`crate::tactics::MarchRules`).
//!
//! A formation move that attack-moves (right click, `Engage::Infantry` or
//! `All`) and `Secure` mark their units with the id of the group's path
//! request (`Drill::march`); the plain move (M, `Engage::Never`) and
//! `MoveTo` do not. Every tick before the movement, [`Sim::march_caps`]
//! gives each marching member a speed cap:
//!
//! - **Pace:** at most the speed the slowest member type in the group
//!   would have on the ground under this member (terrain speed, soft
//!   ground, stance), so on a road the trucks wait for the tanks and in a
//!   field everybody walks with the infantry.
//! - **Together:** members compare the length of the route still ahead of
//!   them (to their own slot, so a deep formation keeps its depth). One
//!   ahead of the rearmost by more than `lead_mm` drives at `slow_pct` of
//!   the pace, by more than `far_mm` at `far_pct`, never below
//!   `min_speed_mm_per_s`. Nobody stops to wait: a waiting member stood
//!   on decks and in lanes in the way of the others behind it, which
//!   then stalled and gave up (stuck audit). On and near an intact bridge
//!   (the 25 m zone where columns queue) every member drives at its own
//!   speed, so the deck is clear again soon: vehicles crawling over it at
//!   infantry pace kept other groups waiting until they stalled. A member behind the
//!   middle of the group by more than `catch_up_mm` drives at its own
//!   full speed.
//! - **Column at bridges:** a member within `gate_mm` of an intact
//!   bridge's deck zone (the 25 m around the deck where columns queue),
//!   still outside it and heading for the bridge, waits while the member
//!   just ahead of it (by route still ahead) is less than `gap_mm` in
//!   front. So the group crosses one after another in the order of their
//!   distance, and whoever waits does so before the deck, not on it. The
//!   front member never waits, so the column cannot lock itself. It also
//!   waits while a unit in the deck zone drives towards it (one-lane
//!   decks take one direction at a time); waiting members stand outside
//!   the zone, so two columns cannot wait for each other.
//! - Only members that follow a route, drive (`speed_pct > 0`), ride on
//!   nothing and are not fighting (a target of their own, or fired at
//!   within `discipline.return_remember_ms`) count; the others leave the
//!   pace while that lasts; a member that arrives or stops leaves the
//!   march.
//!
//! State: only the march id in `Drill` (hashed with the drill). The caps
//! (and who waits, `Sim::march_waiting`) are derived each tick.

use super::{
    NEAR_BRIDGE_M, Route, Sim, Unit, bridge_within, current_waypoint, heading, near_bridge,
};
use crate::math::{Fixed, Vec2};

/// Length of the route still ahead of `unit` (to its current waypoint,
/// then along the rest).
fn route_left(unit: &Unit) -> Fixed {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return Fixed::ZERO;
    };
    let mut left = (waypoints[*next] - unit.position).length();
    for pair in waypoints[*next..].windows(2) {
        left += (pair[1] - pair[0]).length();
    }
    left
}

/// A member type's speeds: cross-country, road (m per tick), stance
/// percent and path profile (soft ground).
pub(super) type Pace = (Fixed, Fixed, u32, usize);

impl Sim {
    /// Whether `unit` waited in front of a bridge for its group in the
    /// last tick (prompt 187; for tools such as the stuck audit).
    pub fn waits_for_group(&self, unit: crate::command::UnitId) -> bool {
        self.units
            .binary_search_by_key(&unit, |u| u.id)
            .is_ok_and(|i| self.march_waiting.get(i) == Some(&true))
    }

    /// Whether unit `i` counts for its march this tick.
    fn marching(&self, i: usize) -> bool {
        let u = &self.units[i];
        u.drill.march != 0
            && u.alive()
            && u.link.is_none()
            && u.speed_pct > 0
            && matches!(u.route, Route::Following { .. })
            && u.combat.attack.is_none()
            && u.combat.auto_target.is_none()
            && u.under_fire.attackers_empty()
    }

    /// The bridge (its center) unit `i` is about to cross: it stands
    /// within `gate` of an intact bridge's deck, outside its zone
    /// (`near_bridge`), and its next waypoint lies towards the bridge.
    fn before_bridge(&self, i: usize, gate: Fixed) -> Option<Vec2> {
        let unit = &self.units[i];
        let at = unit.position;
        if near_bridge(&self.map, at) {
            return None;
        }
        let center = bridge_within(&self.map, at, gate)?;
        let next = current_waypoint(unit)?;
        let (to_bridge, to_next) = (center - at, next - at);
        (to_bridge.x * to_next.x + to_bridge.y * to_next.y > Fixed::ZERO).then_some(center)
    }

    /// Whether a unit in the zone of the bridge at `center` drives towards
    /// unit `i` (against its way): one-lane decks take one direction at a
    /// time, so `i` lets it come off first.
    fn oncoming(&self, i: usize, center: Vec2, near: &mut Vec<u32>) -> bool {
        let Some(mine) = heading(&self.units[i]) else {
            return false;
        };
        let r = Fixed::from_num(NEAR_BRIDGE_M * 3);
        self.unit_grid
            .query(center - Vec2::new(r, r), center + Vec2::new(r, r), near);
        near.iter().any(|&j| {
            let other = &self.units[j as usize];
            j as usize != i
                && other.alive()
                && other.link.is_none()
                && heading(other).is_some_and(|h| mine.x * h.x + mine.y * h.y < Fixed::ZERO)
                && bridge_within(&self.map, other.position, Fixed::from_num(NEAR_BRIDGE_M))
                    == Some(center)
        })
    }

    /// Speed of a member type on the ground at `at` (m per tick), before
    /// the grade; `None` if it cannot drive there.
    pub(super) fn pace_at(&self, pace: Pace, at: Vec2) -> Option<Fixed> {
        let (offroad, road, pct, profile) = pace;
        let id = usize::from(self.map.terrain_at(at));
        let mut speed = self.terrain.types[id].speed(offroad, road);
        if let Some(&g) = self.paths.grid.ground_pct(profile).get(id) {
            speed = speed * Fixed::from_num(g) / 100;
        }
        speed = speed * Fixed::from_num(pct) / 100;
        (speed > Fixed::ZERO).then_some(speed)
    }

    /// Speed caps (m per tick) of the units for this tick by index: `None`
    /// drives at its own speed, `Some(0)` waits in front of a bridge.
    /// Empty if nobody marches.
    /// Units that arrived or stopped leave their march here.
    pub(super) fn march_caps(&mut self) -> Vec<Option<Fixed>> {
        for unit in &mut self.units {
            if unit.drill.march != 0 && unit.route == Route::None {
                unit.drill.march = 0;
            }
        }
        let rules = self.tactics.rules.march;
        if rules.enabled == 0 {
            return Vec::new();
        }
        let mut members: Vec<(u32, usize)> = (0..self.units.len())
            .filter(|&i| self.marching(i))
            .map(|i| (self.units[i].drill.march, i))
            .collect();
        if members.is_empty() {
            return Vec::new();
        }
        members.sort_unstable();
        let mm = |v: u32| Fixed::from_num(v) / 1000;
        let (lead, far, catch_up) = (mm(rules.lead_mm), mm(rules.far_mm), mm(rules.catch_up_mm));
        let floor = mm(rules.min_speed_mm_per_s) / Fixed::from_num(self.tick_hz.max(1));
        let (gate, gap) = (
            Fixed::from_num(NEAR_BRIDGE_M) + mm(rules.gate_mm),
            mm(rules.gap_mm),
        );
        let mut caps = vec![None; self.units.len()];
        let mut near = Vec::new();
        for group in members.chunk_by(|a, b| a.0 == b.0) {
            if group.len() < 2 {
                continue;
            }
            let mut paces: Vec<Pace> = group
                .iter()
                .map(|&(_, i)| {
                    let u = &self.units[i];
                    (
                        u.speed_offroad,
                        u.speed_road,
                        u.speed_pct,
                        u.unit_type as usize,
                    )
                })
                .collect();
            paces.sort_unstable();
            paces.dedup();
            let left: Vec<Fixed> = group
                .iter()
                .map(|&(_, i)| route_left(&self.units[i]))
                .collect();
            let mut sorted = left.clone();
            sorted.sort_unstable();
            let (rear, middle) = (sorted[sorted.len() - 1], sorted[sorted.len() / 2]);
            // Front first (ties by index): who is just ahead of whom.
            let mut order: Vec<(Fixed, usize)> =
                left.iter().zip(group).map(|(&l, &(_, i))| (l, i)).collect();
            order.sort_unstable();
            for (k, &(l, i)) in order.iter().enumerate() {
                let at = self.units[i].position;
                if let Some(center) = self.before_bridge(i, gate)
                    && ((k > 0 && l - order[k - 1].0 < gap) || self.oncoming(i, center, &mut near))
                {
                    caps[i] = Some(Fixed::ZERO);
                    continue;
                }
                // On and near a deck everybody drives at its own speed, so
                // the deck is clear again soon.
                if l - middle > catch_up || near_bridge(&self.map, at) {
                    continue;
                }
                let Some(pace) = paces.iter().filter_map(|&p| self.pace_at(p, at)).min() else {
                    continue;
                };
                let ahead = rear - l;
                let pct = if ahead <= lead {
                    100
                } else if ahead > far {
                    rules.far_pct
                } else {
                    rules.slow_pct
                };
                let slowed = pace * Fixed::from_num(pct) / 100;
                caps[i] = Some(if pct < 100 {
                    slowed.max(floor.min(pace))
                } else {
                    pace
                });
            }
        }
        caps
    }
}
