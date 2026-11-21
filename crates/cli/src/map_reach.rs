//! Reachability of a map (prompt 081): which ground vehicles and infantry
//! can use, which parts hang together, and where the starts and markers
//! lie in them. Uses the simulation's path grid (`sim::path::PathGrid`), so
//! the numbers are the rules a match plays by: grade limit, margins at
//! water, blocking objects.

use sim::path::{PathGrid, SizeClass};
use sim::{Catalog, Map, Scale, TerrainRules, Vec2};
use std::fmt::Write as _;
use std::path::Path;

/// One tank and one rifle squad: the two size classes.
const TYPES: [&str; 2] = ["de_pz4_h", "de_inf_rifle_squad"];

/// A marker counts as reached when a passable cell lies this close (m),
/// like the supply point check of `map-check`.
const NEAR_M: i32 = 20;

/// Kind of a path cell for the report and the picture.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Water,
    Vehicles,
    Infantry,
    Objects,
    Nobody,
}

struct Components {
    label: Vec<u32>,
    sizes: Vec<u32>,
}

/// 4-neighbor components of the passable cells (labels from 1).
fn components(width: u32, height: u32, open: &[bool]) -> Components {
    let mut label = vec![0u32; open.len()];
    let mut sizes = vec![0u32];
    let mut stack = Vec::new();
    for start in 0..open.len() {
        if !open[start] || label[start] != 0 {
            continue;
        }
        let id = u32::try_from(sizes.len()).unwrap_or(u32::MAX);
        let mut size = 0u32;
        label[start] = id;
        stack.push(start);
        while let Some(k) = stack.pop() {
            size += 1;
            let (x, y) = (k as u32 % width, k as u32 / width);
            let mut visit = |nx: u32, ny: u32| {
                let n = (ny * width + nx) as usize;
                if open[n] && label[n] == 0 {
                    label[n] = id;
                    stack.push(n);
                }
            };
            if x > 0 {
                visit(x - 1, y);
            }
            if x + 1 < width {
                visit(x + 1, y);
            }
            if y > 0 {
                visit(x, y - 1);
            }
            if y + 1 < height {
                visit(x, y + 1);
            }
        }
        sizes.push(size);
    }
    Components { label, sizes }
}

/// Component of the nearest open cell within `NEAR_M` of `p` (0 = none).
fn component_near(grid: &PathGrid, c: &Components, width: u32, height: u32, p: Vec2) -> u32 {
    let (cx, cy) = grid.cell_of(p);
    let reach = NEAR_M / 4;
    let mut best = (i64::MAX, 0u32);
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (x, y) = (i64::from(cx) + i64::from(dx), i64::from(cy) + i64::from(dy));
            if x < 0 || y < 0 || x >= i64::from(width) || y >= i64::from(height) {
                continue;
            }
            let d = i64::from(dx * dx + dy * dy);
            if d > i64::from(reach * reach) {
                continue;
            }
            let id = c.label[(y * i64::from(width) + x) as usize];
            if id != 0 && d < best.0 {
                best = (d, id);
            }
        }
    }
    best.1
}

