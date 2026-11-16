//! Test map generator: `cli gen-map`.
//!
//! Builds a map from a seed: gentle rolling ground, hills, a river bed with
//! banks, fords and bridges (prompt 014), a flat start meadow around the
//! origin, village squares, field tracks and a road, and writes `map.toml`,
//! `height.png` and `terrain.png` (format: `docs/MAPS.md`). Objects (woods,
//! hedges, villages, bridges, rocks in the river) are placed by [`objects`]
//! into `objects.csv`.
//!
//! The generator runs outside the simulation and may use floats: its output
//! files are the source of truth and are committed. It only uses basic
//! arithmetic and `sqrt` (both exactly rounded in IEEE 754), so the same seed
//! gives the same files on every platform.

use sim::{Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

mod objects;

const CELL_M: f64 = 2.0;

// Terrain ids of data/terrain.toml.
const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const SAND: u8 = 3;
const MUD: u8 = 4;
const SHALLOW_WATER: u8 = 6;
const DEEP_WATER: u8 = 7;
/// Half width of a ford: where a track or road crosses the river, the water
/// is shallow within this distance of it. Wide enough for tanks, which keep
/// a margin from deep water (docs/ARCHITECTURE.md, "Pathfinding").
const FORD_HALF_WIDTH_M: f64 = 7.0;
/// The river's deep channel fades out over this distance beyond the ford.
const FORD_BLEND_M: f64 = 6.0;

// River profile, meters below the natural ground (prompt 014).
/// Banks at the water's edge.
const BANK_CUT_M: f64 = 3.0;
/// Waterline below the banks (the presentation computes the water level
/// from the bank heights).
const WATERLINE_M: f64 = 0.15;
/// Shallow water: down to this depth below the waterline.
const SHALLOW_DEPTH_M: f64 = 0.5;
/// Deep channel: this much deeper than the shallow water.
const CHANNEL_DEPTH_M: f64 = 1.3;

// Bridges (prompt 014).
/// Seed offset of the bridge crossings' random stream ("bridges"), so the
/// rest of the map does not change with them.
const BRIDGE_SALT: u64 = 0x6272_6964_6765_7300;
/// The river is straight within this distance (along y) of a crossing.
const BRIDGE_STRAIGHT_M: f64 = 30.0;
/// The river keeps its narrowed width within this distance of a crossing
/// and returns to its full width at `BRIDGE_TAPER_M`.
const BRIDGE_NARROW_M: f64 = 15.0;
const BRIDGE_TAPER_M: f64 = 60.0;
/// Land under each end of a deck: water plus this on both sides. Keeps the
/// cells beyond the deck clear of the large-unit margin.
const BRIDGE_LAND_M: f64 = 4.0;
/// Rows within this distance of the axis count when measuring the water.
const BRIDGE_MEASURE_M: f64 = 5.0;
/// Grade of the ramps up to the deck, and of the embankment sides.
const RAMP_GRADE: f64 = 0.12;
const EMBANKMENT_SIDE_GRADE: f64 = 0.5;
/// Path cell size: bridge axes lie on cell center lines.
const PATH_CELL_M: f64 = 4.0;

/// A river crossing with a bridge.
pub struct Crossing {
    pub stone: bool,
    /// Axis (east-west, on a path cell center line).
    pub y: f64,
    /// River center at `y`.
    pub x: f64,
    /// Half width of the water at the crossing (narrowed).
    pub half_width: f64,
    /// Running number, picks among equal variants.
    pub index: usize,
}

/// A bridge as placed by [`generate`].
pub struct PlacedBridge {
    pub kind: &'static str,
    pub variant: &'static str,
    pub x: f64,
    pub y: f64,
    pub length: f64,
}

/// Parameters of `cli gen-map`.
pub struct Options<'a> {
    pub name: &'a str,
    pub size_m: u32,
    pub seed: u64,
    /// Number of start positions: 2, 4 or 6 (prompt 037, see
    /// [`start_positions`]).
    pub players: u8,
    /// `biome` in map.toml (presentation only).
    pub biome: &'a str,
    pub out: &'a Path,
}

/// Random numbers in `[0, 1)` from the simulation's generator.
struct Random(Rng);

