//! Missions and the tutorial (prompt 039): scripts as data.
//!
//! A mission is a TOML file next to its map (`data/maps/<map>/missions/<id>.toml`)
//! with a header, the units at the start, objectives and triggers. Trigger
//! areas and spawn points come from the map's `mission_points.toml` (written
//! by the map editor) or from the mission file itself. The format is
//! described in `docs/MISSIONS.md`.
//!
//! [`parse_mission`] turns the files into a [`SimConfig`] whose
//! [`SimConfig::mission`] holds the resolved script: names become indices,
//! places become positions, seconds become ticks, catalog ids become unit
//! types. The simulation runs the triggers every tick (`sim/mission.rs`), so
//! everything a mission changes (units appearing, orders, victory) is part
//! of the deterministic state, the replay and saved games.

use crate::air::AirMission;
use crate::catalog::Catalog;
use crate::combat::{CombatRules, FireMode};
use crate::command::PlayerId;
use crate::data::{UnitType, unit_types_from_catalog};
use crate::formation::Engage;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::scale::Scale;
use crate::sim::{SimConfig, Spawn, formation_slots, formation_spacing};
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Name of the file with the trigger areas and spawn points of a map
/// (written by the map editor), in the map folder.
pub const POINTS_FILE: &str = "mission_points.toml";
/// Folder of the mission scripts, in the map folder.
pub const MISSIONS_DIR: &str = "missions";
/// Longest signal name a [`crate::Command::MissionSignal`] may carry.
pub const MAX_SIGNAL_LEN: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub struct MissionError(pub String);

impl fmt::Display for MissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid mission: {}", self.0)
    }
}

impl std::error::Error for MissionError {}

fn err<T>(message: impl Into<String>) -> Result<T, MissionError> {
    Err(MissionError(message.into()))
}

// --- Resolved script (part of `SimConfig`) -------------------------------------

/// A mission as the simulation runs it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    /// File name without extension; progress is stored under it.
    pub id: String,
    pub title: String,
    /// The tutorial (shows step numbers, offered by its own menu entry).
    pub tutorial: bool,
    /// The human player.
    pub player: PlayerId,
    /// Enemies are defeated when they lose their last unit, as in a
    /// skirmish. Without it only the script decides the match (the player
    /// still loses with their last unit).
    pub elimination: bool,
    /// Computer players and their difficulty (`ai::Difficulty` names).
    pub computers: Vec<(PlayerId, String)>,
    pub areas: Vec<Area>,
    pub objectives: Vec<Objective>,
    /// Names of the unit groups; triggers and actions use their index.
    pub groups: Vec<String>,
    /// Groups of the start units.
    pub start_groups: Vec<StartGroup>,
    pub triggers: Vec<Trigger>,
}

/// A circle on the ground that triggers watch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Area {
    pub name: String,
    pub center: Vec2,
    pub radius: Fixed,
}

impl Area {
    pub fn contains(&self, p: Vec2) -> bool {
        let d = p - self.center;
        // Compare squares in i128 bits: no square root, no overflow.
        let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
        let r = i128::from(self.radius.to_bits());
        x * x + y * y <= r * r
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub text: String,
    /// Where it is (drawn as a ring, used by the self-test's autopilot).
    pub area: Option<u16>,
    /// The catalog id it is about (a building to build, a unit to produce).
    pub unit: Option<String>,
    /// Shown from the start (otherwise when an action activates it).
    pub shown: bool,
}

/// State of an objective.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveState {
    #[default]
    Hidden,
    Active,
    Done,
    Failed,
}

impl ObjectiveState {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Active => "active",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "hidden" => Self::Hidden,
            "active" => Self::Active,
            "done" => Self::Done,
            "failed" => Self::Failed,
            _ => return None,
        })
    }
}

/// Units of the start that form a group: the spawns `first..first+count`
/// of `SimConfig::spawns` (their unit ids, since ids follow spawn order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartGroup {
    pub group: u16,
    pub first: u32,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trigger {
    pub id: String,
    /// Triggers that must have fired before this one is armed.
    pub after: Vec<u16>,
    /// Ticks between the condition being met and the actions.
    pub delay_ticks: u32,
    pub condition: Condition,
    pub actions: Vec<Action>,
}

