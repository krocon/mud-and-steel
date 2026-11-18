//! The combat update: target selection, aiming, firing, projectile flight
//! and hits (prompt 017). Rules and data types: `crate::combat`.
//!
//! Order within a tick (see [`Sim::step`]): commands, path planning,
//! [`Sim::aim_and_fire`] (targets, attack movement, turrets, new
//! projectiles), unit movement, [`Sim::fly_projectiles`] (flight, hits,
//! destruction).

use super::infantry::{InfantryRt, inside};
use super::{Route, Sim, Unit, stop};
use crate::combat::{
    CombatEvent, CombatRules, Face, FireMode, HitKind, ProjectileSnapshot, Vec3, WeaponType,
    deg_to_rad, sin_cos_rad,
};
use crate::command::UnitId;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::infantry::Stance;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::{Shape, sin_cos_deg};
use serde::{Deserialize, Serialize};

/// An attack order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Attack {
    Unit(UnitId),
    Ground(Vec2),
}

/// Combat state of one unit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CombatState {
    /// Direction of the turret / main weapon (unit length).
    pub(super) aim: Vec2,
    /// Ticks until each weapon can fire again.
    reload: Vec<u32>,
    pub(super) fire_mode: FireMode,
    pub(super) attack: Option<Attack>,
    /// The unit drives towards its attack target.
    pub(super) attack_moving: bool,
    /// Automatically chosen target.
    pub(super) auto_target: Option<UnitId>,
    /// Where the player last saw the target of an attack order (prompt 018).
    last_seen: Option<Vec2>,
}

impl CombatState {
    pub(super) fn new(facing: Vec2, weapons: usize, fire_mode: FireMode) -> Self {
        Self {
            aim: facing,
            reload: vec![0; weapons],
            fire_mode,
            attack: None,
            attack_moving: false,
            auto_target: None,
            last_seen: None,
        }
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_i64(self.aim.x.to_bits());
        h.write_i64(self.aim.y.to_bits());
        h.write_u64(self.reload.len() as u64);
        for &r in &self.reload {
            h.write_u32(r);
        }
        h.write_u8(u8::from(self.fire_mode == FireMode::HoldFire));
        match self.attack {
            None => h.write_u8(0),
            Some(Attack::Unit(id)) => {
                h.write_u8(1);
                h.write_u32(id.0);
            }
            Some(Attack::Ground(p)) => {
                h.write_u8(2);
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
            }
        }
        h.write_u8(u8::from(self.attack_moving));
        h.write_u32(self.auto_target.map_or(u32::MAX, |t| t.0));
        match self.last_seen {
            None => h.write_u8(0),
            Some(p) => {
                h.write_u8(1);
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
            }
        }
    }
}

/// A weapon with the rules applied, in game units per tick.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct WeaponRt {
    weapon: WeaponType,
    reload_ticks: u32,
    /// Meters per tick.
    speed: Fixed,
    range: Fixed,
    vehicle_damage: u32,
    infantry_damage: u32,
}

/// Combat values of a unit type with the rules applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct TypeCombat {
    pub(super) max_hp: u32,
    infantry: bool,
    half_length: Fixed,
    half_width: Fixed,
    pub(super) height: Fixed,
    armor: crate::combat::Armor,
    /// `(sin, cos)` of the turret turn per tick; `None` = instant.
    turret_step: Option<(Fixed, Fixed)>,
    /// `(sin, cos)` of the hull turn per tick (casemates).
    pub(super) hull_step: Option<(Fixed, Fixed)>,
    /// Cosine of half the casemate traverse; `None` = turret.
    arc_cos: Option<Fixed>,
    weapons: Vec<WeaponRt>,
    pub(super) max_range: Fixed,
    /// Gravity in meters per tick².
    gravity: Fixed,
    /// Real meters per game meter (direct-fire range compression).
    real_per_game: Fixed,
    /// Game angle per real angle (the inverse of the range compression).
    angle_scale: Fixed,
}

fn ratio((num, den): (u32, u32)) -> Fixed {
    if num == 0 || den == 0 {
        Fixed::ONE
    } else {
        Fixed::from_num(num) / Fixed::from_num(den)
    }
}

impl TypeCombat {
    pub(super) fn new(unit_type: &UnitType, rules: &CombatRules, tick_hz: u32) -> Self {
        let c = &unit_type.combat;
        let hz = Fixed::from_num(tick_hz.max(1));
        let time = ratio(c.time_ratio);
        let range_factor = ratio(c.range_ratio);
        let step = |deg_per_s: u32| {
            let deg = Fixed::from_num(deg_per_s) * time / hz;
            Some(sin_cos_rad(deg_to_rad(deg.min(Fixed::from_num(57)))))
        };
        let infantry = c.is_infantry();
        let footprint = unit_type.footprint_m;
        let dim = |v: Fixed| if v > Fixed::ZERO { v } else { footprint };
        let weapons: Vec<WeaponRt> = c
            .weapons
            .iter()
            .map(|w| WeaponRt {
                reload_ticks: rules.reload_ticks(w, tick_hz),
                speed: w.speed_m_per_s / hz,
                range: w.range_m,
                vehicle_damage: rules.vehicle_damage(w),
                infantry_damage: rules.infantry_damage(w),
                weapon: w.clone(),
            })
            .collect();
        let g =
            Fixed::from_num(rules.ballistics.gravity_mm_per_s2) / 1000 * time * time / (hz * hz);
        Self {
            max_hp: rules.max_hp(c),
            infantry,
            half_length: dim(c.length_m) / 2,
            half_width: dim(c.width_m) / 2,
            height: if c.height_m > Fixed::ZERO {
                c.height_m
            } else {
                Fixed::from_num(rules.damage.infantry_height_mm) / 1000
            },
            armor: c.armor,
            turret_step: if infantry {
                None
            } else {
                step(rules.turret_deg_per_s(c.category))
            },
            hull_step: if infantry {
                None
            } else {
                step(rules.aim.hull_deg_per_s)
            },
            arc_cos: c
                .gun_traverse_deg
                .filter(|_| !infantry)
                .map(|deg| sin_cos_rad(deg_to_rad(Fixed::from_num(deg) / 2)).1),
            max_range: weapons.iter().map(|w| w.range).max().unwrap_or(Fixed::ZERO),
            weapons,
            gravity: g,
            real_per_game: Fixed::ONE / range_factor,
            angle_scale: Fixed::ONE / range_factor,
        }
    }

    /// Whether `weapon` can hurt a unit of type `target` at `distance` game
    /// meters (against its thinnest plate).
    fn can_damage(
        &self,
        weapon: usize,
        rules: &CombatRules,
        target: &Self,
        distance: Fixed,
    ) -> bool {
        let w = &self.weapons[weapon];
        if distance > w.range {
            return false;
        }
        let thinnest = target.armor.thinnest();
        target.infantry
            || thinnest == 0
            || rules.penetration_at(&w.weapon, distance * self.real_per_game)
                >= Fixed::from_num(thinnest)
    }
}

