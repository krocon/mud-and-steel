//! Landscape maps of prompt 056: `cli gen-land --map <name>`.
//!
//! Three 1 km maps for two or four players (two teams), each built around
//! real places that carry the supply points:
//!
//! - `winter_village`: a snowed-in village (biome `winter`, default
//!   weather snow) with a plowed main road, the church at the **village
//!   square**, a **trench strongpoint** on a hill in the east and a
//!   **farmstead** in the west; conifer woods, bare orchards and snow
//!   drifts (terrain `snow`) in the open.
//! - `farms`: rolling farmland with five **farmsteads** (farmhouse, barn,
//!   hay, yard) joined by field tracks lined with hedgerows, haystacks in
//!   the fields, orchards and small woods.
//! - `river_land`: a river from north to south between the teams (west
//!   and east), a stone bridge with a hamlet at both ends in the middle and
//!   two timber bridges (each a **bridgehead**), fords near the map edges,
//!   riverside woods and a farmstead on each side.
//!
//! Supply point markers (`supply_*`) stand on these places; the objects
//! there (houses, barns, the trench strongpoint of a `strongpoint_*`
//! marker) are placed with them. Writes `map.toml`, `height.png`,
//! `terrain.png` and `objects.csv` (format: `docs/MAPS.md`).
//!
//! Like `gen_map` it runs outside the simulation and may use floats, but
//! only basic arithmetic and `sqrt` (exactly rounded by IEEE 754) and the
//! simulation's random generator, so the same seed gives the same files
//! everywhere. No real place names.

use crate::gen_map::{Random, write_png};
use editor::PlacedObject;
use editor::compound::half_extent;
use sim::objects::CSV_HEADER;
use sim::{ObjectRules, Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;
const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const SAND: u8 = 3;
const MUD: u8 = 4;
const SNOW: u8 = 5;
const SHALLOW_WATER: u8 = 6;
const DEEP_WATER: u8 = 7;
const TALL_GRASS: u8 = 8;

/// Every map is this wide and deep (m).
pub const SIZE_M: u32 = 1000;
const HALF: f64 = 500.0;
const SAMPLES: usize = 501;

/// Radius kept flat and free of objects around each start (m).
const START_CLEAR_M: f64 = 60.0;
/// Radius kept free of objects around a strongpoint marker (trench ring
/// 16 m, wire ring 30 m; prompt 052).
const STRONGPOINT_CLEAR_M: f64 = 42.0;

/// River (river_land): banks cut this far below the ground (m) over
/// `RIVER_VALLEY_M` beyond the water; like `gen_map`.
const BANK_CUT_M: f64 = 3.0;
const RIVER_VALLEY_M: f64 = 40.0;
const WATERLINE_M: f64 = 0.15;
const SHALLOW_DEPTH_M: f64 = 0.5;
const CHANNEL_DEPTH_M: f64 = 1.3;
const FORD_HALF_WIDTH_M: f64 = 7.0;
const FORD_BLEND_M: f64 = 6.0;
/// Bridges: water narrowed within this distance of the axis, tapering out.
const BRIDGE_NARROW_M: f64 = 15.0;
const BRIDGE_TAPER_M: f64 = 60.0;
const BRIDGE_LAND_M: f64 = 4.0;
const RAMP_GRADE: f64 = 0.12;
const EMBANKMENT_SIDE_GRADE: f64 = 0.5;

/// The maps of prompt 056.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LandMap {
    WinterVillage,
    Farms,
    RiverLand,
}

impl LandMap {
    pub const ALL: [Self; 3] = [Self::WinterVillage, Self::Farms, Self::RiverLand];

    pub fn parse(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|m| m.name() == name)
            .ok_or_else(|| format!("unknown map {name:?}: winter_village, farms or river_land"))
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::WinterVillage => "winter_village",
            Self::Farms => "farms",
            Self::RiverLand => "river_land",
        }
    }

    /// The seed the committed map was generated with.
    pub const fn default_seed(self) -> u64 {
        match self {
            Self::WinterVillage => 56,
            Self::Farms => 57,
            Self::RiverLand => 58,
        }
    }
}

/// Parameters of `cli gen-land`.
pub struct Options<'a> {
    pub map: LandMap,
    pub seed: u64,
    pub out: &'a Path,
}

fn coord(index: usize) -> f64 {
    index as f64 * CELL_M - HALF
}

