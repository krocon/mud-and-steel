//! In-process [`Link`]s for tests: a hub that connects any number of
//! endpoints, addressed by `u64` like Steam ids.

use super::link::{Link, LinkEvent};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Default)]
struct Hub {
    inboxes: BTreeMap<u64, VecDeque<LinkEvent<u64>>>,
    /// Open connections, stored in both directions.
    connections: BTreeSet<(u64, u64)>,
    round_trip_ms: Option<(u32, u32)>,
}

/// Creates connected [`MemoryLink`]s. Cheap to clone.
#[derive(Clone, Default)]
pub struct MemoryNetwork {
    hub: Arc<Mutex<Hub>>,
}

impl MemoryNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new endpoint with address `id`.
    pub fn endpoint(&self, id: u64) -> MemoryLink {
        self.lock().inboxes.entry(id).or_default();
        MemoryLink {
            id,
            network: self.clone(),
        }
    }

    /// The round trip every link reports for its connected peers.
    pub fn set_round_trip_ms(&self, rtt: Option<(u32, u32)>) {
        self.lock().round_trip_ms = rtt;
    }

    fn lock(&self) -> MutexGuard<'_, Hub> {
        // A panicking test thread must not hide the original panic.
        self.hub
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// One endpoint of a [`MemoryNetwork`]. Like Steam Networking Messages it is
/// connectionless: the first packet to a peer opens the connection, and the
/// receiver sees [`LinkEvent::Connected`] before it. Dropping a link
/// disconnects it from everybody.
pub struct MemoryLink {
    id: u64,
    network: MemoryNetwork,
}

impl MemoryLink {
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Opens the connection to `peer` and reports it as connected (a
    /// client's first step).
    pub fn connect(&mut self, peer: u64) {
        let mut hub = self.network.lock();
        if hub.inboxes.contains_key(&peer) {
            hub.connections.insert((self.id, peer));
            hub.connections.insert((peer, self.id));
            push(&mut hub, self.id, LinkEvent::Connected(peer));
        } else {
            push(&mut hub, self.id, LinkEvent::Disconnected(peer));
        }
    }
}

fn push(hub: &mut Hub, to: u64, event: LinkEvent<u64>) {
    if let Some(inbox) = hub.inboxes.get_mut(&to) {
        inbox.push_back(event);
    }
}

impl Link for MemoryLink {
    type Peer = u64;

    fn poll(&mut self) -> Result<Option<LinkEvent<u64>>, String> {
        let mut hub = self.network.lock();
        Ok(hub.inboxes.get_mut(&self.id).and_then(VecDeque::pop_front))
    }

    fn send(&mut self, peer: u64, channel: u8, data: &[u8]) {
        let mut hub = self.network.lock();
        if !hub.inboxes.contains_key(&peer) {
            return;
        }
        if hub.connections.insert((peer, self.id)) {
            hub.connections.insert((self.id, peer));
            push(&mut hub, peer, LinkEvent::Connected(self.id));
        }
        let event = LinkEvent::Received {
            peer: self.id,
            channel,
            data: data.to_vec(),
        };
        push(&mut hub, peer, event);
    }

    fn disconnect(&mut self, peer: u64, _now: bool) {
        let mut hub = self.network.lock();
        if hub.connections.remove(&(self.id, peer)) {
            hub.connections.remove(&(peer, self.id));
            push(&mut hub, peer, LinkEvent::Disconnected(self.id));
        }
    }

    fn disconnect_all(&mut self, now: bool) {
        let peers: Vec<u64> = {
            let hub = self.network.lock();
            hub.connections
                .iter()
                .filter(|(from, _)| *from == self.id)
                .map(|&(_, to)| to)
                .collect()
        };
        for peer in peers {
            self.disconnect(peer, now);
        }
    }

    fn round_trip_ms(&self, peer: u64) -> Option<(u32, u32)> {
        let hub = self.network.lock();
        hub.connections
            .contains(&(self.id, peer))
            .then_some(hub.round_trip_ms)
            .flatten()
    }
}

impl Drop for MemoryLink {
    fn drop(&mut self) {
        self.disconnect_all(true);
        self.network.lock().inboxes.remove(&self.id);
    }
}
