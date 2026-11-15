//! Saved single-player games (prompt 038).
//!
//! # File format
//!
//! ```text
//! offset  size  content
//! 0       4     magic "MSSV" (Mud and Steel SaVe)
//! 4       2     format version, u16 little endian (FORMAT_VERSION)
//! 6       ..    postcard encoding of `SaveGame`, starting with `ReplayInfo`
//! ```
//!
//! A saved game is the match so far as a [`Replay`] plus a snapshot of the
//! simulation at the save tick ([`crate::snapshot`]), the commands already
//! given but not yet simulated (input delay) and the state of the computer
//! players. Loading restores the snapshot (no re-simulation) and checks its
//! state hash; the replay keeps growing, so the replay written at the end
//! covers the whole match from tick 0. [`SaveGame::verify`] re-simulates the
//! replay and compares it with the snapshot (tests, `cli save-check`).

use crate::replay::{Replay, ReplayError, ReplayInfo};
use crate::session::PendingFrames;
use crate::snapshot::{self, SnapshotError};
use serde::{Deserialize, Serialize};
use sim::{Map, PlayerId, Sim};
use std::fmt;
use std::sync::Arc;

pub const MAGIC: [u8; 4] = *b"MSSV";

/// Version of the body layout.
pub const FORMAT_VERSION: u16 = 1;

const HEADER_LEN: usize = MAGIC.len() + 2;

/// The saved state of a computer player hosted in the session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedState {
    pub player: PlayerId,
    /// Encoded by the `ai` crate (`ai::Ai::save_state`).
    pub state: Vec<u8>,
}

/// A saved game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveGame {
    /// First, so [`SaveGame::read_info`] can read it alone. `ticks` is the
    /// save tick.
    pub info: ReplayInfo,
    /// The match from tick 0 to the save tick.
    pub replay: Replay,
    /// [`crate::snapshot::encode`] of the simulation at the save tick.
    pub snapshot: Vec<u8>,
    /// The lockstep session's frames not simulated yet.
    pub pending: PendingFrames,
    pub input_delay: u32,
    pub hosted: Vec<HostedState>,
    /// Settings of the presentation (biome, game mode, …), as key and value.
    pub settings: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum SaveError {
    BadMagic,
    UnsupportedVersion(u16),
    Decode(postcard::Error),
    Replay(ReplayError),
    Snapshot(SnapshotError),
    /// Snapshot and replay disagree (tick or state).
    Inconsistent(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a Mud and Steel saved game"),
            Self::UnsupportedVersion(v) if *v < FORMAT_VERSION => write!(
                f,
                "saved game format version {v} is from an older game version (expected {FORMAT_VERSION})"
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "saved game format version {v} is from a newer game version (expected {FORMAT_VERSION})"
            ),
            Self::Decode(e) => write!(f, "malformed saved game: {e}"),
            Self::Replay(ReplayError::RulesChanged { game_version }) => write!(
                f,
                "saved game is from a game version with other rules ({game_version}); it cannot be loaded"
            ),
            Self::Replay(e) => write!(f, "{e}"),
            Self::Snapshot(e) => write!(f, "{e}"),
            Self::Inconsistent(e) => write!(f, "saved game is inconsistent: {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

fn body(bytes: &[u8]) -> Result<&[u8], SaveError> {
    if bytes.len() < HEADER_LEN || bytes[..MAGIC.len()] != MAGIC {
        return Err(SaveError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FORMAT_VERSION {
        return Err(SaveError::UnsupportedVersion(version));
    }
    Ok(&bytes[HEADER_LEN..])
}

impl SaveGame {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.snapshot.len());
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        postcard::to_extend(self, bytes).expect("serializing into a Vec cannot fail")
    }

    /// Decodes a saved game and checks that this build can continue it
    /// (same rules hash).
    pub fn decode(bytes: &[u8]) -> Result<Self, SaveError> {
        let save: Self = postcard::from_bytes(body(bytes)?).map_err(SaveError::Decode)?;
        save.info.check_rules().map_err(SaveError::Replay)?;
        Ok(save)
    }

    /// Only the info at the start (for the load list), without the rules
    /// check.
    pub fn read_info(bytes: &[u8]) -> Result<ReplayInfo, SaveError> {
        postcard::take_from_bytes(body(bytes)?)
            .map(|(info, _)| info)
            .map_err(SaveError::Decode)
    }

    /// The simulation at the save tick, on `loaded` (the map as loaded).
    pub fn restore(&self, loaded: Arc<Map>) -> Result<Sim, SaveError> {
        let sim = snapshot::decode(&self.snapshot, loaded).map_err(SaveError::Snapshot)?;
        if sim.tick() != self.replay.ticks || sim.state_hash() != self.replay.end_hash {
            return Err(SaveError::Inconsistent(format!(
                "snapshot at tick {}, replay up to tick {}",
                sim.tick(),
                self.replay.ticks
            )));
        }
        Ok(sim)
    }

    /// Re-simulates the replay from tick 0 and checks that it ends in the
    /// snapshot's state.
    pub fn verify(&self, loaded: Arc<Map>) -> Result<(), SaveError> {
        let restored = self.restore(loaded.clone())?;
        let played = self.replay.play(loaded).map_err(SaveError::Replay)?;
        if played.state_hash() == restored.state_hash() {
            Ok(())
        } else {
            Err(SaveError::Inconsistent(format!(
                "re-simulation ends in {:#018x}, snapshot has {:#018x}",
                played.state_hash(),
                restored.state_hash()
            )))
        }
    }

    /// A setting saved with the game, if present.
    pub fn setting(&self, key: &str) -> Option<&str> {
        self.settings
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}
