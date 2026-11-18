//! Deterministic pathfinding: cost grid, A* and line-of-sight smoothing.
//!
//! The map is covered by a coarse grid of path cells ([`PATH_CELL_MM`],
//! 4 m = 2 x 2 samples on the usual 2 m height field). Every cell is
//! passable or blocked per size class (large vehicles keep a margin from
//! impassable ground), and every unit
//! type has its own integer cost per cell (time to cross it, from the terrain
//! speeds and the slope). A* runs on that grid with integer costs, an octile
//! heuristic and a total order on open nodes, so results never depend on heap
//! internals. Searches are resumable: [`Pathfinder`] works through a FIFO
//! queue with a fixed node budget per tick.
//!
//! Design and reasons: `docs/ARCHITECTURE.md` (section "Pathfinding").

use crate::command::UnitId;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::{Blocks, MapObject, Shape};
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// Target edge length of a path cell in millimeters. The grid uses a whole
/// number of map cells per path cell (2 on 2 m maps).
pub const PATH_CELL_MM: u32 = 4000;

/// A* node expansions per tick, shared by all queued requests.
pub const NODE_BUDGET_PER_TICK: u32 = 32_768;

/// Lookahead of the path smoothing in raw path cells. Bounds its cost; on
/// long straight stretches it leaves a (collinear) waypoint every 32 cells.
const SMOOTH_WINDOW: usize = 32;

/// Cost of crossing a cell of the fastest terrain on flat ground. All cell
/// costs are at least this, which keeps the heuristic admissible.
pub const MIN_CELL_COST: u16 = 100;

/// Where the heuristic's cost per cell lies between the cheapest cell
/// (0 %: admissible, optimal paths) and plain cross-country ground (100 %).
/// Above 0 the search gets faster but stops taking roads that are not on
/// the straight line (see `docs/ARCHITECTURE.md`).
pub const HEURISTIC_WEIGHT_PCT: i32 = 0;

/// How far (in cells) [`PathGrid::lead_in`] looks for a passable cell a
/// unit on a blocked cell can drive to.
const LEAD_IN_CELLS: u32 = 4;

/// How far (in cells) a search that starts on a blocked cell without
/// passable neighbors looks for a passable start cell.
const START_SEARCH_CELLS: u32 = 16;

/// Footprints above this (meters) belong to [`SizeClass::Large`].
const SMALL_MAX_FOOTPRINT_M: i32 = 4;

/// Clearance classes. For large units, impassable ground is grown by a
/// margin before cells are judged, so a unit whose center follows passable
/// cells keeps its hull off water and cliffs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SizeClass {
    /// Infantry and small vehicles (footprint up to 4 m): no margin.
    Small,
    /// Tanks and trucks: a margin of one map sample (2 m) around
    /// impassable terrain and the map edge.
    Large,
}

impl SizeClass {
    pub const ALL: [Self; 2] = [Self::Small, Self::Large];

    pub fn of_footprint(footprint_m: Fixed) -> Self {
        if footprint_m <= Fixed::from_num(SMALL_MAX_FOOTPRINT_M) {
            Self::Small
        } else {
            Self::Large
        }
    }

    /// Flags that block a cell for this class.
    const fn blocking(self) -> u8 {
        match self {
            Self::Small => {
                BLOCKED_TERRAIN
                    | BLOCKED_STEEP
                    | BLOCKED_OBSTACLE
                    | BLOCKED_OBJECT_SMALL
                    | BLOCKED_WORK_SMALL
            }
            Self::Large => {
                BLOCKED_TERRAIN
                    | BLOCKED_STEEP
                    | BLOCKED_OBSTACLE
                    | BLOCKED_MARGIN
                    | BLOCKED_OBJECT_LARGE
                    | BLOCKED_WORK_LARGE
            }
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Small => 0,
            Self::Large => 1,
        }
    }
}

/// Inclusive rectangle of path cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl CellRect {
    fn grow(self, by: u32, width: u32, height: u32) -> Self {
        Self {
            x0: self.x0.saturating_sub(by),
            y0: self.y0.saturating_sub(by),
            x1: (self.x1 + by).min(width - 1),
            y1: (self.y1 + by).min(height - 1),
        }
    }

    pub const fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }

    pub const fn overlaps(self, other: Self) -> bool {
        self.x0 <= other.x1 && other.x0 <= self.x1 && self.y0 <= other.y1 && other.y0 <= self.y1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ClassGrid {
    passable: Vec<bool>,
}

/// Connected regions for all profiles with the same size class and load
/// tier (they block exactly the same cells).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ComponentGrid {
    class: SizeClass,
    tier: u32,
    /// Connected region of every passable cell (4-neighborhood, which equals
    /// the reachability of the 8-neighborhood without corner cutting); 0 for
    /// blocked cells. Only equality of labels matters (searches compare
    /// them), not their values: [`PathGrid::relabel_components_in`] keeps
    /// the partition exact but numbers new regions differently from a full
    /// labeling.
    component: Vec<u32>,
    /// Largest label handed out so far.
    last_label: u32,
}

/// Movement profile of one unit type: its class and its cost per cell.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Profile {
    class: SizeClass,
    offroad: Fixed,
    road: Fixed,
    mass_kg: u32,
    /// Number of distinct bridge load limits of the map below `mass_kg`:
    /// profiles with the same class and tier block the same cells.
    tier: u32,
    /// Index into [`PathGrid::components`].
    components: usize,
    /// 0 = blocked, otherwise at least [`MIN_CELL_COST`].
    cost: Vec<u16>,
    /// Cost per cell assumed by the A* heuristic.
    heuristic: u16,
    /// Speed on soft ground in percent of the terrain speed, per terrain id
    /// (prompt 040, weather); empty: 100 % everywhere.
    #[serde(default)]
    ground_pct: Vec<u32>,
}

/// Cells around changed areas within which
/// [`PathGrid::relabel_components_in`] looks for a detour before it labels
/// the whole map again (32 m around a wreck or a crushed hedge; with 16 m,
/// clusters of wrecks in `cli bench` fell back now and then).
const RELABEL_MARGIN: u32 = 8;

/// Base flags of a cell.
/// One of its own samples is impassable terrain.
const BLOCKED_TERRAIN: u8 = 1;
/// Blocked by an obstacle ([`PathGrid::set_obstacle`]).
const BLOCKED_OBSTACLE: u8 = 2;
/// Too steep to climb somewhere within the cell.
const BLOCKED_STEEP: u8 = 4;
/// Impassable terrain or the map edge within one sample around the cell
/// (blocks [`SizeClass::Large`] only).
const BLOCKED_MARGIN: u8 = 8;
/// A map object that blocks small units overlaps the cell.
const BLOCKED_OBJECT_SMALL: u8 = 16;
/// A map object that blocks large units overlaps the cell.
const BLOCKED_OBJECT_LARGE: u8 = 32;
/// A field work that blocks small units (wire, prompt 035) overlaps the
/// cell.
const BLOCKED_WORK_SMALL: u8 = 64;
/// A field work that blocks large units (anti-tank obstacles, prompt 035)
/// overlaps the cell.
const BLOCKED_WORK_LARGE: u8 = 128;
/// Flags that do not come from the map samples.
const PLACED_FLAGS: u8 = BLOCKED_OBSTACLE
    | BLOCKED_OBJECT_SMALL
    | BLOCKED_OBJECT_LARGE
    | BLOCKED_WORK_SMALL
    | BLOCKED_WORK_LARGE;

/// Step cost factor in cells a search avoids (known minefields of the
/// requesting player, prompt 035). They stay passable, so a unit inside a
/// minefield still finds its way out.
pub const AVOID_FACTOR: u32 = 10;

/// Passability and costs of the path grid, derived from the map, the terrain
/// rules and the unit types of a match.
///
/// A path cell is exactly `stride` x `stride` map samples (2 x 2 on 2 m
/// maps) and covers the squares those samples own (`Map::terrain_at` uses
/// the nearest sample), so a cell is blocked exactly when the movement rule
/// would refuse some point in it. Cell `(x, y)` owns samples
/// `x * stride .. x * stride + stride`; samples left over at the far edges
/// belong to no cell.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathGrid {
    width: u32,
    height: u32,
    /// Map samples per path cell along each axis.
    stride: u32,
    map_cell_mm: u32,
    /// Edge length of a path cell in meters.
    cell: Fixed,
    /// Corner of cell (0, 0): half a map cell outside the first sample.
    origin: Vec2,
    /// `BLOCKED_*` flags per cell.
    base: Vec<u8>,
    /// Slope cost factor in permille (1000 = flat).
    slope_pml: Vec<u16>,
    classes: [ClassGrid; 2],
    /// One per (class, load tier) used by a profile.
    components: Vec<ComponentGrid>,
    /// Per cell: 1 + index of the intact bridge whose deck covers the cell
    /// center ([`crate::Map::bridges`]), 0 for none.
    deck: Vec<u16>,
    /// Per bridge: the cells near its deck (bounds grown by two cells).
    bridge_cells: Vec<CellRect>,
    /// Per bridge: unit vector across its deck. Steps between a deck cell
    /// and a cell beside it (across the axis) cross the railing.
    bridge_right: Vec<Vec2>,
    /// Indexed like [`crate::SimConfig::unit_types`].
    profiles: Vec<Profile>,
    /// Incremented by every change after construction.
    version: u32,
}

impl PathGrid {
    /// Builds the grid for `map` and one profile per unit type.
    pub fn new(map: &Map, rules: &TerrainRules, unit_types: &[UnitType]) -> Self {
        let (sx, sy) = map.samples();
        let stride = (PATH_CELL_MM / map.cell_size_mm()).max(1);
        let width = (sx / stride).max(1);
        let height = (sy / stride).max(1);
        let count = (width * height) as usize;
        let size = map.size();
        let half_sample = Fixed::from_num(map.cell_size_mm()) / Fixed::from_num(2000);
        let empty = || ClassGrid {
            passable: vec![false; count],
        };
        // Load tiers: distinct load limits of the map's bridges.
        let mut loads: Vec<u32> = map.bridges().iter().map(|b| b.max_load_kg).collect();
        loads.sort_unstable();
        loads.dedup();
        let tier = |mass: u32| loads.iter().filter(|&&l| l < mass).count() as u32;
        let mut components: Vec<ComponentGrid> = Vec::new();
        let mut grid = Self {
            width,
            height,
            stride,
            map_cell_mm: map.cell_size_mm(),
            cell: Fixed::from_num(stride * map.cell_size_mm()) / Fixed::from_num(1000),
            origin: Vec2::new(-size.x / 2 - half_sample, -size.y / 2 - half_sample),
            base: vec![0; count],
            slope_pml: vec![1000; count],
            classes: [empty(), empty()],
            profiles: unit_types
                .iter()
                .map(|t| {
                    let class = SizeClass::of_footprint(t.footprint_m);
                    let tier = tier(t.mass_kg);
                    let index = components
                        .iter()
                        .position(|c| c.class == class && c.tier == tier)
                        .unwrap_or_else(|| {
                            components.push(ComponentGrid {
                                class,
                                tier,
                                component: vec![0; count],
                                last_label: 0,
                            });
                            components.len() - 1
                        });
                    Profile {
                        class,
                        offroad: t.speed_m_per_s,
                        road: t.road_speed_m_per_s,
                        mass_kg: t.mass_kg,
                        tier,
                        components: index,
                        cost: vec![0; count],
                        heuristic: heuristic_cost(t),
                        ground_pct: Vec::new(),
                    }
                })
                .collect(),
            components: Vec::new(),
            deck: vec![0; count],
            bridge_cells: Vec::new(),
            bridge_right: map.bridges().iter().map(|b| b.right).collect(),
            version: 0,
        };
        grid.components = components;
        let two_cells = grid.cell * 2;
        grid.bridge_cells = map
            .bridges()
            .iter()
            .map(|b| {
                let (lo, hi) = b.bounds();
                let grow = Vec2::new(two_cells, two_cells);
                grid.cells_in(lo - grow, hi + grow)
            })
            .collect();
        let all = CellRect {
            x0: 0,
            y0: 0,
            x1: width - 1,
            y1: height - 1,
        };
        grid.rasterize_objects(map);
        grid.rebuild_cells(map, rules, all);
        grid.relabel_components();
        grid.version = 0;
        grid
    }

