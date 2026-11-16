//! Compounds and villages (prompt 059): groups of map objects placed as one
//! stamp.
//!
//! A compound is a house with its walled yard, gate and yard props, or a
//! landmark (mosque, police post, shop row), defined in
//! `data/compounds.toml`. Stamping it turns it in steps of 90° and expands
//! it into ordinary map objects; the courtyard wall is laid out here from
//! the yard size (segments, corner piers, the gate), leaving out wall
//! pieces inside the compound's buildings. The village brush packs
//! compounds in rows with alleys and lanes between them, deterministic
//! from the editor's seed. Integers only, like the rest of the editor.

use crate::brush::Brush;
use crate::{EditMap, PlacedObject};
use sim::objects::Shape;
use sim::{Fixed, ObjectRules, Rng};

/// The compound definitions compiled into the editor.
const BUILTIN: &str = include_str!("../../../data/compounds.toml");

/// Terrain the village brush does not build on: field tracks, roads,
/// shallow and deep water (checked every 2 m over a footprint).
const NO_VILLAGE: [u8; 4] = [1, 2, 6, 7];
/// Largest gap between compounds in a row (an alley for infantry; at least
/// half of it) and the gap between rows
/// (a lane wide enough for tanks: two path cells), in millimeters.
pub const ALLEY_MM: i32 = 4000;
pub const LANE_MM: i32 = 8000;
/// Margin a building keeps from wall pieces: a piece whose center lies
/// within the building's box plus this is left out.
const BUILDING_MARGIN_MM: i64 = 500;

/// Side of a yard at heading 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    North,
    South,
    East,
    West,
}

impl Side {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "north" => Ok(Self::North),
            "south" => Ok(Self::South),
            "east" => Ok(Self::East),
            "west" => Ok(Self::West),
            _ => Err(format!("unknown gate side '{s}'")),
        }
    }
}

/// The wall pieces compounds are built with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WallKind {
    pub kind: String,
    pub segment_mm: i32,
    pub segment: String,
    pub low: String,
    pub broken: String,
    pub gate: String,
    pub corner: String,
}

/// One compound definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compound {
    pub name: String,
    pub label: String,
    /// Yard size `[x, y]` at heading 0; `[0, 0]` = no wall.
    pub yard_mm: [i32; 2],
    pub gate: Option<Side>,
    /// Variant of the wall segments.
    pub wall: String,
    /// The village brush and generator may pick it.
    pub village: bool,
    /// Objects relative to the yard center at heading 0.
    pub objects: Vec<PlacedObject>,
}

/// All compounds of `data/compounds.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compounds {
    pub wall: WallKind,
    pub list: Vec<Compound>,
}

fn table_str(t: &toml::Table, key: &str, what: &str) -> Result<String, String> {
    t.get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{what}: '{key}' missing or not a string"))
}

fn table_int(t: &toml::Table, key: &str, what: &str) -> Result<i32, String> {
    t.get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|v| i32::try_from(v).ok())
        .ok_or_else(|| format!("{what}: '{key}' missing or not an integer"))
}

impl Compounds {
    /// The compounds compiled into the editor.
    pub fn builtin() -> Result<Self, String> {
        Self::parse(BUILTIN)
    }

