//! Test map generator: `cli gen-map`.
//!
//! Builds a map from a seed: gentle rolling ground, hills, a river bed with
//! banks, a flat start meadow around the origin, village squares, field
//! tracks and a road, and writes `map.toml`, `height.png` and `terrain.png`
//! (format: `docs/MAPS.md`).
//!
//! The generator runs outside the simulation and may use floats: its output
//! files are the source of truth and are committed. It only uses basic
//! arithmetic and `sqrt` (both exactly rounded in IEEE 754), so the same seed
//! gives the same files on every platform.

use sim::{Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;

// Terrain ids of data/terrain.toml.
const GRASS: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const SAND: u8 = 3;
const MUD: u8 = 4;
const SHALLOW_WATER: u8 = 6;
const DEEP_WATER: u8 = 7;

/// Parameters of `cli gen-map`.
pub struct Options<'a> {
    pub name: &'a str,
    pub size_m: u32,
    pub seed: u64,
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
    squares: Vec<Square>,
    tracks: Vec<Path2>,
    roads: Vec<Path2>,
    noise: Vec<(ValueNoise, f64)>,
    mud: ValueNoise,
}

fn layout(random: &mut Random, size: f64) -> Layout {
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
    Layout {
        half,
        meadow_radius,
        hills,
        river,
        river_width,
        squares,
        tracks,
        roads,
        noise,
        mud,
    }
}

impl Layout {
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

        let w = self.river_width;
        let valley = w + 40.0;
        let river = if self.river.near(x, y, valley) {
            self.river.distance(x, y)
        } else {
            f64::INFINITY
        };

        for (paths, width, id) in [(&self.roads, 3.0, ROAD), (&self.tracks, 1.8, DIRT_TRACK)] {
            if terrain == GRASS
                && paths
                    .iter()
                    .any(|p| p.near(x, y, width) && p.distance(x, y) < width)
            {
                terrain = id;
            }
        }

        if river < valley {
            // A bed 3.5 m deep with gentle banks, cut into the ground.
            let depth = 3.5 * (1.0 - smoothstep(w * 0.5, valley, river));
            h -= depth;
            let ford = terrain == DIRT_TRACK;
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

/// Generates the map and writes its three files into `options.out`.
pub fn generate(options: &Options) -> Result<String, String> {
    if options.size_m < 100 || !options.size_m.is_multiple_of(4) || options.size_m > 16_000 {
        return Err("--size-m must be a multiple of 4 between 100 and 16000".into());
    }
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let size = f64::from(options.size_m);
    let samples = (options.size_m / 2 + 1) as usize;
    let mut random = Random(Rng::new(options.seed));
    let layout = layout(&mut random, size);

    let mut heights = Vec::with_capacity(samples * samples);
    let mut terrain = Vec::with_capacity(samples * samples);
    for j in 0..samples {
        for i in 0..samples {
            let (h, t) = layout.sample(coord(i, layout.half), coord(j, layout.half));
            heights.push(h);
            terrain.push(t);
        }
    }

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
    let village = &layout.squares[0];
    let mut toml = String::new();
    let _ = writeln!(
        toml,
        "# Generated by `cli gen-map`; edit the PNGs freely (docs/MAPS.md).\n\
         name = \"{}\"\n\
         generator = \"cli gen-map --name {} --size-m {} --seed {}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = {step_mm}\n\n\
         [[start]]\nplayer = 0\nposition_m = [0, 60]\n\n\
         [[start]]\nplayer = 1\nposition_m = [0, -60]\n\n\
         [[marker]]\nname = \"hill_top\"\nposition_m = [{hill_x}, {hill_y}]\n\n\
         [[marker]]\nname = \"village\"\nposition_m = [{}, {}]",
        options.name,
        options.name,
        options.size_m,
        options.seed,
        village.x.round() as i32,
        village.y.round() as i32,
    );
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;

    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, step {step_mm} mm, \
         {} hills, hill_top at ({hill_x}, {hill_y})",
        options.out.display(),
        layout.hills.len()
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
