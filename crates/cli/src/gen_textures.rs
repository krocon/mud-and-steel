//! Procedural ground textures for the terrain: `cli gen-terrain-textures`.
//!
//! Writes the material layers of the terrain shader as two vertical strips
//! (one square tile per layer, all seamlessly tileable) plus a macro noise
//! texture into `godot/assets/textures/terrain/`:
//!
//! - `terrain_albedo.png`: RGB albedo (sRGB), A height (0..1, linear)
//! - `terrain_normal.png`: RG tangent-space normal (x = image right, y = image
//!   down), B roughness, A ambient occlusion
//! - `terrain_macro.png`: four channels of tileable low-frequency noise for
//!   large-scale variation
//! - `terrain_clumps.png`: grass clumps for the meadow relief (prompt 191):
//!   R height (0..1), G and B its gradient per cell (x, y; 0.5 + g / 8),
//!   32 x 32 cells per tile
//!
//! Everything is drawn from noise, cells and strokes; no photo source, so no
//! license question (see `godot/assets/textures/CREDITS.md`). The output files
//! are committed; the generator is a tool and may use floats freely.
//!
//! The layer order is shared with `godot/assets/materials/terrain.gdshader`
//! and `scenes/terrain/terrain.gd` (`LAYERS`).

use std::f32::consts::TAU;
use std::path::Path;

/// Material layers, in strip order. Terrain ids 0..=7 of `data/terrain.toml`
/// map to the layers with the same index; rock and earth are optical only.
pub const LAYERS: [&str; 10] = [
    "grass",
    "dirt_track",
    "road_gravel",
    "sand",
    "mud",
    "snow",
    "river_pebbles",
    "river_silt",
    "rock",
    "earth",
];

/// Parameters of `cli gen-terrain-textures`.
pub struct Options<'a> {
    pub out: &'a Path,
    /// Pixels per tile side (power of two).
    pub size: u32,
    pub seed: u32,
    /// Optional contact sheet: every layer tiled 2 x 2 at half size, lit from
    /// the north-west, for a quick look without Godot.
    pub preview: Option<&'a Path>,
}

/// One generated layer: linear albedo, height and roughness per pixel.
struct Layer {
    n: usize,
    albedo: Vec<[f32; 3]>,
    height: Vec<f32>,
    rough: Vec<f32>,
    /// Height range in meters that `height` 0..1 spans (for the normal map).
    relief_m: f32,
    /// Tile side in meters (must match `tile_m` in terrain.gd).
    tile_m: f32,
}

impl Layer {
    fn new(n: usize, tile_m: f32, relief_m: f32) -> Self {
        Self {
            n,
            albedo: vec![[0.0; 3]; n * n],
            height: vec![0.0; n * n],
            rough: vec![0.9; n * n],
            relief_m,
            tile_m,
        }
    }

    fn index(&self, x: i64, y: i64) -> usize {
        let n = self.n as i64;
        (y.rem_euclid(n) * n + x.rem_euclid(n)) as usize
    }
}

// --- Hashing and tileable noise ---------------------------------------------

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Small deterministic random stream for strokes and stones.
struct Stream(u32);

impl Stream {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9);
        hash01(self.0 as i32, 17, 3)
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Periodic gradient noise in about -1..1. `u`, `v` in tile units (0..1),
/// `period` lattice cells per tile.
fn gradient(u: f32, v: f32, period: i32, seed: u32) -> f32 {
    let x = u * period as f32;
    let y = v * period as f32;
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (ix, iy) = (x0 as i32, y0 as i32);
    let corner = |dx: i32, dy: i32| {
        let a = hash01(
            (ix + dx).rem_euclid(period),
            (iy + dy).rem_euclid(period),
            seed,
        ) * TAU;
        a.cos() * (fx - dx as f32) + a.sin() * (fy - dy as f32)
    };
    let (sx, sy) = (fade(fx), fade(fy));
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * sx;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * sx;
    (top + (bottom - top) * sy) * 1.41
}

