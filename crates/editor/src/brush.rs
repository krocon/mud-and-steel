//! Brushes: height, terrain, rivers and objects.
//!
//! A brush is a circle (center and radius in millimeters). Height brushes
//! weigh every sample by a smooth falloff `(1 - d²/r²)²` in 1/1024 steps,
//! so the center gets the full effect and the rim none. Every function
//! returns the samples it may have changed (for redrawing) and records the
//! old state for undo first.

use crate::{EditMap, HEIGHT_LIMIT_MM, PlacedObject, SampleRect};
use sim::Rng;

/// Full weight of the falloff.
pub const WEIGHT_ONE: i64 = 1024;

/// Terrain ids the river brush paints (`data/terrain.toml`).
pub const SAND: u8 = 3;
pub const SHALLOW_WATER: u8 = 6;
pub const DEEP_WATER: u8 = 7;
/// Below the bank level: down to this the bed is sand (the bank), below it
/// shallow water.
pub const RIVER_BANK_MM: i32 = 300;
/// Deeper than this below the bank level the river is deep water (the
/// generator's channel is 1.8 m deep, its shallow parts 0.5 m below the
/// waterline).
pub const RIVER_DEEP_MM: i32 = 800;

/// Terrain the object brush does not plant on: field tracks, roads,
/// shallow and deep water.
const NO_SCATTER: [u8; 4] = [1, 2, SHALLOW_WATER, DEEP_WATER];
/// Attempts per object to find a point inside the circle.
const SCATTER_TRIES: u32 = 8;

/// A circle on the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brush {
    pub x_mm: i32,
    pub y_mm: i32,
    pub radius_mm: u32,
}

impl Brush {
    pub const fn new(x_mm: i32, y_mm: i32, radius_mm: u32) -> Self {
        Self {
            x_mm,
            y_mm,
            radius_mm,
        }
    }

    /// Falloff weight (`0..=WEIGHT_ONE`) at squared distance `d2` (mm²).
    pub fn weight(self, d2: i64) -> i64 {
        let r2 = i64::from(self.radius_mm).pow(2).max(1);
        if d2 > r2 {
            return 0;
        }
        let q = WEIGHT_ONE - d2 * WEIGHT_ONE / r2;
        q * q / WEIGHT_ONE
    }
}

/// How many of `rate_pml` per mille: `value * rate / 1000`, rounded towards
/// zero, at least one step in the direction of `value` when both are
/// positive (so a weak brush still converges).
fn fraction(value: i64, weight: i64, strength_pml: u32) -> i64 {
    let step = value * weight * i64::from(strength_pml.min(1000)) / (WEIGHT_ONE * 1000);
    if step == 0 && value != 0 && weight > 0 && strength_pml > 0 {
        value.signum()
    } else {
        step
    }
}

fn clamp_height(h: i64) -> i32 {
    h.clamp(-i64::from(HEIGHT_LIMIT_MM), i64::from(HEIGHT_LIMIT_MM)) as i32
}

impl EditMap {
    /// Samples within the brush circle's bounding box, clipped to the map.
    pub fn brush_rect(&self, brush: Brush) -> Option<SampleRect> {
        let cell = i64::from(self.cell_mm);
        let r = i64::from(brush.radius_mm);
        let axis = |center: i32, half: i32, n: u32| {
            let lo = (i64::from(center) - r + i64::from(half) + cell - 1)
                .div_euclid(cell)
                .max(0);
            let hi = (i64::from(center) + r + i64::from(half))
                .div_euclid(cell)
                .min(i64::from(n) - 1);
            (lo <= hi).then(|| (lo as u32, (hi - lo + 1) as u32))
        };
        let (i0, w) = axis(brush.x_mm, self.half_x_mm, self.samples_x)?;
        let (j0, h) = axis(brush.y_mm, self.half_y_mm, self.samples_y)?;
        Some(SampleRect { i0, j0, w, h })
    }

