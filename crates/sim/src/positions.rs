//! Positions tier 2 (prompt 052): the rules of `[positions]` in
//! `data/works.toml` and the strongpoint layout. The update (zigzag
//! trenches, firing positions, dug-in vehicles, camouflage nets, bunkers,
//! map strongpoints): `crate::sim::positions`. See docs/ARCHITECTURE.md
//! "Positions".

use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::works::{BUILTIN_WORKS, WorkKind, WorkRule, WorksError};
use serde::{Deserialize, Serialize};

/// Owner of the works of map strongpoints (no player; prompt 052).
pub const NEUTRAL_OWNER: crate::command::PlayerId = crate::command::PlayerId(u8::MAX);

/// Cover of a soldier per stance (stand, kneel, prone) in percent.
pub type StancePct = [u32; 3];

/// Cost and times of the kinds of prompt 052.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PositionWorkTable {
    pub comm_trench: WorkRule,
    pub vehicle_pit: WorkRule,
    pub mg_nest: WorkRule,
    pub dugout: WorkRule,
    pub camo_net: WorkRule,
    pub bunker: WorkRule,
    pub teeth: WorkRule,
}

/// Rules of `[positions]` (prompt 052).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PositionRules {
    pub zigzag_mm: u32,
    pub spot_spacing_mm: u32,
    pub wall_mm: u32,
    pub close_mm: u32,
    pub steep_pct: u32,
    pub edge_mm: u32,
    pub nest_pct: StancePct,
    pub dugout_pct: StancePct,
    pub nest_radius_mm: u32,
    pub dugout_radius_mm: u32,
    pub pit_radius_mm: u32,
    pub pit_reach_mm: u32,
    pub enter_s: u32,
    pub net_radius_mm: u32,
    pub spot_mm: u32,
    pub fire_reveal_s: u32,
    pub bunkers: Vec<String>,
    pub strongpoint_prefix: String,
    pub ring_mm: u32,
    pub ring_sides: u32,
    pub wire_mm: u32,
    pub work: PositionWorkTable,
}

impl Default for PositionRules {
    fn default() -> Self {
        Self::builtin().expect("data/works.toml has valid [positions] rules")
    }
}

impl PositionRules {
    /// Checks the value ranges (called by `WorksRules::parse`).
    pub fn check(&self) -> Result<(), WorksError> {
        let r = self;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(WorksError(format!(
                    "positions.{name} = {value} outside {min}..={max}"
                )))
            }
        };
        check("zigzag_mm", r.zigzag_mm, 0, 4000)?;
        check("spot_spacing_mm", r.spot_spacing_mm, 500, 10_000)?;
        check("wall_mm", r.wall_mm, 0, 2000)?;
        check("close_mm", r.close_mm, 0, 500_000)?;
        check("steep_pct", r.steep_pct, 1, 10_000)?;
        check("edge_mm", r.edge_mm, 1000, 50_000)?;
        for (name, v) in [("nest_pct", r.nest_pct), ("dugout_pct", r.dugout_pct)] {
            for p in v {
                check(name, p, 0, 100)?;
            }
        }
        check("nest_radius_mm", r.nest_radius_mm, 0, 10_000)?;
        check("dugout_radius_mm", r.dugout_radius_mm, 0, 10_000)?;
        check("pit_radius_mm", r.pit_radius_mm, 0, 10_000)?;
        check("pit_reach_mm", r.pit_reach_mm, 500, 20_000)?;
        check("enter_s", r.enter_s, 0, 600)?;
        check("net_radius_mm", r.net_radius_mm, 1000, 30_000)?;
        check("spot_mm", r.spot_mm, 0, 1_000_000)?;
        check("fire_reveal_s", r.fire_reveal_s, 0, 3600)?;
        check("ring_mm", r.ring_mm, 4000, 100_000)?;
        check("ring_sides", r.ring_sides, 3, 32)?;
        check("wire_mm", r.wire_mm, 0, 200_000)?;
        if r.strongpoint_prefix.is_empty() {
            return Err(WorksError("positions.strongpoint_prefix is empty".into()));
        }
        for kind in WorkKind::ALL.into_iter().filter(|k| k.is_positions()) {
            let w = r.rule(kind).expect("kinds of prompt 052");
            check(&format!("work.{}.build_s", kind.name()), w.build_s, 1, 3600)?;
            check(&format!("work.{}.clear_s", kind.name()), w.clear_s, 0, 3600)?;
            check(
                &format!("work.{}.piece_mm", kind.name()),
                w.piece_mm,
                0,
                100_000,
            )?;
        }
        for kind in [WorkKind::CommTrench, WorkKind::Teeth] {
            if r.rule(kind).is_some_and(|w| w.piece_mm < 1000) {
                return Err(WorksError(format!(
                    "positions.work.{}.piece_mm must be at least 1000",
                    kind.name()
                )));
            }
        }
        Ok(())
    }

    /// The rules from `data/works.toml`, compiled in.
    pub fn builtin() -> Result<Self, WorksError> {
        crate::works::WorksRules::parse(BUILTIN_WORKS).map(|w| w.positions)
    }

    /// The rule of a kind of prompt 052 (`None` for the kinds of 035).
    pub const fn rule(&self, kind: WorkKind) -> Option<&WorkRule> {
        let w = &self.work;
        Some(match kind {
            WorkKind::CommTrench => &w.comm_trench,
            WorkKind::VehiclePit => &w.vehicle_pit,
            WorkKind::MgNest => &w.mg_nest,
            WorkKind::Dugout => &w.dugout,
            WorkKind::CamoNet => &w.camo_net,
            WorkKind::Bunker => &w.bunker,
            WorkKind::Teeth => &w.teeth,
            _ => return None,
        })
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        for v in [
            self.zigzag_mm,
            self.spot_spacing_mm,
            self.wall_mm,
            self.close_mm,
            self.steep_pct,
            self.edge_mm,
        ] {
            h.write_u32(v);
        }
        for v in self.nest_pct.iter().chain(&self.dugout_pct) {
            h.write_u32(*v);
        }
        for v in [
            self.nest_radius_mm,
            self.dugout_radius_mm,
            self.pit_radius_mm,
            self.pit_reach_mm,
            self.enter_s,
            self.net_radius_mm,
            self.spot_mm,
            self.fire_reveal_s,
            self.ring_mm,
            self.ring_sides,
            self.wire_mm,
        ] {
            h.write_u32(v);
        }
        h.write_u64(self.bunkers.len() as u64);
        for b in &self.bunkers {
            h.write_bytes(b.as_bytes());
            h.write_u8(0);
        }
        h.write_bytes(self.strongpoint_prefix.as_bytes());
        h.write_u8(0);
        for kind in WorkKind::ALL {
            if let Some(r) = self.rule(kind) {
                kind.hash_into(h);
                for v in r.cost {
                    h.write_u32(v);
                }
                h.write_u32(r.build_s);
                h.write_u32(r.clear_s);
                h.write_u32(r.piece_mm);
            }
        }
    }
}

