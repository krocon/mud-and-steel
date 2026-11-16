//! Object placement of `cli gen-map`: woods, hedgerows, villages with yards
//! and orchards, an avenue and telegraph poles along the road, single trees,
//! haystacks, rocks, the bridges and stones in the river. Writes
//! `objects.csv` (format: `docs/MAPS.md`).
//!
//! Uses its own random stream, so heights and terrain stay exactly as
//! before. Like the rest of the generator it only uses basic arithmetic and
//! `sqrt`; angles come from the simulation's integer sine table, so the same
//! seed gives the same file everywhere.

use super::{
    CELL_M, DEEP_WATER, DIRT_TRACK, FORD_HALF_WIDTH_M, Layout, MUD, Path2, PlacedBridge, ROAD,
    Random, SAND, SHALLOW_WATER, ValueNoise, coord,
};
use sim::objects::{CSV_HEADER, sin_cos_deg};
use std::fmt::Write as _;

/// Seed offset of the object stream ("objects").
const SEED_SALT: u64 = 0x006f_626a_6563_7473;

/// Trees in woods stand on a jittered grid with this spacing (m).
const WOOD_SPACING: f64 = 6.5;
/// Trunks keep this distance from the edge of tracks and roads (m).
const WAY_CLEARANCE: f64 = 6.0;
/// Hedges run this far from the center of a track (m).
const HEDGE_OFFSET: f64 = 5.5;
/// Distance between hedge segments: a little less than their length
/// (catalog `env_hedge`, 3 m), so the rounded ends overlap on curves.
const HEDGE_STEP: f64 = 2.7;
/// Dry-stone wall and fence segment lengths (catalog).
const WALL_SEGMENT: f64 = 3.0;
const FENCE_SEGMENT: f64 = 2.5;

struct Object {
    kind: &'static str,
    variant: &'static str,
    x: f64,
    y: f64,
    heading: u16,
}

/// Count per kind for the report.
pub struct Summary {
    pub total: usize,
    pub trees: usize,
    pub hedges: usize,
    pub buildings: usize,
}

/// `(sin, cos)` of whole degrees from the simulation's table.
fn sin_cos(deg: u16) -> (f64, f64) {
    let (s, c) = sin_cos_deg(deg);
    (s.to_num::<f64>(), c.to_num::<f64>())
}

/// Forward axis of an object with this heading (model +Y): 0 = north (-y).
fn forward(deg: u16) -> (f64, f64) {
    let (s, c) = sin_cos(deg);
    (s, -c)
}

/// The whole-degree heading whose forward axis is closest to `(dx, dy)`.
fn heading_of(dx: f64, dy: f64) -> u16 {
    let mut best = (f64::MIN, 0);
    for deg in 0..360u16 {
        let (fx, fy) = forward(deg);
        let dot = fx * dx + fy * dy;
        if dot > best.0 {
            best = (dot, deg);
        }
    }
    best.1
}

/// Distance transform on the sample grid (chamfer 3-4, in meters).
fn distance_to(samples: usize, hit: impl Fn(usize) -> bool) -> Vec<f32> {
    let far = f32::MAX / 4.0;
    let mut d: Vec<f32> = (0..samples * samples)
        .map(|k| if hit(k) { 0.0 } else { far })
        .collect();
    let (a, b) = (CELL_M as f32, CELL_M as f32 * 1.4);
    let n = samples;
    for j in 0..n {
        for i in 0..n {
            let mut v = d[j * n + i];
            if i > 0 {
                v = v.min(d[j * n + i - 1] + a);
            }
            if j > 0 {
                v = v.min(d[(j - 1) * n + i] + a);
                if i > 0 {
                    v = v.min(d[(j - 1) * n + i - 1] + b);
                }
                if i + 1 < n {
                    v = v.min(d[(j - 1) * n + i + 1] + b);
                }
            }
            d[j * n + i] = v;
        }
    }
    for j in (0..n).rev() {
        for i in (0..n).rev() {
            let mut v = d[j * n + i];
            if i + 1 < n {
                v = v.min(d[j * n + i + 1] + a);
            }
            if j + 1 < n {
                v = v.min(d[(j + 1) * n + i] + a);
                if i + 1 < n {
                    v = v.min(d[(j + 1) * n + i + 1] + b);
                }
                if i > 0 {
                    v = v.min(d[(j + 1) * n + i - 1] + b);
                }
            }
            d[j * n + i] = v;
        }
    }
    d
}