    /// Visits every sample in the brush with its falloff weight (> 0) and
    /// lets `f` change height and terrain. `f` gets `(i, j, weight, d2,
    /// height, terrain)`.
    fn paint_samples(
        &mut self,
        brush: Brush,
        mut f: impl FnMut(&Self, u32, u32, i64, i64) -> (i32, u8),
    ) -> Option<SampleRect> {
        let rect = self.brush_rect(brush)?;
        let mut updates: Vec<(usize, i32, u8)> = Vec::with_capacity(rect.count());
        for j in rect.j0..rect.j1() {
            for i in rect.i0..rect.i1() {
                let (x, y) = self.sample_position(i, j);
                let dx = i64::from(x) - i64::from(brush.x_mm);
                let dy = i64::from(y) - i64::from(brush.y_mm);
                let d2 = dx * dx + dy * dy;
                let weight = brush.weight(d2);
                if d2 > i64::from(brush.radius_mm).pow(2) {
                    continue;
                }
                let k = self.index(i, j);
                let (h, t) = f(self, i, j, weight, d2);
                if h != self.heights[k] || t != self.terrain[k] {
                    updates.push((k, h, t));
                }
            }
        }
        if updates.is_empty() {
            return None;
        }
        self.edit(|map| {
            map.touch(rect);
            for (k, h, t) in updates {
                map.heights[k] = h;
                map.terrain[k] = t;
            }
        });
        Some(rect)
    }

    /// Raises the ground by `amount_mm` at the center (negative: lowers),
    /// less towards the rim.
    pub fn raise(&mut self, brush: Brush, amount_mm: i32) -> Option<SampleRect> {
        self.paint_samples(brush, |map, i, j, weight, _| {
            let k = map.index(i, j);
            let h = i64::from(map.heights[k]) + i64::from(amount_mm) * weight / WEIGHT_ONE;
            (clamp_height(h), map.terrain[k])
        })
    }

    /// Moves every sample towards the mean of its 3 x 3 neighborhood by
    /// `strength_pml` per mille at the center.
    pub fn smooth(&mut self, brush: Brush, strength_pml: u32) -> Option<SampleRect> {
        self.paint_samples(brush, |map, i, j, weight, _| {
            let mut sum = 0i64;
            for dj in -1i64..=1 {
                for di in -1i64..=1 {
                    let ni = (i64::from(i) + di).clamp(0, i64::from(map.samples_x) - 1) as u32;
                    let nj = (i64::from(j) + dj).clamp(0, i64::from(map.samples_y) - 1) as u32;
                    sum += i64::from(map.heights[map.index(ni, nj)]);
                }
            }
            let k = map.index(i, j);
            let h = i64::from(map.heights[k]);
            // Mean rounded to the nearest millimeter.
            let mean = (sum * 2 + 9 * sum.signum()) / 18;
            (
                clamp_height(h + fraction(mean - h, weight, strength_pml)),
                map.terrain[k],
            )
        })
    }

    /// Moves the ground towards `target_mm` by `strength_pml` per mille at
    /// the center (1000: the center reaches the target at once).
    pub fn flatten(
        &mut self,
        brush: Brush,
        target_mm: i32,
        strength_pml: u32,
    ) -> Option<SampleRect> {
        let target = i64::from(clamp_height(i64::from(target_mm)));
        self.paint_samples(brush, |map, i, j, weight, _| {
            let k = map.index(i, j);
            let h = i64::from(map.heights[k]);
            (
                clamp_height(h + fraction(target - h, weight, strength_pml)),
                map.terrain[k],
            )
        })
    }

    /// Paints terrain `id` on every sample within the circle.
    pub fn paint(&mut self, brush: Brush, id: u8) -> Result<Option<SampleRect>, String> {
        if usize::from(id) >= self.terrain_rules.types.len() {
            return Err(format!("unknown terrain id {id}"));
        }
        Ok(self.paint_samples(brush, |map, i, j, _, _| (map.heights[map.index(i, j)], id)))
    }