    /// Marks every cell overlapped by the shape of a blocking map object.
    fn rasterize_objects(&mut self, map: &Map) {
        let half = self.cell / 2;
        for object in map.objects() {
            let kind = &map.object_kinds()[usize::from(object.kind)];
            let flags = object_flags(kind.blocks);
            if flags == 0 {
                continue;
            }
            let extent = object.half_extent(kind.shape);
            let rect = self.cells_in(object.position - extent, object.position + extent);
            for y in rect.y0..=rect.y1 {
                for x in rect.x0..=rect.x1 {
                    if object.overlaps_square(kind.shape, self.center(x, y), half) {
                        let i = self.index(x, y);
                        self.base[i] |= flags;
                    }
                }
            }
        }
    }

    /// Whether a map object blocks the cell for `class`.
    pub fn object_blocked(&self, class: SizeClass, x: u32, y: u32) -> bool {
        let flag = match class {
            SizeClass::Small => BLOCKED_OBJECT_SMALL,
            SizeClass::Large => BLOCKED_OBJECT_LARGE,
        };
        self.base[self.index(x, y)] & flag != 0
    }

    /// Number of cells per axis.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Edge length of a cell in meters.
    pub const fn cell_size(&self) -> Fixed {
        self.cell
    }

    /// Corner of cell (0, 0) (smallest x and y).
    pub const fn origin(&self) -> Vec2 {
        self.origin
    }

    /// Number of changes since construction.
    pub const fn version(&self) -> u32 {
        self.version
    }

    pub fn class_of(&self, profile: usize) -> SizeClass {
        self.profiles[profile].class
    }

    /// Number of profiles (one per unit type).
    pub fn profiles(&self) -> usize {
        self.profiles.len()
    }

    /// Cost per cell of a profile (row-major, 0 = blocked).
    pub fn costs(&self, profile: usize) -> &[u16] {
        &self.profiles[profile].cost
    }

    pub fn passable(&self, class: SizeClass, x: u32, y: u32) -> bool {
        self.classes[class.index()].passable[self.index(x, y)]
    }

    /// Whether the cell containing `p` is passable for `class`.
    pub fn passable_at(&self, class: SizeClass, p: Vec2) -> bool {
        let (x, y) = self.cell_of(p);
        self.passable(class, x, y)
    }

    /// Whether the cell containing `p` is passable for unit type `profile`
    /// (its class, and bridges that carry its mass).
    pub fn passable_for(&self, profile: usize, p: Vec2) -> bool {
        let (x, y) = self.cell_of(p);
        self.profiles[profile].cost[self.index(x, y)] != 0
    }

    /// How restrictive a profile is: its size class, then its load tier
    /// (bridges it may not cross). A path planned for the most restrictive
    /// profile of a group fits every member.
    pub fn restriction(&self, profile: usize) -> (SizeClass, u32) {
        let p = &self.profiles[profile];
        (p.class, p.tier)
    }

    /// Speed on soft ground of profile `profile` in percent of the terrain
    /// speed, per terrain id (prompt 040); empty: 100 % everywhere.
    pub fn ground_pct(&self, profile: usize) -> &[u32] {
        &self.profiles[profile].ground_pct
    }

    /// Sets the soft-ground speeds of every profile (prompt 040, one list
    /// per profile, see [`Self::ground_pct`]). The costs follow only with
    /// [`Self::recost_rows`].
    pub fn set_ground_pct(&mut self, per_profile: Vec<Vec<u32>>) {
        for (profile, pct) in self.profiles.iter_mut().zip(per_profile) {
            profile.ground_pct = pct;
        }
    }

    /// Computes the costs of every profile again in the cell rows
    /// `y0..=y1` (clamped to the grid), after [`Self::set_ground_pct`].
    /// Passability and components do not change.
    pub fn recost_rows(&mut self, map: &Map, rules: &TerrainRules, y0: u32, y1: u32) {
        let y1 = y1.min(self.height - 1);
        if y0 > y1 {
            return;
        }
        self.version = self.version.wrapping_add(1);
        for p in 0..self.profiles.len() {
            let times = terrain_times(rules, &self.profiles[p]);
            for y in y0..=y1 {
                for x in 0..self.width {
                    let cost = self.cell_cost(map, &times, p, x, y);
                    let i = self.index(x, y);
                    self.profiles[p].cost[i] = cost;
                }
            }
        }
    }

    /// The bridge (index into [`crate::Map::bridges`]) whose intact deck
    /// covers the center of cell `(x, y)`.
    pub fn deck_of(&self, x: u32, y: u32) -> Option<usize> {
        self.deck[self.index(x, y)].checked_sub(1).map(usize::from)
    }

    /// Where a search for a unit at `p` starts (its lead-in point, which
    /// the unit drives to in a straight line first):
    ///
    /// - On an intact deck whose path cells do not contain `p` (a unit
    ///   pushed to the edge of a narrow deck stands in the cell beside it,
    ///   which is the ground or water below): the nearest point of the
    ///   deck's axis. A path from the cell beside would lead over the side
    ///   of the deck, which movement never allows.
    /// - On a cell blocked for `profile` (a unit pushed onto a bank or
    ///   beside a deck): the center of the nearest passable cell within
    ///   [`LEAD_IN_CELLS`] rings that the unit can drive to in a straight line
    ///   ([`drivable`]). A search from the blocked cell itself may lead
    ///   down a bank or over a railing the unit cannot take.
    /// - Otherwise (or if there is no such cell) `p` itself.
    pub fn lead_in(&self, map: &Map, rules: &TerrainRules, profile: usize, p: Vec2) -> Vec2 {
        let (x, y) = self.cell_of(p);
        if let Some((index, bridge)) = map
            .bridges()
            .iter()
            .enumerate()
            .find(|(_, b)| b.intact && b.covers(p))
        {
            if self.deck_of(x, y) == Some(index) {
                return p;
            }
            let (along, _) = bridge.local(p);
            let reach = (bridge.half_length - self.cell / 2).max(Fixed::ZERO);
            let axis = bridge.center + bridge.forward.scale(along.clamp(-reach, reach));
            let (ax, ay) = self.cell_of(axis);
            return if self.deck_of(ax, ay) == Some(index) {
                axis
            } else {
                p
            };
        }
        let cost = &self.profiles[profile].cost;
        if cost[self.index(x, y)] != 0 {
            return p;
        }
        let mut cells: Vec<(i128, u32)> = Vec::new();
        let rings = LEAD_IN_CELLS as i32;
        for dy in -rings..=rings {
            for dx in -rings..=rings {
                let Some((nx, ny)) = self.offset(x, y, dx, dy) else {
                    continue;
                };
                let n = self.index(nx, ny);
                if cost[n] == 0 {
                    continue;
                }
                let d = self.center(nx, ny) - p;
                let (ddx, ddy) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
                cells.push((ddx * ddx + ddy * ddy, n as u32));
            }
        }
        cells.sort_unstable();
        cells
            .into_iter()
            .map(|(_, n)| self.center(n % self.width, n / self.width))
            .find(|&c| drivable(map, rules, p, c))
            .unwrap_or(p)
    }

    /// The cells whose flags may depend on bridge `bridge` (its deck
    /// bounds grown by two cells).
    pub fn bridge_cells(&self, bridge: usize) -> Option<CellRect> {
        self.bridge_cells.get(bridge).copied()
    }

    const fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    const fn coords(&self, index: u32) -> (u32, u32) {
        (index % self.width, index / self.width)
    }

    /// Grid coordinates of a point (in cells, clamped into the grid).
    fn grid_point(&self, p: Vec2) -> (Fixed, Fixed) {
        let clamp = |v: Fixed, n: u32| v.clamp(Fixed::ZERO, Fixed::from_num(n) - Fixed::DELTA);
        (
            clamp((p.x - self.origin.x) / self.cell, self.width),
            clamp((p.y - self.origin.y) / self.cell, self.height),
        )
    }

    /// The cell containing `p` (points outside use the nearest edge cell).
    pub fn cell_of(&self, p: Vec2) -> (u32, u32) {
        let (gx, gy) = self.grid_point(p);
        (gx.to_num::<u32>(), gy.to_num::<u32>())
    }

    /// Center of a cell in map coordinates.
    pub fn center(&self, x: u32, y: u32) -> Vec2 {
        let half_cell = self.cell / 2;
        Vec2::new(
            self.origin.x + Fixed::from_num(x) * self.cell + half_cell,
            self.origin.y + Fixed::from_num(y) * self.cell + half_cell,
        )
    }

    /// Cells overlapped by the axis-aligned box `a`..`b` (any corner order).
    pub fn cells_in(&self, a: Vec2, b: Vec2) -> CellRect {
        let (ax, ay) = self.cell_of(a);
        let (bx, by) = self.cell_of(b);
        CellRect {
            x0: ax.min(bx),
            y0: ay.min(by),
            x1: ax.max(bx),
            y1: ay.max(by),
        }
    }

    /// Marks the cells of `rect` as blocked (or clears the mark) by an
    /// obstacle such as a building, and recomputes what depends on them.
    /// Returns the rectangle of cells whose passability or cost may have
    /// changed.
    pub fn set_obstacle(
        &mut self,
        map: &Map,
        rules: &TerrainRules,
        rect: CellRect,
        blocked: bool,
    ) -> CellRect {
        for y in rect.y0..=rect.y1 {
            for x in rect.x0..=rect.x1 {
                let i = self.index(x, y);
                if blocked {
                    self.base[i] |= BLOCKED_OBSTACLE;
                } else {
                    self.base[i] &= !BLOCKED_OBSTACLE;
                }
            }
        }
        self.rebuild(map, rules, rect)
    }

    /// Takes over bridge `bridge` of `map` after it was added or moved
    /// (bridges laid by bridge-laying tanks, prompt 055): its cells and
    /// railing direction, and the load tiers if its load limit is new
    /// (then every component grid is labeled again). Returns the cells
    /// whose flags may depend on it, before and after; the caller rebuilds
    /// them ([`Self::rebuild`]).
    pub fn sync_bridge(&mut self, map: &Map, bridge: usize) -> Option<CellRect> {
        let b = map.bridges().get(bridge)?;
        let two_cells = self.cell * 2;
        let (lo, hi) = b.bounds();
        let grow = Vec2::new(two_cells, two_cells);
        let rect = self.cells_in(lo - grow, hi + grow);
        let old = if bridge < self.bridge_cells.len() {
            let old = self.bridge_cells[bridge];
            self.bridge_cells[bridge] = rect;
            self.bridge_right[bridge] = b.right;
            Some(old)
        } else if bridge == self.bridge_cells.len() {
            self.bridge_cells.push(rect);
            self.bridge_right.push(b.right);
            None
        } else {
            return None;
        };
        self.retier(map);
        Some(old.map_or(rect, |o| CellRect {
            x0: o.x0.min(rect.x0),
            y0: o.y0.min(rect.y0),
            x1: o.x1.max(rect.x1),
            y1: o.y1.max(rect.y1),
        }))
    }

