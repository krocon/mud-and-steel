//! `MapEditor` (prompt 028): the map editor's node. It owns an
//! `editor::EditMap`, forwards the brushes, undo, saving and validation, and
//! offers the same read functions as `SimHost` that the renderers use
//! (`terrain.gd`, `map_objects.gd`, `water.gd`, `ground_pick.gd`), so the
//! editor draws a map exactly like a match does.
//!
//! Positions are Godot coordinates in meters (x east, z south), converted to
//! millimeters here. Heights and terrain are read from the live rasters, so
//! every brush dab shows at once; objects, decoration and water use a
//! simulation map that is rebuilt after each stroke (`display_dirty`).

use crate::SimHost;
use crate::objects::{self, DecoLayer, DecoRasters};
use crate::terrain_layers::Grid;
use crate::water::WaterField;
use editor::brush::Brush;
use editor::{Changed, EditMap, PlacedObject, SampleRect};
use godot::classes::{INode, Node};
use godot::prelude::*;
use sim::Map;
use std::cell::OnceCell;
use std::path::Path;

/// Rays longer than this (meters) do not hit the ground in `ray_pick`.
const MAX_PICK_DISTANCE: f32 = 20_000.0;

/// The simulation's view of the edited map for objects, decoration and
/// water, built on first use after a change.
struct Display {
    map: Map,
    deco: OnceCell<DecoRasters>,
    water: OnceCell<WaterField>,
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct MapEditor {
    map: Option<EditMap>,
    display: OnceCell<Option<Display>>,
    /// A stroke is open (`begin_stroke`): the display waits for its end.
    in_stroke: bool,
    /// Highest sample in millimeters (for ray picking).
    top_mm: i32,
    error: String,
    /// Trigger areas and spawn points of the map (prompt 039,
    /// `mission_funcs.rs`), saved with the map.
    pub(crate) points: sim::mission::MissionPoints,
    base: Base<Node>,
}

#[godot_api]
impl INode for MapEditor {
    fn init(base: Base<Node>) -> Self {
        Self {
            map: None,
            display: OnceCell::new(),
            in_stroke: false,
            top_mm: 0,
            error: String::new(),
            points: sim::mission::MissionPoints::default(),
            base,
        }
    }
}

fn mm(meters: f32) -> i32 {
    let v = (f64::from(meters) * 1000.0).round();
    if v.is_finite() {
        v.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
    } else {
        0
    }
}

fn radius_mm(meters: f32) -> u32 {
    u32::try_from(mm(meters).max(0)).unwrap_or(0)
}

fn pml(fraction: f32) -> u32 {
    (fraction.clamp(0.0, 1.0) * 1000.0).round() as u32
}

fn rect2i(rect: Option<SampleRect>) -> Rect2i {
    rect.map_or_else(Rect2i::default, |r| {
        let c = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
        Rect2i::new(
            Vector2i::new(c(r.i0), c(r.j0)),
            Vector2i::new(c(r.w), c(r.h)),
        )
    })
}

fn changed_dict(changed: Changed) -> VarDictionary {
    vdict! {
        "any" => changed.any(),
        "rect" => rect2i(changed.rect),
        "objects" => changed.objects,
        "meta" => changed.meta,
    }
}

fn strings<'a>(items: impl IntoIterator<Item = &'a String>) -> PackedStringArray {
    items
        .into_iter()
        .map(|s| GString::from(s.as_str()))
        .collect()
}

impl MapEditor {
    /// Result of a counting edit for GDScript: the count, or -1 after
    /// recording the error.
    fn counted(&mut self, result: Option<Result<usize, String>>) -> i64 {
        match result {
            Some(Ok(n)) => {
                if n > 0 {
                    self.edited(None);
                }
                i64::try_from(n).unwrap_or(i64::MAX)
            }
            Some(Err(e)) => {
                self.fail(e);
                -1
            }
            None => -1,
        }
    }

    fn fail(&mut self, message: String) -> bool {
        godot_warn!("map editor: {message}");
        self.error = message;
        false
    }

    /// Takes a new or loaded map and resets the presentation caches.
    fn install(&mut self, map: EditMap) {
        self.top_mm = map.heights().iter().copied().max().unwrap_or(0);
        self.map = Some(map);
        self.display = OnceCell::new();
        self.in_stroke = false;
        self.error.clear();
    }

