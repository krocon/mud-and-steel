//! Vision and fog of war (prompt 018): rules, the static vision grid of a map
//! and the sight computation of one unit (its viewshed).
//!
//! The map is divided into square vision cells (`grid.cell_mm`). Every unit
//! sees the cells inside its sight range that a ray from its eye reaches
//! without passing below the horizon formed by the ground, solid obstacles
//! (buildings, walls, hedges) and forest. Everything is integer math on
//! millimeters: slopes are compared by cross-multiplication, never divided.
//! How the simulation keeps the viewsheds up to date: `crate::sim::vision`.

use crate::catalog::Category;
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::{MapObject, Shape};
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation (`data/vision.toml`).
pub const BUILTIN_VISION: &str = include_str!("../../../data/vision.toml");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridRules {
    pub cell_mm: u32,
    pub target_height_mm: u32,
    pub foliage_min_objects: u32,
    pub foliage_depth_cells: u32,
    pub budget_steps_per_tick: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightOverride {
    pub category: Category,
    pub range_real_m: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SightRules {
    pub range_real_m: u32,
    #[serde(default, rename = "category")]
    pub categories: Vec<SightOverride>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSight {
    pub solid: Vec<String>,
    pub foliage: Vec<String>,
}

/// Seeing downhill (prompt 060): a vision cell whose ground lies more than
/// `min_drop_m` below the eye is seen farther than the sight range, by
/// `pct_per_10m` percent per 10 m of further drop, at most `max_pct`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeightRules {
    pub min_drop_m: u32,
    pub pct_per_10m: u32,
    pub max_pct: u32,
}

impl HeightRules {
    /// Sight radius in cells including the largest bonus.
    pub fn reach_cells(&self, radius: i32) -> i32 {
        let pct = 100 + self.max_pct as i32;
        (radius * pct + 99) / 100
    }

    /// Bonus in percent of the sight range for ground `drop_mm` below the
    /// eye.
    pub fn bonus_pct(&self, drop_mm: i64) -> i64 {
        let over = drop_mm - i64::from(self.min_drop_m) * 1000;
        if over <= 0 {
            return 0;
        }
        (over * i64::from(self.pct_per_10m) / 10_000).min(i64::from(self.max_pct))
    }
}

/// Infantry ambush (prompt 056) [provisional]: a squad standing in a
/// forest cell or garrisoned in a building, that has not fired for
/// `reveal_s` seconds, is seen only by hostile ground units within
/// `spot_real_m` (real meters, compressed like sight ranges). Zero
/// `spot_real_m` turns the rule off. Hashed with the ambush state, only
/// once a squad has been hidden (`crate::sim` vision).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbushRules {
    pub spot_real_m: u32,
    pub reveal_s: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    grid: GridRules,
    sight: SightRules,
    objects: ObjectSight,
    height: HeightRules,
    #[serde(default)]
    ambush: AmbushRules,
}

/// All vision rules, with the sight range factor of `data/scale.toml`
/// resolved. Part of [`crate::SimConfig`], so replays carry them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionRules {
    pub grid: GridRules,
    pub sight: SightRules,
    pub objects: ObjectSight,
    /// `range.sight` of the scale, `(num, den)`.
    pub sight_ratio: (u32, u32),
    /// Seeing farther downhill (prompt 060).
    #[serde(default)]
    pub height: HeightRules,
    /// Infantry ambush (prompt 056).
    #[serde(default)]
    pub ambush: AmbushRules,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisionError(pub String);

impl fmt::Display for VisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid vision rules: {}", self.0)
    }
}

impl std::error::Error for VisionError {}

/// Largest sight range in game meters (keeps ray tables small).
const MAX_RANGE_M: u32 = 2000;

/// How a map object affects sight.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sight {
    Clear,
    Solid,
    Foliage,
}

