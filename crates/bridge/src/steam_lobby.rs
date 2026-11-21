//! Online lobby over Steam for Godot (prompt 026): connect to Steam, host,
//! list, join by code or invite, ready, start.
//!
//! Like `LanLobby` a `RefCounted`, so it survives the scene change to
//! `main.tscn`; `SimHost.start_online` then takes the started connection.
//! The class always exists: built without the `steam` feature, or without a
//! running Steam client, `connect_steam()` fails with a readable message.
//! The Steam code itself lives in `crates/steam`.

use crate::SimHost;
use godot::classes::{IRefCounted, RefCounted};
use godot::prelude::*;

#[cfg(feature = "steam")]
use net::lobby::LobbyState;
#[cfg(feature = "steam")]
use steam::{OnlineLobby, OnlinePhase, Steam};

/// One online game from the lobby's point of view. Call `poll()` every frame.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct SteamLobby {
    #[cfg(feature = "steam")]
    lobby: Option<OnlineLobby>,
    message: String,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for SteamLobby {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            #[cfg(feature = "steam")]
            lobby: None,
            message: String::new(),
            base,
        }
    }
}

#[godot_api]
impl SteamLobby {
    /// Whether this build includes Steam (the `steam` cargo feature).
    #[func]
    fn is_built_with_steam() -> bool {
        cfg!(feature = "steam")
    }

    /// The Steam app id (`steam::APP_ID`), 0 without Steam.
    #[func]
    fn app_id() -> i64 {
        #[cfg(feature = "steam")]
        return i64::from(steam::APP_ID);
        #[cfg(not(feature = "steam"))]
        0
    }

    /// The lobby code from the launch option `+connect_lobby <id>` (Steam
    /// starts the game like that to join a friend), or "".
    #[func]
    fn launch_lobby_code() -> GString {
        #[cfg(feature = "steam")]
        {
            let args: Vec<String> = std::env::args().collect();
            if let Some(lobby) = steam::connect_lobby_arg(&args) {
                return GString::from(lobby.to_string().as_str());
            }
        }
        GString::new()
    }

    /// Initializes Steam (once per process). Returns false (see
    /// `message()`) if Steam is not running or this build has no Steam.
    #[func]
    fn connect_steam(&mut self) -> bool {
        #[cfg(feature = "steam")]
        {
            if self.lobby.is_some() {
                return true;
            }
            match Steam::get() {
                Ok(steam) => {
                    self.lobby = Some(OnlineLobby::new(steam));
                    self.message.clear();
                    true
                }
                Err(e) => self.fail(e),
            }
        }
        #[cfg(not(feature = "steam"))]
        self.fail("This build has no Steam support (cargo feature `steam`).".into())
    }

    /// Our Steam name ("" before `connect_steam`).
    #[func]
    fn user_name(&self) -> GString {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &self.lobby {
            return GString::from(lobby.user_name());
        }
        GString::new()
    }

    /// Creates a Steam lobby and hosts the demo scenario in it. `public`:
    /// listed for everyone; otherwise friends only.
    #[func]
    fn host(&mut self, max_players: i64, public: bool) -> bool {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &mut self.lobby {
            self.message.clear();
            lobby.host(usize::try_from(max_players).unwrap_or(2), public);
            return true;
        }
        let _ = (max_players, public);
        self.not_connected()
    }

    /// Joins the lobby with the given code (the Steam lobby id).
    #[func]
    fn join(&mut self, code: GString) -> bool {
        let code = code.to_string();
        let Ok(id) = code.trim().parse::<u64>() else {
            return self.fail(format!("invalid lobby code '{code}'"));
        };
        #[cfg(feature = "steam")]
        if let Some(lobby) = &mut self.lobby {
            self.message.clear();
            lobby.join(id);
            return true;
        }
        let _ = id;
        self.not_connected()
    }

    /// Asks Steam for open lobbies; they arrive in `found_lobbies()`.
    #[func]
    fn refresh(&mut self) {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &mut self.lobby {
            lobby.refresh();
        }
    }