/// Fractal sum of `octaves` gradient noises, period doubling, about -1..1.
fn fbm(u: f32, v: f32, period: i32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut p) = (0.0, 1.0, 0.0, period);
    for o in 0..octaves {
        sum += gradient(u, v, p, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        p *= 2;
    }
    sum / norm
}

/// Periodic cellular noise: distances (in cells) to the nearest and second
/// nearest feature point and a random value of the nearest cell.
fn worley(u: f32, v: f32, period: i32, seed: u32) -> (f32, f32, f32) {
    let x = u * period as f32;
    let y = v * period as f32;
    let (ix, iy) = (x.floor() as i32, y.floor() as i32);
    let (mut f1, mut f2, mut id) = (9.0f32, 9.0f32, 0.0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (cx, cy) = (ix + dx, iy + dy);
            let (wx, wy) = (cx.rem_euclid(period), cy.rem_euclid(period));
            let px = cx as f32 + hash01(wx, wy, seed);
            let py = cy as f32 + hash01(wx, wy, seed ^ 0x55);
            let d = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = hash01(wx, wy, seed ^ 0xaa);
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2, id)
}

// --- Colors -------------------------------------------------------------------

fn srgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    let lin = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(r), lin(g), lin(b)]
}

fn to_srgb8(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5) as u8
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// --- Shared building blocks -----------------------------------------------

/// Fills the layer per pixel from `f(u, v) -> (albedo, height, roughness)`.
fn paint(layer: &mut Layer, f: impl Fn(f32, f32) -> ([f32; 3], f32, f32)) {
    let n = layer.n;
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32);
            let (a, h, r) = f(u, v);
            let k = y * n + x;
            layer.albedo[k] = a;
            layer.height[k] = h;
            layer.rough[k] = r;
        }
    }
}

/// Draws grass blades: short tapered strokes that raise the height and
/// paint their color where they end up on top (a simple height buffer).
struct Blades {
    count: usize,
    length_m: (f32, f32),
    width_px: f32,
    colors: [[f32; 3]; 3],
    /// Optional mask (u, v) -> 0..1: chance that a blade grows there.
    lean: f32,
}

fn draw_blades(layer: &mut Layer, blades: &Blades, mask: impl Fn(f32, f32) -> f32, seed: u32) {
    let n = layer.n as f32;
    let px_per_m = n / layer.tile_m;
    let mut rng = Stream(seed);
    // A common wind direction with jitter, so the sward has a grain.
    let lean = rng.range(0.0, TAU);
    for _ in 0..blades.count {
        let (u, v) = (rng.next(), rng.next());
        if rng.next() > mask(u, v) {
            continue;
        }
        let angle = if rng.next() < blades.lean {
            lean + rng.range(-0.6, 0.6)
        } else {
            rng.range(0.0, TAU)
        };
        let len = rng.range(blades.length_m.0, blades.length_m.1) * px_per_m;
        let tint = rng.next();
        let color = if tint < 0.5 {
            mix(blades.colors[0], blades.colors[1], tint * 2.0)
        } else {
            mix(blades.colors[1], blades.colors[2], (tint - 0.5) * 2.0)
        };
        let shade = rng.range(0.75, 1.2);
        let base_h = rng.range(0.35, 0.6);
        let (dx, dy) = (angle.cos(), angle.sin());
        let steps = (len * 1.5).max(2.0) as i32;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let width = blades.width_px * (1.0 - 0.7 * t);
            let (cx, cy) = (u * n + dx * len * t, v * n + dy * len * t);
            let h = base_h + 0.4 * t;
            let r = width.ceil() as i32;
            for oy in -r..=r {
                for ox in -r..=r {
                    let (px, py) = (cx.floor() as i64 + ox as i64, cy.floor() as i64 + oy as i64);
                    let d =
                        ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
                    if d > width * 0.5 + 0.5 {
                        continue;
                    }
                    let k = layer.index(px, py);
                    if h > layer.height[k] {
                        layer.height[k] = h;
                        // Tips catch more light than the base.
                        layer.albedo[k] = scale(color, shade * (0.75 + 0.45 * t));
                        layer.rough[k] = 0.8;
                    }
                }
            }
        }
    }
}

