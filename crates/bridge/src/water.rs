//! Water surfaces for drawing (prompt 014, presentation only, floats allowed).
//!
//! The simulation only knows water as terrain types (`shallow_water`,
//! `deep_water`) on a river bed. This module derives what the renderer needs,
//! per map sample:
//!
//! - **level**: the water surface, from the heights of the banks (land
//!   samples next to water; abutments under bridge decks are left out and
//!   steep banks are capped), carried to every water sample from its
//!   nearest bank and smoothed along the water. Land within a few samples of
//!   the water gets the level of its nearest water too (wet band of the
//!   terrain shader, reeds).
//! - **shore distance**: meters from a water sample to the nearest land or
//!   stone in the water (foam, flow speed), and from a land sample to the
//!   nearest water (reeds).
//! - **flow**: a velocity along the banks (perpendicular to the gradient of
//!   the shore distance), turned towards the map's `water_flow_deg`, faster
//!   in the middle and in deep water, smoothed; it bends around stones.
//!
//! [`WaterField::mesh`] turns that into one triangle mesh covering the water
//! plus one ring of land samples (the terrain hides what lies above it).

use sim::Map;

/// Water surface below the bank height (m).
const LEVEL_BELOW_BANK_M: f32 = 0.08;
/// A bank counts at most this far above the bed next to it (m): keeps
/// abutments and steep embankments from lifting the water.
const MAX_BANK_ABOVE_BED_M: f32 = 0.9;
/// Smoothing passes over the water samples (3 x 3 mean each).
const LEVEL_SMOOTHING: usize = 8;
const FLOW_SMOOTHING: usize = 6;
/// Flow speed in the middle of deep water (m/s, presentation).
const FLOW_SPEED_M_S: f32 = 0.9;
/// Shore distance at which the flow reaches full speed (m).
const FLOW_FULL_AT_M: f32 = 6.0;
/// Land samples within this many samples of water get a level.
const LAND_BAND_SAMPLES: f32 = 4.0;

/// Per-sample water data of a map (row-major like the map samples).
pub struct WaterField {
    pub samples: (u32, u32),
    pub cell: f32,
    pub half: (f32, f32),
    pub is_water: Vec<bool>,
    /// Water surface height (m); NaN where unknown (far from water).
    pub level: Vec<f32>,
    /// Water: distance to land or a stone (m). Land: distance to water.
    pub shore: Vec<f32>,
    /// Flow velocity (x, z) in m/s on water samples.
    pub flow: Vec<[f32; 2]>,
    /// Ground height (m) per sample.
    pub ground: Vec<f32>,
}

/// Two-pass chamfer distance (8-neighborhood) from the samples where
/// `source` is `Some(value)`; returns the distance in meters and the value
/// of the nearest source.
fn chamfer(
    sx: usize,
    sy: usize,
    cell: f32,
    source: impl Fn(usize) -> Option<f32>,
) -> (Vec<f32>, Vec<f32>) {
    let n = sx * sy;
    let mut d = vec![f32::INFINITY; n];
    let mut v = vec![f32::NAN; n];
    for k in 0..n {
        if let Some(value) = source(k) {
            d[k] = 0.0;
            v[k] = value;
        }
    }
    let (a, b) = (cell, cell * std::f32::consts::SQRT_2);
    let relax = |d: &mut Vec<f32>, v: &mut Vec<f32>, k: usize, from: usize, w: f32| {
        let c = d[from] + w;
        if c < d[k] {
            d[k] = c;
            v[k] = v[from];
        }
    };
    for j in 0..sy {
        for i in 0..sx {
            let k = j * sx + i;
            if i > 0 {
                relax(&mut d, &mut v, k, k - 1, a);
            }
            if j > 0 {
                relax(&mut d, &mut v, k, k - sx, a);
                if i > 0 {
                    relax(&mut d, &mut v, k, k - sx - 1, b);
                }
                if i + 1 < sx {
                    relax(&mut d, &mut v, k, k - sx + 1, b);
                }
            }
        }
    }
    for j in (0..sy).rev() {
        for i in (0..sx).rev() {
            let k = j * sx + i;
            if i + 1 < sx {
                relax(&mut d, &mut v, k, k + 1, a);
            }
            if j + 1 < sy {
                relax(&mut d, &mut v, k, k + sx, a);
                if i + 1 < sx {
                    relax(&mut d, &mut v, k, k + sx + 1, b);
                }
                if i > 0 {
                    relax(&mut d, &mut v, k, k + sx - 1, b);
                }
            }
        }
    }
    (d, v)
}