    /// Lobbies from the last `refresh`: dictionaries with `code`, `name`,
    /// `players`, `max_players` and `compatible`.
    #[func]
    fn found_lobbies(&self) -> Array<VarDictionary> {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &self.lobby {
            return lobby
                .found()
                .iter()
                .map(|found| {
                    vdict! {
                        "code" => found.id.to_string(),
                        "name" => found.name.clone(),
                        "players" => i64::try_from(found.players).unwrap_or(0),
                        "max_players" => i64::try_from(found.max_players).unwrap_or(0),
                        "compatible" => found.compatible,
                    }
                })
                .collect();
        }
        Array::new()
    }

    /// Processes Steam callbacks and lobby messages. Call every frame.
    #[func]
    fn poll(&mut self) {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &mut self.lobby {
            lobby.poll();
        }
    }

    /// "unavailable" (not connected to Steam), "idle", "creating",
    /// "joining", "connecting", "lobby", "started" or "closed".
    #[func]
    fn state(&self) -> GString {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &self.lobby {
            let state = match lobby.phase() {
                OnlinePhase::Idle => "idle",
                OnlinePhase::Creating => "creating",
                OnlinePhase::Joining => "joining",
                OnlinePhase::InLobby(LobbyState::Connecting) => "connecting",
                OnlinePhase::InLobby(LobbyState::Lobby) => "lobby",
                OnlinePhase::InLobby(LobbyState::Started) => "started",
                OnlinePhase::InLobby(LobbyState::Closed(_)) | OnlinePhase::Closed(_) => "closed",
            };
            return GString::from(state);
        }
        GString::from("unavailable")
    }

    /// Why the game closed or an action failed, or a hint ("" if none).
    #[func]
    fn message(&self) -> GString {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &self.lobby {
            match lobby.phase() {
                OnlinePhase::Closed(reason) | OnlinePhase::InLobby(LobbyState::Closed(reason)) => {
                    return GString::from(reason.as_str());
                }
                _ if self.message.is_empty() => return GString::from(lobby.message()),
                _ => {}
            }
        }
        GString::from(self.message.as_str())
    }

    #[func]
    fn is_host(&self) -> bool {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_ref().and_then(OnlineLobby::node) {
            return node.is_host();
        }
        false
    }

    /// The code of the current lobby (to give to friends), or "".
    #[func]
    fn lobby_code(&self) -> GString {
        #[cfg(feature = "steam")]
        if let Some(id) = self.lobby.as_ref().and_then(OnlineLobby::lobby_id) {
            return GString::from(id.to_string().as_str());
        }
        GString::new()
    }

    /// The code of a lobby we were invited to while busy, or "".
    #[func]
    fn invitation_code(&self) -> GString {
        #[cfg(feature = "steam")]
        if let Some(id) = self.lobby.as_ref().and_then(OnlineLobby::invitation) {
            return GString::from(id.to_string().as_str());
        }
        GString::new()
    }

    /// Players in the lobby: dictionaries with `id`, `name`, `ready`, `local`.
    #[func]
    fn players(&self) -> Array<VarDictionary> {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_ref().and_then(OnlineLobby::node) {
            let local = node.local_player();
            return node
                .players()
                .iter()
                .map(|p| {
                    vdict! {
                        "id" => i64::from(p.player.0),
                        "name" => p.name.clone(),
                        "ready" => p.ready,
                        "local" => Some(p.player) == local,
                    }
                })
                .collect();
        }
        Array::new()
    }

