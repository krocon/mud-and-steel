//! One LAN peer: lobby first, then the match transport.

use super::PROTOCOL_VERSION;
use super::wire::{
    CHANNEL_COUNT, CONTROL_CHANNEL, Control, GAME_CHANNEL, LobbyPlayer, StartInfo, decode, encode,
};
use crate::{PlayerId, Transport, TransportError};
use rusty_enet::{EventNoRef, Host, HostSettings, Packet, PeerID};
use sim::SimConfig;
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

/// ENet disconnects a peer that has not answered for this long (ms).
const TIMEOUT_MAX_MS: u32 = 10_000;
const TIMEOUT_MIN_MS: u32 = 3_000;
/// Pings measure the round trip time; ENet's default is 500 ms.
const PING_INTERVAL_MS: u32 = 250;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LanState {
    /// Client: connecting to the host, not yet accepted.
    Connecting,
    /// In the lobby, waiting for the start.
    Lobby,
    /// The match runs; the node is now a `Transport`.
    Started,
    /// Connection lost, rejected or a player left. Final.
    Closed(String),
}

enum Role {
    Host {
        /// Connected peers (ENet ids) and their player once they said hello.
        peers: BTreeMap<PeerID, Option<PlayerId>>,
        max_players: usize,
    },
    Client {
        server: PeerID,
    },
}

/// One peer of a LAN game. See the module documentation.
///
/// Call [`poll`](Self::poll) regularly (every frame) in the lobby. During the
/// match, [`Transport::recv`] polls.
pub struct LanNode {
    host: Host<UdpSocket>,
    role: Role,
    name: String,
    local: Option<PlayerId>,
    lobby: Vec<LobbyPlayer>,
    start: Option<StartInfo>,
    state: LanState,
    /// Received lockstep packets, in arrival order.
    inbox: VecDeque<Vec<u8>>,
}

