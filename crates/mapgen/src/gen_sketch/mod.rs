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
//! Prompt 128 added climbs for infantry on mesas (`climbs_m`: steep
//! ramps without a track, closed to vehicles by boulders).
//! Prompt 129 added terraced hills: mesa and tier edges as even slopes
//! (`slope_grade`) with rock bands in sectors (`cliffs_m`), ramps winding
//! along lines (`ramp_paths_m`) and boulder rows that stop vehicles on the
//! open slopes (`stop_vehicles`).
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

use crate::gen_land::{Garden, Objects, Way, dist, heading_of, noise};
use crate::gen_map::{Random, write_png_rect};
use sim::objects::CSV_HEADER;
use sim::prepared::PreparedWorkKind;
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
pub const ALL: [&str; 23] = [
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
    "battery_heights",
    "meander_valley",
    "desert_plateau",
    "mesa_desert",
    "kessel_lake",
    "pit_hill",
    "omega_bend",
    "bocage_village",
    "autumn_village",
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

mod mesa;
mod plan;
mod schema;
mod settlements;
#[cfg(test)]
mod tests;
mod works;

use mesa::*;
use plan::*;
use schema::*;
use settlements::*;
use works::*;

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
                reach: c.reach_m.unwrap_or(LONG_REACH_M),
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
        // Prompt 130: the path cell center line next to x = 0.
        Some("west") => Some(Mirror::West(snap(0.0, half.0))),
        Some(other) => {
            return Err(format!(
                "unknown mirror {other:?} (\"north\", \"point\" or \"west\")"
            ));
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
    let dry_bed = |x: f64, y: f64| sketch.wadi.iter().any(|w| w.in_bed((x, y)));
    let mut bridges = place_bridges(
        &crossings,
        (sx, sy),
        half,
        &mut heights,
        &mut terrain,
        &dry_bed,
    )?;
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
    for pl in &sketch.plank {
        plank_road(&mut objects, pl);
    }
    for h in &sketch.hedge {
        hedge_line(&mut objects, h, &plan.ways);
    }
    for f in &sketch.fence {
        fence_line(&mut objects, f, &plan.ways);
    }
    for g in &sketch.garden {
        garden_beds(&mut objects, g, &plan.ways);
    }
    cliff_lines(
        &mut objects,
        &sketch,
        &plan.ways,
        &bridges,
        &plan.mesas,
        &biome,
        seed,
    );
    // Prompt 128: boulders that stop vehicles on the climbs for infantry
    // (none on maps without climbs).
    wadi_stoppers(&mut objects, &sketch.wadi, &bridges);
    let stoppers = climb_stoppers(&mut objects, &plan.mesas)
        + slope_stoppers(&mut objects, &plan.mesas, &plan.fray);
    let boulders = mesa_boulders(&mut objects, &plan.mesas, &plan.fray, seed) + stoppers;
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
                // Prompt 129: ramps along lines with their length and grade.
                for r in &e.paths {
                    let (f, t) = (r.points[0], r.top(0.0));
                    ramps.push(format!(
                        "({:.0}, {:.0})->({:.0}, {:.0}) along {:.0} m at {:.1} %",
                        t.0,
                        t.1,
                        f.0,
                        f.1,
                        r.len,
                        (e.hi - e.lo) / r.len * 100.0
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
            "{summary} (objects in the {} half), {} object rows after mirroring",
            if m.across_y() { "north" } else { "west" },
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
