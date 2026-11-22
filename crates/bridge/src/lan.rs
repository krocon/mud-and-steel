//! LAN lobby for Godot: host, search, join, ready, start (prompt 025).
//!
//! `LanLobby` is a `RefCounted`, so it survives the scene change from the
//! lobby scene to `main.tscn`; `SimHost.start_lan` then takes the started
//! connection out of it. The network code itself lives in `net::lan`.

use crate::{SimHost, team_funcs};
use godot::classes::{IRefCounted, RefCounted};
use godot::prelude::*;
use net::lan::{
    DEFAULT_DISCOVERY_PORT, DEFAULT_GAME_PORT, DiscoveryBrowser, DiscoveryResponder, GameInfo,
    LanNode, LanState, PROTOCOL_VERSION,
};
use net::lobby::{Link, LobbyNode};
use sim::Sim;
use sim::scenario::{BUILTIN_MAP_NAME, builtin_map};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

/// State hashes are compared every this many ticks in LAN games (1 s).
pub const LAN_HASH_INTERVAL: u64 = 20;

/// Initial input delay of LAN games; adapted to the round trip afterwards.
pub const LAN_INPUT_DELAY: u32 = 3;

/// One LAN game from the lobby's point of view. Call `poll()` every frame.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct LanLobby {
    node: Option<LanNode>,
    responder: Option<DiscoveryResponder>,
    browser: Option<DiscoveryBrowser>,
    name: String,
    message: String,
    /// Host: every unit starts with hold fire (self-tests that end by
    /// surrender, prompt 037). Part of the config, so the same everywhere.
    #[var]
    hold_fire_at_start: bool,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for LanLobby {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            node: None,
            responder: None,
            browser: None,
            name: String::new(),
            message: String::new(),
            hold_fire_at_start: false,
            base,
        }
    }
}

#[godot_api]
impl LanLobby {
    #[constant]
    const DEFAULT_GAME_PORT: i64 = DEFAULT_GAME_PORT as i64;
    #[constant]
    const DEFAULT_DISCOVERY_PORT: i64 = DEFAULT_DISCOVERY_PORT as i64;

    /// Hosts a game on `port` and answers searches on `discovery_port`:
    /// the battle on the built-in map with two slots until `set_match`
    /// changes it (prompt 037). Returns false (see `message()`) if the
    /// port is taken.
    #[func]
    fn host(&mut self, name: GString, port: i64, discovery_port: i64) -> bool {
        self.reset();
        self.name = name.to_string();
        let mut node = match LanNode::host(&self.name, to_port(port), team_funcs::MAX_SLOTS) {
            Ok(node) => node,
            Err(e) => return self.fail(format!("cannot host on port {port}: {e}")),
        };
        team_funcs::set_lobby_match(&mut node, BUILTIN_MAP_NAME, "battle", true, 2);
        match DiscoveryResponder::new(to_port(discovery_port), self.game_info(&node)) {
            Ok(responder) => self.responder = Some(responder),
            Err(e) => {
                self.message = format!(
                    "Not visible in the LAN search (port {discovery_port}: {e}); join by address."
                );
            }
        }
        self.node = Some(node);
        true
    }

    /// Joins the game at `address` ("ip" or "ip:port"; default port 47810).
    #[func]
    fn join(&mut self, name: GString, address: GString) -> bool {
        let address = address.to_string();
        let Some(address) = parse_address(address.trim()) else {
            return self.fail(format!("invalid address '{address}'"));
        };
        self.reset();
        self.name = name.to_string();
        match LanNode::join(&self.name, address) {
            Ok(node) => {
                self.node = Some(node);
                true
            }
            Err(e) => self.fail(format!("cannot join {address}: {e}")),
        }
    }

    /// Sends a search query on the LAN; answers arrive in `found_games()`.
    #[func]
    fn search(&mut self, discovery_port: i64) -> bool {
        if self.browser.is_none() {
            match DiscoveryBrowser::new(to_port(discovery_port)) {
                Ok(browser) => self.browser = Some(browser),
                Err(e) => return self.fail(format!("cannot search the LAN: {e}")),
            }
        }
        if let Some(browser) = &mut self.browser {
            browser.clear();
            browser.query();
        }
        true
    }

    /// Processes network events. Call every frame while the lobby is shown.
    #[func]
    fn poll(&mut self) {
        if let Some(browser) = &mut self.browser {
            browser.poll();
        }
        let Some(node) = &mut self.node else {
            return;
        };
        node.poll();
        if *node.state() == LanState::Lobby {
            let info = self.game_info(self.node.as_ref().expect("checked above"));
            if let Some(responder) = &mut self.responder {
                responder.set_info(info);
                responder.poll();
            }
        } else {
            // Not joinable any more.
            self.responder = None;
        }
    }

