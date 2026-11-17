//! Map objects and decoration for drawing (presentation only, floats allowed).
//!
//! The simulation owns the placed objects (`sim::Map::objects`); this module
//! turns them into `MultiMesh` buffers grouped by kind and chunk, and
//! scatters decoration (grass, flowers, bushes, small stones) that exists
//! only in Godot. Both are reproducible: the same map gives the same
//! instances, decoration is derived from a seed and the lattice position.
//!
//! A `MultiMesh` buffer holds 12 floats per instance, the rows of the 3 x 4
//! transform: `[b.x.x, b.y.x, b.z.x, o.x, b.x.y, b.y.y, b.z.y, o.y, b.x.z,
//! b.y.z, b.z.z, o.z]`. Models are built with their +Y (Blender) axis along
//! the heading, which is Godot -Z; a sim heading of `h` degrees (clockwise
//! from north) is a rotation of `-h` about Godot's Y axis.

use sim::objects::Shape;
use sim::{Fixed, Map, MapObject, Vec2};
use std::collections::BTreeMap;

/// Presentation-only size jitter per kind prefix (fraction, ±); the last
/// matching prefix counts. Rock groups and ridge pieces keep the size of
/// their blocking box (prompt 074).
const SCALE_JITTER: [(&str, f32); 7] = [
    ("env_tree_", 0.12),
    ("env_bush", 0.15),
    ("env_rock", 0.25),
    ("env_rock_", 0.0),
    ("env_rock_slab", 0.08),
    ("env_hay_bale", 0.08),
    ("env_tree_stump", 0.15),
];
/// Kinds that lie on the slope like a sheet (prompt 074): sheared onto the
/// plane of the ground under their box instead of standing at its lowest
/// corner.
const SHEET_KINDS: [&str; 1] = ["env_rock_slab"];
/// Kinds with a seam of scree, tufts and bushes around them (prompt 074).
const ROCK_KINDS: [&str; 3] = ["env_rock_slab", "env_rock_outcrop", "env_rock_massif"];
/// Width (m) of that seam.
const ROCK_SEAM_M: i32 = 3;
/// The boxes of solid rocks are this much (dm) larger than the model's
/// foot (data/objects.toml); decoration grows up to the foot.
const ROCK_BOX_MARGIN_DM: i32 = 4;
/// Terrain ids the seam decoration grows on (grass, sand, mud, snow, tall
/// grass; not on tracks, roads or water).
const SEAM_TERRAIN: [u8; 5] = [0, 3, 4, 5, 8];
/// Trees are sunk into the ground a little, so slopes show no floating roots.
const TREE_SINK_M: f32 = 0.15;
/// Samples within this distance (m) of a box-shaped object get no decoration.
const DECO_CLEARANCE_M: i32 = 1;
/// Cell size (m) of the wood density raster.
const WOOD_CELL_M: f32 = 8.0;
/// Trunks in the 3 x 3 cells around a wood cell that count as dense wood.
const WOOD_FULL: f32 = 6.0;

fn f(v: Fixed) -> f32 {
    v.to_num::<f32>()
}

/// SplitMix64: a well-mixed hash of a 64-bit value.
pub fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A value in `[0, 1)` from the top bits of a hash.
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Writes one transform (yaw about Y, uniform scale, origin) into `out`.
fn push_transform(out: &mut Vec<f32>, yaw: f32, scale: f32, origin: [f32; 3]) {
    push_sheared(out, yaw, scale, origin, (0.0, 0.0));
}

/// Like [`push_transform`], then sheared so that the model's ground plane
/// follows the slope `y = gx * x + gz * z` (world gradient of the ground).
fn push_sheared(out: &mut Vec<f32>, yaw: f32, scale: f32, origin: [f32; 3], (gx, gz): (f32, f32)) {
    let (s, c) = yaw.sin_cos();
    let row0 = [c * scale, 0.0, s * scale];
    let row2 = [-s * scale, 0.0, c * scale];
    let row1 = [
        gx * row0[0] + gz * row2[0],
        scale,
        gx * row0[2] + gz * row2[2],
    ];
    out.extend_from_slice(&[
        row0[0], row0[1], row0[2], origin[0], row1[0], row1[1], row1[2], origin[1], row2[0],
        row2[1], row2[2], origin[2],
    ]);
}

