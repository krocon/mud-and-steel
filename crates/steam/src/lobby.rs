//! Steam lobbies: create, find, join, invite. Inside a lobby the game runs
//! the usual lobby protocol (`SteamNode`) with the lobby owner as host.

use crate::client::LobbyEvent;
use crate::{LOBBY_GAME, LOBBY_GAME_KEY, Steam, SteamLink, SteamNode};
use net::lobby::{LobbyState, PROTOCOL_VERSION};
use std::time::{Duration, Instant};
use steamworks::{DistanceFilter, LobbyId, LobbyKey, LobbyType, StringFilter, StringFilterKind};

/// Lobby data keys besides [`LOBBY_GAME_KEY`].
const KEY_PROTOCOL: &str = "protocol";
const KEY_NAME: &str = "name";
const KEY_STARTED: &str = "started";

/// A client gives up when the host does not answer its hello in time.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

/// A lobby from the lobby list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundLobby {
    pub id: u64,
    /// The host's name.
    pub name: String,
    pub players: usize,
    pub max_players: usize,
    /// Same protocol version as ours.
    pub compatible: bool,
}

/// Where an [`OnlineLobby`] is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnlinePhase {
    Idle,
    /// Waiting for Steam to create our lobby.
    Creating,
    /// Waiting for Steam to let us into a lobby.
    Joining,
    /// Inside a Steam lobby; the lobby protocol runs (see `node()`).
    InLobby(LobbyState),
    /// Failed or left; the reason. Hosting or joining starts over.
    Closed(String),
}

/// The online lobby screen's state. Call [`poll`](Self::poll) every frame.
pub struct OnlineLobby {
    steam: Steam,
    name: String,
    phase: Phase,
    found: Vec<FoundLobby>,
    max_players: usize,
    listing: bool,
    /// A lobby the user was invited to while busy.
    invitation: Option<u64>,
    /// The last problem that did not close anything (lobby list).
    message: String,
    /// The rich presence last set: (status, connect).
    presence: Option<(String, Option<String>)>,
}

enum Phase {
    Idle,
    Creating,
    Joining,
    In {
        node: Box<SteamNode>,
        since: Instant,
        /// Whether the lobby was already marked as started.
        marked_started: bool,
    },
    Closed(String),
}

impl OnlineLobby {
    /// Uses the Steam client (see [`Steam::get`]); the player name is the
    /// Steam persona name.
    pub fn new(steam: Steam) -> Self {
        let name = steam.user_name();
        Self {
            steam,
            name,
            phase: Phase::Idle,
            found: Vec::new(),
            max_players: 2,
            listing: false,
            invitation: None,
            message: String::new(),
            presence: None,
        }
    }

    pub const fn steam(&self) -> &Steam {
        &self.steam
    }

    pub fn user_name(&self) -> &str {
        &self.name
    }

    pub fn phase(&self) -> OnlinePhase {
        match &self.phase {
            Phase::Idle => OnlinePhase::Idle,
            Phase::Creating => OnlinePhase::Creating,
            Phase::Joining => OnlinePhase::Joining,
            Phase::In { node, .. } => OnlinePhase::InLobby(node.state().clone()),
            Phase::Closed(reason) => OnlinePhase::Closed(reason.clone()),
        }
    }

    /// The lobby protocol node, once inside a lobby.
    pub fn node(&self) -> Option<&SteamNode> {
        match &self.phase {
            Phase::In { node, .. } => Some(node),
            _ => None,
        }
    }

    pub fn node_mut(&mut self) -> Option<&mut SteamNode> {
        match &mut self.phase {
            Phase::In { node, .. } => Some(node),
            _ => None,
        }
    }

    /// The Steam lobby id ("lobby code"), once inside a lobby.
    pub fn lobby_id(&self) -> Option<u64> {
        self.node().map(|node| node.link().lobby())
    }

    /// The lobbies found by the last [`refresh`](Self::refresh).
    pub fn found(&self) -> &[FoundLobby] {
        &self.found
    }

    /// A hint about the last problem that did not close the lobby ("" if none).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// A lobby we were invited to while we were busy; join it to accept.
    pub const fn invitation(&self) -> Option<u64> {
        self.invitation
    }

