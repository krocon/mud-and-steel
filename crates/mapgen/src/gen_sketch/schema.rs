//! Sketch TOML schema and loader (prompt 088).

use super::mesa::MesaDef;
use super::plan::{edge_distance, inside, polyline_distance};
use crate::gen_land::{dist, smoothstep};
use serde::Deserialize;
use std::path::Path;

// --- The sketch -----------------------------------------------------------

pub(super) type P = [f64; 2];

#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Sketch {
    #[serde(default)]
    pub(super) name: String,
    /// Another sketch this one changes (only the fields named here).
    #[serde(default)]
    pub(super) base: Option<String>,
    #[serde(default)]
    pub(super) size_m: Option<[u32; 2]>,
    #[serde(default)]
    pub(super) biome: Option<String>,
    #[serde(default)]
    pub(super) weather: Option<String>,
    #[serde(default)]
    pub(super) seed: Option<u64>,
    #[serde(default)]
    pub(super) water_flow_deg: Option<u32>,
    #[serde(default)]
    pub(super) start: Vec<StartDef>,
    #[serde(default)]
    pub(super) supply: Vec<Named>,
    #[serde(default)]
    pub(super) marker: Vec<Named>,
    #[serde(default)]
    pub(super) river: Vec<RiverDef>,
    #[serde(default)]
    pub(super) island: Vec<Island>,
    #[serde(default)]
    pub(super) lake: Vec<Area>,
    #[serde(default)]
    pub(super) crossing: Vec<CrossingDef>,
    #[serde(default)]
    pub(super) road: Vec<RoadDef>,
    #[serde(default)]
    pub(super) ring_road: Vec<RingRoad>,
    #[serde(default)]
    pub(super) square: Vec<Area>,
    #[serde(default)]
    pub(super) wood: Vec<Area>,
    #[serde(default)]
    pub(super) scatter: Vec<Area>,
    #[serde(default)]
    pub(super) field: Vec<Area>,
    #[serde(default)]
    pub(super) row: Vec<Row>,
    #[serde(default)]
    pub(super) ring: Vec<Ring>,
    #[serde(default)]
    pub(super) farm: Vec<Farm>,
    #[serde(default)]
    pub(super) building: Vec<Building>,
    #[serde(default)]
    pub(super) hill: Vec<Hill>,
    #[serde(default)]
    pub(super) rail: Vec<Rail>,
    #[serde(default)]
    pub(super) paint: Vec<Paint>,
    #[serde(default)]
    pub(super) village: Vec<Village>,
    /// True: everything outside the `land` polygons is sea (stage 2).
    #[serde(default)]
    pub(super) sea: bool,
    #[serde(default)]
    pub(super) land: Vec<Area>,
    #[serde(default)]
    pub(super) causeway: Vec<Strip>,
    #[serde(default)]
    pub(super) shoal: Vec<Strip>,
    /// Sea maps: water inside the land that belongs to the sea (lagoons,
    /// inlets), at the same level; polygons may overlap.
    #[serde(default)]
    pub(super) bay: Vec<Area>,
    #[serde(default)]
    pub(super) town: Vec<Town>,
    #[serde(default)]
    pub(super) patchwork: Vec<Patchwork>,
    /// Look on top of the biome (prompt 122): written to `map.toml`.
    #[serde(default)]
    pub(super) look: Option<String>,
    /// Waterfalls (prompt 122): presentation only, two markers each.
    #[serde(default)]
    pub(super) waterfall: Vec<Waterfall>,
    /// Timber piers into the water (prompt 122): scenery, block nothing.
    #[serde(default)]
    pub(super) pier: Vec<Pier>,
    /// Rapids (prompt 122): stones in the water along a line.
    #[serde(default)]
    pub(super) rapids: Vec<Rapids>,
    /// Prompt 123: `"north"` makes the map mirror-symmetric: the north half
    /// (up to the path cell center line next to y = 0) is generated and
    /// copied, mirrored, onto the south half (ground, terrain, objects).
    /// Prompt 124: `"point"` turns the north half by 180°. Prompt 130:
    /// `"west"` mirrors the west half onto the east half (west against
    /// east).
    #[serde(default)]
    pub(super) mirror: Option<String>,
    /// Prompt 123: prepared positions of field works (gun pits, bunkers,
    /// trenches) facing the enemy.
    #[serde(default)]
    pub(super) works: Vec<Works>,
    /// Prompt 124: trench lines (prepared field works of the map).
    #[serde(default)]
    pub(super) trench: Vec<TrenchDef>,
    /// Prompt 124: barbed wire along lines, with gaps.
    #[serde(default)]
    pub(super) wire: Vec<WireDef>,
    /// Prompt 124: lanes kept free of craters, wire and objects (ways for
    /// vehicles through a crater field).
    #[serde(default)]
    pub(super) lane: Vec<Strip>,
    /// Prompt 124: shell crater fields (prepared craters of the map).
    #[serde(default)]
    pub(super) crater_field: Vec<CraterField>,
    /// Prompt 124: wet hollows with puddles.
    #[serde(default)]
    pub(super) marsh: Vec<Marsh>,
    /// Prompt 124: camps of tents, dumps and stores.
    #[serde(default)]
    pub(super) camp: Vec<Camp>,
    /// Prompt 202: plank roads over mud (pieces of `env_plank_road` end to
    /// end along the line; pieces that would overlap something are left out).
    #[serde(default)]
    pub(super) plank: Vec<PlankDef>,
    /// Prompt 221: continuous Bocage hedgerows (`env_hedge` pieces end to end
    /// along polylines, breaking automatically where ways cross).
    #[serde(default)]
    pub(super) hedge: Vec<HedgeDef>,
    /// Prompt 226: wooden fence or pasture-post polylines (`env_fence_wood` or
    /// `env_fence_picket` pieces along polylines, breaking where ways cross).
    #[serde(default)]
    pub(super) fence: Vec<FenceDef>,
    /// Prompt 241: compact rural vegetable garden beds (parallel green crop rows
    /// over dark ploughed soil, optionally enclosed by a picket fence).
    #[serde(default)]
    pub(super) garden: Vec<GardenDef>,
    /// Prompt 133: dry river beds (wadis), see [`WadiDef`].
    #[serde(default)]
    pub(super) wadi: Vec<WadiDef>,
    /// Prompt 126: table mountains with steps, hollows and ramps.
    #[serde(default)]
    pub(super) mesa: Vec<MesaDef>,
    /// Prompt 243: modular rock cliff walls (`env_rock_cliff`) along polylines.
    #[serde(default)]
    pub(super) cliff: Vec<CliffDef>,
    /// Prompt 148: tunnels through rock massifs (`[[tunnel]]`).
    #[serde(default)]
    pub(super) tunnel: Vec<TunnelDef>,
}

