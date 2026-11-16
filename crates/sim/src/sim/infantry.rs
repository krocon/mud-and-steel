//! The infantry update (prompt 022): soldiers in formation, stances, cover
//! spots and buildings. Rules: `crate::infantry` (`data/infantry.toml`).
//!
//! A squad is one [`Unit`]: orders, paths, vision and target selection stay
//! per squad. Its soldiers have their own positions and hit points; every
//! hit strikes one soldier (see `sim/combat.rs`). Order within a tick (see
//! [`Sim::step`]): commands, path planning, [`Sim::update_stances`] (stance
//! and speed), aiming and firing, unit movement, [`Sim::update_squads`]
//! (soldiers follow their places, cover, entering buildings), projectiles.

use super::{Route, Sim, Unit, stop};
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::infantry::{InfantryRules, Stance, StanceOrder};
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::Shape;
use crate::path::PathGrid;
use serde::{Deserialize, Serialize};

/// State of a building as far as infantry cares (prompt 020 decides it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildingCondition {
    Intact,
    Damaged,
    /// Cannot be occupied any more; still cover.
    Ruin,
}

/// One soldier of a squad.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Soldier {
    pub(super) position: Vec2,
    /// Ground height at `position` in meters.
    pub(super) height: Fixed,
    pub(super) hp: u32,
    /// Cover spot (or window in a building) while the squad stands.
    spot: Option<Vec2>,
    /// Object whose soft cover the spot uses.
    pub(super) cover: Option<u32>,
    /// Ticks left knocked down by a blast (prompt 048): lying, getting up,
    /// picking up the weapon; no moving, no firing.
    pub(super) knocked: u32,
    /// Where the blast that knocked him down (last) went off.
    pub(super) knock_from: Vec2,
}

impl Soldier {
    pub(super) const fn alive(&self) -> bool {
        self.hp > 0
    }

    /// Whether the soldier stands at his cover spot.
    pub(super) fn at_spot(&self) -> bool {
        self.spot
            .is_some_and(|s| (s - self.position).length() < Fixed::ONE / 2)
    }
}

/// Infantry state of a unit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Squad {
    pub(super) soldiers: Vec<Soldier>,
    pub(super) stance: Stance,
    pub(super) order: StanceOrder,
    /// Ticks left of a stance change (no movement, no fire).
    pub(super) changing: u32,
    /// Ticks left under fire.
    pub(super) suppressed: u32,
    /// The building (object index) the squad is in.
    pub(super) garrison: Option<u32>,
    /// Where the squad entered its building; it leaves there.
    door: Vec2,
    /// Building to enter on arrival.
    enter: Option<u32>,
    /// Round robin over living soldiers for muzzles.
    pub(super) turn: u32,
    /// Squad position for which the cover spots were chosen.
    pub(super) spots_at: Option<Vec2>,
}

impl Squad {
    pub(super) fn alive_count(&self) -> u32 {
        self.soldiers.iter().filter(|s| s.alive()).count() as u32
    }

    /// The `k`-th living soldier (counted cyclically), if any lives.
    pub(super) fn living(&self, k: u32) -> Option<&Soldier> {
        let alive = self.alive_count();
        if alive == 0 {
            return None;
        }
        self.soldiers
            .iter()
            .filter(|s| s.alive())
            .nth((k % alive) as usize)
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.soldiers.len() as u64);
        for s in &self.soldiers {
            h.write_i64(s.position.x.to_bits());
            h.write_i64(s.position.y.to_bits());
            h.write_i64(s.height.to_bits());
            h.write_u32(s.hp);
            match s.spot {
                None => h.write_u8(0),
                Some(p) => {
                    h.write_u8(1);
                    h.write_i64(p.x.to_bits());
                    h.write_i64(p.y.to_bits());
                }
            }
            h.write_u32(s.cover.unwrap_or(u32::MAX));
            h.write_u32(s.knocked);
            h.write_i64(s.knock_from.x.to_bits());
            h.write_i64(s.knock_from.y.to_bits());
        }
        h.write_u8(self.stance.index() as u8);
        h.write_u8(match self.order {
            StanceOrder::Auto => 255,
            StanceOrder::Fixed(s) => s.index() as u8,
        });
        h.write_u32(self.changing);
        h.write_u32(self.suppressed);
        h.write_u32(self.garrison.unwrap_or(u32::MAX));
        h.write_i64(self.door.x.to_bits());
        h.write_i64(self.door.y.to_bits());
        h.write_u32(self.enter.unwrap_or(u32::MAX));
        h.write_u32(self.turn);
        match self.spots_at {
            None => h.write_u8(0),
            Some(p) => {
                h.write_u8(1);
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
            }
        }
    }
}

