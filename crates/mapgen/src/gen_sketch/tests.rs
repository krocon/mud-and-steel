use super::{ALL, Options, generate};
use sim::Map;

/// Every sketch map loads with its starts, and the same sketch gives the
/// same files twice.
#[test]
fn sketch_maps_are_the_same_every_time() {
    // Tests run in the crate folder; sketches live in the workspace.
    let sketches = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/map_sketches"
    ));
    // One thread per map and run (the large maps take long unoptimized).
    std::thread::scope(|scope| {
        for name in ALL {
            scope.spawn(move || check(name, sketches));
        }
    });
}

/// Prompt 123: `river_front` is mirror-symmetric about the path cell
/// center line y = 1 m: heights, terrain, objects, starts and markers,
/// the open path cells and the path lengths of both sides (the generated
/// ramp markers of the south table mountains end in `_image`).
#[test]
fn river_front_is_mirror_symmetric() {
    check_symmetry("river_front", Sym::North, &|n: &str| {
        if let Some(north) = n.strip_suffix("_image") {
            return north.to_string();
        }
        if n.starts_with("ramp_") {
            return format!("{n}_image");
        }
        n.replace("_n_", "_x_")
            .replace("_s_", "_n_")
            .replace("_x_", "_s_")
            .replace("battery_n", "battery_x")
            .replace("battery_s", "battery_n")
            .replace("battery_x", "battery_s")
    });
}

/// Prompt 124: `trench_line` is point-symmetric about the path cell
/// center point (1, 1): the same as above with every point turned by
/// 180°, and its prepared field works, craters and puddles too.
#[test]
fn trench_line_is_point_symmetric() {
    check_symmetry("trench_line", Sym::Point, &|n: &str| {
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        let n = swap(n, "_n_", "_s_");
        let n = swap(&n, "_w_", "_e_");
        let n = swap(&n, "battery_n", "battery_s");
        let n = swap(&n, "front_n", "front_s");
        let n = swap(&n, "lane_w", "lane_e");
        let n = swap(&n, "smoke_1", "smoke_2");
        let n = swap(&n, "smoke_3", "smoke_4");
        swap(&n, "mist_1", "mist_2")
    });
}