/// A tunnel through an impassable mountain ridge (prompt 148).
/// First and last points are the two portals; intermediate points curve the bore.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct TunnelDef {
    /// `"road"` (default) or `"rail"`.
    #[serde(default = "default_tunnel_kind")]
    pub(super) kind: String,
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) width_mm: Option<u32>,
    #[serde(default)]
    pub(super) clearance_height_mm: Option<u32>,
    #[serde(default)]
    pub(super) clearance_width_mm: Option<u32>,
    /// Optional explicit sole heights `[portal 0, portal 1]` in meters.
    #[serde(default)]
    pub(super) portal_heights_m: Option<[f64; 2]>,
    /// Optional knife-edge rock ridge height (m) sculpted directly over the
    /// tunnel bore (leaving the portal forecourts level), ensuring the ground
    /// above the bore is steep impassable rock (one layer per cell).
    #[serde(default)]
    pub(super) ridge_m: Option<f64>,
    /// Optional lateral half-width (m, default 70) of the sculpted ridge across the bore.
    #[serde(default)]
    pub(super) ridge_width_m: Option<f64>,
}

fn default_tunnel_kind() -> String {
    "road".into()
}

const fn default_true() -> bool {
    true
}

/// A modular cliff wall (`env_rock_cliff`) along a polyline (prompt 243).
/// `facing_m` is a direction vector `[fx, fy]` (x east, y south) toward the
/// valley side so each module's steep `+X` showcase front faces the valley.
/// When `height_m > 0.0`, the generator shapes an upper slope/ledge behind
/// the wall (`-facing` side) with a clean foot on the valley side.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct CliffDef {
    pub(super) points_m: Vec<P>,
    pub(super) facing_m: P,
    #[serde(default)]
    pub(super) height_m: f64,
    #[serde(default = "default_true")]
    pub(super) boulders: bool,
    #[serde(default = "default_true")]
    pub(super) forest_top: bool,
}

