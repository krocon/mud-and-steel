//! Rivers, bridges and vehicle camouflage (prompt 055): the rules of
//! `data/crossings.toml` and the public views and events. The update itself
//! (collapsing decks, swimming and drowning, bridge-laying tanks,
//! camouflage, paratroopers landing): `crate::sim::crossings`. See
//! docs/ARCHITECTURE.md "Bridges, bridge-laying tanks and camouflage".

use crate::Vec3;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_CROSSINGS: &str = include_str!("../../../data/crossings.toml");

/// The "shooter" of drowned squads in combat events (nobody fired; like
/// `crate::air::AIR_SHOOTER`).
pub const WATER_SHOOTER: UnitId = UnitId(u32::MAX - 55);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossingError(pub String);

impl fmt::Display for CrossingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid crossing rules: {}", self.0)
    }
}

impl std::error::Error for CrossingError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SwimRules {
    pub speed_mm_per_s: u32,
    pub breath_min_s: u32,
    pub breath_max_s: u32,
    pub search_m: u32,
    pub fall_loss_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CollapseRules {
    pub deck_reach_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayerRules {
    pub lay_s: u32,
    pub pickup_s: u32,
    pub reach_m: u32,
    pub search_m: u32,
    pub bearing_mm: u32,
    pub max_grade_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CamouflageRules {
    pub apply_s: u32,
    pub settle_s: u32,
    pub seen_pct: u32,
}

/// One bridge kind that heavy hits bring down (prompt 115).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShellKind {
    /// Catalog id of the bridge object (`data/objects.toml`, with a deck).
    pub id: String,
    /// Hit points of one intact piece.
    pub hp: u32,
}

/// Bridges shot to pieces (prompt 115): heavy rounds and explosions on the
/// deck cost hit points as in prompt 020 (caliber² × factor).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShellingRules {
    /// Smaller rounds do nothing (tenths of a millimeter).
    pub min_caliber_mm_x10: u32,
    /// The kinds, `[[shelling.kind]]`.
    #[serde(default, rename = "kind")]
    pub kinds: Vec<ShellKind>,
}

impl ShellingRules {
    /// Hit points of a piece of kind `id`, `None` if it cannot be shot down.
    pub fn hp_of(&self, id: &str) -> Option<u32> {
        self.kinds.iter().find(|k| k.id == id).map(|k| k.hp)
    }
}

/// Rules of `data/crossings.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrossingRules {
    pub swim: SwimRules,
    pub collapse: CollapseRules,
    pub layer: LayerRules,
    pub camouflage: CamouflageRules,
    /// Prompt 115; absent in older saved games.
    #[serde(default)]
    pub shelling: ShellingRules,
}

impl CrossingRules {
    pub fn parse(text: &str) -> Result<Self, CrossingError> {
        let rules: Self = toml::from_str(text).map_err(|e| CrossingError(e.to_string()))?;
        for k in &rules.shelling.kinds {
            if k.hp == 0 {
                return Err(CrossingError(format!(
                    "shelling kind '{}': hp must be above 0",
                    k.id
                )));
            }
        }
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(CrossingError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let (s, c, l, k) = (
            &rules.swim,
            &rules.collapse,
            &rules.layer,
            &rules.camouflage,
        );
        check("swim.speed_mm_per_s", s.speed_mm_per_s, 1, 5000)?;
        check("swim.breath_min_s", s.breath_min_s, 1, 600)?;
        check("swim.breath_max_s", s.breath_max_s, s.breath_min_s, 600)?;
        check("swim.search_m", s.search_m, 4, 500)?;
        check("swim.fall_loss_pct", s.fall_loss_pct, 0, 100)?;
        check("collapse.deck_reach_mm", c.deck_reach_mm, 100, 10_000)?;
        check("layer.lay_s", l.lay_s, 1, 3600)?;
        check("layer.pickup_s", l.pickup_s, 1, 3600)?;
        check("layer.reach_m", l.reach_m, 2, 100)?;
        check("layer.search_m", l.search_m, 0, 200)?;
        check("layer.bearing_mm", l.bearing_mm, 0, 5000)?;
        check("layer.max_grade_pct", l.max_grade_pct, 0, 100)?;
        check("camouflage.apply_s", k.apply_s, 0, 3600)?;
        check("camouflage.settle_s", k.settle_s, 0, 600)?;
        check("camouflage.seen_pct", k.seen_pct, 1, 100)?;
        Ok(rules)
    }

    /// The rules from `data/crossings.toml`, compiled in.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static CrossingError> {
        static CACHE: std::sync::OnceLock<Result<CrossingRules, CrossingError>> =
            std::sync::OnceLock::new();
        CACHE
            .get_or_init(|| Self::parse(BUILTIN_CROSSINGS))
            .as_ref()
    }