    /// Creates a lobby for up to `max_players` players and hosts the game
    /// in it. `public`: listed for everyone, otherwise friends only.
    pub fn host(&mut self, max_players: usize, public: bool) {
        self.leave();
        self.max_players = max_players.clamp(2, 250);
        let kind = if public {
            LobbyType::Public
        } else {
            LobbyType::FriendsOnly
        };
        let sink = self.steam.event_sink();
        let limit = u32::try_from(self.max_players).unwrap_or(250);
        self.steam
            .client()
            .matchmaking()
            .create_lobby(kind, limit, move |result| {
                sink(LobbyEvent::Created(
                    result
                        .map(|lobby| lobby.raw())
                        .map_err(|e| format!("cannot create a Steam lobby: {e}")),
                ));
            });
        self.phase = Phase::Creating;
    }

    /// Joins the Steam lobby `lobby` (from the list, a code or an invite).
    pub fn join(&mut self, lobby: u64) {
        self.leave();
        self.invitation = None;
        let sink = self.steam.event_sink();
        self.steam
            .client()
            .matchmaking()
            .join_lobby(LobbyId::from_raw(lobby), move |result| {
                sink(LobbyEvent::Joined(result.map(|lobby| lobby.raw()).map_err(
                    |()| "cannot join the lobby (full, closed or no longer there)".to_owned(),
                )));
            });
        self.phase = Phase::Joining;
    }

    /// Asks Steam for open lobbies of this game; the answer arrives in
    /// [`found`](Self::found).
    pub fn refresh(&mut self) {
        if self.listing {
            return;
        }
        self.listing = true;
        self.message.clear();
        let matchmaking = self.steam.client().matchmaking();
        matchmaking
            .add_request_lobby_list_string_filter(StringFilter(
                LobbyKey::new(LOBBY_GAME_KEY),
                LOBBY_GAME,
                StringFilterKind::Equal,
            ))
            .set_request_lobby_list_distance_filter(DistanceFilter::Worldwide);
        let sink = self.steam.event_sink();
        matchmaking.request_lobby_list(move |result| {
            sink(LobbyEvent::List(
                result
                    .map(|lobbies| lobbies.iter().map(LobbyId::raw).collect())
                    .map_err(|e| format!("cannot list lobbies: {e}")),
            ));
        });
    }

    /// Opens the Steam overlay's invite dialog. Returns false without
    /// a lobby. The overlay only works when Steam started the game; friends
    /// can always join through the friends list or the lobby code.
    pub fn invite_friends(&self) -> bool {
        let Some(lobby) = self.lobby_id() else {
            return false;
        };
        self.steam
            .client()
            .friends()
            .activate_invite_dialog(LobbyId::from_raw(lobby));
        true
    }

    /// Whether the Steam overlay can be shown (needed for invites).
    pub fn overlay_enabled(&self) -> bool {
        self.steam.client().utils().is_overlay_enabled()
    }

    /// Processes Steam callbacks and lobby messages. Never blocks.
    pub fn poll(&mut self) {
        self.steam.run_callbacks();
        let events: Vec<LobbyEvent> = self.steam.shared().lobby_events.drain(..).collect();
        for event in events {
            self.on_event(event);
        }
        let mut timed_out = false;
        if let Phase::In {
            node,
            since,
            marked_started,
        } = &mut self.phase
        {
            node.poll();
            if *node.state() == LobbyState::Connecting && since.elapsed() > CONNECT_TIMEOUT {
                timed_out = true;
            }
            if *node.state() == LobbyState::Started && !*marked_started && node.is_host() {
                *marked_started = true;
                let lobby = LobbyId::from_raw(node.link().lobby());
                let matchmaking = self.steam.client().matchmaking();
                matchmaking.set_lobby_data(lobby, KEY_STARTED, "1");
                matchmaking.set_lobby_joinable(lobby, false);
            }
        }
        if timed_out {
            self.close("the host does not answer".into());
        } else {
            self.update_presence();
        }
    }

