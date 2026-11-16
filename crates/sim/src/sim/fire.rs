//! Fire (prompt 041): igniting map objects and wrecks, damage over time,
//! spreading to neighbors, dumps exploding, smoke and burning occupants.
//! The rules: `crate::fire`.
//!
//! [`Sim::update_fire`] runs once per tick after the projectiles and shells
//! and before [`Sim::apply_destruction`], so objects that burn down this
//! tick update the path and vision grids in the same tick:
//!
//! 1. ignite from this tick's combat events: hits on objects, shell bursts
//!    on the ground near objects, destroyed vehicles (wrecks);
//! 2. explode dumps whose fuse ran out (blast, neighbors catch fire);
//! 3. every spread interval: damage, burnt-out fires, spreading (one roll
//!    per neighbor within reach that is not behind a firebreak), occupants,
//!    smoke clouds;
//! 4. ignite from the events step 2 and 3 caused (wrecks of vehicles the
//!    blasts destroyed).
//!
//! Everything iterates in index order (`BTreeMap`) and rolls the match's
//! random generator only when there is something to roll for, so a match
//! without fire has the same random sequence as before.

use super::Sim;
use crate::combat::{CombatEvent, HitKind, Vec3};
use crate::command::UnitId;
use crate::destruction::ObjectStage;
use crate::fire::{FireEvent, FireEventKind, FireRules, FireSource, FireView};
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// A burning map object.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Burning {
    /// Credited with damage and kills (the unit whose hit started it).
    by: UnitId,
    started: u64,
    ends: u64,
    /// Hit points the fire has taken so far.
    dealt: u32,
    /// Tick of the explosion (dumps), until it happened.
    blast_at: Option<u64>,
    next_smoke: u64,
}

/// A burning vehicle wreck.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct WreckFire {
    at: Vec2,
    half: Fixed,
    by: UnitId,
    started: u64,
    ends: u64,
}

/// Per fire class, in ticks.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ClassTicks {
    burn: u64,
    fuse: u64,
}

/// Fire state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Fire {
    rules: FireRules,
    /// Per map object kind: index into `rules.classes`.
    kind_class: Vec<Option<usize>>,
    class_ticks: Vec<ClassTicks>,
    interval: u64,
    smoke_interval: u64,
    smoke_ticks: u64,
    wreck_ticks: u64,
    /// Occupant damage per spread interval.
    occupant_damage: u32,
    /// Per terrain id: fire does not cross it.
    firebreak: Vec<bool>,
    objects: BTreeMap<u32, Burning>,
    wrecks: BTreeMap<UnitId, WreckFire>,
    /// Objects whose fire burned out; they do not burn again.
    burnt: BTreeSet<u32>,
    /// Fire smoke clouds: (id, expiry tick).
    clouds: Vec<(u32, u64)>,
    /// Events of the last tick (derived, not hashed).
    pub(super) events: Vec<FireEvent>,
}