impl Random {
    fn unit(&mut self) -> f64 {
        (self.0.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.unit()
    }
}

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Value noise: random values on a square lattice, blended with smoothstep.
struct ValueNoise {
    cells: usize,
    spacing: f64,
    origin: f64,
    values: Vec<f64>,
}

impl ValueNoise {
    fn new(random: &mut Random, size: f64, wavelength: f64) -> Self {
        let cells = (size / wavelength).ceil() as usize + 1;
        let values = (0..(cells + 1) * (cells + 1))
            .map(|_| random.range(-1.0, 1.0))
            .collect();
        Self {
            cells,
            spacing: wavelength,
            origin: -size / 2.0,
            values,
        }
    }

    fn at(&self, x: f64, y: f64) -> f64 {
        let gx = ((x - self.origin) / self.spacing).clamp(0.0, self.cells as f64 - 1e-9);
        let gy = ((y - self.origin) / self.spacing).clamp(0.0, self.cells as f64 - 1e-9);
        let (i, j) = (gx.floor() as usize, gy.floor() as usize);
        let (fx, fy) = (
            smoothstep(0.0, 1.0, gx.fract()),
            smoothstep(0.0, 1.0, gy.fract()),
        );
        let v = |i: usize, j: usize| self.values[j * (self.cells + 1) + i];
        let top = v(i, j) + (v(i + 1, j) - v(i, j)) * fx;
        let bottom = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * fx;
        top + (bottom - top) * fy
    }
}

struct Hill {
    x: f64,
    y: f64,
    radius: f64,
    height: f64,
}

impl Hill {
    /// Smooth bump `(1 - d²/r²)²`; its steepest grade is about
    /// `1.54 * height / radius`.
    fn at(&self, x: f64, y: f64) -> f64 {
        let d2 = ((x - self.x) * (x - self.x) + (y - self.y) * (y - self.y))
            / (self.radius * self.radius);
        if d2 >= 1.0 {
            0.0
        } else {
            self.height * (1.0 - d2) * (1.0 - d2)
        }
    }
}

/// A polyline with a width, e.g. a track or the river.
struct Path2 {
    points: Vec<(f64, f64)>,
}

impl Path2 {
    /// Distance from `(x, y)` to the polyline.
    fn distance(&self, x: f64, y: f64) -> f64 {
        let mut best = f64::INFINITY;
        for pair in self.points.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (px, py) = (ax + t * dx - x, ay + t * dy - y);
            best = best.min(px * px + py * py);
        }
        best.sqrt()
    }

    /// Bounding box grown by `margin`, to skip far samples quickly.
    fn near(&self, x: f64, y: f64, margin: f64) -> bool {
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for &(px, py) in &self.points {
            x0 = x0.min(px);
            y0 = y0.min(py);
            x1 = x1.max(px);
            y1 = y1.max(py);
        }
        x >= x0 - margin && x <= x1 + margin && y >= y0 - margin && y <= y1 + margin
    }

    /// Wobbly line from `a` to `b` with `segments` pieces, displaced
    /// sideways by up to `wobble` meters.
    fn wobbly(
        random: &mut Random,
        a: (f64, f64),
        b: (f64, f64),
        segments: usize,
        wobble: f64,
    ) -> Self {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (nx, ny) = (-dy / len, dx / len);
        let points = (0..=segments)
            .map(|k| {
                let t = k as f64 / segments as f64;
                let side = if k == 0 || k == segments {
                    0.0
                } else {
                    random.range(-wobble, wobble)
                };
                (a.0 + dx * t + nx * side, a.1 + dy * t + ny * side)
            })
            .collect();
        Self { points }
    }
}

struct Square {
    x: f64,
    y: f64,
    radius: f64,
}

/// Everything placed on the map, in meters.
struct Layout {
    half: f64,
    meadow_radius: f64,
    hills: Vec<Hill>,
    river: Path2,
    river_width: f64,
    crossings: Vec<Crossing>,
    squares: Vec<Square>,
    tracks: Vec<Path2>,
    /// Tracks over the bridges (no fords).
    bridge_tracks: Vec<Path2>,
    roads: Vec<Path2>,
    noise: Vec<(ValueNoise, f64)>,
    mud: ValueNoise,
}