fn index(x: f64, y: f64) -> usize {
    let c = |v: f64| {
        ((v + HALF) / CELL_M)
            .round()
            .clamp(0.0, (SAMPLES - 1) as f64) as usize
    };
    c(y) * SAMPLES + c(x)
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt()
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Compass heading (0 = north, clockwise, whole degrees) of the direction
/// `(dx, dy)` in map coordinates (y south). Uses a polynomial arctangent
/// (error below 0.3°) instead of `atan2`, whose last bit may differ between
/// platforms.
fn heading_of(dx: f64, dy: f64) -> u16 {
    let (east, north) = (dx, -dy);
    let (ax, ay) = (east.abs(), north.abs());
    if ax == 0.0 && ay == 0.0 {
        return 0;
    }
    // Angle from north towards east within the first quadrant (degrees).
    let quarter = if ax <= ay {
        let t = ax / ay;
        (180.0 / std::f64::consts::PI) * t * (std::f64::consts::FRAC_PI_4 + 0.273 * (1.0 - t))
    } else {
        let t = ay / ax;
        90.0 - (180.0 / std::f64::consts::PI)
            * t
            * (std::f64::consts::FRAC_PI_4 + 0.273 * (1.0 - t))
    };
    let deg = match (east >= 0.0, north >= 0.0) {
        (true, true) => quarter,
        (true, false) => 180.0 - quarter,
        (false, false) => 180.0 + quarter,
        (false, true) => 360.0 - quarter,
    };
    (deg.round() as u16) % 360
}

/// Smooth value noise in `0..1` with features of about `scale` meters.
struct Noise {
    salt: u64,
    scale: f64,
}

impl Noise {
    fn lattice(&self, ix: i64, iy: i64) -> f64 {
        let mut z = self.salt
            ^ (ix as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (iy as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }

    fn at(&self, x: f64, y: f64) -> f64 {
        let (fx, fy) = (x / self.scale + 1000.0, y / self.scale + 1000.0);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let (ix, iy) = (x0 as i64, y0 as i64);
        let a = self.lattice(ix, iy);
        let b = self.lattice(ix + 1, iy);
        let c = self.lattice(ix, iy + 1);
        let d = self.lattice(ix + 1, iy + 1);
        let top = a + (b - a) * sx;
        let bottom = c + (d - c) * sx;
        top + (bottom - top) * sy
    }
}

/// A road or track: a polyline with a half width and a terrain id.
struct Way {
    points: Vec<(f64, f64)>,
    half: f64,
    id: u8,
}

impl Way {
    fn new(points: &[(f64, f64)], half: f64, id: u8) -> Self {
        Self {
            points: points.to_vec(),
            half,
            id,
        }
    }

    fn distance(&self, x: f64, y: f64) -> f64 {
        self.points
            .windows(2)
            .map(|s| segment_distance((x, y), s[0], s[1]))
            .fold(f64::INFINITY, f64::min)
    }
}

fn segment_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// A bridge crossing of the river: axis east-west at `y` (a path cell
/// center line), water narrowed to `half_width`.
#[derive(Clone, Copy)]
struct Crossing {
    y: f64,
    half_width: f64,
    stone: bool,
}

/// The river of `river_land`: roughly north-south, straight at crossings.
struct River {
    bends: Vec<f64>,
    half_width: f64,
    crossings: Vec<Crossing>,
}

impl River {
    fn center_x(&self, y: f64) -> f64 {
        let t = ((y + HALF) / 125.0).clamp(0.0, (self.bends.len() - 1) as f64);
        let i = (t as usize).min(self.bends.len() - 2);
        let f = t - i as f64;
        let f = f * f * (3.0 - 2.0 * f);
        let raw = self.bends[i] * (1.0 - f) + self.bends[i + 1] * f;
        let fade = self
            .crossings
            .iter()
            .map(|c| smoothstep(35.0, 110.0, (y - c.y).abs()))
            .fold(1.0, f64::min);
        raw * fade
    }

    fn half_width_at(&self, y: f64) -> f64 {
        let w = self.half_width;
        self.crossings.iter().fold(w, |acc, c| {
            let blend = smoothstep(BRIDGE_NARROW_M, BRIDGE_TAPER_M, (y - c.y).abs());
            acc.min(c.half_width + (w - c.half_width) * blend)
        })
    }
}

/// Everything that shapes the ground of one map.
struct Plan {
    bumps: Vec<(f64, f64, f64, f64)>,
    /// Flat areas: center and radius, at the natural height of the center.
    flats: Vec<((f64, f64), f64)>,
    flat_levels: Vec<f64>,
    /// Paved squares: center and half edge.
    squares: Vec<((f64, f64), f64)>,
    ways: Vec<Way>,
    river: Option<River>,
    /// Fords of the river: y of the crossing track.
    fords: Vec<f64>,
    ground: Noise,
    detail: Noise,
    mud: Noise,
    drift: Noise,
    /// Snow drifts (terrain `snow`) in the open where `drift` exceeds this.
    drift_level: f64,
    mud_level: f64,
}

impl Plan {
    fn natural(&self, x: f64, y: f64) -> f64 {
        let mut h = 2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y);
        for &(bx, by, r, a) in &self.bumps {
            let d2 = ((x - bx) * (x - bx) + (y - by) * (y - by)) / (r * r);
            if d2 < 1.0 {
                let t = 1.0 - d2;
                h += a * t * t;
            }
        }
        h
    }

    fn cache_levels(&mut self) {
        self.flat_levels = self
            .flats
            .iter()
            .map(|&(c, _)| self.natural(c.0, c.1))
            .collect();
    }

    fn sample(&self, x: f64, y: f64) -> (f64, u8) {
        let mut h = self.natural(x, y);
        for (k, &(c, r)) in self.flats.iter().enumerate() {
            let level = self.flat_levels[k];
            let blend = smoothstep(r, r + 40.0, dist((x, y), c));
            h = level + (h - level) * blend;
        }
        let mut terrain = GRASS;
        if self
            .squares
            .iter()
            .any(|&(c, e)| (x - c.0).abs() <= e && (y - c.1).abs() <= e)
        {
            terrain = ROAD;
        }
        if terrain == GRASS {
            for way in &self.ways {
                if way.distance(x, y) < way.half {
                    terrain = way.id;
                    break;
                }
            }
        }
        if let Some(river) = &self.river {
            let w = river.half_width_at(y);
            let r = (x - river.center_x(y)).abs();
            if r < w + RIVER_VALLEY_M {
                let reach = FORD_HALF_WIDTH_M + FORD_BLEND_M;
                let ford_distance = self
                    .fords
                    .iter()
                    .map(|f| (y - f).abs())
                    .fold(f64::INFINITY, f64::min);
                let ford = ford_distance < FORD_HALF_WIDTH_M;
                let channel = 1.0 - smoothstep(reach, FORD_HALF_WIDTH_M, ford_distance);
                h -= if r < w {
                    BANK_CUT_M
                        + WATERLINE_M
                        + SHALLOW_DEPTH_M * smoothstep(w, w * 0.55, r)
                        + CHANNEL_DEPTH_M * channel * smoothstep(w * 0.55, w * 0.25, r)
                } else {
                    BANK_CUT_M * (1.0 - smoothstep(w, w + RIVER_VALLEY_M, r))
                };
                terrain = if r < w * 0.55 && !ford {
                    DEEP_WATER
                } else if r < w {
                    SHALLOW_WATER
                } else if r < w + 3.0 {
                    SAND
                } else if r < w + 14.0 && terrain == GRASS && self.mud.at(x, y) > 0.55 {
                    MUD
                } else {
                    terrain
                };
                return (h, terrain);
            }
        }
        if terrain == GRASS {
            if self.mud.at(x, y) > self.mud_level {
                terrain = MUD;
            } else if self.drift.at(x, y) > self.drift_level {
                terrain = SNOW;
            }
        }
        (h, terrain)
    }
}

/// A placed bridge (`objects.csv` row).
struct Bridge {
    kind: &'static str,
    variant: &'static str,
    x: f64,
    y: f64,
}

/// Measures the water at every crossing, picks the shortest bridge that
/// spans it plus 4 m of bank on each side and raises embankments with ramps
/// to the deck (as `gen_map`, prompt 014).
fn place_bridges(
    river: &River,
    heights: &mut [f64],
    terrain: &mut [u8],
) -> Result<Vec<Bridge>, String> {
    const WOOD: [(&str, f64); 3] = [("Span8", 8.0), ("Span12", 12.0), ("Span16", 16.0)];
    const STONE: [(&str, f64); 2] = [("Arch2", 20.0), ("Arch3", 30.0)];
    let mut placed = Vec::new();
    for c in &river.crossings {
        let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
        for j in 0..SAMPLES {
            let y = coord(j);
            if (y - c.y).abs() > 5.0 {
                continue;
            }
            for i in 0..SAMPLES {
                let x = coord(i);
                if x.abs() < 40.0 && matches!(terrain[j * SAMPLES + i], SHALLOW_WATER | DEEP_WATER)
                {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                }
            }
        }
        if x0 > x1 {
            return Err(format!("no water at the crossing y = {}", c.y));
        }
        let need = x1 - x0 + 2.0 * BRIDGE_LAND_M;
        let list: &[(&str, f64)] = if c.stone { &STONE } else { &WOOD };
        let Some(&(variant, length)) = list.iter().find(|v| v.1 >= need) else {
            return Err(format!(
                "the river at y = {} is {:.1} m wide, too wide for the bridge",
                c.y,
                x1 - x0
            ));
        };
        let (rise, half_width) = if c.stone { (3.0, 2.55) } else { (1.5, 2.05) };
        let cx = ((x0 + x1) / 2.0 * 1000.0).round() / 1000.0;
        let (d0, d1) = (cx - length / 2.0, cx + length / 2.0);
        let bank = heights[index(d0, c.y)].max(heights[index(d1, c.y)]);
        let deck = bank + rise;
        let top_half = half_width + 1.0;
        let reach_x = rise / RAMP_GRADE + 10.0;
        let reach_y = top_half + rise / EMBANKMENT_SIDE_GRADE + 4.0;
        for j in 0..SAMPLES {
            let y = coord(j);
            let dy = (y - c.y).abs();
            if dy > reach_y {
                continue;
            }
            for i in 0..SAMPLES {
                let x = coord(i);
                if x < d0 - reach_x || x > d1 + reach_x {
                    continue;
                }
                let k = j * SAMPLES + i;
                if matches!(terrain[k], SHALLOW_WATER | DEEP_WATER) {
                    continue;
                }
                let along = if x < d0 {
                    d0 - x
                } else if x > d1 {
                    x - d1
                } else {
                    0.0
                };
                let top = deck - RAMP_GRADE * along;
                let level = top - EMBANKMENT_SIDE_GRADE * (dy - top_half).max(0.0);
                if level > heights[k] {
                    heights[k] = level;
                    terrain[k] = if dy <= 2.5 {
                        if c.stone { ROAD } else { DIRT_TRACK }
                    } else {
                        GRASS
                    };
                }
            }
        }
        placed.push(Bridge {
            kind: if c.stone {
                "env_bridge_stone"
            } else {
                "env_bridge_wood"
            },
            variant,
            x: cx,
            y: c.y,
        });
    }
    Ok(placed)
}

/// Axis-aligned box in meters.
#[derive(Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

const BUCKET_M: f64 = 16.0;
const BUCKETS: usize = (SIZE_M as usize).div_ceil(16) + 1;

/// Objects of a map: placed without overlaps, off roads, tracks and water.
struct Objects<'a> {
    rules: &'a ObjectRules,
    terrain: &'a [u8],
    random: Random,
    rows: Vec<String>,
    rects: Vec<Rect>,
    buckets: Vec<Vec<usize>>,
    /// Circles kept free of objects (starts, strongpoints).
    reserved: Vec<((f64, f64), f64)>,
    buildings: usize,
    trees: usize,
    other: usize,
}

impl<'a> Objects<'a> {
    fn new(rules: &'a ObjectRules, terrain: &'a [u8], seed: u64) -> Self {
        Self {
            rules,
            terrain,
            random: Random(Rng::new(seed ^ 0x6c61_6e64)),
            rows: Vec::new(),
            rects: Vec::new(),
            buckets: vec![Vec::new(); BUCKETS * BUCKETS],
            reserved: Vec::new(),
            buildings: 0,
            trees: 0,
            other: 0,
        }
    }

    fn terrain_at(&self, x: f64, y: f64) -> u8 {
        self.terrain[index(x, y)]
    }

    fn bucket_range(r: &Rect) -> (usize, usize, usize, usize) {
        let b = |v: f64| {
            ((v + HALF) / BUCKET_M)
                .floor()
                .clamp(0.0, (BUCKETS - 1) as f64) as usize
        };
        (b(r.x0), b(r.y0), b(r.x1), b(r.y1))
    }

    /// Places an object unless it would overlap another one within
    /// `margin` m, leave the map, stand in a reserved circle or on a road,
    /// track or water. Returns whether it was placed.
    fn place(
        &mut self,
        kind: &str,
        variant: &str,
        (x, y): (f64, f64),
        heading: u16,
        margin: f64,
    ) -> bool {
        let (x_mm, y_mm) = ((x * 1000.0).round() as i32, (y * 1000.0).round() as i32);
        let object = PlacedObject {
            kind: kind.into(),
            variant: variant.into(),
            x_mm,
            y_mm,
            heading_deg: heading % 360,
        };
        let (ex, ey) = half_extent(self.rules, &object);
        let (hx, hy) = ((ex as f64 / 1000.0).max(0.3), (ey as f64 / 1000.0).max(0.3));
        let rect = Rect {
            x0: x - hx,
            y0: y - hy,
            x1: x + hx,
            y1: y + hy,
        };
        let limit = HALF - 3.0;
        if rect.x0 < -limit || rect.x1 > limit || rect.y0 < -limit || rect.y1 > limit {
            return false;
        }
        if self.reserved.iter().any(|&(c, r)| {
            dist(
                (c.0.clamp(rect.x0, rect.x1), c.1.clamp(rect.y0, rect.y1)),
                c,
            ) < r
        }) {
            return false;
        }
        for (cx, cy) in [
            (rect.x0, rect.y0),
            (rect.x1, rect.y0),
            (rect.x0, rect.y1),
            (rect.x1, rect.y1),
            (x, y),
        ] {
            if matches!(
                self.terrain_at(cx, cy),
                DIRT_TRACK | ROAD | SHALLOW_WATER | DEEP_WATER
            ) {
                return false;
            }
        }
        let probe = Rect {
            x0: rect.x0 - margin,
            y0: rect.y0 - margin,
            x1: rect.x1 + margin,
            y1: rect.y1 + margin,
        };
        let (i0, j0, i1, j1) = Self::bucket_range(&probe);
        for j in j0..=j1 {
            for i in i0..=i1 {
                for &k in &self.buckets[j * BUCKETS + i] {
                    let o = &self.rects[k];
                    if probe.x0 < o.x1 && o.x0 < probe.x1 && probe.y0 < o.y1 && o.y0 < probe.y1 {
                        return false;
                    }
                }
            }
        }
        let id = self.rects.len();
        self.rects.push(rect);
        let (i0, j0, i1, j1) = Self::bucket_range(&rect);
        for j in j0..=j1 {
            for i in i0..=i1 {
                self.buckets[j * BUCKETS + i].push(id);
            }
        }
        self.rows.push(format!(
            "{kind},{variant},{x_mm},{y_mm},{}",
            object.heading_deg
        ));
        if kind.starts_with("env_tree") {
            self.trees += 1;
        } else if kind.starts_with("env_house")
            || matches!(kind, "env_barn" | "env_church" | "env_warehouse")
        {
            self.buildings += 1;
        } else {
            self.other += 1;
        }
        true
    }

    fn pick<'k>(&mut self, list: &[&'k str]) -> &'k str {
        let i = (self.random.unit() * list.len() as f64) as usize;
        list[i.min(list.len() - 1)]
    }

    fn heading(&mut self) -> u16 {
        (self.random.unit() * 360.0) as u16 % 360
    }

    /// Houses along an axis-aligned street from `a` to `b` on one side
    /// (`side` +1: east or south of it, -1: west or north), long side
    /// along the street, doors (model +X) towards it.
    fn street(
        &mut self,
        (a, b): ((f64, f64), (f64, f64)),
        side: f64,
        street_half: f64,
        kinds: &[(&str, f64, f64)],
        damaged: f64,
        gardens: &Garden,
    ) -> usize {
        let along_x = (b.1 - a.1).abs() < (b.0 - a.0).abs();
        let (from, to) = if along_x { (a.0, b.0) } else { (a.1, b.1) };
        let (from, to) = (from.min(to), from.max(to));
        let mut u = from + self.random.range(0.0, 6.0);
        let mut n = 0;
        while u < to {
            let (kind, length, width) =
                kinds[(self.random.unit() * kinds.len() as f64) as usize % kinds.len()];
            let setback = 3.0 + self.random.range(0.0, 3.0);
            let offset = street_half + setback + width / 2.0;
            let c = u + length / 2.0;
            if c + length / 2.0 > to {
                break;
            }
            let (x, y, heading) = if along_x {
                (c, a.1 + side * offset, if side < 0.0 { 90 } else { 270 })
            } else {
                (a.0 + side * offset, c, if side < 0.0 { 0 } else { 180 })
            };
            let roll = self.random.unit();
            let variant = if roll < damaged * 0.35 {
                "Ruin"
            } else if roll < damaged {
                "Damaged"
            } else {
                "Intact"
            };
            if self.place(kind, variant, (x, y), heading, 1.0) {
                n += 1;
                let back = offset + width / 2.0 + 5.0;
                let (gx, gy, gdx, gdy) = if along_x {
                    (c, a.1 + side * back, 1.0, 0.0)
                } else {
                    (a.0 + side * back, c, 0.0, 1.0)
                };
                let (nx, ny) = if along_x { (0.0, side) } else { (side, 0.0) };
                self.garden((gx, gy), (gdx, gdy), (nx, ny), length, gardens);
            }
            u += length + self.random.range(5.0, 12.0);
        }
        n
    }

    /// A garden behind a house: woodpile or hay, a fence along the back,
    /// a few fruit trees.
    fn garden(
        &mut self,
        (x, y): (f64, f64),
        (dx, dy): (f64, f64),
        (nx, ny): (f64, f64),
        length: f64,
        g: &Garden,
    ) {
        let heading = heading_of(dx, dy);
        if self.random.unit() < 0.5 {
            self.place(
                "env_woodpile",
                "Stack",
                (x + dx * 3.0 + nx, y + dy * 3.0 + ny),
                heading,
                0.5,
            );
        } else {
            let hay = if self.random.unit() < 0.5 {
                "Rick"
            } else {
                "Round"
            };
            self.place(
                "env_hay_bale",
                hay,
                (x + nx * 3.0, y + ny * 3.0),
                heading,
                0.8,
            );
        }
        let trees = 1 + (self.random.unit() * 3.0) as usize;
        for t in 0..trees {
            let s = -length / 2.0 + (t as f64 + 0.5) * length / trees as f64;
            let depth = 9.0 + self.random.range(0.0, 3.0);
            let tree = self.pick(g.trees);
            let h = self.heading();
            self.place(
                g.tree_kind,
                tree,
                (x + dx * s + nx * depth, y + dy * s + ny * depth),
                h,
                1.5,
            );
        }
        if !g.fence.is_empty() {
            // Fence along the back of the garden.
            let back = 14.0;
            let segments = (length / 2.5) as i32;
            for k in 0..segments {
                if self.random.unit() < 0.15 {
                    continue;
                }
                let s = -length / 2.0 + (f64::from(k) + 0.5) * 2.5;
                self.place(
                    g.fence,
                    "Segment",
                    (x + dx * s + nx * back, y + dy * s + ny * back),
                    heading,
                    -0.05,
                );
            }
        }
    }

    /// A farmstead around a yard at `c`: farmhouse west (door east), barn
    /// east (doors west), a shed or second house north, hay ricks, firewood,
    /// crates and barrels; a fence or wall around. Returns the number of
    /// buildings placed.
    fn farmstead(&mut self, c: (f64, f64), g: &Garden, house: &str, damaged: bool) -> usize {
        let variant = if damaged { "Damaged" } else { "Intact" };
        let mut n = 0;
        n += usize::from(self.place(house, "Intact", (c.0 - 13.0, c.1 - 2.0), 0, 0.5));
        n += usize::from(self.place("env_barn", variant, (c.0 + 15.0, c.1), 180, 0.5));
        n += usize::from(self.place("env_house_log", "Intact", (c.0 + 1.0, c.1 - 17.0), 90, 0.5));
        for k in 0..4 {
            let hay = if k % 2 == 0 { "Rick" } else { "Round" };
            self.place(
                "env_hay_bale",
                hay,
                (c.0 - 14.0 + 6.0 * f64::from(k), c.1 + 20.0),
                90,
                0.6,
            );
        }
        self.place(
            "env_hay_bale_square",
            "Stack",
            (c.0 + 4.0, c.1 + 9.0),
            30,
            0.3,
        );
        self.place(
            "env_hay_bale_square",
            "Single",
            (c.0 + 6.0, c.1 + 11.0),
            75,
            0.2,
        );
        self.place("env_woodpile", "Stack", (c.0 - 5.0, c.1 - 11.0), 90, 0.4);
        self.place("env_crate", "Stack", (c.0 + 5.0, c.1 - 8.0), 0, 0.3);
        self.place("env_barrel", "Group", (c.0 + 7.0, c.1 - 6.0), 0, 0.3);
        // Fence around the farm (gaps east and west for vehicles).
        let (hx, hy): (f64, f64) = (34.0, 30.0);
        let fence = if g.fence.is_empty() {
            "env_fence_wood"
        } else {
            g.fence
        };
        let mut s = -hx + 1.25;
        while s < hx {
            for y in [c.1 - hy, c.1 + hy] {
                if s.abs() > 6.0 && self.random.unit() > 0.1 {
                    self.place(fence, "Segment", (c.0 + s, y), 90, -0.05);
                }
            }
            s += 2.5;
        }
        // Orchard south of the hay.
        let tree_kind = g.tree_kind;
        for j in 0..2 {
            for i in 0..5 {
                let tree = self.pick(g.trees);
                let h = self.heading();
                self.place(
                    tree_kind,
                    tree,
                    (
                        c.0 - 20.0 + 9.0 * f64::from(i),
                        c.1 + 38.0 + 9.0 * f64::from(j),
                    ),
                    h,
                    1.0,
                );
            }
        }
        n
    }

    /// Trees on a jittered grid of `spacing` m wherever `inside(x, y)`,
    /// keeping 6 m off roads and tracks.
    fn wood(
        &mut self,
        stands: &[(&str, &[&str])],
        spacing: f64,
        area: Rect,
        inside: &dyn Fn(f64, f64) -> bool,
        stand_noise: &Noise,
    ) -> usize {
        let mut n = 0;
        let mut y = area.y0;
        while y <= area.y1 {
            let mut x = area.x0;
            while x <= area.x1 {
                let px = x + self.random.range(-0.4, 0.4) * spacing;
                let py = y + self.random.range(-0.4, 0.4) * spacing;
                let roll = self.random.unit();
                let h = self.heading();
                x += spacing;
                if !inside(px, py) {
                    continue;
                }
                let clear = [(6.0, 0.0), (-6.0, 0.0), (0.0, 6.0), (0.0, -6.0)]
                    .iter()
                    .all(|(dx, dy)| {
                        !matches!(self.terrain_at(px + dx, py + dy), DIRT_TRACK | ROAD)
                    });
                if !clear {
                    continue;
                }
                let s =
                    ((stand_noise.at(px, py) * stands.len() as f64) as usize).min(stands.len() - 1);
                let (kind, variants) = stands[s];
                let v = variants[((roll * variants.len() as f64) as usize).min(variants.len() - 1)];
                if self.place(kind, v, (px, py), h, 0.8) {
                    n += 1;
                }
            }
            y += spacing;
        }
        n
    }

    /// A hedgerow 5.5 m beside a way on both sides, segments 2.7 m apart,
    /// in runs of 30-120 m with gaps of 10-20 m (wide enough for tanks).
    fn hedgerows(&mut self, way: &Way, offset: f64) {
        for side in [-1.0, 1.0] {
            let mut gap_left = self.random.range(0.0, 20.0);
            let mut run_left = 0.0;
            for s in way.points.windows(2) {
                let (a, b) = (s[0], s[1]);
                let len = dist(a, b);
                let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
                let (nx, ny) = (-dy * side, dx * side);
                let heading = heading_of(dx, dy);
                let mut t = 1.35;
                while t < len {
                    if gap_left > 0.0 {
                        gap_left -= 2.7;
                    } else {
                        if run_left <= 0.0 {
                            run_left = self.random.range(30.0, 120.0);
                        }
                        let p = (a.0 + dx * t + nx * offset, a.1 + dy * t + ny * offset);
                        let variant = if self.random.unit() < 0.04 {
                            "Corner"
                        } else {
                            "Segment"
                        };
                        self.place("env_hedge", variant, p, heading, -0.6);
                        run_left -= 2.7;
                        if run_left <= 0.0 {
                            gap_left = self.random.range(10.0, 20.0);
                        }
                    }
                    t += 2.7;
                }
            }
        }
    }

    /// Haystacks in groups in a field around `c`.
    fn hay_field(&mut self, c: (f64, f64), count: usize) {
        for _ in 0..count {
            let p = (
                c.0 + self.random.range(-35.0, 35.0),
                c.1 + self.random.range(-25.0, 25.0),
            );
            let roll = self.random.unit();
            let h = self.heading();
            let (kind, variant) = if roll < 0.45 {
                ("env_hay_bale", "Rick")
            } else if roll < 0.8 {
                ("env_hay_bale", "Round")
            } else {
                ("env_hay_bale_square", "Stack")
            };
            self.place(kind, variant, p, h, 2.0);
        }
    }

    /// Telegraph poles every 45 m beside a way.
    fn poles(&mut self, way: &Way, offset: f64) {
        for s in way.points.windows(2) {
            let (a, b) = (s[0], s[1]);
            let len = dist(a, b);
            let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let mut t = 10.0;
            while t < len {
                self.place(
                    "env_pole_telegraph",
                    "Pole",
                    (a.0 + dx * t - dy * offset, a.1 + dy * t + dx * offset),
                    0,
                    0.5,
                );
                t += 45.0;
            }
        }
    }
}

