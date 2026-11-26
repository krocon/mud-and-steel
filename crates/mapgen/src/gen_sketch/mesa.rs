//! Table mountains (mesas), terraced hills, ramps, climbs and cliff boulders (prompts 126, 128, 129).

use super::plan::{bounds, edge_distance, inside};
use super::schema::P;
use crate::gen_land::{Objects, dist, heading_of, smoothstep};
use crate::gen_map::Random;
use serde::Deserialize;
use sim::Rng;

// --- Prompt 126: table mountains (mesas) -----------------------------------

/// A table mountain (prompt 126): a plateau `height_m` above the basin
/// whose foot is `polygon_m`. From the foot inwards a narrow scree slope
/// (35-60 %, boulders on it) and a cliff (180 %, impassable) rise to the
/// plateau; the outline (rounded by cutting its corners) is frayed by
/// noise (`rough_m`, default 8 m, tiers 0.6 of it; an own noise stream). `tier`: steps (higher than the plateau) and hollows
/// (lower) with their own edge. `ramps_m`: points on the outline (of the
/// mesa or of a tier) where a ramp for vehicles leads from the lower to the
/// higher level: `ramp_grade` (default 20 %), `ramp_width_m` wide (default
/// 18 m), half cut into the higher level, half heaped onto the lower one,
/// a dirt track on top. A mesa without ramps is scenery (rock needles).
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct MesaDef {
    /// Name of the ramp markers (`ramp_<name>_<k>`, tiers
    /// `ramp_<name>_t<i>_<k>`, on the higher level beyond each ramp's top);
    /// without a name no markers.
    #[serde(default)]
    pub(super) name: String,
    pub(super) polygon_m: Vec<P>,
    pub(super) height_m: f64,
    #[serde(default)]
    pub(super) ramps_m: Vec<P>,
    #[serde(default)]
    pub(super) tier: Vec<TierDef>,
    #[serde(default)]
    pub(super) rough_m: Option<f64>,
    #[serde(default)]
    pub(super) ramp_grade: Option<f64>,
    #[serde(default)]
    pub(super) ramp_width_m: Option<f64>,
    /// Boulders on the scree per 10 m of foot (default 1).
    #[serde(default)]
    pub(super) boulders: Option<f64>,
    /// Prompt 128: points on the outline where a narrow climb for infantry
    /// leads up: like a ramp, but `climb_grade` steep (default 38 %, under
    /// the 60 % limit also on a slant to the path grid) and `climb_width_m`
    /// wide (default 12 m), without a track; two rows of large boulders
    /// across its cut stop vehicles. Markers `climb_<name>_<k>`.
    #[serde(default)]
    pub(super) climbs_m: Vec<P>,
    #[serde(default)]
    pub(super) climb_grade: Option<f64>,
    #[serde(default)]
    pub(super) climb_width_m: Option<f64>,
    /// Prompt 129: the mesa's own edge as a slope (see `SlopeDef`).
    #[serde(default)]
    pub(super) slope_grade: Option<f64>,
    #[serde(default)]
    pub(super) cliffs_m: Vec<[P; 2]>,
    #[serde(default)]
    pub(super) cliff_m: Option<f64>,
    #[serde(default)]
    pub(super) center_m: Option<P>,
    #[serde(default)]
    pub(super) ramp_paths_m: Vec<Vec<P>>,
    #[serde(default)]
    pub(super) stop_vehicles: bool,
}

/// A step (`height_m` above the plateau's height) or a hollow (below it)
/// on a mesa (prompt 126); `height_m` counts from the basin. A tier lies on
/// the mesa's plateau, or on an earlier tier that holds its first point.
#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct TierDef {
    pub(super) polygon_m: Vec<P>,
    pub(super) height_m: f64,
    #[serde(default)]
    pub(super) ramps_m: Vec<P>,
    /// Prompt 129: the tier's edge as a slope (see `SlopeDef`).
    #[serde(default)]
    pub(super) slope_grade: Option<f64>,
    #[serde(default)]
    pub(super) cliffs_m: Vec<[P; 2]>,
    #[serde(default)]
    pub(super) cliff_m: Option<f64>,
    #[serde(default)]
    pub(super) center_m: Option<P>,
    #[serde(default)]
    pub(super) ramp_paths_m: Vec<Vec<P>>,
    #[serde(default)]
    pub(super) stop_vehicles: bool,
}