/// Ground plane under a box: the center height and the world gradient
/// `(dy/dx, dy/dz)` from the four corners (sheet kinds, prompt 074).
fn ground_plane(map: &Map, object: &MapObject, shape: Shape) -> (f32, (f32, f32)) {
    let (x, z) = (f(object.position.x), f(object.position.y));
    let center = ground(map, x, z);
    let Shape::Box {
        half_length,
        half_width,
    } = shape
    else {
        return (center, (0.0, 0.0));
    };
    let (fw, rt) = object.axes();
    let (hl, hw) = (f(half_length), f(half_width));
    let at = |a: f32, b: f32| {
        ground(
            map,
            x + f(fw.x) * hl * a + f(rt.x) * hw * b,
            z + f(fw.y) * hl * a + f(rt.y) * hw * b,
        )
    };
    let (pp, pm, mp, mm) = (at(1.0, 1.0), at(1.0, -1.0), at(-1.0, 1.0), at(-1.0, -1.0));
    // slopes along the box axes, then into world x / z
    let along = ((pp + pm) - (mp + mm)) / (4.0 * hl.max(0.01));
    let across = ((pp + mp) - (pm + mm)) / (4.0 * hw.max(0.01));
    let gx = along * f(fw.x) + across * f(rt.x);
    let gz = along * f(fw.y) + across * f(rt.y);
    (center, (gx, gz))
}

/// Bilinear ground height in meters (presentation; matches the simulation
/// to well below a millimeter).
pub fn ground(map: &Map, x: f32, z: f32) -> f32 {
    let (sx, sy) = map.samples();
    let size = map.size();
    let cell = map.cell_size_mm() as f32 / 1000.0;
    let grid = |v: f32, half: f32, n: u32| ((v + half) / cell).clamp(0.0, (n - 1) as f32);
    let gx = grid(x, f(size.x) / 2.0, sx);
    let gz = grid(z, f(size.y) / 2.0, sy);
    let i = (gx as u32).min(sx - 2);
    let j = (gz as u32).min(sy - 2);
    let (fx, fz) = (gx - i as f32, gz - j as f32);
    let h = |i: u32, j: u32| map.sample_height_mm(i, j) as f32;
    let top = h(i, j) + (h(i + 1, j) - h(i, j)) * fx;
    let bottom = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * fx;
    (top + (bottom - top) * fz) / 1000.0
}

/// Height an object stands at: the lowest ground under a box (so walls and
/// houses on slopes do not float), the center height for round shapes.
fn object_height(map: &Map, object: &MapObject, shape: Shape, tree: bool) -> f32 {
    let (x, z) = (f(object.position.x), f(object.position.y));
    match shape {
        Shape::Circle { .. } => ground(map, x, z) - if tree { TREE_SINK_M } else { 0.0 },
        Shape::Box {
            half_length,
            half_width,
        } => {
            let (fw, rt) = object.axes();
            let (hl, hw) = (f(half_length), f(half_width));
            let mut low = ground(map, x, z);
            for (a, b) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let px = x + f(fw.x) * hl * a + f(rt.x) * hw * b;
                let pz = z + f(fw.y) * hl * a + f(rt.y) * hw * b;
                low = low.min(ground(map, px, pz));
            }
            low
        }
    }
}

/// Instances of one kind in one chunk.
pub struct Group {
    pub kind: u16,
    pub chunk: (i32, i32),
    pub buffer: Vec<f32>,
}

