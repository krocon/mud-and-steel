//! Map editing (prompt 028): the data behind the in-game map editor.
//!
//! An [`EditMap`] holds a map in its file form (heights in millimeters,
//! terrain ids, objects as `objects.csv` lines, start positions, markers,
//! biome) and changes it with brushes ([`brush`]), every stroke undoable
//! ([`history`]). It loads from and saves to a map folder in the format of
//! `docs/MAPS.md`; [`validate`] checks a map exactly the way the simulation
//! will load it.
//!
//! Like the simulation, the editor uses integers only: the same edit script
//! produces the same files and therefore the same map hash. It is a tool,
//! though, not part of the simulation: `crates/sim` stays the game rules.

pub mod brush;
pub mod compound;
mod files;
pub mod history;
pub mod mission_points;
pub mod validate;

pub use files::MapFiles;
pub use history::Changed;
pub use validate::Report;

use history::History;
use sim::map::{MapLayout, Marker, StartPosition};
use sim::objects::{CSV_HEADER, ObjectList};
use sim::{Biome, Map, ObjectRules, TerrainRules, Vec2};

/// Distance between samples of new maps (the grid of all maps so far).
pub const CELL_MM: u32 = 2000;
/// Smallest and largest side of a new map in meters.
pub const MIN_SIZE_M: u32 = 64;
pub const MAX_SIZE_M: u32 = 8000;
/// Edited heights stay within ±this many millimeters, so the saved height
/// encoding stays within the simulation's ±10 km.
pub const HEIGHT_LIMIT_MM: i32 = 9_000_000;

/// A placed object as one line of `objects.csv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedObject {
    /// Catalog id, e.g. `env_tree_deciduous`.
    pub kind: String,
    /// Model variant, e.g. `Oak`.
    pub variant: String,
    pub x_mm: i32,
    pub y_mm: i32,
    /// `0..360`, clockwise; 0 = the model's length points north.
    pub heading_deg: u16,
}

/// Everything in `map.toml` besides the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    /// Map (and folder) name.
    pub name: String,
    /// How the map was made (kept from the loaded file).
    pub generator: Option<String>,
    pub biome: Biome,
    pub water_flow_deg: u16,
    /// Default weather and time of day of matches (prompt 040).
    pub weather: sim::WeatherSetup,
    /// `(player, [x, y] meters)`, sorted by player.
    pub starts: Vec<(u8, [i32; 2])>,
    /// `(player, team)` for starts with a suggested team (prompt 037),
    /// sorted by player.
    pub start_teams: Vec<(u8, u8)>,
    /// `(name, [x, y] meters)`, kept as loaded.
    pub markers: Vec<(String, [i32; 2])>,
    /// Other start layouts (prompt 182, `[[variant]]`).
    pub variants: Vec<VariantMeta>,
}

/// A start variant in the editor (prompt 182): like the standard layout
/// in [`Meta`], plus the supply points it uses (`None`: all).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantMeta {
    pub name: String,
    pub title: String,
    pub starts: Vec<(u8, [i32; 2])>,
    pub start_teams: Vec<(u8, u8)>,
    pub supply: Option<Vec<String>>,
}

impl Meta {
    /// The suggested team of `player`'s start, if any.
    pub fn start_team(&self, player: u8) -> Option<u8> {
        self.start_teams.iter().find(|s| s.0 == player).map(|s| s.1)
    }
}

/// Starts and teams of a start layout: `(player, [x, y] m)` and
/// `(player, team)`.
pub type StartsView<'a> = (&'a [(u8, [i32; 2])], &'a [(u8, u8)]);

/// Starts and teams of layout `active` (0 = standard, see
/// `EditMap::active`); a stale index edits the standard one.
type Layout<'a> = (&'a mut Vec<(u8, [i32; 2])>, &'a mut Vec<(u8, u8)>);

fn layout_mut(m: &mut Meta, active: usize) -> Layout<'_> {
    match active.checked_sub(1) {
        Some(k) if k < m.variants.len() => {
            let v = &mut m.variants[k];
            (&mut v.starts, &mut v.start_teams)
        }
        _ => (&mut m.starts, &mut m.start_teams),
    }
}

