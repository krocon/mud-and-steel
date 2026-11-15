//! Map objects: trees, buildings, walls, hedges and props placed on a map.
//!
//! A map lists its objects in `objects.csv` (one object per line: catalog
//! id, model variant, position in millimeters, heading in degrees). The
//! rules in `data/objects.toml` say which path size classes an object
//! blocks and which ground shape it has. Both are resolved when the map is
//! loaded ([`crate::Map::add_objects`]) and become part of the map hash; the
//! path grid rasterizes the shapes into its cells. Format and reasons:
//! `docs/MAPS.md`.

use crate::catalog::Catalog;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// The object rules compiled into the simulation.
pub const BUILTIN_OBJECTS: &str = include_str!("../../../data/objects.toml");

/// Header line of `objects.csv`.
pub const CSV_HEADER: &str = "kind,variant,x_mm,y_mm,heading_deg";

/// Most objects a map may hold.
pub const MAX_OBJECTS: usize = 4_000_000;

#[derive(Debug, PartialEq, Eq)]
pub struct ObjectError(pub String);

impl fmt::Display for ObjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid objects: {}", self.0)
    }
}

impl std::error::Error for ObjectError {}

/// Which path size classes an object blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Blocks {
    None,
    /// Infantry and small vehicles only (barbed wire).
    Small,
    /// Tanks and trucks only (trees, hedges, walls).
    Large,
    All,
}

impl Blocks {
    pub const fn small(self) -> bool {
        matches!(self, Self::Small | Self::All)
    }

    pub const fn large(self) -> bool {
        matches!(self, Self::Large | Self::All)
    }

    const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Small => 1,
            Self::Large => 2,
            Self::All => 3,
        }
    }
}

/// Ground shape of an object, centered on its position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Circle {
        radius: Fixed,
    },
    /// A rectangle: `half_length` along the heading, `half_width` across.
    Box {
        half_length: Fixed,
        half_width: Fixed,
    },
}

/// Drivable deck of a bridge: a box along the object's heading with the
/// object's shape length and `half_width` across.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deck {
    /// Half of the roadway width between the railings, in meters.
    pub half_width: Fixed,
    /// Heaviest unit (kg) that may cross.
    pub max_load_kg: u32,
    /// Terrain id whose speed applies on the deck.
    pub terrain: u8,
}

/// One placeable kind: a catalog id with one model variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKind {
    pub id: String,
    pub variant: String,
    pub blocks: Blocks,
    pub shape: Shape,
    /// Height from the catalog in meters (projectiles below it hit the
    /// object, prompt 017). Not part of the map hash.
    pub height: Fixed,
    /// Bridges: the deck units drive on (needs a box shape).
    pub deck: Option<Deck>,
}

/// A placed object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapObject {
    /// Index into [`crate::Map::object_kinds`].
    pub kind: u16,
    pub position: Vec2,
    /// `0..360`, clockwise seen from above; 0 = the model's +Y axis points
    /// north (-y).
    pub heading_deg: u16,
}

impl MapObject {
    /// Unit vectors `(forward, right)` of the object in map coordinates:
    /// forward is the model's +Y axis (length), right its +X axis.
    pub fn axes(&self) -> (Vec2, Vec2) {
        let (sin, cos) = sin_cos_deg(self.heading_deg);
        (Vec2::new(sin, -cos), Vec2::new(cos, sin))
    }

    /// Half extents of the axis-aligned box around the shape.
    pub fn half_extent(&self, shape: Shape) -> Vec2 {
        match shape {
            Shape::Circle { radius } => Vec2::new(radius, radius),
            Shape::Box {
                half_length,
                half_width,
            } => {
                let (f, r) = self.axes();
                Vec2::new(
                    half_length * f.x.abs() + half_width * r.x.abs(),
                    half_length * f.y.abs() + half_width * r.y.abs(),
                )
            }
        }
    }

    /// Whether the shape overlaps the open axis-aligned square around
    /// `center` with half edge `half` (touching does not count).
    pub fn overlaps_square(&self, shape: Shape, center: Vec2, half: Fixed) -> bool {
        let d = center - self.position;
        match shape {
            Shape::Circle { radius } => {
                let gap = |v: Fixed| (v.abs() - half).max(Fixed::ZERO);
                let (gx, gy) = (gap(d.x), gap(d.y));
                gx * gx + gy * gy < radius * radius
            }
            Shape::Box {
                half_length,
                half_width,
            } => {
                // Separating axis test: the two box axes and x, y.
                let extent = self.half_extent(shape);
                if d.x.abs() >= half + extent.x || d.y.abs() >= half + extent.y {
                    return false;
                }
                let (f, r) = self.axes();
                let along = |a: Vec2| d.x * a.x + d.y * a.y;
                let square = |a: Vec2| half * (a.x.abs() + a.y.abs());
                along(f).abs() < half_length + square(f) && along(r).abs() < half_width + square(r)
            }
        }
    }
}

