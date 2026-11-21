//! Feedback on orders (prompt 175): what the presentation needs to tell the
//! player why an order does nothing, what a unit is doing and why it does
//! not fire. Nothing here changes a rule:
//!
//! - [`OrderEvent::Rejected`]: an order the simulation dropped as a whole,
//!   with a [`RejectReason`] code (no texts in the simulation). Derived,
//!   cleared every tick, not hashed.
//! - [`Sim::activity`]: what a unit does now, read from its state.
//! - [`Sim::fire_status`]: whether a unit fires and, if not, why: the same
//!   checks as the combat update (`aim_main`, `choose_target`), only read.
//! - [`Sim::penetration_view`]: which hull faces of a target the best
//!   weapon of a shooter penetrates at their distance, head-on (the
//!   penetration formula of a hit without the random spread).
//! - [`Sim::reachable_point`]: where a unit ordered to a point actually
//!   ends up (the nearest reachable cell, as the path search does).

use super::combat::Attack;
use super::{Route, Sim, transport};
use crate::combat::FireMode;
use crate::command::{PlayerId, UnitId};
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Why the simulation dropped an order (prompt 175). The numbers are the
/// codes the bridge hands to Godot; never reuse one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RejectReason {
    /// The target point lies beyond the map.
    OutsideMap = 1,
    /// None of the listed units can move (buildings, guns without a
    /// tractor, units aboard or towed, out of fuel or immobilized).
    NoMobileUnits = 2,
    /// The attack target is an own or allied unit.
    TargetFriendly = 3,
    /// The attack target no longer exists.
    TargetGone = 4,
    /// No listed unit has a weapon that can hurt the target.
    CannotHurtTarget = 5,
    /// Every listed unit that could fire is out of ammunition.
    NoAmmo = 6,
    /// The map object cannot be shelled (no hit points or already
    /// destroyed).
    NotShellable = 7,
    /// No listed unit fires rounds that damage buildings and objects.
    NoObjectWeapon = 8,
    /// The vehicle to mount is not an own living vehicle with seats.
    NoSeats = 9,
    /// No infantry squad among the listed units (mounting, occupying).
    NoSquads = 10,
    /// The unit to tow is not an own free gun, or is already being
    /// hitched.
    NotAGun = 11,
    /// No listed vehicle can pull the gun (too weak, or towing already).
    NoTractor = 12,
    /// None of the listed units tows or is a towed gun.
    NothingToUnlimber = 13,
    /// The map object is not a building squads can enter.
    NotABuilding = 14,
    /// The building cannot be placed there.
    CannotPlace = 15,
    /// The player cannot pay for it.
    CannotAfford = 16,
    /// Not available: not producible here, no aircraft or battery of the
    /// faction, still in its cooldown, or the producer is not ready.
    NotAvailable = 17,
    /// The production queue is full.
    QueueFull = 18,
    /// No listed unit has an indirect weapon (barrage).
    NoArtillery = 19,
    /// No engineer squad among the listed units.
    NoEngineers = 20,
}

