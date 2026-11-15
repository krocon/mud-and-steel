//! The multiplayer lobby, independent of the network: host, join, ready,
//! start, then the match transport. LAN ([`crate::lan`], ENet) and Steam
//! (`crates/steam`) only provide the [`Link`] underneath.
//!
//! - [`LobbyNode`] is one peer: the host (always player 0) or a client. It
//!   runs the lobby protocol and then implements
//!   [`Transport`](crate::Transport) for the match. Clients only talk to the
//!   host, which relays the lockstep packets of every client to all other
//!   clients (star).
//! - [`MemoryNetwork`] connects [`MemoryLink`]s in one process, for tests.
//!
//! Protocol and timeouts: `docs/LAN.md`; Steam specifics: `docs/ONLINE.md`.

mod link;
mod memory;
mod node;
mod wire;

pub use link::{Link, LinkEvent};
pub use memory::{MemoryLink, MemoryNetwork};
pub use node::{LobbyNode, LobbyState};
pub use wire::{CHANNEL_COUNT, LobbyPlayer, StartInfo};

/// Version of the multiplayer protocol (lobby messages, LAN discovery and
/// lockstep messages). Peers with a different version are rejected when
/// joining.
pub const PROTOCOL_VERSION: u16 = 2;
