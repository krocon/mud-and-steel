//! Boats on the water (prompt 119): landing craft and assault boats are
//! units of their own size class in the path grid
//! ([`crate::path::SizeClass::Boat`]). Rules: `crate::boats`
//! (`data/boats.toml`); passengers ride like in trucks
//! (`crate::sim::transport`), with the boat carriers added to the
//! transport rules.
//!
//! - **Moving:** [`move_boat`] drives a boat along its route on the water
//!   (no terrain speed, no grade); it floats at the water level of its
//!   path cell.
//! - **Landing** (`Command::Land`): the boat plans a path towards the
//!   shore point (the search ends on the open water nearest to it), then
//!   runs on straight at `run_in_pct` of its speed until the water under
//!   its bow is shallower than its draft (it grounds), lowers its ramp
//!   (`ramp_ms`) and lies **beached**: only now do men and vehicles get on
//!   and off. Backing off (`Command::Retract`, or any move order while
//!   beached) raises the ramp and backs off at the same speed
//!   (`retract_ms`) onto open water.
//! - **Cargo:** squads mount and dismount over the existing `Embark` and
//!   `Disembark`; vehicles and guns ride as passengers too
//!   (`Link::Aboard`, unseen, not hit, not firing), limited by the
//!   catalog's `cargo_vehicles` and `cargo_kg` (plus `overload_pct`); the
//!   soldiers' seats shrink by the share of the payload the vehicles use.
//!   Everybody leaves over the bow: squads wade, or swim in deep water
//!   (prompt 055); a vehicle that leaves into deep water is lost.
//! - **Losses:** a destroyed boat loses the vehicles aboard; its men swim.
//!   Its wreck (and the wrecks of its cargo) sinks in water at least
//!   `sink_depth_mm` deep and stays as an obstacle in shallower water.
//! - **Paddled boats** go slower when men are lost.
//!
//! State and rules enter the state hash only in matches with a boat type,
//! so every other match keeps its hash.

use super::transport::{Link, aboard};
use super::{Moved, Route, Sim, Unit, advance_route, angled, heading, stop};
use crate::boats::BoatRules;
use crate::combat::Vec3;
use crate::command::{PlayerId, UnitId};
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::path::{PathGrid, SizeClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What a boat is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Phase {
    /// On the water: drives where it is ordered.
    Afloat,
    /// On its way to the open water off `shore`.
    Landing { shore: Vec2 },
    /// Runs on towards `shore` until it grounds; `left_mm` of the run left.
    RunIn { shore: Vec2, left_mm: u32 },
    /// Aground, lowering the ramp: `left` ticks.
    Ramp { left: u32 },
    /// Aground with the ramp down: loading and unloading.
    Beached,
    /// Raising the ramp and backing off: `left` ticks; then landing at
    /// `land` if set (ordered to land elsewhere while beached).
    Retract { left: u32, land: Option<Vec2> },
}

impl Phase {
    fn hash_into(self, h: &mut StableHasher) {
        let point = |h: &mut StableHasher, p: Vec2| {
            h.write_i64(p.x.to_bits());
            h.write_i64(p.y.to_bits());
        };
        match self {
            Self::Afloat => h.write_u8(0),
            Self::Landing { shore } => {
                h.write_u8(1);
                point(h, shore);
            }
            Self::RunIn { shore, left_mm } => {
                h.write_u8(2);
                point(h, shore);
                h.write_u32(left_mm);
            }
            Self::Ramp { left } => {
                h.write_u8(3);
                h.write_u32(left);
            }
            Self::Beached => h.write_u8(4),
            Self::Retract { left, land } => {
                h.write_u8(5);
                h.write_u32(left);
                if let Some(p) = land {
                    point(h, p);
                }
            }
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Afloat => "afloat",
            Self::Landing { .. } => "landing",
            Self::RunIn { .. } => "run_in",
            Self::Ramp { .. } => "ramp",
            Self::Beached => "beached",
            Self::Retract { .. } => "retract",
        }
    }
}

/// Boat values of a unit type with the rules applied.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TypeRt {
    boat: bool,
    draft_mm: u32,
    ramp_ticks: u32,
    retract_ticks: u32,
    run_in_pct: u32,
    load_ticks: u32,
    unload_ticks: u32,
    /// Most vehicles at once and their total mass (kg, overload included).
    cargo_vehicles: u32,
    cargo_kg: u32,
    /// Payload of the catalog (kg), for the seats vehicles take.
    payload_kg: u32,
    paddled: bool,
    crew: u32,
    half_length: Fixed,
}