/// Prompt 126: `mesa_desert` is point-symmetric about (1, 1): mesas,
/// ramps, boulders, starts and markers (the generated ramp markers of
/// the south half end in `_image`).
#[test]
fn mesa_desert_is_point_symmetric() {
    check_symmetry("mesa_desert", Sym::Point, &|n: &str| {
        if let Some(north) = n.strip_suffix("_image") {
            return north.to_string();
        }
        if n.starts_with("ramp_") {
            return format!("{n}_image");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        let n = swap(n, "_n_", "_s_");
        swap(&n, "battery_n", "battery_s")
    });
}

/// Prompt 128: `kessel_lake` is point-symmetric about (1, 1): the rock
/// ring, its climbs, the lakes with their fords, starts and markers (the
/// generated climb markers of the south half end in `_image`).
#[test]
fn kessel_lake_is_point_symmetric() {
    check_symmetry("kessel_lake", Sym::Point, &|n: &str| {
        if let Some(north) = n.strip_suffix("_image") {
            return north.to_string();
        }
        if n.starts_with("climb_") {
            return format!("{n}_image");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        swap(n, "_n_", "_s_")
    });
}

/// Prompt 128: vehicles get into the Kessel of `kessel_lake` only
/// through its two passages (each at least 60 m wide), infantry also over
/// the climbs; the fords are open to tanks.
#[test]
fn kessel_lake_ring_has_two_passages() {
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps"))
        .join("kessel_lake");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .unwrap();
    let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
    let (w, h) = (map.samples().0 / 2, map.samples().1 / 2);
    let center = Vec2::new(sim::Fixed::from_num(1), sim::Fixed::from_num(1));
    let radius = |x: u32, y: u32| (grid.center(x, y) - center).length().to_num::<f64>();
    // The passages: |x - 1| < 60 m between radius 180 and 320 m.
    let in_passage = |x: u32, y: u32| {
        let c = grid.center(x, y);
        (c.x - center.x).abs() < sim::Fixed::from_num(60) && (180.0..320.0).contains(&radius(x, y))
    };
    // Flood fill from the centre; with the passages closed.
    let fill = |class: SizeClass, closed: bool| {
        let mut seen = vec![false; (w * h) as usize];
        let (x0, y0) = grid.cell_of(center);
        let mut stack = vec![(x0, y0)];
        seen[(y0 * w + x0) as usize] = true;
        let mut out = 0usize;
        while let Some((x, y)) = stack.pop() {
            if radius(x, y) > 330.0 {
                out += 1;
            }
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let k = (ny * w + nx) as usize;
                if seen[k] || !grid.passable(class, nx, ny) || (closed && in_passage(nx, ny)) {
                    continue;
                }
                seen[k] = true;
                stack.push((nx, ny));
            }
        }
        out
    };
    assert!(
        fill(SizeClass::Large, false) > 10_000,
        "vehicles get out of the Kessel"
    );
    assert_eq!(
        fill(SizeClass::Large, true),
        0,
        "vehicles only through the passages"
    );
    assert!(
        fill(SizeClass::Small, true) > 10_000,
        "infantry over the climbs"
    );
    // Each passage at its narrowest (along the ring's center line, 251 m
    // from the centre) is open to vehicles over 60 m.
    for sign in [-1.0f64, 1.0] {
        let y = 1.0 + sign * 251.0;
        let mut open = 0;
        let mut x = -80.0;
        while x <= 82.0 {
            let p = Vec2::new(sim::Fixed::from_num(x), sim::Fixed::from_num(y));
            if grid.passable_at(SizeClass::Large, p) {
                open += 1;
            }
            x += 2.0;
        }
        assert!(open * 2 >= 60, "passage at y = {y}: {} m open", open * 2);
    }
    // The fords: a tank drives from the shore over the islet to the
    // beach (and the same in the south-east).
    for (a, b) in [((-548, -438), (-598, -628)), ((550, 440), (600, 630))] {
        let p = |(x, y): (i32, i32)| Vec2::new(sim::Fixed::from_num(x), sim::Fixed::from_num(y));
        let path = grid.find_path(p(a), p(b), 0).expect("ford");
        assert!(
            (path.goal - p(b)).length() < sim::Fixed::from_num(8),
            "ford {a:?} -> {b:?}"
        );
    }
}

/// Prompt 129: `pit_hill` is point-symmetric about (1, 1): the terraced
/// hill, its winding ramps, boulders and pits, the ridges, mountains and
/// lakes, starts and markers (generated ramp markers of the south half
/// end in `_image`).
#[test]
fn pit_hill_is_point_symmetric() {
    check_symmetry("pit_hill", Sym::Point, &|n: &str| {
        if let Some(north) = n.strip_suffix("_image") {
            return north.to_string();
        }
        if n.starts_with("ramp_") {
            return format!("{n}_image");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        swap(n, "_n_", "_s_")
    });
}

/// Prompt 131: `battery_heights` is point-symmetric: heights, rock bands
/// and ramps, river, bridges and fords, trenches, starts and markers
/// (west and east names swap; ramp markers of the south half end in
/// `_image`).
#[test]
fn battery_heights_is_point_symmetric() {
    check_symmetry("battery_heights", Sym::Point, &|n: &str| {
        if n.contains("_image_") {
            return n.replace("_image_", "_");
        }
        if let Some((a, b)) = n.strip_prefix("ramp_").and_then(|r| r.rsplit_once("_p")) {
            return format!("ramp_{a}_image_p{b}");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        if n.contains("_w") || n.contains("_e") {
            return swap(n, "_w", "_e");
        }
        swap(n, "_n", "_s")
    });
}

/// Prompt 133: `desert_plateau` is point-symmetric (plateaus, ramps,
/// wadis, bridges, starts, markers; west and east names swap, ramp
/// markers of the south half end in `_image`).
#[test]
fn desert_plateau_is_point_symmetric() {
    check_symmetry("desert_plateau", Sym::Point, &|n: &str| {
        // The middle plateau is its own image: its north ramp markers
        // and their generated images.
        if let Some(north) = n.strip_suffix("_image") {
            return north.to_string();
        }
        if n.starts_with("ramp_middle_") {
            return format!("{n}_image");
        }
        if n.contains("_image_") {
            return n.replace("_image_", "_");
        }
        if let Some((a, b)) = n.strip_prefix("ramp_").and_then(|r| r.rsplit_once("_p")) {
            return format!("ramp_{a}_image_p{b}");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        if n.contains("_w") || n.contains("_e") {
            return swap(n, "_w", "_e");
        }
        swap(n, "_n", "_s")
    });
}

/// Prompt 133: the wadis of `desert_plateau` split the land into three
/// strips for vehicles: with the bridges and fords closed the first
/// start does not reach every other start; over the fords alone (the
/// six bridges blown) it reaches all; infantry crosses the beds
/// anywhere (also with bridges and fords closed).
#[test]
fn desert_plateau_wadis_stop_vehicles_but_not_infantry() {
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let map = load_map("desert_plateau");
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types =
        sim::unit_types_from_catalog(&catalog, &scale, &["de_tiger2", "de_inf_rifle_squad"])
            .unwrap();
    let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
    let (w, h) = (map.samples().0 / 2, map.samples().1 / 2);
    let near = |p: Vec2, prefix: &str, r: f64| {
        map.markers()
            .iter()
            .filter(|m| m.name.starts_with(prefix))
            .any(|m| (m.position - p).length() < sim::Fixed::from_num(r))
    };
    let fill = |class: SizeClass, fords: bool| {
        let mut seen = vec![false; (w * h) as usize];
        let (x0, y0) = grid.cell_of(map.starts()[0].position);
        let mut stack = vec![(x0, y0)];
        seen[(y0 * w + x0) as usize] = true;
        while let Some((x, y)) = stack.pop() {
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let k = (ny * w + nx) as usize;
                let c = grid.center(nx, ny);
                if seen[k]
                    || !grid.passable(class, nx, ny)
                    || near(c, "bridge_", 45.0)
                    || (!fords && near(c, "ford_", 60.0))
                {
                    continue;
                }
                seen[k] = true;
                stack.push((nx, ny));
            }
        }
        map.starts()
            .iter()
            .filter(|s| {
                let (x, y) = grid.cell_of(s.position);
                seen[(y * w + x) as usize]
            })
            .count()
    };
    let all = map.starts().len();
    assert_eq!(
        fill(SizeClass::Large, true),
        all,
        "the fords alone carry vehicles"
    );
    assert_eq!(
        fill(SizeClass::Small, false),
        all,
        "infantry crosses the beds anywhere"
    );
    // Along the rims, away from bridges, fords and the ends of a wadi,
    // the boulder rows close the rim cells to vehicles.
    let sketches = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/map_sketches"
    ));
    let sketch = super::load(sketches, "desert_plateau").unwrap();
    let (mut open, mut closed) = (0, 0);
    for wadi in &sketch.wadi {
        let (first, last) = (wadi.points_m[0], wadi.points_m[wadi.points_m.len() - 1]);
        for seg in wadi.points_m.windows(2) {
            let (a, b) = ((seg[0][0], seg[0][1]), (seg[1][0], seg[1][1]));
            let len = super::dist(a, b);
            let (dx, dy) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
            let mut t = 0.0;
            while t < len {
                let c = (a.0 + dx * t, a.1 + dy * t);
                t += 20.0;
                let at = Vec2::new(sim::Fixed::from_num(c.0), sim::Fixed::from_num(c.1));
                let ends =
                    super::dist(c, (first[0], first[1])).min(super::dist(c, (last[0], last[1])));
                if near(at, "bridge_", 90.0) || near(at, "ford_", 110.0) || ends < 120.0 {
                    continue;
                }
                for side in [-1.0, 1.0] {
                    let off = side * (wadi.width_m / 2.0 + super::WadiDef::RIM_M * 0.5);
                    let p = Vec2::new(
                        sim::Fixed::from_num(c.0 - dy * off),
                        sim::Fixed::from_num(c.1 + dx * off),
                    );
                    if !map.contains(p) {
                        continue;
                    }
                    let (x, y) = grid.cell_of(p);
                    if grid.passable(SizeClass::Large, x, y) {
                        open += 1;
                    } else {
                        closed += 1;
                    }
                }
            }
        }
    }
    assert!(
        closed > 200 && open * 20 < closed,
        "rim cells: {closed} closed, {open} open to vehicles"
    );
}

/// Prompt 132: `meander_valley` is point-symmetric (mesas, ramps, river,
/// bridges, trenches, starts, markers; north and south names swap, ramp
/// markers of the south half end in `_image`).
#[test]
fn meander_valley_is_point_symmetric() {
    check_symmetry("meander_valley", Sym::Point, &|n: &str| {
        if n.contains("_image_") {
            return n.replace("_image_", "_");
        }
        if let Some((a, b)) = n.strip_prefix("ramp_").and_then(|r| r.rsplit_once("_p")) {
            return format!("ramp_{a}_image_p{b}");
        }
        let swap =
            |n: &str, a: &str, b: &str| n.replace(a, "\u{1}").replace(b, a).replace('\u{1}', b);
        swap(n, "_n_", "_s_")
    });
}

/// Prompt 132: on `meander_valley` every ramp of every mesa of the north-
/// west alone carries vehicles (the large path class: the heaviest tank,
/// a tractor with a towed gun) from the first start onto the top and
/// back: the other ramps of the mesa closed, the top still connects to
/// the start; every ramp is under 12 %.
#[test]
fn meander_valley_every_ramp_alone() {
    every_ramp_alone("meander_valley", 11);
}

/// Prompt 133: the same on `desert_plateau` (nine plateaus, the middle
/// one with four ramps).
#[test]
fn desert_plateau_every_ramp_alone() {
    every_ramp_alone("desert_plateau", 12);
}

fn load_map(name: &str) -> Map {
    let dir =
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps")).join(name);
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let toml = String::from_utf8(read("map.toml")).unwrap();
    let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    map
}

/// Every ramp of every mesa with two or more ramps reaching into the north half
/// alone (the other ramps of that mesa closed) carries the large path
/// class from the mesa's top (the marker nearest to its middle) to the
/// first start; every ramp is under 12 %.
fn every_ramp_alone(name: &str, at_least: usize) {
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let map = load_map(name);
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types =
        sim::unit_types_from_catalog(&catalog, &scale, &["de_tiger2", "de_inf_rifle_squad"])
            .unwrap();
    let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
    let (w, h) = (map.samples().0 / 2, map.samples().1 / 2);
    let start = map
        .starts()
        .iter()
        .find(|s| s.player == 0)
        .unwrap()
        .position;
    let sketches = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/map_sketches"
    ));
    let sketch = super::load(sketches, name).unwrap();
    let length = |l: &[super::P]| {
        l.windows(2)
            .map(|w| super::dist((w[0][0], w[0][1]), (w[1][0], w[1][1])))
            .sum::<f64>()
    };
    let mut checked = 0;
    for mesa in sketch
        .mesa
        .iter()
        .filter(|m| m.ramp_paths_m.len() >= 2 && m.polygon_m.iter().any(|p| p[1] < 1.0))
    {
        // The top: the middle of the mesa's outline (on its plateau).
        let c = super::bounds(&mesa.polygon_m);
        let gun = Vec2::new(
            sim::Fixed::from_num((c.x0 + c.x1) / 2.0),
            sim::Fixed::from_num((c.y0 + c.y1) / 2.0),
        );
        for (k, ramp) in mesa.ramp_paths_m.iter().enumerate() {
            assert!(
                mesa.height_m / length(ramp) < 0.12,
                "{} ramp {k} grade",
                mesa.name
            );
            let closed = |p: Vec2| {
                let q = (p.x.to_num::<f64>(), p.y.to_num::<f64>());
                mesa.ramp_paths_m
                    .iter()
                    .enumerate()
                    .any(|(j, l)| j != k && super::polyline_distance(q, l) < 14.0)
            };
            let mut seen = vec![false; (w * h) as usize];
            let (x0, y0) = grid.cell_of(gun);
            let mut stack = vec![(x0, y0)];
            seen[(y0 * w + x0) as usize] = true;
            let target = grid.cell_of(start);
            let mut reached = false;
            while let Some((x, y)) = stack.pop() {
                if (x, y) == target {
                    reached = true;
                    break;
                }
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let (nx, ny) = (nx as u32, ny as u32);
                    let i = (ny * w + nx) as usize;
                    if seen[i]
                        || !grid.passable(SizeClass::Large, nx, ny)
                        || closed(grid.center(nx, ny))
                    {
                        continue;
                    }
                    seen[i] = true;
                    stack.push((nx, ny));
                }
            }
            assert!(
                reached,
                "{}: ramp {k} alone does not carry vehicles to the top",
                mesa.name
            );
            checked += 1;
        }
    }
    assert!(checked >= at_least, "{name}: {checked} ramps checked");
}

