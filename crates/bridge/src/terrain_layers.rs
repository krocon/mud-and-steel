//! Rasters for the terrain renderer (presentation only, floats allowed).
//!
//! The renderer in `godot/scenes/terrain/terrain.gd` keeps one texel per map
//! sample in a few GPU textures. This module computes their content for any
//! rectangle of samples, so a change (a crater in prompt 021) only recomputes
//! and uploads the rectangle around it:
//!
//! - heights in meters (`f32`)
//! - normals (RG8: x and z of the unit normal, mapped to 0..255)
//! - splat weights of terrain types 0..=3 and 4..=7 (two RGBA8 rasters,
//!   3 x 3 tent filter over the nearest-sample terrain ids, so borders blend
//!   over about two samples instead of stepping)
//! - field track direction (RGBA8): RG = doubled angle of the direction along
//!   the track (cos 2a, sin 2a mapped to 0..255; doubled, so that a and a + 180
//!   degrees are the same and the field can be filtered), B = blurred track
//!   mask, A = 255. The direction comes from the smoothed structure tensor of
//!   the track mask, which is well defined on the center line too.
//!   Tall grass (prompt 073) has no splat weight of its own (the shader has
//!   eight): it counts as grass there, and A holds its mask (3 x 3 tent
//!   filter like the weights), which the shader uses for the dark ground
//!   under the tufts and the far look of tall grass.
//! - fields (RGBA8, prompt 088): R, G, B = masks of grain, green fields and
//!   ploughed soil (3 x 3 tent filter, the fields count as grass in the
//!   splat weights), A = direction of the rows: the long axis of the
//!   connected field the sample belongs to (from the second moments of its
//!   samples), as an angle in 0..180 degrees mapped to 0..255. The
//!   direction is presentation only and is not stored in the map.

use std::collections::HashMap;

/// Terrain id of field tracks (`data/terrain.toml`).
const DIRT_TRACK: u8 = 1;
/// Terrain id of tall grass (`data/terrain.toml`, prompt 073).
const TALL_GRASS: u8 = 8;
/// Terrain ids of the fields (prompt 088): grain, green, ploughed.
const FIELDS: [u8; 3] = [9, 10, 11];
/// Box blur radius (samples) of the structure tensor; applied twice.
const TENSOR_RADIUS: i64 = 3;
/// Extra samples around a rectangle so blurs are exact inside it.
const MARGIN: i64 = 2 * TENSOR_RADIUS + 2;

/// Read-only view of the map rasters plus optional display offsets.
pub struct Grid<'a> {
    pub samples: (u32, u32),
    pub cell_m: f32,
    pub heights_mm: &'a [i32],
    pub ids: &'a [u8],
    /// Presentation-only height offsets in meters (debug craters), same
    /// layout as `heights_mm`, or empty.
    pub offsets: &'a [f32],
}

/// A rectangle of samples: first column and row, width and height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub i0: u32,
    pub j0: u32,
    pub w: u32,
    pub h: u32,
}

/// The rasters of one rectangle, row by row.
pub struct Layers {
    pub heights: Vec<f32>,
    pub normals: Vec<u8>,
    pub splat0: Vec<u8>,
    pub splat1: Vec<u8>,
    pub track: Vec<u8>,
    pub field: Vec<u8>,
}