impl Fire {
    pub(super) fn new(
        rules: FireRules,
        map: &Map,
        terrain: &TerrainRules,
        tick_hz: u32,
    ) -> Result<Self, String> {
        let time = crate::Scale::builtin()
            .map_err(|e| e.to_string())?
            .time_factor;
        let hz = u64::from(tick_hz.max(1));
        let (num, den) = (u64::from(time.num.max(1)), u64::from(time.den.max(1)));
        // Real ms → game ticks (rounded, at least 1).
        let ticks = |ms: u32| ((u64::from(ms) * den * hz + 500 * num) / (1000 * num)).max(1);
        let kind_class = map
            .object_kinds()
            .iter()
            .map(|k| rules.class_of(&k.id))
            .collect();
        let class_ticks = rules
            .classes
            .iter()
            .map(|c| ClassTicks {
                burn: ticks(c.burn_ms),
                fuse: ticks(c.fuse_ms),
            })
            .collect();
        let mut firebreak = vec![false; terrain.types.len()];
        for name in &rules.spread.firebreak {
            let i = terrain
                .types
                .iter()
                .position(|t| &t.name == name)
                .ok_or_else(|| format!("fire: unknown firebreak terrain '{name}'"))?;
            firebreak[i] = true;
        }
        let occupant_damage = u32::try_from(
            u64::from(rules.occupants.damage_per_s) * u64::from(rules.spread.interval_ms) / 1000,
        )
        .unwrap_or(u32::MAX);
        Ok(Self {
            kind_class,
            class_ticks,
            interval: ticks(rules.spread.interval_ms),
            smoke_interval: ticks(rules.smoke.interval_ms),
            smoke_ticks: ticks(rules.smoke.lifetime_ms),
            wreck_ticks: ticks(rules.wreck.burn_ms),
            occupant_damage,
            firebreak,
            objects: BTreeMap::new(),
            wrecks: BTreeMap::new(),
            burnt: BTreeSet::new(),
            clouds: Vec::new(),
            events: Vec::new(),
            rules,
        })
    }

    fn class_index(&self, map: &Map, i: u32) -> Option<usize> {
        let k = usize::from(map.objects()[i as usize].kind);
        self.kind_class[k]
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u64(self.objects.len() as u64);
        for (&i, b) in &self.objects {
            h.write_u32(i);
            h.write_u32(b.by.0);
            h.write_u64(b.started);
            h.write_u64(b.ends);
            h.write_u32(b.dealt);
            h.write_u64(b.blast_at.unwrap_or(u64::MAX));
            h.write_u64(b.next_smoke);
        }
        h.write_u64(self.wrecks.len() as u64);
        for (id, w) in &self.wrecks {
            h.write_u32(id.0);
            h.write_i64(w.at.x.to_bits());
            h.write_i64(w.at.y.to_bits());
            h.write_i64(w.half.to_bits());
            h.write_u32(w.by.0);
            h.write_u64(w.started);
            h.write_u64(w.ends);
        }
        h.write_u64(self.burnt.len() as u64);
        for &i in &self.burnt {
            h.write_u32(i);
        }
        h.write_u64(self.clouds.len() as u64);
        for &(id, expires) in &self.clouds {
            h.write_u32(id);
            h.write_u64(expires);
        }
    }
}

/// Points `p` on box `a` and `q` on box `b` (axis-aligned, center and half
/// extent) closest to each other.
fn closest_points(a: (Vec2, Vec2), b: (Vec2, Vec2)) -> (Vec2, Vec2) {
    let axis = |ac: Fixed, ah: Fixed, bc: Fixed, bh: Fixed| -> (Fixed, Fixed) {
        let (a0, a1, b0, b1) = (ac - ah, ac + ah, bc - bh, bc + bh);
        if a1 < b0 {
            (a1, b0)
        } else if b1 < a0 {
            (a0, b1)
        } else {
            let m = (a0.max(b0) + a1.min(b1)) / 2;
            (m, m)
        }
    };
    let (px, qx) = axis(a.0.x, a.1.x, b.0.x, b.1.x);
    let (py, qy) = axis(a.0.y, a.1.y, b.0.y, b.1.y);
    (Vec2::new(px, py), Vec2::new(qx, qy))
}

/// Ground point of a map object.
fn at3(map: &Map, p: Vec2) -> Vec3 {
    Vec3::new(p.x, p.y, map.height_at(p))
}

impl Sim {
    /// One tick of fire (see the module documentation).
    pub(super) fn update_fire(&mut self) {
        self.fire.events.clear();
        let mut seen = 0;
        self.ignite_from_events(&mut seen);
        self.explode_dumps();
        if self.tick.is_multiple_of(self.fire.interval) {
            self.burn_interval();
        }
        self.ignite_from_events(&mut seen);
    }

