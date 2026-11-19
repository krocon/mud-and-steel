//! Destruction of map objects (prompt 020): the rules and the public types.
//!
//! `data/destruction.toml` says per catalog id how many hit points an
//! object has, when it counts as damaged, what remains when it is destroyed
//! (a ruin, a felled tree, a gap, a flattened fence) and which vehicles
//! crush it. The state of every object and its effect on paths, sight and
//! projectiles: `crate::sim::destruction`.

use crate::combat::Vec3;
use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::objects::Blocks;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_DESTRUCTION: &str = include_str!("../../../data/destruction.toml");

#[derive(Debug, PartialEq, Eq)]
pub struct DestructionError(pub String);

impl fmt::Display for DestructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid destruction rules: {}", self.0)
    }
}

impl std::error::Error for DestructionError {}

/// Damage stage of a map object.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ObjectStage {
    #[default]
    Intact,
    Damaged,
    /// A ruin, a felled tree, a gap or a flattened fence (see [`Remains`]).
    Destroyed,
}

impl ObjectStage {
    pub const fn code(self) -> u8 {
        match self {
            Self::Intact => 0,
            Self::Damaged => 1,
            Self::Destroyed => 2,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Intact => "intact",
            Self::Damaged => "damaged",
            Self::Destroyed => "destroyed",
        }
    }
}

/// What is left of a destroyed object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Remains {
    /// A lower ruin that still blocks some units, projectiles and sight.
    Ruin,
    /// A felled tree: blocks nothing and is no forest any more.
    Fallen,
    /// Nothing (walls, hedges).
    Gone,
    /// Flattened (fences, barbed wire).
    Flat,
}

impl Remains {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ruin => "ruin",
            Self::Fallen => "fallen",
            Self::Gone => "gone",
            Self::Flat => "flat",
        }
    }
}

/// The rules of one group of catalog ids.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClassRule {
    pub ids: Vec<String>,
    pub hp: u32,
    pub damaged_pct: u32,
    pub remains: Remains,
    #[serde(default = "no_blocks")]
    pub ruin_blocks: Blocks,
    #[serde(default)]
    pub ruin_height_pct: u32,
    #[serde(default)]
    pub crush_min_mass_kg: u32,
    #[serde(default)]
    pub damaged_variant: Option<String>,
    #[serde(default)]
    pub ruin_variant: Option<String>,
    /// Takes damage from the blast of big explosions around the impact
    /// point ([`BlastRules`], prompt 085), not only from direct hits.
    #[serde(default)]
    pub blast: bool,
    /// A damaged object no longer counts for sight (a tree stripped of its
    /// leaves: its bare crown does not form forest).
    #[serde(default)]
    pub damaged_clear: bool,
    /// A damaged object stops projectiles only up to this share of its
    /// catalog height (a bare tree: trunk and main branches).
    #[serde(default = "full_height")]
    pub damaged_height_pct: u32,
}

const fn full_height() -> u32 {
    100
}

const fn no_blocks() -> Blocks {
    Blocks::None
}

impl ClassRule {
    /// Hit points at which an object counts as damaged.
    pub const fn damaged_hp(&self) -> u32 {
        // At most 100 % of a u32: fits in u64.
        (self.hp as u64 * self.damaged_pct as u64 / 100) as u32
    }

