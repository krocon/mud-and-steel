//! Relief profiles of `cli gen-map` (prompt 060, `--relief`).
//!
//! - `gentle` (default): the rolling ground and hills of the test maps,
//!   unchanged (bit-identical files).
//! - `hilly`: the same layout with ground noise and hills [`HILLY_FACTOR`]
//!   times as high.
//! - `mountains`: ridged mountains up to [`PEAK_M`] between the valleys.
//!   Every track, road and the river runs in a valley whose floor stays at
//!   the level of the central meadow; where a track crosses the range
//!   between two starts it is a pass. Distant starts lie in basins
//!   [`BASIN_DROP_M`] below the passes. Cliff bands turn some slopes into
//!   benches and rock walls, and a serpentine road with hairpins climbs from
//!   the meadow to a lookout plateau (marker `lookout`).
//!
//! Its own random stream ([`MOUNTAIN_SALT`]), so the layout and the objects
//! of a seed do not depend on the profile.

use super::{Layout, Path2, Random, ValueNoise, smoothstep};
use sim::Rng;
use std::f64::consts::FRAC_1_SQRT_2;

/// Terrain profile of the generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relief {
    Gentle,
    Hilly,
    Mountains,
}

impl Relief {
    pub const NAMES: [&'static str; 3] = ["gentle", "hilly", "mountains"];

    pub fn parse(name: &str) -> Result<Self, String> {
        match name {
            "gentle" => Ok(Self::Gentle),
            "hilly" => Ok(Self::Hilly),
            "mountains" => Ok(Self::Mountains),
            _ => Err(format!(
                "unknown relief '{name}' (gentle, hilly, mountains)"
            )),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Gentle => "gentle",
            Self::Hilly => "hilly",
            Self::Mountains => "mountains",
        }
    }
}

/// `hilly`: ground noise and hills this many times as high. The steepest
/// hill grade (about 35 % on `gentle`) stays just under the 60 % limit.
pub const HILLY_FACTOR: f64 = 1.7;

/// Seed offset of the mountains' random stream ("mountain").
const MOUNTAIN_SALT: u64 = 0x6d6f_756e_7461_696e;
/// Height of the highest ridges above the valley floors.
pub const PEAK_M: f64 = 480.0;
/// Starts far from the center lie in basins this far below the passes.
pub const BASIN_DROP_M: f64 = 30.0;
/// Flat floor of a basin around a start, and the distance over which it
/// rises to the pass level and the mountains begin.
const BASIN_RADIUS_M: f64 = 210.0;
const BASIN_RISE_M: f64 = 230.0;
/// Half width of the valley floor along tracks and roads.
const VALLEY_FLOOR_M: f64 = 22.0;
/// Along the river: its half width plus this.
const RIVER_FLOOR_M: f64 = 50.0;
/// Distance from a valley floor over which the mountains reach full height.
const VALLEY_RISE_M: f64 = 150.0;
/// Height of one cliff band: benches with rock walls between them.
const CLIFF_STEP_M: f64 = 14.0;

// Serpentine (marker `lookout`).
/// Grade of the serpentine road, well under the 60 % limit.
pub const SERPENTINE_GRADE: f64 = 0.11;
const LEGS: usize = 6;
/// Distance between neighboring legs (twice the hairpin radius).
const LEG_SPACING_M: f64 = 26.0;
/// Half length of a leg.
const LEG_HALF_M: f64 = 50.0;
/// Half width of the flat road bed (tanks need about 8 m, see MAPS.md).
const ROAD_HALF_M: f64 = 6.0;
/// Half width of the gravel road painted on the bed.
const ROAD_PAINT_M: f64 = 3.0;
/// Grade of the cut and fill walls beside the road bed (rock walls).
const ROAD_SIDE_GRADE: f64 = 1.2;
const PLATEAU_RADIUS_M: f64 = 24.0;

/// Mountain heights for the `mountains` profile.
pub(super) struct Mountains {
    massif: ValueNoise,
    ridges: Vec<(ValueNoise, f64)>,
    cliffs: ValueNoise,
    /// Basins around distant starts: (x, y).
    basins: Vec<(f64, f64)>,
    pub(super) serpentine: Serpentine,
}

impl Mountains {
    pub(super) fn new(seed: u64, layout: &Layout) -> Self {
        let mut random = Random(Rng::new(seed ^ MOUNTAIN_SALT));
        let size = layout.half * 2.0;
        let massif = ValueNoise::new(&mut random, size, 900.0);
        let ridges = [(620.0, 1.0), (260.0, 0.5), (110.0, 0.22), (45.0, 0.08)]
            .into_iter()
            .map(|(wavelength, weight)| (ValueNoise::new(&mut random, size, wavelength), weight))
            .collect();
        let cliffs = ValueNoise::new(&mut random, size, 380.0);
        Self {
            massif,
            ridges,
            cliffs,
            basins: layout
                .start_meadows
                .iter()
                .map(|&(x, y, _)| (x, y))
                .collect(),
            serpentine: Serpentine::new(layout.meadow_radius),
        }
    }