/// What gardens and farmsteads of a map grow.
struct Garden<'a> {
    tree_kind: &'a str,
    trees: &'a [&'a str],
    fence: &'a str,
}

/// The start positions: player, team, x, y (m).
type Start = (u8, u8, i32, i32);

/// North against south (winter village, farms).
const STARTS_NS: [Start; 4] = [
    (0, 0, -120, 420),
    (1, 1, 120, -420),
    (2, 0, 120, 420),
    (3, 1, -120, -420),
];
/// West against east (river land).
const STARTS_WE: [Start; 4] = [
    (0, 0, -420, -120),
    (1, 1, 420, 120),
    (2, 0, -420, 120),
    (3, 1, 420, -120),
];

/// A map's ground plan, metadata and object placement.
type Built = (Plan, Meta, Box<dyn Fn(&mut Objects)>);

/// What a map writes besides the rasters and objects.
struct Meta {
    biome: &'static str,
    weather: Option<&'static str>,
    starts: &'static [Start],
    markers: Vec<(String, (f64, f64))>,
}

fn noise(seed: u64, salt: u64, scale: f64) -> Noise {
    Noise {
        salt: seed.wrapping_mul(0x2545_f491_4f6c_dd1d) ^ salt,
        scale,
    }
}

fn base_plan(seed: u64, random: &mut Random, bumps: usize, amp: (f64, f64)) -> Plan {
    let bumps = (0..bumps)
        .map(|_| {
            (
                random.range(-HALF, HALF),
                random.range(-HALF, HALF),
                random.range(70.0, 170.0),
                random.range(amp.0, amp.1),
            )
        })
        .collect();
    Plan {
        bumps,
        flats: Vec::new(),
        flat_levels: Vec::new(),
        squares: Vec::new(),
        ways: Vec::new(),
        river: None,
        fords: Vec::new(),
        ground: noise(seed, 1, 140.0),
        detail: noise(seed, 2, 31.0),
        mud: noise(seed, 3, 45.0),
        drift: noise(seed, 4, 55.0),
        drift_level: 2.0,
        mud_level: 0.86,
    }
}