pub fn run(dir: &Path, map: &Map, png: Option<&Path>) -> Result<String, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let types =
        sim::unit_types_from_catalog(&catalog, &scale, &TYPES).map_err(|e| e.to_string())?;
    let grid = PathGrid::new(map, &rules, &types);
    let (sx, sy) = map.samples();
    let stride = (4000 / map.cell_size_mm()).max(1);
    let (width, height) = ((sx / stride).max(1), (sy / stride).max(1));
    let count = (width * height) as usize;

    let mut kinds = vec![Kind::Nobody; count];
    let mut road = vec![false; count];
    let mut tally = [0u64; 5];
    for y in 0..height {
        for x in 0..width {
            let k = (y * width + x) as usize;
            let terrain = map.sample_terrain(x * stride, y * stride);
            let water = rules
                .types
                .get(usize::from(terrain))
                .is_some_and(|t| !t.passable)
                && grid.deck_of(x, y).is_none();
            road[k] = terrain == 2 || terrain == 1;
            kinds[k] = if water {
                Kind::Water
            } else if grid.passable(SizeClass::Large, x, y) {
                Kind::Vehicles
            } else if grid.passable(SizeClass::Small, x, y) {
                Kind::Infantry
            } else if grid.object_blocked(SizeClass::Small, x, y) {
                Kind::Objects
            } else {
                Kind::Nobody
            };
            tally[kinds[k] as usize] += 1;
        }
    }
    let land = (count as u64 - tally[Kind::Water as usize]).max(1);
    let pct = |n: u64| n as f64 * 100.0 / land as f64;
    let km2 = |n: u64| n as f64 * 16.0 / 1e6;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "map '{}' ({}): {} x {} path cells of 4 m",
        map.name(),
        dir.display(),
        width,
        height
    );
    let _ = writeln!(
        out,
        "  land {:.2} km² (water {:.2} km²)",
        km2(land),
        km2(tally[Kind::Water as usize])
    );
    for (name, kind) in [
        ("vehicles and infantry", Kind::Vehicles),
        ("infantry only", Kind::Infantry),
        ("nobody: too steep, edges", Kind::Nobody),
        ("nobody: buildings and objects", Kind::Objects),
    ] {
        let n = tally[kind as usize];
        let _ = writeln!(
            out,
            "  {name:<32} {:5.1} % of the land ({:.2} km²)",
            pct(n),
            km2(n)
        );
    }

    let open_large: Vec<bool> = kinds.iter().map(|k| *k == Kind::Vehicles).collect();
    let open_small: Vec<bool> = kinds
        .iter()
        .map(|k| matches!(k, Kind::Vehicles | Kind::Infantry))
        .collect();
    let large = components(width, height, &open_large);
    let small = components(width, height, &open_small);
    for (name, c) in [("vehicles", &large), ("infantry", &small)] {
        let mut sizes: Vec<(u32, usize)> = c
            .sizes
            .iter()
            .enumerate()
            .skip(1)
            .map(|(id, &s)| (s, id))
            .collect();
        sizes.sort_unstable_by(|a, b| b.cmp(a));
        let open: u64 = sizes.iter().map(|(s, _)| u64::from(*s)).sum();
        let largest = sizes.first().map_or(0, |s| s.0);
        let _ = writeln!(
            out,
            "  {name}: {} components; the largest holds {:.1} % of their ground ({:.2} km²), the next {}",
            sizes.len(),
            f64::from(largest) * 100.0 / open.max(1) as f64,
            km2(u64::from(largest)),
            sizes
                .iter()
                .skip(1)
                .take(4)
                .map(|(s, _)| format!("{:.3} km²", km2(u64::from(*s))))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // Prompt 182: every start layout, the standard one first, with the
    // supply points it uses (`None`: all).
    let mut layouts: Vec<(String, Vec<sim::map::StartPosition>, Option<Vec<String>>)> =
        vec![(String::new(), map.starts().to_vec(), None)];
    layouts.extend(
        map.variants()
            .iter()
            .map(|v| (v.name.clone(), v.starts.clone(), v.supply.clone())),
    );
    let start_name = |layout: &str, player: u8| {
        if layout.is_empty() {
            format!("start {player}")
        } else {
            format!("start {player} ({layout})")
        }
    };
    // Starts and markers: the component they lie in, per class.
    let mut points: Vec<(String, Vec2)> = layouts
        .iter()
        .flat_map(|(name, starts, _)| {
            starts
                .iter()
                .map(|s| (start_name(name, s.player), s.position))
                .collect::<Vec<_>>()
        })
        .collect();
    points.extend(map.markers().iter().map(|m| (m.name.clone(), m.position)));
    let main_of = |c: &Components| {
        c.sizes
            .iter()
            .enumerate()
            .skip(1)
            .max_by_key(|(id, s)| (**s, std::cmp::Reverse(*id)))
            .map_or(0, |(id, _)| id as u32)
    };
    let (main_large, main_small) = (main_of(&large), main_of(&small));
    let _ = writeln!(
        out,
        "  {:<28} {:>18} {:>18}",
        "point", "vehicles", "infantry"
    );
    let describe = |id: u32, main: u32| match id {
        0 => "none within 20 m".to_string(),
        id if id == main => "main".to_string(),
        id => format!("separate #{id}"),
    };
    for (name, p) in &points {
        let a = component_near(&grid, &large, width, height, *p);
        let b = component_near(&grid, &small, width, height, *p);
        let _ = writeln!(
            out,
            "  {name:<28} {:>18} {:>18}",
            describe(a, main_large),
            describe(b, main_small)
        );
    }

    // Path lengths from every start to every supply point (balance): the
    // smoothed path of a tank and of a rifle squad; "(x m short)" when the
    // nearest reachable cell is that far from the point.
    for (layout, starts, only) in &layouts {
        let supplies: Vec<&(String, Vec2)> = points
            .iter()
            .filter(|(n, _)| n.starts_with("supply"))
            .filter(|(n, _)| only.as_ref().is_none_or(|names| names.contains(n)))
            .collect();
        if !supplies.is_empty() && !starts.is_empty() {
            let _ = writeln!(
                out,
                "  path length start -> supply point (m), vehicles / infantry{}:",
                if layout.is_empty() {
                    String::new()
                } else {
                    format!(", start variant '{layout}'")
                }
            );
            let mut header = format!("  {:<24}", "");
            for s in starts {
                let _ = write!(header, " {:>22}", format!("start {}", s.player));
            }
            let _ = writeln!(out, "{header}");
            for (name, p) in &supplies {
                let mut line = format!("  {name:<24}");
                for s in starts {
                    let mut cell = String::new();
                    for profile in 0..2 {
                        let text = match grid.find_path(s.position, *p, profile) {
                            Some(path) => {
                                let short = (path.goal - *p).length().to_num::<f64>();
                                let length = path.length().to_num::<f64>();
                                if short > f64::from(NEAR_M) {
                                    format!("({length:.0}, {short:.0} short)")
                                } else {
                                    format!("{length:.0}")
                                }
                            }
                            None => "-".to_string(),
                        };
                        let _ = write!(cell, "{}{text}", if profile == 0 { "" } else { " / " });
                    }
                    let _ = write!(line, " {cell:>22}");
                }
                let _ = writeln!(out, "{line}");
            }
        }
    }

    let heavy_open = crushers(
        map, &rules, &catalog, &scale, &kinds, &points, width, height, &mut out,
    )?;

    if let Some(path) = png {
        let mut rgb = vec![0u8; count * 3];
        for (k, kind) in kinds.iter().enumerate() {
            let color: [u8; 3] = match kind {
                Kind::Water => [40, 70, 110],
                Kind::Vehicles if road[k] => [60, 130, 60],
                Kind::Vehicles => [140, 200, 110],
                _ if heavy_open[k] => [200, 60, 200],
                Kind::Infantry => [235, 190, 60],
                Kind::Objects => [70, 60, 60],
                Kind::Nobody => [200, 60, 50],
            };
            rgb[k * 3..k * 3 + 3].copy_from_slice(&color);
        }
        for (_, p) in &points {
            let (cx, cy) = grid.cell_of(*p);
            for dy in -2i64..=2 {
                for dx in -2i64..=2 {
                    let (x, y) = (i64::from(cx) + dx, i64::from(cy) + dy);
                    if x >= 0 && y >= 0 && x < i64::from(width) && y < i64::from(height) {
                        let k = (y * i64::from(width) + x) as usize;
                        let edge = dx.abs() == 2 || dy.abs() == 2;
                        let c = if edge { [0, 0, 0] } else { [255, 255, 255] };
                        rgb[k * 3..k * 3 + 3].copy_from_slice(&c);
                    }
                }
            }
        }
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgb).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        let _ = writeln!(
            out,
            "  picture {} (light green: vehicles, dark green: their roads and tracks, yellow: infantry only, red: nobody, dark: buildings and objects, blue: water, magenta: open only to a heavy tank that breaks walls and trees, white: starts and markers)",
            path.display()
        );
    }
    Ok(out)
}

