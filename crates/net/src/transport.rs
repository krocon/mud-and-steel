//! Byte transport abstraction. Real transports (Steam, ENet) come later.

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