fn layout(random: &mut Random, size: f64, seed: u64) -> Layout {
    let half = size / 2.0;
    let meadow_radius = (0.38 * half).min(160.0);
    // The river runs roughly north-south on the east side, the landmark
    // hill stands north-west of the meadow, villages lie in the south-west.
    let river_x = 0.6 * half;
    let segments = (size / 60.0).ceil().max(6.0) as usize;
    let river = Path2::wobbly(
        random,
        (river_x, -half - 20.0),
        (river_x, half + 20.0),
        segments,
        0.06 * half,
    );
    let river_width = (size / 80.0).clamp(6.0, 14.0);

    let mut hills = Vec::new();
    // Landmark hill (marker `hill_top`): within reach of the meadow.
    let landmark_radius = (0.36 * half).clamp(90.0, 260.0);
    hills.push(Hill {
        x: -0.42 * half,
        y: -0.56 * half,
        radius: landmark_radius,
        height: landmark_radius * 0.22,
    });
    let extra = ((size / 500.0) * (size / 500.0) * 0.6).round().max(2.0) as usize;
    // Bounded attempts, so a crowded small map cannot loop forever.
    for _ in 0..10_000 {
        if hills.len() > extra {
            break;
        }
        let (x, y) = (random.range(-half, half), random.range(-half, half));
        let radius = random.range(0.04, 0.12) * size.max(1000.0);
        // Keep the meadow and the river line free.
        let landmark = &hills[0];
        let to_landmark =
            ((x - landmark.x) * (x - landmark.x) + (y - landmark.y) * (y - landmark.y)).sqrt();
        if (x * x + y * y).sqrt() < meadow_radius + radius * 0.8
            || (x - river_x).abs() < radius * 0.7 + 0.1 * half
            || to_landmark < radius + landmark.radius
        {
            continue;
        }
        hills.push(Hill {
            x,
            y,
            radius,
            height: radius * random.range(0.06, 0.16),
        });
    }

    let villages = ((size / 1500.0).round() as usize).max(1);
    let mut squares = Vec::new();
    squares.push(Square {
        x: -0.55 * half,
        y: 0.55 * half,
        radius: (size / 25.0).clamp(18.0, 40.0),
    });
    for _ in 0..10_000 {
        if squares.len() >= villages {
            break;
        }
        let (x, y) = (
            random.range(-0.85, 0.85) * half,
            random.range(-0.85, 0.85) * half,
        );
        if (x * x + y * y).sqrt() < meadow_radius * 2.0 || (x - river_x).abs() < 0.25 * half {
            continue;
        }
        squares.push(Square {
            x,
            y,
            radius: random.range(25.0, 40.0),
        });
    }

    let wobble = 0.05 * half;
    let pieces = |a: (f64, f64), b: (f64, f64)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        ((dx * dx + dy * dy).sqrt() / 50.0).ceil().max(2.0) as usize
    };
    let mut tracks = Vec::new();
    let mut track = |random: &mut Random, a: (f64, f64), b: (f64, f64)| {
        tracks.push(Path2::wobbly(random, a, b, pieces(a, b), wobble));
    };
    // Meadow to the north and south edges, to every village, to the hill
    // foot and across the river (a ford) to the east edge.
    track(random, (0.0, 0.0), (0.1 * half, -half));
    track(random, (0.0, 0.0), (-0.1 * half, half));
    track(random, (0.0, 0.0), (half, 0.15 * half));
    let hill = &hills[0];
    track(random, (0.0, 0.0), (hill.x * 0.5, hill.y * 0.5));
    for square in &squares {
        track(random, (0.0, 0.0), (square.x, square.y));
    }
    let first = &squares[0];
    let roads = vec![Path2::wobbly(
        random,
        (-half, first.y + 0.1 * half),
        (first.x, first.y),
        pieces((-half, first.y), (first.x, first.y)),
        wobble * 0.5,
    )];

    let noise = vec![
        (ValueNoise::new(random, size, (size / 3.0).max(150.0)), 3.0),
        (ValueNoise::new(random, size, (size / 10.0).max(60.0)), 1.0),
        (ValueNoise::new(random, size, 24.0), 0.15),
    ];
    let mud = ValueNoise::new(random, size, 40.0);
    let mut layout = Layout {
        half,
        meadow_radius,
        hills,
        river,
        river_width,
        crossings: Vec::new(),
        squares,
        tracks,
        bridge_tracks: Vec::new(),
        roads,
        noise,
        mud,
    };
    add_crossings(&mut layout, seed);
    layout
}

/// River center x at `y` (the river runs north-south, its points sorted
/// by y).
fn river_x_at(river: &Path2, y: f64) -> f64 {
    for pair in river.points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        if (ay..=by).contains(&y) && by > ay {
            return ax + (bx - ax) * (y - ay) / (by - ay);
        }
    }
    river.points.first().map_or(0.0, |p| p.0)
}

