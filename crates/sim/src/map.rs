//! Maps: height field and terrain raster, owned by the simulation.
//!
//! A map is a regular grid of samples, centered on the origin. Sample
//! `(i, j)` lies at `(-width/2 + i * cell, -depth/2 + j * cell)`; `i` grows
//! east (+x), `j` grows south (+y), exactly like image columns and rows.
//! Heights are integer millimeters, interpolated bilinearly in [`Fixed`].
//! Every sample also carries a terrain type (an index into
//! [`crate::TerrainRules::types`]) that covers the square of one cell around
//! it.
//!
//! The simulation never touches the file system: callers read the files of
//! `data/maps/<name>/` and hand the bytes to [`Map::load`]. File format and
//! reasons: `docs/MAPS.md`.

use crate::bridges::Bridge;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::{MapObject, ObjectKind, ObjectList, ObjectRules};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Cursor;

/// Largest supported number of samples per axis.
pub const MAX_SAMPLES: u32 = 8193;
/// Supported cell sizes in millimeters.
pub const CELL_SIZE_RANGE_MM: std::ops::RangeInclusive<u32> = 100..=100_000;
/// Every height must lie within ±this many millimeters (10 km). Keeps the
/// fixed-point interpolation in millimeters far from overflow.
pub const HEIGHT_LIMIT_MM: i64 = 10_000_000;

/// Identifies the map a [`crate::SimConfig`] is played on. The hash covers
/// the complete content, so peers (and replays) cannot silently use
/// different versions of a map with the same name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapRef {
    pub name: String,
    pub hash: u64,
}

/// A start position from `map.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartPosition {
    pub player: u8,
    pub position: Vec2,
}

/// A named point from `map.toml` (for tools and tests, e.g. `hill_top`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub name: String,
    pub position: Vec2,
}

/// Climate look of a map from `map.toml` (`biome = "forest"`). Presentation
/// only: the simulation validates and stores it for the bridge, but no rule
/// reads it, and it is not part of the map hash or the state hash.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Biome {
    /// Temperate: meadows, fields, deciduous and pine forest.
    #[default]
    Forest,
    Desert,
    Winter,
}

impl Biome {
    pub const ALL: [Self; 3] = [Self::Forest, Self::Desert, Self::Winter];

    /// The name used in `map.toml`, palettes and presets.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Forest => "forest",
            Self::Desert => "desert",
            Self::Winter => "winter",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MapError(pub String);

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid map: {}", self.0)
    }
}

impl std::error::Error for MapError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PointEntry {
    position_m: [i32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartEntry {
    player: u8,
    #[serde(flatten)]
    point: PointEntry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkerEntry {
    name: String,
    #[serde(flatten)]
    point: PointEntry,
}

/// Contents of `map.toml`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapFile {
    name: String,
    /// How the map was made (informational).
    #[serde(default)]
    generator: Option<String>,
    cell_size_mm: u32,
    /// `[x, y]` = width and height of both PNGs.
    samples: [u32; 2],
    /// Height of PNG value 0.
    height_base_mm: i32,
    /// Millimeters per PNG step.
    height_step_mm: u32,
    /// Presentation only, see [`Biome`].
    #[serde(default)]
    biome: Biome,
    /// Presentation only: compass direction (degrees, 0 = north, 90 =
    /// east) the rivers flow towards; see [`Map::water_flow_deg`].
    #[serde(default)]
    water_flow_deg: Option<u16>,
    #[serde(default)]
    start: Vec<StartEntry>,
    #[serde(default)]
    marker: Vec<MarkerEntry>,
}

/// Default of [`Map::water_flow_deg`]: south.
pub const DEFAULT_WATER_FLOW_DEG: u16 = 180;

/// A loaded map.
///
/// `Clone` is for copy-on-write changes during a match (a destroyed
/// bridge, [`crate::Sim::set_bridge_intact`]); everybody else shares it
/// through an `Arc`.
#[derive(Clone)]
pub struct Map {
    name: String,
    samples_x: u32,
    samples_y: u32,
    cell_size_mm: u32,
    /// Half of the width and depth in meters (whole meters, so exact).
    half: Vec2,
    /// Grid cells per meter: `1000 / cell_size_mm`.
    cells_per_m: Fixed,
    /// Row-major (`j * samples_x + i`).
    heights_mm: Vec<i32>,
    /// Row-major, same layout as `heights_mm`.
    terrain: Vec<u8>,
    starts: Vec<StartPosition>,
    markers: Vec<Marker>,
    /// Not part of `hash`.
    biome: Biome,
    /// Not part of `hash` (presentation only).
    water_flow_deg: u16,
    /// Placed objects (`objects.csv`), empty until [`Map::add_objects`].
    objects: ObjectList,
    /// Derived from the objects whose kind has a deck.
    bridges: Vec<Bridge>,
    hash: u64,
}

impl fmt::Debug for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map")
            .field("name", &self.name)
            .field("samples", &(self.samples_x, self.samples_y))
            .field("cell_size_mm", &self.cell_size_mm)
            .field("hash", &format_args!("{:#018x}", self.hash))
            .finish_non_exhaustive()
    }
}