/// Boat state of a match.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Boats {
    /// `None` in matches without a boat type.
    rules: Option<BoatRules>,
    /// Per unit type.
    types: Vec<TypeRt>,
    /// Phase of every boat that is not simply afloat.
    phases: BTreeMap<UnitId, Phase>,
}

/// A boat for the presentation (bridge `SimHost.boats`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoatView {
    pub unit: UnitId,
    /// "afloat", "landing", "run_in", "ramp", "beached" or "retract".
    pub phase: &'static str,
    /// Ramp opening in percent (0 closed, 100 down).
    pub ramp_pct: u32,
    /// Soldiers and vehicles aboard.
    pub soldiers: u32,
    pub vehicles: u32,
}

impl Boats {
    pub(super) fn new(rules: Option<BoatRules>, unit_types: &[UnitType], tick_hz: u32) -> Self {
        let Some(rules) = rules.filter(|_| unit_types.iter().any(UnitType::is_boat)) else {
            return Self::default();
        };
        let types = unit_types
            .iter()
            .map(|t| {
                if !t.is_boat() {
                    return TypeRt::default();
                }
                let time = t.combat.time_ratio;
                let ticks = |ms: u32| super::transport::ticks(ms, time, tick_hz);
                let kind = rules.kind(t.combat.category);
                let payload = t.transport.cargo_kg;
                TypeRt {
                    boat: true,
                    draft_mm: t.transport.draft_mm,
                    ramp_ticks: kind.map_or(0, |k| ticks(k.ramp_ms)),
                    retract_ticks: kind.map_or(0, |k| ticks(k.retract_ms)),
                    run_in_pct: kind.map_or(50, |k| k.run_in_pct),
                    load_ticks: kind.map_or(0, |k| ticks(k.vehicle_load_ms)),
                    unload_ticks: kind.map_or(0, |k| ticks(k.vehicle_unload_ms)),
                    cargo_vehicles: u32::from(t.transport.cargo_vehicles),
                    cargo_kg: u32::try_from(
                        u64::from(payload) * u64::from(100 + rules.rules.overload_pct) / 100,
                    )
                    .unwrap_or(u32::MAX),
                    payload_kg: payload,
                    paddled: t.transport.paddled,
                    crew: u32::from(t.transport.crew),
                    half_length: t.combat.length_m / 2,
                }
            })
            .collect();
        Self {
            rules: Some(rules),
            types,
            phases: BTreeMap::new(),
        }
    }

    /// Whether the match has boats at all (rules and state are hashed).
    pub(super) const fn active(&self) -> bool {
        self.rules.is_some()
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        let Some(rules) = &self.rules else {
            return;
        };
        rules.hash_into(h);
        h.write_u64(self.phases.len() as u64);
        for (id, phase) in &self.phases {
            h.write_u32(id.0);
            phase.hash_into(h);
        }
    }
}

