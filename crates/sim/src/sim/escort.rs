//! Infantry advances behind the tanks of its group (prompt 198). Rules:
//! `[escort]` in `data/tactics.toml` (`crate::tactics::EscortRules`).
//!
//! - **Places:** a group order that drives together (attack-move,
//!   `Secure`) in an open formation, with squads and tanks (tank
//!   destroyers, assault guns), while its player sees an enemy near it or
//!   its target, lays only the vehicles out in the formation; every squad
//!   gets a slot behind a tank (`sim/drill.rs`) and remembers the tank and
//!   the distance (`Drill::escort`). It first walks into the tank's track
//!   ([`Sim::escort_join`], a waypoint in `take_path`).
//! - **Following:** while squad and tank drive in the same march
//!   (`sim/march.rs`), the squad is not charging and stays within
//!   `leash_mm`, both away from bridges, the squad escorts the tank
//!   ([`Sim::escort_of`]): its soldiers walk in two files in the track of
//!   the hull ([`Sim::escort_file`]), crouched with the automatic stance
//!   while it or the tank is in a fight ([`Sim::escort_crouches`]), without
//!   bounding or halting to fight; it waits while it is ahead of its place,
//!   and the tank drives at the squad's pace, slower while the squad is
//!   behind and not at all while it is far behind ([`Sim::escort_caps`]).
//! - **Cover:** the hull itself stops the rounds that fly at the soldiers
//!   behind it (rounds hit the first unit on their way, `sim/combat.rs`);
//!   soldiers in its track do not throw themselves down under bursts
//!   (prompt 196), only under blasts.
//!
//! State: only `Drill::escort` (hashed with the drill); everything else is
//! derived each tick.

use super::{NEAR_BRIDGE_M, Route, Sim, bridge_within};
use crate::catalog::Category;
use crate::command::UnitId;
use crate::math::{Fixed, Vec2};
use crate::tactics::EscortRules;

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// The direction of a tank's track: towards its next waypoint while it
/// follows a route (steadier than the hull, which turns with every push
/// aside), else its hull.
fn track(tank: &super::Unit) -> Vec2 {
    super::heading(tank).unwrap_or(tank.facing)
}