impl Grid<'_> {
    fn clamp(&self, i: i64, j: i64) -> usize {
        let i = i.clamp(0, i64::from(self.samples.0) - 1);
        let j = j.clamp(0, i64::from(self.samples.1) - 1);
        (j * i64::from(self.samples.0) + i) as usize
    }

    pub fn height(&self, i: i64, j: i64) -> f32 {
        let k = self.clamp(i, j);
        let offset = self.offsets.get(k).copied().unwrap_or(0.0);
        self.heights_mm[k] as f32 / 1000.0 + offset
    }

    fn id(&self, i: i64, j: i64) -> u8 {
        self.ids[self.clamp(i, j)]
    }

    /// Clips a rectangle to the map; `None` if nothing is left.
    pub fn clip(&self, i0: i64, j0: i64, w: i64, h: i64) -> Option<Rect> {
        let (sx, sy) = (i64::from(self.samples.0), i64::from(self.samples.1));
        let (a, b) = (i0.max(0), j0.max(0));
        let (c, d) = ((i0 + w).min(sx), (j0 + h).min(sy));
        (c > a && d > b).then(|| Rect {
            i0: a as u32,
            j0: b as u32,
            w: (c - a) as u32,
            h: (d - b) as u32,
        })
    }

    pub fn layers(&self, rect: Rect) -> Layers {
        let n = (rect.w * rect.h) as usize;
        let mut out = Layers {
            heights: Vec::with_capacity(n),
            normals: Vec::with_capacity(n * 2),
            splat0: Vec::with_capacity(n * 4),
            splat1: Vec::with_capacity(n * 4),
            track: Vec::with_capacity(n * 4),
            field: Vec::with_capacity(n * 4),
        };
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        for j in rect.j0..rect.j0 + rect.h {
            for i in rect.i0..rect.i0 + rect.w {
                let (i, j) = (i64::from(i), i64::from(j));
                out.heights.push(self.height(i, j));
                let dx = (self.height(i + 1, j) - self.height(i - 1, j)) / (2.0 * self.cell_m);
                let dz = (self.height(i, j + 1) - self.height(i, j - 1)) / (2.0 * self.cell_m);
                let len = (dx * dx + dz * dz + 1.0).sqrt();
                out.normals.push(byte(-dx / len * 0.5 + 0.5));
                out.normals.push(byte(-dz / len * 0.5 + 0.5));
                let mut weights = [0u32; 8];
                for (oj, wj) in [(-1, 1), (0, 2), (1, 1)] {
                    for (oi, wi) in [(-1, 1), (0, 2), (1, 1)] {
                        let id = self.id(i + oi, j + oj) as usize;
                        // Unknown ids and tall grass count as grass.
                        weights[if id < 8 { id } else { 0 }] += wi * wj;
                    }
                }
                for (k, w) in weights.iter().enumerate() {
                    let v = ((w * 255 + 8) / 16) as u8;
                    if k < 4 {
                        out.splat0.push(v);
                    } else {
                        out.splat1.push(v);
                    }
                }
            }
        }
        self.track_field(rect, &mut out.track);
        self.fields(rect, &mut out.field);
        out
    }

    /// Field masks and row directions, see the module documentation.
    fn fields(&self, rect: Rect, out: &mut Vec<u8>) {
        let (sx, sy) = (i64::from(self.samples.0), i64::from(self.samples.1));
        // Row direction per sample of every field that reaches the
        // rectangle: each connected field is filled once.
        // (The rectangle plus one sample, for the filtered edges.)
        let mut angle: HashMap<usize, u8> = HashMap::new();
        let (i0, j0) = (i64::from(rect.i0) - 1, i64::from(rect.j0) - 1);
        let (i1, j1) = (
            i64::from(rect.i0 + rect.w) + 1,
            i64::from(rect.j0 + rect.h) + 1,
        );
        for j in j0.max(0)..j1.min(sy) {
            for i in i0.max(0)..i1.min(sx) {
                let id = self.id(i, j);
                let k = self.clamp(i, j);
                if !FIELDS.contains(&id) || angle.contains_key(&k) {
                    continue;
                }
                let mut stack = vec![(i, j)];
                let mut members = Vec::new();
                let mut seen = std::collections::HashSet::new();
                seen.insert(k);
                let (mut n, mut mx, mut my) = (0.0f64, 0.0f64, 0.0f64);
                let (mut xx, mut yy, mut xy) = (0.0f64, 0.0f64, 0.0f64);
                while let Some((a, b)) = stack.pop() {
                    members.push(self.clamp(a, b));
                    let (fa, fb) = (a as f64, b as f64);
                    n += 1.0;
                    mx += fa;
                    my += fb;
                    xx += fa * fa;
                    yy += fb * fb;
                    xy += fa * fb;
                    for (da, db) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (c, d) = (a + da, b + db);
                        if c < 0 || d < 0 || c >= sx || d >= sy || self.id(c, d) != id {
                            continue;
                        }
                        if seen.insert(self.clamp(c, d)) {
                            stack.push((c, d));
                        }
                    }
                }
                let (mx, my) = (mx / n, my / n);
                let (cxx, cyy, cxy) = (xx / n - mx * mx, yy / n - my * my, xy / n - mx * my);
                // Direction of the long axis, 0..pi.
                let mut a = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
                if a < 0.0 {
                    a += std::f64::consts::PI;
                }
                let byte = ((a / std::f64::consts::PI) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                for m in members {
                    angle.insert(m, byte);
                }
            }
        }
        for j in rect.j0..rect.j0 + rect.h {
            for i in rect.i0..rect.i0 + rect.w {
                let (i, j) = (i64::from(i), i64::from(j));
                let mut sums = [0u32; 3];
                for (oj, wj) in [(-1, 1), (0, 2), (1, 1)] {
                    for (oi, wi) in [(-1, 1), (0, 2), (1, 1)] {
                        let id = self.id(i + oi, j + oj);
                        if let Some(f) = FIELDS.iter().position(|&x| x == id) {
                            sums[f] += wi * wj;
                        }
                    }
                }
                for sum in sums {
                    out.push(((sum * 255 + 8) / 16) as u8);
                }
                // Outside a field: the direction of a field next to it, so
                // the filtered edge keeps its rows.
                let mut a = angle.get(&self.clamp(i, j)).copied();
                if a.is_none() {
                    for (oi, oj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if FIELDS.contains(&self.id(i + oi, j + oj)) {
                            a = angle.get(&self.clamp(i + oi, j + oj)).copied();
                            break;
                        }
                    }
                }
                out.push(a.unwrap_or(0));
            }
        }
    }

    /// Direction and mask of field tracks, see the module documentation.
    fn track_field(&self, rect: Rect, out: &mut Vec<u8>) {
        let (sx, sy) = (i64::from(self.samples.0), i64::from(self.samples.1));
        // Work area: the rectangle plus a margin, clipped to the map.
        let a0 = (i64::from(rect.i0) - MARGIN).max(0);
        let b0 = (i64::from(rect.j0) - MARGIN).max(0);
        let a1 = (i64::from(rect.i0 + rect.w) + MARGIN).min(sx);
        let b1 = (i64::from(rect.j0 + rect.h) + MARGIN).min(sy);
        let (w, h) = ((a1 - a0) as usize, (b1 - b0) as usize);
        let mask = |i: i64, j: i64| f32::from(u8::from(self.id(i, j) == DIRT_TRACK));
        let tall = |i: i64, j: i64| u32::from(self.id(i, j) == TALL_GRASS);
        let mut jxx = vec![0.0f32; w * h];
        let mut jyy = vec![0.0f32; w * h];
        let mut jxy = vec![0.0f32; w * h];
        let mut wear = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let (i, j) = (a0 + x as i64, b0 + y as i64);
                // Sobel gradient of the mask.
                let gx = (mask(i + 1, j - 1) + 2.0 * mask(i + 1, j) + mask(i + 1, j + 1))
                    - (mask(i - 1, j - 1) + 2.0 * mask(i - 1, j) + mask(i - 1, j + 1));
                let gy = (mask(i - 1, j + 1) + 2.0 * mask(i, j + 1) + mask(i + 1, j + 1))
                    - (mask(i - 1, j - 1) + 2.0 * mask(i, j - 1) + mask(i + 1, j - 1));
                let k = y * w + x;
                jxx[k] = gx * gx;
                jyy[k] = gy * gy;
                jxy[k] = gx * gy;
                wear[k] = mask(i, j);
            }
        }
        for field in [&mut jxx, &mut jyy, &mut jxy] {
            for _ in 0..2 {
                box_blur(field, w, h, TENSOR_RADIUS);
            }
        }
        box_blur(&mut wear, w, h, 1);
        for j in rect.j0..rect.j0 + rect.h {
            for i in rect.i0..rect.i0 + rect.w {
                let (x, y) = ((i64::from(i) - a0) as usize, (i64::from(j) - b0) as usize);
                let k = y * w + x;
                // Doubled angle of the gradient (across the track); along the
                // track is 90 degrees off, i.e. the doubled angle negated.
                let (c, s) = (-(jxx[k] - jyy[k]), -2.0 * jxy[k]);
                let len = (c * c + s * s).sqrt();
                let (c, s) = if len > 1e-6 {
                    (c / len, s / len)
                } else {
                    (1.0, 0.0)
                };
                out.push(((c * 0.5 + 0.5) * 255.0 + 0.5) as u8);
                out.push(((s * 0.5 + 0.5) * 255.0 + 0.5) as u8);
                out.push((wear[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
                // Tall grass mask, 3 x 3 tent filter (weights sum to 16).
                let (i, j) = (i64::from(i), i64::from(j));
                let mut sum = 0;
                for (oj, wj) in [(-1, 1), (0, 2), (1, 1)] {
                    for (oi, wi) in [(-1, 1), (0, 2), (1, 1)] {
                        sum += tall(i + oi, j + oj) * wi * wj;
                    }
                }
                out.push(((sum * 255 + 8) / 16) as u8);
            }
        }
    }

    /// Lowest and highest height of every chunk of `cells` x `cells` cells,
    /// row by row (chunks along x first). The last chunks may stick out of
    /// the map.
    pub fn chunk_ranges(&self, cells: u32) -> Vec<(f32, f32)> {
        let cells = cells.max(1);
        let (cx, cy) = self.chunk_count(cells);
        let mut out = Vec::with_capacity((cx * cy) as usize);
        for c in 0..cy {
            for r in 0..cx {
                out.push(self.chunk_range(cells, r, c));
            }
        }
        out
    }

    /// Like [`Self::chunk_ranges`], only for the chunks that contain a
    /// sample of `rect` (craters, prompt 021): `(index, lowest, highest)`.
    pub fn chunk_ranges_in(&self, cells: u32, rect: Rect) -> Vec<(u32, f32, f32)> {
        let cells = cells.max(1);
        let (cx, cy) = self.chunk_count(cells);
        // A sample on a chunk border belongs to both chunks.
        let first = |v: u32| v.saturating_sub(1) / cells;
        let last = |v: u32, n: u32| (v / cells).min(n - 1);
        let mut out = Vec::new();
        for c in first(rect.j0)..=last(rect.j0 + rect.h.max(1) - 1, cy) {
            for r in first(rect.i0)..=last(rect.i0 + rect.w.max(1) - 1, cx) {
                let (lo, hi) = self.chunk_range(cells, r, c);
                out.push((c * cx + r, lo, hi));
            }
        }
        out
    }

    fn chunk_count(&self, cells: u32) -> (u32, u32) {
        let (sx, sy) = self.samples;
        ((sx - 1).div_ceil(cells), (sy - 1).div_ceil(cells))
    }

    fn chunk_range(&self, cells: u32, r: u32, c: u32) -> (f32, f32) {
        let (sx, sy) = self.samples;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for j in c * cells..=((c + 1) * cells).min(sy - 1) {
            for i in r * cells..=((r + 1) * cells).min(sx - 1) {
                let v = self.height(i64::from(i), i64::from(j));
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        (lo, hi)
    }
}

/// Separable box blur with clamped edges, in place.
fn box_blur(field: &mut [f32], w: usize, h: usize, radius: i64) {
    let mut tmp = vec![0.0f32; field.len()];
    let norm = 1.0 / (2 * radius + 1) as f32;
    let clamp = |v: i64, n: usize| v.clamp(0, n as i64 - 1) as usize;
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for o in -radius..=radius {
                sum += field[y * w + clamp(x as i64 + o, w)];
            }
            tmp[y * w + x] = sum * norm;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for o in -radius..=radius {
                sum += tmp[clamp(y as i64 + o, h) * w + x];
            }
            field[y * w + x] = sum * norm;
        }
    }
}

/// Bowl-shaped dent with a raised rim (presentation-only test crater):
/// adds to `offsets` around `(ci, cj)` (sample coordinates) and returns the
/// changed rectangle.
pub fn dent(
    samples: (u32, u32),
    offsets: &mut [f32],
    center: (f32, f32),
    radius: f32,
    depth: f32,
) -> Option<Rect> {
    let (sx, sy) = (i64::from(samples.0), i64::from(samples.1));
    let reach = radius * 1.5;
    let i0 = ((center.0 - reach).floor() as i64).max(0);
    let j0 = ((center.1 - reach).floor() as i64).max(0);
    let i1 = ((center.0 + reach).ceil() as i64).min(sx - 1);
    let j1 = ((center.1 + reach).ceil() as i64).min(sy - 1);
    if i1 < i0 || j1 < j0 {
        return None;
    }
    for j in j0..=j1 {
        for i in i0..=i1 {
            let d = ((i as f32 - center.0).powi(2) + (j as f32 - center.1).powi(2)).sqrt() / radius;
            let change = if d < 1.0 {
                -depth * (1.0 - d * d) + depth * 0.25 * d * d
            } else if d < 1.5 {
                // Rim falling off to zero.
                depth * 0.25 * (1.0 - (d - 1.0) / 0.5).powi(2)
            } else {
                0.0
            };
            offsets[(j * sx + i) as usize] += change;
        }
    }
    Some(Rect {
        i0: i0 as u32,
        j0: j0 as u32,
        w: (i1 - i0 + 1) as u32,
        h: (j1 - j0 + 1) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid<'a>(heights: &'a [i32], ids: &'a [u8], n: u32) -> Grid<'a> {
        Grid {
            samples: (n, n),
            cell_m: 2.0,
            heights_mm: heights,
            ids,
            offsets: &[],
        }
    }

    #[test]
    fn flat_ground_has_upward_normals_and_full_weights() {
        let n = 8;
        let heights = vec![1500; 64];
        let ids = vec![3u8; 64];
        let layers = grid(&heights, &ids, n).layers(Rect {
            i0: 2,
            j0: 1,
            w: 3,
            h: 2,
        });
        assert_eq!(layers.heights, vec![1.5; 6]);
        assert!(layers.normals.iter().all(|&v| v == 128));
        // Sand (id 3) is channel A of the first raster.
        assert!(layers.splat0.chunks(4).all(|c| c == [0, 0, 0, 255]));
        assert!(layers.splat1.iter().all(|&v| v == 0));
    }

    #[test]
    fn tall_grass_counts_as_grass_and_has_a_mask() {
        let n = 8;
        let heights = vec![0; 64];
        // Left half tall grass (8), right half grass.
        let ids: Vec<u8> = (0..64).map(|k| if k % 8 < 4 { 8 } else { 0 }).collect();
        let layers = grid(&heights, &ids, n).layers(Rect {
            i0: 1,
            j0: 3,
            w: 6,
            h: 1,
        });
        // Every sample is all grass in the splat weights.
        assert!(layers.splat0.chunks(4).all(|c| c == [255, 0, 0, 0]));
        assert!(layers.splat1.iter().all(|&v| v == 0));
        // Mask in A: full inside, half on the border, none beyond.
        let mask: Vec<u8> = layers.track.chunks(4).map(|c| c[3]).collect();
        assert_eq!(mask, vec![255, 255, 191, 64, 0, 0]);
    }

    #[test]
    fn slope_tilts_the_normal_downhill() {
        let n = 8u32;
        // Rising 1 m per 2 m cell towards +x.
        let heights: Vec<i32> = (0..64).map(|k| (k % 8) * 1000).collect();
        let ids = vec![0u8; 64];
        let layers = grid(&heights, &ids, n).layers(Rect {
            i0: 3,
            j0: 3,
            w: 1,
            h: 1,
        });
        // n = (-0.5, 1, 0) / |..| -> x component -0.447 -> 0.2764 * 255 -> byte 70.
        assert_eq!(layers.normals, vec![70, 128]);
    }

    #[test]
    fn track_direction_follows_a_straight_track() {
        // A track along z (column 10): the direction along it is z, i.e. the
        // doubled angle is 180 degrees -> cos = -1 (byte 0), sin = 0 (128).
        let n = 21u32;
        let heights = vec![0; (n * n) as usize];
        let ids: Vec<u8> = (0..n * n).map(|k| u8::from(k % n == 10)).collect();
        let layers = grid(&heights, &ids, n).layers(Rect {
            i0: 10,
            j0: 10,
            w: 1,
            h: 1,
        });
        assert!(layers.track[0] < 10, "{:?}", &layers.track[..4]);
        assert!((120..=136).contains(&layers.track[1]));
        assert!(layers.track[2] > 60);
    }

    #[test]
    fn region_matches_full_computation() {
        let n = 40u32;
        let heights: Vec<i32> = (0..n * n).map(|k| ((k * 7919) % 3000) as i32).collect();
        let ids: Vec<u8> = (0..n * n).map(|k| ((k / 3 + k / 41) % 5) as u8).collect();
        let g = grid(&heights, &ids, n);
        let full = g.layers(Rect {
            i0: 0,
            j0: 0,
            w: n,
            h: n,
        });
        let part = g.layers(Rect {
            i0: 12,
            j0: 20,
            w: 5,
            h: 4,
        });
        for y in 0..4 {
            for x in 0..5 {
                let (k, f) = (y * 5 + x, (20 + y) * n as usize + 12 + x);
                assert_eq!(part.heights[k], full.heights[f]);
                assert_eq!(part.track[k * 4..k * 4 + 4], full.track[f * 4..f * 4 + 4]);
                assert_eq!(part.splat0[k * 4..k * 4 + 4], full.splat0[f * 4..f * 4 + 4]);
            }
        }
    }

    #[test]
    fn dent_lowers_the_center_and_reports_its_rectangle() {
        let mut offsets = vec![0.0; 100];
        let rect = dent((10, 10), &mut offsets, (5.0, 5.0), 2.0, 1.0).unwrap();
        assert_eq!(
            rect,
            Rect {
                i0: 2,
                j0: 2,
                w: 7,
                h: 7
            }
        );
        assert!((offsets[55] + 1.0).abs() < 1e-6);
        assert_eq!(offsets[0], 0.0);
    }

    #[test]
    fn chunk_ranges_cover_the_map() {
        let n = 9u32;
        let heights: Vec<i32> = (0..81).map(|k| k * 10).collect();
        let ids = vec![0u8; 81];
        let ranges = grid(&heights, &ids, n).chunk_ranges(4);
        assert_eq!(ranges.len(), 4);
        assert_eq!(ranges[0], (0.0, 0.4));
        assert_eq!(ranges[3], (0.4, 0.8));
    }

    #[test]
    fn chunk_ranges_in_match_the_full_computation() {
        let n = 17u32;
        let heights: Vec<i32> = (0..n * n).map(|k| ((k * 7919) % 3000) as i32).collect();
        let ids = vec![0u8; (n * n) as usize];
        let g = grid(&heights, &ids, n);
        let all = g.chunk_ranges(4);
        // Samples 4..=8 x 9..=9: chunks x 0..=2 (4 and 8 lie on borders), y 2.
        let part = g.chunk_ranges_in(
            4,
            Rect {
                i0: 4,
                j0: 9,
                w: 5,
                h: 1,
            },
        );
        let indices: Vec<u32> = part.iter().map(|p| p.0).collect();
        assert_eq!(indices, vec![8, 9, 10]);
        for (k, lo, hi) in part {
            assert_eq!(all[k as usize], (lo, hi));
        }
    }
}