/// `sin(k degrees)` for `k = 0..=90` as raw Q32.32 bits, `round(sin * 2^32)`. Part of the rules: changing it changes
/// object footprints and the map hash.
const SIN_BITS: [i64; 91] = [
    0, 74957515, 149892197, 224781220, 299601773, 374331065, 448946331, 523424844, 597743917,
    671880911, 745813244, 819518395, 892973913, 966157422, 1039046630, 1111619334, 1183853429,
    1255726910, 1327217885, 1398304576, 1468965330, 1539178623, 1608923068, 1678177418, 1746920580,
    1815131613, 1882789739, 1949874349, 2016365009, 2082241464, 2147483648, 2212071688, 2275985909,
    2339206844, 2401715233, 2463492036, 2524518436, 2584775843, 2644245902, 2702910498, 2760751762,
    2817752074, 2873894071, 2929160652, 2983534983, 3037000500, 3089540917, 3141140230, 3191782722,
    3241452965, 3290135830, 3337816489, 3384480416, 3430113397, 3474701533, 3518231241, 3560689261,
    3602062661, 3642338838, 3681505524, 3719550787, 3756463039, 3792231035, 3826843882, 3860291035,
    3892562305, 3923647864, 3953538241, 3982224333, 4009697400, 4035949075, 4060971360, 4084756634,
    4107297652, 4128587547, 4148619834, 4167388412, 4184887562, 4201111956, 4216056650, 4229717092,
    4242089121, 4253168970, 4262953261, 4271439016, 4278623649, 4284504972, 4289081193, 4292350918,
    4294313152, 4294967296,
];

