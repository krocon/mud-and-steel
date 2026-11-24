//! Sight from the heights of a map (prompt 113): what an observer or a gun
//! crew sees from every supply point and every gun position (`height_*`
//! markers; since prompt 125 also from the front trenches, `front_*`),
//! with the simulation's own viewshed (`sim::vision`: ground, buildings,
//! forest, the downhill bonus), and whether towed guns of each
//! side get onto a height that looks down into a valley towards the enemy.
//!
//! Prompt 123: prepared gun positions behind cover (`battery_*` markers):
//! how much of the other side each one sees (it should see none), which
//! enemy points see it, and which supply points, bridges, fords and enemy
//! positions lie in range of which indirect weapons of the catalog, from
//! the positions and from the supply points (forward positions).

use sim::path::PathGrid;
use sim::vision::{VisionGrid, VisionRules};
use sim::{Catalog, Fixed, Map, Scale, TerrainRules, Vec2};
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
        if !(m.name.starts_with("supply")
            || m.name.starts_with("height")
            || m.name.starts_with("battery")
            || m.name.starts_with("front_"))
        {
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

/// `only` (prompt 126): name prefixes of the points whose sight the
/// picture shows (empty: every point).
pub fn run(map: &Map, png: Option<&Path>, only: &[String]) -> Result<String, String> {
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

    if map.markers().iter().any(|m| m.name.starts_with("battery")) {
        out.push_str(&batteries(map, &grid, &all)?);
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
        let shown = all
            .iter()
            .filter(|s| only.is_empty() || only.iter().any(|o| s.name.starts_with(o.as_str())));
        for (n, s) in shown.enumerate() {
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

/// The team whose start center lies nearer to `p` (two teams); `None`
/// within 4 m of the middle (points on a mirror axis belong to nobody).
fn side_of(centers: &[(u8, Vec2)], p: Vec2) -> Option<u8> {
    let mut by: Vec<(Fixed, u8)> = centers
        .iter()
        .map(|(t, c)| ((*c - p).length(), *t))
        .collect();
    by.sort_unstable();
    match by.as_slice() {
        [a, b, ..] if b.0 - a.0 < Fixed::from_num(4) => None,
        [a, ..] => Some(a.1),
        [] => None,
    }
}

/// Indirect weapons of the catalog: unit id, game range (m), category.
pub fn indirect_weapons() -> Result<Vec<(String, f64, &'static str)>, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let mut out: Vec<(String, f64, &'static str)> = catalog
        .entries
        .iter()
        .filter_map(|e| {
            sim::indirect::IndirectWeapon::from_catalog(e, &scale)
                .map(|w| (e.id.clone(), w.range_m.to_num::<f64>(), e.category.code()))
        })
        .collect();
    out.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    Ok(out)
}

/// One firing point and what it reaches: (target, distance in m, unit ids
/// of the indirect weapons whose range covers it).
pub struct Reach {
    pub from: String,
    pub targets: Vec<(String, f64, Vec<String>)>,
}

/// Ranges from every gun position (`battery_*`) and supply point to the
/// starts, supply points, gun positions and (prompt 126) basins
/// (`basin_*`) of the other side and to every bridge and ford (`bridge_*`,
/// `ford_*`).
pub fn ranges(map: &Map) -> Result<Vec<Reach>, String> {
    let weapons = indirect_weapons()?;
    let centers = team_centers(map);
    let mut out = Vec::new();
    for from in map
        .markers()
        .iter()
        .filter(|m| m.name.starts_with("battery") || m.name.starts_with("supply"))
    {
        let own = side_of(&centers, from.position);
        let mut targets = Vec::new();
        let starts = map
            .starts()
            .iter()
            .filter(|s| own.is_some_and(|t| t != team_of(s)))
            .map(|s| (format!("start {}", s.player), s.position));
        for (name, position) in starts {
            let d = (position - from.position).length().to_num::<f64>();
            let reached = weapons
                .iter()
                .filter(|(_, r, _)| *r >= d)
                .map(|(id, _, _)| id.clone())
                .collect();
            targets.push((name, d, reached));
        }
        for t in map.markers() {
            let crossing = t.name.starts_with("bridge") || t.name.starts_with("ford");
            let enemy = (t.name.starts_with("supply")
                || t.name.starts_with("battery")
                || t.name.starts_with("basin"))
                && (own.is_none() || side_of(&centers, t.position) != own);
            if !(crossing || enemy) || t.name == from.name {
                continue;
            }
            let d = (t.position - from.position).length().to_num::<f64>();
            let reached = weapons
                .iter()
                .filter(|(_, r, _)| *r >= d)
                .map(|(id, _, _)| id.clone())
                .collect();
            targets.push((t.name.clone(), d, reached));
        }
        out.push(Reach {
            from: from.name.clone(),
            targets,
        });
    }
    Ok(out)
}

fn team_centers(map: &Map) -> Vec<(u8, Vec2)> {
    let mut teams: Vec<u8> = map.starts().iter().map(team_of).collect();
    teams.sort_unstable();
    teams.dedup();
    teams.iter().map(|&t| (t, team_center(map, t))).collect()
}

/// Per gun position: visible cells on the other side (nearer to the enemy
/// starts) and the enemy supply points and gun positions that see it.
pub fn battery_sight(
    map: &Map,
    grid: &VisionGrid,
    all: &[Sight],
) -> Vec<(String, usize, Vec<String>)> {
    let centers = team_centers(map);
    let mut out = Vec::new();
    for b in all.iter().filter(|s| s.name.starts_with("battery")) {
        let own = side_of(&centers, b.position);
        let beyond = b
            .cells
            .iter()
            .filter(|&&i| {
                let c = grid.center(i % grid.columns, i / grid.columns);
                side_of(&centers, c) != own
            })
            .count();
        let cell = grid.index_of(b.position);
        let seen_by = all
            .iter()
            .filter(|o| side_of(&centers, o.position) != own)
            .filter(|o| o.cells.binary_search(&cell).is_ok())
            .map(|o| o.name.clone())
            .collect();
        out.push((b.name.clone(), beyond, seen_by));
    }
    out
}

fn batteries(map: &Map, grid: &VisionGrid, all: &[Sight]) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "  gun positions: cells seen on the other side (8 m cells) and enemy points that see them:"
    );
    for (name, beyond, seen_by) in battery_sight(map, grid, all) {
        let _ = writeln!(
            out,
            "    {name:<24} {beyond:>5} cells; seen by {}",
            if seen_by.is_empty() {
                "nobody".to_string()
            } else {
                seen_by.join(", ")
            }
        );
    }
    let weapons = indirect_weapons()?;
    let _ = writeln!(
        out,
        "  indirect weapons (game range = real x indirect_fire factor of data/scale.toml):"
    );
    for (id, r, category) in &weapons {
        let _ = writeln!(out, "    {id:<22} {category:<18} {r:>6.0} m");
    }
    let _ = writeln!(
        out,
        "  ranges: target, distance, how many of the {} indirect weapons reach it and the shortest-ranged of them:",
        weapons.len()
    );
    for reach in ranges(map)? {
        let _ = writeln!(out, "    from {}:", reach.from);
        for (target, d, reached) in &reach.targets {
            let _ = writeln!(
                out,
                "      {target:<22} {d:>6.0} m  {:>2}  {}",
                reached.len(),
                reached.first().map_or("-", String::as_str)
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::gun_heights;
    use std::path::PathBuf;

    fn load(name: &str) -> sim::Map {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/maps")
            .join(name);
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let mut map = sim::Map::load(
            &String::from_utf8(read("map.toml")).unwrap(),
            &read("height.png"),
            &read("terrain.png"),
        )
        .unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        map
    }

    /// Prompt 123: on `river_front` the gun positions see nothing of the
    /// other side and nobody there sees them; only the farthest gun reaches
    /// an enemy position from a position; the starts lie beyond the light
    /// howitzers of the enemy positions; every bridge and ford lies within
    /// the shortest light howitzer's range of a position of each side;
    /// mortars reach no enemy supply point from the positions, but heavy
    /// mortars reach an island on the axis from the observation hills (the
    /// river is too broad to reach the enemy bank from them).
    #[test]
    fn river_front_ranges_and_sight() {
        let map = load("river_front");
        let (grid, all) = super::sights(&map).unwrap();
        for (name, beyond, seen_by) in super::battery_sight(&map, &grid, &all) {
            assert_eq!(beyond, 0, "{name} sees the other side");
            assert!(seen_by.is_empty(), "{name} is seen by {seen_by:?}");
        }
        let weapons = super::indirect_weapons().unwrap();
        let range = |id: &str| weapons.iter().find(|w| w.0 == id).unwrap().1;
        let farthest = &weapons.last().unwrap().0;
        let light_max = weapons
            .iter()
            .filter(|w| w.2 == "gun_howitzer")
            .map(|w| w.1)
            .fold(0.0, f64::max);
        let light_min = range("us_m2a1_105");
        let reaches = super::ranges(&map).unwrap();
        let mut covered: Vec<(String, char)> = Vec::new();
        for r in reaches.iter().filter(|r| r.from.starts_with("battery")) {
            let side = if r.from.starts_with("battery_n") {
                'n'
            } else {
                's'
            };
            let mut opposite = false;
            for (target, d, reached) in &r.targets {
                if target.starts_with("battery") {
                    assert!(
                        reached.iter().all(|id| id == farthest),
                        "{} -> {target}: {reached:?}",
                        r.from
                    );
                    opposite |= !reached.is_empty();
                }
                if target.starts_with("start") {
                    assert!(*d > light_max, "{} -> {target}: {d:.0} m", r.from);
                }
                if (target.starts_with("bridge") || target.starts_with("ford")) && *d <= light_min {
                    covered.push((target.clone(), side));
                }
                if target.starts_with("supply") {
                    assert!(
                        !reached
                            .iter()
                            .any(|id| weapons.iter().any(|w| &w.0 == id && w.2.contains("mortar"))),
                        "{} -> {target}: a mortar reaches it",
                        r.from
                    );
                }
            }
            assert!(
                opposite,
                "{}: no enemy position within the farthest gun",
                r.from
            );
        }
        for m in map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with("bridge") || m.name.starts_with("ford"))
        {
            for side in ['n', 's'] {
                assert!(
                    covered.contains(&(m.name.clone(), side)),
                    "{} out of the light howitzers of side {side}",
                    m.name
                );
            }
        }
        for r in reaches.iter().filter(|r| r.from.contains("_obs_")) {
            let heavy = range("uk_42in_mortar");
            let (mut island, mut hill) = (f64::INFINITY, f64::INFINITY);
            for (t, d, _) in &r.targets {
                if t.starts_with("supply") && t.contains("island") {
                    island = island.min(*d);
                }
                if t.contains("_obs_") {
                    hill = hill.min(*d);
                }
            }
            assert!(
                island <= heavy,
                "{}: the nearest island is {island:.0} m away",
                r.from
            );
            assert!(
                hill > heavy,
                "{}: an enemy hill is {hill:.0} m away",
                r.from
            );
        }
    }

    /// Prompt 124: on `trench_line` the gun positions see nothing of the
    /// other side and nobody there sees them; only the farthest gun
    /// reaches an enemy gun position, nothing an enemy start; the light
    /// howitzers of the middle position reach the enemy front, the 105 mm
    /// M2A1 of every position the enemy trench supply point; every medium
    /// mortar reaches the enemy front trench from the own one.
    #[test]
    fn trench_line_ranges_and_sight() {
        let map = crate::prepared_ground(load("trench_line"), &["de_inf_rifle_squad"])
            .unwrap()
            .0;
        let (grid, all) = super::sights(&map).unwrap();
        for (name, beyond, seen_by) in super::battery_sight(&map, &grid, &all) {
            assert_eq!(beyond, 0, "{name} sees the other side");
            assert!(seen_by.is_empty(), "{name} is seen by {seen_by:?}");
        }
        let weapons = super::indirect_weapons().unwrap();
        let range = |id: &str| weapons.iter().find(|w| w.0 == id).unwrap().1;
        let farthest = &weapons.last().unwrap().0;
        let light = |f: fn(f64, f64) -> f64, init: f64| {
            weapons
                .iter()
                .filter(|w| w.2 == "gun_howitzer")
                .map(|w| w.1)
                .fold(init, f)
        };
        let (light_min, light_max) = (light(f64::min, f64::INFINITY), light(f64::max, 0.0));
        let reaches = super::ranges(&map).unwrap();
        for r in reaches.iter().filter(|r| r.from.starts_with("battery")) {
            let mut opposite = false;
            for (target, d, reached) in &r.targets {
                if target.starts_with("battery") {
                    assert!(
                        reached.iter().all(|id| id == farthest),
                        "{} -> {target}: {reached:?}",
                        r.from
                    );
                    opposite |= !reached.is_empty();
                }
                if target.starts_with("start") {
                    assert!(*d > range(farthest), "{} -> {target}: {d:.0} m", r.from);
                    assert!(*d > light_max);
                }
                if target.ends_with("_trench") {
                    assert!(
                        *d <= range("us_m2a1_105"),
                        "{} -> {target}: {d:.0} m",
                        r.from
                    );
                }
            }
            assert!(
                opposite,
                "{}: no enemy position within the farthest gun",
                r.from
            );
        }
        let at = |name: &str| map.marker(name).unwrap();
        let d = |a: sim::Vec2, b: sim::Vec2| (a - b).length().to_num::<f64>();
        assert!(d(at("battery_n2"), at("front_s")) <= light_min);
        assert!(d(at("battery_s2"), at("front_n")) <= light_min);
        // Front trenches: the fire trenches nearest the middle line.
        let fronts: Vec<&sim::prepared::PreparedWork> = map
            .prepared()
            .works
            .iter()
            .filter(|w| w.kind == sim::prepared::PreparedWorkKind::Trench)
            .filter(|w| {
                w.points
                    .iter()
                    .all(|p| p.y.abs() < sim::Fixed::from_num(200))
            })
            .collect();
        assert_eq!(fronts.len(), 2, "two front trenches");
        let medium = weapons
            .iter()
            .filter(|w| w.2 == "inf_mortar_medium")
            .map(|w| w.1)
            .fold(f64::INFINITY, f64::min);
        for (own, enemy) in [(fronts[0], fronts[1]), (fronts[1], fronts[0])] {
            for &p in &own.points {
                let nearest = enemy
                    .points
                    .iter()
                    .map(|&q| d(p, q))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest <= medium,
                    "the enemy front is {nearest:.0} m from ({}, {})",
                    p.x,
                    p.y
                );
            }
        }
    }

    /// Prompt 126: on `mesa_desert` the gun positions at the rims of the
    /// plateaus look down into the basins (2 ha or more 20 m below) but see
    /// less than a twentieth of the land (sight 1/4); a basin sees nothing
    /// from above; no mortar or rocket launcher reaches an enemy start from
    /// a plateau, light howitzers reach the open centre from every gun
    /// position, and the buttes of the centre ring lie within
    /// the medium mortars of the centre.
    #[test]
    fn mesa_desert_ranges_and_sight() {
        let map = load("mesa_desert");
        let (grid, all) = super::sights(&map).unwrap();
        let land = grid.cells();
        let drop = |s: &super::Sight| {
            s.cells
                .iter()
                .filter(|&&i| grid.ground_mm(i) <= s.eye_mm - super::VALLEY_DROP_MM)
                .count()
        };
        // 8 m cells: 2 ha are 312 cells.
        for s in all.iter().filter(|s| s.name.starts_with("battery")) {
            assert!(drop(s) >= 312, "{} sees {} cells below", s.name, drop(s));
            assert!(s.cells.len() * 20 < land, "{} sees too much", s.name);
        }
        for s in all.iter().filter(|s| s.name.ends_with("_basin")) {
            assert_eq!(drop(s), 0, "{}", s.name);
        }
        let weapons = super::indirect_weapons().unwrap();
        let howitzer = |w: &&(String, f64, &str)| w.2 == "gun_howitzer" || w.2 == "self_propelled";
        let short_max = weapons
            .iter()
            .filter(|w| !howitzer(w))
            .map(|w| w.1)
            .fold(0.0, f64::max);
        let light_min = weapons
            .iter()
            .filter(|w| w.2 == "gun_howitzer")
            .map(|w| w.1)
            .fold(f64::INFINITY, f64::min);
        let light_max = weapons
            .iter()
            .filter(|w| w.2 == "gun_howitzer")
            .map(|w| w.1)
            .fold(0.0, f64::max);
        let medium = weapons
            .iter()
            .filter(|w| w.2 == "inf_mortar_medium")
            .map(|w| w.1)
            .fold(f64::INFINITY, f64::min);
        let d = |a: sim::Vec2, b: sim::Vec2| (a - b).length().to_num::<f64>();
        let at = |n: &str| {
            map.markers()
                .iter()
                .find(|m| m.name == n)
                .unwrap_or_else(|| panic!("{n}"))
                .position
        };
        for b in map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with("battery"))
        {
            let north = b.name.starts_with("battery_n");
            for s in map.starts() {
                if (s.team == Some(0)) != north {
                    let x = d(b.position, s.position);
                    assert!(x > short_max, "{} -> start {}: {x:.0} m", b.name, s.player);
                }
            }
            // The positions on the north and east plateau (1, 2) reach the
            // centre with every light howitzer, the one at the north-west
            // corner (3) with the longer ones.
            let c = d(b.position, at("supply_center"));
            let reach = if b.name.ends_with('3') {
                light_max
            } else {
                light_min
            };
            assert!(c <= reach, "{}: centre {c:.0} m", b.name);
        }
        for butte in ["supply_n_butte", "supply_s_butte"] {
            assert!(d(at(butte), at("supply_center")) <= medium);
        }
    }

    /// Prompt 125: on `river_fork_front` (not symmetric) the gun positions
    /// see nothing of the other side and nobody there sees them; no gun
    /// reaches an enemy gun position or an enemy start from a position;
    /// from every position the shortest light howitzer reaches the enemy
    /// front; every medium mortar reaches the enemy front from every point
    /// of the own front trench.
    #[test]
    fn river_fork_front_ranges_and_sight() {
        let map = crate::prepared_ground(load("river_fork_front"), &["de_inf_rifle_squad"])
            .unwrap()
            .0;
        let (grid, all) = super::sights(&map).unwrap();
        for (name, beyond, seen_by) in super::battery_sight(&map, &grid, &all) {
            assert_eq!(beyond, 0, "{name} sees the other side");
            assert!(seen_by.is_empty(), "{name} is seen by {seen_by:?}");
        }
        let weapons = super::indirect_weapons().unwrap();
        let farthest = weapons.last().unwrap().1;
        let light_min = weapons
            .iter()
            .filter(|w| w.2 == "gun_howitzer")
            .map(|w| w.1)
            .fold(f64::INFINITY, f64::min);
        let medium = weapons
            .iter()
            .filter(|w| w.2 == "inf_mortar_medium")
            .map(|w| w.1)
            .fold(f64::INFINITY, f64::min);
        let d = |a: sim::Vec2, b: sim::Vec2| (a - b).length().to_num::<f64>();
        // Fire trenches within 200 m of no man's land (y = -0.06 x): the
        // fronts, north (y below the line) and south.
        let mid = |p: sim::Vec2| p.y.to_num::<f64>() + 0.06 * p.x.to_num::<f64>();
        let fronts: Vec<&sim::prepared::PreparedWork> = map
            .prepared()
            .works
            .iter()
            .filter(|w| w.kind == sim::prepared::PreparedWorkKind::Trench)
            .filter(|w| w.points.iter().all(|&p| mid(p).abs() < 200.0))
            .collect();
        let side = |w: &sim::prepared::PreparedWork| mid(w.points[0]) > 0.0;
        let north: Vec<sim::Vec2> = fronts
            .iter()
            .filter(|w| !side(w))
            .flat_map(|w| w.points.clone())
            .collect();
        let south: Vec<sim::Vec2> = fronts
            .iter()
            .filter(|w| side(w))
            .flat_map(|w| w.points.clone())
            .collect();
        assert_eq!(fronts.len(), 6, "three front trenches a side");
        let nearest = |p: sim::Vec2, set: &[sim::Vec2]| {
            set.iter().map(|&q| d(p, q)).fold(f64::INFINITY, f64::min)
        };
        for (own, enemy) in [(&north, &south), (&south, &north)] {
            for &p in own.iter() {
                let n = nearest(p, enemy);
                assert!(
                    n <= medium,
                    "the enemy front is {n:.0} m from ({}, {})",
                    p.x,
                    p.y
                );
            }
        }
        let positions: Vec<&sim::map::Marker> = map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with("battery"))
            .collect();
        for b in &positions {
            let north_side = b.name.starts_with("battery_n");
            let enemy = if north_side { &south } else { &north };
            let n = nearest(b.position, enemy);
            assert!(
                n <= light_min,
                "{}: the enemy front is {n:.0} m away",
                b.name
            );
            for o in &positions {
                if o.name.starts_with("battery_n") != north_side {
                    let x = d(b.position, o.position);
                    assert!(x > farthest, "{} -> {}: {x:.0} m", b.name, o.name);
                }
            }
            for s in map.starts() {
                if (s.team == Some(0)) != north_side {
                    let x = d(b.position, s.position);
                    assert!(x > farthest, "{} -> start {}: {x:.0} m", b.name, s.player);
                }
            }
        }
    }

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
