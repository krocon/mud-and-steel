//! The four files of a map folder: encoding, reading and writing.

use crate::EditMap;
use sim::Map;
use std::fmt::Write;
use std::path::Path;

/// Contents of a map folder (`docs/MAPS.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapFiles {
    pub map_toml: String,
    pub height_png: Vec<u8>,
    pub terrain_png: Vec<u8>,
    /// Header only if the map has no objects.
    pub objects_csv: String,
}

impl MapFiles {
    /// Reads `dir/map.toml`, `dir/height.png`, `dir/terrain.png` and, if
    /// present, `dir/objects.csv`.
    pub fn read_dir(dir: &Path) -> Result<Self, String> {
        let read = |file: &str| {
            let path = dir.join(file);
            std::fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
        };
        let text = |file: &str| String::from_utf8(read(file)?).map_err(|e| format!("{file}: {e}"));
        let objects_csv = if dir.join("objects.csv").exists() {
            text("objects.csv")?
        } else {
            format!("{}\n", sim::objects::CSV_HEADER)
        };
        Ok(Self {
            map_toml: text("map.toml")?,
            height_png: read("height.png")?,
            terrain_png: read("terrain.png")?,
            objects_csv,
        })
    }

    /// Writes all four files into `dir` (created if needed).
    pub fn write_dir(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        for (file, bytes) in [
            ("map.toml", self.map_toml.as_bytes()),
            ("height.png", &self.height_png[..]),
            ("terrain.png", &self.terrain_png[..]),
            ("objects.csv", self.objects_csv.as_bytes()),
        ] {
            let path = dir.join(file);
            std::fs::write(&path, bytes)
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        }
        Ok(())
    }

    /// Loads the map exactly like the simulation's callers do
    /// (`Map::load`, then `Map::add_objects`).
    pub fn load(&self) -> Result<Map, String> {
        let mut map = Map::load(&self.map_toml, &self.height_png, &self.terrain_png)
            .map_err(|e| e.to_string())?;
        map.add_objects(&self.objects_csv)
            .map_err(|e| e.to_string())?;
        Ok(map)
    }
}

/// `(height_base_mm, height_step_mm)` for these heights: the base one meter
/// below the lowest point (whole meters), the smallest step that covers the
/// range with 65 536 values (like `cli gen-map`).
pub fn height_encoding(heights: &[i32]) -> (i32, u32) {
    let low = heights.iter().copied().min().unwrap_or(0);
    let high = heights.iter().copied().max().unwrap_or(0);
    let base = low.div_euclid(1000) * 1000 - 1000;
    let range = i64::from(high) - i64::from(base);
    let step = u32::try_from((range + 65_534) / 65_535)
        .unwrap_or(u32::MAX)
        .max(1);
    (base, step)
}

