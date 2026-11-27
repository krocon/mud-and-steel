//! Settlement rows, villages, towns, patchwork fields, bays and mirroring.

use super::CELL_M;
use super::plan::{BridgeRow, LONG_PIECE_M, bounds, inside, segment_distance};
use super::schema::*;
use super::works::Mirror;
use crate::gen_land::{Objects, Way, dist, heading_of};
use crate::gen_map::Random;
use sim::{ObjectRules, Rng};

/// Copies the north half of a sample raster (rows of `sx` samples, 2 m
/// apart from `-half_y`) mirrored about `axis` onto the south half
/// (prompt 123).
pub(super) fn mirror_south<T: Copy>(raster: &mut [T], sx: usize, half_y: f64, axis: f64) {
    let sy = raster.len() / sx;
    for j in 0..sy {
        let y = -half_y + j as f64 * CELL_M;
        if y <= axis {
            continue;
        }
        let source = ((2.0 * axis - y + half_y) / CELL_M).round() as usize;
        raster.copy_within(source * sx..(source + 1) * sx, j * sx);
    }
}

/// Bridges of a mirrored map (prompt 123): a chain that lies in the north
/// half gets its mirrored copy; a chain across the axis must already be
/// symmetric (its water is) and stays as it is.
pub(super) fn mirror_bridges(bridges: Vec<BridgeRow>, m: Mirror) -> Result<Vec<BridgeRow>, String> {
    let axis = m.axis();
    let mut out = Vec::new();
    let mut copies = Vec::new();
    for b in &bridges {
        let chain: Vec<&BridgeRow> = bridges
            .iter()
            .filter(|o| o.crossing == b.crossing)
            .collect();
        // Half a piece along the way (heading 0: along y) or half the deck
        // across (heading 90), measured across the mirror line.
        let along = (b.heading == 0) == m.across_y();
        let reach = if along { LONG_PIECE_M } else { 4.0 };
        let north = chain.iter().all(|o| m.across((o.x, o.y)) + reach < axis);
        let (mx, my) = m.map((b.x, b.y));
        if north {
            copies.push(BridgeRow {
                crossing: b.crossing,
                kind: b.kind,
                variant: b.variant,
                x: mx,
                y: my,
                heading: b.heading,
            });
        } else if !bridges
            .iter()
            .any(|o| (o.x - mx).abs() < 0.002 && (o.y - my).abs() < 0.002)
        {
            return Err(format!(
                "the bridge at ({:.1}, {:.1}) crosses the mirror axis but is not symmetric",
                b.x, b.y
            ));
        }
        out.push(BridgeRow { ..*b });
    }
    out.extend(copies);
    Ok(out)
}

/// Object rows of a mirrored map (prompt 123): every object that lies
/// wholly in the north half and its copy mirrored about `axis` (heading
/// mirrored too); objects reaching over the axis are left out, so no copy
/// overlaps its original.
pub(super) fn mirror_rows(
    rows: Vec<String>,
    rules: &ObjectRules,
    m: Mirror,
) -> Result<Vec<String>, String> {
    let axis_mm = (m.axis() * 1000.0).round() as i64;
    // Point symmetry (prompt 124): an object standing on the center point
    // is its own image and stays once.
    let center_mm = match m {
        Mirror::North(_) | Mirror::West(_) => None,
        Mirror::Point(cx, _) => Some(((cx * 1000.0).round() as i64, axis_mm)),
    };
    let mut north = Vec::new();
    let mut south = Vec::new();
    for row in rows {
        let f: Vec<&str> = row.split(',').collect();
        let [kind, variant, x, y, heading] = f[..] else {
            return Err(format!("bad object row {row:?}"));
        };
        let num = |v: &str| v.parse::<i64>().map_err(|e| format!("{row:?}: {e}"));
        let (x_mm, y_mm, heading) = (num(x)?, num(y)?, num(heading)?);
        let object = editor::PlacedObject {
            kind: kind.into(),
            variant: variant.into(),
            x_mm: x_mm as i32,
            y_mm: y_mm as i32,
            heading_deg: heading as u16,
        };
        if center_mm == Some((x_mm, y_mm)) {
            north.push(row);
            continue;
        }
        if let Mirror::West(_) = m {
            // Prompt 130: an object centered on the axis and turned along it
            // (heading 0 or 180) is its own image and stays once (a church,
            // a station); other objects of the west half get their image
            // (heading 360 - h).
            if x_mm == axis_mm && heading % 180 == 0 {
                north.push(row);
                continue;
            }
            let (ex, _) = editor::compound::half_extent(rules, &object);
            if x_mm + ex.max(300) > axis_mm {
                continue;
            }
            let mirrored_heading = if kind == "env_rock_cliff" {
                // Asymmetric +X valley face: reflect rt across x = axis.
                (540 - heading) % 360
            } else {
                (360 - heading) % 360
            };
            south.push(format!(
                "{kind},{variant},{},{y_mm},{mirrored_heading}",
                2 * axis_mm - x_mm,
            ));
            north.push(row);
            continue;
        }
        let (_, ey) = editor::compound::half_extent(rules, &object);
        if y_mm + ey.max(300) > axis_mm {
            continue;
        }
        match center_mm {
            None => {
                let mirrored_heading = if kind == "env_rock_cliff" {
                    // Asymmetric +X valley face: reflect rt across y = axis.
                    (360 - heading) % 360
                } else {
                    (540 - heading) % 360
                };
                south.push(format!(
                    "{kind},{variant},{x_mm},{},{mirrored_heading}",
                    2 * axis_mm - y_mm,
                ));
            }
            Some((cx_mm, _)) => south.push(format!(
                "{kind},{variant},{},{},{}",
                2 * cx_mm - x_mm,
                2 * axis_mm - y_mm,
                (heading + 180) % 360
            )),
        }
        north.push(row);
    }
    north.append(&mut south);
    Ok(north)
}