/// One piece of a strongpoint layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutPiece {
    pub kind: WorkKind,
    pub from: Vec2,
    pub to: Vec2,
}

/// The works of a strongpoint around `center` (map markers
/// `strongpoint_<name>`, prompt 052): a zigzag fire trench ring of
/// `ring_sides` sides at `ring_mm`, an MG nest in every second corner, two
/// communication trenches from the ring to two dugouts beside the center,
/// and a wire ring at `wire_mm` with two gaps. Pure geometry (the
/// simulation and the map editor's preview use it); the simulation drops
/// pieces on impassable ground.
pub fn strongpoint_layout(rules: &PositionRules, center: Vec2) -> Vec<LayoutPiece> {
    let mut out = Vec::new();
    let sides = rules.ring_sides.max(3);
    let ring = Fixed::from_num(rules.ring_mm) / 1000;
    let corner = |k: u32, r: Fixed| {
        // Start at 22.5° for eight sides so that no side faces straight
        // along an axis.
        let deg = u16::try_from(360 * (2 * k + 1) / (2 * sides)).unwrap_or(0);
        let (sin, cos) = crate::objects::sin_cos_deg(deg);
        center + Vec2::new(cos * r, sin * r)
    };
    for k in 0..sides {
        out.push(LayoutPiece {
            kind: WorkKind::Trench,
            from: corner(k, ring),
            to: corner(k + 1, ring),
        });
    }
    for k in (0..sides).step_by(2) {
        let p = corner(k, ring + Fixed::ONE);
        out.push(LayoutPiece {
            kind: WorkKind::MgNest,
            from: p,
            to: p,
        });
    }
    // Communication trenches from the middle of two opposite sides to the
    // dugouts 5 m beside the center.
    let half = sides / 2;
    for k in [0, half] {
        let a = corner(k, ring);
        let b = corner(k + 1, ring);
        let mid = a + (b - a).scale(Fixed::ONE / 2);
        let dir = mid - center;
        let len = dir.length();
        if len == Fixed::ZERO {
            continue;
        }
        let unit = Vec2::new(dir.x / len, dir.y / len);
        let dugout = center + unit.scale(Fixed::from_num(5));
        out.push(LayoutPiece {
            kind: WorkKind::CommTrench,
            from: mid,
            to: dugout + unit.scale(Fixed::from_num(2)),
        });
        out.push(LayoutPiece {
            kind: WorkKind::Dugout,
            from: dugout,
            to: dugout,
        });
    }
    if rules.wire_mm > 0 {
        let wire = Fixed::from_num(rules.wire_mm) / 1000;
        let n = sides * 2;
        let ring_corner = |k: u32| {
            let deg = u16::try_from(360 * k / n).unwrap_or(0);
            let (sin, cos) = crate::objects::sin_cos_deg(deg);
            center + Vec2::new(cos * wire, sin * wire)
        };
        for k in 0..n {
            // Two gaps (lanes for the own troops) on opposite sides.
            if k == n / 4 || k == 3 * n / 4 {
                continue;
            }
            out.push(LayoutPiece {
                kind: WorkKind::Wire,
                from: ring_corner(k),
                to: ring_corner(k + 1),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = PositionRules::builtin().unwrap();
        assert!(rules.rule(WorkKind::CamoNet).is_some());
        assert!(rules.rule(WorkKind::Trench).is_none());
        assert_eq!(rules.bunkers.len(), 4);
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let bad = BUILTIN_WORKS.replace("ring_sides = 8", "ring_sides = 2");
        assert!(crate::works::WorksRules::parse(&bad).is_err());
    }

    #[test]
    fn strongpoints_ring_the_center() {
        let rules = PositionRules::builtin().unwrap();
        let c = Vec2::from_meters(10, -20);
        let pieces = strongpoint_layout(&rules, c);
        let trench = pieces.iter().filter(|p| p.kind == WorkKind::Trench).count();
        assert_eq!(trench, rules.ring_sides as usize);
        assert_eq!(
            pieces.iter().filter(|p| p.kind == WorkKind::Dugout).count(),
            2
        );
        for p in pieces.iter().filter(|p| p.kind == WorkKind::Trench) {
            let r = (p.from - c).length();
            assert!((r - Fixed::from_num(16)).abs() < Fixed::from_num(1) / 10);
        }
    }
}
