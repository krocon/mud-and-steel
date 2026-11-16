//! Teams and lobby slots for Godot (prompt 037): starting a single-player
//! match from slots, relations between players for colors, pings to
//! allies, and the slot functions shared by the LAN and the Steam lobby.

use crate::{Game, SimHost, ground_to_godot, load_map_dir, meters_to_millimeters};
use godot::prelude::*;
use net::lobby::{Link, LobbyNode, LobbySetup, LobbySlot, SlotKind};
use sim::scenario::{
    DEMO_SCENARIO, SKIRMISH_SCENARIO, SlotSetup, builtin_map, parse_scenario_for_slots,
};
use sim::{Catalog, Command, Map, PlayerCommand, PlayerId, Scale, SimConfig, TerrainRules, Vec2};
use std::path::Path;
use std::sync::Arc;

/// Most slots a lobby offers (the largest maps have six starts).
pub(crate) const MAX_SLOTS: usize = 6;

/// Relation of a player to the local one, for colors.
const OWN: u8 = 0;
const ALLY: u8 = 1;
const ENEMY: u8 = 2;

/// Remembers the pings of the local player and its allies in this tick's
/// commands (the simulation ignores them).
pub(crate) fn collect_pings(game: &mut Game, commands: &[PlayerCommand]) {
    let local = game.session.local_player();
    for c in commands {
        if let Command::Ping { target } = c.command
            && game.sim.allied(c.player, local)
        {
            game.pings.push((c.player, target));
        }
    }
}

/// The scenario of a mode (`"base"`: base building, else the battle).
fn scenario_of(mode: &str) -> &'static str {
    if mode == "base" {
        SKIRMISH_SCENARIO
    } else {
        DEMO_SCENARIO
    }
}

/// The match of `slots` (players taking part with team and faction) on
/// `map`.
pub(crate) fn slots_config(
    mode: &str,
    map: &Map,
    slots: &[SlotSetup],
    shared_vision: bool,
) -> Result<SimConfig, String> {
    let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
    let scale = Scale::builtin().map_err(|e| e.to_string())?;
    let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
    parse_scenario_for_slots(
        scenario_of(mode),
        slots,
        shared_vision,
        &catalog,
        &scale,
        &terrain,
        map,
    )
    .map_err(|e| e.to_string())
}

/// The map in the folder `map_dir` (absolute), or the built-in one for "".
fn load_map(map_dir: &str) -> Result<Arc<Map>, String> {
    if map_dir.is_empty() {
        Ok(builtin_map())
    } else {
        load_map_dir(Path::new(map_dir)).map(Arc::new)
    }
}

/// The players taking part in a lobby setup.
fn setup_slots(setup: &LobbySetup) -> Vec<SlotSetup> {
    setup
        .participants()
        .into_iter()
        .map(|p| {
            let slot = &setup.slots[usize::from(p.0)];
            SlotSetup {
                player: p,
                team: slot.team,
                faction: (!slot.faction.is_empty()).then(|| slot.faction.clone()),
            }
        })
        .collect()
}

/// Host: starts the match of the lobby's setup for everyone, on the map
/// in `map_dir` (absolute path, "" = built-in); `hold_fire`: every unit
/// starts with hold fire (self-tests). Shared by the LAN and the Steam
/// lobby.
pub(crate) fn start_setup<L: Link>(
    node: &mut LobbyNode<L>,
    seed: i64,
    map_dir: &str,
    hold_fire: bool,
) -> Result<(), String> {
    let map = load_map(map_dir)?;
    let setup = node.setup().clone();
    let mut config = slots_config(&setup.mode, &map, &setup_slots(&setup), setup.shared_vision)?;
    if hold_fire {
        for spawn in &mut config.spawns {
            spawn.fire_mode = sim::FireMode::HoldFire;
        }
    }
    // The seed is a bit pattern; reinterpreting the sign is intended.
    node.start(
        seed.cast_unsigned(),
        config,
        crate::lan::LAN_INPUT_DELAY,
        crate::lan::LAN_HASH_INTERVAL,
    )
    .map(|_| ())
}

fn kind_name(kind: &SlotKind) -> &'static str {
    match kind {
        SlotKind::Open => "open",
        SlotKind::Human => "human",
        SlotKind::Computer(_) => "computer",
        SlotKind::Closed => "closed",
    }
}

