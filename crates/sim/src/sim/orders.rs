//! Waypoint chains and patrols (prompt 031): a queue of move targets per
//! unit, filled by [`Command::QueueMove`] and [`Command::Patrol`].
//!
//! - `QueueMove` assigns formation slots around the waypoint exactly like
//!   `MoveTo` (grid, units in the order of their position) and appends each
//!   unit's slot to its queue. If every unit of the order is idle and has an
//!   empty queue, it is a plain `MoveTo` (one group path).
//! - A living unit without target, route and attack order takes the next
//!   waypoint of its queue and plans its own path there. This runs right
//!   after the commands of a tick, so an idle unit starts at once.
//! - `Patrol` is a `MoveTo` to the target; afterwards the unit shuttles
//!   between where it started and its slot at the target (the queue cycles).
//! - Every other order that moves, attacks, stops, garrisons or fires a
//!   barrage (prompt 032) drops its queue; a unit with an indirect fire
//!   order is not idle.
//!
//! State hash: only units with a queue (or a patrol) are written, after all
//! other state, so matches without chains keep their hash.

use super::{Route, Sim, Unit, formation_slots, formation_spacing};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Waypoints a unit keeps at most; more are ignored (forged commands).
pub const MAX_QUEUED: usize = 64;

/// The queued move targets of one unit. Serializable for saved games
/// (prompt 038 serializes the whole `Sim`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct OrderQueue {
    /// Formation slots, the next one first.
    points: Vec<Vec2>,
    /// A taken waypoint goes back to the end of the queue.
    patrol: bool,
}

impl OrderQueue {
    fn is_empty(&self) -> bool {
        self.points.is_empty() && !self.patrol
    }

    pub(super) fn clear(&mut self) {
        self.points.clear();
        self.patrol = false;
    }
}

/// Whether `command` replaces the current orders of its units (and so
/// drops their queues).
const fn replaces_queue(command: &Command) -> bool {
    matches!(
        command,
        Command::MoveTo { .. }
            | Command::Stop { .. }
            | Command::Attack { .. }
            | Command::AttackGround { .. }
            | Command::AttackObject { .. }
            | Command::Garrison { .. }
            | Command::Barrage { .. }
            | Command::Patrol { .. }
            | Command::FormationMove { .. }
            | Command::Secure { .. }
    )
}

/// The unit has nothing to do: no target, no route, no attack order, no
/// attack-move or secured area (prompt 046).
fn idle(unit: &Unit) -> bool {
    unit.alive()
        && unit.target.is_none()
        && unit.route == Route::None
        && unit.combat.attack.is_none()
        && !unit.drill.busy()
}

impl Sim {
    /// Whether unit `id` is idle (prompt 101, the "idle unit" key): alive,
    /// not riding or towed, no target to drive to, no waypoints, no attack
    /// order and no target it fires at by itself, no indirect fire order,
    /// no attack-move or secured area, no engineer job. Read only.
    pub fn unit_idle(&self, id: UnitId) -> bool {
        let Ok(i) = self.units.binary_search_by_key(&id, |u| u.id) else {
            return false;
        };
        let u = &self.units[i];
        u.alive()
            && u.link.is_none()
            && u.target.is_none()
            && matches!(u.route, Route::None)
            && u.orders.is_empty()
            && u.combat.attack.is_none()
            && u.combat.auto_target.is_none()
            && !u.drill.busy()
            && !self.has_indirect_order(i)
            && !self.has_work_job(i)
    }

    /// Drops the queues of the units of `command` if it replaces their
    /// orders. Called before the command is applied.
    pub(super) fn drop_replaced_queues(&mut self, command: &PlayerCommand) {
        if !replaces_queue(&command.command) {
            return;
        }
        for index in self.owned_indices(command.player, command.command.units()) {
            self.units[index].orders.clear();
        }
    }