/// The height a sample has after saving with `base` and `step`.
pub fn quantize(height: i32, base: i32, step: u32) -> i32 {
    let base64 = i64::from(base);
    base64
        .saturating_add(encoded(height, base, step) * i64::from(step))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn encoded(height: i32, base: i32, step: u32) -> i64 {
    let step = i64::from(step);
    ((i64::from(height) - i64::from(base) + step / 2) / step).clamp(0, 65_535)
}

pub fn encode(map: &EditMap) -> MapFiles {
    let (sx, sy) = map.samples();
    let (base, step) = height_encoding(map.heights());
    let height_data: Vec<u8> = map
        .heights()
        .iter()
        .flat_map(|&h| (encoded(h, base, step) as u16).to_be_bytes())
        .collect();
    let palette: Vec<u8> = map
        .terrain_rules()
        .types
        .iter()
        .flat_map(|t| {
            [
                (t.debug_color >> 16) as u8,
                (t.debug_color >> 8) as u8,
                t.debug_color as u8,
            ]
        })
        .collect();
    MapFiles {
        map_toml: map_toml(map, base, step),
        height_png: png_bytes(sx, sy, &height_data, None),
        terrain_png: png_bytes(sx, sy, map.terrain(), Some(palette)),
        objects_csv: map.objects_csv(),
    }
}

fn quoted(s: &str) -> String {
    toml::Value::String(s.into()).to_string()
}

fn map_toml(map: &EditMap, base: i32, step: u32) -> String {
    let meta = map.meta();
    let (sx, sy) = map.samples();
    let mut t = String::from("# Saved by the map editor (prompt 028); format: docs/MAPS.md.\n");
    let _ = writeln!(t, "name = {}", quoted(&meta.name));
    if let Some(generator) = &meta.generator {
        let _ = writeln!(t, "generator = {}", quoted(generator));
    }
    let _ = writeln!(t, "cell_size_mm = {}", map.cell_mm());
    let _ = writeln!(t, "samples = [{sx}, {sy}]");
    let _ = writeln!(t, "height_base_mm = {base}");
    let _ = writeln!(t, "height_step_mm = {step}");
    let _ = writeln!(t, "biome = {}", quoted(meta.biome.name()));
    // Prompt 122: only maps with a look of their own name one.
    if meta.look != sim::map::Look::Plain {
        let _ = writeln!(t, "look = {}", quoted(meta.look.name()));
    }
    let _ = writeln!(t, "water_flow_deg = {}", meta.water_flow_deg);
    // Prompt 040: written only when not clear by day (the default).
    if meta.weather.weather != sim::Weather::default() {
        let _ = writeln!(t, "weather = {}", quoted(meta.weather.weather.name()));
    }
    if meta.weather.daytime != sim::Daytime::default() {
        let _ = writeln!(t, "daytime = {}", quoted(meta.weather.daytime.name()));
    }
    for (player, p) in &meta.starts {
        let team = meta
            .start_team(*player)
            .map_or(String::new(), |team| format!("team = {team}\n"));
        let _ = write!(
            t,
            "\n[[start]]\nplayer = {player}\n{team}position_m = [{}, {}]\n",
            p[0], p[1]
        );
    }
    for (name, p) in &meta.markers {
        let _ = write!(
            t,
            "\n[[marker]]\nname = {}\nposition_m = [{}, {}]\n",
            quoted(name),
            p[0],
            p[1]
        );
    }
    // Prompt 182: other start layouts.
    for v in &meta.variants {
        let _ = write!(t, "\n[[variant]]\nname = {}\n", quoted(&v.name));
        if !v.title.is_empty() && v.title != v.name {
            let _ = writeln!(t, "title = {}", quoted(&v.title));
        }
        if let Some(supply) = &v.supply {
            let names: Vec<String> = supply.iter().map(|n| quoted(n)).collect();
            let _ = writeln!(t, "supply = [{}]", names.join(", "));
        }
        for (player, p) in &v.starts {
            let team = v
                .start_teams
                .iter()
                .find(|s| s.0 == *player)
                .map_or(String::new(), |s| format!("team = {}\n", s.1));
            let _ = write!(
                t,
                "\n[[variant.start]]\nplayer = {player}\n{team}position_m = [{}, {}]\n",
                p[0], p[1]
            );
        }
    }
    t
}

/// A 16-bit grayscale PNG, or an 8-bit indexed one with `palette`.
fn png_bytes(width: u32, height: u32, data: &[u8], palette: Option<Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        match palette {
            Some(palette) => {
                encoder.set_color(png::ColorType::Indexed);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.set_palette(palette);
            }
            None => {
                encoder.set_color(png::ColorType::Grayscale);
                encoder.set_depth(png::BitDepth::Sixteen);
            }
        }
        encoder.set_compression(png::Compression::High);
        // Writing into memory cannot fail; the sizes match by construction.
        let mut writer = encoder.write_header().expect("PNG header");
        writer.write_image_data(data).expect("PNG data");
        writer.finish().expect("PNG end");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_covers_the_range() {
        assert_eq!(height_encoding(&[0, 500]), (-1000, 1));
        assert_eq!(height_encoding(&[-1500, 2000]), (-3000, 1));
        let (base, step) = height_encoding(&[0, 100_000]);
        assert_eq!((base, step), (-1000, 2));
        assert_eq!(quantize(100_000, base, step), 100_000);
        assert_eq!(quantize(3, base, step), 4);
        assert!(i64::from(base) + 65_535 * i64::from(step) >= 100_000);
    }
}
