//! Maps from picture sketches (prompt 088): `cli gen-sketch <name>`.
//!
//! A sketch (`data/map_sketches/<name>.toml`) holds the layout a template
//! picture gives, simplified to lines and areas in meters: size, biome,
//! weather, seed; rivers with their width, islands, lakes and sea, bridge
//! crossings and fords, roads and field tracks, paved squares and ring
//! roads, woods and scattered trees, fields (grain, green, ploughed), rows
//! and rings of buildings, farmsteads, single buildings, hills, starts,
//! supply points and markers. Everything else (single trees, houses of a
//! row, their gardens) comes from the generator's random stream, so a sketch
//! is short and the map can be made again at any time. A sketch with `base`
//! takes everything from another sketch and changes only what it names
//! (`river_fields_winter`: the same land in winter).
//!
//! Prompt 122 added long stone bridges (`stone_long`: chains of high
//! arches), rock mountains (`[[hill]] kind = "peak"`, coarser height steps
//! above 60 m of relief), islands in bays, crag woods, waterfalls (markers
//! for the presentation), timber piers, rapids stones and the map's `look`.
//!
//! The generator reuses the building blocks of `gen_land` (object placer
//! without overlaps, woods, farmsteads, noise) and the river and bridge
//! rules of `gen_map` (water narrowed at crossings, embankments with 12 %
//! ramps, bridge axes on path cell center lines, fords). Like every
//! generator it uses floats outside the simulation but only basic
//! arithmetic and `sqrt`, so the same sketch gives the same files on every
//! platform. No real place names.

use crate::gen_land::{Garden, Objects, Way, dist, heading_of, noise, smoothstep};
use crate::gen_map::{Random, write_png_rect};
use serde::Deserialize;
use sim::objects::CSV_HEADER;
use sim::{ObjectRules, Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;
const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const SAND: u8 = 3;
const SNOW: u8 = 5;
const SHALLOW_WATER: u8 = 6;
const DEEP_WATER: u8 = 7;
/// Fields (prompt 088): terrain ids of `data/terrain.toml`.
pub(crate) const FIELD_GRAIN: u8 = 9;
pub(crate) const FIELD_GREEN: u8 = 10;
pub(crate) const FIELD_PLOUGHED: u8 = 11;

/// Folder of the sketches.
pub const SKETCH_DIR: &str = "data/map_sketches";
/// The maps made from sketches (`just gen-sketch` without a name).
pub const ALL: [&str; 11] = [
    "river_fields",
    "river_fields_winter",
    "river_fork",
    "island_chain",
    "desert_lagoon",
    "winter_town",
    "river_town",
    "valley_fields",
    "river_valley",
    "river_city",
    "archipelago",
];

const START_CLEAR_M: f64 = 60.0;
/// River: banks cut below the ground over a valley beside the water.
const BANK_CUT_M: f64 = 3.0;
const RIVER_VALLEY_M: f64 = 40.0;
const WATERLINE_M: f64 = 0.15;
const SHALLOW_DEPTH_M: f64 = 0.5;
const CHANNEL_DEPTH_M: f64 = 1.3;
const FORD_HALF_WIDTH_M: f64 = 7.0;
/// Water narrowed within this distance of a crossing, tapering out.
const NARROW_M: f64 = 18.0;
const TAPER_M: f64 = 70.0;
const BRIDGE_LAND_M: f64 = 4.0;
const RAMP_GRADE: f64 = 0.12;
const EMBANKMENT_SIDE_GRADE: f64 = 0.5;
/// Fields keep a strip of grass inside their outline (m).
const FIELD_EDGE_M: f64 = 2.5;
/// Railway: half width of the ballast bed and of the embankment (m).
const RAIL_BED_M: f64 = 3.5;
const RAIL_SHOULDER_M: f64 = 12.0;
/// Sea maps: the height land falls to at the coast (m).
const SEA_COAST_M: f64 = 0.35;

// --- The sketch -----------------------------------------------------------

type P = [f64; 2];

#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
struct Sketch {
    #[serde(default)]
    name: String,
    /// Another sketch this one changes (only the fields named here).
    #[serde(default)]
    base: Option<String>,
    #[serde(default)]
    size_m: Option<[u32; 2]>,
    #[serde(default)]
    biome: Option<String>,
    #[serde(default)]
    weather: Option<String>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    water_flow_deg: Option<u32>,
    #[serde(default)]
    start: Vec<StartDef>,
    #[serde(default)]
    supply: Vec<Named>,
    #[serde(default)]
    marker: Vec<Named>,
    #[serde(default)]
    river: Vec<RiverDef>,
    #[serde(default)]
    island: Vec<Island>,
    #[serde(default)]
    lake: Vec<Area>,
    #[serde(default)]
    crossing: Vec<CrossingDef>,
    #[serde(default)]
    road: Vec<RoadDef>,
    #[serde(default)]
    ring_road: Vec<RingRoad>,
    #[serde(default)]
    square: Vec<Area>,
    #[serde(default)]
    wood: Vec<Area>,
    #[serde(default)]
    scatter: Vec<Area>,
    #[serde(default)]
    field: Vec<Area>,
    #[serde(default)]
    row: Vec<Row>,
    #[serde(default)]
    ring: Vec<Ring>,
    #[serde(default)]
    farm: Vec<Farm>,
    #[serde(default)]
    building: Vec<Building>,
    #[serde(default)]
    hill: Vec<Hill>,
    #[serde(default)]
    rail: Vec<Rail>,
    #[serde(default)]
    paint: Vec<Paint>,
    #[serde(default)]
    village: Vec<Village>,
    /// True: everything outside the `land` polygons is sea (stage 2).
    #[serde(default)]
    sea: bool,
    #[serde(default)]
    land: Vec<Area>,
    #[serde(default)]
    causeway: Vec<Strip>,
    #[serde(default)]
    shoal: Vec<Strip>,
    /// Sea maps: water inside the land that belongs to the sea (lagoons,
    /// inlets), at the same level; polygons may overlap.
    #[serde(default)]
    bay: Vec<Area>,
    #[serde(default)]
    town: Vec<Town>,
    #[serde(default)]
    patchwork: Vec<Patchwork>,
    /// Look on top of the biome (prompt 122): written to `map.toml`.
    #[serde(default)]
    look: Option<String>,
    /// Waterfalls (prompt 122): presentation only, two markers each.
    #[serde(default)]
    waterfall: Vec<Waterfall>,
    /// Timber piers into the water (prompt 122): scenery, block nothing.
    #[serde(default)]
    pier: Vec<Pier>,
    /// Rapids (prompt 122): stones in the water along a line.
    #[serde(default)]
    rapids: Vec<Rapids>,
}

/// A waterfall (prompt 122): presentation only. The generator writes the
/// markers `waterfall_<n>_top` (the rock edge it falls from) and
/// `waterfall_<n>_foot` (where it meets the water); Godot draws the falling
/// water, spray and sound between them. No rule reads them.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Waterfall {
    top_m: P,
    foot_m: P,
}

/// A timber pier (prompt 122): `env_pier_wood` segments of 10 m from the
/// first point (on the shore) along the line into the water; `boat`
/// (default true) moors a small boat at the last segment. Scenery: the
/// pier blocks nothing and stands on the water level.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Pier {
    points_m: [P; 2],
    #[serde(default)]
    boat: Option<bool>,
}

/// Rapids (prompt 122): `count` stones (`env_rock`, small and medium: they
/// block nothing) in the water within `width_m` of the line, so the water
/// foams around them. Deep water stays deep (no ford).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Rapids {
    points_m: Vec<P>,
    width_m: f64,
    count: u32,
}

/// A patchwork of fields (stage 3): inside the polygon, plots of `size_m`
/// (length, width; each varied by up to a third) in a grid turned along
/// `axis` (a direction, x east and y south), separated by 6 m of grass;
/// the kind of each plot is drawn from `kinds`, and a share `meadow` stays
/// grass. Plots whose middle lies outside the polygon are left out.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Patchwork {
    polygon_m: Vec<P>,
    #[serde(default)]
    axis: Option<P>,
    size_m: P,
    kinds: Vec<String>,
    #[serde(default)]
    meadow: f64,
}

