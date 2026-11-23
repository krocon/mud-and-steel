//! Unit types as the simulation uses them, derived from the catalog.

use crate::catalog::{Catalog, Entry, Group};
use crate::combat::CombatType;
use crate::math::Fixed;
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Game properties of a unit type, already converted by [`Scale`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitType {
    /// Catalog id.
    pub name: String,
    /// Cross-country movement speed in game meters per second.
    pub speed_m_per_s: Fixed,
    /// Road speed in game meters per second (equal to `speed_m_per_s` for
    /// entries without a road speed, and for aircraft).
    pub road_speed_m_per_s: Fixed,
    /// Larger of length and width in meters (sizes are 1:1, so this is the
    /// real value). Used for formation spacing.
    pub footprint_m: Fixed,
    /// Combat mass in kilograms from the catalog (0 if unknown, e.g.
    /// infantry). Bridges carry units up to their load limit (prompt 014).
    pub mass_kg: u32,
    /// Weapons, armor and hit box (prompt 017).
    pub combat: CombatType,
    /// Troop seats and engine power (prompt 034).
    #[serde(default)]
    pub transport: crate::transport::TransportType,
}

/// Footprint of entries without dimensions (infantry): one soldier.
pub const DEFAULT_FOOTPRINT_MM: u32 = 1000;

#[derive(Debug, PartialEq, Eq)]
pub struct DataError(pub String);

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid unit data: {}", self.0)
    }
}

impl std::error::Error for DataError {}

impl UnitType {
    /// Converts a catalog entry into a unit type.
    ///
    /// Ground units get their cross-country and road speed (terrain picks
    /// one, see `data/terrain.toml`), aircraft their cruise speed (or maximum
    /// speed if no cruise speed is known) for both. Returns `None` for
    /// entries that cannot move.
    pub fn from_catalog(entry: &Entry, scale: &Scale) -> Option<Self> {
        let (speed, road) = match entry.category.group() {
            // Boats (prompt 119) drive their water speed everywhere they
            // can go.
            Group::Vehicle if entry.category.is_boat() => {
                let water = entry.speed_water_kmh_x10?;
                (water, water)
            }
            Group::Infantry | Group::Vehicle => {
                let offroad = entry.speed_offroad_kmh_x10?;
                (offroad, entry.speed_road_kmh_x10.unwrap_or(offroad))
            }
            Group::Aircraft => {
                let speed = entry.speed_cruise_kmh_x10.or(entry.speed_max_kmh_x10)?;
                (speed, speed)
            }
            Group::Gun => {
                // Towed guns move only as fast as their crew pushes them
                // (prompt 034); the speed is already a game speed.
                let speed = crate::transport::manhandle_speed(entry, scale)?;
                return Some(Self::from_parts(entry, scale, speed, speed));
            }
            Group::Fortification | Group::Environment => return None,
        };
        Some(Self::from_parts(
            entry,
            scale,
            scale.game_speed(entry.category, speed),
            scale.game_speed(entry.category, road),
        ))
    }

    /// A unit type of a catalog entry with the given game speeds.
    fn from_parts(entry: &Entry, scale: &Scale, offroad: Fixed, road: Fixed) -> Self {
        let footprint_mm = entry
            .length_mm
            .into_iter()
            .chain(entry.width_mm)
            .max()
            .unwrap_or(DEFAULT_FOOTPRINT_MM);
        Self {
            name: entry.id.clone(),
            speed_m_per_s: offroad,
            road_speed_m_per_s: road,
            footprint_m: Fixed::from_num(footprint_mm) / Fixed::from_num(1000),
            mass_kg: entry.mass_kg.unwrap_or(0),
            combat: CombatType::from_catalog(entry, scale),
            transport: crate::transport::TransportType::from_catalog(entry),
        }
    }
}

impl UnitType {
    /// A stationary base building (prompt 023): sizes in millimeters, the
    /// same armor on every face, no weapons, speed 0.
    pub fn structure(
        name: &str,
        length_mm: u32,
        width_mm: u32,
        height_mm: u32,
        armor_mm: u16,
        scale: &Scale,
    ) -> Self {
        let m = |v: u32| Fixed::from_num(v) / Fixed::from_num(1000);
        Self {
            name: name.to_owned(),
            speed_m_per_s: Fixed::ZERO,
            road_speed_m_per_s: Fixed::ZERO,
            footprint_m: m(length_mm.max(width_mm)),
            mass_kg: 0,
            combat: CombatType {
                length_m: m(length_mm),
                width_m: m(width_mm),
                height_m: m(height_mm),
                armor: crate::combat::Armor::uniform(armor_mm),
                range_ratio: (scale.direct_fire_range.num, scale.direct_fire_range.den),
                ..CombatType::default()
            },
            transport: crate::transport::TransportType::default(),
        }
    }

    /// A catalog entry as a stationary unit (fortifications and guns placed
    /// as buildings, prompt 023): its weapons and dimensions, speed 0.
    /// `armor_mm` > 0 replaces the catalog armor on every face (bunkers have
    /// none in the catalog).
    pub fn fixed_from_catalog(entry: &Entry, scale: &Scale, armor_mm: u16) -> Self {
        let footprint_mm = entry
            .length_mm
            .into_iter()
            .chain(entry.width_mm)
            .max()
            .unwrap_or(DEFAULT_FOOTPRINT_MM);
        let mut combat = CombatType::from_catalog(entry, scale);
        if armor_mm > 0 {
            combat.armor = crate::combat::Armor::uniform(armor_mm);
        }
        Self {
            name: entry.id.clone(),
            speed_m_per_s: Fixed::ZERO,
            road_speed_m_per_s: Fixed::ZERO,
            footprint_m: Fixed::from_num(footprint_mm) / Fixed::from_num(1000),
            mass_kg: entry.mass_kg.unwrap_or(0),
            combat,
            transport: crate::transport::TransportType::default(),
        }
    }

    /// Whether units of this type move on the water (prompt 119).
    pub const fn is_boat(&self) -> bool {
        self.transport.draft_mm > 0
    }

    /// Whether units of this type can move (buildings cannot).
    pub fn mobile(&self) -> bool {
        self.speed_m_per_s > Fixed::ZERO
    }
}

/// Looks up the given catalog ids and converts them, keeping their order.
pub fn unit_types_from_catalog(
    catalog: &Catalog,
    scale: &Scale,
    ids: &[&str],
) -> Result<Vec<UnitType>, DataError> {
    ids.iter()
        .map(|id| {
            let entry = catalog
                .get(id)
                .ok_or_else(|| DataError(format!("unknown catalog id '{id}'")))?;
            UnitType::from_catalog(entry, scale)
                .ok_or_else(|| DataError(format!("catalog entry '{id}' cannot move")))
        })
        .collect()
}