    /// After an edit: the highest point and, outside strokes, the display.
    fn edited(&mut self, rect: Option<SampleRect>) {
        let (Some(map), Some(r)) = (&self.map, rect) else {
            if !self.in_stroke {
                self.display = OnceCell::new();
            }
            return;
        };
        let (sx, _) = map.samples();
        for j in r.j0..r.j1() {
            let row = j as usize * sx as usize;
            for &h in &map.heights()[row + r.i0 as usize..row + r.i1() as usize] {
                self.top_mm = self.top_mm.max(h);
            }
        }
        if !self.in_stroke {
            self.display = OnceCell::new();
        }
    }

    fn after_history(&mut self, changed: Changed) -> VarDictionary {
        if changed.any() {
            self.top_mm = self
                .map
                .as_ref()
                .and_then(|m| m.heights().iter().copied().max())
                .unwrap_or(0);
            self.display = OnceCell::new();
        }
        changed_dict(changed)
    }

    fn brush(&self, x: f32, z: f32, radius: f32) -> Brush {
        Brush::new(mm(x), mm(z), radius_mm(radius))
    }

    fn display(&self) -> Option<&Display> {
        self.display
            .get_or_init(|| {
                let map = self.map.as_ref()?.to_sim_map();
                match map {
                    Ok(map) => Some(Display {
                        map,
                        deco: OnceCell::new(),
                        water: OnceCell::new(),
                    }),
                    Err(e) => {
                        godot_warn!("map editor: cannot show the map: {e}");
                        None
                    }
                }
            })
            .as_ref()
    }

    fn water(&self) -> Option<&WaterField> {
        let d = self.display()?;
        Some(d.water.get_or_init(|| WaterField::new(&d.map)))
    }

    fn grid(&self) -> Option<Grid<'_>> {
        let map = self.map.as_ref()?;
        Some(Grid {
            samples: map.samples(),
            cell_m: map.cell_mm() as f32 / 1000.0,
            heights_mm: map.heights(),
            ids: map.terrain(),
            offsets: &[],
        })
    }

    /// Bilinear height in meters from the live rasters.
    fn height_f32(map: &EditMap, x: f32, z: f32) -> f32 {
        let (sx, sy) = map.samples();
        let (w, d) = map.size_mm();
        let cell = map.cell_mm() as f32;
        let grid = |v: f32, size: i32, n: u32| {
            ((v * 1000.0 + size as f32 / 2.0) / cell).clamp(0.0, (n - 1) as f32)
        };
        let gx = grid(x, w, sx);
        let gz = grid(z, d, sy);
        let i = (gx as u32).min(sx - 2);
        let j = (gz as u32).min(sy - 2);
        let (fx, fz) = (gx - i as f32, gz - j as f32);
        let h = |i: u32, j: u32| map.sample_height(i, j) as f32;
        let top = h(i, j) + (h(i + 1, j) - h(i, j)) * fx;
        let bottom = h(i, j + 1) + (h(i + 1, j + 1) - h(i, j + 1)) * fx;
        (top + (bottom - top) * fz) / 1000.0
    }
}

#[godot_api]
impl MapEditor {
    // --- Maps ---------------------------------------------------------

    /// Starts a new flat map (sides in meters, multiples of 4).
    #[func]
    fn new_map(&mut self, name: GString, width_m: i64, depth_m: i64, biome: GString) -> bool {
        let Some(biome) = editor::biome_from_name(&biome.to_string()) else {
            return self.fail(format!("unknown biome '{biome}'"));
        };
        let side = |v: i64| u32::try_from(v).unwrap_or(0);
        match EditMap::new(&name.to_string(), side(width_m), side(depth_m), biome) {
            Ok(map) => {
                self.install(map);
                self.points = sim::mission::MissionPoints::default();
                true
            }
            Err(e) => self.fail(e),
        }
    }

    /// Loads the map folder `dir` (absolute path).
    #[func]
    fn load_map(&mut self, dir: GString) -> bool {
        match EditMap::load_dir(Path::new(&dir.to_string())) {
            Ok(map) => {
                self.install(map);
                // Prompt 039: an unreadable points file starts empty.
                self.points = editor::mission_points::load_points(Path::new(&dir.to_string()))
                    .unwrap_or_default();
                true
            }
            Err(e) => self.fail(e),
        }
    }