    /// Load tiers and component grids from the load limits of every bridge
    /// of `map`; labels every component grid again if they changed.
    fn retier(&mut self, map: &Map) {
        let mut loads: Vec<u32> = map.bridges().iter().map(|b| b.max_load_kg).collect();
        loads.sort_unstable();
        loads.dedup();
        let tier = |mass: u32| loads.iter().filter(|&&l| l < mass).count() as u32;
        if self.profiles.iter().all(|p| p.tier == tier(p.mass_kg)) {
            return;
        }
        let count = (self.width * self.height) as usize;
        let mut components: Vec<ComponentGrid> = Vec::new();
        for p in &mut self.profiles {
            p.tier = tier(p.mass_kg);
            let (class, tier) = (p.class, p.tier);
            p.components = components
                .iter()
                .position(|c| c.class == class && c.tier == tier)
                .unwrap_or_else(|| {
                    components.push(ComponentGrid {
                        class,
                        tier,
                        component: vec![0; count],
                        last_label: 0,
                    });
                    components.len() - 1
                });
        }
        self.components = components;
        self.version = self.version.wrapping_add(1);
        self.relabel_components();
    }

    /// Recomputes flags, slope, class passability, costs and components
    /// after the map samples or obstacles of the cells in `rect` changed.
    /// Returns the affected rectangle (`rect` and its neighbor cells, whose
    /// margin may have changed).
    pub fn rebuild(&mut self, map: &Map, rules: &TerrainRules, rect: CellRect) -> CellRect {
        let area = self.rebuild_cells(map, rules, rect);
        self.relabel_components_in(&[area]);
        area
    }

    /// Clears the map object flags of the cells in `rect` and marks them
    /// again from `objects` (position and shape, with the classes they block
    /// now), for objects that changed (destruction, prompt 020). Pass every
    /// object whose shape may overlap `rect`; cells outside `rect` are not
    /// touched. Call [`Self::rebuild_cells`] afterwards.
    pub fn rasterize_objects_in(
        &mut self,
        rect: CellRect,
        objects: impl IntoIterator<Item = (MapObject, Shape, Blocks)>,
    ) {
        for y in rect.y0..=rect.y1 {
            for x in rect.x0..=rect.x1 {
                let i = self.index(x, y);
                self.base[i] &= !(BLOCKED_OBJECT_SMALL | BLOCKED_OBJECT_LARGE);
            }
        }
        let half = self.cell / 2;
        for (object, shape, blocks) in objects {
            let flags = object_flags(blocks);
            if flags == 0 {
                continue;
            }
            let extent = object.half_extent(shape);
            let r = self.cells_in(object.position - extent, object.position + extent);
            for y in r.y0.max(rect.y0)..=r.y1.min(rect.y1) {
                for x in r.x0.max(rect.x0)..=r.x1.min(rect.x1) {
                    if object.overlaps_square(shape, self.center(x, y), half) {
                        let i = self.index(x, y);
                        self.base[i] |= flags;
                    }
                }
            }
        }
    }

    /// Clears the field work flags (prompt 035) of the cells in `rect` and
    /// marks them again from `cells` (cell coordinates with the classes a
    /// work there blocks; cells outside `rect` are skipped). Call
    /// [`Self::rebuild_cells`] afterwards.
    pub fn mark_works_in(
        &mut self,
        rect: CellRect,
        cells: impl IntoIterator<Item = ((u32, u32), Blocks)>,
    ) {
        for y in rect.y0..=rect.y1 {
            for x in rect.x0..=rect.x1 {
                let i = self.index(x, y);
                self.base[i] &= !(BLOCKED_WORK_SMALL | BLOCKED_WORK_LARGE);
            }
        }
        for ((x, y), blocks) in cells {
            if !rect.contains(x, y) {
                continue;
            }
            let i = self.index(x, y);
            if blocks.small() {
                self.base[i] |= BLOCKED_WORK_SMALL;
            }
            if blocks.large() {
                self.base[i] |= BLOCKED_WORK_LARGE;
            }
        }
    }

    /// Whether a field work blocks the cell for `class` (prompt 035).
    pub fn work_blocked(&self, class: SizeClass, x: u32, y: u32) -> bool {
        let flag = match class {
            SizeClass::Small => BLOCKED_WORK_SMALL,
            SizeClass::Large => BLOCKED_WORK_LARGE,
        };
        self.base[self.index(x, y)] & flag != 0
    }

    /// Row-major index of cell `(x, y)` (as in [`Path::cells`]).
    pub const fn cell_index(&self, x: u32, y: u32) -> u32 {
        y * self.width + x
    }

    /// Labels the components of the whole map again (after
    /// [`Self::rebuild_cells`]).
    pub fn relabel_components(&mut self) {
        for c in 0..self.components.len() {
            self.label_components(c);
        }
    }

    /// Updates the components after the cells of `areas` (as returned by
    /// [`Self::rebuild_cells`]) changed; cells outside them must be
    /// unchanged. Gives the same partition as [`Self::relabel_components`]
    /// (prompt 016): flood fills around the areas, within
    /// [`RELABEL_MARGIN`] cells, find the regions the changed cells belong
    /// to; a region whose pieces do not meet within the margin is split by
    /// [`resolve_split`]. Only when two regions merged (freed cells join
    /// them), the whole component grid is labeled again. A full labeling of
    /// a 4 km map takes about 8 ms per component grid, and every crushed
    /// bush and every wreck used to do it.
    pub fn relabel_components_in(&mut self, areas: &[CellRect]) {
        let cells: u64 = areas
            .iter()
            .map(|r| u64::from(r.x1 - r.x0 + 1) * u64::from(r.y1 - r.y0 + 1))
            .sum();
        let large = cells * 4 > u64::from(self.width) * u64::from(self.height);
        for c in 0..self.components.len() {
            if large || !self.relabel_local(c, areas) {
                self.label_components(c);
            }
        }
    }

    /// The local part of [`Self::relabel_components_in`] for component
    /// grid `c`; `false` (nothing changed) if the whole grid needs a new
    /// labeling (two regions merged).
    ///
    /// Why this is exact: let X be the changed cells (`areas`) and W the
    /// window (X grown by the margin). Every passable cell next to or in X
    /// starts a flood fill limited to W. Cells outside X keep their old
    /// labels and edges, so an old path between two cells outside X leaves
    /// and re-enters X only at cells next to X; if all such cells of one
    /// old region land in one fill, the region is still connected, and a
    /// fill that meets only one old label cannot connect two regions. A
    /// fill that meets no old label lies inside X and is a new region.
    fn relabel_local(&mut self, c: usize, areas: &[CellRect]) -> bool {
        let (width, height) = (self.width, self.height);
        let Some(profile) = self.profiles.iter().find(|p| p.components == c) else {
            return true;
        };
        let cost = &profile.cost;
        let (deck, rights) = (&self.deck, &self.bridge_right);
        let component = &self.components[c].component;
        let windows: Vec<CellRect> = areas
            .iter()
            .map(|r| r.grow(RELABEL_MARGIN, width, height))
            .collect();
        let Some(first) = windows.first() else {
            return true;
        };
        let bounds = windows.iter().fold(*first, |b, w| CellRect {
            x0: b.x0.min(w.x0),
            y0: b.y0.min(w.y0),
            x1: b.x1.max(w.x1),
            y1: b.y1.max(w.y1),
        });
        let in_x = |x: u32, y: u32| areas.iter().any(|r| r.contains(x, y));
        let in_w = |x: u32, y: u32| windows.iter().any(|r| r.contains(x, y));
        let bw = bounds.x1 - bounds.x0 + 1;
        let local = |x: u32, y: u32| ((y - bounds.y0) * bw + (x - bounds.x0)) as usize;
        // Fill number + 1 per cell of the bounds, 0 = not reached.
        let mut fill_of = vec![0u32; (bw * (bounds.y1 - bounds.y0 + 1)) as usize];
        // Per fill: the old label it met outside X (0: none yet).
        let mut met: Vec<u32> = Vec::new();
        // Cells of X per fill, and the others, as (fill, cell index).
        let mut inside: Vec<(u32, u32)> = Vec::new();
        let mut outside: Vec<(u32, u32)> = Vec::new();
        let mut stack: Vec<u32> = Vec::new();
        for area in areas {
            let seeds = area.grow(1, width, height);
            for sy in seeds.y0..=seeds.y1 {
                for sx in seeds.x0..=seeds.x1 {
                    let seed = sy * width + sx;
                    if cost[seed as usize] == 0 || fill_of[local(sx, sy)] != 0 {
                        continue;
                    }
                    let fill = met.len() as u32;
                    met.push(0);
                    fill_of[local(sx, sy)] = fill + 1;
                    stack.push(seed);
                    while let Some(cell) = stack.pop() {
                        let (x, y) = (cell % width, cell / width);
                        if in_x(x, y) {
                            inside.push((fill, cell));
                        } else {
                            outside.push((fill, cell));
                            let old = component[cell as usize];
                            let seen = &mut met[fill as usize];
                            if *seen == 0 {
                                *seen = old;
                            } else if *seen != old {
                                // Two regions merged.
                                return false;
                            }
                        }
                        let mut visit = |nx: u32, ny: u32| {
                            let n = ny * width + nx;
                            if in_w(nx, ny)
                                && cost[n as usize] != 0
                                && fill_of[local(nx, ny)] == 0
                                && !railing(deck, rights, width, cell as usize, n as usize)
                            {
                                fill_of[local(nx, ny)] = fill + 1;
                                stack.push(n);
                            }
                        };
                        if x + 1 < width {
                            visit(x + 1, y);
                        }
                        if x > 0 {
                            visit(x - 1, y);
                        }
                        if y + 1 < height {
                            visit(x, y + 1);
                        }
                        if y > 0 {
                            visit(x, y - 1);
                        }
                    }
                }
            }
        }
        // An old label met by several fills: the region may have split,
        // or the fills still connect outside the window.
        let mut by_label: Vec<(u32, u32)> = (0u32..)
            .zip(&met)
            .filter(|&(_, &l)| l != 0)
            .map(|(f, &l)| (l, f))
            .collect();
        by_label.sort_unstable();
        // Per fill: the region it belongs to after a split (`None`: it
        // keeps its old label); per new region: all its cells.
        let mut part_of: Vec<Option<usize>> = vec![None; met.len()];
        let mut parts: Vec<Vec<u32>> = Vec::new();
        for same in by_label.chunk_by(|a, b| a.0 == b.0) {
            if same.len() < 2 {
                continue;
            }
            let fills: Vec<Vec<u32>> = same
                .iter()
                .map(|&(_, f)| {
                    inside
                        .iter()
                        .chain(&outside)
                        .filter(|&&(g, _)| g == f)
                        .map(|&(_, cell)| cell)
                        .collect()
                })
                .collect();
            let (of_fill, new_parts) = resolve_split(cost, deck, rights, width, height, &fills);
            for (&(_, f), part) in same.iter().zip(of_fill) {
                part_of[f as usize] = part.map(|p| p + parts.len());
            }
            parts.extend(new_parts);
        }
        let grid = &mut self.components[c];
        for area in areas {
            for y in area.y0..=area.y1 {
                for x in area.x0..=area.x1 {
                    grid.component[(y * width + x) as usize] = 0;
                }
            }
        }
        let fresh: Vec<u32> = parts
            .iter()
            .map(|_| {
                grid.last_label += 1;
                grid.last_label
            })
            .collect();
        for (fill, label) in met.iter_mut().enumerate() {
            if let Some(p) = part_of[fill] {
                *label = fresh[p];
            } else if *label == 0 {
                grid.last_label += 1;
                *label = grid.last_label;
            }
        }
        for &(fill, cell) in &inside {
            grid.component[cell as usize] = met[fill as usize];
        }
        for (cells, &label) in parts.iter().zip(&fresh) {
            for &cell in cells {
                grid.component[cell as usize] = label;
            }
        }
        true
    }