/// All objects of the map as `MultiMesh` buffers, grouped by kind and by
/// square chunks of `chunk_m` meters (chunk `(0, 0)` starts at the map's
/// north-west corner). Sorted by kind, then chunk.
pub fn groups(map: &Map, chunk_m: f32) -> Vec<Group> {
    let size = map.size();
    let (half_x, half_z) = (f(size.x) / 2.0, f(size.y) / 2.0);
    let kinds = map.object_kinds();
    let jitter: Vec<f32> = kinds
        .iter()
        .map(|k| {
            SCALE_JITTER
                .iter()
                .filter(|(prefix, _)| k.id.starts_with(prefix))
                .map(|&(_, j)| j)
                .next_back()
                .unwrap_or(0.0)
        })
        .collect();
    // Bridges stand at the deck height in their middle; the renderer lowers
    // each model by its own deck height (glTF extra `deck_height_m`).
    let decks: BTreeMap<u32, f32> = map
        .bridges()
        .iter()
        .map(|b| (b.object, f(b.height_at(b.center))))
        .collect();
    let mut groups: BTreeMap<(u16, i32, i32), Vec<f32>> = BTreeMap::new();
    for (index, object) in (0u32..).zip(map.objects()) {
        let kind = &kinds[usize::from(object.kind)];
        let (x, z) = (f(object.position.x), f(object.position.y));
        let chunk = chunk_of(object, (half_x, half_z), chunk_m);
        let h = mix(object.position.x.to_bits().cast_unsigned()
            ^ mix(object.position.y.to_bits().cast_unsigned()));
        let scale = 1.0 + jitter[usize::from(object.kind)] * (unit(h) * 2.0 - 1.0);
        let yaw = -f32::from(object.heading_deg).to_radians();
        let buffer = groups.entry((object.kind, chunk.0, chunk.1)).or_default();
        if SHEET_KINDS.contains(&kind.id.as_str()) {
            let (y, slope) = ground_plane(map, object, kind.shape);
            push_sheared(buffer, yaw, scale, [x, y, z], slope);
            continue;
        }
        let y = decks.get(&index).copied().unwrap_or_else(|| {
            object_height(map, object, kind.shape, kind.id.starts_with("env_tree_"))
        });
        push_transform(buffer, yaw, scale, [x, y, z]);
    }
    groups
        .into_iter()
        .map(|((kind, cx, cz), buffer)| Group {
            kind,
            chunk: (cx, cz),
            buffer,
        })
        .collect()
}

/// Chunk of an object in [`groups`].
fn chunk_of(object: &MapObject, (half_x, half_z): (f32, f32), chunk_m: f32) -> (i32, i32) {
    let (x, z) = (f(object.position.x), f(object.position.y));
    (
        ((x + half_x) / chunk_m).floor() as i32,
        ((z + half_z) / chunk_m).floor() as i32,
    )
}

/// Where object `index` is drawn by [`groups`]: its kind, chunk and slot
/// (instance number within the group's buffer).
pub fn instance_of(map: &Map, index: usize, chunk_m: f32) -> Option<(u16, (i32, i32), u32)> {
    let size = map.size();
    let half = (f(size.x) / 2.0, f(size.y) / 2.0);
    let objects = map.objects();
    let object = objects.get(index)?;
    let chunk = chunk_of(object, half, chunk_m);
    let slot = objects[..index]
        .iter()
        .filter(|o| o.kind == object.kind && chunk_of(o, half, chunk_m) == chunk)
        .count();
    Some((object.kind, chunk, u32::try_from(slot).ok()?))
}

/// Rasters derived from the objects, for decoration.
pub struct DecoRasters {
    samples: (u32, u32),
    cell_m: f32,
    half: (f32, f32),
    /// Map samples covered by non-tree objects (houses, walls, hedges, hay).
    blocked: Vec<bool>,
    /// Map samples in the seam around rocks (prompt 074), 0 or 1.
    rock: Vec<f32>,
    /// Wood density 0..1 per `WOOD_CELL_M` cell.
    wood: Vec<f32>,
    wood_size: (usize, usize),
}