impl VisionRules {
    pub fn parse(text: &str, scale: &Scale) -> Result<Self, VisionError> {
        let file: File = toml::from_str(text).map_err(|e| VisionError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(VisionError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let g = &file.grid;
        check("grid.cell_mm", g.cell_mm, 1000, 100_000)?;
        check("grid.target_height_mm", g.target_height_mm, 0, 100_000)?;
        check("grid.foliage_min_objects", g.foliage_min_objects, 1, 1000)?;
        check("grid.foliage_depth_cells", g.foliage_depth_cells, 0, 16)?;
        check(
            "grid.budget_steps_per_tick",
            g.budget_steps_per_tick,
            1,
            100_000_000,
        )?;
        let hr = &file.height;
        check("height.min_drop_m", hr.min_drop_m, 0, 10_000)?;
        check("height.pct_per_10m", hr.pct_per_10m, 0, 100)?;
        check("height.max_pct", hr.max_pct, 0, 100)?;
        check("ambush.spot_real_m", file.ambush.spot_real_m, 0, 8000)?;
        check("ambush.reveal_s", file.ambush.reveal_s, 0, 3600)?;
        let ratio = (scale.sight_range.num, scale.sight_range.den);
        let rules = Self {
            grid: file.grid,
            sight: file.sight,
            objects: file.objects,
            sight_ratio: ratio,
            height: file.height,
            ambush: file.ambush,
        };
        let ranges = std::iter::once(rules.sight.range_real_m)
            .chain(rules.sight.categories.iter().map(|o| o.range_real_m));
        for real in ranges {
            let game = scale.sight_range_m(real);
            if game < Fixed::ONE || game > Fixed::from_num(MAX_RANGE_M) {
                return Err(VisionError(format!(
                    "sight range {real} m real is {game} m in game, outside 1..={MAX_RANGE_M}"
                )));
            }
        }
        Ok(rules)
    }

    /// Borrowed reference to the built-in vision rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static VisionError> {
        static CACHE: std::sync::OnceLock<Result<VisionRules, VisionError>> =
            std::sync::OnceLock::new();
        CACHE
            .get_or_init(|| {
                let scale = Scale::builtin_ref().map_err(|e| VisionError(e.to_string()))?;
                Self::parse(BUILTIN_VISION, scale)
            })
            .as_ref()
    }

    /// The rules from `data/vision.toml` with the built-in scale.
    pub fn builtin() -> Result<Self, VisionError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    /// Edge of a vision cell in meters.
    pub fn cell_m(&self) -> Fixed {
        Fixed::from_num(self.grid.cell_mm) / Fixed::from_num(1000)
    }

    /// Sight range in game meters for a category.
    pub fn range_m(&self, category: Option<Category>) -> Fixed {
        let real = self
            .sight
            .categories
            .iter()
            .find(|o| Some(o.category) == category)
            .map_or(self.sight.range_real_m, |o| o.range_real_m);
        let (num, den) = self.sight_ratio;
        Fixed::from_num(real) * Fixed::from_num(num) / Fixed::from_num(den)
    }

    /// Ambush spotting distance in game meters (prompt 056).
    pub fn ambush_spot_m(&self) -> Fixed {
        let (num, den) = self.sight_ratio;
        Fixed::from_num(self.ambush.spot_real_m) * Fixed::from_num(num) / Fixed::from_num(den)
    }

    /// Sight range in whole vision cells (at least 1).
    pub fn range_cells(&self, category: Option<Category>) -> i32 {
        (self.range_m(category) / self.cell_m())
            .to_num::<i32>()
            .max(1)
    }

    pub fn object_sight(&self, id: &str) -> Sight {
        if self.objects.solid.iter().any(|s| s == id) {
            Sight::Solid
        } else if self.objects.foliage.iter().any(|s| s == id) {
            Sight::Foliage
        } else {
            Sight::Clear
        }
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let g = &self.grid;
        for v in [
            g.cell_mm,
            g.target_height_mm,
            g.foliage_min_objects,
            g.foliage_depth_cells,
            g.budget_steps_per_tick,
            self.sight.range_real_m,
            self.sight_ratio.0,
            self.sight_ratio.1,
            self.height.min_drop_m,
            self.height.pct_per_10m,
            self.height.max_pct,
        ] {
            h.write_u32(v);
        }
        h.write_u64(self.sight.categories.len() as u64);
        for o in &self.sight.categories {
            h.write_bytes(o.category.code().as_bytes());
            h.write_u32(o.range_real_m);
        }
        for list in [&self.objects.solid, &self.objects.foliage] {
            h.write_u64(list.len() as u64);
            for id in list {
                h.write_bytes(id.as_bytes());
                h.write_u8(0);
            }
        }
    }
}

/// No obstacle in a cell.
const NONE: i32 = i32::MIN;

/// The rays of a sight radius: cell offsets, ray by ray.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RayTable {
    radius: i32,
    /// Offsets of all rays, one after the other.
    offsets: Vec<(i16, i16)>,
    /// Start of every ray in `offsets`, plus the end.
    starts: Vec<u32>,
}

impl RayTable {
    /// Rays from the center to every cell on the edge of the square with
    /// half edge `radius`; each ray visits the cell nearest to it at every
    /// step along its major axis, and ends at the sight circle. Together
    /// they visit every cell of the circle.
    fn new(radius: i32) -> Self {
        let mut ends = Vec::with_capacity(8 * radius as usize);
        for i in -radius..radius {
            ends.push((i, -radius));
            ends.push((radius, i));
            ends.push((-i, radius));
            ends.push((-radius, -i));
        }
        let r2 = i64::from(radius) * i64::from(radius);
        let mut offsets = Vec::new();
        let mut starts = vec![0u32];
        for (ex, ey) in ends {
            for k in 1..=radius {
                let dx = round_div(ex * k, radius);
                let dy = round_div(ey * k, radius);
                if i64::from(dx * dx + dy * dy) > r2 {
                    break;
                }
                offsets.push((dx as i16, dy as i16));
            }
            starts.push(offsets.len() as u32);
        }
        Self {
            radius,
            offsets,
            starts,
        }
    }
}

/// `a / b` rounded half away from zero (`b > 0`).
const fn round_div(a: i32, b: i32) -> i32 {
    if a >= 0 {
        (2 * a + b) / (2 * b)
    } else {
        -((-2 * a + b) / (2 * b))
    }
}