impl RejectReason {
    pub const ALL: [Self; 20] = [
        Self::OutsideMap,
        Self::NoMobileUnits,
        Self::TargetFriendly,
        Self::TargetGone,
        Self::CannotHurtTarget,
        Self::NoAmmo,
        Self::NotShellable,
        Self::NoObjectWeapon,
        Self::NoSeats,
        Self::NoSquads,
        Self::NotAGun,
        Self::NoTractor,
        Self::NothingToUnlimber,
        Self::NotABuilding,
        Self::CannotPlace,
        Self::CannotAfford,
        Self::NotAvailable,
        Self::QueueFull,
        Self::NoArtillery,
        Self::NoEngineers,
    ];

    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Stable name for tools and translations (`ORDER_REJECTED_<NAME>`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::OutsideMap => "outside_map",
            Self::NoMobileUnits => "no_mobile_units",
            Self::TargetFriendly => "target_friendly",
            Self::TargetGone => "target_gone",
            Self::CannotHurtTarget => "cannot_hurt_target",
            Self::NoAmmo => "no_ammo",
            Self::NotShellable => "not_shellable",
            Self::NoObjectWeapon => "no_object_weapon",
            Self::NoSeats => "no_seats",
            Self::NoSquads => "no_squads",
            Self::NotAGun => "not_a_gun",
            Self::NoTractor => "no_tractor",
            Self::NothingToUnlimber => "nothing_to_unlimber",
            Self::NotABuilding => "not_a_building",
            Self::CannotPlace => "cannot_place",
            Self::CannotAfford => "cannot_afford",
            Self::NotAvailable => "not_available",
            Self::QueueFull => "queue_full",
            Self::NoArtillery => "no_artillery",
            Self::NoEngineers => "no_engineers",
        }
    }
}

/// What happened to an order during one tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderEvent {
    /// `player`'s order was dropped; `unit`: the unit it was about (a
    /// target, a vehicle, a building), if any.
    Rejected {
        player: PlayerId,
        reason: RejectReason,
        unit: Option<UnitId>,
    },
}

/// What a unit does now (prompt 175), most specific first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    /// Its path is being planned.
    Planning,
    Moving,
    /// Drives towards its attack target.
    Attacking,
    /// Has a target and fights (aims, fires, reloads).
    Fighting,
    /// Fires indirectly (artillery order).
    Bombarding,
    /// Walks to a vehicle to mount, or climbs aboard.
    Mounting,
    /// Gets off its vehicle.
    Dismounting,
    /// Rides in or on a vehicle.
    Riding,
    /// Drives to a gun to hitch it, or is hitching it.
    Hitching,
    /// Is being unhitched and brought into action.
    Unlimbering,
    /// Is towed.
    Towed,
    /// Engineers at work (mines, wire, trenches, bridges).
    Working,
    /// Occupies a building.
    InBuilding,
    /// Pinned down under fire.
    Pinned,
    /// A gun without crew (prompt 177).
    Abandoned,
}

impl Activity {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::Moving => "moving",
            Self::Attacking => "attacking",
            Self::Fighting => "fighting",
            Self::Bombarding => "bombarding",
            Self::Mounting => "mounting",
            Self::Dismounting => "dismounting",
            Self::Riding => "riding",
            Self::Hitching => "hitching",
            Self::Unlimbering => "unlimbering",
            Self::Towed => "towed",
            Self::Working => "working",
            Self::InBuilding => "in_building",
            Self::Pinned => "pinned",
            Self::Abandoned => "abandoned",
        }
    }
}

/// Whether a unit fires, and why not (prompt 175).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireStatus {
    /// Fires at its target (or reloads between shots).
    Firing,
    Reloading,
    /// Turns its turret, gun or hull towards the target.
    Aiming,
    /// No enemy in its own sight.
    NoTarget,
    /// It holds its fire (fire mode) and has no attack order.
    HoldFire,
    /// It returns fire only (prompt 176) and the enemy in sight has not
    /// fired at it.
    ReturnFire,
    /// Its fire limits (close range only, vehicles only) keep it from the
    /// enemy in sight (prompt 176).
    Limited,
    /// Its attack target is not in its own sight.
    NoSight,
    /// The target is beyond the range of its weapons.
    OutOfRange,
    /// No weapon can hurt the target at this distance.
    Ineffective,
    /// Out of ammunition.
    NoAmmo,
    /// Pinned down: fires little or blindly.
    Suppressed,
    /// Riding, towed, limbered or mounting: cannot fire now.
    CannotFire,
    /// A gun without crew (prompt 177).
    Abandoned,
}

