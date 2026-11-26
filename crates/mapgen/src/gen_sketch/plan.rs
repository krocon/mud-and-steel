//! Geometry helpers, crossing/bridge layout and terrain/height planning.

use super::mesa::{Fray, MesaPlan};
use super::schema::*;
use super::settlements::peak_profile;
use super::{
    BANK_CUT_M, BRIDGE_LAND_M, CELL_M, CHANNEL_DEPTH_M, DEEP_WATER, DIRT_TRACK,
    EMBANKMENT_SIDE_GRADE, FIELD_EDGE_M, FIELD_GRAIN, FIELD_GREEN, FIELD_PLOUGHED,
    FORD_HALF_WIDTH_M, GRASS, MUD, NARROW_M, RAIL_BED_M, RAIL_SHOULDER_M, RAMP_GRADE,
    RIVER_VALLEY_M, ROAD, SAND, SEA_COAST_M, SHALLOW_DEPTH_M, SHALLOW_WATER, TAPER_M, WATERLINE_M,
};
use crate::gen_land::{Way, dist, smoothstep};

// --- Geometry --------------------------------------------------------------

pub(super) fn segment_distance(p: (f64, f64), a: P, b: P) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a[0]) * dx + (p.1 - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, (a[0] + t * dx, a[1] + t * dy))
}

pub(super) fn polyline_distance(p: (f64, f64), points: &[P]) -> f64 {
    points
        .windows(2)
        .map(|s| segment_distance(p, s[0], s[1]))
        .fold(f64::INFINITY, f64::min)
}

pub(super) fn inside(p: (f64, f64), polygon: &[P]) -> bool {
    let mut odd = false;
    let n = polygon.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (polygon[i], polygon[j]);
        if (a[1] > p.1) != (b[1] > p.1) {
            let x = a[0] + (p.1 - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p.0 < x {
                odd = !odd;
            }
        }
        j = i;
    }
    odd
}

