//! Unit definitions loaded from TOML (see `data/units.toml`).
//!
//! Values are integers in explicit units (e.g. millimeters per second), so
//! parsing never goes through floating point.

use serde::Deserialize;
use std::fmt;

/// Static properties of a unit type.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitType {
    pub name: String,
    /// Driving speed in millimeters per second.
    pub speed_mm_per_s: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitFile {
    unit: Vec<UnitType>,
}

#[derive(Debug)]
pub struct DataError(String);

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid unit data: {}", self.0)
    }
}

impl std::error::Error for DataError {}

/// Parses a unit definition file with one `[[unit]]` table per unit type.
pub fn parse_unit_types(source: &str) -> Result<Vec<UnitType>, DataError> {
    let file: UnitFile = toml::from_str(source).map_err(|e| DataError(e.to_string()))?;
    let mut names: Vec<&str> = file.unit.iter().map(|u| u.name.as_str()).collect();
    names.sort_unstable();
    if let Some(pair) = names.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(DataError(format!("duplicate unit type '{}'", pair[0])));
    }
    Ok(file.unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_unit_data_parses() {
        let types = parse_unit_types(include_str!("../../../data/units.toml")).unwrap();
        assert!(types.iter().any(|t| t.name == "placeholder_tank"));
    }

    #[test]
    fn rejects_duplicates() {
        let src = "[[unit]]\nname = \"a\"\nspeed_mm_per_s = 1\n[[unit]]\nname = \"a\"\nspeed_mm_per_s = 2\n";
        assert!(parse_unit_types(src).is_err());
    }
}