struct Placer<'a> {
    layout: &'a Layout,
    samples: usize,
    heights: &'a [f64],
    terrain: &'a [u8],
    /// Distance to the nearest track or road sample.
    way: Vec<f32>,
    /// Distance to the nearest sand or water sample.
    wet: Vec<f32>,
    /// Samples covered by placed objects (plus a small margin).
    taken: Vec<bool>,
    /// Keep-out: the summit of the landmark hill and the way up from the
    /// meadow (self-test and demo drive there).
    summit: (f64, f64),
    objects: Vec<Object>,
}

impl Placer<'_> {
    fn index(&self, x: f64, y: f64) -> Option<usize> {
        let half = self.layout.half;
        let i = ((x + half) / CELL_M).round();
        let j = ((y + half) / CELL_M).round();
        let n = self.samples as f64;
        (i >= 0.0 && j >= 0.0 && i < n && j < n).then(|| j as usize * self.samples + i as usize)
    }

    fn height(&self, x: f64, y: f64) -> f64 {
        self.index(x, y).map_or(0.0, |k| self.heights[k])
    }

    /// Whether `(x, y)` is open ground for an object: inside the map, on
    /// grass or mud, `clear` meters from tracks, away from water, the
    /// meadow, the village squares and the keep-out corridor, and not taken.
    fn open(&self, x: f64, y: f64, clear: f64) -> bool {
        let l = self.layout;
        if x.abs() > l.half - 6.0 || y.abs() > l.half - 6.0 {
            return false;
        }
        let Some(k) = self.index(x, y) else {
            return false;
        };
        let t = self.terrain[k];
        if t != super::GRASS && t != MUD {
            return false;
        }
        if f64::from(self.way[k]) < clear || self.wet[k] < 8.0 || self.taken[k] {
            return false;
        }
        if (x * x + y * y).sqrt() < l.meadow_radius + 15.0 {
            return false;
        }
        if l.squares.iter().any(|s| {
            let d = ((x - s.x) * (x - s.x) + (y - s.y) * (y - s.y)).sqrt();
            d < s.radius + 6.0
        }) {
            return false;
        }
        let (sx, sy) = self.summit;
        if ((x - sx) * (x - sx) + (y - sy) * (y - sy)).sqrt() < 35.0 {
            return false;
        }
        let corridor = Path2 {
            points: vec![(0.0, 0.0), self.summit],
        };
        corridor.distance(x, y) >= 18.0
    }

    /// Marks the samples within `radius` of `(x, y)` as taken.
    fn take(&mut self, x: f64, y: f64, radius: f64) {
        let r = (radius / CELL_M).ceil() as i64;
        let half = self.layout.half;
        let ci = ((x + half) / CELL_M).round() as i64;
        let cj = ((y + half) / CELL_M).round() as i64;
        let n = self.samples as i64;
        for j in (cj - r).max(0)..=(cj + r).min(n - 1) {
            for i in (ci - r).max(0)..=(ci + r).min(n - 1) {
                let (px, py) = (coord(i as usize, half), coord(j as usize, half));
                if (px - x) * (px - x) + (py - y) * (py - y) <= radius * radius {
                    self.taken[j as usize * self.samples + i as usize] = true;
                }
            }
        }
    }

    /// Marks an oriented rectangle (half length along `heading`) as taken.
    fn take_box(&mut self, x: f64, y: f64, heading: u16, half_length: f64, half_width: f64) {
        let (fx, fy) = forward(heading);
        let steps = (half_length / 1.0).ceil() as i32;
        for s in -steps..=steps {
            let t = f64::from(s) * half_length / f64::from(steps.max(1));
            self.take(x + fx * t, y + fy * t, half_width + 1.0);
        }
    }

    fn put(&mut self, kind: &'static str, variant: &'static str, x: f64, y: f64, heading: u16) {
        self.objects.push(Object {
            kind,
            variant,
            x,
            y,
            heading: heading % 360,
        });
    }

    /// A tree with a random heading; takes its trunk area.
    fn tree(
        &mut self,
        random: &mut Random,
        kind: &'static str,
        variant: &'static str,
        x: f64,
        y: f64,
    ) {
        let heading = (random.unit() * 360.0) as u16;
        self.put(kind, variant, x, y, heading);
        self.take(x, y, 1.5);
    }

    /// Whether every sample of an oriented box is open ground.
    fn box_open(
        &self,
        x: f64,
        y: f64,
        heading: u16,
        half_length: f64,
        half_width: f64,
        clear: f64,
    ) -> bool {
        let (fx, fy) = forward(heading);
        let (rx, ry) = (-fy, fx);
        let mut low = f64::MAX;
        let mut high = f64::MIN;
        for a in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            for b in [-1.0, 0.0, 1.0] {
                let px = x + fx * half_length * a + rx * half_width * b;
                let py = y + fy * half_length * a + ry * half_width * b;
                if !self.open(px, py, clear) {
                    return false;
                }
                let h = self.height(px, py);
                low = low.min(h);
                high = high.max(h);
            }
        }
        high - low < 1.6
    }

    /// A line of boundary segments from `a` to `b` (wall, fence, hedge),
    /// each segment centered on its piece and turned along the line.
    /// Pieces on closed ground are skipped.
    fn boundary(
        &mut self,
        kind: &'static str,
        length: f64,
        a: (f64, f64),
        b: (f64, f64),
        clear: f64,
    ) -> usize {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let total = (dx * dx + dy * dy).sqrt();
        let pieces = (total / length).floor() as usize;
        if pieces == 0 {
            return 0;
        }
        let heading = heading_of(dx, dy);
        let mut placed = 0;
        for k in 0..pieces {
            let t = (k as f64 + 0.5) * length / total;
            let (x, y) = (a.0 + dx * t, a.1 + dy * t);
            if !self.open(x, y, clear) {
                continue;
            }
            self.put(kind, "Segment", x, y, heading);
            placed += 1;
        }
        // Take afterwards, so the pieces of one line do not block each other.
        for k in 0..pieces {
            let t = (k as f64 + 0.5) * length / total;
            self.take(a.0 + dx * t, a.1 + dy * t, 1.0);
        }
        placed
    }
}