    /// Digs a river bed: `depth_mm` below `level_mm` (the bank) in the
    /// middle, a parabolic profile up to the bank at the rim; the ground is
    /// only lowered. Deep water in the channel, shallow water beside it, a
    /// sand bank at the rim; existing water is never made shallower or dry.
    /// Drag it along the river with a constant level.
    pub fn river(&mut self, brush: Brush, level_mm: i32, depth_mm: u32) -> Option<SampleRect> {
        let r2 = i64::from(brush.radius_mm).pow(2).max(1);
        let rank = |t: u8| match t {
            DEEP_WATER => 3,
            SHALLOW_WATER => 2,
            SAND => 1,
            _ => 0,
        };
        self.paint_samples(brush, |map, i, j, _, d2| {
            let k = map.index(i, j);
            let below = i64::from(depth_mm) * (r2 - d2) / r2;
            let bed = clamp_height(i64::from(level_mm) - below);
            let h = map.heights[k].min(bed);
            let wet = if below > i64::from(RIVER_DEEP_MM) {
                DEEP_WATER
            } else if below > i64::from(RIVER_BANK_MM) {
                SHALLOW_WATER
            } else {
                SAND
            };
            let t = map.terrain[k];
            (h, if rank(wet) > rank(t) { wet } else { t })
        })
    }

    // --- Objects ------------------------------------------------------

    fn check_kind(&self, kind: &str, variant: &str) -> Result<(), String> {
        self.rules
            .find(kind, variant)
            .map(|_| ())
            .ok_or_else(|| format!("unknown object '{kind}' variant '{variant}'"))
    }

