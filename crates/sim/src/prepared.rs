//! Prepared ground of a map (prompt 124): field works and shell craters a
//! map brings along, listed in `map.toml` (`[[field_work]]`, `[[crater]]`)
//! and made by the simulation before the first tick with the same rules as
//! works engineers dig and craters shells leave (`crate::sim::prepared`).
//! So a prepared trench gives the cover, firing bays and look of a dug one,
//! and a prepared crater the cover and mud of a shell crater. Puddles
//! (`[[puddle]]`) are presentation only.
//!
//! Field works and craters are game rules and part of the map hash (only
//! when a map has some, so every other map keeps its hash); the water flag
//! of a crater and the puddles are not. Format: `docs/MAPS.md` "Prepared
//! ground".

use crate::hash::StableHasher;
use crate::math::Vec2;
use serde::Deserialize;

/// Kind of a prepared field work (the dug and obstacle kinds of
/// `crate::works::WorkKind` a map may bring).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedWorkKind {
    /// Fire trench: zigzags like a trench engineers dig (prompt 052).
    Trench,
    /// Communication trench (straight).
    CommTrench,
    /// Barbed wire (an obstacle for infantry, flattened by vehicles).
    Wire,
    /// MG nest at a trench (one point).
    MgNest,
    /// Dugout at a trench (one point).
    Dugout,
}

impl PreparedWorkKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Trench => "trench",
            Self::CommTrench => "comm_trench",
            Self::Wire => "wire",
            Self::MgNest => "mg_nest",
            Self::Dugout => "dugout",
        }
    }

    /// Kinds placed at single points (the others run along lines).
    pub const fn is_point(self) -> bool {
        matches!(self, Self::MgNest | Self::Dugout)
    }
}

/// A prepared field work: a line of pieces (each segment of `points` is
/// ordered like an engineer's line) or, for point kinds, one work per
/// point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedWork {
    pub kind: PreparedWorkKind,
    pub points: Vec<Vec2>,
}

/// A prepared shell crater.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparedCrater {
    pub position: Vec2,
    /// Caliber of the shell (whole millimeters); the crater rules of
    /// `data/craters.toml` give its size.
    pub caliber_mm: u32,
    /// Presentation only: rain water stands in the bowl.
    pub water: bool,
}

/// A puddle on the ground (presentation only): wet hollows beside a brook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Puddle {
    pub position: Vec2,
    pub radius_mm: u32,
}

/// Everything prepared on a map.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prepared {
    pub works: Vec<PreparedWork>,
    pub craters: Vec<PreparedCrater>,
    pub puddles: Vec<Puddle>,
}

impl Prepared {
    /// Whether the map brings field works or craters (puddles do not
    /// count: they change nothing).
    pub fn has_ground(&self) -> bool {
        !self.works.is_empty() || !self.craters.is_empty()
    }

    /// Feeds the game-relevant part (works and craters, not the water
    /// flags and puddles) into the map hash.
    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0x7c);
        h.write_u64(self.works.len() as u64);
        for w in &self.works {
            h.write_bytes(w.kind.name().as_bytes());
            h.write_u64(w.points.len() as u64);
            for p in &w.points {
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
            }
        }
        h.write_u64(self.craters.len() as u64);
        for c in &self.craters {
            h.write_i64(c.position.x.to_bits());
            h.write_i64(c.position.y.to_bits());
            h.write_u32(c.caliber_mm);
        }
    }
}

/// `[[field_work]]` of `map.toml`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldWorkEntry {
    pub kind: PreparedWorkKind,
    pub points_m: Vec<[i32; 2]>,
}

/// `[[crater]]` of `map.toml`: craters of one caliber.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CraterEntry {
    pub caliber_mm: u32,
    #[serde(default)]
    pub water: bool,
    pub at_m: Vec<[i32; 2]>,
}

/// `[[puddle]]` of `map.toml`: puddles of one radius.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PuddleEntry {
    pub radius_m: u32,
    pub at_m: Vec<[i32; 2]>,
}

