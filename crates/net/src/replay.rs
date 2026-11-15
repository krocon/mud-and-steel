//! Replays: everything needed to re-simulate a match bit for bit.
//!
//! # File format
//!
//! ```text
//! offset  size  content
//! 0       4     magic "MSRP" (Mud and Steel RePlay)
//! 4       2     format version, u16 little endian (FORMAT_VERSION)
//! 6       ..    postcard encoding of `Replay`
//! ```
//!
//! The header is plain bytes so that a reader can reject unknown versions
//! before it tries to decode the body (postcard is not self-describing).
//!
//! The body stores the fully resolved [`SimConfig`] (unit types already
//! converted from the catalog), so a replay stays valid when the catalog or
//! the scenario files change later. It does not survive changes of the
//! simulation rules: then the end hash no longer matches, which playback
//! reports.
//!
//! The map is not stored, only referenced by name and content hash
//! (`SimConfig::map`): playback needs the same map, which the caller loads
//! from `data/maps/<name>/`.

use serde::{Deserialize, Serialize};
use sim::{ConfigError, Map, PlayerCommand, PlayerId, Sim, SimConfig};
use std::fmt;
use std::sync::Arc;

pub const MAGIC: [u8; 4] = *b"MSRP";

/// Version of the body layout. Bump on every incompatible change of
/// [`Replay`] or of the types it contains (commands, config).
pub const FORMAT_VERSION: u16 = 3;

const HEADER_LEN: usize = MAGIC.len() + 2;

/// The commands of one tick, in the order the simulation applied them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub tick: u64,
    pub commands: Vec<PlayerCommand>,
}

/// A recorded match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    pub seed: u64,
    pub config: SimConfig,
    /// Players of the lockstep session, ascending.
    pub players: Vec<PlayerId>,
    /// Number of simulated ticks.
    pub ticks: u64,
    /// Only ticks with at least one command, strictly ascending; every other
    /// tick below `ticks` had no commands.
    pub frames: Vec<ReplayFrame>,
    /// `Sim::state_hash` after `ticks` ticks.
    pub end_hash: u64,
}

#[derive(Debug)]
pub enum ReplayError {
    /// Not a replay file.
    BadMagic,
    /// Written by a different format version.
    UnsupportedVersion(u16),
    Decode(postcard::Error),
    Config(ConfigError),
    /// Frames are not strictly ascending or lie beyond the end.
    BadFrameOrder {
        tick: u64,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a Mud and Steel replay"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "replay format version {v} is not supported (expected {FORMAT_VERSION})"
            ),
            Self::Decode(e) => write!(f, "malformed replay: {e}"),
            Self::Config(e) => write!(f, "invalid replay config: {e}"),
            Self::BadFrameOrder { tick } => write!(f, "replay frame for tick {tick} out of order"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// Outcome of re-simulating a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackReport {
    pub ticks: u64,
    pub expected_hash: u64,
    pub actual_hash: u64,
}

impl PlaybackReport {
    pub const fn matches(&self) -> bool {
        self.expected_hash == self.actual_hash
    }
}

impl Replay {
    /// Header plus postcard body.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN);
        bytes.extend_from_slice(&MAGIC);
        bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        postcard::to_extend(self, bytes).expect("serializing into a Vec cannot fail")
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ReplayError> {
        if bytes.len() < HEADER_LEN || bytes[..MAGIC.len()] != MAGIC {
            return Err(ReplayError::BadMagic);
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != FORMAT_VERSION {
            return Err(ReplayError::UnsupportedVersion(version));
        }
        postcard::from_bytes(&bytes[HEADER_LEN..]).map_err(ReplayError::Decode)
    }

    /// Re-simulates the match on `map` (the map `config.map` names) and
    /// returns the final simulation.
    pub fn play(&self, map: Arc<Map>) -> Result<Sim, ReplayError> {
        let mut sim = Sim::new(&self.config, map, self.seed).map_err(ReplayError::Config)?;
        let mut frames = self.frames.iter().peekable();
        for tick in 0..self.ticks {
            match frames.next_if(|f| f.tick == tick) {
                Some(frame) => sim.step(&frame.commands),
                None => sim.step(&[]),
            }
            if let Some(frame) = frames.peek()
                && frame.tick <= tick
            {
                return Err(ReplayError::BadFrameOrder { tick: frame.tick });
            }
        }
        if let Some(frame) = frames.next() {
            return Err(ReplayError::BadFrameOrder { tick: frame.tick });
        }
        Ok(sim)
    }

    /// Re-simulates the match and compares the end hash.
    pub fn verify(&self, map: Arc<Map>) -> Result<PlaybackReport, ReplayError> {
        let sim = self.play(map)?;
        Ok(PlaybackReport {
            ticks: sim.tick(),
            expected_hash: self.end_hash,
            actual_hash: sim.state_hash(),
        })
    }
}

/// Collects the commands of every simulated tick.
///
/// Call [`record`](Self::record) with exactly the commands passed to
/// `Sim::step`, for every tick in order, then [`finish`](Self::finish).
#[derive(Clone, Debug)]
pub struct ReplayRecorder {
    seed: u64,
    config: SimConfig,
    players: Vec<PlayerId>,
    ticks: u64,
    frames: Vec<ReplayFrame>,
}

impl ReplayRecorder {
    pub fn new(seed: u64, config: SimConfig, players: &[PlayerId]) -> Self {
        let mut players = players.to_vec();
        players.sort_unstable();
        players.dedup();
        Self {
            seed,
            config,
            players,
            ticks: 0,
            frames: Vec::new(),
        }
    }

