//! LAN multiplayer: ENet over UDP plus game discovery by UDP broadcast.
//!
//! Everything ENet-specific lives in this module, so a different transport
//! (C ENet, Godot ENet, Steam) only replaces it; [`LockstepSession`] and the
//! lobby protocol stay.
//!
//! - [`LanNode`] is one peer: the host (ENet server, always player 0) or a
//!   client. It runs the lobby (join, ready, start) and then implements
//!   [`Transport`] for the match. Clients only talk to the host, which relays
//!   the lockstep packets of every client to all other clients (star).
//! - [`DiscoveryResponder`] answers search queries while a host is in the
//!   lobby; [`DiscoveryBrowser`] sends them and collects the answers.
//!
//! See `docs/LAN.md` for the protocol, ports and how to test with two machines.
//!
//! [`LockstepSession`]: crate::LockstepSession
//! [`Transport`]: crate::Transport

mod discovery;
mod node;
mod wire;

pub use discovery::{DiscoveryBrowser, DiscoveryResponder, FoundGame, GameInfo};
pub use node::{LanNode, LanState};
pub use wire::{LobbyPlayer, StartInfo};

/// Version of the LAN protocol (lobby messages, discovery and lockstep
/// messages). Peers with a different version are rejected when joining.
pub const PROTOCOL_VERSION: u16 = 2;

/// Default UDP port of a hosted game (ENet).
pub const DEFAULT_GAME_PORT: u16 = 47810;

/// Default UDP port on which hosts answer search queries.
pub const DEFAULT_DISCOVERY_PORT: u16 = 47811;
