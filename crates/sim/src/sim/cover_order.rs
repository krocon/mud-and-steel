//! Targeted cover command, cover preview and squad cover grades (prompt 233).
//!
//! Single source of truth for cover spot placement: [`Sim::cover_preview`] and
//! [`Sim::order_take_cover`] use the exact same spot-generation and assignment
//! logic as `Sim::choose_spots_for` in `sim/infantry.rs`. No duplicate cover
//! geometry lives in GDScript.

use super::craters::nearest_sample;
use super::infantry::{distance_to_shape, mm, right_of, spots_behind};
use super::{RejectReason, Sim, within_world};
use crate::command::{PlayerId, UnitId};
use crate::formation::{Engage, Formation};
use crate::infantry::Stance;
use crate::math::{Fixed, Vec2};

fn normalized(v: Vec2) -> Option<Vec2> {
    let len = v.length();
    (len > Fixed::ZERO).then(|| Vec2::new(v.x / len, v.y / len))
}

/// Search radius (m) for the "nearest cover" quick command (double-press of
/// cover key or clicking within `NEAREST_TRIGGER_M` of the squad's position).
const NEAREST_COVER_M: i32 = 25;
/// Click distance (m) from the group center that triggers "nearest cover".
const NEAREST_TRIGGER_M: i32 = 2;

/// One previewed soldier cover spot (world position, facing away from cover,
/// and cover grade: `0` open, `1` light, `2` heavy).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverSpot {
    pub position: Vec2,
    pub facing: Vec2,
    pub grade: u8,
}

/// Internal cover candidate tag so grades stay faithful to the actual source.
#[derive(Clone, Copy)]
enum SpotKind {
    Object(u32),
    Trench,
    Sandbag,
    Crater,
}

impl Sim {
    /// Cover grade of map object `object`: `0` none, `1` light (`< 60 %` soft
    /// cover, hedges, wooden fences), `2` heavy (stone/mud walls, sandbags
    /// `>= 60 %`, rocks, buildings/ruins, solid trunks).
    pub(super) fn object_cover_grade(&self, object: u32) -> u8 {
        if !self.gives_cover(object) {
            return 0;
        }
        let pct = self.infantry.cover_pct(&self.map, object);
        if pct >= 60 {
            return 2;
        }
        if pct > 0 {
            return 1;
        }
        let kind = &self.map.object_kinds()[usize::from(self.map.objects()[object as usize].kind)];
        if kind.id.contains("hedge") || kind.id.contains("fence") {
            return 1;
        }
        if kind.blocks.large() && kind.height > Fixed::ZERO {
            return 2;
        }
        0
    }

    /// Cover grade at ground point `p` (optionally tied to map object `obj`).
    fn ground_cover_grade(&self, p: Vec2, obj: Option<u32>) -> u8 {
        let mut grade = obj.map_or(0, |o| self.object_cover_grade(o));
        if grade == 2 {
            return 2;
        }
        if self.near_sandbag(p) || self.map.near_tunnel_portal(p) {
            return 2;
        }
        if let Some(sample) = nearest_sample(&self.map, p) {
            if self.dug_cover_pct(sample, Stance::Prone).is_some() {
                return 2;
            }
            if self.crater_cover_pct(sample, Stance::Prone) > 0 {
                grade = grade.max(1);
            }
        }
        grade
    }

    /// Overall cover grade of unit `id` (`0` open / vehicle, `1` light cover,
    /// `2` heavy cover or garrisoned building).
    pub fn unit_cover_grade(&self, id: UnitId) -> u8 {
        let Ok(i) = self.units.binary_search_by_key(&id, |u| u.id) else {
            return 0;
        };
        let u = &self.units[i];
        if !u.alive() || u.link.is_some() {
            return 0;
        }
        let Some(squad) = &u.squad else {
            return 0;
        };
        if squad.garrison.is_some() {
            return 2;
        }
        if u.route != super::Route::None {
            return 0;
        }
        let mut alive = 0u32;
        let mut heavy = 0u32;
        let mut covered = 0u32;
        for s in &squad.soldiers {
            if !s.alive() {
                continue;
            }
            alive += 1;
            let g = if s.at_spot() {
                self.ground_cover_grade(s.position, s.cover)
            } else {
                self.ground_cover_grade(s.position, None)
            };
            if g >= 2 {
                heavy += 1;
                covered += 1;
            } else if g == 1 {
                covered += 1;
            }
        }
        if alive == 0 {
            return 0;
        }
        if heavy * 2 >= alive {
            2
        } else if covered * 2 >= alive {
            1
        } else {
            0
        }
    }