/// Rasters from the plan (starts flattened).
fn rasters(plan: &mut Plan, starts: &[Start]) -> (Vec<f64>, Vec<u8>) {
    for &(_, _, x, y) in starts {
        plan.flats
            .push(((f64::from(x), f64::from(y)), START_CLEAR_M));
    }
    plan.cache_levels();
    let mut heights = Vec::with_capacity(SAMPLES * SAMPLES);
    let mut terrain = Vec::with_capacity(SAMPLES * SAMPLES);
    for j in 0..SAMPLES {
        for i in 0..SAMPLES {
            let (h, t) = plan.sample(coord(i), coord(j));
            heights.push(h);
            terrain.push(t);
        }
    }
    // Starts on plain grass.
    for &(_, _, sx, sy) in starts {
        for j in 0..SAMPLES {
            for i in 0..SAMPLES {
                let k = j * SAMPLES + i;
                if dist((coord(i), coord(j)), (f64::from(sx), f64::from(sy))) < START_CLEAR_M
                    && matches!(terrain[k], MUD | SNOW | SAND)
                {
                    terrain[k] = GRASS;
                }
            }
        }
    }
    (heights, terrain)
}

fn reserve_starts(objects: &mut Objects, starts: &[Start]) {
    for &(_, _, x, y) in starts {
        objects
            .reserved
            .push(((f64::from(x), f64::from(y)), START_CLEAR_M - 5.0));
    }
}