/// Prompt 129: the edge of a mesa or tier as an even slope of
/// `slope_grade` (under 60 %: open to everybody) instead of scree and a
/// cliff; without `slope_grade` the edge stays as before. `cliffs_m`:
/// sectors (pairs of points, clockwise on the map from the direction of
/// the first to that of the second as seen from `center_m`, default the
/// mean of the outline's points; two equal points: all round) where the
/// top `cliff_m` of the slope (default half the step) is a rock band
/// (180 %, impassable) and the rest below it gentler, the edge keeping its
/// width; blended over 8 m across the sector's bounding rays.
/// `ramp_paths_m`: ramps along lines (from the foot on the lower level to
/// the top on the higher one) whose bed rises evenly along them, so a
/// winding ramp keeps a low grade; built like ramps (half cut, half
/// heaped, `ramp_width_m`, a track), markers `ramp_<name>_p<k>`.
/// `stop_vehicles`: two rows of large boulders along the middle of the
/// slope outside the rock bands, ramps and climbs, which stop vehicles,
/// not infantry.
pub(super) struct SlopeDef<'a> {
    pub(super) slope_grade: Option<f64>,
    pub(super) cliffs_m: &'a [[P; 2]],
    pub(super) cliff_m: Option<f64>,
    pub(super) center_m: Option<P>,
    pub(super) ramp_paths_m: &'a [Vec<P>],
    pub(super) stop_vehicles: bool,
}

/// The slope fields of a mesa or a tier.
macro_rules! slope_def {
    ($d:expr) => {
        SlopeDef {
            slope_grade: $d.slope_grade,
            cliffs_m: &$d.cliffs_m,
            cliff_m: $d.cliff_m,
            center_m: $d.center_m,
            ramp_paths_m: &$d.ramp_paths_m,
            stop_vehicles: $d.stop_vehicles,
        }
    };
}

/// Noise that frays the outlines of mesas: three octaves (60, 20, 7 m).
pub(super) type Fray = [crate::gen_land::Noise; 3];

/// Rounds a polygon by cutting its corners (Chaikin, twice): mesas and
/// their tiers are drawn with a few points, their outlines are organic.
pub(super) fn rounded(polygon: &[P]) -> Vec<P> {
    let mut out = polygon.to_vec();
    for _ in 0..2 {
        let n = out.len();
        out = (0..n)
            .flat_map(|i| {
                let (a, b) = (out[i], out[(i + 1) % n]);
                [
                    [0.75 * a[0] + 0.25 * b[0], 0.75 * a[1] + 0.25 * b[1]],
                    [0.25 * a[0] + 0.75 * b[0], 0.25 * a[1] + 0.75 * b[1]],
                ]
            })
            .collect();
    }
    out
}

/// Scree at the foot of a mesa edge: a fifth of the height, at most 5 m,
/// with a grade from 35 to 60 % (mean 47.5 %).
pub(super) const MESA_TALUS_SHARE: f64 = 0.2;
pub(super) const MESA_TALUS_MAX_M: f64 = 5.0;
pub(super) const MESA_TALUS_GRADE: f64 = 0.475;
/// Grade of the cliff above the scree (impassable: over 60 %).
pub(super) const MESA_CLIFF_GRADE: f64 = 1.8;
/// Side slopes of a ramp: heaped (fill, passable scree) and cut (rock).
pub(super) const RAMP_FILL_GRADE: f64 = 0.5;
pub(super) const RAMP_CUT_GRADE: f64 = 1.5;
/// Share of a ramp cut into the higher level.
pub(super) const RAMP_CUT: f64 = 0.5;
/// Width of a ramp's flat bed without a track on it on each side (m).
pub(super) const RAMP_SHOULDER_M: f64 = 4.0;

/// A ramp of a mesa edge: from `at` (on the outline) the ground falls along
/// `dir` (towards the lower level) at `grade`; it reaches the higher level
/// at `u_top` (negative: inside the higher level) and the lower one `len`
/// further.
pub(super) struct MesaRamp {
    pub(super) at: (f64, f64),
    pub(super) dir: (f64, f64),
    pub(super) u_top: f64,
    pub(super) len: f64,
    pub(super) half_w: f64,
    pub(super) grade: f64,
    /// Prompt 128: a climb for infantry (no track, boulders stop vehicles).
    pub(super) climb: bool,
}

impl MesaRamp {
    /// Distance along and across the ramp.
    pub(super) fn frame(&self, p: (f64, f64)) -> (f64, f64) {
        let (dx, dy) = (p.0 - self.at.0, p.1 - self.at.1);
        (
            dx * self.dir.0 + dy * self.dir.1,
            (dy * self.dir.0 - dx * self.dir.1).abs(),
        )
    }

    /// The ground `value` of the edge between `lo` and `hi` with the ramp:
    /// on the bed the ramp's height, beside it cut walls and heaped sides.
    pub(super) fn apply(&self, p: (f64, f64), value: f64, lo: f64, hi: f64) -> f64 {
        let (u, v) = self.frame(p);
        let spread = (hi - lo) / RAMP_FILL_GRADE;
        // Beyond the foot the bed lies on the lower level: nothing to cut
        // or heap (a far rim, e.g. of a hollow, stays as it is).
        if u < self.u_top || u > self.u_top + self.len || v > self.half_w + spread {
            return value;
        }
        let bed = (hi - self.grade * (u - self.u_top)).clamp(lo, hi);
        let s = (v - self.half_w).max(0.0);
        value.clamp(bed - s * RAMP_FILL_GRADE, bed + s * RAMP_CUT_GRADE)
    }