/// A projectile in flight.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Projectile {
    id: u32,
    shooter: UnitId,
    shooter_type: u32,
    weapon: u8,
    position: Vec3,
    /// Meters per tick.
    velocity: Vec3,
    /// Horizontal meters left before it vanishes.
    remaining: Fixed,
    /// Horizontal game meters flown.
    traveled: Fixed,
    caliber_mm_x10: u32,
    /// The building the shooter fires out of (prompt 022): not hit.
    skip_object: Option<u32>,
}

impl Projectile {
    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.id);
        h.write_u32(self.shooter.0);
        h.write_u32(self.shooter_type);
        h.write_u8(self.weapon);
        for v in [
            self.position.x,
            self.position.y,
            self.position.z,
            self.velocity.x,
            self.velocity.y,
            self.velocity.z,
            self.remaining,
            self.traveled,
        ] {
            h.write_i64(v.to_bits());
        }
        if let Some(o) = self.skip_object {
            h.write_u32(o);
        }
    }

    pub(super) fn snapshot(&self, tick_hz: u32) -> ProjectileSnapshot {
        let hz = Fixed::from_num(tick_hz);
        ProjectileSnapshot {
            id: self.id,
            shooter: self.shooter,
            position: self.position,
            velocity: Vec3::new(
                self.velocity.x * hz,
                self.velocity.y * hz,
                self.velocity.z * hz,
            ),
            caliber_mm_x10: self.caliber_mm_x10,
        }
    }
}

/// Map objects that stop projectiles (trees, buildings, walls, hedges),
/// bucketed for segment queries.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ObjectIndex {
    origin: Vec2,
    columns: u32,
    rows: u32,
    buckets: Vec<Vec<u32>>,
}

const BUCKET_M: i32 = 16;

impl ObjectIndex {
    pub(super) fn new(map: &Map) -> Self {
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
            let kind = &kinds[usize::from(object.kind)];
            if !kind.blocks.large() || kind.height <= Fixed::ZERO {
                continue;
            }
            let extent = object.half_extent(kind.shape);
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

    /// Earliest object on the segment `a → b`: `(t, object index)`.
    /// `height_of` gives the height up to which an object stops projectiles
    /// now (`None`: it does not, prompt 020); objects for which
    /// `skip_inside` holds are ignored when the segment starts inside them
    /// (a round fired from within a ruin).
    pub(super) fn hit(
        &self,
        map: &Map,
        a: Vec3,
        b: Vec3,
        height_of: impl Fn(u32) -> Option<Fixed>,
        skip_inside: impl Fn(u32) -> bool,
    ) -> Option<(Fixed, u32)> {
        let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let high = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        let (c0, r0) = self.cell(low);
        let (c1, r1) = self.cell(high);
        let mut candidates: Vec<u32> = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                candidates.extend(&self.buckets[(r * self.columns + c) as usize]);
            }
        }
        candidates.sort_unstable();
        candidates.dedup();
        let kinds = map.object_kinds();
        let objects = map.objects();
        let d = sub(b, a);
        let mut best: Option<(Fixed, u32)> = None;
        for i in candidates {
            let object = &objects[i as usize];
            let kind = &kinds[usize::from(object.kind)];
            let Some(height) = height_of(i) else {
                continue;
            };
            let t = match kind.shape {
                Shape::Circle { radius } => {
                    segment_circle(a.ground(), d.ground(), object.position, radius)
                }
                Shape::Box {
                    half_length,
                    half_width,
                } => {
                    let (forward, right) = object.axes();
                    let rel = a.ground() - object.position;
                    slab_2d(
                        [dot(rel, forward), dot(rel, right)],
                        [dot(d.ground(), forward), dot(d.ground(), right)],
                        [half_length, half_width],
                    )
                }
            };
            let Some(t) = t else { continue };
            if best.is_some_and(|(bt, _)| bt <= t) || (t == Fixed::ZERO && skip_inside(i)) {
                continue;
            }
            let p = at(a, d, t);
            if p.z - map.height_at(p.ground()) < height {
                best = Some((t, i));
            }
        }
        best
    }
}

/// Earliest hit of a projectile so far: `(t, what, unit index, face and
/// soldier)`.
type Candidate = (Fixed, HitKind, Option<(usize, Face, Option<usize>)>);

/// What a unit fires at this tick.
#[derive(Clone, Copy)]
enum Goal {
    Unit(usize),
    Ground(Vec2),
    /// The attack target is out of sight: drive to where it was last seen.
    Search(Vec2),
}

impl Sim {
    pub(super) fn start_attack(&mut self, index: usize, attack: Attack) {
        let unit = &mut self.units[index];
        if self.types[unit.unit_type as usize].weapons.is_empty() {
            return;
        }
        if unit.combat.attack_moving || unit.combat.attack.is_some() {
            stop(unit);
        }
        unit.combat.attack = Some(attack);
        unit.combat.attack_moving = false;
        unit.combat.last_seen = None;
    }

    /// Whether some weapon of unit `i` can hurt unit `j` from half its
    /// range (prompt 046: who a securing unit goes after).
    pub(super) fn could_hurt(&self, i: usize, j: usize) -> bool {
        let mine = &self.types[self.units[i].unit_type as usize];
        let theirs = &self.types[self.units[j].unit_type as usize];
        (0..mine.weapons.len())
            .any(|w| mine.can_damage(w, &self.combat_rules, theirs, mine.weapons[w].range / 2))
    }

