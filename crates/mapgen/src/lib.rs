//! Procedural map and terrain texture generators (extracted from `cli`, prompt 216).
//!
//! All generators run outside `crates/sim` and may use floating-point math
//! (restricted to basic arithmetic and `sqrt` so generated map files stay
//! bit-identical across platforms).

pub mod gen_blocks;
pub mod gen_borgo;
pub mod gen_city;
pub mod gen_land;
pub mod gen_map;
pub mod gen_sketch;
pub mod gen_textures;
pub mod gen_town;
pub mod gen_village;

/// Applies a map's prepared battleground (`map.toml` trenches, craters, puddles)
/// by constructing a dummy simulation, returning the dug map and its path grid.
pub fn prepared_ground(
    map: sim::Map,
    types: &[&str],
) -> Result<(sim::Map, Option<sim::path::PathGrid>), String> {
    if !map.prepared().has_ground() {
        return Ok((map, None));
    }
    let e = |e: &dyn std::fmt::Display| e.to_string();
    let catalog = sim::Catalog::builtin().map_err(|x| e(&x))?;
    let scale = sim::Scale::builtin().map_err(|x| e(&x))?;
    let config = sim::SimConfig {
        tick_hz: sim::SimConfig::DEFAULT_TICK_HZ,
        unit_types: sim::unit_types_from_catalog(&catalog, &scale, types).map_err(|x| e(&x))?,
        spawns: Vec::new(),
        map: map.reference(),
        terrain: sim::TerrainRules::builtin().map_err(|x| e(&x))?,
        combat: sim::CombatRules::builtin().map_err(|x| e(&x))?,
        vision: sim::VisionRules::builtin().map_err(|x| e(&x))?,
        infantry: sim::InfantryRules::builtin().map_err(|x| e(&x))?,
        teams: sim::Teams::default(),
        weather: sim::WeatherSetup::default(),
        mission: None,
        variant: None,
    };
    let sim = sim::Sim::new(&config, std::sync::Arc::new(map), 1).map_err(|x| e(&x))?;
    Ok(((**sim.map()).clone(), Some(sim.path_grid().clone())))
}
