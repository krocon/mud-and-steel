//! Stances and buildings of the AI's squads (prompt 178). Only ordinary
//! commands (`SetStance`, `Garrison`), only what the player view shows.
//!
//! The simulation's automatic stance (prompt 047) already kneels a squad
//! that stands, fights or is fired at while moving, lays it down when
//! pinned or in a close fight, and lets its fire teams bound. Attacking
//! squads therefore keep the automatic stance: an AI rule that ordered a
//! fixed kneel when an enemy came within twice the range (tried first)
//! switched the bounds off and stalled the attack (village scenario, Hard:
//! 30 defenders killed before, none with the rule). The AI adds only:
//!
//! - **Hold in a house** (Normal, Hard): a squad that stands without an
//!   order within [`GUARD_M`] of an own supply point or building (it
//!   defends) and sees an enemy within [`KNEEL_RANGE_PCT`] of its range
//!   (at least [`KNEEL_MIN_M`]) occupies the nearest building within
//!   [`HOUSE_M`] (once per [`HOUSE_COOLDOWN_TICKS`]); without one it lies
//!   down when suppressed by [`PRONE_AT`] or more, else it kneels; back to
//!   the automatic stance [`STAND_AFTER_TICKS`] after the last enemy.
//! - A stance holds at least [`MIN_HOLD_TICKS`]: no twitching.

use crate::{Ai, OwnUnit, PlayerView, Role};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, StanceOrder, UnitId, Vec2};
use std::collections::BTreeMap;

/// An enemy within this share of a squad's longest range counts as near.
const KNEEL_RANGE_PCT: i64 = 200;
/// ... but at least this far (meters).
const KNEEL_MIN_M: i32 = 60;
/// Ticks without such an enemy before the squad stands up again (10 s).
const STAND_AFTER_TICKS: u64 = 200;
/// A stance holds at least this long (5 s).
const MIN_HOLD_TICKS: u64 = 100;
/// A squad within this distance of an own supply point or building
/// defends it (meters).
const GUARD_M: i32 = 80;
/// Buildings within this distance of a waiting squad (meters).
const HOUSE_M: i32 = 30;
const HOUSE_COOLDOWN_TICKS: u64 = 1200;
/// Suppression (0..=100) from which a waiting squad lies down.
const PRONE_AT: u8 = 30;

/// What the AI last ordered a squad.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Posture {
    /// 0 automatic, 1 kneel, 2 prone.
    stance: u8,
    since: u64,
    /// Last tick an enemy was near.
    enemy_seen: u64,
    /// Last garrison order.
    house_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct PostureState {
    squads: BTreeMap<UnitId, Posture>,
}

impl Ai {
    pub(crate) fn command_posture(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        let houses = self.params.garrison;
        let mut orders: BTreeMap<u8, Vec<UnitId>> = BTreeMap::new();
        let mut seen = Vec::new();
        for u in view.own.iter().filter(|u| u.alive() && u.carrier.is_none()) {
            let info = self.type_info(u.unit_type);
            // Squads leaving a strike's area, boarding or on a supply trip
            // are left to that.
            if info.role != Role::Infantry
                || u.stance.is_none()
                || self.strike.busy(u.id, view.tick)
                || self.transport.boarding(u.id, view.tick)
                || self.supply.busy(u.id)
            {
                continue;
            }
            seen.push(u.id);
            let reach = (info.range * Fixed::from_num(KNEEL_RANGE_PCT) / 100)
                .max(Fixed::from_num(KNEEL_MIN_M));
            let near = view
                .enemies
                .iter()
                .any(|e| e.alive() && (e.position - u.position).length() <= reach);
            let tick = view.tick;
            let p = self.posture.squads.entry(u.id).or_default();
            if near {
                p.enemy_seen = tick;
            }
            let moving = u.target.is_some();
            // Hold in a house (Normal, Hard), only where the squad defends:
            // at an own supply point or an own building.
            let guard = Fixed::from_num(GUARD_M);
            let defends = view.supply.iter().any(|s| {
                s.owner == Some(view.player) && (s.position - u.position).length() <= guard
            }) || view.structures.iter().any(|s| {
                view.own
                    .iter()
                    .find(|o| o.id == s.unit)
                    .is_some_and(|o| (o.position - u.position).length() <= guard)
            });
            if houses && defends && near && !moving && u.garrison.is_none() {
                let cooled = p.house_at.is_none_or(|t| tick >= t + HOUSE_COOLDOWN_TICKS);
                if cooled && let Some(b) = nearest_house(&self.houses, u) {
                    p.house_at = Some(tick);
                    out.push(Command::Garrison {
                        units: vec![u.id],
                        building: b,
                    });
                    continue;
                }
            }
            if u.garrison.is_some() {
                continue;
            }
            // A moving squad keeps the automatic stance: the simulation
            // kneels it in contact and lets its fire teams bound; a fixed
            // stance would switch that off (measured: the attack stalled).
            let wanted = if near && houses && defends && !moving {
                if u.suppression >= PRONE_AT { 2 } else { 1 }
            } else if tick >= p.enemy_seen + STAND_AFTER_TICKS || p.enemy_seen == 0 {
                0
            } else {
                p.stance
            };
            if wanted != p.stance && tick >= p.since + MIN_HOLD_TICKS {
                p.stance = wanted;
                p.since = tick;
                orders.entry(wanted).or_default().push(u.id);
            }
        }
        self.posture.squads.retain(|id, _| seen.contains(id));
        for (stance, units) in orders {
            let stance = match stance {
                1 => StanceOrder::Fixed(sim::Stance::Kneel),
                2 => StanceOrder::Fixed(sim::Stance::Prone),
                _ => StanceOrder::Auto,
            };
            out.push(Command::SetStance { units, stance });
        }
    }
}

/// The nearest enterable building within [`HOUSE_M`] of squad `u`.
fn nearest_house(houses: &[(u32, Vec2)], u: &OwnUnit) -> Option<u32> {
    let limit = Fixed::from_num(HOUSE_M);
    houses
        .iter()
        .map(|&(b, p)| ((p - u.position).length(), b))
        .filter(|&(d, _)| d <= limit)
        .min()
        .map(|(_, b)| b)
}

/// Enterable buildings of the map (public map data).
pub(crate) fn houses(map: &sim::Map) -> Vec<(u32, Vec2)> {
    let Ok(rules) = sim::InfantryRules::builtin() else {
        return Vec::new();
    };
    let kinds = map.object_kinds();
    (0u32..)
        .zip(map.objects())
        .filter(|(_, o)| {
            kinds
                .get(usize::from(o.kind))
                .is_some_and(|k| rules.building(&k.id).is_some())
        })
        .map(|(i, o)| (i, o.position))
        .collect()
}