/// Moves boat `unit` one tick along its route on the water (prompt 119),
/// deflected by `push` like [`super::move_unit`]: only onto cells open to
/// its draft, or, off the open water (beached, pushed into the shallows),
/// onto water at least as deep as where it lies. It floats at the water
/// level of its cell.
pub(super) fn move_boat(
    unit: &mut Unit,
    push: Vec2,
    (map, grid): (&Map, &PathGrid),
    profile: usize,
) -> Moved {
    let Route::Following {
        waypoints, next, ..
    } = &unit.route
    else {
        return Moved::Ok;
    };
    if unit.speed_pct == 0 {
        return Moved::Ok;
    }
    let (waypoint, last) = (waypoints[*next], *next + 1 == waypoints.len());
    if let Some(direction) = heading(unit) {
        let speed = unit.speed_offroad * Fixed::from_num(unit.speed_pct) / 100;
        let from = unit.position;
        let off = !grid.passable_for(profile, from);
        let here = grid.water_depth_at(from);
        let valid = |p: Vec2| {
            map.contains(p)
                && (grid.passable_for(profile, p) || (off && grid.water_depth_at(p) >= here))
        };
        let candidates = if (waypoint - from).length() <= speed {
            [Some(waypoint), None, None, None]
        } else {
            let mut velocity = direction + push;
            let length = velocity.length();
            if length > Fixed::ONE {
                velocity = Vec2::new(velocity.x / length, velocity.y / length);
            }
            let step = direction.scale(speed);
            let long = |d: Fixed| d.abs() * 4 >= speed;
            [
                Some(from + velocity.scale(speed)),
                Some(from + step),
                long(step.x).then_some(from + Vec2::new(step.x, Fixed::ZERO)),
                long(step.y).then_some(from + Vec2::new(Fixed::ZERO, step.y)),
            ]
        };
        let next_position = candidates
            .into_iter()
            .flatten()
            .find(|&p| valid(p))
            .or_else(|| {
                angled(direction)
                    .into_iter()
                    .map(|d| from + d.scale(speed))
                    .find(|&p| valid(p))
            });
        let Some(p) = next_position else {
            return Moved::Blocked;
        };
        unit.position = p;
        if let Some(level) = grid.water_level_at(p) {
            unit.height = level;
        }
    }
    advance_route(unit, waypoint, last)
}

impl Sim {
    fn boat_type(&self, i: usize) -> Option<&TypeRt> {
        self.boats
            .types
            .get(self.units[i].unit_type as usize)
            .filter(|t| t.boat)
    }

    /// Whether unit `i` is a boat.
    pub(super) fn is_boat(&self, i: usize) -> bool {
        self.boat_type(i).is_some()
    }

    fn phase(&self, id: UnitId) -> Phase {
        self.boats.phases.get(&id).copied().unwrap_or(Phase::Afloat)
    }

    fn set_phase(&mut self, id: UnitId, phase: Phase) {
        if phase == Phase::Afloat {
            self.boats.phases.remove(&id);
        } else {
            self.boats.phases.insert(id, phase);
        }
    }

    /// Whether carrier `c` lets men and vehicles on and off now: any
    /// vehicle (it stands still, checked by the transport tasks), a boat
    /// only while beached.
    pub(super) fn boat_ready(&self, c: usize) -> bool {
        !self.is_boat(c) || self.phase(self.units[c].id) == Phase::Beached
    }

    /// Where squads and vehicles get on and off boat `c`: in front of its
    /// bow (`gap` beyond it).
    pub(super) fn bow_point(&self, c: usize, gap: Fixed) -> Vec2 {
        let v = &self.units[c];
        let half = self.boat_type(c).map_or(v.footprint / 2, |t| t.half_length);
        v.position + v.facing.scale(half + gap)
    }

    /// Where men of unit type `profile` line up to board boat `c`: in
    /// front of its bow, or farther up the bank on the first ground they
    /// can stand on (a steep bank: they jump down into the boat).
    pub(super) fn boat_door(&self, c: usize, profile: usize) -> Vec2 {
        let grid = &self.paths.grid;
        (1..=8)
            .map(|m| self.bow_point(c, Fixed::from_num(m)))
            .find(|&p| self.map.contains(p) && grid.passable_for(profile, p))
            .unwrap_or_else(|| self.bow_point(c, Fixed::ONE))
    }

