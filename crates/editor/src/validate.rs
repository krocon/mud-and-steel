//! Checks a map the way a match will use it: loading, start positions,
//! room for the units of the scenario and reachable paths between the
//! starts and to the supply points.

use crate::MapFiles;
use sim::path::{PathGrid, SizeClass};
use sim::scenario::{DEMO_SCENARIO, parse_scenario_with_factions, scenario_factions};
use sim::{
    Catalog, Map, PlayerId, Scale, Sim, SimConfig, TerrainRules, Vec2, unit_types_from_catalog,
};
use std::sync::Arc;

/// Infantry used for the path check (the skirmish has tanks only so far).
pub const INFANTRY_UNIT: &str = "de_inf_rifle_squad";
/// Players of the skirmish (prompt 027): both need a start position.
pub const PLAYERS: [u8; 2] = [0, 1];
/// A path that ends this close to a supply point reaches it (units take a
/// point within 25 m, `data/economy.toml`).
pub const SUPPLY_REACH_M: i32 = 20;
/// Blocked unit positions listed at most per player.
const MAX_LISTED: usize = 3;

/// Result of [`validate_files`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The map cannot be played (or would load with errors).
    pub errors: Vec<String>,
    /// Playable, but probably not as intended.
    pub warnings: Vec<String>,
    /// Content hash of the loaded map (`sim::Map::hash`), if it loaded.
    pub map_hash: Option<u64>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// The demo scenario on `map` with every player's units from `faction`
/// (or as in the scenario for `None`).
fn scenario(map: &Map, faction: Option<&str>) -> Result<SimConfig, String> {
    let catalog = Catalog::builtin_ref().map_err(|e| e.to_string())?;
    let scale = Scale::builtin_ref().map_err(|e| e.to_string())?;
    let terrain = TerrainRules::builtin_ref().map_err(|e| e.to_string())?;
    let factions: Vec<(PlayerId, &str)> = faction
        .map(|f| PLAYERS.iter().map(|&p| (PlayerId(p), f)).collect())
        .unwrap_or_default();
    parse_scenario_with_factions(DEMO_SCENARIO, &factions, catalog, scale, terrain, map)
        .map_err(|e| e.to_string())
}

/// Loads the files like the simulation and checks them.
pub fn validate_files(files: &MapFiles) -> Report {
    match files.load() {
        Ok(map) => validate_map(map),
        Err(e) => Report {
            errors: vec![format!("the map does not load: {e}")],
            ..Report::default()
        },
    }
}

/// Checks a loaded map: its standard start layout and every start variant
/// (prompt 182; their messages begin with "start variant '<name>':").
pub fn validate_map(map: Map) -> Report {
    let variants = map.variants().to_vec();
    let mut variant_reports: Vec<(String, Report)> = variants
        .iter()
        .map(|v| {
            (
                v.name.clone(),
                validate_layout(map.with_variant(v), v.supply.as_deref()),
            )
        })
        .collect();
    let mut report = validate_layout(map, None);
    for (name, r) in variant_reports.drain(..) {
        let tag = |m: String| format!("start variant '{name}': {m}");
        report.errors.extend(r.errors.into_iter().map(tag));
        report.warnings.extend(r.warnings.into_iter().map(tag));
    }
    report
}