    /// True on the track along the ramp's bed.
    pub(super) fn on_track(&self, p: (f64, f64)) -> bool {
        if self.climb {
            return false;
        }
        let (u, v) = self.frame(p);
        u >= self.u_top - RAMP_SHOULDER_M
            && u <= self.u_top + self.len
            && v < self.half_w - RAMP_SHOULDER_M
    }

    /// A point `beyond` m past the ramp's top on the higher level.
    pub(super) fn top(&self, beyond: f64) -> (f64, f64) {
        let u = self.u_top - beyond;
        (self.at.0 + self.dir.0 * u, self.at.1 + self.dir.1 * u)
    }

    /// The ramp's foot on the lower level.
    pub(super) fn foot(&self) -> (f64, f64) {
        let u = self.u_top + self.len;
        (self.at.0 + self.dir.0 * u, self.at.1 + self.dir.1 * u)
    }
}

/// Prompt 129: a ramp along a line: its bed rises evenly from the lower
/// level at the first point to the higher one at the last.
pub(super) struct PathRamp {
    pub(super) points: Vec<(f64, f64)>,
    /// Length along the line up to each point.
    pub(super) along: Vec<f64>,
    pub(super) len: f64,
    pub(super) half_w: f64,
    pub(super) bounds: crate::gen_land::Rect,
}

impl PathRamp {
    pub(super) fn new(points: &[P], half_w: f64) -> Result<Self, String> {
        let points: Vec<(f64, f64)> = points.iter().map(|p| (p[0], p[1])).collect();
        let mut along = vec![0.0];
        for w in points.windows(2) {
            along.push(along[along.len() - 1] + dist(w[0], w[1]));
        }
        let len = along[along.len() - 1];
        if points.len() < 2 || len <= 0.0 {
            return Err(format!(
                "ramp path at {:?}: needs two points",
                points.first()
            ));
        }
        let polygon: Vec<P> = points.iter().map(|p| [p.0, p.1]).collect();
        Ok(Self {
            bounds: bounds(&polygon),
            points,
            along,
            len,
            half_w,
        })
    }

    /// Length along the line to the nearest point and the distance to it.
    pub(super) fn frame(&self, p: (f64, f64)) -> (f64, f64) {
        let (mut best, mut s) = (f64::INFINITY, 0.0);
        for (i, w) in self.points.windows(2).enumerate() {
            let (a, b) = (w[0], w[1]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let l2 = dx * dx + dy * dy;
            let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0);
            let d = dist(p, (a.0 + t * dx, a.1 + t * dy));
            if d < best {
                best = d;
                s = self.along[i] + t * (self.along[i + 1] - self.along[i]);
            }
        }
        (s, best)
    }

    pub(super) fn apply(&self, p: (f64, f64), value: f64, lo: f64, hi: f64) -> f64 {
        let spread = (hi - lo) / RAMP_FILL_GRADE;
        if !in_rect(&grow(self.bounds, self.half_w + spread), p) {
            return value;
        }
        let (s, v) = self.frame(p);
        if v > self.half_w + spread {
            return value;
        }
        let bed = lo + (hi - lo) * (s / self.len).clamp(0.0, 1.0);
        let off = (v - self.half_w).max(0.0);
        value.clamp(bed - off * RAMP_FILL_GRADE, bed + off * RAMP_CUT_GRADE)
    }

    pub(super) fn near(&self, p: (f64, f64), margin: f64) -> bool {
        in_rect(&grow(self.bounds, self.half_w + margin), p)
            && self.frame(p).1 < self.half_w + margin
    }

    /// A point `beyond` m past the last point along the last piece.
    pub(super) fn top(&self, beyond: f64) -> (f64, f64) {
        let n = self.points.len();
        let (a, b) = (self.points[n - 2], self.points[n - 1]);
        let l = dist(a, b);
        (
            b.0 + (b.0 - a.0) / l * beyond,
            b.1 + (b.1 - a.1) / l * beyond,
        )
    }
}

/// Prompt 129: an edge as a slope with rock bands in sectors (`SlopeDef`).
pub(super) struct Slope {
    pub(super) grade: f64,
    pub(super) cliff: f64,
    pub(super) center: (f64, f64),
    /// Unit directions bounding each rock band sector (clockwise from the
    /// first to the second); equal directions: all round.
    pub(super) sectors: Vec<((f64, f64), (f64, f64))>,
    pub(super) stop_vehicles: bool,
}

/// Blend of a rock band sector across its bounding rays (m).
pub(super) const SLOPE_BLEND_M: f64 = 8.0;