/// Places all objects and returns `objects.csv` and a summary.
pub fn generate(
    layout: &Layout,
    seed: u64,
    samples: usize,
    heights: &[f64],
    terrain: &[u8],
    summit: (f64, f64),
    bridges: &[PlacedBridge],
) -> (String, Summary) {
    let mut random = Random(sim::Rng::new(seed ^ SEED_SALT));
    // Tracks and roads, but not the paved village squares.
    let in_square = |k: usize| {
        let (x, y) = (
            coord(k % samples, layout.half),
            coord(k / samples, layout.half),
        );
        layout
            .squares
            .iter()
            .any(|s| (x - s.x) * (x - s.x) + (y - s.y) * (y - s.y) < s.radius * s.radius)
    };
    let way = distance_to(samples, |k| {
        (terrain[k] == DIRT_TRACK || terrain[k] == ROAD) && !in_square(k)
    });
    let wet = distance_to(samples, |k| {
        matches!(terrain[k], SAND | SHALLOW_WATER | DEEP_WATER)
    });
    let mut p = Placer {
        layout,
        samples,
        heights,
        terrain,
        way,
        wet,
        taken: vec![false; samples * samples],
        summit,
        objects: Vec::new(),
    };

    let buildings = villages(&mut p, &mut random);
    road_side(&mut p, &mut random);
    let hedges = hedgerows(&mut p, &mut random);
    woods(&mut p, &mut random);
    fields(&mut p, &mut random);
    rocks(&mut p, &mut random);
    // Last, so everything above stays as it was before bridges existed.
    river_stones(&mut p, &mut random, bridges);

    let mut csv = format!(
        "# Generated by `cli gen-map` (docs/MAPS.md). Rules: data/objects.toml.\n{CSV_HEADER}\n"
    );
    for b in bridges {
        let _ = writeln!(
            csv,
            "{},{},{},{},90",
            b.kind,
            b.variant,
            (b.x * 1000.0).round() as i32,
            (b.y * 1000.0).round() as i32
        );
    }
    for o in &p.objects {
        let _ = writeln!(
            csv,
            "{},{},{},{},{}",
            o.kind,
            o.variant,
            (o.x * 1000.0).round() as i32,
            (o.y * 1000.0).round() as i32,
            o.heading
        );
    }
    let trees = p
        .objects
        .iter()
        .filter(|o| o.kind.starts_with("env_tree_"))
        .count();
    let summary = Summary {
        total: p.objects.len() + bridges.len(),
        trees,
        hedges,
        buildings,
    };
    (csv, summary)
}