    /// Stage of an object with `hp` hit points left.
    pub const fn stage_at(&self, hp: u32) -> ObjectStage {
        if hp == 0 {
            ObjectStage::Destroyed
        } else if hp <= self.damaged_hp() {
            ObjectStage::Damaged
        } else {
            ObjectStage::Intact
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DamageRules {
    pub min_caliber_mm: u32,
}

/// Damage of a big explosion to the objects around its impact point
/// (prompt 085; only classes with `blast = true`). Explosions below
/// `min_caliber_mm` do nothing (so a mortar barrage does not add up). At
/// the impact point an explosion of `caliber` mm deals `damage_per_mm ×
/// (caliber − base_caliber_mm)` hit points, falling linearly to 0 at
/// `radius_mm_per_mm × caliber` (at most `max_radius_m`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlastRules {
    pub min_caliber_mm: u32,
    pub base_caliber_mm: u32,
    pub damage_per_mm: u32,
    pub radius_mm_per_mm: u32,
    pub max_radius_m: u32,
}

impl BlastRules {
    /// Damage at the impact point and radius (mm) of an explosion of
    /// `caliber_mm_x10`; `None` below the threshold.
    pub fn reach(&self, caliber_mm_x10: u32) -> Option<(u32, u32)> {
        let base = self.base_caliber_mm * 10;
        if caliber_mm_x10 < self.min_caliber_mm * 10
            || caliber_mm_x10 <= base
            || self.damage_per_mm == 0
        {
            return None;
        }
        let damage = u64::from(caliber_mm_x10 - base) * u64::from(self.damage_per_mm) / 10;
        let radius = (u64::from(caliber_mm_x10) * u64::from(self.radius_mm_per_mm) / 10)
            .min(u64::from(self.max_radius_m) * 1000);
        (radius > 0).then(|| {
            (
                u32::try_from(damage).unwrap_or(u32::MAX),
                u32::try_from(radius).unwrap_or(u32::MAX),
            )
        })
    }
}

/// Vehicles breaking through objects that block them (walls, prompt 093):
/// classes with `crush_min_mass_kg` whose objects block large units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrushRules {
    /// Path cost of a cell with such an object, as meters of detour: a
    /// vehicle breaks through only where the way around is longer.
    pub detour_m: u32,
    /// Speed of a vehicle that just broke an object with hit points, in
    /// percent of its speed (0 or 100: no slow-down).
    pub slow_pct: u32,
    /// Ticks the slow-down lasts per broken object.
    pub slow_ticks: u32,
    /// Most ticks of slow-down a vehicle collects.
    pub slow_max_ticks: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    damage: DamageRules,
    #[serde(default)]
    blast: Option<BlastRules>,
    #[serde(default)]
    crush: CrushRules,
    class: Vec<ClassRule>,
}

/// All destruction rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructionRules {
    pub damage: DamageRules,
    /// Area damage of big explosions (`None`: only direct hits).
    #[serde(default)]
    pub blast: Option<BlastRules>,
    /// Breaking through walls (prompt 093).
    #[serde(default)]
    pub crush: CrushRules,
    pub classes: Vec<ClassRule>,
}

impl DestructionRules {
    pub fn parse(text: &str) -> Result<Self, DestructionError> {
        let file: File = toml::from_str(text).map_err(|e| DestructionError(e.to_string()))?;
        for (n, class) in file.class.iter().enumerate() {
            let err = |what: &str| DestructionError(format!("class {}: {what}", n + 1));
            if class.ids.is_empty() {
                return Err(err("needs at least one id"));
            }
            if class.damaged_pct >= 100 {
                return Err(err("damaged_pct must be below 100"));
            }
            if class.hp == 0 && class.crush_min_mass_kg == 0 {
                return Err(err("needs hp or crush_min_mass_kg"));
            }
            if class.ruin_height_pct > 100 {
                return Err(err("ruin_height_pct above 100"));
            }
            if class.damaged_height_pct > 100 {
                return Err(err("damaged_height_pct above 100"));
            }
            if class.remains != Remains::Ruin
                && (class.ruin_blocks != Blocks::None || class.ruin_height_pct != 0)
            {
                return Err(err("ruin_blocks / ruin_height_pct need remains = \"ruin\""));
            }
            for id in &class.ids {
                if file.class[..n].iter().any(|c| c.ids.contains(id)) {
                    return Err(err(&format!("'{id}' is in two classes")));
                }
            }
        }
        if file.crush.slow_pct > 100 {
            return Err(DestructionError("crush: slow_pct above 100".into()));
        }
        Ok(Self {
            damage: file.damage,
            blast: file.blast,
            crush: file.crush,
            classes: file.class,
        })
    }

    /// The rules from `data/destruction.toml`.
    pub fn builtin() -> Result<Self, DestructionError> {
        Self::parse(BUILTIN_DESTRUCTION)
    }

    /// Index of the class of a catalog id.
    pub fn class_of(&self, id: &str) -> Option<usize> {
        self.classes
            .iter()
            .position(|c| c.ids.iter().any(|i| i == id))
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.damage.min_caliber_mm);
        match self.blast {
            None => h.write_u8(0),
            Some(b) => {
                h.write_u8(1);
                h.write_u32(b.min_caliber_mm);
                h.write_u32(b.base_caliber_mm);
                h.write_u32(b.damage_per_mm);
                h.write_u32(b.radius_mm_per_mm);
                h.write_u32(b.max_radius_m);
            }
        }
        h.write_u32(self.crush.detour_m);
        h.write_u32(self.crush.slow_pct);
        h.write_u32(self.crush.slow_ticks);
        h.write_u32(self.crush.slow_max_ticks);
        h.write_u64(self.classes.len() as u64);
        for c in &self.classes {
            h.write_u64(c.ids.len() as u64);
            for id in &c.ids {
                h.write_bytes(id.as_bytes());
                h.write_u8(0);
            }
            h.write_u32(c.hp);
            h.write_u32(c.damaged_pct);
            h.write_bytes(c.remains.name().as_bytes());
            h.write_u8(match c.ruin_blocks {
                Blocks::None => 0,
                Blocks::Small => 1,
                Blocks::Large => 2,
                Blocks::All => 3,
            });
            h.write_u32(c.ruin_height_pct);
            h.write_u32(c.crush_min_mass_kg);
            for v in [&c.damaged_variant, &c.ruin_variant] {
                h.write_bytes(v.as_deref().unwrap_or("").as_bytes());
                h.write_u8(0);
            }
            h.write_u8(u8::from(c.blast) | u8::from(c.damaged_clear) << 1);
            h.write_u32(c.damaged_height_pct);
        }
    }
}