/// The slots of a lobby: dictionaries with `index` (= player id), `kind`
/// (`open`, `human`, `computer`, `closed`), `level` (computer difficulty),
/// `team`, `faction` ("" = the scenario's), `name` and `ready` (humans),
/// `local` (our slot).
pub(crate) fn lobby_slots<L: Link>(node: &LobbyNode<L>) -> Array<VarDictionary> {
    let local = node.local_player();
    (0u8..)
        .zip(&node.setup().slots)
        .map(|(i, slot)| {
            let human = node.players().iter().find(|p| p.player.0 == i);
            let level = match &slot.kind {
                SlotKind::Computer(level) => level.clone(),
                _ => String::new(),
            };
            vdict! {
                "index" => i64::from(i),
                "kind" => kind_name(&slot.kind),
                "level" => level,
                "team" => i64::from(slot.team),
                "faction" => slot.faction.clone(),
                "name" => human.map_or(String::new(), |p| p.name.clone()),
                "ready" => human.is_some_and(|p| p.ready),
                "local" => local == Some(PlayerId(i)),
            }
        })
        .collect()
}

/// The match settings of a lobby: `map`, `mode`, `shared_vision`.
pub(crate) fn lobby_match<L: Link>(node: &LobbyNode<L>) -> VarDictionary {
    let setup = node.setup();
    vdict! {
        "map" => setup.map.clone(),
        "mode" => setup.mode.clone(),
        "shared_vision" => setup.shared_vision,
    }
}

/// Host: what slot `index` holds: `open`, `closed` or `computer` with the
/// difficulty `level` (slots of humans stay theirs). Client: nothing.
pub(crate) fn set_slot_kind<L: Link>(node: &mut LobbyNode<L>, index: i64, kind: &str, level: &str) {
    let kind = match kind {
        "open" => SlotKind::Open,
        "closed" => SlotKind::Closed,
        "computer" => match level.parse::<ai::Difficulty>() {
            Ok(d) => SlotKind::Computer(d.name().into()),
            Err(e) => {
                godot_warn!("{e}");
                return;
            }
        },
        other => {
            godot_warn!("unknown slot kind '{other}'");
            return;
        }
    };
    let Ok(index) = usize::try_from(index) else {
        return;
    };
    node.edit_setup(|s| {
        if let Some(slot) = s.slots.get_mut(index) {
            slot.kind = kind;
        }
    });
}

/// Team and faction of slot `index`: the host sets any slot, a client only
/// its own (the host decides).
pub(crate) fn set_slot_side<L: Link>(
    node: &mut LobbyNode<L>,
    index: i64,
    team: i64,
    faction: &str,
) {
    let (Ok(index), Ok(team)) = (usize::try_from(index), u8::try_from(team)) else {
        return;
    };
    if node.is_host() {
        node.edit_setup(|s| {
            if let Some(slot) = s.slots.get_mut(index) {
                slot.team = team;
                faction.clone_into(&mut slot.faction);
            }
        });
    } else if node.local_player().map(|p| usize::from(p.0)) == Some(index) {
        node.choose(team, faction);
    }
}

/// Host: map (folder name), mode, shared vision and the number of slots
/// (the map's start positions, at most [`MAX_SLOTS`]).
pub(crate) fn set_lobby_match<L: Link>(
    node: &mut LobbyNode<L>,
    map: &str,
    mode: &str,
    shared_vision: bool,
    slots: i64,
) {
    let count = usize::try_from(slots).unwrap_or(2).clamp(2, MAX_SLOTS);
    node.edit_setup(|s| {
        map.clone_into(&mut s.map);
        mode.clone_into(&mut s.mode);
        s.shared_vision = shared_vision;
        s.slots.truncate(count);
        while s.slots.len() < count {
            let team = u8::try_from(s.slots.len() % 2).unwrap_or(0);
            s.slots.push(LobbySlot {
                kind: SlotKind::Open,
                team,
                faction: String::new(),
            });
        }
    });
}