    /// Combat events from `seen` on: hits ignite objects, destroyed
    /// vehicles become burning wrecks.
    fn ignite_from_events(&mut self, seen: &mut usize) {
        let ignite = self.fire.rules.ignite.clone();
        while *seen < self.events.len() {
            let event = self.events[*seen];
            *seen += 1;
            match event {
                CombatEvent::Hit {
                    shooter,
                    at,
                    kind,
                    caliber_mm_x10,
                    ..
                } => {
                    if caliber_mm_x10 < ignite.min_caliber_mm * 10 {
                        continue;
                    }
                    match kind {
                        HitKind::Object(o) => self.roll_ignite(o, shooter, false),
                        HitKind::Ground if caliber_mm_x10 >= ignite.burst_min_caliber_mm * 10 => {
                            let radius = Fixed::from_num(ignite.burst_radius_mm) / 1000;
                            self.ignite_near(at.ground(), radius, shooter, false);
                        }
                        _ => {}
                    }
                }
                CombatEvent::Destroyed { unit, by, .. } => self.start_wreck_fire(unit, by),
                _ => {}
            }
        }
    }

    /// Rolls whether object `o` catches fire (`hit_pct`, or `catch_pct`
    /// when `spread`) and sets it on fire.
    fn roll_ignite(&mut self, o: u32, by: UnitId, spread: bool) {
        let Some(c) = self.fire.class_index(&self.map, o) else {
            return;
        };
        if !self.can_ignite(o) {
            return;
        }
        let class = &self.fire.rules.classes[c];
        let pct = if spread {
            class.catch_pct
        } else {
            class.hit_pct
        };
        if pct > 0 && self.rng.below(100) < pct {
            self.ignite(o, by);
        }
    }

    /// Whether object `o` could catch fire now.
    fn can_ignite(&self, o: u32) -> bool {
        self.fire.class_index(&self.map, o).is_some()
            && !self.fire.objects.contains_key(&o)
            && !self.fire.burnt.contains(&o)
            && self.destruction.state(&self.map, o).stage != ObjectStage::Destroyed
    }

    /// Rolls every burning-capable object whose outline lies within
    /// `radius` of `p` (ascending index).
    fn ignite_near(&mut self, p: Vec2, radius: Fixed, by: UnitId, spread: bool) {
        let r = Vec2::new(radius, radius);
        for o in self.destruction.query(p - r, p + r) {
            let object = &self.map.objects()[o as usize];
            let shape = self.map.object_kinds()[usize::from(object.kind)].shape;
            let (q, _) = closest_points(
                (object.position, object.half_extent(shape)),
                (p, Vec2::ZERO),
            );
            if (q - p).length() <= radius {
                self.roll_ignite(o, by, spread);
            }
        }
    }

    /// Sets object `o` on fire (no roll).
    fn ignite(&mut self, o: u32, by: UnitId) {
        let Some(c) = self.fire.class_index(&self.map, o) else {
            return;
        };
        let t = self.fire.class_ticks[c];
        let explosive = self.fire.rules.classes[c].blast_caliber_mm > 0;
        let tick = self.tick;
        self.fire.objects.insert(
            o,
            Burning {
                by,
                started: tick,
                ends: tick + t.burn,
                dealt: 0,
                blast_at: explosive.then_some(tick + t.fuse),
                next_smoke: tick,
            },
        );
        let at = self.map.objects()[o as usize].position;
        self.fire.events.push(FireEvent {
            source: FireSource::Object(o),
            kind: FireEventKind::Ignited,
            at,
        });
    }