/// The static vision data of a map: cell heights and obstacles.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisionGrid {
    pub origin: Vec2,
    pub cell: Fixed,
    pub columns: u32,
    pub rows: u32,
    /// Ground height at the cell center, millimeters.
    ground_mm: Vec<i32>,
    /// Top of solid obstacles (absolute, mm) or [`NONE`].
    solid_mm: Vec<i32>,
    /// Top of forest (absolute, mm) or [`NONE`].
    foliage_mm: Vec<i32>,
    /// Top of smoke (absolute, mm) or [`NONE`] (prompt 032).
    smoke_mm: Vec<i32>,
    target_height_mm: i32,
    foliage_depth: i32,
    /// Trees per cell that make forest.
    foliage_min: u32,
    tables: Vec<RayTable>,
    /// Seeing farther downhill (prompt 060).
    #[serde(default)]
    height: HeightRules,
    /// Lowest ground (mm) per block of [`BLOCK_CELLS`] × [`BLOCK_CELLS`]
    /// cells, row-major: decides cheaply whether a unit can see anything
    /// beyond its sight range at all (prompt 060).
    #[serde(default)]
    block_min_mm: Vec<i32>,
    /// Ground below a see-through deck (steel bridges, prompt 115) at the
    /// cell center (mm), [`NONE`] elsewhere; empty on maps without such
    /// decks. Sight lines pass over this ground under the deck, while
    /// `ground_mm` (the deck) is where units there stand and are seen.
    #[serde(default)]
    deck_low_mm: Vec<i32>,
}

/// Ground below see-through decks per cell (see `VisionGrid::deck_low_mm`).
fn deck_low(map: &Map, p: Vec2) -> i32 {
    match map.deck_at(p) {
        Some(b) if b.see_through => to_mm(map.ground_height_at(p)),
        _ => NONE,
    }
}

/// Edge of a block of [`VisionGrid::lowest_within`] in vision cells.
const BLOCK_CELLS: u32 = 8;

impl VisionGrid {
    /// Builds the grid of `map`; `radii` are the sight radii (cells) used.
    pub fn new(map: &Map, rules: &VisionRules, radii: &[i32]) -> Self {
        let size = map.size();
        let cell = rules.cell_m();
        let origin = Vec2::new(-size.x / 2, -size.y / 2);
        let columns = (size.x / cell).ceil().to_num::<u32>().max(1);
        let rows = (size.y / cell).ceil().to_num::<u32>().max(1);
        let count = (columns * rows) as usize;
        let half = cell / 2;
        let center = |c: u32, r: u32| {
            Vec2::new(
                origin.x + cell * Fixed::from_num(c) + half,
                origin.y + cell * Fixed::from_num(r) + half,
            )
        };
        let to_mm = |h: Fixed| (h * Fixed::from_num(1000)).round().to_num::<i32>();
        let mut ground_mm = Vec::with_capacity(count);
        for r in 0..rows {
            for c in 0..columns {
                ground_mm.push(to_mm(map.height_at(center(c, r))));
            }
        }
        let mut solid_mm = vec![NONE; count];
        let mut foliage_mm = vec![NONE; count];
        let mut foliage_count = vec![0u32; count];
        let kinds = map.object_kinds();
        let sights: Vec<Sight> = kinds.iter().map(|k| rules.object_sight(&k.id)).collect();
        let cell_index = |v: Fixed, o: Fixed, n: u32| {
            ((v - o) / cell)
                .floor()
                .max(Fixed::ZERO)
                .to_num::<u32>()
                .min(n - 1)
        };
        for object in map.objects() {
            let k = usize::from(object.kind);
            let sight = sights[k];
            let kind = &kinds[k];
            if sight == Sight::Clear || kind.height <= Fixed::ZERO {
                continue;
            }
            let height = to_mm(kind.height);
            let extent = object.half_extent(kind.shape);
            let low = object.position - extent;
            let high = object.position + extent;
            for r in cell_index(low.y, origin.y, rows)..=cell_index(high.y, origin.y, rows) {
                for c in
                    cell_index(low.x, origin.x, columns)..=cell_index(high.x, origin.x, columns)
                {
                    if !object.overlaps_square(kind.shape, center(c, r), half) {
                        continue;
                    }
                    let i = (r * columns + c) as usize;
                    let top = ground_mm[i].saturating_add(height);
                    match sight {
                        Sight::Solid => solid_mm[i] = solid_mm[i].max(top),
                        Sight::Foliage => {
                            foliage_count[i] += 1;
                            foliage_mm[i] = foliage_mm[i].max(top);
                        }
                        Sight::Clear => {}
                    }
                }
            }
        }
        for (top, &n) in foliage_mm.iter_mut().zip(&foliage_count) {
            if n < rules.grid.foliage_min_objects {
                *top = NONE;
            }
        }
        let mut wanted: Vec<i32> = radii.to_vec();
        wanted.sort_unstable();
        wanted.dedup();
        let mut grid = Self {
            origin,
            cell,
            columns,
            rows,
            ground_mm,
            solid_mm,
            foliage_mm,
            smoke_mm: vec![NONE; count],
            target_height_mm: rules.grid.target_height_mm as i32,
            foliage_depth: rules.grid.foliage_depth_cells as i32,
            foliage_min: rules.grid.foliage_min_objects,
            tables: wanted.into_iter().map(RayTable::new).collect(),
            height: rules.height,
            block_min_mm: Vec::new(),
            deck_low_mm: Vec::new(),
        };
        if map.bridges().iter().any(|b| b.see_through) {
            let mut low = vec![NONE; count];
            for r in 0..rows {
                for c in 0..columns {
                    low[(r * columns + c) as usize] = deck_low(map, center(c, r));
                }
            }
            grid.deck_low_mm = low;
        }
        let (bw, bh) = grid.blocks();
        let mut block_min = vec![i32::MAX; (bw * bh) as usize];
        for r in 0..rows {
            let br = r / BLOCK_CELLS;
            let row_off = (r * columns) as usize;
            let blk_row = (br * bw) as usize;
            for c in 0..columns {
                let b = blk_row + (c / BLOCK_CELLS) as usize;
                let h = grid.ground_mm[row_off + c as usize];
                if h < block_min[b] {
                    block_min[b] = h;
                }
            }
        }
        grid.block_min_mm = block_min;
        grid
    }

