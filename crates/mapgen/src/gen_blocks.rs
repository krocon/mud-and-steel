//! Apartment block quarter: `cli gen-blocks` (prompt 078).
//!
//! A small flat desert map to look at the apartment blocks: two east-west
//! streets and a north-south cross street, 16 m wide (four path cells: two
//! tanks pass each other), lined by eleven blocks (`env_block_desert`,
//! `env_block_desert_long`, a terrace of two `env_block_town`). Between the
//! rows lie courtyards with mud-brick walls, low houses and palms; gaps of
//! at least 12 m between the blocks lead into them. The blocks face the
//! streets with their entrance side (+X). No tests, AI tournaments or
//! benchmarks use this map. Writes `map.toml`, `height.png`, `terrain.png`
//! and `objects.csv` (format: `docs/MAPS.md`).
//!
//! Runs outside the simulation (floats allowed), deterministic: no
//! randomness is needed for the fixed layout except the courtyard props,
//! which use the simulation's random generator.

use crate::gen_map::{Random, write_png};
use sim::objects::CSV_HEADER;
use sim::{Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;
const GROUND: u8 = 0;
const DIRT_TRACK: u8 = 1;

/// The map is this wide and deep (m).
pub const SIZE_M: u32 = 400;
/// Half width of the streets (m).
pub const STREET_HALF: f64 = 8.0;
/// Center lines of the east-west streets (y) and the cross street (x = 0).
pub const STREETS_Y: [f64; 2] = [-44.0, 44.0];

/// Parameters of `cli gen-blocks`.
pub struct Options<'a> {
    pub name: &'a str,
    pub seed: u64,
    pub out: &'a Path,
}

/// Street at `(x, y)`: the two east-west streets and the cross street.
fn street(x: f64, y: f64) -> bool {
    let lim = f64::from(SIZE_M) / 2.0 - 10.0;
    STREETS_Y
        .iter()
        .any(|s| (y - s).abs() <= STREET_HALF && x.abs() <= lim)
        || (x.abs() <= STREET_HALF && y.abs() <= lim)
}

/// Heading that turns the entrance side (+X) of a block to `(dx, dy)`
/// (`MapObject::axes`: the right axis is `(cos h, sin h)`).
fn facing(dx: i32, dy: i32) -> u16 {
    match (dx, dy) {
        (1, 0) => 0,
        (0, 1) => 90,
        (-1, 0) => 180,
        _ => 270,
    }
}

/// The blocks: (catalog id, center x, y, facing).
pub const BLOCKS: [(&str, f64, f64, (i32, i32)); 11] = [
    // middle band, west: long blocks and blocks back to back
    ("env_block_desert_long", -40.0, -28.5, (0, -1)),
    ("env_block_desert_long", -40.0, 28.5, (0, 1)),
    ("env_block_desert", -92.0, -29.0, (0, -1)),
    ("env_block_desert", -92.0, 29.0, (0, 1)),
    // middle band, east: a terrace of two tenements, two blocks behind
    ("env_block_town", 28.5, -28.5, (0, -1)),
    ("env_block_town", 58.5, -28.5, (0, -1)),
    ("env_block_desert", 30.0, 29.0, (0, 1)),
    ("env_block_desert", 66.0, 29.0, (0, 1)),
    // outer rows
    ("env_block_desert_long", -40.0, -60.5, (0, 1)),
    ("env_block_desert", 40.0, -61.0, (0, 1)),
    ("env_block_desert_long", 40.0, 60.5, (0, -1)),
];

struct Quarter {
    random: Random,
    rows: Vec<String>,
    blocks: usize,
    other: usize,
}

impl Quarter {
    fn put(&mut self, kind: &str, variant: &str, (x, y): (f64, f64), heading: u16) {
        self.rows.push(format!(
            "{kind},{variant},{},{},{heading}",
            (x * 1000.0).round() as i64,
            (y * 1000.0).round() as i64
        ));
        if kind.starts_with("env_block") {
            self.blocks += 1;
        } else {
            self.other += 1;
        }
    }