    /// `QueueMove`: appends a formation slot around `target` to the queue
    /// of every listed (own, mobile) unit.
    pub(super) fn order_queue_move(&mut self, player: PlayerId, units: &[UnitId], target: Vec2) {
        if self.reject_movement(player, units, target) {
            return;
        }
        let group = self.mobile_indices(player, units);
        if group.iter().all(|&i| {
            idle(&self.units[i]) && self.units[i].orders.is_empty() && !self.has_indirect_order(i)
        }) {
            let ids = group.iter().map(|&i| self.units[i].id).collect();
            self.apply(&PlayerCommand::new(
                player,
                Command::MoveTo { units: ids, target },
            ));
            return;
        }
        for (i, slot) in self.assign_slots(group, target) {
            let queue = &mut self.units[i].orders;
            if queue.points.len() < MAX_QUEUED {
                queue.points.push(slot);
            }
        }
    }

    /// `Patrol`: a `MoveTo` to `target`, then back and forth between the
    /// start and the slot at the target.
    pub(super) fn order_patrol(&mut self, player: PlayerId, units: &[UnitId], target: Vec2) {
        if self.reject_movement(player, units, target) {
            return;
        }
        let group = self.mobile_indices(player, units);
        let starts: Vec<(usize, Vec2)> =
            group.iter().map(|&i| (i, self.units[i].position)).collect();
        let slots = self.assign_slots(group.clone(), target);
        let ids = group.iter().map(|&i| self.units[i].id).collect();
        self.apply(&PlayerCommand::new(
            player,
            Command::MoveTo { units: ids, target },
        ));
        for (i, start) in starts {
            let slot = slots
                .iter()
                .find(|&&(j, _)| j == i)
                .map_or(target, |&(_, s)| s);
            let queue = &mut self.units[i].orders;
            queue.points = vec![start, slot];
            queue.patrol = true;
        }
    }

    /// Formation slots around `target` for the unit indices `group`, as
    /// `MoveTo` assigns them: slots row by row, units in the order of their
    /// position (y, then x, then id).
    fn assign_slots(&self, mut group: Vec<usize>, target: Vec2) -> Vec<(usize, Vec2)> {
        let footprint = group
            .iter()
            .map(|&i| self.units[i].spread)
            .max()
            .unwrap_or(Fixed::ZERO);
        let slots = formation_slots(target, group.len(), formation_spacing(footprint));
        group.sort_by_key(|&i| {
            let u = &self.units[i];
            (u.position.y, u.position.x, u.id)
        });
        group.into_iter().zip(slots).collect()
    }

    /// Idle units with a queue drive to their next waypoint (own path).
    /// Destroyed units lose their queue.
    pub(super) fn advance_queues(&mut self) {
        for index in 0..self.units.len() {
            let unit = &mut self.units[index];
            if unit.orders.is_empty() {
                continue;
            }
            if !unit.alive() {
                unit.orders.clear();
                continue;
            }
            if !idle(unit) || self.has_indirect_order(index) {
                continue;
            }
            let unit = &mut self.units[index];
            let queue = &mut unit.orders;
            if queue.points.is_empty() {
                queue.patrol = false;
                continue;
            }
            let next = queue.points.remove(0);
            if queue.patrol {
                queue.points.push(next);
            }
            self.leave_building(index);
            let unit = &mut self.units[index];
            let id = self.paths.request(
                (&self.map, &self.terrain),
                unit.position,
                next,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(next);
            unit.route = Route::Waiting(id);
            unit.replans = 0;
        }
    }

    /// The queued waypoints of a unit (the next one first) and whether it
    /// patrols; `None` if the unit does not exist.
    pub fn queued_waypoints(&self, id: UnitId) -> Option<(&[Vec2], bool)> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        let queue = &self.units[index].orders;
        Some((&queue.points, queue.patrol))
    }

    /// Hashes the queues of the units that have one.
    pub(super) fn hash_queues(&self, h: &mut StableHasher) {
        for unit in self.units.iter().filter(|u| !u.orders.is_empty()) {
            h.write_u32(unit.id.0);
            h.write_u8(u8::from(unit.orders.patrol));
            h.write_u64(unit.orders.points.len() as u64);
            for p in &unit.orders.points {
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
            }
        }
    }
}