/// Places a group of field works (prompt 123); returns the pieces that did
/// not fit (roads, water or other objects in the way).
pub(super) fn works(objects: &mut Objects, w: &Works) -> Result<Vec<String>, String> {
    let len = (w.facing_m[0] * w.facing_m[0] + w.facing_m[1] * w.facing_m[1]).sqrt();
    if len == 0.0 {
        return Err("works: facing_m must not be zero".into());
    }
    // f: towards the enemy, a: across (to the right of the facing).
    let f = (w.facing_m[0] / len, w.facing_m[1] / len);
    let a = (-f.1, f.0);
    let front = heading_of(f.0, f.1);
    let side = (front + 90) % 360;
    // (kind, variant, across, along, heading)
    let pieces: Vec<(&str, &str, f64, f64, u16)> = match w.kind.as_str() {
        "battery" => vec![
            ("fort_sandbag_position", "Position", -22.5, 0.0, front),
            ("fort_sandbag_position", "Position", -7.5, 0.0, front),
            ("fort_sandbag_position", "Position", 7.5, 0.0, front),
            ("fort_sandbag_position", "Position", 22.5, 0.0, front),
            ("fort_trench_section", "Straight", -35.0, -6.0, front),
            ("fort_trench_section", "Straight", 35.0, -6.0, front),
            // Prompt 202: a parked gun behind each pit (scenery).
            ("env_gun_parked", "Ready", -22.5, -7.5, front),
            ("env_gun_parked", "Covered", -7.5, -7.5, front),
            ("env_gun_parked", "Ready", 7.5, -7.5, front),
            ("env_gun_parked", "Ready", 22.5, -7.5, front),
        ],
        "bridgehead" => vec![
            ("fort_bunker_mg", "Pillbox", 0.0, 0.0, front),
            ("fort_trench_section", "Straight", 7.0, -2.0, side),
            ("fort_trench_section", "Straight", 12.5, -2.0, side),
            ("fort_trench_section", "End", 18.0, -2.0, side),
            ("fort_sandbag_position", "Position", -8.0, -1.0, front),
        ],
        other => return Err(format!("unknown works kind {other:?}")),
    };
    let mut missing = Vec::new();
    for (kind, variant, across, along, heading) in pieces {
        let p = (
            w.position_m[0] + a.0 * across + f.0 * along,
            w.position_m[1] + a.1 * across + f.1 * along,
        );
        if !objects.place(kind, variant, p, heading, 0.3) {
            missing.push(format!("{kind} at ({:.0}, {:.0})", p.0, p.1));
        }
    }
    Ok(missing)
}

/// Height profile of a rock mountain (prompt 122) at `d` (distance from
/// the center over the radius, 0..1) with the crag noise value `n` (0..1):
/// a pointed top with concave flanks, `(1 - d)^1.8`, and crags from ridged
/// noise that are strongest on the upper flanks; 0 at the foot.
pub(super) fn peak_profile(d: f64, n: f64) -> f64 {
    let rest = 1.0 - d;
    let ridge = 1.0 - (2.0 * n - 1.0).abs();
    // (1 - d)^1.8 and (1 - d)^0.8 by sqrt only: x^0.8 ≈ x^0.75 · x^0.05
    // is close enough for a profile; x^0.75 = sqrt(x · sqrt(x)).
    let p075 = (rest * rest.sqrt()).sqrt();
    rest * p075 * 0.96 + 0.07 * p075 * ridge * ridge
}