    /// Resolves cover facing at `target`: explicit `facing` if valid; otherwise
    /// looks from `target` across the nearest cover object within `search_mm`
    /// so hovering on either side of a wall/hedge/sandbag line immediately
    /// faces over it; falls back to `target - from` or `fallback`.
    fn resolve_cover_facing(
        &self,
        target: Vec2,
        from: Vec2,
        facing: Option<Vec2>,
        fallback: Vec2,
    ) -> Vec2 {
        if let Some(f) = facing.filter(|v| within_world(*v)).and_then(normalized) {
            return f;
        }
        let search = mm(self.infantry.rules.cover.search_mm);
        let kinds = self.map.object_kinds();
        let objects = self.map.objects();
        let mut best: Option<(Fixed, u32, Vec2)> = None;
        for o in self.infantry.index.near(target, search) {
            let object = &objects[o as usize];
            let kind_index = usize::from(object.kind);
            if !self.infantry.cover_spot[kind_index] || !self.gives_cover(o) {
                continue;
            }
            let shape = kinds[kind_index].shape;
            let d = distance_to_shape(target, object, shape);
            if d > search {
                continue;
            }
            // Direction from the clicked side across the obstacle's wall.
            let dir = match shape {
                crate::objects::Shape::Circle { .. } => normalized(object.position - target),
                crate::objects::Shape::Box {
                    half_length,
                    half_width,
                } => {
                    let rel = target - object.position;
                    let (forward, right) = object.axes();
                    let a = rel.x * forward.x + rel.y * forward.y;
                    let b = rel.x * right.x + rel.y * right.y;
                    // Pick the face axis the target stands outside of (or the
                    // thinner axis if aligned along the wall).
                    let along_a = a.abs() - half_length;
                    let along_b = b.abs() - half_width;
                    let axis = if along_b >= along_a {
                        if b >= Fixed::ZERO {
                            Vec2::ZERO - right
                        } else {
                            right
                        }
                    } else if a >= Fixed::ZERO {
                        Vec2::ZERO - forward
                    } else {
                        forward
                    };
                    normalized(axis)
                }
            };
            if let Some(dir) = dir
                && best.is_none_or(|(bd, bo, _)| (d, o) < (bd, bo))
            {
                best = Some((d, o, dir));
            }
        }
        if let Some((_, _, dir)) = best {
            return dir;
        }
        if let Some(dir) = self.nearest_sandbag_facing(target, search) {
            return dir;
        }
        normalized(target - from)
            .or_else(|| normalized(fallback))
            .unwrap_or(Vec2::new(Fixed::ZERO, -Fixed::ONE))
    }