impl Slope {
    pub(super) fn new(def: &SlopeDef<'_>, polygon: &[P]) -> Result<Option<Self>, String> {
        let Some(grade) = def.slope_grade else {
            if !def.cliffs_m.is_empty() || def.stop_vehicles || def.cliff_m.is_some() {
                return Err("cliffs_m, cliff_m and stop_vehicles need a slope_grade".into());
            }
            return Ok(None);
        };
        if grade <= 0.0 || grade >= 0.6 {
            return Err(format!("slope_grade {grade} must lie between 0 and 0.6"));
        }
        let n = polygon.len() as f64;
        let center = def.center_m.map_or_else(
            || {
                (
                    polygon.iter().map(|p| p[0]).sum::<f64>() / n,
                    polygon.iter().map(|p| p[1]).sum::<f64>() / n,
                )
            },
            |c| (c[0], c[1]),
        );
        let unit = |p: P| {
            let (dx, dy) = (p[0] - center.0, p[1] - center.1);
            let l = (dx * dx + dy * dy).sqrt();
            (dx / l, dy / l)
        };
        Ok(Some(Self {
            grade,
            cliff: def.cliff_m.unwrap_or(f64::NAN),
            center,
            sectors: def
                .cliffs_m
                .iter()
                .map(|[a, b]| (unit(*a), unit(*b)))
                .collect(),
            stop_vehicles: def.stop_vehicles,
        }))
    }

    /// Weight of the rock band at `p`: 1 inside a sector, 0 outside.
    pub(super) fn cliff_weight(&self, p: (f64, f64)) -> f64 {
        let d = (p.0 - self.center.0, p.1 - self.center.1);
        let cross = |u: (f64, f64), v: (f64, f64)| u.0 * v.1 - u.1 * v.0;
        let ray = |u: (f64, f64)| {
            if u.0 * d.0 + u.1 * d.1 > 0.0 {
                cross(u, d).abs()
            } else {
                (d.0 * d.0 + d.1 * d.1).sqrt()
            }
        };
        let mut w = 0.0f64;
        for &(a, b) in &self.sectors {
            if a == b {
                return 1.0;
            }
            let inside = if cross(a, b) >= 0.0 {
                cross(a, d) >= 0.0 && cross(d, b) >= 0.0
            } else {
                !(cross(b, d) > 0.0 && cross(d, a) > 0.0)
            };
            let s = ray(a).min(ray(b));
            let s = if inside { s } else { -s };
            w = w.max(smoothstep(-SLOPE_BLEND_M, SLOPE_BLEND_M, s));
        }
        w
    }

    /// Height of the rock band for a step of `d` m.
    pub(super) fn band(&self, d: f64) -> f64 {
        if self.cliff.is_nan() {
            d * 0.5
        } else {
            self.cliff.min(d)
        }
    }

    /// Distance from the foot to the foot of the rock band.
    pub(super) fn band_foot(&self, d: f64) -> f64 {
        (d / self.grade - self.band(d) / MESA_CLIFF_GRADE).max(1.0)
    }

    /// Rise `x` m from the foot of a step of `d` m, rock band weight `w`.
    pub(super) fn rise(&self, x: f64, d: f64, w: f64) -> f64 {
        let open = (x * self.grade).clamp(0.0, d);
        if w <= 0.0 {
            return open;
        }
        let c = self.band(d);
        let low = self.band_foot(d);
        let cliff = if x <= low {
            (d - c) * (x / low).max(0.0)
        } else {
            ((d - c) + (x - low) * MESA_CLIFF_GRADE).min(d)
        };
        open + (cliff - open) * w
    }
}

/// One edge of a mesa: its outline, the levels below (`lo`) and above
/// (`hi`) it, which side is high (inside: the mesa and its steps; outside:
/// a hollow) and its ramps.
pub(super) struct MesaEdge {
    pub(super) polygon: Vec<P>,
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) high_inside: bool,
    pub(super) rough: f64,
    pub(super) ramps: Vec<MesaRamp>,
    /// Prompt 129: ramps along lines and the slope (none before).
    pub(super) paths: Vec<PathRamp>,
    pub(super) slope: Option<Slope>,
    /// Bounds of the outline grown by the roughness: outside them the edge
    /// is its outer level.
    pub(super) near: crate::gen_land::Rect,
    /// Bounds of everything the edge changes (outline and ramps).
    pub(super) reach: crate::gen_land::Rect,
}

pub(super) fn in_rect(r: &crate::gen_land::Rect, p: (f64, f64)) -> bool {
    p.0 >= r.x0 && p.0 <= r.x1 && p.1 >= r.y0 && p.1 <= r.y1
}

pub(super) fn grow(r: crate::gen_land::Rect, by: f64) -> crate::gen_land::Rect {
    crate::gen_land::Rect {
        x0: r.x0 - by,
        y0: r.y0 - by,
        x1: r.x1 + by,
        y1: r.y1 + by,
    }
}