    /// `Land`: boats among `units` drive to the open water off `target`
    /// and run onto the beach there.
    pub(super) fn order_land(&mut self, player: PlayerId, units: &[UnitId], target: Vec2) {
        if !self.boats.active() || !self.map.contains(target) {
            return;
        }
        let boats: Vec<usize> = self
            .mobile_indices(player, units)
            .into_iter()
            .filter(|&i| self.is_boat(i))
            .collect();
        for i in boats {
            let id = self.units[i].id;
            self.units[i].combat.attack = None;
            let aground = matches!(
                self.phase(id),
                Phase::Beached | Phase::Ramp { .. } | Phase::RunIn { .. } | Phase::Retract { .. }
            );
            if !aground {
                self.request_group_path(&[(i, target)], target);
            }
            // A boat aground backs off first.
            let phase = match self.phase(id) {
                Phase::Beached | Phase::Ramp { .. } | Phase::RunIn { .. } => Phase::Retract {
                    left: self.boat_type(i).map_or(0, |t| t.retract_ticks),
                    land: Some(target),
                },
                Phase::Retract { left, .. } => Phase::Retract {
                    left,
                    land: Some(target),
                },
                Phase::Afloat | Phase::Landing { .. } => Phase::Landing { shore: target },
            };
            self.set_phase(id, phase);
        }
    }

    /// `Retract`: beached boats among `units` raise the ramp and back off.
    pub(super) fn order_retract(&mut self, player: PlayerId, units: &[UnitId]) {
        if !self.boats.active() {
            return;
        }
        for i in self.owned_indices(player, units) {
            if !self.is_boat(i) {
                continue;
            }
            let id = self.units[i].id;
            match self.phase(id) {
                Phase::Beached | Phase::Ramp { .. } | Phase::RunIn { .. } => {
                    let left = self.boat_type(i).map_or(0, |t| t.retract_ticks);
                    self.set_phase(id, Phase::Retract { left, land: None });
                }
                Phase::Landing { .. } => self.set_phase(id, Phase::Afloat),
                Phase::Afloat | Phase::Retract { .. } => {}
            }
        }
    }

    /// Orders that move a boat call off its landing (not a run-in under
    /// way: it is aground soon; a beached boat backs off by itself).
    pub(super) fn drop_boat_orders(&mut self, command: &crate::command::PlayerCommand) {
        use crate::command::Command;
        if self.boats.phases.is_empty()
            || !matches!(
                command.command,
                Command::MoveTo { .. }
                    | Command::Stop { .. }
                    | Command::FormationMove { .. }
                    | Command::QueueMove { .. }
                    | Command::Patrol { .. }
                    | Command::Secure { .. }
            )
        {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            let id = self.units[i].id;
            if matches!(self.phase(id), Phase::Landing { .. }) {
                self.set_phase(id, Phase::Afloat);
            }
        }
    }

    /// After the movement: landing, ramps, backing off, the speed of
    /// paddled boats and the water level boats float at.
    pub(super) fn update_boats(&mut self) {
        if !self.boats.active() {
            return;
        }
        let loads = self.transport_loads();
        for i in 0..self.units.len() {
            let Some(t) = self.boat_type(i).cloned() else {
                continue;
            };
            if !self.units[i].alive() {
                self.boats.phases.remove(&self.units[i].id);
                continue;
            }
            let id = self.units[i].id;
            let phase = self.phase(id);
            let phase = self.advance_boat(i, &t, phase);
            self.set_phase(id, phase);
            // Speed: halted while aground; paddled boats by their paddlers.
            let busy = !matches!(phase, Phase::Afloat | Phase::Landing { .. });
            self.units[i].speed_pct = if busy {
                0
            } else if t.paddled {
                let (soldiers, _) = super::transport::load_of(&loads, id);
                let seats = self.carrier_seats(i);
                let men = t.crew + u32::from(soldiers);
                let full = (t.crew + seats / 2).max(1);
                let min = self
                    .boats
                    .rules
                    .as_ref()
                    .map_or(25, |r| r.rules.paddle_min_pct);
                (men * 100 / full).clamp(min, 100)
            } else {
                100
            };
        }
    }

