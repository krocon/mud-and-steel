//! Map previews for the map picker (prompt 201): a top view in the style of
//! the minimap and a coarse height grid for the small 3D relief, read from
//! a map folder without starting a match, plus what the picker draws over
//! it (starts per start layout, supply points). Presentation only: nothing
//! here reaches a simulation.
//!
//! Building the picture of a large map takes a moment, so the menu asks for
//! it with `map_preview_start` (a plain Rust thread that touches no Godot
//! object) and collects it with `map_preview_poll`; the menu caches the
//! result in `user://map_previews/` (`menu/map_preview.gd`).

use crate::SimHost;
use crate::hud_funcs::ground_rgb;
use godot::prelude::*;
use sim::vision::{VisionGrid, VisionRules};
use sim::{Fixed, Map, TerrainRules};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

/// Longest edge of a preview picture in pixels, about: the cell is the
/// map's longest edge divided by this, in whole meters.
const TARGET_PX: f32 = 640.0;
/// Finest and coarsest preview cell (m): the map's sample spacing and the
/// minimap's vision cell.
const MIN_CELL_M: u32 = 2;
const MAX_CELL_M: u32 = 8;
/// Vertices of the relief along the map's longer edge.
const RELIEF_STEPS: u32 = 128;
/// Bridge decks in the picture.
const DECK: [u8; 3] = [0xd8, 0xc8, 0xa0];

/// A finished preview.
struct Preview {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
    relief_columns: u32,
    relief_rows: u32,
    /// Ground height in meters, row-major from the north-west corner.
    relief: Vec<f32>,
    size_m: (f32, f32),
    ms: f32,
}

enum Job {
    Running,
    Done(Result<Preview, String>),
}

/// Jobs by map folder.
static JOBS: Mutex<BTreeMap<String, Job>> = Mutex::new(BTreeMap::new());

fn jobs() -> std::sync::MutexGuard<'static, BTreeMap<String, Job>> {
    JOBS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The map of `dir` with its objects (the built-in map for "").
fn load(dir: &str) -> Result<Map, String> {
    if dir.is_empty() {
        return Ok((*sim::scenario::builtin_map()).clone());
    }
    crate::load_map_dir(Path::new(dir))
}

/// Preview cell edge in meters for a map of `size_m`.
fn cell_m(size_m: (f32, f32)) -> u32 {
    let wanted = (size_m.0.max(size_m.1) / TARGET_PX).ceil();
    (wanted as u32).clamp(MIN_CELL_M, MAX_CELL_M)
}

fn build(dir: &str) -> Result<Preview, String> {
    let started = Instant::now();
    let map = load(dir)?;
    let size = map.size();
    let size_m = (crate::fixed_to_f32(size.x), crate::fixed_to_f32(size.y));
    let mut rules = VisionRules::builtin().map_err(|e| e.to_string())?;
    // A finer grid than the match's on small maps; the forest threshold
    // (trees per cell) shrinks with the cell area.
    let cell = cell_m(size_m);
    let old_mm = rules.grid.cell_mm.max(1);
    let new_mm = cell * 1000;
    let scaled = u64::from(rules.grid.foliage_min_objects) * u64::from(new_mm).pow(2)
        / u64::from(old_mm).pow(2);
    rules.grid.foliage_min_objects = u32::try_from(scaled).unwrap_or(1).max(1);
    rules.grid.cell_mm = new_mm;
    let grid = VisionGrid::new(&map, &rules, &[]);
    let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let mut rgb = ground_rgb(&map, &grid, &terrain.types);
    draw_decks(&map, &grid, &mut rgb);
    let (relief_columns, relief_rows, relief) = relief(&map);
    Ok(Preview {
        width: grid.columns,
        height: grid.rows,
        rgb,
        relief_columns,
        relief_rows,
        relief,
        size_m,
        ms: started.elapsed().as_secs_f32() * 1000.0,
    })
}

