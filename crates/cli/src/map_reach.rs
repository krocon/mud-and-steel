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

    // Starts and markers: the component they lie in, per class.
    let mut points: Vec<(String, Vec2)> = map
        .starts()
        .iter()
        .map(|s| (format!("start {}", s.player), s.position))
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
    let supplies: Vec<&(String, Vec2)> = points
        .iter()
        .filter(|(n, _)| n.starts_with("supply"))
        .collect();
    if !supplies.is_empty() && !map.starts().is_empty() {
        let _ = writeln!(
            out,
            "  path length start -> supply point (m), vehicles / infantry:"
        );
        let mut header = format!("  {:<24}", "");
        for s in map.starts() {
            let _ = write!(header, " {:>22}", format!("start {}", s.player));
        }
        let _ = writeln!(out, "{header}");
        for (name, p) in &supplies {
            let mut line = format!("  {name:<24}");
            for s in map.starts() {
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

    if let Some(path) = png {
        let mut rgb = vec![0u8; count * 3];
        for (k, kind) in kinds.iter().enumerate() {
            let color: [u8; 3] = match kind {
                Kind::Water => [40, 70, 110],
                Kind::Vehicles if road[k] => [60, 130, 60],
                Kind::Vehicles => [140, 200, 110],
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
            "  picture {} (light green: vehicles, dark green: their roads and tracks, yellow: infantry only, red: nobody, dark: buildings and objects, blue: water, white: starts and markers)",
            path.display()
        );
    }
    Ok(out)
}