    /// Number of blocks across and down.
    const fn blocks(&self) -> (u32, u32) {
        (
            self.columns.div_ceil(BLOCK_CELLS),
            self.rows.div_ceil(BLOCK_CELLS),
        )
    }

    /// Recomputes the lowest ground of block `(bc, br)`.
    fn update_block_coords(&mut self, bc: u32, br: u32) {
        let mut low = i32::MAX;
        for r in br * BLOCK_CELLS..((br + 1) * BLOCK_CELLS).min(self.rows) {
            for c in bc * BLOCK_CELLS..((bc + 1) * BLOCK_CELLS).min(self.columns) {
                low = low.min(self.ground_mm[(r * self.columns + c) as usize]);
            }
        }
        let b = (br * self.blocks().0 + bc) as usize;
        self.block_min_mm[b] = low;
    }

    /// A lower bound of the ground (mm) of every cell within `radius`
    /// cells (a square) of cell `anchor`.
    pub(crate) fn lowest_within(&self, anchor: u32, radius: i32) -> i32 {
        let (ac, ar) = (
            (anchor % self.columns) as i32,
            (anchor / self.columns) as i32,
        );
        let block = BLOCK_CELLS as i32;
        let (bw, bh) = self.blocks();
        let span = |center: i32, n: u32| {
            (
                ((center - radius).max(0) / block) as u32,
                ((center + radius).min(n as i32 - 1) / block) as u32,
            )
        };
        let (c0, c1) = span(ac, self.columns);
        let (r0, r1) = span(ar, self.rows);
        let mut low = i32::MAX;
        for r in r0..=r1.min(bh - 1) {
            for c in c0..=c1.min(bw - 1) {
                low = low.min(self.block_min_mm[(r * bw + c) as usize]);
            }
        }
        low
    }

    /// Sight radius for an eye at `eye_mm` in cell `anchor` with sight
    /// range `base` and longest reach `reach` (cells, prompt 060): the
    /// reach only if some ground within it is low enough for the downhill
    /// bonus.
    pub(crate) fn downhill_radius(&self, anchor: u32, eye_mm: i32, base: i32, reach: i32) -> i32 {
        let low = self.lowest_within(anchor, reach);
        if reach > base && self.height.bonus_pct(i64::from(eye_mm) - i64::from(low)) > 0 {
            reach
        } else {
            base
        }
    }

    /// Computes the obstacles of the cells `c0..=c1` × `r0..=r1` again
    /// from `objects` (position, shape, how it affects sight now and its
    /// height in meters), after objects there changed (destruction, prompt
    /// 020). Pass every object whose shape may overlap the cells; cells
    /// outside the rectangle are not touched.
    pub fn rebuild_objects(
        &mut self,
        (c0, r0, c1, r1): (u32, u32, u32, u32),
        objects: impl IntoIterator<Item = (MapObject, Shape, Sight, Fixed)>,
    ) {
        let (c1, r1) = (c1.min(self.columns - 1), r1.min(self.rows - 1));
        let width = (c1 - c0 + 1) as usize;
        let mut counts = vec![0u32; width * (r1 - r0 + 1) as usize];
        for r in r0..=r1 {
            for c in c0..=c1 {
                let i = (r * self.columns + c) as usize;
                self.solid_mm[i] = NONE;
                self.foliage_mm[i] = NONE;
            }
        }
        let half = self.cell / 2;
        for (object, shape, sight, height) in objects {
            if sight == Sight::Clear || height <= Fixed::ZERO {
                continue;
            }
            let height = to_mm(height);
            let extent = object.half_extent(shape);
            let (lc, lr) = self.cell_of(object.position - extent);
            let (hc, hr) = self.cell_of(object.position + extent);
            for r in (lr as u32).max(r0)..=(hr as u32).min(r1) {
                for c in (lc as u32).max(c0)..=(hc as u32).min(c1) {
                    if !object.overlaps_square(shape, self.center(c, r), half) {
                        continue;
                    }
                    let i = (r * self.columns + c) as usize;
                    let top = self.ground_mm[i].saturating_add(height);
                    match sight {
                        Sight::Solid => self.solid_mm[i] = self.solid_mm[i].max(top),
                        Sight::Foliage => {
                            counts[(r - r0) as usize * width + (c - c0) as usize] += 1;
                            self.foliage_mm[i] = self.foliage_mm[i].max(top);
                        }
                        Sight::Clear => {}
                    }
                }
            }
        }
        for r in r0..=r1 {
            for c in c0..=c1 {
                if counts[(r - r0) as usize * width + (c - c0) as usize] < self.foliage_min {
                    self.foliage_mm[(r * self.columns + c) as usize] = NONE;
                }
            }
        }
    }

