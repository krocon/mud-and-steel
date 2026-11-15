//! Terrain deformation (prompt 021): craters change the heights and the
//! terrain types of the map, and the path grid, the vision grid, unit
//! heights and infantry cover follow. The rules: `crate::craters`.
//!
//! The map is copied once on the first crater (copy-on-write through
//! `Arc::make_mut`) and changed in place afterwards. A raster of `i16`
//! deviations from the loaded heights (allocated with the first crater)
//! bounds how deep and how high the ground may go and tells infantry and
//! the presentation where craters are. Memory stays constant however many
//! craters there are.
//!
//! Order within a tick (see [`Sim::step`]): ground hits of heavy rounds are
//! collected in [`Sim::fly_projectiles`] (at most `max_per_tick`), and
//! [`Sim::apply_craters`] (after [`Sim::apply_destruction`], before the
//! vision update) changes the samples, rebuilds the path cells (components
//! and routes only when passability changed), the vision cells whose ground
//! moved (and queues the viewsheds that contain them), and the heights of
//! units and soldiers standing there.

use super::{Route, Sim};
use crate::craters::{CraterEvent, CraterRules};
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::path::{CellRect, SizeClass};
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Crater state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Craters {
    rules: CraterRules,
    /// Terrain id of the bowl.
    bowl_terrain: u8,
    /// Per terrain id: never changes, takes no craters.
    keep: Vec<bool>,
    /// Per map sample: height minus the loaded height (mm). Empty until
    /// the first crater.
    delta_mm: Vec<i16>,
    /// Ground hits of this tick that make craters: point and caliber.
    pending: Vec<(Vec2, u32)>,
    /// Craters made so far.
    count: u64,
    /// Running hash over every crater and every sample it changed.
    checksum: u64,
    /// Craters of the last tick (derived, not hashed).
    pub(super) events: Vec<CraterEvent>,
}

impl Craters {
    pub(super) fn new(rules: CraterRules, terrain: &TerrainRules) -> Result<Self, String> {
        let id_of = |name: &str| {
            terrain
                .types
                .iter()
                .position(|t| t.name == name)
                .and_then(|i| u8::try_from(i).ok())
                .ok_or_else(|| format!("unknown terrain type '{name}'"))
        };
        let bowl_terrain = id_of(&rules.crater.terrain)?;
        let mut keep = vec![false; terrain.types.len()];
        for name in &rules.crater.keep {
            keep[usize::from(id_of(name)?)] = true;
        }
        Ok(Self {
            rules,
            bowl_terrain,
            keep,
            delta_mm: Vec::new(),
            pending: Vec::new(),
            count: 0,
            checksum: 0,
            events: Vec::new(),
        })
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u64(self.count);
        h.write_u64(self.checksum);
        h.write_u64(self.pending.len() as u64);
        for (p, caliber) in &self.pending {
            h.write_i64(p.x.to_bits());
            h.write_i64(p.y.to_bits());
            h.write_u32(*caliber);
        }
    }

    /// Deviation of sample `k` from the loaded map (mm).
    fn delta(&self, k: usize) -> i32 {
        self.delta_mm.get(k).map_or(0, |&d| i32::from(d))
    }
}

/// Index of the map sample nearest to `p` (`None` outside the map).
fn nearest_sample(map: &Map, p: Vec2) -> Option<usize> {
    if !map.contains(p) {
        return None;
    }
    let (sx, sy) = map.samples();
    let cell = Fixed::from_num(map.cell_size_mm()) / 1000;
    let half = map.size();
    let axis = |v: Fixed, half: Fixed, n: u32| {
        ((v + half / 2) / cell)
            .round()
            .to_num::<i64>()
            .clamp(0, i64::from(n) - 1) as usize
    };
    Some(axis(p.y, half.y, sy) * sx as usize + axis(p.x, half.x, sx))
}

fn to_mm(v: Fixed) -> i64 {
    (v * Fixed::from_num(1000)).round().to_num::<i64>()
}