impl FireStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Firing => "firing",
            Self::Reloading => "reloading",
            Self::Aiming => "aiming",
            Self::NoTarget => "no_target",
            Self::HoldFire => "hold_fire",
            Self::ReturnFire => "return_fire",
            Self::Limited => "limited",
            Self::NoSight => "no_sight",
            Self::OutOfRange => "out_of_range",
            Self::Ineffective => "ineffective",
            Self::NoAmmo => "no_ammo",
            Self::Suppressed => "suppressed",
            Self::CannotFire => "cannot_fire",
            Self::Abandoned => "abandoned",
        }
    }
}

impl Sim {
    /// Reports a dropped order.
    pub(super) fn reject(&mut self, player: PlayerId, reason: RejectReason, unit: Option<UnitId>) {
        self.order_events.push(OrderEvent::Rejected {
            player,
            reason,
            unit,
        });
    }

    /// For movement orders: reports and returns `true` when the target lies
    /// beyond the world, or when none of the listed own units can move.
    pub(super) fn reject_movement(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
    ) -> bool {
        if !super::within_world(target) {
            self.reject(player, RejectReason::OutsideMap, None);
            return true;
        }
        self.reject_immobile(player, units)
    }

    /// Reports and returns `true` when some listed units are the player's
    /// but none of them can move.
    pub(super) fn reject_immobile(&mut self, player: PlayerId, units: &[UnitId]) -> bool {
        if self.mobile_indices(player, units).is_empty() {
            if !self.owned_indices(player, units).is_empty() {
                self.reject(player, RejectReason::NoMobileUnits, None);
            }
            return true;
        }
        false
    }

    /// Attack on unit `target`: reports when no listed unit can hurt it
    /// (at any distance) or every armed one is out of ammunition.
    pub(super) fn reject_attack(&mut self, player: PlayerId, units: &[UnitId], t: usize) {
        let armed: Vec<usize> = self
            .owned_indices(player, units)
            .into_iter()
            .filter(|&i| {
                !self.types[self.units[i].unit_type as usize]
                    .weapons
                    .is_empty()
            })
            .collect();
        if armed.is_empty() {
            return;
        }
        let theirs = &self.types[self.units[t].unit_type as usize];
        let hurts = armed.iter().any(|&i| {
            let mine = &self.types[self.units[i].unit_type as usize];
            (0..mine.weapons.len())
                .any(|w| mine.can_damage(w, &self.combat_rules, theirs, Fixed::ZERO))
        }) || armed.iter().any(|&i| self.artillery_type_of(i));
        let target = Some(self.units[t].id);
        if !hurts {
            self.reject(player, RejectReason::CannotHurtTarget, target);
        } else if armed.iter().all(|&i| !self.has_ammo(i)) {
            self.reject(player, RejectReason::NoAmmo, target);
        }
    }

    /// Order events of the last tick.
    pub fn order_events(&self) -> &[OrderEvent] {
        &self.order_events
    }

    /// What unit `id` does now; `None` for unknown or destroyed units.
    pub fn activity(&self, id: UnitId) -> Option<(Activity, Option<UnitId>)> {
        let i = self.index_of(id)?;
        let unit = &self.units[i];
        if !unit.alive() {
            return None;
        }
        match unit.link {
            Some(transport::Link::Aboard(c)) => return Some((Activity::Riding, Some(c))),
            Some(transport::Link::Towed(t)) => {
                let what = if self.transport_task_name(id) == Some("unhitch") {
                    Activity::Unlimbering
                } else {
                    Activity::Towed
                };
                return Some((what, Some(t)));
            }
            None => {}
        }
        match self.transport_task_name(id) {
            Some("embark") => return Some((Activity::Mounting, None)),
            Some("disembark") => return Some((Activity::Dismounting, None)),
            Some("hitch") => return Some((Activity::Hitching, None)),
            Some("unhitch") => return Some((Activity::Unlimbering, None)),
            _ => {}
        }
        if unit.crew.abandoned() {
            return Some((Activity::Abandoned, None));
        }
        if unit.squad.as_ref().is_some_and(|s| s.tactics.pinned) || self.crew_pinned(i) {
            return Some((Activity::Pinned, None));
        }
        if self.has_work_job(i) {
            return Some((Activity::Working, None));
        }
        if self.has_fire_mission(id) {
            return Some((Activity::Bombarding, None));
        }
        let target = match unit.combat.attack {
            Some(Attack::Unit(t)) => Some(t),
            _ => unit.combat.auto_target,
        };
        if unit.combat.attack.is_some() && unit.combat.attack_moving {
            return Some((Activity::Attacking, target));
        }
        if unit.combat.attack.is_some() || unit.combat.auto_target.is_some() {
            return Some((Activity::Fighting, target));
        }
        match unit.route {
            Route::Waiting(_) => return Some((Activity::Planning, None)),
            Route::Following { .. } => return Some((Activity::Moving, None)),
            Route::None => {}
        }
        if unit.squad.as_ref().is_some_and(|s| s.garrison.is_some()) {
            return Some((Activity::InBuilding, None));
        }
        Some((Activity::Idle, None))
    }