/// A trench line (prompt 124). `fire`: the line becomes a zigzag trace of
/// teeth `tooth_m` long, every tooth a fire bay pushed `depth_m` towards
/// `facing_m` (a direction) over the middle of the tooth, with MG nests in
/// every `nest_every`-th bay and dugouts behind every `dugout_every`-th
/// tooth (0: none); the simulation digs it as a fire trench (which adds its
/// own small zigzag). `line`: a fire trench exactly along the points (a
/// hand-drawn trace of large jags, redans and closed works), with MG nests
/// at `nests_m` and dugouts at `dugouts_m`. `comm`: a straight
/// communication trench along the points. Both become prepared field works of the map (`map.toml`
/// `[[field_work]]`), dug before the first tick like an engineer's trench.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct TrenchDef {
    pub(super) kind: String,
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) tooth_m: Option<f64>,
    #[serde(default)]
    pub(super) depth_m: Option<f64>,
    #[serde(default)]
    pub(super) facing_m: Option<P>,
    #[serde(default)]
    pub(super) nest_every: u32,
    #[serde(default)]
    pub(super) dugout_every: u32,
    /// `line`: MG nests and dugouts at these points.
    #[serde(default)]
    pub(super) nests_m: Vec<P>,
    #[serde(default)]
    pub(super) dugouts_m: Vec<P>,
}

/// Barbed wire along a line (prompt 124): prepared wire works of the map,
/// cut open `gap_m` wide (default 16) at the points of `gaps_m` and where a
/// lane crosses.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct WireDef {
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) gaps_m: Vec<P>,
    #[serde(default)]
    pub(super) gap_m: Option<f64>,
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
pub(super) struct CraterField {
    pub(super) polygon_m: Vec<P>,
    pub(super) density_ha: f64,
    pub(super) calibers: Vec<P>,
    #[serde(default)]
    pub(super) water: f64,
    #[serde(default)]
    pub(super) edge_m: Option<f64>,
}

/// Wet hollows (prompt 124): `count` puddles of `radius_m` (min, max)
/// inside the polygon: a shallow dip of mud with standing water (the water
/// is presentation only, `map.toml` `[[puddle]]`). Own random stream.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Marsh {
    pub(super) polygon_m: Vec<P>,
    pub(super) count: u32,
    pub(super) radius_m: P,
}

/// A camp (prompt 124): `count` objects of each item placed loosely inside
/// the polygon, turned along the nearest way.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Camp {
    pub(super) polygon_m: Vec<P>,
    pub(super) items: Vec<CampItem>,
}

/// A dry river bed (prompt 133): a gravel bed (dirt-track ground) `width_m` wide, `depth_m`
/// (default 3.5) below the ground, its rims falling over `WADI_RIM_M` (under
/// 60 % also on a slant to the path grid: infantry crosses everywhere);
/// rows of large boulders along both rims stop vehicles except at the fords
/// (`fords_m`: within `ford_m`, default 40 m, of the point the rims run out
/// over `WADI_FORD_RIM_M`) and at the bridges (a `[[crossing]]` on a wadi
/// spans its bed like water). No water.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct WadiDef {
    pub(super) width_m: f64,
    #[serde(default)]
    pub(super) depth_m: Option<f64>,
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) fords_m: Vec<P>,
    #[serde(default)]
    pub(super) ford_m: Option<f64>,
}

impl WadiDef {
    pub(super) const RIM_M: f64 = 14.0;
    pub(super) const FORD_RIM_M: f64 = 32.0;

    /// Width of the rim at `p`: long ramps at the fords.
    pub(super) fn rim(&self, p: (f64, f64)) -> f64 {
        let r = self.ford_m.unwrap_or(40.0);
        let fd = self
            .fords_m
            .iter()
            .map(|f| dist(p, (f[0], f[1])))
            .fold(f64::INFINITY, f64::min);
        Self::RIM_M + (Self::FORD_RIM_M - Self::RIM_M) * (1.0 - smoothstep(r * 0.6, r, fd))
    }

    /// Whether `p` lies in the bed (bridges span it).
    pub(super) fn in_bed(&self, p: (f64, f64)) -> bool {
        polyline_distance(p, &self.points_m) < self.width_m / 2.0
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct PlankDef {
    pub(super) points_m: Vec<P>,
}

/// A hedgerow polyline (prompt 221): `env_hedge` segments (default `Bocage`)
/// placed end to end with gentle overlap; automatically omitted on ways.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct HedgeDef {
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) variant: Option<String>,
}

/// A wooden fence or pasture-post polyline (prompts 226, 241): `env_fence_wood`
/// (default) or `env_fence_picket` (`kind = "picket"`) pieces placed along the
/// polyline, automatically omitted on ways.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct FenceDef {
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) kind: Option<String>,
    #[serde(default)]
    pub(super) variant: Option<String>,
    #[serde(default)]
    pub(super) spacing_m: Option<f64>,
}

