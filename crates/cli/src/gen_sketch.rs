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
//! Prompt 126 added table mountains (`[[mesa]]`: cliffs, scree, steps,
//! hollows and ramps for vehicles) and dry scrub (`kind = "scrub"`).
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
const MUD: u8 = 4;
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
pub const ALL: [&str; 15] = [
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
    "river_front",
    "trench_line",
    "river_fork_front",
    "mesa_desert",
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
    /// Prompt 123: `"north"` makes the map mirror-symmetric: the north half
    /// (up to the path cell center line next to y = 0) is generated and
    /// copied, mirrored, onto the south half (ground, terrain, objects).
    #[serde(default)]
    mirror: Option<String>,
    /// Prompt 123: prepared positions of field works (gun pits, bunkers,
    /// trenches) facing the enemy.
    #[serde(default)]
    works: Vec<Works>,
    /// Prompt 124: trench lines (prepared field works of the map).
    #[serde(default)]
    trench: Vec<TrenchDef>,
    /// Prompt 124: barbed wire along lines, with gaps.
    #[serde(default)]
    wire: Vec<WireDef>,
    /// Prompt 124: lanes kept free of craters, wire and objects (ways for
    /// vehicles through a crater field).
    #[serde(default)]
    lane: Vec<Strip>,
    /// Prompt 124: shell crater fields (prepared craters of the map).
    #[serde(default)]
    crater_field: Vec<CraterField>,
    /// Prompt 124: wet hollows with puddles.
    #[serde(default)]
    marsh: Vec<Marsh>,
    /// Prompt 124: camps of tents, dumps and stores.
    #[serde(default)]
    camp: Vec<Camp>,
    /// Prompt 126: table mountains with steps, hollows and ramps.
    #[serde(default)]
    mesa: Vec<MesaDef>,
}

/// A trench line (prompt 124). `fire`: the line becomes a zigzag trace of
/// teeth `tooth_m` long, every tooth a fire bay pushed `depth_m` towards
/// `facing_m` (a direction) over the middle of the tooth, with MG nests in
/// every `nest_every`-th bay and dugouts behind every `dugout_every`-th
/// tooth (0: none); the simulation digs it as a fire trench (which adds its
/// own small zigzag). `comm`: a straight communication trench along the
/// points. Both become prepared field works of the map (`map.toml`
/// `[[field_work]]`), dug before the first tick like an engineer's trench.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct TrenchDef {
    kind: String,
    points_m: Vec<P>,
    #[serde(default)]
    tooth_m: Option<f64>,
    #[serde(default)]
    depth_m: Option<f64>,
    #[serde(default)]
    facing_m: Option<P>,
    #[serde(default)]
    nest_every: u32,
    #[serde(default)]
    dugout_every: u32,
}

/// Barbed wire along a line (prompt 124): prepared wire works of the map,
/// cut open `gap_m` wide (default 16) at the points of `gaps_m` and where a
/// lane crosses.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct WireDef {
    points_m: Vec<P>,
    #[serde(default)]
    gaps_m: Vec<P>,
    #[serde(default)]
    gap_m: Option<f64>,
}

/// A field of shell craters (prompt 124): about `density_ha` craters per
/// hectare inside the polygon (thinning out over `edge_m` towards the
/// outline), calibers drawn from `calibers` (`[mm, weight]`), a share
/// `water` with rain water in the bowl. They become prepared craters of
/// the map (`map.toml` `[[crater]]`), made before the first tick like
/// craters of shells. None on lanes, ways, water, near trenches, wire,
/// bridges, starts and supply points. Own random stream.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct CraterField {
    polygon_m: Vec<P>,
    density_ha: f64,
    calibers: Vec<P>,
    #[serde(default)]
    water: f64,
    #[serde(default)]
    edge_m: Option<f64>,
}

/// Wet hollows (prompt 124): `count` puddles of `radius_m` (min, max)
/// inside the polygon: a shallow dip of mud with standing water (the water
/// is presentation only, `map.toml` `[[puddle]]`). Own random stream.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Marsh {
    polygon_m: Vec<P>,
    count: u32,
    radius_m: P,
}

/// A camp (prompt 124): `count` objects of each item placed loosely inside
/// the polygon, turned along the nearest way.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Camp {
    polygon_m: Vec<P>,
    items: Vec<CampItem>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct CampItem {
    kind: String,
    variant: String,
    count: u32,
}

/// Prepared field works (prompt 123), placed as a group facing `facing_m`
/// (a direction, x east and y south): `battery`: four sandbag gun pits
/// 15 m apart in a line across the facing and a trench section on each
/// flank (a way in from behind stays free); `bridgehead`: an MG bunker,
/// three trench sections in a line beside it and a sandbag position.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Works {
    kind: String,
    position_m: P,
    facing_m: P,
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
    /// Prompt 123: radius kept flat and free of objects around the point
    /// (gun positions); supply points are kept flat over 20 m anyway.
    #[serde(default)]
    flat_m: Option<f64>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct RiverDef {
    width_m: f64,
    points_m: Vec<P>,
    /// Prompt 124: a brook, shallow over its whole width (no deep channel).
    #[serde(default)]
    shallow: bool,
    /// Prompt 125: the river cuts through hills: its water follows the
    /// plain ground (the ground noise without bumps and hills, like a
    /// `[[lake]] kind = "river"`) and the banks fall to it over `valley_m`
    /// (default 40 m) instead of the water climbing over the hills.
    #[serde(default)]
    through_hills: bool,
    /// Width of the bank valley beside the water (m); default 40.
    #[serde(default)]
    valley_m: Option<f64>,
}

/// An island: a circle (`center_m`, `radius_m`) or (prompt 123) a polygon.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct Island {
    #[serde(default)]
    center_m: P,
    #[serde(default)]
    radius_m: f64,
    #[serde(default)]
    polygon_m: Vec<P>,
}

impl Island {
    /// How far `p` lies inside the island (m); negative outside.
    fn depth(&self, p: (f64, f64)) -> f64 {
        if self.polygon_m.is_empty() {
            self.radius_m - dist(p, (self.center_m[0], self.center_m[1]))
        } else if inside(p, &self.polygon_m) {
            edge_distance(p, &self.polygon_m)
        } else {
            -edge_distance(p, &self.polygon_m)
        }
    }
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
    /// Puddles of the marshes (prompt 124): center and radius.
    puddles: Vec<(f64, f64, f64)>,
    /// Table mountains (prompt 126) and the noise that frays their edges.
    mesas: Vec<MesaPlan>,
    fray: Fray,
}

