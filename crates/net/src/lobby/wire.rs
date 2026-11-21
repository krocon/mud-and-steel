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

/// What a lobby slot holds (prompt 037). Slot `i` is player `i`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    /// Free for a human to join.
    Open,
    /// The human player with this id (see [`LobbyPlayer`]).
    Human,
    /// A computer player of this difficulty (`ai::Difficulty` name), hosted
    /// by the host.
    Computer(String),
    /// Nobody plays here.
    Closed,
}

/// One slot of the lobby.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbySlot {
    pub kind: SlotKind,
    pub team: u8,
    /// Faction id (`"de"`, `"su"`, `"us"`); empty: the scenario's units.
    pub faction: String,
}

/// The match the host prepares (prompt 037): slots, map, mode. Sent to
/// every client whenever it changes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbySetup {
    /// Slot `i` is player `i`; slot 0 is the host.
    pub slots: Vec<LobbySlot>,
    /// Map folder name (`data/maps/<map>/`).
    pub map: String,
    /// `"battle"` or `"base"`.
    pub mode: String,
    /// Allies see what the other players of their team see.
    pub shared_vision: bool,
    /// Start layout of the map (prompt 182): "" or `standard`, `random`
    /// (picked from the match seed) or a `[[variant]]` name.
    #[serde(default)]
    pub variant: String,
}

impl LobbySetup {
    /// `slots` slots: the host in slot 0, the others open, teams
    /// alternating (0, 1, 0, …), the battle on the built-in map.
    pub fn new(slots: usize) -> Self {
        Self {
            slots: (0..slots.max(1))
                .map(|i| LobbySlot {
                    kind: if i == 0 {
                        SlotKind::Human
                    } else {
                        SlotKind::Open
                    },
                    team: u8::try_from(i % 2).unwrap_or(0),
                    faction: String::new(),
                })
                .collect(),
            map: String::new(),
            mode: "battle".into(),
            shared_vision: true,
            variant: String::new(),
        }
    }

    fn player(index: usize) -> PlayerId {
        PlayerId(u8::try_from(index).unwrap_or(u8::MAX))
    }

    /// Players taking part (humans and computers), ascending.
    pub fn participants(&self) -> Vec<PlayerId> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s.kind, SlotKind::Human | SlotKind::Computer(_)))
            .map(|(i, _)| Self::player(i))
            .collect()
    }

    /// Computer players with their difficulty, ascending.
    pub fn computers(&self) -> Vec<(PlayerId, String)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| match &s.kind {
                SlotKind::Computer(level) => Some((Self::player(i), level.clone())),
                _ => None,
            })
            .collect()
    }

    /// Number of teams among the participants.
    pub fn team_count(&self) -> usize {
        let mut teams: Vec<u8> = self
            .participants()
            .iter()
            .map(|p| self.slots[usize::from(p.0)].team)
            .collect();
        teams.sort_unstable();
        teams.dedup();
        teams.len()
    }
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
    /// Players of the lockstep session (humans and computers), ascending.
    pub players: Vec<PlayerId>,
    /// The computer players among them with their difficulty; the host
    /// hosts them (prompt 037).
    pub computers: Vec<(PlayerId, String)>,
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
    /// Host → all: slots, map and mode (prompt 037).
    Setup(LobbySetup),
    /// Client → host: team and faction of the client's own slot.
    Choose { team: u8, faction: String },
}

pub fn encode(message: &Control) -> Vec<u8> {
    postcard::to_allocvec(message).expect("serializing into a Vec cannot fail")
}

pub fn decode(bytes: &[u8]) -> Option<Control> {
    postcard::from_bytes(bytes).ok()
}