    /// One tick of boat `i` in `phase`; the phase it goes on with.
    fn advance_boat(&mut self, i: usize, t: &TypeRt, phase: Phase) -> Phase {
        match phase {
            Phase::Afloat => {
                if let Some(level) = self.paths.grid.water_level_at(self.units[i].position)
                    && self
                        .paths
                        .grid
                        .passable_for(self.units[i].unit_type as usize, self.units[i].position)
                {
                    self.units[i].height = level;
                }
                Phase::Afloat
            }
            Phase::Landing { shore } => {
                if self.units[i].route != Route::None {
                    return phase;
                }
                let max = self
                    .boats
                    .rules
                    .as_ref()
                    .map_or(80, |r| r.rules.run_in_max_m);
                Phase::RunIn {
                    shore,
                    left_mm: max.saturating_mul(1000),
                }
            }
            Phase::RunIn { shore, left_mm } => self.run_in(i, t, shore, left_mm),
            Phase::Ramp { left } => {
                if left <= 1 {
                    Phase::Beached
                } else {
                    Phase::Ramp { left: left - 1 }
                }
            }
            Phase::Beached => {
                // A new route (a move order): back off first.
                if self.units[i].route == Route::None {
                    Phase::Beached
                } else {
                    Phase::Retract {
                        left: t.retract_ticks,
                        land: None,
                    }
                }
            }
            Phase::Retract { left, land } => self.back_off(i, t, left, land),
        }
    }

    /// One tick of running onto the beach: straight on towards `shore`
    /// (and beyond it, along the same line) until the water under the bow
    /// is shallower than the draft.
    fn run_in(&mut self, i: usize, t: &TypeRt, shore: Vec2, left_mm: u32) -> Phase {
        let (pos, facing) = (self.units[i].position, self.units[i].facing);
        let level = self
            .paths
            .grid
            .water_level_at(pos)
            .unwrap_or(self.units[i].height);
        let draft = Fixed::from_num(t.draft_mm) / 1000;
        let to = shore - pos;
        let dir = if to.length() > Fixed::ONE {
            let l = to.length();
            Vec2::new(to.x / l, to.y / l)
        } else {
            facing
        };
        let speed = self.units[i].speed_offroad * Fixed::from_num(t.run_in_pct) / 100;
        let bow = |p: Vec2| p + dir.scale(t.half_length);
        let grounded = |sim: &Self, p: Vec2| {
            let b = bow(p);
            !sim.map.contains(b) || level - sim.map.height_at(b) < draft
        };
        if grounded(self, pos) {
            self.units[i].facing = dir;
            return Phase::Ramp { left: t.ramp_ticks };
        }
        let step_mm = (speed * 1000).to_num::<u32>().max(1);
        let next = pos + dir.scale(speed);
        if left_mm <= step_mm || !self.map.contains(next) {
            // Deep water up to here: no beach.
            return Phase::Afloat;
        }
        let unit = &mut self.units[i];
        unit.position = next;
        unit.facing = dir;
        unit.height = level;
        unit.vision_dirty = true;
        Phase::RunIn {
            shore,
            left_mm: left_mm - step_mm,
        }
    }

    /// One tick of backing off: the ramp comes up and the boat backs onto
    /// open water; afloat once both are done.
    fn back_off(&mut self, i: usize, t: &TypeRt, left: u32, land: Option<Vec2>) -> Phase {
        let profile = self.units[i].unit_type as usize;
        let grid = &self.paths.grid;
        let pos = self.units[i].position;
        let open = grid.passable_for(profile, pos);
        if !open {
            let speed = self.units[i].speed_offroad * Fixed::from_num(t.run_in_pct) / 100;
            let f = self.units[i].facing;
            let back = pos - f.scale(speed);
            let here = grid.water_depth_at(pos);
            if self.map.contains(back) && grid.water_depth_at(back) >= here {
                self.units[i].position = back;
                self.units[i].vision_dirty = true;
            } else {
                // Cannot back off straight: turn towards the open water.
                let target = grid.lead_in(&self.map, &self.terrain, profile, pos);
                let d = target - pos;
                if d.length() > Fixed::ZERO {
                    let l = d.length();
                    let step = Vec2::new(d.x / l, d.y / l).scale(speed.min(l));
                    self.units[i].position = pos + step;
                }
            }
        }
        if left <= 1
            && self
                .paths
                .grid
                .passable_for(profile, self.units[i].position)
        {
            match land {
                Some(shore) => {
                    self.request_group_path(&[(i, shore)], shore);
                    Phase::Landing { shore }
                }
                None => Phase::Afloat,
            }
        } else {
            Phase::Retract {
                left: left.saturating_sub(1),
                land,
            }
        }
    }