    /// Parses and checks a compounds file (sizes; kinds are checked
    /// against the object rules when stamping and in a test).
    pub fn parse(text: &str) -> Result<Self, String> {
        let root: toml::Table = text.parse().map_err(|e| format!("compounds: {e}"))?;
        let w = root
            .get("wall")
            .and_then(toml::Value::as_table)
            .ok_or("compounds: [wall] missing")?;
        let wall = WallKind {
            kind: table_str(w, "kind", "[wall]")?,
            segment_mm: table_int(w, "segment_mm", "[wall]")?,
            segment: table_str(w, "segment", "[wall]")?,
            low: table_str(w, "low", "[wall]")?,
            broken: table_str(w, "broken", "[wall]")?,
            gate: table_str(w, "gate", "[wall]")?,
            corner: table_str(w, "corner", "[wall]")?,
        };
        if wall.segment_mm <= 0 {
            return Err("compounds: [wall] segment_mm must be positive".into());
        }
        let mut list = Vec::new();
        for c in root
            .get("compound")
            .and_then(toml::Value::as_array)
            .ok_or("compounds: no [[compound]]")?
        {
            let c = c.as_table().ok_or("compounds: [[compound]] is no table")?;
            let name = table_str(c, "name", "[[compound]]")?;
            let what = format!("compound '{name}'");
            let yard = c
                .get("yard_mm")
                .and_then(toml::Value::as_array)
                .ok_or_else(|| format!("{what}: yard_mm missing"))?;
            let yard: Vec<i32> = yard
                .iter()
                .filter_map(|v| v.as_integer().and_then(|v| i32::try_from(v).ok()))
                .collect();
            let [yx, yy] = yard[..] else {
                return Err(format!("{what}: yard_mm must be [x, y]"));
            };
            for v in [yx, yy] {
                if v < 0 || v % wall.segment_mm != 0 || (v == 0) != (yx == 0) {
                    return Err(format!(
                        "{what}: yard sides must both be 0 or multiples of {} mm",
                        wall.segment_mm
                    ));
                }
            }
            let gate = match c.get("gate").and_then(toml::Value::as_str) {
                Some(s) => Some(Side::parse(s).map_err(|e| format!("{what}: {e}"))?),
                None => None,
            };
            let wall_variant = match c.get("wall").and_then(toml::Value::as_str) {
                Some(v) => v.to_owned(),
                None => wall.segment.clone(),
            };
            let mut objects = Vec::new();
            for o in c
                .get("objects")
                .and_then(toml::Value::as_array)
                .ok_or_else(|| format!("{what}: objects missing"))?
            {
                let o = o
                    .as_table()
                    .ok_or_else(|| format!("{what}: an object is no table"))?;
                let heading = table_int(o, "heading_deg", &what)?;
                if !(0..360).contains(&heading) {
                    return Err(format!("{what}: heading_deg must be 0..359"));
                }
                objects.push(PlacedObject {
                    kind: table_str(o, "kind", &what)?,
                    variant: table_str(o, "variant", &what)?,
                    x_mm: table_int(o, "x_mm", &what)?,
                    y_mm: table_int(o, "y_mm", &what)?,
                    heading_deg: heading as u16,
                });
            }
            list.push(Compound {
                label: table_str(c, "label", &what)?,
                name,
                yard_mm: [yx, yy],
                gate,
                wall: wall_variant,
                village: c
                    .get("village")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false),
                objects,
            });
        }
        Ok(Self { wall, list })
    }

    /// The compound called `name`.
    pub fn find(&self, name: &str) -> Option<&Compound> {
        self.list.iter().find(|c| c.name == name)
    }

    /// Footprint `[x, y]` of a compound turned by `heading_deg` (multiple
    /// of 90): the yard, or the box around the objects without a yard.
    pub fn footprint_mm(&self, c: &Compound, rules: &ObjectRules, heading_deg: u16) -> [i32; 2] {
        let [mut x, mut y] = c.yard_mm;
        if x == 0 {
            let (mut x0, mut y0, mut x1, mut y1) = (0i64, 0i64, 0i64, 0i64);
            for o in &c.objects {
                let (hx, hy) = half_extent(rules, o);
                x0 = x0.min(i64::from(o.x_mm) - hx);
                x1 = x1.max(i64::from(o.x_mm) + hx);
                y0 = y0.min(i64::from(o.y_mm) - hy);
                y1 = y1.max(i64::from(o.y_mm) + hy);
            }
            // Symmetric around the center, so turning keeps it inside.
            x = i32::try_from(2 * x0.abs().max(x1.abs())).unwrap_or(i32::MAX);
            y = i32::try_from(2 * y0.abs().max(y1.abs())).unwrap_or(i32::MAX);
        }
        if (heading_deg / 90) % 2 == 1 {
            [y, x]
        } else {
            [x, y]
        }
    }

    /// The objects of compound `c` with its center at `(x_mm, y_mm)`,
    /// turned clockwise by `heading_deg` (rounded to a multiple of 90).
    /// With `rng`, houses start damaged (10 %) or ruined (5 %) and wall
    /// segments broken (8 %), as in a fought-over village.
    pub fn stamp(
        &self,
        c: &Compound,
        rules: &ObjectRules,
        (x_mm, y_mm): (i32, i32),
        heading_deg: u16,
        mut rng: Option<&mut Rng>,
    ) -> Vec<PlacedObject> {
        let quarter = ((u32::from(heading_deg) + 45) / 90) % 4;
        let mut local = c.objects.clone();
        local.extend(self.wall_pieces(c, rules));
        local
            .into_iter()
            .map(|mut o| {
                if let Some(rng) = rng.as_deref_mut() {
                    self.vary(&mut o, rules, rng);
                }
                let (mut dx, mut dy) = (o.x_mm, o.y_mm);
                for _ in 0..quarter {
                    // Clockwise seen from above (x east, y south).
                    (dx, dy) = (-dy, dx);
                }
                o.x_mm = x_mm.saturating_add(dx);
                o.y_mm = y_mm.saturating_add(dy);
                o.heading_deg = ((u32::from(o.heading_deg) + quarter * 90) % 360) as u16;
                o
            })
            .collect()
    }

    /// Random damage of one object (see [`Self::stamp`]).
    fn vary(&self, o: &mut PlacedObject, rules: &ObjectRules, rng: &mut Rng) {
        let roll = rng.below(100);
        if o.kind == self.wall.kind {
            if o.variant == self.wall.segment && roll < 8 {
                o.variant = self.wall.broken.clone();
            }
            return;
        }
        if o.variant != "Intact" {
            return;
        }
        let variant = if roll < 5 {
            "Ruin"
        } else if roll < 15 {
            "Damaged"
        } else {
            return;
        };
        if rules.find(&o.kind, variant).is_some() {
            o.variant = variant.into();
        }
    }

    /// Wall segments, corner piers and the gate of a compound's yard at
    /// heading 0, without the pieces inside its buildings.
    fn wall_pieces(&self, c: &Compound, rules: &ObjectRules) -> Vec<PlacedObject> {
        let [yx, yy] = c.yard_mm;
        if yx == 0 {
            return Vec::new();
        }
        let seg = self.wall.segment_mm;
        let (hx, hy) = (yx / 2, yy / 2);
        let buildings: Vec<(i64, i64, i64, i64)> = c
            .objects
            .iter()
            .filter(|o| rules.find(&o.kind, &o.variant).is_some_and(is_building))
            .map(|o| {
                let (ex, ey) = half_extent(rules, o);
                (i64::from(o.x_mm), i64::from(o.y_mm), ex, ey)
            })
            .collect();
        let inside = |x: i32, y: i32| {
            buildings.iter().any(|&(bx, by, ex, ey)| {
                (i64::from(x) - bx).abs() <= ex + BUILDING_MARGIN_MM
                    && (i64::from(y) - by).abs() <= ey + BUILDING_MARGIN_MM
            })
        };
        let mut out = Vec::new();
        let mut piece = |variant: &str, x: i32, y: i32, heading: u16| {
            if !inside(x, y) {
                out.push(PlacedObject {
                    kind: self.wall.kind.clone(),
                    variant: variant.into(),
                    x_mm: x,
                    y_mm: y,
                    heading_deg: heading,
                });
            }
        };
        // North and south sides run east-west (heading 90), east and west
        // sides north-south (heading 0).
        for (side, along_x, fixed) in [
            (Side::North, true, -hy),
            (Side::South, true, hy),
            (Side::West, false, -hx),
            (Side::East, false, hx),
        ] {
            let length = if along_x { yx } else { yy };
            let n = length / seg;
            for k in 0..n {
                let u = -length / 2 + seg / 2 + k * seg;
                let variant = if c.gate == Some(side) && k == n / 2 {
                    &self.wall.gate
                } else {
                    &c.wall
                };
                if along_x {
                    piece(variant, u, fixed, 90);
                } else {
                    piece(variant, fixed, u, 0);
                }
            }
        }
        for (x, y) in [(-hx, -hy), (hx, -hy), (-hx, hy), (hx, hy)] {
            piece(&self.wall.corner, x, y, 0);
        }
        out
    }
}