/// Neighbors of sample `k` (8-neighborhood) inside the grid.
fn neighbors(sx: usize, sy: usize, k: usize) -> impl Iterator<Item = usize> {
    let (i, j) = ((k % sx) as i64, (k / sx) as i64);
    (-1i64..=1).flat_map(move |dj| {
        (-1i64..=1).filter_map(move |di| {
            let (ni, nj) = (i + di, j + dj);
            ((di, dj) != (0, 0) && ni >= 0 && nj >= 0 && ni < sx as i64 && nj < sy as i64)
                .then(|| nj as usize * sx + ni as usize)
        })
    })
}

/// Mean over the 3 x 3 neighborhood of every listed sample, counting only
/// samples where `keep` holds; `passes` times.
fn smooth<T: Copy + Default + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>>(
    sx: usize,
    sy: usize,
    list: &[usize],
    keep: &[bool],
    values: &mut [T],
    passes: usize,
) {
    let mut next: Vec<T> = values.to_vec();
    for _ in 0..passes {
        for &k in list {
            let mut sum = values[k];
            let mut count = 1.0;
            for n in neighbors(sx, sy, k) {
                if keep[n] {
                    sum = sum + values[n];
                    count += 1.0;
                }
            }
            next[k] = sum * (1.0 / count);
        }
        for &k in list {
            values[k] = next[k];
        }
    }
}

#[derive(Clone, Copy, Default)]
struct V2(f32, f32);

impl std::ops::Add for V2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self(self.0 + o.0, self.1 + o.1)
    }
}

impl std::ops::Mul<f32> for V2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self(self.0 * s, self.1 * s)
    }
}