/// Scatters round stones (domes) with colors from `palette`, `count` per
/// tile with radii in meters. Stones only grow where they rise above the
/// current height.
fn draw_stones(
    layer: &mut Layer,
    count: usize,
    radius_m: (f32, f32),
    palette: &[[f32; 3]],
    roughness: f32,
    seed: u32,
) {
    let n = layer.n as f32;
    let px_per_m = n / layer.tile_m;
    let mut rng = Stream(seed);
    for _ in 0..count {
        let (cx, cy) = (rng.next() * n, rng.next() * n);
        let r = rng.range(radius_m.0, radius_m.1) * px_per_m;
        let stretch = rng.range(0.6, 1.0);
        let angle = rng.range(0.0, TAU);
        let (ca, sa) = (angle.cos(), angle.sin());
        let color = scale(
            palette[(rng.next() * palette.len() as f32) as usize % palette.len()],
            rng.range(0.75, 1.25),
        );
        let top = rng.range(0.55, 1.0);
        let ri = r.ceil() as i64 + 1;
        for oy in -ri..=ri {
            for ox in -ri..=ri {
                let (px, py) = (cx.floor() as i64 + ox, cy.floor() as i64 + oy);
                let (lx, ly) = (px as f32 + 0.5 - cx, py as f32 + 0.5 - cy);
                let (a, b) = (
                    (lx * ca + ly * sa) / r,
                    (-lx * sa + ly * ca) / (r * stretch),
                );
                let d2 = a * a + b * b;
                if d2 >= 1.0 {
                    continue;
                }
                let dome = (1.0 - d2).sqrt();
                let h = top * (0.35 + 0.65 * dome);
                let k = layer.index(px, py);
                if h > layer.height[k] {
                    layer.height[k] = h;
                    // Lighter on top, darker at the rim.
                    layer.albedo[k] = scale(color, 0.7 + 0.35 * dome);
                    layer.rough[k] = roughness;
                }
            }
        }
    }
}

// --- Layers ---------------------------------------------------------------------

fn grass(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 4.0, 0.06);
    let soil = srgb(58, 50, 34);
    let litter = srgb(92, 82, 52);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 6, 4, seed);
        let a = mix(soil, litter, 0.5 + 0.5 * p);
        (a, 0.15 + 0.1 * p, 0.95)
    });
    let patches = |u: f32, v: f32| fbm(u, v, 3, 3, seed + 7);
    let blades = Blades {
        count: n * n / 9,
        length_m: (0.03, 0.09),
        width_px: 1.6 * n as f32 / 1024.0,
        colors: [srgb(46, 66, 24), srgb(78, 96, 38), srgb(122, 120, 62)],
        lean: 0.5,
    };
    draw_blades(&mut l, &blades, |u, v| 0.8 + 0.2 * patches(u, v), seed + 1);
    // Dry patches: tint towards straw where the patch noise is high.
    let straw = srgb(150, 136, 82);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
            let dry = smoothstep(0.15, 0.6, patches(u, v)) * 0.35;
            let k = y * n + x;
            l.albedo[k] = mix(l.albedo[k], scale(straw, l.albedo[k][1] / straw[1]), dry);
        }
    }
    l
}

fn earth_base(l: &mut Layer, seed: u32, dark: [f32; 3], light: [f32; 3], clod_period: i32) {
    paint(l, |u, v| {
        let p = fbm(u, v, 5, 5, seed);
        // Crumbly clods: rounded lumps from two cell sizes, soft gaps.
        let (f1, _, id) = worley(u, v, clod_period, seed + 3);
        let (g1, _, _) = worley(u, v, clod_period * 3, seed + 4);
        let crumb = fbm(u, v, clod_period * 4, 2, seed + 5);
        let clod = (1.0 - f1).powi(2) * 0.6 + (1.0 - g1).powi(2) * 0.4;
        let shade = 0.78 + 0.22 * clod + 0.1 * crumb;
        let a = scale(mix(dark, light, 0.5 + 0.5 * p + 0.12 * (id - 0.5)), shade);
        let h = 0.25 + 0.3 * clod + 0.15 * p + 0.05 * crumb;
        (a, h, 0.92 - 0.05 * p)
    });
}

