//! Fire (prompt 041): the rules of `data/fire.toml` and the public types.
//!
//! Map objects of a fire class (woods, hedges, hay, buildings, dumps) catch
//! fire from hits, shell bursts, explosions and burning neighbors; vehicle
//! wrecks burn for a while. A fire damages its object through the hit
//! points of `data/destruction.toml`, spreads deterministically and burns
//! out; dumps explode. The update: `crate::sim::fire`.

use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_FIRE: &str = include_str!("../../../data/fire.toml");

#[derive(Debug, PartialEq, Eq)]
pub struct FireError(pub String);

impl fmt::Display for FireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid fire rules: {}", self.0)
    }
}

impl std::error::Error for FireError {}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IgniteRules {
    pub min_caliber_mm: u32,
    pub burst_min_caliber_mm: u32,
    pub burst_radius_mm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadRules {
    pub interval_ms: u32,
    /// Terrain type names fire does not cross.
    pub firebreak: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FireSmokeRules {
    pub interval_ms: u32,
    pub lifetime_ms: u32,
    pub max_clouds: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WreckRules {
    pub burn_ms: u32,
    pub reach_mm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OccupantRules {
    pub damage_per_s: u32,
}

/// How one group of catalog ids burns.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FireClass {
    pub ids: Vec<String>,
    pub hit_pct: u32,
    pub catch_pct: u32,
    pub reach_mm: u32,
    pub burn_ms: u32,
    pub burn_hp_pct: u32,
    #[serde(default)]
    pub smoke_radius_mm: u32,
    #[serde(default)]
    pub blast_caliber_mm: u32,
    #[serde(default)]
    pub fuse_ms: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    ignite: IgniteRules,
    spread: SpreadRules,
    smoke: FireSmokeRules,
    wreck: WreckRules,
    occupants: OccupantRules,
    class: Vec<FireClass>,
}

/// All fire rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireRules {
    pub ignite: IgniteRules,
    pub spread: SpreadRules,
    pub smoke: FireSmokeRules,
    pub wreck: WreckRules,
    pub occupants: OccupantRules,
    pub classes: Vec<FireClass>,
}

/// Longest time a rule may name (one hour).
const MAX_MS: u32 = 3_600_000;
/// Longest distance a rule may name (100 m).
const MAX_MM: u32 = 100_000;

impl FireRules {
    pub fn parse(text: &str) -> Result<Self, FireError> {
        let file: File = toml::from_str(text).map_err(|e| FireError(e.to_string()))?;
        let err = |what: String| Err(FireError(what));
        if file.spread.interval_ms == 0 || file.spread.interval_ms > MAX_MS {
            return err("spread.interval_ms out of range".into());
        }
        if file.smoke.interval_ms == 0 || file.smoke.lifetime_ms == 0 {
            return err("smoke times must be positive".into());
        }
        if file.wreck.burn_ms > MAX_MS || file.wreck.reach_mm > MAX_MM {
            return err("wreck values out of range".into());
        }
        if file.ignite.burst_radius_mm > MAX_MM {
            return err("ignite.burst_radius_mm out of range".into());
        }
        for (n, class) in file.class.iter().enumerate() {
            let at = |what: &str| format!("class {}: {what}", n + 1);
            if class.ids.is_empty() {
                return err(at("needs at least one id"));
            }
            if class.hit_pct > 100 || class.catch_pct > 100 || class.burn_hp_pct > 100 {
                return err(at("percentages must be at most 100"));
            }
            if class.burn_ms == 0 || class.burn_ms > MAX_MS {
                return err(at("burn_ms out of range"));
            }
            if class.reach_mm > MAX_MM || class.smoke_radius_mm > MAX_MM {
                return err(at("distance out of range"));
            }
            if class.blast_caliber_mm > 0 && (class.fuse_ms == 0 || class.fuse_ms > class.burn_ms) {
                return err(at("an explosive class needs 0 < fuse_ms <= burn_ms"));
            }
            for id in &class.ids {
                if file.class[..n].iter().any(|c| c.ids.contains(id)) {
                    return err(at(&format!("'{id}' is in two classes")));
                }
            }
        }
        Ok(Self {
            ignite: file.ignite,
            spread: file.spread,
            smoke: file.smoke,
            wreck: file.wreck,
            occupants: file.occupants,
            classes: file.class,
        })
    }

    /// The rules from `data/fire.toml`.
    pub fn builtin() -> Result<Self, FireError> {
        Self::parse(BUILTIN_FIRE)
    }

    /// Index of the class of a catalog id.
    pub fn class_of(&self, id: &str) -> Option<usize> {
        self.classes
            .iter()
            .position(|c| c.ids.iter().any(|i| i == id))
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let i = &self.ignite;
        for v in [
            i.min_caliber_mm,
            i.burst_min_caliber_mm,
            i.burst_radius_mm,
            self.spread.interval_ms,
            self.smoke.interval_ms,
            self.smoke.lifetime_ms,
            self.smoke.max_clouds,
            self.wreck.burn_ms,
            self.wreck.reach_mm,
            self.occupants.damage_per_s,
        ] {
            h.write_u32(v);
        }
        h.write_u64(self.spread.firebreak.len() as u64);
        for name in &self.spread.firebreak {
            h.write_bytes(name.as_bytes());
            h.write_u8(0);
        }
        h.write_u64(self.classes.len() as u64);
        for c in &self.classes {
            h.write_u64(c.ids.len() as u64);
            for id in &c.ids {
                h.write_bytes(id.as_bytes());
                h.write_u8(0);
            }
            for v in [
                c.hit_pct,
                c.catch_pct,
                c.reach_mm,
                c.burn_ms,
                c.burn_hp_pct,
                c.smoke_radius_mm,
                c.blast_caliber_mm,
                c.fuse_ms,
            ] {
                h.write_u32(v);
            }
        }
    }
}

/// What is burning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FireSource {
    /// Map object (index into [`crate::Map::objects`]).
    Object(u32),
    /// The wreck of this vehicle.
    Wreck(UnitId),
}

/// What happened to a fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireEventKind {
    /// It caught fire.
    Ignited,
    /// A dump exploded.
    Exploded,
    /// The fire burned out.
    BurntOut,
}