    /// Games found by `search`: dictionaries with `name`, `address`
    /// ("ip:port"), `players`, `max_players` and `compatible`.
    #[func]
    fn found_games(&self) -> Array<VarDictionary> {
        let Some(browser) = &self.browser else {
            return Array::new();
        };
        browser
            .games()
            .iter()
            .map(|game| {
                vdict! {
                    "name" => game.info.name.clone(),
                    "address" => game.address.to_string(),
                    "players" => i64::from(game.info.players),
                    "max_players" => i64::from(game.info.max_players),
                    "compatible" => game.info.protocol == PROTOCOL_VERSION,
                }
            })
            .collect()
    }

    /// "idle", "connecting", "lobby", "started" or "closed".
    #[func]
    fn state(&self) -> GString {
        let state = match self.node.as_ref().map(LanNode::state) {
            None => "idle",
            Some(LanState::Connecting) => "connecting",
            Some(LanState::Lobby) => "lobby",
            Some(LanState::Started) => "started",
            Some(LanState::Closed(_)) => "closed",
        };
        GString::from(state)
    }

    /// Why the game closed or an action failed, or a hint ("" if none).
    #[func]
    fn message(&self) -> GString {
        match self.node.as_ref().map(LanNode::state) {
            Some(LanState::Closed(reason)) => GString::from(reason.as_str()),
            _ => GString::from(self.message.as_str()),
        }
    }

    #[func]
    fn is_host(&self) -> bool {
        self.node.as_ref().is_some_and(LanNode::is_host)
    }

    /// UDP port of the hosted game (0 if not hosting).
    #[func]
    fn port(&self) -> i64 {
        self.node
            .as_ref()
            .filter(|n| n.is_host())
            .map_or(0, |n| i64::from(n.port()))
    }

    /// Players in the lobby: dictionaries with `id`, `name`, `ready`, `local`.
    #[func]
    fn players(&self) -> Array<VarDictionary> {
        let Some(node) = &self.node else {
            return Array::new();
        };
        let local = node.local_player();
        node.players()
            .iter()
            .map(|p| {
                vdict! {
                    "id" => i64::from(p.player.0),
                    "name" => p.name.clone(),
                    "ready" => p.ready,
                    "local" => Some(p.player) == local,
                }
            })
            .collect()
    }

    /// Client: tells the host whether we are ready.
    #[func]
    fn set_ready(&mut self, ready: bool) {
        if let Some(node) = &mut self.node {
            node.set_ready(ready);
        }
    }

    /// Host: whether all players are ready (at least two).
    #[func]
    fn can_start(&self) -> bool {
        self.node.as_ref().is_some_and(LanNode::can_start)
    }

    /// Slots of the lobby (prompt 037), see `team_funcs::lobby_slots`.
    #[func]
    fn slots(&self) -> Array<VarDictionary> {
        self.node
            .as_ref()
            .map_or_else(Array::new, team_funcs::lobby_slots)
    }

    /// Map, mode and shared vision of the lobby.
    #[func]
    fn match_settings(&self) -> VarDictionary {
        self.node
            .as_ref()
            .map_or_else(VarDictionary::new, team_funcs::lobby_match)
    }

    /// Host: `open`, `closed` or `computer` (with difficulty `level`) for
    /// slot `index`.
    #[func]
    fn set_slot_kind(&mut self, index: i64, kind: GString, level: GString) {
        if let Some(node) = &mut self.node {
            team_funcs::set_slot_kind(node, index, &kind.to_string(), &level.to_string());
        }
    }

    /// Team and faction of slot `index` (a client only its own).
    #[func]
    fn set_slot_side(&mut self, index: i64, team: i64, faction: GString) {
        if let Some(node) = &mut self.node {
            team_funcs::set_slot_side(node, index, team, &faction.to_string());
        }
    }

    /// Host: map name, mode (`battle`, `base`), shared vision and number
    /// of slots (the map's starts).
    #[func]
    fn set_match(&mut self, map: GString, mode: GString, shared_vision: bool, slots: i64) {
        if let Some(node) = &mut self.node {
            team_funcs::set_lobby_match(
                node,
                &map.to_string(),
                &mode.to_string(),
                shared_vision,
                slots,
            );
        }
    }

