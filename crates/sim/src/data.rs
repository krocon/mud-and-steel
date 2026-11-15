//! Unit types as the simulation uses them, derived from the catalog.

use crate::catalog::{Catalog, Entry, Group};
use crate::math::Fixed;
use crate::scale::Scale;
use std::fmt;

/// Game properties of a unit type, already converted by [`Scale`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitType {
    /// Catalog id.
    pub name: String,
    /// Movement speed in game meters per second.
    pub speed_m_per_s: Fixed,
}

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
    /// Until terrain exists (prompt 010), ground units move at their
    /// cross-country speed and aircraft at their cruise speed (or maximum
    /// speed if no cruise speed is known). Returns `None` for entries that
    /// cannot move.
    pub fn from_catalog(entry: &Entry, scale: &Scale) -> Option<Self> {
        let speed = match entry.category.group() {
            Group::Infantry | Group::Vehicle => entry.speed_offroad_kmh_x10,
            Group::Aircraft => entry.speed_cruise_kmh_x10.or(entry.speed_max_kmh_x10),
            Group::Gun | Group::Fortification | Group::Environment => None,
        }?;
        Some(Self {
            name: entry.id.clone(),
            speed_m_per_s: scale.game_speed(entry.category, speed),
        })
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