fn pick<T: Copy>(random: &mut Random, items: &[(T, f64)]) -> T {
    let total: f64 = items.iter().map(|i| i.1).sum();
    let mut r = random.unit() * total;
    for &(item, weight) in items {
        if r < weight {
            return item;
        }
        r -= weight;
    }
    items[items.len() - 1].0
}

/// Wartime condition of a building.
fn damage(random: &mut Random) -> &'static str {
    pick(
        random,
        &[("Intact", 0.82), ("Damaged", 0.12), ("Ruin", 0.06)],
    )
}

/// Houses around every village square, turned with their long side to the
/// square; yards with a wall or fence, barns, a church in the first village,
/// orchards behind the yards, haystacks, woodpiles, crates and barrels.
fn villages(p: &mut Placer, random: &mut Random) -> usize {
    let mut count = 0;
    let squares: Vec<(f64, f64, f64)> = p
        .layout
        .squares
        .iter()
        .map(|s| (s.x, s.y, s.radius))
        .collect();
    for (v, &(sx, sy, radius)) in squares.iter().enumerate() {
        let slots = ((radius * 0.32) as usize).clamp(7, 12);
        let start = (random.unit() * 360.0) as u16;
        for slot in 0..slots {
            let angle =
                (start as usize + slot * 360 / slots + (random.unit() * 14.0) as usize) % 360;
            let angle = angle as u16;
            // Radial direction from the square center (forward of `angle`).
            let (ux, uy) = forward(angle);
            let church = v == 0 && slot == 0;
            let (kind, half_l, half_w) = if church {
                ("env_church", 10.0, 5.0)
            } else {
                pick(
                    random,
                    &[
                        (("env_house_farm", 7.0, 4.5), 0.45),
                        (("env_house_farm_brick", 6.0, 4.0), 0.3),
                        (("env_house_stone", 5.0, 4.0), 0.25),
                    ],
                )
            };
            let r = radius + half_w + 7.0;
            let (hx, hy) = (sx + ux * r, sy + uy * r);
            // Long side to the square: the length runs along the tangent.
            let heading = (angle + 90) % 360;
            if !p.box_open(hx, hy, heading, half_l, half_w, 9.0) {
                continue;
            }
            let variant = if church { "Intact" } else { damage(random) };
            p.put(kind, variant, hx, hy, heading);
            p.take_box(hx, hy, heading, half_l, half_w);
            count += 1;
            if church {
                continue;
            }
            // The yard behind the house: barn, props, a wall or fence.
            let back = r + half_w + 12.0;
            let (tx, ty) = (-uy, ux);
            if random.unit() < 0.55 {
                let (bx, by) = (sx + ux * (back + 5.0), sy + uy * (back + 5.0));
                if p.box_open(bx, by, heading, 10.0, 5.0, 8.0) {
                    p.put("env_barn", damage(random), bx, by, heading);
                    p.take_box(bx, by, heading, 10.0, 5.0);
                    count += 1;
                    let side = if random.unit() < 0.5 { -1.0 } else { 1.0 };
                    let (px, py) = (bx + tx * side * 14.0, by + ty * side * 14.0);
                    if p.open(px, py, 6.0) {
                        let variant = pick(random, &[("Rick", 0.5), ("Round", 0.5)]);
                        p.put(
                            "env_hay_bale",
                            variant,
                            px,
                            py,
                            (random.unit() * 360.0) as u16,
                        );
                        p.take(px, py, 3.0);
                    }
                }
            }
            for (kind, variants, offset) in [
                ("env_woodpile", &["Stack"][..], 0.6),
                ("env_crate", &["Crate", "Stack", "Small"][..], -0.4),
                ("env_barrel", &["Standing", "Group", "Lying"][..], 0.2),
            ] {
                if random.unit() < 0.5 {
                    let d = r + half_w + 3.0;
                    let (px, py) = (
                        sx + ux * d + tx * half_l * offset,
                        sy + uy * d + ty * half_l * offset,
                    );
                    if p.open(px, py, 5.0) {
                        let variant = variants[(random.unit() * variants.len() as f64) as usize];
                        p.put(kind, variant, px, py, heading);
                        p.take(px, py, 1.0);
                    }
                }
            }
            let (kind, length) = if random.unit() < 0.5 {
                ("env_wall_stone", WALL_SEGMENT)
            } else {
                ("env_fence_wood", FENCE_SEGMENT)
            };
            let fence_r = back + 14.0;
            let half_span = half_l + 6.0;
            let a = (
                sx + ux * fence_r - tx * half_span,
                sy + uy * fence_r - ty * half_span,
            );
            let b = (
                sx + ux * fence_r + tx * half_span,
                sy + uy * fence_r + ty * half_span,
            );
            p.boundary(kind, length, a, b, 7.0);
            // Orchard behind the yard: young trees on an 8 m grid.
            if random.unit() < 0.6 {
                for row in 0..3 {
                    for col in -2..=2 {
                        let d = fence_r + 7.0 + f64::from(row) * 8.0;
                        let t = f64::from(col) * 8.0;
                        let x = sx + ux * d + tx * t + random.range(-0.8, 0.8);
                        let y = sy + uy * d + ty * t + random.range(-0.8, 0.8);
                        if p.open(x, y, WAY_CLEARANCE) {
                            p.tree(random, "env_tree_deciduous", "OakYoung", x, y);
                        }
                    }
                }
            }
        }
    }
    count
}

