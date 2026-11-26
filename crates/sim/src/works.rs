//! Field works of engineers (prompt 035): the rules of `data/works.toml`
//! and the public types of the orders and views. The update itself (sites,
//! engineer jobs, mines, obstacles, digging, bridges): `crate::sim::works`.
//! See docs/ARCHITECTURE.md "Field works".

use crate::catalog::{Catalog, Category};
use crate::command::PlayerId;
use crate::economy::Resources;
use crate::hash::StableHasher;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_WORKS: &str = include_str!("../../../data/works.toml");

/// The shooter of mine explosions in combat events (no unit fired; like
/// `crate::air::AIR_SHOOTER`).
pub const MINE_SHOOTER: crate::command::UnitId = crate::command::UnitId(u32::MAX - 1);

/// Map object ids (`data/objects.toml`) that the map editor places as
/// hidden minefields; each becomes a section of neutral mines when a match
/// starts.
pub const MAP_MINEFIELD_AT: &str = "fort_mine_at";
pub const MAP_MINEFIELD_AP: &str = "fort_mine_ap";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorksError(pub String);

impl fmt::Display for WorksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid works rules: {}", self.0)
    }
}

impl std::error::Error for WorksError {}

/// What engineers lay (`Command::Lay`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LayKind {
    /// A section of anti-tank mines.
    MinesAt,
    /// A section of anti-personnel mines.
    MinesAp,
    /// Barbed wire: blocks infantry and small vehicles.
    Wire,
    /// Anti-tank obstacles (hedgehogs): block tanks and trucks.
    Hedgehogs,
    /// A camouflage net over a position (prompt 052).
    CamoNet,
    /// A bunker at a trench edge (prompt 052): `to` gives its facing.
    Bunker,
    /// Concrete anti-tank teeth along a line (prompt 052).
    Teeth,
}

/// What engineers dig (`Command::Dig`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DigKind {
    Foxhole,
    Trench,
    /// Prompt 052: a straight communication trench, a vehicle pit, an MG
    /// nest and a dugout.
    CommTrench,
    VehiclePit,
    MgNest,
    Dugout,
}

/// Every kind of field work. New kinds (prompt 052: vehicle pits, camouflage
/// nets) are appended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WorkKind {
    MinesAt,
    MinesAp,
    Wire,
    Hedgehogs,
    Foxhole,
    Trench,
    CommTrench,
    VehiclePit,
    MgNest,
    Dugout,
    CamoNet,
    Bunker,
    Teeth,
}

impl WorkKind {
    pub const ALL: [Self; 13] = [
        Self::MinesAt,
        Self::MinesAp,
        Self::Wire,
        Self::Hedgehogs,
        Self::Foxhole,
        Self::Trench,
        Self::CommTrench,
        Self::VehiclePit,
        Self::MgNest,
        Self::Dugout,
        Self::CamoNet,
        Self::Bunker,
        Self::Teeth,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::MinesAt => "mines_at",
            Self::MinesAp => "mines_ap",
            Self::Wire => "wire",
            Self::Hedgehogs => "hedgehogs",
            Self::Foxhole => "foxhole",
            Self::Trench => "trench",
            Self::CommTrench => "comm_trench",
            Self::VehiclePit => "vehicle_pit",
            Self::MgNest => "mg_nest",
            Self::Dugout => "dugout",
            Self::CamoNet => "camo_net",
            Self::Bunker => "bunker",
            Self::Teeth => "teeth",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    pub const fn is_mines(self) -> bool {
        matches!(self, Self::MinesAt | Self::MinesAp)
    }

    pub const fn is_dig(self) -> bool {
        matches!(
            self,
            Self::Foxhole
                | Self::Trench
                | Self::CommTrench
                | Self::VehiclePit
                | Self::MgNest
                | Self::Dugout
        )
    }

    /// Kinds of prompt 052 (rules in `[positions]`).
    pub const fn is_positions(self) -> bool {
        self.code() >= 6
    }

    /// Finished works of this kind block path cells.
    pub const fn is_obstacle(self) -> bool {
        matches!(self, Self::Wire | Self::Hedgehogs | Self::Teeth)
    }

    const fn code(self) -> u8 {
        match self {
            Self::MinesAt => 0,
            Self::MinesAp => 1,
            Self::Wire => 2,
            Self::Hedgehogs => 3,
            Self::Foxhole => 4,
            Self::Trench => 5,
            Self::CommTrench => 6,
            Self::VehiclePit => 7,
            Self::MgNest => 8,
            Self::Dugout => 9,
            Self::CamoNet => 10,
            Self::Bunker => 11,
            Self::Teeth => 12,
        }
    }

    pub(crate) fn hash_into(self, h: &mut StableHasher) {
        h.write_u8(self.code());
    }
}

impl From<LayKind> for WorkKind {
    fn from(kind: LayKind) -> Self {
        match kind {
            LayKind::MinesAt => Self::MinesAt,
            LayKind::MinesAp => Self::MinesAp,
            LayKind::Wire => Self::Wire,
            LayKind::Hedgehogs => Self::Hedgehogs,
            LayKind::CamoNet => Self::CamoNet,
            LayKind::Bunker => Self::Bunker,
            LayKind::Teeth => Self::Teeth,
        }
    }
}

impl From<DigKind> for WorkKind {
    fn from(kind: DigKind) -> Self {
        match kind {
            DigKind::Foxhole => Self::Foxhole,
            DigKind::Trench => Self::Trench,
            DigKind::CommTrench => Self::CommTrench,
            DigKind::VehiclePit => Self::VehiclePit,
            DigKind::MgNest => Self::MgNest,
            DigKind::Dugout => Self::Dugout,
        }
    }
}

/// Anti-tank or anti-personnel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MineKind {
    AntiTank,
    AntiPersonnel,
}