/// Prompt 132: a drive test on `meander_valley`: on every mesa of the
/// north-west a King Tiger drives from the foot of its first ramp up
/// onto the top and down its second ramp to that ramp's foot, in the
/// simulation (nothing else moves). Prints the times (docs/MAPS.md).
#[test]
fn meander_valley_tanks_drive_over_every_mesa() {
    drive_over_mesas("meander_valley", 5);
}

/// Prompt 133: the same on `desert_plateau` (the four plateaus of the
/// west and the middle one).
#[test]
fn desert_plateau_tanks_drive_over_every_plateau() {
    drive_over_mesas("desert_plateau", 5);
}

fn drive_over_mesas(name: &str, count: usize) {
    use sim::{Command, FireMode, PlayerCommand, PlayerId, Sim, SimConfig, Spawn, UnitId, Vec2};
    use std::sync::Arc;
    let map = Arc::new(load_map(name));
    let sketches = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/map_sketches"
    ));
    let sketch = super::load(sketches, name).unwrap();
    let catalog = sim::Catalog::builtin().unwrap();
    let scale = sim::Scale::builtin().unwrap();
    let at = |p: super::P| Vec2::new(sim::Fixed::from_num(p[0]), sim::Fixed::from_num(p[1]));
    let mut drove = 0;
    for mesa in sketch
        .mesa
        .iter()
        .filter(|m| m.ramp_paths_m.len() >= 2 && m.polygon_m.iter().any(|p| p[1] < 1.0))
    {
        let up = &mesa.ramp_paths_m[0];
        let down = &mesa.ramp_paths_m[1];
        let config = SimConfig {
            tick_hz: 20,
            unit_types: sim::unit_types_from_catalog(
                &catalog,
                &scale,
                &["de_inf_rifle_squad", "de_tiger2"],
            )
            .unwrap(),
            spawns: vec![
                Spawn {
                    owner: PlayerId(0),
                    unit_type: "de_tiger2".into(),
                    position: at(up[0]),
                    fire_mode: FireMode::HoldFire,
                },
                Spawn {
                    owner: PlayerId(1),
                    unit_type: "de_inf_rifle_squad".into(),
                    position: Vec2::from_meters(1700, 1700),
                    fire_mode: FireMode::HoldFire,
                },
            ],
            map: map.reference(),
            terrain: sim::TerrainRules::builtin().unwrap(),
            combat: sim::CombatRules::builtin().unwrap(),
            vision: sim::VisionRules::builtin().unwrap(),
            infantry: sim::InfantryRules::builtin().unwrap(),
            teams: sim::Teams::default(),
            weather: sim::WeatherSetup::default(),
            mission: None,
            variant: None,
        };
        let mut sim = Sim::new(&config, Arc::clone(&map), 7).unwrap();
        let tank = UnitId(0);
        let top = at(up[up.len() - 1]);
        let foot = at(down[0]);
        let order = |target: Vec2| {
            PlayerCommand::new(
                PlayerId(0),
                Command::MoveTo {
                    units: vec![tank],
                    target,
                },
            )
        };
        let position = |sim: &Sim| {
            sim.snapshot()
                .units
                .iter()
                .find(|u| u.id == tank)
                .unwrap()
                .position
        };
        let mut times = Vec::new();
        for target in [top, foot] {
            sim.step(&[order(target)]);
            let start = sim.tick();
            while (position(&sim) - target).length() > sim::Fixed::from_num(12) {
                assert!(
                    sim.tick() - start < 20 * 600,
                    "{}: the tank did not arrive at ({}, {}), it is at {:?}",
                    mesa.name,
                    target.x,
                    target.y,
                    position(&sim)
                );
                sim.step(&[]);
            }
            times.push((sim.tick() - start) / 20);
        }
        eprintln!(
            "{name} {}: up {} s, over the top and down {} s",
            mesa.name, times[0], times[1]
        );
        drove += 1;
    }
    assert_eq!(drove, count);
}