    /// [`Self::rebuild`] without the components: several areas can be
    /// rebuilt before one [`Self::relabel_components_in`].
    pub fn rebuild_cells(&mut self, map: &Map, rules: &TerrainRules, rect: CellRect) -> CellRect {
        self.version = self.version.wrapping_add(1);
        let area = rect.grow(1, self.width, self.height);
        for y in area.y0..=area.y1 {
            for x in area.x0..=area.x1 {
                self.rebuild_base(map, rules, x, y);
                let i = self.index(x, y);
                for class in SizeClass::ALL {
                    self.classes[class.index()].passable[i] = self.base[i] & class.blocking() == 0;
                }
            }
        }
        for p in 0..self.profiles.len() {
            let times = terrain_times(rules, &self.profiles[p]);
            for y in area.y0..=area.y1 {
                for x in area.x0..=area.x1 {
                    let cost = self.cell_cost(map, &times, p, x, y);
                    let i = self.index(x, y);
                    self.profiles[p].cost[i] = cost;
                }
            }
        }
        area
    }

    /// Flags and slope of one cell from its map samples.
    fn rebuild_base(&mut self, map: &Map, rules: &TerrainRules, x: u32, y: u32) {
        let (sx, sy) = map.samples();
        let k = self.stride;
        let (i0, j0) = (x * k, y * k);
        let i = self.index(x, y);
        let near_bridge = self.bridge_cells.iter().any(|r| r.contains(x, y));
        // Samples under an intact deck do not count as impassable (the deck
        // bridges them), so cells next to a deck get no margin from them.
        let impassable = |si: u32, sj: u32| {
            si >= sx
                || sj >= sy
                || (rules
                    .types
                    .get(usize::from(map.sample_terrain(si, sj)))
                    .is_none_or(|t| !t.passable)
                    && !(near_bridge && map.sample_under_deck(si, sj)))
        };
        let mut flags = self.base[i] & PLACED_FLAGS;
        if (j0..j0 + k).any(|j| (i0..i0 + k).any(|si| impassable(si, j))) {
            flags |= BLOCKED_TERRAIN;
        }
        // The ring of samples around the cell; beyond the map edge counts
        // as impassable.
        let ring = |a: u32, n: u32| a.checked_sub(1).into_iter().chain([a + k]).any(|v| v >= n);
        let edge = ring(i0, sx) || ring(j0, sy) || i0 == 0 || j0 == 0;
        let margin = edge
            || (j0 - 1..=j0 + k).any(|j| {
                (i0 - 1..=i0 + k).any(|si| {
                    let inside = (j0..j0 + k).contains(&j) && (i0..i0 + k).contains(&si);
                    !inside && impassable(si, j)
                })
            });
        if margin {
            flags |= BLOCKED_MARGIN;
        }
        // Largest height difference per map cell over the height patches
        // the cell overlaps: `sum` is |dx| + |dy| (an upper bound of the
        // gradient, used for blocking, so the movement rule never meets a
        // grade the grid let through); `octagonal` approximates the gradient
        // length within 12 % (cost).
        let (mut sum, mut octagonal) = (0i64, 0i64);
        let h = |a: u32, b: u32| i64::from(map.sample_height_mm(a, b));
        for j in j0.saturating_sub(1)..j0 + k {
            for si in i0.saturating_sub(1)..i0 + k {
                let gx = (h(si + 1, j) - h(si, j))
                    .abs()
                    .max((h(si + 1, j + 1) - h(si, j + 1)).abs());
                let gy = (h(si, j + 1) - h(si, j))
                    .abs()
                    .max((h(si + 1, j + 1) - h(si + 1, j)).abs());
                sum = sum.max(gx + gy);
                octagonal = octagonal.max(gx.max(gy) + gx.min(gy) / 2);
            }
        }
        let cell_mm = i64::from(self.map_cell_mm);
        if 100 * sum > i64::from(rules.slope.max_grade_pct) * cell_mm {
            flags |= BLOCKED_STEEP;
        }
        let mut grade = Fixed::from_num(octagonal) / Fixed::from_num(cell_mm);
        // A cell whose center lies on an intact deck is the deck: passable
        // at the deck's grade, whatever lies below.
        self.deck[i] = 0;
        if near_bridge {
            let center = self.center(x, y);
            if let Some(b) = map
                .bridges()
                .iter()
                .position(|b| b.intact && b.covers(center))
            {
                let bridge = &map.bridges()[b];
                flags &= !(BLOCKED_TERRAIN | BLOCKED_STEEP | BLOCKED_MARGIN);
                grade = (bridge.height_end - bridge.height_start).abs() / (bridge.half_length * 2);
                self.deck[i] = u16::try_from(b + 1).unwrap_or(0);
            }
        }
        self.base[i] = flags;
        // Driving up costs 1 / factor, down 1: average both directions.
        let factor = rules
            .slope
            .factor(grade)
            .unwrap_or(Fixed::from_num(rules.slope.min_speed_pct) / Fixed::from_num(100));
        let pml = (Fixed::from_num(1000) + Fixed::from_num(1000) / factor) / 2;
        self.slope_pml[i] = pml.to_num::<u32>().min(u32::from(u16::MAX)) as u16;
    }

    /// Cost of a cell for profile `p`: mean crossing time over its samples,
    /// relative to the fastest terrain, times the slope factor. 0 if blocked
    /// for the profile's class.
    fn cell_cost(&self, map: &Map, times: &[Fixed], p: usize, x: u32, y: u32) -> u16 {
        let i = self.index(x, y);
        if !self.classes[self.profiles[p].class.index()].passable[i] {
            return 0;
        }
        let mean = if let Some(b) = self.deck_of(x, y) {
            // On a deck: its terrain, if it carries the unit at all.
            let bridge = &map.bridges()[b];
            if bridge.max_load_kg < self.profiles[p].mass_kg {
                return 0;
            }
            times
                .get(usize::from(bridge.terrain))
                .copied()
                .unwrap_or(Fixed::ONE)
        } else {
            let k = self.stride;
            let (i0, j0) = (x * k, y * k);
            let mut sum = Fixed::ZERO;
            for j in j0..j0 + k {
                for si in i0..i0 + k {
                    let t = usize::from(map.sample_terrain(si, j));
                    sum += times.get(t).copied().unwrap_or(Fixed::ONE);
                }
            }
            sum / Fixed::from_num(k * k)
        };
        let cost = mean * Fixed::from_num(MIN_CELL_COST) * Fixed::from_num(self.slope_pml[i])
            / Fixed::from_num(1000);
        cost.to_num::<u32>()
            .clamp(u32::from(MIN_CELL_COST), u32::from(u16::MAX)) as u16
    }

    /// Labels the connected regions of component grid `c` from the costs of
    /// a profile that uses it (all of them block the same cells).
    fn label_components(&mut self, c: usize) {
        let width = self.width;
        let height = self.height;
        let Some(profile) = self.profiles.iter().find(|p| p.components == c) else {
            return;
        };
        let cost = &profile.cost;
        let (deck, rights) = (&self.deck, &self.bridge_right);
        let grid = &mut self.components[c];
        grid.component.fill(0);
        let mut next = 0u32;
        let mut stack = Vec::new();
        for start in 0..cost.len() {
            if cost[start] == 0 || grid.component[start] != 0 {
                continue;
            }
            next += 1;
            grid.component[start] = next;
            stack.push(start as u32);
            while let Some(c) = stack.pop() {
                let (x, y) = (c % width, c / width);
                let mut visit = |nx: u32, ny: u32| {
                    let n = (ny * width + nx) as usize;
                    if cost[n] != 0
                        && grid.component[n] == 0
                        && !railing(deck, rights, width, c as usize, n)
                    {
                        grid.component[n] = next;
                        stack.push(n as u32);
                    }
                };
                if x + 1 < width {
                    visit(x + 1, y);
                }
                if x > 0 {
                    visit(x - 1, y);
                }
                if y + 1 < height {
                    visit(x, y + 1);
                }
                if y > 0 {
                    visit(x, y - 1);
                }
            }
        }
        grid.last_label = next;
    }

    /// Visits every cell the segment `a`..`b` touches, from `a` to `b`, and
    /// returns `false` as soon as `ok` rejects one (the start cell is not
    /// checked). Where the segment passes exactly through a cell corner,
    /// both side cells must be accepted. Integer-exact (cross products of
    /// the raw fixed-point bits).
    fn walk_line(&self, a: Vec2, b: Vec2, mut ok: impl FnMut(usize) -> bool) -> bool {
        let (ax, ay) = self.grid_point(a);
        let (bx, by) = self.grid_point(b);
        let (mut cx, mut cy) = (ax.to_num::<i64>(), ay.to_num::<i64>());
        let (ex, ey) = (bx.to_num::<i64>(), by.to_num::<i64>());
        let (dx, dy) = (bx - ax, by - ay);
        let step_x: i64 = if dx > 0 { 1 } else { -1 };
        let step_y: i64 = if dy > 0 { 1 } else { -1 };
        let abs_dx = i128::from(dx.abs().to_bits());
        let abs_dy = i128::from(dy.abs().to_bits());
        let one = Fixed::ONE.to_bits();
        let index = |x: i64, y: i64| (y as u32 * self.width + x as u32) as usize;
        let mut prev = index(cx, cy);
        let mut check = |from: usize, x: i64, y: i64| {
            let i = index(x, y);
            !railing(&self.deck, &self.bridge_right, self.width, from, i) && ok(i)
        };
        let max_steps = (ex - cx).abs() + (ey - cy).abs();
        for _ in 0..max_steps {
            if (cx, cy) == (ex, ey) {
                break;
            }
            // Distance (in grid units, raw bits) to the next boundary on
            // each axis; the axis reached first (smaller distance / delta)
            // is stepped.
            let next_x = if step_x > 0 { (cx + 1) * one } else { cx * one };
            let next_y = if step_y > 0 { (cy + 1) * one } else { cy * one };
            let to_x = i128::from((next_x - ax.to_bits()).abs());
            let to_y = i128::from((next_y - ay.to_bits()).abs());
            let order = if cx == ex {
                std::cmp::Ordering::Greater
            } else if cy == ey {
                std::cmp::Ordering::Less
            } else {
                (to_x * abs_dy).cmp(&(to_y * abs_dx))
            };
            match order {
                std::cmp::Ordering::Less => cx += step_x,
                std::cmp::Ordering::Greater => cy += step_y,
                std::cmp::Ordering::Equal => {
                    if !check(prev, cx + step_x, cy) || !check(prev, cx, cy + step_y) {
                        return false;
                    }
                    cx += step_x;
                    cy += step_y;
                }
            }
            if !check(prev, cx, cy) {
                return false;
            }
            prev = index(cx, cy);
        }
        true
    }

