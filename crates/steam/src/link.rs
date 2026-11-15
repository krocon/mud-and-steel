//! Steam Networking Messages as the [`Link`] under the lobby protocol.

use crate::Steam;
use net::lobby::{CHANNEL_COUNT, Link, LinkEvent, LobbyNode};
use std::collections::{BTreeSet, VecDeque};
use steamworks::networking_types::{NetworkingIdentity, SendFlags};
use steamworks::{LobbyId, SteamId};

/// Messages taken from Steam per channel and poll.
const BATCH: usize = 64;

/// One peer of an online game: the lobby protocol over Steam.
pub type SteamNode = LobbyNode<SteamLink>;

/// Sends to and receives from the members of one Steam lobby, addressed by
/// Steam id. Reliable and ordered per peer and channel; Steam relays and
/// punches through NAT. Leaving (disconnecting all, or dropping the link)
/// leaves the Steam lobby, which the other members see at once.
pub struct SteamLink {
    steam: Steam,
    lobby: u64,
    /// Peers we exchanged messages with and that did not leave.
    connected: BTreeSet<u64>,
    events: VecDeque<LinkEvent<u64>>,
    left: bool,
}

impl SteamLink {
    /// The host's link in its own `lobby`.
    pub fn host(steam: Steam, lobby: u64) -> Self {
        Self::new(steam, lobby)
    }

    /// A client's link to `server` (the lobby owner) in `lobby`.
    pub fn client(steam: Steam, lobby: u64, server: u64) -> Self {
        let mut link = Self::new(steam, lobby);
        // Networking Messages are connectionless: the hello opens the session.
        link.connected.insert(server);
        link.events.push_back(LinkEvent::Connected(server));
        link
    }

    fn new(steam: Steam, lobby: u64) -> Self {
        steam.shared().current_lobby = Some(lobby);
        Self {
            steam,
            lobby,
            connected: BTreeSet::new(),
            events: VecDeque::new(),
            left: false,
        }
    }

    pub const fn lobby(&self) -> u64 {
        self.lobby
    }

    /// Takes everything Steam has for us into `events`.
    fn pump(&mut self) {
        if self.left {
            return;
        }
        self.steam.run_callbacks();
        let messages = self.steam.client().networking_messages();
        // Messages first: a peer's last packets come before its loss.
        for channel in 0..CHANNEL_COUNT {
            let channel = u8::try_from(channel).expect("few channels");
            for message in messages.receive_messages_on_channel(u32::from(channel), BATCH) {
                let Some(peer) = message.identity_peer().steam_id().map(|id| id.raw()) else {
                    continue;
                };
                if self.connected.insert(peer) {
                    self.events.push_back(LinkEvent::Connected(peer));
                }
                self.events.push_back(LinkEvent::Received {
                    peer,
                    channel,
                    data: message.data().to_vec(),
                });
            }
        }
        let gone: Vec<u64> = {
            let mut shared = self.steam.shared();
            let lobby = self.lobby;
            let mut gone: Vec<u64> = shared
                .left
                .drain(..)
                .filter(|&(l, _)| l == lobby)
                .map(|(_, user)| user)
                .collect();
            gone.append(&mut shared.failed);
            gone
        };
        for peer in gone {
            if self.connected.remove(&peer) {
                self.events.push_back(LinkEvent::Disconnected(peer));
            }
        }
    }

    fn leave_lobby(&mut self) {
        if self.left {
            return;
        }
        self.left = true;
        let client = self.steam.client();
        client
            .matchmaking()
            .leave_lobby(LobbyId::from_raw(self.lobby));
        client.friends().clear_rich_presence();
        let mut shared = self.steam.shared();
        if shared.current_lobby == Some(self.lobby) {
            shared.current_lobby = None;
        }
    }
}

impl Link for SteamLink {
    type Peer = u64;

    fn poll(&mut self) -> Result<Option<LinkEvent<u64>>, String> {
        if self.events.is_empty() {
            self.pump();
        }
        Ok(self.events.pop_front())
    }

    fn send(&mut self, peer: u64, channel: u8, data: &[u8]) {
        if self.left || !self.connected.contains(&peer) {
            return;
        }
        let identity = NetworkingIdentity::new_steam_id(SteamId::from_raw(peer));
        // No Nagle delay: a lockstep frame should leave at once.
        let sent = self
            .steam
            .client()
            .networking_messages()
            .send_message_to_user(
                identity,
                SendFlags::RELIABLE_NO_NAGLE,
                data,
                u32::from(channel),
            );
        if sent.is_err() {
            self.steam.shared().failed.push(peer);
        }
    }

    fn disconnect(&mut self, peer: u64, _now: bool) {
        // Steam has no way to close a peer's session from here; forgetting
        // the peer ignores its further messages. A rejected client leaves
        // the lobby by itself.
        self.connected.remove(&peer);
    }

    fn disconnect_all(&mut self, _now: bool) {
        self.connected.clear();
        self.leave_lobby();
    }

    fn round_trip_ms(&self, peer: u64) -> Option<(u32, u32)> {
        if !self.connected.contains(&peer) {
            return None;
        }
        let identity = NetworkingIdentity::new_steam_id(SteamId::from_raw(peer));
        let (_, _, status) = self
            .steam
            .client()
            .networking_messages()
            .get_session_connection_info(&identity);
        let ping = u32::try_from(status?.ping()).ok()?;
        // Steam reports no variance; a quarter of the ping covers jitter.
        Some((ping, ping / 4))
    }
}

impl Drop for SteamLink {
    fn drop(&mut self) {
        self.leave_lobby();
    }
}
