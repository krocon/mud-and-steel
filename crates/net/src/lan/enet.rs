//! ENet (`rusty_enet`) as the [`Link`] of LAN games.

use crate::lobby::{CHANNEL_COUNT, Link, LinkEvent, LobbyNode, PROTOCOL_VERSION};
use rusty_enet::{EventNoRef, Host, HostSettings, Packet, PeerID};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

/// ENet disconnects a peer that has not answered for this long (ms).
const TIMEOUT_MAX_MS: u32 = 10_000;
const TIMEOUT_MIN_MS: u32 = 3_000;
/// Pings measure the round trip time; ENet's default is 500 ms.
const PING_INTERVAL_MS: u32 = 250;

/// One peer of a LAN game: the lobby over ENet. See [`LobbyNode`].
pub type LanNode = LobbyNode<EnetLink>;

/// An ENet host bound to a UDP socket.
pub struct EnetLink {
    host: Host<UdpSocket>,
}

impl LobbyNode<EnetLink> {
    /// Hosts a game on `port` (0 = any free port) for up to `max_players`
    /// players including the host, who is player 0.
    pub fn host(name: &str, port: u16, max_players: usize) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
        // A few spare slots so that peers beyond the limit can be told why.
        let host = new_host(socket, max_players.max(1) + 4)?;
        Ok(Self::new_host(EnetLink { host }, name, max_players))
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
        Ok(Self::new_client(EnetLink { host }, server, name))
    }

    /// Local UDP port (the game port for a host).
    pub fn port(&self) -> u16 {
        self.link()
            .host
            .socket()
            .local_addr()
            .map_or(0, |a| a.port())
    }
}

impl Link for EnetLink {
    type Peer = PeerID;

    fn poll(&mut self) -> Result<Option<LinkEvent<PeerID>>, String> {
        let event = match self.host.service() {
            Ok(Some(event)) => event.no_ref(),
            Ok(None) => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        Ok(Some(match event {
            EventNoRef::Connect { peer, .. } => {
                configure_peer(self.host.peer_mut(peer));
                LinkEvent::Connected(peer)
            }
            EventNoRef::Disconnect { peer, .. } => LinkEvent::Disconnected(peer),
            EventNoRef::Receive {
                peer,
                channel_id,
                packet,
            } => LinkEvent::Received {
                peer,
                channel: channel_id,
                data: packet.data().to_vec(),
            },
        }))
    }

    fn send(&mut self, peer: PeerID, channel: u8, data: &[u8]) {
        // A failed send means the peer is gone; its disconnect event follows.
        let _ = self
            .host
            .peer_mut(peer)
            .send(channel, &Packet::reliable(data));
    }

    fn flush(&mut self) {
        self.host.flush();
    }

    fn disconnect(&mut self, peer: PeerID, now: bool) {
        let peer = self.host.peer_mut(peer);
        if now {
            peer.disconnect_now(0);
        } else {
            peer.disconnect_later(0);
        }
    }

    fn disconnect_all(&mut self, now: bool) {
        for peer in self.host.connected_peers_mut() {
            if now {
                peer.disconnect_now(0);
            } else {
                peer.disconnect_later(0);
            }
        }
    }

    fn round_trip_ms(&self, peer: PeerID) -> Option<(u32, u32)> {
        let millis = |d: std::time::Duration| u32::try_from(d.as_millis()).unwrap_or(u32::MAX);
        self.host
            .get_peer(peer)
            .filter(|peer| peer.connected())
            .map(|peer| {
                (
                    millis(peer.round_trip_time()),
                    millis(peer.round_trip_time_variance()),
                )
            })
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