const FULL: Rect = Rect {
    x0: -HALF + 4.0,
    y0: -HALF + 4.0,
    x1: HALF - 4.0,
    y1: HALF - 4.0,
};

// --- Winter village ---------------------------------------------------------

const WV_TRENCH: (f64, f64) = (300.0, 10.0);
const WV_FARM: (f64, f64) = (-300.0, -10.0);

fn winter_village(seed: u64) -> Result<Built, String> {
    let mut random = Random(Rng::new(seed ^ 0x7769_6e74));
    let mut plan = base_plan(seed, &mut random, 14, (-2.0, 5.0));
    plan.drift_level = 0.76;
    // Frozen ground: no mud.
    plan.mud_level = 2.0;
    // The strongpoint hill in the east, its top flat for the trenches.
    plan.bumps.push((WV_TRENCH.0, WV_TRENCH.1, 120.0, 11.0));
    plan.bumps.push((-WV_TRENCH.0 - 60.0, 160.0, 90.0, 7.0));
    plan.flats.push((WV_TRENCH, 36.0));
    plan.flats.push(((0.0, 0.0), 150.0));
    plan.flats.push((WV_FARM, 48.0));
    plan.squares.push(((0.0, 0.0), 22.0));
    // Plowed main road north-south, the village street east-west, field
    // tracks to the farmstead, the hill and the starts.
    plan.ways.push(Way::new(
        &[
            (-40.0, -500.0),
            (-10.0, -260.0),
            (0.0, -120.0),
            (0.0, 120.0),
            (10.0, 260.0),
            (40.0, 500.0),
        ],
        3.5,
        ROAD,
    ));
    plan.ways
        .push(Way::new(&[(-150.0, 0.0), (150.0, 0.0)], 3.0, ROAD));
    plan.ways.push(Way::new(
        &[
            (150.0, 0.0),
            (220.0, 12.0),
            (WV_TRENCH.0 - 40.0, WV_TRENCH.1),
        ],
        2.0,
        DIRT_TRACK,
    ));
    plan.ways.push(Way::new(
        &[
            (-150.0, 0.0),
            (-220.0, -12.0),
            (WV_FARM.0 + 36.0, WV_FARM.1),
        ],
        2.0,
        DIRT_TRACK,
    ));
    for &(_, _, x, y) in &STARTS_NS {
        let (x, y) = (f64::from(x), f64::from(y));
        plan.ways.push(Way::new(
            &[(x, y), (x * 0.4, y * 0.75), (x * 0.05, y * 0.62)],
            2.0,
            DIRT_TRACK,
        ));
    }
    // A track along the hill's foot to the north and south starts.
    plan.ways.push(Way::new(
        &[
            (WV_TRENCH.0, WV_TRENCH.1 + 40.0),
            (260.0, 220.0),
            (140.0, 380.0),
        ],
        2.0,
        DIRT_TRACK,
    ));
    plan.ways.push(Way::new(
        &[
            (WV_FARM.0, WV_FARM.1 - 40.0),
            (-260.0, -220.0),
            (-140.0, -380.0),
        ],
        2.0,
        DIRT_TRACK,
    ));
    let meta = Meta {
        biome: "winter",
        weather: Some("snow"),
        starts: &STARTS_NS,
        markers: vec![
            ("village".into(), (0.0, 0.0)),
            ("church".into(), (34.0, -36.0)),
            ("trench_hill".into(), WV_TRENCH),
            ("farmstead".into(), WV_FARM),
            ("supply_square".into(), (0.0, 0.0)),
            ("strongpoint_hill".into(), WV_TRENCH),
            ("supply_trench".into(), WV_TRENCH),
            ("supply_farmstead".into(), WV_FARM),
        ],
    };
    let wood_noise = noise(seed, 5, 120.0);
    let stand_noise = noise(seed, 6, 70.0);
    let place = move |o: &mut Objects| {
        o.reserved.push((WV_TRENCH, STRONGPOINT_CLEAR_M));
        o.reserved.push(((0.0, 0.0), 30.0));
        o.place("env_church", "Intact", (34.0, -38.0), 90, 1.0);
        let houses: [(&str, f64, f64); 5] = [
            ("env_house_farm", 14.0, 9.0),
            ("env_house_farm_brick", 12.0, 8.0),
            ("env_house_stone", 10.0, 8.0),
            ("env_house_log", 9.0, 7.0),
            ("env_house_log", 9.0, 7.0),
        ];
        let garden = Garden {
            tree_kind: "env_tree_deciduous",
            trees: &["Bare", "Bare", "OakYoung"],
            fence: "env_fence_wood",
        };
        for side in [-1.0, 1.0] {
            o.street(
                ((0.0, -175.0), (0.0, -30.0)),
                side,
                3.5,
                &houses,
                0.12,
                &garden,
            );
            o.street(
                ((0.0, 30.0), (0.0, 175.0)),
                side,
                3.5,
                &houses,
                0.12,
                &garden,
            );
            o.street(
                ((-150.0, 0.0), (-30.0, 0.0)),
                side,
                3.0,
                &houses,
                0.12,
                &garden,
            );
            o.street(
                ((30.0, 0.0), (150.0, 0.0)),
                side,
                3.0,
                &houses,
                0.12,
                &garden,
            );
        }
        // Square: a well of logs, carts' worth of crates and barrels.
        o.place("env_woodpile", "Stack", (-15.0, -15.0), 0, 0.5);
        o.place("env_crate", "Stack", (15.0, 16.0), 20, 0.5);
        o.place("env_barrel", "Group", (-16.0, 15.0), 0, 0.5);
        o.farmstead(WV_FARM, &garden, "env_house_farm", false);
        // Two farms by the south and north tracks.
        o.farmstead((-260.0, 250.0), &garden, "env_house_farm_brick", true);
        o.farmstead((260.0, -250.0), &garden, "env_house_farm_brick", false);
        let road = Way::new(
            &[(-40.0, -500.0), (-10.0, -260.0), (0.0, -120.0)],
            3.5,
            ROAD,
        );
        o.poles(&road, 7.0);
        let road = Way::new(&[(0.0, 120.0), (10.0, 260.0), (40.0, 500.0)], 3.5, ROAD);
        o.poles(&road, 7.0);
        // Snowy conifer woods with birch and bare oak at their edges.
        let stands: [(&str, &[&str]); 4] = [
            (
                "env_tree_conifer",
                &["Spruce", "Spruce", "Pine", "SpruceYoung"],
            ),
            ("env_tree_conifer", &["Pine", "Pine", "Spruce"]),
            ("env_tree_birch", &["Birch", "BirchYoung"]),
            ("env_tree_deciduous", &["Bare", "Bare", "Oak"]),
        ];
        let wn = &wood_noise;
        o.wood(
            &stands,
            6.5,
            FULL,
            &|x, y| wn.at(x, y) > 0.66 && dist((x, y), (0.0, 0.0)) > 200.0,
            &stand_noise,
        );
        // Single trees on the snowfields.
        let singles: [(&str, &[&str]); 2] = [
            ("env_tree_deciduous", &["Bare"]),
            ("env_tree_conifer", &["Spruce", "SpruceYoung"]),
        ];
        o.wood(
            &singles,
            38.0,
            FULL,
            &|x, y| wn.at(x, y) > 0.45 && dist((x, y), (0.0, 0.0)) > 190.0,
            &stand_noise,
        );
    };
    Ok((plan, meta, Box::new(place)))
}