impl WaterField {
    pub fn new(map: &Map) -> Self {
        let (sx, sy) = map.samples();
        let (nx, ny) = (sx as usize, sy as usize);
        let n = nx * ny;
        let cell = map.cell_size_mm() as f32 / 1000.0;
        let size = map.size();
        let half = (size.x.to_num::<f32>() / 2.0, size.y.to_num::<f32>() / 2.0);
        let rules = sim::TerrainRules::builtin().ok();
        let water_ids: Vec<u8> = rules
            .iter()
            .flat_map(|r| r.types.iter().enumerate())
            .filter(|(_, t)| t.name.contains("water"))
            .map(|(i, _)| i as u8)
            .collect();
        let ids = map.terrain_ids();
        let is_water: Vec<bool> = ids.iter().map(|id| water_ids.contains(id)).collect();
        let ground: Vec<f32> = map
            .heights_mm()
            .iter()
            .map(|&h| h as f32 / 1000.0)
            .collect();
        let water_list: Vec<usize> = (0..n).filter(|&k| is_water[k]).collect();
        if water_list.is_empty() {
            return Self {
                samples: (sx, sy),
                cell,
                half,
                is_water,
                level: vec![f32::NAN; n],
                shore: vec![f32::INFINITY; n],
                flow: vec![[0.0; 2]; n],
                ground,
            };
        }

        // Stones in the water (map objects): obstacles for shore distance
        // and flow.
        let mut stone = vec![false; n];
        let sample_of = |x: f32, z: f32| {
            let i = ((x + half.0) / cell).round();
            let j = ((z + half.1) / cell).round();
            (i >= 0.0 && j >= 0.0 && i < sx as f32 && j < sy as f32)
                .then(|| j as usize * nx + i as usize)
        };
        for o in map.objects() {
            let kind = &map.object_kinds()[usize::from(o.kind)];
            if kind.id == "env_rock"
                && let Some(k) = sample_of(o.position.x.to_num(), o.position.y.to_num())
                && is_water[k]
            {
                stone[k] = true;
            }
        }

        // Level: banks (land next to water, not under a deck), capped above
        // the bed next to them, carried to the water by nearest bank.
        let under_deck = |k: usize| map.sample_under_deck((k % nx) as u32, (k / nx) as u32);
        let bank = |k: usize| -> Option<f32> {
            if is_water[k] || under_deck(k) {
                return None;
            }
            let bed = neighbors(nx, ny, k)
                .filter(|&m| is_water[m])
                .map(|m| ground[m])
                .fold(f32::INFINITY, f32::min);
            bed.is_finite()
                .then(|| ground[k].min(bed + MAX_BANK_ABOVE_BED_M))
        };
        let (_, bank_level) = chamfer(nx, ny, cell, bank);
        let mut level = vec![f32::NAN; n];
        for &k in &water_list {
            level[k] = bank_level[k] - LEVEL_BELOW_BANK_M;
        }
        smooth(nx, ny, &water_list, &is_water, &mut level, LEVEL_SMOOTHING);

        // Land near the water: level of the nearest water; distance to it.
        let (land_distance, land_level) = chamfer(nx, ny, cell, |k| is_water[k].then(|| level[k]));
        // Water: distance to land or a stone.
        let (water_distance, _) =
            chamfer(nx, ny, cell, |k| (!is_water[k] || stone[k]).then_some(0.0));
        let mut shore = vec![0.0; n];
        for k in 0..n {
            if is_water[k] {
                shore[k] = if stone[k] { 0.0 } else { water_distance[k] };
            } else {
                shore[k] = land_distance[k];
                if land_distance[k] <= LAND_BAND_SAMPLES * cell * 1.5 {
                    level[k] = land_level[k];
                }
            }
        }

        // Flow along the banks, turned towards the map's direction.
        let deg = f32::from(map.water_flow_deg()).to_radians();
        let main = V2(deg.sin(), -deg.cos());
        let at = |i: i64, j: i64| -> f32 {
            let (i, j) = (i.clamp(0, nx as i64 - 1), j.clamp(0, ny as i64 - 1));
            let k = j as usize * nx + i as usize;
            if is_water[k] { shore[k] } else { 0.0 }
        };
        let mut flow = vec![V2::default(); n];
        for &k in &water_list {
            let (i, j) = ((k % nx) as i64, (k / nx) as i64);
            let gx = at(i + 1, j) - at(i - 1, j);
            let gz = at(i, j + 1) - at(i, j - 1);
            let len = (gx * gx + gz * gz).sqrt();
            if len < 1e-4 {
                continue;
            }
            let mut t = V2(-gz / len, gx / len);
            if t.0 * main.0 + t.1 * main.1 < 0.0 {
                t = t * -1.0;
            }
            flow[k] = t;
        }
        smooth(nx, ny, &water_list, &is_water, &mut flow, FLOW_SMOOTHING);
        let mut out = vec![[0.0; 2]; n];
        for &k in &water_list {
            let V2(x, z) = flow[k];
            let len = (x * x + z * z).sqrt();
            if len < 1e-4 || stone[k] {
                continue;
            }
            let depth = (level[k] - ground[k]).max(0.0);
            let edge = (shore[k] / FLOW_FULL_AT_M).clamp(0.0, 1.0);
            let speed = FLOW_SPEED_M_S * edge * (0.5 + 0.5 * (depth / 1.5).min(1.0));
            out[k] = [x / len * speed, z / len * speed];
        }

        Self {
            samples: (sx, sy),
            cell,
            half,
            is_water,
            level,
            shore,
            flow: out,
            ground,
        }
    }

    /// Sample index nearest to Godot `(x, z)`.
    pub fn sample(&self, x: f32, z: f32) -> Option<usize> {
        let (sx, sy) = self.samples;
        let i = ((x + self.half.0) / self.cell).round();
        let j = ((z + self.half.1) / self.cell).round();
        (i >= 0.0 && j >= 0.0 && i < sx as f32 && j < sy as f32)
            .then(|| j as usize * sx as usize + i as usize)
    }