/// Bridge crossings: one per started kilometer of river, at least two,
/// alternating stone and timber, spread along the river and away from the
/// ford of the east track. The river runs straight and narrower at each
/// crossing; a field track leads from the edge of the meadow over the
/// bridge to the east edge. Uses its own random stream.
fn add_crossings(layout: &mut Layout, seed: u64) {
    let mut random = Random(Rng::new(seed ^ BRIDGE_SALT));
    let half = layout.half;
    let count = ((2.0 * half / 1000.0).ceil() as usize).max(2);
    let w = layout.river_width;
    for k in 0..count {
        let t = (k as f64 + 0.5) / count as f64;
        let mut y = -0.8 * half + 1.6 * half * t;
        // Away from the ford of the east track (about 0.1 * half).
        if (y - 0.1 * half).abs() < 0.1 * half {
            y = 0.1 * half + 0.1 * half * if y < 0.1 * half { -1.0 } else { 1.0 };
        }
        // Axis on a path cell center line: -half + 1 + 4 k.
        let y = -half + 1.0 + PATH_CELL_M * ((y + half - 1.0) / PATH_CELL_M).round();
        let stone = k % 2 == 0;
        let half_width = if stone {
            if w > 8.0 { 11.0 } else { w.min(5.5) }
        } else {
            w.min(3.5)
        };
        let x = river_x_at(&layout.river, y);
        layout.crossings.push(Crossing {
            stone,
            y,
            x,
            half_width,
            index: k,
        });
    }
    // Straighten the river around every crossing.
    let mut points: Vec<(f64, f64)> = layout
        .river
        .points
        .iter()
        .copied()
        .filter(|&(_, py)| {
            layout
                .crossings
                .iter()
                .all(|c| (py - c.y).abs() >= BRIDGE_STRAIGHT_M)
        })
        .collect();
    for c in &layout.crossings {
        points.push((c.x, c.y - BRIDGE_STRAIGHT_M));
        points.push((c.x, c.y + BRIDGE_STRAIGHT_M));
    }
    points.sort_by(|a, b| a.1.total_cmp(&b.1));
    layout.river.points = points;
    // Tracks over the bridges.
    let wobble = 0.05 * half;
    for c in &layout.crossings {
        let a = (c.x - 55.0, c.y);
        let b = (c.x + 55.0, c.y);
        let (dx, dy) = (a.0, a.1);
        let pieces = ((dx * dx + dy * dy).sqrt() / 50.0).ceil().max(2.0) as usize;
        // From the rim of the start meadow, so the meadow (where the demo
        // battle is fought) stays as it is.
        let len = (a.0 * a.0 + a.1 * a.1).sqrt().max(1.0);
        let rim = layout.meadow_radius + 25.0;
        let start = (a.0 / len * rim, a.1 / len * rim);
        let mut west = Path2::wobbly(&mut random, start, a, pieces, wobble);
        let east_end = (half, c.y + random.range(-0.05, 0.05) * half);
        let pieces = ((half - b.0) / 50.0).ceil().max(2.0) as usize;
        let east = Path2::wobbly(&mut random, b, east_end, pieces, wobble * 0.5);
        west.points.extend(east.points);
        layout.bridge_tracks.push(west);
    }
}

impl Layout {
    /// Half width of the water at `y`: narrowed near bridge crossings.
    fn river_half_width(&self, y: f64) -> f64 {
        let w = self.river_width;
        self.crossings.iter().fold(w, |acc, c| {
            let blend = smoothstep(BRIDGE_NARROW_M, BRIDGE_TAPER_M, (y - c.y).abs());
            acc.min(c.half_width + (w - c.half_width) * blend)
        })
    }

    /// Ground height without flattened areas.
    fn natural_height(&self, x: f64, y: f64) -> f64 {
        let mut h: f64 = self.noise.iter().map(|(n, amp)| n.at(x, y) * amp).sum();
        h += self.hills.iter().map(|hill| hill.at(x, y)).sum::<f64>();
        h
    }