#[godot_api(secondary)]
impl SimHost {
    /// Starts a single-player match from slots (prompt 037): `slots[i]` is
    /// player `i`, a dictionary with `kind` (`human` for the local player
    /// in slot 0, `computer`, `closed` or `open`), `level` (difficulty of a
    /// computer), `team` and `faction` ("" = the scenario's). The mode is
    /// `SimHost.mode`; `map_dir` as in `start_match`.
    #[func]
    fn start_slots(
        &mut self,
        seed: i64,
        map_dir: GString,
        slots: VarArray,
        shared_vision: bool,
    ) -> bool {
        let map = match load_map(&map_dir.to_string()) {
            Ok(map) => map,
            Err(e) => {
                godot_error!("{e}");
                return false;
            }
        };
        let mut setup = Vec::new();
        let mut computers = Vec::new();
        for (i, slot) in (0u8..).zip(slots.iter_shared()) {
            let Ok(slot) = slot.try_to::<VarDictionary>() else {
                godot_error!("start_slots: slot {i} is no dictionary");
                return false;
            };
            let get = |key: &str| slot.get(key).map(|v| v.to::<GString>().to_string());
            let kind = get("kind").unwrap_or_default();
            if kind != "human" && kind != "computer" {
                continue;
            }
            if kind == "computer" {
                let level = get("level").unwrap_or_else(|| "normal".into());
                match level.parse::<ai::Difficulty>() {
                    Ok(d) => computers.push((PlayerId(i), d)),
                    Err(e) => {
                        godot_error!("{e}");
                        return false;
                    }
                }
            }
            let team = slot
                .get("team")
                .map_or(i % 2, |v| u8::try_from(v.to::<i64>()).unwrap_or(i % 2));
            let faction = get("faction").filter(|f| !f.is_empty());
            setup.push(SlotSetup {
                player: PlayerId(i),
                team,
                faction,
            });
        }
        if setup.first().map(|s| s.player) != Some(PlayerId(0)) {
            godot_error!("start_slots: slot 0 must be the local player");
            return false;
        }
        let config = match slots_config(&self.mode.to_string(), &map, &setup, shared_vision) {
            Ok(config) => config,
            Err(e) => {
                godot_error!("{e}");
                return false;
            }
        };
        self.computers = computers;
        self.start_config(seed, map, config)
    }

    /// Queues a ping at the ground point `(x, z)`: the games of the
    /// allies show it (prompt 037).
    #[func]
    fn ping(&mut self, x: f32, z: f32) {
        if let (Some(x), Some(z)) = (meters_to_millimeters(x), meters_to_millimeters(z))
            && self.game.is_some()
        {
            self.pending.push(Command::Ping {
                target: Vec2::from_millimeters(x, z),
            });
        }
    }

    /// Pings of the local player and its allies since the last call:
    /// dictionaries with `player`, `own` and `position` (Vector3 on the
    /// ground).
    #[func]
    fn take_pings(&mut self) -> Array<VarDictionary> {
        let Some(game) = self.game.as_mut() else {
            return Array::new();
        };
        let local = game.session.local_player();
        let pings = std::mem::take(&mut game.pings);
        pings
            .into_iter()
            .map(|(player, at)| {
                let height = game.sim.map().height_at(at);
                vdict! {
                    "player" => i64::from(player.0),
                    "own" => player == local,
                    "position" => ground_to_godot(at, height),
                }
            })
            .collect()
    }

    /// 0 for the local player, 1 for an ally, 2 for an enemy (prompt 037).
    #[func]
    fn relation(&self, player: i64) -> i64 {
        let Some(game) = &self.game else {
            return i64::from(ENEMY);
        };
        let local = PlayerId(u8::try_from(self.local_player()).unwrap_or(0));
        let Ok(player) = u8::try_from(player).map(PlayerId) else {
            return i64::from(ENEMY);
        };
        i64::from(relation(game, local, player))
    }

    /// [`relation`](Self::relation) of the owner of every unit, in the
    /// order of `unit_ids`.
    #[func]
    fn unit_relations(&self) -> PackedByteArray {
        let Some(game) = &self.game else {
            return PackedByteArray::new();
        };
        let local = PlayerId(u8::try_from(self.local_player()).unwrap_or(0));
        self.current
            .units
            .iter()
            .map(|u| relation(game, local, u.owner))
            .collect()
    }

    /// Players of the match: dictionaries with `id`, `team` (-1: on a team
    /// of their own), `relation` and `defeated`.
    #[func]
    fn match_players(&self) -> Array<VarDictionary> {
        let Some(game) = &self.game else {
            return Array::new();
        };
        let local = PlayerId(u8::try_from(self.local_player()).unwrap_or(0));
        let teams = game.sim.teams();
        game.sim
            .standings()
            .iter()
            .map(|s| {
                let team = teams
                    .members
                    .iter()
                    .find(|(p, _)| *p == s.player)
                    .map_or(-1, |&(_, t)| i64::from(t));
                vdict! {
                    "id" => i64::from(s.player.0),
                    "team" => team,
                    "relation" => i64::from(relation(game, local, s.player)),
                    "defeated" => s.defeat.is_some(),
                }
            })
            .collect()
    }

    /// Whether the local player's team won the decided match.
    #[func]
    fn local_team_won(&self) -> bool {
        let Some(game) = &self.game else {
            return false;
        };
        let local = PlayerId(u8::try_from(self.local_player()).unwrap_or(0));
        game.sim.winners().contains(&local)
    }
}

fn relation(game: &Game, local: PlayerId, player: PlayerId) -> u8 {
    if player == local {
        OWN
    } else if game.sim.allied(local, player) {
        ALLY
    } else {
        ENEMY
    }
}