// --- Farms ----------------------------------------------------------------

/// The farmsteads with a supply point: name and yard center.
const FARMS: [(&str, (f64, f64)); 5] = [
    ("center", (0.0, 0.0)),
    ("west", (-300.0, -10.0)),
    ("east", (300.0, 10.0)),
    ("south", (60.0, 260.0)),
    ("north", (-60.0, -260.0)),
];

fn farms(seed: u64) -> Result<Built, String> {
    let mut random = Random(Rng::new(seed ^ 0x6661_726d));
    let mut plan = base_plan(seed, &mut random, 20, (-3.0, 7.0));
    plan.drift_level = 2.0;
    plan.mud_level = 0.9;
    for &(_, c) in &FARMS {
        plan.flats.push((c, 45.0));
    }
    // Field tracks joining the farmsteads and the starts (a loop around the
    // center farm, spokes to the others).
    let tracks: Vec<Vec<(f64, f64)>> = vec![
        vec![
            (-60.0, -60.0),
            (60.0, -60.0),
            (60.0, 60.0),
            (-60.0, 60.0),
            (-60.0, -60.0),
        ],
        vec![(-60.0, 60.0), (-250.0, 60.0), (-500.0, 70.0)],
        vec![(60.0, -60.0), (250.0, -60.0), (500.0, -70.0)],
        vec![(-250.0, 60.0), (-250.0, 300.0), (-120.0, 420.0)],
        vec![(250.0, -60.0), (250.0, -300.0), (120.0, -420.0)],
        vec![(60.0, 60.0), (150.0, 200.0), (120.0, 420.0)],
        vec![(-60.0, -60.0), (-150.0, -200.0), (-120.0, -420.0)],
        vec![(-60.0, -60.0), (-300.0, -80.0), (-500.0, -60.0)],
        vec![(60.0, 60.0), (300.0, 80.0), (500.0, 60.0)],
    ];

    for t in &tracks {
        plan.ways.push(Way::new(t, 2.0, DIRT_TRACK));
    }
    // A road north-south along the east of the center farm.
    plan.ways.push(Way::new(
        &[
            (20.0, -500.0),
            (-5.0, -300.0),
            (0.0, -100.0),
            (100.0, -70.0),
            (100.0, 70.0),
            (0.0, 100.0),
            (5.0, 300.0),
            (-20.0, 500.0),
        ],
        3.0,
        ROAD,
    ));
    let mut markers = vec![("village".to_string(), (0.0, 0.0))];
    for &(name, c) in &FARMS {
        markers.push((format!("farm_{name}"), c));
        markers.push((format!("supply_farm_{name}"), c));
    }
    let meta = Meta {
        biome: "forest",
        weather: None,
        starts: &STARTS_NS,
        markers,
    };
    let wood_noise = noise(seed, 5, 110.0);
    let stand_noise = noise(seed, 6, 60.0);
    let field_noise = noise(seed, 7, 60.0);
    let place = move |o: &mut Objects| {
        let garden = Garden {
            tree_kind: "env_tree_deciduous",
            trees: &["OakYoung", "OakYoung", "Beech"],
            fence: "env_fence_wood",
        };
        for (k, &(_, c)) in FARMS.iter().enumerate() {
            let house = ["env_house_farm", "env_house_farm_brick", "env_house_stone"][k % 3];
            o.farmstead(c, &garden, house, k == 3);
            // Cottages of the farm hands and a second barn outside the fence.
            let cottages = ["env_house_farm_brick", "env_house_stone", "env_house_log"];
            for (dx, dy, heading) in [(-52.0, -12.0, 0), (52.0, 18.0, 180), (-22.0, -48.0, 90)] {
                let kind = o.pick(&cottages);
                o.place(kind, "Intact", (c.0 + dx, c.1 + dy), heading, 2.0);
            }
            o.place("env_barn", "Intact", (c.0 + 26.0, c.1 - 46.0), 90, 2.0);
        }
        // Hedgerows along the field tracks (not the road).
        for t in &tracks {
            o.hedgerows(&Way::new(t, 2.0, DIRT_TRACK), 5.5);
        }
        // Haystacks in the fields.
        let fields = &field_noise;
        let mut y = -440.0;
        while y < 440.0 {
            let mut x = -440.0;
            while x < 440.0 {
                if fields.at(x, y) > 0.62 && FARMS.iter().all(|&(_, c)| dist((x, y), c) > 80.0) {
                    o.hay_field((x, y), 6);
                }
                x += 90.0;
            }
            y += 90.0;
        }
        // Small woods and copses.
        let stands: [(&str, &[&str]); 3] = [
            ("env_tree_deciduous", &["Oak", "Beech", "Oak", "OakYoung"]),
            ("env_tree_birch", &["Birch", "BirchYoung"]),
            ("env_tree_conifer", &["Pine", "Spruce"]),
        ];
        let wn = &wood_noise;
        o.wood(
            &stands,
            6.5,
            FULL,
            &|x, y| wn.at(x, y) > 0.66 && FARMS.iter().all(|&(_, c)| dist((x, y), c) > 70.0),
            &stand_noise,
        );
        let singles: [(&str, &[&str]); 1] = [("env_tree_deciduous", &["Oak", "Beech"])];
        o.wood(
            &singles,
            45.0,
            FULL,
            &|x, y| wn.at(x, y) > 0.4,
            &stand_noise,
        );
    };
    Ok((plan, meta, Box::new(place)))
}

// --- River land -------------------------------------------------------------

/// Crossings: the stone bridge in the middle, timber bridges north and
/// south. Axes on path cell center lines (`y = -499 + 4 k`).
const RL_CROSSINGS: [Crossing; 3] = [
    Crossing {
        y: 1.0,
        half_width: 5.5,
        stone: true,
    },
    Crossing {
        y: -279.0,
        half_width: 3.5,
        stone: false,
    },
    Crossing {
        y: 281.0,
        half_width: 3.5,
        stone: false,
    },
];
const RL_FORD_Y: [f64; 2] = [-440.0, 440.0];
const RL_FARMS: [(f64, f64); 2] = [(-250.0, 200.0), (250.0, -200.0)];

