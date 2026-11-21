//! Last resort against stuck units (prompt 173).
//!
//! The movement rules (`move_unit`, `Sim::move_units`) skip waypoints a
//! unit cannot reach, slide, turn and plan again. If a unit following a
//! route still does not get closer to its target, it **stops cleanly** and
//! the simulation reports [`MoveEvent::Stuck`]:
//!
//! - **No progress:** the length of its remaining route (to the current
//!   waypoint, then along the rest) has not dropped by [`PROGRESS_MM`] for
//!   [`STUCK_SECONDS`]. Halts on purpose (stance changes, fire-team bounds,
//!   `speed_pct == 0`) do not count; a new route (order or re-plan)
//!   starts afresh. Re-plans are limited (`MAX_REPLANS` per waypoint).
//! - **Gave up:** the existing rules stop a unit short of its target (its
//!   re-plans are used up, a search from where it stands finds nothing, or
//!   it stalls on its last waypoint farther than its own width plus
//!   [`SHORT_MM`] from it).
//!
//! No teleporting and no pushing through obstacles: the unit stays where it
//! is, the player sees the message and gives a new order.
//!
//! State: [`Progress`] per unit (in the state hash). The events are derived
//! for presentation and tools, cleared every tick.

use super::{Route, Sim, Unit};
use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Seconds without progress after which a unit stops.
pub const STUCK_SECONDS: u32 = 20;
/// Less progress than this (mm of remaining route) does not count.
pub const PROGRESS_MM: i32 = 1000;
/// A unit stopped farther than its width plus this (mm) from its target
/// gave up.
pub const SHORT_MM: i32 = 8000;

/// What happened to a unit's movement during one tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveEvent {
    /// The unit stopped short of its target and waits for a new order.
    Stuck { unit: UnitId, at: Vec2 },
}

/// Progress of a unit along its route.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Progress {
    /// Shortest remaining route length so far; `None`: not measured yet
    /// (new route).
    best: Option<Fixed>,
    /// Ticks since `best` last improved.
    ticks: u32,
    /// The route `best` belongs to (its first waypoint and length): a new
    /// route starts afresh. Orders and re-plans are often planned within
    /// the tick they arrive, so the unit is never seen waiting in between.
    route: Option<(Vec2, u32)>,
    /// Cache: the waypoint index and the length of the route from it to
    /// the end (derived from the route, so not hashed).
    #[serde(default)]
    tail: Option<(u32, Fixed)>,
}

impl Progress {
    pub(super) fn hash_into(self, h: &mut StableHasher) {
        h.write_i64(self.best.map_or(-1, Fixed::to_bits));
        h.write_u32(self.ticks);
        match self.route {
            None => h.write_u8(0),
            Some((first, len)) => {
                h.write_u8(1);
                h.write_i64(first.x.to_bits());
                h.write_i64(first.y.to_bits());
                h.write_u32(len);
            }
        }
    }
}

/// The route's key (first waypoint, number of waypoints); `None` if the
/// unit follows none.
fn route_key(unit: &Unit) -> Option<(Vec2, u32)> {
    let Route::Following { waypoints, .. } = &unit.route else {
        return None;
    };
    Some((
        waypoints[0],
        u32::try_from(waypoints.len()).unwrap_or(u32::MAX),
    ))
}

/// Length of the route still ahead of a unit, with the length from its
/// current waypoint to the end cached in `tail`.
fn remaining(unit: &Unit, tail: &mut Option<(u32, Fixed)>) -> Fixed {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return Fixed::ZERO;
    };
    let index = u32::try_from(*next).unwrap_or(u32::MAX);
    let rest = match *tail {
        Some((i, rest)) if i == index => rest,
        _ => {
            let mut rest = Fixed::ZERO;
            for pair in waypoints[*next..].windows(2) {
                rest += (pair[1] - pair[0]).length();
            }
            *tail = Some((index, rest));
            rest
        }
    };
    (waypoints[*next] - unit.position).length() + rest
}

/// Whether a unit stopped at `at` is short of `target`.
pub(super) fn short_of(unit: &Unit, target: Vec2) -> bool {
    (target - unit.position).length() > unit.spread + Fixed::from_num(SHORT_MM) / 1000
}

impl Sim {
    /// After the movement: stops units that made no progress for
    /// [`STUCK_SECONDS`] and reports them.
    pub(super) fn check_progress(&mut self) {
        let limit = STUCK_SECONDS * self.tick_hz;
        let step = Fixed::from_num(PROGRESS_MM) / 1000;
        for i in 0..self.units.len() {
            let unit = &mut self.units[i];
            let Some(key) = route_key(unit).filter(|_| unit.alive() && unit.link.is_none()) else {
                unit.progress = Progress::default();
                continue;
            };
            if unit.speed_pct == 0 {
                // Halted on purpose: the clock waits.
                continue;
            }
            if unit.progress.route != Some(key) {
                unit.progress = Progress {
                    route: Some(key),
                    ..Progress::default()
                };
            }
            let mut tail = unit.progress.tail;
            let left = remaining(unit, &mut tail);
            let p = &mut unit.progress;
            p.tail = tail;
            if p.best.is_none_or(|best| left + step <= best) {
                p.best = Some(left);
                p.ticks = 0;
                continue;
            }
            p.ticks += 1;
            if p.ticks >= limit {
                let (id, at) = (unit.id, unit.position);
                super::stop(unit);
                unit.progress = Progress::default();
                self.move_events.push(MoveEvent::Stuck { unit: id, at });
            }
        }
    }

    /// Reports unit `i` as stuck if it stopped short of `target`.
    pub(super) fn report_short(&mut self, i: usize, target: Option<Vec2>) {
        let unit = &self.units[i];
        if target.is_some_and(|t| short_of(unit, t)) {
            self.move_events.push(MoveEvent::Stuck {
                unit: unit.id,
                at: unit.position,
            });
        }
    }

    /// Movement events of the last tick.
    pub fn move_events(&self) -> &[MoveEvent] {
        &self.move_events
    }
}