/// Street building (stage 2): inside the polygon, buildings of `kinds`
/// line both sides of every road and track of the sketch, long side along
/// the street, `setback_m` (default 3) behind its edge, `gap_m` (default
/// 4) apart on average; a share `damaged` damaged or ruined.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Town {
    polygon_m: Vec<P>,
    kinds: Vec<String>,
    #[serde(default)]
    setback_m: Option<f64>,
    #[serde(default)]
    gap_m: Option<f64>,
    #[serde(default)]
    damaged: f64,
}

/// A strip along a line: a causeway (land across the sea, open under the
/// bridges of its crossings) or a shoal (shallow water, a ford for tanks).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Strip {
    width_m: f64,
    points_m: Vec<P>,
}

/// A railway line (stage 2): a 7 m ballast bed (terrain like a field
/// track, open to everybody) on a low embankment with telegraph poles
/// beside it. Only presentation: no rails yet, no trains.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Rail {
    points_m: Vec<P>,
    #[serde(default)]
    embankment_m: Option<f64>,
}

/// Ground painted with a terrain type of `data/terrain.toml` (e.g. `sand`,
/// `mud`, `dirt_track`) inside a polygon; with `noise_m` only in blotches
/// covering about `cover` of it (trampled sand between ruins).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Paint {
    terrain: String,
    polygon_m: Vec<P>,
    #[serde(default)]
    noise_m: Option<f64>,
    #[serde(default)]
    cover: Option<f64>,
}