    /// Height added to the gentle ground at `(x, y)`.
    pub(super) fn height(&self, layout: &Layout, x: f64, y: f64) -> f64 {
        // Basins: 1 on the floor around a distant start, 0 beyond the rise.
        let basin = self
            .basins
            .iter()
            .map(|&(bx, by)| {
                let d = ((x - bx) * (x - bx) + (y - by) * (y - by)).sqrt();
                1.0 - smoothstep(BASIN_RADIUS_M, BASIN_RADIUS_M + BASIN_RISE_M, d)
            })
            .fold(0.0, f64::max);
        let floor = -BASIN_DROP_M * basin;
        let away = self.valley_distance(layout, x, y);
        if away <= 0.0 {
            return floor;
        }
        let mask = smoothstep(0.0, VALLEY_RISE_M, away) * (1.0 - basin);
        if mask <= 0.0 {
            return floor;
        }
        // Ridged noise: creases of the value noise become crests.
        let (mut sum, mut weights) = (0.0, 0.0);
        for (noise, weight) in &self.ridges {
            let r = 1.0 - noise.at(x, y).abs();
            sum += r * r * weight;
            weights += weight;
        }
        let ridged = sum / weights;
        let massif = 0.55 + 0.45 * self.massif.at(x, y);
        let mut m = PEAK_M * massif * ridged * mask;
        // Cliff bands: benches and rock walls where the cliff noise is high.
        let cliff = smoothstep(0.1, 0.35, self.cliffs.at(x, y));
        if cliff > 0.0 {
            let steps = m / CLIFF_STEP_M;
            let terraced = CLIFF_STEP_M * (steps.floor() + smoothstep(0.72, 1.0, steps.fract()));
            m += (terraced - m) * cliff;
        }
        floor + m
    }

    /// Distance beyond the nearest valley floor or low area (negative
    /// inside), capped at the rise distance.
    fn valley_distance(&self, layout: &Layout, x: f64, y: f64) -> f64 {
        let mut best = VALLEY_RISE_M;
        let mut low = |d: f64| best = best.min(d);
        for paths in [&layout.tracks, &layout.bridge_tracks, &layout.roads] {
            for p in paths.iter() {
                if p.near(x, y, VALLEY_FLOOR_M + VALLEY_RISE_M) {
                    low(p.distance(x, y) - VALLEY_FLOOR_M);
                }
            }
        }
        let river_floor = layout.river_width + RIVER_FLOOR_M;
        if layout.river.near(x, y, river_floor + VALLEY_RISE_M) {
            low(layout.river.distance(x, y) - river_floor);
        }
        let round = |cx: f64, cy: f64| ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt();
        low(round(0.0, 0.0) - layout.meadow_radius - 30.0);
        for s in &layout.squares {
            low(round(s.x, s.y) - s.radius - 30.0);
        }
        // The landmark hill stays a hill (supply point on top).
        if let Some(hill) = layout.hills.first() {
            low(round(hill.x, hill.y) - hill.radius);
        }
        best
    }
}

/// A road with hairpins from the west rim of the central meadow up to a
/// plateau, cut into a ramp at [`SERPENTINE_GRADE`].
pub(super) struct Serpentine {
    pub(super) road: Path2,
    /// Road height at every point of `road`.
    heights: Vec<f64>,
    /// Plateau at the top: center, height.
    pub(super) plateau: (f64, f64, f64),
    /// Ramp under the legs: (x, height) by falling x.
    ramp: Vec<(f64, f64)>,
    /// Rectangle of the ramp: x0, y0, x1, y1.
    zone: (f64, f64, f64, f64),
}

impl Serpentine {
    fn new(meadow_radius: f64) -> Self {
        let first_x = -meadow_radius - 30.0;
        let mut points = vec![(-meadow_radius + 10.0, -LEG_HALF_M), (first_x, -LEG_HALF_M)];
        let mut ramp_x = Vec::new();
        let r = LEG_SPACING_M / 2.0;
        for k in 0..LEGS {
            let x = first_x - k as f64 * LEG_SPACING_M;
            let s = if k % 2 == 0 { 1.0 } else { -1.0 };
            // The leg: across the slope.
            points.push((x, s * LEG_HALF_M));
            ramp_x.push(x);
            if k + 1 < LEGS {
                // Hairpin: half circle to the next leg.
                for (c, sn) in HAIRPIN {
                    points.push((x - r + r * c, s * (LEG_HALF_M + r * sn)));
                }
                points.push((x - LEG_SPACING_M, s * LEG_HALF_M));
            }
        }
        let (last_x, last_y) = *points.last().expect("legs");
        let plateau_x = last_x - 40.0;
        points.push((plateau_x, last_y));
        let mut heights = vec![0.0];
        let mut along = 0.0;
        for pair in points.windows(2) {
            let (dx, dy) = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            along += (dx * dx + dy * dy).sqrt();
            heights.push(along * SERPENTINE_GRADE);
        }
        let top = along * SERPENTINE_GRADE;
        // Ramp: the road height in the middle of every leg (y = 0).
        let mut ramp: Vec<(f64, f64)> = Vec::new();
        for &x in &ramp_x {
            let (_, h) = nearest(&points, &heights, x, 0.0);
            ramp.push((x, h));
        }
        ramp.push((plateau_x, top));
        let zone = (
            plateau_x - PLATEAU_RADIUS_M,
            -LEG_HALF_M - r - 8.0,
            first_x + 8.0,
            LEG_HALF_M + r + 8.0,
        );
        Self {
            road: Path2 { points },
            heights,
            plateau: (plateau_x, last_y, top),
            ramp,
            zone,
        }
    }