    fn sample(&self, x: f64, y: f64) -> (f64, u8) {
        let mut h = self.natural_height(x, y);
        let mut terrain = GRASS;

        // Flat start meadow at height 0 around the origin.
        let d = (x * x + y * y).sqrt();
        let blend = smoothstep(self.meadow_radius, self.meadow_radius + 40.0, d);
        h *= blend;

        // Flat village squares at their natural center height.
        for square in &self.squares {
            let d = ((x - square.x) * (x - square.x) + (y - square.y) * (y - square.y)).sqrt();
            let level = self.natural_height(square.x, square.y);
            let blend = smoothstep(square.radius, square.radius + 25.0, d);
            h = level + (h - level) * blend;
            if d < square.radius {
                terrain = ROAD;
            }
        }

        let w = self.river_half_width(y);
        let valley = w + 40.0;
        let river = if self.river.near(x, y, valley) {
            self.river.distance(x, y)
        } else {
            f64::INFINITY
        };

        for (paths, width, id) in [
            (&self.roads, 3.0, ROAD),
            (&self.tracks, 1.8, DIRT_TRACK),
            (&self.bridge_tracks, 1.8, DIRT_TRACK),
        ] {
            if terrain == GRASS
                && paths
                    .iter()
                    .any(|p| p.near(x, y, width) && p.distance(x, y) < width)
            {
                terrain = id;
            }
        }

        if river < valley {
            // Gentle banks down to BANK_CUT_M below the ground, then shallow
            // water and a deep channel (none in fords).
            let reach = FORD_HALF_WIDTH_M + FORD_BLEND_M;
            let ford_distance = self
                .tracks
                .iter()
                .chain(&self.roads)
                .filter(|p| p.near(x, y, reach))
                .map(|p| p.distance(x, y))
                .fold(f64::INFINITY, f64::min);
            let ford = ford_distance < FORD_HALF_WIDTH_M;
            let channel = 1.0 - smoothstep(reach, FORD_HALF_WIDTH_M, ford_distance);
            h -= if river < w {
                BANK_CUT_M
                    + WATERLINE_M
                    + SHALLOW_DEPTH_M * smoothstep(w, w * 0.55, river)
                    + CHANNEL_DEPTH_M * channel * smoothstep(w * 0.55, w * 0.25, river)
            } else {
                BANK_CUT_M * (1.0 - smoothstep(w, valley, river))
            };
            terrain = if river < w * 0.55 && !ford {
                DEEP_WATER
            } else if river < w {
                SHALLOW_WATER
            } else if river < w + 4.0 {
                SAND
            } else if river < w + 14.0 && self.mud.at(x, y) > 0.1 && terrain == GRASS {
                MUD
            } else {
                terrain
            };
        } else if terrain == GRASS && self.mud.at(x, y) > 0.82 {
            terrain = MUD;
        }
        (h, terrain)
    }

    /// Highest sample within the landmark hill.
    fn hill_top(&self, heights: &[f64], samples: usize) -> (i32, i32) {
        let hill = &self.hills[0];
        let mut best = (f64::MIN, 0, 0);
        for j in 0..samples {
            for i in 0..samples {
                let (x, y) = (coord(i, self.half), coord(j, self.half));
                let d2 = (x - hill.x) * (x - hill.x) + (y - hill.y) * (y - hill.y);
                if d2 < hill.radius * hill.radius * 0.25 && heights[j * samples + i] > best.0 {
                    best = (heights[j * samples + i], i, j);
                }
            }
        }
        (
            coord(best.1, self.half).round() as i32,
            coord(best.2, self.half).round() as i32,
        )
    }
}

fn coord(index: usize, half: f64) -> f64 {
    -half + index as f64 * CELL_M
}

/// Bridge variants per kind: (variant, length in m); the shortest that
/// spans the water plus `BRIDGE_LAND_M` on both sides is used. Lengths as in
/// `data/objects.toml`.
const WOOD_VARIANTS: [(&str, f64); 3] = [("Span8", 8.0), ("Span12", 12.0), ("Span16", 16.0)];
const STONE_VARIANTS: [[(&str, f64); 2]; 2] = [
    [("Arch1", 20.0), ("Arch3", 30.0)],
    [("Arch2", 20.0), ("Arch3", 30.0)],
];
/// Deck above the bank: timber, stone.
const DECK_RISE_M: [f64; 2] = [1.5, 3.0];
/// Half of the roadway (`data/objects.toml`): timber, stone.
const DECK_HALF_WIDTH_M: [f64; 2] = [2.05, 2.55];

