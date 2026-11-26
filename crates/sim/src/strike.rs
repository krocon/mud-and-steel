//! Heavy rocket strike (prompt 054): the rules of `data/strike.toml` and
//! the public view, option and event types. The update (calls, warning,
//! cancelling, the salvo): `crate::sim::strike`. See docs/ARCHITECTURE.md
//! "Heavy strike".
//!
//! The strike also carries the big-blast events the presentation dresses
//! up (prompt 054): [`StrikeEvent::AmmoExplosion`] when a destroyed
//! vehicle's ammunition goes up (`crate::sim::knockdown`).

use crate::combat::Vec3;
use crate::command::{PlayerId, UnitId};
use crate::economy::Resources;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_STRIKE: &str = include_str!("../../../data/strike.toml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrikeError(pub String);

impl fmt::Display for StrikeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid strike rules: {}", self.0)
    }
}

impl std::error::Error for StrikeError {}

/// Rocket caliber of one faction.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FactionRocket {
    pub faction: String,
    pub caliber_mm_x10: u32,
}

/// Rules of `data/strike.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StrikeRules {
    pub building: String,
    pub unlock_s: u32,
    pub cost: Resources,
    pub cooldown_s: u32,
    pub warning_s: u32,
    pub rockets: u32,
    pub ripple_ms: u32,
    pub radius_m: u32,
    pub speed_m_per_s: u32,
    pub apex_pct: u32,
    pub min_flight_ms: u32,
    #[serde(rename = "faction")]
    pub factions: Vec<FactionRocket>,
}

impl StrikeRules {
    pub fn parse(text: &str) -> Result<Self, StrikeError> {
        let r: Self = toml::from_str(text).map_err(|e| StrikeError(e.to_string()))?;
        let err = |what: &str| Err(StrikeError(what.to_owned()));
        if r.building.is_empty() {
            return err("building must be set");
        }
        if r.rockets == 0 || r.rockets > 200 {
            return err("rockets must be 1..=200");
        }
        if r.radius_m > 500 || r.warning_s > 600 || r.cooldown_s > 3600 {
            return err("radius_m above 500, warning_s above 600 or cooldown_s above 3600");
        }
        if r.speed_m_per_s == 0 || r.speed_m_per_s > 2000 || r.apex_pct > 100 {
            return err("speed_m_per_s must be 1..=2000 and apex_pct at most 100");
        }
        if r.factions.iter().any(|f| f.caliber_mm_x10 == 0) {
            return err("rocket calibers must be above 0");
        }
        Ok(r)
    }

    /// The rules from `data/strike.toml`.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static StrikeError> {
        static CACHE: std::sync::OnceLock<Result<StrikeRules, StrikeError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_STRIKE)).as_ref()
    }

    pub fn builtin() -> Result<Self, StrikeError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    /// Rocket caliber of `faction` (tenths of a millimeter).
    pub fn caliber(&self, faction: Option<&str>) -> Option<u32> {
        let faction = faction?;
        self.factions
            .iter()
            .find(|f| f.faction == faction)
            .map(|f| f.caliber_mm_x10)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_bytes(self.building.as_bytes());
        for v in [
            self.unlock_s,
            self.cooldown_s,
            self.warning_s,
            self.rockets,
            self.ripple_ms,
            self.radius_m,
            self.speed_m_per_s,
            self.apex_pct,
            self.min_flight_ms,
        ] {
            h.write_u32(v);
        }
        for c in self.cost {
            h.write_u32(c);
        }
        h.write_u64(self.factions.len() as u64);
        for f in &self.factions {
            h.write_bytes(f.faction.as_bytes());
            h.write_u32(f.caliber_mm_x10);
        }
    }
}

/// Why a player cannot call a strike now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrikeBlock {
    /// The player is defeated or unknown.
    Defeated,
    /// The faction has no rockets.
    NoRockets,
    /// No completed rocket battery position.
    NoBattery,
    /// Too early in the match.
    Locked,
    /// Called recently.
    Cooldown,
    /// The player's last strike has not been fired yet.
    Busy,
    NotEnoughResources,
}

impl StrikeBlock {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Defeated => "defeated",
            Self::NoRockets => "no rockets",
            Self::NoBattery => "no rocket battery",
            Self::Locked => "not yet available",
            Self::Cooldown => "not ready",
            Self::Busy => "strike under way",
            Self::NotEnoughResources => "not enough resources",
        }
    }
}

/// Whether and when a player can call a strike (the call bar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrikeOption {
    pub cost: Resources,
    /// Ticks until it can be called again, and the full cooldown.
    pub cooldown_left: u32,
    pub cooldown_total: u32,
    /// Ticks until the strike is unlocked (0: unlocked).
    pub unlock_left: u32,
    /// The completed own battery position that would fire.
    pub battery: Option<UnitId>,
    pub rockets: u32,
    pub radius_m: u32,
    pub warning_ticks: u32,
    /// `None` if it can be called now.
    pub blocked: Option<StrikeBlock>,
}

/// A called strike, as everybody sees it: the smoke on the target and the
/// siren are public, and the battery position is revealed to the enemies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrikeView {
    pub id: u32,
    pub owner: PlayerId,
    pub battery: UnitId,
    /// Where the battery position stands.
    pub from: Vec2,
    pub target: Vec2,
    pub radius: Fixed,
    /// Ticks until the first rocket leaves (0: firing).
    pub warning_left: u32,
    pub rockets_left: u32,
}

/// What happened with heavy strikes and big blasts during one tick, for
/// effects, sound and the HUD. Derived, not state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrikeEvent {
    /// A player called a strike: smoke marks `target`, the siren sounds.
    Called {
        strike: u32,
        owner: PlayerId,
        battery: UnitId,
        from: Vec2,
        target: Vec2,
        radius: Fixed,
        warning_ticks: u32,
    },
    /// The battery position was lost before the salvo was fired.
    Cancelled {
        strike: u32,
        owner: PlayerId,
        target: Vec2,
    },
    /// The first rocket of the salvo left the position.
    Launched {
        strike: u32,
        owner: PlayerId,
        from: Vec2,
        target: Vec2,
    },
    /// A destroyed vehicle's ammunition exploded (prompt 048 rule): the
    /// presentation blows the turret off.
    AmmoExplosion { unit: UnitId, at: Vec3 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse() {
        let r = StrikeRules::builtin().unwrap();
        assert_eq!(r.building, "base_rocket_battery");
        assert_eq!(r.caliber(Some("de")), Some(1500));
        assert_eq!(r.caliber(Some("su")), Some(1320));
        assert_eq!(r.caliber(Some("us")), Some(1140));
        assert_eq!(r.caliber(None), None);
    }

    #[test]
    fn bad_rules_are_rejected() {
        let bad = BUILTIN_STRIKE.replace("rockets = 24", "rockets = 0");
        assert!(StrikeRules::parse(&bad).is_err());
        let bad = BUILTIN_STRIKE.replace("speed_m_per_s = 300", "speed_m_per_s = 0");
        assert!(StrikeRules::parse(&bad).is_err());
    }
}