fn earth(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 4.0, 0.05);
    earth_base(&mut l, seed, srgb(66, 50, 34), srgb(118, 96, 68), 28);
    let stones = [srgb(120, 112, 100), srgb(98, 88, 74), srgb(140, 128, 110)];
    draw_stones(&mut l, n * n / 900, (0.004, 0.012), &stones, 0.8, seed + 9);
    let blades = Blades {
        count: n * n / 160,
        length_m: (0.02, 0.06),
        width_px: 1.3 * n as f32 / 1024.0,
        colors: [srgb(70, 78, 36), srgb(118, 110, 60), srgb(150, 132, 80)],
        lean: 0.2,
    };
    draw_blades(
        &mut l,
        &blades,
        |u, v| smoothstep(0.0, 0.5, fbm(u, v, 4, 2, seed + 4)),
        seed + 5,
    );
    l
}

/// Field track: compacted earth with ruts along the image's y axis (the
/// shader turns them along the track). Two tank tracks with grousers and two
/// narrow wheel ruts per tile, puddles in the deepest spots and a few grass
/// tufts between them.
fn dirt_track(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 4.0, 0.12);
    let dry = srgb(128, 106, 76);
    let dark = srgb(84, 66, 46);
    let wet = srgb(52, 42, 32);
    // (center u, half width u, depth, tread)
    let ruts: [(f32, f32, f32, bool); 4] = [
        (0.20, 0.065, 1.0, true),
        (0.80, 0.065, 1.0, true),
        (0.43, 0.018, 0.7, false),
        (0.58, 0.018, 0.7, false),
    ];
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 6, 5, seed);
        // Ruts wander a little; the wobble repeats once per tile along v.
        let wobble = 0.004 * (TAU * v).sin() + 0.006 * fbm(0.0, v, 3, 2, seed + 2);
        let mut depth = 0.0f32;
        let mut tread = 0.0f32;
        for (i, &(c, w, d, grousers)) in ruts.iter().enumerate() {
            let du = ((u - c - wobble * (1.0 + i as f32 * 0.3)).abs()) / w;
            if du < 1.0 {
                let profile = 0.5 + 0.5 * (du * std::f32::consts::PI).cos();
                depth = depth.max(profile * d);
                if grousers {
                    // Grouser marks every ~15 cm (26 per 4 m tile).
                    let g = (v * 26.0 + 0.15 * (u * 9.0).sin()).fract();
                    tread = tread
                        .max(smoothstep(0.55, 0.7, g) * (1.0 - smoothstep(0.85, 1.0, g)) * profile);
                }
            }
        }
        let puddle =
            smoothstep(0.75, 0.95, depth) * smoothstep(0.05, 0.35, fbm(u, v, 2, 2, seed + 6));
        let h = 0.55 + 0.12 * p - 0.45 * depth + 0.06 * tread - 0.05 * puddle;
        let mut a = mix(dry, dark, 0.35 + 0.35 * p + 0.5 * depth);
        a = mix(a, wet, puddle * 0.9);
        let r = 0.9 - 0.25 * depth - 0.6 * puddle;
        (a, h.clamp(0.0, 1.0), r)
    });
    let stones = [srgb(130, 120, 104), srgb(104, 96, 82)];
    draw_stones(&mut l, n * n / 1200, (0.004, 0.01), &stones, 0.8, seed + 11);
    let blades = Blades {
        count: n * n / 40,
        length_m: (0.03, 0.08),
        width_px: 1.5 * n as f32 / 1024.0,
        colors: [srgb(54, 70, 28), srgb(90, 98, 44), srgb(132, 124, 70)],
        lean: 0.3,
    };
    // Grass only grows between the ruts (u around 0.5 and at the tile edge).
    let tufts = move |u: f32, v: f32| {
        let between = smoothstep(0.1, 0.0, (u - 0.505).abs())
            + smoothstep(0.07, 0.0, (u - 0.0).abs().min((u - 1.0).abs()));
        between * (0.6 + 0.4 * fbm(u, v, 5, 2, seed + 12))
    };
    draw_blades(&mut l, &blades, tufts, seed + 13);
    l
}