/// Prompt 129: vehicles get onto the top of `pit_hill` only over the two
/// winding ramps (rock bands and boulders close the rest of its slope),
/// infantry also elsewhere; the upland ring and both pits are open to
/// vehicles; the ramps are under 12 %.
#[test]
fn pit_hill_top_only_over_ramps() {
    use sim::path::{PathGrid, SizeClass};
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps"))
        .join("pit_hill");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let toml = String::from_utf8(read("map.toml")).unwrap();
    let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .unwrap();
    let grid = PathGrid::new(&map, &TerrainRules::builtin().unwrap(), &types);
    let (w, h) = (map.samples().0 / 2, map.samples().1 / 2);
    let at = |x: f64, y: f64| Vec2::new(sim::Fixed::from_num(x), sim::Fixed::from_num(y));
    let marker = |name: &str| {
        map.markers()
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .position
    };
    // The winding ramps of the sketch: under 12 %; a cell within 16 m of
    // an upper ramp (from the upland ring to the top) counts as ramp.
    let sketches = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/map_sketches"
    ));
    let sketch = super::load(sketches, "pit_hill").unwrap();
    let hill = sketch.mesa.iter().find(|m| m.name == "hill").unwrap();
    let top = &hill.tier[0];
    let length = |l: &[super::P]| {
        l.windows(2)
            .map(|w| super::dist((w[0][0], w[0][1]), (w[1][0], w[1][1])))
            .sum::<f64>()
    };
    for l in &hill.ramp_paths_m {
        assert!(hill.height_m / length(l) < 0.12, "lower ramp grade");
    }
    for l in &top.ramp_paths_m {
        assert!(
            (top.height_m - hill.height_m) / length(l) < 0.12,
            "upper ramp grade"
        );
    }
    let on_ramp = |p: Vec2| {
        let q = (p.x.to_num::<f64>(), p.y.to_num::<f64>());
        top.ramp_paths_m
            .iter()
            .any(|l| super::polyline_distance(q, l) < 16.0)
    };
    // Flood fill from the centre: cells beyond 340 m lie below the top.
    let fill = |class: SizeClass, closed: bool| {
        let mut seen = vec![false; (w * h) as usize];
        let (x0, y0) = grid.cell_of(at(1.0, 1.0));
        let mut stack = vec![(x0, y0)];
        seen[(y0 * w + x0) as usize] = true;
        let mut out = 0usize;
        while let Some((x, y)) = stack.pop() {
            if (grid.center(x, y) - at(1.0, 1.0)).length() > sim::Fixed::from_num(340) {
                out += 1;
            }
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                let k = (ny * w + nx) as usize;
                if seen[k]
                    || !grid.passable(class, nx, ny)
                    || (closed && on_ramp(grid.center(nx, ny)))
                {
                    continue;
                }
                seen[k] = true;
                stack.push((nx, ny));
            }
        }
        out
    };
    assert!(
        fill(SizeClass::Large, false) > 10_000,
        "vehicles get down the ramps"
    );
    assert_eq!(
        fill(SizeClass::Large, true),
        0,
        "vehicles only over the ramps"
    );
    assert!(
        fill(SizeClass::Small, true) > 10_000,
        "infantry over the open slopes"
    );
    // The rock bands of the lower slope close it for everybody: on a ray
    // through the north-north-west band (and its image) some cell is
    // closed even to infantry.
    for (dx, dy) in [(-0.5, -0.866), (0.5, 0.866)] {
        let closed = (150..220)
            .filter(|&k| {
                let r = f64::from(k) * 2.0;
                !grid.passable_at(SizeClass::Small, at(1.0 + dx * r, 1.0 + dy * r))
            })
            .count();
        assert!(closed > 0, "rock band on the ray ({dx}, {dy})");
    }
    // A tank drives from each start up to the top, onto the upland ring
    // and into both pits.
    for s in map.starts() {
        for goal in [
            at(1.0, 1.0),
            marker("height_n_plateau"),
            marker("height_s_plateau"),
            marker("pit_n_floor"),
            marker("pit_s_floor"),
        ] {
            let path = grid.find_path(s.position, goal, 0).expect("path");
            assert!(
                (path.goal - goal).length() < sim::Fixed::from_num(20),
                "start {} -> {goal:?}",
                s.player
            );
        }
    }
}