    /// Mud-brick wall from `a` to `b` along x or y, in 3 m segments, with a
    /// gate gap of `gap` m in the middle.
    fn wall(&mut self, a: (f64, f64), b: (f64, f64), gap: f64) {
        let along_x = (b.1 - a.1).abs() < 1e-6;
        let len = if along_x { b.0 - a.0 } else { b.1 - a.1 };
        let n = (len.abs() / 3.0).floor() as i32;
        let step = len.signum() * 3.0;
        let mid = n as f64 / 2.0;
        for k in 0..n {
            let c = f64::from(k) + 0.5;
            if (c - mid).abs() * 3.0 < gap / 2.0 {
                continue;
            }
            let p = if along_x {
                (a.0 + step * c, a.1)
            } else {
                (a.0, a.1 + step * c)
            };
            let variant = if self.random.range(0.0, 1.0) < 0.15 {
                "Broken"
            } else {
                "Segment"
            };
            self.put("env_wall_mud", variant, p, if along_x { 90 } else { 0 });
        }
    }

    fn palms(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, count: usize) {
        let kinds = ["Palm", "PalmShort", "PalmLeaning"];
        for k in 0..count {
            let x = self.random.range(x0, x1);
            let y = self.random.range(y0, y1);
            let heading = self.random.range(0.0, 360.0) as u16;
            self.put("env_tree_palm", kinds[k % kinds.len()], (x, y), heading);
        }
    }
}

/// Writes the map; returns a report line.
pub fn generate(options: &Options) -> Result<String, String> {
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let size = f64::from(SIZE_M);
    let half = size / 2.0;
    let samples = (SIZE_M / 2 + 1) as usize;
    let mut q = Quarter {
        random: Random(Rng::new(options.seed ^ 0x626c_6f63)),
        rows: vec![
            "# Generated by `cli gen-blocks` (prompt 078, docs/MAPS.md). Rules: data/objects.toml."
                .into(),
            CSV_HEADER.into(),
        ],
        blocks: 0,
        other: 0,
    };
    let mut heights = Vec::with_capacity(samples * samples);
    let mut terrain = Vec::with_capacity(samples * samples);
    for j in 0..samples {
        for i in 0..samples {
            let x = i as f64 * CELL_M - half;
            let y = j as f64 * CELL_M - half;
            // flat quarter, the land rising gently outside
            let out = ((x.abs() - 120.0).max(0.0) + (y.abs() - 110.0).max(0.0)) / 80.0;
            heights.push(2.0 * out.min(1.0) + 0.002 * y);
            terrain.push(if street(x, y) { DIRT_TRACK } else { GROUND });
        }
    }
    for (kind, x, y, (dx, dy)) in BLOCKS {
        q.put(kind, "Intact", (x, y), facing(dx, dy));
    }
    // Courtyards of the middle band: walls between the block rows with
    // gates, low houses and palms.
    q.wall((-111.0, -18.0), (-21.0, -18.0), 6.0);
    q.wall((-111.0, 18.0), (-21.0, 18.0), 6.0);
    q.wall((-111.0, -15.0), (-111.0, 15.0), 6.0);
    q.put("env_house_desert_two", "Intact", (-66.0, 0.0), 0);
    q.put("env_house_desert_hut", "Intact", (-95.0, 6.0), 180);
    q.put("env_well_desert", "Frame", (-40.0, 2.0), 0);
    q.palms(-105.0, -12.0, -25.0, 12.0, 9);
    q.wall((14.0, 14.0), (80.0, 14.0), 6.0);
    q.put("env_house_desert_long", "Intact", (44.0, 0.0), 90);
    q.palms(15.0, -16.0, 78.0, -8.0, 4);
    q.palms(15.0, 6.0, 78.0, 11.0, 4);
    // Palms along the cross street.
    for k in 0..6 {
        let y = -150.0 + f64::from(k) * 60.0 + 15.0;
        q.put("env_tree_palm", "Palm", (-11.5, y), 0);
        q.put("env_tree_palm", "PalmShort", (11.5, y + 20.0), 0);
    }

    let low = heights.iter().copied().fold(f64::MAX, f64::min);
    let high = heights.iter().copied().fold(f64::MIN, f64::max);
    let base_mm = ((low - 1.0).floor() * 1000.0) as i32;
    let encoded: Vec<u8> = heights
        .iter()
        .flat_map(|h| {
            let value = (h * 1000.0 - f64::from(base_mm)).round();
            (value.clamp(0.0, 65535.0) as u16).to_be_bytes()
        })
        .collect();
    std::fs::create_dir_all(options.out).map_err(|e| e.to_string())?;
    let side = samples as u32;
    write_png(&options.out.join("height.png"), side, &encoded, None)?;
    let palette: Vec<u8> = rules
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
    write_png(
        &options.out.join("terrain.png"),
        side,
        &terrain,
        Some(palette),
    )?;
    let mut csv = q.rows.join("\n");
    csv.push('\n');
    std::fs::write(options.out.join("objects.csv"), csv).map_err(|e| e.to_string())?;
    let mut toml = String::new();
    let _ = write!(
        toml,
        "# Generated by `cli gen-blocks` (prompt 078); edit the PNGs freely (docs/MAPS.md).\n\
         name = \"{}\"\n\
         generator = \"cli gen-blocks --name {} --seed {}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = 1\n\
         biome = \"desert\"\n\
         water_flow_deg = 180\n\n\
         [[start]]\nplayer = 0\nposition_m = [0, 165]\n\n\
         [[start]]\nplayer = 1\nposition_m = [0, -165]\n\n\
         [[marker]]\nname = \"blocks\"\nposition_m = [-10, 0]\n\n\
         [[marker]]\nname = \"town_square\"\nposition_m = [0, 44]\n",
        options.name, options.name, options.seed,
    );
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;
    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, {} objects ({} blocks)",
        options.out.display(),
        q.blocks + q.other,
        q.blocks,
    ))
}

