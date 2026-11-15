//! Scenarios (start setups) and a scripted command stream for tests.
//!
//! A scenario file (`data/scenarios/*.toml`) places groups of catalog units
//! for each player. The demo scenario is compiled in and used by the game, the
//! headless CLI, the golden-hash test and the network tests, so all of them
//! exercise exactly the same setup.

use crate::catalog::Catalog;
use crate::combat::CombatRules;
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::data::unit_types_from_catalog;
use crate::map::Map;
use crate::math::Vec2;
use crate::rng::Rng;
use crate::scale::Scale;
use crate::sim::{Sim, SimConfig, Spawn};
use crate::terrain::TerrainRules;
use serde::Deserialize;
use std::fmt;
use std::sync::{Arc, OnceLock};

/// The demo scenario: eight Panzer IV against eight T-34/76.
pub const DEMO_SCENARIO: &str = include_str!("../../../data/scenarios/demo.toml");

/// Name of the map compiled into the simulation.
pub const BUILTIN_MAP_NAME: &str = "test_small";

/// The files of `data/maps/test_small/`, compiled in (needs Git LFS).
const BUILTIN_MAP_TOML: &str = include_str!("../../../data/maps/test_small/map.toml");
const BUILTIN_MAP_HEIGHT: &[u8] = include_bytes!("../../../data/maps/test_small/height.png");
const BUILTIN_MAP_TERRAIN: &[u8] = include_bytes!("../../../data/maps/test_small/terrain.png");
const BUILTIN_MAP_OBJECTS: &str = include_str!("../../../data/maps/test_small/objects.csv");

/// Ticks between two scripted commands.
pub const COMMAND_INTERVAL: u64 = 40;

/// Scripted targets lie within this distance of the origin per axis (mm).
const SCRIPT_RANGE_MM: i32 = 150_000;

#[derive(Debug, PartialEq, Eq)]
pub struct ScenarioError(pub String);

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid scenario: {}", self.0)
    }
}

impl std::error::Error for ScenarioError {}

/// A block of identical units of one player, placed in a grid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    owner: u8,
    /// Catalog id.
    unit: String,
    count: u32,
    columns: u32,
    spacing_m: u32,
    /// Center of the grid, `[x, y]` in meters.
    center_m: [i32; 2],
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioFile {
    /// Name of the map the scenario is made for (`data/maps/<map>/`).
    map: String,
    group: Vec<Group>,
}

/// The map a scenario file names.
pub fn scenario_map_name(text: &str) -> Result<String, ScenarioError> {
    let file: ScenarioFile = toml::from_str(text).map_err(|e| ScenarioError(e.to_string()))?;
    Ok(file.map)
}

/// Parses a scenario file and resolves its units against the catalog. The
/// match is played on `map` (usually the one [`scenario_map_name`] names;
/// any map large enough for the spawns works).
///
/// Unit ids are assigned in file order: group by group, row by row.
pub fn parse_scenario(
    text: &str,
    catalog: &Catalog,
    scale: &Scale,
    terrain: &TerrainRules,
    map: &Map,
) -> Result<SimConfig, ScenarioError> {
    let file: ScenarioFile = toml::from_str(text).map_err(|e| ScenarioError(e.to_string()))?;

    let mut names: Vec<&str> = Vec::new();
    for group in &file.group {
        if !names.contains(&group.unit.as_str()) {
            names.push(&group.unit);
        }
    }
    let unit_types =
        unit_types_from_catalog(catalog, scale, &names).map_err(|e| ScenarioError(e.0))?;

    let mut spawns = Vec::new();
    for group in &file.group {
        if group.columns == 0 {
            return Err(ScenarioError(format!(
                "group '{}' has no columns",
                group.unit
            )));
        }
        spawns.extend(grid(group)?.into_iter().map(|position| Spawn {
            owner: PlayerId(group.owner),
            unit_type: group.unit.clone(),
            position,
            fire_mode: crate::combat::FireMode::default(),
        }));
    }
    Ok(SimConfig {
        tick_hz: SimConfig::DEFAULT_TICK_HZ,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: terrain.clone(),
        combat: CombatRules::builtin().map_err(|e| ScenarioError(e.to_string()))?,
    })
}

/// Grid positions of a group, row by row, every row centered.
fn grid(group: &Group) -> Result<Vec<Vec2>, ScenarioError> {
    let too_large = || ScenarioError(format!("group '{}' is too large", group.unit));
    let half_mm = i64::from(group.spacing_m) * 500;
    // Offset in mm of slot `index` in a line of `len` slots.
    let offset = |index: u32, len: u32| (2 * i64::from(index) - i64::from(len) + 1) * half_mm;
    let to_mm = |center_m: i32, offset_mm: i64| {
        i32::try_from(i64::from(center_m) * 1000 + offset_mm).map_err(|_| too_large())
    };

    let rows = group.count.div_ceil(group.columns);
    let mut positions = Vec::new();
    for row in 0..rows {
        let in_row = group.columns.min(group.count - row * group.columns);
        let y = to_mm(group.center_m[1], offset(row, rows))?;
        for column in 0..in_row {
            let x = to_mm(group.center_m[0], offset(column, in_row))?;
            positions.push(Vec2::from_millimeters(x, y));
        }
    }
    Ok(positions)
}