    /// Ramp height at `x` (linear between the legs).
    fn ramp_at(&self, x: f64) -> f64 {
        let first = self.ramp[0];
        if x >= first.0 {
            return (first.1 - (x - first.0) * first.1 / 30.0).max(0.0);
        }
        for pair in self.ramp.windows(2) {
            let ((x0, h0), (x1, h1)) = (pair[0], pair[1]);
            if x <= x0 && x >= x1 {
                return h0 + (h1 - h0) * (x0 - x) / (x0 - x1);
            }
        }
        self.ramp.last().map_or(0.0, |p| p.1)
    }

    /// Shapes the ground at `(x, y)` (height `h`): the ramp, the plateau
    /// and the road bed with its walls. Returns the height and whether the
    /// road is painted here.
    pub(super) fn apply(&self, x: f64, y: f64, mut h: f64) -> (f64, bool) {
        let (x0, y0, x1, y1) = self.zone;
        let outside = (x0 - x)
            .max(x - x1)
            .max(0.0)
            .max((y0 - y).max(y - y1).max(0.0));
        if outside >= 40.0 {
            return (h, false);
        }
        let zone = 1.0 - smoothstep(0.0, 40.0, outside);
        h += (self.ramp_at(x) - h) * zone;
        let (px, py, top) = self.plateau;
        let dp = ((x - px) * (x - px) + (y - py) * (y - py)).sqrt();
        h += (top - h) * (1.0 - smoothstep(PLATEAU_RADIUS_M, PLATEAU_RADIUS_M + 16.0, dp));
        let (d, road) = nearest(&self.road.points, &self.heights, x, y);
        if d < ROAD_HALF_M {
            h = road;
        } else {
            let wall = ROAD_SIDE_GRADE * (d - ROAD_HALF_M);
            h = h.clamp(road - wall, road + wall);
        }
        (h, d < ROAD_PAINT_M)
    }
}

/// `(cos, sin)` of `k * 22.5°` for `k = 1..=7`, written out so the files do
/// not depend on the platform's `sin`/`cos` (the generator only relies on
/// exactly rounded operations).
const HAIRPIN: [(f64, f64); 7] = [
    (0.923_879_532_511_286_7, 0.382_683_432_365_089_8),
    (FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (0.382_683_432_365_089_8, 0.923_879_532_511_286_7),
    (0.0, 1.0),
    (-0.382_683_432_365_089_8, 0.923_879_532_511_286_7),
    (-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (-0.923_879_532_511_286_7, 0.382_683_432_365_089_8),
];

/// Distance from `(x, y)` to the polyline `points` and the height
/// (interpolated from `heights`) at the nearest point.
fn nearest(points: &[(f64, f64)], heights: &[f64], x: f64, y: f64) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    for (k, pair) in points.windows(2).enumerate() {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 {
            (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (px, py) = (ax + t * dx - x, ay + t * dy - y);
        let d2 = px * px + py * py;
        if d2 < best.0 {
            best = (d2, heights[k] + (heights[k + 1] - heights[k]) * t);
        }
    }
    (best.0.sqrt(), best.1)
}