    /// Scatters objects into the circle: `per_hectare` on average, picked
    /// evenly from `kinds` (`(catalog id, variant)`), random headings, at
    /// least `min_spacing_mm` from every other object, never on tracks,
    /// roads or water. Returns how many were placed.
    pub fn scatter(
        &mut self,
        brush: Brush,
        kinds: &[(String, String)],
        per_hectare: u32,
        min_spacing_mm: u32,
    ) -> Result<usize, String> {
        if kinds.is_empty() {
            return Err("no object kinds to scatter".into());
        }
        for (kind, variant) in kinds {
            self.check_kind(kind, variant)?;
        }
        self.dabs += 1;
        let mut rng = Rng::new(self.seed ^ self.dabs.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        // Expected count = pi r² * per_hectare / 10 000 m², in mm².
        let r = i64::from(brush.radius_mm);
        let numerator = i128::from(r * r) * 314_159 * i128::from(per_hectare);
        let denominator: i128 = 100_000 * 1_000_000 * 10_000;
        let mut count = numerator / denominator;
        let rest = u32::try_from((numerator % denominator) * 1_000_000 / denominator).unwrap_or(0);
        if rng.below(1_000_000) < rest {
            count += 1;
        }
        let spacing2 = i64::from(min_spacing_mm).pow(2);
        let reach = r + i64::from(min_spacing_mm);
        // Objects near the brush, plus those placed now.
        let mut near: Vec<(i32, i32)> = self
            .objects
            .iter()
            .filter(|o| {
                (i64::from(o.x_mm) - i64::from(brush.x_mm)).abs() <= reach
                    && (i64::from(o.y_mm) - i64::from(brush.y_mm)).abs() <= reach
            })
            .map(|o| (o.x_mm, o.y_mm))
            .collect();
        let radius = i32::try_from(brush.radius_mm).unwrap_or(i32::MAX);
        let mut placed = Vec::new();
        for _ in 0..count {
            let point = (0..SCATTER_TRIES).find_map(|_| {
                let dx = rng.range_i32(-radius, radius);
                let dy = rng.range_i32(-radius, radius);
                (i64::from(dx).pow(2) + i64::from(dy).pow(2) <= r * r)
                    .then(|| (brush.x_mm.saturating_add(dx), brush.y_mm.saturating_add(dy)))
            });
            let pick = rng.below(u32::try_from(kinds.len()).unwrap_or(u32::MAX)) as usize;
            let heading = rng.below(360) as u16;
            let Some((x, y)) = point else { continue };
            if !self.contains(x, y) || NO_SCATTER.contains(&self.terrain_near(x, y)) {
                continue;
            }
            let free = near.iter().all(|&(ox, oy)| {
                let dx = i64::from(ox) - i64::from(x);
                let dy = i64::from(oy) - i64::from(y);
                dx * dx + dy * dy >= spacing2
            });
            if !free {
                continue;
            }
            near.push((x, y));
            placed.push(PlacedObject {
                kind: kinds[pick].0.clone(),
                variant: kinds[pick].1.clone(),
                x_mm: x,
                y_mm: y,
                heading_deg: heading,
            });
        }
        let n = placed.len();
        if n > 0 {
            self.edit(|map| {
                map.history.touch_objects(&map.objects);
                map.objects.extend(placed);
            });
        }
        Ok(n)
    }

    /// Places one object. Bridges snap to a heading of a multiple of 90°
    /// and with their axis onto a center line of the 4 m path grid (see
    /// `docs/MAPS.md`, "Bridges"). Returns the object as placed.
    pub fn place(&mut self, object: PlacedObject) -> Result<PlacedObject, String> {
        self.check_kind(&object.kind, &object.variant)?;
        let mut o = object;
        o.heading_deg %= 360;
        let is_bridge = self
            .rules
            .find(&o.kind, &o.variant)
            .is_some_and(|k| k.deck.is_some());
        if is_bridge {
            o.heading_deg = ((o.heading_deg + 45) / 90 * 90) % 360;
            if o.heading_deg.is_multiple_of(180) {
                o.x_mm = self.snap_to_path_line(o.x_mm, self.half_x_mm);
            } else {
                o.y_mm = self.snap_to_path_line(o.y_mm, self.half_y_mm);
            }
        }
        if !self.contains(o.x_mm, o.y_mm) {
            return Err("the object lies outside the map".into());
        }
        let placed = o.clone();
        self.edit(|map| {
            map.history.touch_objects(&map.objects);
            map.objects.push(o);
        });
        Ok(placed)
    }

    /// Nearest center line of the path grid: cells start half a sample
    /// outside the first sample (`sim::path::PathGrid`).
    fn snap_to_path_line(&self, v: i32, half: i32) -> i32 {
        let stride = (sim::path::PATH_CELL_MM / self.cell_mm).max(1);
        let cell = i64::from(stride * self.cell_mm);
        let origin = -i64::from(half) - i64::from(self.cell_mm / 2) + cell / 2;
        let k = (i64::from(v) - origin + cell / 2).div_euclid(cell);
        let mut line = origin + k * cell;
        while line > i64::from(half) {
            line -= cell;
        }
        while line < -i64::from(half) {
            line += cell;
        }
        line as i32
    }

    /// Removes the objects whose position lies in the circle (only those of
    /// catalog id `kind` if given). Returns how many.
    pub fn erase(&mut self, brush: Brush, kind: Option<&str>) -> usize {
        let r2 = i64::from(brush.radius_mm).pow(2);
        let hit = |o: &PlacedObject| {
            let dx = i64::from(o.x_mm) - i64::from(brush.x_mm);
            let dy = i64::from(o.y_mm) - i64::from(brush.y_mm);
            dx * dx + dy * dy <= r2 && kind.is_none_or(|k| o.kind == k)
        };
        let n = self.objects.iter().filter(|o| hit(o)).count();
        if n > 0 {
            self.edit(|map| {
                map.history.touch_objects(&map.objects);
                map.objects.retain(|o| !hit(o));
            });
        }
        n
    }
}