/// What a trigger waits for, once armed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    /// Nothing: fires `delay_ticks` after it was armed (a timer).
    Always,
    /// At least `count` living units of `player` (whose catalog id starts
    /// with `unit`, if given) stand in the area.
    Area {
        area: u16,
        player: PlayerId,
        count: u32,
        unit: Option<String>,
    },
    /// Every unit of the group is destroyed (and it had units).
    Destroyed { group: u16 },
    /// `player` has no living unit left.
    Eliminated { player: PlayerId },
    /// `player` holds the supply point (map marker) `point`.
    Captured { point: String, player: PlayerId },
    /// The human player's presentation reported something only it knows
    /// (camera moved, units selected) with [`crate::Command::MissionSignal`].
    Signal { name: String },
    /// `player` owns at least `count` living units whose catalog id starts
    /// with `unit` (structures only when completed).
    Units {
        player: PlayerId,
        unit: String,
        count: u32,
    },
}

/// What a trigger does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Shows a message (replaces the previous one).
    Text { text: String },
    /// Sets the state of an objective.
    Objective { index: u16, state: ObjectiveState },
    /// `count` units of `unit_type` appear for `owner` around `at` and join
    /// the group.
    Spawn {
        group: u16,
        owner: PlayerId,
        unit_type: u32,
        count: u32,
        at: Vec2,
        fire_mode: FireMode,
    },
    /// Moves the camera there (presentation).
    Camera { at: Vec2 },
    /// The living units of the group move to `target` (attack-move with
    /// `Engage::All`), like a command of their owner.
    Order {
        group: u16,
        target: Vec2,
        engage: Engage,
    },
    /// Fire mode of the group.
    FireMode { group: u16, mode: FireMode },
    /// The human player wins.
    Victory,
    /// The human player loses.
    Defeat,
    /// The group's units with an indirect weapon fire `salvos` salvos at
    /// random points within `radius_m` of `target`
    /// ([`crate::Command::Barrage`] of their owner; prompt 082).
    Barrage {
        group: u16,
        target: Vec2,
        radius_m: u16,
        salvos: u16,
    },
    /// `player` calls air support on `target`
    /// ([`crate::Command::CallAir`]), or along `target` → `to`
    /// ([`crate::Command::CallAirLine`]); price and cooldown apply.
    Air {
        player: PlayerId,
        mission: AirMission,
        target: Vec2,
        to: Option<Vec2>,
    },
    /// The group's vehicles and guns put up or take off their camouflage
    /// ([`crate::Command::SetCamouflage`]).
    Camouflage { group: u16, on: bool },
}

// --- Files -------------------------------------------------------------------

/// Header of a mission file, also read by the menu.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionHeader {
    pub title: String,
    /// `"mission"` (default) or `"tutorial"`.
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Position in the mission list.
    #[serde(default)]
    pub order: u32,
    /// Faction id of the player, for the menu.
    #[serde(default)]
    pub faction: String,
    #[serde(default)]
    pub briefing: String,
    /// Look of the map (`forest`, `desert`, `winter`; empty: the map's).
    #[serde(default)]
    pub biome: String,
    #[serde(default)]
    pub player: u8,
    #[serde(default = "yes")]
    pub elimination: bool,
    /// Base building: adds buildings and production lists of the factions.
    #[serde(default)]
    pub economy: bool,
}

fn default_kind() -> String {
    "mission".into()
}

const fn yes() -> bool {
    true
}

const fn one() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MissionFile {
    mission: MissionHeader,
    #[serde(default)]
    computer: Vec<ComputerFile>,
    #[serde(default)]
    group: Vec<GroupFile>,
    #[serde(default)]
    area: Vec<AreaEntry>,
    #[serde(default)]
    spawn: Vec<SpawnEntry>,
    #[serde(default)]
    objective: Vec<ObjectiveFile>,
    #[serde(default)]
    trigger: Vec<TriggerFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ComputerFile {
    player: u8,
    difficulty: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupFile {
    #[serde(default)]
    name: String,
    owner: u8,
    unit: String,
    #[serde(default = "one")]
    count: u32,
    /// A spawn point, area or map marker.
    #[serde(default)]
    at: String,
    #[serde(default)]
    position_m: Option<[i32; 2]>,
    #[serde(default)]
    spacing_m: Option<u32>,
    #[serde(default)]
    hold_fire: bool,
}

/// A trigger area in `mission_points.toml` or a mission file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AreaEntry {
    pub name: String,
    pub center_m: [i32; 2],
    pub radius_m: u32,
}

/// A spawn point in `mission_points.toml` or a mission file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnEntry {
    pub name: String,
    pub position_m: [i32; 2],
}