    /// Signed shore distance at a sample: meters to the water on land,
    /// negative meters to the land in the water.
    pub fn signed_shore(&self, k: usize) -> f32 {
        if self.is_water[k] {
            -self.shore[k]
        } else {
            self.shore[k]
        }
    }

    /// The water surface as a mesh: one vertex per sample that is water or
    /// next to water, two triangles per map cell whose corners all are and
    /// that touches water. Vertex data: position (Godot x, y, z), `uv` =
    /// flow (x, z) in m/s, `uv2` = (shore distance in m, depth in m:
    /// negative above the ground).
    pub fn mesh(&self) -> Mesh {
        let (sx, sy) = self.samples;
        let (nx, ny) = (sx as usize, sy as usize);
        let mut index = vec![u32::MAX; nx * ny];
        let mut mesh = Mesh::default();
        let near_water =
            |k: usize| self.is_water[k] || neighbors(nx, ny, k).any(|m| self.is_water[m]);
        for k in 0..nx * ny {
            if !near_water(k) || self.level[k].is_nan() {
                continue;
            }
            index[k] = mesh.vertices.len() as u32;
            let x = -self.half.0 + (k % nx) as f32 * self.cell;
            let z = -self.half.1 + (k / nx) as f32 * self.cell;
            mesh.vertices.push([x, self.level[k], z]);
            mesh.uv.push(self.flow[k]);
            let shore = if self.is_water[k] { self.shore[k] } else { 0.0 };
            mesh.uv2.push([shore, self.level[k] - self.ground[k]]);
        }
        for j in 0..ny.saturating_sub(1) {
            for i in 0..nx.saturating_sub(1) {
                let k = j * nx + i;
                let corners = [k, k + 1, k + nx, k + nx + 1];
                if corners.iter().any(|&c| index[c] == u32::MAX)
                    || !corners.iter().any(|&c| self.is_water[c])
                {
                    continue;
                }
                let [a, b, c, d] = corners.map(|c| index[c]);
                // Counter-clockwise seen from above (+y) in Godot.
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        mesh
    }
}

/// Buffers of [`WaterField::mesh`].
#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub uv: Vec<[f32; 2]>,
    pub uv2: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::map::MapLayout;

    /// 60 m x 40 m, banks at 0 m, a river along z with its bed at -1 m for
    /// |x| <= 6 m.
    fn river() -> Map {
        let (sx, sy) = (31u32, 21u32);
        let mut heights = Vec::new();
        let mut terrain = Vec::new();
        for _j in 0..sy {
            for i in 0..sx {
                let x = 2 * i as i32 - 30;
                let water = x.abs() <= 6;
                heights.push(if water { -1000 } else { 0 });
                terrain.push(if water { 7 } else { 0 });
            }
        }
        let layout = MapLayout {
            name: "w".into(),
            samples_x: sx,
            samples_y: sy,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        Map::from_rasters(layout, heights, terrain).unwrap()
    }

    #[test]
    fn level_flow_and_mesh() {
        let field = WaterField::new(&river());
        let k = field.sample(0.0, 0.0).unwrap();
        assert!(field.is_water[k]);
        // Banks at 0 m but capped at bed + 0.9: level 0.82 m below them.
        assert!((field.level[k] - (-1.0 + MAX_BANK_ABOVE_BED_M - LEVEL_BELOW_BANK_M)).abs() < 1e-4);
        assert!((field.shore[k] - 8.0).abs() < 1e-4, "{}", field.shore[k]);
        // Default direction: south (+z), along the river.
        let [fx, fz] = field.flow[k];
        assert!(fz > 0.5 && fx.abs() < 1e-3, "{fx} {fz}");
        let near_bank = field.sample(-6.0, 0.0).unwrap();
        assert!(field.flow[near_bank][1] < fz);
        let mesh = field.mesh();
        // 7 water columns + 2 ring columns, 21 rows; 8 x 20 cells.
        assert_eq!(mesh.vertices.len(), 9 * 21);
        assert_eq!(mesh.indices.len(), 8 * 20 * 6);
        let land = field.sample(-20.0, 0.0).unwrap();
        assert!(field.level[land].is_nan());
        assert!((field.signed_shore(land) - 14.0).abs() < 1e-4);
    }
}