/// Ground point of sample `(i, j)`.
fn sample_point(map: &Map, i: u32, j: u32) -> Vec2 {
    let (hx, hy) = map.half_mm();
    let cell = i64::from(map.cell_size_mm());
    let mm = |v: i64| i32::try_from(v).unwrap_or(i32::MAX);
    Vec2::from_millimeters(mm(i64::from(i) * cell - hx), mm(i64::from(j) * cell - hy))
}

impl Sim {
    /// A round struck the ground at `point`: a heavy one leaves a crater
    /// (applied at the end of the tick).
    pub(super) fn ground_hit(&mut self, point: Vec2, caliber_mm_x10: u32) {
        let c = &mut self.craters;
        if c.rules.makes_crater(caliber_mm_x10)
            && c.pending.len() < c.rules.crater.max_per_tick as usize
        {
            c.pending.push((point, caliber_mm_x10));
        }
    }

    /// Queues a crater of a round of `caliber_mm_x10` at `point`, as a
    /// ground hit would; it is made in the next tick (and counts towards
    /// that tick's limit). For game systems and tools (benchmarks, the
    /// debug menu), never for player commands.
    pub fn queue_crater(&mut self, point: Vec2, caliber_mm_x10: u32) {
        self.ground_hit(point, caliber_mm_x10);
    }

    /// Makes the craters of this tick (see the module documentation).
    pub(super) fn apply_craters(&mut self) {
        self.craters.events.clear();
        let pending = std::mem::take(&mut self.craters.pending);
        if pending.is_empty() {
            return;
        }
        // Samples to change, decided on the unchanged map.
        let mut plans = Vec::with_capacity(pending.len());
        for &(point, caliber) in &pending {
            if let Some(plan) = self.plan_crater(point, caliber) {
                plans.push(plan);
            }
        }
        if plans.is_empty() {
            return;
        }
        let (sx, sy) = self.map.samples();
        let c = &mut self.craters;
        if c.delta_mm.is_empty() {
            c.delta_mm = vec![0; sx as usize * sy as usize];
        }
        let (max_depth, max_rise) = (
            -i64::from(c.rules.crater.max_depth_mm),
            i64::from(c.rules.crater.max_rise_mm),
        );
        let (heights, terrain) = Arc::make_mut(&mut self.map).rasters_mut();
        let mut areas = Vec::with_capacity(plans.len());
        for plan in plans {
            let mut h = StableHasher::new();
            h.write_u64(c.checksum);
            h.write_i64(plan.event.at.x.to_bits());
            h.write_i64(plan.event.at.y.to_bits());
            h.write_u32(plan.event.caliber_mm_x10);
            for &(k, dh, bowl) in &plan.samples {
                let old = i64::from(c.delta_mm[k]);
                let new = (old + dh).clamp(max_depth, max_rise);
                heights[k] += i32::try_from(new - old).unwrap_or(0);
                c.delta_mm[k] = i16::try_from(new).unwrap_or(0);
                if bowl && !c.keep[usize::from(terrain[k])] {
                    terrain[k] = c.bowl_terrain;
                }
                h.write_u64(k as u64);
                h.write_u32(heights[k].cast_unsigned());
                h.write_u8(terrain[k]);
            }
            c.checksum = h.finish();
            c.count += 1;
            areas.push(plan.area);
            c.events.push(plan.event);
        }
        self.after_craters(&areas);
    }