/// Grid layout and metadata of a map, without the rasters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapLayout {
    pub name: String,
    pub samples_x: u32,
    pub samples_y: u32,
    pub cell_size_mm: u32,
    pub starts: Vec<StartPosition>,
    pub markers: Vec<Marker>,
}

impl Map {
    /// Decodes a map from the contents of `map.toml`, `height.png` (16-bit
    /// grayscale) and `terrain.png` (8-bit grayscale or 8-bit indexed; the
    /// raw value or palette index is the terrain id).
    ///
    /// Decoding is integer-only: PNG is lossless, so every correct decoder
    /// yields the same samples, and no color or gamma transformation is
    /// applied.
    pub fn load(map_toml: &str, height_png: &[u8], terrain_png: &[u8]) -> Result<Self, MapError> {
        let file: MapFile = toml::from_str(map_toml).map_err(|e| MapError(e.to_string()))?;
        let _ = file.generator;
        let [samples_x, samples_y] = file.samples;

        let (width, height, raw) = decode_png(height_png, "height.png", png::BitDepth::Sixteen)?;
        if (width, height) != (samples_x, samples_y) {
            return Err(MapError(format!(
                "height.png is {width} x {height}, map.toml says {samples_x} x {samples_y}"
            )));
        }
        let base = i64::from(file.height_base_mm);
        let step = i64::from(file.height_step_mm);
        let top = base + step * i64::from(u16::MAX);
        if base.abs() > HEIGHT_LIMIT_MM || top.abs() > HEIGHT_LIMIT_MM {
            return Err(MapError(format!(
                "heights must stay within ±{HEIGHT_LIMIT_MM} mm (base {base}, top {top})"
            )));
        }
        let heights_mm = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&b| {
                let value = i64::from(u16::from_be_bytes(b));
                // Within ±HEIGHT_LIMIT_MM, checked above.
                i32::try_from(base + value * step).unwrap_or(0)
            })
            .collect();

        let (width, height, terrain) =
            decode_png(terrain_png, "terrain.png", png::BitDepth::Eight)?;
        if (width, height) != (samples_x, samples_y) {
            return Err(MapError(format!(
                "terrain.png is {width} x {height}, map.toml says {samples_x} x {samples_y}"
            )));
        }