    /// Target selection, attack movement, aiming and firing of every unit.
    pub(super) fn aim_and_fire(&mut self) {
        let retarget = self
            .tick
            .is_multiple_of(u64::from(self.combat_rules.fire.retarget_ticks));
        for i in 0..self.units.len() {
            if !self.units[i].alive() || self.transport_blocks_fire(i) {
                continue;
            }
            let ty = self.units[i].unit_type as usize;
            if self.types[ty].weapons.is_empty() {
                continue;
            }
            for r in &mut self.units[i].combat.reload {
                *r = r.saturating_sub(1);
            }
            let goal = self.goal(i, retarget);
            let Some(goal) = goal else {
                // Back to the front.
                let unit = &mut self.units[i];
                let facing = unit.facing;
                unit.combat.aim =
                    rotate_towards(unit.combat.aim, facing, self.types[ty].turret_step).0;
                continue;
            };
            let (point, sees) = match goal {
                Goal::Unit(t) => {
                    let p = self.units[t].position;
                    (p, self.sees_unit(i, t))
                }
                Goal::Ground(p) => (p, true),
                Goal::Search(p) => (p, false),
            };
            let delta = point - self.units[i].position;
            let distance = delta.length();
            let cell = self.vision.grid.cell;
            if matches!(goal, Goal::Search(_)) && distance < cell {
                // Nothing found where the target was last seen.
                let unit = &mut self.units[i];
                unit.combat.attack = None;
                unit.combat.last_seen = None;
                if unit.combat.attack_moving {
                    unit.combat.attack_moving = false;
                    stop(unit);
                }
                continue;
            }
            // Without a line of sight the unit drives on until it sees the
            // target (or reaches the point where it was last seen).
            let wanted = if sees {
                self.types[ty].max_range * Fixed::from_num(self.combat_rules.fire.attack_range_pct)
                    / 100
            } else {
                cell / 2
            };
            self.attack_move(i, point, distance, wanted, retarget);
            if distance < Fixed::ONE || !sees {
                continue;
            }
            let want = Vec2::new(delta.x / distance, delta.y / distance);
            let aimed = self.turn_to(i, want);
            if !aimed {
                continue;
            }
            for w in 0..self.types[ty].weapons.len() {
                // Out of ammunition: no weapon fires (prompt 036).
                if self.units[i].combat.reload[w] > 0 || !self.has_ammo(i) {
                    continue;
                }
                let ok = match goal {
                    Goal::Unit(t) => {
                        let target = &self.types[self.units[t].unit_type as usize];
                        self.types[ty].can_damage(w, &self.combat_rules, target, distance)
                    }
                    Goal::Ground(_) => distance <= self.types[ty].weapons[w].range,
                    Goal::Search(_) => false,
                };
                if ok {
                    let aim_point = match goal {
                        Goal::Unit(t) => self.aim_point(t),
                        Goal::Ground(p) | Goal::Search(p) => {
                            Vec3::new(p.x, p.y, self.map.height_at(p))
                        }
                    };
                    if self.units[i].squad.is_some() {
                        self.fire_squad(i, w, aim_point);
                    } else {
                        self.fire(i, w, aim_point);
                    }
                    if w == 0 && self.units[i].combat.reload[0] > 0 {
                        self.spend_salvo(i, false);
                    }
                }
            }
        }
    }

    /// Blind fire of squad `i` at `point`, where its target went out of
    /// sight (prompt 047): weapons that hurt infantry and reach the point,
    /// with the blind dispersion and a share of the soldiers.
    pub(super) fn blind_fire(&mut self, i: usize, point: Vec2) {
        if self.transport_blocks_fire(i) || !self.has_ammo(i) {
            return;
        }
        let ty = self.units[i].unit_type as usize;
        let delta = point - self.units[i].position;
        let distance = delta.length();
        if distance < Fixed::ONE {
            return;
        }
        self.units[i].combat.aim = Vec2::new(delta.x / distance, delta.y / distance);
        let aim_point = Vec3::new(point.x, point.y, self.map.height_at(point) + Fixed::ONE / 2);
        let mut fired = false;
        for w in 0..self.types[ty].weapons.len() {
            let weapon = &self.types[ty].weapons[w];
            if self.units[i].combat.reload[w] > 0
                || weapon.infantry_damage == 0
                || distance > weapon.range
            {
                continue;
            }
            self.fire_squad_as(i, w, aim_point, true);
            fired = true;
            if w == 0 && self.units[i].combat.reload[0] > 0 {
                self.spend_salvo(i, false);
            }
        }
        if fired && let Some(s) = &mut self.units[i].squad {
            s.tactics.blind_now = true;
        }
    }

    /// What unit `i` fires at: its attack order, else (fire at will) the
    /// automatic target, chosen again every `retarget_ticks`.
    fn goal(&mut self, i: usize, retarget: bool) -> Option<Goal> {
        match self.units[i].combat.attack {
            Some(Attack::Unit(id)) => {
                if let Ok(t) = self.units.binary_search_by_key(&id, |u| u.id)
                    && self.units[t].alive()
                {
                    // Only what the player sees is followed (prompt 018).
                    if self.owner_sees(i, t) {
                        self.units[i].combat.last_seen = Some(self.units[t].position);
                        return Some(Goal::Unit(t));
                    }
                    if let Some(p) = self.units[i].combat.last_seen {
                        return Some(Goal::Search(p));
                    }
                }
                // Target destroyed or never seen: the order is done.
                let unit = &mut self.units[i];
                unit.combat.attack = None;
                unit.combat.last_seen = None;
                if unit.combat.attack_moving {
                    unit.combat.attack_moving = false;
                    stop(unit);
                }
            }
            Some(Attack::Ground(p)) => return Some(Goal::Ground(p)),
            None => {}
        }
        if self.units[i].combat.fire_mode == FireMode::HoldFire {
            self.units[i].combat.auto_target = None;
            return None;
        }
        let ty = &self.types[self.units[i].unit_type as usize];
        let current = self.units[i].combat.auto_target.and_then(|id| {
            let t = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
            let target = &self.units[t];
            let distance = (target.position - self.units[i].position).length();
            (target.alive() && distance <= ty.max_range && self.sees_unit(i, t)).then_some(t)
        });
        let chosen = if retarget || current.is_none() {
            self.choose_target(i)
        } else {
            current
        };
        self.units[i].combat.auto_target = chosen.map(|t| self.units[t].id);
        chosen.map(Goal::Unit)
    }

    /// Best automatic target of unit `i`: enemies the main weapon can hurt
    /// first, then enemies that can hurt it, then the nearest, then the
    /// lowest id. Only targets some weapon can hurt are considered.
    fn choose_target(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        let mine = &self.types[me.unit_type as usize];
        let rules = &self.combat_rules;
        let mut best: Option<((bool, bool, i64, UnitId), usize)> = None;
        // The key is a total order, so the candidates' order does not matter.
        let range = Vec2::new(mine.max_range, mine.max_range);
        let mut near = Vec::new();
        self.unit_grid
            .query(me.position - range, me.position + range, &mut near);
        for &j in &near {
            let j = j as usize;
            let other = &self.units[j];
            if self.teams.allied(other.owner, me.owner) || !other.alive() {
                continue;
            }
            let d = other.position - me.position;
            if d.x.abs() > mine.max_range || d.y.abs() > mine.max_range {
                continue;
            }
            // Only targets in the unit's own line of sight (prompt 018).
            if !self.sees_unit(i, j) {
                continue;
            }
            let distance = d.length();
            let theirs = &self.types[other.unit_type as usize];
            if !(0..mine.weapons.len()).any(|w| mine.can_damage(w, rules, theirs, distance)) {
                continue;
            }
            let main = mine.can_damage(0, rules, theirs, distance);
            let threat =
                (0..theirs.weapons.len()).any(|w| theirs.can_damage(w, rules, mine, distance));
            let key = (!main, !threat, distance.to_bits(), other.id);
            if best.is_none_or(|(b, _)| key < b) {
                best = Some((key, j));
            }
        }
        best.map(|(_, j)| j)
    }