/// Contents of `mission_points.toml`: the places the map editor sets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionPoints {
    #[serde(default)]
    pub area: Vec<AreaEntry>,
    #[serde(default)]
    pub spawn: Vec<SpawnEntry>,
}

impl MissionPoints {
    pub fn parse(text: &str) -> Result<Self, MissionError> {
        toml::from_str(text).map_err(|e| MissionError(format!("{POINTS_FILE}: {e}")))
    }

    pub fn is_empty(&self) -> bool {
        self.area.is_empty() && self.spawn.is_empty()
    }

    /// The file contents, in a fixed layout.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# Trigger areas and spawn points of this map for mission scripts\n\
             # (prompt 039, docs/MISSIONS.md). Written by the map editor.\n",
        );
        for a in &self.area {
            out.push_str(&format!(
                "\n[[area]]\nname = \"{}\"\ncenter_m = [{}, {}]\nradius_m = {}\n",
                a.name, a.center_m[0], a.center_m[1], a.radius_m
            ));
        }
        for s in &self.spawn {
            out.push_str(&format!(
                "\n[[spawn]]\nname = \"{}\"\nposition_m = [{}, {}]\n",
                s.name, s.position_m[0], s.position_m[1]
            ));
        }
        out
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectiveFile {
    id: String,
    text: String,
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    shown: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TriggerFile {
    id: String,
    #[serde(default)]
    after: Vec<String>,
    /// `time`, `area`, `destroyed`, `eliminated`, `captured`, `signal`,
    /// `units`.
    when: String,
    #[serde(default)]
    delay_s: u32,
    #[serde(default)]
    area: Option<String>,
    #[serde(default)]
    player: Option<u8>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    count: Option<u32>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    point: Option<String>,
    #[serde(default)]
    signal: Option<String>,
    #[serde(default)]
    actions: Vec<ActionFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionFile {
    /// `text`, `objective`, `spawn`, `camera`, `order`, `fire`, `barrage`,
    /// `air`, `camouflage`, `victory`, `defeat`.
    #[serde(rename = "do")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    objective: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    owner: Option<u8>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    count: Option<u32>,
    #[serde(default)]
    at: Option<String>,
    #[serde(default)]
    to: Option<String>,
    #[serde(default)]
    hold_fire: Option<bool>,
    /// `order`: `advance` (default, halt to fight) or `move`.
    #[serde(default)]
    order: Option<String>,
    /// `barrage`: radius around the place and salvos per unit.
    #[serde(default)]
    radius_m: Option<u16>,
    #[serde(default)]
    salvos: Option<u16>,
    /// `air`: who calls (default: the mission's player) and what.
    #[serde(default)]
    player: Option<u8>,
    #[serde(default)]
    mission: Option<String>,
    /// `camouflage`: put it up (default) or take it off.
    #[serde(default)]
    on: Option<bool>,
}

/// The header of a mission file (for menus).
pub fn mission_header(text: &str) -> Result<MissionHeader, MissionError> {
    let file: MissionFile = toml::from_str(text).map_err(|e| MissionError(e.to_string()))?;
    Ok(file.mission)
}

/// Every text of a mission file the player reads, in file order without
/// repeats: title, briefing, objective texts and `text` actions. The keys
/// of its translations (`<id>.<lang>.po`, prompt 043).
pub fn mission_texts(text: &str) -> Result<Vec<String>, MissionError> {
    let file: MissionFile = toml::from_str(text).map_err(|e| MissionError(e.to_string()))?;
    let mut texts: Vec<String> = vec![file.mission.title, file.mission.briefing];
    texts.extend(file.objective.into_iter().map(|o| o.text));
    texts.extend(
        file.trigger
            .into_iter()
            .flat_map(|t| t.actions)
            .filter_map(|a| a.text),
    );
    let mut out: Vec<String> = Vec::new();
    for t in texts {
        if !t.is_empty() && !out.contains(&t) {
            out.push(t);
        }
    }
    Ok(out)
}

// --- Parsing -----------------------------------------------------------------

/// Builds the match of a mission: `text` is the mission file, `points` the
/// map's `mission_points.toml` (if any), `id` the mission's file name
/// without extension. Unit ids follow the order of the `[[group]]` entries.
pub fn parse_mission(
    id: &str,
    text: &str,
    points: Option<&str>,
    catalog: &Catalog,
    scale: &Scale,
    terrain: &TerrainRules,
    map: &Map,
) -> Result<SimConfig, MissionError> {
    let file: MissionFile = toml::from_str(text).map_err(|e| MissionError(e.to_string()))?;
    let mut places = match points {
        Some(points) => MissionPoints::parse(points)?,
        None => MissionPoints::default(),
    };
    // Entries of the mission file win over the map's of the same name.
    for a in file.area {
        places.area.retain(|x| x.name != a.name);
        places.area.push(a);
    }
    for s in file.spawn {
        places.spawn.retain(|x| x.name != s.name);
        places.spawn.push(s);
    }
    let header = file.mission;
    let tutorial = match header.kind.as_str() {
        "mission" => false,
        "tutorial" => true,
        other => return err(format!("unknown kind '{other}'")),
    };
    let player = PlayerId(header.player);

    // Unit types: the groups, then every spawned unit, then the economy.
    let mut names: Vec<&str> = Vec::new();
    let spawned = file
        .trigger
        .iter()
        .flat_map(|t| &t.actions)
        .filter(|a| a.kind == "spawn")
        .filter_map(|a| a.unit.as_deref());
    for name in file.group.iter().map(|g| g.unit.as_str()).chain(spawned) {
        if !names.contains(&name) && !is_structure(name) {
            names.push(name);
        }
    }
    let mut unit_types =
        unit_types_from_catalog(catalog, scale, &names).map_err(|e| MissionError(e.0))?;
    if header.economy {
        let mut factions: Vec<&str> = names
            .iter()
            .filter_map(|n| crate::economy::faction_of(n))
            .collect();
        factions.sort_unstable();
        factions.dedup();
        let rules =
            crate::economy::EconomyRules::builtin().map_err(|e| MissionError(e.to_string()))?;
        let more = rules
            .unit_types(catalog, scale, &factions, &unit_types)
            .map_err(|e| MissionError(e.0))?;
        unit_types.extend(more);
    }
    let type_index = |name: &str| -> Result<u32, MissionError> {
        unit_types
            .iter()
            .position(|t| t.name == name)
            .map(|i| i as u32)
            .ok_or_else(|| {
                MissionError(format!(
                    "unknown unit '{name}' (buildings need `economy = true`)"
                ))
            })
    };

    let areas: Vec<Area> = places
        .area
        .iter()
        .map(|a| Area {
            name: a.name.clone(),
            center: Vec2::from_meters(a.center_m[0], a.center_m[1]),
            radius: Fixed::from_num(a.radius_m),
        })
        .collect();
    let area_index = |name: &str| -> Result<u16, MissionError> {
        areas
            .iter()
            .position(|a| a.name == name)
            .map(|i| i as u16)
            .ok_or_else(|| MissionError(format!("unknown area '{name}'")))
    };
    // A place: spawn point, area (its center) or map marker.
    let place = |name: &str| -> Result<Vec2, MissionError> {
        let p = places
            .spawn
            .iter()
            .find(|s| s.name == name)
            .map(|s| Vec2::from_meters(s.position_m[0], s.position_m[1]))
            .or_else(|| areas.iter().find(|a| a.name == name).map(|a| a.center))
            .or_else(|| map.marker(name))
            .ok_or_else(|| MissionError(format!("unknown place '{name}'")))?;
        if !map.contains(p) {
            return err(format!("place '{name}' lies outside the map"));
        }
        Ok(p)
    };

    // Groups: the names of start groups and spawn actions.
    let mut groups: Vec<String> = Vec::new();
    let group_of = |name: &str, groups: &mut Vec<String>| -> u16 {
        if let Some(i) = groups.iter().position(|g| g == name) {
            return i as u16;
        }
        groups.push(name.to_owned());
        (groups.len() - 1) as u16
    };

    // Start units.
    let mut spawns: Vec<Spawn> = Vec::new();
    let mut start_groups = Vec::new();
    for (k, g) in file.group.iter().enumerate() {
        if g.count == 0 {
            return err(format!("group {k} ('{}') has no units", g.unit));
        }
        let center = match (&g.position_m, g.at.as_str()) {
            (Some(p), "") => Vec2::from_meters(p[0], p[1]),
            (None, at) if !at.is_empty() => place(at)?,
            _ => return err(format!("group {k} needs either `at` or `position_m`")),
        };
        let t = type_index(&g.unit)?;
        let spacing = g.spacing_m.map_or_else(
            || formation_spacing(unit_types[t as usize].footprint_m),
            Fixed::from_num,
        );
        let first = spawns.len() as u32;
        let fire_mode = if g.hold_fire {
            FireMode::HoldFire
        } else {
            FireMode::FireAtWill
        };
        for position in formation_slots(center, g.count as usize, spacing) {
            spawns.push(Spawn {
                owner: PlayerId(g.owner),
                unit_type: g.unit.clone(),
                position,
                fire_mode,
            });
        }
        let name = if g.name.is_empty() {
            format!("group{k}")
        } else {
            g.name.clone()
        };
        start_groups.push(StartGroup {
            group: group_of(&name, &mut groups),
            first,
            count: g.count,
        });
    }
    let mut players: Vec<PlayerId> = spawns.iter().map(|s| s.owner).collect();
    players.sort_unstable();
    players.dedup();
    if !players.contains(&player) {
        return err(format!("the player {} has no units", player.0));
    }
    let known_player = |p: u8| -> Result<PlayerId, MissionError> {
        if players.contains(&PlayerId(p)) {
            Ok(PlayerId(p))
        } else {
            err(format!("player {p} has no units at the start"))
        }
    };

    let objectives = file
        .objective
        .iter()
        .map(|o| {
            Ok(Objective {
                id: o.id.clone(),
                text: o.text.clone(),
                area: o.area.as_deref().map(area_index).transpose()?,
                unit: o.unit.clone(),
                shown: o.shown,
            })
        })
        .collect::<Result<Vec<_>, MissionError>>()?;
    let objective_index = |id: &str| -> Result<u16, MissionError> {
        objectives
            .iter()
            .position(|o| o.id == id)
            .map(|i| i as u16)
            .ok_or_else(|| MissionError(format!("unknown objective '{id}'")))
    };

    let tick_hz = SimConfig::DEFAULT_TICK_HZ;
    let ids: Vec<&str> = file.trigger.iter().map(|t| t.id.as_str()).collect();
    for (i, id) in ids.iter().enumerate() {
        if ids[..i].contains(id) {
            return err(format!("trigger '{id}' is defined twice"));
        }
    }
    let mut triggers = Vec::new();
    for t in &file.trigger {
        let ctx = |what: &str| MissionError(format!("trigger '{}': {what}", t.id));
        let after = t
            .after
            .iter()
            .map(|a| {
                ids.iter()
                    .position(|id| id == a)
                    .map(|i| i as u16)
                    .ok_or_else(|| ctx(&format!("unknown trigger '{a}' in `after`")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let who = known_player(t.player.unwrap_or(player.0))?;
        let need = |v: &Option<String>, field: &str| {
            v.clone()
                .ok_or_else(|| ctx(&format!("`when = \"{}\"` needs `{field}`", t.when)))
        };
        let condition = match t.when.as_str() {
            "time" => Condition::Always,
            "area" => Condition::Area {
                area: area_index(&need(&t.area, "area")?)?,
                player: who,
                count: t.count.unwrap_or(1).max(1),
                unit: t.unit.clone(),
            },
            "destroyed" => Condition::Destroyed {
                group: group_of(&need(&t.group, "group")?, &mut groups),
            },
            "eliminated" => Condition::Eliminated { player: who },
            "captured" => {
                let point = need(&t.point, "point")?;
                if map.marker(&point).is_none() {
                    return Err(ctx(&format!("the map has no supply point '{point}'")));
                }
                Condition::Captured { point, player: who }
            }
            "signal" => {
                let name = need(&t.signal, "signal")?;
                if name.len() > MAX_SIGNAL_LEN {
                    return Err(ctx("signal name too long"));
                }
                Condition::Signal { name }
            }
            "units" => Condition::Units {
                player: who,
                unit: need(&t.unit, "unit")?,
                count: t.count.unwrap_or(1).max(1),
            },
            other => return Err(ctx(&format!("unknown `when = \"{other}\"`"))),
        };
        let mut actions = Vec::new();
        for a in &t.actions {
            let need = |v: &Option<String>, field: &str| {
                v.clone()
                    .ok_or_else(|| ctx(&format!("action `{}` needs `{field}`", a.kind)))
            };
            actions.push(match a.kind.as_str() {
                "text" => Action::Text {
                    text: need(&a.text, "text")?,
                },
                "objective" => Action::Objective {
                    index: objective_index(&need(&a.objective, "objective")?)?,
                    state: ObjectiveState::parse(&need(&a.state, "state")?)
                        .ok_or_else(|| ctx("state must be hidden, active, done or failed"))?,
                },
                "spawn" => {
                    let unit = need(&a.unit, "unit")?;
                    if is_structure(&unit) {
                        return Err(ctx("buildings cannot be spawned"));
                    }
                    Action::Spawn {
                        group: group_of(&need(&a.group, "group")?, &mut groups),
                        owner: known_player(a.owner.unwrap_or(player.0))?,
                        unit_type: type_index(&unit)?,
                        count: a.count.unwrap_or(1).clamp(1, 50),
                        at: place(&need(&a.at, "at")?)?,
                        fire_mode: if a.hold_fire.unwrap_or(false) {
                            FireMode::HoldFire
                        } else {
                            FireMode::FireAtWill
                        },
                    }
                }
                "camera" => Action::Camera {
                    at: place(&need(&a.at, "at")?)?,
                },
                "order" => Action::Order {
                    group: group_of(&need(&a.group, "group")?, &mut groups),
                    target: place(&need(&a.to, "to")?)?,
                    engage: match a.order.as_deref().unwrap_or("advance") {
                        "advance" => Engage::All,
                        "move" => Engage::Never,
                        _ => return Err(ctx("order must be advance or move")),
                    },
                },
                "fire" => Action::FireMode {
                    group: group_of(&need(&a.group, "group")?, &mut groups),
                    mode: if a.hold_fire.unwrap_or(false) {
                        FireMode::HoldFire
                    } else {
                        FireMode::FireAtWill
                    },
                },
                "victory" => Action::Victory,
                "defeat" => Action::Defeat,
                "barrage" => Action::Barrage {
                    group: group_of(&need(&a.group, "group")?, &mut groups),
                    target: place(&need(&a.at, "at")?)?,
                    radius_m: a.radius_m.unwrap_or(0),
                    salvos: a.salvos.unwrap_or(1).max(1),
                },
                "air" => {
                    let name = need(&a.mission, "mission")?;
                    Action::Air {
                        player: known_player(a.player.unwrap_or(player.0))?,
                        mission: AirMission::ALL
                            .into_iter()
                            .find(|m| m.name() == name)
                            .ok_or_else(|| ctx(&format!("unknown air mission '{name}'")))?,
                        target: place(&need(&a.at, "at")?)?,
                        to: a.to.as_deref().map(&place).transpose()?,
                    }
                }
                "camouflage" => Action::Camouflage {
                    group: group_of(&need(&a.group, "group")?, &mut groups),
                    on: a.on.unwrap_or(true),
                },
                other => return Err(ctx(&format!("unknown action `do = \"{other}\"`"))),
            });
        }
        triggers.push(Trigger {
            id: t.id.clone(),
            after,
            delay_ticks: t.delay_s.saturating_mul(tick_hz),
            condition,
            actions,
        });
    }
    let computers = file
        .computer
        .iter()
        .map(|c| Ok((known_player(c.player)?, c.difficulty.clone())))
        .collect::<Result<Vec<_>, MissionError>>()?;
    if computers.iter().any(|&(p, _)| p == player) {
        return err("the player cannot be a computer player");
    }

    let mission = Mission {
        id: id.to_owned(),
        title: header.title,
        tutorial,
        player,
        elimination: header.elimination,
        computers,
        areas,
        objectives,
        groups,
        start_groups,
        triggers,
    };
    Ok(SimConfig {
        tick_hz,
        unit_types,
        spawns,
        map: map.reference(),
        terrain: terrain.clone(),
        combat: CombatRules::builtin().map_err(|e| MissionError(e.to_string()))?,
        vision: crate::vision::VisionRules::builtin().map_err(|e| MissionError(e.to_string()))?,
        infantry: crate::infantry::InfantryRules::builtin()
            .map_err(|e| MissionError(e.to_string()))?,
        teams: crate::teams::Teams::default(),
        weather: map.weather(),
        mission: Some(mission),
    })
}

/// Base buildings and fortifications come from the economy rules.
fn is_structure(name: &str) -> bool {
    crate::economy::EconomyRules::builtin().is_ok_and(|r| r.structure(name).is_some())
}

/// The catalog id of a unit type of the match (for messages and tests).
pub fn type_name(types: &[UnitType], index: u32) -> &str {
    types.get(index as usize).map_or("", |t| t.name.as_str())
}