    /// Saves into the folder `dir` (absolute path, created if needed).
    #[func]
    fn save_map(&mut self, dir: GString) -> bool {
        let Some(map) = self.map.as_mut() else {
            return self.fail("no map".into());
        };
        map.end_stroke();
        self.in_stroke = false;
        let saved = map.save_dir(Path::new(&dir.to_string())).and_then(|()| {
            editor::mission_points::save_points(&self.points, Path::new(&dir.to_string()))
        });
        match saved {
            Ok(()) => {
                self.display = OnceCell::new();
                true
            }
            Err(e) => self.fail(e),
        }
    }

    /// Why the last call failed.
    #[func]
    fn last_error(&self) -> GString {
        GString::from(self.error.as_str())
    }

    #[func]
    fn has_map(&self) -> bool {
        self.map.is_some()
    }

    /// Renames the map (the folder name when saving).
    #[func]
    fn set_map_name(&mut self, name: GString) -> bool {
        let Some(map) = self.map.as_mut() else {
            return false;
        };
        match map.set_name(&name.to_string()) {
            Ok(()) => true,
            Err(e) => self.fail(e),
        }
    }

    #[func]
    fn set_map_biome(&mut self, biome: GString) -> bool {
        let (Some(map), Some(biome)) = (
            self.map.as_mut(),
            editor::biome_from_name(&biome.to_string()),
        ) else {
            return false;
        };
        map.set_biome(biome);
        true
    }

    /// Default weather and time of day of matches on this map (prompt
    /// 040): names as in `SimHost.weather_names()` / `daytime_names()`.
    #[func]
    fn set_map_weather(&mut self, weather: GString, daytime: GString) -> bool {
        let (Some(map), Some(weather), Some(daytime)) = (
            self.map.as_mut(),
            sim::Weather::from_name(&weather.to_string()),
            sim::Daytime::from_name(&daytime.to_string()),
        ) else {
            return false;
        };
        map.set_weather(sim::WeatherSetup { weather, daytime });
        true
    }

    /// `[weather, time of day]` names of the map's default.
    #[func]
    fn map_weather(&self) -> PackedStringArray {
        self.map.as_ref().map_or_else(PackedStringArray::new, |m| {
            let w = m.meta().weather;
            [w.weather.name(), w.daytime.name()]
                .iter()
                .map(|s| GString::from(*s))
                .collect()
        })
    }

    /// Checks the map like a match will load it: `{ok: bool, errors,
    /// warnings: PackedStringArray, map_hash: String}`.
    #[func]
    fn validate(&self) -> VarDictionary {
        let Some(map) = self.map.as_ref() else {
            return vdict! { "ok" => false, "errors" => &PackedStringArray::from(&[GString::from("no map")]) };
        };
        let report = editor::validate::validate_files(&map.files());
        vdict! {
            "ok" => report.ok(),
            "errors" => &strings(&report.errors),
            "warnings" => &strings(&report.warnings),
            "map_hash" => report.map_hash.map_or_else(String::new, |h| format!("{h:#018x}")),
        }
    }

    // --- Brushes ------------------------------------------------------

    /// Opens a stroke: everything until `end_stroke` is one undo step.
    #[func]
    fn begin_stroke(&mut self) {
        if let Some(map) = self.map.as_mut() {
            map.begin_stroke();
            self.in_stroke = true;
        }
    }

    /// Closes the stroke; returns what it changed (`any`, `rect` of samples,
    /// `objects`, `meta`).
    #[func]
    fn end_stroke(&mut self) -> VarDictionary {
        let Some(map) = self.map.as_mut() else {
            return changed_dict(Changed::default());
        };
        let changed = map.end_stroke();
        self.in_stroke = false;
        self.display = OnceCell::new();
        changed_dict(changed)
    }

    /// Raises (negative: lowers) the ground by `amount` meters at the
    /// center. Returns the changed samples.
    #[func]
    fn raise(&mut self, x: f32, z: f32, radius: f32, amount: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let rect = self.map.as_mut().and_then(|m| m.raise(brush, mm(amount)));
        self.edited(rect);
        rect2i(rect)
    }

    /// Smooths towards the local mean (`strength` 0..1 at the center).
    #[func]
    fn smooth(&mut self, x: f32, z: f32, radius: f32, strength: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let rect = self
            .map
            .as_mut()
            .and_then(|m| m.smooth(brush, pml(strength)));
        self.edited(rect);
        rect2i(rect)
    }