/// Measures the water at every crossing, picks the bridge variant, and
/// builds embankments with ramps up to the deck (heights and track terrain).
fn place_bridges(
    layout: &Layout,
    samples: usize,
    heights: &mut [f64],
    terrain: &mut [u8],
) -> Result<Vec<PlacedBridge>, String> {
    let half = layout.half;
    let index = |x: f64, y: f64| {
        let i = ((x + half) / CELL_M)
            .round()
            .clamp(0.0, (samples - 1) as f64) as usize;
        let j = ((y + half) / CELL_M)
            .round()
            .clamp(0.0, (samples - 1) as f64) as usize;
        j * samples + i
    };
    let mut placed = Vec::new();
    for c in &layout.crossings {
        // Extent of the water near the axis.
        let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
        for j in 0..samples {
            let y = coord(j, half);
            if (y - c.y).abs() > BRIDGE_MEASURE_M {
                continue;
            }
            for i in 0..samples {
                let x = coord(i, half);
                if (x - c.x).abs() < 40.0
                    && matches!(terrain[j * samples + i], SHALLOW_WATER | DEEP_WATER)
                {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                }
            }
        }
        if x0 > x1 {
            return Err(format!("no water at the bridge crossing y = {}", c.y));
        }
        let need = x1 - x0 + 2.0 * BRIDGE_LAND_M;
        let wood: &[(&str, f64)] = &WOOD_VARIANTS;
        let stone: &[(&str, f64)] = &STONE_VARIANTS[c.index / 2 % 2];
        let choice = if c.stone {
            stone.iter().find(|v| v.1 >= need).map(|v| (true, *v))
        } else {
            wood.iter()
                .find(|v| v.1 >= need)
                .map(|v| (false, *v))
                .or_else(|| stone.iter().find(|v| v.1 >= need).map(|v| (true, *v)))
        };
        let Some((is_stone, (variant, length))) = choice else {
            return Err(format!(
                "the river at y = {} is {:.1} m wide, too wide for a bridge",
                c.y,
                x1 - x0
            ));
        };
        let kind = usize::from(is_stone);
        // Center to the millimeter, as in objects.csv.
        let cx = ((x0 + x1) / 2.0 * 1000.0).round() / 1000.0;
        let (d0, d1) = (cx - length / 2.0, cx + length / 2.0);
        let bank = heights[index(d0, c.y)].max(heights[index(d1, c.y)]);
        let deck = bank + DECK_RISE_M[kind];
        let top_half = DECK_HALF_WIDTH_M[kind] + 1.0;
        // Embankment: flat top under the deck ends, ramps beyond them,
        // sloped sides; never in the water.
        let reach_x = DECK_RISE_M[kind] / RAMP_GRADE + 10.0;
        let reach_y = top_half + DECK_RISE_M[kind] / EMBANKMENT_SIDE_GRADE + 4.0;
        for j in 0..samples {
            let y = coord(j, half);
            let dy = (y - c.y).abs();
            if dy > reach_y {
                continue;
            }
            for i in 0..samples {
                let x = coord(i, half);
                if x < d0 - reach_x || x > d1 + reach_x {
                    continue;
                }
                let k = j * samples + i;
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
                    if dy <= 2.0 {
                        terrain[k] = DIRT_TRACK;
                    }
                }
            }
        }
        placed.push(PlacedBridge {
            kind: if is_stone {
                "env_bridge_stone"
            } else {
                "env_bridge_wood"
            },
            variant,
            x: cx,
            y: c.y,
            length,
        });
    }
    Ok(placed)
}

/// Where the track to the east edge (the third track) crosses the river.
fn ford_point(layout: &Layout) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0, 0.0);
    for pair in layout.tracks[2].points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        for k in 0..=50 {
            let t = f64::from(k) / 50.0;
            let (x, y) = (ax + (bx - ax) * t, ay + (by - ay) * t);
            let d = layout.river.distance(x, y);
            if d < best.0 {
                best = (d, x, y);
            }
        }
    }
    (best.1, best.2)
}

/// Start positions `(player, team, x, y)` on the start meadow (prompt
/// 037): two teams, south (even players) and north (odd players), every
/// start mirrored through the center from its opponent's. Two players
/// stand where the demo scenario expects them.
pub fn start_positions(players: u8) -> Vec<(u8, u8, i32, i32)> {
    let south: &[(i32, i32)] = match players {
        0..=2 => &[(0, 60)],
        3 | 4 => &[(-55, 65), (55, 65)],
        _ => &[(0, 60), (-90, 75), (90, 75)],
    };
    let mut out = Vec::new();
    for (k, &(x, y)) in (0u8..).zip(south) {
        out.push((2 * k, 0, x, y));
        out.push((2 * k + 1, 1, -x, -y));
    }
    out
}