/// `(sin, cos)` of a whole number of degrees, from a fixed table.
pub fn sin_cos_deg(deg: u16) -> (Fixed, Fixed) {
    let sin = |d: u16| -> Fixed {
        let d = d % 360;
        let (index, negative) = match d {
            0..=90 => (d, false),
            91..=180 => (180 - d, false),
            181..=270 => (d - 180, true),
            _ => (360 - d, true),
        };
        let v = Fixed::from_bits(SIN_BITS[usize::from(index)]);
        if negative { -v } else { v }
    };
    (sin(deg), sin((deg % 360 + 90) % 360))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    blocks: Option<Blocks>,
    radius_mm: Option<u32>,
    length_mm: Option<u32>,
    width_mm: Option<u32>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeckEntry {
    width_mm: u32,
    max_load_kg: u32,
    terrain: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    variants: Vec<String>,
    blocks: Blocks,
    radius_mm: Option<u32>,
    length_mm: Option<u32>,
    width_mm: Option<u32>,
    deck: Option<DeckEntry>,
    #[serde(default)]
    variant: BTreeMap<String, Override>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesFile {
    object: Vec<Entry>,
}

/// Every placeable kind with its blocking class and shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRules {
    /// One per (catalog id, variant), in file order.
    pub kinds: Vec<ObjectKind>,
}

/// Shapes must stay below 1 km.
const MAX_SIZE_MM: u32 = 1_000_000;

impl ObjectRules {
    /// Parses the rules; box sizes default to the catalog dimensions.
    pub fn parse(text: &str, catalog: &Catalog) -> Result<Self, ObjectError> {
        let file: RulesFile = toml::from_str(text).map_err(|e| ObjectError(e.to_string()))?;
        let mut kinds: Vec<ObjectKind> = Vec::new();
        for entry in file.object {
            let err = |what: String| ObjectError(format!("object '{}': {what}", entry.id));
            let catalog_entry = catalog
                .get(&entry.id)
                .ok_or_else(|| err("not in the catalog".into()))?;
            if entry.variants.is_empty() {
                return Err(err("needs at least one variant".into()));
            }
            if let Some(name) = entry.variant.keys().find(|n| !entry.variants.contains(n)) {
                return Err(err(format!("override for unknown variant '{name}'")));
            }
            for name in &entry.variants {
                if kinds.iter().any(|k| k.id == entry.id && &k.variant == name) {
                    return Err(err(format!("duplicate variant '{name}'")));
                }
                let o = entry.variant.get(name);
                let pick = |base: Option<u32>, over: Option<Option<u32>>| over.flatten().or(base);
                let radius = pick(entry.radius_mm, o.map(|o| o.radius_mm));
                let shape = if let Some(radius) = radius {
                    Shape::Circle {
                        radius: mm(radius, MAX_SIZE_MM).map_err(err)?,
                    }
                } else {
                    let length = pick(entry.length_mm, o.map(|o| o.length_mm))
                        .or(catalog_entry.length_mm)
                        .ok_or_else(|| err("needs radius_mm or a length".into()))?;
                    let width = pick(entry.width_mm, o.map(|o| o.width_mm))
                        .or(catalog_entry.width_mm)
                        .ok_or_else(|| err("needs radius_mm or a width".into()))?;
                    Shape::Box {
                        half_length: mm(length, MAX_SIZE_MM).map_err(err)? / 2,
                        half_width: mm(width, MAX_SIZE_MM).map_err(err)? / 2,
                    }
                };
                let deck = match (entry.deck, shape) {
                    (None, _) => None,
                    (Some(d), Shape::Box { half_width, .. }) => {
                        let half = mm(d.width_mm, MAX_SIZE_MM).map_err(&err)? / 2;
                        if half > half_width {
                            return Err(err("deck wider than the object".into()));
                        }
                        Some(Deck {
                            half_width: half,
                            max_load_kg: d.max_load_kg,
                            terrain: d.terrain,
                        })
                    }
                    (Some(_), Shape::Circle { .. }) => {
                        return Err(err("a deck needs a box shape".into()));
                    }
                };
                kinds.push(ObjectKind {
                    id: entry.id.clone(),
                    variant: name.clone(),
                    blocks: o.and_then(|o| o.blocks).unwrap_or(entry.blocks),
                    shape,
                    height: Fixed::from_num(catalog_entry.height_mm.unwrap_or(0))
                        / Fixed::from_num(1000),
                    deck,
                });
            }
        }
        Ok(Self { kinds })
    }

    /// The rules from `data/objects.toml` with the built-in catalog.
    pub fn builtin() -> Result<Self, ObjectError> {
        let catalog = Catalog::builtin().map_err(|e| ObjectError(e.to_string()))?;
        Self::parse(BUILTIN_OBJECTS, &catalog)
    }

    pub fn find(&self, id: &str, variant: &str) -> Option<&ObjectKind> {
        self.kinds
            .iter()
            .find(|k| k.id == id && k.variant == variant)
    }
}

fn mm(value: u32, max: u32) -> Result<Fixed, String> {
    if value == 0 || value > max {
        return Err(format!("size {value} mm out of range 1..={max}"));
    }
    Ok(Fixed::from_num(value) / Fixed::from_num(1000))
}

/// Parsed `objects.csv`: the kinds it uses (in order of first use) and the
/// objects in file order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectList {
    pub kinds: Vec<ObjectKind>,
    pub objects: Vec<MapObject>,
}