/// Loose houses inside a polygon: `count` buildings of `kinds`, turned
/// along the nearest way, a share `damaged` of them damaged or ruined, and
/// `walls` short runs of broken walls between them.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Village {
    polygon_m: Vec<P>,
    kinds: Vec<String>,
    count: u32,
    #[serde(default)]
    damaged: f64,
    #[serde(default)]
    walls: u32,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct StartDef {
    player: u8,
    team: u8,
    position_m: P,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Named {
    name: String,
    position_m: P,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct RiverDef {
    width_m: f64,
    points_m: Vec<P>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Island {
    center_m: P,
    radius_m: f64,
}

/// An area: a polygon with an optional kind (wood: `mixed`, `deciduous`,
/// `conifer`, `birch`, `palm`; field: `grain`, `green`, `ploughed`; lake:
/// `deep`, `shallow`) and spacing (woods, scattered trees).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Area {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    spacing_m: Option<f64>,
    polygon_m: Vec<P>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct CrossingDef {
    /// `stone`, `wood`, `ford` or `stone_long` (prompt 122: a chain of
    /// high stone arches over the whole width of the water, 60-120 m).
    kind: String,
    position_m: P,
    /// `ns`: the way crosses north–south (the bridge axis runs along y),
    /// `we`: west–east.
    axis: String,
    /// Deck height above the bank (m); default 3 (stone), 1.5 (timber), 6
    /// (long stone bridge).
    #[serde(default)]
    rise_m: Option<f64>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct RoadDef {
    /// `road` or `track`.
    class: String,
    width_m: f64,
    points_m: Vec<P>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct RingRoad {
    center_m: P,
    radius_m: f64,
    width_m: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Row {
    kinds: Vec<String>,
    from_m: P,
    to_m: P,
    gap_m: f64,
    #[serde(default)]
    damaged: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Ring {
    kinds: Vec<String>,
    center_m: P,
    radius_m: P,
    count: u32,
    #[serde(default)]
    damaged: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Farm {
    center_m: P,
    house: String,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Building {
    kind: String,
    #[serde(default)]
    variant: Option<String>,
    position_m: P,
    heading_deg: u16,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Hill {
    center_m: P,
    radius_m: f64,
    height_m: f64,
    /// `""`: a smooth bump; `peak` (prompt 122): a rock mountain with a
    /// pointed top, steep upper flanks and crags (ridged noise), gentle at
    /// the foot.
    #[serde(default)]
    kind: String,
}

/// Reads a sketch from `dir`, resolving `base`.
fn load(dir: &Path, name: &str) -> Result<Sketch, String> {
    let path = dir.join(format!("{name}.toml"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sketch: Sketch = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if let Some(base) = sketch.base.clone() {
        let mut full = load(dir, &base)?;
        full.name = name.into();
        full.base = None;
        if sketch.size_m.is_some() {
            full.size_m = sketch.size_m;
        }
        if sketch.biome.is_some() {
            full.biome = sketch.biome;
        }
        if sketch.weather.is_some() {
            full.weather = sketch.weather;
        }
        if sketch.seed.is_some() {
            full.seed = sketch.seed;
        }
        if sketch.look.is_some() {
            full.look = sketch.look;
        }
        return Ok(full);
    }
    if sketch.name != name {
        return Err(format!("{}: name is {:?}", path.display(), sketch.name));
    }
    Ok(sketch)
}

// --- Geometry --------------------------------------------------------------

fn segment_distance(p: (f64, f64), a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a[0]) * dx + (p.1 - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, (a[0] + t * dx, a[1] + t * dy))
}

fn polyline_distance(p: (f64, f64), points: &[P]) -> f64 {
    points
        .windows(2)
        .map(|s| segment_distance(p, s[0], s[1]))
        .fold(f64::INFINITY, f64::min)
}

fn inside(p: (f64, f64), polygon: &[P]) -> bool {
    let mut odd = false;
    let n = polygon.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a[1] > p.1) != (b[1] > p.1) {
            let x = a[0] + (p.1 - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p.0 < x {
                odd = !odd;
            }
        }
        j = i;
    }
    odd
}

/// Distance from `p` to the outline of `polygon`.
fn edge_distance(p: (f64, f64), polygon: &[P]) -> f64 {
    let n = polygon.len();
    (0..n)
        .map(|i| segment_distance(p, polygon[i], polygon[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

fn bounds(polygon: &[P]) -> crate::gen_land::Rect {
    let mut r = crate::gen_land::Rect {
        x0: f64::INFINITY,
        y0: f64::INFINITY,
        x1: f64::NEG_INFINITY,
        y1: f64::NEG_INFINITY,
    };
    for p in polygon {
        r.x0 = r.x0.min(p[0]);
        r.y0 = r.y0.min(p[1]);
        r.x1 = r.x1.max(p[0]);
        r.y1 = r.y1.max(p[1]);
    }
    r
}

// --- The plan --------------------------------------------------------------

/// A crossing after snapping: its axis on a path cell center line.
#[derive(Clone, Copy)]
struct Crossing {
    kind: CrossingKind,
    x: f64,
    y: f64,
    /// True: the way runs north–south (bridge heading 0).
    ns: bool,
    /// Deck height above the bank (m).
    rise: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CrossingKind {
    Stone,
    Wood,
    Ford,
    /// Prompt 122: high stone arches (`env_bridge_stone` `ArchHigh`, 20 m
    /// each) end to end over the whole water; the deck runs over the chain
    /// (prompt 115).
    StoneLong,
}

impl CrossingKind {
    /// Half width of the water at the crossing (the longest bridge of the
    /// kind spans it with 4 m of bank on each side).
    const fn half_water(self) -> f64 {
        match self {
            Self::Stone => 10.5,
            Self::Wood => 3.6,
            Self::Ford | Self::StoneLong => f64::INFINITY,
        }
    }

    const fn is_bridge(self) -> bool {
        matches!(self, Self::Stone | Self::Wood | Self::StoneLong)
    }

    /// Default deck height above the bank (m).
    const fn rise(self) -> f64 {
        match self {
            Self::Wood => 1.5,
            Self::StoneLong => 6.0,
            Self::Stone | Self::Ford => 3.0,
        }
    }
}

/// Length of one piece of a long stone bridge (m).
const LONG_PIECE_M: f64 = 20.0;
/// Reach of a long bridge's water measurement and approach along the way.
const LONG_REACH_M: f64 = 150.0;

struct Plan<'a> {
    sketch: &'a Sketch,
    ways: Vec<Way>,
    /// Painted terrain: id, blotch noise and its threshold.
    paints: Vec<(u8, Option<crate::gen_land::Noise>, f64)>,
    crossings: Vec<Crossing>,
    flats: Vec<((f64, f64), f64, f64)>,
    ground: crate::gen_land::Noise,
    detail: crate::gen_land::Noise,
    bumps: Vec<(f64, f64, f64, f64)>,
    /// Rock mountains (prompt 122): center, radius, height.
    peaks: Vec<(f64, f64, f64, f64)>,
    crag: crate::gen_land::Noise,
    /// Field plots of the patchworks.
    plots: Vec<Area>,
    /// Outline segments of the bays that are not inside another bay (the
    /// coast of the bays).
    bay_coast: Vec<(P, P)>,
}

impl Plan<'_> {
    fn natural(&self, x: f64, y: f64) -> f64 {
        let mut h = 2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y);
        for &(bx, by, r, a) in &self.bumps {
            let d2 = ((x - bx) * (x - bx) + (y - by) * (y - by)) / (r * r);
            if d2 < 1.0 {
                let t = 1.0 - d2;
                h += a * t * t;
            }
        }
        for &(px, py, r, a) in &self.peaks {
            let d2 = ((x - px) * (x - px) + (y - py) * (y - py)) / (r * r);
            if d2 < 1.0 {
                h += a * peak_profile(d2.sqrt(), self.crag.at(x, y));
            }
        }
        h
    }

    /// Half width of `river` near `p` (narrowed at crossings).
    fn river_half(&self, river: &RiverDef, p: (f64, f64)) -> f64 {
        let w = river.width_m / 2.0;
        self.crossings.iter().fold(w, |acc, c| {
            let hw = c.kind.half_water();
            if hw >= w {
                return acc;
            }
            let blend = smoothstep(NARROW_M, TAPER_M, dist(p, (c.x, c.y)));
            acc.min(hw + (w - hw) * blend)
        })
    }

    fn sample(&self, x: f64, y: f64) -> (f64, u8) {
        let p = (x, y);
        let mut h = self.natural(x, y);
        for &(c, r, level) in &self.flats {
            let blend = smoothstep(r, r + 40.0, dist(p, c));
            h = level + (h - level) * blend;
        }
        let s = self.sketch;
        let mut terrain = GRASS;
        for f in s.field.iter().chain(&self.plots) {
            if inside(p, &f.polygon_m) && edge_distance(p, &f.polygon_m) > FIELD_EDGE_M {
                terrain = match f.kind.as_str() {
                    "green" => FIELD_GREEN,
                    "ploughed" => FIELD_PLOUGHED,
                    _ => FIELD_GRAIN,
                };
            }
        }
        for (paint, (id, blotch, threshold)) in s.paint.iter().zip(&self.paints) {
            if !inside(p, &paint.polygon_m) {
                continue;
            }
            let painted = blotch.as_ref().is_none_or(|n| {
                // Blotches: two octaves, denser along ways, thinning out
                // towards the outline (no straight edges).
                let way = self
                    .ways
                    .iter()
                    .map(|w| w.distance(x, y) - w.half)
                    .fold(f64::INFINITY, f64::min);
                let value = 0.7 * n.at(x, y)
                    + 0.3 * n.at(x * 2.7 + 500.0, y * 2.7 - 300.0)
                    + 0.22 * (1.0 - smoothstep(0.0, 18.0, way))
                    - 0.3 * (1.0 - smoothstep(0.0, 45.0, edge_distance(p, &paint.polygon_m)));
                value > *threshold
            });
            if painted {
                terrain = *id;
            }
        }
        for way in &self.ways {
            if way.distance(x, y) < way.half {
                terrain = way.id;
                break;
            }
        }
        for r in &s.ring_road {
            if (dist(p, (r.center_m[0], r.center_m[1])) - r.radius_m).abs() < r.width_m / 2.0 {
                terrain = DIRT_TRACK;
            }
        }
        if s.square.iter().any(|a| inside(p, &a.polygon_m)) {
            terrain = ROAD;
        }
        for rail in &s.rail {
            let d = polyline_distance(p, &rail.points_m);
            if d < RAIL_SHOULDER_M {
                h += rail.embankment_m.unwrap_or(0.8)
                    * (1.0 - smoothstep(RAIL_BED_M, RAIL_SHOULDER_M, d));
                if d < RAIL_BED_M {
                    terrain = DIRT_TRACK;
                }
            }
        }
        // Rivers.
        for river in &s.river {
            let d = polyline_distance(p, &river.points_m);
            if d > river.width_m / 2.0 + RIVER_VALLEY_M {
                continue;
            }
            let w = self.river_half(river, p);
            if d >= w + RIVER_VALLEY_M {
                continue;
            }
            let island = s
                .island
                .iter()
                .map(|i| dist(p, (i.center_m[0], i.center_m[1])) - i.radius_m)
                .fold(f64::INFINITY, f64::min);
            let ford = self.crossings.iter().any(|c| {
                c.kind == CrossingKind::Ford
                    && if c.ns {
                        (x - c.x).abs() < FORD_HALF_WIDTH_M && (y - c.y).abs() < 60.0
                    } else {
                        (y - c.y).abs() < FORD_HALF_WIDTH_M && (x - c.x).abs() < 60.0
                    }
            });
            if d < w && island > 0.0 {
                let channel = if ford { 0.0 } else { 1.0 };
                h = h.min(self.natural(x, y))
                    - BANK_CUT_M
                    - WATERLINE_M
                    - SHALLOW_DEPTH_M * smoothstep(w, w * 0.55, d)
                    - CHANNEL_DEPTH_M * channel * smoothstep(w * 0.55, w * 0.25, d);
                terrain = if d < w * 0.55 && !ford {
                    DEEP_WATER
                } else {
                    SHALLOW_WATER
                };
                if island < 3.0 {
                    terrain = SHALLOW_WATER;
                }
                return (h, terrain);
            }
            let cut = if island <= 0.0 {
                BANK_CUT_M * smoothstep(0.0, -6.0, island).max(0.0) * 0.6
            } else {
                BANK_CUT_M * (1.0 - smoothstep(w, w + RIVER_VALLEY_M, d))
            };
            h -= cut.max(0.0);
            if (d < w + 3.0 || island <= 0.0) && terrain == GRASS {
                terrain = SAND;
            }
        }
        if s.sea {
            if let Some(result) = self.sea(p, h, terrain) {
                return result;
            }
            let e = s
                .land
                .iter()
                .filter(|a| inside(p, &a.polygon_m))
                .map(|a| edge_distance(p, &a.polygon_m))
                .fold(f64::INFINITY, f64::min);
            let e = e.min(self.bay_coast_distance(p));
            let islet = self.islet_edge(p);
            let e = if islet > 0.0 { e.min(islet) } else { e };
            if e.is_finite() {
                // Land falls towards one coast height (a flat sea level).
                h = SEA_COAST_M + (h - SEA_COAST_M) * smoothstep(0.0, 60.0, e);
                if e < 5.0 && terrain == GRASS {
                    terrain = SAND;
                }
            }
        }
        // Lakes.
        for lake in &s.lake {
            // Islands of the sketch also stand in lakes (stage 3).
            let island = s
                .island
                .iter()
                .any(|i| dist(p, (i.center_m[0], i.center_m[1])) < i.radius_m);
            if inside(p, &lake.polygon_m) && !island {
                let e = edge_distance(p, &lake.polygon_m);
                let deep = lake.kind != "shallow" && e > 8.0;
                h = h.min(self.natural(x, y))
                    - BANK_CUT_M
                    - WATERLINE_M
                    - SHALLOW_DEPTH_M * smoothstep(0.0, 8.0, e)
                    - if deep {
                        CHANNEL_DEPTH_M * smoothstep(8.0, 20.0, e)
                    } else {
                        0.0
                    };
                return (h, if deep { DEEP_WATER } else { SHALLOW_WATER });
            }
            let e = edge_distance(p, &lake.polygon_m);
            if e < RIVER_VALLEY_M {
                h -= BANK_CUT_M * (1.0 - smoothstep(0.0, RIVER_VALLEY_M, e));
                if e < 3.0 && terrain == GRASS {
                    terrain = SAND;
                }
            }
        }
        (h, terrain)
    }
}

impl Plan<'_> {
    /// The sea at `p` (outside every land polygon): water whose depth grows
    /// with the distance to the coast, shallow on shoals; causeways stay
    /// land except under their bridges. `None` on land.
    /// Sea maps: how far `p` lies inside an island of the sketch (m), or
    /// a negative value outside every island.
    fn islet_edge(&self, p: (f64, f64)) -> f64 {
        self.sketch
            .island
            .iter()
            .map(|i| i.radius_m - dist(p, (i.center_m[0], i.center_m[1])))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    fn bay_coast_distance(&self, p: (f64, f64)) -> f64 {
        self.bay_coast
            .iter()
            .map(|&(a, b)| segment_distance(p, a, b))
            .fold(f64::INFINITY, f64::min)
    }

    fn sea(&self, p: (f64, f64), h: f64, terrain: u8) -> Option<(f64, u8)> {
        let s = self.sketch;
        // Islands of the sketch also stand in the bays (prompt 122: the
        // islet in the lagoon).
        if self.islet_edge(p) > 0.0 {
            return None;
        }
        let in_bay = s.bay.iter().any(|a| inside(p, &a.polygon_m));
        if !in_bay && s.land.iter().any(|a| inside(p, &a.polygon_m)) {
            return None;
        }
        // Under a bridge the water cuts across the whole causeway.
        let gap = self.crossings.iter().any(|c| {
            let (u, v) = if c.ns {
                (p.1 - c.y, p.0 - c.x)
            } else {
                (p.0 - c.x, p.1 - c.y)
            };
            matches!(c.kind, CrossingKind::Stone | CrossingKind::Wood)
                && u.abs() < c.kind.half_water()
                && v.abs() < 40.0
        });
        let causeway = s
            .causeway
            .iter()
            .map(|c| polyline_distance(p, &c.points_m) - c.width_m / 2.0)
            .fold(f64::INFINITY, f64::min);
        if causeway < 0.0 && !gap {
            let top = SEA_COAST_M + 0.6;
            let terrain = if terrain == GRASS && causeway > -1.5 {
                SAND
            } else {
                terrain
            };
            return Some((top.max(h.min(top + 0.5)), terrain));
        }
        let e = s
            .land
            .iter()
            .map(|a| edge_distance(p, &a.polygon_m))
            .fold(causeway.max(0.0), f64::min)
            .min(self.bay_coast_distance(p))
            .min(-self.islet_edge(p));
        let shoal = s
            .shoal
            .iter()
            .any(|c| polyline_distance(p, &c.points_m) < c.width_m / 2.0);
        let deep = !shoal && e > 10.0;
        let bed = SEA_COAST_M
            - WATERLINE_M
            - SHALLOW_DEPTH_M * smoothstep(0.0, 10.0, e)
            - if deep {
                CHANNEL_DEPTH_M * smoothstep(10.0, 25.0, e) + 1.5 * smoothstep(25.0, 120.0, e)
            } else {
                0.0
            };
        Some((bed, if deep { DEEP_WATER } else { SHALLOW_WATER }))
    }
}

/// A placed bridge (`objects.csv` row).
struct BridgeRow {
    kind: &'static str,
    variant: &'static str,
    x: f64,
    y: f64,
    heading: u16,
}

/// Measures the water along each bridge crossing's axis, picks the shortest
/// bridge that spans it plus 4 m of bank on each side and raises
/// embankments with ramps to the deck (like `gen_map`, prompt 014).
fn place_bridges(
    crossings: &[Crossing],
    (sx, sy): (usize, usize),
    half: (f64, f64),
    heights: &mut [f64],
    terrain: &mut [u8],
) -> Result<Vec<BridgeRow>, String> {
    const WOOD: [(&str, f64); 3] = [("Span8", 8.0), ("Span12", 12.0), ("Span16", 16.0)];
    const STONE: [(&str, f64); 2] = [("Arch2", 20.0), ("Arch3", 30.0)];
    let coord = |i: usize, h: f64| -h + i as f64 * CELL_M;
    let mut placed = Vec::new();
    for c in crossings.iter().filter(|c| c.kind.is_bridge()) {
        let stone = c.kind != CrossingKind::Wood;
        let long = c.kind == CrossingKind::StoneLong;
        let reach = if long { LONG_REACH_M } else { 60.0 };
        // u: along the way, v: across (the axis).
        let to_uv = |x: f64, y: f64| {
            if c.ns {
                (y - c.y, x - c.x)
            } else {
                (x - c.x, y - c.y)
            }
        };
        let (mut u0, mut u1) = (f64::INFINITY, f64::NEG_INFINITY);
        for j in 0..sy {
            for i in 0..sx {
                let (u, v) = to_uv(coord(i, half.0), coord(j, half.1));
                if v.abs() <= 3.0
                    && u.abs() < reach
                    && matches!(terrain[j * sx + i], SHALLOW_WATER | DEEP_WATER)
                {
                    u0 = u0.min(u);
                    u1 = u1.max(u);
                }
            }
        }
        if u0 > u1 {
            return Err(format!("no water at the crossing ({}, {})", c.x, c.y));
        }
        let need = u1 - u0 + 2.0 * BRIDGE_LAND_M;
        let list: &[(&str, f64)] = if stone { &STONE } else { &WOOD };
        // A long bridge: as many 20 m pieces as the water needs.
        let pieces = (need / LONG_PIECE_M).ceil().max(2.0);
        let chain = [("ArchHigh", pieces * LONG_PIECE_M)];
        let list: &[(&str, f64)] = if long { &chain } else { list };
        let Some(&(variant, length)) = list.iter().find(|v| v.1 >= need) else {
            return Err(format!(
                "the water at ({}, {}) is {:.1} m wide, too wide for the bridge",
                c.x,
                c.y,
                u1 - u0
            ));
        };
        let (rise, half_width) = (c.rise, if stone { 2.55 } else { 2.05 });
        let mid = ((u0 + u1) / 2.0 * 1000.0).round() / 1000.0;
        let (d0, d1) = (mid - length / 2.0, mid + length / 2.0);
        let at = |u: f64| -> usize {
            let (x, y) = if c.ns { (c.x, c.y + u) } else { (c.x + u, c.y) };
            let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
            let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
            j * sx + i
        };
        let bank = heights[at(d0)].max(heights[at(d1)]);
        let deck = bank + rise;
        let top_half = half_width + 1.0;
        let reach_u = rise / RAMP_GRADE + 10.0;
        let reach_v = top_half + rise / EMBANKMENT_SIDE_GRADE + 4.0;
        for j in 0..sy {
            for i in 0..sx {
                let (u, v) = to_uv(coord(i, half.0), coord(j, half.1));
                if v.abs() > reach_v || u < d0 - reach_u || u > d1 + reach_u {
                    continue;
                }
                let k = j * sx + i;
                if matches!(terrain[k], SHALLOW_WATER | DEEP_WATER) {
                    continue;
                }
                let along = if u < d0 {
                    d0 - u
                } else if u > d1 {
                    u - d1
                } else {
                    0.0
                };
                let top = deck - RAMP_GRADE * along;
                let level = top - EMBANKMENT_SIDE_GRADE * (v.abs() - top_half).max(0.0);
                if level > heights[k] {
                    heights[k] = level;
                    terrain[k] = if v.abs() <= 2.5 {
                        if stone { ROAD } else { DIRT_TRACK }
                    } else {
                        GRASS
                    };
                }
            }
        }
        // Pieces end to end (a single bridge is one piece of `length`).
        let (count, piece) = if long {
            (pieces as usize, LONG_PIECE_M)
        } else {
            (1, length)
        };
        for k in 0..count {
            // (One piece: its middle exactly, as before prompt 122.)
            let at = if long {
                d0 + piece * (k as f64 + 0.5)
            } else {
                mid
            };
            let (x, y) = if c.ns {
                (c.x, c.y + at)
            } else {
                (c.x + at, c.y)
            };
            placed.push(BridgeRow {
                kind: if stone {
                    "env_bridge_stone"
                } else {
                    "env_bridge_wood"
                },
                variant,
                x,
                y,
                heading: if c.ns { 0 } else { 90 },
            });
        }
    }
    Ok(placed)
}

/// Wood stands per kind and biome: (catalog id, variants).
fn stands(kind: &str, biome: &str) -> Vec<(&'static str, &'static [&'static str])> {
    const OAK: (&str, &[&str]) = ("env_tree_deciduous", &["Oak", "Beech", "OakYoung"]);
    const BARE: (&str, &[&str]) = ("env_tree_deciduous", &["Bare"]);
    const CONIFER: (&str, &[&str]) = ("env_tree_conifer", &["Spruce", "Pine", "SpruceYoung"]);
    const BIRCH: (&str, &[&str]) = ("env_tree_birch", &["Birch", "BirchYoung"]);
    const PALM: (&str, &[&str]) = ("env_tree_palm", &["Palm", "PalmShort", "PalmLeaning"]);
    const BUSH: (&str, &[&str]) = ("env_bush", &["Hazel", "Round", "Wide"]);
    // Prompt 120: Mediterranean stands.
    const STONE_PINE: (&str, &[&str]) = (
        "env_tree_stone_pine",
        &["StonePine", "StonePineYoung", "StonePineLeaning"],
    );
    const OLIVE: (&str, &[&str]) = ("env_tree_olive", &["Olive", "OliveOld", "OliveYoung"]);
    const CYPRESS: (&str, &[&str]) = ("env_tree_cypress", &["Cypress", "CypressShort"]);
    const MACCHIA: (&str, &[&str]) = ("env_bush_macchia", &["Holm", "Strawberry", "Broom"]);
    let winter = biome == "winter";
    match (kind, biome) {
        ("grove", "mediterranean") => vec![OLIVE, OLIVE, MACCHIA, OLIVE, CYPRESS],
        (_, "mediterranean") => vec![STONE_PINE, MACCHIA, STONE_PINE, OLIVE, MACCHIA, CYPRESS],
        // Palm and bush groves (stage 2): bushes show the groves from above.
        ("grove", _) => vec![PALM, PALM, BUSH, PALM, BUSH],
        // Rock heights (stage 3): outcrops and boulders between conifers.
        ("rock", _) => vec![
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
            CONIFER,
            ("env_rock", &["Medium", "Small", "Large"]),
            CONIFER,
        ],
        // Rock mountains (prompt 122): massifs and outcrops between a few
        // conifers.
        ("crag", _) => vec![
            (
                "env_rock_massif",
                &["Ridge", "Long", "Short", "High", "Double"],
            ),
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
            CONIFER,
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
        ],
        (_, "desert") => vec![PALM],
        ("conifer", _) => vec![CONIFER],
        ("birch", _) => vec![BIRCH],
        ("palm", _) => vec![PALM],
        ("deciduous", _) => vec![if winter { BARE } else { OAK }],
        // Mixed: in winter mostly conifers (the template shows dark woods).
        _ if winter => vec![CONIFER, CONIFER, BIRCH, CONIFER, BARE],
        _ => vec![OAK, CONIFER, OAK, BIRCH, OAK],
    }
}

/// Parameters of `cli gen-sketch`.
pub struct Options<'a> {
    pub name: &'a str,
    /// Folder of the sketches (normally `SKETCH_DIR`).
    pub sketches: &'a Path,
    pub out: &'a Path,
}

#[allow(clippy::too_many_lines)]
pub fn generate(options: &Options) -> Result<String, String> {
    let sketch = load(options.sketches, options.name)?;
    let terrain_rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let object_rules = ObjectRules::builtin().map_err(|e| e.0)?;
    let [width, depth] = sketch.size_m.ok_or("the sketch has no size_m")?;
    if width % 4 != 0 || depth % 4 != 0 || width > 4000 || depth > 4000 {
        return Err("size_m must be multiples of 4 m, at most 4000".into());
    }
    let seed = sketch.seed.unwrap_or(88);
    let biome = sketch.biome.clone().unwrap_or_else(|| "forest".into());
    let half = (f64::from(width) / 2.0, f64::from(depth) / 2.0);
    let (sx, sy) = ((width / 2 + 1) as usize, (depth / 2 + 1) as usize);
    let mut random = Random(Rng::new(seed ^ 0x736b_6574));

    // Crossings on path cell center lines (-half + 1 + 4 k).
    let snap = |v: f64, h: f64| -h + 1.0 + 4.0 * ((v + h - 1.0) / 4.0).round();
    let crossings: Vec<Crossing> = sketch
        .crossing
        .iter()
        .map(|c| {
            let ns = c.axis == "ns";
            let kind = match c.kind.as_str() {
                "stone" => CrossingKind::Stone,
                "wood" => CrossingKind::Wood,
                "stone_long" => CrossingKind::StoneLong,
                _ => CrossingKind::Ford,
            };
            let (x, y) = (c.position_m[0], c.position_m[1]);
            Crossing {
                kind,
                x: if ns { snap(x, half.0) } else { x },
                y: if ns { y } else { snap(y, half.1) },
                ns,
                rise: c.rise_m.unwrap_or_else(|| kind.rise()),
            }
        })
        .collect();

    // Ways: the sketch's roads plus straight approaches over each crossing.
    let mut ways: Vec<Way> = Vec::new();
    for c in &crossings {
        let reach = if c.kind == CrossingKind::StoneLong {
            125.0
        } else {
            55.0
        };
        let (a, b) = if c.ns {
            ((c.x, c.y - reach), (c.x, c.y + reach))
        } else {
            ((c.x - reach, c.y), (c.x + reach, c.y))
        };
        let id = if matches!(c.kind, CrossingKind::Stone | CrossingKind::StoneLong) {
            ROAD
        } else {
            DIRT_TRACK
        };
        ways.push(Way::new(&[a, b], 4.0, id));
    }
    for r in &sketch.road {
        let points: Vec<(f64, f64)> = r.points_m.iter().map(|p| (p[0], p[1])).collect();
        let id = if r.class == "road" { ROAD } else { DIRT_TRACK };
        ways.push(Way::new(&points, r.width_m / 2.0, id));
    }

    // Ground: noise, a few random bumps, the sketch's hills; flat places.
    let bumps = (0..(width as usize * depth as usize / 160_000).max(4))
        .map(|_| {
            (
                random.range(-half.0, half.0),
                random.range(-half.1, half.1),
                random.range(80.0, 200.0),
                random.range(-1.5, 3.5),
            )
        })
        .chain(
            sketch
                .hill
                .iter()
                .filter(|h| h.kind != "peak")
                .map(|h| (h.center_m[0], h.center_m[1], h.radius_m, h.height_m)),
        )
        .collect();
    let peaks = sketch
        .hill
        .iter()
        .filter(|h| h.kind == "peak")
        .map(|h| (h.center_m[0], h.center_m[1], h.radius_m, h.height_m))
        .collect();
    let mut paints = Vec::new();
    for (n, paint) in sketch.paint.iter().enumerate() {
        let id = terrain_rules
            .types
            .iter()
            .position(|t| t.name == paint.terrain)
            .ok_or_else(|| format!("unknown terrain {:?}", paint.terrain))?;
        let blotch = paint.noise_m.map(|m| noise(seed, 80 + n as u64, m));
        // Noise values lie in about 0..1: the threshold that leaves `cover`.
        paints.push((id as u8, blotch, 1.0 - paint.cover.unwrap_or(0.5)));
    }
    let mut plan = Plan {
        sketch: &sketch,
        ways,
        paints,
        plots: patchwork_plots(&sketch.patchwork, seed),
        bay_coast: bay_coast(&sketch.bay),
        crossings: crossings.clone(),
        flats: Vec::new(),
        ground: noise(seed, 1, 160.0),
        detail: noise(seed, 2, 35.0),
        bumps,
        peaks,
        crag: noise(seed, 90, 70.0),
    };
    let mut flat_spots: Vec<((f64, f64), f64)> = Vec::new();
    for s in &sketch.start {
        flat_spots.push(((s.position_m[0], s.position_m[1]), START_CLEAR_M));
    }
    for s in &sketch.supply {
        flat_spots.push(((s.position_m[0], s.position_m[1]), 20.0));
    }
    for f in &sketch.farm {
        flat_spots.push(((f.center_m[0], f.center_m[1]), 40.0));
    }
    for r in &sketch.ring {
        flat_spots.push((
            (r.center_m[0], r.center_m[1]),
            r.radius_m[0].max(r.radius_m[1]) + 10.0,
        ));
    }
    for r in &sketch.ring_road {
        flat_spots.push(((r.center_m[0], r.center_m[1]), r.radius_m + 6.0));
    }
    plan.flats = flat_spots
        .iter()
        .map(|&(c, r)| (c, r, plan.natural(c.0, c.1)))
        .collect();

    let mut heights = Vec::with_capacity(sx * sy);
    let mut terrain = Vec::with_capacity(sx * sy);
    for j in 0..sy {
        for i in 0..sx {
            let (h, t) = plan.sample(-half.0 + i as f64 * CELL_M, -half.1 + j as f64 * CELL_M);
            heights.push(h);
            terrain.push(t);
        }
    }
    let bridges = place_bridges(&crossings, (sx, sy), half, &mut heights, &mut terrain)?;
    if biome == "winter" {
        // Open ground drifts a little; fields and ways stay.
        let drift = noise(seed, 4, 55.0);
        for j in 0..sy {
            for i in 0..sx {
                let k = j * sx + i;
                let (x, y) = (-half.0 + i as f64 * CELL_M, -half.1 + j as f64 * CELL_M);
                if terrain[k] == GRASS
                    && drift.at(x, y) > 0.78
                    && sketch
                        .start
                        .iter()
                        .all(|s| dist((x, y), (s.position_m[0], s.position_m[1])) > START_CLEAR_M)
                {
                    terrain[k] = SNOW;
                }
            }
        }
    }

    // Objects.
    let mut objects = Objects::sized_rect(
        &object_rules,
        &terrain,
        Random(Rng::new(seed ^ 0x6f62_6a73)),
        (width, depth),
    );
    for s in &sketch.start {
        objects
            .reserved
            .push(((s.position_m[0], s.position_m[1]), START_CLEAR_M - 5.0));
    }
    for s in &sketch.supply {
        objects
            .reserved
            .push(((s.position_m[0], s.position_m[1]), 6.0));
    }
    for b in &bridges {
        objects.reserved.push(((b.x, b.y), 22.0));
    }
    let garden = Garden {
        tree_kind: "env_tree_deciduous",
        trees: if biome == "winter" {
            &["Bare"]
        } else {
            &["OakYoung"]
        },
        fence: "env_fence_wood",
    };
    for b in &sketch.building {
        let variant = b.variant.clone().unwrap_or_else(|| "Intact".into());
        objects.place(
            &b.kind,
            &variant,
            (b.position_m[0], b.position_m[1]),
            b.heading_deg,
            0.5,
        );
    }
    for f in &sketch.farm {
        objects.farmstead((f.center_m[0], f.center_m[1]), &garden, &f.house, false);
    }
    for r in &sketch.row {
        let (a, b) = ((r.from_m[0], r.from_m[1]), (r.to_m[0], r.to_m[1]));
        let len = dist(a, b);
        let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        // Long sides across the row (staggered huts and barracks).
        let heading = (heading_of(dx, dy) + 90) % 360;
        let mut t = 0.0;
        let mut k = 0;
        while t <= len {
            let kind = &r.kinds[k % r.kinds.len()];
            let variant = variant_of(&mut objects, r.damaged);
            objects.place(kind, variant, (a.0 + dx * t, a.1 + dy * t), heading, 0.5);
            t += r.gap_m + 8.0;
            k += 1;
        }
    }
    for r in &sketch.ring {
        for k in 0..r.count {
            // Angle by a polynomial-free walk: points on the ellipse from the
            // unit circle by rotation steps (no trigonometry).
            let (c, s) = unit_circle(k, r.count);
            let p = (
                r.center_m[0] + c * r.radius_m[0],
                r.center_m[1] + s * r.radius_m[1],
            );
            let heading = heading_of(-s * r.radius_m[0], c * r.radius_m[1]);
            let kind = &r.kinds[k as usize % r.kinds.len()];
            // Walls, fences and hedges (a walled height, stage 3) are
            // segments that touch.
            let segment = ["env_wall", "env_fence", "env_hedge"]
                .iter()
                .any(|w| kind.starts_with(w));
            let (variant, margin) = if segment {
                ("Segment", -0.05)
            } else {
                (variant_of(&mut objects, r.damaged), 0.4)
            };
            objects.place(kind, variant, p, heading, margin);
        }
    }
    for t in &sketch.town {
        town(
            &mut objects,
            &object_rules,
            t,
            &plan.ways[crossings.len()..],
        );
    }
    for v in &sketch.village {
        village(&mut objects, v, &plan.ways);
    }
    for r in &sketch.rail {
        let points: Vec<(f64, f64)> = r.points_m.iter().map(|p| (p[0], p[1])).collect();
        objects.poles(&Way::new(&points, RAIL_BED_M, DIRT_TRACK), 7.0);
    }
    let field_at = |x: f64, y: f64, terrain: &[u8]| {
        let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
        let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
        matches!(
            terrain[j * sx + i],
            FIELD_GRAIN | FIELD_GREEN | FIELD_PLOUGHED | SAND | SHALLOW_WATER | DEEP_WATER
        )
    };
    for (n, w) in sketch.wood.iter().enumerate() {
        let list = stands(&w.kind, &biome);
        let polygon = w.polygon_m.clone();
        let stand_noise = noise(seed, 10 + n as u64, 60.0);
        let area = bounds(&polygon);
        objects.wood(
            &list,
            w.spacing_m.unwrap_or(6.5),
            area,
            &|x, y| inside((x, y), &polygon) && !field_at(x, y, &terrain),
            &stand_noise,
        );
    }
    for (n, w) in sketch.scatter.iter().enumerate() {
        let list = stands(&w.kind, &biome);
        let polygon = w.polygon_m.clone();
        let stand_noise = noise(seed, 40 + n as u64, 60.0);
        let clump = noise(seed, 60 + n as u64, 70.0);
        let area = bounds(&polygon);
        objects.wood(
            &list,
            w.spacing_m.unwrap_or(30.0),
            area,
            &|x, y| inside((x, y), &polygon) && !field_at(x, y, &terrain) && clump.at(x, y) > 0.45,
            &stand_noise,
        );
    }
    let summary = format!(
        "{} buildings, {} trees, {} other objects, {} bridges",
        objects.buildings,
        objects.trees,
        objects.other,
        bridges.len()
    );
    let mut rows = vec![
        format!(
            "# Generated by `cli gen-sketch {}` from data/map_sketches (prompt 088, docs/MAPS.md). Rules: data/objects.toml.",
            options.name
        ),
        CSV_HEADER.to_string(),
    ];
    for b in &bridges {
        rows.push(format!(
            "{},{},{},{},{}",
            b.kind,
            b.variant,
            (b.x * 1000.0).round() as i64,
            (b.y * 1000.0).round() as i64,
            b.heading
        ));
    }
    rows.append(&mut objects.rows);
    drop(objects);
    let scenery = scenery_rows(&sketch, seed, &bridges, &|x, y| {
        let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
        let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
        matches!(terrain[j * sx + i], SHALLOW_WATER | DEEP_WATER)
    });
    let summary = if scenery.is_empty() {
        summary
    } else {
        format!("{summary}, {} piers and stones", scenery.len())
    };
    rows.extend(scenery);

    // Files.
    let low = heights.iter().copied().fold(f64::MAX, f64::min);
    let high = heights.iter().copied().fold(f64::MIN, f64::max);
    if high - low > 250.0 {
        return Err(format!("relief of {:.1} m is too high", high - low));
    }
    // 1 mm steps up to 60 m of relief (every sketch map before prompt 122),
    // coarser above (16-bit PNG).
    let step_mm: u32 = if high - low > 60.0 {
        ((high - low + 2.0) * 1000.0 / 65535.0).ceil() as u32
    } else {
        1
    };
    let base_mm = ((low - 1.0).floor() * 1000.0) as i32;
    let encoded: Vec<u8> = heights
        .iter()
        .flat_map(|h| {
            let value = ((h * 1000.0 - f64::from(base_mm)) / f64::from(step_mm)).round();
            (value.clamp(0.0, 65535.0) as u16).to_be_bytes()
        })
        .collect();
    std::fs::create_dir_all(options.out).map_err(|e| e.to_string())?;
    let size = (sx as u32, sy as u32);
    write_png_rect(&options.out.join("height.png"), size, &encoded, None)?;
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
    write_png_rect(
        &options.out.join("terrain.png"),
        size,
        &terrain,
        Some(palette),
    )?;
    let mut csv = rows.join("\n");
    csv.push('\n');
    std::fs::write(options.out.join("objects.csv"), csv).map_err(|e| e.to_string())?;

    let name = options.name;
    let mut toml = String::new();
    let _ = write!(
        toml,
        "# Generated by `cli gen-sketch {name}` from data/map_sketches/ (prompt 088);\n\
         # change the sketch and generate again (docs/MAPS.md).\n\
         name = \"{name}\"\n\
         generator = \"cli gen-sketch {name}\"\n\
         cell_size_mm = 2000\n\
         samples = [{}, {}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = {step_mm}\n\
         biome = \"{biome}\"\n",
        sx, sy,
    );
    if let Some(look) = &sketch.look {
        let _ = writeln!(toml, "look = \"{look}\"");
    }
    let _ = writeln!(
        toml,
        "water_flow_deg = {}",
        sketch.water_flow_deg.unwrap_or(180)
    );
    if let Some(weather) = &sketch.weather {
        let _ = writeln!(toml, "weather = \"{weather}\"");
    }
    for s in &sketch.start {
        let _ = write!(
            toml,
            "\n[[start]]\nplayer = {}\nteam = {}\nposition_m = [{}, {}]\n",
            s.player,
            s.team,
            s.position_m[0].round() as i32,
            s.position_m[1].round() as i32
        );
    }
    for m in sketch.supply.iter().chain(&sketch.marker) {
        let _ = write!(
            toml,
            "\n[[marker]]\nname = \"{}\"\nposition_m = [{}, {}]\n",
            m.name,
            m.position_m[0].round() as i32,
            m.position_m[1].round() as i32
        );
    }
    for (n, w) in sketch.waterfall.iter().enumerate() {
        for (end, p) in [("top", w.top_m), ("foot", w.foot_m)] {
            let _ = write!(
                toml,
                "\n[[marker]]\nname = \"waterfall_{}_{end}\"\nposition_m = [{}, {}]\n",
                n + 1,
                p[0].round() as i32,
                p[1].round() as i32
            );
        }
    }
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;
    Ok(format!(
        "wrote {}: {sx} x {sy} samples, heights {low:.2} .. {high:.2} m, {summary}",
        options.out.display(),
    ))
}

/// Height profile of a rock mountain (prompt 122) at `d` (distance from
/// the center over the radius, 0..1) with the crag noise value `n` (0..1):
/// a pointed top with concave flanks, `(1 - d)^1.8`, and crags from ridged
/// noise that are strongest on the upper flanks; 0 at the foot.
fn peak_profile(d: f64, n: f64) -> f64 {
    let rest = 1.0 - d;
    let ridge = 1.0 - (2.0 * n - 1.0).abs();
    // (1 - d)^1.8 and (1 - d)^0.8 by sqrt only: x^0.8 ≈ x^0.75 · x^0.05
    // is close enough for a profile; x^0.75 = sqrt(x · sqrt(x)).
    let p075 = (rest * rest.sqrt()).sqrt();
    rest * p075 * 0.96 + 0.07 * p075 * ridge * ridge
}

/// Piers and rapids stones (prompt 122): rows of `objects.csv` in the
/// water, beside the object placer (which keeps out of water), from an own
/// random stream (sketches without them stay the same).
fn scenery_rows(
    sketch: &Sketch,
    seed: u64,
    bridges: &[BridgeRow],
    water: &dyn Fn(f64, f64) -> bool,
) -> Vec<String> {
    const PIER_SEGMENT_M: f64 = 10.0;
    let mut rows = Vec::new();
    for pier in &sketch.pier {
        let ([ax, ay], [bx, by]) = (pier.points_m[0], pier.points_m[1]);
        let len = dist((ax, ay), (bx, by));
        let (dx, dy) = ((bx - ax) / len, (by - ay) / len);
        let heading = heading_of(dx, dy);
        let count = (len / PIER_SEGMENT_M).floor().max(1.0) as usize;
        for k in 0..count {
            let t = PIER_SEGMENT_M * (k as f64 + 0.5);
            let variant = if k + 1 == count && pier.boat.unwrap_or(true) {
                "Boat"
            } else {
                "Span"
            };
            rows.push(format!(
                "env_pier_wood,{variant},{},{},{heading}",
                ((ax + dx * t) * 1000.0).round() as i64,
                ((ay + dy * t) * 1000.0).round() as i64
            ));
        }
    }
    let mut random = Random(Rng::new(seed ^ 0x7261_7069));
    for r in &sketch.rapids {
        let lengths: Vec<f64> = r
            .points_m
            .windows(2)
            .map(|s| dist((s[0][0], s[0][1]), (s[1][0], s[1][1])))
            .collect();
        let total: f64 = lengths.iter().sum();
        let mut placed: Vec<(f64, f64)> = Vec::new();
        let mut tries = 0;
        while placed.len() < r.count as usize && tries < r.count * 30 {
            tries += 1;
            let mut t = random.range(0.0, total);
            let across = random.range(-r.width_m / 2.0, r.width_m / 2.0);
            let variant = if random.unit() < 0.6 {
                "Small"
            } else {
                "Medium"
            };
            let heading = (random.unit() * 360.0) as u16 % 360;
            let mut k = 0;
            while k + 1 < lengths.len() && t > lengths[k] {
                t -= lengths[k];
                k += 1;
            }
            let (a, b) = (r.points_m[k], r.points_m[k + 1]);
            let (dx, dy) = ((b[0] - a[0]) / lengths[k], (b[1] - a[1]) / lengths[k]);
            let p = (a[0] + dx * t - dy * across, a[1] + dy * t + dx * across);
            if !water(p.0, p.1)
                || bridges.iter().any(|b| dist(p, (b.x, b.y)) < 16.0)
                || placed.iter().any(|&q| dist(p, q) < 3.0)
            {
                continue;
            }
            placed.push(p);
            rows.push(format!(
                "env_rock,{variant},{},{},{heading}",
                (p.0 * 1000.0).round() as i64,
                (p.1 * 1000.0).round() as i64
            ));
        }
    }
    rows
}

/// The plots of the patchworks (own random stream: sketches without a
/// patchwork stay the same).
fn patchwork_plots(patchworks: &[Patchwork], seed: u64) -> Vec<Area> {
    const LANE_M: f64 = 6.0;
    let mut random = Random(Rng::new(seed ^ 0x7061_7463));
    let mut plots = Vec::new();
    for pw in patchworks {
        let [ax, ay] = pw.axis.unwrap_or([1.0, 0.0]);
        let n = (ax * ax + ay * ay).sqrt();
        let (a, b) = ((ax / n, ay / n), (-ay / n, ax / n));
        let r = bounds(&pw.polygon_m);
        let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
        let reach = dist((r.x0, r.y0), (r.x1, r.y1)) / 2.0;
        // Rows across b, plots along a with varying lengths.
        let mut v = -reach;
        while v < reach {
            let width = pw.size_m[1] * random.range(0.67, 1.33);
            let mut u = -reach + random.range(0.0, pw.size_m[0]);
            while u < reach {
                let length = pw.size_m[0] * random.range(0.67, 1.33);
                let roll = random.unit();
                let k = (random.unit() * pw.kinds.len() as f64) as usize;
                let corner = |s: f64, t: f64| [cx + a.0 * s + b.0 * t, cy + a.1 * s + b.1 * t];
                let polygon = vec![
                    corner(u, v),
                    corner(u + length, v),
                    corner(u + length, v + width),
                    corner(u, v + width),
                ];
                let mid = (
                    cx + a.0 * (u + length / 2.0) + b.0 * (v + width / 2.0),
                    cy + a.1 * (u + length / 2.0) + b.1 * (v + width / 2.0),
                );
                if roll >= pw.meadow && inside(mid, &pw.polygon_m) {
                    plots.push(Area {
                        kind: pw.kinds[k.min(pw.kinds.len() - 1)].clone(),
                        spacing_m: None,
                        polygon_m: polygon,
                    });
                }
                u += length + LANE_M;
            }
            v += width + LANE_M;
        }
    }
    plots
}

/// The outline segments of `bays` whose middle lies in no other bay.
fn bay_coast(bays: &[Area]) -> Vec<(P, P)> {
    let mut coast = Vec::new();
    for (k, bay) in bays.iter().enumerate() {
        let n = bay.polygon_m.len();
        for i in 0..n {
            let (a, b) = (bay.polygon_m[i], bay.polygon_m[(i + 1) % n]);
            let mid = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
            if !bays
                .iter()
                .enumerate()
                .any(|(j, other)| j != k && inside(mid, &other.polygon_m))
            {
                coast.push((a, b));
            }
        }
    }
    coast
}

/// Street building of a town (see `Town`).
fn town(objects: &mut Objects, rules: &ObjectRules, t: &Town, ways: &[Way]) {
    // Length (along the street) and depth of every kind.
    let sizes: Vec<(&str, f64, f64)> = t
        .kinds
        .iter()
        .map(|k| {
            let probe = editor::PlacedObject {
                kind: k.clone(),
                variant: "Intact".into(),
                x_mm: 0,
                y_mm: 0,
                heading_deg: 0,
            };
            let (ex, ey) = editor::compound::half_extent(rules, &probe);
            let (a, b) = (ex as f64 / 500.0, ey as f64 / 500.0);
            (k.as_str(), a.max(b), a.min(b))
        })
        .collect();
    let setback = t.setback_m.unwrap_or(3.0);
    let gap = t.gap_m.unwrap_or(4.0);
    for way in ways {
        for s in way.points.windows(2) {
            let (a, b) = (s[0], s[1]);
            let len = dist(a, b);
            if len < 1.0 {
                continue;
            }
            let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let heading = heading_of(dx, dy);
            for side in [1.0, -1.0] {
                // Left of the direction for side 1.
                let (nx, ny) = (dy * side, -dx * side);
                let mut u = objects.random.range(0.0, gap);
                loop {
                    let k = (objects.random.unit() * sizes.len() as f64) as usize;
                    let (kind, length, depth) = sizes[k.min(sizes.len() - 1)];
                    if u + length > len {
                        break;
                    }
                    let offset = way.half + setback + objects.random.range(0.0, 2.0) + depth / 2.0;
                    let c = u + length / 2.0;
                    let p = (a.0 + dx * c + nx * offset, a.1 + dy * c + ny * offset);
                    let variant = variant_of(objects, t.damaged);
                    let h = if side > 0.0 {
                        heading
                    } else {
                        (heading + 180) % 360
                    };
                    if inside(p, &t.polygon_m) {
                        objects.place(kind, variant, p, h, 1.0);
                    }
                    u += length + gap * objects.random.range(0.5, 1.5);
                }
            }
        }
    }
}

/// Loose houses of a village (see `Village`).
fn village(objects: &mut Objects, v: &Village, ways: &[Way]) {
    let area = bounds(&v.polygon_m);
    let kinds: Vec<&str> = v.kinds.iter().map(String::as_str).collect();
    let mut tries = 0;
    let mut placed = 0;
    while placed < v.count && tries < v.count * 40 {
        tries += 1;
        let p = (
            objects.random.range(area.x0, area.x1),
            objects.random.range(area.y0, area.y1),
        );
        let kind = objects.pick(&kinds);
        let variant = variant_of(objects, v.damaged);
        let turn = if objects.random.unit() < 0.5 { 0 } else { 180 };
        let heading = way_heading(ways, p, 40.0).map_or_else(
            || (objects.random.unit() * 4.0) as u16 % 4 * 90,
            |h| (h + turn) % 360,
        );
        if inside(p, &v.polygon_m) && objects.place(kind, variant, p, heading, 3.0) {
            placed += 1;
        }
    }
    // Broken walls: short runs of a few segments.
    let mut tries = 0;
    let mut runs = 0;
    while runs < v.walls && tries < v.walls * 40 {
        tries += 1;
        let p = (
            objects.random.range(area.x0, area.x1),
            objects.random.range(area.y0, area.y1),
        );
        let along_x = objects.random.unit() < 0.5;
        let segments = 2 + (objects.random.unit() * 4.0) as u32;
        if !inside(p, &v.polygon_m) {
            continue;
        }
        runs += 1;
        for k in 0..segments {
            let s = f64::from(k) * 2.4;
            let q = if along_x {
                (p.0 + s, p.1)
            } else {
                (p.0, p.1 + s)
            };
            let variant = if objects.random.unit() < 0.6 {
                "Broken"
            } else {
                "Low"
            };
            objects.place(
                "env_wall_mud",
                variant,
                q,
                if along_x { 90 } else { 0 },
                -0.05,
            );
        }
    }
}

/// Heading of the way segment nearest to `p` within `reach` m.
fn way_heading(ways: &[Way], p: (f64, f64), reach: f64) -> Option<u16> {
    let mut best: Option<(f64, u16)> = None;
    for way in ways {
        for s in way.points.windows(2) {
            let d = segment_distance(p, [s[0].0, s[0].1], [s[1].0, s[1].1]);
            if d < reach && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, heading_of(s[1].0 - s[0].0, s[1].1 - s[0].1)));
            }
        }
    }
    best.map(|(_, h)| h)
}

fn variant_of(objects: &mut Objects, damaged: f64) -> &'static str {
    let roll = objects.random.unit();
    if roll < damaged * 0.35 {
        "Ruin"
    } else if roll < damaged {
        "Damaged"
    } else {
        "Intact"
    }
}

/// Point `k` of `n` on the unit circle (cos, sin), by repeated rotation
/// (only basic arithmetic: the same on every platform).
fn unit_circle(k: u32, n: u32) -> (f64, f64) {
    // cos and sin of 2π / n from their Taylor series (close enough for
    // placing huts on a ring).
    let a = 2.0 * std::f64::consts::PI / f64::from(n.max(3));
    let a2 = a * a;
    let cos = 1.0 - a2 / 2.0 + a2 * a2 / 24.0 - a2 * a2 * a2 / 720.0 + a2 * a2 * a2 * a2 / 40320.0;
    let sin = a
        * (1.0 - a2 / 6.0 + a2 * a2 / 120.0 - a2 * a2 * a2 / 5040.0
            + a2 * a2 * a2 * a2 / 362_880.0);
    let (mut c, mut s) = (1.0, 0.0);
    for _ in 0..k {
        (c, s) = (c * cos - s * sin, c * sin + s * cos);
    }
    (c, s)
}

#[cfg(test)]
mod tests {
    use super::{ALL, Options, generate};
    use sim::Map;

    /// Every sketch map loads with its starts, and the same sketch gives the
    /// same files twice.
    #[test]
    fn sketch_maps_are_the_same_every_time() {
        // Tests run in the crate folder; sketches live in the workspace.
        let sketches = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/map_sketches"
        ));
        // One thread per map and run (the large maps take long unoptimized).
        std::thread::scope(|scope| {
            for name in ALL {
                scope.spawn(move || check(name, sketches));
            }
        });
    }

    fn check(name: &str, sketches: &std::path::Path) {
        {
            let dirs: Vec<_> = (0..2)
                .map(|k| {
                    std::env::temp_dir()
                        .join(format!("mas-gen-sketch-{name}-{k}-{}", std::process::id()))
                })
                .collect();
            std::thread::scope(|scope| {
                for dir in &dirs {
                    scope.spawn(move || {
                        let _ = std::fs::remove_dir_all(dir);
                        generate(&Options {
                            name,
                            sketches,
                            out: dir,
                        })
                        .unwrap();
                    });
                }
            });
            for file in ["map.toml", "height.png", "terrain.png", "objects.csv"] {
                let a = std::fs::read(dirs[0].join(file)).unwrap();
                let b = std::fs::read(dirs[1].join(file)).unwrap();
                assert!(a == b, "{name}/{file} differs between two runs");
            }
            let read = |f: &str| std::fs::read(dirs[0].join(f)).unwrap();
            let toml = String::from_utf8(read("map.toml")).unwrap();
            let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
            map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
                .unwrap();
            assert!(map.starts().len() >= 2, "{name}: starts");
            for dir in &dirs {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
    }
}