/// Piers and rapids stones (prompt 122): rows of `objects.csv` in the
/// water, beside the object placer (which keeps out of water), from an own
/// random stream (sketches without them stay the same).
pub(super) fn scenery_rows(
    sketch: &Sketch,
    seed: u64,
    bridges: &[BridgeRow],
    water: &dyn Fn(f64, f64) -> bool,
) -> Vec<String> {
    const PIER_SEGMENT_M: f64 = 10.0;
    let mut rows = Vec::new();
    for pier in &sketch.pier {
        let ([ax, ay], [bx, by]) = (pier.points_m[0], pier.points_m[1]);
        let len = dist((ax, ay), (bx, by));
        let (dx, dy) = ((bx - ax) / len, (by - ay) / len);
        let heading = heading_of(dx, dy);
        let count = (len / PIER_SEGMENT_M).floor().max(1.0) as usize;
        for k in 0..count {
            let t = PIER_SEGMENT_M * (k as f64 + 0.5);
            let variant = if k + 1 == count && pier.boat.unwrap_or(true) {
                "Boat"
            } else {
                "Span"
            };
            rows.push(format!(
                "env_pier_wood,{variant},{},{},{heading}",
                ((ax + dx * t) * 1000.0).round() as i64,
                ((ay + dy * t) * 1000.0).round() as i64
            ));
        }
    }
    // Prompt 202: marker poles along both edges of every shoal (ford), in
    // the water, every `FORD_POLE_M`; a pair at both ends.
    const FORD_POLE_M: f64 = 24.0;
    for shoal in &sketch.shoal {
        let pts = &shoal.points_m;
        let total: f64 = pts
            .windows(2)
            .map(|w| dist((w[0][0], w[0][1]), (w[1][0], w[1][1])))
            .sum();
        let count = (total / FORD_POLE_M).floor().max(1.0) as usize;
        for k in 0..=count {
            let mut t = total * k as f64 / count as f64;
            let mut i = 0;
            loop {
                let len = dist((pts[i][0], pts[i][1]), (pts[i + 1][0], pts[i + 1][1]));
                if t <= len || i + 2 >= pts.len() {
                    let (a, b) = (pts[i], pts[i + 1]);
                    let (dx, dy) = ((b[0] - a[0]) / len, (b[1] - a[1]) / len);
                    let heading = heading_of(dx, dy);
                    let variant = if k == 0 || k == count {
                        "Pair"
                    } else {
                        "Single"
                    };
                    for side in [-1.0, 1.0] {
                        let off = side * (shoal.width_m / 2.0 + 1.0);
                        let p = (a[0] + dx * t - dy * off, a[1] + dy * t + dx * off);
                        if water(p.0, p.1) {
                            rows.push(format!(
                                "env_pole_ford,{variant},{},{},{heading}",
                                (p.0 * 1000.0).round() as i64,
                                (p.1 * 1000.0).round() as i64
                            ));
                        }
                    }
                    break;
                }
                t -= len;
                i += 1;
            }
        }
    }
    let mut random = Random(Rng::new(seed ^ 0x7261_7069));
    for r in &sketch.rapids {
        let lengths: Vec<f64> = r
            .points_m
            .windows(2)
            .map(|s| dist((s[0][0], s[0][1]), (s[1][0], s[1][1])))
            .collect();
        let total: f64 = lengths.iter().sum();
        let mut placed: Vec<(f64, f64)> = Vec::new();
        let mut tries = 0;
        while placed.len() < r.count as usize && tries < r.count * 30 {
            tries += 1;
            let mut t = random.range(0.0, total);
            let across = random.range(-r.width_m / 2.0, r.width_m / 2.0);
            let variant = if random.unit() < 0.6 {
                "Small"
            } else {
                "Medium"
            };
            let heading = (random.unit() * 360.0) as u16 % 360;
            let mut k = 0;
            while k + 1 < lengths.len() && t > lengths[k] {
                t -= lengths[k];
                k += 1;
            }
            let (a, b) = (r.points_m[k], r.points_m[k + 1]);
            let (dx, dy) = ((b[0] - a[0]) / lengths[k], (b[1] - a[1]) / lengths[k]);
            let p = (a[0] + dx * t - dy * across, a[1] + dy * t + dx * across);
            if !water(p.0, p.1)
                || bridges.iter().any(|b| dist(p, (b.x, b.y)) < 16.0)
                || placed.iter().any(|&q| dist(p, q) < 3.0)
            {
                continue;
            }
            placed.push(p);
            rows.push(format!(
                "env_rock,{variant},{},{},{heading}",
                (p.0 * 1000.0).round() as i64,
                (p.1 * 1000.0).round() as i64
            ));
        }
    }
    rows
}

