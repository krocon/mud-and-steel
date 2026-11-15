//! One peer of a multiplayer game: lobby first, then the match transport.
//! Generic over the [`Link`] that moves the bytes (ENet, Steam, memory).

use super::PROTOCOL_VERSION;
use super::link::{Link, LinkEvent};
use super::wire::{CONTROL_CHANNEL, Control, GAME_CHANNEL, LobbyPlayer, StartInfo, decode, encode};
use crate::{PlayerId, Transport, TransportError};
use sim::SimConfig;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyState {
    /// Client: connecting to the host, not yet accepted.
    Connecting,
    /// In the lobby, waiting for the start.
    Lobby,
    /// The match runs; the node is now a `Transport`.
    Started,
    /// Connection lost, rejected or a player left. Final.
    Closed(String),
}

enum Role<P> {
    Host {
        /// Connected peers and their player once they said hello.
        peers: BTreeMap<P, Option<PlayerId>>,
        max_players: usize,
    },
    Client {
        server: P,
    },
}

/// One peer of a game. See the [module documentation](super).
///
/// Call [`poll`](Self::poll) regularly (every frame) in the lobby. During the
/// match, [`Transport::recv`] polls.
pub struct LobbyNode<L: Link> {
    link: L,
    role: Role<L::Peer>,
    name: String,
    local: Option<PlayerId>,
    lobby: Vec<LobbyPlayer>,
    start: Option<StartInfo>,
    state: LobbyState,
    /// Received lockstep packets, in arrival order.
    inbox: VecDeque<Vec<u8>>,
}

impl<L: Link> LobbyNode<L> {
    /// Hosts a game on `link` for up to `max_players` players including the
    /// host, who is player 0. Peers connect through the link.
    pub fn new_host(link: L, name: &str, max_players: usize) -> Self {
        let local = PlayerId(0);
        Self {
            link,
            role: Role::Host {
                peers: BTreeMap::new(),
                max_players: max_players.clamp(1, usize::from(u8::MAX)),
            },
            name: name.to_owned(),
            local: Some(local),
            lobby: vec![LobbyPlayer {
                player: local,
                name: name.to_owned(),
                ready: true,
            }],
            start: None,
            state: LobbyState::Lobby,
            inbox: VecDeque::new(),
        }
    }

    /// Joins the game hosted by `server`. The hello is sent once the link
    /// reports the server as connected.
    pub fn new_client(link: L, server: L::Peer, name: &str) -> Self {
        Self {
            link,
            role: Role::Client { server },
            name: name.to_owned(),
            local: None,
            lobby: Vec::new(),
            start: None,
            state: LobbyState::Connecting,
            inbox: VecDeque::new(),
        }
    }

    pub const fn link(&self) -> &L {
        &self.link
    }

    pub const fn link_mut(&mut self) -> &mut L {
        &mut self.link
    }

    pub const fn is_host(&self) -> bool {
        matches!(self.role, Role::Host { .. })
    }

    pub const fn state(&self) -> &LobbyState {
        &self.state
    }

    /// Our player id, once accepted (the host is always player 0).
    pub const fn local_player(&self) -> Option<PlayerId> {
        self.local
    }

    /// Players in the lobby, ascending by id.
    pub fn players(&self) -> &[LobbyPlayer] {
        &self.lobby
    }

    pub const fn max_players(&self) -> Option<usize> {
        match self.role {
            Role::Host { max_players, .. } => Some(max_players),
            Role::Client { .. } => None,
        }
    }

    /// The start message, once the match started.
    pub const fn start_info(&self) -> Option<&StartInfo> {
        self.start.as_ref()
    }

    /// Host: whether at least two players are in the lobby and all are ready.
    pub fn can_start(&self) -> bool {
        self.is_host()
            && self.state == LobbyState::Lobby
            && self.lobby.len() >= 2
            && self.lobby.iter().all(|p| p.ready)
    }

    /// Client: tells the host whether we are ready. The host is always ready.
    pub fn set_ready(&mut self, ready: bool) {
        let (Role::Client { server }, Some(local)) = (&self.role, self.local) else {
            return;
        };
        let server = *server;
        if let Some(entry) = self.lobby.iter_mut().find(|p| p.player == local) {
            entry.ready = ready;
        }
        self.send_control(server, &Control::Ready(ready));
        self.link.flush();
    }