/// Checks a map in one start layout; `supply`: the supply points of that
/// layout (`None`: all).
fn validate_layout(map: Map, supply: Option<&[String]>) -> Report {
    let mut report = Report {
        map_hash: Some(map.hash()),
        ..Report::default()
    };
    let err = |report: &mut Report, e: String| report.errors.push(e);
    for player in PLAYERS {
        if !map.starts().iter().any(|s| s.player == player) {
            err(
                &mut report,
                format!("player {player} has no start position"),
            );
        }
    }
    if !report.ok() {
        return report;
    }
    let start = |player: u8| {
        map.starts()
            .iter()
            .find(|s| s.player == player)
            .map_or(Vec2::ZERO, |s| s.position)
    };
    let (a, b) = (start(PLAYERS[0]), start(PLAYERS[1]));
    if (b - a).length() < sim::Fixed::from_num(50) {
        report
            .warnings
            .push("the start positions are less than 50 m apart".into());
    }

    // Units of every faction (tanks so far) and infantry.
    let mut names: Vec<String> = Vec::new();
    let mut configs = Vec::new();
    let factions = scenario_factions(DEMO_SCENARIO).unwrap_or_default();
    for faction in &factions {
        match scenario(&map, Some(faction)) {
            Ok(config) => {
                for t in &config.unit_types {
                    if !names.contains(&t.name) {
                        names.push(t.name.clone());
                    }
                }
                configs.push(config);
            }
            Err(e) => err(&mut report, format!("scenario: {e}")),
        }
    }
    names.push(INFANTRY_UNIT.into());
    let (catalog, scale, terrain) = match (
        Catalog::builtin(),
        Scale::builtin(),
        TerrainRules::builtin(),
    ) {
        (Ok(c), Ok(s), Ok(t)) => (c, s, t),
        _ => {
            err(&mut report, "built-in data does not load".into());
            return report;
        }
    };
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let types = match unit_types_from_catalog(&catalog, &scale, &refs) {
        Ok(types) => types,
        Err(e) => {
            err(&mut report, e.0);
            return report;
        }
    };
    let grid = PathGrid::new(&map, &terrain, &types);
    let profile = |name: &str| types.iter().position(|t| t.name == name);

    // Room for the units at the starts (positions are the same for every
    // faction).
    if let Some(config) = configs.first() {
        for player in PLAYERS {
            let blocked: Vec<String> = config
                .spawns
                .iter()
                .filter(|s| s.owner == PlayerId(player))
                .filter(|s| {
                    !map.contains(s.position) || !grid.passable_at(SizeClass::Large, s.position)
                })
                .map(|s| format!("({}, {})", s.position.x.round(), s.position.y.round()))
                .collect();
            if !blocked.is_empty() {
                let listed: Vec<&str> = blocked
                    .iter()
                    .take(MAX_LISTED)
                    .map(String::as_str)
                    .collect();
                err(
                    &mut report,
                    format!(
                        "player {player}: {} of the units at the start cannot stand there \
                         (water, too steep, objects or map edge), e.g. at {} m",
                        blocked.len(),
                        listed.join(", ")
                    ),
                );
            }
        }
    }

    // Paths from start to start.
    for name in &names {
        let Some(p) = profile(name) else { continue };
        let reached = grid
            .find_path(a, b, p)
            .is_some_and(|path| grid.cell_of(path.goal) == grid.cell_of(b));
        if !reached {
            let message = format!("{name} cannot drive from player 0's start to player 1's start");
            if name == INFANTRY_UNIT {
                report.warnings.push(message);
            } else {
                err(&mut report, message);
            }
        }
    }

    // Supply points (prompt 060): reachable from player 0's start. Units
    // take a point within `SUPPLY_REACH_M`; an error if nobody gets there,
    // a warning if some unit types cannot.
    for marker in map
        .markers()
        .iter()
        .filter(|m| m.name.starts_with("supply"))
        .filter(|m| supply.is_none_or(|names| names.contains(&m.name)))
    {
        let missing: Vec<&str> = names
            .iter()
            .filter(|name| {
                profile(name).is_some_and(|p| {
                    !grid.find_path(a, marker.position, p).is_some_and(|path| {
                        (path.goal - marker.position).length()
                            <= sim::Fixed::from_num(SUPPLY_REACH_M)
                    })
                })
            })
            .map(String::as_str)
            .collect();
        if missing.len() == names.len() {
            err(
                &mut report,
                format!(
                    "no unit can reach the supply point '{}' from player 0's start",
                    marker.name
                ),
            );
        } else if !missing.is_empty() {
            report.warnings.push(format!(
                "{} cannot reach the supply point '{}' from player 0's start",
                missing.join(", "),
                marker.name
            ));
        }
    }

    // A match starts on it.
    if report.ok()
        && let Some(config) = configs.first()
        && let Err(e) = Sim::new(config, Arc::new(map), 1)
    {
        err(&mut report, format!("the match does not start: {e}"));
    }
    report
}