impl MesaEdge {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        polygon: &[P],
        (lo, hi): (f64, f64),
        high_inside: bool,
        rough: f64,
        ramps: &[P],
        (grade, width): (f64, f64),
        (climbs, (climb_grade, climb_width)): (&[P], (f64, f64)),
        def: &SlopeDef<'_>,
    ) -> Result<Self, String> {
        let bad = |g: f64| g <= 0.0 || g >= 0.6;
        if polygon.len() < 3 || hi <= lo || bad(grade) || (!climbs.is_empty() && bad(climb_grade)) {
            return Err(format!(
                "mesa edge at {:?}: needs a polygon, a height step and a ramp grade under 60 %",
                polygon.first()
            ));
        }
        let slope = Slope::new(def, polygon)?;
        let paths: Vec<PathRamp> = def
            .ramp_paths_m
            .iter()
            .map(|p| PathRamp::new(p, width / 2.0))
            .collect::<Result<_, _>>()?;
        let polygon = &rounded(polygon)[..];
        let ramps: Vec<MesaRamp> = ramps
            .iter()
            .map(|r| (r, grade, width, false))
            .chain(climbs.iter().map(|r| (r, climb_grade, climb_width, true)))
            .map(|(r, grade, width, climb)| {
                let len = (hi - lo) / grade;
                // The nearest point of the outline and its normal towards the
                // lower level.
                let n = polygon.len();
                let (mut best, mut at, mut dir) = (f64::INFINITY, (0.0, 0.0), (0.0, 0.0));
                for i in 0..n {
                    let (a, b) = (polygon[i], polygon[(i + 1) % n]);
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let l2 = dx * dx + dy * dy;
                    if l2 <= 0.0 {
                        continue;
                    }
                    let t = (((r[0] - a[0]) * dx + (r[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
                    let q = (a[0] + t * dx, a[1] + t * dy);
                    let d = dist(q, (r[0], r[1]));
                    if d < best {
                        let l = l2.sqrt();
                        best = d;
                        at = q;
                        dir = (dy / l, -dx / l);
                    }
                }
                let out = (at.0 + dir.0, at.1 + dir.1);
                if inside(out, polygon) == high_inside {
                    dir = (-dir.0, -dir.1);
                }
                MesaRamp {
                    at,
                    dir,
                    u_top: -RAMP_CUT * len,
                    len,
                    half_w: width / 2.0,
                    grade,
                    climb,
                }
            })
            .collect();
        let near = grow(bounds(polygon), rough + 1.0);
        let spread = (hi - lo) / RAMP_FILL_GRADE;
        let mut reach = near;
        for r in &paths {
            let around = grow(r.bounds, r.half_w + spread);
            reach.x0 = reach.x0.min(around.x0);
            reach.y0 = reach.y0.min(around.y0);
            reach.x1 = reach.x1.max(around.x1);
            reach.y1 = reach.y1.max(around.y1);
        }
        for r in &ramps {
            let w = r.half_w + spread;
            for p in [r.top(0.0), r.foot()] {
                let around = crate::gen_land::Rect {
                    x0: p.0 - w - spread,
                    y0: p.1 - w - spread,
                    x1: p.0 + w + spread,
                    y1: p.1 + w + spread,
                };
                reach.x0 = reach.x0.min(around.x0);
                reach.y0 = reach.y0.min(around.y0);
                reach.x1 = reach.x1.max(around.x1);
                reach.y1 = reach.y1.max(around.y1);
            }
        }
        Ok(Self {
            polygon: polygon.to_vec(),
            lo,
            hi,
            high_inside,
            rough,
            ramps,
            paths,
            slope,
            near,
            reach,
        })
    }

    /// Height and width of the scree at the foot.
    pub(super) fn talus(&self) -> (f64, f64) {
        let h = ((self.hi - self.lo) * MESA_TALUS_SHARE).min(MESA_TALUS_MAX_M);
        (h, h / MESA_TALUS_GRADE)
    }

    /// Width of the whole edge (scree and cliff, or the slope).
    pub(super) fn width(&self) -> f64 {
        if let Some(s) = &self.slope {
            return (self.hi - self.lo) / s.grade;
        }
        let (th, tw) = self.talus();
        tw + (self.hi - self.lo - th) / MESA_CLIFF_GRADE
    }

    /// Rise of the edge `x` m from its foot towards the high side.
    pub(super) fn profile(&self, x: f64) -> f64 {
        let d = self.hi - self.lo;
        let (th, tw) = self.talus();
        if x <= 0.0 {
            0.0
        } else if x < tw {
            // Grade from 35 % at the foot to 60 % at the cliff.
            let t = x / tw;
            th * t * (0.737 + 0.263 * t)
        } else {
            (th + (x - tw) * MESA_CLIFF_GRADE).min(d)
        }
    }

    /// Signed distance into the outline (negative outside), frayed.
    pub(super) fn edge(&self, p: (f64, f64), fray: &Fray) -> f64 {
        let d = edge_distance(p, &self.polygon);
        let signed = if inside(p, &self.polygon) { d } else { -d };
        // Prompt 129: a slope keeps only the broad octave; the fine ones
        // would steepen it locally beyond its grade.
        if self.slope.is_some() {
            return signed + self.rough * (fray[0].at(p.0, p.1) - 0.5) * 1.2;
        }
        signed
            + self.rough
                * ((fray[0].at(p.0, p.1) - 0.5) * 1.2
                    + (fray[1].at(p.0, p.1) - 0.5) * 0.6
                    + (fray[2].at(p.0, p.1) - 0.5) * 0.3)
    }

    /// The level away from the edge on its outer side.
    pub(super) const fn outer(&self) -> f64 {
        if self.high_inside { self.lo } else { self.hi }
    }

    pub(super) fn value(&self, p: (f64, f64), fray: &Fray) -> f64 {
        if !in_rect(&self.reach, p) {
            return self.outer();
        }
        let mut v = if in_rect(&self.near, p) {
            let e = self.edge(p, fray);
            let x = if self.high_inside {
                e
            } else {
                self.width() - e
            };
            match &self.slope {
                None => self.lo + self.profile(x),
                Some(s) => self.lo + s.rise(x, self.hi - self.lo, s.cliff_weight(p)),
            }
        } else {
            self.outer()
        };
        for r in &self.ramps {
            v = r.apply(p, v, self.lo, self.hi);
        }
        for r in &self.paths {
            v = r.apply(p, v, self.lo, self.hi);
        }
        v
    }
}

/// A mesa ready to sample: its outer edge first, then its tiers.
pub(super) struct MesaPlan {
    pub(super) name: String,
    pub(super) edges: Vec<MesaEdge>,
    pub(super) reach: crate::gen_land::Rect,
    pub(super) boulders: f64,
}

impl MesaPlan {
    pub(super) fn new(m: &MesaDef) -> Result<Self, String> {
        let shape = (m.ramp_grade.unwrap_or(0.2), m.ramp_width_m.unwrap_or(18.0));
        let climb = (
            m.climb_grade.unwrap_or(0.38),
            m.climb_width_m.unwrap_or(12.0),
        );
        let rough = m.rough_m.unwrap_or(8.0);
        let mut edges = vec![MesaEdge::new(
            &m.polygon_m,
            (0.0, m.height_m),
            true,
            rough,
            &m.ramps_m,
            shape,
            (&m.climbs_m, climb),
            &slope_def!(m),
        )?];
        for (i, t) in m.tier.iter().enumerate() {
            let first = (t.polygon_m[0][0], t.polygon_m[0][1]);
            let parent = m.tier[..i]
                .iter()
                .rev()
                .find(|o| inside(first, &o.polygon_m))
                .map_or(m.height_m, |o| o.height_m);
            let step = t.height_m > parent;
            let levels = if step {
                (parent, t.height_m)
            } else {
                (t.height_m, parent)
            };
            edges.push(MesaEdge::new(
                &t.polygon_m,
                levels,
                step,
                rough * 0.6,
                &t.ramps_m,
                shape,
                (&[], climb),
                &slope_def!(t),
            )?);
        }
        let mut reach = edges[0].reach;
        for e in &edges[1..] {
            reach.x0 = reach.x0.min(e.reach.x0);
            reach.y0 = reach.y0.min(e.reach.y0);
            reach.x1 = reach.x1.max(e.reach.x1);
            reach.y1 = reach.y1.max(e.reach.y1);
        }
        Ok(Self {
            name: m.name.clone(),
            edges,
            reach,
            boulders: m.boulders.unwrap_or(1.0),
        })
    }

    /// Height of the mesa above the basin at `p` (0 off the mesa).
    pub(super) fn height(&self, p: (f64, f64), fray: &Fray) -> f64 {
        if !in_rect(&self.reach, p) {
            return 0.0;
        }
        let mut h = self.edges[0].value(p, fray);
        for e in &self.edges[1..] {
            h += e.value(p, fray) - e.outer();
        }
        h
    }

    pub(super) fn on_track(&self, p: (f64, f64)) -> bool {
        in_rect(&self.reach, p)
            && self.edges.iter().any(|e| {
                e.ramps.iter().any(|r| r.on_track(p))
                    || e.paths.iter().any(|r| r.near(p, -RAMP_SHOULDER_M))
            })
    }

    /// Ramp markers: name and position beyond the top of each ramp.
    pub(super) fn markers(&self) -> Vec<(String, (f64, f64))> {
        if self.name.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            let mut climbs = 0;
            for (k, r) in e.ramps.iter().filter(|r| !r.climb).enumerate() {
                let name = if i == 0 {
                    format!("ramp_{}_{}", self.name, k + 1)
                } else {
                    format!("ramp_{}_t{i}_{}", self.name, k + 1)
                };
                out.push((name, r.top(12.0)));
            }
            for r in e.ramps.iter().filter(|r| r.climb) {
                climbs += 1;
                out.push((format!("climb_{}_{climbs}", self.name), r.top(12.0)));
            }
            for (k, r) in e.paths.iter().enumerate() {
                let name = if i == 0 {
                    format!("ramp_{}_p{}", self.name, k + 1)
                } else {
                    format!("ramp_{}_t{i}_p{}", self.name, k + 1)
                };
                out.push((name, r.top(12.0)));
            }
        }
        out
    }
}

/// Prompt 128: two rows of large boulders (`env_rock` `Large`, they stop
/// vehicles, not infantry) across the cut of every climb, 4 m apart; then
/// the climb's bed is kept free of other objects (woods, scree boulders).
/// Returns the boulders placed.
pub(super) fn climb_stoppers(objects: &mut Objects, mesas: &[MesaPlan]) -> usize {
    let mut placed = 0;
    let mut keep = Vec::new();
    for m in mesas {
        for e in &m.edges {
            for r in e.ramps.iter().filter(|r| r.climb) {
                let side = (-r.dir.1, r.dir.0);
                for (row, u) in [0.5 * r.u_top, 0.5 * r.u_top + 4.0].into_iter().enumerate() {
                    let mut v = -r.half_w + 1.5;
                    let mut k = 0;
                    while v <= r.half_w - 1.5 + 1e-9 {
                        let p = (
                            r.at.0 + r.dir.0 * u + side.0 * v,
                            r.at.1 + r.dir.1 * u + side.1 * v,
                        );
                        let heading = ((row * 7 + k) * 67 % 360) as u16;
                        if objects.place("env_rock", "Large", p, heading, -0.2) {
                            placed += 1;
                        }
                        v += 3.0;
                        k += 1;
                    }
                }
                // Prompt 202: a path of log steps up the middle, below and
                // above the boulder rows (the pieces climb towards the top).
                let rows = (0.5 * r.u_top - 2.5, 0.5 * r.u_top + 6.5);
                let heading = heading_of(-r.dir.0, -r.dir.1);
                let mut u = r.u_top + 2.0;
                let mut k = 0;
                while u <= r.u_top + r.len - 2.0 {
                    if u <= rows.0 || u >= rows.1 {
                        let p = (r.at.0 + r.dir.0 * u, r.at.1 + r.dir.1 * u);
                        let variant = if k % 5 == 3 { "Ladder" } else { "Steps" };
                        objects.place("env_climb_steps", variant, p, heading, -0.2);
                        k += 1;
                    }
                    u += 3.5;
                }
                // The bed from beyond the top to the foot.
                let mut u = r.u_top - 4.0;
                while u <= r.u_top + r.len + 4.0 {
                    keep.push(((r.at.0 + r.dir.0 * u, r.at.1 + r.dir.1 * u), r.half_w + 2.0));
                    u += 4.0;
                }
            }
        }
    }
    // Prompt 129: the beds of the ramps along lines too.
    for m in mesas {
        for e in &m.edges {
            for r in &e.paths {
                let mut s = 0.0;
                while s <= r.len {
                    let mut i = 0;
                    while i + 2 < r.along.len() && r.along[i + 1] < s {
                        i += 1;
                    }
                    let t = (s - r.along[i]) / (r.along[i + 1] - r.along[i]);
                    let (a, b) = (r.points[i], r.points[i + 1]);
                    keep.push((
                        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t),
                        r.half_w + 2.0,
                    ));
                    s += 4.0;
                }
            }
        }
    }
    objects.reserved.extend(keep);
    placed
}

/// Prompt 129: two rows of large boulders (`env_rock` `Large`, 3 m apart,
/// the rows 4 m apart) along the middle of every slope with
/// `stop_vehicles`, outside its rock bands, ramps and climbs: vehicles
/// stop there, infantry climbs on. Returns the boulders placed.
pub(super) fn slope_stoppers(objects: &mut Objects, mesas: &[MesaPlan], fray: &Fray) -> usize {
    let mut placed = 0;
    for m in mesas {
        for e in &m.edges {
            let Some(s) = e.slope.as_ref().filter(|s| s.stop_vehicles) else {
                continue;
            };
            let width = e.width();
            let n = e.polygon.len();
            for (row, depth) in [0.5 * width - 2.0, 0.5 * width + 2.0]
                .into_iter()
                .enumerate()
            {
                // Walk the outline every 3 m and move each point onto the
                // frayed contour `depth` m from the foot.
                let mut carry = 0.0;
                let mut k = 0usize;
                for i in 0..n {
                    let (a, b) = (e.polygon[i], e.polygon[(i + 1) % n]);
                    let len = dist((a[0], a[1]), (b[0], b[1]));
                    if len <= 0.0 {
                        continue;
                    }
                    let (mut nx, mut ny) = ((b[1] - a[1]) / len, (a[0] - b[0]) / len);
                    let mid = (0.5 * (a[0] + b[0]) + nx, 0.5 * (a[1] + b[1]) + ny);
                    if inside(mid, &e.polygon) != e.high_inside {
                        (nx, ny) = (-nx, -ny);
                    }
                    let mut t = carry;
                    while t < len {
                        let base = (
                            a[0] + (b[0] - a[0]) * t / len,
                            a[1] + (b[1] - a[1]) * t / len,
                        );
                        let mut off = depth;
                        let mut q = base;
                        for _ in 0..3 {
                            q = (base.0 + nx * off, base.1 + ny * off);
                            let x = if e.high_inside {
                                e.edge(q, fray)
                            } else {
                                width - e.edge(q, fray)
                            };
                            off += depth - x;
                        }
                        let open = s.cliff_weight(q) < 0.05
                            && !e.ramps.iter().any(|r| {
                                let (u, v) = r.frame(q);
                                u > r.u_top - 6.0 && u < r.u_top + r.len + 6.0 && v < r.half_w + 6.0
                            })
                            && !e.paths.iter().any(|r| r.near(q, 6.0));
                        if open {
                            let heading = ((row * 7 + k) * 67 % 360) as u16;
                            if objects.place("env_rock", "Large", q, heading, -0.2) {
                                placed += 1;
                            }
                        }
                        k += 1;
                        t += 3.0;
                    }
                    carry = t - len;
                }
            }
        }
    }
    placed
}

/// Boulders on the scree at the foot of every mesa edge (prompt 126), from
/// an own random stream; none on or beside the ramps.
pub(super) fn mesa_boulders(
    objects: &mut Objects,
    mesas: &[MesaPlan],
    fray: &Fray,
    seed: u64,
) -> usize {
    const ROCKS: [&str; 6] = ["Group", "Block", "Split", "Pair", "Leaning", "Layered"];
    let mut random = Random(Rng::new(seed ^ 0x6d65_7361));
    let mut placed = 0;
    for m in mesas {
        for e in &m.edges {
            let (_, tw) = e.talus();
            let width = e.width();
            let n = e.polygon.len();
            for i in 0..n {
                let (a, b) = (e.polygon[i], e.polygon[(i + 1) % n]);
                let len = dist((a[0], a[1]), (b[0], b[1]));
                if len <= 0.0 {
                    continue;
                }
                let attempts = (len / 10.0 * m.boulders).round() as usize;
                // Inward normal of the segment (either way; checked below).
                let (nx, ny) = ((b[1] - a[1]) / len, (a[0] - b[0]) / len);
                for _ in 0..attempts {
                    let t = random.unit();
                    let base = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                    let off = random.range(-e.rough - tw, width + e.rough);
                    let roll = random.unit();
                    let heading = (random.unit() * 360.0) as u16 % 360;
                    let pick = random.unit();
                    for sign in [1.0, -1.0] {
                        let q = (base.0 + sign * nx * off, base.1 + sign * ny * off);
                        let d = e.edge(q, fray);
                        // Scree band: at the foot outside a hollow's
                        // floor, inside a mesa or step.
                        let x = if e.high_inside { d } else { width - d };
                        // Prompt 129: on a slope, debris at the foot of
                        // its rock bands only.
                        if let Some(s) = &e.slope {
                            let low = s.band_foot(e.hi - e.lo);
                            if x <= low - 6.0 || x >= low - 0.5 || s.cliff_weight(q) < 0.6 {
                                continue;
                            }
                        } else if x <= 0.4 || x >= tw - 0.3 {
                            continue;
                        }
                        let on_ramp = e.ramps.iter().any(|r| {
                            let (u, v) = r.frame(q);
                            u > r.u_top - 6.0 && u < r.u_top + r.len + 6.0 && v < r.half_w + 14.0
                        }) || e.paths.iter().any(|r| r.near(q, 14.0));
                        if on_ramp {
                            break;
                        }
                        let (kind, variant) = if roll < 0.06 {
                            ("env_rock_outcrop", ROCKS[(pick * 6.0) as usize % 6])
                        } else if roll < 0.2 {
                            ("env_rock", "Large")
                        } else if roll < 0.6 {
                            ("env_rock", "Medium")
                        } else {
                            ("env_rock", "Small")
                        };
                        if objects.place(kind, variant, q, heading, 0.3) {
                            placed += 1;
                        }
                        break;
                    }
                }
            }
        }
    }
    placed
}