    /// The samples a crater at `point` changes: `(index, height change,
    /// in the bowl)`, with its area and event. `None` on water, on a deck
    /// or off the map.
    fn plan_crater(&self, point: Vec2, caliber: u32) -> Option<CraterPlan> {
        let map = &self.map;
        let c = &self.craters;
        let k = nearest_sample(map, point)?;
        if c.keep[usize::from(map.terrain_ids()[k])] || map.deck_at(point).is_some() {
            return None;
        }
        let size = c.rules.size(caliber);
        let (sx, sy) = map.samples();
        let (hx, hy) = map.half_mm();
        let cell = i64::from(map.cell_size_mm());
        let (px, py) = (to_mm(point.x), to_mm(point.y));
        let floor = |v: i64| v.div_euclid(cell);
        let ceil = |v: i64| -(-v).div_euclid(cell);
        let range = |p: i64, half: i64, n: u32| {
            let lo = ceil(p - size.outer_mm + half).max(0);
            let hi = floor(p + size.outer_mm + half).min(i64::from(n) - 1);
            (lo <= hi).then_some((lo as u32, hi as u32))
        };
        let (i0, i1) = range(px, hx, sx)?;
        let (j0, j1) = range(py, hy, sy)?;
        let r2 = size.radius_mm * size.radius_mm;
        let mut samples = Vec::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let dx = i64::from(i) * cell - hx - px;
                let dy = i64::from(j) * cell - hy - py;
                let d2 = dx * dx + dy * dy;
                let dh = size.profile(d2);
                let bowl = d2 < r2;
                if (dh == 0 && !bowl) || map.sample_under_deck(i, j) {
                    continue;
                }
                samples.push((j as usize * sx as usize + i as usize, dh, bowl));
            }
        }
        let outer = Fixed::from_num(size.outer_mm) / 1000;
        Some(CraterPlan {
            samples,
            area: (
                point - Vec2::new(outer, outer),
                point + Vec2::new(outer, outer),
            ),
            event: CraterEvent {
                at: point,
                caliber_mm_x10: caliber,
                radius: Fixed::from_num(size.radius_mm) / 1000,
                depth: Fixed::from_num(size.depth_mm) / 1000,
                samples: (i0, j0, i1, j1),
            },
        })
    }

    /// Path grid, vision grid and unit heights after craters changed the
    /// ground in `areas`.
    fn after_craters(&mut self, areas: &[(Vec2, Vec2)]) {
        let map = Arc::clone(&self.map);
        // Path cells: flags and costs; components and routes only when a
        // class may now enter a cell it could not (or the reverse).
        let grid = &mut self.paths.grid;
        let (width, height) = grid.size();
        let mut blocked_changed = Vec::new();
        for &(low, high) in areas {
            let rect = grid.cells_in(low, high);
            let area = CellRect {
                x0: rect.x0.saturating_sub(1),
                y0: rect.y0.saturating_sub(1),
                x1: (rect.x1 + 1).min(width - 1),
                y1: (rect.y1 + 1).min(height - 1),
            };
            let before = passability(grid, area);
            let rebuilt = grid.rebuild_cells(&map, &self.terrain, rect);
            if passability(grid, area) != before {
                blocked_changed.push(rebuilt);
            }
        }
        if !blocked_changed.is_empty() {
            // Only these areas changed which cells are passable (prompt 016:
            // local relabeling, same regions as a full one).
            self.paths.grid.relabel_components_in(&blocked_changed);
            for rect in blocked_changed {
                self.replan_through(rect);
            }
        }
        // Vision cells whose ground moved.
        let mut moved = Vec::new();
        let mut changes = Vec::new();
        for &(low, high) in areas {
            let vision = &self.vision.grid;
            let (c0, r0) = vision.cell_of(low);
            let (c1, r1) = vision.cell_of(high);
            let rect = (c0 as u32, r0 as u32, c1 as u32, r1 as u32);
            let list = vision.ground_changes(rect, &map);
            if !list.is_empty() {
                changes.extend(list);
                moved.push(rect);
            }
        }
        if !changes.is_empty() {
            Arc::make_mut(&mut self.vision.grid).set_ground(&changes);
            for rect in moved {
                self.requeue_viewsheds(rect);
            }
        }
        // Units and soldiers standing there.
        let inside = |p: Vec2, reach: Fixed| {
            areas.iter().any(|&(low, high)| {
                p.x >= low.x - reach
                    && p.x <= high.x + reach
                    && p.y >= low.y - reach
                    && p.y <= high.y + reach
            })
        };
        let search = Fixed::from_num(self.infantry.rules.cover.search_mm) / 1000;
        for unit in &mut self.units {
            let garrisoned = unit.squad.as_ref().is_some_and(|s| s.garrison.is_some());
            // Standing squads nearby look for cover again (new craters).
            if !garrisoned
                && inside(unit.position, unit.spread + search)
                && let Some(squad) = &mut unit.squad
            {
                squad.spots_at = None;
            }
            if !inside(unit.position, unit.spread) {
                continue;
            }
            if !garrisoned {
                unit.height = map.height_at(unit.position);
            }
            if let Some(squad) = &mut unit.squad {
                for s in &mut squad.soldiers {
                    if inside(s.position, Fixed::ZERO) {
                        s.height = map.height_at(s.position);
                    }
                }
            }
        }
    }

    /// Whether the crater soldier `k` of squad `j` lies in catches a hit
    /// (prompt 021): the squad stands, the soldier's ground lies at least
    /// `min_depth_mm` below the loaded map, and the roll for his stance
    /// succeeds.
    pub(super) fn crater_catches(&mut self, j: usize, k: usize) -> bool {
        if self.craters.delta_mm.is_empty() {
            return false;
        }
        let unit = &self.units[j];
        let Some(squad) = &unit.squad else {
            return false;
        };
        if unit.route != Route::None || squad.garrison.is_some() {
            return false;
        }
        let Some(sample) = nearest_sample(&self.map, squad.soldiers[k].position) else {
            return false;
        };
        let cover = self.craters.rules.cover;
        if self.craters.delta(sample) > -i32::try_from(cover.min_depth_mm).unwrap_or(i32::MAX) {
            return false;
        }
        let pct = cover.pct(squad.stance);
        pct > 0 && self.rng.below(100) < pct
    }

    /// Crater samples (at least `min_depth_mm` deep) within `half` of
    /// `center` on both axes, as cover spots for infantry, row by row.
    pub(super) fn crater_spots(&self, center: Vec2, half: Fixed) -> Vec<Vec2> {
        if self.craters.delta_mm.is_empty() {
            return Vec::new();
        }
        let map = &self.map;
        let (sx, sy) = map.samples();
        let (hx, hy) = map.half_mm();
        let cell = i64::from(map.cell_size_mm());
        let min = -i32::try_from(self.craters.rules.cover.min_depth_mm).unwrap_or(i32::MAX);
        let range = |c: Fixed, half_map: i64, n: u32| {
            let lo = (to_mm(c - half) + half_map + cell - 1)
                .div_euclid(cell)
                .max(0);
            let hi = (to_mm(c + half) + half_map)
                .div_euclid(cell)
                .min(i64::from(n) - 1);
            (lo, hi)
        };
        let (i0, i1) = range(center.x, hx, sx);
        let (j0, j1) = range(center.y, hy, sy);
        let mut out = Vec::new();
        for j in j0..=j1 {
            for i in i0..=i1 {
                let k = j as usize * sx as usize + i as usize;
                if self.craters.delta(k) <= min {
                    out.push(sample_point(map, i as u32, j as u32));
                }
            }
        }
        out
    }

    /// Craters made during the last tick, in order.
    pub fn crater_events(&self) -> &[CraterEvent] {
        &self.craters.events
    }

    /// Number of craters made so far.
    pub const fn crater_count(&self) -> u64 {
        self.craters.count
    }

    /// Height of map sample `(i, j)` minus its loaded height, in
    /// millimeters (0 off the map or before the first crater).
    pub fn deformation_mm(&self, i: u32, j: u32) -> i32 {
        let (sx, sy) = self.map.samples();
        if i >= sx || j >= sy {
            return 0;
        }
        self.craters.delta(j as usize * sx as usize + i as usize)
    }

    /// Whether any crater has been made (the deformation raster exists).
    pub fn has_craters(&self) -> bool {
        !self.craters.delta_mm.is_empty()
    }

    /// The crater rules of this match.
    pub const fn crater_rules(&self) -> &CraterRules {
        &self.craters.rules
    }
}

/// Samples a crater changes, with its ground area and event.
struct CraterPlan {
    samples: Vec<(usize, i64, bool)>,
    area: (Vec2, Vec2),
    event: CraterEvent,
}

/// Passability of every size class over `rect`, for comparison.
fn passability(grid: &crate::path::PathGrid, rect: CellRect) -> Vec<bool> {
    let mut out = Vec::new();
    for class in SizeClass::ALL {
        for y in rect.y0..=rect.y1 {
            for x in rect.x0..=rect.x1 {
                out.push(grid.passable(class, x, y));
            }
        }
    }
    out
}