/// Medium and heavy tank for [`crushers`]: the first fells thin trees and
/// breaks walls, the second also thick trees (`data/destruction.toml`).
const CRUSHERS: [&str; 2] = ["de_pz4_h", "de_tiger1_e"];

/// Vehicle ground of the tanks that break walls and push trees over
/// (prompts 093 and 106): open cells for each, how much more than without
/// breaking anything, through how many cells with crushable objects, the
/// largest connected part and whether the starts and markers lie in it.
/// Returns the open cells of the heaviest tank.
#[allow(clippy::too_many_arguments)]
fn crushers(
    map: &Map,
    rules: &TerrainRules,
    catalog: &Catalog,
    scale: &Scale,
    kinds: &[Kind],
    points: &[(String, Vec2)],
    width: u32,
    height: u32,
    out: &mut String,
) -> Result<Vec<bool>, String> {
    let destruction = sim::DestructionRules::builtin().map_err(|e| e.to_string())?;
    let crushing = destruction.crushing(map);
    let types =
        sim::unit_types_from_catalog(catalog, scale, &CRUSHERS).map_err(|e| e.to_string())?;
    let grid = PathGrid::with_crushing(map, rules, &types, &crushing);
    let plain = kinds.iter().filter(|k| **k == Kind::Vehicles).count();
    let land = kinds.iter().filter(|k| **k != Kind::Water).count().max(1);
    let _ = writeln!(
        out,
        "  breaking walls and trees ({} objects in dense stands stay closed):",
        crushing.never.len()
    );
    let mut heaviest = Vec::new();
    for (profile, ty) in types.iter().enumerate() {
        let open: Vec<bool> = grid.costs(profile).iter().map(|&c| c > 0).collect();
        let through = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| open[(y * width + x) as usize] && grid.crushable_by(ty.mass_kg, x, y))
            .count();
        let n = open.iter().filter(|o| **o).count();
        let c = components(width, height, &open);
        let (main, largest) = c
            .sizes
            .iter()
            .enumerate()
            .skip(1)
            .max_by_key(|(id, s)| (**s, std::cmp::Reverse(*id)))
            .map_or((0, 0), |(id, &s)| (id as u32, s));
        let outside: Vec<&str> = points
            .iter()
            .filter(|(_, p)| component_near(&grid, &c, width, height, *p) != main)
            .map(|(name, _)| name.as_str())
            .collect();
        let _ = writeln!(
            out,
            "    {:<14} {:>4.1} t: {:5.1} % of the land ({:+.1} points), {} cells through objects; largest part {:.1} %{}",
            ty.name,
            f64::from(ty.mass_kg) / 1000.0,
            n as f64 * 100.0 / land as f64,
            (n as f64 - plain as f64) * 100.0 / land as f64,
            through,
            f64::from(largest) * 100.0 / n.max(1) as f64,
            if outside.is_empty() {
                String::new()
            } else {
                format!("; outside it: {}", outside.join(", "))
            }
        );
        heaviest = open;
    }
    Ok(heaviest)
}