/// A rectangle of samples: columns `i0..i0 + w`, rows `j0..j0 + h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRect {
    pub i0: u32,
    pub j0: u32,
    pub w: u32,
    pub h: u32,
}

impl SampleRect {
    pub const fn i1(self) -> u32 {
        self.i0 + self.w
    }

    pub const fn j1(self) -> u32 {
        self.j0 + self.h
    }

    /// Smallest rectangle containing both.
    pub fn union(self, other: Self) -> Self {
        let i0 = self.i0.min(other.i0);
        let j0 = self.j0.min(other.j0);
        Self {
            i0,
            j0,
            w: self.i1().max(other.i1()) - i0,
            h: self.j1().max(other.j1()) - j0,
        }
    }

    pub const fn contains(self, other: Self) -> bool {
        other.i0 >= self.i0
            && other.j0 >= self.j0
            && other.i1() <= self.i1()
            && other.j1() <= self.j1()
    }

    pub const fn count(self) -> usize {
        self.w as usize * self.h as usize
    }
}

/// A map being edited.
pub struct EditMap {
    samples_x: u32,
    samples_y: u32,
    cell_mm: u32,
    /// Half width and depth in millimeters.
    half_x_mm: i32,
    half_y_mm: i32,
    /// Row-major, like `sim::Map`.
    heights: Vec<i32>,
    terrain: Vec<u8>,
    objects: Vec<PlacedObject>,
    meta: Meta,
    rules: ObjectRules,
    terrain_rules: TerrainRules,
    history: History,
    /// Seed of the object brush; every scatter dab draws from its own stream.
    seed: u64,
    dabs: u64,
    /// Incremented by every change (for caches of the presentation).
    version: u64,
    /// The start layout the start tools edit (prompt 182): 0 = standard,
    /// `k` = `meta.variants[k - 1]`. View state, not undone.
    active: usize,
}

impl EditMap {
    /// A flat grass map of `width_m` x `depth_m` meters (multiples of 4 m)
    /// at height 0 with start positions for players 0 and 1.
    pub fn new(name: &str, width_m: u32, depth_m: u32, biome: Biome) -> Result<Self, String> {
        check_name(name)?;
        for side in [width_m, depth_m] {
            if !(MIN_SIZE_M..=MAX_SIZE_M).contains(&side) || !side.is_multiple_of(4) {
                return Err(format!(
                    "map sides must be multiples of 4 m between {MIN_SIZE_M} and {MAX_SIZE_M} m, got {side} m"
                ));
            }
        }
        let samples_x = width_m * 1000 / CELL_MM + 1;
        let samples_y = depth_m * 1000 / CELL_MM + 1;
        let count = samples_x as usize * samples_y as usize;
        let d = i32::try_from(depth_m / 2)
            .unwrap_or(0)
            .saturating_sub(40)
            .min(60);
        let meta = Meta {
            name: name.into(),
            generator: Some("map editor".into()),
            biome,
            water_flow_deg: sim::map::DEFAULT_WATER_FLOW_DEG,
            weather: sim::WeatherSetup::default(),
            starts: vec![(0, [0, d]), (1, [0, -d])],
            start_teams: Vec::new(),
            markers: Vec::new(),
            variants: Vec::new(),
        };
        Self::from_parts(
            (samples_x, samples_y),
            CELL_MM,
            vec![0; count],
            vec![0; count],
            Vec::new(),
            meta,
        )
    }