impl Sim {
    /// The tank squad `id` advances behind right now (prompt 198; for the
    /// presentation, tools and tests).
    pub fn escorting(&self, id: UnitId) -> Option<UnitId> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        self.escort_of(i).map(|(v, _)| self.units[v].id)
    }

    /// The escort rules of the match.
    pub const fn escort_rules(&self) -> &EscortRules {
        &self.tactics.rules.escort
    }

    /// Replaces the escort rules of the match (tests that compare with
    /// the rule off). Not a player command.
    pub fn set_escort_rules(&mut self, rules: EscortRules) {
        self.tactics.rules.escort = rules;
    }

    /// Whether unit `i` is a vehicle infantry advances behind: tanks, tank
    /// destroyers and assault guns.
    pub(super) fn shields_infantry(&self, i: usize) -> bool {
        let unit = &self.units[i];
        unit.squad.is_none()
            && matches!(
                self.unit_types[unit.unit_type as usize].combat.category,
                Some(
                    Category::TankLight
                        | Category::TankMedium
                        | Category::TankHeavy
                        | Category::TankDestroyer
                        | Category::AssaultGun
                )
            )
    }

    /// Whether the player of the units `group` sees an enemy within
    /// `contact_mm` of one of `points` (prompt 198: the squads of a group
    /// order advance behind its tanks only then).
    pub(super) fn enemy_in_sight(&self, group: &[usize], points: &[Vec2]) -> bool {
        let Some(&first) = group.first() else {
            return false;
        };
        let owner = self.units[first].owner;
        let reach = mm(self.tactics.rules.escort.contact_mm);
        let mut near = Vec::new();
        points.iter().any(|&p| {
            self.unit_grid.query(
                p - Vec2::new(reach, reach),
                p + Vec2::new(reach, reach),
                &mut near,
            );
            near.iter().any(|&j| {
                let other = &self.units[j as usize];
                other.alive()
                    && !self.teams.allied(other.owner, owner)
                    && (other.position - p).length() <= reach
                    && self.owner_sees(first, j as usize)
            })
        })
    }

    /// The tank squad `i` escorts right now (its index) and how far behind
    /// its center the squad's place is: both alive, in the same march, the
    /// squad out of buildings and vehicles, not charging (prompt 196),
    /// within `leash_mm` of the tank and both away from bridges (the zone
    /// where columns queue and the gate in front of it, `[march]`).
    pub(super) fn escort_of(&self, i: usize) -> Option<(usize, Fixed)> {
        let rules = &self.tactics.rules.escort;
        let unit = &self.units[i];
        let escort = unit.drill.escort?;
        let squad = unit.squad.as_ref()?;
        if rules.enabled == 0
            || unit.drill.march == 0
            || !unit.alive()
            || unit.link.is_some()
            || squad.garrison.is_some()
            || squad.tactics.assault
            || squad.tactics.running
        {
            return None;
        }
        let v = self
            .units
            .binary_search_by_key(&escort.vehicle, |u| u.id)
            .ok()?;
        let tank = &self.units[v];
        let leash = mm(rules.leash_mm) + escort.behind;
        // At bridges the group crosses as a column (prompt 187): within the
        // gate in front of a deck everybody goes by himself.
        let gate = Fixed::from_num(NEAR_BRIDGE_M) + mm(self.tactics.rules.march.gate_mm);
        let at_bridge = |p: Vec2| bridge_within(&self.map, p, gate).is_some();
        (tank.alive()
            && tank.link.is_none()
            && tank.drill.march == unit.drill.march
            && (tank.position - unit.position).length() <= leash
            && !at_bridge(unit.position)
            && !at_bridge(tank.position))
        .then_some((v, escort.behind))
    }

    /// Whether escorting squad `i` moves crouched (`kneel`): it or its tank
    /// is in a fight (contact, a target, suppressed or fired at lately, the
    /// direction of the fire remembered, prompt 110).
    /// Out of contact they walk upright, so the group keeps the marching
    /// pace.
    pub(super) fn escort_crouches(&self, i: usize) -> bool {
        if self.tactics.rules.escort.kneel == 0 {
            return false;
        }
        let unit = &self.units[i];
        let fired_at = unit
            .squad
            .as_ref()
            .is_some_and(|s| s.suppressed > 0 || s.tactics.contact > 0)
            || !unit.under_fire.attackers_empty()
            || unit.alert.direction().is_some();
        self.escort_of(i).is_some_and(|(v, _)| {
            let tank = &self.units[v];
            fired_at
                || tank.combat.attack.is_some()
                || tank.combat.auto_target.is_some()
                || !tank.under_fire.attackers_empty()
                || tank.alert.direction().is_some()
        })
    }

    /// Where escorting squad `i` joins its tank's track before it follows
    /// the group path (`path`): behind the tank's position against the
    /// path's first leg. `None` when it escorts nothing or stands there.
    pub(super) fn escort_join(&self, i: usize, path: &crate::path::Path) -> Option<Vec2> {
        let unit = &self.units[i];
        let escort = unit.drill.escort?;
        if self.tactics.rules.escort.enabled == 0 || unit.squad.is_none() || unit.drill.march == 0 {
            return None;
        }
        let v = self
            .units
            .binary_search_by_key(&escort.vehicle, |u| u.id)
            .ok()?;
        let tank = &self.units[v];
        let first = *path.waypoints.get(1)?;
        let dir = crate::formation::normalized(first - path.waypoints[0])?;
        let join = tank.position - dir.scale(escort.behind);
        ((join - unit.position).length() > mm(self.tactics.rules.escort.ahead_mm)).then_some(join)
    }

    /// The front of the files of squad `i` behind its tank and the tank's
    /// direction, while the squad escorts it on the move and stands behind
    /// it (a tank turning back towards its squad leaves the soldiers to
    /// their formation places and the step aside of prompt 093). `None`
    /// otherwise.
    pub(super) fn escort_file(&self, i: usize) -> Option<(Vec2, Vec2)> {
        let (v, behind) = self.escort_of(i)?;
        let unit = &self.units[i];
        if !matches!(unit.route, Route::Following { .. }) {
            return None;
        }
        let tank = &self.units[v];
        let forward = track(tank);
        if dot(unit.position - tank.position, forward) >= Fixed::ZERO {
            return None;
        }
        let squad = unit.squad.as_ref()?;
        let rows = Fixed::from_num(squad.alive_count().div_ceil(2).max(1) - 1);
        let depth = mm(self.tactics.rules.escort.row_mm) * rows;
        Some((tank.position - forward.scale(behind - depth / 2), forward))
    }

    /// Place of the `m`-th living soldier in the files that start at
    /// `front` and lead back against `forward`: two files `file_mm` apart,
    /// rows `row_mm` apart.
    pub(super) fn escort_place(&self, front: Vec2, forward: Vec2, m: usize) -> Vec2 {
        let rules = &self.tactics.rules.escort;
        let right = Vec2::new(-forward.y, forward.x);
        let row = Fixed::from_num(u32::try_from(m / 2).unwrap_or(0));
        let side = if m.is_multiple_of(2) {
            -Fixed::ONE
        } else {
            Fixed::ONE
        };
        front - forward.scale(mm(rules.row_mm) * row) + right.scale(side * mm(rules.file_mm) / 2)
    }

    /// Per soldier of squad `j`: whether he walks in the track of the tank
    /// he escorts, behind its hull (prompt 198: bursts from the front do
    /// not throw him down). Empty when the squad escorts no tank.
    pub(super) fn sheltered_soldiers(&self, j: usize) -> Vec<bool> {
        let Some((v, behind)) = self.escort_of(j) else {
            return Vec::new();
        };
        let tank = &self.units[v];
        let ty = &self.types[tank.unit_type as usize];
        let forward = track(tank);
        let right = Vec2::new(-forward.y, forward.x);
        let rules = &self.tactics.rules.escort;
        let reach = behind * 2 + mm(rules.row_mm) * 5;
        let Some(squad) = &self.units[j].squad else {
            return Vec::new();
        };
        squad
            .soldiers
            .iter()
            .map(|s| {
                let d = s.position - tank.position;
                let along = dot(d, forward);
                along < -ty.half_length && along > -reach && dot(d, right).abs() <= ty.half_width
            })
            .collect()
    }

    /// Speed caps of escorting squads and their tanks (prompt 198), on top
    /// of the march caps (`None`: no cap, `Some(0)`: wait). A squad more
    /// than `ahead_mm` in front of its place waits; a tank drives at most at
    /// the pace of its squads on its ground, at `slow_pct` while one is more
    /// than `slow_mm` behind its place and waits while one is more than
    /// `wait_mm` behind.
    pub(super) fn escort_caps(&self, caps: &mut Vec<Option<Fixed>>) {
        let rules = self.tactics.rules.escort;
        if rules.enabled == 0 {
            return;
        }
        let (ahead, slow, wait) = (mm(rules.ahead_mm), mm(rules.slow_mm), mm(rules.wait_mm));
        for i in 0..self.units.len() {
            if self.units[i].drill.escort.is_none() {
                continue;
            }
            let Some((v, behind)) = self.escort_of(i) else {
                continue;
            };
            let (squad, tank) = (&self.units[i], &self.units[v]);
            if !matches!(squad.route, Route::Following { .. }) {
                continue;
            }
            if caps.is_empty() {
                caps.resize(self.units.len(), None);
            }
            let dir = track(tank);
            let place = tank.position - dir.scale(behind);
            let along = dot(squad.position - place, dir);
            let mut cap = |k: usize, c: Fixed| {
                caps[k] = Some(caps[k].map_or(c, |old| old.min(c)));
            };
            // Not while it walks back to the tank's track (its way to it
            // leads against the tank's direction).
            let forward = super::heading(squad).is_some_and(|h| dot(h, dir) > Fixed::ZERO);
            if along > ahead && forward {
                cap(i, Fixed::ZERO);
            }
            if !matches!(tank.route, Route::Following { .. }) {
                continue;
            }
            let pace = (
                squad.speed_offroad,
                squad.speed_road,
                squad.speed_pct,
                squad.unit_type as usize,
            );
            let Some(pace) = self.pace_at(pace, tank.position) else {
                continue;
            };
            let late = -along;
            let pct = if late > wait {
                0
            } else if late > slow {
                rules.slow_pct
            } else {
                100
            };
            cap(v, pace * Fixed::from_num(pct) / 100);
        }
    }
}
