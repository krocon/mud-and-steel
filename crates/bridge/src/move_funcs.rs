//! `SimHost` functions for units that get stuck (prompt 173). The
//! simulation stops a unit that makes no progress and reports it
//! (`sim::MoveEvent::Stuck`); Godot shows a feed message and an alarm
//! (`scenes/hud/alerts.gd`) and a state mark (`scenes/hud/state_marks.gd`)
//! until the unit gets a new order.

use crate::{SimHost, ground_to_godot};
use godot::prelude::*;
use sim::{MoveEvent, RouteView, Sim, UnitId};
use std::collections::BTreeSet;

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 1024;

/// Appends the movement events of the latest tick and remembers the
/// stuck units.
pub(crate) fn collect_events(
    pending: &mut Vec<(u64, MoveEvent)>,
    stuck: &mut BTreeSet<u32>,
    sim: &Sim,
) {
    let tick = sim.tick().saturating_sub(1);
    for &e in sim.move_events() {
        let MoveEvent::Stuck { unit, .. } = e;
        stuck.insert(unit.0);
        pending.push((tick, e));
    }
    if pending.len() > MAX_PENDING_EVENTS {
        let excess = pending.len() - MAX_PENDING_EVENTS;
        pending.drain(..excess);
    }
    // A unit counts as stuck until it moves again (a new order).
    if !stuck.is_empty() {
        stuck.retain(|&id| sim.route_of(UnitId(id)) == Some(RouteView::Idle));
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Movement events since the last call, one dictionary each: `tick`,
    /// `type` ("stuck"), `unit` and `at` (Vector3 on the ground). Every
    /// player's units: callers filter by owner.
    #[func]
    fn take_move_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.move_events);
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let map = game.map();
        for (tick, event) in events {
            let MoveEvent::Stuck { unit, at } = event;
            let mut d = VarDictionary::new();
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            d.set("type", "stuck");
            d.set("unit", i64::from(unit.0));
            d.set("at", ground_to_godot(at, map.height_at(at)));
            out.push(&d);
        }
        out
    }

    /// Ids of the units that stopped because they got stuck and have not
    /// been given a new order since.
    #[func]
    fn stuck_units(&self) -> PackedInt64Array {
        self.stuck_units.iter().map(|&id| i64::from(id)).collect()
    }
}