impl LanNode {
    /// Hosts a game on `port` (0 = any free port) for up to `max_players`
    /// players including the host, who is player 0.
    pub fn host(name: &str, port: u16, max_players: usize) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
        // A few spare slots so that peers beyond the limit can be told why.
        let host = new_host(socket, max_players.max(1) + 4)?;
        let local = PlayerId(0);
        Ok(Self {
            host,
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
            state: LanState::Lobby,
            inbox: VecDeque::new(),
        })
    }

    /// Connects to the game hosted at `address`.
    pub fn join(name: &str, address: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        let mut host = new_host(socket, 1)?;
        let peer = host
            .connect(address, CHANNEL_COUNT, u32::from(PROTOCOL_VERSION))
            .map_err(|_| io::Error::other("no free ENet peer slot"))?;
        configure_peer(peer);
        let server = peer.id();
        Ok(Self {
            host,
            role: Role::Client { server },
            name: name.to_owned(),
            local: None,
            lobby: Vec::new(),
            start: None,
            state: LanState::Connecting,
            inbox: VecDeque::new(),
        })
    }

    pub const fn is_host(&self) -> bool {
        matches!(self.role, Role::Host { .. })
    }

    /// Local UDP port (the game port for a host).
    pub fn port(&self) -> u16 {
        self.host.socket().local_addr().map_or(0, |a| a.port())
    }

    pub fn state(&self) -> &LanState {
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

    pub fn max_players(&self) -> Option<usize> {
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
            && self.state == LanState::Lobby
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
        self.host.flush();
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
        self.host.flush();
        self.start = Some(info.clone());
        self.state = LanState::Started;
        Ok(info)
    }

    /// Processes all pending network events. Never blocks.
    pub fn poll(&mut self) {
        loop {
            let event = match self.host.service() {
                Ok(Some(event)) => event.no_ref(),
                Ok(None) => break,
                Err(e) => {
                    self.close(format!("network error: {e}"));
                    break;
                }
            };
            match event {
                EventNoRef::Connect { peer, .. } => self.on_connect(peer),
                EventNoRef::Disconnect { peer, .. } => self.on_disconnect(peer),
                EventNoRef::Receive {
                    peer,
                    channel_id,
                    packet,
                } => {
                    if channel_id == GAME_CHANNEL {
                        self.on_game_packet(peer, packet);
                    } else if let Some(message) = decode(packet.data()) {
                        self.on_control(peer, message);
                    }
                }
            }
        }
        self.host.flush();
    }

    /// Disconnects from all peers (gracefully, if `poll` runs a bit longer).
    pub fn leave(&mut self) {
        for peer in self.host.connected_peers_mut() {
            peer.disconnect_later(0);
        }
        self.host.flush();
        if !matches!(self.state, LanState::Closed(_)) {
            self.state = LanState::Closed("left the game".into());
        }
    }

    fn close(&mut self, reason: String) {
        if !matches!(self.state, LanState::Closed(_)) {
            self.state = LanState::Closed(reason);
        }
    }

    fn on_connect(&mut self, peer: PeerID) {
        configure_peer(self.host.peer_mut(peer));
        match &mut self.role {
            Role::Host { peers, .. } => {
                // The player id is assigned when the hello arrives.
                peers.insert(peer, None);
            }
            Role::Client { .. } => {
                let hello = Control::Hello {
                    protocol: PROTOCOL_VERSION,
                    name: self.name.clone(),
                };
                self.send_control(peer, &hello);
            }
        }
    }

    fn on_disconnect(&mut self, peer: PeerID) {
        match &mut self.role {
            Role::Host { peers, .. } => {
                let Some(Some(player)) = peers.remove(&peer) else {
                    return;
                };
                if self.state == LanState::Started {
                    self.broadcast_control(&Control::PeerLeft(player));
                    self.close(format!("player {} left the game", player.0));
                } else {
                    self.lobby.retain(|p| p.player != player);
                    self.broadcast_lobby();
                }
            }
            Role::Client { server } => {
                if *server == peer {
                    let reason = if self.state == LanState::Connecting {
                        "could not connect to the host"
                    } else {
                        "connection to the host lost"
                    };
                    self.close(reason.into());
                }
            }
        }
    }

    fn on_game_packet(&mut self, from: PeerID, packet: Packet) {
        if let Role::Host { peers, .. } = &self.role {
            // Relay to every other accepted client; order per sender is kept.
            let others: Vec<PeerID> = peers
                .iter()
                .filter(|&(&p, player)| p != from && player.is_some())
                .map(|(&p, _)| p)
                .collect();
            for peer in others {
                // A failed send means the peer is gone; its disconnect event follows.
                let _ = self.host.peer_mut(peer).send(GAME_CHANNEL, &packet);
            }
        }
        self.inbox.push_back(packet.data().to_vec());
    }

    fn on_control(&mut self, from: PeerID, message: Control) {
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
                self.state = LanState::Lobby;
            }
            (Role::Client { .. }, Control::Reject { reason }) => {
                self.close(format!("rejected by the host: {reason}"));
            }
            (Role::Client { .. }, Control::Lobby { players }) => self.lobby = players,
            (Role::Client { .. }, Control::Start(info)) => {
                if self
                    .local
                    .is_some_and(|local| info.players.contains(&local))
                {
                    self.start = Some(*info);
                    self.state = LanState::Started;
                } else {
                    self.close("the match started without us".into());
                }
            }
            (Role::Client { .. }, Control::PeerLeft(player)) => {
                self.close(format!("player {} left the game", player.0));
            }
            // Anything else is a protocol violation by the sender; ignore it.
            _ => {}
        }
    }

    fn on_hello(&mut self, from: PeerID, protocol: u16, name: String) {
        let Role::Host { peers, max_players } = &mut self.role else {
            return;
        };
        if peers.get(&from) != Some(&None) {
            return; // Unknown peer or second hello.
        }
        let reject = if protocol != PROTOCOL_VERSION {
            Some(format!(
                "protocol version {protocol}, the host has {PROTOCOL_VERSION}"
            ))
        } else if self.state != LanState::Lobby {
            Some("the match already started".into())
        } else if self.lobby.len() >= *max_players {
            Some("the game is full".into())
        } else {
            None
        };
        if let Some(reason) = reject {
            self.send_control(from, &Control::Reject { reason });
            self.host.peer_mut(from).disconnect_later(0);
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
        let Role::Host { peers, .. } = &self.role else {
            return;
        };
        let accepted: Vec<PeerID> = peers
            .iter()
            .filter(|(_, player)| player.is_some())
            .map(|(&p, _)| p)
            .collect();
        for peer in accepted {
            self.send_control(peer, message);
        }
    }

    fn send_control(&mut self, peer: PeerID, message: &Control) {
        let packet = Packet::reliable(encode(message));
        // A failed send means the peer is gone; its disconnect event follows.
        let _ = self.host.peer_mut(peer).send(CONTROL_CHANNEL, &packet);
    }

    fn game_peers(&self) -> Vec<PeerID> {
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

impl Transport for LanNode {
    fn send(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if let LanState::Closed(_) = self.state {
            return Err(TransportError::Disconnected);
        }
        let packet = Packet::reliable(packet);
        for peer in self.game_peers() {
            // A failed send means the peer is gone; `recv` reports it.
            let _ = self.host.peer_mut(peer).send(GAME_CHANNEL, &packet);
        }
        self.host.flush();
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        if self.inbox.is_empty() {
            self.poll();
        }
        match self.inbox.pop_front() {
            Some(packet) => Ok(Some(packet)),
            // Everything received before the loss is delivered first.
            None if matches!(self.state, LanState::Closed(_)) => Err(TransportError::Disconnected),
            None => Ok(None),
        }
    }

    fn round_trip_ms(&self) -> Option<(u32, u32)> {
        let millis = |d: std::time::Duration| u32::try_from(d.as_millis()).unwrap_or(u32::MAX);
        self.game_peers()
            .into_iter()
            .filter_map(|id| self.host.get_peer(id))
            .filter(|peer| peer.connected())
            .map(|peer| {
                (
                    millis(peer.round_trip_time()),
                    millis(peer.round_trip_time_variance()),
                )
            })
            .max_by_key(|&(rtt, variance)| rtt.saturating_add(variance))
    }
}

impl std::fmt::Debug for LanNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanNode")
            .field("host", &self.is_host())
            .field("port", &self.port())
            .field("state", &self.state)
            .field("local", &self.local)
            .field("lobby", &self.lobby)
            .finish_non_exhaustive()
    }
}

fn new_host(socket: UdpSocket, peer_limit: usize) -> io::Result<Host<UdpSocket>> {
    Host::new(
        socket,
        HostSettings {
            peer_limit,
            channel_limit: CHANNEL_COUNT,
            ..HostSettings::default()
        },
    )
    .map_err(|e| io::Error::other(format!("cannot create ENet host: {e:?}")))
}

fn configure_peer(peer: &mut rusty_enet::Peer<UdpSocket>) {
    peer.set_timeout(0, TIMEOUT_MIN_MS, TIMEOUT_MAX_MS);
    peer.set_ping_interval(PING_INTERVAL_MS);
}
