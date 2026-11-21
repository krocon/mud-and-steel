//! Reaction to fire from the flank (prompt 110): standing vehicles and guns
//! remember where rounds came from and turn there. Rules: `[flank]` in
//! `data/tactics.toml` (`crate::tactics::FlankRules`).
//!
//! - [`Sim::fly_projectiles`] reports every round of another player that
//!   hits a vehicle or gun, or lands within `near_miss_mm` of its hull
//!   ([`Sim::report_fire`]). The direction is the reverse of the round's
//!   flight, its danger the level (penetrating hit, bounced hit, near miss)
//!   and then the shorter flight.
//! - A new report from about the same direction (`same_deg`) refreshes the
//!   memory; one from elsewhere takes over only when it is more dangerous
//!   and the current direction is `switch_ms` old.
//! - [`Sim::alert_turn`] (from the combat update, only when the unit has no
//!   target) turns turret, carriage or hull towards the remembered direction
//!   with the normal turn speeds once the crew reacted (`react_ms`, longer
//!   under heavy fire). Not while it moves, turns to an ordered facing or
//!   that facing still holds (`ordered_hold_ms`). Sight is unchanged: the
//!   unit fires when it sees the shooter by the normal rules.
//!
//! State hash: only units with a non-empty alert are written, after the
//! drills.

use super::Sim;
use super::infantry::inside;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::sin_cos_deg;
use crate::tactics::FlankRules;
use serde::{Deserialize, Serialize};

/// How dangerous a report is (higher first).
pub(super) const NEAR_MISS: u8 = 0;
pub(super) const BOUNCED: u8 = 1;
pub(super) const PENETRATED: u8 = 2;

/// The `[flank]` rules in ticks and game units, built once. Default: off
/// (saved games from before prompt 110).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct FlankRt {
    pub(super) enabled: bool,
    remember: u32,
    switch: u32,
    react: u32,
    shaken: u32,
    shaken_max: u32,
    hold: u32,
    /// Game meters.
    pub(super) near_miss: Fixed,
    /// Cosine of `same_deg`.
    same_cos: Fixed,
}

impl FlankRt {
    /// `ticks` converts real milliseconds into ticks (time factor applied).
    pub(super) fn new(rules: &FlankRules, ticks: impl Fn(u32) -> u32) -> Self {
        Self {
            enabled: rules.enabled == 1,
            remember: ticks(rules.remember_ms),
            switch: ticks(rules.switch_ms),
            react: ticks(rules.react_ms),
            shaken: ticks(rules.shaken_ms),
            shaken_max: ticks(rules.shaken_max_ms),
            hold: ticks(rules.ordered_hold_ms),
            near_miss: Fixed::from_num(rules.near_miss_mm) / 1000,
            same_cos: sin_cos_deg(u16::try_from(rules.same_deg.min(180)).unwrap_or(180)).1,
        }
    }
}

/// Where a unit was last fired at from, and the hold of an ordered facing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Alert {
    /// Direction towards the fire (unit length); only while `left > 0`.
    dir: Vec2,
    /// Danger of the current direction ([`PENETRATED`] ...).
    level: u8,
    /// Flight of the most dangerous round from it (game meters).
    distance: Fixed,
    /// Ticks the direction is still remembered.
    left: u32,
    /// Ticks since the direction was taken (saturating).
    since: u32,
    /// Ticks before the crew turns.
    delay: u32,
    /// Ticks an ordered facing still holds.
    hold: u32,
    /// The facing the unit turns to was ordered by its player.
    ordered: bool,
}

impl Alert {
    pub(super) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The remembered direction of fire, if any.
    pub(super) const fn direction(&self) -> Option<Vec2> {
        if self.left > 0 { Some(self.dir) } else { None }
    }

    /// A new order: an ordered facing (`ordered`) or none; ends the hold.
    pub(super) const fn set_ordered(&mut self, ordered: bool) {
        self.ordered = ordered;
        self.hold = 0;
    }