    /// Whether a unit of `class` can drive straight from `a` to `b`: every
    /// touched cell except the start cell is passable.
    pub fn line_of_sight(&self, class: SizeClass, a: Vec2, b: Vec2) -> bool {
        let passable = &self.classes[class.index()].passable;
        self.walk_line(a, b, |i| passable[i])
    }

    /// Whether unit type `profile` can drive straight from `a` to `b`
    /// (like [`PathGrid::line_of_sight`], with the bridges it may cross).
    pub fn line_of_sight_for(&self, profile: usize, a: Vec2, b: Vec2) -> bool {
        let cost = &self.profiles[profile].cost;
        self.walk_line(a, b, |i| cost[i] != 0)
    }

    /// Estimated cost of driving straight from `a` to `b` with profile `p`
    /// (mean cost of the touched cells times the length, in the units of
    /// the search), or `None` if a touched cell (except the start cell) is
    /// blocked.
    fn line_cost(&self, p: usize, a: Vec2, b: Vec2, avoid: &[u32]) -> Option<Fixed> {
        let cost = &self.profiles[p].cost;
        let (sx, sy) = self.cell_of(a);
        let start = self.index(sx, sy);
        let mut sum = u64::from(cost[start].max(MIN_CELL_COST)) * u64::from(avoided(avoid, start));
        let mut count = 1u64;
        let clear = self.walk_line(a, b, |i| {
            sum += u64::from(cost[i]) * u64::from(avoided(avoid, i));
            count += 1;
            cost[i] != 0
        });
        if !clear {
            return None;
        }
        let mean = Fixed::from_num(sum) / Fixed::from_num(count);
        Some(mean * (b - a).length() / self.cell * Fixed::from_num(10))
    }

    /// Start cell and component of a search from cell `(x, y)`: the cell
    /// itself if passable; for a blocked cell (a unit standing in the
    /// margin) the cell itself with the first passable neighbor's
    /// component (not across a railing: a unit beside a deck, below it on
    /// the bank, cannot climb over the side); with no such neighbor, the
    /// nearest passable cell off any deck within [`START_SEARCH_CELLS`]
    /// rings (the unit drives there in a straight line). `None` if there is
    /// none.
    fn search_start(&self, profile: usize, x: u32, y: u32, start: Vec2) -> Option<(u32, u32)> {
        let grid = &self.components[self.profiles[profile].components];
        let cell = self.index(x, y) as u32;
        let own = grid.component[cell as usize];
        if own != 0 {
            return Some((cell, own));
        }
        let neighbor = NEIGHBORS
            .iter()
            .filter_map(|&(dx, dy, _)| self.offset(x, y, dx, dy))
            .map(|(nx, ny)| self.index(nx, ny))
            .filter(|&n| !railing(&self.deck, &self.bridge_right, self.width, cell as usize, n))
            .map(|n| grid.component[n])
            .find(|&c| c != 0);
        if let Some(component) = neighbor {
            return Some((cell, component));
        }
        let nearest = self.nearest_cell(profile, None, start, START_SEARCH_CELLS, true)?;
        Some((nearest, grid.component[nearest as usize]))
    }

    fn offset(&self, x: u32, y: u32, dx: i32, dy: i32) -> Option<(u32, u32)> {
        let nx = x.checked_add_signed(dx)?;
        let ny = y.checked_add_signed(dy)?;
        (nx < self.width && ny < self.height).then_some((nx, ny))
    }

