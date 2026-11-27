//! Infantry rules (`data/infantry.toml`, prompt 022): squad formation,
//! stances, cover and buildings. The update itself lives in
//! `sim/infantry.rs`. See docs/INFANTRY.md "Behavior".

use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The infantry rules compiled into the simulation.
pub const BUILTIN_INFANTRY: &str = include_str!("../../../data/infantry.toml");

/// Body position of a squad.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Stance {
    #[default]
    Stand,
    Kneel,
    Prone,
}

impl Stance {
    pub const fn index(self) -> usize {
        match self {
            Self::Stand => 0,
            Self::Kneel => 1,
            Self::Prone => 2,
        }
    }
}

/// What the player ordered: a fixed stance, or automatic (stand while
/// moving, kneel when standing still, prone under fire).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StanceOrder {
    #[default]
    Auto,
    Fixed(Stance),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationRules {
    pub spacing_mm: u32,
    pub row_size: u32,
    pub catch_up_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StanceValues {
    pub speed_pct: u32,
    pub height_mm: u32,
    pub dispersion_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StanceRules {
    pub change_ms: u32,
    pub suppressed_ms: u32,
    pub near_miss_mm: u32,
    pub stand: StanceValues,
    pub kneel: StanceValues,
    pub prone: StanceValues,
}

impl StanceRules {
    pub const fn values(&self, stance: Stance) -> &StanceValues {
        match stance {
            Stance::Stand => &self.stand,
            Stance::Kneel => &self.kneel,
            Stance::Prone => &self.prone,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoldierRules {
    pub half_width_mm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverObject {
    pub id: String,
    pub pct: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRules {
    pub search_mm: u32,
    pub spot_spacing_mm: u32,
    pub gap_mm: u32,
    pub reach_mm: u32,
    #[serde(default, rename = "object")]
    pub objects: Vec<CoverObject>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingKind {
    pub id: String,
    pub capacity: u32,
    pub exposure_pct: u32,
    pub heavy_exposure_pct: u32,
}

/// Blasts knock soldiers down (prompt 048, `[knockdown]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnockdownRules {
    pub radius_mm_per_mm: u32,
    pub kneel_pct: u32,
    pub prone_pct: u32,
    pub cover_pct: u32,
    pub throw_mm: u32,
    /// Prompt 194: the throw grows with the charge, `throw_mm_per_mm` per
    /// mm of caliber (never below `throw_mm`, at most `throw_max_mm`).
    pub throw_mm_per_mm: u32,
    pub throw_max_mm: u32,
    pub down_ms: u32,
    pub ammo_explosion_pct: u32,
    pub ammo_radius_mm: u32,
    /// Prompt 194: throw at the wreck of an ammunition explosion.
    pub ammo_throw_mm: u32,
    pub ammo_min_caliber_mm_x10: u32,
}

impl KnockdownRules {
    /// Knock-down radius of a high-explosive charge in mm.
    pub const fn radius_mm(&self, caliber_mm_x10: u32) -> u32 {
        caliber_mm_x10.saturating_mul(self.radius_mm_per_mm) / 10
    }

    /// Prompt 194: farthest throw (at the point of impact) of a
    /// high-explosive charge in mm. Cube-root scaling: the charge mass of
    /// similar shells grows with the cube of the caliber, blast effects at a
    /// distance with its cube root, so the throw grows linearly with the
    /// caliber; small charges keep `throw_mm`.
    pub fn throw_mm(&self, caliber_mm_x10: u32) -> u32 {
        let grown = caliber_mm_x10.saturating_mul(self.throw_mm_per_mm) / 10;
        grown.min(self.throw_max_mm).max(self.throw_mm)
    }

    /// Share of the radius that knocks a soldier in `stance` down.
    pub const fn stance_pct(&self, stance: Stance) -> u32 {
        match stance {
            Stance::Stand => 100,
            Stance::Kneel => self.kneel_pct,
            Stance::Prone => self.prone_pct,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildingRules {
    pub enter_mm: u32,
    pub heavy_caliber_mm_x10: u32,
    /// Extra exposure of occupants in a damaged building (prompt 020).
    pub damaged_extra_pct: u32,
    /// Chance that a round striking a ruin hits a soldier in its rubble.
    pub ruin_exposure_pct: u32,
    /// Chance of every occupant to die when the building collapses.
    pub collapse_loss_pct: u32,
    #[serde(default, rename = "kind")]
    pub kinds: Vec<BuildingKind>,
}

/// Hand grenades and smoke grenades for infantry squads (prompt 232,
/// `[grenades]` in `data/infantry.toml`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrenadeRules {
    pub enabled: u32,
    #[serde(default)]
    pub categories: Vec<String>,
    pub frag_stock: u32,
    pub smoke_stock: u32,
    pub range_mm: u32,
    pub cooldown_ms: u32,
    pub flight_ms: u32,
    pub fuse_ms: u32,
    pub arc_apex_mm: u32,
    pub frag_radius_mm: u32,
    pub frag_damage: u32,
    pub frag_bunker_damage: u32,
    pub frag_cover_pct: u32,
    pub frag_building_exposure_pct: u32,
    pub frag_caliber_mm_x10: u32,
    pub smoke_radius_mm: u32,
    pub smoke_lifetime_ms: u32,
}

impl Default for GrenadeRules {
    fn default() -> Self {
        Self {
            enabled: 0,
            categories: Vec::new(),
            frag_stock: 0,
            smoke_stock: 0,
            range_mm: 24_000,
            cooldown_ms: 16_000,
            flight_ms: 2_200,
            fuse_ms: 1_000,
            arc_apex_mm: 5_000,
            frag_radius_mm: 6_000,
            frag_damage: 140,
            frag_bunker_damage: 1_400,
            frag_cover_pct: 40,
            frag_building_exposure_pct: 75,
            frag_caliber_mm_x10: 600,
            smoke_radius_mm: 14_000,
            smoke_lifetime_ms: 44_000,
        }
    }
}

impl GrenadeRules {
    /// Whether a unit of catalog `category` carries infantry grenades.
    pub fn carries(&self, category: Option<crate::catalog::Category>) -> bool {
        self.enabled == 1 && category.is_some_and(|c| self.categories.iter().any(|k| k == c.code()))
    }
}

/// All infantry rules. Part of [`crate::SimConfig`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InfantryRules {
    pub formation: FormationRules,
    pub stance: StanceRules,
    pub soldier: SoldierRules,
    pub knockdown: KnockdownRules,
    pub cover: CoverRules,
    pub building: BuildingRules,
    /// Prompt 232; missing in saved games from before (then off).
    #[serde(default)]
    pub grenades: GrenadeRules,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfantryError(pub String);

impl fmt::Display for InfantryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid infantry rules: {}", self.0)
    }
}

impl std::error::Error for InfantryError {}

impl InfantryRules {
    pub fn parse(text: &str) -> Result<Self, InfantryError> {
        let rules: Self = toml::from_str(text).map_err(|e| InfantryError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(InfantryError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let (f, s, c, b) = (
            &rules.formation,
            &rules.stance,
            &rules.cover,
            &rules.building,
        );
        check("formation.spacing_mm", f.spacing_mm, 500, 20_000)?;
        check("formation.row_size", f.row_size, 1, 20)?;
        check("formation.catch_up_pct", f.catch_up_pct, 100, 1000)?;
        check("stance.change_ms", s.change_ms, 0, 60_000)?;
        check("stance.suppressed_ms", s.suppressed_ms, 0, 600_000)?;
        check("stance.near_miss_mm", s.near_miss_mm, 0, 50_000)?;
        for (name, v) in [("stand", s.stand), ("kneel", s.kneel), ("prone", s.prone)] {
            check(&format!("stance.{name}.speed_pct"), v.speed_pct, 1, 200)?;
            check(&format!("stance.{name}.height_mm"), v.height_mm, 100, 3000)?;
            check(
                &format!("stance.{name}.dispersion_pct"),
                v.dispersion_pct,
                1,
                1000,
            )?;
        }
        check(
            "soldier.half_width_mm",
            rules.soldier.half_width_mm,
            50,
            2000,
        )?;
        let k = &rules.knockdown;
        check("knockdown.radius_mm_per_mm", k.radius_mm_per_mm, 0, 1000)?;
        check("knockdown.kneel_pct", k.kneel_pct, 0, 100)?;
        check("knockdown.prone_pct", k.prone_pct, 0, 100)?;
        check("knockdown.cover_pct", k.cover_pct, 0, 100)?;
        check("knockdown.throw_mm", k.throw_mm, 0, 10_000)?;
        check("knockdown.throw_mm_per_mm", k.throw_mm_per_mm, 0, 200)?;
        check("knockdown.throw_max_mm", k.throw_max_mm, 0, 30_000)?;
        check("knockdown.ammo_throw_mm", k.ammo_throw_mm, 0, 30_000)?;
        check("knockdown.down_ms", k.down_ms, 0, 60_000)?;
        check("knockdown.ammo_explosion_pct", k.ammo_explosion_pct, 0, 100)?;
        check("knockdown.ammo_radius_mm", k.ammo_radius_mm, 0, 50_000)?;
        check(
            "knockdown.ammo_min_caliber_mm_x10",
            k.ammo_min_caliber_mm_x10,
            0,
            10_000,
        )?;
        check("cover.search_mm", c.search_mm, 0, 50_000)?;
        check("cover.spot_spacing_mm", c.spot_spacing_mm, 300, 20_000)?;
        check("cover.gap_mm", c.gap_mm, 0, 5000)?;
        check("cover.reach_mm", c.reach_mm, 0, 20_000)?;
        for o in &c.objects {
            check(&format!("cover.object {}", o.id), o.pct, 0, 100)?;
        }
        check("building.enter_mm", b.enter_mm, 0, 100_000)?;
        check(
            "building.heavy_caliber_mm_x10",
            b.heavy_caliber_mm_x10,
            1,
            10_000,
        )?;
        check("building.damaged_extra_pct", b.damaged_extra_pct, 0, 100)?;
        check("building.ruin_exposure_pct", b.ruin_exposure_pct, 0, 100)?;
        check("building.collapse_loss_pct", b.collapse_loss_pct, 0, 100)?;
        for k in &b.kinds {
            check(
                &format!("building.kind {} capacity", k.id),
                k.capacity,
                1,
                1000,
            )?;
            check(
                &format!("building.kind {} exposure", k.id),
                k.exposure_pct,
                0,
                100,
            )?;
            check(
                &format!("building.kind {} heavy exposure", k.id),
                k.heavy_exposure_pct,
                0,
                100,
            )?;
        }
        let g = &rules.grenades;
        check("grenades.enabled", g.enabled, 0, 1)?;
        check("grenades.frag_stock", g.frag_stock, 0, 20)?;
        check("grenades.smoke_stock", g.smoke_stock, 0, 20)?;
        check("grenades.range_mm", g.range_mm, 5_000, 100_000)?;
        check("grenades.cooldown_ms", g.cooldown_ms, 0, 600_000)?;
        check("grenades.flight_ms", g.flight_ms, 200, 30_000)?;
        check("grenades.fuse_ms", g.fuse_ms, 0, 30_000)?;
        check("grenades.arc_apex_mm", g.arc_apex_mm, 500, 50_000)?;
        check("grenades.frag_radius_mm", g.frag_radius_mm, 1_000, 50_000)?;
        check("grenades.frag_damage", g.frag_damage, 1, 10_000)?;
        check(
            "grenades.frag_bunker_damage",
            g.frag_bunker_damage,
            0,
            20_000,
        )?;
        check("grenades.frag_cover_pct", g.frag_cover_pct, 0, 100)?;
        check(
            "grenades.frag_building_exposure_pct",
            g.frag_building_exposure_pct,
            0,
            100,
        )?;
        check(
            "grenades.frag_caliber_mm_x10",
            g.frag_caliber_mm_x10,
            100,
            5_000,
        )?;
        check(
            "grenades.smoke_radius_mm",
            g.smoke_radius_mm,
            2_000,
            100_000,
        )?;
        check(
            "grenades.smoke_lifetime_ms",
            g.smoke_lifetime_ms,
            1_000,
            600_000,
        )?;
        Ok(rules)
    }

    /// The rules from `data/infantry.toml`, compiled in.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static InfantryError> {
        static CACHE: std::sync::OnceLock<Result<InfantryRules, InfantryError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_INFANTRY)).as_ref()
    }

    pub fn builtin() -> Result<Self, InfantryError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    /// Soft cover of an object kind in percent (0 = none).
    pub fn cover_pct(&self, id: &str) -> u32 {
        self.cover
            .objects
            .iter()
            .find(|o| o.id == id)
            .map_or(0, |o| o.pct)
    }

    /// Building rules of an object kind, if infantry can occupy it.
    pub fn building(&self, id: &str) -> Option<&BuildingKind> {
        self.building.kinds.iter().find(|k| k.id == id)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let f = &self.formation;
        for v in [f.spacing_mm, f.row_size, f.catch_up_pct] {
            h.write_u32(v);
        }
        let s = &self.stance;
        for v in [s.change_ms, s.suppressed_ms, s.near_miss_mm] {
            h.write_u32(v);
        }
        for v in [s.stand, s.kneel, s.prone] {
            h.write_u32(v.speed_pct);
            h.write_u32(v.height_mm);
            h.write_u32(v.dispersion_pct);
        }
        h.write_u32(self.soldier.half_width_mm);
        let k = &self.knockdown;
        for v in [
            k.radius_mm_per_mm,
            k.kneel_pct,
            k.prone_pct,
            k.cover_pct,
            k.throw_mm,
            k.throw_mm_per_mm,
            k.throw_max_mm,
            k.down_ms,
            k.ammo_explosion_pct,
            k.ammo_radius_mm,
            k.ammo_throw_mm,
            k.ammo_min_caliber_mm_x10,
        ] {
            h.write_u32(v);
        }
        let c = &self.cover;
        for v in [c.search_mm, c.spot_spacing_mm, c.gap_mm, c.reach_mm] {
            h.write_u32(v);
        }
        h.write_u64(c.objects.len() as u64);
        for o in &c.objects {
            h.write_bytes(o.id.as_bytes());
            h.write_u32(o.pct);
        }
        let b = &self.building;
        h.write_u32(b.enter_mm);
        h.write_u32(b.heavy_caliber_mm_x10);
        h.write_u32(b.damaged_extra_pct);
        h.write_u32(b.ruin_exposure_pct);
        h.write_u32(b.collapse_loss_pct);
        h.write_u64(b.kinds.len() as u64);
        for k in &b.kinds {
            h.write_bytes(k.id.as_bytes());
            h.write_u32(k.capacity);
            h.write_u32(k.exposure_pct);
            h.write_u32(k.heavy_exposure_pct);
        }
        // Prompt 232: only when enabled, so older rules hash unchanged.
        let g = &self.grenades;
        if g.enabled == 1 {
            h.write_u64(g.categories.len() as u64);
            for c in &g.categories {
                h.write_bytes(c.as_bytes());
            }
            for v in [
                g.frag_stock,
                g.smoke_stock,
                g.range_mm,
                g.cooldown_ms,
                g.flight_ms,
                g.fuse_ms,
                g.arc_apex_mm,
                g.frag_radius_mm,
                g.frag_damage,
                g.frag_bunker_damage,
                g.frag_cover_pct,
                g.frag_building_exposure_pct,
                g.frag_caliber_mm_x10,
                g.smoke_radius_mm,
                g.smoke_lifetime_ms,
            ] {
                h.write_u32(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = InfantryRules::builtin().unwrap();
        assert_eq!(rules.stance.values(Stance::Prone).height_mm, 450);
        assert_eq!(rules.cover_pct("fort_sandbag_position"), 80);
        assert_eq!(rules.cover_pct("env_house_farm"), 0);
        assert_eq!(rules.building("env_house_farm").unwrap().capacity, 12);
        assert!(rules.building("env_hedge").is_none());
        // 8 cm: 4.8 m knock-down radius.
        assert_eq!(rules.knockdown.radius_mm(800), 4800);
        assert_eq!(rules.knockdown.stance_pct(Stance::Prone), 40);
        let bad = BUILTIN_INFANTRY.replace("kneel_pct = 75", "kneel_pct = 101");
        assert!(InfantryRules::parse(&bad).is_err());
        let bad = BUILTIN_INFANTRY.replace("row_size = 5", "row_size = 0");
        assert!(InfantryRules::parse(&bad).is_err());
        let unknown = format!("{BUILTIN_INFANTRY}\n[extra]\nx = 1\n");
        assert!(InfantryRules::parse(&unknown).is_err());
    }

    #[test]
    fn cover_and_building_ids_exist_in_the_object_rules() {
        let objects = crate::objects::ObjectRules::builtin().unwrap();
        let rules = InfantryRules::builtin().unwrap();
        let known = |id: &str| objects.kinds.iter().any(|o| o.id == id);
        for o in &rules.cover.objects {
            assert!(known(&o.id), "unknown cover object {}", o.id);
        }
        for k in &rules.building.kinds {
            assert!(known(&k.id), "unknown building {}", k.id);
        }
    }
}