    /// Loads a map from its files (the same checks as the simulation).
    pub fn from_files(files: &MapFiles) -> Result<Self, String> {
        let map = files.load()?;
        let generator = files
            .map_toml
            .parse::<toml::Table>()
            .ok()
            .and_then(|t| t.get("generator")?.as_str().map(String::from));
        let meters = |p: Vec2| [p.x.round().to_num::<i32>(), p.y.round().to_num::<i32>()];
        let mut starts: Vec<(u8, [i32; 2])> = map
            .starts()
            .iter()
            .map(|s| (s.player, meters(s.position)))
            .collect();
        starts.sort_by_key(|s| s.0);
        let mut start_teams: Vec<(u8, u8)> = map
            .starts()
            .iter()
            .filter_map(|s| Some((s.player, s.team?)))
            .collect();
        start_teams.sort_by_key(|s| s.0);
        let meta = Meta {
            name: map.name().into(),
            generator,
            biome: map.biome(),
            water_flow_deg: map.water_flow_deg(),
            weather: map.weather(),
            starts,
            start_teams,
            markers: map
                .markers()
                .iter()
                .map(|m| (m.name.clone(), meters(m.position)))
                .collect(),
            variants: map
                .variants()
                .iter()
                .map(|v| {
                    let mut starts: Vec<(u8, [i32; 2])> = v
                        .starts
                        .iter()
                        .map(|s| (s.player, meters(s.position)))
                        .collect();
                    starts.sort_by_key(|s| s.0);
                    let mut start_teams: Vec<(u8, u8)> = v
                        .starts
                        .iter()
                        .filter_map(|s| Some((s.player, s.team?)))
                        .collect();
                    start_teams.sort_by_key(|s| s.0);
                    VariantMeta {
                        name: v.name.clone(),
                        title: v.title.clone(),
                        starts,
                        start_teams,
                        supply: v.supply.clone(),
                    }
                })
                .collect(),
        };
        let objects = parse_objects(&files.objects_csv)?;
        Self::from_parts(
            map.samples(),
            map.cell_size_mm(),
            map.heights_mm().to_vec(),
            map.terrain_ids().to_vec(),
            objects,
            meta,
        )
    }

    fn from_parts(
        samples: (u32, u32),
        cell_mm: u32,
        heights: Vec<i32>,
        terrain: Vec<u8>,
        objects: Vec<PlacedObject>,
        meta: Meta,
    ) -> Result<Self, String> {
        let half = |n: u32| {
            i32::try_from(u64::from(n - 1) * u64::from(cell_mm) / 2)
                .map_err(|_| "map too large".to_string())
        };
        Ok(Self {
            samples_x: samples.0,
            samples_y: samples.1,
            cell_mm,
            half_x_mm: half(samples.0)?,
            half_y_mm: half(samples.1)?,
            heights,
            terrain,
            objects,
            meta,
            rules: ObjectRules::builtin().map_err(|e| e.to_string())?,
            terrain_rules: TerrainRules::builtin().map_err(|e| e.to_string())?,
            history: History::default(),
            seed: 0x0028_ed17_0000_0001,
            dabs: 0,
            version: 0,
            active: 0,
        })
    }

    /// Reads and loads `dir/` (see [`MapFiles::read_dir`]).
    pub fn load_dir(dir: &std::path::Path) -> Result<Self, String> {
        Self::from_files(&MapFiles::read_dir(dir)?)
    }

    /// Writes the map into `dir/` (created if needed). If the height range
    /// needs coarser steps than 1 mm, the edited heights take the saved,
    /// rounded values, so editor and file agree.
    pub fn save_dir(&mut self, dir: &std::path::Path) -> Result<(), String> {
        let files = self.files();
        files.write_dir(dir)?;
        let (base, step) = files::height_encoding(&self.heights);
        if step > 1 {
            for h in &mut self.heights {
                *h = files::quantize(*h, base, step);
            }
            self.version += 1;
        }
        Ok(())
    }

    /// The map in file form.
    pub fn files(&self) -> MapFiles {
        files::encode(self)
    }

    /// The map as the simulation would load it, for the presentation
    /// (objects are not checked for bridge alignment; [`validate`] does).
    pub fn to_sim_map(&self) -> Result<Map, String> {
        let point = |p: [i32; 2]| Vec2::from_meters(p[0], p[1]);
        let layout = MapLayout {
            name: self.meta.name.clone(),
            samples_x: self.samples_x,
            samples_y: self.samples_y,
            cell_size_mm: self.cell_mm,
            starts: self
                .meta
                .starts
                .iter()
                .map(|&(player, p)| StartPosition {
                    player,
                    position: point(p),
                    team: self.meta.start_team(player),
                })
                .collect(),
            markers: self
                .meta
                .markers
                .iter()
                .map(|(name, p)| Marker {
                    name: name.clone(),
                    position: point(*p),
                })
                .collect(),
        };
        let mut map = Map::from_rasters(layout, self.heights.clone(), self.terrain.clone())
            .map_err(|e| e.to_string())?;
        map.set_variants(
            self.meta
                .variants
                .iter()
                .map(|v| sim::map::MapVariant {
                    name: v.name.clone(),
                    title: v.title.clone(),
                    starts: v
                        .starts
                        .iter()
                        .map(|&(player, p)| StartPosition {
                            player,
                            position: point(p),
                            team: v.start_teams.iter().find(|t| t.0 == player).map(|t| t.1),
                        })
                        .collect(),
                    supply: v.supply.clone(),
                })
                .collect(),
        )
        .map_err(|e| e.to_string())?;
        let half = Vec2::from_millimeters(self.half_x_mm, self.half_y_mm);
        let list =
            ObjectList::parse(&self.objects_csv(), &self.rules, half).map_err(|e| e.to_string())?;
        map.set_objects(list);
        Ok(map)
    }