/// Prompt 130: `omega_bend` is mirror-symmetric west against east about
/// x = 1 m: the river with its loop, the four long stone bridges, the
/// church and the station on the axis, starts and markers.
#[test]
fn omega_bend_is_mirror_symmetric() {
    check_symmetry("omega_bend", Sym::West, &|n: &str| {
        n.replace("_w_", "_x_")
            .replace("_e_", "_w_")
            .replace("_x_", "_e_")
    });
}

/// Prompt 130: on `omega_bend` the north quarters are reached only over
/// bridges and the peninsula in the loop both over the neck and over
/// the loop bridges: with every bridge down nobody gets from a start to
/// the north quarter, but still to the peninsula village (the neck);
/// with only the south bridges down the peninsula is reached from the
/// north quarter (the loop bridges); everything is open with the bridges
/// up, for tanks and infantry.
#[test]
fn omega_bend_north_only_over_bridges() {
    use sim::path::PathGrid;
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps"))
        .join("omega_bend");
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let toml = String::from_utf8(read("map.toml")).unwrap();
    let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .unwrap();
    let rules = TerrainRules::builtin().unwrap();
    let marker = |map: &Map, name: &str| {
        map.markers()
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("{name}"))
            .position
    };
    // Reached: the path ends within 20 m of the goal.
    let reaches = |grid: &PathGrid, from: Vec2, to: Vec2, profile: usize| {
        grid.find_path(from, to, profile)
            .is_some_and(|p| (p.goal - to).length() < sim::Fixed::from_num(20))
    };
    let north = [
        "supply_w_loop_bridge",
        "supply_e_loop_bridge",
        "height_w_north",
        "height_e_north",
    ];
    let grid = PathGrid::new(&map, &rules, &types);
    for s in map.starts() {
        for name in north.iter().chain(&["supply_village", "supply_pond"]) {
            for profile in 0..2 {
                assert!(
                    reaches(&grid, s.position, marker(&map, name), profile),
                    "start {} -> {name} (profile {profile})",
                    s.player
                );
            }
        }
    }
    assert_eq!(map.bridges().len(), 20, "four bridges of five arches");
    // The south bridges (north-south, at x = -711 and 713) down.
    let mut south_down = map.clone();
    for i in 0..map.bridges().len() {
        if map.bridges()[i].center.y > sim::Fixed::from_num(-400) {
            south_down.set_bridge_intact(i, false);
        }
    }
    let grid = PathGrid::new(&south_down, &rules, &types);
    for (from, to) in [
        ("height_w_north", "supply_village"),
        ("height_e_north", "supply_village"),
    ] {
        for profile in 0..2 {
            assert!(
                reaches(&grid, marker(&map, from), marker(&map, to), profile),
                "{from} -> {to} over a loop bridge (profile {profile})"
            );
        }
    }
    let mut all_down = map.clone();
    for i in 0..map.bridges().len() {
        all_down.set_bridge_intact(i, false);
    }
    let grid = PathGrid::new(&all_down, &rules, &types);
    for s in map.starts() {
        for profile in 0..2 {
            assert!(
                reaches(&grid, s.position, marker(&map, "supply_village"), profile),
                "start {} -> the peninsula over the neck (profile {profile})",
                s.player
            );
            for name in north {
                assert!(
                    !reaches(&grid, s.position, marker(&map, name), profile),
                    "start {} -> {name} without bridges (profile {profile})",
                    s.player
                );
            }
        }
    }
}