/// Generates the map and writes its three files into `options.out`.
pub fn generate(options: &Options) -> Result<String, String> {
    if options.size_m < 100 || !options.size_m.is_multiple_of(4) || options.size_m > 16_000 {
        return Err("--size-m must be a multiple of 4 between 100 and 16000".into());
    }
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let size = f64::from(options.size_m);
    let samples = (options.size_m / 2 + 1) as usize;
    let mut random = Random(Rng::new(options.seed));
    let layout = layout(&mut random, size, options.seed);

    let mut heights = Vec::with_capacity(samples * samples);
    let mut terrain = Vec::with_capacity(samples * samples);
    for j in 0..samples {
        for i in 0..samples {
            let (h, t) = layout.sample(coord(i, layout.half), coord(j, layout.half));
            heights.push(h);
            terrain.push(t);
        }
    }

    let bridges = place_bridges(&layout, samples, &mut heights, &mut terrain)?;

    let low = heights.iter().copied().fold(f64::MAX, f64::min);
    let high = heights.iter().copied().fold(f64::MIN, f64::max);
    // Base: one meter below the lowest point, whole meters. One millimeter
    // per step covers 65 m; larger ranges use coarser steps.
    let base_mm = ((low - 1.0).floor() * 1000.0) as i32;
    let range_mm = (high * 1000.0) as i64 - i64::from(base_mm);
    let step_mm = (range_mm / 65_000 + 1) as u32;
    let encoded: Vec<u8> = heights
        .iter()
        .flat_map(|h| {
            let value = ((h * 1000.0 - f64::from(base_mm)) / f64::from(step_mm)).round();
            (value.clamp(0.0, 65535.0) as u16).to_be_bytes()
        })
        .collect();

    std::fs::create_dir_all(options.out).map_err(|e| e.to_string())?;
    let side = samples as u32;
    write_png(&options.out.join("height.png"), side, &encoded, None)?;
    let palette: Vec<u8> = rules
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

    let (hill_x, hill_y) = layout.hill_top(&heights, samples);
    let (csv, summary) = objects::generate(
        &layout,
        options.seed,
        samples,
        &heights,
        &terrain,
        (f64::from(hill_x), f64::from(hill_y)),
        &bridges,
    );
    std::fs::write(options.out.join("objects.csv"), csv).map_err(|e| e.to_string())?;
    let village = &layout.squares[0];
    let mut bridge_markers = String::new();
    for kind in ["env_bridge_stone", "env_bridge_wood"] {
        if let Some(b) = bridges.iter().find(|b| b.kind == kind) {
            let _ = write!(
                bridge_markers,
                "\n\n[[marker]]\nname = \"{}\"\nposition_m = [{}, {}]",
                kind.trim_start_matches("env_"),
                b.x.round() as i32,
                b.y.round() as i32
            );
        }
    }
    let (ford_x, ford_y) = ford_point(&layout);
    let _ = write!(
        bridge_markers,
        "\n\n[[marker]]\nname = \"ford\"\nposition_m = [{}, {}]",
        ford_x.round() as i32,
        ford_y.round() as i32
    );
    // Supply points (prompt 023, docs/GAME_DESIGN.md): the village, the
    // landmark hill and the ford, worth fighting over.
    for (name, x, y) in [
        (
            "supply_village",
            village.x.round() as i32,
            village.y.round() as i32,
        ),
        ("supply_hill", hill_x, hill_y),
        ("supply_ford", ford_x.round() as i32, ford_y.round() as i32),
    ] {
        let _ = write!(
            bridge_markers,
            "\n\n[[marker]]\nname = \"{name}\"\nposition_m = [{x}, {y}]"
        );
    }
    // More players (prompt 037): neutral points on the start meadow (kept
    // free of objects), as far from one team as from the other.
    let extra_supply: &[(&str, i32, i32)] = match options.players {
        0..=2 => &[],
        3 | 4 => &[
            ("supply_meadow_west", -120, 0),
            ("supply_meadow_east", 120, 0),
        ],
        _ => &[
            ("supply_meadow_west", -130, 0),
            ("supply_meadow_east", 130, 0),
            ("supply_meadow_center", 0, 0),
        ],
    };
    for (name, x, y) in extra_supply {
        let _ = write!(
            bridge_markers,
            "\n\n[[marker]]\nname = \"{name}\"\nposition_m = [{x}, {y}]"
        );
    }
    let mut starts = String::new();
    for (player, team, x, y) in start_positions(options.players) {
        let team = if options.players > 2 {
            format!("team = {team}\n")
        } else {
            String::new()
        };
        let _ = write!(
            starts,
            "[[start]]\nplayer = {player}\n{team}position_m = [{x}, {y}]\n\n"
        );
    }
    let generator_players = if options.players > 2 {
        format!(" --players {}", options.players)
    } else {
        String::new()
    };
    let mut toml = String::new();
    let _ = writeln!(
        toml,
        "# Generated by `cli gen-map`; edit the PNGs freely (docs/MAPS.md).\n\
         name = \"{}\"\n\
         generator = \"cli gen-map --name {} --size-m {} --seed {}{generator_players}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = {step_mm}\n\
         biome = \"{}\"\n\
         water_flow_deg = 180\n\n\
         {starts}\
         [[marker]]\nname = \"hill_top\"\nposition_m = [{hill_x}, {hill_y}]\n\n\
         [[marker]]\nname = \"village\"\nposition_m = [{}, {}]{bridge_markers}",
        options.name,
        options.name,
        options.size_m,
        options.seed,
        options.biome,
        village.x.round() as i32,
        village.y.round() as i32,
    );
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;

    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, step {step_mm} mm, \
         {} hills, hill_top at ({hill_x}, {hill_y}), {} bridges, {} objects ({} trees, \
         {} hedge segments, {} buildings)",
        options.out.display(),
        layout.hills.len(),
        bridges.len(),
        summary.total,
        summary.trees,
        summary.hedges,
        summary.buildings,
    ))
}