/// Compact rural vegetable garden (prompt 241): places tight parallel crop rows
/// (`env_bush:Round` / `env_sunflowers`) inside the polygon (pair with a
/// `[[field]] kind = "ploughed"` for dark tilled soil) and optionally frames
/// the perimeter with `env_fence_picket`.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct GardenDef {
    pub(super) polygon_m: Vec<P>,
    #[serde(default)]
    pub(super) angle_deg: f64,
    #[serde(default)]
    pub(super) row_spacing_m: Option<f64>,
    #[serde(default)]
    pub(super) plant_spacing_m: Option<f64>,
    #[serde(default)]
    pub(super) fenced: bool,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct CampItem {
    pub(super) kind: String,
    pub(super) variant: String,
    pub(super) count: u32,
}

/// Prepared field works (prompt 123), placed as a group facing `facing_m`
/// (a direction, x east and y south): `battery`: four sandbag gun pits
/// 15 m apart in a line across the facing, a parked gun behind each (prompt
/// 202) and a trench section on each flank (a way in from behind stays free); `bridgehead`: an MG bunker,
/// three trench sections in a line beside it and a sandbag position.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Works {
    pub(super) kind: String,
    pub(super) position_m: P,
    pub(super) facing_m: P,
}

/// A waterfall (prompt 122): presentation only. The generator writes the
/// markers `waterfall_<n>_top` (the rock edge it falls from) and
/// `waterfall_<n>_foot` (where it meets the water); Godot draws the falling
/// water, spray and sound between them. No rule reads them.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Waterfall {
    pub(super) top_m: P,
    pub(super) foot_m: P,
}

/// A timber pier (prompt 122): `env_pier_wood` segments of 10 m from the
/// first point (on the shore) along the line into the water; `boat`
/// (default true) moors a small boat at the last segment. Scenery: the
/// pier blocks nothing and stands on the water level.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Pier {
    pub(super) points_m: [P; 2],
    #[serde(default)]
    pub(super) boat: Option<bool>,
}

/// Rapids (prompt 122): `count` stones (`env_rock`, small and medium: they
/// block nothing) in the water within `width_m` of the line, so the water
/// foams around them. Deep water stays deep (no ford).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Rapids {
    pub(super) points_m: Vec<P>,
    pub(super) width_m: f64,
    pub(super) count: u32,
}

/// A patchwork of fields (stage 3): inside the polygon, plots of `size_m`
/// (length, width; each varied by up to a third) in a grid turned along
/// `axis` (a direction, x east and y south), separated by 6 m of grass;
/// the kind of each plot is drawn from `kinds`, and a share `meadow` stays
/// grass. Plots whose middle lies outside the polygon are left out.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Patchwork {
    pub(super) polygon_m: Vec<P>,
    #[serde(default)]
    pub(super) axis: Option<P>,
    pub(super) size_m: P,
    pub(super) kinds: Vec<String>,
    #[serde(default)]
    pub(super) meadow: f64,
}

/// Street building (stage 2): inside the polygon, buildings of `kinds`
/// line both sides of every road and track of the sketch, long side along
/// the street, `setback_m` (default 3) behind its edge, `gap_m` (default
/// 4) apart on average; a share `damaged` damaged or ruined.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Town {
    pub(super) polygon_m: Vec<P>,
    pub(super) kinds: Vec<String>,
    #[serde(default)]
    pub(super) setback_m: Option<f64>,
    #[serde(default)]
    pub(super) gap_m: Option<f64>,
    #[serde(default)]
    pub(super) damaged: f64,
}

/// A strip along a line: a causeway (land across the sea, open under the
/// bridges of its crossings) or a shoal (shallow water, a ford for tanks).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Strip {
    pub(super) width_m: f64,
    pub(super) points_m: Vec<P>,
}

/// A railway line (stage 2): a 7 m ballast bed (terrain like a field
/// track, open to everybody) on a low embankment with telegraph poles
/// beside it. Only presentation: no rails yet, no trains.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Rail {
    pub(super) points_m: Vec<P>,
    #[serde(default)]
    pub(super) embankment_m: Option<f64>,
}

/// Ground painted with a terrain type of `data/terrain.toml` (e.g. `sand`,
/// `mud`, `dirt_track`) inside a polygon; with `noise_m` only in blotches
/// covering about `cover` of it (trampled sand between ruins).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Paint {
    pub(super) terrain: String,
    pub(super) polygon_m: Vec<P>,
    #[serde(default)]
    pub(super) noise_m: Option<f64>,
    #[serde(default)]
    pub(super) cover: Option<f64>,
}

