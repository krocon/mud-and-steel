//! Large city map: `cli gen-city` (prompt 079).
//!
//! A 3 km map with a city in the middle, from the inside out:
//!
//! - **Center:** a paved square (80 × 60 m) with the town hall and the
//!   church, where the two main streets cross; they run on to the map edge
//!   as country roads.
//! - **Block quarter** (to 360 m): a street grid of 120 m (main streets
//!   16 m wide, side streets 12 m) whose cells hold apartment blocks
//!   (prompt 078) in rings, rows, angles and terraces around courtyards,
//!   with trees, sheds and walls; town houses in the corner cells, a park
//!   and a warehouse quarter in each half.
//! - **Suburb** (to about 700 m): single houses with gardens, fences, fruit
//!   trees and firewood along 8 m streets and two ring streets.
//! - **Land:** fields, field tracks with hedgerows, farmsteads, woods and
//!   gentle hills; a bypass ring road; the four starts in two teams (team 0
//!   south, team 1 north).
//!
//! Street network, squares, supply points and starts are point-symmetric
//! about `CENTER`, the path cell center next to the map center, so every
//! street axis lies on a center line of the 4 m path grid in both halves.
//! Building kinds, damage and growth differ between the halves.
//!
//! Like the other generators it runs outside the simulation and may use
//! floats, but only basic arithmetic and `sqrt` (exactly rounded by IEEE
//! 754) and the simulation's random generator, so the same seed gives the
//! same files everywhere. Objects are placed with the `gen_land` placer
//! (no overlaps, off roads and tracks). No real place names.

use crate::gen_land::{Garden, Objects, Rect, Way, dist, noise, smoothstep};
use crate::gen_map::{Random, write_png};
use sim::objects::CSV_HEADER;
use sim::{ObjectRules, Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;
const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const MUD: u8 = 4;
const SNOW: u8 = 5;

/// The map is this wide and deep (m).
pub const SIZE_M: u32 = 3000;
const HALF: f64 = 1500.0;
const SAMPLES: usize = 1501;

/// Center of the point symmetry in map coordinates: the center of the path
/// cell next to the map center (path cells are 4 m, their center lines lie
/// at `-1499 + 4 k` m on this map).
pub const CENTER: (f64, f64) = (1.0, 1.0);

/// Street grid of the block quarter (local coordinates around `CENTER`).
pub const GRID: [f64; 7] = [-360.0, -240.0, -120.0, 0.0, 120.0, 240.0, 360.0];
/// The three north-south (and east-west) main streets through the city.
pub const ROUTES: [f64; 3] = [-240.0, 0.0, 240.0];
/// Half widths of main streets, side streets, suburb streets, the country
/// roads beyond the suburb and the bypass (m).
const MAIN_HALF: f64 = 8.0;
const SIDE_HALF: f64 = 6.0;
const SUBURB_HALF: f64 = 6.0;
const BYPASS_HALF: f64 = 4.0;
const COUNTRY_HALF: f64 = 5.0;
/// Block quarter and suburb extents (Chebyshev distance from the center).
pub const BLOCK_EDGE: f64 = 360.0;
const SUBURB_RING: f64 = 520.0;
pub const SUBURB_EDGE: f64 = 660.0;
/// Suburb houses stand within this distance of the center (rounds the city).
const SUBURB_RADIUS: f64 = 780.0;
/// Bypass ring road (Chebyshev distance, corners cut at `BYPASS_CORNER`).
pub const BYPASS: f64 = 950.0;
const BYPASS_CORNER: f64 = 700.0;
/// The city is flat within this distance; the land blends in beyond.
const FLAT_RADIUS: f64 = 780.0;
const FLAT_BLEND: f64 = 240.0;
/// Half extents of the square (east-west, north-south).
pub const SQUARE: (f64, f64) = (40.0, 30.0);
/// Distance of building fronts from the street edge (m).
const SETBACK: f64 = 4.0;

const START_CLEAR_M: f64 = 60.0;
const STRONGPOINT_CLEAR_M: f64 = 42.0;

/// Starts (player, team, local x, y): team 0 south, team 1 north; a match
/// of two uses players 0 and 1, diagonally opposite.
pub const STARTS: [(u8, u8, f64, f64); 4] = [
    (0, 0, -250.0, 1100.0),
    (1, 1, 250.0, -1100.0),
    (2, 0, 250.0, 1100.0),
    (3, 1, -250.0, -1100.0),
];

/// Places of the south half (local coordinates); the north half has them
/// mirrored through the center.
const PARK: (f64, f64) = (-180.0, 180.0);
const WAREHOUSES: (f64, f64) = (180.0, 300.0);
const CROSSING: (f64, f64) = (-240.0, SUBURB_RING);
const FARM: (f64, f64) = (-620.0, 810.0);
const STRONGPOINT: (f64, f64) = (330.0, 810.0);
/// Farmsteads without a supply point (south half; mirrored north).
const FARMS_OTHER: [(f64, f64); 2] = [(-1200.0, 650.0), (1000.0, 1250.0)];

/// Parameters of `cli gen-city`.
pub struct Options<'a> {
    pub name: &'a str,
    pub seed: u64,
    pub biome: &'a str,
    pub out: &'a Path,
}

/// Map coordinates of the local point `(u, v)` around `CENTER`.
fn m((u, v): (f64, f64)) -> (f64, f64) {
    (u + CENTER.0, v + CENTER.1)
}

/// The local point mirrored through the center.
fn mirror((u, v): (f64, f64)) -> (f64, f64) {
    (-u, -v)
}

fn line_half(g: f64) -> f64 {
    if ROUTES.contains(&g) {
        MAIN_HALF
    } else {
        SIDE_HALF
    }
}

/// Half width of the street on grid line `g` at the distance `along` from
/// the center along it, or `None` where the line has no street.
fn grid_street(g: f64, along: f64) -> Option<f64> {
    let a = along.abs();
    if a <= BLOCK_EDGE + SIDE_HALF {
        Some(line_half(g))
    } else if a <= SUBURB_EDGE {
        Some(if ROUTES.contains(&g) {
            MAIN_HALF
        } else {
            SUBURB_HALF
        })
    } else if g == 0.0 {
        Some(COUNTRY_HALF)
    } else if ROUTES.contains(&g) && a <= BYPASS + BYPASS_HALF {
        Some(BYPASS_HALF)
    } else {
        None
    }
}

/// Paved street at the local point: the square, the grid lines of the block
/// quarter with their continuations, the inner suburb ring.
fn grid_road(u: f64, v: f64) -> bool {
    if u.abs() <= SQUARE.0 && v.abs() <= SQUARE.1 {
        return true;
    }
    for g in GRID {
        if grid_street(g, v).is_some_and(|h| (u - g).abs() <= h)
            || grid_street(g, u).is_some_and(|h| (v - g).abs() <= h)
        {
            return true;
        }
    }
    let r = SUBURB_RING;
    ((u.abs() - r).abs() <= SUBURB_HALF && v.abs() <= r + SUBURB_HALF)
        || ((v.abs() - r).abs() <= SUBURB_HALF && u.abs() <= r + SUBURB_HALF)
}

/// A chamfered square ring through the given Chebyshev distance with the
/// corners cut from `corner` (local coordinates, closed polyline).
fn ring(r: f64, corner: f64) -> Vec<(f64, f64)> {
    vec![
        (r, -corner),
        (r, corner),
        (corner, r),
        (-corner, r),
        (-r, corner),
        (-r, -corner),
        (-corner, -r),
        (corner, -r),
        (r, -corner),
    ]
}