    /// Drives a unit with an attack order into range of `point`, or stops it
    /// there.
    fn attack_move(
        &mut self,
        i: usize,
        point: Vec2,
        distance: Fixed,
        wanted: Fixed,
        retarget: bool,
    ) {
        // Squads in a building fire from there and do not leave it.
        if self.units[i].combat.attack.is_none() || inside(&self.units[i]) {
            return;
        }
        let ty = &self.types[self.units[i].unit_type as usize];
        // No halt on a bridge deck: a group stopping there blocks the
        // crossing, and on the narrow deck its members stand in each other's
        // line of fire. The unit drives on until it is off the deck.
        let on_deck = self.map.deck_at(self.units[i].position).is_some();
        let unit = &mut self.units[i];
        if distance <= wanted && !on_deck {
            if unit.combat.attack_moving {
                unit.combat.attack_moving = false;
                stop(unit);
            }
            return;
        }
        let moved_away = unit
            .target
            .is_none_or(|t| (t - point).length() > ty.max_range / 4);
        if !unit.combat.attack_moving || (retarget && moved_away) {
            unit.combat.attack_moving = true;
            let id = self.paths.request(
                (&self.map, &self.terrain),
                unit.position,
                point,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(point);
            unit.route = Route::Waiting(id);
            unit.replans = 0;
        }
    }

    /// Turns the turret (or, for casemates, the hull) of unit `i` towards
    /// `want`; returns whether the gun points there.
    fn turn_to(&mut self, i: usize, want: Vec2) -> bool {
        let ty = &self.types[self.units[i].unit_type as usize];
        let unit = &mut self.units[i];
        let Some(arc_cos) = ty.arc_cos else {
            let (aim, done) = rotate_towards(unit.combat.aim, want, ty.turret_step);
            unit.combat.aim = aim;
            return done;
        };
        if dot(unit.facing, want) >= arc_cos {
            let (aim, done) = rotate_towards(unit.combat.aim, want, ty.turret_step);
            unit.combat.aim = aim;
            return done;
        }
        // Outside the traverse: the gun goes back to the center, and a
        // standing vehicle turns its hull.
        if unit.route == Route::None {
            unit.facing = rotate_towards(unit.facing, want, ty.hull_step).0;
        }
        unit.combat.aim = rotate_towards(unit.combat.aim, unit.facing, ty.turret_step).0;
        false
    }

    /// Fires weapon `w` of unit `i` at `aim_point` (one projectile per
    /// barrel), with dispersion from the simulation's generator.
    fn fire(&mut self, i: usize, w: usize, aim_point: Vec3) {
        let rules = &self.combat_rules;
        let unit = &self.units[i];
        let ty = &self.types[unit.unit_type as usize];
        let weapon = &ty.weapons[w];
        let muzzle = Vec3::new(
            unit.position.x + unit.combat.aim.x * ty.half_length,
            unit.position.y + unit.combat.aim.y * ty.half_length,
            unit.height + ty.height * Fixed::from_num(rules.fire.muzzle_height_pct) / 100,
        );
        let delta = aim_point.ground() - muzzle.ground();
        let distance = delta.length();
        if distance < Fixed::ONE / 2 {
            return;
        }
        if self.friend_in_line(i, muzzle, aim_point) {
            return;
        }
        let unit = &self.units[i];
        let dir = Vec2::new(delta.x / distance, delta.y / distance);
        let moving = matches!(unit.route, Route::Following { .. });
        let mut spread = Fixed::from_num(rules.dispersion.mrad_x10) / 10_000 * ty.angle_scale;
        if moving {
            spread = spread * Fixed::from_num(rules.dispersion.moving_pct) / 100;
        }
        // Experienced crews shoot more accurately (prompt 050).
        spread = spread * Fixed::from_num(self.experience.dispersion_pct(unit.veterancy)) / 100;
        let (count, speed, gravity, range) = (
            weapon.weapon.count,
            weapon.speed,
            ty.gravity,
            weapon.range * Fixed::from_num(rules.ballistics.max_flight_range_pct) / 100,
        );
        let (shooter, shooter_type, caliber) =
            (unit.id, unit.unit_type, weapon.weapon.caliber_mm_x10);
        let ticks = distance / speed;
        let dz = aim_point.z - muzzle.z;
        let climb = dz / ticks + gravity * (ticks - Fixed::ONE) / 2;
        for _ in 0..count {
            let side = triangular(&mut self.rng) * spread;
            let up = triangular(&mut self.rng) * spread;
            let (s, c) = sin_cos_rad(side);
            let shot = Vec2::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
            let id = self.next_projectile;
            self.next_projectile = self.next_projectile.wrapping_add(1);
            self.projectiles.push(Projectile {
                id,
                shooter,
                shooter_type,
                weapon: u8::try_from(w).unwrap_or(u8::MAX),
                position: muzzle,
                velocity: Vec3::new(shot.x * speed, shot.y * speed, climb + up * speed),
                remaining: range,
                traveled: Fixed::ZERO,
                caliber_mm_x10: caliber,
                skip_object: None,
            });
            self.events.push(CombatEvent::Shot {
                unit: shooter,
                weapon: u8::try_from(w).unwrap_or(u8::MAX),
                projectile: id,
                muzzle,
                caliber_mm_x10: caliber,
            });
        }
        let reload = self.experience.reload_ticks(
            self.units[i].veterancy,
            self.types[shooter_type as usize].weapons[w].reload_ticks,
        );
        self.units[i].combat.reload[w] = reload;
    }

    /// Where shooters aim at unit `t`: the middle of its hull, or a living
    /// soldier of a squad (a different one for every round).
    fn aim_point(&self, t: usize) -> Vec3 {
        if let Some(turret) = self.dug_in_aim(t) {
            return turret;
        }
        let u = &self.units[t];
        let h = self.types[u.unit_type as usize].height;
        if let Some(squad) = &u.squad
            && let Some(s) = squad.aim_soldier(self.next_projectile)
        {
            let z = if squad.garrison.is_some() {
                s.height + self.window_height(squad.garrison)
            } else {
                // Men lying knocked down or wounded (prompts 048, 049).
                let stance = if s.knocked > 0 || s.wound.lying() {
                    crate::infantry::Stance::Prone
                } else {
                    squad.stance
                };
                s.height + self.infantry.stance_height(stance) / 2
            };
            return Vec3::new(s.position.x, s.position.y, z);
        }
        Vec3::new(u.position.x, u.position.y, u.height + h / 2)
    }

    /// Height of the windows over the ground in building `garrison`: half
    /// the building, at most 4 m.
    fn window_height(&self, garrison: Option<u32>) -> Fixed {
        garrison.map_or(Fixed::ZERO, |o| {
            let kind = self.map.objects()[o as usize].kind;
            (self.map.object_kinds()[usize::from(kind)].height / 2).min(Fixed::from_num(4))
        })
    }

    /// Fires weapon `w` of squad `i` at `aim_point`: weapons with several
    /// copies fire in proportion to the living soldiers, every round from
    /// the next living soldier (prompt 022). No fire while the squad
    /// changes its stance.
    fn fire_squad(&mut self, i: usize, w: usize, aim_point: Vec3) {
        self.fire_squad_as(i, w, aim_point, false);
    }

    /// [`Self::fire_squad`]; `blind`: at a point where the target was last
    /// seen (prompt 047). Suppression widens the dispersion, rushing and
    /// jammed soldiers do not fire, every round may jam the weapon.
    fn fire_squad_as(&mut self, i: usize, w: usize, aim_point: Vec3, blind: bool) {
        let unit = &self.units[i];
        let squad = unit.squad.as_ref().expect("squads have soldiers");
        if squad.changing > 0 {
            return;
        }
        let ty = &self.types[unit.unit_type as usize];
        let weapon = &ty.weapons[w];
        let alive = squad.alive_count();
        let size = u32::try_from(squad.soldiers.len()).unwrap_or(1).max(1);
        let count = (u32::from(weapon.weapon.count) * alive).div_ceil(size);
        let count = self.tactics.fire_count(squad, count, blind);
        let rules = &self.combat_rules;
        let moving =
            matches!(unit.route, Route::Following { .. }) && !super::tactics::steady(squad);
        let stance = &self.infantry.rules.stance.values(squad.stance);
        let mut spread = Fixed::from_num(rules.dispersion.mrad_x10) / 10_000 * ty.angle_scale;
        if moving {
            spread = spread * Fixed::from_num(rules.dispersion.moving_pct) / 100;
        }
        spread = spread * Fixed::from_num(stance.dispersion_pct) / 100;
        spread = spread * Fixed::from_num(self.tactics.dispersion_pct(squad, blind)) / 100;
        spread = spread * Fixed::from_num(self.experience.dispersion_pct(unit.veterancy)) / 100;
        let (speed, gravity, range) = (
            weapon.speed,
            ty.gravity,
            weapon.range * Fixed::from_num(rules.ballistics.max_flight_range_pct) / 100,
        );
        let (shooter, shooter_type, caliber) =
            (unit.id, unit.unit_type, weapon.weapon.caliber_mm_x10);
        let garrison = squad.garrison;
        let muzzle_z = if garrison.is_some() {
            self.window_height(garrison)
        } else {
            self.infantry.stance_height(squad.stance)
                * Fixed::from_num(rules.fire.muzzle_height_pct)
                / 100
        };
        let mut fired = false;
        for k in 0..count {
            let unit = &self.units[i];
            let squad = unit.squad.as_ref().expect("squads have soldiers");
            let Some(index) = squad.living_index(squad.turn.wrapping_add(k)) else {
                break;
            };
            let soldier = &squad.soldiers[index];
            // Knocked down by a blast (prompt 048), rushing with his fire
            // team or clearing a jam (prompt 047), wounded or helping a
            // comrade (prompt 049): no shot.
            if soldier.knocked > 0
                || !super::tactics::may_fire(squad, index)
                || !soldier.wound.can_fire()
            {
                continue;
            }
            let muzzle = Vec3::new(
                soldier.position.x + unit.combat.aim.x / 2,
                soldier.position.y + unit.combat.aim.y / 2,
                soldier.height + muzzle_z,
            );
            let delta = aim_point.ground() - muzzle.ground();
            let distance = delta.length();
            if distance < Fixed::ONE / 2 || self.friend_in_line(i, muzzle, aim_point) {
                continue;
            }
            // Soldiers fire out of their building, or over (around) the
            // object they take cover behind.
            let skip_object = garrison.or_else(|| soldier.cover.filter(|_| soldier.at_spot()));
            let dir = Vec2::new(delta.x / distance, delta.y / distance);
            let ticks = distance / speed;
            let dz = aim_point.z - muzzle.z;
            let climb = dz / ticks + gravity * (ticks - Fixed::ONE) / 2;
            let side = triangular(&mut self.rng) * spread;
            let up = triangular(&mut self.rng) * spread;
            let (sn, c) = sin_cos_rad(side);
            let shot = Vec2::new(dir.x * c - dir.y * sn, dir.x * sn + dir.y * c);
            let id = self.next_projectile;
            self.next_projectile = self.next_projectile.wrapping_add(1);
            self.projectiles.push(Projectile {
                id,
                shooter,
                shooter_type,
                weapon: u8::try_from(w).unwrap_or(u8::MAX),
                position: muzzle,
                velocity: Vec3::new(shot.x * speed, shot.y * speed, climb + up * speed),
                remaining: range,
                traveled: Fixed::ZERO,
                caliber_mm_x10: caliber,
                skip_object,
            });
            self.events.push(CombatEvent::Shot {
                unit: shooter,
                weapon: u8::try_from(w).unwrap_or(u8::MAX),
                projectile: id,
                muzzle,
                caliber_mm_x10: caliber,
            });
            fired = true;
            if self.tactics.roll_jam(&mut self.rng)
                && let Some(squad) = &mut self.units[i].squad
            {
                self.tactics.jam(squad, index);
            }
        }
        let reload = self.experience.reload_ticks(
            self.units[i].veterancy,
            self.types[shooter_type as usize].weapons[w].reload_ticks,
        );
        let unit = &mut self.units[i];
        if let Some(squad) = &mut unit.squad {
            squad.turn = squad.turn.wrapping_add(count);
        }
        if fired {
            unit.combat.reload[w] = reload;
        }
    }

    /// Whether soldier `k` of squad `j` is behind soft cover that catches
    /// a round flying along `d`; returns the object (prompt 022).
    fn soft_cover_absorbs(&mut self, j: usize, k: usize, d: Vec3) -> Option<u32> {
        let squad = self.units[j].squad.as_ref()?;
        let soldier = &squad.soldiers[k];
        let object = soldier.cover?;
        if !soldier.at_spot() {
            return None;
        }
        let pct = self.infantry.cover_pct(&self.map, object);
        if pct == 0 || !self.gives_cover(object) {
            return None;
        }
        // The object must lie between the shooter and the soldier.
        let towards = self.map.objects()[object as usize].position - soldier.position;
        if dot(towards, d.ground()) >= Fixed::ZERO {
            return None;
        }
        (self.rng.below(100) < pct).then_some(object)
    }

    /// A round struck map object `object`: if it is an occupied building,
    /// it hits one occupant with the building's exposure chance. Returns
    /// the squad index and soldier.
    pub(super) fn building_hit(
        &mut self,
        object: u32,
        caliber_mm_x10: u32,
    ) -> Option<(usize, usize)> {
        let rules = &self.infantry.rules.building;
        // Rubble of a ruin hides the soldiers in it less than walls do.
        // The exposure holds for a standing soldier and shrinks with the
        // stance height: soldiers lying in the rubble are hard to hit.
        if self.object_is_ruin(object) {
            let exposure = rules.ruin_exposure_pct;
            let inside = self.soldiers_in(object);
            if inside.is_empty() {
                return None;
            }
            let pick = self
                .rng
                .below(u32::try_from(inside.len()).unwrap_or(u32::MAX));
            let (j, k) = inside[pick as usize];
            let stance = self.units[j]
                .squad
                .as_ref()
                .map_or(Stance::Stand, |s| s.stance);
            let stances = &self.infantry.rules.stance;
            let scaled = exposure * stances.values(stance).height_mm / stances.stand.height_mm;
            return (self.rng.below(100) < scaled).then_some((j, k));
        }
        let damaged = self.building_condition(object) == super::BuildingCondition::Damaged;
        let extra = if damaged { rules.damaged_extra_pct } else { 0 };
        let kind = self.infantry.building(&self.map, object)?;
        let exposure = if caliber_mm_x10 >= self.infantry.rules.building.heavy_caliber_mm_x10 {
            kind.heavy_exposure_pct
        } else {
            kind.exposure_pct
        };
        let exposure = (exposure + extra).min(100);
        let inside: Vec<(usize, usize)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.alive())
            .filter_map(|(j, u)| u.squad.as_ref().map(|s| (j, s)))
            .filter(|(_, s)| s.garrison == Some(object))
            .flat_map(|(j, s)| {
                s.soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.alive())
                    .map(move |(k, _)| (j, k))
            })
            .collect();
        if inside.is_empty() || self.rng.below(100) >= exposure {
            return None;
        }
        let pick = self
            .rng
            .below(u32::try_from(inside.len()).unwrap_or(u32::MAX));
        Some(inside[pick as usize])
    }

    /// Whether a living unit of the same player or an ally (prompt 037)
    /// stands in the straight line
    /// from `muzzle` to `aim_point` (units do not fire through friends).
    fn friend_in_line(&self, i: usize, muzzle: Vec3, aim_point: Vec3) -> bool {
        let me = &self.units[i];
        let d = sub(aim_point, muzzle);
        let (low, high) = (
            Vec2::new(muzzle.x.min(aim_point.x), muzzle.y.min(aim_point.y)),
            Vec2::new(muzzle.x.max(aim_point.x), muzzle.y.max(aim_point.y)),
        );
        let reach = Vec2::new(self.unit_grid.max_reach, self.unit_grid.max_reach);
        let mut near = Vec::new();
        self.unit_grid.query(low - reach, high + reach, &mut near);
        near.iter().any(|&j| {
            let u = &self.units[j as usize];
            if u.id == me.id || self.teams.hostile(u.owner, me.owner) || !u.alive() {
                return false;
            }
            let ty = &self.types[u.unit_type as usize];
            let reach = reach_of(u, ty);
            let p = u.position;
            p.x >= low.x - reach
                && p.x <= high.x + reach
                && p.y >= low.y - reach
                && p.y <= high.y + reach
                && friend_hit(u, ty, &self.infantry, muzzle, d)
        })
    }

    /// Moves every projectile one tick and resolves hits: the earliest of
    /// unit, map object and ground along its path wins.
    pub(super) fn fly_projectiles(&mut self) {
        let projectiles = std::mem::take(&mut self.projectiles);
        let mut destroyed: Vec<(usize, UnitId, Vec3)> = Vec::new();
        let mut near = Vec::new();
        for mut p in projectiles {
            let a = p.position;
            let b = add(a, p.velocity);
            let d = p.velocity;
            let mut best: Option<Candidate> = None;
            let better =
                |best: &Option<Candidate>, t: Fixed| best.as_ref().is_none_or(|(bt, ..)| t < *bt);
            // Candidates in index order: the first of equally early hits
            // wins, as with the loop over all units.
            let reach = Vec2::new(self.unit_grid.max_reach, self.unit_grid.max_reach);
            let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
            let high = Vec2::new(a.x.max(b.x), a.y.max(b.y));
            self.unit_grid
                .query_sorted(low - reach, high + reach, &mut near);
            for &j in &near {
                let j = j as usize;
                let unit = &self.units[j];
                if unit.id == p.shooter {
                    continue;
                }
                let ty = &self.types[unit.unit_type as usize];
                if !unit.alive() && ty.infantry {
                    continue;
                }
                let reach = reach_of(unit, ty);
                if unit.position.x < a.x.min(b.x) - reach
                    || unit.position.x > a.x.max(b.x) + reach
                    || unit.position.y < a.y.min(b.y) - reach
                    || unit.position.y > a.y.max(b.y) + reach
                {
                    continue;
                }
                if let Some((t, face, soldier)) = unit_hit(unit, ty, &self.infantry, a, d)
                    && better(&best, t)
                {
                    // The hull of a dug-in vehicle is behind its parapet
                    // (prompt 052).
                    best = Some(if self.below_parapet(j, at(a, d, t)) {
                        (t, HitKind::Ground, None)
                    } else {
                        (t, HitKind::Unit(unit.id), Some((j, face, soldier)))
                    });
                }
            }
            let first = p.traveled == Fixed::ZERO;
            // A squad's own building or cover object is not in its way
            // (prompt 022).
            let height_of = |i: u32| {
                (p.skip_object != Some(i))
                    .then(|| self.destruction.hit_height(&self.map, i))
                    .flatten()
            };
            let skip_inside = |i: u32| first && self.object_is_ruin(i);
            if let Some((t, object)) = self.objects.hit(&self.map, a, b, height_of, skip_inside)
                && better(&best, t)
            {
                best = Some((t, HitKind::Object(object), None));
            }
            let ground = terrain_hit(&self.map, a, d);
            if let Some(Some(t)) = ground
                && better(&best, t)
            {
                best = Some((t, HitKind::Ground, None));
            }
            let Some((t, mut kind, mut unit_hit)) = best else {
                // Flies on, unless it leaves the map or its range.
                let step = d.ground().length();
                p.position = b;
                p.velocity.z -= self.types[p.shooter_type as usize].gravity;
                p.traveled += step;
                p.remaining -= step;
                if ground.is_some() && p.remaining > Fixed::ZERO {
                    self.projectiles.push(p);
                }
                continue;
            };
            let point = at(a, d, t);
            p.traveled += d.ground().length() * t;
            let (mut face, mut turret, mut penetrated, mut damage) = (None, false, false, 0);
            // Soft cover catches some rounds aimed at soldiers behind it,
            // and rounds into an occupied building may hit an occupant
            // (prompt 022).
            if let Some((j, _, Some(soldier))) = unit_hit
                && let Some(object) = self.soft_cover_absorbs(j, soldier, d)
            {
                kind = HitKind::Object(object);
                unit_hit = None;
            }
            // A soldier in a crater may be saved by its rim (prompt 021).
            if let Some((j, _, Some(soldier))) = unit_hit
                && !self.trench_exposed(j, soldier, p.traveled, d)
                && self.crater_catches(j, soldier)
            {
                kind = HitKind::Ground;
                unit_hit = None;
            }
            if let HitKind::Object(object) = kind
                && let Some((j, soldier)) = self.building_hit(object, p.caliber_mm_x10)
            {
                kind = HitKind::Unit(self.units[j].id);
                unit_hit = Some((j, Face::Side, Some(soldier)));
            }
            // Rounds that hit no occupant damage the object (prompt 020).
            if let HitKind::Object(object) = kind {
                let w = &self.types[p.shooter_type as usize].weapons[usize::from(p.weapon)];
                damage = self.destruction.hit(
                    &self.map,
                    object,
                    (p.caliber_mm_x10, w.vehicle_damage),
                    d.ground(),
                    point,
                    p.shooter,
                );
            }
            if let Some((j, hit_face, soldier)) = unit_hit {
                let result = self.damage_unit(j, &p, hit_face, point);
                face = Some(hit_face);
                (turret, penetrated, damage) = result;
                let unit = &mut self.units[j];
                if damage > 0 && unit.alive() {
                    match (&mut unit.squad, soldier) {
                        (Some(squad), Some(k)) => {
                            let s = &mut squad.soldiers[k];
                            s.hp = s.hp.saturating_sub(damage);
                            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
                            self.tactics.hit(squad);
                        }
                        _ => unit.hp = unit.hp.saturating_sub(damage),
                    }
                    if unit.hp == 0 {
                        destroyed.push((j, p.shooter, point));
                    }
                }
            }
            self.near_miss(point.ground(), p.shooter, p.caliber_mm_x10);
            if kind == HitKind::Ground {
                self.ground_hit(point.ground(), p.caliber_mm_x10);
            }
            self.events.push(CombatEvent::Hit {
                projectile: p.id,
                shooter: p.shooter,
                at: point,
                kind,
                face,
                turret,
                penetrated,
                damage,
                caliber_mm_x10: p.caliber_mm_x10,
            });
        }
        for (j, by, at) in destroyed {
            self.destroy(j, by, at);
        }
    }

    /// Armor check of a hit on unit `j`: `(turret, penetrated, damage)`.
    fn damage_unit(
        &mut self,
        j: usize,
        p: &Projectile,
        face: Face,
        point: Vec3,
    ) -> (bool, bool, u32) {
        let rules = &self.combat_rules;
        let target = &self.units[j];
        let ty = &self.types[target.unit_type as usize];
        let shooter = &self.types[p.shooter_type as usize];
        let weapon = &shooter.weapons[usize::from(p.weapon)];
        if !target.alive() {
            return (false, false, 0);
        }
        if ty.infantry {
            return (false, true, weapon.infantry_damage);
        }
        let turret_from =
            ty.height * Fixed::from_num(100 - rules.penetration.turret_height_pct.min(100)) / 100;
        let turret = ty.armor.turret.is_some() && point.z - target.height > turret_from;
        let plates = if turret {
            ty.armor.turret.unwrap_or(ty.armor.hull)
        } else {
            ty.armor.hull
        };
        let plate = match face {
            Face::Front => plates[0],
            Face::Side => plates[1],
            Face::Rear | Face::Top => plates[2],
        };
        if plate == 0 {
            return (turret, true, weapon.vehicle_damage);
        }
        // Angle between the flight path and the plate's normal.
        let right = Vec2::new(-target.facing.y, target.facing.x);
        let d = p.velocity;
        let along = match face {
            Face::Front | Face::Rear => dot(d.ground(), target.facing),
            Face::Side => dot(d.ground(), right),
            Face::Top => d.z,
        };
        let length = d.length();
        let mut cos = if length > Fixed::ZERO {
            along.abs() / length
        } else {
            Fixed::ONE
        };
        if face == Face::Front && !turret {
            cos *= sin_cos_deg(ty.armor.hull_front_deg).1;
        }
        let min_cos = Fixed::from_num(rules.penetration.min_cos_pct) / 100;
        let effective = Fixed::from_num(plate) / cos.max(min_cos);
        let spread = i32::try_from(rules.penetration.spread_pct).unwrap_or(0);
        let roll = self.rng.range_i32(-spread, spread);
        let pen = rules.penetration_at(&weapon.weapon, p.traveled * shooter.real_per_game)
            * Fixed::from_num(100 + roll)
            / 100;
        let penetrated = pen >= effective;
        (
            turret,
            penetrated,
            if penetrated { weapon.vehicle_damage } else { 0 },
        )
    }

    /// Unit `j` lost its last hit point.
    pub(super) fn destroy(&mut self, j: usize, by: UnitId, at: Vec3) {
        let ty = &self.types[self.units[j].unit_type as usize];
        let (infantry, half_length, half_width) = (ty.infantry, ty.half_length, ty.half_width);
        let gun = ty.weapons.iter().map(|w| w.weapon.caliber_mm_x10).max();
        let unit: &mut Unit = &mut self.units[j];
        stop(unit);
        unit.combat.attack = None;
        unit.combat.attack_moving = false;
        unit.combat.auto_target = None;
        let id = unit.id;
        self.events
            .push(CombatEvent::Destroyed { unit: id, by, at });
        if infantry {
            return;
        }
        // The wreck blocks the cells under its hull.
        let (f, c) = (unit.facing, unit.position);
        let ext = Vec2::new(
            half_length * f.x.abs() + half_width * f.y.abs(),
            half_length * f.y.abs() + half_width * f.x.abs(),
        );
        self.set_path_obstacle(c - ext, c + ext, true);
        // Its ammunition may blow up (prompt 048).
        self.ammo_explosion(j, gun.unwrap_or(0));
    }
}

