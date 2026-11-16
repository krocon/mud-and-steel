//! Transport, mounting and towing (prompt 034): infantry squads ride in
//! trucks and halftracks or on tanks, trucks and halftracks tow guns.
//! Rules and catalog data: `crate::transport`.
//!
//! - A passenger squad stays a unit (same id) with
//!   [`Link::Aboard`]: it moves with its vehicle, cannot be seen or hit by
//!   the enemy, does not fire and takes no orders but `Disembark`
//!   (`Sim::owned_indices`). Its soldiers stand at the vehicle's center.
//! - A towed gun stays a unit with [`Link::Towed`]: it trails its tractor
//!   like a trailer, can be seen and hit, cannot fire and takes no move
//!   orders (`Sim::mobile_indices`).
//! - Orders become tasks (one per unit): walking to the vehicle and
//!   mounting, dismounting, driving to the gun and hitching, unhitching.
//!   Timers run only while the vehicle stands.
//! - Losses: a carrier that loses hit points or is hit without damage
//!   costs passengers by chance, a destroyed one most of them; survivors
//!   jump off. A gun whose tractor is destroyed is unhitched (and not
//!   ready until its unlimber time is over).
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::update_transport`]
//! after the movement (tasks, then passengers and guns follow their
//! vehicles), [`Sim::transport_losses`] after the projectiles and shells.

use super::{Route, Sim, Unit, stop};
use crate::combat::{CombatEvent, HitKind, Vec3};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::transport::TransportRules;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How a unit is attached to a vehicle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Link {
    /// A squad riding in or on the vehicle.
    Aboard(UnitId),
    /// A gun towed by the vehicle.
    Towed(UnitId),
}

/// Whether a unit rides in or on a vehicle.
pub(super) const fn aboard(unit: &Unit) -> bool {
    matches!(unit.link, Some(Link::Aboard(_)))
}

/// The vehicle a passenger rides in, or the tractor of a towed gun.
pub(super) const fn carrier_of(unit: &Unit) -> Option<UnitId> {
    match unit.link {
        Some(Link::Aboard(c) | Link::Towed(c)) => Some(c),
        None => None,
    }
}

/// Soldiers aboard and the towed gun of a vehicle, from
/// [`Sim::transport_loads`].
pub(super) fn load_of(
    loads: &[(UnitId, u16, Option<UnitId>)],
    id: UnitId,
) -> (u16, Option<UnitId>) {
    loads
        .binary_search_by_key(&id, |l| l.0)
        .map_or((0, None), |k| (loads[k].1, loads[k].2))
}

/// Transport values of a unit type with the rules applied.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TypeRt {
    /// Soldiers it carries (0: no carrier).
    seats: u32,
    /// Index into the rules' carriers.
    carrier: Option<usize>,
    embark_ticks: u32,
    disembark_ticks: u32,
    /// Engine power (hp) and mass (kg) of a vehicle that can tow.
    tow: Option<(u32, u32)>,
    /// Limber and unlimber ticks and mass (kg) of a towed gun.
    gun: Option<(u32, u32, u32)>,
}

/// What a unit is doing for transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Task {
    /// A squad walks to `carrier` (towards `goal`) and mounts; `left`
    /// ticks of mounting once it stands next to it.
    Embark {
        carrier: UnitId,
        goal: Vec2,
        left: Option<u32>,
    },
    /// A passenger gets off after `left` ticks while its vehicle stands.
    Disembark { left: u32 },
    /// A vehicle drives to `gun` and hitches it (`left` ticks of
    /// limbering once it is there).
    Hitch {
        gun: UnitId,
        goal: Vec2,
        left: Option<u32>,
    },
    /// A gun is unhitched and brought into action in `left` ticks.
    Unhitch { left: u32 },
}

impl Task {
    fn hash_into(&self, h: &mut StableHasher) {
        let opt = |h: &mut StableHasher, v: Option<u32>| h.write_u32(v.unwrap_or(u32::MAX));
        match *self {
            Self::Embark {
                carrier,
                goal,
                left,
            } => {
                h.write_u8(0);
                h.write_u32(carrier.0);
                h.write_i64(goal.x.to_bits());
                h.write_i64(goal.y.to_bits());
                opt(h, left);
            }
            Self::Disembark { left } => {
                h.write_u8(1);
                h.write_u32(left);
            }
            Self::Hitch { gun, goal, left } => {
                h.write_u8(2);
                h.write_u32(gun.0);
                h.write_i64(goal.x.to_bits());
                h.write_i64(goal.y.to_bits());
                opt(h, left);
            }
            Self::Unhitch { left } => {
                h.write_u8(3);
                h.write_u32(left);
            }
        }
    }
}

/// Transport state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Transport {
    rules: TransportRules,
    /// Per unit type.
    types: Vec<TypeRt>,
    /// Running tasks, by unit.
    tasks: BTreeMap<UnitId, Task>,
    /// Hit points of every vehicle with passengers after the last tick.
    hp_seen: BTreeMap<UnitId, u32>,
    gap: Fixed,
    reach: Fixed,
}

/// Real milliseconds → ticks with the time factor `(num, den)`, rounded.
fn ticks(ms: u32, (num, den): (u32, u32), tick_hz: u32) -> u32 {
    let (num, den) = if num == 0 || den == 0 {
        (1, 1)
    } else {
        (u64::from(num), u64::from(den))
    };
    let hz = u64::from(tick_hz.max(1));
    u32::try_from((u64::from(ms) * den * hz + 500 * num) / (1000 * num)).unwrap_or(u32::MAX)
}

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

impl Transport {
    pub(super) fn new(rules: TransportRules, unit_types: &[UnitType], tick_hz: u32) -> Self {
        let types = unit_types
            .iter()
            .map(|t| {
                let category = t.combat.category;
                let time = t.combat.time_ratio;
                let mut rt = TypeRt::default();
                if let Some(k) = rules
                    .carriers
                    .iter()
                    .position(|c| Some(c.category) == category)
                {
                    let c = &rules.carriers[k];
                    rt.seats = if c.riders > 0 {
                        c.riders
                    } else {
                        u32::from(t.transport.passengers)
                    };
                    rt.carrier = (rt.seats > 0).then_some(k);
                    rt.embark_ticks = ticks(c.embark_ms, time, tick_hz);
                    rt.disembark_ticks = ticks(c.disembark_ms, time, tick_hz);
                    if c.tows && t.transport.power_hp > 0 && t.mass_kg > 0 && t.mobile() {
                        rt.tow = Some((t.transport.power_hp, t.mass_kg));
                    }
                }
                // Static gun positions (fortifications, e.g. flak of prompt
                // 033) are no towed guns.
                if let Some(g) = rules.gun(category).filter(|_| t.mobile()) {
                    rt.gun = Some((
                        ticks(g.limber_ms, time, tick_hz),
                        ticks(g.unlimber_ms, time, tick_hz),
                        t.mass_kg,
                    ));
                }
                rt
            })
            .collect();
        let gap = mm(rules.towing.hitch_gap_mm);
        let reach = mm(rules.towing.reach_mm);
        Self {
            rules,
            types,
            tasks: BTreeMap::new(),
            hp_seen: BTreeMap::new(),
            gap,
            reach,
        }
    }

    /// Whether a tractor of type `t` may pull a gun of type `g`: enough
    /// engine power per tonne of both.
    fn can_tow(&self, t: usize, g: usize) -> bool {
        let (Some((hp, mass)), Some((_, _, gun_mass))) = (self.types[t].tow, self.types[g].gun)
        else {
            return false;
        };
        u64::from(hp) * 1000
            >= u64::from(self.rules.towing.min_hp_per_tonne)
                * (u64::from(mass) + u64::from(gun_mass))
    }

    fn carrier_rules(&self, t: usize) -> Option<&crate::transport::CarrierRules> {
        self.types[t].carrier.map(|k| &self.rules.carriers[k])
    }
}

impl Sim {
    fn index_of(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    fn ttype(&self, i: usize) -> &TypeRt {
        &self.transport.types[self.units[i].unit_type as usize]
    }

    /// Soldiers riding in or on vehicle `c`.
    fn seats_used(&self, c: UnitId) -> u32 {
        self.units
            .iter()
            .filter(|u| u.link == Some(Link::Aboard(c)))
            .filter_map(|u| u.squad.as_ref())
            .map(|s| s.alive_count())
            .sum()
    }

    /// Where a squad waits to mount vehicle `c`: behind it.
    fn rear_of(&self, c: usize) -> Vec2 {
        let v = &self.units[c];
        v.position - v.facing.scale(v.footprint / 2 + Fixed::from_num(2))
    }

    /// Plans a path for unit `i` to `goal` (unless it is close already).
    fn approach(&mut self, i: usize, goal: Vec2) {
        self.request_group_path(&[(i, goal)], goal);
    }

    /// Whether units `i` and `j` are close enough for mounting or hitching.
    fn close(&self, i: usize, j: usize) -> bool {
        let (a, b) = (&self.units[i], &self.units[j]);
        (a.position - b.position).length() <= (a.spread + b.spread) / 2 + self.transport.reach
    }

    /// `Embark`: squads of `player` walk to `carrier` and mount it.
    pub(super) fn order_embark(&mut self, player: PlayerId, units: &[UnitId], carrier: UnitId) {
        let Some(c) = self.index_of(carrier) else {
            return;
        };
        let v = &self.units[c];
        if v.owner != player || !v.alive() || v.link.is_some() || self.ttype(c).seats == 0 {
            return;
        }
        let goal = self.rear_of(c);
        for i in self.mobile_indices(player, units) {
            if i == c || self.units[i].squad.is_none() {
                continue;
            }
            self.leave_building(i);
            self.units[i].combat.attack = None;
            self.units[i].combat.attack_moving = false;
            self.cancel_indirect(i);
            self.transport.tasks.insert(
                self.units[i].id,
                Task::Embark {
                    carrier,
                    goal,
                    left: None,
                },
            );
            if self.close(i, c) {
                stop(&mut self.units[i]);
            } else {
                self.approach(i, goal);
            }
        }
    }

    /// `Disembark`: passengers (or everybody aboard listed vehicles) get
    /// off once their vehicle stands.
    pub(super) fn order_disembark(&mut self, player: PlayerId, units: &[UnitId]) {
        let mut ids: Vec<UnitId> = units.to_vec();
        ids.sort_unstable();
        ids.dedup();
        let mut riders: Vec<usize> = Vec::new();
        for id in ids {
            let Some(i) = self.index_of(id) else { continue };
            let u = &self.units[i];
            if u.owner != player || !u.alive() {
                continue;
            }
            if aboard(u) {
                riders.push(i);
            } else {
                riders.extend(
                    (0..self.units.len()).filter(|&k| self.units[k].link == Some(Link::Aboard(id))),
                );
            }
        }
        riders.sort_unstable();
        riders.dedup();
        for i in riders {
            let Some(Link::Aboard(c)) = self.units[i].link else {
                continue;
            };
            let id = self.units[i].id;
            if matches!(self.transport.tasks.get(&id), Some(Task::Disembark { .. })) {
                continue;
            }
            let left = self
                .index_of(c)
                .map_or(0, |c| self.ttype(c).disembark_ticks);
            self.transport.tasks.insert(id, Task::Disembark { left });
        }
    }

    /// `Tow`: the nearest free vehicle of `units` that can pull `gun`
    /// drives to it and hitches it.
    pub(super) fn order_tow(&mut self, player: PlayerId, units: &[UnitId], gun: UnitId) {
        let Some(g) = self.index_of(gun) else { return };
        let gu = &self.units[g];
        if gu.owner != player || !gu.alive() || gu.link.is_some() || self.ttype(g).gun.is_none() {
            return;
        }
        if self
            .transport
            .tasks
            .values()
            .any(|t| matches!(t, Task::Hitch { gun: h, .. } if *h == gun))
        {
            return;
        }
        let towing: Vec<UnitId> = self
            .units
            .iter()
            .filter_map(|u| match u.link {
                Some(Link::Towed(t)) => Some(t),
                _ => None,
            })
            .collect();
        let gun_type = gu.unit_type as usize;
        let at = gu.position;
        let tractor = self
            .mobile_indices(player, units)
            .into_iter()
            .filter(|&t| {
                let id = self.units[t].id;
                self.transport
                    .can_tow(self.units[t].unit_type as usize, gun_type)
                    && !towing.contains(&id)
            })
            .min_by_key(|&t| {
                let d = self.units[t].position - at;
                let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
                (x * x + y * y, self.units[t].id)
            });
        let Some(t) = tractor else { return };
        self.units[t].combat.attack = None;
        self.units[t].combat.attack_moving = false;
        self.cancel_indirect(t);
        stop(&mut self.units[g]);
        self.transport.tasks.insert(
            self.units[t].id,
            Task::Hitch {
                gun,
                goal: at,
                left: None,
            },
        );
        if self.close(t, g) {
            stop(&mut self.units[t]);
        } else {
            self.approach(t, at);
        }
    }

    /// `Unlimber`: towed guns of the listed vehicles (or the listed towed
    /// guns) are unhitched; a hitch under way is called off.
    pub(super) fn order_unlimber(&mut self, player: PlayerId, units: &[UnitId]) {
        for i in self.owned_indices(player, units) {
            let id = self.units[i].id;
            if matches!(self.transport.tasks.get(&id), Some(Task::Hitch { .. })) {
                self.transport.tasks.remove(&id);
                continue;
            }
            let gun = match self.units[i].link {
                Some(Link::Towed(_)) => Some(i),
                _ => (0..self.units.len()).find(|&k| self.units[k].link == Some(Link::Towed(id))),
            };
            let Some(g) = gun else { continue };
            let gid = self.units[g].id;
            if matches!(self.transport.tasks.get(&gid), Some(Task::Unhitch { .. })) {
                continue;
            }
            let left = self.ttype(g).gun.map_or(0, |(_, u, _)| u);
            self.transport.tasks.insert(gid, Task::Unhitch { left });
        }
    }

    /// Orders that replace what a unit does call off its walk to a vehicle
    /// or its drive to a gun (not a dismount or an unhitch under way).
    pub(super) fn drop_transport_tasks(&mut self, command: &PlayerCommand) {
        if !matches!(
            command.command,
            Command::MoveTo { .. }
                | Command::Stop { .. }
                | Command::Attack { .. }
                | Command::AttackGround { .. }
                | Command::Garrison { .. }
                | Command::Barrage { .. }
                | Command::Patrol { .. }
                | Command::Embark { .. }
                | Command::Tow { .. }
        ) || self.transport.tasks.is_empty()
        {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            let id = self.units[i].id;
            if matches!(
                self.transport.tasks.get(&id),
                Some(Task::Embark { .. } | Task::Hitch { .. })
            ) {
                self.transport.tasks.remove(&id);
            }
        }
    }

    /// After the movement: tasks advance, then passengers and towed guns
    /// follow their vehicles.
    pub(super) fn update_transport(&mut self) {
        let ids: Vec<UnitId> = self.transport.tasks.keys().copied().collect();
        for id in ids {
            let Some(task) = self.transport.tasks.get(&id).copied() else {
                continue;
            };
            let keep = match self.index_of(id) {
                Some(i) if self.units[i].alive() => self.advance_task(i, task),
                _ => None,
            };
            match keep {
                Some(t) => {
                    self.transport.tasks.insert(id, t);
                }
                None => {
                    self.transport.tasks.remove(&id);
                }
            }
        }
        self.carry_along();
    }

    /// One tick of a task of unit `i`; the task as it goes on, `None`
    /// when it is done or given up.
    fn advance_task(&mut self, i: usize, task: Task) -> Option<Task> {
        match task {
            Task::Embark {
                carrier,
                goal,
                left,
            } => {
                let c = self.index_of(carrier)?;
                let v = &self.units[c];
                if self.units[i].link.is_some()
                    || !v.alive()
                    || v.link.is_some()
                    || v.owner != self.units[i].owner
                {
                    return None;
                }
                let ticks = self.ttype(c).embark_ticks;
                let (goal, left) = self.walk_up(i, c, goal, left, ticks)?;
                if left == Some(0) {
                    self.board(i, c);
                    return None;
                }
                Some(Task::Embark {
                    carrier,
                    goal,
                    left,
                })
            }
            Task::Disembark { left } => {
                let Some(Link::Aboard(carrier)) = self.units[i].link else {
                    return None;
                };
                let Some(c) = self.index_of(carrier) else {
                    let at = self.units[i].position;
                    self.dismount(i, at);
                    return None;
                };
                if self.units[c].route != Route::None {
                    return Some(Task::Disembark { left });
                }
                let left = left.saturating_sub(1);
                if left == 0 {
                    let at = self.drop_spot(c, i);
                    self.dismount(i, at);
                    return None;
                }
                Some(Task::Disembark { left })
            }
            Task::Hitch { gun, goal, left } => {
                let g = self.index_of(gun)?;
                let tractor = self.units[i].id;
                let gu = &self.units[g];
                if !gu.alive()
                    || gu.link.is_some()
                    || gu.owner != self.units[i].owner
                    || self.units[i].link.is_some()
                    || self
                        .units
                        .iter()
                        .any(|u| u.link == Some(Link::Towed(tractor)))
                {
                    return None;
                }
                let ticks = self.ttype(g).gun.map_or(0, |(l, _, _)| l);
                let (goal, left) = self.walk_up(i, g, goal, left, ticks)?;
                if left == Some(0) {
                    self.hitch(i, g);
                    return None;
                }
                Some(Task::Hitch { gun, goal, left })
            }
            Task::Unhitch { left } => {
                let tractor = match self.units[i].link {
                    Some(Link::Towed(t)) => self.index_of(t),
                    Some(Link::Aboard(_)) => return None,
                    None => None,
                };
                if tractor.is_some_and(|t| self.units[t].route != Route::None) {
                    return Some(Task::Unhitch { left });
                }
                let left = left.saturating_sub(1);
                if left == 0 {
                    self.unhitch(i, tractor);
                    return None;
                }
                Some(Task::Unhitch { left })
            }
        }
    }

    /// Unit `i` walks or drives up to unit `j` (towards `goal`), then waits
    /// `ticks` while both stand. Returns the goal and the ticks left
    /// (`None` while on the way), or `None` if it cannot get there.
    fn walk_up(
        &mut self,
        i: usize,
        j: usize,
        goal: Vec2,
        left: Option<u32>,
        ticks: u32,
    ) -> Option<(Vec2, Option<u32>)> {
        if self.close(i, j) {
            if self.units[i].route != Route::None {
                stop(&mut self.units[i]);
            }
            if self.units[j].route != Route::None {
                // The vehicle drives off: start again once it stands.
                return Some((goal, None));
            }
            let left = left.unwrap_or(ticks.saturating_add(1)).saturating_sub(1);
            return Some((goal, Some(left)));
        }
        // Where to go now: behind a vehicle to mount, to a gun to hitch.
        let wanted = if self.ttype(j).seats > 0 && self.units[i].squad.is_some() {
            self.rear_of(j)
        } else {
            self.units[j].position
        };
        let moved = (wanted - goal).length() > self.transport.reach;
        if self.units[i].route == Route::None {
            if !moved {
                // Arrived (or stuck) without getting close.
                return None;
            }
            self.approach(i, wanted);
            return Some((wanted, None));
        }
        if moved {
            self.approach(i, wanted);
            return Some((wanted, None));
        }
        Some((goal, None))
    }

    /// Squad `i` mounts vehicle `c` if its seats suffice.
    fn board(&mut self, i: usize, c: usize) {
        let carrier = self.units[c].id;
        let need = self.units[i].squad.as_ref().map_or(0, |s| s.alive_count());
        if need == 0 || self.seats_used(carrier) + need > self.ttype(c).seats {
            return;
        }
        let at = self.units[c].position;
        self.set_squad_at(i, at);
        let unit = &mut self.units[i];
        stop(unit);
        unit.link = Some(Link::Aboard(carrier));
        unit.combat.attack = None;
        unit.combat.attack_moving = false;
        unit.combat.auto_target = None;
        unit.orders = super::orders::OrderQueue::default();
        unit.position = at;
        self.cancel_indirect(i);
        let hp = self.units[c].hp;
        self.transport.hp_seen.insert(carrier, hp);
    }

    /// Passenger `i` gets off at `at`.
    fn dismount(&mut self, i: usize, at: Vec2) {
        let facing = carrier_of(&self.units[i])
            .and_then(|c| self.index_of(c))
            .map_or(self.units[i].facing, |c| self.units[c].facing);
        self.set_squad_at(i, at);
        let unit = &mut self.units[i];
        stop(unit);
        unit.link = None;
        unit.position = at;
        unit.height = self.map.height_at(at);
        unit.facing = facing;
        unit.vision_dirty = true;
    }

    /// A free spot next to vehicle `c` for squad `i`: behind it, else to
    /// its sides or in front, else its own position.
    fn drop_spot(&self, c: usize, i: usize) -> Vec2 {
        let v = &self.units[c];
        let f = v.facing;
        let right = Vec2::new(-f.y, f.x);
        let d = v.footprint / 2 + self.units[i].spread / 2 + Fixed::ONE;
        let profile = self.units[i].unit_type as usize;
        let grid = &self.paths.grid;
        [
            v.position - f.scale(d),
            v.position + right.scale(d),
            v.position - right.scale(d),
            v.position + f.scale(d),
        ]
        .into_iter()
        .find(|&p| self.map.contains(p) && grid.passable_for(profile, p))
        .unwrap_or(v.position)
    }

    /// Tractor `t` has hitched gun `g`: the gun trails it, the tractor
    /// slows down by the gun's weight.
    fn hitch(&mut self, t: usize, g: usize) {
        if !self.transport.can_tow(
            self.units[t].unit_type as usize,
            self.units[g].unit_type as usize,
        ) {
            return;
        }
        let tractor = self.units[t].id;
        let (tow, gun) = (self.ttype(t).tow, self.ttype(g).gun);
        if let (Some((_, mass)), Some((_, _, gun_mass))) = (tow, gun) {
            let total = u64::from(mass) + u64::from(gun_mass);
            self.units[t].speed_pct =
                u32::try_from(u64::from(mass) * 100 / total.max(1)).unwrap_or(100);
        }
        // The crew turns the gun round onto the hook.
        let (pos, facing) = (self.units[t].position, self.units[t].facing);
        let hook = pos - facing.scale(self.units[t].footprint / 2);
        let back = Vec2::new(-facing.x, -facing.y);
        let at = hook + back.scale(self.transport.gap + self.units[g].footprint / 2);
        self.cancel_indirect(g);
        let unit = &mut self.units[g];
        stop(unit);
        unit.link = Some(Link::Towed(tractor));
        unit.combat.attack = None;
        unit.combat.attack_moving = false;
        unit.combat.auto_target = None;
        unit.orders = super::orders::OrderQueue::default();
        unit.position = at;
        unit.height = self.map.height_at(at);
        unit.facing = back;
        unit.combat.aim = back;
    }

    /// Gun `g` is unhitched (from tractor `t`, if it still exists) and
    /// points the way the tractor drove.
    fn unhitch(&mut self, g: usize, t: Option<usize>) {
        let facing = t.map_or_else(
            || {
                let f = self.units[g].facing;
                Vec2::new(-f.x, -f.y)
            },
            |t| self.units[t].facing,
        );
        if let Some(t) = t {
            self.units[t].speed_pct = 100;
        }
        let unit = &mut self.units[g];
        if matches!(unit.link, Some(Link::Towed(_))) {
            unit.facing = facing;
            unit.combat.aim = facing;
        }
        unit.link = None;
        unit.vision_dirty = true;
    }

    /// Passengers stand at their vehicle's center; towed guns trail their
    /// tractor at the hook, pointing away from it.
    fn carry_along(&mut self) {
        for i in 0..self.units.len() {
            let Some(link) = self.units[i].link else {
                continue;
            };
            let Some(c) = carrier_of(&self.units[i]).and_then(|c| self.index_of(c)) else {
                continue;
            };
            let (pos, height, facing, half) = {
                let v = &self.units[c];
                (v.position, v.height, v.facing, v.footprint / 2)
            };
            match link {
                Link::Aboard(_) => {
                    if self.units[i].position != pos {
                        self.set_squad_at(i, pos);
                    }
                    let unit = &mut self.units[i];
                    unit.position = pos;
                    unit.height = height;
                    unit.facing = facing;
                }
                Link::Towed(_) => {
                    let hook = pos - facing.scale(half);
                    let unit = &self.units[i];
                    let d = unit.position - hook;
                    let length = d.length();
                    let dir = if length > Fixed::ONE / 100 {
                        Vec2::new(d.x / length, d.y / length)
                    } else {
                        Vec2::new(-facing.x, -facing.y)
                    };
                    let at = hook + dir.scale(self.transport.gap + unit.footprint / 2);
                    let ground = self.map.height_at(at);
                    let unit = &mut self.units[i];
                    unit.position = at;
                    unit.height = ground;
                    unit.facing = dir;
                    unit.combat.aim = dir;
                }
            }
        }
    }

    /// After projectiles and shells: losses of passengers on hit or
    /// destroyed vehicles, guns of destroyed tractors, links of destroyed
    /// passengers and guns.
    pub(super) fn transport_losses(&mut self) {
        let loads = self.transport_loads();
        for &(carrier, _, towing) in &loads {
            let Some(c) = self.index_of(carrier) else {
                continue;
            };
            let rules = self
                .transport
                .carrier_rules(self.units[c].unit_type as usize)
                .copied();
            let by = self.events.iter().rev().find_map(|e| match *e {
                CombatEvent::Destroyed { unit, by, .. } if unit == carrier => Some(by),
                CombatEvent::Hit {
                    kind: HitKind::Unit(u),
                    shooter,
                    ..
                } if u == carrier => Some(shooter),
                _ => None,
            });
            let by = by.unwrap_or(carrier);
            let riders: Vec<usize> = (0..self.units.len())
                .filter(|&k| self.units[k].link == Some(Link::Aboard(carrier)))
                .collect();
            if !self.units[c].alive() {
                let pct = rules.map_or(100, |r| r.destroyed_pct);
                for &k in &riders {
                    self.kill_riders(k, pct, 1, by);
                    if self.units[k].alive() {
                        let at = self.drop_spot(c, k);
                        self.dismount(k, at);
                    } else {
                        self.units[k].link = None;
                    }
                    self.transport.tasks.remove(&self.units[k].id);
                }
                if let Some(gun) = towing
                    && let Some(g) = self.index_of(gun)
                {
                    self.unhitch(g, None);
                    if self.units[g].alive() {
                        let left = self.ttype(g).gun.map_or(0, |(_, u, _)| u);
                        self.transport.tasks.insert(gun, Task::Unhitch { left });
                    }
                }
                continue;
            }
            let Some(rules) = rules else { continue };
            // Every round that hits the carrier hits a passenger with the
            // rule's chance; hit points lost without a round (fragments of
            // a shell) count as one damaging hit.
            let (mut damaging, mut bounces) = (0u32, 0u32);
            for e in &self.events {
                if let CombatEvent::Hit {
                    kind: HitKind::Unit(u),
                    damage,
                    ..
                } = *e
                    && u == carrier
                {
                    if damage > 0 {
                        damaging += 1;
                    } else {
                        bounces += 1;
                    }
                }
            }
            let hurt = self
                .transport
                .hp_seen
                .get(&carrier)
                .is_some_and(|&hp| self.units[c].hp < hp);
            if hurt && damaging == 0 {
                damaging = 1;
            }
            for (rolls, pct) in [(damaging, rules.hit_pct), (bounces, rules.bounce_pct)] {
                for _ in 0..rolls {
                    if pct > 0 && self.rng.below(100) < pct {
                        self.kill_one_rider(&riders, by);
                    }
                }
            }
            for &k in &riders {
                if !self.units[k].alive() {
                    self.units[k].link = None;
                    self.transport.tasks.remove(&self.units[k].id);
                }
            }
        }
        // Destroyed towed guns free their tractor.
        for g in 0..self.units.len() {
            if let Some(Link::Towed(t)) = self.units[g].link
                && !self.units[g].alive()
            {
                self.units[g].link = None;
                if let Some(t) = self.index_of(t) {
                    self.units[t].speed_pct = 100;
                }
            }
        }
        // Hit points of the vehicles that carry somebody now.
        let mut seen = BTreeMap::new();
        for &(carrier, passengers, _) in &self.transport_loads() {
            if passengers > 0
                && let Some(c) = self.index_of(carrier)
                && self.units[c].alive()
            {
                seen.insert(carrier, self.units[c].hp);
            }
        }
        self.transport.hp_seen = seen;
    }

    /// A random living soldier of the squads `riders` dies.
    fn kill_one_rider(&mut self, riders: &[usize], by: UnitId) {
        let alive = |u: &Unit| u.squad.as_ref().map_or(0, |s| s.alive_count());
        let total: u32 = riders.iter().map(|&k| alive(&self.units[k])).sum();
        if total == 0 {
            return;
        }
        let mut pick = self.rng.below(total);
        for &k in riders {
            let n = alive(&self.units[k]);
            if pick >= n {
                pick -= n;
                continue;
            }
            let unit = &mut self.units[k];
            let Some(squad) = unit.squad.as_mut() else {
                return;
            };
            if let Some(soldier) = squad
                .soldiers
                .iter_mut()
                .filter(|s| s.alive())
                .nth(pick as usize)
            {
                soldier.hp = 0;
            }
            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            if unit.hp == 0 {
                let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
                self.destroy(k, by, at);
            }
            return;
        }
    }

    /// Every living soldier of squad `k` dies with `pct` percent, rolled
    /// `rolls` times; a squad without survivors is destroyed (by `by`).
    fn kill_riders(&mut self, k: usize, pct: u32, rolls: u32, by: UnitId) {
        if pct == 0 || !self.units[k].alive() {
            return;
        }
        let unit = &mut self.units[k];
        let Some(squad) = unit.squad.as_mut() else {
            return;
        };
        for soldier in squad.soldiers.iter_mut().filter(|s| s.alive()) {
            for _ in 0..rolls {
                if self.rng.below(100) < pct {
                    soldier.hp = 0;
                    break;
                }
            }
        }
        unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
        if unit.hp == 0 {
            let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
            self.destroy(k, by, at);
        }
    }

    /// Soldiers aboard and the towed gun of every vehicle that has either,
    /// by vehicle id.
    pub(super) fn transport_loads(&self) -> Vec<(UnitId, u16, Option<UnitId>)> {
        let mut loads: BTreeMap<UnitId, (u16, Option<UnitId>)> = BTreeMap::new();
        for u in &self.units {
            match u.link {
                Some(Link::Aboard(c)) => {
                    let n = u.squad.as_ref().map_or(0, |s| s.alive_count());
                    let e = loads.entry(c).or_default();
                    e.0 = e.0.saturating_add(u16::try_from(n).unwrap_or(u16::MAX));
                }
                Some(Link::Towed(t)) => loads.entry(t).or_default().1 = Some(u.id),
                None => {}
            }
        }
        loads.into_iter().map(|(c, (n, g))| (c, n, g)).collect()
    }

    /// Whether unit `id` is mounting, dismounting, limbering or
    /// unlimbering (or a gun being limbered).
    pub(super) fn transport_busy(&self, id: UnitId) -> bool {
        match self.transport.tasks.get(&id) {
            Some(Task::Disembark { .. } | Task::Unhitch { .. }) => return true,
            Some(Task::Embark { left, .. } | Task::Hitch { left, .. }) if left.is_some() => {
                return true;
            }
            _ => {}
        }
        self.transport
            .tasks
            .values()
            .any(|t| matches!(t, Task::Hitch { gun, left: Some(_), .. } if *gun == id))
    }

    /// Whether unit `i` cannot fire because of transport: it rides, is
    /// towed, is being limbered or unlimbered, or is a gun its crew pushes.
    pub(super) fn transport_blocks_fire(&self, i: usize) -> bool {
        let unit = &self.units[i];
        if unit.link.is_some() {
            return true;
        }
        let rt = self.ttype(i);
        if rt.gun.is_none() {
            return false;
        }
        unit.route != Route::None || self.transport_busy(unit.id)
    }

    pub(super) fn hash_transport(&self, h: &mut StableHasher) {
        self.transport.rules.hash_into(h);
        h.write_u64(self.transport.tasks.len() as u64);
        for (id, task) in &self.transport.tasks {
            h.write_u32(id.0);
            task.hash_into(h);
        }
        h.write_u64(self.transport.hp_seen.len() as u64);
        for (id, hp) in &self.transport.hp_seen {
            h.write_u32(id.0);
            h.write_u32(*hp);
        }
        for u in &self.units {
            match u.link {
                None => {}
                Some(Link::Aboard(c)) => {
                    h.write_u32(u.id.0);
                    h.write_u8(1);
                    h.write_u32(c.0);
                }
                Some(Link::Towed(t)) => {
                    h.write_u32(u.id.0);
                    h.write_u8(2);
                    h.write_u32(t.0);
                }
            }
        }
    }

    /// Soldiers riding in or on vehicle `id` and the gun it tows (public
    /// for the HUD and the AI).
    pub fn transport_load(&self, id: UnitId) -> (u32, Option<UnitId>) {
        let loads = self.transport_loads();
        let (n, g) = load_of(&loads, id);
        (u32::from(n), g)
    }

    /// Seats of unit `id` for soldiers (0: no carrier).
    pub fn seats(&self, id: UnitId) -> u32 {
        self.index_of(id).map_or(0, |i| self.ttype(i).seats)
    }

    /// Whether vehicle `tractor` could tow gun `gun` (types and power).
    pub fn can_tow(&self, tractor: UnitId, gun: UnitId) -> bool {
        match (self.index_of(tractor), self.index_of(gun)) {
            (Some(t), Some(g)) => self.transport.can_tow(
                self.units[t].unit_type as usize,
                self.units[g].unit_type as usize,
            ),
            _ => false,
        }
    }

    /// Whether unit type `unit_type` is a towed gun.
    pub fn is_towed_gun_type(&self, unit_type: u32) -> bool {
        self.transport
            .types
            .get(unit_type as usize)
            .is_some_and(|t| t.gun.is_some())
    }

    /// Seats of unit type `unit_type` and whether it can tow at all.
    pub fn transport_type(&self, unit_type: u32) -> (u32, bool) {
        self.transport
            .types
            .get(unit_type as usize)
            .map_or((0, false), |t| (t.seats, t.tow.is_some()))
    }
}