    /// The passable cell (of `component`, if given) whose center is nearest
    /// to `goal` (ties: the smaller index), searched ring by ring around the
    /// goal cell up to `max_rings`; with `off_decks` no cell of a deck.
    fn nearest_cell(
        &self,
        profile: usize,
        component: Option<u32>,
        goal: Vec2,
        max_rings: u32,
        off_decks: bool,
    ) -> Option<u32> {
        let grid = &self.components[self.profiles[profile].components];
        let (gx, gy) = self.cell_of(goal);
        let rings = self.width.max(self.height).min(max_rings.saturating_add(1));
        let mut best: Option<(i128, u32)> = None;
        for r in 0..rings {
            // Every cell on ring r is at least (r - 1/2) cells from the goal,
            // so once that exceeds the best distance, no ring can do better.
            if let Some((d, _)) = best {
                let bound = i128::from((Fixed::from_num(r) * self.cell - self.cell / 2).to_bits());
                if bound > 0 && bound * bound > d {
                    break;
                }
            }
            let x0 = gx.saturating_sub(r);
            let x1 = (gx + r).min(self.width - 1);
            let y0 = gy.saturating_sub(r);
            let y1 = (gy + r).min(self.height - 1);
            let mut visit = |x: u32, y: u32| {
                let i = self.index(x, y);
                let c = grid.component[i];
                if c == 0
                    || component.is_some_and(|wanted| c != wanted)
                    || (off_decks && self.deck[i] != 0)
                {
                    return;
                }
                let d = self.center(x, y) - goal;
                let (ddx, ddy) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
                let key = (ddx * ddx + ddy * ddy, i as u32);
                if best.is_none_or(|b| key < b) {
                    best = Some(key);
                }
            };
            // Only the cells on ring r (rows and columns at distance r that
            // lie inside the grid). The minimum over (distance, index) does
            // not depend on the visiting order.
            for y in y0..=y1 {
                if y.abs_diff(gy) == r {
                    for x in x0..=x1 {
                        visit(x, y);
                    }
                } else {
                    for x in [gx.checked_sub(r), Some(gx + r)].into_iter().flatten() {
                        if x <= x1 {
                            visit(x, y);
                        }
                    }
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// The center of the passable cell nearest to `near` (within `radius`
    /// rings of cells) that unit type `profile` can reach in a straight
    /// line from `from`; ties go to the smaller cell index.
    pub fn nearest_visible(
        &self,
        profile: usize,
        from: Vec2,
        near: Vec2,
        radius: u32,
    ) -> Option<Vec2> {
        let (nx, ny) = self.cell_of(near);
        let cost = &self.profiles[profile].cost;
        let mut best: Option<(i128, usize, Vec2)> = None;
        for y in ny.saturating_sub(radius)..=(ny + radius).min(self.height - 1) {
            for x in nx.saturating_sub(radius)..=(nx + radius).min(self.width - 1) {
                let i = self.index(x, y);
                if cost[i] == 0 {
                    continue;
                }
                let center = self.center(x, y);
                let d = center - near;
                let (dx, dy) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
                let key = dx * dx + dy * dy;
                if best.is_some_and(|(k, b, _)| (k, b) <= (key, i)) {
                    continue;
                }
                if self.line_of_sight_for(profile, from, center) {
                    best = Some((key, i, center));
                }
            }
        }
        best.map(|(_, _, p)| p)
    }

    /// Finds a path in one go (no budget). For tests and tools; the
    /// simulation uses [`Pathfinder`].
    pub fn find_path(&self, start: Vec2, goal: Vec2, profile: usize) -> Option<Path> {
        self.find_path_avoiding(start, goal, profile, Vec::new())
    }

    /// [`Self::find_path`] that avoids the cells `avoid` (sorted cell
    /// indices) like the simulation's searches for a player with known
    /// minefields (prompt 035): they cost [`AVOID_FACTOR`] times as much.
    pub fn find_path_avoiding(
        &self,
        start: Vec2,
        goal: Vec2,
        profile: usize,
        avoid: Vec<u32>,
    ) -> Option<Path> {
        let mut workspace = Workspace::default();
        let search = match Search::begin(self, &mut workspace, start, goal, profile, avoid) {
            Begin::Done(result) => return result,
            Begin::Search(search) => search,
        };
        let mut budget = u32::MAX;
        loop {
            if let Some(result) = search.run(self, &mut workspace, &mut budget) {
                return result;
            }
        }
    }

    /// Shortens a raw cell path by line of sight: from each waypoint, the
    /// farthest later path point (within [`SMOOTH_WINDOW`]) that can be
    /// reached in a straight line over passable cells at no more cost than
    /// the skipped stretch of the raw path. Keeps units on roads and off
    /// expensive ground the search avoided.
    fn smooth(&self, p: usize, cells: &[u32], start: Vec2, goal: Vec2, avoid: &[u32]) -> Vec<Vec2> {
        let n = cells.len() - 1;
        let cost = &self.profiles[p].cost;
        let point = |i: usize| {
            if i == 0 {
                start
            } else if i == n {
                goal
            } else {
                let (x, y) = self.coords(cells[i]);
                self.center(x, y)
            }
        };
        // Cost of the raw path up to every point, measured like a straight
        // line (mean cost of both cells times the length), so a shortcut is
        // compared with the same yardstick.
        let mut prefix = Vec::with_capacity(cells.len());
        prefix.push(Fixed::ZERO);
        for m in 0..n {
            let weight = |c: u32| {
                u32::from(cost[c as usize].max(MIN_CELL_COST)) * avoided(avoid, c as usize)
            };
            let (a, b) = (weight(cells[m]), weight(cells[m + 1]));
            let mean = Fixed::from_num(a + b) / 2;
            let length = (point(m + 1) - point(m)).length() / self.cell;
            prefix.push(prefix[m] + mean * length * Fixed::from_num(10));
        }
        let mut out = vec![start];
        let mut i = 0;
        while i < n {
            // The next raw point is always reachable (it is a grid step).
            let mut best = i + 1;
            for j in i + 2..=n.min(i + SMOOTH_WINDOW) {
                let stretch = prefix[j] - prefix[i];
                if self
                    .line_cost(p, point(i), point(j), avoid)
                    .is_none_or(|line| line > stretch)
                {
                    break;
                }
                best = j;
            }
            out.push(point(best));
            i = best;
        }
        out
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.width);
        h.write_u32(self.height);
        h.write_u32(self.version);
    }
}

/// Whether a unit can drive from `a` to `b` in a straight line: every
/// half meter on passable terrain, within the grade limit and not over the
/// side of a deck ([`Map::deck_step_ok`]), like the movement rules.
pub(crate) fn drivable(map: &Map, rules: &TerrainRules, a: Vec2, b: Vec2) -> bool {
    let length = (b - a).length();
    let steps = (length * 2).ceil().to_num::<u32>().max(1);
    let max_grade = Fixed::from_num(rules.slope.max_grade_pct) / 100;
    let step_length = length / Fixed::from_num(steps);
    let mut from = a;
    (1..=steps).all(|k| {
        let to = a + (b - a).scale(Fixed::from_num(k) / Fixed::from_num(steps));
        let ok = map.contains(to)
            && rules.types[usize::from(map.terrain_at(to))].passable
            && map.deck_step_ok(from, to)
            && (map.height_at(to) - map.height_at(from)).abs() <= step_length * max_grade;
        from = to;
        ok
    })
}

/// Cost factor of cell `i` for a search that avoids `avoid` (sorted).
fn avoided(avoid: &[u32], i: usize) -> u32 {
    if !avoid.is_empty() && u32::try_from(i).is_ok_and(|i| avoid.binary_search(&i).is_ok()) {
        AVOID_FACTOR
    } else {
        1
    }
}

/// Object flags of a blocking class.
const fn object_flags(blocks: Blocks) -> u8 {
    let mut flags = 0;
    if blocks.small() {
        flags |= BLOCKED_OBJECT_SMALL;
    }
    if blocks.large() {
        flags |= BLOCKED_OBJECT_LARGE;
    }
    flags
}

/// Splits one old region after cells were blocked: `fills` are its pieces
/// around the changed cells (connected within the window, not yet known to
/// be connected outside it). Flood fills grow from all pieces in turn; two
/// that meet are one region. As soon as at most one region still grows,
/// all others are complete. Costs about as much as the regions that broke
/// off (pockets behind wrecks), not the whole map.
///
/// Returns per fill `None` if it stays in the region that keeps the old
/// label, else the index of its new region, and per new region its cells.
fn resolve_split(
    cost: &[u16],
    deck: &[u16],
    rights: &[Vec2],
    width: u32,
    height: u32,
    fills: &[Vec<u32>],
) -> (Vec<Option<usize>>, Vec<Vec<u32>>) {
    fn root(parent: &mut [usize], mut g: usize) -> usize {
        while parent[g] != g {
            parent[g] = parent[parent[g]];
            g = parent[g];
        }
        g
    }
    let k = fills.len();
    // Group + 1 per cell, 0 = not reached.
    let mut owner = vec![0u32; (width * height) as usize];
    let mut parent: Vec<usize> = (0..k).collect();
    let mut cells: Vec<Vec<u32>> = fills.to_vec();
    let mut frontier: Vec<VecDeque<u32>> =
        fills.iter().map(|f| f.iter().copied().collect()).collect();
    for (g, fill) in (1u32..).zip(fills) {
        for &cell in fill {
            owner[cell as usize] = g;
        }
    }
    loop {
        let growing: Vec<usize> = (0..k)
            .filter(|&g| parent[g] == g && !frontier[g].is_empty())
            .collect();
        if growing.len() <= 1 {
            break;
        }
        for g in growing {
            if parent[g] != g {
                continue;
            }
            let Some(cell) = frontier[g].pop_front() else {
                continue;
            };
            let (x, y) = (cell % width, cell / width);
            let mut neighbors = [None; 4];
            if x + 1 < width {
                neighbors[0] = Some(cell + 1);
            }
            if x > 0 {
                neighbors[1] = Some(cell - 1);
            }
            if y + 1 < height {
                neighbors[2] = Some(cell + width);
            }
            if y > 0 {
                neighbors[3] = Some(cell - width);
            }
            for n in neighbors.into_iter().flatten() {
                if cost[n as usize] == 0 || railing(deck, rights, width, cell as usize, n as usize)
                {
                    continue;
                }
                match owner[n as usize] {
                    0 => {
                        owner[n as usize] = g as u32 + 1;
                        cells[g].push(n);
                        frontier[g].push_back(n);
                    }
                    o => {
                        let other = root(&mut parent, o as usize - 1);
                        if other != g {
                            parent[other] = g;
                            let moved = std::mem::take(&mut cells[other]);
                            cells[g].extend(moved);
                            let moved = std::mem::take(&mut frontier[other]);
                            frontier[g].extend(moved);
                        }
                    }
                }
            }
        }
    }
    // The region still growing keeps the old label; if all are complete,
    // the largest keeps it (fewest cells to write).
    let roots: Vec<usize> = (0..k).filter(|&g| parent[g] == g).collect();
    let keeper = roots
        .iter()
        .copied()
        .find(|&g| !frontier[g].is_empty())
        .or_else(|| {
            roots
                .iter()
                .copied()
                .max_by_key(|&g| (cells[g].len(), usize::MAX - g))
        });
    let mut index = vec![None; k];
    let mut parts = Vec::new();
    for &g in &roots {
        if Some(g) != keeper {
            index[g] = Some(parts.len());
            parts.push(std::mem::take(&mut cells[g]));
        }
    }
    let of_fill = (0..k).map(|f| index[root(&mut parent, f)]).collect();
    (of_fill, parts)
}

/// Whether a step between neighboring cells `a` and `b` crosses the
/// railing of a bridge: one of them is a deck cell and the other is not a
/// cell of the same deck, and the step is not along the deck's axis. Units
/// get on and off a deck only over its ends ([`crate::Map::deck_step_ok`]).
///
/// The common case (neither cell on a deck) is inlined into the searches,
/// which ask for every neighbor (prompt 016).
#[inline]
fn railing(deck: &[u16], rights: &[Vec2], width: u32, a: usize, b: usize) -> bool {
    deck[a] != deck[b] && railing_crossed(deck[a], deck[b], rights, width, a, b)
}

/// [`railing`] for two cells with different decks.
#[inline(never)]
fn railing_crossed(da: u16, db: u16, rights: &[Vec2], width: u32, a: usize, b: usize) -> bool {
    let (dx, dy) = (
        i64::from(b as u32 % width) - i64::from(a as u32 % width),
        i64::from(b as u32 / width) - i64::from(a as u32 / width),
    );
    let half = Fixed::ONE / 2;
    [da, db].into_iter().filter(|&d| d != 0).any(|d| {
        rights
            .get(usize::from(d - 1))
            .is_some_and(|r| (Fixed::from_num(dx) * r.x + Fixed::from_num(dy) * r.y).abs() > half)
    })
}

/// Cost per cell the A* heuristic assumes for a unit type, see
/// [`HEURISTIC_WEIGHT_PCT`].
fn heuristic_cost(t: &UnitType) -> u16 {
    let off = t.speed_m_per_s;
    let fastest = t.road_speed_m_per_s.max(off);
    let plain = if off > Fixed::ZERO {
        fastest / off * Fixed::from_num(MIN_CELL_COST)
    } else {
        Fixed::from_num(MIN_CELL_COST)
    };
    let min = Fixed::from_num(MIN_CELL_COST);
    let blend = min + (plain - min) * Fixed::from_num(HEURISTIC_WEIGHT_PCT) / 100;
    blend
        .to_num::<u32>()
        .clamp(u32::from(MIN_CELL_COST), u32::from(u16::MAX)) as u16
}

/// Crossing time per meter of every terrain type for a profile, relative to
/// its fastest passable terrain (so the fastest is exactly 1).
fn terrain_times(rules: &TerrainRules, profile: &Profile) -> Vec<Fixed> {
    let speeds: Vec<Fixed> = rules
        .types
        .iter()
        .enumerate()
        .map(|(id, t)| {
            let speed = t.speed(profile.offroad, profile.road);
            match profile.ground_pct.get(id) {
                Some(&pct) if pct != 100 => speed * Fixed::from_num(pct) / 100,
                _ => speed,
            }
        })
        .collect();
    let fastest = rules
        .types
        .iter()
        .zip(&speeds)
        .filter(|(t, _)| t.passable)
        .map(|(_, &s)| s)
        .max()
        .unwrap_or(Fixed::ZERO);
    // A terrain where the unit cannot move at all is avoided as far as the
    // u16 costs allow.
    let slowest = Fixed::from_num(u16::MAX / MIN_CELL_COST);
    speeds
        .iter()
        .map(|&s| {
            if s <= Fixed::ZERO || fastest <= Fixed::ZERO {
                slowest
            } else {
                (fastest / s).min(slowest)
            }
        })
        .collect()
}

/// Neighbor offsets in search order, with the step length (10 straight,
/// 14 diagonal).
const NEIGHBORS: [(i32, i32, u32); 8] = [
    (1, 0, 10),
    (0, 1, 10),
    (-1, 0, 10),
    (0, -1, 10),
    (1, 1, 14),
    (-1, 1, 14),
    (-1, -1, 14),
    (1, -1, 14),
];

/// Octile distance between two cells times `cost` per cell.
fn heuristic(cost: u16, ax: u32, ay: u32, bx: u32, by: u32) -> u32 {
    let dx = ax.abs_diff(bx);
    let dy = ay.abs_diff(by);
    let (long, short) = (dx.max(dy), dx.min(dy));
    u32::from(cost).saturating_mul(10 * long + 4 * short)
}

/// Search state of one cell. Valid only if `mark` belongs to the current
/// generation: `2 * generation` = seen, `2 * generation + 1` = closed.
/// One array of these keeps a neighbor's state in one cache line.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Node {
    g: u32,
    parent: u32,
    mark: u32,
}

/// Search arrays, reused by all searches of a [`Pathfinder`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Workspace {
    nodes: Vec<Node>,
    generation: u32,
    /// Open list. The key `(f, h, cell)` is a total order, so the pop order
    /// does not depend on how the heap arranges equal costs.
    open: BinaryHeap<Reverse<(u32, u32, u32)>>,
}

impl Workspace {
    fn reset(&mut self, cells: usize) {
        if self.nodes.len() != cells || self.generation >= u32::MAX / 2 - 1 {
            self.nodes = vec![Node::default(); cells];
            self.generation = 0;
        }
        self.generation += 1;
        self.open.clear();
    }
}

/// A found path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// Raw path as cell indices (row-major), start cell first.
    pub cells: Vec<u32>,
    /// Smoothed waypoints: the start point first, the goal point last.
    pub waypoints: Vec<Vec2>,
    /// Sum of the step costs of the raw path.
    pub cost: u32,
    /// The goal that was used: the requested one, or the center of the
    /// nearest reachable cell if that was not reachable.
    pub goal: Vec2,
}

impl Path {
    /// Length of the smoothed path in meters.
    pub fn length(&self) -> Fixed {
        self.waypoints
            .windows(2)
            .map(|w| (w[1] - w[0]).length())
            .sum()
    }
}

enum Begin {
    Done(Option<Path>),
    Search(Search),
}

/// A resumable A* search.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Search {
    profile: usize,
    start: Vec2,
    goal: Vec2,
    start_cell: u32,
    goal_cell: u32,
    /// Cells to avoid (sorted indices, prompt 035).
    #[serde(default)]
    avoid: Vec<u32>,
}

impl Search {
    fn begin(
        grid: &PathGrid,
        ws: &mut Workspace,
        start: Vec2,
        goal: Vec2,
        profile: usize,
        avoid: Vec<u32>,
    ) -> Begin {
        let (sx, sy) = grid.cell_of(start);
        let Some((start_cell, component)) = grid.search_start(profile, sx, sy, start) else {
            return Begin::Done(None);
        };
        let (gx, gy) = grid.cell_of(goal);
        let mut goal_cell = grid.index(gx, gy) as u32;
        let mut goal = goal;
        let components = &grid.components[grid.profiles[profile].components].component;
        if components[goal_cell as usize] != component {
            let Some(nearest) = grid.nearest_cell(profile, Some(component), goal, u32::MAX, false)
            else {
                return Begin::Done(None);
            };
            goal_cell = nearest;
            let (x, y) = grid.coords(nearest);
            goal = grid.center(x, y);
        }
        if start_cell == goal_cell {
            return Begin::Done(Some(Path {
                cells: vec![start_cell],
                waypoints: vec![start, goal],
                cost: 0,
                goal,
            }));
        }
        ws.reset(grid.base.len());
        ws.nodes[start_cell as usize] = Node {
            g: 0,
            parent: start_cell,
            mark: 2 * ws.generation,
        };
        let (gx, gy) = grid.coords(goal_cell);
        let (sx, sy) = grid.coords(start_cell);
        let h = heuristic(grid.profiles[profile].heuristic, sx, sy, gx, gy);
        ws.open.push(Reverse((h, h, start_cell)));
        Begin::Search(Self {
            profile,
            start,
            goal,
            start_cell,
            goal_cell,
            avoid,
        })
    }

