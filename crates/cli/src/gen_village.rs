//! Desert village map generator: `cli gen-village` (prompt 059).
//!
//! Builds an 800 m desert map for two or four players around a North
//! African village: a graded piste (north-south) crosses the village
//! square with the mosque, shop rows with market stalls and a well; walled
//! courtyard compounds packed in rows with alleys and lanes fill the
//! village (the map editor's village brush, `editor::compound`); a field
//! track leads west to an oasis (a pond in a hollow, sand banks, a palm
//! grove, a well) and east to a walled police post. Dunes with sand in the
//! open, rocks and thorn bushes. Supply points at the square, the oasis
//! well and the police post. Writes `map.toml`, `height.png`,
//! `terrain.png` and `objects.csv` (format: `docs/MAPS.md`).
//!
//! The ground is computed like `gen_town` (floats, basic arithmetic only);
//! the objects are placed through `editor::EditMap` (integers), so the
//! same seed gives the same files everywhere. No real place names.

use crate::gen_map::{Random, write_png};
use editor::compound::half_extent;
use editor::{EditMap, PlacedObject, brush::Brush};
use sim::objects::CSV_HEADER;
use sim::{ObjectRules, Rng, TerrainRules};
use std::fmt::Write as _;
use std::path::Path;

const CELL_M: f64 = 2.0;
const GROUND: u8 = 0;
const DIRT_TRACK: u8 = 1;
const ROAD: u8 = 2;
const SAND: u8 = 3;
const SHALLOW_WATER: u8 = 6;

/// The map is this wide and deep (m).
pub const SIZE_M: u32 = 800;
/// Half edge of the village square (m) and radius of the village.
const SQUARE_HALF: f64 = 24.0;
const VILLAGE_R: f64 = 165.0;
/// Oasis: pond center, water radius and hollow radius (m).
const OASIS: (f64, f64) = (-235.0, 70.0);
const POND_R: f64 = 26.0;
const HOLLOW_R: f64 = 75.0;
/// Police post (fort) center (m).
const FORT: (f64, f64) = (235.0, -30.0);
/// Oasis well (m), a supply point.
const WELL: (f64, f64) = (-188.0, 40.0);
/// Starts: team 0 south, team 1 north (players 0 and 1 face each other in
/// a two-player match).
const STARTS: [(u8, u8, i32, i32); 4] = [
    (0, 0, -90, 330),
    (1, 1, 90, -330),
    (2, 0, 90, 330),
    (3, 1, -90, -330),
];

/// Parameters of `cli gen-village`.
pub struct Options<'a> {
    pub name: &'a str,
    pub seed: u64,
    pub out: &'a Path,
}

/// Piecewise linear wiggle of the piste: x offset (m) at `y`.
fn piste_x(y: f64, bends: &[f64]) -> f64 {
    if y.abs() < 70.0 {
        return 0.0;
    }
    let half = f64::from(SIZE_M) / 2.0;
    let t = ((y + half) / 100.0).clamp(0.0, (bends.len() - 1) as f64);
    let i = (t as usize).min(bends.len() - 2);
    let f = t - i as f64;
    let raw = bends[i] * (1.0 - f) + bends[i + 1] * f;
    // Fade into the straight village stretch.
    let fade = ((y.abs() - 70.0) / 60.0).min(1.0);
    raw * fade
}

/// y (m) of the east-west track at `x`: to the oasis in the west and the
/// police post in the east.
fn track_y(x: f64) -> f64 {
    if x < 0.0 {
        let t = (-x / (-OASIS.0)).min(1.0);
        OASIS.1 * t * t
    } else {
        let t = (x / FORT.0).min(1.0);
        FORT.1 * t
    }
}

fn dist(x: f64, y: f64, (cx, cy): (f64, f64)) -> f64 {
    ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt()
}

/// Ground type at `(x, y)`.
fn terrain_at(x: f64, y: f64, bends: &[f64], dune: f64) -> u8 {
    let d_oasis = dist(x, y, OASIS);
    if d_oasis <= POND_R {
        return SHALLOW_WATER;
    }
    if x.abs() <= SQUARE_HALF && y.abs() <= SQUARE_HALF {
        return DIRT_TRACK;
    }
    if (x - piste_x(y, bends)).abs() <= 4.0 {
        return ROAD;
    }
    let to_oasis = x < 0.0 && x > OASIS.0 + POND_R;
    let to_fort = x > 0.0 && x < FORT.0 - 12.0;
    if (to_oasis || to_fort) && (y - track_y(x)).abs() <= 3.0 {
        return DIRT_TRACK;
    }
    if d_oasis <= POND_R + 7.0 {
        return SAND;
    }
    // Loose sand on the dune crests outside the village and the oasis.
    if dune > 2.2 && dist(x, y, (0.0, 0.0)) > VILLAGE_R + 30.0 && d_oasis > HOLLOW_R {
        return SAND;
    }
    GROUND
}