    /// The objects as `objects.csv`.
    pub fn objects_csv(&self) -> String {
        use std::fmt::Write;
        let mut csv = String::with_capacity(48 * self.objects.len() + 128);
        csv.push_str("# Saved by the map editor (docs/MAPS.md). Rules: data/objects.toml.\n");
        csv.push_str(CSV_HEADER);
        csv.push('\n');
        for o in &self.objects {
            let _ = writeln!(
                csv,
                "{},{},{},{},{}",
                o.kind, o.variant, o.x_mm, o.y_mm, o.heading_deg
            );
        }
        csv
    }

    pub const fn samples(&self) -> (u32, u32) {
        (self.samples_x, self.samples_y)
    }

    pub const fn cell_mm(&self) -> u32 {
        self.cell_mm
    }

    /// Width and depth in millimeters.
    pub const fn size_mm(&self) -> (i32, i32) {
        (self.half_x_mm * 2, self.half_y_mm * 2)
    }

    pub fn heights(&self) -> &[i32] {
        &self.heights
    }

    pub fn terrain(&self) -> &[u8] {
        &self.terrain
    }

    pub fn objects(&self) -> &[PlacedObject] {
        &self.objects
    }

    pub const fn meta(&self) -> &Meta {
        &self.meta
    }

    pub const fn object_rules(&self) -> &ObjectRules {
        &self.rules
    }

    pub const fn terrain_rules(&self) -> &TerrainRules {
        &self.terrain_rules
    }

    /// Changes with every edit, undo and redo.
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Seed of the object brush (the default is fixed, so scripts repeat).
    pub const fn set_seed(&mut self, seed: u64) {
        self.seed = seed;
        self.dabs = 0;
    }

    /// Height of sample `(i, j)` in millimeters (clamped to the grid).
    pub fn sample_height(&self, i: u32, j: u32) -> i32 {
        self.heights[self.index(i.min(self.samples_x - 1), j.min(self.samples_y - 1))]
    }

    /// Ground height at a point in millimeters: the nearest sample (enough
    /// for brush targets; the simulation interpolates).
    pub fn height_near(&self, x_mm: i32, y_mm: i32) -> i32 {
        let (i, j) = self.nearest_sample(x_mm, y_mm);
        self.sample_height(i, j)
    }

    /// Terrain id of the sample nearest to a point.
    pub fn terrain_near(&self, x_mm: i32, y_mm: i32) -> u8 {
        let (i, j) = self.nearest_sample(x_mm, y_mm);
        self.terrain[self.index(i, j)]
    }

    /// Whether a point lies on the map (edges included).
    pub const fn contains(&self, x_mm: i32, y_mm: i32) -> bool {
        x_mm.abs() <= self.half_x_mm && y_mm.abs() <= self.half_y_mm
    }

    fn nearest_sample(&self, x_mm: i32, y_mm: i32) -> (u32, u32) {
        let axis = |v: i32, half: i32, n: u32| {
            let g = (i64::from(v) + i64::from(half) + i64::from(self.cell_mm / 2))
                / i64::from(self.cell_mm);
            u32::try_from(g.clamp(0, i64::from(n - 1))).unwrap_or(0)
        };
        (
            axis(x_mm, self.half_x_mm, self.samples_x),
            axis(y_mm, self.half_y_mm, self.samples_y),
        )
    }