/// A way with a bounding box, so rasterizing skips it far away.
struct Lane {
    way: Way,
    bounds: Rect,
    hedges: bool,
}

impl Lane {
    /// From local points.
    fn new(points: &[(f64, f64)], half: f64, id: u8, hedges: bool) -> Self {
        let points: Vec<(f64, f64)> = points.iter().map(|&p| m(p)).collect();
        let mut bounds = Rect {
            x0: f64::MAX,
            y0: f64::MAX,
            x1: f64::MIN,
            y1: f64::MIN,
        };
        for &(x, y) in &points {
            bounds.x0 = bounds.x0.min(x - half);
            bounds.y0 = bounds.y0.min(y - half);
            bounds.x1 = bounds.x1.max(x + half);
            bounds.y1 = bounds.y1.max(y + half);
        }
        Self {
            way: Way::new(&points, half, id),
            bounds,
            hedges,
        }
    }
}

/// Points (local), half width, terrain id and whether hedges line it.
type LaneSpec = (Vec<(f64, f64)>, f64, u8, bool);

/// The ways outside the street grid: suburb ring, bypass, tracks to the
/// starts and farms, field tracks. South half given, north half mirrored.
fn lanes() -> Vec<Lane> {
    let mut lanes = vec![
        Lane::new(&ring(SUBURB_EDGE, 400.0), SUBURB_HALF, ROAD, false),
        Lane::new(&ring(BYPASS, BYPASS_CORNER), BYPASS_HALF, ROAD, false),
    ];
    let edge = HALF - 4.0;
    let (s0, s2) = ((STARTS[0].2, STARTS[0].3), (STARTS[2].2, STARTS[2].3));
    let farm_n = FARM.1 - 32.0;
    let farm_s = FARM.1 + 32.0;
    let south: Vec<LaneSpec> = vec![
        // tracks from the starts to the bypass and between the starts
        (vec![s0, (s0.0, BYPASS + 4.0)], 3.0, DIRT_TRACK, false),
        (vec![s2, (s2.0, BYPASS + 4.0)], 3.0, DIRT_TRACK, false),
        (vec![s0, s2], 3.0, DIRT_TRACK, false),
        // the farm with the supply point: from the suburb to the bypass
        (
            vec![(FARM.0, SUBURB_EDGE + 4.0), (FARM.0, farm_n)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![(FARM.0, farm_s), (FARM.0, BYPASS - 4.0)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        // the other farms
        (
            vec![
                (-BYPASS - 4.0, 600.0),
                (FARMS_OTHER[0].0, 600.0),
                (FARMS_OTHER[0].0, FARMS_OTHER[0].1 - 32.0),
            ],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![
                (825.0, 825.0),
                (FARMS_OTHER[1].0, 1000.0),
                (FARMS_OTHER[1].0, FARMS_OTHER[1].1 - 32.0),
            ],
            3.0,
            DIRT_TRACK,
            true,
        ),
        // field tracks between the suburb and the bypass
        (
            vec![
                (-400.0, SUBURB_EDGE + 4.0),
                (-380.0, 800.0),
                (-420.0, BYPASS - 4.0),
            ],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![(450.0, SUBURB_EDGE + 4.0), (470.0, BYPASS - 4.0)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        // field tracks
        (
            vec![(-BYPASS - 4.0, 300.0), (-1200.0, 340.0), (-edge, 320.0)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![(-450.0, BYPASS + 4.0), (-480.0, 1220.0), (-430.0, edge)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![(600.0, BYPASS + 4.0), (640.0, 1240.0), (700.0, edge)],
            3.0,
            DIRT_TRACK,
            true,
        ),
        (
            vec![(BYPASS + 4.0, 450.0), (1220.0, 520.0), (edge, 500.0)],
            3.0,
            DIRT_TRACK,
            true,
        ),
    ];
    for (points, half, id, hedges) in south {
        let mirrored: Vec<(f64, f64)> = points.iter().map(|&p| mirror(p)).collect();
        lanes.push(Lane::new(&points, half, id, hedges));
        lanes.push(Lane::new(&mirrored, half, id, hedges));
    }
    lanes
}

/// Ground plan: relief, flat places, lanes, snow drifts or mud.
struct Plan {
    bumps: Vec<(f64, f64, f64, f64)>,
    ground: crate::gen_land::Noise,
    detail: crate::gen_land::Noise,
    patches: crate::gen_land::Noise,
    /// Flat places (local center, radius) at their natural height.
    flats: Vec<((f64, f64), f64, f64)>,
    city_level: f64,
    lanes: Vec<Lane>,
    winter: bool,
}

impl Plan {
    fn natural(&self, u: f64, v: f64) -> f64 {
        let mut h = 6.0 * self.ground.at(u, v) + 0.8 * self.detail.at(u, v);
        for &(bx, by, r, a) in &self.bumps {
            let d2 = ((u - bx) * (u - bx) + (v - by) * (v - by)) / (r * r);
            if d2 < 1.0 {
                let t = 1.0 - d2;
                h += a * t * t;
            }
        }
        h
    }

    /// Height and terrain at the local point.
    fn sample(&self, u: f64, v: f64) -> (f64, u8) {
        let mut h = self.natural(u, v);
        let r = (u * u + v * v).sqrt();
        let blend = smoothstep(FLAT_RADIUS, FLAT_RADIUS + FLAT_BLEND, r);
        h = self.city_level + (h - self.city_level) * blend;
        for &(c, radius, level) in &self.flats {
            let blend = smoothstep(radius, radius + 40.0, dist((u, v), c));
            h = level + (h - level) * blend;
        }
        if grid_road(u, v) {
            return (h, ROAD);
        }
        let (x, y) = m((u, v));
        for lane in &self.lanes {
            let b = &lane.bounds;
            if x < b.x0 || x > b.x1 || y < b.y0 || y > b.y1 {
                continue;
            }
            if lane.way.distance(x, y) < lane.way.half {
                return (h, lane.way.id);
            }
        }
        // Park paths: a cross through each park.
        for p in [PARK, mirror(PARK)] {
            let (du, dv) = ((u - p.0).abs(), (v - p.1).abs());
            if (du <= 2.0 && dv <= 52.0) || (dv <= 2.0 && du <= 52.0) {
                return (h, DIRT_TRACK);
            }
        }
        // Snow drifts in the open land (winter), else a few mud patches.
        let open = r > FLAT_RADIUS + 40.0
            && STARTS
                .iter()
                .all(|s| dist((u, v), (s.2, s.3)) > START_CLEAR_M + 30.0)
            && supply_points().iter().all(|(_, c)| dist((u, v), *c) > 50.0);
        if open {
            let n = self.patches.at(u, v);
            if self.winter && n > 0.74 {
                return (h, SNOW);
            }
            if !self.winter && n > 0.86 {
                return (h, MUD);
            }
        }
        (h, GRASS)
    }
}

/// Supply points (name, local position): two on the square (it counts
/// twice), then per half the warehouse quarter, a suburb crossing and a
/// farmstead in front of the city.
pub fn supply_points() -> Vec<(String, (f64, f64))> {
    let mut points = vec![
        ("supply_square_south".to_string(), (-20.0, 15.0)),
        ("supply_square_north".to_string(), (20.0, -15.0)),
    ];
    for (name, p) in [
        ("warehouses", WAREHOUSES),
        ("crossing", CROSSING),
        ("farm", FARM),
    ] {
        points.push((format!("supply_{name}_south"), p));
        points.push((format!("supply_{name}_north"), mirror(p)));
    }
    points
}

/// Markers for tools and shots (local positions).
fn markers() -> Vec<(String, (f64, f64))> {
    let mut list = vec![("town_square".to_string(), (0.0, 0.0))];
    for (name, p) in [
        ("park", PARK),
        ("warehouses", WAREHOUSES),
        ("crossing", CROSSING),
        ("farm", FARM),
        ("strongpoint", STRONGPOINT),
        ("block_edge", (-180.0, BLOCK_EDGE)),
        ("suburb", (0.0, 590.0)),
        ("main_street", (0.0, 200.0)),
        ("side_street", (-60.0, 120.0)),
        ("start", (STARTS[0].2, STARTS[0].3)),
    ] {
        list.push((format!("{name}_south"), p));
        list.push((format!("{name}_north"), mirror(p)));
    }
    // A block cell (ring) for the fire benchmark (`just fire-bench`).
    list.push(("fire_start".into(), (-61.0, 179.0)));
    list.push(("bypass_west".into(), (-BYPASS, 0.0)));
    list.push(("bypass_east".into(), (BYPASS, 0.0)));
    list.extend(supply_points());
    list
}

/// Catalog box (length along the heading, width) of the city's buildings.
fn size_of(kind: &str) -> (f64, f64) {
    match kind {
        "env_block_desert" => (24.0, 12.0),
        "env_block_desert_long" => (40.0, 13.0),
        "env_block_town" => (30.0, 13.0),
        "env_town_hall" => (22.0, 13.0),
        "env_church" => (20.0, 10.0),
        "env_warehouse" => (30.0, 16.0),
        "env_house_log" => (9.0, 7.0),
        _ => (10.0, 8.0),
    }
}

const LONG: &str = "env_block_desert_long";
const TOWN: &str = "env_block_town";
const SMALL: &str = "env_block_desert";

/// A building facing a street: the middle of its front (local) and the
/// direction its entrance side (+X) faces.
#[derive(Clone, Copy)]
struct Slot {
    kind: &'static str,
    /// Kind of the mirrored copy (the town hall mirrors the church).
    mirror_kind: &'static str,
    front: (f64, f64),
    facing: (f64, f64),
}

impl Slot {
    fn new(kind: &'static str, front: (f64, f64), facing: (f64, f64)) -> Self {
        Self {
            kind,
            mirror_kind: kind,
            front,
            facing,
        }
    }

    fn mirrored(&self) -> Self {
        Self {
            kind: self.mirror_kind,
            mirror_kind: self.kind,
            front: mirror(self.front),
            facing: (-self.facing.0, -self.facing.1),
        }
    }

    /// Center (local) and heading.
    fn placement(&self) -> ((f64, f64), u16) {
        let (_, width) = size_of(self.kind);
        let c = (
            self.front.0 - self.facing.0 * width / 2.0,
            self.front.1 - self.facing.1 * width / 2.0,
        );
        let heading = match (self.facing.0 as i32, self.facing.1 as i32) {
            (1, 0) => 0,
            (0, 1) => 90,
            (-1, 0) => 180,
            _ => 270,
        };
        (c, heading)
    }
}

/// Inner rectangle of grid cell `(i, j)` (between the street edges, local).
fn cell_rect(i: usize, j: usize) -> Rect {
    Rect {
        x0: GRID[i] + line_half(GRID[i]),
        y0: GRID[j] + line_half(GRID[j]),
        x1: GRID[i + 1] - line_half(GRID[i + 1]),
        y1: GRID[j + 1] - line_half(GRID[j + 1]),
    }
}

/// What a cell of the block quarter holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CellKind {
    /// One block on every side around a courtyard, open corners.
    Ring,
    /// Two rows of two blocks on opposite sides.
    Rows,
    /// Two blocks in an L at one corner, two single ones opposite.
    Angle,
    /// Tenements gable to gable: three on one side, two opposite.
    Terrace,
    /// Terraces of town houses with gardens (the corner cells).
    TownHouses,
    Park,
    Warehouses,
    /// The center cells at the square: the church (mirrored: the town hall)
    /// or the largest blocks.
    CenterChurch,
    CenterBlocks,
}

/// Cells of the south half, rows `j = 3, 4, 5` (v from 0 to 360), columns
/// `i = 0..6` (u from -360 to 360). Cell `(i, j)` mirrors to `(5 - i, 5 - j)`.
const SOUTH_CELLS: [[CellKind; 6]; 3] = {
    use CellKind::*;
    [
        [Rows, Ring, CenterChurch, CenterBlocks, Terrace, Angle],
        [TownHouses, Park, Ring, Angle, Rows, Ring],
        [TownHouses, Terrace, Rows, Ring, Warehouses, TownHouses],
    ]
};

/// Slots of a canonical cell: center `(0, 0)`, half extent `s`, fronts on
/// the street edges; turned later by a multiple of 90°.
fn canonical_slots(kind: CellKind, s: f64, r: &mut Random) -> Vec<Slot> {
    let f = s - SETBACK;
    let north = (0.0, -1.0);
    let south = (0.0, 1.0);
    let west = (-1.0, 0.0);
    let east = (1.0, 0.0);
    let pick = |r: &mut Random| {
        let x = r.unit();
        if x < 0.4 {
            LONG
        } else if x < 0.75 {
            TOWN
        } else {
            SMALL
        }
    };
    let mut slots = Vec::new();
    match kind {
        CellKind::Ring => {
            for facing in [north, south, west, east] {
                let k = pick(r);
                let shift = r.range(-3.0, 3.0);
                let front = if facing.0 == 0.0 {
                    (shift, facing.1 * f)
                } else {
                    (facing.0 * f, shift)
                };
                slots.push(Slot::new(k, front, facing));
            }
            // A small block in the middle of the courtyard (24 m from the
            // others).
            let facing = [north, south, west, east][(r.unit() * 4.0) as usize % 4];
            let w = size_of(SMALL).1 / 2.0;
            slots.push(Slot::new(SMALL, (facing.0 * w, facing.1 * w), facing));
        }
        CellKind::Rows => {
            for facing in [north, south] {
                let first = if r.unit() < 0.6 { LONG } else { TOWN };
                let second = if r.unit() < 0.5 { SMALL } else { TOWN };
                let gap = r.range(20.0, 26.0);
                let (l1, l2) = (size_of(first).0, size_of(second).0);
                let start = -(l1 + gap + l2) / 2.0;
                slots.push(Slot::new(first, (start + l1 / 2.0, facing.1 * f), facing));
                slots.push(Slot::new(
                    second,
                    (start + l1 + gap + l2 / 2.0, facing.1 * f),
                    facing,
                ));
            }
            // A third row in the middle, about 30 m from the others.
            let k = if r.unit() < 0.5 { LONG } else { TOWN };
            let w = size_of(k).1 / 2.0;
            slots.push(Slot::new(k, (r.range(-8.0, 8.0), w), south));
        }
        CellKind::Angle => {
            // The L at the north-west corner: a long block along the north
            // street from the west street's building line, a tenement along
            // the west street below it (12 m apart).
            let depth = SETBACK + 13.0;
            slots.push(Slot::new(LONG, (-s + SETBACK + 20.0, -f), north));
            slots.push(Slot::new(TOWN, (-f, -s + depth + 12.0 + 15.0), west));
            slots.push(Slot::new(TOWN, (f, -s + 8.0 + 15.0), east));
            slots.push(Slot::new(SMALL, (-s + 60.0, f), south));
            slots.push(Slot::new(SMALL, (12.0, 2.0), east));
        }
        CellKind::Terrace => {
            for a in [-30.0, 0.0, 30.0] {
                slots.push(Slot::new(TOWN, (a, -f), north));
            }
            for a in [-15.0, 15.0] {
                slots.push(Slot::new(TOWN, (a, f), south));
            }
            slots.push(Slot::new(SMALL, (0.0, -6.0), north));
        }
        _ => {}
    }
    slots
}

/// Turns a canonical slot by `turns` quarter turns and moves it to `c`.
fn turned(slot: Slot, turns: u32, c: (f64, f64)) -> Slot {
    let rot = |(a, b): (f64, f64)| match turns % 4 {
        0 => (a, b),
        1 => (-b, a),
        2 => (-a, -b),
        _ => (b, -a),
    };
    let front = rot(slot.front);
    Slot {
        front: (c.0 + front.0, c.1 + front.1),
        facing: rot(slot.facing),
        ..slot
    }
}

/// Slots of the two center cells of the south half (local), fixed.
fn center_slots(kind: CellKind) -> Vec<Slot> {
    let (north, south, west, east) = ((0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0));
    match kind {
        CellKind::CenterBlocks => vec![
            Slot::new(LONG, (80.0, 12.0), north),
            Slot::new(LONG, (12.0, 82.0), west),
            Slot::new(TOWN, (60.0, 110.0), south),
            Slot::new(TOWN, (90.0, 110.0), south),
            Slot::new(SMALL, (110.0, 58.0), east),
        ],
        CellKind::CenterChurch => vec![
            Slot {
                kind: "env_church",
                mirror_kind: "env_town_hall",
                front: (-24.0, SQUARE.1 + SETBACK),
                facing: north,
            },
            Slot::new(LONG, (-12.0, 82.0), east),
            Slot::new(LONG, (-70.0, 110.0), south),
            Slot::new(TOWN, (-110.0, 60.0), west),
            Slot::new(SMALL, (-75.0, 12.0), north),
        ],
        _ => Vec::new(),
    }
}

/// Slots of the warehouse quarter (cell rect `c`, local).
fn warehouse_slots(c: &Rect) -> Vec<Slot> {
    let (north, south, east) = ((0.0, -1.0), (0.0, 1.0), (1.0, 0.0));
    let w = "env_warehouse";
    vec![
        Slot::new(w, (c.x0 + 30.0, c.y0 + SETBACK), north),
        Slot::new(w, (c.x0 + 76.0, c.y0 + SETBACK), north),
        Slot::new(w, ((c.x0 + c.x1) / 2.0, c.y1 - SETBACK), south),
        Slot::new(w, (c.x1 - SETBACK, (c.y0 + c.y1) / 2.0), east),
    ]
}

/// The generator's state: the object placer and counters.
struct City<'a> {
    o: Objects<'a>,
}

impl City<'_> {
    fn put(&mut self, kind: &str, variant: &str, p: (f64, f64), heading: u16, margin: f64) -> bool {
        self.o.place(kind, variant, m(p), heading, margin)
    }

    /// Courtyard of a block cell: trees, a shed, firewood and crates, now
    /// and then a stone wall; all inside `inner` (local), away from the
    /// corner openings.
    fn courtyard(&mut self, inner: Rect) {
        let r = &mut self.o.random;
        let trees = 8 + (r.unit() * 9.0) as usize;
        let mut wanted = Vec::new();
        for _ in 0..trees {
            let p = (r.range(inner.x0, inner.x1), r.range(inner.y0, inner.y1));
            let roll = r.unit();
            let (kind, variant) = if roll < 0.5 {
                ("env_tree_deciduous", "Bare")
            } else if roll < 0.75 {
                ("env_tree_deciduous", "OakYoung")
            } else if roll < 0.9 {
                ("env_tree_birch", "Birch")
            } else {
                ("env_tree_conifer", "SpruceYoung")
            };
            let heading = (r.unit() * 360.0) as u16 % 360;
            wanted.push((kind, variant, p, heading, 3.0));
        }
        if r.unit() < 0.5 {
            let p = (r.range(inner.x0, inner.x1), r.range(inner.y0, inner.y1));
            let variant = if r.unit() < 0.2 { "Damaged" } else { "Intact" };
            let heading = if r.unit() < 0.5 { 0 } else { 90 };
            wanted.push(("env_house_log", variant, p, heading, 4.0));
        }
        for _ in 0..(2 + (r.unit() * 3.0) as usize) {
            let p = (r.range(inner.x0, inner.x1), r.range(inner.y0, inner.y1));
            let roll = r.unit();
            let (kind, variant) = if roll < 0.4 {
                ("env_woodpile", "Stack")
            } else if roll < 0.7 {
                ("env_crate", "Stack")
            } else {
                ("env_barrel", "Group")
            };
            let heading = (r.unit() * 4.0) as u16 * 90 % 360;
            wanted.push((kind, variant, p, heading, 1.0));
        }
        let wall = r.unit() < 0.4;
        let along_x = r.unit() < 0.5;
        let segments = 4 + (r.unit() * 5.0) as usize;
        let start = (r.range(inner.x0, inner.x1), r.range(inner.y0, inner.y1));
        for (kind, variant, p, heading, margin) in wanted {
            self.put(kind, variant, p, heading, margin);
        }
        if wall {
            for k in 0..segments {
                let t = k as f64 * 3.0;
                let p = if along_x {
                    (start.0 + t, start.1)
                } else {
                    (start.0, start.1 + t)
                };
                if p.0 > inner.x1 || p.1 > inner.y1 {
                    break;
                }
                let heading = if along_x { 90 } else { 0 };
                self.put("env_wall_stone", "Segment", p, heading, -0.05);
            }
        }
    }

    /// A park: trees on an open, jittered grid, hedges along two sides.
    fn park(&mut self, c: Rect) {
        let mut v = c.y0 + 8.0;
        while v < c.y1 - 6.0 {
            let mut u = c.x0 + 8.0;
            while u < c.x1 - 6.0 {
                let r = &mut self.o.random;
                let p = (u + r.range(-4.0, 4.0), v + r.range(-4.0, 4.0));
                let roll = r.unit();
                let heading = (r.unit() * 360.0) as u16 % 360;
                if roll < 0.55 {
                    let variant = ["Oak", "Beech", "Bare", "Bare"][(roll * 7.3) as usize % 4];
                    self.put("env_tree_deciduous", variant, p, heading, 2.0);
                } else if roll < 0.65 {
                    self.put("env_tree_conifer", "Spruce", p, heading, 2.0);
                }
                u += 13.0;
            }
            v += 13.0;
        }
    }

    /// The warehouse yard: dumps, crates and barrels around the middle.
    fn warehouse_yard(&mut self, c: Rect) {
        let mid = ((c.x0 + c.x1) / 2.0, (c.y0 + c.y1) / 2.0);
        self.put(
            "env_dump_ammo",
            "Intact",
            (mid.0 - 22.0, mid.1 - 8.0),
            0,
            1.0,
        );
        self.put(
            "env_dump_fuel",
            "Intact",
            (mid.0 + 18.0, mid.1 + 14.0),
            0,
            1.0,
        );
        for k in 0..10 {
            let r = &mut self.o.random;
            let p = (mid.0 + r.range(-34.0, 30.0), mid.1 + r.range(-26.0, 26.0));
            let kind = if k % 2 == 0 {
                ("env_crate", "Stack")
            } else {
                ("env_barrel", "Group")
            };
            let heading = (r.unit() * 4.0) as u16 * 90 % 360;
            if dist(p, mid) > 9.0 {
                self.put(kind.0, kind.1, p, heading, 1.0);
            }
        }
    }

    /// Houses with gardens along an axis-aligned street segment (local
    /// points), both sides, only within the suburb radius.
    fn houses(&mut self, a: (f64, f64), b: (f64, f64), half: f64, damaged: f64) {
        let along_x = (b.1 - a.1).abs() < (b.0 - a.0).abs();
        // Clip to the part within SUBURB_RADIUS of the center.
        let across = if along_x { a.1 } else { a.0 };
        if across.abs() >= SUBURB_RADIUS {
            return;
        }
        let reach = (SUBURB_RADIUS * SUBURB_RADIUS - across * across).sqrt();
        let (from, to) = if along_x { (a.0, b.0) } else { (a.1, b.1) };
        let (from, to) = (from.min(to).max(-reach), from.max(to).min(reach));
        if to - from < 20.0 {
            return;
        }
        let (a, b) = if along_x {
            ((from, across), (to, across))
        } else {
            ((across, from), (across, to))
        };
        let kinds: [(&str, f64, f64); 5] = [
            ("env_house_farm", 14.0, 9.0),
            ("env_house_farm_brick", 12.0, 8.0),
            ("env_house_stone", 10.0, 8.0),
            ("env_house_log", 9.0, 7.0),
            ("env_house_farm_brick", 12.0, 8.0),
        ];
        let garden = Garden {
            tree_kind: "env_tree_deciduous",
            trees: &["Bare", "Bare", "OakYoung"],
            fence: "env_fence_wood",
        };
        for side in [-1.0, 1.0] {
            self.o
                .street((m(a), m(b)), side, half, &kinds, damaged, &garden);
        }
    }
}

/// Damage stages for `n` buildings: about every eighth damaged and every
/// twentieth a ruin, at random places.
fn damage_stages(n: usize, r: &mut Random) -> Vec<&'static str> {
    let ruins = (n + 10) / 20;
    let damaged = (n + 4) / 8;
    let mut stages = vec!["Intact"; n];
    for (k, stage) in stages.iter_mut().enumerate() {
        if k < ruins {
            *stage = "Ruin";
        } else if k < ruins + damaged {
            *stage = "Damaged";
        }
    }
    for k in (1..n).rev() {
        let j = (r.unit() * (k + 1) as f64) as usize;
        stages.swap(k, j.min(k));
    }
    stages
}

/// Writes the city map into `options.out`.
pub fn generate(options: &Options) -> Result<String, String> {
    let terrain_rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let object_rules = ObjectRules::builtin().map_err(|e| e.0)?;
    let seed = options.seed;
    let winter = options.biome == "winter";
    let mut random = Random(Rng::new(seed ^ 0x6369_7479));
    let bumps = (0..36)
        .map(|_| {
            (
                random.range(-HALF, HALF),
                random.range(-HALF, HALF),
                random.range(120.0, 320.0),
                random.range(-3.0, 9.0),
            )
        })
        .collect();
    let mut plan = Plan {
        bumps,
        ground: noise(seed, 1, 320.0),
        detail: noise(seed, 2, 60.0),
        patches: noise(seed, 4, 70.0),
        flats: Vec::new(),
        city_level: 0.0,
        lanes: lanes(),
        winter,
    };
    plan.city_level = plan.natural(0.0, 0.0);
    let mut flat_places: Vec<((f64, f64), f64)> = Vec::new();
    for s in STARTS {
        flat_places.push(((s.2, s.3), START_CLEAR_M));
    }
    for p in [FARM, FARMS_OTHER[0], FARMS_OTHER[1]] {
        flat_places.push((p, 48.0));
        flat_places.push((mirror(p), 48.0));
    }
    flat_places.push((STRONGPOINT, 36.0));
    flat_places.push((mirror(STRONGPOINT), 36.0));
    plan.flats = flat_places
        .iter()
        .map(|&(c, r)| (c, r, plan.natural(c.0, c.1)))
        .collect();

    let mut heights = Vec::with_capacity(SAMPLES * SAMPLES);
    let mut terrain = Vec::with_capacity(SAMPLES * SAMPLES);
    for j in 0..SAMPLES {
        for i in 0..SAMPLES {
            let x = i as f64 * CELL_M - HALF;
            let y = j as f64 * CELL_M - HALF;
            let (h, t) = plan.sample(x - CENTER.0, y - CENTER.1);
            heights.push(h);
            terrain.push(t);
        }
    }

    let mut city = City {
        o: Objects::sized(
            &object_rules,
            &terrain,
            Random(Rng::new(seed ^ 0x626c_6f63)),
            SIZE_M,
        ),
    };
    // Kept free: starts, supply flags, strongpoints.
    for s in STARTS {
        city.o.reserved.push((m((s.2, s.3)), START_CLEAR_M - 5.0));
    }
    for (_, p) in supply_points() {
        city.o.reserved.push((m(p), 6.0));
    }
    for p in [STRONGPOINT, mirror(STRONGPOINT)] {
        city.o.reserved.push((m(p), STRONGPOINT_CLEAR_M));
    }

    // Buildings of the block quarter: slots of the south half and their
    // mirrors, with damage stages drawn per half.
    let mut layout = Random(Rng::new(seed ^ 0x6c61_796f));
    let mut south_slots: Vec<Slot> = Vec::new();
    let mut courtyards: Vec<Rect> = Vec::new();
    let mut townhouse_cells: Vec<(usize, usize)> = Vec::new();
    let mut parks: Vec<Rect> = Vec::new();
    let mut yards: Vec<Rect> = Vec::new();
    for (row, kinds) in SOUTH_CELLS.iter().enumerate() {
        for (i, &kind) in kinds.iter().enumerate() {
            let j = 3 + row;
            let c = cell_rect(i, j);
            let mc = cell_rect(5 - i, 5 - j);
            let slots = match kind {
                CellKind::Ring | CellKind::Rows | CellKind::Angle | CellKind::Terrace => {
                    let s = (c.x1 - c.x0).min(c.y1 - c.y0) / 2.0;
                    let mid = ((c.x0 + c.x1) / 2.0, (c.y0 + c.y1) / 2.0);
                    let turns = (layout.unit() * 4.0) as u32;
                    canonical_slots(kind, s, &mut layout)
                        .into_iter()
                        .map(|slot| turned(slot, turns, mid))
                        .collect()
                }
                CellKind::CenterBlocks | CellKind::CenterChurch => center_slots(kind),
                CellKind::Warehouses => warehouse_slots(&c),
                CellKind::TownHouses | CellKind::Park => Vec::new(),
            };
            south_slots.extend(slots);
            if kind == CellKind::TownHouses {
                townhouse_cells.push((i, j));
                townhouse_cells.push((5 - i, 5 - j));
            }
            for rect in [c, mc] {
                match kind {
                    CellKind::TownHouses => {}
                    CellKind::Park => parks.push(rect),
                    CellKind::Warehouses => yards.push(rect),
                    _ => courtyards.push(Rect {
                        x0: rect.x0 + 20.0,
                        y0: rect.y0 + 20.0,
                        x1: rect.x1 - 20.0,
                        y1: rect.y1 - 20.0,
                    }),
                }
            }
        }
    }
    let south_stages = damage_stages(south_slots.len(), &mut layout);
    let north_stages = damage_stages(south_slots.len(), &mut layout);
    let mut missing = Vec::new();
    for (k, slot) in south_slots.iter().enumerate() {
        for (s, stage) in [(*slot, south_stages[k]), (slot.mirrored(), north_stages[k])] {
            let (c, heading) = s.placement();
            if !city.put(s.kind, stage, c, heading, -0.01) {
                missing.push(format!("{} at {:?}", s.kind, m(c)));
            }
        }
    }
    if !missing.is_empty() {
        return Err(format!("buildings did not fit: {}", missing.join(", ")));
    }
    for rect in yards {
        city.warehouse_yard(rect);
    }
    for rect in parks {
        city.park(rect);
    }
    for rect in courtyards {
        city.courtyard(rect);
    }
    // Trees along the main streets of the block quarter, every 16 m on
    // both sides, 1.8 m from the street edge.
    for g in ROUTES {
        for side in [-1.0, 1.0] {
            let offset = g + side * (MAIN_HALF + 1.8);
            let mut t = -BLOCK_EDGE + 12.0;
            while t < BLOCK_EDGE - 8.0 {
                for p in [(offset, t), (t, offset)] {
                    let r = &mut city.o.random;
                    let variant = if r.unit() < 0.7 { "Bare" } else { "OakYoung" };
                    let heading = (r.unit() * 360.0) as u16 % 360;
                    city.put("env_tree_deciduous", variant, p, heading, 1.0);
                }
                t += 16.0;
            }
        }
    }
    // Town houses in the corner cells: terraces along all four streets.
    let town: [(&str, f64, f64); 3] = [
        ("env_house_town", 12.0, 10.0),
        ("env_house_town", 12.0, 10.0),
        ("env_house_farm_brick", 12.0, 8.0),
    ];
    let town_garden = Garden {
        tree_kind: "env_tree_deciduous",
        trees: &["Bare", "OakYoung"],
        fence: "env_fence_wood",
    };
    for (i, j) in townhouse_cells {
        let c = cell_rect(i, j);
        // (street axis, along x, side of the houses, from, to)
        let streets = [
            (GRID[j], true, 1.0, c.x0, c.x1),
            (GRID[j + 1], true, -1.0, c.x0, c.x1),
            (GRID[i], false, 1.0, c.y0, c.y1),
            (GRID[i + 1], false, -1.0, c.y0, c.y1),
        ];
        for (axis, along_x, side, from, to) in streets {
            let (pa, pb) = if along_x {
                ((from + 6.0, axis), (to - 6.0, axis))
            } else {
                ((axis, from + 6.0), (axis, to - 6.0))
            };
            city.o.street(
                (m(pa), m(pb)),
                side,
                line_half(axis),
                &town,
                0.15,
                &town_garden,
            );
        }
    }

    // Suburb: houses along the continued grid lines, the inner ring and the
    // straight parts of the outer ring, and facing the block quarter.
    let inner = BLOCK_EDGE + SIDE_HALF + 2.0;
    for g in GRID {
        let half = if ROUTES.contains(&g) {
            MAIN_HALF
        } else {
            SUBURB_HALF
        };
        for sign in [-1.0, 1.0] {
            let (a, b) = (sign * inner, sign * (SUBURB_EDGE - 4.0));
            city.houses((g, a), (g, b), half, 0.1);
            city.houses((a, g), (b, g), half, 0.1);
        }
    }
    for sign in [-1.0, 1.0] {
        let r = SUBURB_RING;
        city.houses((-r, sign * r), (r, sign * r), SUBURB_HALF, 0.1);
        city.houses((sign * r, -r), (sign * r, r), SUBURB_HALF, 0.1);
        let e = SUBURB_EDGE;
        city.houses((-400.0, sign * e), (400.0, sign * e), SUBURB_HALF, 0.1);
        city.houses((sign * e, -400.0), (sign * e, 400.0), SUBURB_HALF, 0.1);
    }
    // Houses outside the block quarter's edge streets, facing them.
    for sign in [-1.0, 1.0] {
        let e = BLOCK_EDGE;
        let garden = Garden {
            tree_kind: "env_tree_deciduous",
            trees: &["Bare", "OakYoung"],
            fence: "env_fence_wood",
        };
        let kinds: [(&str, f64, f64); 3] = [
            ("env_house_farm_brick", 12.0, 8.0),
            ("env_house_stone", 10.0, 8.0),
            ("env_house_town", 12.0, 10.0),
        ];
        city.o.street(
            (m((-e, sign * e)), m((e, sign * e))),
            sign,
            SIDE_HALF,
            &kinds,
            0.1,
            &garden,
        );
        city.o.street(
            (m((sign * e, -e)), m((sign * e, e))),
            sign,
            SIDE_HALF,
            &kinds,
            0.1,
            &garden,
        );
    }
    // Orchards and allotments between the suburb streets.
    let orchard_noise = noise(seed, 8, 60.0);
    let stand_noise = noise(seed, 6, 70.0);
    let on = &orchard_noise;
    let orchards: [(&str, &[&str]); 1] = [("env_tree_deciduous", &["OakYoung", "Bare", "Bare"])];
    city.o.wood(
        &orchards,
        9.0,
        Rect {
            x0: -SUBURB_EDGE,
            y0: -SUBURB_EDGE,
            x1: SUBURB_EDGE,
            y1: SUBURB_EDGE,
        },
        &|x, y| {
            let (u, v) = (x - CENTER.0, y - CENTER.1);
            u.abs().max(v.abs()) > BLOCK_EDGE + 20.0
                && (u * u + v * v).sqrt() < SUBURB_RADIUS
                && on.at(u, v) > 0.66
        },
        &stand_noise,
    );

    // Land: farmsteads, dumps at the starts, hedgerows, poles, woods.
    let garden = Garden {
        tree_kind: "env_tree_deciduous",
        trees: &["Bare", "Bare", "OakYoung"],
        fence: "env_fence_wood",
    };
    for (k, p) in [FARM, FARMS_OTHER[0], FARMS_OTHER[1]]
        .into_iter()
        .enumerate()
    {
        for (n, c) in [p, mirror(p)].into_iter().enumerate() {
            let house = ["env_house_farm", "env_house_farm_brick", "env_house_stone"][(k + n) % 3];
            city.o.farmstead(m(c), &garden, house, (k + n) % 2 == 1);
            for dx in [-30.0, -10.0, 10.0, 30.0] {
                let hay = (c.0 + dx, c.1 + if c.1 > 0.0 { 75.0 } else { -75.0 });
                let r = &mut city.o.random;
                let heading = (r.unit() * 360.0) as u16 % 360;
                city.put("env_hay_bale", "Rick", hay, heading, 2.0);
            }
        }
    }
    for p in [(-420.0, 800.0), (520.0, 770.0), (100.0, 830.0)] {
        for c in [p, mirror(p)] {
            city.o.hay_field(m(c), 5);
        }
    }
    for s in STARTS {
        // A supply dump in front of each start (towards the city), the
        // ammunition and fuel 14 m apart.
        let toward = if s.3 > 0.0 { -1.0 } else { 1.0 };
        let c = (s.2 + toward * -35.0, s.3 + toward * 85.0);
        city.put("env_dump_ammo", "Intact", (c.0 - 14.0, c.1), 0, 1.0);
        city.put("env_dump_ammo", "Intact", c, 0, 1.0);
        city.put("env_dump_fuel", "Intact", (c.0 + 14.0, c.1), 0, 1.0);
        city.put("env_dump_fuel", "Intact", (c.0 + 14.0, c.1 + 14.0), 0, 1.0);
    }
    for lane in &plan.lanes {
        if lane.hedges {
            city.o.hedgerows(&lane.way, 5.5);
        }
    }
    let edge = HALF - 4.0;
    for (a, b) in [
        ((0.0, SUBURB_EDGE + 8.0), (0.0, edge)),
        ((0.0, -SUBURB_EDGE - 8.0), (0.0, -edge)),
        ((SUBURB_EDGE + 8.0, 0.0), (edge, 0.0)),
        ((-SUBURB_EDGE - 8.0, 0.0), (-edge, 0.0)),
    ] {
        city.o
            .poles(&Way::new(&[m(a), m(b)], COUNTRY_HALF, ROAD), 9.0);
    }
    let wood_noise = noise(seed, 5, 160.0);
    let wn = &wood_noise;
    let farms: Vec<(f64, f64)> = [FARM, FARMS_OTHER[0], FARMS_OTHER[1]]
        .into_iter()
        .flat_map(|p| [p, mirror(p)])
        .collect();
    let away_from_farms = |u: f64, v: f64| farms.iter().all(|&c| dist((u, v), c) > 80.0);
    let stands: [(&str, &[&str]); 4] = [
        (
            "env_tree_conifer",
            &["Spruce", "Spruce", "Pine", "SpruceYoung"],
        ),
        ("env_tree_conifer", &["Pine", "Pine", "Spruce"]),
        ("env_tree_birch", &["Birch", "BirchYoung"]),
        ("env_tree_deciduous", &["Bare", "Bare", "Oak"]),
    ];
    let full = Rect {
        x0: -HALF + 4.0,
        y0: -HALF + 4.0,
        x1: HALF - 4.0,
        y1: HALF - 4.0,
    };
    city.o.wood(
        &stands,
        6.5,
        full,
        &|x, y| {
            let (u, v) = (x - CENTER.0, y - CENTER.1);
            let r = (u * u + v * v).sqrt();
            ((r > 1020.0 && wn.at(u, v) > 0.63) || (r > 760.0 && wn.at(u, v) > 0.71))
                && away_from_farms(u, v)
        },
        &stand_noise,
    );
    let singles: [(&str, &[&str]); 2] = [
        ("env_tree_deciduous", &["Bare"]),
        ("env_tree_conifer", &["Spruce", "SpruceYoung"]),
    ];
    city.o.wood(
        &singles,
        42.0,
        full,
        &|x, y| {
            let (u, v) = (x - CENTER.0, y - CENTER.1);
            (u * u + v * v).sqrt() > 740.0 && wn.at(u, v) > 0.45 && away_from_farms(u, v)
        },
        &stand_noise,
    );

    // Count what was placed.
    let mut counts = Counts::default();
    for row in &city.o.rows {
        counts.add(row.split(',').next().unwrap_or(""));
    }

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
    let mut rows = vec![
        "# Generated by `cli gen-city` (prompt 079, docs/MAPS.md). Rules: data/objects.toml."
            .to_string(),
        CSV_HEADER.to_string(),
    ];
    rows.append(&mut city.o.rows);
    let mut csv = rows.join("\n");
    csv.push('\n');
    std::fs::write(options.out.join("objects.csv"), csv).map_err(|e| e.to_string())?;

    let name = options.name;
    let mut toml = String::new();
    let _ = write!(
        toml,
        "# Generated by `cli gen-city` (prompt 079); regenerate with `just gen-city` (docs/MAPS.md).\n\
         name = \"{name}\"\n\
         generator = \"cli gen-city --name {name} --seed {seed} --biome {}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = 1\n\
         biome = \"{}\"\n\
         water_flow_deg = 180\n",
        options.biome, options.biome,
    );
    if winter {
        let _ = writeln!(toml, "weather = \"snow\"");
    }
    for (player, team, u, v) in STARTS {
        let (x, y) = m((u, v));
        let _ = write!(
            toml,
            "\n[[start]]\nplayer = {player}\nteam = {team}\nposition_m = [{x}, {y}]\n"
        );
    }
    let mut all_markers = markers();
    all_markers.push(("strongpoint_south".into(), STRONGPOINT));
    all_markers.push(("strongpoint_north".into(), mirror(STRONGPOINT)));
    for (marker, p) in &all_markers {
        let (x, y) = m(*p);
        let _ = write!(
            toml,
            "\n[[marker]]\nname = \"{marker}\"\nposition_m = [{}, {}]\n",
            x.round() as i32,
            y.round() as i32
        );
    }
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;
    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, {} objects ({} blocks, {} other buildings, {} houses, {} trees, {} other)",
        options.out.display(),
        counts.total(),
        counts.blocks,
        counts.buildings,
        counts.houses,
        counts.trees,
        counts.other,
    ))
}

#[derive(Default)]
struct Counts {
    blocks: usize,
    buildings: usize,
    houses: usize,
    trees: usize,
    other: usize,
}

impl Counts {
    fn add(&mut self, kind: &str) {
        if kind.starts_with("env_block") {
            self.blocks += 1;
        } else if kind.starts_with("env_house") {
            self.houses += 1;
        } else if matches!(
            kind,
            "env_church" | "env_town_hall" | "env_warehouse" | "env_barn"
        ) {
            self.buildings += 1;
        } else if kind.starts_with("env_tree") {
            self.trees += 1;
        } else {
            self.other += 1;
        }
    }

    fn total(&self) -> usize {
        self.blocks + self.buildings + self.houses + self.trees + self.other
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BYPASS, CENTER, GRID, Options, ROUTES, SAMPLES, STARTS, cell_rect, generate, m, mirror,
        supply_points,
    };
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Map, Scale, TerrainRules, Vec2, unit_types_from_catalog};
    use std::collections::{BTreeSet, VecDeque};

    /// Cells reachable from `from` for `class`, restricted to `allowed`.
    fn reach(
        grid: &PathGrid,
        class: SizeClass,
        from: (i32, i32),
        allowed: &dyn Fn(f64, f64) -> bool,
    ) -> BTreeSet<(u32, u32)> {
        let start = grid.cell_of(Vec2::from_meters(from.0, from.1));
        let (mx, my) = grid.cell_of(Vec2::from_meters(1499, 1499));
        let mut seen = BTreeSet::from([start]);
        let mut todo = VecDeque::from([start]);
        while let Some((x, y)) = todo.pop_front() {
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0 || ny < 0 || nx > i64::from(mx) || ny > i64::from(my) {
                    continue;
                }
                let c = (nx as u32, ny as u32);
                // Cell centers lie at -1499 + 4 k m.
                let (cx, cy) = (nx as f64 * 4.0 - 1499.0, ny as f64 * 4.0 - 1499.0);
                if !seen.contains(&c) && grid.passable(class, c.0, c.1) && allowed(cx, cy) {
                    seen.insert(c);
                    todo.push_back(c);
                }
            }
        }
        seen
    }

    /// The city generates bit for bit the same twice, has its blocks,
    /// houses, places and supply points, is point-symmetric in its streets,
    /// starts and supply points, and has five independent vehicle routes
    /// from south to north (three through the city, two bypasses).
    #[test]
    fn city_map_is_reproducible_symmetric_and_passable() {
        let dir = std::env::temp_dir().join(format!("mas-gen-city-{}", std::process::id()));
        let options = Options {
            name: "city_large",
            seed: 79,
            biome: "winter",
            out: &dir,
        };
        let report = generate(&options).unwrap();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let first = (read("objects.csv"), read("height.png"), read("terrain.png"));
        generate(&options).unwrap();
        assert!(first.0 == read("objects.csv"), "objects differ");
        assert!(first.1 == read("height.png"), "heights differ");
        assert!(first.2 == read("terrain.png"), "terrain differs");
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = Map::load(&toml, &first.1, &first.2).unwrap();
        map.add_objects(&String::from_utf8(first.0).unwrap())
            .unwrap();
        // Opened and saved by the map editor, the map stays the same map.
        let saved = dir.join("editor");
        let mut edit = editor::EditMap::load_dir(&dir).unwrap();
        edit.save_dir(&saved).unwrap();
        let reread = |f: &str| std::fs::read(saved.join(f)).unwrap();
        let mut again = Map::load(
            &String::from_utf8(reread("map.toml")).unwrap(),
            &reread("height.png"),
            &reread("terrain.png"),
        )
        .unwrap();
        again
            .add_objects(&String::from_utf8(reread("objects.csv")).unwrap())
            .unwrap();
        assert_eq!(
            again.reference().hash,
            map.reference().hash,
            "editor round trip"
        );
        assert_eq!(again.markers().len(), map.markers().len());
        let _ = std::fs::remove_dir_all(&dir);

        let count = |prefix: &str| {
            map.objects()
                .iter()
                .filter(|o| {
                    map.object_kinds()[usize::from(o.kind)]
                        .id
                        .starts_with(prefix)
                })
                .count()
        };
        assert!(count("env_block") >= 100, "{report}");
        assert!(count("env_house") >= 300, "{report}");
        assert_eq!(count("env_church"), 1, "{report}");
        assert_eq!(count("env_town_hall"), 1, "{report}");
        assert_eq!(count("env_warehouse"), 8, "{report}");
        // Damage per half the same: the city buildings north and south of
        // the center.
        for stage in ["Damaged", "Ruin"] {
            let half = |south: bool| {
                map.objects()
                    .iter()
                    .filter(|o| {
                        let k = &map.object_kinds()[usize::from(o.kind)];
                        (k.id.starts_with("env_block")
                            || matches!(
                                k.id.as_str(),
                                "env_church" | "env_town_hall" | "env_warehouse"
                            ))
                            && k.variant == stage
                            && (o.position.y.to_num::<f64>() > CENTER.1) == south
                    })
                    .count()
            };
            assert_eq!(half(true), half(false), "{stage} city buildings per half");
            assert!(half(true) > 0, "{stage} blocks");
        }

        // Places and supply points.
        let names: Vec<&str> = map.markers().iter().map(|m| m.name.as_str()).collect();
        for place in [
            "town_square",
            "park_south",
            "park_north",
            "strongpoint_south",
        ] {
            assert!(names.contains(&place), "marker {place}");
        }
        let supply = names.iter().filter(|n| n.starts_with("supply")).count();
        assert_eq!(supply, 8);

        // Symmetry of starts, supply points and the street network.
        let starts = map.starts();
        assert_eq!(starts.len(), 4);
        for (a, b) in [(0, 1), (2, 3)] {
            let (pa, pb) = (STARTS[a], STARTS[b]);
            assert_eq!(mirror((pa.2, pa.3)), (pb.2, pb.3));
        }
        let points = supply_points();
        for (_, p) in &points {
            assert!(points.iter().any(|(_, q)| *q == mirror(*p)), "{p:?}");
        }
        let ways = |t: u8| t == 1 || t == 2;
        for j in 1..SAMPLES {
            for i in 1..SAMPLES {
                let p = Vec2::from_meters(i as i32 * 2 - 1500, j as i32 * 2 - 1500);
                let q = Vec2::from_meters(1502 - i as i32 * 2, 1502 - j as i32 * 2);
                let (tp, tq) = (map.ground_terrain_at(p), map.ground_terrain_at(q));
                assert_eq!(
                    ways(tp),
                    ways(tq),
                    "street network not symmetric at ({}, {})",
                    i as i32 * 2 - 1500,
                    j as i32 * 2 - 1500
                );
            }
        }

        // Routes: from the south start, tanks reach the north start along
        // each main street through the city and each bypass alone.
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types =
            unit_types_from_catalog(&catalog, &scale, &["de_inf_rifle_squad", "de_pz4_h"]).unwrap();
        let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
        let s0 = m((STARTS[0].2, STARTS[0].3));
        let s1 = m((STARTS[1].2, STARTS[1].3));
        let goal = grid.cell_of(Vec2::from_meters(s1.0 as i32, s1.1 as i32));
        let from = (s0.0 as i32, s0.1 as i32);
        let corridors = [-BYPASS, ROUTES[0], ROUTES[1], ROUTES[2], BYPASS];
        for line in corridors {
            // Across the middle band only the corridor of this route is open.
            let allowed = |x: f64, y: f64| {
                let (u, v) = (x - CENTER.0, y - CENTER.1);
                v.abs() > 700.0 || (u - line).abs() <= 14.0
            };
            let seen = reach(&grid, SizeClass::Large, from, &allowed);
            assert!(seen.contains(&goal), "no tank route along u = {line}");
        }
        // Infantry reaches every courtyard of the block quarter.
        let all = |_: f64, _: f64| true;
        let infantry = reach(&grid, SizeClass::Small, from, &all);
        let tanks = reach(&grid, SizeClass::Large, from, &all);
        for i in 0..6 {
            for j in 0..6 {
                let c = cell_rect(i, j);
                let mid = m(((c.x0 + c.x1) / 2.0, (c.y0 + c.y1) / 2.0));
                let near = |set: &BTreeSet<(u32, u32)>| {
                    (-3..=3).any(|dx| {
                        (-3..=3).any(|dy| {
                            let p = Vec2::from_meters(mid.0 as i32 + dx * 4, mid.1 as i32 + dy * 4);
                            set.contains(&grid.cell_of(p))
                        })
                    })
                };
                assert!(
                    near(&infantry),
                    "infantry cannot reach cell ({i}, {j}) at {mid:?}"
                );
            }
        }
        // Every street crossing of the grid is reachable for tanks.
        for gu in GRID {
            for gv in GRID {
                let p = m((gu, gv));
                let cell = grid.cell_of(Vec2::from_meters(p.0 as i32, p.1 as i32));
                assert!(tanks.contains(&cell), "crossing ({gu}, {gv})");
            }
        }
    }
}