fn is_building(k: &sim::objects::ObjectKind) -> bool {
    k.blocks == sim::objects::Blocks::All
}

/// Half extents `(x, y)` in millimeters of an object's box at its heading
/// (multiples of 90°; other headings use the larger extent both ways).
pub fn half_extent(rules: &ObjectRules, o: &PlacedObject) -> (i64, i64) {
    let mm = |f: Fixed| i64::from((f * Fixed::from_num(1000)).round().to_num::<i32>());
    let Some(kind) = rules.find(&o.kind, &o.variant) else {
        return (0, 0);
    };
    let (along, across) = match kind.shape {
        Shape::Box {
            half_length,
            half_width,
        } => (mm(half_length), mm(half_width)),
        Shape::Circle { radius } => (mm(radius), mm(radius)),
    };
    match o.heading_deg % 180 {
        0 => (across, along),
        90 => (along, across),
        _ => (along.max(across), along.max(across)),
    }
}

impl EditMap {
    /// Places compound `name` centered at `(x_mm, y_mm)`, turned by
    /// `heading_deg` (rounded to 90°), as one undo step. Objects off the
    /// map are left out. Returns how many objects were placed.
    pub fn place_compound(
        &mut self,
        name: &str,
        x_mm: i32,
        y_mm: i32,
        heading_deg: u16,
    ) -> Result<usize, String> {
        let compounds = Compounds::builtin()?;
        let c = compounds
            .find(name)
            .ok_or_else(|| format!("unknown compound '{name}'"))?;
        let placed: Vec<PlacedObject> = compounds
            .stamp(c, &self.rules, (x_mm, y_mm), heading_deg, None)
            .into_iter()
            .filter(|o| self.contains(o.x_mm, o.y_mm))
            .collect();
        for o in &placed {
            self.check_kind(&o.kind, &o.variant)?;
        }
        Ok(self.add_objects(placed))
    }