#[cfg(test)]
mod tests {
    use super::TYPES;
    use sim::path::PathGrid;
    use sim::{Catalog, Map, Scale, TerrainRules};
    use std::path::PathBuf;

    fn harbor_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/harbor_city")
    }

    /// The map from real geodata (prompt 081): loads, survives the map
    /// editor, both sides reach each other and every supply point
    /// (infantry everywhere, vehicles at least half of them; here all), and
    /// no real place name got into its files. That it regenerates
    /// identically is checked by `tools/geo_import.py check` (`just check`).
    #[test]
    fn harbor_city_plays_and_carries_no_real_names() {
        let dir = harbor_dir();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let csv = String::from_utf8(read("objects.csv")).unwrap();
        for text in [&toml, &csv] {
            let lower = text.to_lowercase();
            for name in [
                "monaco",
                "monte",
                "carlo",
                "fontvieille",
                "condamine",
                "larvotto",
                "beausoleil",
                "turbie",
                "roquebrune",
                "hercule",
                "grimaldi",
            ] {
                assert!(!lower.contains(name), "real name '{name}' in the map files");
            }
        }
        let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&csv).unwrap();

        // Opened and saved by the map editor, the map stays the same map.
        let saved = std::env::temp_dir().join(format!("mas-harbor-{}", std::process::id()));
        let mut edit = editor::EditMap::load_dir(&dir).unwrap();
        edit.save_dir(&saved).unwrap();
        let reread = |f: &str| std::fs::read(saved.join(f)).unwrap();
        let mut again = Map::load(
            &String::from_utf8(reread("map.toml")).unwrap(),
            &reread("height.png"),
            &reread("terrain.png"),
        )
        .unwrap();
        again
            .add_objects(&String::from_utf8(reread("objects.csv")).unwrap())
            .unwrap();
        assert_eq!(
            again.reference().hash,
            map.reference().hash,
            "editor round trip"
        );
        let _ = std::fs::remove_dir_all(&saved);

        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let rules = TerrainRules::builtin().unwrap();
        let types = sim::unit_types_from_catalog(&catalog, &scale, &TYPES).unwrap();
        let grid = PathGrid::new(&map, &rules, &types);
        let reaches = |from: sim::Vec2, to: sim::Vec2, profile: usize| {
            grid.find_path(from, to, profile)
                .is_some_and(|p| (p.goal - to).length() <= sim::Fixed::from_num(20))
        };
        assert_eq!(map.starts().len(), 4);
        let west = map.starts()[0].position;
        let east = map.starts()[1].position;
        assert!(reaches(west, east, 1), "infantry from start to start");
        assert!(reaches(west, east, 0), "tanks from start to start");
        let supplies: Vec<_> = map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with("supply"))
            .collect();
        assert!(supplies.len() >= 4);
        let mut by_vehicle = 0;
        for m in &supplies {
            assert!(reaches(west, m.position, 1), "infantry to {}", m.name);
            assert!(reaches(east, m.position, 1), "infantry to {}", m.name);
            by_vehicle += usize::from(reaches(west, m.position, 0) && reaches(east, m.position, 0));
        }
        assert!(
            2 * by_vehicle >= supplies.len(),
            "vehicles reach {by_vehicle} supply points"
        );
    }
}
