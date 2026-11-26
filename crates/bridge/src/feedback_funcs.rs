//! `SimHost` functions for feedback on orders (prompt 175): dropped orders
//! with their reason, what a unit does, why it does not fire, and which
//! faces of an enemy it penetrates. Everything is read from the simulation
//! (`sim/feedback.rs`); Godot only shows it (`hud/order_feedback.gd`,
//! `hud/unit_panel.gd`, `hud/order_preview.gd`).

use crate::SimHost;
use crate::helpers::push_capped;
use godot::prelude::*;
use sim::{OrderEvent, Sim, UnitId};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 256;

/// Appends the local player's dropped orders of the latest tick.
pub(crate) fn collect_events(
    pending: &mut Vec<OrderEvent>,
    sim: &Sim,
    local: Option<sim::PlayerId>,
) {
    let Some(local) = local else { return };
    push_capped(
        pending,
        sim.order_events()
            .iter()
            .filter(|e| matches!(e, OrderEvent::Rejected { player, .. } if *player == local))
            .copied(),
        MAX_PENDING_EVENTS,
    );
}

fn unit_id(id: i64) -> Option<UnitId> {
    u32::try_from(id).ok().map(UnitId)
}

#[godot_api(secondary)]
impl SimHost {
    /// The local player's orders the simulation dropped since the last
    /// call: `reason` (name, e.g. "no_seats"; translation key
    /// `ORDER_REJECTED_<NAME>`), `code` and `unit` (-1: none).
    #[func]
    fn take_order_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.order_events);
        let mut out = Array::new();
        for event in events {
            let OrderEvent::Rejected { reason, unit, .. } = event;
            let mut d = VarDictionary::new();
            d.set("type", "rejected");
            d.set("reason", reason.name());
            d.set("code", i64::from(reason.code()));
            d.set("unit", unit.map_or(-1, |u| i64::from(u.0)));
            out.push(&d);
        }
        out
    }

    /// What unit `id` does now: `activity` (name, translation key
    /// `ACTIVITY_<NAME>`) and `target` (-1: none); empty for unknown units.
    #[func]
    fn unit_activity(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Some(id)) = (&self.game, unit_id(id)) else {
            return d;
        };
        if let Some((activity, target)) = game.sim.activity(id) {
            d.set("activity", activity.name());
            d.set("target", target.map_or(-1, |t| i64::from(t.0)));
        }
        d
    }

    /// Activity names of several units at once (multiple selection).
    #[func]
    fn unit_activities(&self, ids: PackedInt64Array) -> PackedStringArray {
        let Some(game) = &self.game else {
            return PackedStringArray::new();
        };
        ids.as_slice()
            .iter()
            .map(|&id| {
                unit_id(id)
                    .and_then(|id| game.sim.activity(id))
                    .map_or("", |(a, _)| a.name())
            })
            .map(GString::from)
            .collect()
    }

    /// Whether unit `id` fires and why not: `status` (name, translation
    /// key `FIRE_STATUS_<NAME>`) and `target` (-1: none); empty for units
    /// without weapons.
    #[func]
    fn unit_fire_status(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Some(id)) = (&self.game, unit_id(id)) else {
            return d;
        };
        if let Some((status, target)) = game.sim.fire_status(id) {
            d.set("status", status.name());
            d.set("target", target.map_or(-1, |t| i64::from(t.0)));
        }
        d
    }

    /// Which hull faces of `target` (front, side, rear: 1 = penetrated)
    /// the best weapon of `shooter` penetrates at their distance; empty if
    /// unknown or unarmed.
    #[func]
    fn penetration_view(&self, shooter: i64, target: i64) -> PackedByteArray {
        let (Some(game), Some(a), Some(b)) = (&self.game, unit_id(shooter), unit_id(target)) else {
            return PackedByteArray::new();
        };
        game.sim
            .penetration_view(a, b)
            .map(|faces| faces.iter().map(|&f| u8::from(f)).collect())
            .unwrap_or_default()
    }
}
