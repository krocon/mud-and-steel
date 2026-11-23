//! Sight from the heights of a map (prompt 113): what an observer or a gun
//! crew sees from every supply point and every gun position (`height_*`
//! markers), with the simulation's own viewshed (`sim::vision`: ground,
//! buildings, forest, the downhill bonus), and whether towed guns of each
//! side get onto a height that looks down into a valley towards the enemy.

use sim::path::PathGrid;
use sim::vision::{VisionGrid, VisionRules};
use sim::{Catalog, Map, Scale, TerrainRules, Vec2};
use std::fmt::Write as _;
use std::path::Path;

/// Eye height of an observer standing at a gun position (mm).
pub const EYE_MM: i32 = 2000;
/// A visible cell counts as "down in a valley" when its ground lies this
/// far below the eye: the drop from which the downhill bonus starts
/// (`[height] min_drop_m` of data/vision.toml).
pub const VALLEY_DROP_MM: i32 = 20_000;
/// Valley cells towards the enemy a gun position needs (8 m cells: 100
/// cells are 0.64 ha).
pub const MIN_VALLEY_CELLS: usize = 100;
/// A marker counts as reached when the path ends this close (m).
const NEAR_M: i32 = 20;

/// What one point sees.
pub struct Sight {
    pub name: String,
    pub position: Vec2,
    pub eye_mm: i32,
    /// Visible vision cells (grid indices).
    pub cells: Vec<u32>,
}

/// The vision grid of the map and the sight from every supply point and
/// gun position.
pub fn sights(map: &Map) -> Result<(VisionGrid, Vec<Sight>), String> {
    let rules = VisionRules::builtin().map_err(|e| e.to_string())?;
    let mut grid = VisionGrid::new(map, &rules, &[]);
    let mut out = Vec::new();
    for m in map.markers() {
        if !(m.name.starts_with("supply") || m.name.starts_with("height")) {
            continue;
        }
        let cells = grid.sight_from(&rules, m.position, EYE_MM, None);
        let eye_mm = grid.ground_mm(grid.index_of(m.position)) + EYE_MM;
        out.push(Sight {
            name: m.name.clone(),
            position: m.position,
            eye_mm,
            cells,
        });
    }
    Ok((grid, out))
}

/// Visible cells of `sight` that lie at least [`VALLEY_DROP_MM`] below its
/// eye and towards `enemy` (the half-plane in front of the point, seen
/// from `own`).
pub fn valley_cells_towards(grid: &VisionGrid, sight: &Sight, own: Vec2, enemy: Vec2) -> usize {
    let dir = enemy - own;
    sight
        .cells
        .iter()
        .filter(|&&i| {
            let c = grid.center(i % grid.columns, i / grid.columns);
            let d = c - sight.position;
            let front = d.x.to_num::<i64>() * dir.x.to_num::<i64>()
                + d.y.to_num::<i64>() * dir.y.to_num::<i64>()
                > 0;
            front && grid.ground_mm(i) <= sight.eye_mm - VALLEY_DROP_MM
        })
        .count()
}

/// Team of a start (a start without one is a team of its own).
fn team_of(s: &sim::map::StartPosition) -> u8 {
    s.team.unwrap_or(s.player)
}

/// Mean position of a team's starts.
fn team_center(map: &Map, team: u8) -> Vec2 {
    let starts: Vec<Vec2> = map
        .starts()
        .iter()
        .filter(|s| team_of(s) == team)
        .map(|s| s.position)
        .collect();
    let n = i64::try_from(starts.len().max(1)).unwrap_or(1);
    let sum = starts.iter().fold(Vec2::ZERO, |a, &p| a + p);
    Vec2::new(sum.x / n, sum.y / n)
}

/// Per start: the gun positions (`height_*`) a vehicle (the tank size
/// class, which trucks with a gun share) reaches from it and that look
/// down into a valley towards the enemy (at least [`MIN_VALLEY_CELLS`]).
pub fn gun_heights(map: &Map) -> Result<Vec<(u8, u8, Vec<String>)>, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .map_err(|e| e.to_string())?;
    let path = PathGrid::new(map, &rules, &types);
    let (grid, all) = sights(map)?;
    let mut out = Vec::new();
    for s in map.starts() {
        let team = team_of(s);
        let own = team_center(map, team);
        let enemy_team = map
            .starts()
            .iter()
            .map(team_of)
            .find(|&t| t != team)
            .unwrap_or(team);
        let enemy = team_center(map, enemy_team);
        let mut good = Vec::new();
        for sight in all.iter().filter(|x| x.name.starts_with("height")) {
            let reached = path
                .find_path(s.position, sight.position, 0)
                .is_some_and(|p| {
                    (p.goal - sight.position).length() <= sim::Fixed::from_num(NEAR_M)
                });
            if reached && valley_cells_towards(&grid, sight, own, enemy) >= MIN_VALLEY_CELLS {
                good.push(sight.name.clone());
            }
        }
        out.push((s.player, team, good));
    }
    Ok(out)
}