fn write_png(path: &Path, side: u32, data: &[u8], palette: Option<Vec<u8>>) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), side, side);
    match palette {
        Some(palette) => {
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(palette);
        }
        None => {
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Sixteen);
        }
    }
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(data).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{Options, generate};
    use sim::path::PathGrid;
    use sim::{Catalog, Map, Scale, TerrainRules, Vec2};

    /// Generates the parameters of `test_small` into a temporary folder.
    fn small_map() -> Map {
        let dir = std::env::temp_dir().join(format!("mas-gen-bridges-{}", std::process::id()));
        let options = Options {
            name: "test_small",
            size_m: 500,
            seed: 7,
            players: 2,
            biome: "forest",
            out: &dir,
        };
        generate(&options).unwrap();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        map
    }

    /// Every crossing of `test_small` connects both banks: infantry and a
    /// half-track cross both bridges, a Panzer IV (25 t) the stone bridge
    /// and the ford but not the timber bridge.
    #[test]
    fn crossings_connect_both_banks() {
        let map = small_map();
        let kinds: Vec<&str> = map
            .bridges()
            .iter()
            .map(|b| {
                let o = &map.objects()[b.object as usize];
                map.object_kinds()[usize::from(o.kind)].id.as_str()
            })
            .collect();
        assert_eq!(kinds, ["env_bridge_stone", "env_bridge_wood"]);
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types = sim::unit_types_from_catalog(
            &catalog,
            &scale,
            &["de_inf_rifle_squad", "de_sdkfz251_1", "de_pz4_h"],
        )
        .unwrap();
        assert!(types[2].mass_kg > map.bridges()[1].max_load_kg);
        assert!(types[1].mass_kg <= map.bridges()[1].max_load_kg);
        let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
        let uses_deck = |path: &sim::path::Path, bridge: usize| {
            path.cells.iter().any(|&c| {
                let (w, _) = grid.size();
                grid.deck_of(c % w, c / w) == Some(bridge)
            })
        };
        let across = |center: Vec2| {
            let off = Vec2::from_meters(30, 0);
            (center - off, center + off)
        };
        for (b, bridge) in map.bridges().iter().enumerate() {
            let (west, east) = across(bridge.center);
            for profile in 0..3 {
                let path = grid.find_path(west, east, profile).unwrap();
                let heavy = profile == 2 && bridge.max_load_kg < types[2].mass_kg;
                assert_eq!(uses_deck(&path, b), !heavy, "bridge {b} profile {profile}");
                if !heavy {
                    assert_eq!(path.goal, east, "bridge {b} profile {profile}");
                    assert!(path.length() < sim::Fixed::from_num(80), "bridge {b}");
                }
            }
        }
        // The ford: all of them wade through, no detour over a bridge.
        let ford = map.marker("ford").unwrap();
        let (west, east) = across(ford);
        for profile in 0..3 {
            let path = grid.find_path(west, east, profile).unwrap();
            // The goal may snap to a free cell nearby, but on the east bank.
            assert!(
                path.goal.x > ford.x + sim::Fixed::from_num(20),
                "ford profile {profile}"
            );
            assert!(
                path.length() < sim::Fixed::from_num(90),
                "ford profile {profile}"
            );
            assert!(!uses_deck(&path, 0) && !uses_deck(&path, 1));
        }
        for name in ["bridge_stone", "bridge_wood"] {
            let p = map.marker(name).unwrap();
            assert!(map.deck_at(p).is_some(), "{name}");
        }
    }
}