/// The map compiled into the simulation (`data/maps/test_small/`),
/// decoded once.
pub fn builtin_map() -> Arc<Map> {
    static MAP: OnceLock<Arc<Map>> = OnceLock::new();
    MAP.get_or_init(|| {
        const HINT: &str =
            "built-in map data/maps/test_small is valid (is Git LFS installed and pulled?)";
        let mut map =
            Map::load(BUILTIN_MAP_TOML, BUILTIN_MAP_HEIGHT, BUILTIN_MAP_TERRAIN).expect(HINT);
        map.add_objects(BUILTIN_MAP_OBJECTS).expect(HINT);
        Arc::new(map)
    })
    .clone()
}

/// The demo scenario with the built-in catalog, scale, terrain and map.
pub fn demo_config() -> SimConfig {
    let catalog = Catalog::builtin().expect("built-in catalog is valid");
    let scale = Scale::builtin().expect("built-in scale is valid");
    let terrain = TerrainRules::builtin().expect("built-in terrain rules are valid");
    parse_scenario(DEMO_SCENARIO, &catalog, &scale, &terrain, &builtin_map())
        .expect("demo scenario is valid")
}

/// Deterministic command generator for one player.
///
/// Every [`COMMAND_INTERVAL`] ticks (except tick 0) it picks a random group of
/// the player's units and sends it to a random point, or every fifth time
/// stops it. Unit lists are deliberately unsorted and may repeat units, to
/// exercise the simulation's order independence.
///
/// Because tick 0 never has commands, a lockstep peer with an input delay
/// below [`COMMAND_INTERVAL`] that asks for the commands of the tick it
/// schedules reproduces [`run_demo`] exactly.
#[derive(Clone, Debug)]
pub struct CommandScript {
    rng: Rng,
    units: Vec<UnitId>,
}

impl CommandScript {
    /// The script uses its own generator, derived from `seed` and `player`,
    /// so it never disturbs the simulation's generator.
    pub fn new(seed: u64, player: PlayerId, config: &SimConfig) -> Self {
        let units = (0u32..)
            .zip(&config.spawns)
            .filter(|(_, spawn)| spawn.owner == player)
            .map(|(index, _)| UnitId(index))
            .collect();
        let salt = 0xC0FF_EE00_5C21_4700 ^ (u64::from(player.0) << 56);
        Self {
            rng: Rng::new(seed ^ salt),
            units,
        }
    }

    /// Commands of this player for `tick`. Must be called for consecutive
    /// ticks (gaps are fine as long as no command tick is skipped).
    pub fn commands_for_tick(&mut self, tick: u64) -> Vec<Command> {
        let count = u32::try_from(self.units.len()).unwrap_or(u32::MAX);
        if count == 0 || tick == 0 || !tick.is_multiple_of(COMMAND_INTERVAL) {
            return Vec::new();
        }
        let size = 1 + self.rng.below(count);
        let first = self.rng.below(count);
        // Consecutive units (wrapping), listed backwards, plus the first one
        // again as a duplicate.
        let mut units: Vec<UnitId> = (0..size)
            .rev()
            .map(|i| self.units[usize::try_from((first + i) % count).expect("small")])
            .collect();
        units.push(units[units.len() - 1]);

        if (tick / COMMAND_INTERVAL).is_multiple_of(5) {
            return vec![Command::Stop { units }];
        }
        let target = Vec2::from_millimeters(
            self.rng.range_i32(-SCRIPT_RANGE_MM, SCRIPT_RANGE_MM),
            self.rng.range_i32(-SCRIPT_RANGE_MM, SCRIPT_RANGE_MM),
        );
        vec![Command::MoveTo { units, target }]
    }
}

/// One script per player of `config`, in ascending player order.
pub fn demo_scripts(seed: u64, config: &SimConfig) -> Vec<(PlayerId, CommandScript)> {
    config
        .players()
        .into_iter()
        .map(|player| (player, CommandScript::new(seed, player, config)))
        .collect()
}

/// Runs the demo scenario for `ticks` ticks with every player's script and
/// returns the final simulation.
pub fn run_demo(seed: u64, ticks: u64) -> Sim {
    run_demo_with(seed, ticks, |_, _| {})
}