pub fn run(map: &Map, png: Option<&Path>) -> Result<String, String> {
    let (grid, all) = sights(map)?;
    let cell_m2 = {
        let c = grid.cell.to_num::<f64>();
        c * c
    };
    let land: usize = (0..grid.cells())
        .filter(|&i| grid.ground_mm(i as u32) > 0)
        .count();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "map '{}': sight from supply points and gun positions (eye {} m above the ground, {} m cells)",
        map.name(),
        EYE_MM / 1000,
        grid.cell.to_num::<f64>()
    );
    let centers: Vec<(u8, Vec2)> = {
        let mut teams: Vec<u8> = map.starts().iter().map(team_of).collect();
        teams.sort_unstable();
        teams.dedup();
        teams.iter().map(|&t| (t, team_center(map, t))).collect()
    };
    for s in &all {
        let below = s
            .cells
            .iter()
            .filter(|&&i| grid.ground_mm(i) <= s.eye_mm - VALLEY_DROP_MM)
            .count();
        let mut line = format!(
            "  {:<24} ground {:>5.0} m: sees {:>5.1} ha ({:.1} % of the land), {:>5.1} ha of it 20 m or more below",
            s.name,
            f64::from(s.eye_mm - EYE_MM) / 1000.0,
            s.cells.len() as f64 * cell_m2 / 1e4,
            s.cells.len() as f64 * 100.0 / land.max(1) as f64,
            below as f64 * cell_m2 / 1e4
        );
        if centers.len() == 2 {
            for (k, (team, own)) in centers.iter().enumerate() {
                let enemy = centers[1 - k].1;
                let n = valley_cells_towards(&grid, s, *own, enemy);
                let _ = write!(
                    line,
                    "; towards team {} from {team}: {n} cells",
                    centers[1 - k].0
                );
            }
        }
        let seen: Vec<&str> = map
            .markers()
            .iter()
            .filter(|m| m.name != s.name && !m.name.starts_with("start"))
            .filter(|m| s.cells.binary_search(&grid.index_of(m.position)).is_ok())
            .map(|m| m.name.as_str())
            .collect();
        if !seen.is_empty() {
            let _ = write!(line, "; markers in sight: {}", seen.join(", "));
        }
        let _ = writeln!(out, "{line}");
    }
    if map.starts().len() >= 2 {
        let _ = writeln!(
            out,
            "  gun positions a towed gun reaches from each start and that look down into a valley towards the enemy ({MIN_VALLEY_CELLS}+ cells 20 m below):"
        );
        for (player, team, good) in gun_heights(map)? {
            let _ = writeln!(
                out,
                "    start {player} (team {team}): {}",
                if good.is_empty() {
                    "none".to_string()
                } else {
                    good.join(", ")
                }
            );
        }
    }

    if let Some(path) = png {
        let (w, h) = (grid.columns, grid.rows);
        let count = grid.cells();
        let (lo, hi) = (0..count).fold((i32::MAX, i32::MIN), |(a, b), i| {
            let g = grid.ground_mm(i as u32);
            (a.min(g), b.max(g))
        });
        let span = f64::from((hi - lo).max(1));
        let mut rgb = vec![0u8; count * 3];
        for i in 0..count {
            let g = grid.ground_mm(i as u32);
            let c = if g <= 0 {
                [40, 70, 110]
            } else {
                let t = f64::from(g - lo) / span;
                let v = (70.0 + 150.0 * t) as u8;
                [v, v, v]
            };
            rgb[i * 3..i * 3 + 3].copy_from_slice(&c);
        }
        // Every point's sight in its own tint over the grey heights.
        let tints: [[u8; 3]; 8] = [
            [230, 80, 60],
            [60, 160, 230],
            [240, 200, 40],
            [120, 210, 90],
            [200, 90, 220],
            [250, 140, 40],
            [60, 220, 200],
            [240, 120, 160],
        ];
        for (n, s) in all.iter().enumerate() {
            let t = tints[n % tints.len()];
            for &i in &s.cells {
                let k = i as usize * 3;
                for c in 0..3 {
                    rgb[k + c] = ((u16::from(rgb[k + c]) + 2 * u16::from(t[c])) / 3) as u8;
                }
            }
        }
        let mut mark = |p: Vec2, color: [u8; 3]| {
            let (cx, cy) = grid.cell_of(p);
            for dy in -2i32..=2 {
                for dx in -2i32..=2 {
                    let (x, y) = (cx + dx, cy + dy);
                    if x >= 0 && y >= 0 && x < w as i32 && y < h as i32 {
                        let k = (y as usize * w as usize + x as usize) * 3;
                        let edge = dx.abs() == 2 || dy.abs() == 2;
                        rgb[k..k + 3].copy_from_slice(if edge { &[0, 0, 0] } else { &color });
                    }
                }
            }
        };
        for m in map.markers() {
            mark(m.position, [255, 255, 255]);
        }
        for s in map.starts() {
            mark(s.position, [255, 255, 0]);
        }
        let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(&rgb).map_err(|e| e.to_string())?;
        writer.finish().map_err(|e| e.to_string())?;
        let _ = writeln!(
            out,
            "  picture {} (one pixel per {} m cell: grey heights, each point's sight tinted, white markers, yellow starts)",
            path.display(),
            grid.cell.to_num::<f64>()
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::gun_heights;
    use std::path::PathBuf;

    /// Prompt 113: from every start towed guns reach at least two gun
    /// positions on the heights that look down into a valley towards the
    /// enemy.
    #[test]
    fn coastal_heights_guns_reach_heights_with_sight() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/maps/coastal_heights");
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let mut map = sim::Map::load(
            &String::from_utf8(read("map.toml")).unwrap(),
            &read("height.png"),
            &read("terrain.png"),
        )
        .unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        let per_start = gun_heights(&map).unwrap();
        assert_eq!(per_start.len(), 4);
        for (player, _, good) in per_start {
            assert!(
                good.len() >= 2,
                "start {player}: gun positions with sight into a valley: {good:?}"
            );
        }
    }
}