/// An avenue and telegraph poles along every road, outside the villages.
fn road_side(p: &mut Placer, random: &mut Random) {
    let roads: Vec<Vec<(f64, f64)>> = p.layout.roads.iter().map(|r| r.points.clone()).collect();
    for points in roads {
        walk(&points, 14.0, |x, y, dx, dy, k| {
            let (nx, ny) = (-dy, dx);
            for side in [-1.0, 1.0] {
                // `way` is measured from the road edge (3 m from its center).
                let (tx, ty) = (x + nx * side * 8.0, y + ny * side * 8.0);
                if p.open(tx, ty, 4.0) {
                    let variant = pick(random, &[("Oak", 0.6), ("Beech", 0.4)]);
                    p.tree(random, "env_tree_deciduous", variant, tx, ty);
                }
            }
            if k % 3 == 0 {
                let (tx, ty) = (x + nx * 5.0, y + ny * 5.0);
                if p.open(tx, ty, 1.5) {
                    p.put("env_pole_telegraph", "Pole", tx, ty, heading_of(dx, dy));
                    p.take(tx, ty, 0.5);
                }
            }
        });
    }
}

/// Calls `f(x, y, dir_x, dir_y, index)` every `step` meters along a polyline.
fn walk(points: &[(f64, f64)], step: f64, mut f: impl FnMut(f64, f64, f64, f64, usize)) {
    let mut carry = step / 2.0;
    let mut index = 0;
    for pair in points.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len);
        let mut t = carry;
        while t < len {
            f(ax + ux * t, ay + uy * t, ux, uy, index);
            index += 1;
            t += step;
        }
        carry = t - len;
    }
}