    /// Expands nodes until the goal is reached (returns the path), the open
    /// list runs empty (returns `Some(None)`), or `budget` is used up
    /// (returns `None`; call again with fresh budget).
    fn run(&self, grid: &PathGrid, ws: &mut Workspace, budget: &mut u32) -> Option<Option<Path>> {
        let cost = &grid.profiles[self.profile].cost;
        let h_cost = grid.profiles[self.profile].heuristic;
        let seen = 2 * ws.generation;
        let closed = seen + 1;
        let (gx, gy) = grid.coords(self.goal_cell);
        while *budget > 0 {
            let Some(Reverse((_, _, cell))) = ws.open.pop() else {
                return Some(None);
            };
            let c = cell as usize;
            if ws.nodes[c].mark == closed {
                continue;
            }
            ws.nodes[c].mark = closed;
            let g_here = ws.nodes[c].g;
            *budget -= 1;
            if cell == self.goal_cell {
                return Some(Some(self.finish(grid, ws)));
            }
            let (x, y) = grid.coords(cell);
            // A blocked start cell (unit in the dilated margin) counts as
            // cheapest ground.
            let own = cost[c].max(MIN_CELL_COST);
            for &(dx, dy, len) in &NEIGHBORS {
                let Some((nx, ny)) = grid.offset(x, y, dx, dy) else {
                    continue;
                };
                let n = grid.index(nx, ny);
                if cost[n] == 0
                    || ws.nodes[n].mark == closed
                    || railing(&grid.deck, &grid.bridge_right, grid.width, c, n)
                {
                    continue;
                }
                // No corner cutting: both orthogonal cells must be free.
                if dx != 0
                    && dy != 0
                    && (cost[grid.index(nx, y)] == 0 || cost[grid.index(x, ny)] == 0)
                {
                    continue;
                }
                let step =
                    (u32::from(own) + u32::from(cost[n])) * len / 2 * avoided(&self.avoid, n);
                let g = g_here.saturating_add(step);
                let node = &mut ws.nodes[n];
                if node.mark == seen && g >= node.g {
                    continue;
                }
                *node = Node {
                    g,
                    parent: cell,
                    mark: seen,
                };
                let h = heuristic(h_cost, nx, ny, gx, gy);
                ws.open.push(Reverse((g.saturating_add(h), h, n as u32)));
            }
        }
        None
    }

    fn finish(&self, grid: &PathGrid, ws: &Workspace) -> Path {
        let mut cells = vec![self.goal_cell];
        let mut c = self.goal_cell;
        while c != self.start_cell {
            c = ws.nodes[c as usize].parent;
            cells.push(c);
        }
        cells.reverse();
        Path {
            waypoints: grid.smooth(self.profile, &cells, self.start, self.goal, &self.avoid),
            cost: ws.nodes[self.goal_cell as usize].g,
            cells,
            goal: self.goal,
        }
    }
}

/// A queued path request: one path for a group of units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Request {
    pub id: u32,
    pub start: Vec2,
    pub goal: Vec2,
    /// Index of the unit type whose profile is used.
    pub profile: usize,
    /// Units waiting for this path, with their offset from the group path
    /// (formation slot minus group target).
    pub units: Vec<(UnitId, Vec2)>,
}

/// Counters for tools and benchmarks (not part of the state hash).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathStats {
    pub requests: u64,
    pub found: u64,
    pub failed: u64,
    /// Requests dropped because no unit waited for them any more.
    pub dropped: u64,
    pub expanded: u64,
    /// Nodes expanded in the last tick.
    pub expanded_last_tick: u32,
}

/// The path request queue with its per-tick node budget.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Pathfinder {
    pub grid: PathGrid,
    workspace: Workspace,
    queue: VecDeque<Request>,
    /// Search for the front of the queue, if started.
    active: Option<Search>,
    next_id: u32,
    pub stats: PathStats,
}

impl Pathfinder {
    pub fn new(grid: PathGrid) -> Self {
        Self {
            grid,
            workspace: Workspace::default(),
            queue: VecDeque::new(),
            active: None,
            next_id: 0,
            stats: PathStats::default(),
        }
    }