/// Dunes and the oasis hollow (m), flat in the village and at the fort.
fn height_at(x: f64, y: f64, bumps: &[(f64, f64, f64, f64)]) -> (f64, f64) {
    let mut h = 0.0;
    for &(bx, by, r, a) in bumps {
        let d2 = ((x - bx) * (x - bx) + (y - by) * (y - by)) / (r * r);
        if d2 < 1.0 {
            let t = 1.0 - d2;
            h += a * t * t;
        }
    }
    let flat = |d: f64, r: f64| ((d - r) / 80.0).clamp(0.0, 1.0);
    let keep = flat(dist(x, y, (0.0, 0.0)), VILLAGE_R + 15.0)
        .min(flat(dist(x, y, FORT), 30.0))
        .min(flat(dist(x, y, OASIS), HOLLOW_R));
    let dune = h * keep;
    let mut z = dune + 0.003 * y;
    let d = dist(x, y, OASIS);
    if d < HOLLOW_R {
        let t = 1.0 - d * d / (HOLLOW_R * HOLLOW_R);
        z -= 2.4 * t * t;
    }
    (z, dune)
}

/// Objects placed by the generator outside the editor's brushes, kept off
/// buildings, walls and tracks.
struct Placer<'a> {
    map: EditMap,
    rules: &'a ObjectRules,
    random: Random,
}

impl Placer<'_> {
    /// True when nothing stands within `margin_mm` of `(x, y)` (object
    /// boxes, trunks as points) and the ground is no track, road or water.
    fn free(&self, x: i32, y: i32, margin_mm: i64) -> bool {
        if !self.map.contains(x, y) || [1, 2, 6, 7].contains(&self.map.terrain_near(x, y)) {
            return false;
        }
        self.map.objects().iter().all(|o| {
            let (ex, ey) = half_extent(self.rules, o);
            (i64::from(x) - i64::from(o.x_mm)).abs() > ex + margin_mm
                || (i64::from(y) - i64::from(o.y_mm)).abs() > ey + margin_mm
        })
    }

    fn put(&mut self, kind: &str, variant: &str, (x, y): (f64, f64), heading: u16) -> bool {
        let (x, y) = ((x * 1000.0).round() as i32, (y * 1000.0).round() as i32);
        self.map
            .place(PlacedObject {
                kind: kind.into(),
                variant: variant.into(),
                x_mm: x,
                y_mm: y,
                heading_deg: heading,
            })
            .is_ok()
    }

    /// Scatters `kinds` on a jittered grid of `spacing` m inside the ring
    /// `r0..r1` around `center`, with probability `chance(d)`.
    fn grove(
        &mut self,
        kinds: &[(&str, &str)],
        center: (f64, f64),
        (r0, r1): (f64, f64),
        spacing: f64,
        margin_mm: i64,
        chance: impl Fn(f64) -> f64,
    ) -> usize {
        let mut n = 0;
        let steps = (r1 / spacing).ceil() as i32;
        for j in -steps..=steps {
            for i in -steps..=steps {
                let x = center.0 + f64::from(i) * spacing + self.random.range(-0.35, 0.35) * spacing;
                let y = center.1 + f64::from(j) * spacing + self.random.range(-0.35, 0.35) * spacing;
                let d = dist(x, y, center);
                let roll = self.random.unit();
                let pick = (self.random.unit() * kinds.len() as f64) as usize;
                let heading = (self.random.unit() * 360.0) as u16 % 360;
                if d < r0 || d > r1 || roll > chance(d) {
                    continue;
                }
                let (xm, ym) = ((x * 1000.0) as i32, (y * 1000.0) as i32);
                if !self.free(xm, ym, margin_mm) {
                    continue;
                }
                let (kind, variant) = kinds[pick.min(kinds.len() - 1)];
                if self.put(kind, variant, (x, y), heading) {
                    n += 1;
                }
            }
        }
        n
    }
}