    /// Host: starts the match with all lobby players. Fails unless
    /// [`can_start`](Self::can_start). Late joiners are rejected afterwards.
    pub fn start(
        &mut self,
        seed: u64,
        config: SimConfig,
        input_delay: u32,
        hash_interval: u64,
    ) -> Result<StartInfo, String> {
        if !self.can_start() {
            return Err("not every player is ready".into());
        }
        let info = StartInfo {
            seed,
            config,
            players: self.lobby.iter().map(|p| p.player).collect(),
            input_delay,
            hash_interval,
        };
        self.broadcast_control(&Control::Start(Box::new(info.clone())));
        self.link.flush();
        self.start = Some(info.clone());
        self.state = LobbyState::Started;
        Ok(info)
    }

    /// Processes all pending network events. Never blocks.
    pub fn poll(&mut self) {
        loop {
            let event = match self.link.poll() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(e) => {
                    self.close_state(format!("network error: {e}"));
                    break;
                }
            };
            match event {
                LinkEvent::Connected(peer) => self.on_connect(peer),
                LinkEvent::Disconnected(peer) => self.on_disconnect(peer),
                LinkEvent::Received {
                    peer,
                    channel,
                    data,
                } => {
                    if channel == GAME_CHANNEL {
                        self.on_game_packet(peer, data);
                    } else if channel == CONTROL_CHANNEL
                        && let Some(message) = decode(&data)
                    {
                        self.on_control(peer, message);
                    }
                }
            }
        }
        self.link.flush();
    }

    /// Disconnects from all peers (gracefully, if `poll` runs a bit longer).
    pub fn leave(&mut self) {
        self.link.disconnect_all(false);
        self.link.flush();
        self.close_state("left the game".into());
    }

    fn close_state(&mut self, reason: String) {
        if !matches!(self.state, LobbyState::Closed(_)) {
            self.state = LobbyState::Closed(reason);
        }
    }

    fn on_connect(&mut self, peer: L::Peer) {
        match &mut self.role {
            Role::Host { peers, .. } => {
                // The player id is assigned when the hello arrives.
                peers.entry(peer).or_insert(None);
            }
            Role::Client { server } => {
                if *server == peer {
                    let hello = Control::Hello {
                        protocol: PROTOCOL_VERSION,
                        name: self.name.clone(),
                    };
                    self.send_control(peer, &hello);
                }
            }
        }
    }

    fn on_disconnect(&mut self, peer: L::Peer) {
        match &mut self.role {
            Role::Host { peers, .. } => {
                let Some(Some(player)) = peers.remove(&peer) else {
                    return;
                };
                if self.state == LobbyState::Started {
                    self.broadcast_control(&Control::PeerLeft(player));
                    self.close_state(format!("player {} left the game", player.0));
                } else {
                    self.lobby.retain(|p| p.player != player);
                    self.broadcast_lobby();
                }
            }
            Role::Client { server } => {
                if *server == peer {
                    let reason = if self.state == LobbyState::Connecting {
                        "could not connect to the host"
                    } else {
                        "connection to the host lost"
                    };
                    self.close_state(reason.into());
                }
            }
        }
    }

    fn on_game_packet(&mut self, from: L::Peer, data: Vec<u8>) {
        match &self.role {
            Role::Host { peers, .. } => {
                if peers.get(&from).copied().flatten().is_none() {
                    return; // Not an accepted player.
                }
                // Relay to every other accepted client; order per sender is kept.
                let others: Vec<L::Peer> = peers
                    .iter()
                    .filter(|&(&p, player)| p != from && player.is_some())
                    .map(|(&p, _)| p)
                    .collect();
                for peer in others {
                    self.link.send(peer, GAME_CHANNEL, &data);
                }
            }
            Role::Client { server } => {
                if *server != from {
                    return;
                }
            }
        }
        self.inbox.push_back(data);
    }

    fn on_control(&mut self, from: L::Peer, message: Control) {
        if let Role::Client { server } = &self.role
            && *server != from
        {
            return; // Clients only listen to their host.
        }
        match (&self.role, message) {
            (Role::Host { .. }, Control::Hello { protocol, name }) => {
                self.on_hello(from, protocol, name);
            }
            (Role::Host { peers, .. }, Control::Ready(ready)) => {
                if let Some(Some(player)) = peers.get(&from).copied()
                    && let Some(entry) = self.lobby.iter_mut().find(|p| p.player == player)
                {
                    entry.ready = ready;
                    self.broadcast_lobby();
                }
            }
            (Role::Client { .. }, Control::Welcome { player }) => {
                self.local = Some(player);
                self.state = LobbyState::Lobby;
            }
            (Role::Client { .. }, Control::Reject { reason }) => {
                self.close_state(format!("rejected by the host: {reason}"));
            }
            (Role::Client { .. }, Control::Lobby { players }) => self.lobby = players,
            (Role::Client { .. }, Control::Start(info)) => {
                if self
                    .local
                    .is_some_and(|local| info.players.contains(&local))
                {
                    self.start = Some(*info);
                    self.state = LobbyState::Started;
                } else {
                    self.close_state("the match started without us".into());
                }
            }
            (Role::Client { .. }, Control::PeerLeft(player)) => {
                self.close_state(format!("player {} left the game", player.0));
            }
            // Anything else is a protocol violation by the sender; ignore it.
            _ => {}
        }
    }

    fn on_hello(&mut self, from: L::Peer, protocol: u16, name: String) {
        let Role::Host { peers, max_players } = &mut self.role else {
            return;
        };
        // A link without connect events (Steam) may deliver the hello first.
        let entry = peers.entry(from).or_insert(None);
        if entry.is_some() {
            return; // Second hello.
        }
        let reject = if protocol != PROTOCOL_VERSION {
            Some(format!(
                "protocol version {protocol}, the host has {PROTOCOL_VERSION}"
            ))
        } else if self.state != LobbyState::Lobby {
            Some("the match already started".into())
        } else if self.lobby.len() >= *max_players {
            Some("the game is full".into())
        } else {
            None
        };
        if let Some(reason) = reject {
            peers.remove(&from);
            self.send_control(from, &Control::Reject { reason });
            self.link.disconnect(from, false);
            return;
        }
        // Smallest free id; the host is 0.
        let player = (1..=u8::MAX)
            .map(PlayerId)
            .find(|id| self.lobby.iter().all(|p| p.player != *id))
            .expect("max_players is below 256");
        peers.insert(from, Some(player));
        self.lobby.push(LobbyPlayer {
            player,
            name,
            ready: false,
        });
        self.lobby.sort_by_key(|p| p.player);
        self.send_control(from, &Control::Welcome { player });
        self.broadcast_lobby();
    }

    fn broadcast_lobby(&mut self) {
        let players = self.lobby.clone();
        self.broadcast_control(&Control::Lobby { players });
    }

    /// Host: sends to every accepted client.
    fn broadcast_control(&mut self, message: &Control) {
        for peer in self.game_peers() {
            self.send_control(peer, message);
        }
    }

    fn send_control(&mut self, peer: L::Peer, message: &Control) {
        self.link.send(peer, CONTROL_CHANNEL, &encode(message));
    }

    /// Host: accepted clients. Client: the host.
    fn game_peers(&self) -> Vec<L::Peer> {
        match &self.role {
            Role::Host { peers, .. } => peers
                .iter()
                .filter(|(_, player)| player.is_some())
                .map(|(&p, _)| p)
                .collect(),
            Role::Client { server } => vec![*server],
        }
    }
}

