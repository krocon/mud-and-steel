//! Lobby messages (link channel 0).

use crate::{LockstepConfig, PlayerId};
use serde::{Deserialize, Serialize};
use sim::SimConfig;

/// Link channel of the lobby protocol.
pub const CONTROL_CHANNEL: u8 = 0;
/// Link channel of the opaque lockstep packets (`Transport`).
pub const GAME_CHANNEL: u8 = 1;
/// Number of channels a [`Link`](super::Link) must provide.
pub const CHANNEL_COUNT: usize = 2;

/// A player in the lobby.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyPlayer {
    pub player: PlayerId,
    pub name: String,
    pub ready: bool,
}

/// Everything a peer needs to start the match, sent by the host.
///
/// The config is the host's fully resolved one (like in a replay), so peers
/// with a different catalog cannot diverge. The map is only referenced by
/// name and content hash; every peer loads it locally and `Sim::new` rejects
/// a different one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartInfo {
    pub seed: u64,
    pub config: SimConfig,
    /// Human players of the lockstep session, ascending.
    pub players: Vec<PlayerId>,
    /// Initial input delay, identical on every peer.
    pub input_delay: u32,
    pub hash_interval: u64,
}

impl StartInfo {
    pub fn lockstep_config(&self) -> LockstepConfig {
        LockstepConfig {
            players: self.players.clone(),
            input_delay: self.input_delay,
            hash_interval: self.hash_interval,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Control {
    /// Client → host, first message after connecting.
    Hello { protocol: u16, name: String },
    /// Host → client: accepted as `player`.
    Welcome { player: PlayerId },
    /// Host → client: not accepted; the host disconnects afterwards.
    Reject { reason: String },
    /// Host → all: the current lobby.
    Lobby { players: Vec<LobbyPlayer> },
    /// Client → host.
    Ready(bool),
    /// Host → all: the match starts.
    Start(Box<StartInfo>),
    /// Host → all: a player left the running match.
    PeerLeft(PlayerId),
}

pub fn encode(message: &Control) -> Vec<u8> {
    postcard::to_allocvec(message).expect("serializing into a Vec cannot fail")
}

pub fn decode(bytes: &[u8]) -> Option<Control> {
    postcard::from_bytes(bytes).ok()
}