    /// Whether unit `id` fires, and why not, with the target it is about;
    /// `None` for unknown units and units without weapons.
    pub fn fire_status(&self, id: UnitId) -> Option<(FireStatus, Option<UnitId>)> {
        let i = self.index_of(id)?;
        let unit = &self.units[i];
        let ty = &self.types[unit.unit_type as usize];
        if !unit.alive() || ty.weapons.is_empty() {
            return None;
        }
        if self.transport_blocks_fire(i) || unit.link.is_some() {
            return Some((FireStatus::CannotFire, None));
        }
        if unit.crew.abandoned() {
            return Some((FireStatus::Abandoned, None));
        }
        if !self.has_ammo(i) {
            return Some((FireStatus::NoAmmo, None));
        }
        if unit.squad.as_ref().is_some_and(|s| s.tactics.pinned) || self.crew_pinned(i) {
            return Some((FireStatus::Suppressed, None));
        }
        let target = match unit.combat.attack {
            Some(Attack::Unit(t)) => self.index_of(t),
            Some(Attack::Ground(_) | Attack::Object(_)) => {
                return Some((self.fire_or_reload(i), None));
            }
            None => unit
                .combat
                .auto_target
                .and_then(|t| self.index_of(t))
                .filter(|&t| self.units[t].alive()),
        };
        let Some(t) = target else {
            // No target: the nearest enemy in its own sight says why.
            let Some(j) = self.nearest_seen_enemy(i) else {
                return Some((FireStatus::NoTarget, None));
            };
            let other = self.units[j].id;
            if unit.combat.fire_mode == FireMode::HoldFire {
                return Some((FireStatus::HoldFire, Some(other)));
            }
            if let Some(status) = self.range_status(i, j) {
                return Some((status, Some(other)));
            }
            let distance = (self.units[j].position - unit.position).length();
            let range = self.types[unit.unit_type as usize].max_range;
            if !self.may_engage(i, j, distance, range) {
                let status = if unit.combat.fire_mode == FireMode::ReturnFire {
                    FireStatus::ReturnFire
                } else {
                    FireStatus::Limited
                };
                return Some((status, Some(other)));
            }
            return Some((FireStatus::Aiming, Some(other)));
        };
        let other = self.units[t].id;
        if !self.sees_unit(i, t) {
            return Some((FireStatus::NoSight, Some(other)));
        }
        if let Some(status) = self.range_status(i, t) {
            return Some((status, Some(other)));
        }
        let delta = self.units[t].position - unit.position;
        let distance = delta.length();
        if distance > Fixed::ONE {
            let want = Vec2::new(delta.x / distance, delta.y / distance);
            let aim = unit.combat.aim;
            // Within about 3° (cos 0.9986) counts as aimed.
            if aim.x * want.x + aim.y * want.y < Fixed::from_num(9986) / 10_000 {
                return Some((FireStatus::Aiming, Some(other)));
            }
        }
        Some((self.fire_or_reload(i), Some(other)))
    }