fn road_gravel(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 3.0, 0.03);
    let fines = srgb(112, 102, 86);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 4, 5, seed);
        let a = scale(fines, 0.85 + 0.2 * p);
        (a, 0.2 + 0.1 * p, 0.95)
    });
    let stones = [
        srgb(150, 146, 136),
        srgb(120, 114, 104),
        srgb(166, 150, 124),
        srgb(96, 90, 82),
    ];
    draw_stones(&mut l, n * n / 30, (0.006, 0.018), &stones, 0.75, seed + 1);
    // Dust and wheel polish: large patches a bit lighter and smoother.
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
            let dust = smoothstep(-0.1, 0.5, fbm(u, v, 2, 3, seed + 2));
            let k = y * n + x;
            l.albedo[k] = mix(l.albedo[k], srgb(150, 136, 112), dust * 0.25);
        }
    }
    l
}

fn sand(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 6.0, 0.03);
    let light = srgb(206, 186, 140);
    let shade = srgb(168, 144, 100);
    paint(&mut l, |u, v| {
        let warp = fbm(u, v, 3, 3, seed) * 0.8;
        let ripple = (TAU * (v * 18.0 + u * 3.0 + warp)).sin();
        let crest = 0.5 + 0.5 * ripple;
        let grain = hash01((u * 4096.0) as i32, (v * 4096.0) as i32, seed) - 0.5;
        let damp = smoothstep(0.2, 0.7, fbm(u, v, 2, 3, seed + 1));
        let a = scale(
            mix(shade, light, 0.55 + 0.3 * crest + 0.3 * grain * 0.3),
            1.0 - 0.3 * damp,
        );
        (a, 0.4 + 0.3 * crest + 0.05 * grain, 0.95 - 0.2 * damp)
    });
    l
}

fn mud(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 5.0, 0.08);
    let mud = srgb(78, 60, 42);
    let wet = srgb(44, 36, 28);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 4, 6, seed);
        let lumps = fbm(u, v, 16, 3, seed + 1);
        let h = 0.5 + 0.3 * p + 0.15 * lumps;
        // Water stands in the low spots.
        let puddle = smoothstep(0.42, 0.34, h);
        let a = mix(
            scale(mud, 0.9 + 0.5 * lumps + 0.3 * p),
            wet,
            0.25 + 0.75 * puddle,
        );
        (a, h.max(0.36), 0.55 - 0.45 * puddle)
    });
    // Tire and boot marks: a few short rut segments in random directions.
    let mut rng = Stream(seed + 2);
    for _ in 0..3 {
        let (cx, cy) = (rng.next() * n as f32, rng.next() * n as f32);
        let angle = rng.range(0.0, TAU);
        let (dx, dy) = (angle.cos(), angle.sin());
        let len = rng.range(0.3, 0.8) * n as f32;
        let half = rng.range(0.025, 0.05) * n as f32;
        let r = half as i64 + 1;
        for s in 0..(len as i64) {
            for o in -r..=r {
                let (px, py) = (
                    cx + dx * s as f32 - dy * o as f32,
                    cy + dy * s as f32 + dx * o as f32,
                );
                let k = l.index(px as i64, py as i64);
                let profile = 1.0 - (o as f32 / half).abs();
                if profile > 0.0 {
                    l.height[k] = (l.height[k] - 0.08 * profile).max(0.2);
                    l.albedo[k] = scale(l.albedo[k], 1.0 - 0.06 * profile);
                    l.rough[k] = (l.rough[k] - 0.15 * profile).max(0.1);
                }
            }
        }
    }
    l
}