    /// Takes the started match's node (the transport for `SimHost`).
    pub fn take_started(&mut self) -> Result<SteamNode, String> {
        match self.node().map(|node| node.state().clone()) {
            Some(LobbyState::Started) => {}
            Some(LobbyState::Closed(reason)) => return Err(reason),
            Some(_) => return Err("the online match has not started".into()),
            None => return Err("not in a lobby".into()),
        }
        self.set_presence("In a match", None);
        match std::mem::replace(&mut self.phase, Phase::Idle) {
            Phase::In { node, .. } => Ok(*node),
            _ => unreachable!("checked above"),
        }
    }

    /// Leaves the lobby (if any); back to idle.
    pub fn leave(&mut self) {
        if let Phase::In { node, .. } = &mut self.phase {
            node.leave();
            node.poll();
        }
        // Dropping the node leaves the Steam lobby and clears the presence.
        self.phase = Phase::Idle;
        self.presence = None;
    }

    fn close(&mut self, reason: String) {
        if let Phase::In { node, .. } = &mut self.phase {
            node.leave();
        }
        self.phase = Phase::Closed(reason);
        self.presence = None;
    }

    fn on_event(&mut self, event: LobbyEvent) {
        match event {
            LobbyEvent::Created(result) => {
                if !matches!(self.phase, Phase::Creating) {
                    // Gave up in the meantime: do not stay in the lobby.
                    if let Ok(lobby) = result {
                        self.leave_steam_lobby(lobby);
                    }
                    return;
                }
                match result {
                    Ok(lobby) => self.enter_hosted(lobby),
                    Err(reason) => self.phase = Phase::Closed(reason),
                }
            }
            LobbyEvent::Joined(result) => {
                if !matches!(self.phase, Phase::Joining) {
                    if let Ok(lobby) = result {
                        self.leave_steam_lobby(lobby);
                    }
                    return;
                }
                match result {
                    Ok(lobby) => self.enter_joined(lobby),
                    Err(reason) => self.phase = Phase::Closed(reason),
                }
            }
            LobbyEvent::List(result) => {
                self.listing = false;
                match result {
                    Ok(lobbies) => self.found = self.describe(&lobbies),
                    Err(reason) => {
                        self.found.clear();
                        self.message = reason;
                    }
                }
            }
            LobbyEvent::JoinRequested(lobby) => match self.phase {
                Phase::Idle | Phase::Closed(_) => self.join(lobby),
                _ => self.invitation = Some(lobby),
            },
        }
    }

    fn describe(&self, lobbies: &[u64]) -> Vec<FoundLobby> {
        let matchmaking = self.steam.client().matchmaking();
        lobbies
            .iter()
            .filter_map(|&id| {
                let lobby = LobbyId::from_raw(id);
                let data = |key: &str| matchmaking.lobby_data(lobby, key).unwrap_or_default();
                parse_found_lobby(
                    id,
                    data(KEY_NAME),
                    &data(KEY_PROTOCOL),
                    matchmaking.lobby_member_count(lobby),
                    matchmaking.lobby_member_limit(lobby).unwrap_or(0),
                )
            })
            .collect()
    }

    fn enter_hosted(&mut self, lobby: u64) {
        let id = LobbyId::from_raw(lobby);
        let matchmaking = self.steam.client().matchmaking();
        matchmaking.set_lobby_data(id, LOBBY_GAME_KEY, LOBBY_GAME);
        matchmaking.set_lobby_data(id, KEY_PROTOCOL, &PROTOCOL_VERSION.to_string());
        matchmaking.set_lobby_data(id, KEY_NAME, &without_nul(&self.name));
        let link = SteamLink::host(self.steam.clone(), lobby);
        self.phase = Phase::In {
            node: Box::new(SteamNode::new_host(link, &self.name, self.max_players)),
            since: Instant::now(),
            marked_started: false,
        };
    }

    fn enter_joined(&mut self, lobby: u64) {
        let id = LobbyId::from_raw(lobby);
        let matchmaking = self.steam.client().matchmaking();
        let owner = matchmaking.lobby_owner(id).raw();
        let game = matchmaking
            .lobby_data(id, LOBBY_GAME_KEY)
            .unwrap_or_default();
        if let Err(reason) = validate_join_target(&game, owner, self.steam.user_id()) {
            self.leave_steam_lobby(lobby);
            self.phase = Phase::Closed(reason.into());
            return;
        }
        let link = SteamLink::client(self.steam.clone(), lobby, owner);
        self.phase = Phase::In {
            node: Box::new(SteamNode::new_client(link, owner, &self.name)),
            since: Instant::now(),
            marked_started: false,
        };
    }