impl DecoRasters {
    pub fn new(map: &Map) -> Self {
        let (sx, sy) = map.samples();
        let size = map.size();
        let cell_m = map.cell_size_mm() as f32 / 1000.0;
        let half = (f(size.x) / 2.0, f(size.y) / 2.0);
        let mut blocked = vec![false; (sx * sy) as usize];
        let mut rock = vec![0.0f32; (sx * sy) as usize];
        let wood_size = (
            (f(size.x) / WOOD_CELL_M).ceil() as usize + 1,
            (f(size.y) / WOOD_CELL_M).ceil() as usize + 1,
        );
        let mut trunks = vec![0.0f32; wood_size.0 * wood_size.1];
        let cell = Fixed::from_num(map.cell_size_mm()) / Fixed::from_num(1000);
        let half_sample = cell / 2;
        for object in map.objects() {
            let kind = &map.object_kinds()[usize::from(object.kind)];
            if kind.id.starts_with("env_tree_") {
                let wx = ((f(object.position.x) + half.0) / WOOD_CELL_M) as usize;
                let wz = ((f(object.position.y) + half.1) / WOOD_CELL_M) as usize;
                if wx < wood_size.0 && wz < wood_size.1 {
                    trunks[wz * wood_size.0 + wx] += 1.0;
                }
                continue;
            }
            // Grow the shape by the clearance and mark the samples it covers.
            // Grass grows right up to rocks; around them lies a seam of
            // scree and tufts (prompt 074).
            let is_rock = ROCK_KINDS.contains(&kind.id.as_str());
            let grow = |m: Fixed| match kind.shape {
                Shape::Circle { radius } => Shape::Circle { radius: radius + m },
                Shape::Box {
                    half_length,
                    half_width,
                } => Shape::Box {
                    half_length: half_length + m,
                    half_width: half_width + m,
                },
            };
            // Solid rocks' boxes are 0.4 m larger than their foot.
            let margin = if !is_rock {
                Fixed::from_num(DECO_CLEARANCE_M)
            } else if kind.blocks == sim::objects::Blocks::None {
                Fixed::ZERO
            } else {
                -Fixed::from_num(ROCK_BOX_MARGIN_DM) / 10
            };
            let shape = grow(margin);
            let seam = grow(Fixed::from_num(ROCK_SEAM_M));
            let extent = object.half_extent(if is_rock { seam } else { shape });
            let to_index = |v: Fixed, half: f32, n: u32| {
                ((f(v) + half) / cell_m).round().clamp(0.0, (n - 1) as f32) as u32
            };
            let (i0, i1) = (
                to_index(object.position.x - extent.x, half.0, sx),
                to_index(object.position.x + extent.x, half.0, sx),
            );
            let (j0, j1) = (
                to_index(object.position.y - extent.y, half.1, sy),
                to_index(object.position.y + extent.y, half.1, sy),
            );
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let center = Vec2::new(
                        Fixed::from_num(i) * cell - size.x / 2,
                        Fixed::from_num(j) * cell - size.y / 2,
                    );
                    if object.overlaps_square(shape, center, half_sample) {
                        blocked[(j * sx + i) as usize] = true;
                    } else if is_rock && object.overlaps_square(seam, center, half_sample) {
                        rock[(j * sx + i) as usize] = 1.0;
                    }
                }
            }
        }
        // 3 x 3 sum of trunks, scaled to 0..1.
        let (w, h) = wood_size;
        let mut wood = vec![0.0; w * h];
        for z in 0..h {
            for x in 0..w {
                let mut sum = 0.0;
                for dz in z.saturating_sub(1)..=(z + 1).min(h - 1) {
                    for dx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                        sum += trunks[dz * w + dx];
                    }
                }
                wood[z * w + x] = (sum / WOOD_FULL).min(1.0);
            }
        }
        Self {
            samples: (sx, sy),
            cell_m,
            half,
            blocked,
            rock,
            wood,
            wood_size,
        }
    }

    fn sample(&self, x: f32, z: f32) -> Option<usize> {
        let i = ((x + self.half.0) / self.cell_m).round();
        let j = ((z + self.half.1) / self.cell_m).round();
        let (sx, sy) = self.samples;
        (i >= 0.0 && j >= 0.0 && i < sx as f32 && j < sy as f32)
            .then(|| j as usize * sx as usize + i as usize)
    }

    /// `value` of the map samples bilinearly interpolated at a point
    /// (clamped to the map).
    pub fn interpolate(&self, x: f32, z: f32, value: impl Fn(usize) -> f32) -> f32 {
        let (sx, sy) = (self.samples.0 as usize, self.samples.1 as usize);
        let gx = ((x + self.half.0) / self.cell_m).clamp(0.0, (sx - 1) as f32);
        let gz = ((z + self.half.1) / self.cell_m).clamp(0.0, (sy - 1) as f32);
        let (i, j) = (
            (gx as usize).min(sx.saturating_sub(2)),
            (gz as usize).min(sy.saturating_sub(2)),
        );
        let (fx, fz) = (gx - i as f32, gz - j as f32);
        let v = |a: usize, b: usize| value(b.min(sy - 1) * sx + a.min(sx - 1));
        let top = v(i, j) + (v(i + 1, j) - v(i, j)) * fx;
        let bottom = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * fx;
        top + (bottom - top) * fz
    }

    /// Wood density 0..1 at a point, bilinear over the wood cells.
    pub fn wood_at(&self, x: f32, z: f32) -> f32 {
        let (w, h) = self.wood_size;
        let gx = ((x + self.half.0) / WOOD_CELL_M - 0.5).clamp(0.0, (w - 1) as f32);
        let gz = ((z + self.half.1) / WOOD_CELL_M - 0.5).clamp(0.0, (h - 1) as f32);
        let (i, j) = (
            (gx as usize).min(w.saturating_sub(2)),
            (gz as usize).min(h.saturating_sub(2)),
        );
        let (fx, fz) = (gx - i as f32, gz - j as f32);
        let v = |a: usize, b: usize| self.wood[(b.min(h - 1)) * w + a.min(w - 1)];
        let top = v(i, j) + (v(i + 1, j) - v(i, j)) * fx;
        let bottom = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * fx;
        top + (bottom - top) * fz
    }
}