        let layout = MapLayout {
            name: file.name,
            samples_x,
            samples_y,
            cell_size_mm: file.cell_size_mm,
            starts: file
                .start
                .into_iter()
                .map(|s| StartPosition {
                    player: s.player,
                    position: point(&s.point),
                })
                .collect(),
            markers: file
                .marker
                .into_iter()
                .map(|m| Marker {
                    name: m.name,
                    position: point(&m.point),
                })
                .collect(),
        };
        let mut map = Self::from_rasters(layout, heights_mm, terrain)?;
        map.biome = file.biome;
        if let Some(deg) = file.water_flow_deg {
            if deg >= 360 {
                return Err(MapError(format!(
                    "water_flow_deg must be 0..360, got {deg}"
                )));
            }
            map.water_flow_deg = deg;
        }
        Ok(map)
    }

    /// Builds a map from already decoded rasters (row-major, one value per
    /// sample). Used by [`Map::load`], tests and tools.
    pub fn from_rasters(
        layout: MapLayout,
        heights_mm: Vec<i32>,
        terrain: Vec<u8>,
    ) -> Result<Self, MapError> {
        let MapLayout {
            name,
            samples_x,
            samples_y,
            cell_size_mm,
            starts,
            markers,
        } = layout;
        if !(2..=MAX_SAMPLES).contains(&samples_x) || !(2..=MAX_SAMPLES).contains(&samples_y) {
            return Err(MapError(format!(
                "samples must be 2..={MAX_SAMPLES} per axis, got {samples_x} x {samples_y}"
            )));
        }
        if !CELL_SIZE_RANGE_MM.contains(&cell_size_mm) {
            return Err(MapError(format!(
                "cell_size_mm must be in {CELL_SIZE_RANGE_MM:?}, got {cell_size_mm}"
            )));
        }
        let extent_mm = |samples: u32| u64::from(samples - 1) * u64::from(cell_size_mm);
        let (width_mm, depth_mm) = (extent_mm(samples_x), extent_mm(samples_y));
        if width_mm % 2000 != 0 || depth_mm % 2000 != 0 {
            return Err(MapError(format!(
                "width and depth must be even whole meters, got {width_mm} x {depth_mm} mm"
            )));
        }
        let count = usize::try_from(u64::from(samples_x) * u64::from(samples_y))
            .map_err(|_| MapError("map too large".into()))?;
        if heights_mm.len() != count || terrain.len() != count {
            return Err(MapError(format!(
                "expected {count} samples, got {} heights and {} terrain values",
                heights_mm.len(),
                terrain.len()
            )));
        }
        if heights_mm
            .iter()
            .any(|&h| i64::from(h).abs() > HEIGHT_LIMIT_MM)
        {
            return Err(MapError(format!(
                "heights must stay within ±{HEIGHT_LIMIT_MM} mm"
            )));
        }
        let half_m = |extent_mm: u64| {
            // At most MAX_SAMPLES * 100 m / 2, far below i32::MAX.
            Fixed::from_num(i32::try_from(extent_mm / 2000).unwrap_or(i32::MAX))
        };
        let half = Vec2::new(half_m(width_mm), half_m(depth_mm));
        for (what, position) in starts
            .iter()
            .map(|s| ("start position", s.position))
            .chain(markers.iter().map(|m| ("marker", m.position)))
        {
            if position.x.abs() > half.x || position.y.abs() > half.y {
                return Err(MapError(format!(
                    "{what} ({}, {}) lies outside the map",
                    position.x, position.y
                )));
            }
        }

        let mut map = Self {
            name,
            samples_x,
            samples_y,
            cell_size_mm,
            half,
            cells_per_m: Fixed::from_num(1000) / Fixed::from_num(cell_size_mm),
            heights_mm,
            terrain,
            starts,
            markers,
            objects: ObjectList::default(),
            bridges: Vec::new(),
            biome: Biome::default(),
            water_flow_deg: DEFAULT_WATER_FLOW_DEG,
            hash: 0,
        };
        map.hash = map.raster_hash();
        Ok(map)
    }

    /// Adds the objects of `objects.csv` (format: `docs/MAPS.md`), resolved
    /// against the built-in rules (`data/objects.toml`). Replaces earlier
    /// objects. The map hash then covers the objects and their rules.
    pub fn add_objects(&mut self, csv: &str) -> Result<(), MapError> {
        let rules = ObjectRules::builtin().map_err(|e| MapError(e.to_string()))?;
        self.add_objects_with(csv, &rules)
    }

    /// [`Map::add_objects`] with explicit rules (tests, tools).
    pub fn add_objects_with(&mut self, csv: &str, rules: &ObjectRules) -> Result<(), MapError> {
        let list = ObjectList::parse(csv, rules, self.half).map_err(|e| MapError(e.to_string()))?;
        self.set_objects(list);
        self.check_bridges()
    }

    /// Every bridge must lie on the map, and the path cells along its axis
    /// (but the last half cell at each end) must all have their center on
    /// the deck, so the path grid opens a
    /// continuous lane (in practice: the axis runs along the 4 m path grid,
    /// close to the cell centers; see `docs/MAPS.md`).
    pub fn check_bridges(&self) -> Result<(), MapError> {
        let stride = (crate::path::PATH_CELL_MM / self.cell_size_mm).max(1);
        let cell = Fixed::from_num(stride * self.cell_size_mm) / Fixed::from_num(1000);
        let half_sample = Fixed::from_num(self.cell_size_mm) / Fixed::from_num(2000);
        let origin = Vec2::new(-self.half.x - half_sample, -self.half.y - half_sample);
        let center_of = |v: Fixed, o: Fixed| {
            let k = ((v - o) / cell).floor();
            o + k * cell + cell / 2
        };
        for b in &self.bridges {
            let o = &self.objects.objects[b.object as usize];
            let kind = &self.objects.kinds[usize::from(o.kind)];
            let err = |what: &str| {
                MapError(format!(
                    "bridge {} {} at ({} m, {} m): {what}",
                    kind.id, kind.variant, o.position.x, o.position.y
                ))
            };
            let (start, end) = b.ends();
            if !self.contains(start) || !self.contains(end) {
                return Err(err("deck reaches beyond the map"));
            }
            // Points every quarter cell along the axis, except the last half
            // cell at both ends (those cells may lie on the banks).
            let inner = (b.half_length - cell / 2).max(Fixed::ZERO);
            let (a, z) = (
                b.center - b.forward.scale(inner),
                b.center + b.forward.scale(inner),
            );
            let steps = (inner * 8 / cell).ceil().to_num::<i64>().max(1);
            for k in 0..=steps {
                let p = a + (z - a).scale(Fixed::from_num(k) / Fixed::from_num(steps));
                let c = Vec2::new(center_of(p.x, origin.x), center_of(p.y, origin.y));
                if !b.covers(c) {
                    return Err(err(&format!(
                        "the path cell at ({}, {}) on its axis has its center off the deck; \
                         align the axis with the 4 m path grid",
                        c.x, c.y
                    )));
                }
            }
        }
        Ok(())
    }

    /// Sets already resolved objects (tests, tools).
    pub fn set_objects(&mut self, list: ObjectList) {
        let base = self.raster_hash();
        self.objects = list;
        self.bridges = (0u32..)
            .zip(&self.objects.objects)
            .filter_map(|(index, o)| {
                let kind = &self.objects.kinds[usize::from(o.kind)];
                let deck = kind.deck?;
                Bridge::new(index, o, kind.shape, deck, |p| self.ground_height_at(p))
            })
            .collect();
        self.hash = if self.objects.objects.is_empty() {
            base
        } else {
            let mut h = StableHasher::new();
            h.write_u64(base);
            self.objects.hash_into(&mut h);
            h.finish()
        };
    }

    fn raster_hash(&self) -> u64 {
        let mut h = StableHasher::new();
        h.write_u32(self.samples_x);
        h.write_u32(self.samples_y);
        h.write_u32(self.cell_size_mm);
        for &height in &self.heights_mm {
            h.write_bytes(&height.to_le_bytes());
        }
        h.write_bytes(&self.terrain);
        h.finish()
    }

    /// The kinds used by [`Map::objects`] (index = `MapObject::kind`).
    pub fn object_kinds(&self) -> &[ObjectKind] {
        &self.objects.kinds
    }

    /// Placed objects in file order.
    pub fn objects(&self) -> &[MapObject] {
        &self.objects.objects
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Climate look for the presentation (not part of the hash).
    pub const fn biome(&self) -> Biome {
        self.biome
    }

    /// Compass direction (degrees, 0 = north, 90 = east) rivers flow
    /// towards, for drawing water (`map.toml`: `water_flow_deg`, default
    /// south). Presentation only, not part of the hash.
    pub const fn water_flow_deg(&self) -> u16 {
        self.water_flow_deg
    }

    /// Bridges of the map, in object order.
    pub fn bridges(&self) -> &[Bridge] {
        &self.bridges
    }

    /// The intact bridge whose deck covers `point` (the first in object
    /// order if decks overlap).
    pub fn deck_at(&self, point: Vec2) -> Option<&Bridge> {
        self.bridges.iter().find(|b| b.intact && b.covers(point))
    }

    /// Destroys a bridge (no deck any more) or restores it. Returns whether
    /// anything changed. Use through [`crate::Sim::set_bridge_intact`],
    /// which also updates the path grid.
    pub fn set_bridge_intact(&mut self, bridge: usize, intact: bool) -> bool {
        match self.bridges.get_mut(bridge) {
            Some(b) if b.intact != intact => {
                b.intact = intact;
                true
            }
            _ => false,
        }
    }

    /// Feeds the state of the bridges (intact or not) into a hash. Writes
    /// nothing for maps without bridges.
    pub(crate) fn hash_bridge_state(&self, h: &mut StableHasher) {
        Bridge::hash_state(&self.bridges, h);
    }

    /// Hash of the complete content (grid, heights, terrain, objects with
    /// their resolved rules), independent of the name and of `map.toml`
    /// metadata.
    pub const fn hash(&self) -> u64 {
        self.hash
    }

    /// Reference for [`crate::SimConfig::map`].
    pub fn reference(&self) -> MapRef {
        MapRef {
            name: self.name.clone(),
            hash: self.hash,
        }
    }

    /// Number of samples per axis (`x`, `y`).
    pub const fn samples(&self) -> (u32, u32) {
        (self.samples_x, self.samples_y)
    }

    pub const fn cell_size_mm(&self) -> u32 {
        self.cell_size_mm
    }

    /// Width (x) and depth (y) in meters. The map spans `-size/2..=size/2`.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.half.x * 2, self.half.y * 2)
    }

    pub fn starts(&self) -> &[StartPosition] {
        &self.starts
    }

    pub fn markers(&self) -> &[Marker] {
        &self.markers
    }

    pub fn marker(&self, name: &str) -> Option<Vec2> {
        self.markers
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.position)
    }

    /// Raw heights in millimeters, row-major.
    pub fn heights_mm(&self) -> &[i32] {
        &self.heights_mm
    }

    /// Raw terrain ids, row-major.
    pub fn terrain_ids(&self) -> &[u8] {
        &self.terrain
    }

    /// Whether `point` lies on the map (edges included).
    pub fn contains(&self, point: Vec2) -> bool {
        point.x.abs() <= self.half.x && point.y.abs() <= self.half.y
    }

    /// Height in meters of the surface units stand on at `point`: the
    /// deck of an intact bridge (never below the ground), else the ground
    /// ([`Map::ground_height_at`]).
    pub fn height_at(&self, point: Vec2) -> Fixed {
        let ground = self.ground_height_at(point);
        match self.deck_at(point) {
            Some(bridge) => bridge.height_at(point).max(ground),
            None => ground,
        }
    }

    /// Terrain id of the surface at `point`: the deck terrain of an intact
    /// bridge, else the ground ([`Map::ground_terrain_at`]).
    pub fn terrain_at(&self, point: Vec2) -> u8 {
        match self.deck_at(point) {
            Some(bridge) => bridge.terrain,
            None => self.ground_terrain_at(point),
        }
    }

    /// Whether sample `(i, j)` lies under the deck of an intact bridge.
    pub fn sample_under_deck(&self, i: u32, j: u32) -> bool {
        if self.bridges.is_empty() {
            return false;
        }
        let cell = Fixed::from_num(self.cell_size_mm) / Fixed::from_num(1000);
        let p = Vec2::new(
            Fixed::from_num(i) * cell - self.half.x,
            Fixed::from_num(j) * cell - self.half.y,
        );
        self.deck_at(p).is_some()
    }

    /// Ground height in meters at `point` (bridges ignored), interpolated
    /// bilinearly between the four surrounding samples. Points outside the
    /// map use the nearest edge.
    ///
    /// The interpolation runs in millimeters and is divided by 1000 once at
    /// the end, so heights at samples and at simple fractions of a cell are
    /// exact whenever the result is representable.
    pub fn ground_height_at(&self, point: Vec2) -> Fixed {
        let (i, fx) = self.cell_of(point.x, self.half.x, self.samples_x);
        let (j, fy) = self.cell_of(point.y, self.half.y, self.samples_y);
        let sample =
            |i: usize, j: usize| Fixed::from_num(self.heights_mm[j * self.samples_x as usize + i]);
        let h00 = sample(i, j);
        let h10 = sample(i + 1, j);
        let h01 = sample(i, j + 1);
        let h11 = sample(i + 1, j + 1);
        let top = h00 + (h10 - h00) * fx;
        let bottom = h01 + (h11 - h01) * fx;
        (top + (bottom - top) * fy) / Fixed::from_num(1000)
    }

    /// Ground terrain id at `point` (bridges ignored): the nearest sample
    /// (points outside the map use the nearest edge sample).
    pub fn ground_terrain_at(&self, point: Vec2) -> u8 {
        let i = self.nearest(point.x, self.half.x, self.samples_x);
        let j = self.nearest(point.y, self.half.y, self.samples_y);
        self.terrain[j * self.samples_x as usize + i]
    }

    /// Height of a sample in millimeters (indices clamped to the grid).
    pub fn sample_height_mm(&self, i: u32, j: u32) -> i32 {
        let i = i.min(self.samples_x - 1) as usize;
        let j = j.min(self.samples_y - 1) as usize;
        self.heights_mm[j * self.samples_x as usize + i]
    }

    /// Terrain id of a sample (indices clamped to the grid).
    pub fn sample_terrain(&self, i: u32, j: u32) -> u8 {
        let i = i.min(self.samples_x - 1) as usize;
        let j = j.min(self.samples_y - 1) as usize;
        self.terrain[j * self.samples_x as usize + i]
    }

    /// Grid coordinate of `value` on an axis, clamped to `0..=samples-1`.
    fn grid(&self, value: Fixed, half: Fixed, samples: u32) -> Fixed {
        let g = value.saturating_add(half).saturating_mul(self.cells_per_m);
        g.clamp(Fixed::ZERO, Fixed::from_num(samples - 1))
    }

    /// Index of the cell (its lower sample) and the fraction within it.
    fn cell_of(&self, value: Fixed, half: Fixed, samples: u32) -> (usize, Fixed) {
        let g = self.grid(value, half, samples);
        // g >= 0, so to_num truncates towards the floor.
        let index = g.to_num::<u32>().min(samples - 2);
        (index as usize, g - Fixed::from_num(index))
    }

    fn nearest(&self, value: Fixed, half: Fixed, samples: u32) -> usize {
        let g = self.grid(value, half, samples) + Fixed::ONE / 2;
        g.to_num::<u32>().min(samples - 1) as usize
    }
}

