//! Point/axis symmetry, trenches, wire, craters, marshes, hedgerows and camps.

use super::plan::{BridgeRow, bounds, edge_distance, inside, polyline_distance, segment_distance};
use super::schema::*;
use super::settlements::{mirror_south, way_heading};
use super::{CELL_M, DEEP_WATER, DIRT_TRACK, ROAD, SHALLOW_WATER};
use crate::gen_land::{Objects, Way, dist, heading_of, smoothstep};
use crate::gen_map::Random;
use sim::Rng;

// --- Prompt 124: point symmetry, trenches, wire, craters, marshes, camps ---

/// How a symmetric map copies its north half (prompts 123, 124).
#[derive(Clone, Copy)]
pub(super) enum Mirror {
    /// Mirrored about the line y = axis (north against south).
    North(f64),
    /// Turned by 180° about the point (x, y): point symmetry.
    Point(f64, f64),
    /// Prompt 130: mirrored about the line x = axis (west against east);
    /// the west half is the source.
    West(f64),
}

impl Mirror {
    /// The line between the two halves (y; x for [`Mirror::West`]).
    pub(super) const fn axis(self) -> f64 {
        match self {
            Self::North(a) | Self::Point(_, a) | Self::West(a) => a,
        }
    }

    /// True: the two halves are north and south (the line runs along x).
    pub(super) const fn across_y(self) -> bool {
        !matches!(self, Self::West(_))
    }

    /// The coordinate of `p` across the line (compared with [`Self::axis`];
    /// the source half lies below it).
    pub(super) const fn across(self, (x, y): (f64, f64)) -> f64 {
        if self.across_y() { y } else { x }
    }

    /// The image of `p`.
    pub(super) fn map(self, (x, y): (f64, f64)) -> (f64, f64) {
        match self {
            Self::North(a) => (x, 2.0 * a - y),
            Self::Point(cx, cy) => (2.0 * cx - x, 2.0 * cy - y),
            Self::West(a) => (2.0 * a - x, y),
        }
    }
}