/// Parameters of one decoration layer (e.g. grass clumps).
pub struct DecoLayer<'a> {
    /// Lattice spacing in meters: at most one instance per lattice point.
    pub spacing: f32,
    pub seed: u64,
    /// Probability per terrain id (missing ids: 0).
    pub density: &'a [f32],
    /// Density multiplier in dense wood (blended by wood density).
    pub wood_factor: f32,
    /// 0: uniform; 1: strongly clumped into patches (about 25 m).
    pub patchiness: f32,
    /// Relative weights of the model variants.
    pub weights: &'a [f32],
    pub scale: (f32, f32),
    /// Sink into the ground (m).
    pub sink: f32,
    /// Signed shore band `(from, to)` in meters (negative: in the water)
    /// the layer is limited to; `(0, 0)`: everywhere.
    pub shore: (f32, f32),
    /// Frayed edges (prompt 073), meters; 0: the density of the nearest
    /// sample. Above 0 the density is interpolated between the four
    /// samples around a point that is moved by noise of this amplitude, so
    /// the edge of a stand does not follow the 2 m grid, and instances get
    /// shorter towards the edge ([`edge_scale`]).
    pub fray: f32,
    /// Extra density in the seam around rocks (prompt 074), added on soft
    /// ground (grass, sand, mud, snow, tall grass).
    pub rock: f32,
}

/// Scale factor of an instance at the edge of a frayed layer: `coverage`
/// is the interpolated density over the largest density of the layer
/// (0..1). Short at the very edge, full height inside.
pub fn edge_scale(coverage: f32) -> f32 {
    let t = ((coverage - 0.3) / 0.65).clamp(0.0, 1.0);
    0.4 + 0.6 * t * t * (3.0 - 2.0 * t)
}

