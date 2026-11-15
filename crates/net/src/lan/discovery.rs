//! Game discovery: a query by UDP broadcast, answers by unicast.
//!
//! ```text
//! query:   "MSLAN?" + protocol version (u16 LE)
//! answer:  "MSLAN!" + postcard(GameInfo)
//! ```
//!
//! Clients ask instead of hosts announcing, so several searching clients can
//! run on one machine (only the host binds the fixed discovery port).

use super::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

const QUERY_MAGIC: &[u8; 6] = b"MSLAN?";
const ANSWER_MAGIC: &[u8; 6] = b"MSLAN!";

/// What a host tells searching clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameInfo {
    pub protocol: u16,
    /// Name of the hosting player.
    pub name: String,
    /// ENet port of the game; the address is the one the answer came from.
    pub game_port: u16,
    pub players: u8,
    pub max_players: u8,
}

/// A game found by [`DiscoveryBrowser`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoundGame {
    /// Address to join (answering host, game port).
    pub address: SocketAddr,
    pub info: GameInfo,
}

/// Host side: answers search queries on the discovery port.
#[derive(Debug)]
pub struct DiscoveryResponder {
    socket: UdpSocket,
    info: GameInfo,
}

impl DiscoveryResponder {
    /// Listens on `port` (all interfaces). Fails if the port is taken, e.g. by
    /// another host on the same machine; the game can still be joined by
    /// address then.
    pub fn new(port: u16, info: GameInfo) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, info })
    }

    /// The bound port (useful when created with port 0).
    pub fn port(&self) -> u16 {
        self.socket.local_addr().map_or(0, |a| a.port())
    }

    pub fn set_info(&mut self, info: GameInfo) {
        self.info = info;
    }

    /// Answers all pending queries. Never blocks.
    pub fn poll(&mut self) {
        let mut buffer = [0u8; 64];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, from)) => {
                    let query = &buffer[..len];
                    if query.len() == QUERY_MAGIC.len() + 2 && query.starts_with(QUERY_MAGIC) {
                        let mut answer = ANSWER_MAGIC.to_vec();
                        answer = postcard::to_extend(&self.info, answer)
                            .expect("serializing into a Vec cannot fail");
                        // Best effort: the client asks again.
                        let _ = self.socket.send_to(&answer, from);
                    }
                }
                // Windows reports ICMP "port unreachable" of earlier sends here.
                Err(e) if e.kind() == ErrorKind::ConnectionReset => {}
                Err(_) => break,
            }
        }
    }
}

/// Client side: sends search queries and collects the answers.
#[derive(Debug)]
pub struct DiscoveryBrowser {
    socket: UdpSocket,
    port: u16,
    games: Vec<FoundGame>,
}

impl DiscoveryBrowser {
    /// Searches for hosts listening on `discovery_port`.
    pub fn new(discovery_port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_nonblocking(true)?;
        socket.set_broadcast(true)?;
        Ok(Self {
            socket,
            port: discovery_port,
            games: Vec::new(),
        })
    }

    /// Sends a query to the limited broadcast address, to the broadcast
    /// address of every IPv4 interface (needed on Windows with several
    /// adapters) and to localhost. Errors of single targets are ignored.
    pub fn query(&mut self) {
        let mut query = QUERY_MAGIC.to_vec();
        query.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        for target in broadcast_targets() {
            let _ = self
                .socket
                .send_to(&query, SocketAddr::new(target, self.port));
        }
    }

    /// Collects pending answers. Never blocks.
    pub fn poll(&mut self) {
        let mut buffer = [0u8; 1024];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, from)) => {
                    let Some(body) = buffer[..len].strip_prefix(ANSWER_MAGIC) else {
                        continue;
                    };
                    let Ok(info) = postcard::from_bytes::<GameInfo>(body) else {
                        continue;
                    };
                    let address = SocketAddr::new(from.ip(), info.game_port);
                    match self.games.iter_mut().find(|g| g.address == address) {
                        Some(game) => game.info = info,
                        None => self.games.push(FoundGame { address, info }),
                    }
                    self.games.sort_by_key(|g| g.address);
                }
                Err(e) if e.kind() == ErrorKind::ConnectionReset => {}
                Err(_) => break,
            }
        }
    }

    /// Games that answered since the last [`clear`](Self::clear), by address.
    pub fn games(&self) -> &[FoundGame] {
        &self.games
    }

    pub fn clear(&mut self) {
        self.games.clear();
    }
}

fn broadcast_targets() -> Vec<IpAddr> {
    let mut targets = vec![IpAddr::V4(Ipv4Addr::BROADCAST)];
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for interface in interfaces {
            if let if_addrs::IfAddr::V4(v4) = interface.addr
                && let Some(broadcast) = v4.broadcast
                && !v4.ip.is_loopback()
            {
                targets.push(IpAddr::V4(broadcast));
            }
        }
    }
    targets.push(IpAddr::V4(Ipv4Addr::LOCALHOST));
    targets.sort_unstable();
    targets.dedup();
    targets
}
