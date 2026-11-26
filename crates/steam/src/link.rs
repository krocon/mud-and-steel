//! Steam Networking Messages as the [`Link`] under the lobby protocol.

use crate::Steam;
use net::lobby::{CHANNEL_COUNT, Link, LinkEvent, LobbyNode};
use std::collections::{BTreeSet, VecDeque};
use steamworks::networking_types::{NetworkingIdentity, SendFlags};
use steamworks::{LobbyId, SteamId};

/// Messages taken from Steam per channel and receive call.
const BATCH: usize = 64;
/// Upper bound on receive batches per channel per [`SteamLink::pump`] call
/// so a flooding peer cannot starve the frame loop (prompt 212).
const MAX_BATCHES_PER_POLL: usize = 16;

/// Drains batches from `recv(BATCH)` while batches remain full, capped at
/// [`MAX_BATCHES_PER_POLL`] batches so bursts are not delayed by extra frames
/// while floods remain bounded.
fn drain_batches<M>(mut recv: impl FnMut(usize) -> Vec<M>, mut on_message: impl FnMut(M)) {
    for _ in 0..MAX_BATCHES_PER_POLL {
        let batch = recv(BATCH);
        let count = batch.len();
        for message in batch {
            on_message(message);
        }
        if count < BATCH {
            break;
        }
    }
}

/// Filters `left` for `lobby` and combines with `failed` peer ids.
fn collect_gone(lobby: u64, left: &mut Vec<(u64, u64)>, failed: &mut Vec<u64>) -> Vec<u64> {
    let mut gone: Vec<u64> = left
        .drain(..)
        .filter(|&(l, _)| l == lobby)
        .map(|(_, user)| user)
        .collect();
    gone.append(failed);
    gone
}

/// Turns a Steam session ping (ms) into `(rtt_ms, jitter_ms)` for lockstep.
fn rtt_from_ping(ping: i32) -> Option<(u32, u32)> {
    let ping = u32::try_from(ping).ok()?;
    // Steam reports no variance; a quarter of the ping covers jitter.
    Some((ping, ping / 4))
}

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
            drain_batches(
                |limit| messages.receive_messages_on_channel(u32::from(channel), limit),
                |message| {
                    let Some(peer) = message.identity_peer().steam_id().map(|id| id.raw()) else {
                        return;
                    };
                    if self.connected.insert(peer) {
                        self.events.push_back(LinkEvent::Connected(peer));
                    }
                    self.events.push_back(LinkEvent::Received {
                        peer,
                        channel,
                        data: message.data().to_vec(),
                    });
                },
            );
        }
        let gone: Vec<u64> = {
            let shared = &mut *self.steam.shared();
            collect_gone(self.lobby, &mut shared.left, &mut shared.failed)
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
        rtt_from_ping(status?.ping())
    }
}

impl Drop for SteamLink {
    fn drop(&mut self) {
        self.leave_lobby();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_batches_reads_burst_and_stops_on_short_batch() {
        let mut remaining = BATCH * 2 + 13;
        let mut calls = 0usize;
        let mut received = Vec::new();
        drain_batches(
            |limit| {
                calls += 1;
                let take = remaining.min(limit);
                remaining -= take;
                (0..take).collect()
            },
            |m| received.push(m),
        );
        assert_eq!(calls, 3);
        assert_eq!(received.len(), BATCH * 2 + 13);
    }

    #[test]
    fn drain_batches_caps_flood_at_max_batches() {
        let mut calls = 0usize;
        let mut count = 0usize;
        drain_batches(
            |limit| {
                calls += 1;
                vec![1u8; limit]
            },
            |_| count += 1,
        );
        assert_eq!(calls, MAX_BATCHES_PER_POLL);
        assert_eq!(count, MAX_BATCHES_PER_POLL * BATCH);
    }

    #[test]
    fn collect_gone_filters_by_lobby_and_drains_failed() {
        let mut left = vec![(10, 100), (20, 200), (10, 101)];
        let mut failed = vec![300];
        let gone = collect_gone(10, &mut left, &mut failed);
        assert_eq!(gone, vec![100, 101, 300]);
        assert!(left.is_empty());
        assert!(failed.is_empty());
    }

    #[test]
    fn rtt_from_ping_rejects_negative_and_adds_quarter_jitter() {
        assert_eq!(rtt_from_ping(-1), None);
        assert_eq!(rtt_from_ping(0), Some((0, 0)));
        assert_eq!(rtt_from_ping(80), Some((80, 20)));
        assert_eq!(rtt_from_ping(43), Some((43, 10)));
    }
}