fn river_land(seed: u64) -> Result<Built, String> {
    let mut random = Random(Rng::new(seed ^ 0x7269_7665));
    let mut plan = base_plan(seed, &mut random, 16, (-2.0, 6.0));
    plan.drift_level = 2.0;
    plan.mud_level = 0.9;
    let bends: Vec<f64> = (0..9).map(|_| random.range(-45.0, 45.0)).collect();
    plan.river = Some(River {
        bends,
        half_width: 10.0,
        crossings: RL_CROSSINGS.to_vec(),
    });
    plan.fords = RL_FORD_Y.to_vec();
    for &c in &RL_FARMS {
        plan.flats.push((c, 45.0));
    }
    // Hamlets at both ends of the stone bridge, flat.
    plan.flats.push(((-110.0, 0.0), 60.0));
    plan.flats.push(((110.0, 0.0), 60.0));
    // The road over the stone bridge, field tracks over the timber bridges
    // and through the fords.
    plan.ways.push(Way::new(
        &[(-500.0, -60.0), (-300.0, 1.0), (300.0, 1.0), (500.0, 60.0)],
        3.0,
        ROAD,
    ));
    for c in &RL_CROSSINGS[1..] {
        let s = c.y.signum();
        plan.ways.push(Way::new(
            &[
                (-420.0, 120.0 * s),
                (-200.0, c.y),
                (200.0, c.y),
                (420.0, 120.0 * s),
            ],
            2.0,
            DIRT_TRACK,
        ));
    }
    for &y in &RL_FORD_Y {
        let s = y.signum();
        plan.ways.push(Way::new(
            &[
                (-420.0, 120.0 * s),
                (-150.0, y),
                (150.0, y),
                (420.0, 120.0 * s),
            ],
            2.0,
            DIRT_TRACK,
        ));
    }
    for &c in &RL_FARMS {
        // From the fence gap on the side towards the center (the yard
        // stays free).
        let gap = (c.0, c.1 - c.1.signum() * 34.0);
        plan.ways
            .push(Way::new(&[gap, (c.0 * 0.4, c.1 * 0.2)], 2.0, DIRT_TRACK));
    }
    let meta = Meta {
        biome: "forest",
        weather: Some("overcast"),
        starts: &STARTS_WE,
        markers: Vec::new(),
    };
    let wood_noise = noise(seed, 5, 100.0);
    let stand_noise = noise(seed, 6, 60.0);
    let place = move |o: &mut Objects| {
        let garden = Garden {
            tree_kind: "env_tree_deciduous",
            trees: &["OakYoung", "Beech"],
            fence: "env_wall_stone",
        };
        let houses: [(&str, f64, f64); 4] = [
            ("env_house_farm", 14.0, 9.0),
            ("env_house_farm_brick", 12.0, 8.0),
            ("env_house_stone", 10.0, 8.0),
            ("env_house_town", 12.0, 10.0),
        ];
        for side in [-1.0, 1.0] {
            o.street(
                ((-180.0, 1.0), (-55.0, 1.0)),
                side,
                3.0,
                &houses,
                0.15,
                &garden,
            );
            o.street(
                ((55.0, 1.0), (180.0, 1.0)),
                side,
                3.0,
                &houses,
                0.15,
                &garden,
            );
        }
        o.place("env_church", "Damaged", (-120.0, -40.0), 90, 1.0);
        for &c in &RL_FARMS {
            o.farmstead(c, &garden, "env_house_farm", false);
        }
        let stands: [(&str, &[&str]); 3] = [
            ("env_tree_deciduous", &["Oak", "Beech", "OakYoung"]),
            ("env_tree_birch", &["Birch", "BirchYoung"]),
            ("env_tree_conifer", &["Spruce", "Pine"]),
        ];
        let wn = &wood_noise;
        o.wood(
            &stands,
            6.5,
            FULL,
            &|x, y| wn.at(x, y) > 0.64 && x.abs() > 60.0,
            &stand_noise,
        );
        // Riverside trees: birches and young oaks on the banks, not at the
        // crossings.
        let bank: [(&str, &[&str]); 2] = [
            ("env_tree_birch", &["Birch", "BirchYoung"]),
            ("env_tree_deciduous", &["OakYoung", "Beech"]),
        ];
        o.wood(
            &bank,
            9.0,
            FULL,
            &|x, y| {
                x.abs() < 75.0
                    && RL_CROSSINGS.iter().all(|c| (y - c.y).abs() > 45.0)
                    && RL_FORD_Y.iter().all(|f| (y - f).abs() > 35.0)
                    && wn.at(y, x) > 0.35
            },
            &stand_noise,
        );
        let singles: [(&str, &[&str]); 1] = [("env_tree_deciduous", &["Oak", "Beech"])];
        o.wood(
            &singles,
            50.0,
            FULL,
            &|x, y| wn.at(x, y) > 0.4,
            &stand_noise,
        );
    };
    Ok((plan, meta, Box::new(place)))
}

/// Writes the map into `options.out`.
/// Tall grass (prompt 073), sparingly where it belongs: along the river
/// banks of river_land (a frayed band a few to about 20 m wide) and on a few
/// fallow patches of farms. Only on grass outside woods, never near starts
/// or supply points, never on field tracks and roads; hash noise only, so
/// nothing else of the map changes. The rules of tall grass are those of
/// grass.
fn paint_tall_grass(map: LandMap, seed: u64, meta: &Meta, wood: &[bool], terrain: &mut [u8]) {
    let band = noise(seed, 0x7461_6c6c, 23.0);
    let fine = noise(seed, 0x6772_6173, 7.0);
    let keep_free = |x: f64, y: f64| {
        meta.starts.iter().any(|&(_, _, sx, sy)| {
            dist((x, y), (f64::from(sx), f64::from(sy))) < START_CLEAR_M + 20.0
        }) || meta
            .markers
            .iter()
            .any(|(name, c)| name.starts_with("supply") && dist((x, y), *c) < 40.0)
    };
    let source = terrain.to_vec();
    let near = |i: usize, j: usize, r: usize, ids: &[u8]| {
        let (i0, i1) = (i.saturating_sub(r), (i + r).min(SAMPLES - 1));
        let (j0, j1) = (j.saturating_sub(r), (j + r).min(SAMPLES - 1));
        let mut best = f64::MAX;
        for jj in j0..=j1 {
            for ii in i0..=i1 {
                if ids.contains(&source[jj * SAMPLES + ii]) {
                    let d = (((ii as f64 - i as f64).powi(2) + (jj as f64 - j as f64).powi(2))
                        .sqrt())
                        * CELL_M;
                    best = best.min(d);
                }
            }
        }
        best
    };
    for j in 0..SAMPLES {
        for i in 0..SAMPLES {
            let k = j * SAMPLES + i;
            if source[k] != GRASS || wood[k] {
                continue;
            }
            let (x, y) = (coord(i), coord(j));
            if keep_free(x, y) {
                continue;
            }
            let ways = near(i, j, 2, &[DIRT_TRACK, ROAD]);
            if ways < 3.0 {
                continue;
            }
            let wanted = match map {
                LandMap::RiverLand => {
                    // Banks: up to 10..32 m from the water or the sand of the bank.
                    let water = near(i, j, 17, &[SHALLOW_WATER, DEEP_WATER, SAND]);
                    water < 10.0 + 22.0 * band.at(x, y) + 4.0 * (fine.at(x, y) - 0.5)
                }
                LandMap::Farms => {
                    // Fallow: a few patches where two noises are high.
                    band.at(x * 0.4, y * 0.4) * 0.88 + fine.at(x, y) * 0.12 > 0.8
                }
                LandMap::WinterVillage => false,
            };
            if wanted {
                terrain[k] = TALL_GRASS;
            }
        }
    }
}