/// Writes the desert village map into `options.out`.
pub fn generate(options: &Options) -> Result<String, String> {
    let rules = TerrainRules::builtin().map_err(|e| e.to_string())?;
    let object_rules = ObjectRules::builtin().map_err(|e| e.0)?;
    let size = f64::from(SIZE_M);
    let half = size / 2.0;
    let samples = (SIZE_M / 2 + 1) as usize;
    let mut random = Random(Rng::new(options.seed ^ 0x7669_6c6c));
    let bumps: Vec<(f64, f64, f64, f64)> = (0..26)
        .map(|_| {
            (
                random.range(-half, half),
                random.range(-half, half),
                random.range(35.0, 110.0),
                random.range(-1.5, 5.5),
            )
        })
        .collect();
    let bends: Vec<f64> = (0..=8).map(|_| random.range(-18.0, 18.0)).collect();

    let mut heights = Vec::with_capacity(samples * samples);
    let mut terrain = Vec::with_capacity(samples * samples);
    for j in 0..samples {
        for i in 0..samples {
            let x = i as f64 * CELL_M - half;
            let y = j as f64 * CELL_M - half;
            let (z, dune) = height_at(x, y, &bumps);
            heights.push(z);
            terrain.push(terrain_at(x, y, &bends, dune));
        }
    }
    // Starts stand on level ground.
    for &(_, _, sx, sy) in &STARTS {
        for j in 0..samples {
            for i in 0..samples {
                let x = i as f64 * CELL_M - half;
                let y = j as f64 * CELL_M - half;
                let d = dist(x, y, (f64::from(sx), f64::from(sy)));
                if d < 60.0 {
                    let k = j * samples + i;
                    let w = ((d - 30.0) / 30.0).clamp(0.0, 1.0);
                    heights[k] = heights[k] * w + 0.003 * y * (1.0 - w);
                    if terrain[k] == SAND {
                        terrain[k] = GROUND;
                    }
                }
            }
        }
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
    std::fs::write(options.out.join("objects.csv"), format!("{CSV_HEADER}\n"))
        .map_err(|e| e.to_string())?;
    let mut toml = String::new();
    let _ = write!(
        toml,
        "# Generated by `cli gen-village` (prompt 059); edit the PNGs freely (docs/MAPS.md).\n\
         name = \"{}\"\n\
         generator = \"cli gen-village --name {} --seed {}\"\n\
         cell_size_mm = 2000\n\
         samples = [{side}, {side}]\n\
         height_base_mm = {base_mm}\n\
         height_step_mm = 1\n\
         biome = \"desert\"\n\
         water_flow_deg = 180\n",
        options.name, options.name, options.seed,
    );
    for &(player, team, x, y) in &STARTS {
        let _ = write!(
            toml,
            "\n[[start]]\nplayer = {player}\nteam = {team}\nposition_m = [{x}, {y}]\n"
        );
    }
    let m = |v: f64| v.round() as i32;
    for (name, (x, y)) in [
        ("village", (0.0, 0.0)),
        ("oasis", OASIS),
        ("police_post", FORT),
        ("supply_square", (0.0, 0.0)),
        ("supply_well", WELL),
        ("supply_fort", FORT),
    ] {
        let _ = write!(
            toml,
            "\n[[marker]]\nname = \"{name}\"\nposition_m = [{}, {}]\n",
            m(x),
            m(y)
        );
    }
    std::fs::write(options.out.join("map.toml"), toml).map_err(|e| e.to_string())?;

    // Objects, through the map editor.
    let mut map = EditMap::load_dir(options.out)?;
    map.set_seed(options.seed);
    let mm = |v: f64| (v * 1000.0).round() as i32;
    // The square: mosque west, shop rows north and south, well, stalls.
    map.place_compound("mosque_yard", mm(-37.0), 0, 0)?;
    for (x, y, heading) in [
        (-16.0, -31.0, 90),
        (16.0, -31.0, 90),
        (-16.0, 31.0, 270),
        (16.0, 31.0, 270),
    ] {
        map.place_compound("shop_row", mm(x), mm(y), heading)?;
    }
    map.place_compound("police_post", mm(FORT.0), mm(FORT.1), 180)?;
    let mut placer = Placer {
        map,
        rules: &object_rules,
        random: Random(Rng::new(options.seed ^ 0x6f61_7369)),
    };
    placer.put("env_well_desert", "Frame", (11.0, 11.0), 0);
    placer.put("env_well_desert", "Frame", WELL, 30);
    for (x, y, h) in [
        (-14.0, -14.0, 0),
        (-14.0, -6.0, 0),
        (-14.0, 8.0, 0),
        (12.0, -12.0, 90),
    ] {
        placer.put("env_market_stall", if h == 0 && y > 0.0 { "Empty" } else { "Stall" }, (x, y), h);
    }
    placer.put("env_cart_wood", "Cart", (17.0, 2.0), 20);
    placer.put("env_jars", "Group", (-17.0, 16.0), 0);
    // The village: compounds in rows around the square (the editor's
    // village brush: it keeps off the piste, the tracks, the square and
    // the landmarks).
    let compounds = placer.map.village(Brush::new(0, 0, mm(VILLAGE_R) as u32))?;
    // A gardener's yard at the oasis.
    placer
        .map
        .place_compound("hut_yard", mm(WELL.0 + 6.0), mm(WELL.1 - 30.0), 180)?;

    // Palms: the oasis grove (dense near the water), single palms in the
    // village and its yards, a few along the track.
    let palms = [
        ("env_tree_palm", "Palm"),
        ("env_tree_palm", "Palm"),
        ("env_tree_palm", "PalmShort"),
        ("env_tree_palm", "PalmLeaning"),
    ];
    let mut trees = placer.grove(&palms, OASIS, (POND_R + 3.0, 90.0), 6.5, 1500, |d| {
        (1.15 - (d - POND_R) / 70.0).clamp(0.1, 0.95)
    });
    trees += placer.grove(&palms, (0.0, 0.0), (SQUARE_HALF + 10.0, VILLAGE_R), 9.0, 2500, |_| {
        0.22
    });
    trees += placer.grove(&palms, FORT, (20.0, 45.0), 8.0, 2500, |_| 0.25);
    let bushes = [("env_bush", "Round"), ("env_bush", "Wide")];
    let mut other = placer.grove(&bushes, OASIS, (POND_R + 2.0, 100.0), 9.0, 1500, |_| 0.3);
    // Thorn bushes and rocks in the open desert.
    other += placer.grove(&bushes, (0.0, 0.0), (VILLAGE_R + 20.0, 560.0), 24.0, 3000, |_| {
        0.12
    });
    let rocks = [
        ("env_rock", "Large"),
        ("env_rock", "Medium"),
        ("env_rock", "Medium"),
    ];
    other += placer.grove(&rocks, (0.0, 0.0), (VILLAGE_R + 40.0, 560.0), 30.0, 3000, |_| {
        0.1
    });

    let mut map = placer.map;
    // Keep the start areas clear.
    for &(_, _, sx, sy) in &STARTS {
        map.erase(Brush::new(sx * 1000, sy * 1000, 45_000), None);
    }
    map.save_dir(options.out)?;
    let objects = map.objects();
    let count = |prefix: &str| objects.iter().filter(|o| o.kind.starts_with(prefix)).count();
    Ok(format!(
        "wrote {}: {side} x {side} samples, heights {low:.2} .. {high:.2} m, {} objects \
         ({compounds} village compounds, {} houses, {} wall pieces, {} palms placed, {trees} trees and \
         {other} bushes and rocks scattered)",
        options.out.display(),
        objects.len(),
        count("env_house_desert"),
        count("env_wall_mud"),
        count("env_tree_palm"),
    ))
}

#[cfg(test)]
mod tests {
    use super::{Options, generate};
    use sim::Map;

    /// The generated village loads with four starts, has its compounds,
    /// landmarks and oasis, and the same seed gives the same objects.
    #[test]
    fn village_map_loads_with_its_objects() {
        let dir = std::env::temp_dir().join(format!("mas-gen-village-{}", std::process::id()));
        let options = Options {
            name: "desert_village",
            seed: 59,
            out: &dir,
        };
        let report = generate(&options).unwrap();
        let read = |f: &str| std::fs::read(dir.join(f)).unwrap();
        let first = read("objects.csv");
        generate(&options).unwrap();
        assert_eq!(first, read("objects.csv"), "not deterministic");
        let toml = String::from_utf8(read("map.toml")).unwrap();
        let mut map = Map::load(&toml, &read("height.png"), &read("terrain.png")).unwrap();
        map.add_objects(&String::from_utf8(first).unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let count = |prefix: &str| {
            map.objects()
                .iter()
                .filter(|o| map.object_kinds()[usize::from(o.kind)].id.starts_with(prefix))
                .count()
        };
        assert_eq!(map.starts().len(), 4, "{report}");
        assert!(count("env_house_desert") >= 40, "{report}");
        assert!(count("env_wall_mud") >= 300, "{report}");
        assert_eq!(count("env_mosque"), 1);
        assert_eq!(count("env_police_post"), 1);
        assert!(count("env_tree_palm") >= 80, "{report}");
        assert!(count("env_well_desert") >= 2);
    }
}