/// A mine as a player knows it (`Sim::mines_for`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MineView {
    pub id: u32,
    pub kind: MineKind,
    pub position: Vec2,
    /// `None`: placed with the map (neutral).
    pub owner: Option<PlayerId>,
    /// The mine belongs to the player's team (else it was discovered).
    pub friendly: bool,
}

/// A field work or construction site as a player sees it
/// (`Sim::works_for`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkView {
    pub id: u32,
    pub kind: WorkKind,
    pub owner: PlayerId,
    /// The piece: a point (`from == to`) or a line.
    pub from: Vec2,
    pub to: Vec2,
    /// Finished (an obstacle or a dug position), else a site.
    pub built: bool,
    /// Work done, per mille.
    pub progress_pml: u32,
    /// Engineers worked on it in the last tick.
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EngineerRules {
    pub categories: Vec<Category>,
    pub reach_mm: u32,
    pub detect_mm: u32,
    pub clear_radius_mm: u32,
    pub max_sites: u32,
    pub abandon_s: u32,
    pub max_line_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MineRules {
    pub at_trigger_kg: u32,
    pub ap_trigger_kg: u32,
    pub soldier_kg: u32,
    pub ap_reach_mm: u32,
    pub at_damage_pct: u32,
    pub track_repair_s: u32,
    pub at_crater_caliber_mm_x10: u32,
    pub ap_caliber_mm_x10: u32,
    pub reveal_mm: u32,
    pub map_at_count: u32,
    pub map_ap_count: u32,
    pub section_mm: u32,
    pub jitter_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DigRules {
    pub depth_mm: u32,
    pub parapet_mm: u32,
    pub steps: u32,
    pub stand_pct: u32,
    pub kneel_pct: u32,
    pub prone_pct: u32,
}

impl DigRules {
    pub const fn pct(&self, stance: crate::infantry::Stance) -> u32 {
        match stance {
            crate::infantry::Stance::Stand => self.stand_pct,
            crate::infantry::Stance::Kneel => self.kneel_pct,
            crate::infantry::Stance::Prone => self.prone_pct,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeRules {
    pub demolish_s: u32,
    pub demolish_cost: Resources,
    pub demolish_caliber_mm_x10: u32,
    pub repair_s: u32,
    pub repair_cost: Resources,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrushRules {
    pub wire_kg: u32,
}

/// Cost, build and clear time and piece length of one kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkRule {
    pub cost: Resources,
    pub build_s: u32,
    pub clear_s: u32,
    pub piece_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkTable {
    pub mines_at: WorkRule,
    pub mines_ap: WorkRule,
    pub wire: WorkRule,
    pub hedgehogs: WorkRule,
    pub foxhole: WorkRule,
    pub trench: WorkRule,
}

/// Rules of `data/works.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorksRules {
    pub engineers: EngineerRules,
    pub mines: MineRules,
    pub dig: DigRules,
    pub bridge: BridgeRules,
    pub crush: CrushRules,
    pub work: WorkTable,
    /// Prompt 052 (absent in saved games of older versions).
    #[serde(default)]
    pub positions: crate::positions::PositionRules,
}

impl WorksRules {
    pub fn parse(text: &str) -> Result<Self, WorksError> {
        let rules: Self = toml::from_str(text).map_err(|e| WorksError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(WorksError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let e = &rules.engineers;
        check("engineers.reach_mm", e.reach_mm, 1000, 50_000)?;
        check("engineers.detect_mm", e.detect_mm, 0, 100_000)?;
        check(
            "engineers.clear_radius_mm",
            e.clear_radius_mm,
            1000,
            100_000,
        )?;
        check("engineers.max_sites", e.max_sites, 1, 1000)?;
        check("engineers.abandon_s", e.abandon_s, 1, 3600)?;
        check("engineers.max_line_mm", e.max_line_mm, 1000, 2_000_000)?;
        let m = &rules.mines;
        check("mines.at_trigger_kg", m.at_trigger_kg, 1, 1_000_000)?;
        check("mines.ap_trigger_kg", m.ap_trigger_kg, 1, 1_000_000)?;
        check("mines.soldier_kg", m.soldier_kg, 1, 1000)?;
        check("mines.ap_reach_mm", m.ap_reach_mm, 100, 5000)?;
        check("mines.at_damage_pct", m.at_damage_pct, 0, 100)?;
        check("mines.track_repair_s", m.track_repair_s, 0, 3600)?;
        check(
            "mines.at_crater_caliber_mm_x10",
            m.at_crater_caliber_mm_x10,
            0,
            10_000,
        )?;
        check("mines.ap_caliber_mm_x10", m.ap_caliber_mm_x10, 1, 10_000)?;
        check("mines.reveal_mm", m.reveal_mm, 0, 100_000)?;
        check("mines.map_at_count", m.map_at_count, 1, 200)?;
        check("mines.map_ap_count", m.map_ap_count, 1, 200)?;
        check("mines.section_mm", m.section_mm, 2000, 100_000)?;
        check("mines.jitter_mm", m.jitter_mm, 0, 10_000)?;
        let d = &rules.dig;
        check("dig.depth_mm", d.depth_mm, 1, 3000)?;
        check("dig.parapet_mm", d.parapet_mm, 0, 2000)?;
        check("dig.steps", d.steps, 1, 20)?;
        for (name, v) in [
            ("dig.stand_pct", d.stand_pct),
            ("dig.kneel_pct", d.kneel_pct),
            ("dig.prone_pct", d.prone_pct),
        ] {
            check(name, v, 0, 100)?;
        }
        let b = &rules.bridge;
        check("bridge.demolish_s", b.demolish_s, 1, 3600)?;
        check("bridge.repair_s", b.repair_s, 1, 3600)?;
        check(
            "bridge.demolish_caliber_mm_x10",
            b.demolish_caliber_mm_x10,
            1,
            100_000,
        )?;
        rules.positions.check()?;
        for kind in WorkKind::ALL.into_iter().filter(|k| !k.is_positions()) {
            let r = rules.rule(kind);
            check(&format!("work.{}.build_s", kind.name()), r.build_s, 1, 3600)?;
            check(&format!("work.{}.clear_s", kind.name()), r.clear_s, 0, 3600)?;
            check(
                &format!("work.{}.piece_mm", kind.name()),
                r.piece_mm,
                0,
                100_000,
            )?;
            if matches!(
                kind,
                WorkKind::Wire | WorkKind::Hedgehogs | WorkKind::Trench
            ) && r.piece_mm < 1000
            {
                return Err(WorksError(format!(
                    "work.{}.piece_mm must be at least 1000",
                    kind.name()
                )));
            }
        }
        Ok(rules)
    }

    /// Borrowed reference to the built-in works rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static WorksError> {
        static CACHE: std::sync::OnceLock<Result<WorksRules, WorksError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_WORKS)).as_ref()
    }

    /// The rules from `data/works.toml`, compiled in.
    pub fn builtin() -> Result<Self, WorksError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    pub const fn rule(&self, kind: WorkKind) -> &WorkRule {
        if let Some(rule) = self.positions.rule(kind) {
            return rule;
        }
        match kind {
            WorkKind::MinesAt => &self.work.mines_at,
            WorkKind::MinesAp => &self.work.mines_ap,
            WorkKind::Wire => &self.work.wire,
            WorkKind::Hedgehogs => &self.work.hedgehogs,
            WorkKind::Foxhole => &self.work.foxhole,
            WorkKind::Trench => &self.work.trench,
            _ => &self.work.foxhole,
        }
    }

    /// Whether units of `category` do engineer work.
    pub fn is_engineer(&self, category: Option<Category>) -> bool {
        category.is_some_and(|c| self.engineers.categories.contains(&c))
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let e = &self.engineers;
        h.write_u64(e.categories.len() as u64);
        for c in &e.categories {
            h.write_bytes(c.code().as_bytes());
            h.write_u8(0);
        }
        for v in [
            e.reach_mm,
            e.detect_mm,
            e.clear_radius_mm,
            e.max_sites,
            e.abandon_s,
            e.max_line_mm,
        ] {
            h.write_u32(v);
        }
        let m = &self.mines;
        for v in [
            m.at_trigger_kg,
            m.ap_trigger_kg,
            m.soldier_kg,
            m.ap_reach_mm,
            m.at_damage_pct,
            m.track_repair_s,
            m.at_crater_caliber_mm_x10,
            m.ap_caliber_mm_x10,
            m.reveal_mm,
            m.map_at_count,
            m.map_ap_count,
            m.section_mm,
            m.jitter_mm,
        ] {
            h.write_u32(v);
        }
        let d = &self.dig;
        for v in [
            d.depth_mm,
            d.parapet_mm,
            d.steps,
            d.stand_pct,
            d.kneel_pct,
            d.prone_pct,
        ] {
            h.write_u32(v);
        }
        let b = &self.bridge;
        h.write_u32(b.demolish_s);
        h.write_u32(b.demolish_caliber_mm_x10);
        h.write_u32(b.repair_s);
        for v in b.demolish_cost.iter().chain(&b.repair_cost) {
            h.write_u32(*v);
        }
        h.write_u32(self.crush.wire_kg);
        // Kinds of prompt 052 are hashed with their state
        // (`crate::sim::positions`), so matches without them keep their hash.
        for kind in WorkKind::ALL.into_iter().filter(|k| !k.is_positions()) {
            let r = self.rule(kind);
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

/// A minefield section of a faction from the catalog
/// (`<faction>_minefield_at|ap`): number of mines, length and width (mm).
pub fn catalog_section(
    catalog: &Catalog,
    faction: &str,
    kind: MineKind,
) -> Option<(u32, u32, u32)> {
    let id = match kind {
        MineKind::AntiTank => format!("{faction}_minefield_at"),
        MineKind::AntiPersonnel => format!("{faction}_minefield_ap"),
    };
    let entry = catalog.get(&id)?;
    let count: u32 = entry.weapon.iter().map(|w| u32::from(w.count)).sum();
    (count > 0).then(|| {
        (
            count,
            entry.length_mm.unwrap_or(20_000),
            entry.width_mm.unwrap_or(20_000),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = WorksRules::builtin().unwrap();
        assert!(rules.is_engineer(Some(Category::InfEngineer)));
        assert!(!rules.is_engineer(Some(Category::InfRifle)));
        assert_eq!(rules.rule(WorkKind::Wire).piece_mm, 4000);
    }

    #[test]
    fn invalid_rules_are_rejected() {
        let bad = BUILTIN_WORKS.replace("at_damage_pct = 30", "at_damage_pct = 300");
        assert!(WorksRules::parse(&bad).is_err());
        let unknown = BUILTIN_WORKS.replace("[crush]", "[crush]\ncolour = 1");
        assert!(WorksRules::parse(&unknown).is_err());
    }

    #[test]
    fn catalog_sections_come_from_the_faction_entries() {
        let catalog = Catalog::builtin().unwrap();
        assert_eq!(
            catalog_section(&catalog, "de", MineKind::AntiTank).map(|s| s.0),
            Some(20)
        );
        assert_eq!(
            catalog_section(&catalog, "su", MineKind::AntiPersonnel).map(|s| s.0),
            Some(50)
        );
    }

    #[test]
    fn kinds_have_unique_names() {
        for kind in WorkKind::ALL {
            assert_eq!(WorkKind::from_name(kind.name()), Some(kind));
        }
    }
}