    const fn index(&self, i: u32, j: u32) -> usize {
        j as usize * self.samples_x as usize + i as usize
    }

    /// Position of sample `(i, j)` in millimeters.
    pub const fn sample_position(&self, i: u32, j: u32) -> (i32, i32) {
        (
            (i * self.cell_mm) as i32 - self.half_x_mm,
            (j * self.cell_mm) as i32 - self.half_y_mm,
        )
    }

    // --- Metadata -----------------------------------------------------

    /// Renames the map (also the folder name when saving).
    pub fn set_name(&mut self, name: &str) -> Result<(), String> {
        check_name(name)?;
        self.edit_meta(|m| m.name = name.into());
        Ok(())
    }

    pub fn set_biome(&mut self, biome: Biome) {
        self.edit_meta(|m| m.biome = biome);
    }

    /// Sets the default weather and time of day of matches (prompt 040).
    pub fn set_weather(&mut self, weather: sim::WeatherSetup) {
        self.edit_meta(|m| m.weather = weather);
    }

    /// Sets (or moves) the start position of `player`, rounded to whole
    /// meters and kept on the map.
    pub fn set_start(&mut self, player: u8, x_mm: i32, y_mm: i32) {
        let to_m = |v: i32, half: i32| {
            let v = v.clamp(-half, half);
            (v + v.signum() * 500) / 1000
        };
        let p = [to_m(x_mm, self.half_x_mm), to_m(y_mm, self.half_y_mm)];
        let active = self.active;
        self.edit_meta(|m| {
            let (starts, _) = layout_mut(m, active);
            starts.retain(|s| s.0 != player);
            starts.push((player, p));
            starts.sort_by_key(|s| s.0);
        });
    }

    /// The start layout the start tools edit: 0 = standard, `k` = the
    /// `k`-th variant (prompt 182).
    pub const fn active_variant(&self) -> usize {
        self.active
    }

    /// Chooses the start layout to edit; out of range: the standard one.
    pub fn select_variant(&mut self, k: usize) {
        self.active = if k <= self.meta.variants.len() { k } else { 0 };
        self.version += 1;
    }