    /// Seats left for soldiers on carrier `c`: the catalog seats, on a
    /// boat less the share of the payload its vehicles use.
    pub(super) fn carrier_seats(&self, c: usize) -> u32 {
        let seats = self.seats(self.units[c].id);
        let Some(t) = self.boat_type(c) else {
            return seats;
        };
        let used = self.cargo_mass(self.units[c].id);
        if used == 0 || t.payload_kg == 0 {
            return seats;
        }
        let free = u64::from(t.payload_kg.saturating_sub(used));
        u32::try_from(u64::from(seats) * free / u64::from(t.payload_kg)).unwrap_or(0)
    }

    /// Vehicles aboard boat `c` and their mass (kg).
    fn cargo(&self, c: UnitId) -> (u32, u32) {
        self.units
            .iter()
            .filter(|u| u.link == Some(Link::Aboard(c)) && u.squad.is_none())
            .fold((0, 0), |(n, kg), u| {
                (n + 1, kg + self.unit_types[u.unit_type as usize].mass_kg)
            })
    }

    fn cargo_mass(&self, c: UnitId) -> u32 {
        self.cargo(c).1
    }

    /// Whether vehicle or gun `i` fits on boat `c` now.
    pub(super) fn fits_aboard(&self, i: usize, c: usize) -> bool {
        let Some(t) = self.boat_type(c) else {
            return false;
        };
        let u = &self.units[i];
        if u.squad.is_some() || self.is_boat(i) || !u.alive() {
            return false;
        }
        let (n, kg) = self.cargo(self.units[c].id);
        let mass = self.unit_types[u.unit_type as usize].mass_kg;
        let soldiers = super::transport::load_of(&self.transport_loads(), self.units[c].id).0;
        // Soldiers aboard keep their room: the vehicle needs the payload
        // their seats leave.
        let seats = self.seats(self.units[c].id).max(1);
        let soldier_kg =
            u32::try_from(u64::from(t.payload_kg) * u64::from(soldiers) / u64::from(seats))
                .unwrap_or(u32::MAX);
        n < t.cargo_vehicles && kg + mass + soldier_kg <= t.cargo_kg
    }

    /// Vehicle (or gun) `i` boards boat `c`.
    pub(super) fn load_vehicle(&mut self, i: usize, c: usize) {
        if !self.fits_aboard(i, c) {
            return;
        }
        let boat = self.units[c].id;
        let at = self.units[c].position;
        self.cancel_indirect(i);
        let unit = &mut self.units[i];
        stop(unit);
        unit.link = Some(Link::Aboard(boat));
        unit.combat.attack = None;
        unit.combat.attack_moving = false;
        unit.combat.auto_target = None;
        unit.orders = super::orders::OrderQueue::default();
        unit.position = at;
    }

    /// Ticks one vehicle needs to drive aboard or off boat `c`.
    pub(super) fn vehicle_ticks(&self, c: usize, load: bool) -> u32 {
        self.boat_type(c)
            .map_or(0, |t| if load { t.load_ticks } else { t.unload_ticks })
    }

    /// Vehicle (or gun) `i` leaves boat `c` over the bow: onto the beach
    /// in front of it and a little up it, or into deep water, where it is
    /// lost.
    pub(super) fn unload_vehicle(&mut self, i: usize, c: usize) {
        let half = self.units[i].footprint / 2;
        let at = self.bow_point(c, half + Fixed::ONE);
        let facing = self.units[c].facing;
        let profile = self.units[i].unit_type as usize;
        let ground_ok = self.map.contains(at)
            && self.terrain.types[usize::from(self.map.terrain_at(at))].passable;
        let unit = &mut self.units[i];
        unit.link = None;
        unit.position = at;
        unit.height = self.map.height_at(at);
        unit.facing = facing;
        unit.combat.aim = facing;
        unit.vision_dirty = true;
        stop(unit);
        if !ground_ok {
            // Off the ramp into deep water: the vehicle is lost.
            let height = self.units[i].height;
            self.units[i].hp = 0;
            self.destroy(
                i,
                crate::crossings::WATER_SHOOTER,
                Vec3::new(at.x, at.y, height),
            );
            return;
        }
        // Drive up the beach, out of the way of the next one.
        let up = at + facing.scale(self.units[i].footprint * 2);
        if self.map.contains(up) && self.paths.grid.passable_for(profile, up) {
            self.request_group_path(&[(i, up)], up);
        }
    }

