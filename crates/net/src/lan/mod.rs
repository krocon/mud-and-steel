//! LAN multiplayer: ENet over UDP plus game discovery by UDP broadcast.
//!
//! Everything ENet-specific lives in this module. The lobby protocol and the
//! match transport are the network-independent [`LobbyNode`] in
//! [`crate::lobby`]; here it runs over an [`EnetLink`] as [`LanNode`].
//!
//! - [`LanNode`] is one peer: the host (ENet server, always player 0) or a
//!   client. Clients only talk to the host, which relays the lockstep
//!   packets of every client to all other clients (star).
//! - [`DiscoveryResponder`] answers search queries while a host is in the
//!   lobby; [`DiscoveryBrowser`] sends them and collects the answers.
//!
//! See `docs/LAN.md` for the protocol, ports and how to test with two machines.
//!
//! [`LobbyNode`]: crate::lobby::LobbyNode

mod discovery;
mod enet;

pub use crate::lobby::{LobbyPlayer, LobbyState as LanState, PROTOCOL_VERSION, StartInfo};
pub use discovery::{DiscoveryBrowser, DiscoveryResponder, FoundGame, GameInfo};
pub use enet::{EnetLink, LanNode};

/// Default UDP port of a hosted game (ENet).
pub const DEFAULT_GAME_PORT: u16 = 47810;

/// Default UDP port on which hosts answer search queries.
pub const DEFAULT_DISCOVERY_PORT: u16 = 47811;