/// Hedges beside the field tracks: runs of segments on either side with
/// gaps (field entrances, wide enough for tanks), now and then a tree in
/// the hedge. Returns the number of segments.
fn hedgerows(p: &mut Placer, random: &mut Random) -> usize {
    let tracks: Vec<Vec<(f64, f64)>> = p.layout.tracks.iter().map(|t| t.points.clone()).collect();
    let mut count = 0;
    for points in tracks {
        for side in [-1.0, 1.0] {
            let mut run_left = random.range(20.0, 140.0);
            let mut gap_left = 0.0;
            let mut hedge = random.unit() < 0.7;
            let mut since_tree = random.range(10.0, 30.0);
            let mut pieces = Vec::new();
            walk(&points, HEDGE_STEP, |x, y, dx, dy, _| {
                if gap_left > 0.0 {
                    gap_left -= HEDGE_STEP;
                    return;
                }
                run_left -= HEDGE_STEP;
                if run_left <= 0.0 {
                    // Switch between a hedge run and an open stretch,
                    // separated by an entrance of 9-21 m.
                    hedge = random.unit() < 0.65;
                    run_left = random.range(30.0, 160.0);
                    gap_left = random.range(9.0, 21.0);
                    return;
                }
                if !hedge {
                    return;
                }
                let (nx, ny) = (-dy * side, dx * side);
                let (hx, hy) = (x + nx * HEDGE_OFFSET, y + ny * HEDGE_OFFSET);
                since_tree -= HEDGE_STEP;
                pieces.push((hx, hy, heading_of(dx, dy), since_tree <= 0.0));
                if since_tree <= 0.0 {
                    since_tree = random.range(18.0, 40.0);
                }
            });
            for &(hx, hy, heading, tree) in &pieces {
                // `way` is measured from the track edge (1.8 m from its
                // center): hedges keep clear of crossings and other tracks.
                if !p.open(hx, hy, HEDGE_OFFSET - 2.6) {
                    continue;
                }
                p.put("env_hedge", "Segment", hx, hy, heading);
                count += 1;
                if tree {
                    let variant =
                        pick(random, &[("Oak", 0.6), ("Beech", 0.25), ("OakYoung", 0.15)]);
                    p.put(
                        "env_tree_deciduous",
                        variant,
                        hx,
                        hy,
                        (random.unit() * 360.0) as u16,
                    );
                }
            }
            for &(hx, hy, _, _) in &pieces {
                p.take(hx, hy, 1.5);
            }
        }
    }
    count
}