/// State of one map object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectState {
    /// Hit points left (0 for objects without hit points).
    pub hp: u32,
    pub stage: ObjectStage,
    /// Direction a felled tree or flattened fence fell to, `0..360`
    /// clockwise from north (like `MapObject::heading_deg`).
    pub fall_deg: u16,
}

/// Why an object changed its stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectCause {
    /// A projectile fired by this unit.
    Hit(UnitId),
    /// This vehicle drove over it.
    Crushed(UnitId),
    /// It burned (prompt 041); the unit whose hit started the fire.
    Fire(UnitId),
}

/// An object changed its stage during the last tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEvent {
    /// Index into [`crate::Map::objects`].
    pub object: u32,
    pub stage: ObjectStage,
    /// What remains (only meaningful when `stage` is destroyed).
    pub remains: Remains,
    pub fall_deg: u16,
    /// Where the hit landed (or the crushing vehicle stood).
    pub at: Vec3,
    pub cause: ObjectCause,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    #[test]
    fn builtin_rules_parse_and_name_catalog_objects() {
        let rules = DestructionRules::builtin().unwrap();
        let catalog = Catalog::builtin().unwrap();
        for class in &rules.classes {
            for id in &class.ids {
                assert!(catalog.get(id).is_some(), "{id} is not in the catalog");
            }
        }
        let house = &rules.classes[rules.class_of("env_house_farm").unwrap()];
        assert_eq!(house.stage_at(2500), ObjectStage::Intact);
        assert_eq!(house.stage_at(1251), ObjectStage::Intact);
        assert_eq!(house.stage_at(1250), ObjectStage::Damaged);
        assert_eq!(house.stage_at(0), ObjectStage::Destroyed);
        let tree = &rules.classes[rules.class_of("env_tree_birch").unwrap()];
        assert_eq!(tree.stage_at(tree.hp), ObjectStage::Intact);
        assert_eq!(tree.stage_at(1), ObjectStage::Damaged);
        assert!(tree.blast && tree.damaged_clear);
        let blast = rules.blast.unwrap();
        // Mortars and light guns do not thin a forest; heavy charges do.
        assert!(blast.reach(800).is_none());
        assert!(blast.reach(750).is_none());
        let (h105, r105) = blast.reach(1050).unwrap();
        let (h132, r132) = blast.reach(1320).unwrap();
        assert!(h105 < tree.hp && h105 >= tree.hp - tree.damaged_hp());
        assert!(h132 > tree.hp && r132 > r105);
        assert!(rules.class_of("fort_bunker_mg").is_none());
    }

    #[test]
    fn rules_reject_bad_classes() {
        let bad = [
            "[damage]\nmin_caliber_mm = 20\n[[class]]\nids = []\nhp = 1\ndamaged_pct = 0\nremains = \"gone\"",
            "[damage]\nmin_caliber_mm = 20\n[[class]]\nids = [\"a\"]\nhp = 0\ndamaged_pct = 0\nremains = \"gone\"",
            "[damage]\nmin_caliber_mm = 20\n[[class]]\nids = [\"a\"]\nhp = 1\ndamaged_pct = 100\nremains = \"gone\"",
            "[damage]\nmin_caliber_mm = 20\n[[class]]\nids = [\"a\"]\nhp = 1\ndamaged_pct = 0\nremains = \"gone\"\nruin_height_pct = 5",
            "[damage]\nmin_caliber_mm = 20\n[[class]]\nids = [\"a\"]\nhp = 1\ndamaged_pct = 0\nremains = \"gone\"\n[[class]]\nids = [\"a\"]\nhp = 1\ndamaged_pct = 0\nremains = \"gone\"",
        ];
        for text in bad {
            assert!(DestructionRules::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn variants_exist_in_the_object_rules() {
        let rules = DestructionRules::builtin().unwrap();
        let objects = crate::ObjectRules::builtin().unwrap();
        for class in &rules.classes {
            for id in &class.ids {
                for v in [&class.damaged_variant, &class.ruin_variant]
                    .into_iter()
                    .flatten()
                {
                    assert!(objects.find(id, v).is_some(), "{id} has no variant {v}");
                }
            }
        }
    }
}