    /// A destroyed vehicle (not infantry) burns.
    fn start_wreck_fire(&mut self, unit: UnitId, by: UnitId) {
        let Ok(j) = self.units.binary_search_by_key(&unit, |u| u.id) else {
            return;
        };
        let u = &self.units[j];
        if self.unit_types[u.unit_type as usize].combat.is_infantry()
            || self.fire.wrecks.contains_key(&unit)
        {
            return;
        }
        let at = u.position;
        let tick = self.tick;
        self.fire.wrecks.insert(
            unit,
            WreckFire {
                at,
                half: u.footprint / 2,
                by,
                started: tick,
                ends: tick + self.fire.wreck_ticks,
            },
        );
        self.fire.events.push(FireEvent {
            source: FireSource::Wreck(unit),
            kind: FireEventKind::Ignited,
            at,
        });
    }

    /// Dumps whose fuse ran out explode: a blast like a shell, the dump is
    /// destroyed, neighbors within half the blast radius may catch fire.
    fn explode_dumps(&mut self) {
        let tick = self.tick;
        let due: Vec<(u32, UnitId)> = self
            .fire
            .objects
            .iter()
            .filter(|(_, b)| b.blast_at.is_some_and(|t| t <= tick))
            .map(|(&o, b)| (o, b.by))
            .collect();
        for (o, by) in due {
            if let Some(b) = self.fire.objects.get_mut(&o) {
                b.blast_at = None;
            }
            let Some(c) = self.fire.class_index(&self.map, o) else {
                continue;
            };
            let caliber = self.fire.rules.classes[c].blast_caliber_mm * 10;
            let p = self.map.objects()[o as usize].position;
            let at = at3(&self.map, p);
            let map = Arc::clone(&self.map);
            self.destruction.burn(&map, o, u32::MAX, at, by);
            self.detonate(at, caliber, by);
            self.fire.events.push(FireEvent {
                source: FireSource::Object(o),
                kind: FireEventKind::Exploded,
                at: p,
            });
            let radius = self.artillery.blast_radius(caliber) / 2;
            self.ignite_near(p, radius, by, true);
        }
    }

    /// Damage, burnt-out fires, spreading, occupants and smoke.
    fn burn_interval(&mut self) {
        let tick = self.tick;
        let map = Arc::clone(&self.map);
        // Damage, in proportion to the burn time passed.
        let burning: Vec<(u32, Burning)> =
            self.fire.objects.iter().map(|(&o, &b)| (o, b)).collect();
        let mut out = Vec::new();
        for (o, b) in burning {
            let Some(c) = self.fire.class_index(&map, o) else {
                continue;
            };
            let max = self.object_hp(o).map_or(0, |(_, max)| max);
            let total = u64::from(max) * u64::from(self.fire.rules.classes[c].burn_hp_pct) / 100;
            let duration = (b.ends - b.started).max(1);
            let elapsed = tick.min(b.ends) - b.started;
            let target = u32::try_from(total * elapsed / duration).unwrap_or(u32::MAX);
            let damage = target.saturating_sub(b.dealt);
            let p = map.objects()[o as usize].position;
            self.destruction.burn(&map, o, damage, at3(&map, p), b.by);
            if let Some(entry) = self.fire.objects.get_mut(&o) {
                entry.dealt = target.max(b.dealt);
            }
            if tick >= b.ends && b.blast_at.is_none() {
                out.push((o, p));
            }
        }
        for (o, p) in out {
            self.fire.objects.remove(&o);
            self.fire.burnt.insert(o);
            self.fire.events.push(FireEvent {
                source: FireSource::Object(o),
                kind: FireEventKind::BurntOut,
                at: p,
            });
        }
        let ended: Vec<(UnitId, Vec2)> = self
            .fire
            .wrecks
            .iter()
            .filter(|(_, w)| tick >= w.ends)
            .map(|(&id, w)| (id, w.at))
            .collect();
        for (id, at) in ended {
            self.fire.wrecks.remove(&id);
            self.fire.events.push(FireEvent {
                source: FireSource::Wreck(id),
                kind: FireEventKind::BurntOut,
                at,
            });
        }
        self.spread_fire();
        self.burn_occupants();
        self.fire_smoke();
    }