/// Woods from a noise mask: deciduous, conifer and birch stands, birches
/// and young trees at the edges.
fn woods(p: &mut Placer, random: &mut Random) {
    let size = p.layout.half * 2.0;
    let mask = ValueNoise::new(random, size, (size / 14.0).max(110.0));
    let detail = ValueNoise::new(random, size, 45.0);
    let species = ValueNoise::new(random, size, (size / 25.0).max(90.0));
    let threshold = 0.38;
    let half = p.layout.half;
    let n = (size / WOOD_SPACING).floor() as usize;
    for j in 0..n {
        for i in 0..n {
            let x = -half + (i as f64 + 0.5) * WOOD_SPACING + random.range(-2.4, 2.4);
            let y = -half + (j as f64 + 0.5) * WOOD_SPACING + random.range(-2.4, 2.4);
            let m = mask.at(x, y) + 0.25 * detail.at(x, y);
            if m < threshold || !p.open(x, y, WAY_CLEARANCE) {
                continue;
            }
            let edge = m < threshold + 0.08;
            let s = species.at(x, y);
            let (kind, variant) = if edge && random.unit() < 0.45 {
                pick(
                    random,
                    &[
                        (("env_tree_birch", "Birch"), 0.4),
                        (("env_tree_birch", "BirchYoung"), 0.25),
                        (("env_tree_deciduous", "OakYoung"), 0.2),
                        (("env_tree_conifer", "SpruceYoung"), 0.15),
                    ],
                )
            } else if s > 0.2 {
                pick(
                    random,
                    &[
                        (("env_tree_conifer", "Spruce"), 0.62),
                        (("env_tree_conifer", "Pine"), 0.3),
                        (("env_tree_conifer", "SpruceYoung"), 0.08),
                    ],
                )
            } else if s < -0.45 {
                pick(
                    random,
                    &[
                        (("env_tree_birch", "Birch"), 0.75),
                        (("env_tree_birch", "BirchYoung"), 0.25),
                    ],
                )
            } else {
                pick(
                    random,
                    &[
                        (("env_tree_deciduous", "Oak"), 0.45),
                        (("env_tree_deciduous", "Beech"), 0.4),
                        (("env_tree_deciduous", "OakYoung"), 0.1),
                        (("env_tree_birch", "Birch"), 0.05),
                    ],
                )
            };
            p.tree(random, kind, variant, x, y);
        }
    }
}

/// Single field trees, small groups of haystacks and a few fallen logs.
fn fields(p: &mut Placer, random: &mut Random) {
    let size = p.layout.half * 2.0;
    let half = p.layout.half;
    let area = size * size;
    for _ in 0..(area / 9000.0) as usize {
        let (x, y) = (random.range(-half, half), random.range(-half, half));
        if p.open(x, y, WAY_CLEARANCE + 2.0) {
            let variant = pick(random, &[("Oak", 0.7), ("Beech", 0.2), ("OakYoung", 0.1)]);
            p.tree(random, "env_tree_deciduous", variant, x, y);
            p.take(x, y, 6.0);
        }
    }
    for _ in 0..(area / 60_000.0) as usize {
        let (x, y) = (random.range(-half, half), random.range(-half, half));
        let stacks = 2 + (random.unit() * 4.0) as usize;
        let heading = (random.unit() * 360.0) as u16;
        let (fx, fy) = forward(heading);
        for k in 0..stacks {
            let d = k as f64 * 9.0;
            let (hx, hy) = (
                x + fx * d + random.range(-1.5, 1.5),
                y + fy * d + random.range(-1.5, 1.5),
            );
            if p.open(hx, hy, 6.0) {
                let variant = pick(random, &[("Rick", 0.6), ("Round", 0.4)]);
                p.put("env_hay_bale", variant, hx, hy, heading);
                p.take(hx, hy, 3.0);
            }
        }
    }
    for _ in 0..(area / 120_000.0) as usize {
        let (x, y) = (random.range(-half, half), random.range(-half, half));
        if p.open(x, y, 5.0) {
            p.put(
                "env_log_fallen",
                "Log",
                x,
                y,
                (random.unit() * 360.0) as u16,
            );
            p.take(x, y, 4.0);
        }
    }
}

/// Boulders on the slopes of the hills.
fn rocks(p: &mut Placer, random: &mut Random) {
    let hills: Vec<(f64, f64, f64)> = p
        .layout
        .hills
        .iter()
        .map(|h| (h.x, h.y, h.radius))
        .collect();
    for (hx, hy, radius) in hills {
        let count = (radius * radius / 3000.0) as usize;
        for _ in 0..count {
            let (x, y) = (
                hx + random.range(-radius, radius) * 0.8,
                hy + random.range(-radius, radius) * 0.8,
            );
            if p.open(x, y, 4.0) {
                let variant = pick(random, &[("Medium", 0.7), ("Large", 0.3)]);
                p.put("env_rock", variant, x, y, (random.unit() * 360.0) as u16);
                p.take(x, y, 2.5);
            }
        }
    }
}