/// Paints the bridge decks into the picture.
fn draw_decks(map: &Map, grid: &VisionGrid, rgb: &mut [u8]) {
    for bridge in map.bridges() {
        let center = (
            crate::fixed_to_f32(bridge.center.x),
            crate::fixed_to_f32(bridge.center.y),
        );
        let forward = (
            crate::fixed_to_f32(bridge.forward.x),
            crate::fixed_to_f32(bridge.forward.y),
        );
        let half_length = crate::fixed_to_f32(bridge.half_length);
        // At least one pixel wide.
        let half_width =
            crate::fixed_to_f32(bridge.half_width).max(crate::fixed_to_f32(grid.cell) * 0.5);
        let steps = (half_length * 2.0 / crate::fixed_to_f32(grid.cell) * 2.0).ceil() as i32;
        let across = (half_width * 2.0 / crate::fixed_to_f32(grid.cell) * 2.0).ceil() as i32;
        for a in 0..=steps.max(1) {
            for b in 0..=across.max(1) {
                let t = (a as f32 / steps.max(1) as f32 - 0.5) * 2.0 * half_length;
                let u = (b as f32 / across.max(1) as f32 - 0.5) * 2.0 * half_width;
                let x = center.0 + forward.0 * t - forward.1 * u;
                let y = center.1 + forward.1 * t + forward.0 * u;
                let p = sim::Vec2::new(Fixed::from_num(x), Fixed::from_num(y));
                if !map.contains(p) {
                    continue;
                }
                let i = grid.index_of(p) as usize * 3;
                if let Some(px) = rgb.get_mut(i..i + 3) {
                    px.copy_from_slice(&DECK);
                }
            }
        }
    }
}

/// Ground heights on a coarse grid ([`RELIEF_STEPS`] along the longer
/// edge), from the nearest samples.
fn relief(map: &Map) -> (u32, u32, Vec<f32>) {
    let (sx, sy) = map.samples();
    let longer = sx.max(sy).max(2) - 1;
    let step = longer.div_ceil(RELIEF_STEPS).max(1);
    let columns = (sx - 1).div_ceil(step) + 1;
    let rows = (sy - 1).div_ceil(step) + 1;
    let mut out = Vec::with_capacity((columns * rows) as usize);
    for r in 0..rows {
        for c in 0..columns {
            let i = (c * step).min(sx - 1);
            let j = (r * step).min(sy - 1);
            out.push(map.sample_height_mm(i, j) as f32 / 1000.0);
        }
    }
    (columns, rows, out)
}