    /// Every fire rolls once for every neighbor within its reach that can
    /// burn and is not behind a firebreak. Fires started now spread from
    /// the next interval on.
    fn spread_fire(&mut self) {
        let map = Arc::clone(&self.map);
        let kinds = map.object_kinds();
        let mut sources: Vec<(Option<u32>, Vec2, Vec2, Fixed, UnitId)> = Vec::new();
        for (&o, b) in &self.fire.objects {
            let Some(c) = self.fire.class_index(&map, o) else {
                continue;
            };
            let object = &map.objects()[o as usize];
            let ext = object.half_extent(kinds[usize::from(object.kind)].shape);
            let reach = Fixed::from_num(self.fire.rules.classes[c].reach_mm) / 1000;
            sources.push((Some(o), object.position, ext, reach, b.by));
        }
        let wreck_reach = Fixed::from_num(self.fire.rules.wreck.reach_mm) / 1000;
        for w in self.fire.wrecks.values() {
            sources.push((None, w.at, Vec2::new(w.half, w.half), wreck_reach, w.by));
        }
        let mut caught: Vec<(u32, UnitId)> = Vec::new();
        for (from, center, ext, reach, by) in sources {
            if reach <= Fixed::ZERO {
                continue;
            }
            let r = Vec2::new(reach, reach);
            for o in self.destruction.query(center - ext - r, center + ext + r) {
                if Some(o) == from || !self.can_ignite(o) || caught.iter().any(|&(c, _)| c == o) {
                    continue;
                }
                let object = &map.objects()[o as usize];
                let other = (
                    object.position,
                    object.half_extent(kinds[usize::from(object.kind)].shape),
                );
                let (p, q) = closest_points((center, ext), other);
                let gap = q - p;
                if gap.length() > reach || self.firebreak_between(p, q) {
                    continue;
                }
                let c = self.fire.class_index(&map, o).unwrap_or(0);
                let pct = self.fire.rules.classes[c].catch_pct;
                if pct > 0 && self.rng.below(100) < pct {
                    caught.push((o, by));
                }
            }
        }
        for (o, by) in caught {
            self.ignite(o, by);
        }
    }

    /// Whether a firebreak (road, water) lies on the gap from `p` to `q`.
    fn firebreak_between(&self, p: Vec2, q: Vec2) -> bool {
        if p == q {
            return false;
        }
        (1..=3).any(|k| {
            let s = p + Vec2::new((q.x - p.x) * k / 4, (q.y - p.y) * k / 4);
            self.fire
                .firebreak
                .get(usize::from(self.map.terrain_at(s)))
                .copied()
                .unwrap_or(false)
        })
    }

    /// Soldiers inside burning buildings take damage.
    fn burn_occupants(&mut self) {
        let damage = self.fire.occupant_damage;
        if damage == 0 || self.fire.objects.is_empty() {
            return;
        }
        let mut killed = Vec::new();
        for (j, unit) in self.units.iter_mut().enumerate() {
            if !unit.alive() {
                continue;
            }
            let Some(squad) = unit.squad.as_mut() else {
                continue;
            };
            let Some(b) = squad.garrison.and_then(|o| self.fire.objects.get(&o)) else {
                continue;
            };
            for s in &mut squad.soldiers {
                s.hp = s.hp.saturating_sub(damage);
            }
            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            if unit.hp == 0 {
                let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
                killed.push((j, b.by, at));
            }
        }
        for (j, by, at) in killed {
            self.destroy(j, by, at);
        }
    }