    /// Village brush: packs village compounds into the circle in rows
    /// (alleys of [`ALLEY_MM`] between compounds, lanes of [`LANE_MM`]
    /// between rows), random compounds, turns and damage from the editor's
    /// seed. A compound is only placed where its whole footprint lies in
    /// the circle and on the map, no road or water is under it and no
    /// object stands within it. Returns how many compounds were placed.
    pub fn village(&mut self, brush: Brush) -> Result<usize, String> {
        let compounds = Compounds::builtin()?;
        let pool: Vec<&Compound> = compounds.list.iter().filter(|c| c.village).collect();
        if pool.is_empty() {
            return Err("no village compounds".into());
        }
        self.dabs += 1;
        let mut rng = Rng::new(self.seed ^ self.dabs.wrapping_mul(0xA24B_AED4_963E_E407));
        let r = i32::try_from(brush.radius_mm).unwrap_or(i32::MAX / 4);
        let r2 = i64::from(r).pow(2);
        let in_circle = |x: i32, y: i32| {
            (i64::from(x) - i64::from(brush.x_mm)).pow(2)
                + (i64::from(y) - i64::from(brush.y_mm)).pow(2)
                <= r2
        };
        // Footprints taken so far (existing objects count as points).
        let mut taken: Vec<(i32, i32, i32, i32)> = Vec::new();
        let mut placed = Vec::new();
        let mut count = 0;
        let mut y = brush.y_mm - r + rng.range_i32(0, ALLEY_MM);
        while y < brush.y_mm + r {
            let mut x = brush.x_mm - r + rng.range_i32(0, 2 * ALLEY_MM);
            let mut row_depth = 12_000;
            while x < brush.x_mm + r {
                let c = pool[rng.below(pool.len() as u32) as usize];
                let heading = match rng.below(10) {
                    0..=3 => 0,
                    4..=7 => 180,
                    8 => 90,
                    _ => 270,
                };
                let [w, d] = compounds.footprint_mm(c, &self.rules, heading);
                let jitter = rng.range_i32(0, 3000);
                let (x0, y0) = (x, y + jitter);
                let (x1, y1) = (x0 + w, y0 + d);
                let (cx, cy) = (x0 + w / 2, y0 + d / 2);
                let corners = [(x0, y0), (x1, y0), (x0, y1), (x1, y1), (cx, cy)];
                let fits = corners
                    .iter()
                    .all(|&(px, py)| in_circle(px, py) && self.contains(px, py))
                    && self.ground_free(x0, y0, x1, y1)
                    && !self
                        .objects
                        .iter()
                        .any(|o| o.x_mm > x0 && o.x_mm < x1 && o.y_mm > y0 && o.y_mm < y1)
                    && !taken
                        .iter()
                        .any(|&(a0, b0, a1, b1)| a0 < x1 && x0 < a1 && b0 < y1 && y0 < b1);
                if fits {
                    taken.push((x0, y0, x1, y1));
                    placed.extend(compounds.stamp(
                        c,
                        &self.rules,
                        (cx, cy),
                        heading,
                        Some(&mut rng),
                    ));
                    count += 1;
                    row_depth = row_depth.max(d + jitter);
                }
                // Alleys of 2-4 m: the yards of a row nearly touch.
                x = x1 + rng.range_i32(ALLEY_MM / 2, ALLEY_MM);
            }
            y += row_depth + LANE_MM;
        }
        self.add_objects(placed);
        Ok(count)
    }