    /// Levels towards `target` meters (`strength` 0..1 at the center).
    #[func]
    fn flatten(&mut self, x: f32, z: f32, radius: f32, target: f32, strength: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let rect = self
            .map
            .as_mut()
            .and_then(|m| m.flatten(brush, mm(target), pml(strength)));
        self.edited(rect);
        rect2i(rect)
    }

    /// Paints terrain `id` (`data/terrain.toml`).
    #[func]
    fn paint(&mut self, x: f32, z: f32, radius: f32, id: i64) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let Ok(id) = u8::try_from(id) else {
            return Rect2i::default();
        };
        let result = self.map.as_mut().map(|m| m.paint(brush, id));
        match result {
            Some(Ok(rect)) => {
                self.edited(rect);
                rect2i(rect)
            }
            Some(Err(e)) => {
                self.fail(e);
                Rect2i::default()
            }
            None => Rect2i::default(),
        }
    }

    /// Ridge (positive `amount` meters) or valley (negative) with a linear
    /// falloff to the rim (prompt 060).
    #[func]
    fn ridge(&mut self, x: f32, z: f32, radius: f32, amount: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let rect = self.map.as_mut().and_then(|m| m.ridge(brush, mm(amount)));
        self.edited(rect);
        rect2i(rect)
    }

    /// Terraces of `step` meters (`strength` 0..1 at the center, prompt
    /// 060).
    #[func]
    fn terrace(&mut self, x: f32, z: f32, radius: f32, step: f32, strength: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let rect = self
            .map
            .as_mut()
            .and_then(|m| m.terrace(brush, radius_mm(step), pml(strength)));
        self.edited(rect);
        rect2i(rect)
    }

    /// A road piece from the previous dab at `(from_x, from_z)` and
    /// `from_level` to this one at `level` meters, painted with terrain
    /// `id`, with embankments and cuttings beside it (prompt 060).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn road(
        &mut self,
        x: f32,
        z: f32,
        radius: f32,
        level: f32,
        from_x: f32,
        from_z: f32,
        from_level: f32,
        id: i64,
    ) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let Ok(id) = u8::try_from(id) else {
            return Rect2i::default();
        };
        let from = (mm(from_x), mm(from_z), mm(from_level));
        let result = self
            .map
            .as_mut()
            .map(|m| m.road(brush, mm(level), from, id));
        match result {
            Some(Ok(rect)) => {
                self.edited(rect);
                rect2i(rect)
            }
            Some(Err(e)) => {
                self.fail(e);
                Rect2i::default()
            }
            None => Rect2i::default(),
        }
    }

    /// Digs a river bed `depth` meters below `level` (the bank height).
    #[func]
    fn river(&mut self, x: f32, z: f32, radius: f32, level: f32, depth: f32) -> Rect2i {
        let brush = self.brush(x, z, radius);
        let depth = radius_mm(depth);
        let rect = self
            .map
            .as_mut()
            .and_then(|m| m.river(brush, mm(level), depth));
        self.edited(rect);
        rect2i(rect)
    }

    /// Scatters objects (`"<catalog id>:<variant>"`) into the circle:
    /// `per_hectare` on average, `min_spacing` meters apart. Returns how
    /// many were placed (-1 on an error).
    #[func]
    fn scatter(
        &mut self,
        x: f32,
        z: f32,
        radius: f32,
        kinds: PackedStringArray,
        per_hectare: f32,
        min_spacing: f32,
    ) -> i64 {
        let brush = self.brush(x, z, radius);
        let kinds: Vec<(String, String)> = kinds
            .as_slice()
            .iter()
            .map(|k| {
                let k = k.to_string();
                let (id, variant) = k.split_once(':').unwrap_or((&k, ""));
                (id.to_string(), variant.to_string())
            })
            .collect();
        let per_hectare = per_hectare.clamp(0.0, 100_000.0).round() as u32;
        let result = self
            .map
            .as_mut()
            .map(|m| m.scatter(brush, &kinds, per_hectare, radius_mm(min_spacing)));
        match result {
            Some(Ok(n)) => {
                self.edited(None);
                n as i64
            }
            Some(Err(e)) => {
                self.fail(e);
                -1
            }
            None => -1,
        }
    }

    /// Places one object (`"<catalog id>:<variant>"`); bridges snap onto
    /// the path grid. Returns the placed position and heading as
    /// `Vector3(x, heading_deg, z)`, or null on an error.
    #[func]
    fn place_object(&mut self, kind: GString, x: f32, z: f32, heading_deg: i64) -> Variant {
        let kind = kind.to_string();
        let (id, variant) = kind.split_once(':').unwrap_or((&kind, ""));
        let object = PlacedObject {
            kind: id.into(),
            variant: variant.into(),
            x_mm: mm(x),
            y_mm: mm(z),
            heading_deg: u16::try_from(heading_deg.rem_euclid(360)).unwrap_or(0),
        };
        let result = self.map.as_mut().map(|m| m.place(object));
        match result {
            Some(Ok(o)) => {
                self.edited(None);
                Vector3::new(
                    o.x_mm as f32 / 1000.0,
                    f32::from(o.heading_deg),
                    o.y_mm as f32 / 1000.0,
                )
                .to_variant()
            }
            Some(Err(e)) => {
                self.fail(e);
                Variant::nil()
            }
            None => Variant::nil(),
        }
    }

    /// Places a compound of `data/compounds.toml` (prompt 059) centered at
    /// the point, turned by `heading_deg` (rounded to 90°). Returns how
    /// many objects were placed (-1 on an error).
    #[func]
    fn place_compound(&mut self, name: GString, x: f32, z: f32, heading_deg: i64) -> i64 {
        let heading = u16::try_from(heading_deg.rem_euclid(360)).unwrap_or(0);
        let result = self
            .map
            .as_mut()
            .map(|m| m.place_compound(&name.to_string(), mm(x), mm(z), heading));
        self.counted(result)
    }

    /// Village brush (prompt 059): packs courtyard compounds into the
    /// circle. Returns how many compounds were placed (-1 on an error).
    #[func]
    fn village(&mut self, x: f32, z: f32, radius: f32) -> i64 {
        let brush = self.brush(x, z, radius);
        let result = self.map.as_mut().map(|m| m.village(brush));
        self.counted(result)
    }

    /// Names and labels of the compounds: `"<name>:<label>"`.
    #[func]
    fn compound_names(&self) -> PackedStringArray {
        editor::compound::Compounds::builtin().map_or_else(
            |_| PackedStringArray::new(),
            |c| {
                c.list
                    .iter()
                    .map(|c| GString::from(format!("{}:{}", c.name, c.label).as_str()))
                    .collect()
            },
        )
    }

    /// Removes objects in the circle (only catalog id `kind` unless empty).
    #[func]
    fn erase_objects(&mut self, x: f32, z: f32, radius: f32, kind: GString) -> i64 {
        let brush = self.brush(x, z, radius);
        let kind = kind.to_string();
        let filter = (!kind.is_empty()).then_some(kind.as_str());
        let n = self.map.as_mut().map_or(0, |m| m.erase(brush, filter));
        if n > 0 {
            self.edited(None);
        }
        n as i64
    }

    /// Sets the start position of `player` (rounded to whole meters).
    #[func]
    fn set_start(&mut self, player: i64, x: f32, z: f32) {
        let (Some(map), Ok(player)) = (self.map.as_mut(), u8::try_from(player)) else {
            return;
        };
        map.set_start(player, mm(x), mm(z));
        self.edited(None);
    }

    /// Adds a strongpoint marker at the ground point (prompt 052); returns
    /// its name ("" without a map).
    #[func]
    fn add_strongpoint(&mut self, x: f32, z: f32) -> GString {
        let Some(map) = self.map.as_mut() else {
            return GString::new();
        };
        let name = map.add_strongpoint(mm(x), mm(z));
        self.edited(None);
        GString::from(name.as_str())
    }

    /// Removes the strongpoint markers within `radius` m of the point;
    /// returns how many.
    #[func]
    fn remove_strongpoints(&mut self, x: f32, z: f32, radius: f32) -> i64 {
        let Some(map) = self.map.as_mut() else {
            return 0;
        };
        let n = map.remove_strongpoints(mm(x), mm(z), mm(radius));
        if n > 0 {
            self.edited(None);
        }
        i64::try_from(n).unwrap_or(0)
    }

    /// Strongpoint markers with their layout for the preview: per marker
    /// `{name, position (Vector3), pieces: [{kind, from, to}]}`.
    #[func]
    fn strongpoints(&self) -> Array<VarDictionary> {
        let Some(map) = &self.map else {
            return Array::new();
        };
        let Ok(rules) = sim::PositionRules::builtin() else {
            return Array::new();
        };
        let ground = |p: sim::Vec2| {
            let (x, z) = (crate::fixed_to_f32(p.x), crate::fixed_to_f32(p.y));
            Vector3::new(x, Self::height_f32(map, x, z), z)
        };
        map.meta()
            .markers
            .iter()
            .filter(|(name, _)| name.starts_with(&rules.strongpoint_prefix))
            .map(|(name, p)| {
                let center = sim::Vec2::from_meters(p[0], p[1]);
                let pieces: Array<VarDictionary> = sim::strongpoint_layout(&rules, center)
                    .iter()
                    .map(|piece| {
                        let mut d = VarDictionary::new();
                        d.set("kind", piece.kind.name());
                        d.set("from", ground(piece.from));
                        d.set("to", ground(piece.to));
                        d
                    })
                    .collect();
                let mut d = VarDictionary::new();
                d.set("name", name.as_str());
                d.set("position", ground(center));
                d.set("pieces", &pieces);
                d
            })
            .collect()
    }

    /// Suggested team of `player`'s start for the lobby (prompt 037); -1:
    /// none (the player is on a team of their own).
    #[func]
    fn set_start_team(&mut self, player: i64, team: i64) {
        let (Some(map), Ok(player)) = (self.map.as_mut(), u8::try_from(player)) else {
            return;
        };
        map.set_start_team(player, u8::try_from(team).ok());
        self.edited(None);
    }

    /// Suggested teams of the starts: player id → team (players without
    /// one are left out).
    #[func]
    fn start_teams(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(map) = &self.map {
            for &(player, team) in &map.meta().start_teams {
                d.set(i64::from(player), i64::from(team));
            }
        }
        d
    }

    /// Start positions: player id → ground point (Vector3).
    #[func]
    fn start_positions(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        if let Some(map) = &self.map {
            for &(player, p) in &map.meta().starts {
                let (x, z) = (p[0] as f32, p[1] as f32);
                d.set(
                    i64::from(player),
                    Vector3::new(x, Self::height_f32(map, x, z), z),
                );
            }
        }
        d
    }

    #[func]
    fn undo(&mut self) -> VarDictionary {
        self.in_stroke = false;
        let changed = self.map.as_mut().map(EditMap::undo).unwrap_or_default();
        self.after_history(changed)
    }

    #[func]
    fn redo(&mut self) -> VarDictionary {
        self.in_stroke = false;
        let changed = self.map.as_mut().map(EditMap::redo).unwrap_or_default();
        self.after_history(changed)
    }

    #[func]
    fn undo_steps(&self) -> i64 {
        self.map.as_ref().map_or(0, |m| m.undo_steps() as i64)
    }

    #[func]
    fn redo_steps(&self) -> i64 {
        self.map.as_ref().map_or(0, |m| m.redo_steps() as i64)
    }

    /// Changes with every edit (for tests).
    #[func]
    fn version(&self) -> i64 {
        self.map
            .as_ref()
            .map_or(0, |m| i64::try_from(m.version()).unwrap_or(i64::MAX))
    }

    /// Every placeable object as `"<catalog id>:<variant>"`.
    #[func]
    fn object_catalog(&self) -> PackedStringArray {
        let rules = self.map.as_ref().map(EditMap::object_rules);
        rules.map_or_else(PackedStringArray::new, |r| {
            r.kinds
                .iter()
                .map(|k| GString::from(format!("{}:{}", k.id, k.variant).as_str()))
                .collect()
        })
    }

    /// Names of the terrain types; the index is the id.
    #[func]
    fn terrain_names(&self) -> PackedStringArray {
        let rules = self.map.as_ref().map(EditMap::terrain_rules);
        rules.map_or_else(PackedStringArray::new, |r| {
            r.types
                .iter()
                .map(|t| GString::from(t.name.as_str()))
                .collect()
        })
    }

    /// Debug color of every terrain type, indexed by terrain id (the
    /// palette of the terrain brush, prompt 073).
    #[func]
    fn terrain_colors(&self) -> PackedColorArray {
        let rules = self.map.as_ref().map(EditMap::terrain_rules);
        rules.map_or_else(PackedColorArray::new, |r| {
            r.types
                .iter()
                .map(|t| {
                    let [_, r, g, b] = t.debug_color.to_be_bytes();
                    Color::from_rgba8(r, g, b, 255)
                })
                .collect()
        })
    }

    // --- Read functions shared with SimHost (renderers, picking) -------

    #[func]
    fn map_name(&self) -> GString {
        self.map
            .as_ref()
            .map_or_else(GString::new, |m| GString::from(m.meta().name.as_str()))
    }

    #[func]
    fn map_biome(&self) -> GString {
        self.map
            .as_ref()
            .map_or_else(GString::new, |m| GString::from(m.meta().biome.name()))
    }

    #[func]
    fn map_size(&self) -> Vector2 {
        self.map.as_ref().map_or(Vector2::ZERO, |m| {
            let (w, d) = m.size_mm();
            Vector2::new(w as f32 / 1000.0, d as f32 / 1000.0)
        })
    }

    #[func]
    fn grid_samples(&self, stride: i64) -> Vector2i {
        let stride = u32::try_from(stride.clamp(1, 1024)).unwrap_or(1);
        self.map.as_ref().map_or(Vector2i::ZERO, |m| {
            let (sx, sy) = m.samples();
            let n = |v: u32| i32::try_from((v - 1).div_ceil(stride) + 1).unwrap_or(0);
            Vector2i::new(n(sx), n(sy))
        })
    }

    #[func]
    fn cell_size(&self) -> f32 {
        self.map
            .as_ref()
            .map_or(0.0, |m| m.cell_mm() as f32 / 1000.0)
    }

    #[func]
    pub(crate) fn height_at(&self, x: f32, z: f32) -> f32 {
        self.map.as_ref().map_or(0.0, |m| Self::height_f32(m, x, z))
    }

    /// Terrain id of the sample nearest to `(x, z)`.
    #[func]
    fn terrain_at(&self, x: f32, z: f32) -> i64 {
        self.map
            .as_ref()
            .map_or(0, |m| i64::from(m.terrain_near(mm(x), mm(z))))
    }

    /// Ground point hit by a ray (marching plus bisection like
    /// `SimHost.ray_pick`), on the live heights; null if it misses.
    #[func]
    fn ray_pick(&self, origin: Vector3, direction: Vector3) -> Variant {
        let (Some(map), Some(direction)) = (self.map.as_ref(), direction.try_normalized()) else {
            return Variant::nil();
        };
        let top = self.top_mm as f32 / 1000.0;
        let above = |t: f32| {
            let p = origin + direction * t;
            p.y - Self::height_f32(map, p.x, p.z)
        };
        let point = |t: f32| {
            let p = origin + direction * t;
            Vector3::new(p.x, Self::height_f32(map, p.x, p.z), p.z).to_variant()
        };
        let mut t = 0.0;
        if origin.y > top {
            if direction.y >= 0.0 {
                return Variant::nil();
            }
            t = (origin.y - top) / -direction.y;
        }
        if above(t) <= 0.0 {
            return point(t);
        }
        let mut hit = None;
        while t < MAX_PICK_DISTANCE {
            let step = (0.5 + t * 0.01).min(20.0);
            if above(t + step) <= 0.0 {
                hit = Some((t, t + step));
                break;
            }
            t += step;
            if (origin + direction * t).y > top && direction.y >= 0.0 {
                return Variant::nil();
            }
        }
        let Some((mut low, mut high)) = hit else {
            return Variant::nil();
        };
        for _ in 0..24 {
            let mid = 0.5 * (low + high);
            if above(mid) > 0.0 {
                low = mid;
            } else {
                high = mid;
            }
        }
        point(high)
    }

    /// Rasters of the terrain renderer (see `SimHost.terrain_layers`), from
    /// the live heights and terrain.
    #[func]
    fn terrain_layers(&self, i0: i64, j0: i64, w: i64, h: i64) -> VarDictionary {
        let mut out = VarDictionary::new();
        let Some(grid) = self.grid() else {
            return out;
        };
        let Some(rect) = grid.clip(i0, j0, w, h) else {
            return out;
        };
        let layers = grid.layers(rect);
        out.set(
            "rect",
            rect2i(Some(SampleRect {
                i0: rect.i0,
                j0: rect.j0,
                w: rect.w,
                h: rect.h,
            })),
        );
        out.set(
            "height",
            &PackedFloat32Array::from(layers.heights.as_slice()),
        );
        out.set("normal", &PackedByteArray::from(layers.normals.as_slice()));
        out.set("splat0", &PackedByteArray::from(layers.splat0.as_slice()));
        out.set("splat1", &PackedByteArray::from(layers.splat1.as_slice()));
        out.set("track", &PackedByteArray::from(layers.track.as_slice()));
        out
    }

    #[func]
    fn terrain_chunk_ranges(&self, cells: i64) -> PackedVector2Array {
        let cells = u32::try_from(cells.clamp(1, 4096)).unwrap_or(1);
        self.grid().map_or_else(PackedVector2Array::new, |g| {
            g.chunk_ranges(cells)
                .into_iter()
                .map(|(lo, hi)| Vector2::new(lo, hi))
                .collect()
        })
    }

    /// Water surface mesh (see `SimHost.water_mesh`).
    #[func]
    fn water_mesh(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(mesh) = self.water().map(WaterField::mesh) else {
            return d;
        };
        let v: PackedVector3Array = mesh
            .vertices
            .iter()
            .map(|p| Vector3::new(p[0], p[1], p[2]))
            .collect();
        let uv: PackedVector2Array = mesh.uv.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        let uv2: PackedVector2Array = mesh.uv2.iter().map(|p| Vector2::new(p[0], p[1])).collect();
        let indices: PackedInt32Array = mesh.indices.iter().map(|&i| i as i32).collect();
        d.set("vertices", &v);
        d.set("uv", &uv);
        d.set("uv2", &uv2);
        d.set("indices", &indices);
        d
    }

    /// Water level per sample (see `SimHost.water_levels`).
    #[func]
    fn water_levels(&self) -> PackedFloat32Array {
        self.water()
            .map(|w| {
                w.level
                    .iter()
                    .map(|&l| if l.is_nan() { -10_000.0 } else { l })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Water flow direction (compass degrees).
    #[func]
    fn water_flow_deg(&self) -> i64 {
        self.map
            .as_ref()
            .map_or(180, |m| i64::from(m.meta().water_flow_deg))
    }

    #[func]
    fn object_kinds(&self) -> PackedStringArray {
        self.display().map_or_else(PackedStringArray::new, |d| {
            d.map
                .object_kinds()
                .iter()
                .map(|k| GString::from(format!("{}:{}", k.id, k.variant).as_str()))
                .collect()
        })
    }

    #[func]
    fn object_count(&self) -> i64 {
        self.map.as_ref().map_or(0, |m| m.objects().len() as i64)
    }

    #[func]
    fn object_groups(&self, chunk_m: f32) -> Array<VarDictionary> {
        let Some(d) = self.display() else {
            return Array::new();
        };
        objects::groups(&d.map, chunk_m.max(1.0))
            .into_iter()
            .map(|g| {
                vdict! {
                    "kind" => i64::from(g.kind),
                    "chunk" => Vector2i::new(g.chunk.0, g.chunk.1),
                    "buffer" => &PackedFloat32Array::from(g.buffer.as_slice()),
                }
            })
            .collect()
    }

    /// Decoration of one layer (see `SimHost.scatter_deco`).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn scatter_deco(
        &self,
        x0: f32,
        z0: f32,
        size: f32,
        spacing: f32,
        seed: i64,
        density: PackedFloat32Array,
        wood_factor: f32,
        patchiness: f32,
        weights: PackedFloat32Array,
        scale: Vector2,
        sink: f32,
        #[opt(default = Vector2::ZERO)] shore: Vector2,
        #[opt(default = 0.0)] fray: f32,
        #[opt(default = 0.0)] rock: f32,
    ) -> Array<PackedFloat32Array> {
        let Some(d) = self.display() else {
            return Array::new();
        };
        let rasters = d.deco.get_or_init(|| DecoRasters::new(&d.map));
        let layer = DecoLayer {
            spacing,
            seed: seed.cast_unsigned(),
            density: density.as_slice(),
            wood_factor,
            patchiness,
            weights: weights.as_slice(),
            scale: (scale.x, scale.y),
            sink,
            shore: (shore.x, shore.y),
            fray,
            rock,
        };
        let water =
            (shore != Vector2::ZERO).then(|| d.water.get_or_init(|| WaterField::new(&d.map)));
        objects::scatter(&d.map, rasters, water, &layer, x0, z0, size, |_| false)
            .into_iter()
            .map(|b| PackedFloat32Array::from(b.as_slice()))
            .collect()
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Content hash of the loaded map (`sim::Map::hash`) as hex, "" before
    /// a start; the map editor's check reports the same value (prompt 028).
    #[func]
    fn map_hash(&self) -> GString {
        self.game.as_ref().map_or_else(GString::new, |g| {
            GString::from(format!("{:#018x}", g.map().hash()).as_str())
        })
    }
}