pub fn generate(options: &Options) -> Result<String, String> {
    let terrain_rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let object_rules = ObjectRules::builtin().map_err(|e| e.0)?;
    let seed = options.seed;
    let (mut plan, mut meta, place) = match options.map {
        LandMap::WinterVillage => winter_village(seed)?,
        LandMap::Farms => farms(seed)?,
        LandMap::RiverLand => river_land(seed)?,
    };
    let (mut heights, mut terrain) = rasters(&mut plan, meta.starts);
    let bridges = match &plan.river {
        Some(river) => place_bridges(river, &mut heights, &mut terrain)?,
        None => Vec::new(),
    };
    if options.map == LandMap::RiverLand {
        let river = plan.river.as_ref().expect("river");
        let names = [
            "supply_bridgehead",
            "supply_bridge_north",
            "supply_bridge_south",
        ];
        for (b, name) in bridges.iter().zip(names) {
            meta.markers.push((name.into(), (b.x, b.y)));
        }
        meta.markers
            .push(("bridge_stone".into(), (bridges[0].x, bridges[0].y)));
        meta.markers.push(("village".into(), (-110.0, 1.0)));
        for (k, &y) in RL_FORD_Y.iter().enumerate() {
            meta.markers.push((
                format!("ford_{}", ["north", "south"][k]),
                (river.center_x(y), y),
            ));
        }
        meta.markers.push(("supply_farm_west".into(), RL_FARMS[0]));
        meta.markers.push(("supply_farm_east".into(), RL_FARMS[1]));
        meta.markers.push(("farm_west".into(), RL_FARMS[0]));
        meta.markers.push(("farm_east".into(), RL_FARMS[1]));
    }

    let mut objects = Objects::new(&object_rules, &terrain, seed);
    reserve_starts(&mut objects, meta.starts);
    // Supply flags stand free.
    for (name, c) in &meta.markers {
        if name.starts_with("supply") {
            objects.reserved.push((*c, 6.0));
        }
    }
    let mut bridge_rows = Vec::new();
    for b in &bridges {
        bridge_rows.push(format!(
            "{},{},{},{},90",
            b.kind,
            b.variant,
            (b.x * 1000.0).round() as i64,
            (b.y * 1000.0).round() as i64
        ));
        objects.reserved.push(((b.x, b.y), 22.0));
    }
    place(&mut objects);
    let summary = format!(
        "{} buildings, {} trees, {} other objects, {} bridges",
        objects.buildings,
        objects.trees,
        objects.other,
        bridges.len()
    );
    let mut rows = vec![
        format!(
            "# Generated by `cli gen-land --map {}` (prompt 056, docs/MAPS.md). Rules: data/objects.toml.",
            options.map.name()
        ),
        CSV_HEADER.to_string(),
    ];
    rows.extend(bridge_rows);
    rows.append(&mut objects.rows);
    drop(objects);
    // Tall grass (prompt 073) after the objects, which keep their places;
    // not in woods (within 8 m of a tree).
    let mut wood = vec![false; SAMPLES * SAMPLES];
    for row in &rows {
        let cols: Vec<&str> = row.split(',').collect();
        if cols.len() < 4 || !cols[0].starts_with("env_tree_") {
            continue;
        }
        let (Ok(x), Ok(y)) = (cols[2].parse::<f64>(), cols[3].parse::<f64>()) else {
            continue;
        };
        let (x, y) = (x / 1000.0, y / 1000.0);
        for dy in -4..=4 {
            for dx in -4..=4 {
                let (px, py) = (x + f64::from(dx) * CELL_M, y + f64::from(dy) * CELL_M);
                if dist((x, y), (px, py)) <= 8.0 && px.abs() <= HALF && py.abs() <= HALF {
                    wood[index(px, py)] = true;
                }
            }
        }
    }
    paint_tall_grass(options.map, seed, &meta, &wood, &mut terrain);

    let low = heights.iter().copied().fold(f64::MAX, f64::min);
    let high = heights.iter().copied().fold(f64::MIN, f64::max);
    if high - low > 60.0 {
        return Err(format!(
            "relief of {:.1} m is too high for 1 mm steps",
            high - low
        ));
    }
    let base_mm = ((low - 1.0).floor() * 1000.0) as i32;
    let encoded: Vec<u8> = heights
        .iter()
        .flat_map(|h| {
            let value = (h * 1000.0 - f64::from(base_mm)).round();
            (value.clamp(0.0, 65535.0) as u16).to_be_bytes()
        })
        .collect();
    std::fs::create_dir_all(options.out).map_err(|e| e.to_string())?;
    let side = SAMPLES as u32;
    write_png(&options.out.join("height.png"), side, &encoded, None)?;
    let palette: Vec<u8> = terrain_rules
        .types
        .iter()
        .flat_map(|t| {
            [
                (t.debug_color >> 16) as u8,
                (t.debug_color >> 8) as u8,
                t.debug_color as u8,
            ]
        })
        .collect();
    write_png(
        &options.out.join("terrain.png"),
        side,
        &terrain,
        Some(palette),
    )?;
    let mut csv = rows.join("\n");
    csv.push('\n');
    std::fs::write(options.out.join("objects.csv"), csv).map_err(|e| e.to_string())?;

    let name = options.map.name();
    let mut toml = String::new();
    let _ = write!(
        toml,
        "# Generated by `cli gen-land` (prompt 056); edit freely (docs/MAPS.md).\n\
         name = \"{name}\"\n\
         generator = \"cli gen-land --map {name} --seed {seed}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = 1\n\
         biome = \"{}\"\n\
         water_flow_deg = 180\n",
        meta.biome,
    );
    if let Some(weather) = meta.weather {
        let _ = writeln!(toml, "weather = \"{weather}\"");
    }
    for &(player, team, x, y) in meta.starts {
        let _ = write!(
            toml,
            "\n[[start]]\nplayer = {player}\nteam = {team}\nposition_m = [{x}, {y}]\n"
        );
    }
    for (marker, (x, y)) in &meta.markers {
        let _ = write!(
            toml,
            "\n[[marker]]\nname = \"{marker}\"\nposition_m = [{}, {}]\n",
            x.round() as i32,
            y.round() as i32
        );
    }
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;
    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, {summary}",
        options.out.display(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{LandMap, Options, generate, heading_of};
    use sim::Map;

    #[test]
    fn headings_follow_the_compass() {
        assert_eq!(heading_of(0.0, -1.0), 0);
        assert_eq!(heading_of(1.0, 0.0), 90);
        assert_eq!(heading_of(0.0, 1.0), 180);
        assert_eq!(heading_of(-1.0, 0.0), 270);
        assert_eq!(heading_of(1.0, -1.0), 45);
        assert_eq!(heading_of(-1.0, 1.0), 225);
    }

    /// Every map loads with four starts, its supply points stand on their
    /// places, and the same seed gives the same files.
    #[test]
    fn land_maps_load_with_their_places() {
        for map in LandMap::ALL {
            let dir = std::env::temp_dir().join(format!(
                "mas-gen-land-{}-{}",
                map.name(),
                std::process::id()
            ));
            let options = Options {
                map,
                seed: map.default_seed(),
                out: &dir,
            };
            let report = generate(&options).unwrap();
            let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
            let first = (read("objects.csv"), read("height.png"), read("terrain.png"));
            generate(&options).unwrap();
            assert!(
                first.0 == read("objects.csv"),
                "{}: objects differ",
                map.name()
            );
            assert!(
                first.1 == read("height.png"),
                "{}: heights differ",
                map.name()
            );
            assert!(
                first.2 == read("terrain.png"),
                "{}: terrain differs",
                map.name()
            );
            let toml = String::from_utf8(read("map.toml")).unwrap();
            let mut loaded = Map::load(&toml, &first.1, &first.2).unwrap();
            loaded
                .add_objects(&String::from_utf8(first.0).unwrap())
                .unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(loaded.starts().len(), 4, "{report}");
            let supply = loaded
                .markers()
                .iter()
                .filter(|m| m.name.starts_with("supply"))
                .count();
            assert!(supply >= 3, "{}: {supply} supply points", map.name());
            let count = |prefix: &str| {
                loaded
                    .objects()
                    .iter()
                    .filter(|o| {
                        loaded.object_kinds()[usize::from(o.kind)]
                            .id
                            .starts_with(prefix)
                    })
                    .count()
            };
            assert!(count("env_house") + count("env_barn") >= 10, "{report}");
            assert!(count("env_hay_bale") >= 10, "{report}");
            assert!(count("env_tree") >= 500, "{report}");
            match map {
                LandMap::WinterVillage => {
                    assert!(
                        loaded
                            .markers()
                            .iter()
                            .any(|m| m.name == "strongpoint_hill")
                    );
                    assert_eq!(count("env_church"), 1);
                }
                LandMap::Farms => assert!(count("env_hedge") >= 300, "{report}"),
                LandMap::RiverLand => assert_eq!(count("env_bridge"), 3, "{report}"),
            }
        }
    }
}