    /// No field track, road or water under the rectangle (every 2 m).
    fn ground_free(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let step = usize::try_from(self.cell_mm).unwrap_or(2000).max(1);
        (y0..=y1).step_by(step).all(|y| {
            (x0..=x1)
                .step_by(step)
                .all(|x| !NO_VILLAGE.contains(&self.terrain_near(x, y)))
        })
    }

    /// Adds objects as one undo step; returns how many.
    fn add_objects(&mut self, placed: Vec<PlacedObject>) -> usize {
        let n = placed.len();
        if n > 0 {
            self.edit(|map| {
                map.history.touch_objects(&map.objects);
                map.objects.extend(placed);
            });
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::Biome;

    fn rules() -> ObjectRules {
        ObjectRules::builtin().unwrap()
    }

    /// Every compound parses, uses known kinds and variants, and its
    /// objects stay inside its yard.
    #[test]
    fn builtin_compounds_are_valid() {
        let compounds = Compounds::builtin().unwrap();
        let rules = rules();
        assert!(compounds.list.iter().filter(|c| c.village).count() >= 8);
        for c in &compounds.list {
            let objects = compounds.stamp(c, &rules, (0, 0), 0, None);
            for o in &objects {
                assert!(
                    rules.find(&o.kind, &o.variant).is_some(),
                    "{}: {} {}",
                    c.name,
                    o.kind,
                    o.variant
                );
            }
            if c.yard_mm[0] == 0 {
                continue;
            }
            let (hx, hy) = (i64::from(c.yard_mm[0] / 2), i64::from(c.yard_mm[1] / 2));
            for o in &c.objects {
                let (ex, ey) = half_extent(&rules, o);
                let (x, y) = (i64::from(o.x_mm), i64::from(o.y_mm));
                // Buildings may reach 0.25 m beyond the yard edge.
                assert!(
                    x - ex >= -hx - 300 && x + ex <= hx + 300,
                    "{}: {} sticks out in x",
                    c.name,
                    o.kind
                );
                assert!(
                    y - ey >= -hy - 300 && y + ey <= hy + 300,
                    "{}: {} sticks out in y",
                    c.name,
                    o.kind
                );
            }
            let gates = objects.iter().filter(|o| o.variant == "Gate").count();
            assert_eq!(gates, usize::from(c.gate.is_some()), "{}", c.name);
            assert!(objects.iter().any(|o| o.kind == compounds.wall.kind));
        }
    }

    /// Turning by 90° moves every object a quarter turn clockwise around
    /// the center and turns its heading.
    #[test]
    fn stamp_turns_in_quarters() {
        let compounds = Compounds::builtin().unwrap();
        let rules = rules();
        let c = compounds.find("hut_yard").unwrap();
        let a = compounds.stamp(c, &rules, (1000, 2000), 0, None);
        let b = compounds.stamp(c, &rules, (1000, 2000), 90, None);
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(b.x_mm - 1000, -(a.y_mm - 2000));
            assert_eq!(b.y_mm - 2000, a.x_mm - 1000);
            assert_eq!(b.heading_deg, (a.heading_deg + 90) % 360);
        }
        assert_eq!(compounds.footprint_mm(c, &rules, 90), [12000, 12000]);
    }

    /// The village brush packs compounds without overlaps, the same way
    /// every time, and one undo removes them all.
    #[test]
    fn village_brush_is_deterministic_and_undoable() {
        let run = || {
            let mut map = EditMap::new("village", 400, 400, Biome::Desert).unwrap();
            let n = map.village(Brush::new(0, 0, 120_000)).unwrap();
            (n, map.objects_csv(), map)
        };
        let (n, csv, mut map) = run();
        let (n2, csv2, _) = run();
        assert!(n >= 10, "{n} compounds");
        assert_eq!((n, &csv), (n2, &csv2));
        let houses = map
            .objects()
            .iter()
            .filter(|o| o.kind.starts_with("env_house_desert"))
            .count();
        assert_eq!(houses, n);
        // The map still loads in the simulation (no object off the map).
        map.to_sim_map().unwrap();
        map.undo();
        assert!(map.objects().is_empty());
    }

    #[test]
    fn place_compound_turns_and_checks_names() {
        let mut map = EditMap::new("fort", 256, 256, Biome::Desert).unwrap();
        let n = map.place_compound("police_post", 0, 0, 270).unwrap();
        assert!(n > 20, "{n}");
        assert!(map.place_compound("nothing", 0, 0, 0).is_err());
    }
}