fn point(entry: &PointEntry) -> Vec2 {
    Vec2::from_meters(entry.position_m[0], entry.position_m[1])
}

/// Decodes a single-channel PNG with the given bit depth into its raw sample
/// bytes (16-bit samples stay big endian). No transformation is applied.
fn decode_png(
    bytes: &[u8],
    what: &str,
    depth: png::BitDepth,
) -> Result<(u32, u32, Vec<u8>), MapError> {
    let error = |e: &dyn fmt::Display| MapError(format!("{what}: {e}"));
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(|e| error(&e))?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    let color_ok = match depth {
        png::BitDepth::Eight => matches!(
            info.color_type,
            png::ColorType::Grayscale | png::ColorType::Indexed
        ),
        _ => info.color_type == png::ColorType::Grayscale,
    };
    if !color_ok || info.bit_depth != depth {
        return Err(MapError(format!(
            "{what} must be {}-bit grayscale{}, got {:?} {:?}",
            depth as u8,
            if depth == png::BitDepth::Eight {
                " or indexed"
            } else {
                ""
            },
            info.color_type,
            info.bit_depth
        )));
    }
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| MapError(format!("{what}: image too large")))?;
    let mut buffer = vec![0; size];
    let frame = reader.next_frame(&mut buffer).map_err(|e| error(&e))?;
    let row = frame.line_size;
    let used = row * height as usize;
    buffer.truncate(used);
    let sample_bytes = if depth == png::BitDepth::Sixteen {
        2
    } else {
        1
    };
    if row != width as usize * sample_bytes {
        return Err(MapError(format!("{what}: unexpected row size {row}")));
    }
    Ok((width, height, buffer))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3 x 3 samples, 2 m cells: the map spans -2..=2 m on both axes.
    fn small() -> Map {
        #[rustfmt::skip]
        let heights = vec![
            0,    1000, 2000,
            2000, 4000, 4000,
            -800, 0,    1000,
        ];
        let layout = MapLayout {
            name: "small".into(),
            samples_x: 3,
            samples_y: 3,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        Map::from_rasters(layout, heights, vec![0, 1, 2, 3, 4, 5, 6, 7, 0]).unwrap()
    }

    fn at(x: Fixed, y: Fixed) -> Vec2 {
        Vec2::new(x, y)
    }

    fn m(meters: i32) -> Fixed {
        Fixed::from_num(meters)
    }

    #[test]
    fn heights_at_samples_are_exact() {
        let map = small();
        assert_eq!(map.height_at(at(m(-2), m(-2))), Fixed::ZERO);
        assert_eq!(map.height_at(at(m(0), m(0))), m(4));
        assert_eq!(map.height_at(at(m(2), m(-2))), m(2));
        // -800 mm: compare with the same single division the map performs.
        assert_eq!(
            map.height_at(at(m(-2), m(2))),
            Fixed::from_num(-800) / Fixed::from_num(1000)
        );
    }

    #[test]
    fn bilinear_heights_between_samples_are_exact() {
        let map = small();
        // Cell (0, 0): h00 = 0, h10 = 1000, h01 = 2000, h11 = 4000 mm.
        // x = -1.5 m -> fx = 0.25, y = -1 m -> fy = 0.5:
        // top = 250, bottom = 2500, height = 1375 mm = 1.375 m.
        let x = Fixed::from_num(-3) / 2;
        assert_eq!(map.height_at(at(x, m(-1))), Fixed::from_bits(0x1_6000_0000));
        assert_eq!(map.height_at(at(x, m(-1))), Fixed::from_num(11) / 8);
        // Center of cell (1, 1): (4000 + 4000 + 0 + 1000) / 4 = 2250 mm.
        assert_eq!(map.height_at(at(m(1), m(1))), Fixed::from_num(9) / 4);
        // Edge between two samples: (2000 + 4000) / 2 = 3 m.
        assert_eq!(map.height_at(at(m(-1), m(0))), m(3));
    }

    #[test]
    fn outside_points_use_the_nearest_edge() {
        let map = small();
        assert_eq!(map.height_at(at(m(-50), m(0))), m(2));
        assert_eq!(map.height_at(at(Fixed::MAX, Fixed::MIN)), m(2));
        assert!(!map.contains(at(m(3), m(0))));
        assert!(map.contains(at(m(2), m(-2))));
    }

    #[test]
    fn terrain_uses_the_nearest_sample() {
        let map = small();
        assert_eq!(map.terrain_at(at(m(-2), m(-2))), 0);
        assert_eq!(map.terrain_at(at(Fixed::from_num(-9) / 10, m(-2))), 1);
        assert_eq!(map.terrain_at(at(m(0), m(0))), 4);
        assert_eq!(map.terrain_at(at(m(9), m(9))), 0);
    }

    #[test]
    fn hash_covers_the_content() {
        let a = small();
        let layout = MapLayout {
            name: "other name".into(),
            samples_x: 3,
            samples_y: 3,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        let mut heights = a.heights_mm().to_vec();
        let b =
            Map::from_rasters(layout.clone(), heights.clone(), a.terrain_ids().to_vec()).unwrap();
        assert_eq!(a.hash(), b.hash());
        heights[4] += 1;
        let c = Map::from_rasters(layout, heights, a.terrain_ids().to_vec()).unwrap();
        assert_ne!(a.hash(), c.hash());
    }

    #[test]
    fn rejects_bad_layouts() {
        let layout = |samples_x, cell_size_mm| MapLayout {
            name: "bad".into(),
            samples_x,
            samples_y: 3,
            cell_size_mm,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        // 3 samples of 1.5 m: 3 m wide, half is not a whole meter.
        assert!(Map::from_rasters(layout(3, 1500), vec![0; 9], vec![0; 9]).is_err());
        assert!(Map::from_rasters(layout(1, 2000), vec![0; 3], vec![0; 3]).is_err());
        assert!(Map::from_rasters(layout(3, 2000), vec![0; 8], vec![0; 9]).is_err());
        assert!(Map::from_rasters(layout(3, 2000), vec![i32::MAX; 9], vec![0; 9]).is_err());
    }
}
