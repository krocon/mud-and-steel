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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    damage: DamageRules,
    class: Vec<ClassRule>,
}

/// All destruction rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructionRules {
    pub damage: DamageRules,
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
        Ok(Self {
            damage: file.damage,
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
        assert_eq!(tree.stage_at(1), ObjectStage::Intact);
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