impl FireEventKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ignited => "ignited",
            Self::Exploded => "exploded",
            Self::BurntOut => "burnt_out",
        }
    }
}

/// A fire changed during the last tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireEvent {
    pub source: FireSource,
    pub kind: FireEventKind,
    /// Ground position of the fire.
    pub at: Vec2,
}

/// A fire that is burning now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireView {
    pub source: FireSource,
    pub at: Vec2,
    /// Ticks since it caught fire and until it burns out.
    pub age_ticks: u32,
    pub ticks_left: u32,
    /// Radius of the flames in meters (half the larger side of the object,
    /// at least 1 m), for the presentation.
    pub size_m: crate::math::Fixed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let rules = FireRules::builtin().unwrap();
        assert!(rules.class_of("env_tree_conifer").is_some());
        assert!(rules.class_of("env_dump_ammo").is_some());
        assert!(rules.class_of("env_rock").is_none());
    }

    #[test]
    fn every_class_can_be_damaged_and_exists() {
        let rules = FireRules::builtin().unwrap();
        let destruction = crate::DestructionRules::builtin().unwrap();
        let objects = crate::ObjectRules::builtin().unwrap();
        for class in &rules.classes {
            for id in &class.ids {
                let c = destruction
                    .class_of(id)
                    .unwrap_or_else(|| panic!("{id} has no destruction class"));
                assert!(destruction.classes[c].hp > 0, "{id} has no hit points");
                assert!(
                    objects.kinds.iter().any(|k| &k.id == id),
                    "{id} is not a map object"
                );
            }
        }
        let terrain = crate::TerrainRules::builtin().unwrap();
        for name in &rules.spread.firebreak {
            assert!(terrain.types.iter().any(|t| &t.name == name), "{name}");
        }
    }

    #[test]
    fn rules_reject_bad_classes() {
        let head = "[ignite]\nmin_caliber_mm = 20\nburst_min_caliber_mm = 50\nburst_radius_mm = 1\n\
                    [spread]\ninterval_ms = 1000\nfirebreak = []\n\
                    [smoke]\ninterval_ms = 1\nlifetime_ms = 1\nmax_clouds = 1\n\
                    [wreck]\nburn_ms = 1\nreach_mm = 1\n[occupants]\ndamage_per_s = 1\n";
        let ok = "[[class]]\nids = [\"a\"]\nhit_pct = 1\ncatch_pct = 1\nreach_mm = 1\nburn_ms = 1\nburn_hp_pct = 1\n";
        assert!(FireRules::parse(&format!("{head}{ok}")).is_ok());
        let bad = [
            "[[class]]\nids = []\nhit_pct = 1\ncatch_pct = 1\nreach_mm = 1\nburn_ms = 1\nburn_hp_pct = 1\n",
            "[[class]]\nids = [\"a\"]\nhit_pct = 101\ncatch_pct = 1\nreach_mm = 1\nburn_ms = 1\nburn_hp_pct = 1\n",
            "[[class]]\nids = [\"a\"]\nhit_pct = 1\ncatch_pct = 1\nreach_mm = 1\nburn_ms = 0\nburn_hp_pct = 1\n",
            "[[class]]\nids = [\"a\"]\nhit_pct = 1\ncatch_pct = 1\nreach_mm = 1\nburn_ms = 9\nburn_hp_pct = 1\nblast_caliber_mm = 100\n",
        ];
        for b in bad {
            assert!(FireRules::parse(&format!("{head}{b}")).is_err(), "{b}");
        }
        assert!(FireRules::parse(&format!("{head}{ok}{ok}")).is_err());
    }
}