    /// Queues a request and returns its id. The search starts at the
    /// lead-in point of `start` ([`PathGrid::lead_in`]).
    pub fn request(
        &mut self,
        ground: (&Map, &TerrainRules),
        start: Vec2,
        goal: Vec2,
        profile: usize,
        units: Vec<(UnitId, Vec2)>,
    ) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let start = self.grid.lead_in(ground.0, ground.1, profile, start);
        self.queue.push_back(Request {
            id,
            start,
            goal,
            profile,
            units,
        });
        self.stats.requests += 1;
        id
    }

    pub fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Forgets the progress of the running search (after the grid changed);
    /// it starts over on the next tick.
    pub fn restart_active(&mut self) {
        self.active = None;
    }

    /// Works through the queue in order with `budget` node expansions.
    /// Requests that `wanted` rejects (nobody waits for them) are dropped.
    /// Returns the finished requests in queue order with their paths
    /// (`None`: no path).
    #[cfg(test)]
    pub fn run(
        &mut self,
        budget: u32,
        wanted: impl Fn(u32) -> bool,
    ) -> Vec<(Request, Option<Path>)> {
        self.run_avoiding(budget, wanted, |_| Vec::new())
    }

    /// [`Self::run`] where every search avoids the cells `avoid` gives for
    /// its request (sorted indices, prompt 035: known minefields of the
    /// requesting player), asked once when the search starts.
    pub fn run_avoiding(
        &mut self,
        budget: u32,
        wanted: impl Fn(u32) -> bool,
        avoid: impl Fn(&Request) -> Vec<u32>,
    ) -> Vec<(Request, Option<Path>)> {
        let mut left = budget;
        let mut finished = Vec::new();
        while let Some(front) = self.queue.front() {
            if !wanted(front.id) {
                self.active = None;
                self.queue.pop_front();
                self.stats.dropped += 1;
                continue;
            }
            if left == 0 {
                break;
            }
            let result = if let Some(search) = &self.active {
                search.run(&self.grid, &mut self.workspace, &mut left)
            } else {
                // Starting a search costs one node, so every tick makes
                // progress even with trivial requests.
                left -= 1;
                match Search::begin(
                    &self.grid,
                    &mut self.workspace,
                    front.start,
                    front.goal,
                    front.profile,
                    avoid(front),
                ) {
                    Begin::Done(result) => Some(result),
                    Begin::Search(search) => {
                        self.active = Some(search);
                        continue;
                    }
                }
            };
            let Some(result) = result else {
                break;
            };
            self.active = None;
            if result.is_some() {
                self.stats.found += 1;
            } else {
                self.stats.failed += 1;
            }
            if let Some(request) = self.queue.pop_front() {
                finished.push((request, result));
            }
        }
        let used = budget - left;
        self.stats.expanded += u64::from(used);
        self.stats.expanded_last_tick = used;
        finished
    }

    pub fn hash_into(&self, h: &mut StableHasher) {
        self.grid.hash_into(h);
        h.write_u32(self.next_id);
        h.write_u64(self.queue.len() as u64);
        for request in &self.queue {
            h.write_u32(request.id);
        }
        h.write_u8(u8::from(self.active.is_some()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapLayout;
    use std::sync::Arc;

    const GRASS: u8 = 0;
    const ROAD: u8 = 2;
    const WATER: u8 = 7;

    fn infantry() -> UnitType {
        UnitType {
            name: "infantry".into(),
            speed_m_per_s: Fixed::from_num(2),
            road_speed_m_per_s: Fixed::from_num(2),
            footprint_m: Fixed::ONE,
            mass_kg: 0,
            combat: crate::combat::CombatType::default(),
            transport: crate::transport::TransportType::default(),
        }
    }

    fn tank() -> UnitType {
        UnitType {
            name: "tank".into(),
            speed_m_per_s: Fixed::from_num(4),
            road_speed_m_per_s: Fixed::from_num(10),
            footprint_m: Fixed::from_num(7),
            mass_kg: 0,
            combat: crate::combat::CombatType::default(),
            transport: crate::transport::TransportType::default(),
        }
    }

    /// A flat map of `cells_x` x `cells_y` path cells (4 m, map cells 2 m);
    /// `terrain(i, j)` gives the terrain of every map sample.
    fn make_map(cells_x: u32, cells_y: u32, terrain: impl Fn(u32, u32) -> u8) -> Arc<Map> {
        let (sx, sy) = (cells_x * 2 + 1, cells_y * 2 + 1);
        let ids = (0..sy)
            .flat_map(|j| (0..sx).map(move |i| (i, j)))
            .map(|(i, j)| terrain(i, j))
            .collect();
        let layout = MapLayout {
            name: "path-test".into(),
            samples_x: sx,
            samples_y: sy,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        Arc::new(Map::from_rasters(layout, vec![0; (sx * sy) as usize], ids).unwrap())
    }

    /// A map whose path cells are open where `open(x, y)` holds and deep
    /// water (all four samples of the cell) elsewhere.
    fn cell_map(cells_x: u32, cells_y: u32, open: impl Fn(u32, u32) -> bool) -> Arc<Map> {
        make_map(cells_x, cells_y, |i, j| {
            let (x, y) = (i / 2, j / 2);
            if x < cells_x && y < cells_y && !open(x, y) {
                WATER
            } else {
                GRASS
            }
        })
    }

    fn grid(map: &Map) -> PathGrid {
        PathGrid::new(
            map,
            &TerrainRules::builtin().unwrap(),
            &[infantry(), tank()],
        )
    }

    /// The serpentine of the labyrinth test: rows 1, 4 and 7 from x = 1 to
    /// 11, joined at x = 11 (rows 1..4) and x = 1 (rows 4..7).
    fn serpentine(x: u32, y: u32) -> bool {
        let row = matches!(y, 1 | 4 | 7) && (1..=11).contains(&x);
        let right = x == 11 && (1..=4).contains(&y);
        let left = x == 1 && (4..=7).contains(&y);
        row || right || left
    }

    /// Whether two labelings describe the same regions (labels may differ).
    fn same_partition(a: &[u32], b: &[u32]) -> bool {
        let mut ab = std::collections::BTreeMap::new();
        let mut ba = std::collections::BTreeMap::new();
        a.iter().zip(b).all(|(&x, &y)| {
            (x == 0) == (y == 0) && *ab.entry(x).or_insert(y) == y && *ba.entry(y).or_insert(x) == x
        })
    }

    #[test]
    fn local_relabeling_matches_a_full_labeling() {
        // Random walls and blocks are added and removed; after every change
        // the local update must give the regions a full labeling gives
        // (splits, merges, pockets, several areas at once).
        let map = make_map(48, 40, |i, j| {
            if (i * 7 + j * 3) % 41 == 0 {
                WATER
            } else {
                GRASS
            }
        });
        let rules = TerrainRules::builtin().unwrap();
        let mut grid = grid(&map);
        let mut rng = crate::Rng::new(5);
        let mut placed: Vec<CellRect> = Vec::new();
        for step in 0..400 {
            let remove = !placed.is_empty() && rng.range_i32(0, 3) == 0;
            let rect = if remove {
                let k = rng.range_i32(0, placed.len() as i32 - 1) as usize;
                placed.swap_remove(k)
            } else {
                let x = rng.range_i32(0, 46) as u32;
                let y = rng.range_i32(0, 38) as u32;
                let (w, h) = if rng.range_i32(0, 1) == 0 {
                    (rng.range_i32(0, 14) as u32, 0)
                } else {
                    (0, rng.range_i32(0, 14) as u32)
                };
                let r = CellRect {
                    x0: x,
                    y0: y,
                    x1: (x + w).min(47),
                    y1: (y + h).min(39),
                };
                placed.push(r);
                r
            };
            if step % 5 == 0 {
                // Two areas at once, like destroyed objects in one tick.
                let other = CellRect {
                    x0: rng.range_i32(0, 47) as u32,
                    y0: rng.range_i32(0, 39) as u32,
                    ..rect
                };
                let other = CellRect {
                    x1: other.x0,
                    y1: other.y0,
                    ..other
                };
                for (r, blocked) in [(rect, !remove), (other, step % 10 == 0)] {
                    for y in r.y0..=r.y1 {
                        for x in r.x0..=r.x1 {
                            let i = grid.index(x, y);
                            if blocked {
                                grid.base[i] |= BLOCKED_OBSTACLE;
                            } else {
                                grid.base[i] &= !BLOCKED_OBSTACLE;
                            }
                        }
                    }
                }
                let a = grid.rebuild_cells(&map, &rules, rect);
                let b = grid.rebuild_cells(&map, &rules, other);
                grid.relabel_components_in(&[a, b]);
            } else {
                grid.set_obstacle(&map, &rules, rect, !remove);
            }
            let mut full = grid.clone();
            full.relabel_components();
            for (local, full) in grid.components.iter().zip(&full.components) {
                assert!(
                    same_partition(&local.component, &full.component),
                    "step {step}: regions differ after {rect:?}"
                );
            }
        }
    }

    #[test]
    fn labyrinth_has_the_expected_path() {
        let map = cell_map(13, 9, serpentine);
        let grid = grid(&map);
        let start = grid.center(1, 1);
        let goal = grid.center(11, 7);
        let path = grid.find_path(start, goal, 0).unwrap();
        // 10 + 3 + 10 + 3 + 10 straight steps through a corridor one cell
        // wide (no diagonal can cut its corners).
        assert_eq!(path.cells.len(), 37);
        assert_eq!(path.cost, 36 * 1000);
        assert_eq!(path.length(), Fixed::from_num(36 * 4));
        let corners: Vec<Vec2> = [(1, 1), (11, 1), (11, 4), (1, 4), (1, 7), (11, 7)]
            .iter()
            .map(|&(x, y)| grid.center(x, y))
            .collect();
        assert_eq!(path.waypoints, corners);
    }

    /// Reference: Dijkstra without heuristic, same step costs and rules.
    fn dijkstra(grid: &PathGrid, p: usize, start: u32, goal: u32) -> Option<u32> {
        let cost = grid.costs(p);
        let mut best = vec![u32::MAX; cost.len()];
        let mut open = BinaryHeap::new();
        best[start as usize] = 0;
        open.push(Reverse((0u32, start)));
        while let Some(Reverse((g, c))) = open.pop() {
            if c == goal {
                return Some(g);
            }
            if g > best[c as usize] {
                continue;
            }
            let (x, y) = grid.coords(c);
            for &(dx, dy, len) in &NEIGHBORS {
                let Some((nx, ny)) = grid.offset(x, y, dx, dy) else {
                    continue;
                };
                let n = grid.index(nx, ny);
                let corner = dx != 0
                    && dy != 0
                    && (cost[grid.index(nx, y)] == 0 || cost[grid.index(x, ny)] == 0);
                if cost[n] == 0 || corner {
                    continue;
                }
                let own = cost[c as usize].max(MIN_CELL_COST);
                let next = g + (u32::from(own) + u32::from(cost[n])) * len / 2;
                if next < best[n] {
                    best[n] = next;
                    open.push(Reverse((next, n as u32)));
                }
            }
        }
        None
    }

    /// An open maze with several routes, mud and a road.
    fn maze() -> Arc<Map> {
        make_map(40, 30, |i, j| {
            let (x, y) = (i / 2, j / 2);
            let wall = (x == 10 && j < 50) || (x == 20 && j > 10) || (x == 30 && j < 46);
            if wall && i % 2 == 1 && j % 2 == 1 {
                WATER
            } else if (24..34).contains(&y) || x > 36 {
                4 // mud
            } else if y == 2 {
                ROAD
            } else {
                GRASS
            }
        })
    }

    #[test]
    fn astar_finds_the_optimal_cost() {
        let map = maze();
        let grid = grid(&map);
        for p in 0..2 {
            for (a, b) in [((2, 2), (38, 27)), ((5, 25), (35, 3)), ((15, 15), (25, 12))] {
                let start = grid.center(a.0, a.1);
                let goal = grid.center(b.0, b.1);
                let path = grid.find_path(start, goal, p).unwrap();
                let s = grid.index(a.0, a.1) as u32;
                let g = grid.index(b.0, b.1) as u32;
                assert_eq!(
                    Some(path.cost),
                    dijkstra(&grid, p, s, g),
                    "profile {p} {a:?}"
                );
                for w in path.waypoints.windows(2) {
                    let class = grid.class_of(p);
                    assert!(grid.line_of_sight(class, w[0], w[1]), "{w:?}");
                }
            }
        }
    }

    #[test]
    fn two_runs_give_identical_paths() {
        let map = maze();
        let a = grid(&map);
        let b = grid(&map);
        let start = Vec2::from_millimeters(-70_123, -50_456);
        let goal = Vec2::from_millimeters(65_000, 49_999);
        assert_eq!(a.find_path(start, goal, 1), b.find_path(start, goal, 1));
        // Symmetric open field: many paths of equal cost, always the same one.
        let open = make_map(20, 20, |_, _| GRASS);
        let open = grid(&open);
        let first = open.find_path(open.center(0, 0), open.center(19, 13), 0);
        for _ in 0..3 {
            assert_eq!(
                open.find_path(open.center(0, 0), open.center(19, 13), 0),
                first
            );
        }
    }

    #[test]
    fn tanks_keep_clearance_and_infantry_does_not() {
        // One blocked cell in the middle of an open field.
        let map = cell_map(9, 9, |x, y| (x, y) != (4, 4));
        let grid = grid(&map);
        assert!(!grid.passable(SizeClass::Small, 4, 4));
        assert!(grid.passable(SizeClass::Small, 3, 4));
        assert!(!grid.passable(SizeClass::Large, 3, 4));
        assert!(!grid.passable(SizeClass::Large, 5, 5));
        assert!(grid.passable(SizeClass::Large, 6, 4));
        // The map edge counts as blocked for tanks.
        assert!(!grid.passable(SizeClass::Large, 0, 2));
        assert!(grid.passable(SizeClass::Small, 0, 2));
    }

    #[test]
    fn roads_are_preferred() {
        // Road along row 1 (map samples j = 2..=4), grass elsewhere.
        let map = make_map(
            30,
            12,
            |_, j| if (2..=4).contains(&j) { ROAD } else { GRASS },
        );
        let grid = grid(&map);
        let start = grid.center(2, 3);
        let goal = grid.center(27, 3);
        // Infantry has no road speed: straight across the grass.
        let walk = grid.find_path(start, goal, 0).unwrap();
        assert_eq!(walk.waypoints, vec![start, goal]);
        // The tank drives 2.5 times as fast on the road: detour over it.
        let drive = grid.find_path(start, goal, 1).unwrap();
        assert!(drive.waypoints.len() > 2);
        let on_road = drive
            .cells
            .iter()
            .filter(|&&c| grid.coords(c).1 == 1)
            .count();
        assert!(on_road >= 20, "{on_road} cells on the road");
        // Straight across the grass would cost 25 * 2500.
        assert!(drive.cost < 25 * 2500 / 2, "cost {}", drive.cost);
    }

    #[test]
    fn unreachable_goals_snap_to_the_nearest_reachable_cell() {
        // A lake (cells 10..=14 x 3..=7) with the goal in its middle.
        let map = cell_map(20, 10, |x, y| {
            !((10..=14).contains(&x) && (3..=7).contains(&y))
        });
        let grid = grid(&map);
        let path = grid
            .find_path(grid.center(2, 5), grid.center(11, 5), 0)
            .unwrap();
        assert_eq!(path.goal, grid.center(9, 5));
        assert_eq!(path.waypoints.last(), Some(&grid.center(9, 5)));
    }

    #[test]
    fn budget_spreads_a_search_over_ticks() {
        let map = maze();
        let mut finder = Pathfinder::new(grid(&map));
        let start = finder.grid.center(2, 2);
        let goal = finder.grid.center(38, 27);
        let id = finder.request(
            (&map, &TerrainRules::builtin().unwrap()),
            start,
            goal,
            0,
            vec![(UnitId(0), Vec2::ZERO)],
        );
        let mut ticks = 0;
        let path = loop {
            ticks += 1;
            let done = finder.run(100, |r| r == id);
            assert!(finder.stats.expanded_last_tick <= 100);
            if let Some((request, path)) = done.into_iter().next() {
                assert_eq!(request.id, id);
                break path.unwrap();
            }
            assert!(ticks < 1000);
        };
        assert!(ticks > 3, "finished in {ticks} ticks");
        assert_eq!(Some(path), finder.grid.find_path(start, goal, 0));
        // Unwanted requests are dropped without searching.
        finder.request(
            (&map, &TerrainRules::builtin().unwrap()),
            start,
            goal,
            0,
            Vec::new(),
        );
        assert!(finder.run(100, |_| false).is_empty());
        assert_eq!(finder.queued(), 0);
    }

    #[test]
    fn obstacles_change_paths_until_removed() {
        let map = make_map(30, 10, |_, _| GRASS);
        let rules = TerrainRules::builtin().unwrap();
        let mut grid = grid(&map);
        let start = grid.center(2, 5);
        let goal = grid.center(27, 5);
        let straight = grid.find_path(start, goal, 0).unwrap();
        assert_eq!(straight.waypoints, vec![start, goal]);
        // A wall across rows 1..=9 at x = 15.
        let wall = CellRect {
            x0: 15,
            y0: 1,
            x1: 15,
            y1: 9,
        };
        grid.set_obstacle(&map, &rules, wall, true);
        assert_eq!(grid.version(), 1);
        let detour = grid.find_path(start, goal, 0).unwrap();
        assert!(detour.cost > straight.cost);
        assert!(detour.waypoints.iter().any(|p| grid.cell_of(*p).1 == 0));
        grid.set_obstacle(&map, &rules, wall, false);
        assert_eq!(grid.find_path(start, goal, 0), Some(straight));
    }

    #[test]
    fn nearest_cell_matches_brute_force() {
        let map = maze();
        let grid = grid(&map);
        let profile = 1; // the tank: large
        let (w, h) = grid.size();
        let mut rng = crate::Rng::new(3);
        for _ in 0..200 {
            let goal = Vec2::from_millimeters(
                rng.range_i32(-90_000, 90_000),
                rng.range_i32(-70_000, 70_000),
            );
            let mut best: Option<(i128, u32)> = None;
            for y in 0..h {
                for x in 0..w {
                    let i = grid.index(x, y);
                    if grid.costs(profile)[i] == 0 {
                        continue;
                    }
                    let d = grid.center(x, y) - goal;
                    let (dx, dy) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
                    let key = (dx * dx + dy * dy, i as u32);
                    if best.is_none_or(|b| key < b) {
                        best = Some(key);
                    }
                }
            }
            assert_eq!(
                grid.nearest_cell(profile, None, goal, u32::MAX, false),
                best.map(|b| b.1),
                "{goal:?}"
            );
        }
    }

    #[test]
    fn line_walk_checks_corners() {
        // Diagonal through the shared corner of two blocked cells.
        let map = cell_map(4, 4, |x, y| !matches!((x, y), (1, 0) | (0, 1)));
        let grid = grid(&map);
        let a = grid.center(0, 0);
        let b = grid.center(1, 1);
        assert!(!grid.line_of_sight(SizeClass::Small, a, b));
        assert!(grid.line_of_sight(SizeClass::Small, grid.center(1, 1), grid.center(3, 3)));
    }
}