/// Distance from `p` to the outline of `polygon`.
pub(super) fn edge_distance(p: (f64, f64), polygon: &[P]) -> f64 {
    let n = polygon.len();
    (0..n)
        .map(|i| segment_distance(p, polygon[i], polygon[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

pub(super) fn bounds(polygon: &[P]) -> crate::gen_land::Rect {
    let mut r = crate::gen_land::Rect {
        x0: f64::INFINITY,
        y0: f64::INFINITY,
        x1: f64::NEG_INFINITY,
        y1: f64::NEG_INFINITY,
    };
    for p in polygon {
        r.x0 = r.x0.min(p[0]);
        r.y0 = r.y0.min(p[1]);
        r.x1 = r.x1.max(p[0]);
        r.y1 = r.y1.max(p[1]);
    }
    r
}

// --- The plan --------------------------------------------------------------

/// A crossing after snapping: its axis on a path cell center line.
#[derive(Clone, Copy)]
pub(super) struct Crossing {
    pub(super) kind: CrossingKind,
    pub(super) x: f64,
    pub(super) y: f64,
    /// True: the way runs north–south (bridge heading 0).
    pub(super) ns: bool,
    /// Deck height above the bank (m).
    pub(super) rise: f64,
    /// Long bridges: reach of the water measurement along the way (m).
    pub(super) reach: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CrossingKind {
    Stone,
    Wood,
    Ford,
    /// Prompt 122: high stone arches (`env_bridge_stone` `ArchHigh`, 20 m
    /// each) end to end over the whole water; the deck runs over the chain
    /// (prompt 115).
    StoneLong,
}

impl CrossingKind {
    /// Half width of the water at the crossing (the longest bridge of the
    /// kind spans it with 4 m of bank on each side).
    pub(super) const fn half_water(self) -> f64 {
        match self {
            Self::Stone => 10.5,
            Self::Wood => 3.6,
            Self::Ford | Self::StoneLong => f64::INFINITY,
        }
    }

    pub(super) const fn is_bridge(self) -> bool {
        matches!(self, Self::Stone | Self::Wood | Self::StoneLong)
    }

    /// Default deck height above the bank (m).
    pub(super) const fn rise(self) -> f64 {
        match self {
            Self::Wood => 1.5,
            Self::StoneLong => 6.0,
            Self::Stone | Self::Ford => 3.0,
        }
    }
}

/// Length of one piece of a long stone bridge (m).
pub(super) const LONG_PIECE_M: f64 = 20.0;
/// Reach of a long bridge's water measurement and approach along the way.
pub(super) const LONG_REACH_M: f64 = 150.0;

pub(super) struct Plan<'a> {
    pub(super) sketch: &'a Sketch,
    pub(super) ways: Vec<Way>,
    /// Painted terrain: id, blotch noise and its threshold.
    pub(super) paints: Vec<(u8, Option<crate::gen_land::Noise>, f64)>,
    pub(super) crossings: Vec<Crossing>,
    pub(super) flats: Vec<((f64, f64), f64, f64)>,
    pub(super) ground: crate::gen_land::Noise,
    pub(super) detail: crate::gen_land::Noise,
    pub(super) bumps: Vec<(f64, f64, f64, f64)>,
    /// Rock mountains (prompt 122): center, radius, height.
    pub(super) peaks: Vec<(f64, f64, f64, f64)>,
    pub(super) crag: crate::gen_land::Noise,
    /// Field plots of the patchworks.
    pub(super) plots: Vec<Area>,
    /// Outline segments of the bays that are not inside another bay (the
    /// coast of the bays).
    pub(super) bay_coast: Vec<(P, P)>,
    /// Puddles of the marshes (prompt 124): center and radius.
    pub(super) puddles: Vec<(f64, f64, f64)>,
    /// Table mountains (prompt 126) and the noise that frays their edges.
    pub(super) mesas: Vec<MesaPlan>,
    pub(super) fray: Fray,
}

impl Plan<'_> {
    pub(super) fn natural(&self, x: f64, y: f64) -> f64 {
        let mut h = 2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y);
        if !self.mesas.is_empty() {
            h += self
                .mesas
                .iter()
                .map(|m| m.height((x, y), &self.fray))
                .sum::<f64>();
        }
        for &(bx, by, r, a) in &self.bumps {
            let d2 = ((x - bx) * (x - bx) + (y - by) * (y - by)) / (r * r);
            if d2 < 1.0 {
                let t = 1.0 - d2;
                h += a * t * t;
            }
        }
        for &(px, py, r, a) in &self.peaks {
            let d2 = ((x - px) * (x - px) + (y - py) * (y - py)) / (r * r);
            if d2 < 1.0 {
                h += a * peak_profile(d2.sqrt(), self.crag.at(x, y));
            }
        }
        h
    }

    /// The plain ground at `(x, y)`: the ground noise without bumps,
    /// hills and mountains (prompt 125; the level of rivers through hills
    /// and of broad rivers, prompt 123).
    pub(super) fn plain(&self, x: f64, y: f64) -> f64 {
        2.4 * self.ground.at(x, y) + 0.5 * self.detail.at(x, y)
    }

    /// Half width of `river` near `p` (narrowed at crossings).
    pub(super) fn river_half(&self, river: &RiverDef, p: (f64, f64)) -> f64 {
        let w = river.width_m / 2.0;
        self.crossings.iter().fold(w, |acc, c| {
            let hw = c.kind.half_water();
            if hw >= w {
                return acc;
            }
            let blend = smoothstep(NARROW_M, TAPER_M, dist(p, (c.x, c.y)));
            acc.min(hw + (w - hw) * blend)
        })
    }

    pub(super) fn sample(&self, x: f64, y: f64) -> (f64, u8) {
        let p = (x, y);
        let mut h = self.natural(x, y);
        for &(c, r, level) in &self.flats {
            let blend = smoothstep(r, r + 40.0, dist(p, c));
            h = level + (h - level) * blend;
        }
        let s = self.sketch;
        let mut terrain = GRASS;
        for f in s.field.iter().chain(&self.plots) {
            if inside(p, &f.polygon_m) && edge_distance(p, &f.polygon_m) > FIELD_EDGE_M {
                terrain = match f.kind.as_str() {
                    "green" => FIELD_GREEN,
                    "ploughed" => FIELD_PLOUGHED,
                    _ => FIELD_GRAIN,
                };
            }
        }
        // Prompt 124: lanes keep their ground (firm ways through the mud).
        let lane = s
            .lane
            .iter()
            .any(|l| polyline_distance(p, &l.points_m) < l.width_m / 2.0);
        for (paint, (id, blotch, threshold)) in s.paint.iter().zip(&self.paints) {
            if lane || !inside(p, &paint.polygon_m) {
                continue;
            }
            let painted = blotch.as_ref().is_none_or(|n| {
                // Blotches: two octaves, denser along ways, thinning out
                // towards the outline (no straight edges).
                let way = self
                    .ways
                    .iter()
                    .map(|w| w.distance(x, y) - w.half)
                    .fold(f64::INFINITY, f64::min);
                let value = 0.7 * n.at(x, y)
                    + 0.3 * n.at(x * 2.7 + 500.0, y * 2.7 - 300.0)
                    + 0.22 * (1.0 - smoothstep(0.0, 18.0, way))
                    - 0.3 * (1.0 - smoothstep(0.0, 45.0, edge_distance(p, &paint.polygon_m)));
                value > *threshold
            });
            if painted {
                terrain = *id;
            }
        }
        // Prompt 126: a dirt track on every mesa ramp.
        if self.mesas.iter().any(|m| m.on_track(p)) {
            terrain = DIRT_TRACK;
        }
        for way in &self.ways {
            if way.distance(x, y) < way.half {
                terrain = way.id;
                break;
            }
        }
        for r in &s.ring_road {
            if (dist(p, (r.center_m[0], r.center_m[1])) - r.radius_m).abs() < r.width_m / 2.0 {
                terrain = DIRT_TRACK;
            }
        }
        if s.square.iter().any(|a| inside(p, &a.polygon_m)) {
            terrain = ROAD;
        }
        for rail in &s.rail {
            let d = polyline_distance(p, &rail.points_m);
            if d < RAIL_SHOULDER_M {
                h += rail.embankment_m.unwrap_or(0.8)
                    * (1.0 - smoothstep(RAIL_BED_M, RAIL_SHOULDER_M, d));
                if d < RAIL_BED_M {
                    terrain = DIRT_TRACK;
                }
            }
        }
        // Rivers.
        for river in &s.river {
            let valley = river.valley_m.unwrap_or(RIVER_VALLEY_M);
            let d = polyline_distance(p, &river.points_m);
            if d > river.width_m / 2.0 + valley {
                continue;
            }
            let w = self.river_half(river, p);
            if d >= w + valley {
                continue;
            }
            // Prompt 125: the ground the water and banks are cut into (the
            // plain ground for a river through hills).
            let base = if river.through_hills {
                self.plain(x, y)
            } else {
                self.natural(x, y)
            };
            let island = -self.islet_edge(p);
            let ford = river.shallow
                || self.crossings.iter().any(|c| {
                    c.kind == CrossingKind::Ford
                        && if c.ns {
                            (x - c.x).abs() < FORD_HALF_WIDTH_M && (y - c.y).abs() < 60.0
                        } else {
                            (y - c.y).abs() < FORD_HALF_WIDTH_M && (x - c.x).abs() < 60.0
                        }
                });
            if d < w && island > 0.0 {
                let channel = if ford { 0.0 } else { 1.0 };
                let h_bank = h.min(base) - BANK_CUT_M - WATERLINE_M;
                h = h_bank
                    - SHALLOW_DEPTH_M * smoothstep(w, w * 0.55, d)
                    - CHANNEL_DEPTH_M * channel * smoothstep(w * 0.55, w * 0.25, d);
                terrain = if d < w * 0.55 && !ford {
                    DEEP_WATER
                } else {
                    SHALLOW_WATER
                };
                if island < 3.0 {
                    terrain = SHALLOW_WATER;
                }
                // A joining tributary keeps its own, deeper bed in the mouth.
                for other in s.river.iter().filter(|o| o.joins) {
                    let d2 = polyline_distance(p, &other.points_m);
                    let w2 = self.river_half(other, p);
                    if std::ptr::eq(other, river) || d2 >= w2 {
                        continue;
                    }
                    let bed = h_bank
                        - SHALLOW_DEPTH_M * smoothstep(w2, w2 * 0.55, d2)
                        - CHANNEL_DEPTH_M * smoothstep(w2 * 0.55, w2 * 0.25, d2);
                    if bed < h {
                        h = bed;
                        terrain = if d2 < w2 * 0.55 {
                            DEEP_WATER
                        } else {
                            SHALLOW_WATER
                        };
                    }
                }
                return (h, terrain);
            }
            if river.through_hills && island > 0.0 {
                // The banks fall from the hills to the cut plain ground.
                let level = base - BANK_CUT_M;
                h = h.min(level + (h - level) * smoothstep(w, w + valley, d));
            } else {
                let cut = if island <= 0.0 {
                    BANK_CUT_M * smoothstep(0.0, -6.0, island).max(0.0) * 0.6
                } else {
                    BANK_CUT_M * (1.0 - smoothstep(w, w + valley, d))
                };
                h -= cut.max(0.0);
            }
            if (d < w + 3.0 || island <= 0.0) && terrain == GRASS {
                terrain = SAND;
            }
        }
        // Prompt 133: dry wadis (a sand bed between rims, no water).
        for wadi in &s.wadi {
            let d = polyline_distance(p, &wadi.points_m);
            let half = wadi.width_m / 2.0;
            if d >= half + WadiDef::FORD_RIM_M {
                continue;
            }
            let rim = wadi.rim(p);
            if d >= half + rim {
                continue;
            }
            h -= wadi.depth_m.unwrap_or(3.5) * (1.0 - smoothstep(half, half + rim, d));
            // A gravel bed (the dirt track's ground), sand on the rims.
            if matches!(
                terrain,
                GRASS | SAND | FIELD_GRAIN | FIELD_GREEN | FIELD_PLOUGHED
            ) {
                if d < half {
                    terrain = DIRT_TRACK;
                } else if d < half + rim * 0.4 {
                    terrain = SAND;
                }
            }
        }
        if s.sea {
            if let Some(result) = self.sea(p, h, terrain) {
                return result;
            }
            let e = s
                .land
                .iter()
                .filter(|a| inside(p, &a.polygon_m))
                .map(|a| edge_distance(p, &a.polygon_m))
                .fold(f64::INFINITY, f64::min);
            let e = e.min(self.bay_coast_distance(p));
            let islet = self.islet_edge(p);
            let e = if islet > 0.0 { e.min(islet) } else { e };
            if e.is_finite() {
                // Land falls towards one coast height (a flat sea level).
                h = SEA_COAST_M + (h - SEA_COAST_M) * smoothstep(0.0, 60.0, e);
                if e < 5.0 && terrain == GRASS {
                    terrain = SAND;
                }
            }
        }
        // Lakes.
        for lake in &s.lake {
            if lake.kind == "river" {
                match self.river_lake(p, h, terrain, lake) {
                    (h, t, true) => return (h, t),
                    (nh, nt, false) => {
                        h = nh;
                        terrain = nt;
                    }
                }
                continue;
            }
            // Islands of the sketch also stand in lakes (stage 3).
            let island = self.islet_edge(p) > 0.0;
            if inside(p, &lake.polygon_m) && !island {
                let e = edge_distance(p, &lake.polygon_m);
                let deep = lake.kind != "shallow" && e > 8.0;
                h = h.min(self.natural(x, y))
                    - BANK_CUT_M
                    - WATERLINE_M
                    - SHALLOW_DEPTH_M * smoothstep(0.0, 8.0, e)
                    - if deep {
                        CHANNEL_DEPTH_M * smoothstep(8.0, 20.0, e)
                    } else {
                        0.0
                    };
                return (h, if deep { DEEP_WATER } else { SHALLOW_WATER });
            }
            let e = edge_distance(p, &lake.polygon_m);
            if e < RIVER_VALLEY_M {
                h -= BANK_CUT_M * (1.0 - smoothstep(0.0, RIVER_VALLEY_M, e));
                if e < 3.0 && terrain == GRASS {
                    terrain = SAND;
                }
            }
        }
        // Puddles of the marshes (prompt 124): a shallow dip of mud.
        for &(px, py, r) in &self.puddles {
            let d = dist(p, (px, py));
            if d < r + 1.5 {
                h -= 0.3 * (1.0 - smoothstep(r * 0.5, r + 1.5, d));
                if d < r + 0.5 {
                    terrain = MUD;
                }
            }
        }
        (h, terrain)
    }
}

impl Plan<'_> {
    /// A broad river drawn as a lake polygon (`kind = "river"`, prompt 123):
    /// the water follows the plain ground (the noise without bumps and
    /// hills) 3 m below it, so hills beside it fall to the water over
    /// a steep bank instead of lifting it; islands of the sketch slope down
    /// to the bank level within 25 m of their shore; shoals keep the water
    /// shallow (fords). Returns height, terrain and whether `p` is water.
    pub(super) fn river_lake(
        &self,
        p: (f64, f64),
        h: f64,
        terrain: u8,
        lake: &Area,
    ) -> (f64, u8, bool) {
        const SHORE_M: f64 = 45.0;
        const ISLAND_SHORE_M: f64 = 25.0;
        let level = 2.4 * self.ground.at(p.0, p.1) + 0.5 * self.detail.at(p.0, p.1) - BANK_CUT_M;
        let island = self.islet_edge(p);
        if inside(p, &lake.polygon_m) {
            if island > 0.0 {
                let h = level + (h - level) * smoothstep(0.0, ISLAND_SHORE_M, island);
                let terrain = if island < 4.0 && terrain == GRASS {
                    SAND
                } else {
                    terrain
                };
                return (h, terrain, false);
            }
            let e = edge_distance(p, &lake.polygon_m).min(-island);
            let shoal = self
                .sketch
                .shoal
                .iter()
                .any(|c| polyline_distance(p, &c.points_m) < c.width_m / 2.0);
            let deep = !shoal && e > 8.0;
            let h = level
                - WATERLINE_M
                - SHALLOW_DEPTH_M * smoothstep(0.0, 8.0, e)
                - if deep {
                    CHANNEL_DEPTH_M * smoothstep(8.0, 20.0, e)
                } else {
                    0.0
                };
            return (h, if deep { DEEP_WATER } else { SHALLOW_WATER }, true);
        }
        let e = edge_distance(p, &lake.polygon_m);
        if e >= SHORE_M {
            return (h, terrain, false);
        }
        let h = level + (h - level) * smoothstep(0.0, SHORE_M, e);
        let terrain = if e < 3.0 && terrain == GRASS {
            SAND
        } else {
            terrain
        };
        (h, terrain, false)
    }

    /// The sea at `p` (outside every land polygon): water whose depth grows
    /// with the distance to the coast, shallow on shoals; causeways stay
    /// land except under their bridges. `None` on land.
    /// Sea maps: how far `p` lies inside an island of the sketch (m), or
    /// a negative value outside every island.
    pub(super) fn islet_edge(&self, p: (f64, f64)) -> f64 {
        self.sketch
            .island
            .iter()
            .map(|i| i.depth(p))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    pub(super) fn bay_coast_distance(&self, p: (f64, f64)) -> f64 {
        self.bay_coast
            .iter()
            .map(|&(a, b)| segment_distance(p, a, b))
            .fold(f64::INFINITY, f64::min)
    }

    pub(super) fn sea(&self, p: (f64, f64), h: f64, terrain: u8) -> Option<(f64, u8)> {
        let s = self.sketch;
        // Islands of the sketch also stand in the bays (prompt 122: the
        // islet in the lagoon).
        if self.islet_edge(p) > 0.0 {
            return None;
        }
        let in_bay = s.bay.iter().any(|a| inside(p, &a.polygon_m));
        if !in_bay && s.land.iter().any(|a| inside(p, &a.polygon_m)) {
            return None;
        }
        // Under a bridge the water cuts across the whole causeway.
        let gap = self.crossings.iter().any(|c| {
            let (u, v) = if c.ns {
                (p.1 - c.y, p.0 - c.x)
            } else {
                (p.0 - c.x, p.1 - c.y)
            };
            matches!(c.kind, CrossingKind::Stone | CrossingKind::Wood)
                && u.abs() < c.kind.half_water()
                && v.abs() < 40.0
        });
        let causeway = s
            .causeway
            .iter()
            .map(|c| polyline_distance(p, &c.points_m) - c.width_m / 2.0)
            .fold(f64::INFINITY, f64::min);
        if causeway < 0.0 && !gap {
            let top = SEA_COAST_M + 0.6;
            let terrain = if terrain == GRASS && causeway > -1.5 {
                SAND
            } else {
                terrain
            };
            return Some((top.max(h.min(top + 0.5)), terrain));
        }
        let e = s
            .land
            .iter()
            .map(|a| edge_distance(p, &a.polygon_m))
            .fold(causeway.max(0.0), f64::min)
            .min(self.bay_coast_distance(p))
            .min(-self.islet_edge(p));
        let shoal = s
            .shoal
            .iter()
            .any(|c| polyline_distance(p, &c.points_m) < c.width_m / 2.0);
        let deep = !shoal && e > 10.0;
        let bed = SEA_COAST_M
            - WATERLINE_M
            - SHALLOW_DEPTH_M * smoothstep(0.0, 10.0, e)
            - if deep {
                CHANNEL_DEPTH_M * smoothstep(10.0, 25.0, e) + 1.5 * smoothstep(25.0, 120.0, e)
            } else {
                0.0
            };
        Some((bed, if deep { DEEP_WATER } else { SHALLOW_WATER }))
    }
}

/// A placed bridge (`objects.csv` row).
pub(super) struct BridgeRow {
    /// Index of its crossing (pieces of a chain share it).
    pub(super) crossing: usize,
    pub(super) kind: &'static str,
    pub(super) variant: &'static str,
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) heading: u16,
}

/// Measures the water along each bridge crossing's axis, picks the shortest
/// bridge that spans it plus 4 m of bank on each side and raises
/// embankments with ramps to the deck (like `gen_map`, prompt 014).
pub(super) fn place_bridges(
    crossings: &[Crossing],
    (sx, sy): (usize, usize),
    half: (f64, f64),
    heights: &mut [f64],
    terrain: &mut [u8],
    dry_bed: &dyn Fn(f64, f64) -> bool,
) -> Result<Vec<BridgeRow>, String> {
    const WOOD: [(&str, f64); 3] = [("Span8", 8.0), ("Span12", 12.0), ("Span16", 16.0)];
    const STONE: [(&str, f64); 2] = [("Arch2", 20.0), ("Arch3", 30.0)];
    let coord = |i: usize, h: f64| -h + i as f64 * CELL_M;
    let mut placed = Vec::new();
    for (index, c) in crossings
        .iter()
        .enumerate()
        .filter(|(_, c)| c.kind.is_bridge())
    {
        let stone = c.kind != CrossingKind::Wood;
        let long = c.kind == CrossingKind::StoneLong;
        let reach = if long { c.reach } else { 60.0 };
        // u: along the way, v: across (the axis).
        let to_uv = |x: f64, y: f64| {
            if c.ns {
                (y - c.y, x - c.x)
            } else {
                (x - c.x, y - c.y)
            }
        };
        let (mut u0, mut u1) = (f64::INFINITY, f64::NEG_INFINITY);
        for j in 0..sy {
            for i in 0..sx {
                let (u, v) = to_uv(coord(i, half.0), coord(j, half.1));
                if v.abs() <= 3.0
                    && u.abs() < reach
                    && (matches!(terrain[j * sx + i], SHALLOW_WATER | DEEP_WATER)
                        || dry_bed(coord(i, half.0), coord(j, half.1)))
                {
                    u0 = u0.min(u);
                    u1 = u1.max(u);
                }
            }
        }
        if u0 > u1 {
            return Err(format!("no water at the crossing ({}, {})", c.x, c.y));
        }
        let need = u1 - u0 + 2.0 * BRIDGE_LAND_M;
        let list: &[(&str, f64)] = if stone { &STONE } else { &WOOD };
        // A long bridge: as many 20 m pieces as the water needs.
        let pieces = (need / LONG_PIECE_M).ceil().max(2.0);
        let chain = [("ArchHigh", pieces * LONG_PIECE_M)];
        let list: &[(&str, f64)] = if long { &chain } else { list };
        let Some(&(variant, length)) = list.iter().find(|v| v.1 >= need) else {
            return Err(format!(
                "the water at ({}, {}) is {:.1} m wide, too wide for the bridge",
                c.x,
                c.y,
                u1 - u0
            ));
        };
        let (rise, half_width) = (c.rise, if stone { 2.55 } else { 2.05 });
        let mid = ((u0 + u1) / 2.0 * 1000.0).round() / 1000.0;
        let (d0, d1) = (mid - length / 2.0, mid + length / 2.0);
        let at = |u: f64| -> usize {
            let (x, y) = if c.ns { (c.x, c.y + u) } else { (c.x + u, c.y) };
            let i = ((x + half.0) / CELL_M).round().clamp(0.0, (sx - 1) as f64) as usize;
            let j = ((y + half.1) / CELL_M).round().clamp(0.0, (sy - 1) as f64) as usize;
            j * sx + i
        };
        let bank = heights[at(d0)].max(heights[at(d1)]);
        let deck = bank + rise;
        let top_half = half_width + 1.0;
        let reach_u = rise / RAMP_GRADE + 10.0;
        let reach_v = top_half + rise / EMBANKMENT_SIDE_GRADE + 4.0;
        for j in 0..sy {
            for i in 0..sx {
                let (u, v) = to_uv(coord(i, half.0), coord(j, half.1));
                if v.abs() > reach_v || u < d0 - reach_u || u > d1 + reach_u {
                    continue;
                }
                let k = j * sx + i;
                if matches!(terrain[k], SHALLOW_WATER | DEEP_WATER)
                    || dry_bed(coord(i, half.0), coord(j, half.1))
                {
                    continue;
                }
                let along = if u < d0 {
                    d0 - u
                } else if u > d1 {
                    u - d1
                } else {
                    0.0
                };
                let top = deck - RAMP_GRADE * along;
                let level = top - EMBANKMENT_SIDE_GRADE * (v.abs() - top_half).max(0.0);
                if level > heights[k] {
                    heights[k] = level;
                    terrain[k] = if v.abs() <= 2.5 {
                        if stone { ROAD } else { DIRT_TRACK }
                    } else {
                        GRASS
                    };
                }
            }
        }
        // Pieces end to end (a single bridge is one piece of `length`).
        let (count, piece) = if long {
            (pieces as usize, LONG_PIECE_M)
        } else {
            (1, length)
        };
        for k in 0..count {
            // (One piece: its middle exactly, as before prompt 122.)
            let at = if long {
                d0 + piece * (k as f64 + 0.5)
            } else {
                mid
            };
            let (x, y) = if c.ns {
                (c.x, c.y + at)
            } else {
                (c.x + at, c.y)
            };
            placed.push(BridgeRow {
                crossing: index,
                kind: if stone {
                    "env_bridge_stone"
                } else {
                    "env_bridge_wood"
                },
                variant,
                x,
                y,
                heading: if c.ns { 0 } else { 90 },
            });
        }
    }
    Ok(placed)
}

/// Wood stands per kind and biome: (catalog id, variants).
pub(super) fn stands(kind: &str, biome: &str) -> Vec<(&'static str, &'static [&'static str])> {
    const OAK: (&str, &[&str]) = ("env_tree_deciduous", &["Oak", "Beech", "OakYoung"]);
    const BARE: (&str, &[&str]) = ("env_tree_deciduous", &["Bare"]);
    const CONIFER: (&str, &[&str]) = ("env_tree_conifer", &["Spruce", "Pine", "SpruceYoung"]);
    const BIRCH: (&str, &[&str]) = ("env_tree_birch", &["Birch", "BirchYoung"]);
    const PALM: (&str, &[&str]) = ("env_tree_palm", &["Palm", "PalmShort", "PalmLeaning"]);
    const BUSH: (&str, &[&str]) = ("env_bush", &["Hazel", "Round", "Wide"]);
    // Prompt 120: Mediterranean stands.
    const STONE_PINE: (&str, &[&str]) = (
        "env_tree_stone_pine",
        &["StonePine", "StonePineYoung", "StonePineLeaning"],
    );
    const OLIVE: (&str, &[&str]) = ("env_tree_olive", &["Olive", "OliveOld", "OliveYoung"]);
    const CYPRESS: (&str, &[&str]) = ("env_tree_cypress", &["Cypress", "CypressShort"]);
    const MACCHIA: (&str, &[&str]) = ("env_bush_macchia", &["Holm", "Strawberry", "Broom"]);
    let winter = biome == "winter";
    match (kind, biome) {
        ("grove", "mediterranean") => vec![OLIVE, OLIVE, MACCHIA, OLIVE, CYPRESS],
        (_, "mediterranean") => vec![STONE_PINE, MACCHIA, STONE_PINE, OLIVE, MACCHIA, CYPRESS],
        // Prompt 202: in the desert the scrub is camelthorn.
        ("scrub", "desert") => vec![
            ("env_bush_camelthorn", &["Thicket", "Wide", "Sparse"]),
            ("env_rock", &["Small", "Medium"]),
            ("env_bush_camelthorn", &["Sparse", "Thicket"]),
        ],
        // Prompt 126: dry scrub in desert hollows: bushes and stones.
        ("scrub", _) => vec![
            ("env_bush", &["Round", "Wide", "Hazel"]),
            ("env_rock", &["Small", "Medium"]),
            ("env_bush", &["Wide", "Round"]),
        ],
        // Palm and bush groves (stage 2): bushes show the groves from above.
        ("grove", _) => vec![PALM, PALM, BUSH, PALM, BUSH],
        // Rock heights (stage 3): outcrops and boulders between conifers.
        ("rock", _) => vec![
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
            CONIFER,
            ("env_rock", &["Medium", "Small", "Large"]),
            CONIFER,
        ],
        // Rock mountains (prompt 122): massifs and outcrops between a few
        // conifers.
        ("crag", _) => vec![
            (
                "env_rock_massif",
                &["Ridge", "Long", "Short", "High", "Double"],
            ),
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
            CONIFER,
            (
                "env_rock_outcrop",
                &["Group", "Block", "Split", "Pair", "Leaning", "Layered"],
            ),
        ],
        // Prompt 124: a shot-up wood: bare and broken trees, stumps and
        // fallen trunks.
        ("shattered", _) => vec![
            BARE,
            ("env_tree_stump", &["Broken", "Broken", "Sawn"]),
            BARE,
            ("env_log_fallen", &["Log"]),
            ("env_tree_stump", &["Broken"]),
        ],
        (_, "desert") => vec![PALM],
        ("conifer", _) => vec![CONIFER],
        ("birch", _) => vec![BIRCH],
        ("palm", _) => vec![PALM],
        ("deciduous", _) => vec![if winter { BARE } else { OAK }],
        // Mixed: in winter mostly conifers (the template shows dark woods).
        _ if winter => vec![CONIFER, CONIFER, BIRCH, CONIFER, BARE],
        _ => vec![OAK, CONIFER, OAK, BIRCH, OAK],
    }
}