/// Copies the north half of a sample raster onto the south half (see
/// [`Mirror`]). Point symmetry: every sample south of the center line (and
/// east of the center on it) takes the value of its image; the first row
/// and column have no image and stay.
pub(super) fn mirror_raster<T: Copy>(raster: &mut [T], sx: usize, half: (f64, f64), m: Mirror) {
    match m {
        Mirror::North(axis) => mirror_south(raster, sx, half.1, axis),
        // Prompt 130: every sample east of the line takes the value of its
        // image in the west half; the first column has no image and stays.
        Mirror::West(axis) => {
            let sy = raster.len() / sx;
            for i in 0..sx {
                let x = -half.0 + i as f64 * CELL_M;
                if x <= axis {
                    continue;
                }
                let si = ((2.0 * axis - x + half.0) / CELL_M).round();
                if si < 0.0 {
                    continue;
                }
                let si = si as usize;
                for j in 0..sy {
                    raster[j * sx + i] = raster[j * sx + si];
                }
            }
        }
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
pub(super) const AXIS_BAND_M: f64 = 24.0;

pub(super) fn blend_axis(heights: &mut [f64], sx: usize, half: (f64, f64), (cx, cy): (f64, f64)) {
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
pub(super) struct GenWork {
    pub(super) kind: PreparedWorkKind,
    pub(super) points: Vec<(f64, f64)>,
}

pub(super) fn polyline_len(points: &[(f64, f64)]) -> f64 {
    points.windows(2).map(|s| dist(s[0], s[1])).sum()
}

/// The point `u` meters along `points` (clamped to the ends).
pub(super) fn point_along(points: &[(f64, f64)], u: f64) -> (f64, f64) {
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
pub(super) fn field_works(sketch: &Sketch, mirror: Option<Mirror>) -> Result<Vec<GenWork>, String> {
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
            "line" => {
                for (kind, at) in [
                    (PreparedWorkKind::MgNest, &t.nests_m),
                    (PreparedWorkKind::Dugout, &t.dugouts_m),
                ] {
                    out.extend(at.iter().map(|p| GenWork {
                        kind,
                        points: vec![(p[0], p[1])],
                    }));
                }
                out.push(GenWork {
                    kind: PreparedWorkKind::Trench,
                    points,
                });
            }
            other => return Err(format!("unknown trench kind {other:?} (fire, line, comm)")),
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
            if w.points.iter().any(|&p| m.across(p) >= m.axis()) {
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
pub(super) fn wire_runs(w: &WireDef, lanes: &[Strip]) -> Vec<Vec<(f64, f64)>> {
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
pub(super) fn block_line(objects: &mut Objects, points: &[(f64, f64)], half_width: f64) {
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
pub(super) fn marsh_puddles(
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
                || mirror.is_some_and(|m| m.across(p) + r + 2.0 >= m.axis())
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
pub(super) struct CraterKeepOut<'a> {
    pub(super) works: &'a [GenWork],
    pub(super) bridges: &'a [BridgeRow],
    pub(super) flats: &'a [((f64, f64), f64)],
    pub(super) terrain: &'a dyn Fn(f64, f64) -> u8,
}

/// Craters of the crater fields (own random stream): `(x, y, caliber mm,
/// water)`, north half and mirrored copies.
pub(super) fn crater_fields(
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
                || mirror.is_some_and(|m| m.across(p) >= m.axis())
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
pub(super) fn prepared_ground(
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

/// Prompt 133: rows of large boulders (`env_rock` `Large`, 3 m apart) on
/// both rims of every wadi, which stop vehicles, not infantry; none within
/// a ford's ramps or 30 m of a bridge piece (no random numbers: the object
/// stream stays the same for maps without wadis).
pub(super) fn wadi_stoppers(objects: &mut Objects, wadis: &[WadiDef], bridges: &[BridgeRow]) {
    for w in wadis {
        let r = w.ford_m.unwrap_or(40.0);
        for seg in w.points_m.windows(2) {
            let (a, b) = ((seg[0][0], seg[0][1]), (seg[1][0], seg[1][1]));
            let len = dist(a, b);
            if len <= 0.0 {
                continue;
            }
            let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let mut t = 1.5;
            let mut k = 0;
            while t < len {
                for side in [-1.0, 1.0] {
                    let off = side * (w.width_m / 2.0 + WadiDef::RIM_M * 0.5);
                    let p = (a.0 + dx * t - dy * off, a.1 + dy * t + dx * off);
                    let near_ford = w.fords_m.iter().any(|f| dist(p, (f[0], f[1])) < r + 8.0);
                    let near_bridge = bridges.iter().any(|q| dist(p, (q.x, q.y)) < 30.0);
                    // Only where this segment is the nearest (no rows in
                    // the bed of a bend).
                    if near_ford
                        || near_bridge
                        || polyline_distance(p, &w.points_m) < w.width_m / 2.0 + 2.0
                    {
                        continue;
                    }
                    let heading = ((k * 67) % 360) as u16;
                    objects.place("env_rock", "Large", p, heading, -0.2);
                }
                t += 3.0;
                k += 1;
            }
        }
    }
}

/// A hedgerow polyline (prompt 221): `env_hedge` pieces (`Bocage` by default,
/// 3.2 m long) placed every 2.7 m so the organic lobes merge seamlessly, left
/// out where a road or track crosses. No random numbers.
pub(super) fn hedge_line(objects: &mut Objects, h: &HedgeDef, ways: &[Way]) {
    let variant = h.variant.as_deref().unwrap_or("Bocage");
    let step_m = if variant == "Bocage" { 2.7 } else { 3.5 };
    for w in h.points_m.windows(2) {
        let (a, b) = ((w[0][0], w[0][1]), (w[1][0], w[1][1]));
        let len = dist(a, b);
        if len < 0.5 {
            continue;
        }
        let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let heading = heading_of(dx, dy);
        let n = (len / step_m).round().max(1.0) as usize;
        let actual_step = len / n as f64;
        for i in 0..n {
            let t = actual_step * (i as f64 + 0.5);
            let p = (a.0 + dx * t, a.1 + dy * t);
            if ways
                .iter()
                .any(|way| way.distance(p.0, p.1) < way.half + 2.2)
            {
                continue;
            }
            // Overlap consecutive Bocage lobes intentionally so hedges form a continuous wall.
            objects.place("env_hedge", variant, p, heading, -1.25);
        }
    }
}

/// A wooden fence or pasture-post polyline (prompt 226): `env_fence_wood`
/// pieces (`Segment` 2.5 m by default, or `End` posts at `spacing_m`) placed
/// along the line, left out where a road or track crosses. No random numbers.
pub(super) fn fence_line(objects: &mut Objects, f: &FenceDef, ways: &[Way]) {
    let is_picket = f.kind.as_deref() == Some("picket");
    let obj_id = if is_picket {
        "env_fence_picket"
    } else {
        "env_fence_wood"
    };
    let variant = f.variant.as_deref().unwrap_or("Segment");
    let default_step = if variant == "End" {
        8.0
    } else if is_picket {
        3.90
    } else {
        2.45
    };
    let step_m = f.spacing_m.unwrap_or(default_step);
    let margin = if variant == "End" { 0.0 } else { -0.12 };
    for w in f.points_m.windows(2) {
        let (a, b) = ((w[0][0], w[0][1]), (w[1][0], w[1][1]));
        let len = dist(a, b);
        if len < 0.5 {
            continue;
        }
        let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let heading = heading_of(dx, dy);
        let n = (len / step_m).round().max(1.0) as usize;
        let actual_step = len / n as f64;
        for i in 0..n {
            let t = actual_step * (i as f64 + 0.5);
            let p = (a.0 + dx * t, a.1 + dy * t);
            if ways
                .iter()
                .any(|way| way.distance(p.0, p.1) < way.half + 2.2)
            {
                continue;
            }
            objects.place(obj_id, variant, p, heading, margin);
        }
    }
}

/// Compact rural vegetable garden (prompt 241): tight parallel crop rows inside
/// the polygon plus an optional `env_fence_picket` perimeter.
pub(super) fn garden_beds(objects: &mut Objects, g: &GardenDef, ways: &[Way]) {
    if g.polygon_m.len() < 3 {
        return;
    }
    let b = bounds(&g.polygon_m);
    let cx = (b.x0 + b.x1) * 0.5;
    let cy = (b.y0 + b.y1) * 0.5;
    let rad = g.angle_deg.to_radians();
    let (ca, sa) = (rad.cos(), rad.sin());
    let span = dist((b.x0, b.y0), (b.x1, b.y1)) * 0.55;
    let row_step = g.row_spacing_m.unwrap_or(2.6).max(1.5);
    let plant_step = g.plant_spacing_m.unwrap_or(2.2).max(1.4);
    let mut v = -span;
    let mut row_idx = 0usize;
    while v <= span {
        let mut u = -span;
        let mut col_idx = 0usize;
        while u <= span {
            let px = cx + u * ca - v * sa;
            let py = cy + u * sa + v * ca;
            if inside((px, py), &g.polygon_m)
                && !ways.iter().any(|way| way.distance(px, py) < way.half + 1.5)
            {
                let heading = ((row_idx * 47 + col_idx * 83) % 360) as u16;
                let variant = if (row_idx + col_idx).is_multiple_of(2) {
                    "Round"
                } else {
                    "Wide"
                };
                objects.place("env_bush", variant, (px, py), heading, -0.6);
            }
            u += plant_step;
            col_idx += 1;
        }
        v += row_step;
        row_idx += 1;
    }
    if g.fenced {
        let mut ring = g.polygon_m.clone();
        ring.push(g.polygon_m[0]);
        let f = FenceDef {
            points_m: ring,
            kind: Some("picket".to_string()),
            variant: Some("Segment".to_string()),
            spacing_m: Some(3.90),
        };
        fence_line(objects, &f, ways);
    }
}

/// A plank road (prompt 202): 4 m pieces end to end along the line, every
/// third one worn; no random numbers (the object stream stays the same).
pub(super) fn plank_road(objects: &mut Objects, pl: &PlankDef) {
    const PIECE_M: f64 = 4.0;
    let mut k = 0;
    for w in pl.points_m.windows(2) {
        let (a, b) = ((w[0][0], w[0][1]), (w[1][0], w[1][1]));
        let len = dist(a, b);
        let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        let heading = heading_of(dx, dy);
        let n = (len / PIECE_M).floor().max(1.0) as usize;
        for i in 0..n {
            let t = PIECE_M * (i as f64 + 0.5);
            let variant = if k % 3 == 2 { "Worn" } else { "Intact" };
            objects.place(
                "env_plank_road",
                variant,
                (a.0 + dx * t, a.1 + dy * t),
                heading,
                -0.1,
            );
            k += 1;
        }
    }
}

/// Modular cliff walls (`env_rock_cliff`) along `[[cliff]]` polylines (prompt 243).
/// Uses its own deterministic RNG stream (`seed ^ 0x636c_6966`) and returns early
/// when `sketch.cliff` is empty so existing sketch maps stay bit-for-bit identical.
pub(super) fn cliff_lines(
    objects: &mut Objects,
    sketch: &Sketch,
    ways: &[Way],
    bridges: &[BridgeRow],
    mesas: &[super::mesa::MesaPlan],
    biome: &str,
    seed: u64,
) {
    if sketch.cliff.is_empty() {
        return;
    }
    let mut random = Random(Rng::new(seed ^ 0x636c_6966));
    let is_clear = |p: (f64, f64), pad: f64| -> bool {
        if ways.iter().any(|w| w.distance(p.0, p.1) < w.half + pad) {
            return false;
        }
        if sketch
            .rail
            .iter()
            .any(|r| polyline_distance(p, &r.points_m) < super::RAIL_BED_M + pad)
        {
            return false;
        }
        if bridges.iter().any(|b| dist(p, (b.x, b.y)) < 24.0 + pad) {
            return false;
        }
        if sketch
            .start
            .iter()
            .any(|s| dist(p, (s.position_m[0], s.position_m[1])) < super::START_CLEAR_M)
        {
            return false;
        }
        if sketch
            .supply
            .iter()
            .any(|s| dist(p, (s.position_m[0], s.position_m[1])) < 14.0 + pad)
        {
            return false;
        }
        if mesas.iter().any(|m| m.on_track(p)) {
            return false;
        }
        true
    };

    // Body variants with (name, nominal length m, advance step m for ~4-5 m overlap).
    const BODY: [(&str, f64); 4] = [
        ("Straight", 15.5),
        ("High", 14.0),
        ("Long", 20.5),
        ("Buttress", 12.5),
    ];

    struct CliffSeg {
        a: (f64, f64),
        b: (f64, f64),
        t: (f64, f64),
        rt: (f64, f64),
        len: f64,
    }
    struct CliffSpan {
        a: (f64, f64),
        b: (f64, f64),
        rt: (f64, f64),
        len: f64,
        boulders: bool,
        forest_top: bool,
    }

    let mut placed_spans: Vec<CliffSpan> = Vec::new();

    for (ci, cliff) in sketch.cliff.iter().enumerate() {
        if cliff.points_m.len() < 2 {
            continue;
        }
        let [fx, fy] = cliff.facing_m;
        let fnlen = (fx * fx + fy * fy).sqrt();
        if fnlen <= 1e-6 {
            continue;
        }
        let face = (fx / fnlen, fy / fnlen);
        let mut pts: Vec<(f64, f64)> = cliff.points_m.iter().map(|p| (p[0], p[1])).collect();

        // Ensure walking pts[0] -> ... -> pts[last] has right normal (+X) aligned with `facing_m`.
        let chord = (
            pts[pts.len() - 1].0 - pts[0].0,
            pts[pts.len() - 1].1 - pts[0].1,
        );
        let chord_rt = (-chord.1, chord.0);
        if chord_rt.0 * face.0 + chord_rt.1 * face.1 < 0.0 {
            pts.reverse();
        }

        // Precompute segment tangents, normals and lengths.
        let mut segs: Vec<CliffSeg> = Vec::new();
        for w in pts.windows(2) {
            let len = dist(w[0], w[1]);
            if len < 2.0 {
                continue;
            }
            let t = ((w[1].0 - w[0].0) / len, (w[1].1 - w[0].1) / len);
            let rt = (-t.1, t.0);
            segs.push(CliffSeg {
                a: w[0],
                b: w[1],
                t,
                rt,
                len,
            });
        }
        if segs.is_empty() {
            continue;
        }

        // Place `Corner` modules on bends sharper than 22 deg (cos 22° ≈ 0.927).
        for k in 1..segs.len() {
            let t0 = segs[k - 1].t;
            let v = segs[k].a;
            let t1 = segs[k].t;
            let dot = t0.0 * t1.0 + t0.1 * t1.1;
            if dot < 0.927 && is_clear(v, 6.5) {
                let bx = t0.0 + t1.0;
                let by = t0.1 + t1.1;
                let bn = (bx * bx + by * by).sqrt();
                let h = if bn > 1e-4 {
                    heading_of(bx / bn, by / bn)
                } else {
                    heading_of(t1.0, t1.1)
                };
                objects.place("env_rock_cliff", "Corner", v, h, -6.5);
            }
        }

        let total_len: f64 = segs.iter().map(|s| s.len).sum();
        let mut cursor = 6.0f64;
        let mut piece_idx = ci * 3;
        while cursor < total_len - 4.0 {
            // Locate segment containing `cursor`.
            let mut rem = cursor;
            let mut seg = &segs[segs.len() - 1];
            for s in &segs {
                if rem <= s.len {
                    seg = s;
                    break;
                }
                rem -= s.len;
            }
            let (a, t, rt) = (seg.a, seg.t, seg.rt);
            let is_start = cursor <= 7.0;
            let is_end = cursor + 15.0 >= total_len;
            let (variant, step) = if is_end {
                ("End", 13.0)
            } else if is_start {
                ("Buttress", 12.5)
            } else {
                let pick = (piece_idx + (random.unit() * 2.0) as usize) % BODY.len();
                BODY[pick]
            };
            // Subtle lateral organic weave (±0.7 m) so long walls never look ruler-straight.
            let lat = (random.unit() * 2.0 - 1.0) * 0.7;
            let p = (a.0 + t.0 * rem + rt.0 * lat, a.1 + t.1 * rem + rt.1 * lat);
            let heading = heading_of(t.0, t.1);
            if is_clear(p, 6.5) {
                objects.place("env_rock_cliff", variant, p, heading, -6.5);
            }
            cursor += step;
            piece_idx += 1;
        }

        // Step down the `-Y` head end with leaning outcrops so both ends taper organically.
        let (start_a, start_t, start_rt) = (segs[0].a, segs[0].t, segs[0].rt);
        let head_p = (
            start_a.0 - start_t.0 * 4.5 + start_rt.0 * 1.2,
            start_a.1 - start_t.1 * 4.5 + start_rt.1 * 1.2,
        );
        if is_clear(head_p, 4.5) {
            objects.place(
                "env_rock_outcrop",
                "Leaning",
                head_p,
                heading_of(start_t.0, start_t.1),
                -2.0,
            );
        }

        for s in &segs {
            placed_spans.push(CliffSpan {
                a: s.a,
                b: s.b,
                rt: s.rt,
                len: s.len,
                boulders: cliff.boulders,
                forest_top: cliff.forest_top,
            });
        }
    }

    // Scatter valley-foot boulders/bushes and upper-crest conifer forest after all cliff
    // boxes are registered in `objects.rects` so trees & rocks hug the rim without clipping.
    let outcrops = ["Leaning", "Block", "Split", "Pair"];
    let rocks = ["Large", "Medium", "Small"];
    let bushes = ["Wide", "Round", "Hazel"];
    let conifers = ["Spruce", "Pine", "SpruceYoung"];
    let deciduous = if biome == "winter" {
        &["Bare"][..]
    } else {
        &["Oak", "Beech", "OakYoung"][..]
    };

    for span in placed_spans {
        let (a, b, rt, len, want_boulders, want_forest) = (
            span.a,
            span.b,
            span.rt,
            span.len,
            span.boulders,
            span.forest_top,
        );
        let t = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        if want_boulders {
            let mut u = 4.0;
            while u < len - 2.0 {
                // Occasional leaning/angular outcrop propped along the valley foot (+X).
                if random.unit() < 0.55 {
                    let off = random.range(5.6, 8.8);
                    let p = (a.0 + t.0 * u + rt.0 * off, a.1 + t.1 * u + rt.1 * off);
                    if is_clear(p, 3.5) {
                        let v = outcrops
                            [(random.unit() * outcrops.len() as f64) as usize % outcrops.len()];
                        let h = (random.unit() * 360.0) as u16 % 360;
                        objects.place("env_rock_outcrop", v, p, h, 0.2);
                    }
                }
                // Loose scree stones along the grassy foot embankment.
                for _ in 0..2 {
                    let du = random.range(-4.0, 4.0);
                    let off = random.range(5.2, 11.5);
                    let p = (
                        a.0 + t.0 * (u + du) + rt.0 * off,
                        a.1 + t.1 * (u + du) + rt.1 * off,
                    );
                    if is_clear(p, 2.2) {
                        let v = rocks[(random.unit() * rocks.len() as f64) as usize % rocks.len()];
                        let h = (random.unit() * 360.0) as u16 % 360;
                        objects.place("env_rock", v, p, h, 0.1);
                    }
                }
                if biome != "desert" && random.unit() < 0.65 {
                    let off = random.range(5.8, 10.5);
                    let du = random.range(-3.5, 3.5);
                    let p = (
                        a.0 + t.0 * (u + du) + rt.0 * off,
                        a.1 + t.1 * (u + du) + rt.1 * off,
                    );
                    if is_clear(p, 2.2) {
                        let v =
                            bushes[(random.unit() * bushes.len() as f64) as usize % bushes.len()];
                        let h = (random.unit() * 360.0) as u16 % 360;
                        objects.place("env_bush", v, p, h, 0.1);
                    }
                }
                u += random.range(12.0, 18.0);
            }
        }

        if want_forest && biome != "desert" {
            // 3 staggered rows of dense conifers (plus ~15% deciduous accents & rim bushes)
            // crowning the upper slope right behind the cliff crest (-X side).
            for (row_idx, (off_min, off_max, step_min, step_max)) in [
                (5.8, 9.5, 5.2, 7.6),
                (10.0, 16.0, 5.8, 8.5),
                (16.5, 24.0, 7.0, 10.5),
            ]
            .into_iter()
            .enumerate()
            {
                let mut u = random.range(1.5, 5.0);
                while u < len - 1.5 {
                    let off = -random.range(off_min, off_max);
                    let p = (a.0 + t.0 * u + rt.0 * off, a.1 + t.1 * u + rt.1 * off);
                    if is_clear(p, 2.5) {
                        let roll = random.unit();
                        let h = (random.unit() * 360.0) as u16 % 360;
                        if row_idx == 0 && roll < 0.22 {
                            let v = bushes
                                [(random.unit() * bushes.len() as f64) as usize % bushes.len()];
                            objects.place("env_bush", v, p, h, 0.2);
                        } else if roll < 0.85 {
                            let v = conifers
                                [(random.unit() * conifers.len() as f64) as usize % conifers.len()];
                            objects.place("env_tree_conifer", v, p, h, 0.5);
                        } else {
                            let v = deciduous[(random.unit() * deciduous.len() as f64) as usize
                                % deciduous.len()];
                            objects.place("env_tree_deciduous", v, p, h, 0.6);
                        }
                    }
                    u += random.range(step_min, step_max);
                }
            }
        }
    }
}

/// The objects of a camp (see [`Camp`]).
pub(super) fn camp(objects: &mut Objects, c: &Camp, ways: &[Way]) {
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