/// Loose houses inside a polygon: `count` buildings of `kinds`, turned
/// along the nearest way, a share `damaged` of them damaged or ruined, and
/// `walls` short runs of broken walls between them.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Village {
    pub(super) polygon_m: Vec<P>,
    pub(super) kinds: Vec<String>,
    pub(super) count: u32,
    #[serde(default)]
    pub(super) damaged: f64,
    #[serde(default)]
    pub(super) walls: u32,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct StartDef {
    pub(super) player: u8,
    pub(super) team: u8,
    pub(super) position_m: P,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Named {
    pub(super) name: String,
    pub(super) position_m: P,
    /// Prompt 123: radius kept flat and free of objects around the point
    /// (gun positions); supply points are kept flat over 20 m anyway.
    #[serde(default)]
    pub(super) flat_m: Option<f64>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct RiverDef {
    pub(super) width_m: f64,
    pub(super) points_m: Vec<P>,
    /// Prompt 124: a brook, shallow over its whole width (no deep channel).
    #[serde(default)]
    pub(super) shallow: bool,
    /// Prompt 125: the river cuts through hills: its water follows the
    /// plain ground (the ground noise without bumps and hills, like a
    /// `[[lake]] kind = "river"`) and the banks fall to it over `valley_m`
    /// (default 40 m) instead of the water climbing over the hills.
    #[serde(default)]
    pub(super) through_hills: bool,
    /// Width of the bank valley beside the water (m); default 40.
    #[serde(default)]
    pub(super) valley_m: Option<f64>,
    /// Second layout of `river_fork_front`: a tributary that keeps its own
    /// bed where it runs into a river listed before it. Without this the
    /// first river's shallow margin lies across the mouth as a bar at the
    /// waterline (a ford nobody drew).
    #[serde(default)]
    pub(super) joins: bool,
}

/// An island: a circle (`center_m`, `radius_m`) or (prompt 123) a polygon.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Island {
    #[serde(default)]
    pub(super) center_m: P,
    #[serde(default)]
    pub(super) radius_m: f64,
    #[serde(default)]
    pub(super) polygon_m: Vec<P>,
}

impl Island {
    /// How far `p` lies inside the island (m); negative outside.
    pub(super) fn depth(&self, p: (f64, f64)) -> f64 {
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
pub(super) struct Area {
    #[serde(default)]
    pub(super) kind: String,
    #[serde(default)]
    pub(super) spacing_m: Option<f64>,
    pub(super) polygon_m: Vec<P>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct CrossingDef {
    /// `stone`, `wood`, `ford` or `stone_long` (prompt 122: a chain of
    /// high stone arches over the whole width of the water, 60-120 m).
    pub(super) kind: String,
    pub(super) position_m: P,
    /// `ns`: the way crosses north–south (the bridge axis runs along y),
    /// `we`: west–east.
    pub(super) axis: String,
    /// Deck height above the bank (m); default 3 (stone), 1.5 (timber), 6
    /// (long stone bridge).
    #[serde(default)]
    pub(super) rise_m: Option<f64>,
    /// Long stone bridges: how far along the way from the point water
    /// counts for the bridge (default 150 m). A shorter reach keeps a
    /// bridge to its own channel where a small island carries the road on
    /// to the next bridge (`river_front`).
    #[serde(default)]
    pub(super) reach_m: Option<f64>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct RoadDef {
    /// `road` or `track`.
    pub(super) class: String,
    pub(super) width_m: f64,
    pub(super) points_m: Vec<P>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct RingRoad {
    pub(super) center_m: P,
    pub(super) radius_m: f64,
    pub(super) width_m: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub(super) kinds: Vec<String>,
    pub(super) from_m: P,
    pub(super) to_m: P,
    pub(super) gap_m: f64,
    #[serde(default)]
    pub(super) damaged: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Ring {
    pub(super) kinds: Vec<String>,
    pub(super) center_m: P,
    pub(super) radius_m: P,
    pub(super) count: u32,
    #[serde(default)]
    pub(super) damaged: f64,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Farm {
    pub(super) center_m: P,
    pub(super) house: String,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Building {
    pub(super) kind: String,
    #[serde(default)]
    pub(super) variant: Option<String>,
    pub(super) position_m: P,
    pub(super) heading_deg: u16,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct Hill {
    pub(super) center_m: P,
    pub(super) radius_m: f64,
    pub(super) height_m: f64,
    /// `""`: a smooth bump; `peak` (prompt 122): a rock mountain with a
    /// pointed top, steep upper flanks and crags (ridged noise), gentle at
    /// the foot.
    #[serde(default)]
    pub(super) kind: String,
}

/// Reads a sketch from `dir`, resolving `base`.
pub(super) fn load(dir: &Path, name: &str) -> Result<Sketch, String> {
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