impl Plan<'_> {
    fn natural(&self, x: f64, y: f64) -> f64 {
        let mut h = 2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y);
        if !self.mesas.is_empty() {
            h += self
                .mesas
                .iter()
                .map(|m| m.height((x, y), &self.fray))
                .sum::<f64>();
        }
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

    /// The plain ground at `(x, y)`: the ground noise without bumps,
    /// hills and mountains (prompt 125; the level of rivers through hills
    /// and of broad rivers, prompt 123).
    fn plain(&self, x: f64, y: f64) -> f64 {
        2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y)
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
        // Prompt 124: lanes keep their ground (firm ways through the mud).
        let lane = s
            .lane
            .iter()
            .any(|l| polyline_distance(p, &l.points_m) < l.width_m / 2.0);
        for (paint, (id, blotch, threshold)) in s.paint.iter().zip(&self.paints) {
            if lane || !inside(p, &paint.polygon_m) {
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
        // Prompt 126: a dirt track on every mesa ramp.
        if self.mesas.iter().any(|m| m.on_track(p)) {
            terrain = DIRT_TRACK;
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
            let valley = river.valley_m.unwrap_or(RIVER_VALLEY_M);
            let d = polyline_distance(p, &river.points_m);
            if d > river.width_m / 2.0 + valley {
                continue;
            }
            let w = self.river_half(river, p);
            if d >= w + valley {
                continue;
            }
            // Prompt 125: the ground the water and banks are cut into (the
            // plain ground for a river through hills).
            let base = if river.through_hills {
                self.plain(x, y)
            } else {
                self.natural(x, y)
            };
            let island = -self.islet_edge(p);
            let ford = river.shallow
                || self.crossings.iter().any(|c| {
                    c.kind == CrossingKind::Ford
                        && if c.ns {
                            (x - c.x).abs() < FORD_HALF_WIDTH_M && (y - c.y).abs() < 60.0
                        } else {
                            (y - c.y).abs() < FORD_HALF_WIDTH_M && (x - c.x).abs() < 60.0
                        }
                });
            if d < w && island > 0.0 {
                let channel = if ford { 0.0 } else { 1.0 };
                h = h.min(base)
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
            if river.through_hills && island > 0.0 {
                // The banks fall from the hills to the cut plain ground.
                let level = base - BANK_CUT_M;
                h = h.min(level + (h - level) * smoothstep(w, w + valley, d));
            } else {
                let cut = if island <= 0.0 {
                    BANK_CUT_M * smoothstep(0.0, -6.0, island).max(0.0) * 0.6
                } else {
                    BANK_CUT_M * (1.0 - smoothstep(w, w + valley, d))
                };
                h -= cut.max(0.0);
            }
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
            if lake.kind == "river" {
                match self.river_lake(p, h, terrain, lake) {
                    (h, t, true) => return (h, t),
                    (nh, nt, false) => {
                        h = nh;
                        terrain = nt;
                    }
                }
                continue;
            }
            // Islands of the sketch also stand in lakes (stage 3).
            let island = self.islet_edge(p) > 0.0;
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
        // Puddles of the marshes (prompt 124): a shallow dip of mud.
        for &(px, py, r) in &self.puddles {
            let d = dist(p, (px, py));
            if d < r + 1.5 {
                h -= 0.3 * (1.0 - smoothstep(r * 0.5, r + 1.5, d));
                if d < r + 0.5 {
                    terrain = MUD;
                }
            }
        }
        (h, terrain)
    }
}

impl Plan<'_> {
    /// A broad river drawn as a lake polygon (`kind = "river"`, prompt 123):
    /// the water follows the plain ground (the noise without bumps and
    /// hills) 3 m below it, so hills beside it fall to the water over
    /// a steep bank instead of lifting it; islands of the sketch slope down
    /// to the bank level within 25 m of their shore; shoals keep the water
    /// shallow (fords). Returns height, terrain and whether `p` is water.
    fn river_lake(&self, p: (f64, f64), h: f64, terrain: u8, lake: &Area) -> (f64, u8, bool) {
        const SHORE_M: f64 = 45.0;
        const ISLAND_SHORE_M: f64 = 25.0;
        let level = 2.4 * self.ground.at(p.0, p.1) + 0.5 * self.detail.at(p.0, p.1) - BANK_CUT_M;
        let island = self.islet_edge(p);
        if inside(p, &lake.polygon_m) {
            if island > 0.0 {
                let h = level + (h - level) * smoothstep(0.0, ISLAND_SHORE_M, island);
                let terrain = if island < 4.0 && terrain == GRASS {
                    SAND
                } else {
                    terrain
                };
                return (h, terrain, false);
            }
            let e = edge_distance(p, &lake.polygon_m).min(-island);
            let shoal = self
                .sketch
                .shoal
                .iter()
                .any(|c| polyline_distance(p, &c.points_m) < c.width_m / 2.0);
            let deep = !shoal && e > 8.0;
            let h = level
                - WATERLINE_M
                - SHALLOW_DEPTH_M * smoothstep(0.0, 8.0, e)
                - if deep {
                    CHANNEL_DEPTH_M * smoothstep(8.0, 20.0, e)
                } else {
                    0.0
                };
            return (h, if deep { DEEP_WATER } else { SHALLOW_WATER }, true);
        }
        let e = edge_distance(p, &lake.polygon_m);
        if e >= SHORE_M {
            return (h, terrain, false);
        }
        let h = level + (h - level) * smoothstep(0.0, SHORE_M, e);
        let terrain = if e < 3.0 && terrain == GRASS {
            SAND
        } else {
            terrain
        };
        (h, terrain, false)
    }

    /// The sea at `p` (outside every land polygon): water whose depth grows
    /// with the distance to the coast, shallow on shoals; causeways stay
    /// land except under their bridges. `None` on land.
    /// Sea maps: how far `p` lies inside an island of the sketch (m), or
    /// a negative value outside every island.
    fn islet_edge(&self, p: (f64, f64)) -> f64 {
        self.sketch
            .island
            .iter()
            .map(|i| i.depth(p))
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
    /// Index of its crossing (pieces of a chain share it).
    crossing: usize,
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
    for (index, c) in crossings
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind.is_bridge())
    {
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
                crossing: index,
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
        // Prompt 126: dry scrub in desert hollows: bushes and stones.
        ("scrub", _) => vec![
            ("env_bush", &["Round", "Wide", "Hazel"]),
            ("env_rock", &["Small", "Medium"]),
            ("env_bush", &["Wide", "Round"]),
        ],
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
        // Prompt 124: a shot-up wood: bare and broken trees, stumps and
        // fallen trunks.
        ("shattered", _) => vec![
            BARE,
            ("env_tree_stump", &["Broken", "Broken", "Sawn"]),
            BARE,
            ("env_log_fallen", &["Log"]),
            ("env_tree_stump", &["Broken"]),
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

    // Prompt 123: the mirror axis, the path cell center line next to y = 0
    // (it maps path cells, height samples and bridge axes onto themselves).
    // Prompt 124: `point` turns the north half by 180° about the path
    // cell center point next to the origin.
    let mirror = match sketch.mirror.as_deref() {
        None => None,
        Some("north") => Some(Mirror::North(snap(0.0, half.1))),
        Some("point") => Some(Mirror::Point(snap(0.0, half.0), snap(0.0, half.1))),
        Some(other) => {
            return Err(format!("unknown mirror {other:?} (\"north\" or \"point\")"));
        }
    };

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
    let puddles = marsh_puddles(&sketch, seed, &ways, mirror);
    let mut plan = Plan {
        sketch: &sketch,
        puddles: puddles.clone(),
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
        mesas: sketch
            .mesa
            .iter()
            .map(MesaPlan::new)
            .collect::<Result<_, _>>()?,
        fray: [
            noise(seed, 126, 60.0),
            noise(seed, 127, 20.0),
            noise(seed, 128, 7.0),
        ],
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
    for m in sketch.marker.iter().chain(&sketch.supply) {
        if let Some(r) = m.flat_m {
            flat_spots.push(((m.position_m[0], m.position_m[1]), r));
        }
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
    if let Some(Mirror::Point(cx, cy)) = mirror {
        blend_axis(&mut heights, sx, half, (cx, cy));
    }
    if let Some(m) = mirror {
        mirror_raster(&mut heights, sx, half, m);
        mirror_raster(&mut terrain, sx, half, m);
    }
    let mut bridges = place_bridges(&crossings, (sx, sy), half, &mut heights, &mut terrain)?;
    if let Some(m) = mirror {
        // Again, for the embankments of the bridges in the north.
        mirror_raster(&mut heights, sx, half, m);
        mirror_raster(&mut terrain, sx, half, m);
        bridges = mirror_bridges(bridges, m)?;
        // Prompt 126: under point symmetry the first column has no image;
        // on maps with mesas it would keep the north half's ground beside
        // the image of a mesa (a one-sample wall at the west edge of the
        // south half), so it takes its neighbour's value there. Maps
        // without mesas stay as they were.
        if matches!(m, Mirror::Point(..)) && !plan.mesas.is_empty() {
            for j in 0..sy {
                if -half.1 + j as f64 * CELL_M > m.axis() {
                    heights[j * sx] = heights[j * sx + 1];
                    terrain[j * sx] = terrain[j * sx + 1];
                }
            }
        }
    }
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
    let mut works_missing = Vec::new();
    for w in &sketch.works {
        works_missing.append(&mut works(&mut objects, w)?);
    }
    // Prompt 124: trenches, wire and lanes (kept free of objects).
    let field_works = field_works(&sketch, mirror)?;
    for w in &field_works {
        let half_width = match w.kind {
            PreparedWorkKind::Trench => 4.5,
            PreparedWorkKind::CommTrench => 3.0,
            PreparedWorkKind::Wire => 2.0,
            PreparedWorkKind::MgNest | PreparedWorkKind::Dugout => 4.5,
        };
        block_line(&mut objects, &w.points, half_width);
    }
    for lane in &sketch.lane {
        let points: Vec<(f64, f64)> = lane.points_m.iter().map(|p| (p[0], p[1])).collect();
        block_line(&mut objects, &points, lane.width_m / 2.0);
    }
    for m in sketch.marker.iter().chain(&sketch.supply) {
        if let Some(r) = m.flat_m {
            objects
                .reserved
                .push(((m.position_m[0], m.position_m[1]), r - 5.0));
        }
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
    for c in &sketch.camp {
        camp(&mut objects, c, &plan.ways);
    }
    let boulders = mesa_boulders(&mut objects, &plan.mesas, &plan.fray, seed);
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
    let mut object_rows = std::mem::take(&mut objects.rows);
    drop(objects);
    let scenery = scenery_rows(&sketch, seed, &bridges, &|x, y| {
        let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
        let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
        matches!(terrain[j * sx + i], SHALLOW_WATER | DEEP_WATER)
    });
    let mut summary = if scenery.is_empty() {
        summary
    } else {
        format!("{summary}, {} piers and stones", scenery.len())
    };
    object_rows.extend(scenery);
    // Prompt 126: ramp markers (with their images on symmetric maps).
    let mut mesa_markers: Vec<(String, (f64, f64))> = Vec::new();
    if !plan.mesas.is_empty() {
        let mut ramps = Vec::new();
        for m in &plan.mesas {
            for e in &m.edges {
                for r in &e.ramps {
                    let (t, f) = (r.top(0.0), r.foot());
                    ramps.push(format!(
                        "({:.0}, {:.0})->({:.0}, {:.0})",
                        t.0, t.1, f.0, f.1
                    ));
                }
            }
            mesa_markers.extend(m.markers());
        }
        if let Some(mirror) = mirror {
            let images: Vec<(String, (f64, f64))> = mesa_markers
                .iter()
                .map(|(n, p)| (format!("{n}_image"), mirror.map(*p)))
                .collect();
            mesa_markers.extend(images);
        }
        summary = format!(
            "{summary}; {} mesas, {boulders} boulders, ramps top->foot: {}",
            plan.mesas.len(),
            ramps.join(" ")
        );
    }
    if let Some(m) = mirror {
        object_rows = mirror_rows(object_rows, &object_rules, m)?;
        summary = format!(
            "{summary} (objects in the north half), {} object rows after mirroring",
            object_rows.len()
        );
    }
    if !works_missing.is_empty() {
        summary = format!(
            "{summary}; field works that did not fit: {}",
            works_missing.join(", ")
        );
    }
    rows.extend(object_rows);

    // Prompt 124: craters (on the final ground and terrain) and the
    // prepared ground of map.toml.
    let craters = crater_fields(
        &sketch,
        seed,
        mirror,
        &CraterKeepOut {
            works: &field_works,
            bridges: &bridges,
            flats: &flat_spots,
            terrain: &|x, y| {
                let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
                let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
                terrain[j * sx + i]
            },
        },
    );
    let prepared = prepared_ground(&field_works, &craters, &puddles);
    if prepared.has_ground() || !prepared.puddles.is_empty() {
        summary = format!(
            "{summary}; prepared: {} field works, {} craters ({} with water), {} puddles",
            prepared.works.len(),
            prepared.craters.len(),
            prepared.craters.iter().filter(|c| c.water).count(),
            prepared.puddles.len()
        );
    }

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
    for (name, p) in &mesa_markers {
        let _ = write!(
            toml,
            "\n[[marker]]\nname = \"{name}\"\nposition_m = [{}, {}]\n",
            p.0.round() as i32,
            p.1.round() as i32
        );
    }
    toml.push_str(&prepared.to_toml());
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;
    Ok(format!(
        "wrote {}: {sx} x {sy} samples, heights {low:.2} .. {high:.2} m, {summary}",
        options.out.display(),
    ))
}

/// Copies the north half of a sample raster (rows of `sx` samples, 2 m
/// apart from `-half_y`) mirrored about `axis` onto the south half
/// (prompt 123).
fn mirror_south<T: Copy>(raster: &mut [T], sx: usize, half_y: f64, axis: f64) {
    let sy = raster.len() / sx;
    for j in 0..sy {
        let y = -half_y + j as f64 * CELL_M;
        if y <= axis {
            continue;
        }
        let source = ((2.0 * axis - y + half_y) / CELL_M).round() as usize;
        raster.copy_within(source * sx..(source + 1) * sx, j * sx);
    }
}

/// Bridges of a mirrored map (prompt 123): a chain that lies in the north
/// half gets its mirrored copy; a chain across the axis must already be
/// symmetric (its water is) and stays as it is.
fn mirror_bridges(bridges: Vec<BridgeRow>, m: Mirror) -> Result<Vec<BridgeRow>, String> {
    let axis = m.axis();
    let mut out = Vec::new();
    let mut copies = Vec::new();
    for b in &bridges {
        let chain: Vec<&BridgeRow> = bridges
            .iter()
            .filter(|o| o.crossing == b.crossing)
            .collect();
        // Half a piece along the way (heading 0) or half the deck across
        // (heading 90).
        let reach = if b.heading == 0 { LONG_PIECE_M } else { 4.0 };
        let north = chain.iter().all(|o| o.y + reach < axis);
        let (mx, my) = m.map((b.x, b.y));
        if north {
            copies.push(BridgeRow {
                crossing: b.crossing,
                kind: b.kind,
                variant: b.variant,
                x: mx,
                y: my,
                heading: b.heading,
            });
        } else if !bridges
            .iter()
            .any(|o| (o.x - mx).abs() < 0.002 && (o.y - my).abs() < 0.002)
        {
            return Err(format!(
                "the bridge at ({:.1}, {:.1}) crosses the mirror axis but is not symmetric",
                b.x, b.y
            ));
        }
        out.push(BridgeRow { ..*b });
    }
    out.extend(copies);
    Ok(out)
}

/// Object rows of a mirrored map (prompt 123): every object that lies
/// wholly in the north half and its copy mirrored about `axis` (heading
/// mirrored too); objects reaching over the axis are left out, so no copy
/// overlaps its original.
fn mirror_rows(rows: Vec<String>, rules: &ObjectRules, m: Mirror) -> Result<Vec<String>, String> {
    let axis_mm = (m.axis() * 1000.0).round() as i64;
    // Point symmetry (prompt 124): an object standing on the center point
    // is its own image and stays once.
    let center_mm = match m {
        Mirror::North(_) => None,
        Mirror::Point(cx, _) => Some(((cx * 1000.0).round() as i64, axis_mm)),
    };
    let mut north = Vec::new();
    let mut south = Vec::new();
    for row in rows {
        let f: Vec<&str> = row.split(',').collect();
        let [kind, variant, x, y, heading] = f[..] else {
            return Err(format!("bad object row {row:?}"));
        };
        let num = |v: &str| v.parse::<i64>().map_err(|e| format!("{row:?}: {e}"));
        let (x_mm, y_mm, heading) = (num(x)?, num(y)?, num(heading)?);
        let object = editor::PlacedObject {
            kind: kind.into(),
            variant: variant.into(),
            x_mm: x_mm as i32,
            y_mm: y_mm as i32,
            heading_deg: heading as u16,
        };
        if center_mm == Some((x_mm, y_mm)) {
            north.push(row);
            continue;
        }
        let (_, ey) = editor::compound::half_extent(rules, &object);
        if y_mm + ey.max(300) > axis_mm {
            continue;
        }
        match center_mm {
            None => south.push(format!(
                "{kind},{variant},{x_mm},{},{}",
                2 * axis_mm - y_mm,
                (540 - heading) % 360
            )),
            Some((cx_mm, _)) => south.push(format!(
                "{kind},{variant},{},{},{}",
                2 * cx_mm - x_mm,
                2 * axis_mm - y_mm,
                (heading + 180) % 360
            )),
        }
        north.push(row);
    }
    north.append(&mut south);
    Ok(north)
}

/// Places a group of field works (prompt 123); returns the pieces that did
/// not fit (roads, water or other objects in the way).
fn works(objects: &mut Objects, w: &Works) -> Result<Vec<String>, String> {
    let len = (w.facing_m[0] * w.facing_m[0] + w.facing_m[1] * w.facing_m[1]).sqrt();
    if len == 0.0 {
        return Err("works: facing_m must not be zero".into());
    }
    // f: towards the enemy, a: across (to the right of the facing).
    let f = (w.facing_m[0] / len, w.facing_m[1] / len);
    let a = (-f.1, f.0);
    let front = heading_of(f.0, f.1);
    let side = (front + 90) % 360;
    // (kind, variant, across, along, heading)
    let pieces: Vec<(&str, &str, f64, f64, u16)> = match w.kind.as_str() {
        "battery" => vec![
            ("fort_sandbag_position", "Position", -22.5, 0.0, front),
            ("fort_sandbag_position", "Position", -7.5, 0.0, front),
            ("fort_sandbag_position", "Position", 7.5, 0.0, front),
            ("fort_sandbag_position", "Position", 22.5, 0.0, front),
            ("fort_trench_section", "Straight", -35.0, -6.0, front),
            ("fort_trench_section", "Straight", 35.0, -6.0, front),
        ],
        "bridgehead" => vec![
            ("fort_bunker_mg", "Pillbox", 0.0, 0.0, front),
            ("fort_trench_section", "Straight", 7.0, -2.0, side),
            ("fort_trench_section", "Straight", 12.5, -2.0, side),
            ("fort_trench_section", "End", 18.0, -2.0, side),
            ("fort_sandbag_position", "Position", -8.0, -1.0, front),
        ],
        other => return Err(format!("unknown works kind {other:?}")),
    };
    let mut missing = Vec::new();
    for (kind, variant, across, along, heading) in pieces {
        let p = (
            w.position_m[0] + a.0 * across + f.0 * along,
            w.position_m[1] + a.1 * across + f.1 * along,
        );
        if !objects.place(kind, variant, p, heading, 0.3) {
            missing.push(format!("{kind} at ({:.0}, {:.0})", p.0, p.1));
        }
    }
    Ok(missing)
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

// --- Prompt 124: point symmetry, trenches, wire, craters, marshes, camps ---

/// How a symmetric map copies its north half (prompts 123, 124).
#[derive(Clone, Copy)]
enum Mirror {
    /// Mirrored about the line y = axis (north against south).
    North(f64),
    /// Turned by 180° about the point (x, y): point symmetry.
    Point(f64, f64),
}

impl Mirror {
    /// The line between the two halves (y).
    const fn axis(self) -> f64 {
        match self {
            Self::North(a) | Self::Point(_, a) => a,
        }
    }

    /// The image of `p`.
    fn map(self, (x, y): (f64, f64)) -> (f64, f64) {
        match self {
            Self::North(a) => (x, 2.0 * a - y),
            Self::Point(cx, cy) => (2.0 * cx - x, 2.0 * cy - y),
        }
    }
}

/// Copies the north half of a sample raster onto the south half (see
/// [`Mirror`]). Point symmetry: every sample south of the center line (and
/// east of the center on it) takes the value of its image; the first row
/// and column have no image and stay.
fn mirror_raster<T: Copy>(raster: &mut [T], sx: usize, half: (f64, f64), m: Mirror) {
    match m {
        Mirror::North(axis) => mirror_south(raster, sx, half.1, axis),
        Mirror::Point(cx, cy) => {
            let sy = raster.len() / sx;
            for j in 0..sy {
                let y = -half.1 + j as f64 * CELL_M;
                if y < cy {
                    continue;
                }
                for i in 0..sx {
                    let x = -half.0 + i as f64 * CELL_M;
                    if y == cy && x <= cx {
                        continue;
                    }
                    let si = ((2.0 * cx - x + half.0) / CELL_M).round();
                    let sj = ((2.0 * cy - y + half.1) / CELL_M).round();
                    if si < 0.0 || sj < 0.0 || si >= sx as f64 || sj >= sy as f64 {
                        continue;
                    }
                    raster[j * sx + i] = raster[sj as usize * sx + si as usize];
                }
            }
        }
    }
}

/// Point symmetry (prompt 124): the ground of the two halves meets at the
/// center line from different places (a sample's image lies on the other
/// side of the map), so the heights within `AXIS_BAND_M` of the line are
/// blended with their images, half and half on the line: symmetric and
/// without a step.
const AXIS_BAND_M: f64 = 24.0;

fn blend_axis(heights: &mut [f64], sx: usize, half: (f64, f64), (cx, cy): (f64, f64)) {
    let sy = heights.len() / sx;
    for j in 0..sy {
        let y = -half.1 + j as f64 * CELL_M;
        if y < cy - AXIS_BAND_M || y > cy {
            continue;
        }
        let s = smoothstep(cy + AXIS_BAND_M, cy - AXIS_BAND_M, y);
        for i in 0..sx {
            let x = -half.0 + i as f64 * CELL_M;
            if y == cy && x > cx {
                continue;
            }
            let si = ((2.0 * cx - x + half.0) / CELL_M).round();
            let sj = ((2.0 * cy - y + half.1) / CELL_M).round();
            if si < 0.0 || sj < 0.0 || si >= sx as f64 || sj >= sy as f64 {
                continue;
            }
            let image = heights[sj as usize * sx + si as usize];
            let k = j * sx + i;
            heights[k] = s * heights[k] + (1.0 - s) * image;
        }
    }
}

use sim::prepared::{Prepared, PreparedCrater, PreparedWork, PreparedWorkKind, Puddle};

/// A field work of the generator (meters, before rounding).
struct GenWork {
    kind: PreparedWorkKind,
    points: Vec<(f64, f64)>,
}

fn polyline_len(points: &[(f64, f64)]) -> f64 {
    points.windows(2).map(|s| dist(s[0], s[1])).sum()
}

/// The point `u` meters along `points` (clamped to the ends).
fn point_along(points: &[(f64, f64)], u: f64) -> (f64, f64) {
    let mut left = u.max(0.0);
    for s in points.windows(2) {
        let len = dist(s[0], s[1]);
        if left <= len && len > 0.0 {
            let t = left / len;
            return (
                s[0].0 + (s[1].0 - s[0].0) * t,
                s[0].1 + (s[1].1 - s[0].1) * t,
            );
        }
        left -= len;
    }
    points[points.len() - 1]
}

/// The field works of the sketch's trenches and wire (prompt 124), north
/// half and mirrored copies; points on whole meters.
fn field_works(sketch: &Sketch, mirror: Option<Mirror>) -> Result<Vec<GenWork>, String> {
    let mut out = Vec::new();
    for t in &sketch.trench {
        let points: Vec<(f64, f64)> = t.points_m.iter().map(|p| (p[0], p[1])).collect();
        if points.len() < 2 {
            return Err("trench: two points or more".into());
        }
        match t.kind.as_str() {
            "comm" => out.push(GenWork {
                kind: PreparedWorkKind::CommTrench,
                points,
            }),
            "fire" => {
                let [fx, fy] = t.facing_m.ok_or("fire trench: facing_m missing")?;
                let n = (fx * fx + fy * fy).sqrt();
                if n == 0.0 {
                    return Err("fire trench: facing_m must not be zero".into());
                }
                let f = (fx / n, fy / n);
                let total = polyline_len(&points);
                let teeth = (total / t.tooth_m.unwrap_or(40.0)).floor().max(1.0);
                let l = total / teeth;
                let d = t.depth_m.unwrap_or(10.0);
                let at = |u: f64, out: f64| {
                    let p = point_along(&points, u);
                    (p.0 + f.0 * out, p.1 + f.1 * out)
                };
                let mut trace = Vec::new();
                for k in 0..teeth as u32 {
                    let u0 = f64::from(k) * l;
                    trace.push(at(u0, 0.0));
                    trace.push(at(u0 + 0.2 * l, d));
                    trace.push(at(u0 + 0.5 * l, d));
                    trace.push(at(u0 + 0.7 * l, 0.0));
                    let every = |e: u32| e > 0 && k % e == e / 2;
                    if every(t.nest_every) {
                        out.push(GenWork {
                            kind: PreparedWorkKind::MgNest,
                            points: vec![at(u0 + 0.35 * l, d + 3.0)],
                        });
                    }
                    if every(t.dugout_every) {
                        out.push(GenWork {
                            kind: PreparedWorkKind::Dugout,
                            points: vec![at(u0 + 0.85 * l, -4.0)],
                        });
                    }
                }
                trace.push(at(total, 0.0));
                out.push(GenWork {
                    kind: PreparedWorkKind::Trench,
                    points: trace,
                });
            }
            other => return Err(format!("unknown trench kind {other:?} (fire, comm)")),
        }
    }
    for w in &sketch.wire {
        out.extend(
            wire_runs(w, &sketch.lane)
                .into_iter()
                .map(|points| GenWork {
                    kind: PreparedWorkKind::Wire,
                    points,
                }),
        );
    }
    // Whole meters, no repeated points.
    for w in &mut out {
        let mut points: Vec<(f64, f64)> = Vec::new();
        for &(x, y) in &w.points {
            let p = (x.round(), y.round());
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
        w.points = points;
    }
    out.retain(|w| w.kind.is_point() || w.points.len() >= 2);
    if let Some(m) = mirror {
        let mut copies = Vec::new();
        for w in &out {
            if w.points.iter().any(|p| p.1 >= m.axis()) {
                return Err(format!(
                    "{} at ({:.0}, {:.0}) reaches over the mirror axis",
                    w.kind.name(),
                    w.points[0].0,
                    w.points[0].1
                ));
            }
            copies.push(GenWork {
                kind: w.kind,
                points: w.points.iter().map(|&p| m.map(p)).collect(),
            });
        }
        out.extend(copies);
    }
    Ok(out)
}

/// The runs of a wire line between its gaps and the lanes crossing it.
fn wire_runs(w: &WireDef, lanes: &[Strip]) -> Vec<Vec<(f64, f64)>> {
    let gap = w.gap_m.unwrap_or(16.0) / 2.0;
    let cut = |p: (f64, f64)| {
        w.gaps_m.iter().any(|g| dist(p, (g[0], g[1])) < gap)
            || lanes
                .iter()
                .any(|l| polyline_distance(p, &l.points_m) < l.width_m / 2.0 + 3.0)
    };
    // Points every meter, marked where they are a vertex of the line.
    let mut samples: Vec<((f64, f64), bool)> = Vec::new();
    for (k, s) in w.points_m.windows(2).enumerate() {
        let (a, b) = ((s[0][0], s[0][1]), (s[1][0], s[1][1]));
        let n = dist(a, b).ceil().max(1.0) as u32;
        for i in u32::from(k > 0)..=n {
            let t = f64::from(i) / f64::from(n);
            samples.push((
                (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
                i == 0 || i == n,
            ));
        }
    }
    let mut runs = Vec::new();
    let mut run: Vec<(f64, f64)> = Vec::new();
    let mut last: Option<(f64, f64)> = None;
    for (p, vertex) in samples {
        if cut(p) {
            if let Some(q) = last.take()
                && run.last() != Some(&q)
            {
                run.push(q);
            }
            if run.len() >= 2 {
                runs.push(std::mem::take(&mut run));
            }
            run.clear();
            continue;
        }
        if run.is_empty() || vertex {
            run.push(p);
            last = None;
        } else {
            last = Some(p);
        }
    }
    if let Some(q) = last
        && run.last() != Some(&q)
    {
        run.push(q);
    }
    if run.len() >= 2 {
        runs.push(run);
    }
    runs
}

/// Keeps a strip of `half_width` m on both sides of `points` free of
/// objects (boxes along pieces of at most 4 m).
fn block_line(objects: &mut Objects, points: &[(f64, f64)], half_width: f64) {
    if points.len() == 1 {
        let (x, y) = points[0];
        objects.block(crate::gen_land::Rect {
            x0: x - half_width,
            y0: y - half_width,
            x1: x + half_width,
            y1: y + half_width,
        });
        return;
    }
    for s in points.windows(2) {
        let len = dist(s[0], s[1]);
        let n = (len / 4.0).ceil().max(1.0) as u32;
        for k in 0..n {
            let a = point_along(s, f64::from(k) * len / f64::from(n));
            let b = point_along(s, f64::from(k + 1) * len / f64::from(n));
            objects.block(crate::gen_land::Rect {
                x0: a.0.min(b.0) - half_width,
                y0: a.1.min(b.1) - half_width,
                x1: a.0.max(b.0) + half_width,
                y1: a.1.max(b.1) + half_width,
            });
        }
    }
}

/// Puddles of the marshes (own random stream): north half and mirrored
/// copies; none within 6 m of a way.
fn marsh_puddles(
    sketch: &Sketch,
    seed: u64,
    ways: &[Way],
    mirror: Option<Mirror>,
) -> Vec<(f64, f64, f64)> {
    let mut random = Random(Rng::new(seed ^ 0x6d61_7273));
    let mut out: Vec<(f64, f64, f64)> = Vec::new();
    for m in &sketch.marsh {
        let area = bounds(&m.polygon_m);
        let mut placed = 0;
        let mut tries = 0;
        while placed < m.count && tries < m.count * 50 {
            tries += 1;
            let p = (
                random.range(area.x0, area.x1),
                random.range(area.y0, area.y1),
            );
            let r = random.range(m.radius_m[0], m.radius_m[1]);
            if !inside(p, &m.polygon_m)
                || mirror.is_some_and(|m| p.1 + r + 2.0 >= m.axis())
                || ways.iter().any(|w| w.distance(p.0, p.1) - w.half < r + 6.0)
                || out.iter().any(|&(x, y, q)| dist(p, (x, y)) < r + q + 2.0)
            {
                continue;
            }
            out.push((p.0.round(), p.1.round(), r.round().max(1.0)));
            placed += 1;
        }
    }
    if let Some(m) = mirror {
        let copies: Vec<_> = out
            .iter()
            .map(|&(x, y, r)| {
                let (mx, my) = m.map((x, y));
                (mx, my, r)
            })
            .collect();
        out.extend(copies);
    }
    out
}

/// What crater fields keep away from.
struct CraterKeepOut<'a> {
    works: &'a [GenWork],
    bridges: &'a [BridgeRow],
    flats: &'a [((f64, f64), f64)],
    terrain: &'a dyn Fn(f64, f64) -> u8,
}

/// Craters of the crater fields (own random stream): `(x, y, caliber mm,
/// water)`, north half and mirrored copies.
fn crater_fields(
    sketch: &Sketch,
    seed: u64,
    mirror: Option<Mirror>,
    keep: &CraterKeepOut,
) -> Vec<(f64, f64, u32, bool)> {
    let mut random = Random(Rng::new(seed ^ 0x6372_6174));
    let mut out: Vec<(f64, f64, u32, bool)> = Vec::new();
    let radius = |caliber: u32| (f64::from(caliber) * 0.012).max(2.0);
    for field in &sketch.crater_field {
        let poly = &field.polygon_m;
        let n = poly.len();
        let area: f64 = (0..n)
            .map(|i| {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
            / 2.0;
        let count = (area / 10_000.0 * field.density_ha).round() as u32;
        let total: f64 = field.calibers.iter().map(|c| c[1]).sum();
        let bounds = bounds(poly);
        let edge = field.edge_m.unwrap_or(0.0);
        for _ in 0..count {
            let p = (
                random.range(bounds.x0, bounds.x1),
                random.range(bounds.y0, bounds.y1),
            );
            let mut pick = random.unit() * total;
            let caliber = field
                .calibers
                .iter()
                .find(|c| {
                    pick -= c[1];
                    pick < 0.0
                })
                .map_or(field.calibers[0][0], |c| c[0]) as u32;
            let water = random.unit() < field.water;
            let fade = random.unit();
            let r = radius(caliber);
            if !inside(p, poly)
                || (edge > 0.0 && fade > smoothstep(0.0, edge, edge_distance(p, poly)))
                || mirror.is_some_and(|m| p.1 >= m.axis())
            {
                continue;
            }
            let lane = sketch
                .lane
                .iter()
                .any(|l| polyline_distance(p, &l.points_m) < l.width_m / 2.0 + r);
            let work = keep.works.iter().any(|w| {
                let reach = match w.kind {
                    PreparedWorkKind::Wire => 2.0,
                    _ => 5.0,
                } + r;
                if w.points.len() == 1 {
                    return dist(p, w.points[0]) < reach + 2.0;
                }
                w.points
                    .windows(2)
                    .any(|s| segment_distance(p, [s[0].0, s[0].1], [s[1].0, s[1].1]) < reach)
            });
            let ground = [(0.0, 0.0), (r, 0.0), (-r, 0.0), (0.0, r), (0.0, -r)]
                .iter()
                .any(|(dx, dy)| {
                    matches!(
                        (keep.terrain)(p.0 + dx, p.1 + dy),
                        ROAD | DIRT_TRACK | SHALLOW_WATER | DEEP_WATER
                    )
                });
            if lane
                || work
                || ground
                || keep.bridges.iter().any(|b| dist(p, (b.x, b.y)) < 25.0)
                || keep
                    .flats
                    .iter()
                    .any(|&(c, f)| dist(p, c) < f.min(25.0) + r)
                || out
                    .iter()
                    .any(|&(x, y, c, _)| dist(p, (x, y)) < 0.75 * (r + radius(c)))
            {
                continue;
            }
            out.push((p.0.round(), p.1.round(), caliber, water));
        }
    }
    if let Some(m) = mirror {
        let copies: Vec<_> = out
            .iter()
            .map(|&(x, y, c, w)| {
                let (mx, my) = m.map((x, y));
                (mx, my, c, w)
            })
            .collect();
        out.extend(copies);
    }
    out
}

/// The prepared ground of `map.toml` (positions are whole meters).
fn prepared_ground(
    works: &[GenWork],
    craters: &[(f64, f64, u32, bool)],
    puddles: &[(f64, f64, f64)],
) -> Prepared {
    let at = |x: f64, y: f64| sim::Vec2::from_meters(x.round() as i32, y.round() as i32);
    Prepared {
        works: works
            .iter()
            .map(|w| PreparedWork {
                kind: w.kind,
                points: w.points.iter().map(|&(x, y)| at(x, y)).collect(),
            })
            .collect(),
        craters: craters
            .iter()
            .map(|&(x, y, caliber_mm, water)| PreparedCrater {
                position: at(x, y),
                caliber_mm,
                water,
            })
            .collect(),
        puddles: puddles
            .iter()
            .map(|&(x, y, r)| Puddle {
                position: at(x, y),
                radius_mm: r as u32 * 1000,
            })
            .collect(),
    }
}

/// The objects of a camp (see [`Camp`]).
fn camp(objects: &mut Objects, c: &Camp, ways: &[Way]) {
    let area = bounds(&c.polygon_m);
    for item in &c.items {
        let mut placed = 0;
        let mut tries = 0;
        while placed < item.count && tries < item.count * 60 {
            tries += 1;
            let p = (
                objects.random.range(area.x0, area.x1),
                objects.random.range(area.y0, area.y1),
            );
            let turn = if objects.random.unit() < 0.5 { 0 } else { 180 };
            let heading = way_heading(ways, p, 40.0).map_or_else(
                || (objects.random.unit() * 4.0) as u16 % 4 * 90,
                |h| (h + turn) % 360,
            );
            if inside(p, &c.polygon_m) && objects.place(&item.kind, &item.variant, p, heading, 2.5)
            {
                placed += 1;
            }
        }
    }
}

// --- Prompt 126: table mountains (mesas) -----------------------------------

/// A table mountain (prompt 126): a plateau `height_m` above the basin
/// whose foot is `polygon_m`. From the foot inwards a narrow scree slope
/// (35-60 %, boulders on it) and a cliff (180 %, impassable) rise to the
/// plateau; the outline (rounded by cutting its corners) is frayed by
/// noise (`rough_m`, default 8 m, tiers 0.6 of it; an own noise stream). `tier`: steps (higher than the plateau) and hollows
/// (lower) with their own edge. `ramps_m`: points on the outline (of the
/// mesa or of a tier) where a ramp for vehicles leads from the lower to the
/// higher level: `ramp_grade` (default 20 %), `ramp_width_m` wide (default
/// 18 m), half cut into the higher level, half heaped onto the lower one,
/// a dirt track on top. A mesa without ramps is scenery (rock needles).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct MesaDef {
    /// Name of the ramp markers (`ramp_<name>_<k>`, tiers
    /// `ramp_<name>_t<i>_<k>`, on the higher level beyond each ramp's top);
    /// without a name no markers.
    #[serde(default)]
    name: String,
    polygon_m: Vec<P>,
    height_m: f64,
    #[serde(default)]
    ramps_m: Vec<P>,
    #[serde(default)]
    tier: Vec<TierDef>,
    #[serde(default)]
    rough_m: Option<f64>,
    #[serde(default)]
    ramp_grade: Option<f64>,
    #[serde(default)]
    ramp_width_m: Option<f64>,
    /// Boulders on the scree per 10 m of foot (default 1).
    #[serde(default)]
    boulders: Option<f64>,
}

/// A step (`height_m` above the plateau's height) or a hollow (below it)
/// on a mesa (prompt 126); `height_m` counts from the basin. A tier lies on
/// the mesa's plateau, or on an earlier tier that holds its first point.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct TierDef {
    polygon_m: Vec<P>,
    height_m: f64,
    #[serde(default)]
    ramps_m: Vec<P>,
}

/// Noise that frays the outlines of mesas: three octaves (60, 20, 7 m).
type Fray = [crate::gen_land::Noise; 3];

/// Rounds a polygon by cutting its corners (Chaikin, twice): mesas and
/// their tiers are drawn with a few points, their outlines are organic.
fn rounded(polygon: &[P]) -> Vec<P> {
    let mut out = polygon.to_vec();
    for _ in 0..2 {
        let n = out.len();
        out = (0..n)
            .flat_map(|i| {
                let (a, b) = (out[i], out[(i + 1) % n]);
                [
                    [0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]],
                    [0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]],
                ]
            })
            .collect();
    }
    out
}

/// Scree at the foot of a mesa edge: a fifth of the height, at most 5 m,
/// with a grade from 35 to 60 % (mean 47.5 %).
const MESA_TALUS_SHARE: f64 = 0.2;
const MESA_TALUS_MAX_M: f64 = 5.0;
const MESA_TALUS_GRADE: f64 = 0.475;
/// Grade of the cliff above the scree (impassable: over 60 %).
const MESA_CLIFF_GRADE: f64 = 1.8;
/// Side slopes of a ramp: heaped (fill, passable scree) and cut (rock).
const RAMP_FILL_GRADE: f64 = 0.5;
const RAMP_CUT_GRADE: f64 = 1.5;
/// Share of a ramp cut into the higher level.
const RAMP_CUT: f64 = 0.5;
/// Width of a ramp's flat bed without a track on it on each side (m).
const RAMP_SHOULDER_M: f64 = 4.0;

/// A ramp of a mesa edge: from `at` (on the outline) the ground falls along
/// `dir` (towards the lower level) at `grade`; it reaches the higher level
/// at `u_top` (negative: inside the higher level) and the lower one `len`
/// further.
struct MesaRamp {
    at: (f64, f64),
    dir: (f64, f64),
    u_top: f64,
    len: f64,
    half_w: f64,
    grade: f64,
}

impl MesaRamp {
    /// Distance along and across the ramp.
    fn frame(&self, p: (f64, f64)) -> (f64, f64) {
        let (dx, dy) = (p.0 - self.at.0, p.1 - self.at.1);
        (
            dx * self.dir.0 + dy * self.dir.1,
            (dy * self.dir.0 - dx * self.dir.1).abs(),
        )
    }

    /// The ground `value` of the edge between `lo` and `hi` with the ramp:
    /// on the bed the ramp's height, beside it cut walls and heaped sides.
    fn apply(&self, p: (f64, f64), value: f64, lo: f64, hi: f64) -> f64 {
        let (u, v) = self.frame(p);
        let spread = (hi - lo) / RAMP_FILL_GRADE;
        // Beyond the foot the bed lies on the lower level: nothing to cut
        // or heap (a far rim, e.g. of a hollow, stays as it is).
        if u < self.u_top || u > self.u_top + self.len || v > self.half_w + spread {
            return value;
        }
        let bed = (hi - self.grade * (u - self.u_top)).clamp(lo, hi);
        let s = (v - self.half_w).max(0.0);
        value.clamp(bed - s * RAMP_FILL_GRADE, bed + s * RAMP_CUT_GRADE)
    }

    /// True on the track along the ramp's bed.
    fn on_track(&self, p: (f64, f64)) -> bool {
        let (u, v) = self.frame(p);
        u >= self.u_top - RAMP_SHOULDER_M
            && u <= self.u_top + self.len
            && v < self.half_w - RAMP_SHOULDER_M
    }

    /// A point `beyond` m past the ramp's top on the higher level.
    fn top(&self, beyond: f64) -> (f64, f64) {
        let u = self.u_top - beyond;
        (self.at.0 + self.dir.0 * u, self.at.1 + self.dir.1 * u)
    }

    /// The ramp's foot on the lower level.
    fn foot(&self) -> (f64, f64) {
        let u = self.u_top + self.len;
        (self.at.0 + self.dir.0 * u, self.at.1 + self.dir.1 * u)
    }
}

/// One edge of a mesa: its outline, the levels below (`lo`) and above
/// (`hi`) it, which side is high (inside: the mesa and its steps; outside:
/// a hollow) and its ramps.
struct MesaEdge {
    polygon: Vec<P>,
    lo: f64,
    hi: f64,
    high_inside: bool,
    rough: f64,
    ramps: Vec<MesaRamp>,
    /// Bounds of the outline grown by the roughness: outside them the edge
    /// is its outer level.
    near: crate::gen_land::Rect,
    /// Bounds of everything the edge changes (outline and ramps).
    reach: crate::gen_land::Rect,
}

fn in_rect(r: &crate::gen_land::Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x0 && p.0 <= r.x1 && p.1 >= r.y0 && p.1 <= r.y1
}

fn grow(r: crate::gen_land::Rect, by: f64) -> crate::gen_land::Rect {
    crate::gen_land::Rect {
        x0: r.x0 - by,
        y0: r.y0 - by,
        x1: r.x1 + by,
        y1: r.y1 + by,
    }
}

impl MesaEdge {
    fn new(
        polygon: &[P],
        (lo, hi): (f64, f64),
        high_inside: bool,
        rough: f64,
        ramps: &[P],
        (grade, width): (f64, f64),
    ) -> Result<Self, String> {
        if polygon.len() < 3 || hi <= lo || grade <= 0.0 || grade >= 0.6 {
            return Err(format!(
                "mesa edge at {:?}: needs a polygon, a height step and a ramp grade under 60 %",
                polygon.first()
            ));
        }
        let polygon = &rounded(polygon)[..];
        let len = (hi - lo) / grade;
        let ramps: Vec<MesaRamp> = ramps
            .iter()
            .map(|r| {
                // The nearest point of the outline and its normal towards the
                // lower level.
                let n = polygon.len();
                let (mut best, mut at, mut dir) = (f64::INFINITY, (0.0, 0.0), (0.0, 0.0));
                for i in 0..n {
                    let (a, b) = (polygon[i], polygon[(i + 1) % n]);
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let l2 = dx * dx + dy * dy;
                    if l2 <= 0.0 {
                        continue;
                    }
                    let t = (((r[0] - a[0]) * dx + (r[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
                    let q = (a[0] + t * dx, a[1] + t * dy);
                    let d = dist(q, (r[0], r[1]));
                    if d < best {
                        let l = l2.sqrt();
                        best = d;
                        at = q;
                        dir = (dy / l, -dx / l);
                    }
                }
                let out = (at.0 + dir.0, at.1 + dir.1);
                if inside(out, polygon) == high_inside {
                    dir = (-dir.0, -dir.1);
                }
                MesaRamp {
                    at,
                    dir,
                    u_top: -RAMP_CUT * len,
                    len,
                    half_w: width / 2.0,
                    grade,
                }
            })
            .collect();
        let near = grow(bounds(polygon), rough + 1.0);
        let spread = (hi - lo) / RAMP_FILL_GRADE;
        let mut reach = near;
        for r in &ramps {
            let w = r.half_w + spread;
            for p in [r.top(0.0), r.foot()] {
                let around = crate::gen_land::Rect {
                    x0: p.0 - w - spread,
                    y0: p.1 - w - spread,
                    x1: p.0 + w + spread,
                    y1: p.1 + w + spread,
                };
                reach.x0 = reach.x0.min(around.x0);
                reach.y0 = reach.y0.min(around.y0);
                reach.x1 = reach.x1.max(around.x1);
                reach.y1 = reach.y1.max(around.y1);
            }
        }
        Ok(Self {
            polygon: polygon.to_vec(),
            lo,
            hi,
            high_inside,
            rough,
            ramps,
            near,
            reach,
        })
    }

    /// Height and width of the scree at the foot.
    fn talus(&self) -> (f64, f64) {
        let h = ((self.hi - self.lo) * MESA_TALUS_SHARE).min(MESA_TALUS_MAX_M);
        (h, h / MESA_TALUS_GRADE)
    }

    /// Width of the whole edge (scree and cliff).
    fn width(&self) -> f64 {
        let (th, tw) = self.talus();
        tw + (self.hi - self.lo - th) / MESA_CLIFF_GRADE
    }

    /// Rise of the edge `x` m from its foot towards the high side.
    fn profile(&self, x: f64) -> f64 {
        let d = self.hi - self.lo;
        let (th, tw) = self.talus();
        if x <= 0.0 {
            0.0
        } else if x < tw {
            // Grade from 35 % at the foot to 60 % at the cliff.
            let t = x / tw;
            th * t * (0.737 + 0.263 * t)
        } else {
            (th + (x - tw) * MESA_CLIFF_GRADE).min(d)
        }
    }

    /// Signed distance into the outline (negative outside), frayed.
    fn edge(&self, p: (f64, f64), fray: &Fray) -> f64 {
        let d = edge_distance(p, &self.polygon);
        let signed = if inside(p, &self.polygon) { d } else { -d };
        signed
            + self.rough
                * ((fray[0].at(p.0, p.1) - 0.5) * 1.2
                    + (fray[1].at(p.0, p.1) - 0.5) * 0.6
                    + (fray[2].at(p.0, p.1) - 0.5) * 0.3)
    }

    /// The level away from the edge on its outer side.
    const fn outer(&self) -> f64 {
        if self.high_inside { self.lo } else { self.hi }
    }

    fn value(&self, p: (f64, f64), fray: &Fray) -> f64 {
        if !in_rect(&self.reach, p) {
            return self.outer();
        }
        let mut v = if in_rect(&self.near, p) {
            let e = self.edge(p, fray);
            if self.high_inside {
                self.lo + self.profile(e)
            } else {
                self.lo + self.profile(self.width() - e)
            }
        } else {
            self.outer()
        };
        for r in &self.ramps {
            v = r.apply(p, v, self.lo, self.hi);
        }
        v
    }
}

/// A mesa ready to sample: its outer edge first, then its tiers.
struct MesaPlan {
    name: String,
    edges: Vec<MesaEdge>,
    reach: crate::gen_land::Rect,
    boulders: f64,
}

impl MesaPlan {
    fn new(m: &MesaDef) -> Result<Self, String> {
        let shape = (m.ramp_grade.unwrap_or(0.2), m.ramp_width_m.unwrap_or(18.0));
        let rough = m.rough_m.unwrap_or(8.0);
        let mut edges = vec![MesaEdge::new(
            &m.polygon_m,
            (0.0, m.height_m),
            true,
            rough,
            &m.ramps_m,
            shape,
        )?];
        for (i, t) in m.tier.iter().enumerate() {
            let first = (t.polygon_m[0][0], t.polygon_m[0][1]);
            let parent = m.tier[..i]
                .iter()
                .rev()
                .find(|o| inside(first, &o.polygon_m))
                .map_or(m.height_m, |o| o.height_m);
            let step = t.height_m > parent;
            let levels = if step {
                (parent, t.height_m)
            } else {
                (t.height_m, parent)
            };
            edges.push(MesaEdge::new(
                &t.polygon_m,
                levels,
                step,
                rough * 0.6,
                &t.ramps_m,
                shape,
            )?);
        }
        let mut reach = edges[0].reach;
        for e in &edges[1..] {
            reach.x0 = reach.x0.min(e.reach.x0);
            reach.y0 = reach.y0.min(e.reach.y0);
            reach.x1 = reach.x1.max(e.reach.x1);
            reach.y1 = reach.y1.max(e.reach.y1);
        }
        Ok(Self {
            name: m.name.clone(),
            edges,
            reach,
            boulders: m.boulders.unwrap_or(1.0),
        })
    }

    /// Height of the mesa above the basin at `p` (0 off the mesa).
    fn height(&self, p: (f64, f64), fray: &Fray) -> f64 {
        if !in_rect(&self.reach, p) {
            return 0.0;
        }
        let mut h = self.edges[0].value(p, fray);
        for e in &self.edges[1..] {
            h += e.value(p, fray) - e.outer();
        }
        h
    }

    fn on_track(&self, p: (f64, f64)) -> bool {
        in_rect(&self.reach, p)
            && self
                .edges
                .iter()
                .any(|e| e.ramps.iter().any(|r| r.on_track(p)))
    }

    /// Ramp markers: name and position beyond the top of each ramp.
    fn markers(&self) -> Vec<(String, (f64, f64))> {
        if self.name.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            for (k, r) in e.ramps.iter().enumerate() {
                let name = if i == 0 {
                    format!("ramp_{}_{}", self.name, k + 1)
                } else {
                    format!("ramp_{}_t{i}_{}", self.name, k + 1)
                };
                out.push((name, r.top(12.0)));
            }
        }
        out
    }
}

/// Boulders on the scree at the foot of every mesa edge (prompt 126), from
/// an own random stream; none on or beside the ramps.
fn mesa_boulders(objects: &mut Objects, mesas: &[MesaPlan], fray: &Fray, seed: u64) -> usize {
    const ROCKS: [&str; 6] = ["Group", "Block", "Split", "Pair", "Leaning", "Layered"];
    let mut random = Random(Rng::new(seed ^ 0x6d65_7361));
    let mut placed = 0;
    for m in mesas {
        for e in &m.edges {
            let (_, tw) = e.talus();
            let width = e.width();
            let n = e.polygon.len();
            for i in 0..n {
                let (a, b) = (e.polygon[i], e.polygon[(i + 1) % n]);
                let len = dist((a[0], a[1]), (b[0], b[1]));
                if len <= 0.0 {
                    continue;
                }
                let attempts = (len / 10.0 * m.boulders).round() as usize;
                // Inward normal of the segment (either way; checked below).
                let (nx, ny) = ((b[1] - a[1]) / len, (a[0] - b[0]) / len);
                for _ in 0..attempts {
                    let t = random.unit();
                    let base = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                    let off = random.range(-e.rough - tw, width + e.rough);
                    let roll = random.unit();
                    let heading = (random.unit() * 360.0) as u16 % 360;
                    let pick = random.unit();
                    for sign in [1.0, -1.0] {
                        let q = (base.0 + sign * nx * off, base.1 + sign * ny * off);
                        let d = e.edge(q, fray);
                        // Scree band: at the foot outside a hollow's
                        // floor, inside a mesa or step.
                        let x = if e.high_inside { d } else { width - d };
                        if x <= 0.4 || x >= tw - 0.3 {
                            continue;
                        }
                        let on_ramp = e.ramps.iter().any(|r| {
                            let (u, v) = r.frame(q);
                            u > r.u_top - 6.0 && u < r.u_top + r.len + 6.0 && v < r.half_w + 14.0
                        });
                        if on_ramp {
                            break;
                        }
                        let (kind, variant) = if roll < 0.06 {
                            ("env_rock_outcrop", ROCKS[(pick * 6.0) as usize % 6])
                        } else if roll < 0.2 {
                            ("env_rock", "Large")
                        } else if roll < 0.6 {
                            ("env_rock", "Medium")
                        } else {
                            ("env_rock", "Small")
                        };
                        if objects.place(kind, variant, q, heading, 0.3) {
                            placed += 1;
                        }
                        break;
                    }
                }
            }
        }
    }
    placed
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

    /// Prompt 123: `river_front` is mirror-symmetric about the path cell
    /// center line y = 1 m: heights, terrain, objects, starts and markers,
    /// the open path cells and the path lengths of both sides.
    #[test]
    fn river_front_is_mirror_symmetric() {
        check_symmetry("river_front", false, &|n: &str| {
            n.replace("_n_", "_x_")
                .replace("_s_", "_n_")
                .replace("_x_", "_s_")
                .replace("battery_n", "battery_x")
                .replace("battery_s", "battery_n")
                .replace("battery_x", "battery_s")
        });
    }

    /// Prompt 124: `trench_line` is point-symmetric about the path cell
    /// center point (1, 1): the same as above with every point turned by
    /// 180°, and its prepared field works, craters and puddles too.
    #[test]
    fn trench_line_is_point_symmetric() {
        check_symmetry("trench_line", true, &|n: &str| {
            let swap =
                |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
            let n = swap(n, "_n_", "_s_");
            let n = swap(&n, "_w_", "_e_");
            let n = swap(&n, "battery_n", "battery_s");
            let n = swap(&n, "front_n", "front_s");
            let n = swap(&n, "lane_w", "lane_e");
            let n = swap(&n, "smoke_1", "smoke_2");
            let n = swap(&n, "smoke_3", "smoke_4");
            swap(&n, "mist_1", "mist_2")
        });
    }

    /// Prompt 126: `mesa_desert` is point-symmetric about (1, 1): mesas,
    /// ramps, boulders, starts and markers (the generated ramp markers of
    /// the south half end in `_image`).
    #[test]
    fn mesa_desert_is_point_symmetric() {
        check_symmetry("mesa_desert", true, &|n: &str| {
            if let Some(north) = n.strip_suffix("_image") {
                return north.to_string();
            }
            if n.starts_with("ramp_") {
                return format!("{n}_image");
            }
            let swap =
                |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
            let n = swap(n, "_n_", "_s_");
            swap(&n, "battery_n", "battery_s")
        });
    }

    /// Checks that the map `name` is its own image under the mirror about
    /// y = 1 m (`point` false) or the turn by 180° about (1, 1) (`point`
    /// true), markers renamed by `image_name`.
    #[allow(clippy::too_many_lines)]
    fn check_symmetry(name: &str, point: bool, image_name: &dyn Fn(&str) -> String) {
        use sim::path::PathGrid;
        use sim::{Catalog, Scale, TerrainRules, Vec2};
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps"))
            .join(name);
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let mut map = Map::load(
            &String::from_utf8(read("map.toml")).unwrap(),
            &read("height.png"),
            &read("terrain.png"),
        )
        .unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        let (sx, sy) = map.samples();
        // Samples 2 m apart from the edge: column i mirrors sx - i (point
        // symmetry), row j mirrors sy - j.
        for j in 1..sy {
            for i in u32::from(point)..sx {
                let (mi, mj) = (if point { sx - i } else { i }, sy - j);
                assert_eq!(
                    map.sample_height_mm(i, j),
                    map.sample_height_mm(mi, mj),
                    "height at sample ({i}, {j})"
                );
                assert_eq!(
                    map.sample_terrain(i, j),
                    map.sample_terrain(mi, mj),
                    "terrain at ({i}, {j})"
                );
            }
        }
        let two = sim::Fixed::from_num(2);
        let image = |p: Vec2| {
            if point {
                Vec2::new(two - p.x, two - p.y)
            } else {
                Vec2::new(p.x, two - p.y)
            }
        };
        let mm = |v: sim::Fixed| (v * sim::Fixed::from_num(1000)).round().to_num::<i64>();
        let key = |kind: u16, p: Vec2, h: u16| (kind, mm(p.x), mm(p.y), h);
        let mut keys: Vec<_> = map
            .objects()
            .iter()
            .map(|o| key(o.kind, o.position, o.heading_deg))
            .collect();
        keys.sort_unstable();
        for o in map.objects() {
            let bridge = map.object_kinds()[usize::from(o.kind)].deck.is_some();
            let h = if bridge {
                o.heading_deg
            } else if point {
                (o.heading_deg + 180) % 360
            } else {
                (540 - o.heading_deg) % 360
            };
            let m = image(o.position);
            // Under point symmetry an object on the center is its own image.
            if point && m == o.position {
                continue;
            }
            let k = (o.kind, mm(m.x), mm(m.y), h);
            assert!(keys.binary_search(&k).is_ok(), "no image of object {o:?}");
        }
        let starts = map.starts();
        for s in starts {
            assert!(
                starts
                    .iter()
                    .any(|t| t.position == image(s.position) && t.team != s.team),
                "start {} has no mirrored enemy start",
                s.player
            );
        }
        let markers = map.markers();
        for m in markers {
            let other = image_name(&m.name);
            let t = markers
                .iter()
                .find(|t| t.name == other)
                .unwrap_or_else(|| panic!("{other}"));
            assert_eq!(
                t.position,
                image(m.position),
                "marker {} and {other}",
                m.name
            );
        }
        // Prepared ground (prompt 124): every work, crater and puddle has
        // its image.
        let prepared = map.prepared();
        for w in &prepared.works {
            let points: Vec<Vec2> = w.points.iter().map(|&p| image(p)).collect();
            assert!(
                prepared
                    .works
                    .iter()
                    .any(|o| o.kind == w.kind && o.points == points),
                "no image of {} at {:?}",
                w.kind.name(),
                w.points[0]
            );
        }
        let mut craters: Vec<_> = prepared
            .craters
            .iter()
            .map(|c| (mm(c.position.x), mm(c.position.y), c.caliber_mm, c.water))
            .collect();
        craters.sort_unstable();
        for c in &prepared.craters {
            let m = image(c.position);
            assert!(
                craters
                    .binary_search(&(mm(m.x), mm(m.y), c.caliber_mm, c.water))
                    .is_ok(),
                "no image of crater {c:?}"
            );
        }
        for p in &prepared.puddles {
            assert!(
                prepared
                    .puddles
                    .iter()
                    .any(|q| q.position == image(p.position) && q.radius_mm == p.radius_mm),
                "no image of puddle {p:?}"
            );
        }
        // Open path cells (cell (x, y) mirrors (w - x, h - y) or (x, h - y))
        // on the map as a match starts on it (prepared ground dug).
        let dug = crate::prepared_ground(map.clone(), &["de_inf_rifle_squad"])
            .unwrap()
            .0;
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types =
            sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
                .unwrap();
        let grid = PathGrid::new(&dug, &TerrainRules::builtin().unwrap(), &types);
        let (w, h) = (dug.samples().0 / 2, dug.samples().1 / 2);
        let mut odd = Vec::new();
        for profile in 0..grid.profiles() {
            let class = grid.class_of(profile);
            for y in 1..h {
                for x in u32::from(point)..w {
                    let mx = if point { w - x } else { x };
                    if grid.passable(class, x, y) != grid.passable(class, mx, h - y) {
                        odd.push((profile, x, y));
                    }
                }
            }
        }
        assert!(
            odd.is_empty(),
            "asymmetric path cells: {} {:?}",
            odd.len(),
            &odd[..odd.len().min(20)]
        );
        // Both sides need the same effort: the path costs (time on the
        // ground) agree within 2 %. The search may pick another of two
        // routes of nearly the same cost on the mirrored side (a road
        // detour against the direct way), so lengths can differ more.
        let mut worst = Vec::new();
        let north: Vec<_> = starts.iter().filter(|s| s.position.y < 0).collect();
        for n in north {
            let s = starts
                .iter()
                .find(|t| t.position == image(n.position))
                .unwrap();
            // Prompt 126: `needle_*` markers stand on mesas without a ramp
            // (no path on purpose).
            for m in markers
                .iter()
                .filter(|m| m.position.y < 0 && !m.name.starts_with("needle_"))
            {
                let other = markers
                    .iter()
                    .find(|t| t.name == image_name(&m.name))
                    .unwrap();
                for profile in 0..2 {
                    let a = grid.find_path(n.position, m.position, profile);
                    let b = grid.find_path(s.position, other.position, profile);
                    let (Some(a), Some(b)) = (a, b) else {
                        panic!("no path to {} or {}", m.name, other.name);
                    };
                    // Point maps: the supply point in the middle lies in
                    // the house (taken from within 25 m); the path ends
                    // at the nearest open cell.
                    let reach = sim::Fixed::from_num(if point { 20 } else { 0 });
                    let near = |p: Vec2, q: Vec2| (p - q).length() <= reach;
                    assert!(near(a.goal, m.position), "{} not reached", m.name);
                    assert!(near(b.goal, other.position), "{} not reached", other.name);
                    let (ca, cb) = (f64::from(a.cost), f64::from(b.cost));
                    if (ca - cb).abs() > 0.02 * ca {
                        worst.push(format!(
                            "start {} -> {}: cost {ca}, start {} -> {}: cost {cb} (profile {profile})",
                            n.player, m.name, s.player, other.name
                        ));
                    }
                }
            }
        }
        assert!(worst.is_empty(), "{worst:#?}");
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
