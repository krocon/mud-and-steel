//! The combat update: target selection, aiming, firing, projectile flight
//! and hits (prompt 017). Rules and data types: `crate::combat`.
//!
//! Order within a tick (see [`Sim::step`]): commands, path planning,
//! [`Sim::aim_and_fire`] (targets, attack movement, turrets, new
//! projectiles), unit movement, [`Sim::fly_projectiles`] (flight, hits,
//! destruction).

use super::{Route, Sim, Unit, stop};
use crate::combat::{
    CombatEvent, CombatRules, Face, FireMode, HitKind, ProjectileSnapshot, Vec3, WeaponType,
    deg_to_rad, sin_cos_rad,
};
use crate::command::UnitId;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::{Shape, sin_cos_deg};

/// An attack order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Attack {
    Unit(UnitId),
    Ground(Vec2),
}

/// Combat state of one unit.
#[derive(Clone, Debug)]
pub(super) struct CombatState {
    /// Direction of the turret / main weapon (unit length).
    pub(super) aim: Vec2,
    /// Ticks until each weapon can fire again.
    reload: Vec<u32>,
    pub(super) fire_mode: FireMode,
    pub(super) attack: Option<Attack>,
    /// The unit drives towards its attack target.
    attack_moving: bool,
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
    hull_step: Option<(Fixed, Fixed)>,
    /// Cosine of half the casemate traverse; `None` = turret.
    arc_cos: Option<Fixed>,
    weapons: Vec<WeaponRt>,
    max_range: Fixed,
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
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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
    fn hit(&self, map: &Map, a: Vec3, b: Vec3) -> Option<(Fixed, u32)> {
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
            if best.is_some_and(|(bt, _)| bt <= t) {
                continue;
            }
            let p = at(a, d, t);
            if p.z - map.height_at(p.ground()) < kind.height {
                best = Some((t, i));
            }
        }
        best
    }
}

/// Earliest hit of a projectile so far: `(t, what, unit index and face)`.
type Candidate = (Fixed, HitKind, Option<(usize, Face)>);

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

    /// Target selection, attack movement, aiming and firing of every unit.
    pub(super) fn aim_and_fire(&mut self) {
        let retarget = self
            .tick
            .is_multiple_of(u64::from(self.combat_rules.fire.retarget_ticks));
        for i in 0..self.units.len() {
            if !self.units[i].alive() {
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
                    (p, self.unit_sees(i, p))
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
                if self.units[i].combat.reload[w] > 0 {
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
                        Goal::Unit(t) => {
                            let u = &self.units[t];
                            let h = self.types[u.unit_type as usize].height;
                            Vec3::new(u.position.x, u.position.y, u.height + h / 2)
                        }
                        Goal::Ground(p) | Goal::Search(p) => {
                            Vec3::new(p.x, p.y, self.map.height_at(p))
                        }
                    };
                    self.fire(i, w, aim_point);
                }
            }
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
            (target.alive() && distance <= ty.max_range && self.unit_sees(i, target.position))
                .then_some(t)
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
        for (j, other) in self.units.iter().enumerate() {
            if other.owner == me.owner || !other.alive() {
                continue;
            }
            let d = other.position - me.position;
            if d.x.abs() > mine.max_range || d.y.abs() > mine.max_range {
                continue;
            }
            // Only targets in the unit's own line of sight (prompt 018).
            if !self.unit_sees(i, other.position) {
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
        if self.units[i].combat.attack.is_none() {
            return;
        }
        let ty = &self.types[self.units[i].unit_type as usize];
        let unit = &mut self.units[i];
        if distance <= wanted {
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
            });
            self.events.push(CombatEvent::Shot {
                unit: shooter,
                weapon: u8::try_from(w).unwrap_or(u8::MAX),
                projectile: id,
                muzzle,
                caliber_mm_x10: caliber,
            });
        }
        self.units[i].combat.reload[w] = self.types[shooter_type as usize].weapons[w].reload_ticks;
    }

    /// Whether a living unit of the same player stands in the straight line
    /// from `muzzle` to `aim_point` (units do not fire through friends).
    fn friend_in_line(&self, i: usize, muzzle: Vec3, aim_point: Vec3) -> bool {
        let me = &self.units[i];
        let d = sub(aim_point, muzzle);
        let (low, high) = (
            Vec2::new(muzzle.x.min(aim_point.x), muzzle.y.min(aim_point.y)),
            Vec2::new(muzzle.x.max(aim_point.x), muzzle.y.max(aim_point.y)),
        );
        self.units.iter().any(|u| {
            if u.id == me.id || u.owner != me.owner || !u.alive() {
                return false;
            }
            let ty = &self.types[u.unit_type as usize];
            let reach = ty.half_length + ty.half_width;
            let p = u.position;
            p.x >= low.x - reach
                && p.x <= high.x + reach
                && p.y >= low.y - reach
                && p.y <= high.y + reach
                && box_hit(u, ty, muzzle, d).is_some()
        })
    }

    /// Moves every projectile one tick and resolves hits: the earliest of
    /// unit, map object and ground along its path wins.
    pub(super) fn fly_projectiles(&mut self) {
        let projectiles = std::mem::take(&mut self.projectiles);
        let mut destroyed: Vec<(usize, UnitId, Vec3)> = Vec::new();
        for mut p in projectiles {
            let a = p.position;
            let b = add(a, p.velocity);
            let d = p.velocity;
            let mut best: Option<Candidate> = None;
            let better =
                |best: &Option<Candidate>, t: Fixed| best.as_ref().is_none_or(|(bt, ..)| t < *bt);
            for (j, unit) in self.units.iter().enumerate() {
                if unit.id == p.shooter {
                    continue;
                }
                let ty = &self.types[unit.unit_type as usize];
                if !unit.alive() && ty.infantry {
                    continue;
                }
                let reach = ty.half_length + ty.half_width;
                if unit.position.x < a.x.min(b.x) - reach
                    || unit.position.x > a.x.max(b.x) + reach
                    || unit.position.y < a.y.min(b.y) - reach
                    || unit.position.y > a.y.max(b.y) + reach
                {
                    continue;
                }
                if let Some((t, face)) = box_hit(unit, ty, a, d)
                    && better(&best, t)
                {
                    best = Some((t, HitKind::Unit(unit.id), Some((j, face))));
                }
            }
            if let Some((t, object)) = self.objects.hit(&self.map, a, b)
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
            let Some((t, kind, unit_hit)) = best else {
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
            if let Some((j, hit_face)) = unit_hit {
                let result = self.damage_unit(j, &p, hit_face, point);
                face = Some(hit_face);
                (turret, penetrated, damage) = result;
                let unit = &mut self.units[j];
                if damage > 0 && unit.alive() {
                    unit.hp = unit.hp.saturating_sub(damage);
                    if unit.hp == 0 {
                        destroyed.push((j, p.shooter, point));
                    }
                }
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
    fn destroy(&mut self, j: usize, by: UnitId, at: Vec3) {
        let ty = &self.types[self.units[j].unit_type as usize];
        let (infantry, half_length, half_width) = (ty.infantry, ty.half_length, ty.half_width);
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
    }
}

/// Rotates `current` towards `want` by at most one step `(sin, cos)`;
/// `None` turns at once. Returns the new direction and whether it reached
/// `want`.
fn rotate_towards(current: Vec2, want: Vec2, step: Option<(Fixed, Fixed)>) -> (Vec2, bool) {
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