fn snow(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 6.0, 0.05);
    let white = srgb(236, 240, 245);
    let blue = srgb(186, 198, 218);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 3, 6, seed);
        let wind = 0.5 + 0.5 * (TAU * (u * 7.0 + 0.6 * fbm(u, v, 2, 2, seed + 1))).sin();
        let h = 0.5 + 0.3 * p + 0.04 * wind;
        let sparkle = if hash01((u * 2048.0) as i32, (v * 2048.0) as i32, seed) > 0.995 {
            0.25
        } else {
            0.0
        };
        let a = mix(
            blue,
            white,
            0.55 + 0.45 * smoothstep(0.25, 0.75, h) + sparkle,
        );
        (a, h, 0.7 - sparkle)
    });
    l
}

fn river_pebbles(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 3.0, 0.05);
    let silt = srgb(70, 64, 50);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 4, 4, seed);
        (scale(silt, 0.8 + 0.3 * p), 0.2 + 0.1 * p, 0.35)
    });
    let stones = [
        srgb(110, 104, 92),
        srgb(86, 80, 70),
        srgb(124, 112, 92),
        srgb(72, 70, 66),
    ];
    draw_stones(&mut l, n * n / 80, (0.012, 0.045), &stones, 0.3, seed + 1);
    l
}

fn river_silt(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 6.0, 0.03);
    let silt = srgb(56, 54, 40);
    let algae = srgb(46, 58, 34);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 3, 5, seed);
        let a = mix(silt, algae, smoothstep(0.0, 0.5, fbm(u, v, 5, 3, seed + 1)));
        (scale(a, 0.85 + 0.25 * p), 0.4 + 0.3 * p, 0.45)
    });
    l
}

fn rock(n: usize, seed: u32) -> Layer {
    let mut l = Layer::new(n, 8.0, 0.25);
    let grey = srgb(122, 116, 106);
    let dark = srgb(74, 70, 64);
    let lichen = srgb(118, 122, 80);
    paint(&mut l, |u, v| {
        let p = fbm(u, v, 3, 6, seed);
        let (wu, wv) = (
            u + 0.06 * fbm(u, v, 4, 3, seed + 5),
            v + 0.06 * fbm(u, v, 4, 3, seed + 6),
        );
        let (f1, f2, id) = worley(wu.rem_euclid(1.0), wv.rem_euclid(1.0), 5, seed + 1);
        let crack = 0.4 + 0.6 * smoothstep(0.0, 0.05, f2 - f1);
        let strata = 0.5 + 0.5 * (TAU * (v * 9.0 + 0.5 * p)).sin();
        let mut a = mix(dark, grey, 0.4 + 0.4 * p + 0.2 * id + 0.1 * strata);
        a = scale(a, 0.5 + 0.5 * crack);
        let moss = smoothstep(0.35, 0.6, fbm(u, v, 8, 3, seed + 2)) * 0.6;
        a = mix(a, lichen, moss);
        let h = (0.3 + 0.35 * p + 0.25 * (1.0 - f1) + 0.1 * strata) * (0.4 + 0.6 * crack);
        (a, h, 0.85)
    });
    l
}

// --- Output ---------------------------------------------------------------------