/// Like [`run_demo`], but calls `observe(tick, commands)` before every step.
pub fn run_demo_with(seed: u64, ticks: u64, mut observe: impl FnMut(u64, &[PlayerCommand])) -> Sim {
    let config = demo_config();
    let mut sim = Sim::new(&config, builtin_map(), seed).expect("demo config is valid");
    let mut scripts = demo_scripts(seed, &config);
    for tick in 0..ticks {
        let commands: Vec<PlayerCommand> = scripts
            .iter_mut()
            .flat_map(|(player, script)| {
                let player = *player;
                script
                    .commands_for_tick(tick)
                    .into_iter()
                    .map(move |command| PlayerCommand::new(player, command))
            })
            .collect();
        observe(tick, &commands);
        sim.step(&commands);
    }
    sim
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_has_two_players_with_eight_tanks() {
        let config = demo_config();
        assert_eq!(config.players(), vec![PlayerId(0), PlayerId(1)]);
        for (player, unit) in [(0, "de_pz4_h"), (1, "su_t34_76_1943")] {
            let spawns: Vec<&Spawn> = config
                .spawns
                .iter()
                .filter(|s| s.owner == PlayerId(player))
                .collect();
            assert_eq!(spawns.len(), 8);
            assert!(spawns.iter().all(|s| s.unit_type == unit));
        }
        // First row of player 0: four tanks 12 m apart, centered, at y = 54 m.
        let first: Vec<(i32, i32)> = config.spawns[..4]
            .iter()
            .map(|s| (s.position.x.to_num(), s.position.y.to_num()))
            .collect();
        assert_eq!(first, vec![(-18, 54), (-6, 54), (6, 54), (18, 54)]);
    }

    #[test]
    fn demo_names_the_builtin_map() {
        assert_eq!(scenario_map_name(DEMO_SCENARIO).unwrap(), BUILTIN_MAP_NAME);
        assert_eq!(builtin_map().name(), BUILTIN_MAP_NAME);
    }

    #[test]
    fn biome_is_read_but_not_hashed() {
        use crate::map::Biome;
        let load = |toml: &str| Map::load(toml, BUILTIN_MAP_HEIGHT, BUILTIN_MAP_TERRAIN);
        let without: String = BUILTIN_MAP_TOML
            .lines()
            .filter(|l| !l.starts_with("biome"))
            .map(|l| format!("{l}\n"))
            .collect();
        let base = load(&without).unwrap();
        assert_eq!(base.biome(), Biome::Forest);
        assert_eq!(builtin_map().biome(), Biome::Forest);
        for biome in Biome::ALL {
            let toml = without.replacen(
                "cell_size_mm",
                &format!("biome = \"{}\"\ncell_size_mm", biome.name()),
                1,
            );
            let map = load(&toml).unwrap();
            assert_eq!(map.biome(), biome);
            assert_eq!(map.hash(), base.hash());
        }
        let bad = without.replacen("cell_size_mm", "biome = \"jungle\"\ncell_size_mm", 1);
        assert!(load(&bad).is_err());
    }

    #[test]
    fn builtin_map_has_objects_but_keeps_the_demo_area_open() {
        use crate::path::{PathGrid, SizeClass};
        let map = builtin_map();
        assert!(map.objects().len() > 500, "{} objects", map.objects().len());
        let config = demo_config();
        let grid = PathGrid::new(&map, &config.terrain, &config.unit_types);
        let open = |p: Vec2| grid.passable_at(SizeClass::Large, p);
        // Spawns, the scripted command area of the demo (the meadow), the
        // hill top of the self-test and the demo reel.
        assert!(config.spawns.iter().all(|s| open(s.position)));
        for (x, y) in [(40, -10), (-80, 40), (70, -30), (0, 0)] {
            assert!(open(Vec2::from_meters(x, y)), "({x}, {y})");
        }
        assert!(open(map.marker("hill_top").unwrap()));
    }

    #[test]
    fn rejects_unknown_fields_and_units() {
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let terrain = TerrainRules::builtin().unwrap();
        let map = builtin_map();
        let parse = |text: &str| parse_scenario(text, &catalog, &scale, &terrain, &map);
        let bad_field = "map = \"test_small\"\n[[group]]\nowner = 0\nunit = \"de_pz4_h\"\ncount = 1\ncolumns = 1\n\
                         spacing_m = 1\ncenter_m = [0, 0]\ncolour = 1\n";
        assert!(parse(bad_field).is_err());
        let bad_unit = DEMO_SCENARIO.replace("de_pz4_h", "de_nope");
        assert!(parse(&bad_unit).is_err());
        assert!(parse(&DEMO_SCENARIO.replace("map = ", "# map = ")).is_err());
    }

    #[test]
    fn script_only_commands_own_units() {
        let config = demo_config();
        let mut script = CommandScript::new(7, PlayerId(1), &config);
        let mut issued = 0;
        for tick in 0..2000 {
            for command in script.commands_for_tick(tick) {
                issued += 1;
                let units = command.units();
                assert!(units.iter().all(|u| (8..16).contains(&u.0)), "{units:?}");
            }
        }
        assert_eq!(issued, 2000 / 40 - 1);
    }
}