    /// Starts and teams of the edited layout.
    pub fn active_starts(&self) -> StartsView<'_> {
        match self
            .active
            .checked_sub(1)
            .and_then(|k| self.meta.variants.get(k))
        {
            Some(v) => (&v.starts, &v.start_teams),
            None => (&self.meta.starts, &self.meta.start_teams),
        }
    }

    /// Adds a start variant `name` (lowercase letters, digits, `_`) with a
    /// copy of the standard starts and selects it; returns its index.
    pub fn add_variant(&mut self, name: &str, title: &str) -> Result<usize, String> {
        let ok = !name.is_empty()
            && name.len() <= 32
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && name != sim::map::STANDARD_VARIANT
            && name != sim::map::RANDOM_VARIANT;
        if !ok {
            return Err(format!("bad start variant name '{name}'"));
        }
        if self.meta.variants.iter().any(|v| v.name == name) {
            return Err(format!("start variant '{name}' exists"));
        }
        let v = VariantMeta {
            name: name.into(),
            title: title.into(),
            starts: self.meta.starts.clone(),
            start_teams: self.meta.start_teams.clone(),
            supply: None,
        };
        self.edit_meta(|m| m.variants.push(v));
        self.active = self.meta.variants.len();
        Ok(self.active)
    }

    /// Removes the edited start variant (not the standard layout).
    pub fn remove_variant(&mut self) {
        let Some(k) = self.active.checked_sub(1) else {
            return;
        };
        self.edit_meta(|m| {
            if k < m.variants.len() {
                m.variants.remove(k);
            }
        });
        self.active = 0;
    }

    /// Adds a strongpoint marker (prompt 052: `<prefix><n>`, a neutral trench
    /// strongpoint in matches) at the point, rounded to whole meters and
    /// kept on the map. Returns its name.
    pub fn add_strongpoint(&mut self, x_mm: i32, y_mm: i32) -> String {
        let prefix = sim::PositionRules::builtin()
            .map_or_else(|_| "strongpoint_".to_string(), |r| r.strongpoint_prefix);
        let to_m = |v: i32, half: i32| {
            let v = v.clamp(-half, half);
            (v + v.signum() * 500) / 1000
        };
        let p = [to_m(x_mm, self.half_x_mm), to_m(y_mm, self.half_y_mm)];
        let mut n = 1;
        while self
            .meta
            .markers
            .iter()
            .any(|(name, _)| *name == format!("{prefix}{n}"))
        {
            n += 1;
        }
        let name = format!("{prefix}{n}");
        let added = name.clone();
        self.edit_meta(|m| m.markers.push((added, p)));
        name
    }

    /// Removes the strongpoint markers within `radius_mm` of the point;
    /// returns how many.
    pub fn remove_strongpoints(&mut self, x_mm: i32, y_mm: i32, radius_mm: i32) -> usize {
        let prefix = sim::PositionRules::builtin()
            .map_or_else(|_| "strongpoint_".to_string(), |r| r.strongpoint_prefix);
        let near = |p: &[i32; 2]| {
            let dx = i64::from(p[0]) * 1000 - i64::from(x_mm);
            let dy = i64::from(p[1]) * 1000 - i64::from(y_mm);
            dx * dx + dy * dy <= i64::from(radius_mm) * i64::from(radius_mm)
        };
        let count = self
            .meta
            .markers
            .iter()
            .filter(|(name, p)| name.starts_with(&prefix) && near(p))
            .count();
        if count > 0 {
            self.edit_meta(|m| {
                m.markers
                    .retain(|(name, p)| !(name.starts_with(&prefix) && near(p)))
            });
        }
        count
    }

    /// Removes the start position of `player` (and its team) from the
    /// edited layout.
    pub fn remove_start(&mut self, player: u8) {
        let active = self.active;
        self.edit_meta(|m| {
            let (starts, teams) = layout_mut(m, active);
            starts.retain(|s| s.0 != player);
            teams.retain(|s| s.0 != player);
        });
    }

    /// Sets the suggested team of `player`'s start in the edited layout
    /// (prompt 037; `None`: no suggestion, the player is on a team of
    /// their own).
    pub fn set_start_team(&mut self, player: u8, team: Option<u8>) {
        let active = self.active;
        self.edit_meta(|m| {
            let (_, teams) = layout_mut(m, active);
            teams.retain(|s| s.0 != player);
            if let Some(team) = team {
                teams.push((player, team));
                teams.sort_by_key(|s| s.0);
            }
        });
    }

    fn edit_meta(&mut self, f: impl FnOnce(&mut Meta)) {
        self.edit(|map| {
            map.history.touch_meta(&map.meta);
            f(&mut map.meta);
        });
    }

    /// Runs `f` inside the open stroke, or as a stroke of its own.
    fn edit<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let own = !self.history.is_open();
        if own {
            self.begin_stroke();
        }
        let result = f(self);
        self.version += 1;
        if own {
            self.end_stroke();
        }
        result
    }
}

/// Map names are folder names: lowercase letters, digits, `_` and `-`.
pub fn check_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if ok {
        Ok(())
    } else {
        Err(format!(
            "map name '{name}': use 1 to 64 lowercase letters, digits, '_' or '-'"
        ))
    }
}

/// Reads the lines of `objects.csv` (already checked by `Map::add_objects`).
fn parse_objects(csv: &str) -> Result<Vec<PlacedObject>, String> {
    let mut objects = Vec::new();
    for line in csv.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line == CSV_HEADER {
            continue;
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        let [kind, variant, x, y, heading] = fields[..] else {
            return Err(format!("objects.csv: bad line '{line}'"));
        };
        let bad = || format!("objects.csv: bad line '{line}'");
        objects.push(PlacedObject {
            kind: kind.into(),
            variant: variant.into(),
            x_mm: x.parse().map_err(|_| bad())?,
            y_mm: y.parse().map_err(|_| bad())?,
            heading_deg: heading.parse().map_err(|_| bad())?,
        });
    }
    Ok(objects)
}

/// The biome with this `map.toml` name.
pub fn biome_from_name(name: &str) -> Option<Biome> {
    Biome::ALL.into_iter().find(|b| b.name() == name)
}