/// Height-derived normal (tangent space, x right, y down) and cavity AO.
fn normal_and_ao(l: &Layer) -> Vec<[u8; 4]> {
    let n = l.n;
    let m_per_px = l.tile_m / n as f32;
    let h = |x: i64, y: i64| l.height[l.index(x, y)] * l.relief_m;
    // Blurred height for the cavity term (box blur, radius 4 px).
    let r = 4i64;
    let mut out = Vec::with_capacity(n * n);
    for y in 0..n as i64 {
        for x in 0..n as i64 {
            let dx = (h(x + 1, y) - h(x - 1, y)) / (2.0 * m_per_px);
            let dy = (h(x, y + 1) - h(x, y - 1)) / (2.0 * m_per_px);
            let len = (dx * dx + dy * dy + 1.0).sqrt();
            let (nx, ny) = (-dx / len, -dy / len);
            let mut sum = 0.0;
            for oy in [-r, 0, r] {
                for ox in [-r, 0, r] {
                    sum += h(x + ox, y + oy);
                }
            }
            let cavity = (sum / 9.0 - h(x, y)) / l.relief_m.max(1e-6);
            let ao = (1.0 - cavity * 2.0).clamp(0.4, 1.0);
            let k = (y as usize) * n + x as usize;
            out.push([
                ((nx * 0.5 + 0.5) * 255.0 + 0.5) as u8,
                ((ny * 0.5 + 0.5) * 255.0 + 0.5) as u8,
                (l.rough[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                (ao * 255.0 + 0.5) as u8,
            ]);
        }
    }
    out
}

fn macro_noise(n: usize, seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32);
            let c = [
                fbm(u, v, 3, 5, seed),
                fbm(u, v, 7, 4, seed + 1),
                fbm(u, v, 5, 4, seed + 2),
                fbm(u, v, 24, 3, seed + 3),
            ];
            for v in c {
                out.push(((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            }
        }
    }
    out
}

/// Grass clumps (prompt 191): `cells` x `cells` cells per tile, up to two
/// domes per cell of random radius anywhere in the cell, about one cell in
/// four empty (bare gaps); the highest dome wins. Tileable (cells wrap).
/// Irregular on purpose: one dome per cell on a lattice reads as a knitted
/// carpet from the game camera.
fn clumps(n: usize, cells: i32, seed: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let gx = (x as f32 + 0.5) / n as f32 * cells as f32;
            let gy = (y as f32 + 0.5) / n as f32 * cells as f32;
            let (ix, iy) = (gx.floor() as i32, gy.floor() as i32);
            let mut best = (0.0f32, 0.0f32, 0.0f32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (cx, cy) = (ix + dx, iy + dy);
                    let (wx, wy) = (cx.rem_euclid(cells), cy.rem_euclid(cells));
                    let count = match hash01(wx, wy, seed + 9) {
                        v if v < 0.25 => 0,
                        v if v < 0.7 => 1,
                        _ => 2,
                    };
                    for k in 0..count {
                        let s2 = seed + 16 * k;
                        let hx = hash01(wx, wy, s2);
                        let hy = hash01(wx, wy, s2 + 1);
                        let r = 0.25 + 0.6 * hash01(wx, wy, s2 + 2).powf(1.5);
                        let tall = 0.6 + 0.4 * hash01(wx, wy, s2 + 3);
                        let ddx = gx - (cx as f32 + hx);
                        let ddy = gy - (cy as f32 + hy);
                        let len = (ddx * ddx + ddy * ddy).sqrt();
                        let t = (len / r).min(1.0);
                        let h = (1.0 - t * t) * (1.0 - t * t) * tall;
                        if h > best.0 {
                            let dh = -4.0 * t * (1.0 - t * t) / r * tall;
                            let (gxv, gyv) = if len > 1e-4 {
                                (ddx / len * dh, ddy / len * dh)
                            } else {
                                (0.0, 0.0)
                            };
                            best = (h, gxv, gyv);
                        }
                    }
                }
            }
            let enc = |g: f32| ((0.5 + g / 8.0).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            out.extend_from_slice(&[
                (best.0.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
                enc(best.1),
                enc(best.2),
                255,
            ]);
        }
    }
    out
}

fn write_rgba(path: &Path, width: u32, height: u32, data: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::High);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(data).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

/// Generates all layers and writes the four PNGs; returns a report.
pub fn generate(options: &Options) -> Result<String, String> {
    let n = options.size as usize;
    if !n.is_power_of_two() || !(64..=4096).contains(&n) {
        return Err("size must be a power of two between 64 and 4096".into());
    }
    std::fs::create_dir_all(options.out).map_err(|e| e.to_string())?;
    let s = options.seed;
    let builders: [fn(usize, u32) -> Layer; 10] = [
        grass,
        dirt_track,
        road_gravel,
        sand,
        mud,
        snow,
        river_pebbles,
        river_silt,
        rock,
        earth,
    ];
    let mut albedo = Vec::with_capacity(n * n * 4 * LAYERS.len());
    let mut normal = Vec::with_capacity(n * n * 4 * LAYERS.len());
    let mut report = String::new();
    for (i, build) in builders.iter().enumerate() {
        let layer = build(n, s.wrapping_add(i as u32 * 1000));
        for (a, h) in layer.albedo.iter().zip(&layer.height) {
            albedo.extend_from_slice(&[
                to_srgb8(a[0]),
                to_srgb8(a[1]),
                to_srgb8(a[2]),
                (h.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            ]);
        }
        for px in normal_and_ao(&layer) {
            normal.extend_from_slice(&px);
        }
        report += &format!("layer {i} {:<14} tile {} m\n", LAYERS[i], layer.tile_m);
    }
    let strip = (n * LAYERS.len()) as u32;
    write_rgba(
        &options.out.join("terrain_albedo.png"),
        n as u32,
        strip,
        &albedo,
    )?;
    write_rgba(
        &options.out.join("terrain_normal.png"),
        n as u32,
        strip,
        &normal,
    )?;
    write_rgba(
        &options.out.join("terrain_macro.png"),
        512,
        512,
        &macro_noise(512, s + 77),
    )?;
    write_rgba(
        &options.out.join("terrain_clumps.png"),
        512,
        512,
        &clumps(512, 32, s + 91),
    )?;
    if let Some(path) = options.preview {
        write_preview(path, n, &albedo, &normal)?;
        report += &format!("preview {}\n", path.display());
    }
    report += &format!(
        "wrote {} ({} layers of {n} px)",
        options.out.display(),
        LAYERS.len()
    );
    Ok(report)
}

/// Contact sheet: 5 x 2 layers, each tiled 2 x 2 at half resolution and lit.
fn write_preview(path: &Path, n: usize, albedo: &[u8], normal: &[u8]) -> Result<(), String> {
    let (cols, rows) = (5, LAYERS.len().div_ceil(5));
    let (w, h) = (cols * n, rows * n);
    let mut out = vec![0u8; w * h * 4];
    let light = [-0.45f32, -0.45, 0.77];
    for layer in 0..LAYERS.len() {
        let (ox, oy) = ((layer % cols) * n, (layer / cols) * n);
        for y in 0..n {
            for x in 0..n {
                // Half size, 2 x 2 repeats: source pixel (2x mod n, 2y mod n).
                let (sx, sy) = ((2 * x) % n, (2 * y) % n);
                let k = ((layer * n + sy) * n + sx) * 4;
                let nx = normal[k] as f32 / 127.5 - 1.0;
                let ny = normal[k + 1] as f32 / 127.5 - 1.0;
                let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
                let ao = normal[k + 3] as f32 / 255.0;
                let lit =
                    (0.45 + 0.75 * (nx * light[0] + ny * light[1] + nz * light[2]).max(0.0)) * ao;
                let o = ((oy + y) * w + ox + x) * 4;
                for c in 0..3 {
                    let lin = srgb(albedo[k], albedo[k + 1], albedo[k + 2])[c] * lit;
                    out[o + c] = to_srgb8(lin);
                }
                out[o + 3] = 255;
            }
        }
    }
    write_rgba(path, w as u32, h as u32, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_tiles_seamlessly() {
        for &(u, v) in &[(0.0, 0.3), (0.7, 0.0)] {
            let a = fbm(u, v, 5, 4, 1);
            let b = fbm(
                u + if u == 0.0 { 1.0 } else { 0.0 },
                v + if v == 0.0 { 1.0 } else { 0.0 },
                5,
                4,
                1,
            );
            assert!((a - b).abs() < 1e-4, "{a} vs {b}");
            let (w1, ..) = worley(u, v, 7, 2);
            let (w2, ..) = worley(
                u + if u == 0.0 { 1.0 } else { 0.0 },
                v + if v == 0.0 { 1.0 } else { 0.0 },
                7,
                2,
            );
            assert!((w1 - w2).abs() < 1e-4);
        }
    }

    #[test]
    fn small_generation_works() {
        let dir = std::env::temp_dir().join(format!("ms_textures_{}", std::process::id()));
        let report = generate(&Options {
            out: &dir,
            size: 64,
            seed: 1,
            preview: None,
        })
        .unwrap();
        assert!(report.contains("10 layers"));
        assert!(dir.join("terrain_albedo.png").exists());
        assert!(dir.join("terrain_clumps.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