    /// Client: tells the host whether we are ready.
    #[func]
    fn set_ready(&mut self, ready: bool) {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            node.set_ready(ready);
        }
        let _ = ready;
    }

    /// Host: whether all players are ready (at least two).
    #[func]
    fn can_start(&self) -> bool {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_ref().and_then(OnlineLobby::node) {
            return node.can_start();
        }
        false
    }

    /// Slots of the lobby (prompt 037), see `team_funcs::lobby_slots`.
    #[func]
    fn slots(&self) -> Array<VarDictionary> {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_ref().and_then(OnlineLobby::node) {
            return crate::team_funcs::lobby_slots(node);
        }
        Array::new()
    }

    /// Map, mode and shared vision of the lobby.
    #[func]
    fn match_settings(&self) -> VarDictionary {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_ref().and_then(OnlineLobby::node) {
            return crate::team_funcs::lobby_match(node);
        }
        VarDictionary::new()
    }

    /// Host: `open`, `closed` or `computer` (with difficulty `level`) for
    /// slot `index`.
    #[func]
    fn set_slot_kind(&mut self, index: i64, kind: GString, level: GString) {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            crate::team_funcs::set_slot_kind(node, index, &kind.to_string(), &level.to_string());
        }
        let _ = (index, kind, level);
    }

    /// Team and faction of slot `index` (a client only its own).
    #[func]
    fn set_slot_side(&mut self, index: i64, team: i64, faction: GString) {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            crate::team_funcs::set_slot_side(node, index, team, &faction.to_string());
        }
        let _ = (index, team, faction);
    }

    /// Host: map name, mode (`battle`, `base`), shared vision and number
    /// of slots (the map's starts).
    #[func]
    fn set_match(&mut self, map: GString, mode: GString, shared_vision: bool, slots: i64) {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            crate::team_funcs::set_lobby_match(
                node,
                &map.to_string(),
                &mode.to_string(),
                shared_vision,
                slots,
            );
        }
        let _ = (map, mode, shared_vision, slots);
    }

    /// Host: the start layout of the map (prompt 182), as `LanLobby.set_variant`.
    #[func]
    fn set_variant(&mut self, variant: GString) {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            crate::team_funcs::set_lobby_variant(node, &variant.to_string());
        }
        let _ = variant;
    }

    /// Host: starts the lobby's match for everyone, on the built-in map or
    /// the map folder `map_dir` (absolute path, "" = built-in).
    #[func]
    fn start_match(&mut self, seed: i64, map_dir: GString) -> bool {
        #[cfg(feature = "steam")]
        if let Some(node) = self.lobby.as_mut().and_then(OnlineLobby::node_mut) {
            return match crate::team_funcs::start_setup(node, seed, &map_dir.to_string(), false) {
                Ok(()) => true,
                Err(e) => self.fail(e),
            };
        }
        let _ = (seed, map_dir);
        self.fail("not hosting".into())
    }

    /// Opens the Steam overlay's invite dialog (only when Steam started the
    /// game). Returns false if the overlay is not available.
    #[func]
    fn invite_friends(&mut self) -> bool {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &self.lobby {
            if !lobby.overlay_enabled() {
                return self.fail(
                    "The Steam overlay is not available (game not started by Steam). \
                     Friends can join from the Steam friends list or with the lobby code."
                        .into(),
                );
            }
            return lobby.invite_friends();
        }
        false
    }

    /// Leaves the lobby (not started yet). Steam stays connected.
    #[func]
    fn leave(&mut self) {
        #[cfg(feature = "steam")]
        if let Some(lobby) = &mut self.lobby {
            lobby.leave();
        }
        self.message.clear();
    }
}

impl SteamLobby {
    fn fail(&mut self, message: String) -> bool {
        godot_warn!("Steam: {message}");
        self.message = message;
        false
    }

    fn not_connected(&mut self) -> bool {
        self.fail("not connected to Steam".into())
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Starts the online match that `lobby` (state "started") prepared, like
    /// `start_lan`. Returns false (and logs why) on failure.
    #[func]
    fn start_online(&mut self, mut lobby: Gd<SteamLobby>, maps_dir: GString) -> bool {
        #[cfg(feature = "steam")]
        {
            let started = lobby
                .bind_mut()
                .lobby
                .as_mut()
                .ok_or_else(|| "not connected to Steam".to_owned())
                .and_then(OnlineLobby::take_started);
            match started {
                Ok(node) => self.start_networked(node, &maps_dir.to_string()),
                Err(e) => {
                    godot_error!("cannot start the online match: {e}");
                    false
                }
            }
        }
        #[cfg(not(feature = "steam"))]
        {
            let _ = (&mut lobby, maps_dir);
            godot_error!("cannot start the online match: built without Steam");
            false
        }
    }
}