    /// Cells of `c0..=c1` × `r0..=r1` whose ground height (at the center)
    /// differs from `map` now, with the new height in millimeters (craters,
    /// prompt 021). Apply them with [`Self::set_ground`].
    pub fn ground_changes(
        &self,
        (c0, r0, c1, r1): (u32, u32, u32, u32),
        map: &Map,
    ) -> Vec<(u32, i32)> {
        let (c1, r1) = (c1.min(self.columns - 1), r1.min(self.rows - 1));
        let mut out = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                let i = r * self.columns + c;
                let mm = to_mm(map.height_at(self.center(c, r)));
                if mm != self.ground_mm[i as usize] {
                    out.push((i, mm));
                }
            }
        }
        out
    }

    /// After a see-through deck (prompt 115) collapsed or was restored:
    /// the ground and the ground below decks of the cells `c0..=c1` ×
    /// `r0..=r1` from `map`. Returns whether anything changed.
    pub fn refresh_decks(&mut self, (c0, r0, c1, r1): (u32, u32, u32, u32), map: &Map) -> bool {
        let changes = self.ground_changes((c0, r0, c1, r1), map);
        let mut changed = !changes.is_empty();
        self.set_ground(&changes);
        if !self.deck_low_mm.is_empty() {
            for r in r0..=r1.min(self.rows - 1) {
                for c in c0..=c1.min(self.columns - 1) {
                    let i = (r * self.columns + c) as usize;
                    let low = deck_low(map, self.center(c, r));
                    if low != self.deck_low_mm[i] {
                        self.deck_low_mm[i] = low;
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    /// Sets the ground height of cells (index, millimeters). Obstacle tops
    /// are absolute heights and stay as they are.
    pub fn set_ground(&mut self, changes: &[(u32, i32)]) {
        for &(i, mm) in changes {
            self.ground_mm[i as usize] = mm;
        }
        if changes.is_empty() {
            return;
        }
        let bw = self.blocks().0;
        let mut blocks: Vec<u32> = changes
            .iter()
            .map(|&(i, _)| {
                let bc = (i % self.columns) / BLOCK_CELLS;
                let br = (i / self.columns) / BLOCK_CELLS;
                br * bw + bc
            })
            .collect();
        blocks.sort_unstable();
        blocks.dedup();
        for b in blocks {
            self.update_block_coords(b % bw, b / bw);
        }
    }

    /// Puts smoke up to `height_mm` above the ground over cell `index`, or
    /// clears it (`None`, prompt 032). Sight lines below the top end
    /// there, and targets in the cell are hidden.
    pub fn set_smoke(&mut self, index: u32, height_mm: Option<i32>) {
        let i = index as usize;
        self.smoke_mm[i] = height_mm.map_or(NONE, |h| self.ground_mm[i].saturating_add(h));
    }

    /// Whether cell `index` is under smoke.
    pub fn smoke_at(&self, index: u32) -> bool {
        self.smoke_mm[index as usize] != NONE
    }

    /// Ground height at the center of cell `index`, millimeters.
    pub fn ground_mm(&self, index: u32) -> i32 {
        self.ground_mm[index as usize]
    }

    /// Center of cell `(column, row)`.
    pub fn center(&self, c: u32, r: u32) -> Vec2 {
        let half = self.cell / 2;
        Vec2::new(
            self.origin.x + self.cell * Fixed::from_num(c) + half,
            self.origin.y + self.cell * Fixed::from_num(r) + half,
        )
    }

    pub const fn cells(&self) -> usize {
        (self.columns * self.rows) as usize
    }

    /// Cell `(column, row)` containing `p` (clamped to the grid).
    pub fn cell_of(&self, p: Vec2) -> (i32, i32) {
        let axis = |v: Fixed, o: Fixed, n: u32| {
            ((v - o) / self.cell)
                .floor()
                .max(Fixed::ZERO)
                .to_num::<u32>()
                .min(n - 1) as i32
        };
        (
            axis(p.x, self.origin.x, self.columns),
            axis(p.y, self.origin.y, self.rows),
        )
    }

    pub fn index_of(&self, p: Vec2) -> u32 {
        let (c, r) = self.cell_of(p);
        r as u32 * self.columns + c as u32
    }

    /// Whether cell `i` holds a solid obstacle / forest (tools and tests).
    pub fn blocker_at(&self, i: usize) -> Sight {
        if self.solid_mm[i] != NONE {
            Sight::Solid
        } else if self.foliage_mm[i] != NONE {
            Sight::Foliage
        } else {
            Sight::Clear
        }
    }

    /// Whether viewsheds of `radius` cells can be computed.
    pub(crate) fn has_table(&self, radius: i32) -> bool {
        self.tables.iter().any(|t| t.radius == radius)
    }

    /// Adds the ray table for viewsheds of `radius` cells (prompt 033).
    pub(crate) fn add_table(&mut self, radius: i32) {
        if !self.has_table(radius) {
            self.tables.push(RayTable::new(radius));
            self.tables.sort_by_key(|t| t.radius);
        }
    }

    /// The cells (grid indices) seen by an eye `eye_height_mm` above the
    /// ground at `p` with the sight range of `category`, the downhill
    /// bonus included: the viewshed a unit there gets in a match (prompt
    /// 113, for tools and tests such as `cli map-sight`). Adds the ray
    /// tables it needs.
    pub fn sight_from(
        &mut self,
        rules: &VisionRules,
        p: Vec2,
        eye_height_mm: i32,
        category: Option<Category>,
    ) -> Vec<u32> {
        let base = rules.range_cells(category);
        let reach = rules.height.reach_cells(base);
        self.add_table(base);
        self.add_table(reach);
        let anchor = self.index_of(p);
        let eye = self.ground_mm(anchor).saturating_add(eye_height_mm);
        let radius = self.downhill_radius(anchor, eye, base, reach);
        let (shed, _) = self.viewshed_downhill(anchor, eye, radius, base);
        let mut out = Vec::with_capacity(shed.count() as usize);
        shed.for_each(self, |i| out.push(i as u32));
        out.sort_unstable();
        out
    }

    /// Computes the viewshed of an eye at absolute height `eye_mm` in cell
    /// `anchor` with sight radius `radius` (cells). Returns it and the
    /// number of ray steps taken.
    pub(crate) fn viewshed(&self, anchor: u32, eye_mm: i32, radius: i32) -> (Viewshed, u32) {
        self.viewshed_downhill(anchor, eye_mm, radius, radius)
    }

    /// Like [`Self::viewshed`], but the eye sees cells beyond its sight
    /// range of `base` cells (up to `radius`, see
    /// [`HeightRules::reach_cells`]) whose ground lies far enough below it
    /// (prompt 060, [`HeightRules`]).
    pub(crate) fn viewshed_downhill(
        &self,
        anchor: u32,
        eye_mm: i32,
        radius: i32,
        base: i32,
    ) -> (Viewshed, u32) {
        let mut shed = Viewshed {
            anchor,
            eye_mm,
            radius,
            bits: Vec::new(),
        };
        let steps = self.viewshed_downhill_into(anchor, eye_mm, radius, base, &mut shed);
        (shed, steps)
    }

    /// Computes a downhill viewshed directly into `shed`, reusing its `bits`
    /// allocation (prompt 209).
    pub(crate) fn viewshed_downhill_into(
        &self,
        anchor: u32,
        eye_mm: i32,
        radius: i32,
        base: i32,
        shed: &mut Viewshed,
    ) -> u32 {
        let table = self
            .tables
            .iter()
            .find(|t| t.radius == radius)
            .expect("ray table for every sight radius");
        let side = (2 * radius + 1) as usize;
        let words = (side * side).div_ceil(64);
        shed.anchor = anchor;
        shed.eye_mm = eye_mm;
        shed.radius = radius;
        shed.bits.clear();
        shed.bits.resize(words, 0);
        shed.set(0, 0);
        let ac = (anchor % self.columns) as i32;
        let ar = (anchor / self.columns) as i32;
        let eye = i64::from(eye_mm);
        let target = i64::from(self.target_height_mm);
        // A sight line may cross `depth` forest cells whose canopy is above
        // it; the (depth + 1)-th steepest canopy slope is the forest horizon.
        let keep = self.foliage_depth as usize + 1;
        let mut steps = 0u32;
        // Steepest canopy slopes of the ray so far, descending: (num, den).
        // Stack buffer avoids a per-viewshed heap allocation for all normal
        // `foliage_depth` values (`keep <= 16`).
        let mut canopy_buf = [(0i64, 1i64); 17];
        let mut canopy_heap: Vec<(i64, i64)> = if keep > 16 {
            Vec::with_capacity(keep + 1)
        } else {
            Vec::new()
        };
        // `a/b > c/d` for positive `b`, `d`.
        let steeper = |(a, b): (i64, i64), (c, d): (i64, i64)| a * d > c * b;
        let base2 = i64::from(base) * i64::from(base);
        // Beyond the sight range only ground this far below the line from
        // the eye at the target height can be seen (prompt 060).
        let min_rise = target - i64::from(self.height.min_drop_m) * 1000;
        let has_decks = !self.deck_low_mm.is_empty();
        for ray in table.starts.windows(2) {
            // Horizon of ground and solid obstacles; `None` = open.
            let mut horizon: Option<(i64, i64)> = None;
            let mut canopy_len = 0usize;
            if keep > 16 {
                canopy_heap.clear();
            }
            let ray_steps = i64::from(ray[1] - ray[0]);
            for (k, &(dx, dy)) in (1i32..).zip(&table.offsets[ray[0] as usize..ray[1] as usize]) {
                let c = ac + i32::from(dx);
                let r = ar + i32::from(dy);
                if c < 0 || r < 0 || c >= self.columns as i32 || r >= self.rows as i32 {
                    break;
                }
                let d2 = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
                let far = d2 > base2;
                // Beyond the sight range a cell needs ground below the eye
                // by more than `min_drop` and above the horizon. The horizon
                // only rises along the ray, so once `horizon * ray_steps`
                // is above `target - min_drop` no later cell qualifies.
                if far && horizon.is_some_and(|(num, den)| num * ray_steps > min_rise * den) {
                    break;
                }
                steps += 1;
                let i = (r as u32 * self.columns + c as u32) as usize;
                let ground = i64::from(self.ground_mm[i]);
                let kk = i64::from(k);
                let line = (ground + target - eye, kk);
                // A target inside smoke is hidden unless it is next to the
                // eye (prompt 032).
                let smoke = self.smoke_mm[i];
                let smoked = smoke != NONE && k > 1 && ground + target < i64::from(smoke);
                let in_range = !far || {
                    let pct = 100 + self.height.bonus_pct(eye - ground);
                    d2 * 10_000 <= base2 * pct * pct
                };
                let canopy_blocks = if keep <= 16 {
                    canopy_len >= keep && steeper(canopy_buf[keep - 1], line)
                } else {
                    canopy_heap.len() >= keep && steeper(canopy_heap[keep - 1], line)
                };
                let visible = in_range
                    && !smoked
                    && horizon.is_none_or(|h| !steeper(h, line))
                    && !canopy_blocks;
                if visible {
                    shed.set(i32::from(dx), i32::from(dy));
                }
                let mut raise = |top: i64| {
                    let slope = (top - eye, kk);
                    if horizon.is_none_or(|h| steeper(slope, h)) {
                        horizon = Some(slope);
                    }
                };
                if has_decks {
                    let low = self.deck_low_mm[i];
                    if low != NONE {
                        raise(i64::from(low));
                    } else {
                        raise(ground);
                    }
                } else {
                    raise(ground);
                }
                if self.solid_mm[i] != NONE {
                    raise(i64::from(self.solid_mm[i]));
                }
                if smoke != NONE {
                    raise(i64::from(smoke));
                }
                if self.foliage_mm[i] != NONE {
                    let slope = (i64::from(self.foliage_mm[i]) - eye, kk);
                    if keep <= 16 {
                        let pos = canopy_buf[..canopy_len]
                            .iter()
                            .position(|&s| steeper(slope, s))
                            .unwrap_or(canopy_len);
                        if pos < keep {
                            let end = canopy_len.min(keep - 1);
                            if end > pos {
                                canopy_buf.copy_within(pos..end, pos + 1);
                            }
                            canopy_buf[pos] = slope;
                            if canopy_len < keep {
                                canopy_len += 1;
                            }
                        }
                    } else {
                        let at = canopy_heap.iter().position(|&s| steeper(slope, s));
                        canopy_heap.insert(at.unwrap_or(canopy_heap.len()), slope);
                        canopy_heap.truncate(keep);
                    }
                }
            }
        }
        steps
    }
}

/// The cells one unit sees: a bit field over the square around its anchor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Viewshed {
    /// Cell index of the eye.
    pub(crate) anchor: u32,
    pub(crate) eye_mm: i32,
    pub(crate) radius: i32,
    bits: Vec<u64>,
}

impl Viewshed {
    const fn bit(&self, dx: i32, dy: i32) -> usize {
        let side = 2 * self.radius + 1;
        ((dy + self.radius) * side + dx + self.radius) as usize
    }

    fn set(&mut self, dx: i32, dy: i32) {
        let b = self.bit(dx, dy);
        self.bits[b / 64] |= 1 << (b % 64);
    }

    /// Whether cell `(c, r)` of `grid` is in the viewshed.
    pub(crate) fn contains(&self, grid: &VisionGrid, c: i32, r: i32) -> bool {
        let dx = c - (self.anchor % grid.columns) as i32;
        let dy = r - (self.anchor / grid.columns) as i32;
        if dx.abs() > self.radius || dy.abs() > self.radius {
            return false;
        }
        let b = self.bit(dx, dy);
        self.bits[b / 64] & (1 << (b % 64)) != 0
    }

    /// Calls `f` with the grid index of every visible cell.
    pub(crate) fn for_each(&self, grid: &VisionGrid, mut f: impl FnMut(usize)) {
        let side = 2 * self.radius + 1;
        let ac = (self.anchor % grid.columns) as i32;
        let ar = (self.anchor / grid.columns) as i32;
        for (w, &word) in self.bits.iter().enumerate() {
            let mut word = word;
            while word != 0 {
                let b = w * 64 + word.trailing_zeros() as usize;
                word &= word - 1;
                let b = b as i32;
                let c = ac + b % side - self.radius;
                let r = ar + b / side - self.radius;
                // Rays stop at the map edge, so every set cell is inside.
                f((r as u32 * grid.columns + c as u32) as usize);
            }
        }
    }

    /// Number of visible cells.
    pub(crate) fn count(&self) -> u32 {
        self.bits.iter().map(|w| w.count_ones()).sum()
    }
}

/// What a player knows about a vision cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CellState {
    Unexplored = 0,
    /// Seen before, not now.
    Explored = 1,
    Visible = 2,
}

/// Vision of one player: how many own viewsheds contain each cell, and
/// which cells were ever seen.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PlayerVision {
    counts: Vec<u32>,
    explored: Vec<u64>,
    /// Increased whenever a cell changes its [`CellState`].
    pub(crate) version: u64,
}

impl PlayerVision {
    pub(crate) fn new(cells: usize) -> Self {
        Self {
            counts: vec![0; cells],
            explored: vec![0; cells.div_ceil(64)],
            version: 0,
        }
    }

    pub(crate) fn add(&mut self, grid: &VisionGrid, shed: &Viewshed) {
        let mut changed = false;
        shed.for_each(grid, |i| {
            self.counts[i] += 1;
            if self.counts[i] == 1 {
                changed = true;
                self.explored[i / 64] |= 1 << (i % 64);
            }
        });
        self.version += u64::from(changed);
    }

    pub(crate) fn remove(&mut self, grid: &VisionGrid, shed: &Viewshed) {
        let mut changed = false;
        shed.for_each(grid, |i| {
            self.counts[i] -= 1;
            changed |= self.counts[i] == 0;
        });
        self.version += u64::from(changed);
    }

    pub(crate) fn visible(&self, i: usize) -> bool {
        self.counts[i] > 0
    }

    pub(crate) fn state(&self, i: usize) -> CellState {
        if self.counts[i] > 0 {
            CellState::Visible
        } else if self.explored[i / 64] & (1 << (i % 64)) != 0 {
            CellState::Explored
        } else {
            CellState::Unexplored
        }
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        // Counts follow from the hashed viewsheds; the explored cells are
        // history, so they are hashed as a summary.
        let explored: u32 = self.explored.iter().map(|w| w.count_ones()).sum();
        h.write_u32(explored);
    }
}

/// Meters to whole millimeters (as [`VisionGrid::new`] rounds heights).
fn to_mm(h: Fixed) -> i32 {
    (h * Fixed::from_num(1000)).round().to_num::<i32>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = VisionRules::builtin().unwrap();
        // 2000 m real × 1/8 = 250 m, 31 cells of 8 m (prompt 097).
        assert_eq!(rules.range_m(None), Fixed::from_num(250));
        assert_eq!(rules.range_cells(None), 31);
        assert_eq!(
            rules.range_m(Some(Category::ReconVehicle)),
            Fixed::from_num(350)
        );
        assert_eq!(rules.object_sight("env_hedge"), Sight::Solid);
        assert_eq!(rules.object_sight("env_tree_conifer"), Sight::Foliage);
        assert_eq!(rules.object_sight("env_bush"), Sight::Clear);
    }

    #[test]
    fn rays_cover_every_cell_of_the_circle() {
        for radius in [1, 2, 5, 17, 62] {
            let table = RayTable::new(radius);
            let side = (2 * radius + 1) as usize;
            let mut hit = vec![false; side * side];
            for &(dx, dy) in &table.offsets {
                hit[(i32::from(dy) + radius) as usize * side + (i32::from(dx) + radius) as usize] =
                    true;
            }
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if (dx, dy) != (0, 0) && dx * dx + dy * dy <= radius * radius {
                        let i = (dy + radius) as usize * side + (dx + radius) as usize;
                        assert!(hit[i], "radius {radius}: cell ({dx}, {dy}) missed");
                    }
                }
            }
        }
    }

    #[test]
    fn downhill_bonus_follows_the_rules() {
        let h = VisionRules::builtin().unwrap().height;
        assert_eq!(h.reach_cells(62), 78);
        assert_eq!(h.bonus_pct(0), 0);
        assert_eq!(h.bonus_pct(20_000), 0);
        // 5 % per 10 m beyond the first 20 m, at most 25 %.
        assert_eq!(h.bonus_pct(40_000), 10);
        assert_eq!(h.bonus_pct(69_999), 24);
        assert_eq!(h.bonus_pct(300_000), 25);
    }

    #[test]
    fn rounding_is_symmetric() {
        assert_eq!(round_div(5, 2), 3);
        assert_eq!(round_div(-5, 2), -3);
        assert_eq!(round_div(4, 3), 1);
        assert_eq!(round_div(-4, 3), -1);
    }
}