/// The plots of the patchworks (own random stream: sketches without a
/// patchwork stay the same).
pub(super) fn patchwork_plots(patchworks: &[Patchwork], seed: u64) -> Vec<Area> {
    const LANE_M: f64 = 6.0;
    let mut random = Random(Rng::new(seed ^ 0x7061_7463));
    let mut plots = Vec::new();
    for pw in patchworks {
        let [ax, ay] = pw.axis.unwrap_or([1.0, 0.0]);
        let n = (ax * ax + ay * ay).sqrt();
        let (a, b) = ((ax / n, ay / n), (-ay / n, ax / n));
        let r = bounds(&pw.polygon_m);
        let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
        let reach = dist((r.x0, r.y0), (r.x1, r.y1)) / 2.0;
        // Rows across b, plots along a with varying lengths.
        let mut v = -reach;
        while v < reach {
            let width = pw.size_m[1] * random.range(0.67, 1.33);
            let mut u = -reach + random.range(0.0, pw.size_m[0]);
            while u < reach {
                let length = pw.size_m[0] * random.range(0.67, 1.33);
                let roll = random.unit();
                let k = (random.unit() * pw.kinds.len() as f64) as usize;
                let corner = |s: f64, t: f64| [cx + a.0 * s + b.0 * t, cy + a.1 * s + b.1 * t];
                let polygon = vec![
                    corner(u, v),
                    corner(u + length, v),
                    corner(u + length, v + width),
                    corner(u, v + width),
                ];
                let mid = (
                    cx + a.0 * (u + length / 2.0) + b.0 * (v + width / 2.0),
                    cy + a.1 * (u + length / 2.0) + b.1 * (v + width / 2.0),
                );
                if roll >= pw.meadow && inside(mid, &pw.polygon_m) {
                    plots.push(Area {
                        kind: pw.kinds[k.min(pw.kinds.len() - 1)].clone(),
                        spacing_m: None,
                        polygon_m: polygon,
                    });
                }
                u += length + LANE_M;
            }
            v += width + LANE_M;
        }
    }
    plots
}