/// Infantry values of a unit type with the rules applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct TypeInfantry {
    /// Formation places relative to the squad: (forward, right) in meters.
    places: Vec<(Fixed, Fixed)>,
    /// Width of the formation plus one spacing (like a footprint).
    pub(super) spread: Fixed,
    change_ticks: u32,
    pub(super) suppressed_ticks: u32,
    /// Ticks a soldier stays down after a blast (prompt 048).
    pub(super) knock_ticks: u32,
}

/// Infantry rules with per-type and per-object values, built once.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct InfantryRt {
    pub(super) rules: InfantryRules,
    /// Per unit type; `None` for vehicles.
    pub(super) types: Vec<Option<TypeInfantry>>,
    /// Per object kind: soft cover in percent.
    cover_pct: Vec<u32>,
    /// Per object kind: whether a squad uses it as a cover spot.
    cover_spot: Vec<bool>,
    /// Per object kind: index into `rules.building.kinds`.
    building: Vec<Option<usize>>,
    /// Objects that give cover spots, bucketed.
    index: Buckets,
}

/// Ticks of `ms` real milliseconds at `tick_hz`, divided by the time factor
/// `(num, den)`.
fn ticks(ms: u32, tick_hz: u32, (num, den): (u32, u32)) -> u32 {
    let (num, den) = if num == 0 || den == 0 {
        (1, 1)
    } else {
        (u64::from(num), u64::from(den))
    };
    let t = u64::from(ms) * u64::from(tick_hz) * den / (1000 * num);
    u32::try_from(t).unwrap_or(u32::MAX)
}

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

/// Formation places of `n` soldiers: rows of `row_size` across the facing,
/// the leader in the middle of the front row, every second row shifted by
/// half a spacing.
fn places(n: usize, row_size: usize, spacing: Fixed) -> Vec<(Fixed, Fixed)> {
    let row_size = row_size.max(1);
    let rows = n.div_ceil(row_size);
    let half = spacing / 2;
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let (row, col) = (k / row_size, k % row_size);
        let in_row = row_size.min(n - row * row_size);
        // Lateral slots sorted from the middle outwards (left first).
        let mut order: Vec<usize> = (0..in_row).collect();
        order.sort_by_key(|&j| ((2 * j).abs_diff(in_row - 1), j));
        let j = order[col];
        let steps = i64::try_from(2 * j).unwrap_or(0) - i64::try_from(in_row).unwrap_or(0) + 1;
        let mut right = half * Fixed::from_num(steps);
        if row % 2 == 1 {
            right += half;
        }
        let forward_steps =
            i64::try_from(rows).unwrap_or(0) - 1 - 2 * i64::try_from(row).unwrap_or(0);
        out.push((half * Fixed::from_num(forward_steps), right));
    }
    out
}

impl InfantryRt {
    pub(super) fn new(
        rules: &InfantryRules,
        unit_types: &[crate::data::UnitType],
        map: &Map,
        tick_hz: u32,
    ) -> Self {
        let spacing = mm(rules.formation.spacing_mm);
        let types = unit_types
            .iter()
            .map(|t| {
                let c = &t.combat;
                c.is_infantry().then(|| {
                    let n = usize::from(c.squad_size);
                    let row = rules.formation.row_size as usize;
                    let width = Fixed::from_num(n.min(row).saturating_sub(1)) * spacing;
                    TypeInfantry {
                        places: places(n, row, spacing),
                        spread: (width + spacing).max(t.footprint_m),
                        change_ticks: ticks(rules.stance.change_ms, tick_hz, c.time_ratio),
                        suppressed_ticks: ticks(rules.stance.suppressed_ms, tick_hz, c.time_ratio),
                        knock_ticks: ticks(rules.knockdown.down_ms, tick_hz, c.time_ratio),
                    }
                })
            })
            .collect();
        let kinds = map.object_kinds();
        let cover_pct: Vec<u32> = kinds.iter().map(|k| rules.cover_pct(&k.id)).collect();
        let building: Vec<Option<usize>> = kinds
            .iter()
            .map(|k| rules.building.kinds.iter().position(|b| b.id == k.id))
            .collect();
        let cover_spot: Vec<bool> = kinds
            .iter()
            .zip(&cover_pct)
            .map(|(k, &pct)| {
                k.deck.is_none() && (pct > 0 || (k.blocks.large() && k.height > Fixed::ZERO))
            })
            .collect();
        let index = Buckets::new(map, |kind| cover_spot[usize::from(kind)]);
        Self {
            rules: rules.clone(),
            types,
            cover_pct,
            cover_spot,
            building,
            index,
        }
    }