    /// Computes preview cover spots for one squad (`profile`, `n_soldiers`)
    /// placed at `center` facing `facing`, matching `Sim::choose_spots_for`
    /// bit-for-bit.
    fn squad_cover_spots(
        &self,
        profile: usize,
        n_soldiers: usize,
        center: Vec2,
        facing: Vec2,
    ) -> Vec<CoverSpot> {
        let Some(ty) = self.infantry.types.get(profile).and_then(|t| t.as_ref()) else {
            return Vec::new();
        };
        let rules = &self.infantry.rules.cover;
        let (gap, spacing, search) = (
            mm(rules.gap_mm),
            mm(rules.spot_spacing_mm),
            mm(rules.search_mm),
        );
        let right = right_of(facing);
        let kinds = self.map.object_kinds();
        let objects = self.map.objects();
        let mut spots: Vec<(Vec2, SpotKind)> = Vec::new();
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
            spots.extend(buffer.iter().map(|&p| (p, SpotKind::Object(o))));
        }
        for p in self.crater_spots(center, ty.spread / 2 + search) {
            spots.push((p, SpotKind::Crater));
        }
        for p in self.trench_spots(center, ty.spread / 2 + search, facing) {
            spots.push((p, SpotKind::Trench));
        }
        for p in self.sandbag_spots(center, ty.spread / 2 + search, facing) {
            spots.push((p, SpotKind::Sandbag));
        }
        let grid = &self.paths.grid;
        let map = &self.map;
        spots.retain(|&(p, _)| map.contains(p) && grid.passable_for(profile, p));