/// The outline segments of `bays` whose middle lies in no other bay.
pub(super) fn bay_coast(bays: &[Area]) -> Vec<(P, P)> {
    let mut coast = Vec::new();
    for (k, bay) in bays.iter().enumerate() {
        let n = bay.polygon_m.len();
        for i in 0..n {
            let (a, b) = (bay.polygon_m[i], bay.polygon_m[(i + 1) % n]);
            let mid = ((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0);
            if !bays
                .iter()
                .enumerate()
                .any(|(j, other)| j != k && inside(mid, &other.polygon_m))
            {
                coast.push((a, b));
            }
        }
    }
    coast
}

/// Street building of a town (see `Town`).
pub(super) fn town(objects: &mut Objects, rules: &ObjectRules, t: &Town, ways: &[Way]) {
    // Length (along the street) and depth of every kind.
    let sizes: Vec<(&str, f64, f64)> = t
        .kinds
        .iter()
        .map(|k| {
            let probe = editor::PlacedObject {
                kind: k.clone(),
                variant: "Intact".into(),
                x_mm: 0,
                y_mm: 0,
                heading_deg: 0,
            };
            let (ex, ey) = editor::compound::half_extent(rules, &probe);
            let (a, b) = (ex as f64 / 500.0, ey as f64 / 500.0);
            (k.as_str(), a.max(b), a.min(b))
        })
        .collect();
    let setback = t.setback_m.unwrap_or(3.0);
    let gap = t.gap_m.unwrap_or(4.0);
    for way in ways {
        for s in way.points.windows(2) {
            let (a, b) = (s[0], s[1]);
            let len = dist(a, b);
            if len < 1.0 {
                continue;
            }
            let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let heading = heading_of(dx, dy);
            for side in [1.0, -1.0] {
                // Left of the direction for side 1.
                let (nx, ny) = (dy * side, -dx * side);
                let mut u = objects.random.range(0.0, gap);
                loop {
                    let k = (objects.random.unit() * sizes.len() as f64) as usize;
                    let (kind, length, depth) = sizes[k.min(sizes.len() - 1)];
                    if u + length > len {
                        break;
                    }
                    let offset = way.half + setback + objects.random.range(0.0, 2.0) + depth / 2.0;
                    let c = u + length / 2.0;
                    let p = (a.0 + dx * c + nx * offset, a.1 + dy * c + ny * offset);
                    let variant = variant_of(objects, t.damaged);
                    let h = if side > 0.0 {
                        heading
                    } else {
                        (heading + 180) % 360
                    };
                    if inside(p, &t.polygon_m) {
                        objects.place(kind, variant, p, h, 1.0);
                    }
                    u += length + gap * objects.random.range(0.5, 1.5);
                }
            }
        }
    }
}

/// Loose houses of a village (see `Village`).
pub(super) fn village(objects: &mut Objects, v: &Village, ways: &[Way]) {
    let area = bounds(&v.polygon_m);
    let kinds: Vec<&str> = v.kinds.iter().map(String::as_str).collect();
    let mut tries = 0;
    let mut placed = 0;
    while placed < v.count && tries < v.count * 40 {
        tries += 1;
        let p = (
            objects.random.range(area.x0, area.x1),
            objects.random.range(area.y0, area.y1),
        );
        let kind = objects.pick(&kinds);
        let variant = variant_of(objects, v.damaged);
        let turn = if objects.random.unit() < 0.5 { 0 } else { 180 };
        let heading = way_heading(ways, p, 40.0).map_or_else(
            || (objects.random.unit() * 4.0) as u16 % 4 * 90,
            |h| (h + turn) % 360,
        );
        if inside(p, &v.polygon_m) && objects.place(kind, variant, p, heading, 3.0) {
            placed += 1;
        }
    }
    // Broken walls: short runs of a few segments.
    let mut tries = 0;
    let mut runs = 0;
    while runs < v.walls && tries < v.walls * 40 {
        tries += 1;
        let p = (
            objects.random.range(area.x0, area.x1),
            objects.random.range(area.y0, area.y1),
        );
        let along_x = objects.random.unit() < 0.5;
        let segments = 2 + (objects.random.unit() * 4.0) as u32;
        if !inside(p, &v.polygon_m) {
            continue;
        }
        runs += 1;
        for k in 0..segments {
            let s = f64::from(k) * 2.4;
            let q = if along_x {
                (p.0 + s, p.1)
            } else {
                (p.0, p.1 + s)
            };
            let variant = if objects.random.unit() < 0.6 {
                "Broken"
            } else {
                "Low"
            };
            objects.place(
                "env_wall_mud",
                variant,
                q,
                if along_x { 90 } else { 0 },
                -0.05,
            );
        }
    }
}

/// Heading of the way segment nearest to `p` within `reach` m.
pub(super) fn way_heading(ways: &[Way], p: (f64, f64), reach: f64) -> Option<u16> {
    let mut best: Option<(f64, u16)> = None;
    for way in ways {
        for s in way.points.windows(2) {
            let d = segment_distance(p, [s[0].0, s[0].1], [s[1].0, s[1].1]);
            if d < reach && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, heading_of(s[1].0 - s[0].0, s[1].1 - s[0].1)));
            }
        }
    }
    best.map(|(_, h)| h)
}

pub(super) fn variant_of(objects: &mut Objects, damaged: f64) -> &'static str {
    let roll = objects.random.unit();
    if roll < damaged * 0.35 {
        "Ruin"
    } else if roll < damaged {
        "Damaged"
    } else {
        "Intact"
    }
}

/// Point `k` of `n` on the unit circle (cos, sin), by repeated rotation
/// (only basic arithmetic: the same on every platform).
pub(super) fn unit_circle(k: u32, n: u32) -> (f64, f64) {
    // cos and sin of 2π / n from their Taylor series (close enough for
    // placing huts on a ring).
    let a = 2.0 * std::f64::consts::PI / f64::from(n.max(3));
    let a2 = a * a;
    let cos = 1.0 - a2 / 2.0 + a2 * a2 / 24.0 - a2 * a2 * a2 / 720.0 + a2 * a2 * a2 * a2 / 40320.0;
    let sin = a
        * (1.0 - a2 / 6.0 + a2 * a2 / 120.0 - a2 * a2 * a2 / 5040.0
            + a2 * a2 * a2 * a2 / 362_880.0);
    let (mut c, mut s) = (1.0, 0.0);
    for _ in 0..k {
        (c, s) = (c * cos - s * sin, c * sin + s * cos);
    }
    (c, s)
}
