//! Byte transport abstraction. Network games use a
//! [`LobbyNode`](crate::lobby::LobbyNode) over ENet ([`crate::lan`]) or Steam
//! (`crates/steam`).

use std::fmt;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Disconnected,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "peer disconnected"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Sends and receives opaque packets. Knows nothing about the protocol.
///
/// Implementations must deliver packets reliably and in order per peer.
pub trait Transport {
    /// Sends a packet to all other peers.
    fn send(&mut self, packet: &[u8]) -> Result<(), TransportError>;

    /// Returns the next received packet, or `None` if nothing is pending.
    /// Never blocks.
    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError>;

    /// Measured round trip time to the slowest peer and its variance, in
    /// milliseconds, or `None` if the transport does not measure it.
    fn round_trip_ms(&self) -> Option<(u32, u32)> {
        None
    }

    /// Leaves the match: tells the other peers right away instead of letting
    /// them run into a timeout. Called when a match is dropped.
    fn close(&mut self) {}
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn send(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        (**self).send(packet)
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        (**self).recv()
    }

    fn round_trip_ms(&self) -> Option<(u32, u32)> {
        (**self).round_trip_ms()
    }

    fn close(&mut self) {
        (**self).close();
    }
}

/// In-process transport between exactly two peers, for tests.
pub struct LoopbackTransport {
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl LoopbackTransport {
    /// Creates two connected endpoints.
    pub fn pair() -> (Self, Self) {
        let (a_tx, b_rx) = channel();
        let (b_tx, a_rx) = channel();
        (Self { tx: a_tx, rx: a_rx }, Self { tx: b_tx, rx: b_rx })
    }
}

impl Transport for LoopbackTransport {
    fn send(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        self.tx
            .send(packet.to_vec())
            .map_err(|_| TransportError::Disconnected)
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        match self.rx.try_recv() {
            Ok(packet) => Ok(Some(packet)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

/// Transport of a match without remote peers (single player). Sends go
/// nowhere and nothing is ever received.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalTransport;

impl Transport for LocalTransport {
    fn send(&mut self, _packet: &[u8]) -> Result<(), TransportError> {
        Ok(())
    }

    fn recv(&mut self) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivers_in_order() {
        let (mut a, mut b) = LoopbackTransport::pair();
        a.send(b"one").unwrap();
        a.send(b"two").unwrap();
        assert_eq!(b.recv().unwrap().as_deref(), Some(&b"one"[..]));
        assert_eq!(b.recv().unwrap().as_deref(), Some(&b"two"[..]));
        assert_eq!(b.recv().unwrap(), None);
        assert_eq!(a.recv().unwrap(), None);
    }
}