    /// Burning objects with smoke put a dark cloud above them.
    fn fire_smoke(&mut self) {
        let tick = self.tick;
        self.fire.clouds.retain(|&(_, expires)| expires > tick);
        let max = self.fire.rules.smoke.max_clouds as usize;
        let due: Vec<(u32, Fixed)> = self
            .fire
            .objects
            .iter()
            .filter(|(_, b)| b.next_smoke <= tick)
            .filter_map(|(&o, _)| {
                let c = self.fire.class_index(&self.map, o)?;
                let r = self.fire.rules.classes[c].smoke_radius_mm;
                (r > 0).then(|| (o, Fixed::from_num(r) / 1000))
            })
            .collect();
        for (o, radius) in due {
            if let Some(b) = self.fire.objects.get_mut(&o) {
                b.next_smoke = tick + self.fire.smoke_interval;
            }
            if self.fire.clouds.len() >= max {
                continue;
            }
            let center = self.map.objects()[o as usize].position;
            let ticks = self.fire.smoke_ticks;
            let id = self.artillery.add_fire_cloud(tick, center, radius, ticks);
            self.fire.clouds.push((id, tick + ticks));
        }
    }

    /// Sets map object `index` on fire, credited to `by` (tests, tools and
    /// scenarios; it is a state change like a command, so it must happen at
    /// the same tick on every peer). Returns whether it caught fire: it
    /// must be able to burn, not burn already and not be destroyed.
    pub fn ignite_object(&mut self, index: u32, by: UnitId) -> bool {
        if (index as usize) >= self.map.objects().len() || !self.can_ignite(index) {
            return false;
        }
        self.ignite(index, by);
        true
    }

    /// Fires burning now: objects (ascending index), then wrecks
    /// (ascending unit id).
    pub fn fires(&self) -> Vec<FireView> {
        let tick = self.tick;
        let left = |ends: u64| u32::try_from(ends.saturating_sub(tick)).unwrap_or(u32::MAX);
        let age = |started: u64| u32::try_from(tick.saturating_sub(started)).unwrap_or(u32::MAX);
        let kinds = self.map.object_kinds();
        let mut out: Vec<FireView> = self
            .fire
            .objects
            .iter()
            .map(|(&o, b)| {
                let object = &self.map.objects()[o as usize];
                let ext = object.half_extent(kinds[usize::from(object.kind)].shape);
                FireView {
                    source: FireSource::Object(o),
                    at: object.position,
                    age_ticks: age(b.started),
                    ticks_left: left(b.ends),
                    size_m: ext.x.max(ext.y).max(Fixed::ONE),
                }
            })
            .collect();
        out.extend(self.fire.wrecks.iter().map(|(&id, w)| FireView {
            source: FireSource::Wreck(id),
            at: w.at,
            age_ticks: age(w.started),
            ticks_left: left(w.ends),
            size_m: w.half.max(Fixed::ONE),
        }));
        out
    }

    /// Whether map object `index` is burning.
    pub fn object_burning(&self, index: u32) -> bool {
        self.fire.objects.contains_key(&index)
    }

    /// Whether the fire of map object `index` burned out (it is charred).
    pub fn object_burnt(&self, index: u32) -> bool {
        self.fire.burnt.contains(&index)
    }

    /// Every object whose fire burned out, ascending.
    pub fn burnt_objects(&self) -> impl Iterator<Item = u32> + '_ {
        self.fire.burnt.iter().copied()
    }

    /// Fires that started, exploded or burned out during the last tick.
    pub fn fire_events(&self) -> &[FireEvent] {
        &self.fire.events
    }

    /// The fire rules of this match.
    pub const fn fire_rules(&self) -> &FireRules {
        &self.fire.rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_points_of_boxes() {
        let a = (Vec2::from_meters(0, 0), Vec2::from_meters(1, 1));
        let b = (Vec2::from_meters(5, 0), Vec2::from_meters(1, 2));
        let (p, q) = closest_points(a, b);
        assert_eq!(p, Vec2::from_meters(1, 0));
        assert_eq!(q, Vec2::from_meters(4, 0));
        // Overlapping boxes touch: no gap.
        let c = (Vec2::from_meters(1, 1), Vec2::from_meters(1, 1));
        let (p, q) = closest_points(a, c);
        assert_eq!(p, q);
    }
}