    fn fire_or_reload(&self, i: usize) -> FireStatus {
        if self.units[i].combat.reload.first().copied().unwrap_or(0) > 0 {
            FireStatus::Reloading
        } else {
            FireStatus::Firing
        }
    }

    /// Out of range or no weapon that hurts unit `j`, from unit `i`.
    fn range_status(&self, i: usize, j: usize) -> Option<FireStatus> {
        let mine = &self.types[self.units[i].unit_type as usize];
        let theirs = &self.types[self.units[j].unit_type as usize];
        let distance = (self.units[j].position - self.units[i].position).length();
        if distance > mine.max_range {
            return Some(FireStatus::OutOfRange);
        }
        let hurts = (0..mine.weapons.len())
            .any(|w| mine.can_damage(w, &self.combat_rules, theirs, distance));
        (!hurts).then_some(FireStatus::Ineffective)
    }

    /// The nearest living enemy that unit `i` sees itself.
    fn nearest_seen_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        let range = self.types[me.unit_type as usize].max_range * 2;
        let mut best: Option<(i128, usize)> = None;
        for (j, other) in self.units.iter().enumerate() {
            if !other.alive() || self.teams.allied(other.owner, me.owner) || other.link.is_some() {
                continue;
            }
            let d = other.position - me.position;
            if d.x.abs() > range || d.y.abs() > range || !self.sees_unit(i, j) {
                continue;
            }
            let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
            let key = x * x + y * y;
            if best.is_none_or(|(b, _)| key < b) {
                best = Some((key, j));
            }
        }
        best.map(|(_, j)| j)
    }

    /// Which hull faces of `target` (front, side, rear) the best weapon of
    /// `shooter` penetrates head-on at their current distance, with the
    /// front plate's slope; `None` for unknown units or a shooter without
    /// weapons. Soft targets (infantry, unarmored) are penetrated
    /// everywhere. Read only: the hit itself adds a random spread
    /// (`[penetration] spread_pct`) and the real impact angle.
    pub fn penetration_view(&self, shooter: UnitId, target: UnitId) -> Option<[bool; 3]> {
        let (i, j) = (self.index_of(shooter)?, self.index_of(target)?);
        let mine = &self.types[self.units[i].unit_type as usize];
        let theirs = &self.types[self.units[j].unit_type as usize];
        if mine.weapons.is_empty() {
            return None;
        }
        if theirs.infantry || theirs.armor.thinnest() == 0 {
            return Some([true; 3]);
        }
        let distance = (self.units[j].position - self.units[i].position).length();
        let rules = &self.combat_rules;
        let best = mine
            .weapons
            .iter()
            .map(|w| rules.penetration_at(&w.weapon, distance * mine.real_per_game))
            .max()
            .unwrap_or(Fixed::ZERO);
        let hull = theirs.armor.hull;
        let slope = crate::objects::sin_cos_deg(theirs.armor.hull_front_deg).1;
        let min_cos = Fixed::from_num(rules.penetration.min_cos_pct) / 100;
        let front = Fixed::from_num(hull[0]) / slope.max(min_cos);
        Some([
            best >= front,
            best >= Fixed::from_num(hull[1]),
            best >= Fixed::from_num(hull[2]),
        ])
    }

    /// Where unit `id` ends up when ordered to `goal`: `goal` if a path
    /// leads there, else the nearest reachable cell's center (what the path
    /// search does with unreachable targets). `None` for unknown units.
    pub fn reachable_point(&self, id: UnitId, goal: Vec2) -> Option<Vec2> {
        let unit = self.unit(id)?;
        Some(
            self.paths
                .grid
                .reachable_point(unit.unit_type as usize, unit.position, goal),
        )
    }
}