    fn hash_into(&self, h: &mut StableHasher) {
        h.write_i64(self.dir.x.to_bits());
        h.write_i64(self.dir.y.to_bits());
        h.write_u8(self.level);
        h.write_i64(self.distance.to_bits());
        for v in [self.left, self.since, self.delay, self.hold] {
            h.write_u32(v);
        }
        h.write_u8(u8::from(self.ordered));
    }
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

impl Sim {
    /// A round of another player hit unit `j` or landed near it: `back`
    /// points from the impact back along its flight, `level` and
    /// `distance` (game meters flown) rank it, `heavy` keeps the crew down.
    pub(super) fn report_fire(
        &mut self,
        j: usize,
        back: Vec2,
        level: u8,
        distance: Fixed,
        heavy: bool,
    ) {
        let rt = self.tactics.flank;
        if !rt.enabled {
            return;
        }
        let unit = &self.units[j];
        let ty = &self.types[unit.unit_type as usize];
        if !unit.alive()
            || ty.infantry
            || ty.max_range <= Fixed::ZERO
            || unit.link.is_some()
            || unit.speed_offroad <= Fixed::ZERO
        {
            return;
        }
        let Some(dir) = crate::formation::normalized(back) else {
            return;
        };
        let a = &mut self.units[j].alert;
        if heavy {
            a.delay = (a.delay + rt.shaken).min(rt.shaken_max);
        }
        if a.left == 0 {
            a.dir = dir;
            a.level = level;
            a.distance = distance;
            a.left = rt.remember;
            a.since = 0;
            a.delay = a.delay.max(rt.react);
            return;
        }
        let worse = level > a.level || (level == a.level && distance < a.distance);
        if dot(a.dir, dir) >= rt.same_cos {
            a.dir = dir;
            a.left = rt.remember;
            if worse {
                a.level = level;
                a.distance = distance;
            }
        } else if worse && a.since >= rt.switch {
            a.dir = dir;
            a.level = level;
            a.distance = distance;
            a.left = rt.remember;
            a.since = 0;
        }
    }

    /// Reports a round of the player of `shooter` that ended at `point`
    /// (flying along `d`, `traveled` game meters) to every vehicle near it
    /// as a near miss; `hit` is the unit it hit (reported separately).
    pub(super) fn report_near_misses(
        &mut self,
        point: Vec2,
        d: Vec2,
        shooter: crate::command::UnitId,
        traveled: Fixed,
        heavy: bool,
        hit: Option<usize>,
    ) {
        let rt = self.tactics.flank;
        if !rt.enabled || rt.near_miss <= Fixed::ZERO {
            return;
        }
        let Ok(k) = self.units.binary_search_by_key(&shooter, |u| u.id) else {
            return;
        };
        let owner = self.units[k].owner;
        let reach = rt.near_miss + self.unit_grid.max_reach;
        let r = Vec2::new(reach, reach);
        let mut near = Vec::new();
        self.unit_grid.query_sorted(point - r, point + r, &mut near);
        let back = Vec2::new(-d.x, -d.y);
        for j in near {
            let j = j as usize;
            let unit = &self.units[j];
            if Some(j) == hit || self.teams.allied(unit.owner, owner) {
                continue;
            }
            let ty = &self.types[unit.unit_type as usize];
            if ty.infantry {
                continue;
            }
            let edge = ty.half_length.max(ty.half_width) + rt.near_miss;
            if (unit.position - point).length() <= edge {
                self.report_fire(j, back, NEAR_MISS, traveled, heavy);
                // Prompt 176: unarmed vehicles drive away.
                self.unarmed_under_fire(j, back);
            }
        }
    }

    /// Counts down the alerts of every unit.
    pub(super) fn update_alerts(&mut self) {
        self.decay_under_fire();
        for unit in &mut self.units {
            if unit.alert.is_empty() {
                continue;
            }
            if !unit.alive() {
                unit.alert = Alert::default();
                continue;
            }
            let a = &mut unit.alert;
            a.left = a.left.saturating_sub(1);
            a.since = a.since.saturating_add(1);
            a.delay = a.delay.saturating_sub(1);
            a.hold = a.hold.saturating_sub(1);
            if a.left == 0 {
                a.dir = Vec2::ZERO;
                a.level = 0;
                a.distance = Fixed::ZERO;
                a.since = 0;
                a.delay = 0;
            }
        }
    }

    /// An ordered facing was reached: it holds for `ordered_hold_ms`.
    pub(super) fn facing_reached(&mut self, i: usize) {
        let hold = self.tactics.flank.hold;
        let a = &mut self.units[i].alert;
        if a.ordered {
            a.ordered = false;
            a.hold = hold;
        }
    }

    /// Unit `i` (without a target) turns towards the remembered fire;
    /// returns whether the alert decides its aim (while the crew is still
    /// down it keeps the aim it has).
    pub(super) fn alert_turn(&mut self, i: usize) -> bool {
        let unit = &self.units[i];
        let a = unit.alert;
        let Some(dir) = a.direction() else {
            return false;
        };
        if a.hold > 0
            || unit.route != super::Route::None
            || unit.target.is_some()
            || unit.drill.facing_pending()
            || unit.drill.advancing()
            || inside(unit)
        {
            return false;
        }
        if a.delay == 0 {
            self.turn_to(i, dir);
        }
        true
    }

    /// Hashes the alerts of the units that have one.
    pub(super) fn hash_alerts(&self, h: &mut StableHasher) {
        for unit in self.units.iter().filter(|u| !u.alert.is_empty()) {
            h.write_u32(unit.id.0);
            unit.alert.hash_into(h);
        }
    }

    /// Where unit `id` remembers being fired at from (prompt 110), for the
    /// presentation, the AI's view of its own units and tests.
    pub fn alert_direction(&self, id: crate::command::UnitId) -> Option<Vec2> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        self.units[i].alert.direction()
    }
}
