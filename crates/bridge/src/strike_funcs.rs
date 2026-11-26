//! `SimHost` functions for the heavy rocket strike and big blasts (prompt
//! 054): calling a strike, what the local player can call, the called
//! strikes (public), and the strike event stream (calls, cancels,
//! launches, ammunition explosions). Godot only draws and plays what comes
//! out of here.

use crate::helpers::{collect_tick_events, ground_point, vec3_to_godot as to_godot};
use crate::{SimHost, fixed_to_f32, ground_to_godot};
use godot::prelude::*;
use sim::{Command, Sim, StrikeEvent, UnitId, Vec2};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 1024;

/// Appends the strike events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, StrikeEvent)>, sim: &Sim) {
    collect_tick_events(pending, sim.tick(), sim.strike_events(), MAX_PENDING_EVENTS);
}

#[godot_api(secondary)]
impl SimHost {
    /// Calls the local player's heavy rocket strike on the ground point
    /// `(x, z)` in meters.
    #[func]
    fn call_strike(&mut self, x: f32, z: f32) {
        let Some(target) = ground_point(x, z) else {
            return;
        };
        self.pending.push(Command::HeavyStrike { target });
    }

    /// Debug and tools only (prompt 076): one strike rocket of `caliber_mm`
    /// from unit `shooter` to the ground point `(x, z)` right now, like a
    /// rocket of the heavy strike. Not in network matches. False if the
    /// unit is gone or the point is off the map.
    #[func]
    fn debug_rocket(&mut self, shooter: i64, x: f32, z: f32, caliber_mm: f32) -> bool {
        let Some(game) = self.game.as_mut() else {
            return false;
        };
        let (Some(target), Ok(id)) = (ground_point(x, z), u32::try_from(shooter)) else {
            return false;
        };
        if game.networked {
            godot_warn!("debug rockets are not possible in network matches");
            return false;
        }
        let caliber = (caliber_mm * 10.0).round().clamp(0.0, 100_000.0) as u32;
        if !game.sim.debug_rocket(UnitId(id), target, caliber) {
            return false;
        }
        // The shot event was added after the tick's events were read and
        // would be cleared by the next tick: hand it to Godot directly.
        if let Some(&event) = game.sim.events().last() {
            let tick = game.sim.tick().saturating_sub(1);
            self.combat_events.push((tick, event));
        }
        true
    }

    /// Whether the local player can call a strike: `cost` (fuel, ammo,
    /// manpower), `cooldown` / `cooldown_total` and `unlock` (seconds left),
    /// `battery` (unit id, -1: none), `rockets`, `radius`, `warning` (s),
    /// `blocked` ("" if it can be called now, else the reason). Empty
    /// without a match.
    #[func]
    fn strike_option(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(game) = &self.game else {
            return d;
        };
        let hz = self.tick_hz.max(1) as f32;
        let o = game.sim.strike_option(game.session.local_player());
        let cost: PackedInt32Array = o
            .cost
            .iter()
            .map(|&c| i32::try_from(c).unwrap_or(i32::MAX))
            .collect();
        d.set("cost", &cost);
        d.set("cooldown", o.cooldown_left as f32 / hz);
        d.set("cooldown_total", o.cooldown_total as f32 / hz);
        d.set("unlock", o.unlock_left as f32 / hz);
        d.set("battery", o.battery.map_or(-1, |b| i64::from(b.0)));
        d.set("rockets", i64::from(o.rockets));
        d.set("radius", o.radius_m as f32);
        d.set("warning", o.warning_ticks as f32 / hz);
        d.set("blocked", o.blocked.map_or("", |b| b.name()));
        d
    }

    /// Every called strike that has rockets left (public): `id`, `owner`,
    /// `battery` (unit id), `from` (battery position) and `target` on the
    /// ground (Godot coordinates), `radius` (m), `warning` (seconds until
    /// the first rocket, 0: firing), `rockets` (left).
    #[func]
    fn strikes(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let hz = self.tick_hz.max(1) as f32;
        let map = game.map();
        for s in game.sim.strikes() {
            let mut d = VarDictionary::new();
            d.set("id", i64::from(s.id));
            d.set("owner", i64::from(s.owner.0));
            d.set("battery", i64::from(s.battery.0));
            d.set("from", ground_to_godot(s.from, map.height_at(s.from)));
            d.set("target", ground_to_godot(s.target, map.height_at(s.target)));
            d.set("radius", fixed_to_f32(s.radius));
            d.set("warning", s.warning_left as f32 / hz);
            d.set("rockets", i64::from(s.rockets_left));
            out.push(&d);
        }
        out
    }

    /// The strike events since the last call, oldest first, as dictionaries
    /// with `type` and `tick`:
    ///
    /// - called: `strike`, `owner`, `battery`, `from`, `target` (ground
    ///   points), `radius` (m), `warning` (s)
    /// - cancelled: `strike`, `owner`, `target`
    /// - launched: `strike`, `owner`, `from`, `target`
    /// - ammo_explosion: `unit`, `position` (the wreck on the ground)
    ///
    /// Not filtered by the fog of war: smoke and siren of a strike are
    /// public; the views check visibility of ammunition explosions.
    #[func]
    fn take_strike_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.strike_events);
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let hz = self.tick_hz.max(1) as f32;
        let map = game.map();
        let ground = |p: Vec2| ground_to_godot(p, map.height_at(p));
        for (tick, event) in events {
            let mut d = VarDictionary::new();
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            match event {
                StrikeEvent::Called {
                    strike,
                    owner,
                    battery,
                    from,
                    target,
                    radius,
                    warning_ticks,
                } => {
                    d.set("type", "called");
                    d.set("strike", i64::from(strike));
                    d.set("owner", i64::from(owner.0));
                    d.set("battery", i64::from(battery.0));
                    d.set("from", ground(from));
                    d.set("target", ground(target));
                    d.set("radius", fixed_to_f32(radius));
                    d.set("warning", warning_ticks as f32 / hz);
                }
                StrikeEvent::Cancelled {
                    strike,
                    owner,
                    target,
                } => {
                    d.set("type", "cancelled");
                    d.set("strike", i64::from(strike));
                    d.set("owner", i64::from(owner.0));
                    d.set("target", ground(target));
                }
                StrikeEvent::Launched {
                    strike,
                    owner,
                    from,
                    target,
                } => {
                    d.set("type", "launched");
                    d.set("strike", i64::from(strike));
                    d.set("owner", i64::from(owner.0));
                    d.set("from", ground(from));
                    d.set("target", ground(target));
                }
                StrikeEvent::AmmoExplosion { unit, at } => {
                    d.set("type", "ammo_explosion");
                    d.set("unit", i64::from(unit.0));
                    d.set("position", to_godot(at));
                }
            }
            out.push(&d);
        }
        out
    }
}