    /// Number of recorded ticks.
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Records the commands of the next tick.
    pub fn record(&mut self, commands: &[PlayerCommand]) {
        if !commands.is_empty() {
            self.frames.push(ReplayFrame {
                tick: self.ticks,
                commands: commands.to_vec(),
            });
        }
        self.ticks += 1;
    }

    /// Builds the replay; `sim` must be the simulation the recorded commands
    /// were applied to.
    ///
    /// # Panics
    /// Panics if `sim` simulated a different number of ticks than recorded.
    pub fn finish(&self, sim: &Sim) -> Replay {
        assert_eq!(sim.tick(), self.ticks, "recorder and simulation diverged");
        Replay {
            seed: self.seed,
            config: self.config.clone(),
            players: self.players.clone(),
            ticks: self.ticks,
            frames: self.frames.clone(),
            end_hash: sim.state_hash(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::scenario::{builtin_map, demo_config, run_demo_with};

    fn demo_replay(ticks: u64) -> Replay {
        let config = demo_config();
        let mut recorder = ReplayRecorder::new(42, config.clone(), &config.players());
        let sim = run_demo_with(42, ticks, |_, commands| recorder.record(commands));
        recorder.finish(&sim)
    }

    #[test]
    fn header_is_checked() {
        let bytes = demo_replay(10).encode();
        assert_eq!(&bytes[..4], b"MSRP");
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), FORMAT_VERSION);

        assert!(matches!(Replay::decode(b"MS"), Err(ReplayError::BadMagic)));
        let mut wrong_magic = bytes.clone();
        wrong_magic[0] = b'X';
        assert!(matches!(
            Replay::decode(&wrong_magic),
            Err(ReplayError::BadMagic)
        ));
        let mut future = bytes;
        future[4..6].copy_from_slice(&(FORMAT_VERSION + 1).to_le_bytes());
        assert!(matches!(
            Replay::decode(&future),
            Err(ReplayError::UnsupportedVersion(v)) if v == FORMAT_VERSION + 1
        ));
    }

    #[test]
    fn only_ticks_with_commands_are_stored() {
        let replay = demo_replay(200);
        let ticks: Vec<u64> = replay.frames.iter().map(|f| f.tick).collect();
        assert_eq!(ticks, vec![40, 80, 120, 160]);
        assert_eq!(replay.ticks, 200);
    }

    #[test]
    fn frames_out_of_order_are_rejected() {
        let mut replay = demo_replay(200);
        replay.frames.swap(0, 1);
        assert!(matches!(
            replay.play(builtin_map()),
            Err(ReplayError::BadFrameOrder { .. })
        ));
        let mut beyond = demo_replay(200);
        beyond.ticks = 100;
        assert!(matches!(
            beyond.play(builtin_map()),
            Err(ReplayError::BadFrameOrder { tick: 120 })
        ));
    }
}