/// Value noise in 0..1 on a lattice of `cell` meters, from hashes.
fn noise(seed: u64, x: f32, z: f32, cell: f32) -> f32 {
    let (gx, gz) = (x / cell, z / cell);
    let (i, j) = (gx.floor(), gz.floor());
    let (fx, fz) = (gx - i, gz - j);
    let (fx, fz) = (fx * fx * (3.0 - 2.0 * fx), fz * fz * (3.0 - 2.0 * fz));
    let v = |a: f32, b: f32| {
        unit(mix(seed
            ^ mix(
                (a as i64).cast_unsigned() ^ mix((b as i64).cast_unsigned())
            )))
    };
    let top = v(i, j) + (v(i + 1.0, j) - v(i, j)) * fx;
    let bottom = v(i, j + 1.0) + (v(i + 1.0, j + 1.0) - v(i, j + 1.0)) * fx;
    top + (bottom - top) * fz
}

/// Decoration instances of one layer in the square `x0..x0 + size`,
/// `z0..z0 + size`, one `MultiMesh` buffer per variant. Only lattice points
/// inside the square count, so neighboring squares never overlap. Samples
/// for which `deformed(index)` holds (craters, prompt 021) get nothing.
#[allow(clippy::too_many_arguments)]
pub fn scatter(
    map: &Map,
    rasters: &DecoRasters,
    water: Option<&crate::water::WaterField>,
    layer: &DecoLayer,
    x0: f32,
    z0: f32,
    size: f32,
    deformed: impl Fn(usize) -> bool,
) -> Vec<Vec<f32>> {
    let mut out = vec![Vec::new(); layer.weights.len().max(1)];
    let total: f32 = layer.weights.iter().sum();
    let densest = layer.density.iter().copied().fold(0.0f32, f32::max);
    let density_of = |k: usize| {
        layer
            .density
            .get(usize::from(map.terrain_ids()[k]))
            .copied()
            .unwrap_or(0.0)
    };
    let s = layer.spacing.max(0.1);
    let (gx0, gx1) = ((x0 / s).ceil() as i64, ((x0 + size) / s).ceil() as i64);
    let (gz0, gz1) = ((z0 / s).ceil() as i64, ((z0 + size) / s).ceil() as i64);
    let size = map.size();
    let (half_x, half_z) = (f(size.x) / 2.0, f(size.y) / 2.0);
    for gz in gz0..gz1 {
        for gx in gx0..gx1 {
            let h = mix(layer.seed ^ mix(gx.cast_unsigned() ^ mix(gz.cast_unsigned())));
            let x = (gx as f32 + unit(mix(h ^ 1)) - 0.5) * s;
            let z = (gz as f32 + unit(mix(h ^ 2)) - 0.5) * s;
            if x.abs() > half_x - 1.0 || z.abs() > half_z - 1.0 {
                continue;
            }
            let Some(k) = rasters.sample(x, z) else {
                continue;
            };
            if rasters.blocked[k] || deformed(k) {
                continue;
            }
            let mut density = density_of(k);
            let mut edge = 1.0;
            if layer.fray > 0.0 {
                // Frayed edge: interpolated density at a point moved by
                // noise (3.1 m cells, two noises for x and z).
                let wx = x + (noise(layer.seed ^ 0xf7a1, x, z, 3.1) * 2.0 - 1.0) * layer.fray;
                let wz = z + (noise(layer.seed ^ 0xf7a2, x, z, 3.1) * 2.0 - 1.0) * layer.fray;
                density = rasters.interpolate(wx, wz, density_of);
                edge = edge_scale(density / densest.max(1e-6));
            }
            if layer.rock > 0.0 && SEAM_TERRAIN.contains(&map.terrain_ids()[k]) {
                density += layer.rock * rasters.interpolate(x, z, |i| rasters.rock[i]);
            }
            if density <= 0.0 {
                continue;
            }
            if let Some(water) = water {
                // Inside the band, fading out over one meter at its edges.
                let s = water.signed_shore(k);
                let (from, to) = layer.shore;
                density *= ((s - from + 1.0).min(to + 1.0 - s)).clamp(0.0, 1.0);
                let sx = map.samples().0 as usize;
                if density <= 0.0 || map.sample_under_deck((k % sx) as u32, (k / sx) as u32) {
                    continue;
                }
            }
            let patch = noise(layer.seed ^ 0x5eed, x, z, 25.0);
            density *= 1.0 + layer.patchiness * (patch * 2.0 - 1.0);
            let wood = rasters.wood_at(x, z);
            density *= 1.0 + (layer.wood_factor - 1.0) * wood;
            if unit(h) >= density {
                continue;
            }
            let mut pick = unit(mix(h ^ 3)) * total;
            let mut variant = 0;
            for (v, &w) in layer.weights.iter().enumerate() {
                variant = v;
                if pick < w {
                    break;
                }
                pick -= w;
            }
            let yaw = unit(mix(h ^ 4)) * std::f32::consts::TAU;
            let scale = (layer.scale.0 + (layer.scale.1 - layer.scale.0) * unit(mix(h ^ 5))) * edge;
            let y = ground(map, x, z) - layer.sink;
            push_transform(&mut out[variant], yaw, scale, [x, y, z]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::map::MapLayout;

    /// 80 m x 40 m: tall grass (id 8) west of x = 0, grass east of it.
    fn half_tall() -> Map {
        let (sx, sy) = (41u32, 21u32);
        let mut terrain = Vec::new();
        for _j in 0..sy {
            for i in 0..sx {
                terrain.push(if i < 20 { 8 } else { 0 });
            }
        }
        let layout = MapLayout {
            name: "t".into(),
            samples_x: sx,
            samples_y: sy,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        Map::from_rasters(layout, vec![0; (sx * sy) as usize], terrain).unwrap()
    }

    fn layer(density: &[f32], fray: f32) -> DecoLayer<'_> {
        DecoLayer {
            spacing: 0.5,
            seed: 7,
            density,
            wood_factor: 1.0,
            patchiness: 0.0,
            weights: &[1.0],
            scale: (1.0, 1.0),
            sink: 0.0,
            shore: (0.0, 0.0),
            fray,
            rock: 0.0,
        }
    }

    /// Instances as (x, scale).
    fn scatter_x(map: &Map, layer: &DecoLayer) -> Vec<(f32, f32)> {
        let rasters = DecoRasters::new(map);
        let out = scatter(map, &rasters, None, layer, -40.0, -20.0, 80.0, |_| false);
        out[0]
            .chunks(12)
            .map(|t| (t[3], (t[0] * t[0] + t[8] * t[8]).sqrt()))
            .collect()
    }

    #[test]
    fn frayed_edges_leave_the_grid_and_get_shorter() {
        let map = half_tall();
        let density = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        // Nearest sample: a straight edge at the sample border (x = -1 m).
        let sharp = scatter_x(&map, &layer(&density, 0.0));
        assert!(
            sharp
                .iter()
                .all(|&(x, s)| x < -0.9 && (s - 1.0).abs() < 1e-5)
        );
        // Frayed: some plants beyond the edge, some missing before it, and
        // the plants at the edge are shorter than inside.
        let frayed = scatter_x(&map, &layer(&density, 1.1));
        assert!(frayed.iter().any(|&(x, _)| x > -0.5));
        let inside = frayed.iter().filter(|&&(x, _)| x < -8.0);
        assert!(inside.clone().all(|&(_, s)| (s - 1.0).abs() < 1e-4));
        let edge: Vec<f32> = frayed
            .iter()
            .filter(|&&(x, _)| x > -0.5)
            .map(|&(_, s)| s)
            .collect();
        assert!(!edge.is_empty() && edge.iter().all(|&s| s < 1.0));
        assert!(edge.iter().all(|&s| s >= 0.4 - 1e-5));
    }

    #[test]
    fn edge_scale_is_short_at_the_edge_and_full_inside() {
        assert!((edge_scale(0.0) - 0.4).abs() < 1e-6);
        assert!((edge_scale(1.0) - 1.0).abs() < 1e-6);
        assert!(edge_scale(0.5) > 0.4 && edge_scale(0.5) < 1.0);
    }
}