impl ObjectList {
    /// Parses `objects.csv`. Every object must lie within `half` (half the
    /// map width and depth) of the origin.
    pub fn parse(text: &str, rules: &ObjectRules, half: Vec2) -> Result<Self, ObjectError> {
        let mut list = Self::default();
        // Rule index -> index in `list.kinds`.
        let mut used: BTreeMap<usize, u16> = BTreeMap::new();
        let mut header = false;
        for (number, line) in (1u32..).zip(text.lines()) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let err = |what: &str| ObjectError(format!("line {number}: {what}"));
            if !header {
                if line != CSV_HEADER {
                    return Err(err(&format!("expected the header '{CSV_HEADER}'")));
                }
                header = true;
                continue;
            }
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            let [kind, variant, x, y, heading] = fields[..] else {
                return Err(err("expected 5 fields"));
            };
            let rule = rules
                .kinds
                .iter()
                .position(|k| k.id == kind && k.variant == variant)
                .ok_or_else(|| err(&format!("unknown object '{kind}' variant '{variant}'")))?;
            let int = |s: &str| {
                s.parse::<i32>()
                    .map_err(|_| err(&format!("bad number '{s}'")))
            };
            let position = Vec2::from_millimeters(int(x)?, int(y)?);
            if position.x.abs() > half.x || position.y.abs() > half.y {
                return Err(err("object lies outside the map"));
            }
            let heading_deg = heading
                .parse::<u16>()
                .ok()
                .filter(|&h| h < 360)
                .ok_or_else(|| err("heading_deg must be 0..360"))?;
            let next = list.kinds.len();
            let kind = match used.get(&rule) {
                Some(&k) => k,
                None => {
                    let k = u16::try_from(next).map_err(|_| err("too many kinds"))?;
                    used.insert(rule, k);
                    list.kinds.push(rules.kinds[rule].clone());
                    k
                }
            };
            if list.objects.len() == MAX_OBJECTS {
                return Err(err("too many objects"));
            }
            list.objects.push(MapObject {
                kind,
                position,
                heading_deg,
            });
        }
        if !header {
            return Err(ObjectError(format!("missing the header '{CSV_HEADER}'")));
        }
        Ok(list)
    }

    /// Feeds kinds (with their resolved rules) and objects into a hash.
    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.kinds.len() as u64);
        for k in &self.kinds {
            h.write_bytes(k.id.as_bytes());
            h.write_u8(0);
            h.write_bytes(k.variant.as_bytes());
            h.write_u8(0);
            h.write_u8(k.blocks.code());
            match k.shape {
                Shape::Circle { radius } => {
                    h.write_u8(0);
                    h.write_i64(radius.to_bits());
                }
                Shape::Box {
                    half_length,
                    half_width,
                } => {
                    h.write_u8(1);
                    h.write_i64(half_length.to_bits());
                    h.write_i64(half_width.to_bits());
                }
            }
            // Kinds without a deck hash exactly as before decks existed.
            if let Some(deck) = k.deck {
                h.write_u8(2);
                h.write_i64(deck.half_width.to_bits());
                h.write_u32(deck.max_load_kg);
                h.write_u8(deck.terrain);
            }
        }
        h.write_u64(self.objects.len() as u64);
        for o in &self.objects {
            h.write_u32(u32::from(o.kind));
            h.write_i64(o.position.x.to_bits());
            h.write_i64(o.position.y.to_bits());
            h.write_u32(u32::from(o.heading_deg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> ObjectRules {
        ObjectRules::builtin().unwrap()
    }

    fn half() -> Vec2 {
        Vec2::from_meters(100, 100)
    }

    #[test]
    fn builtin_rules_resolve_shapes() {
        let rules = rules();
        let oak = rules.find("env_tree_deciduous", "Oak").unwrap();
        assert_eq!(oak.blocks, Blocks::Large);
        assert_eq!(
            oak.shape,
            Shape::Circle {
                radius: Fixed::from_num(1) / 2
            }
        );
        // Boxes default to the catalog: farmhouse 14 x 9 m.
        let house = rules.find("env_house_farm", "Ruin").unwrap();
        assert_eq!(house.blocks, Blocks::All);
        assert_eq!(
            house.shape,
            Shape::Box {
                half_length: Fixed::from_num(7),
                half_width: Fixed::from_num(9) / 2
            }
        );
        // Per-variant override.
        assert_eq!(
            rules.find("env_rock", "Large").unwrap().blocks,
            Blocks::Large
        );
        assert_eq!(
            rules.find("env_rock", "Small").unwrap().blocks,
            Blocks::None
        );
    }

    #[test]
    fn rules_reject_bad_entries() {
        let catalog = Catalog::builtin().unwrap();
        let bad = [
            "[[object]]\nid = \"nope\"\nvariants = [\"A\"]\nblocks = \"all\"\nradius_mm = 1",
            "[[object]]\nid = \"env_rock\"\nvariants = []\nblocks = \"all\"\nradius_mm = 1",
            "[[object]]\nid = \"env_rock\"\nvariants = [\"A\"]\nblocks = \"all\"\nradius_mm = 0",
            "[[object]]\nid = \"env_rock\"\nvariants = [\"A\"]\nblocks = \"some\"\nradius_mm = 1",
            "[[object]]\nid = \"env_rock\"\nvariants = [\"A\"]\nblocks = \"all\"\n\
             [object.variant.B]\nblocks = \"all\"",
        ];
        for text in bad {
            assert!(ObjectRules::parse(text, &catalog).is_err(), "{text}");
        }
    }

    #[test]
    fn csv_parses_and_reuses_kinds() {
        let text = "# comment\nkind,variant,x_mm,y_mm,heading_deg\n\
                    env_tree_deciduous,Oak,1000,-2000,0\n\
                    env_house_farm,Intact,5000,5000,90\n\
                    env_tree_deciduous,Oak,-1500,0,359\n";
        let list = ObjectList::parse(text, &rules(), half()).unwrap();
        assert_eq!(list.kinds.len(), 2);
        assert_eq!(list.objects.len(), 3);
        assert_eq!(list.objects[2].kind, 0);
        assert_eq!(list.objects[1].heading_deg, 90);
        assert_eq!(list.objects[0].position, Vec2::from_meters(1, -2));
    }

    #[test]
    fn csv_errors_name_the_line() {
        let rules = rules();
        let cases = [
            ("env_tree_deciduous,Oak,0,0,0\n", "header"),
            (
                "kind,variant,x_mm,y_mm,heading_deg\nenv_tree_deciduous,Oak,0,0\n",
                "line 2",
            ),
            (
                "kind,variant,x_mm,y_mm,heading_deg\nenv_tree_deciduous,Elm,0,0,0\n",
                "unknown",
            ),
            (
                "kind,variant,x_mm,y_mm,heading_deg\nenv_rock,Large,0,0,360\n",
                "heading",
            ),
            (
                "kind,variant,x_mm,y_mm,heading_deg\nenv_rock,Large,100001,0,0\n",
                "outside",
            ),
            (
                "kind,variant,x_mm,y_mm,heading_deg\nenv_rock,Large,1.5,0,0\n",
                "bad number",
            ),
            ("", "header"),
        ];
        for (text, expected) in cases {
            let e = ObjectList::parse(text, &rules, half()).unwrap_err();
            assert!(e.0.contains(expected), "{text:?}: {e}");
        }
    }

    #[test]
    #[allow(clippy::float_arithmetic)]
    fn sine_table_matches_f64() {
        for deg in 0..360u16 {
            let (sin, cos) = sin_cos_deg(deg);
            let r = f64::from(deg).to_radians();
            assert!((sin.to_num::<f64>() - r.sin()).abs() < 1e-9, "sin {deg}");
            assert!((cos.to_num::<f64>() - r.cos()).abs() < 1e-9, "cos {deg}");
        }
    }

    fn object(heading_deg: u16) -> MapObject {
        MapObject {
            kind: 0,
            position: Vec2::ZERO,
            heading_deg,
        }
    }

    #[test]
    fn circle_overlaps_only_nearby_squares() {
        let shape = Shape::Circle { radius: Fixed::ONE };
        let o = object(0);
        let two = Fixed::from_num(2);
        assert!(o.overlaps_square(shape, Vec2::from_meters(2, 0), two));
        // Edge of the square at 1.0 m: touching only.
        assert!(!o.overlaps_square(shape, Vec2::from_meters(3, 0), two));
        // Corner at (0.6, 0.6): distance 0.85 < 1.
        let corner = Fixed::from_num(26) / 10;
        assert!(o.overlaps_square(shape, Vec2::new(corner, corner), two));
        // Corner at (0.8, 0.8): distance 1.13 > 1.
        let corner = Fixed::from_num(28) / 10;
        assert!(!o.overlaps_square(shape, Vec2::new(corner, corner), two));
    }

    #[test]
    fn rotated_box_uses_its_axes() {
        // A wall 10 m long, 0.5 m wide.
        let shape = Shape::Box {
            half_length: Fixed::from_num(5),
            half_width: Fixed::ONE / 4,
        };
        let half = Fixed::ONE / 2;
        // Heading 0: along y.
        assert!(object(0).overlaps_square(shape, Vec2::from_meters(0, 4), half));
        assert!(!object(0).overlaps_square(shape, Vec2::from_meters(4, 0), half));
        // Heading 90: along x.
        assert!(object(90).overlaps_square(shape, Vec2::from_meters(4, 0), half));
        assert!(!object(90).overlaps_square(shape, Vec2::from_meters(0, 4), half));
        // Heading 45: along the diagonal (north-east / south-west).
        assert!(object(45).overlaps_square(shape, Vec2::from_meters(3, -3), half));
        assert!(object(45).overlaps_square(shape, Vec2::from_meters(-3, 3), half));
        assert!(!object(45).overlaps_square(shape, Vec2::from_meters(3, 3), half));
        // The bounding box of the diagonal wall contains (3, 3), but the
        // separating axis across the wall rejects it.
        let e = object(45).half_extent(shape);
        assert!(e.x > Fixed::from_num(3) && e.y > Fixed::from_num(3));
    }

    #[test]
    fn axes_turn_clockwise_from_north() {
        let (f, r) = object(0).axes();
        assert_eq!((f, r), (Vec2::from_meters(0, -1), Vec2::from_meters(1, 0)));
        let (f, r) = object(90).axes();
        assert_eq!((f, r), (Vec2::from_meters(1, 0), Vec2::from_meters(0, 1)));
    }
}