    /// Soft cover of map object `object` in percent.
    pub(super) fn cover_pct(&self, map: &Map, object: u32) -> u32 {
        let kind = map.objects()[object as usize].kind;
        self.cover_pct[usize::from(kind)]
    }

    /// Building rules of map object `object`, if infantry can occupy it.
    pub(super) fn building(
        &self,
        map: &Map,
        object: u32,
    ) -> Option<&crate::infantry::BuildingKind> {
        let o = map.objects().get(object as usize)?;
        let b = self.building[usize::from(o.kind)]?;
        Some(&self.rules.building.kinds[b])
    }

    pub(super) fn stance_height(&self, stance: Stance) -> Fixed {
        mm(self.rules.stance.values(stance).height_mm)
    }

    pub(super) fn half_width(&self) -> Fixed {
        mm(self.rules.soldier.half_width_mm)
    }
}

/// Map objects bucketed by 16 m cells, for area queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Buckets {
    origin: Vec2,
    columns: u32,
    rows: u32,
    buckets: Vec<Vec<u32>>,
}

const BUCKET_M: i32 = 16;

impl Buckets {
    fn new(map: &Map, keep: impl Fn(u16) -> bool) -> Self {
        let size = map.size();
        let origin = Vec2::new(-size.x / 2, -size.y / 2);
        let bucket = Fixed::from_num(BUCKET_M);
        let columns = (size.x / bucket).ceil().to_num::<u32>() + 1;
        let rows = (size.y / bucket).ceil().to_num::<u32>() + 1;
        let mut index = Self {
            origin,
            columns,
            rows,
            buckets: vec![Vec::new(); (columns * rows) as usize],
        };
        let kinds = map.object_kinds();
        for (i, object) in (0u32..).zip(map.objects()) {
            if !keep(object.kind) {
                continue;
            }
            let extent = object.half_extent(kinds[usize::from(object.kind)].shape);
            let (c0, r0) = index.cell(object.position - extent);
            let (c1, r1) = index.cell(object.position + extent);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    index.buckets[(r * columns + c) as usize].push(i);
                }
            }
        }
        index
    }

    fn cell(&self, p: Vec2) -> (u32, u32) {
        let bucket = Fixed::from_num(BUCKET_M);
        let axis = |v: Fixed, o: Fixed, n: u32| {
            ((v - o) / bucket)
                .floor()
                .max(Fixed::ZERO)
                .to_num::<u32>()
                .min(n - 1)
        };
        (
            axis(p.x, self.origin.x, self.columns),
            axis(p.y, self.origin.y, self.rows),
        )
    }

    /// Objects whose box overlaps the square `center ± half`, ascending.
    fn near(&self, center: Vec2, half: Fixed) -> Vec<u32> {
        let d = Vec2::new(half, half);
        let (c0, r0) = self.cell(center - d);
        let (c1, r1) = self.cell(center + d);
        let mut out = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                out.extend(&self.buckets[(r * self.columns + c) as usize]);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// Right-hand side of a facing (as in the hit boxes).
fn right_of(f: Vec2) -> Vec2 {
    Vec2::new(-f.y, f.x)
}

/// Distance from `p` to the ground shape of an object (0 inside).
fn distance_to_shape(p: Vec2, object: &crate::objects::MapObject, shape: Shape) -> Fixed {
    let rel = p - object.position;
    match shape {
        Shape::Circle { radius } => (rel.length() - radius).max(Fixed::ZERO),
        Shape::Box {
            half_length,
            half_width,
        } => {
            let (forward, right) = object.axes();
            let a = (dot(rel, forward).abs() - half_length).max(Fixed::ZERO);
            let b = (dot(rel, right).abs() - half_width).max(Fixed::ZERO);
            Vec2::new(a, b).length()
        }
    }
}

/// Cover spots behind an object as seen along `facing`: one behind a round
/// object, a line along a box's long side every `spacing`.
fn spots_behind(
    object: &crate::objects::MapObject,
    shape: Shape,
    facing: Vec2,
    gap: Fixed,
    spacing: Fixed,
    out: &mut Vec<Vec2>,
) {
    match shape {
        Shape::Circle { radius } => out.push(object.position - facing.scale(radius + gap)),
        Shape::Box {
            half_length,
            half_width,
        } => {
            let (forward, right) = object.axes();
            // The axis that runs across the facing carries the line.
            let (along, half_along, depth, half_depth) =
                if dot(forward, facing).abs() <= dot(right, facing).abs() {
                    (forward, half_length, right, half_width)
                } else {
                    (right, half_width, forward, half_length)
                };
            let side = if dot(depth, facing) >= Fixed::ZERO {
                -Fixed::ONE
            } else {
                Fixed::ONE
            };
            let back = object.position + depth.scale(side * (half_depth + gap));
            let steps = (half_along * 2 / spacing).to_num::<i64>().clamp(0, 16);
            for k in 0..=steps {
                let t = if steps == 0 {
                    Fixed::ZERO
                } else {
                    -half_along + half_along * 2 * Fixed::from_num(k) / Fixed::from_num(steps)
                };
                out.push(back + along.scale(t));
            }
        }
    }
}

impl Sim {
    /// Infantry state of a new unit of type `t` at `position`, or `None`
    /// for vehicles.
    pub(super) fn new_squad(
        rt: &InfantryRt,
        t: usize,
        position: Vec2,
        facing: Vec2,
        soldier_hp: u32,
        map: &Map,
    ) -> Option<Box<Squad>> {
        let ty = rt.types[t].as_ref()?;
        let right = right_of(facing);
        let soldiers = ty
            .places
            .iter()
            .map(|&(f, r)| {
                let mut p = position + facing.scale(f) + right.scale(r);
                if !map.contains(p) {
                    p = position;
                }
                Soldier {
                    position: p,
                    height: map.height_at(p),
                    hp: soldier_hp,
                    spot: None,
                    cover: None,
                    knocked: 0,
                    knock_from: Vec2::ZERO,
                }
            })
            .collect();
        Some(Box::new(Squad {
            soldiers,
            stance: Stance::Stand,
            order: StanceOrder::Auto,
            changing: 0,
            suppressed: 0,
            garrison: None,
            door: position,
            enter: None,
            turn: 0,
            spots_at: None,
        }))
    }

    /// Condition of building `object`: the damage stage of prompt 020
    /// (a destroyed building is a ruin; a placed `Ruin` variant starts as
    /// one).
    pub fn building_condition(&self, object: u32) -> BuildingCondition {
        match self.object_stage(object) {
            Some(crate::destruction::ObjectStage::Destroyed) => BuildingCondition::Ruin,
            Some(crate::destruction::ObjectStage::Damaged) => BuildingCondition::Damaged,
            _ => BuildingCondition::Intact,
        }
    }

    /// Whether map object `object` still gives cover: not destroyed, or a
    /// ruin (felled trees, flattened fences and burnt bushes give none).
    pub(super) fn gives_cover(&self, object: u32) -> bool {
        self.object_stage(object) != Some(crate::destruction::ObjectStage::Destroyed)
            || self.object_is_ruin(object)
    }

    /// Soldiers of `player` in building `object`.
    pub fn building_occupants(&self, object: u32) -> (Option<PlayerId>, u32) {
        let mut owner = None;
        let mut count = 0;
        for u in &self.units {
            if let Some(s) = &u.squad
                && u.alive()
                && s.garrison == Some(object)
            {
                owner = Some(u.owner);
                count += s.alive_count();
            }
        }
        (owner, count)
    }

    /// Soldier capacity of building `object`, if infantry can occupy it.
    pub fn building_capacity(&self, object: u32) -> Option<u32> {
        self.infantry
            .building(&self.map, object)
            .map(|b| b.capacity)
    }

    /// The building of squad `i` became a ruin: every occupant dies with
    /// `collapse_loss_pct`; the survivors stay in the rubble at their
    /// window spots (a squad in a ruin, no longer garrisoned).
    fn collapse(&mut self, i: usize) {
        let loss = self.infantry.rules.building.collapse_loss_pct;
        let unit = &mut self.units[i];
        let squad = unit.squad.as_mut().expect("checked");
        for soldier in squad.soldiers.iter_mut().filter(|s| s.alive()) {
            if self.rng.below(100) < loss {
                soldier.hp = 0;
            }
        }
        unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
        squad.garrison = None;
        squad.spots_at = Some(unit.position);
        unit.elevation_mm = 0;
        unit.vision_dirty = true;
        if unit.hp == 0 {
            let (id, at) = (unit.id, unit.position);
            let z = unit.height;
            self.destroy(i, id, crate::combat::Vec3::new(at.x, at.y, z));
        }
    }

    /// Soldiers standing inside the ground shape of map object `object`
    /// (a ruin), as (unit index, soldier index).
    pub(super) fn soldiers_in(&self, object: u32) -> Vec<(usize, usize)> {
        let o = self.map.objects()[object as usize];
        let shape = self.map.object_kinds()[usize::from(o.kind)].shape;
        let reach = o.half_extent(shape);
        let mut out = Vec::new();
        for (j, u) in self.units.iter().enumerate() {
            let Some(s) = &u.squad else { continue };
            if !u.alive() || s.garrison.is_some() {
                continue;
            }
            let d = u.position - o.position;
            if d.x.abs() > reach.x + u.spread || d.y.abs() > reach.y + u.spread {
                continue;
            }
            for (k, x) in s.soldiers.iter().enumerate() {
                if x.alive() && distance_to_shape(x.position, &o, shape) == Fixed::ZERO {
                    out.push((j, k));
                }
            }
        }
        out
    }

    /// Drops a pending order to enter a building (`Stop`).
    pub(super) fn cancel_enter(&mut self, index: usize) {
        if let Some(s) = &mut self.units[index].squad {
            s.enter = None;
        }
    }

    /// `SetStance` for one unit.
    pub(super) fn set_stance(&mut self, index: usize, order: StanceOrder) {
        if let Some(s) = &mut self.units[index].squad {
            s.order = order;
        }
    }

    /// `Garrison` for the listed (owned, living) units.
    pub(super) fn order_garrison(&mut self, indices: &[usize], building: u32) {
        let Some(object) = self.map.objects().get(building as usize).copied() else {
            return;
        };
        if self.infantry.building(&self.map, building).is_none() {
            return;
        }
        // A ruin cannot be occupied: the squads move into the rubble and
        // take cover there.
        let ruin = self.building_condition(building) == BuildingCondition::Ruin;
        for &i in indices {
            if self.units[i].squad.is_none() {
                continue;
            }
            if self.units[i].squad.as_ref().and_then(|s| s.garrison) == Some(building) {
                continue;
            }
            self.leave_building(i);
            let unit = &mut self.units[i];
            unit.combat.attack = None;
            if let Some(s) = &mut unit.squad {
                s.enter = (!ruin).then_some(building);
            }
            let id = self.paths.request(
                unit.position,
                object.position,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(object.position);
            unit.route = Route::Waiting(id);
            unit.replans = 0;
        }
    }

    /// Takes unit `i` out of its building (to where it entered), and drops
    /// a pending order to enter one.
    pub(super) fn leave_building(&mut self, i: usize) {
        let map = &self.map;
        let unit = &mut self.units[i];
        let Some(s) = &mut unit.squad else {
            return;
        };
        s.enter = None;
        if s.garrison.take().is_none() {
            return;
        }
        let door = s.door;
        for soldier in &mut s.soldiers {
            soldier.position = door;
            soldier.height = map.height_at(door);
            soldier.spot = None;
            soldier.cover = None;
        }
        s.spots_at = None;
        unit.position = door;
        unit.height = map.height_at(door);
        unit.elevation_mm = 0;
        unit.vision_dirty = true;
    }

    /// Puts every soldier of squad `i` at `at`, out of cover (mounting and
    /// dismounting, prompt 034).
    pub(super) fn set_squad_at(&mut self, i: usize, at: Vec2) {
        let height = self.map.height_at(at);
        let Some(s) = &mut self.units[i].squad else {
            return;
        };
        for soldier in &mut s.soldiers {
            soldier.position = at;
            soldier.height = height;
            soldier.spot = None;
            soldier.cover = None;
        }
        s.spots_at = None;
    }

    /// Stances and speed factors of every squad, before firing and moving.
    pub(super) fn update_stances(&mut self) {
        for unit in &mut self.units {
            if !unit.alive() {
                continue;
            }
            let moving = matches!(unit.route, Route::Following { .. });
            let Some(s) = &mut unit.squad else {
                continue;
            };
            let ty = self.infantry.types[unit.unit_type as usize]
                .as_ref()
                .expect("squads have infantry types");
            s.suppressed = s.suppressed.saturating_sub(1);
            s.changing = s.changing.saturating_sub(1);
            for soldier in &mut s.soldiers {
                soldier.knocked = soldier.knocked.saturating_sub(1);
            }
            let wanted = if s.garrison.is_some() {
                Stance::Stand
            } else {
                match s.order {
                    StanceOrder::Fixed(stance) => stance,
                    StanceOrder::Auto if moving => Stance::Stand,
                    StanceOrder::Auto if s.suppressed > 0 => Stance::Prone,
                    StanceOrder::Auto => Stance::Kneel,
                }
            };
            if wanted != s.stance && s.changing == 0 {
                s.stance = wanted;
                s.changing = ty.change_ticks;
            }
            unit.speed_pct = if s.changing > 0 {
                0
            } else {
                self.infantry.rules.stance.values(s.stance).speed_pct
            };
        }
    }

    /// After movement: soldiers follow their places (or cover spots),
    /// squads enter buildings.
    pub(super) fn update_squads(&mut self) {
        for i in 0..self.units.len() {
            if self.units[i].squad.is_none()
                || !self.units[i].alive()
                || super::transport::aboard(&self.units[i])
            {
                continue;
            }
            // A building that collapses around its occupants (prompt 020).
            if let Some(b) = self.units[i].squad.as_ref().and_then(|s| s.garrison)
                && self.building_condition(b) == BuildingCondition::Ruin
            {
                self.collapse(i);
                if !self.units[i].alive() {
                    continue;
                }
            }
            self.try_enter(i);
            let standing = self.units[i].route == Route::None;
            let garrisoned = self.units[i]
                .squad
                .as_ref()
                .is_some_and(|s| s.garrison.is_some());
            if standing && !garrisoned {
                let at = self.units[i].position;
                if self.units[i]
                    .squad
                    .as_ref()
                    .is_some_and(|s| s.spots_at != Some(at))
                {
                    self.choose_spots(i);
                }
            }
            self.follow_places(i, standing);
        }
    }

    /// Enters the pending building of unit `i` once it is close enough, if
    /// there is room and no enemy inside; gives up when it stopped short.
    fn try_enter(&mut self, i: usize) {
        let Some(building) = self.units[i].squad.as_ref().and_then(|s| s.enter) else {
            return;
        };
        let object = self.map.objects()[building as usize];
        let kind = &self.map.object_kinds()[usize::from(object.kind)];
        let near = distance_to_shape(self.units[i].position, &object, kind.shape)
            <= mm(self.infantry.rules.building.enter_mm);
        if !near {
            if self.units[i].route == Route::None
                && let Some(s) = &mut self.units[i].squad
            {
                s.enter = None;
            }
            return;
        }
        let (owner, inside) = self.building_occupants(building);
        let capacity = self.building_capacity(building).unwrap_or(0);
        let unit = &self.units[i];
        let squad = unit.squad.as_ref().expect("checked");
        let fits = owner.is_none_or(|o| o == unit.owner)
            && inside + squad.alive_count() <= capacity
            && self.building_condition(building) != BuildingCondition::Ruin;
        let (shape, height) = (kind.shape, kind.height);
        let map = &self.map;
        let unit = &mut self.units[i];
        if fits {
            stop(unit);
        }
        let position = unit.position;
        let squad = unit.squad.as_mut().expect("checked");
        squad.enter = None;
        if !fits {
            return;
        }
        squad.door = position;
        squad.garrison = Some(building);
        squad.spots_at = None;
        // Windows: points around the walls, 1 m inside, by soldier index.
        let windows = windows(&object, shape, squad.soldiers.len());
        for (soldier, w) in squad.soldiers.iter_mut().zip(windows) {
            soldier.position = w;
            soldier.height = map.height_at(w);
            soldier.spot = Some(w);
            soldier.cover = None;
        }
        unit.position = object.position;
        unit.height = map.height_at(object.position);
        // Observers look out of the upper windows, just over the roof.
        let eye = super::vision::eye_mm(height) + 500;
        unit.elevation_mm = eye;
        unit.vision_dirty = true;
    }

    /// Picks cover spots near every soldier's formation place.
    fn choose_spots(&mut self, i: usize) {
        if self.spots_in_ruin(i) {
            return;
        }
        let rules = &self.infantry.rules.cover;
        let (gap, spacing, search) = (
            mm(rules.gap_mm),
            mm(rules.spot_spacing_mm),
            mm(rules.search_mm),
        );
        let unit = &self.units[i];
        let ty = self.infantry.types[unit.unit_type as usize]
            .as_ref()
            .expect("squads have infantry types");
        let facing = unit.facing;
        let right = right_of(facing);
        let center = unit.position;
        let profile = unit.unit_type as usize;
        let kinds = self.map.object_kinds();
        let objects = self.map.objects();
        let mut spots: Vec<(Vec2, Option<u32>)> = Vec::new();
        let mut buffer = Vec::new();
        for o in self.infantry.index.near(center, ty.spread / 2 + search) {
            let object = &objects[o as usize];
            let kind_index = usize::from(object.kind);
            if !self.infantry.cover_spot[kind_index] || !self.gives_cover(o) {
                continue;
            }
            buffer.clear();
            spots_behind(
                object,
                kinds[kind_index].shape,
                facing,
                gap,
                spacing,
                &mut buffer,
            );
            spots.extend(buffer.iter().map(|&p| (p, Some(o))));
        }
        // Craters (prompt 021): spots without an object, cover all round.
        let crater_spots = self.crater_spots(center, ty.spread / 2 + search);
        spots.extend(crater_spots.into_iter().map(|p| (p, None)));
        // Firing positions of trenches and MG nests (prompt 052).
        let trench_spots = self.trench_spots(center, ty.spread / 2 + search, facing);
        spots.extend(trench_spots.into_iter().map(|p| (p, None)));
        let grid = &self.paths.grid;
        let map = &self.map;
        spots.retain(|&(p, _)| map.contains(p) && grid.passable_for(profile, p));
        let mut used = vec![false; spots.len()];
        let unit = &mut self.units[i];
        let squad = unit.squad.as_mut().expect("checked");
        for (soldier, &(f, r)) in squad.soldiers.iter_mut().zip(&ty.places) {
            let place = center + facing.scale(f) + right.scale(r);
            let best = spots
                .iter()
                .enumerate()
                .filter(|&(k, _)| !used[k])
                .map(|(k, &(p, o))| ((p - place).length(), k, p, o))
                .filter(|&(d, ..)| d <= search)
                .min_by_key(|&(d, k, ..)| (d, k));
            match best {
                Some((_, k, p, o)) => {
                    used[k] = true;
                    soldier.spot = Some(p);
                    soldier.cover = o;
                }
                None => {
                    soldier.spot = None;
                    soldier.cover = None;
                }
            }
        }
        squad.spots_at = Some(center);
    }

    /// A squad standing in a ruin (prompt 020) spreads its soldiers over
    /// the rubble, 1 m inside the walls like occupants at windows. Returns
    /// whether it did.
    fn spots_in_ruin(&mut self, i: usize) -> bool {
        let center = self.units[i].position;
        let ruin = self
            .infantry
            .index
            .near(center, Fixed::ONE)
            .into_iter()
            .find(|&o| {
                let object = self.map.objects()[o as usize];
                let shape = self.map.object_kinds()[usize::from(object.kind)].shape;
                self.object_is_ruin(o) && distance_to_shape(center, &object, shape) == Fixed::ZERO
            });
        let Some(ruin) = ruin else {
            return false;
        };
        let object = self.map.objects()[ruin as usize];
        let shape = self.map.object_kinds()[usize::from(object.kind)].shape;
        let unit = &mut self.units[i];
        let squad = unit.squad.as_mut().expect("checked");
        let spots = windows(&object, shape, squad.soldiers.len());
        for (soldier, spot) in squad.soldiers.iter_mut().zip(spots) {
            soldier.spot = Some(spot);
            soldier.cover = Some(ruin);
        }
        squad.spots_at = Some(center);
        true
    }

    /// Moves the soldiers of unit `i` towards their places.
    fn follow_places(&mut self, i: usize, standing: bool) {
        let unit = &self.units[i];
        let ty = self.infantry.types[unit.unit_type as usize]
            .as_ref()
            .expect("squads have infantry types");
        let rules = &self.infantry.rules;
        let speed = unit.speed_road
            * Fixed::from_num(rules.formation.catch_up_pct)
            * Fixed::from_num(
                rules
                    .stance
                    .values(unit.squad.as_ref().expect("checked").stance)
                    .speed_pct,
            )
            / 10_000;
        // Soldiers settle even while the squad changes its stance.
        let speed = speed.max(unit.speed_offroad / 4);
        let (facing, center, profile) = (unit.facing, unit.position, unit.unit_type as usize);
        let right = right_of(facing);
        let grid: &PathGrid = &self.paths.grid;
        let map = &self.map;
        let terrain = &self.terrain;
        let ok = |p: Vec2| {
            map.contains(p)
                && terrain.types[usize::from(map.terrain_at(p))].passable
                && grid.passable_for(profile, p)
        };
        let squad = self.units[i].squad.as_mut().expect("checked");
        if squad.garrison.is_some() {
            return;
        }
        for (soldier, &(f, r)) in squad.soldiers.iter_mut().zip(&ty.places) {
            // Knocked-down soldiers lie where the blast threw them.
            if !soldier.alive() || soldier.knocked > 0 {
                continue;
            }
            let place = center + facing.scale(f) + right.scale(r);
            let wanted = match soldier.spot {
                Some(spot) if standing => spot,
                _ => place,
            };
            let wanted = if ok(wanted) { wanted } else { center };
            let delta = wanted - soldier.position;
            let distance = delta.length();
            if distance == Fixed::ZERO {
                continue;
            }
            let next = if distance <= speed {
                wanted
            } else {
                soldier.position + Vec2::new(delta.x / distance, delta.y / distance).scale(speed)
            };
            if ok(next) || !ok(soldier.position) {
                soldier.position = next;
                soldier.height = map.height_at(next);
            }
        }
    }

    /// Marks squads near `point` as under fire (rounds of other players).
    pub(super) fn near_miss(&mut self, point: Vec2, shooter: UnitId) {
        let owner = match self.units.binary_search_by_key(&shooter, |u| u.id) {
            Ok(k) => self.units[k].owner,
            Err(_) => return,
        };
        let radius = mm(self.infantry.rules.stance.near_miss_mm);
        for unit in &mut self.units {
            if self.teams.allied(unit.owner, owner) || !unit.alive() {
                continue;
            }
            let Some(s) = &mut unit.squad else {
                continue;
            };
            let ty = self.infantry.types[unit.unit_type as usize]
                .as_ref()
                .expect("squads have infantry types");
            let d = point - unit.position;
            let reach = ty.spread + radius;
            if d.x.abs() > reach || d.y.abs() > reach {
                continue;
            }
            if s.soldiers
                .iter()
                .any(|x| x.alive() && (x.position - point).length() <= radius)
            {
                s.suppressed = ty.suppressed_ticks;
            }
        }
    }
}

/// Window spots of a building for `n` soldiers: points 1 m inside its
/// walls, spread around the perimeter.
fn windows(object: &crate::objects::MapObject, shape: Shape, n: usize) -> Vec<Vec2> {
    let inset = Fixed::ONE;
    let (forward, right) = object.axes();
    let (hl, hw) = match shape {
        Shape::Circle { radius } => (radius, radius),
        Shape::Box {
            half_length,
            half_width,
        } => (half_length, half_width),
    };
    let (a, b) = ((hl - inset).max(Fixed::ZERO), (hw - inset).max(Fixed::ZERO));
    // Walk the rectangle's perimeter in n equal steps.
    let perimeter = (a + b) * 4;
    let count = Fixed::from_num(n.max(1));
    (0..n)
        .map(|k| {
            let mut t = perimeter * Fixed::from_num(k) / count + a;
            let (mut u, mut v);
            // Sides: front (+forward), right, back, left.
            if t <= b * 2 {
                (u, v) = (a, t - b);
            } else {
                t -= b * 2;
                if t <= a * 2 {
                    (u, v) = (a - t, b);
                } else {
                    t -= a * 2;
                    if t <= b * 2 {
                        (u, v) = (-a, b - t);
                    } else {
                        t -= b * 2;
                        (u, v) = (-a + t.min(a * 2), -b);
                    }
                }
            }
            if a == Fixed::ZERO {
                u = Fixed::ZERO;
            }
            if b == Fixed::ZERO {
                v = Fixed::ZERO;
            }
            object.position + forward.scale(u) + right.scale(v)
        })
        .collect()
}

/// Whether a unit stands inside a building.
pub(super) fn inside(unit: &Unit) -> bool {
    unit.squad.as_ref().is_some_and(|s| s.garrison.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_put_the_leader_in_front_and_center() {
        let p = places(9, 5, Fixed::from_num(3));
        assert_eq!(p.len(), 9);
        assert_eq!(p[0], (Fixed::from_num(3) / 2, Fixed::ZERO));
        // Back row shifted by half a spacing.
        assert_eq!(p[5].0, -Fixed::from_num(3) / 2);
        let mut lateral: Vec<Fixed> = p[..5].iter().map(|x| x.1).collect();
        lateral.sort();
        assert_eq!(lateral[0], Fixed::from_num(-6));
        assert_eq!(lateral[4], Fixed::from_num(6));
        // One soldier stands on the squad position.
        assert_eq!(places(1, 5, Fixed::ONE), vec![(Fixed::ZERO, Fixed::ZERO)]);
    }

    #[test]
    fn ticks_apply_the_time_factor() {
        assert_eq!(ticks(1000, 20, (2, 1)), 10);
        assert_eq!(ticks(5000, 20, (0, 0)), 100);
    }
}