/// How a map is symmetric: mirrored about y = 1 m, turned by 180°
/// about (1, 1), or (prompt 130) mirrored about x = 1 m.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sym {
    North,
    Point,
    West,
}

/// Checks that the map `name` is its own image under `sym`, markers
/// renamed by `image_name`.
#[allow(clippy::too_many_lines)]
fn check_symmetry(name: &str, sym: Sym, image_name: &dyn Fn(&str) -> String) {
    let point = sym == Sym::Point;
    let west = sym == Sym::West;
    use sim::path::PathGrid;
    use sim::{Catalog, Scale, TerrainRules, Vec2};
    let dir =
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/maps")).join(name);
    let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
    let mut map = Map::load(
        &String::from_utf8(read("map.toml")).unwrap(),
        &read("height.png"),
        &read("terrain.png"),
    )
    .unwrap();
    map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
        .unwrap();
    let (sx, sy) = map.samples();
    // Samples 2 m apart from the edge: column i mirrors sx - i (point
    // symmetry, west), row j mirrors sy - j (point symmetry, north).
    for j in u32::from(!west)..sy {
        for i in u32::from(point || west)..sx {
            let (mi, mj) = (
                if point || west { sx - i } else { i },
                if west { j } else { sy - j },
            );
            assert_eq!(
                map.sample_height_mm(i, j),
                map.sample_height_mm(mi, mj),
                "height at sample ({i}, {j})"
            );
            assert_eq!(
                map.sample_terrain(i, j),
                map.sample_terrain(mi, mj),
                "terrain at ({i}, {j})"
            );
        }
    }
    let two = sim::Fixed::from_num(2);
    let image = |p: Vec2| match sym {
        Sym::Point => Vec2::new(two - p.x, two - p.y),
        Sym::North => Vec2::new(p.x, two - p.y),
        Sym::West => Vec2::new(two - p.x, p.y),
    };
    let mm = |v: sim::Fixed| (v * sim::Fixed::from_num(1000)).round().to_num::<i64>();
    let key = |kind: u16, p: Vec2, h: u16| (kind, mm(p.x), mm(p.y), h);
    let mut keys: Vec<_> = map
        .objects()
        .iter()
        .map(|o| key(o.kind, o.position, o.heading_deg))
        .collect();
    keys.sort_unstable();
    for o in map.objects() {
        let kind = &map.object_kinds()[usize::from(o.kind)];
        let bridge = kind.deck.is_some();
        let cliff = kind.id == "env_rock_cliff";
        let h = if bridge {
            o.heading_deg
        } else {
            match sym {
                Sym::Point => (o.heading_deg + 180) % 360,
                Sym::North if cliff => (360 - o.heading_deg) % 360,
                Sym::North => (540 - o.heading_deg) % 360,
                Sym::West if cliff => (540 - o.heading_deg) % 360,
                Sym::West => (360 - o.heading_deg) % 360,
            }
        };
        let m = image(o.position);
        // Under point symmetry an object on the center is its own image.
        if point && m == o.position {
            continue;
        }
        let k = (o.kind, mm(m.x), mm(m.y), h);
        assert!(keys.binary_search(&k).is_ok(), "no image of object {o:?}");
    }
    let starts = map.starts();
    for s in starts {
        assert!(
            starts
                .iter()
                .any(|t| t.position == image(s.position) && t.team != s.team),
            "start {} has no mirrored enemy start",
            s.player
        );
    }
    let markers = map.markers();
    for m in markers {
        let other = image_name(&m.name);
        let t = markers
            .iter()
            .find(|t| t.name == other)
            .unwrap_or_else(|| panic!("{other}"));
        assert_eq!(
            t.position,
            image(m.position),
            "marker {} and {other}",
            m.name
        );
    }
    // Prepared ground (prompt 124): every work, crater and puddle has
    // its image.
    let prepared = map.prepared();
    for w in &prepared.works {
        let points: Vec<Vec2> = w.points.iter().map(|&p| image(p)).collect();
        assert!(
            prepared
                .works
                .iter()
                .any(|o| o.kind == w.kind && o.points == points),
            "no image of {} at {:?}",
            w.kind.name(),
            w.points[0]
        );
    }
    let mut craters: Vec<_> = prepared
        .craters
        .iter()
        .map(|c| (mm(c.position.x), mm(c.position.y), c.caliber_mm, c.water))
        .collect();
    craters.sort_unstable();
    for c in &prepared.craters {
        let m = image(c.position);
        assert!(
            craters
                .binary_search(&(mm(m.x), mm(m.y), c.caliber_mm, c.water))
                .is_ok(),
            "no image of crater {c:?}"
        );
    }
    for p in &prepared.puddles {
        assert!(
            prepared
                .puddles
                .iter()
                .any(|q| q.position == image(p.position) && q.radius_mm == p.radius_mm),
            "no image of puddle {p:?}"
        );
    }
    // Open path cells (cell (x, y) mirrors (w - x, h - y) or (x, h - y))
    // on the map as a match starts on it (prepared ground dug).
    let dug = crate::prepared_ground(map.clone(), &["de_inf_rifle_squad"])
        .unwrap()
        .0;
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let types = sim::unit_types_from_catalog(&catalog, &scale, &["de_pz4_h", "de_inf_rifle_squad"])
        .unwrap();
    let grid = PathGrid::new(&dug, &TerrainRules::builtin().unwrap(), &types);
    let (w, h) = (dug.samples().0 / 2, dug.samples().1 / 2);
    let mut odd = Vec::new();
    for profile in 0..grid.profiles() {
        let class = grid.class_of(profile);
        for y in u32::from(!west)..h {
            for x in u32::from(point || west)..w {
                let mx = if point || west { w - x } else { x };
                let my = if west { y } else { h - y };
                if grid.passable(class, x, y) != grid.passable(class, mx, my) {
                    odd.push((profile, x, y));
                }
            }
        }
    }
    assert!(
        odd.is_empty(),
        "asymmetric path cells: {} {:?}",
        odd.len(),
        &odd[..odd.len().min(20)]
    );
    // Both sides need the same effort: the path costs (time on the
    // ground) agree within 2 %. The search may pick another of two
    // routes of nearly the same cost on the mirrored side (a road
    // detour against the direct way), so lengths can differ more.
    let mut worst = Vec::new();
    // The source half: north (y < 0) or west (x < 0).
    let source = |p: Vec2| if west { p.x < 0 } else { p.y < 0 };
    let north: Vec<_> = starts.iter().filter(|s| source(s.position)).collect();
    for n in north {
        let s = starts
            .iter()
            .find(|t| t.position == image(n.position))
            .unwrap();
        // Prompt 126: `needle_*` markers stand on mesas without a ramp
        // (no path on purpose).
        for m in markers
            .iter()
            .filter(|m| source(m.position) && !m.name.starts_with("needle_"))
        {
            let other = markers
                .iter()
                .find(|t| t.name == image_name(&m.name))
                .unwrap();
            // Prompt 128: climb tops and rim points are open to infantry
            // only (boulders stop vehicles on the climbs).
            let infantry_only = m.name.starts_with("climb_") || m.name.contains("_rim");
            for profile in usize::from(infantry_only)..2 {
                let a = grid.find_path(n.position, m.position, profile);
                let b = grid.find_path(s.position, other.position, profile);
                let (Some(a), Some(b)) = (a, b) else {
                    panic!("no path to {} or {}", m.name, other.name);
                };
                // Point maps: the supply point in the middle lies in
                // the house (taken from within 25 m); the path ends
                // at the nearest open cell.
                let reach = sim::Fixed::from_num(if point { 20 } else { 0 });
                let near = |p: Vec2, q: Vec2| (p - q).length() <= reach;
                assert!(near(a.goal, m.position), "{} not reached", m.name);
                assert!(near(b.goal, other.position), "{} not reached", other.name);
                let (ca, cb) = (f64::from(a.cost), f64::from(b.cost));
                if (ca - cb).abs() > 0.02 * ca {
                    worst.push(format!(
                        "start {} -> {}: cost {ca}, start {} -> {}: cost {cb} (profile {profile})",
                        n.player, m.name, s.player, other.name
                    ));
                }
            }
        }
    }
    assert!(worst.is_empty(), "{worst:#?}");
}

fn check(name: &str, sketches: &std::path::Path) {
    {
        let dirs: Vec<_> = (0..2)
            .map(|k| {
                std::env::temp_dir()
                    .join(format!("mas-gen-sketch-{name}-{k}-{}", std::process::id()))
            })
            .collect();
        std::thread::scope(|scope| {
            for dir in &dirs {
                scope.spawn(move || {
                    let _ = std::fs::remove_dir_all(dir);
                    generate(&Options {
                        name,
                        sketches,
                        out: dir,
                    })
                    .unwrap();
                });
            }
        });
        for file in ["map.toml", "height.png", "terrain.png", "objects.csv"] {
            let a = std::fs::read(dirs[0].join(file)).unwrap();
            let b = std::fs::read(dirs[1].join(file)).unwrap();
            assert!(a == b, "{name}/{file} differs between two runs");
        }
        let read = |f: &str| std::fs::read(dirs[0].join(f)).unwrap();
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&String::from_utf8(read("objects.csv")).unwrap())
            .unwrap();
        assert!(map.starts().len() >= 2, "{name}: starts");
        for dir in &dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}