impl<L: Link> Transport for LobbyNode<L> {
    fn send(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if let LobbyState::Closed(_) = self.state {
            return Err(TransportError::Disconnected);
        }
        for peer in self.game_peers() {
            // A failed send means the peer is gone; `recv` reports it.
            self.link.send(peer, GAME_CHANNEL, packet);
        }
        self.link.flush();
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        if self.inbox.is_empty() {
            self.poll();
        }
        match self.inbox.pop_front() {
            Some(packet) => Ok(Some(packet)),
            // Everything received before the loss is delivered first.
            None if matches!(self.state, LobbyState::Closed(_)) => {
                Err(TransportError::Disconnected)
            }
            None => Ok(None),
        }
    }

    fn round_trip_ms(&self) -> Option<(u32, u32)> {
        self.game_peers()
            .into_iter()
            .filter_map(|peer| self.link.round_trip_ms(peer))
            .max_by_key(|&(rtt, variance)| rtt.saturating_add(variance))
    }

    fn close(&mut self) {
        // Unlike `leave`, nothing waits for acknowledgements: the process
        // may be about to exit.
        self.link.disconnect_all(true);
        self.link.flush();
        self.close_state("left the game".into());
    }
}

impl<L: Link> std::fmt::Debug for LobbyNode<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LobbyNode")
            .field("host", &self.is_host())
            .field("state", &self.state)
            .field("local", &self.local)
            .field("lobby", &self.lobby)
            .finish_non_exhaustive()
    }
}