    /// Whether a wreck at `p` sinks (prompt 119): in matches with boats,
    /// in water at least `sink_depth_mm` deep.
    pub(super) fn wreck_sinks(&self, p: Vec2) -> bool {
        self.boats.rules.as_ref().is_some_and(|r| {
            self.paths.grid.has_boats()
                && self.paths.grid.water_depth_at(p) >= r.rules.sink_depth_mm
        })
    }

    /// Boats of the match for the presentation (prompt 119).
    pub fn boat_views(&self) -> Vec<BoatView> {
        if !self.boats.active() {
            return Vec::new();
        }
        let loads = self.transport_loads();
        (0..self.units.len())
            .filter(|&i| self.is_boat(i) && self.units[i].alive())
            .map(|i| {
                let id = self.units[i].id;
                let t = self.boat_type(i).cloned().unwrap_or_default();
                let phase = self.phase(id);
                let ramp_pct = match phase {
                    Phase::Beached => 100,
                    Phase::Ramp { left } => 100 - (left * 100 / t.ramp_ticks.max(1)).min(100),
                    Phase::Retract { left, .. } => {
                        // The ramp comes up in the first part of backing off.
                        let gone = t.retract_ticks.saturating_sub(left);
                        100 - (gone * 100 / t.ramp_ticks.max(1)).min(100)
                    }
                    _ => 0,
                };
                BoatView {
                    unit: id,
                    phase: phase.name(),
                    ramp_pct,
                    soldiers: u32::from(super::transport::load_of(&loads, id).0),
                    vehicles: self.cargo(id).0,
                }
            })
            .collect()
    }

    /// Phase of boat `id` ("afloat", "beached", ...), `None` for no boat.
    pub fn boat_phase(&self, id: UnitId) -> Option<&'static str> {
        let i = self.index_of(id)?;
        self.is_boat(i).then(|| self.phase(id).name())
    }

    /// Vehicles aboard boat `id` (sorted ids), for the HUD.
    pub fn boat_cargo(&self, id: UnitId) -> Vec<UnitId> {
        self.units
            .iter()
            .filter(|u| u.link == Some(Link::Aboard(id)) && u.squad.is_none())
            .map(|u| u.id)
            .collect()
    }

    /// Puts unit `unit` (a squad, a vehicle or a gun) aboard carrier
    /// `carrier` at once, as a mission's `ride` does (prompt 119). For
    /// setting up a match before its first tick (tests, tools); during a
    /// match only commands change the state. Returns whether it is aboard.
    pub fn put_aboard(&mut self, unit: UnitId, carrier: UnitId) -> bool {
        let (Some(i), Some(c)) = (self.index_of(unit), self.index_of(carrier)) else {
            return false;
        };
        if i == c || self.units[i].owner != self.units[c].owner {
            return false;
        }
        if self.units[i].squad.is_some() {
            self.board(i, c);
        } else {
            self.load_vehicle(i, c);
        }
        self.units[i].link.is_some()
    }

    /// Whether unit `id` rides aboard a carrier (prompt 119: also vehicles
    /// on boats).
    pub fn is_aboard(&self, id: UnitId) -> bool {
        self.index_of(id).is_some_and(|i| aboard(&self.units[i]))
    }

    /// Whether unit type `unit_type` is a boat (size class of the path
    /// grid).
    pub fn is_boat_type(&self, unit_type: u32) -> bool {
        (unit_type as usize) < self.paths.grid.profiles()
            && self.paths.grid.class_of(unit_type as usize) == SizeClass::Boat
    }
}