fn to_dictionary(result: Result<Preview, String>) -> VarDictionary {
    match result {
        Ok(p) => {
            let mut d = vdict! {
                "width" => i64::from(p.width),
                "height" => i64::from(p.height),
                "relief_columns" => i64::from(p.relief_columns),
                "relief_rows" => i64::from(p.relief_rows),
                "size_m" => Vector2::new(p.size_m.0, p.size_m.1),
                "ms" => p.ms,
            };
            d.set("rgb", &PackedByteArray::from(p.rgb));
            d.set("relief", &PackedFloat32Array::from(p.relief));
            d
        }
        Err(e) => vdict! { "error" => e },
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Builds the preview of the map in `map_dir` ("" = the built-in map)
    /// now: `width`, `height` and `rgb` (RGB8, row by row from the
    /// north-west corner, as the minimap), `relief_columns`,
    /// `relief_rows`, `relief` (heights in m), `size_m`, `ms` (time taken);
    /// or `error`.
    #[func]
    fn map_preview_build(map_dir: GString) -> VarDictionary {
        to_dictionary(build(&map_dir.to_string()))
    }

    /// Starts building the preview of `map_dir` in the background (no-op
    /// while one runs or waits to be collected).
    #[func]
    fn map_preview_start(map_dir: GString) {
        let dir = map_dir.to_string();
        {
            let mut running = jobs();
            if running.contains_key(&dir) {
                return;
            }
            running.insert(dir.clone(), Job::Running);
        }
        std::thread::spawn(move || {
            let result = build(&dir);
            jobs().insert(dir, Job::Done(result));
        });
    }

    /// The preview `map_preview_start` built for `map_dir` (see
    /// `map_preview_build`), handed out once; empty while it is still
    /// being built or when none was started.
    #[func]
    fn map_preview_poll(map_dir: GString) -> VarDictionary {
        let dir = map_dir.to_string();
        let mut all = jobs();
        if !matches!(all.get(&dir), Some(Job::Done(_))) {
            return VarDictionary::new();
        }
        match all.remove(&dir) {
            Some(Job::Done(result)) => to_dictionary(result),
            _ => VarDictionary::new(),
        }
    }

    /// What the picker shows of a map besides the picture, from `map.toml`
    /// of `map_dir` only (fast: the rasters are not read): `size_m`, `biome`, `weather`,
    /// `daytime`, `layouts` (the standard layout first, then the variants:
    /// `name`, `title`, `starts` (array of `player`, `team` (-1 without),
    /// `position`) and `supply` (positions of the supply points that count
    /// in it)). Empty if the file cannot be read.
    #[func]
    fn map_preview_info(map_dir: GString) -> VarDictionary {
        let dir = map_dir.to_string();
        let text =
            std::fs::read_to_string(Path::new(&dir).join("map.toml")).map_err(|e| e.to_string());
        match text.and_then(|t| toml::from_str::<Header>(&t).map_err(|e| e.to_string())) {
            Ok(header) => header.to_dictionary(),
            Err(e) => {
                godot_warn!("map preview: {dir}: {e}");
                VarDictionary::new()
            }
        }
    }
}

/// The part of `map.toml` the picker needs (the full format and its
/// checks: `sim::Map::load`).
#[derive(serde::Deserialize)]
struct Header {
    cell_size_mm: u32,
    samples: [u32; 2],
    #[serde(default)]
    biome: sim::map::Biome,
    #[serde(default)]
    weather: sim::weather::Weather,
    #[serde(default)]
    daytime: sim::weather::Daytime,
    #[serde(default)]
    start: Vec<HeaderStart>,
    #[serde(default)]
    marker: Vec<HeaderMarker>,
    #[serde(default)]
    variant: Vec<HeaderVariant>,
}

#[derive(serde::Deserialize)]
struct HeaderStart {
    player: u8,
    #[serde(default)]
    team: Option<u8>,
    position_m: [i32; 2],
}

#[derive(serde::Deserialize)]
struct HeaderMarker {
    name: String,
    position_m: [i32; 2],
}

#[derive(serde::Deserialize)]
struct HeaderVariant {
    name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    start: Vec<HeaderStart>,
    #[serde(default)]
    supply: Option<Vec<String>>,
}

fn meters(p: [i32; 2]) -> Vector2 {
    Vector2::new(p[0] as f32, p[1] as f32)
}

impl Header {
    fn to_dictionary(&self) -> VarDictionary {
        let prefix = sim::EconomyRules::builtin()
            .map(|e| e.supply.marker_prefix)
            .unwrap_or_else(|_| "supply".to_owned());
        let supply_of = |names: Option<&Vec<String>>| {
            self.marker
                .iter()
                .filter(|m| m.name.starts_with(&prefix))
                .filter(|m| names.is_none_or(|n| n.contains(&m.name)))
                .map(|m| meters(m.position_m))
                .collect::<PackedVector2Array>()
        };
        let starts_of = |starts: &[HeaderStart]| {
            let mut sorted: Vec<&HeaderStart> = starts.iter().collect();
            sorted.sort_by_key(|s| s.player);
            let mut out = Array::<VarDictionary>::new();
            for s in sorted {
                out.push(&vdict! {
                    "player" => i64::from(s.player),
                    "team" => s.team.map_or(-1, i64::from),
                    "position" => meters(s.position_m),
                });
            }
            out
        };
        let layout = |name: &str, title: &str, starts, supply| {
            let mut d = vdict! { "name" => name, "title" => title };
            d.set("starts", &starts);
            d.set("supply", &supply);
            d
        };
        let mut layouts = Array::<VarDictionary>::new();
        layouts.push(&layout(
            sim::map::STANDARD_VARIANT,
            "",
            starts_of(&self.start),
            supply_of(None),
        ));
        for v in &self.variant {
            layouts.push(&layout(
                &v.name,
                &v.title,
                starts_of(&v.start),
                supply_of(v.supply.as_ref()),
            ));
        }
        let extent = |samples: u32| {
            (u64::from(samples.saturating_sub(1)) * u64::from(self.cell_size_mm)) as f32 / 1000.0
        };
        let mut d = vdict! {
            "size_m" => Vector2::new(extent(self.samples[0]), extent(self.samples[1])),
            "biome" => self.biome.name(),
            "weather" => self.weather.name(),
            "daytime" => self.daytime.name(),
        };
        d.set("layouts", &layouts);
        d
    }
}