/// Stones in the river (small and medium, they block nobody) for foam and
/// looks: groups every few meters along the river, in the water, but not
/// near bridges and fords.
fn river_stones(p: &mut Placer, random: &mut Random, bridges: &[PlacedBridge]) {
    let points = p.layout.river.points.clone();
    let mut spots = Vec::new();
    walk(&points, 7.0, |x, y, dx, dy, _| spots.push((x, y, dx, dy)));
    let ways: Vec<&Path2> = p.layout.tracks.iter().chain(&p.layout.roads).collect();
    for (x, y, dx, dy) in spots {
        if random.unit() > 0.45 {
            continue;
        }
        let w = p.layout.river_half_width(y);
        let near_bridge = bridges
            .iter()
            .any(|b| (b.y - y).abs() < 25.0 && (b.x - x).abs() < b.length);
        let near_ford = ways
            .iter()
            .any(|t| t.near(x, y, 25.0) && t.distance(x, y) < FORD_HALF_WIDTH_M + 10.0);
        if near_bridge || near_ford {
            continue;
        }
        let stones = 1 + (random.unit() * 3.0) as usize;
        for _ in 0..stones {
            let side = random.range(-1.0, 1.0) * w * 0.9;
            let along = random.range(-2.0, 2.0);
            let (sx, sy) = (x - dy * side + dx * along, y + dx * side + dy * along);
            let edge = p.layout.half - 2.0;
            if sx.abs() > edge || sy.abs() > edge {
                continue;
            }
            let Some(k) = p.index(sx, sy) else {
                continue;
            };
            if !matches!(p.terrain[k], SHALLOW_WATER | DEEP_WATER) {
                continue;
            }
            let variant = if random.unit() < 0.4 {
                "Medium"
            } else {
                "Small"
            };
            p.put("env_rock", variant, sx, sy, (random.unit() * 360.0) as u16);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Options, generate as generate_map};
    use sim::ObjectRules;
    use std::path::{Path, PathBuf};

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn generate_into(dir: &Path) -> sim::Map {
        let options = Options {
            name: "gen_test",
            size_m: 300,
            seed: 5,
            players: 2,
            biome: "forest",
            out: dir,
        };
        generate_map(&options).unwrap();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = sim::Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        map
    }

    #[test]
    fn two_runs_give_the_same_objects() {
        let base = std::env::temp_dir().join(format!("mas-gen-objects-{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        let first = generate_into(&a);
        let second = generate_into(&b);
        let csv = |dir: &Path| std::fs::read(dir.join("objects.csv")).unwrap();
        assert_eq!(csv(&a), csv(&b));
        assert_eq!(first.hash(), second.hash());
        assert!(!first.objects().is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Every variant named in `data/objects.toml` exists in the model's
    /// blueprint and vice versa.
    #[test]
    fn rules_match_the_blueprints() {
        let rules = ObjectRules::builtin().unwrap();
        let mut ids: Vec<&str> = rules.kinds.iter().map(|k| k.id.as_str()).collect();
        ids.dedup();
        for id in ids {
            let path = repo().join(format!("assets-src/blueprints/neutral/{id}.toml"));
            let text = std::fs::read_to_string(&path).unwrap();
            let blueprint: toml::Value = toml::from_str(&text).unwrap();
            let mut expected: Vec<&str> = blueprint["variant"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["name"].as_str().unwrap())
                .collect();
            let mut actual: Vec<&str> = rules
                .kinds
                .iter()
                .filter(|k| k.id == id)
                .map(|k| k.variant.as_str())
                .collect();
            expected.sort_unstable();
            actual.sort_unstable();
            assert_eq!(actual, expected, "{id}");
        }
    }
}