        let mut used = vec![false; spots.len()];
        let count = n_soldiers.min(ty.places.len());
        let mut out = Vec::with_capacity(count);
        for &(f, r) in ty.places.iter().take(count) {
            let place = center + facing.scale(f) + right.scale(r);
            let best = spots
                .iter()
                .enumerate()
                .filter(|&(k, _)| !used[k])
                .map(|(k, &(p, kind))| ((p - place).length(), k, p, kind))
                .filter(|&(d, ..)| d <= search)
                .min_by_key(|&(d, k, ..)| (d, k));
            match best {
                Some((_, k, p, kind)) => {
                    used[k] = true;
                    let grade = match kind {
                        SpotKind::Object(o) => self.object_cover_grade(o),
                        SpotKind::Trench | SpotKind::Sandbag => 2,
                        SpotKind::Crater => 1,
                    };
                    out.push(CoverSpot {
                        position: p,
                        facing,
                        grade,
                    });
                }
                None => {
                    let p = if map.contains(place) && grid.passable_for(profile, place) {
                        place
                    } else {
                        center
                    };
                    let grade = self.ground_cover_grade(p, None);
                    out.push(CoverSpot {
                        position: p,
                        facing,
                        grade,
                    });
                }
            }
        }
        out
    }

    /// Deterministically searches within `radius_m` of `from` for the best
    /// nearby cover spot (heavy grade 2 preferred over light grade 1, then
    /// nearest distance, deterministic tie-breaker). Returns `(target, facing)`.
    fn find_nearest_cover(&self, i: usize, radius_m: i32) -> Option<(Vec2, Vec2)> {
        let u = &self.units[i];
        let from = u.position;
        let profile = u.unit_type as usize;
        let radius = Fixed::from_num(radius_m);
        let gap = mm(self.infantry.rules.cover.gap_mm);
        let spacing = mm(self.infantry.rules.cover.spot_spacing_mm);
        let kinds = self.map.object_kinds();
        let objects = self.map.objects();
        let mut best: Option<(u8, Fixed, i64, i64, Vec2, Vec2)> = None;
        let mut consider = |p: Vec2, facing: Vec2, grade: u8| {
            if grade == 0 || !self.map.contains(p) || !self.paths.grid.passable_for(profile, p) {
                return;
            }
            let d = (p - from).length();
            if d > radius {
                return;
            }
            // Prefer heavy (2 -> rank 0) over light (1 -> rank 1), then closest.
            let key = (2u8.saturating_sub(grade), d, p.x.to_bits(), p.y.to_bits());
            if best.is_none_or(|(rk, bd, bx, by, ..)| key < (rk, bd, bx, by)) {
                best = Some((key.0, key.1, key.2, key.3, p, facing));
            }
        };
        let mut buf = Vec::new();
        for o in self.infantry.index.near(from, radius) {
            let grade = self.object_cover_grade(o);
            if grade == 0 {
                continue;
            }
            let object = &objects[o as usize];
            let shape = kinds[usize::from(object.kind)].shape;
            // Face away from our own side (looking over the obstacle from `from`).
            let facing = normalized(object.position - from).unwrap_or(u.facing);
            buf.clear();
            spots_behind(object, shape, facing, gap, spacing, &mut buf);
            for &p in &buf {
                consider(p, facing, grade);
            }
        }
        for p in self.trench_spots(from, radius, u.facing) {
            consider(p, u.facing, 2);
        }
        for p in self.sandbag_spots(from, radius, u.facing) {
            let facing = self
                .nearest_sandbag_facing(p, mm(self.infantry.rules.cover.search_mm))
                .unwrap_or(u.facing);
            consider(p, facing, 2);
        }
        for p in self.crater_spots(from, radius) {
            consider(p, u.facing, 1);
        }
        best.map(|(_, _, _, _, p, f)| (p, f))
    }

    /// Preview of the soldier cover spots `Command::TakeCover` would assign for
    /// `units` of `player` at `target` with optional `facing` (prompt 233).
    pub fn cover_preview(
        &self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
        facing: Option<Vec2>,
    ) -> Vec<CoverSpot> {
        if !within_world(target) || !self.map.contains(target) {
            return Vec::new();
        }
        let squads: Vec<usize> = self
            .mobile_indices(player, units)
            .into_iter()
            .filter(|&i| self.units[i].squad.is_some())
            .collect();
        if squads.is_empty() {
            return Vec::new();
        }
        let count = Fixed::from_num(squads.len());
        let center = squads.iter().fold(Vec2::ZERO, |sum, &i| {
            let p = self.units[i].position;
            sum + Vec2::new(p.x / count, p.y / count)
        });
        let resolved =
            self.resolve_cover_facing(target, center, facing, self.units[squads[0]].facing);
        let (assigned, forward, _) =
            self.formation_slots_for(&squads, target, Formation::Line, Some(resolved), false);
        let mut out = Vec::new();
        for (i, slot) in assigned {
            let n = self.units[i]
                .squad
                .as_ref()
                .map_or(0, |s| s.alive_count() as usize);
            out.extend(self.squad_cover_spots(self.units[i].unit_type as usize, n, slot, forward));
        }
        out
    }

    /// Executes [`Command::TakeCover`] for owned infantry squads (prompt 233).
    pub(super) fn order_take_cover(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
        facing: Option<Vec2>,
    ) {
        if !within_world(target) || !self.map.contains(target) {
            self.reject(player, RejectReason::OutsideMap, None);
            return;
        }
        let squads: Vec<usize> = self
            .mobile_indices(player, units)
            .into_iter()
            .filter(|&i| self.units[i].squad.is_some())
            .collect();
        if squads.is_empty() {
            if !self.owned_indices(player, units).is_empty() {
                self.reject(player, RejectReason::NoSquads, None);
            }
            return;
        }
        let count = Fixed::from_num(squads.len());
        let center = squads.iter().fold(Vec2::ZERO, |sum, &i| {
            let p = self.units[i].position;
            sum + Vec2::new(p.x / count, p.y / count)
        });
        // Clicking near own position (or double-pressing the cover key) finds
        // the nearest cover within 25 m deterministically.
        let (move_target, resolved) = if facing.is_none()
            && (target - center).length() <= Fixed::from_num(NEAREST_TRIGGER_M)
        {
            match self.find_nearest_cover(squads[0], NEAREST_COVER_M) {
                Some((p, f)) => (p, f),
                None => (
                    target,
                    self.resolve_cover_facing(target, center, None, self.units[squads[0]].facing),
                ),
            }
        } else {
            (
                target,
                self.resolve_cover_facing(target, center, facing, self.units[squads[0]].facing),
            )
        };
        let squad_ids: Vec<UnitId> = squads.iter().map(|&i| self.units[i].id).collect();
        let _ = self.order_formation_move(
            player,
            &squad_ids,
            move_target,
            Formation::Line,
            Some(resolved),
            Engage::Never,
        );
        for &i in &squads {
            if let Some(s) = &mut self.units[i].squad {
                s.spots_at = None;
            }
        }
    }
}