    /// Host: the start layout of the map (prompt 182): "" / `standard`,
    /// `random` or a variant name (`SimHost.map_variants`).
    #[func]
    fn set_variant(&mut self, variant: GString) {
        if let Some(node) = &mut self.node {
            team_funcs::set_lobby_variant(node, &variant.to_string());
        }
    }

    /// Host: mission id and difficulty of the mode `mission` (prompt 190).
    #[func]
    fn set_mission(&mut self, mission: GString, level: GString) {
        if let Some(node) = &mut self.node {
            team_funcs::set_lobby_mission(node, &mission.to_string(), &level.to_string());
        }
    }

    /// Host: starts the lobby's match for everyone, on the built-in map or
    /// the map folder `map_dir` (absolute path, "" = built-in).
    #[func]
    fn start_match(&mut self, seed: i64, map_dir: GString) -> bool {
        let hold_fire = self.hold_fire_at_start;
        let Some(node) = &mut self.node else {
            return self.fail("not hosting".into());
        };
        match team_funcs::start_setup(node, seed, &map_dir.to_string(), hold_fire) {
            Ok(()) => {
                self.responder = None;
                true
            }
            Err(e) => self.fail(e),
        }
    }

    /// Leaves the game (lobby or not started yet).
    #[func]
    fn leave(&mut self) {
        if let Some(node) = &mut self.node {
            node.leave();
            node.poll();
        }
        self.reset();
    }
}

impl LanLobby {
    /// Takes the started connection (for `SimHost.start_lan`).
    pub fn take_started(&mut self) -> Result<LanNode, String> {
        match self.node.as_ref().map(LanNode::state) {
            Some(LanState::Started) => Ok(self.node.take().expect("checked above")),
            Some(LanState::Closed(reason)) => Err(reason.clone()),
            _ => Err("the LAN match has not started".into()),
        }
    }

    fn game_info(&self, node: &LanNode) -> GameInfo {
        GameInfo {
            protocol: PROTOCOL_VERSION,
            name: self.name.clone(),
            game_port: node.port(),
            players: u8::try_from(node.players().len()).unwrap_or(u8::MAX),
            max_players: node
                .max_players()
                .and_then(|m| u8::try_from(m).ok())
                .unwrap_or(u8::MAX),
        }
    }

    fn reset(&mut self) {
        self.node = None;
        self.responder = None;
        self.message.clear();
    }

    fn fail(&mut self, message: String) -> bool {
        godot_warn!("LAN: {message}");
        self.message = message;
        false
    }
}

fn to_port(port: i64) -> u16 {
    u16::try_from(port).unwrap_or(0)
}

fn parse_address(text: &str) -> Option<SocketAddr> {
    text.parse::<SocketAddr>().ok().or_else(|| {
        Some(SocketAddr::new(
            text.parse::<IpAddr>().ok()?,
            DEFAULT_GAME_PORT,
        ))
    })
}

impl SimHost {
    /// Starts the network match that `node` (state "started") prepared: the
    /// host's seed and config, the map loaded locally (built-in, or from
    /// `maps_dir/<name>/`). Returns false (and logs why) on failure.
    pub(crate) fn start_networked<L: Link + 'static>(
        &mut self,
        node: LobbyNode<L>,
        maps_dir: &str,
    ) -> bool {
        let (Some(info), Some(local)) = (node.start_info().cloned(), node.local_player()) else {
            godot_error!("cannot start the network match: no start message");
            return false;
        };
        let name = &info.config.map.name;
        let map = if *name == BUILTIN_MAP_NAME {
            builtin_map()
        } else {
            match crate::load_map_dir(&Path::new(maps_dir).join(name)) {
                Ok(map) => Arc::new(map),
                Err(e) => {
                    godot_error!("cannot load the map of the network match: {e}");
                    return false;
                }
            }
        };
        // The host runs the computer players (prompt 037).
        let computers: Vec<(sim::PlayerId, ai::Difficulty)> = if node.is_host() {
            info.computers
                .iter()
                .filter_map(|(p, level)| Some((*p, level.parse().ok()?)))
                .collect()
        } else {
            Vec::new()
        };
        // Rejects a map with a different content hash than the host's.
        let sim = match Sim::new(&info.config, map.clone(), info.seed) {
            Ok(sim) => sim,
            Err(e) => {
                godot_error!("cannot start the network match: {e}");
                return false;
            }
        };
        self.input_delay = info.input_delay;
        self.install(
            sim,
            map,
            info.config.clone(),
            info.seed,
            local,
            &info.lockstep_config(),
            Box::new(node),
        );
        if let Some(game) = self.game.as_mut() {
            crate::ai_funcs::host_computers(game, &computers, info.seed);
        }
        true
    }
}