    pub fn builtin() -> Result<Self, CrossingError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let (s, c, l, k) = (&self.swim, &self.collapse, &self.layer, &self.camouflage);
        for v in [
            s.speed_mm_per_s,
            s.breath_min_s,
            s.breath_max_s,
            s.search_m,
            s.fall_loss_pct,
            c.deck_reach_mm,
            l.lay_s,
            l.pickup_s,
            l.reach_m,
            l.search_m,
            l.bearing_mm,
            l.max_grade_pct,
            k.apply_s,
            k.settle_s,
            k.seen_pct,
        ] {
            h.write_u32(v);
        }
        let sh = &self.shelling;
        h.write_u32(sh.min_caliber_mm_x10);
        h.write_u64(sh.kinds.len() as u64);
        for k in &sh.kinds {
            h.write_bytes(k.id.as_bytes());
            h.write_u8(0);
            h.write_u32(k.hp);
        }
    }
}

/// What a bridge-laying tank does (for the presentation and the AI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerState {
    /// Carries its bridge.
    Carrying,
    /// Drives to the gap it will span.
    Driving,
    /// Lays the bridge: done in thousandths.
    Laying(u32),
    /// Its bridge lies across a gap (index into `Map::bridges`).
    Laid(u32),
    /// Drives to its laid bridge to take it up.
    Fetching(u32),
    /// Takes its bridge up: done in thousandths.
    PickingUp(u32, u32),
    /// Its bridge was destroyed.
    Lost,
}

/// A bridge-laying tank of the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayerView {
    pub unit: UnitId,
    pub owner: PlayerId,
    pub state: LayerState,
    /// Length (m) and load limit (kg) of its bridge.
    pub span_mm: u32,
    pub load_kg: u32,
}

/// A gap a bridge-laying tank can span: the deck it would lay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gap {
    /// Ends of the deck (on the banks).
    pub start: Vec2,
    pub end: Vec2,
    /// Length (m, rounded down) of the impassable ground it spans.
    pub width_m: u32,
}

/// A squad in deep water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwimmerView {
    pub unit: UnitId,
    /// Where it swims to (`None`: no bank in reach).
    pub bank: Option<Vec2>,
}

/// Paratroopers under their canopies (prompt 055).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParachuteView {
    pub id: u32,
    pub owner: PlayerId,
    /// Index into `SimConfig::unit_types` of the squad.
    pub unit_type: u32,
    /// Center of the stick now, and where it lands.
    pub position: Vec3,
    pub landing: Vec2,
    /// Soldiers of the squad (one canopy each).
    pub soldiers: u16,
    pub ticks_left: u32,
}

/// What happened at rivers and bridges during one tick, for effects and
/// sound. Derived, not state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossingEvent {
    /// A deck broke (demolished) and fell into the river.
    Collapsed { bridge: u32, at: Vec3 },
    /// A unit fell with a collapsing deck.
    Fell { unit: UnitId, at: Vec3 },
    /// A soldier of `unit` drowned.
    Drowned {
        unit: UnitId,
        soldier: u16,
        at: Vec3,
    },
    /// A squad reached the bank.
    Ashore { unit: UnitId, at: Vec2 },
    /// A bridge-laying tank laid (`laid`) or took up its bridge.
    Bridge {
        unit: UnitId,
        bridge: u32,
        laid: bool,
    },
    /// Paratroopers landed as squad `unit`.
    Landed { unit: UnitId, at: Vec2, water: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = CrossingRules::builtin().unwrap();
        assert!(rules.swim.breath_max_s >= rules.swim.breath_min_s);
        assert!(
            CrossingRules::parse(&BUILTIN_CROSSINGS.replace("seen_pct = 40", "seen_pct = 0"))
                .is_err()
        );
        assert!(
            CrossingRules::parse(&BUILTIN_CROSSINGS.replace("[layer]", "[layer]\nx = 1")).is_err()
        );
    }
}