impl Prepared {
    /// Builds the prepared ground from the `map.toml` tables; checks that
    /// lines have two points or more, calibers and radii are sane.
    pub(crate) fn from_entries(
        works: Vec<FieldWorkEntry>,
        craters: Vec<CraterEntry>,
        puddles: Vec<PuddleEntry>,
    ) -> Result<Self, String> {
        let at = |p: [i32; 2]| Vec2::from_meters(p[0], p[1]);
        let mut out = Self::default();
        for w in works {
            let enough = if w.kind.is_point() {
                !w.points_m.is_empty()
            } else {
                w.points_m.len() >= 2
            };
            if !enough {
                return Err(format!(
                    "field_work {}: too few points ({})",
                    w.kind.name(),
                    w.points_m.len()
                ));
            }
            out.works.push(PreparedWork {
                kind: w.kind,
                points: w.points_m.into_iter().map(at).collect(),
            });
        }
        for c in craters {
            if !(10..=1000).contains(&c.caliber_mm) {
                return Err(format!(
                    "crater caliber_mm must be 10..=1000, got {}",
                    c.caliber_mm
                ));
            }
            out.craters
                .extend(c.at_m.into_iter().map(|p| PreparedCrater {
                    position: at(p),
                    caliber_mm: c.caliber_mm,
                    water: c.water,
                }));
        }
        for p in puddles {
            if !(1..=50).contains(&p.radius_m) {
                return Err(format!(
                    "puddle radius_m must be 1..=50, got {}",
                    p.radius_m
                ));
            }
            out.puddles.extend(p.at_m.into_iter().map(|q| Puddle {
                position: at(q),
                radius_mm: p.radius_m * 1000,
            }));
        }
        Ok(out)
    }

    /// The `map.toml` tables of this prepared ground (map editor, tools):
    /// craters grouped by caliber and water flag, puddles by radius, in
    /// the order they first appear. Positions are whole meters.
    pub fn to_toml(&self) -> String {
        use std::fmt::Write as _;
        let m = |v: crate::math::Fixed| v.round().to_num::<i32>();
        let list = |points: &mut dyn Iterator<Item = Vec2>| {
            let items: Vec<String> = points
                .map(|p| format!("[{}, {}]", m(p.x), m(p.y)))
                .collect();
            let mut out = String::from("[\n");
            for chunk in items.chunks(10) {
                let _ = writeln!(out, "  {},", chunk.join(", "));
            }
            out.push(']');
            out
        };
        let mut t = String::new();
        for w in &self.works {
            let _ = write!(
                t,
                "\n[[field_work]]\nkind = \"{}\"\npoints_m = {}\n",
                w.kind.name(),
                list(&mut w.points.iter().copied())
            );
        }
        let mut groups: Vec<(u32, bool)> = Vec::new();
        for c in &self.craters {
            if !groups.contains(&(c.caliber_mm, c.water)) {
                groups.push((c.caliber_mm, c.water));
            }
        }
        for (caliber, water) in groups {
            let mut points = self
                .craters
                .iter()
                .filter(|c| c.caliber_mm == caliber && c.water == water)
                .map(|c| c.position);
            let _ = write!(
                t,
                "\n[[crater]]\ncaliber_mm = {caliber}\nwater = {water}\nat_m = {}\n",
                list(&mut points)
            );
        }
        let mut radii: Vec<u32> = Vec::new();
        for p in &self.puddles {
            if !radii.contains(&p.radius_mm) {
                radii.push(p.radius_mm);
            }
        }
        for r in radii {
            let mut points = self
                .puddles
                .iter()
                .filter(|p| p.radius_mm == r)
                .map(|p| p.position);
            let _ = write!(
                t,
                "\n[[puddle]]\nradius_m = {}\nat_m = {}\n",
                r / 1000,
                list(&mut points)
            );
        }
        t
    }
}