#[cfg(test)]
mod tests {
    use super::{BLOCKS, Options, STREETS_Y, generate};
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Map, Scale, TerrainRules, Vec2, unit_types_from_catalog};
    use std::collections::VecDeque;

    /// The quarter loads with its blocks, and from the cross street tanks
    /// reach every street and courtyard and infantry every courtyard.
    #[test]
    fn block_quarter_loads_and_every_street_and_courtyard_is_reachable() {
        let dir = std::env::temp_dir().join(format!("mas-gen-blocks-{}", std::process::id()));
        let options = Options {
            name: "city_blocks",
            seed: 78,
            out: &dir,
        };
        let report = generate(&options).unwrap();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let blocks = map
            .objects()
            .iter()
            .filter(|o| {
                map.object_kinds()[usize::from(o.kind)]
                    .id
                    .starts_with("env_block")
            })
            .count();
        assert_eq!(blocks, BLOCKS.len(), "{report}");
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types =
            unit_types_from_catalog(&catalog, &scale, &["de_inf_rifle_squad", "de_pz4_h"]).unwrap();
        let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
        let reach = |class: SizeClass| {
            let (sx, sy) = grid.cell_of(Vec2::from_meters(0, 0));
            let (mx, my) = grid.cell_of(Vec2::from_meters(199, 199));
            let mut seen = std::collections::BTreeSet::new();
            let mut todo = VecDeque::from([(sx, sy)]);
            seen.insert((sx, sy));
            while let Some((x, y)) = todo.pop_front() {
                for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 {
                        continue;
                    }
                    let c = (nx as u32, ny as u32);
                    if !seen.contains(&c)
                        && c.0 <= mx
                        && c.1 <= my
                        && grid.passable(class, c.0, c.1)
                    {
                        seen.insert(c);
                        todo.push_back(c);
                    }
                }
            }
            seen
        };
        let tanks = reach(SizeClass::Large);
        let infantry = reach(SizeClass::Small);
        let at = |x: i32, y: i32| grid.cell_of(Vec2::from_meters(x, y));
        // streets: both ends of both east-west streets
        for y in STREETS_Y {
            for x in [-150, 150] {
                assert!(tanks.contains(&at(x, y as i32)), "street ({x}, {y})");
            }
        }
        // courtyards and the outer sides
        for (x, y) in [
            (-40, 10),
            (-80, -10),
            (-120, 0),
            (44, 8),
            (44, -12),
            (-40, -80),
            (40, 85),
        ] {
            assert!(
                infantry.contains(&at(x, y)),
                "courtyard ({x}, {y}) for infantry"
            );
        }
        for (x, y) in [(-40, 10), (-120, 0), (44, -12), (-40, -80), (40, 85)] {
            assert!(tanks.contains(&at(x, y)), "courtyard ({x}, {y}) for tanks");
        }
    }
}
