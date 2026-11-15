//! The byte link under a [`LobbyNode`](super::LobbyNode): ENet on the LAN,
//! Steam Networking Messages online, an in-process hub in tests.

use std::fmt;

/// Something that happened on a [`Link`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkEvent<P> {
    /// `peer` can now be reached. A client's link reports its server once;
    /// a host's link reports every peer that contacts it, at the latest
    /// before that peer's first packet.
    Connected(P),
    /// `peer` is gone (left, timed out, refused).
    Disconnected(P),
    /// A packet from `peer` on `channel`.
    Received { peer: P, channel: u8, data: Vec<u8> },
}

/// Moves packets between peers. Implementations must deliver reliably and
/// in order per peer and channel, with at least
/// [`CHANNEL_COUNT`](super::CHANNEL_COUNT) channels.
pub trait Link {
    /// Identifies a remote peer (ENet peer id, Steam id, …).
    type Peer: Copy + Ord + fmt::Debug;

    /// The next pending event, or `None`. Never blocks. An error is fatal.
    fn poll(&mut self) -> Result<Option<LinkEvent<Self::Peer>>, String>;

    /// Queues `data` for `peer`. A failure means the peer is gone; its
    /// [`LinkEvent::Disconnected`] follows.
    fn send(&mut self, peer: Self::Peer, channel: u8, data: &[u8]);

    /// Sends what is queued, if the link buffers.
    fn flush(&mut self) {}

    /// Ends the connection to `peer`. `now`: do not wait for queued packets.
    fn disconnect(&mut self, peer: Self::Peer, now: bool);

    /// Ends all connections.
    fn disconnect_all(&mut self, now: bool);

    /// Round trip time to `peer` and its variance in milliseconds, if known.
    fn round_trip_ms(&self, peer: Self::Peer) -> Option<(u32, u32)>;
}