    fn leave_steam_lobby(&self, lobby: u64) {
        self.steam
            .client()
            .matchmaking()
            .leave_lobby(LobbyId::from_raw(lobby));
    }

    /// Rich presence: friends see that we are in a lobby and can join it
    /// from the friends list ("Join game").
    fn update_presence(&mut self) {
        let Phase::In { node, .. } = &self.phase else {
            return;
        };
        let connect = presence_connect(node.link().lobby(), node.is_host(), node.state());
        self.set_presence("In a lobby", connect);
    }

    /// Sets rich presence if it changed. `SteamLink` clears it on leaving.
    fn set_presence(&mut self, status: &str, connect: Option<String>) {
        let wanted = (status.to_owned(), connect);
        if self.presence.as_ref() == Some(&wanted) {
            return;
        }
        let friends = self.steam.client().friends();
        friends.set_rich_presence("status", Some(status));
        friends.set_rich_presence("connect", wanted.1.as_deref());
        // Groups the players of one lobby in the friends list.
        let group = self.lobby_id().map(|lobby| lobby.to_string());
        friends.set_rich_presence("steam_player_group", group.as_deref());
        self.presence = Some(wanted);
    }
}

fn without_nul(text: &str) -> String {
    text.replace('\0', "")
}

fn parse_found_lobby(
    id: u64,
    name: String,
    protocol: &str,
    players: usize,
    max_players: usize,
) -> Option<FoundLobby> {
    if name.is_empty() {
        return None;
    }
    Some(FoundLobby {
        id,
        name,
        players,
        max_players,
        compatible: protocol == PROTOCOL_VERSION.to_string(),
    })
}

fn validate_join_target(game: &str, owner: u64, self_id: u64) -> Result<(), &'static str> {
    if game != LOBBY_GAME {
        Err("not a Mud and Steel lobby")
    } else if owner == self_id {
        Err("this is your own lobby")
    } else {
        Ok(())
    }
}

fn presence_connect(lobby: u64, is_host: bool, state: &LobbyState) -> Option<String> {
    (is_host && *state == LobbyState::Lobby)
        .then(|| format!("{} {lobby}", crate::CONNECT_LOBBY_ARG))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_nul_strips_embedded_nuls() {
        assert_eq!(without_nul("a\0b\0"), "ab");
        assert_eq!(without_nul("plain"), "plain");
    }

    #[test]
    fn parse_found_lobby_filters_empty_and_checks_protocol() {
        let proto = PROTOCOL_VERSION.to_string();
        assert_eq!(parse_found_lobby(1, String::new(), &proto, 1, 4), None);
        let ok = parse_found_lobby(2, "Host".into(), &proto, 2, 4).unwrap();
        assert!(ok.compatible);
        assert_eq!(ok.players, 2);
        let old = parse_found_lobby(3, "Old".into(), "0", 1, 2).unwrap();
        assert!(!old.compatible);
    }

    #[test]
    fn validate_join_target_rejects_foreign_game_and_self_lobby() {
        assert_eq!(validate_join_target(LOBBY_GAME, 42, 99), Ok(()));
        assert!(validate_join_target("other-game", 42, 99).is_err());
        assert!(validate_join_target(LOBBY_GAME, 42, 42).is_err());
    }

    #[test]
    fn presence_connect_only_advertises_joinable_host_lobbies() {
        let expected = format!("{} 12345", crate::CONNECT_LOBBY_ARG);
        assert_eq!(
            presence_connect(12345, true, &LobbyState::Lobby),
            Some(expected)
        );
        assert_eq!(presence_connect(12345, false, &LobbyState::Lobby), None);
        assert_eq!(presence_connect(12345, true, &LobbyState::Started), None);
        assert_eq!(
            presence_connect(12345, true, &LobbyState::Closed("left".into())),
            None
        );
    }
}