/// Rotates `current` towards `want` by at most one step `(sin, cos)`;
/// `None` turns at once. Returns the new direction and whether it reached
/// `want`.
pub(super) fn rotate_towards(
    current: Vec2,
    want: Vec2,
    step: Option<(Fixed, Fixed)>,
) -> (Vec2, bool) {
    let Some((sin, cos)) = step else {
        return (want, true);
    };
    if dot(current, want) >= cos {
        return (want, true);
    }
    let cross = current.x * want.y - current.y * want.x;
    let s = if cross >= Fixed::ZERO { sin } else { -sin };
    let turned = Vec2::new(
        current.x * cos - current.y * s,
        current.x * s + current.y * cos,
    );
    let length = turned.length();
    if length == Fixed::ZERO {
        return (want, true);
    }
    (Vec2::new(turned.x / length, turned.y / length), false)
}

/// Triangular random value in `-1..=1`.
fn triangular(rng: &mut crate::rng::Rng) -> Fixed {
    let a = rng.range_i32(-1000, 1000);
    let b = rng.range_i32(-1000, 1000);
    Fixed::from_num(a + b) / 2000
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

fn add(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn at(a: Vec3, d: Vec3, t: Fixed) -> Vec3 {
    Vec3::new(a.x + d.x * t, a.y + d.y * t, a.z + d.z * t)
}

/// Below this, a direction component counts as parallel to a slab.
fn tiny() -> Fixed {
    Fixed::from_bits(1 << 12)
}

/// Entry of the segment `p + d·t` (`t` in `0..=1`) into the box
/// `|x_k| <= half_k` / `lo_k..=hi_k` per axis: `(t, axis, from the high side)`.
fn slab(
    p: &[Fixed],
    d: &[Fixed],
    lo: &[Fixed],
    hi: &[Fixed],
) -> Option<(Fixed, Option<(usize, bool)>)> {
    let (mut t_in, mut t_out, mut entry) = (Fixed::ZERO, Fixed::ONE, None);
    for k in 0..p.len() {
        if d[k].abs() < tiny() {
            if p[k] < lo[k] || p[k] > hi[k] {
                return None;
            }
            continue;
        }
        let t1 = (lo[k] - p[k]) / d[k];
        let t2 = (hi[k] - p[k]) / d[k];
        let (near, far) = if d[k] > Fixed::ZERO {
            (t1, t2)
        } else {
            (t2, t1)
        };
        if near > t_in {
            t_in = near;
            entry = Some((k, d[k] < Fixed::ZERO));
        }
        t_out = t_out.min(far);
        if t_in > t_out {
            return None;
        }
    }
    Some((t_in, entry))
}

fn slab_2d(p: [Fixed; 2], d: [Fixed; 2], half: [Fixed; 2]) -> Option<Fixed> {
    slab(&p, &d, &[-half[0], -half[1]], &half).map(|(t, _)| t)
}

/// Entry of the segment `a + d·t` into a unit's hit box, and the face.
fn box_hit(unit: &Unit, ty: &TypeCombat, a: Vec3, d: Vec3) -> Option<(Fixed, Face)> {
    let f = unit.facing;
    let r = Vec2::new(-f.y, f.x);
    let rel = a.ground() - unit.position;
    let p = [dot(rel, f), dot(rel, r), a.z - unit.height];
    let v = [dot(d.ground(), f), dot(d.ground(), r), d.z];
    let lo = [-ty.half_length, -ty.half_width, Fixed::ZERO];
    let hi = [ty.half_length, ty.half_width, ty.height];
    let (t, entry) = slab(&p, &v, &lo, &hi)?;
    let face = match entry {
        Some((0, true)) => Face::Front,
        Some((0, false)) => Face::Rear,
        Some((1, _)) => Face::Side,
        _ => Face::Top,
    };
    Some((t, face))
}

/// Half the extent of a unit for bounding-box tests: a squad's formation,
/// else its hull.
pub(super) fn reach_of(unit: &Unit, ty: &TypeCombat) -> Fixed {
    if unit.squad.is_some() {
        unit.spread
    } else {
        ty.half_length + ty.half_width
    }
}

/// Clearance around friendly soldiers that a squad's fire keeps, in
/// meters: rounds spread, so a soldier does not fire past a comrade's head.
const FRIEND_CLEARANCE_M: i32 = 1;

/// Whether the line of fire `a + d·t` passes a friendly unit: its hit box,
/// or a soldier's hit box grown by [`FRIEND_CLEARANCE_M`] on every side.
fn friend_hit(unit: &Unit, ty: &TypeCombat, rt: &InfantryRt, a: Vec3, d: Vec3) -> bool {
    let Some(squad) = &unit.squad else {
        return box_hit(unit, ty, a, d).is_some();
    };
    if squad.garrison.is_some() {
        return false;
    }
    let margin = Fixed::from_num(FRIEND_CLEARANCE_M);
    let half = rt.half_width() + margin;
    let height = rt.stance_height(squad.stance) + margin;
    squad.soldiers.iter().filter(|s| s.alive()).any(|s| {
        let rel = a.ground() - s.position;
        let p = [rel.x, rel.y, a.z - s.height];
        let v = [d.x, d.y, d.z];
        slab(&p, &v, &[-half, -half, Fixed::ZERO], &[half, half, height]).is_some()
    })
}

/// Entry of the segment `a + d·t` into a unit: its hit box, or for a squad
/// the hit box of the soldier hit first (`(t, face, soldier)`). Soldiers in
/// a building are only hit through it ([`Sim::building_hit`]).
fn unit_hit(
    unit: &Unit,
    ty: &TypeCombat,
    rt: &InfantryRt,
    a: Vec3,
    d: Vec3,
) -> Option<(Fixed, Face, Option<usize>)> {
    let Some(squad) = &unit.squad else {
        return box_hit(unit, ty, a, d).map(|(t, face)| (t, face, None));
    };
    if squad.garrison.is_some() {
        return None;
    }
    let f = unit.facing;
    let r = Vec2::new(-f.y, f.x);
    let half = rt.half_width();
    let height = rt.stance_height(squad.stance);
    let lying = rt.stance_height(crate::infantry::Stance::Prone);
    let v = [dot(d.ground(), f), dot(d.ground(), r), d.z];
    let mut best: Option<(Fixed, Face, Option<usize>)> = None;
    for (k, s) in squad.soldiers.iter().enumerate() {
        if !s.alive() {
            continue;
        }
        let rel = a.ground() - s.position;
        let p = [dot(rel, f), dot(rel, r), a.z - s.height];
        // Knocked-down soldiers lie (prompt 048).
        let height = if s.knocked > 0 || s.wound.lying() {
            lying
        } else {
            height
        };
        let Some((t, entry)) = slab(&p, &v, &[-half, -half, Fixed::ZERO], &[half, half, height])
        else {
            continue;
        };
        if best.as_ref().is_some_and(|(bt, ..)| *bt <= t) {
            continue;
        }
        let face = match entry {
            Some((0, true)) => Face::Front,
            Some((0, false)) => Face::Rear,
            Some((1, _)) => Face::Side,
            _ => Face::Top,
        };
        best = Some((t, face, Some(k)));
    }
    best
}

/// First point of `p + d·t` within the circle, or `None`.
fn segment_circle(p: Vec2, d: Vec2, center: Vec2, radius: Fixed) -> Option<Fixed> {
    let f = p - center;
    let c = dot(f, f) - radius * radius;
    if c <= Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    let a = dot(d, d);
    if a == Fixed::ZERO {
        return None;
    }
    let b = dot(f, d);
    let disc = b * b - a * c;
    if disc < Fixed::ZERO || b >= Fixed::ZERO {
        return None;
    }
    let t = (-b - crate::combat::fixed_sqrt(disc)) / a;
    (t <= Fixed::ONE).then_some(t.max(Fixed::ZERO))
}

/// Where the segment `a + d·t` first goes below the ground: `Some(None)` if
/// it stays above, `None` if it leaves the map before.
fn terrain_hit(map: &Map, a: Vec3, d: Vec3) -> Option<Option<Fixed>> {
    let below = |t: Fixed| {
        let p = at(a, d, t);
        p.z < map.height_at(p.ground())
    };
    if !map.contains(a.ground()) {
        return None;
    }
    if below(Fixed::ZERO) {
        return Some(Some(Fixed::ZERO));
    }
    // Samples at most 1 m apart.
    let steps = d.ground().length().ceil().to_num::<u32>().clamp(1, 1000);
    let mut previous = Fixed::ZERO;
    for k in 1..=steps {
        let t = Fixed::from_num(k) / Fixed::from_num(steps);
        if !map.contains(at(a, d, t).ground()) {
            return None;
        }
        if below(t) {
            let (mut low, mut high) = (previous, t);
            for _ in 0..12 {
                let mid = (low + high) / 2;
                if below(mid) {
                    high = mid;
                } else {
                    low = mid;
                }
            }
            return Some(Some(high));
        }
        previous = t;
    }
    Some(None)
}
