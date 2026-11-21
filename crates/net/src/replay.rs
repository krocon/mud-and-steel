//! Replays: everything needed to re-simulate a match bit for bit.
//!
//! # File format
//!
//! ```text
//! offset  size  content
//! 0       4     magic "MSRP" (Mud and Steel RePlay)
//! 4       2     format version, u16 little endian (FORMAT_VERSION)
//! 6       ..    postcard encoding of `Replay`, starting with `ReplayInfo`
//! ```
//!
//! The header is plain bytes so that a reader can reject unknown versions
//! before it tries to decode the body (postcard is not self-describing).
//! [`ReplayInfo`] comes first in the body, so a replay list reads it
//! without decoding the commands ([`Replay::read_info`]). It carries the
//! game version and the rules hash ([`crate::snapshot::rules_hash`]):
//! [`Replay::decode`] rejects replays of a build with other simulation rules
//! instead of playing them back wrongly (prompt 038).
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
/// [`Replay`] or of the types it contains (commands, config). 8 since
/// prompt 040 (`SimConfig::weather`); 9 since prompt 039
/// (`SimConfig::mission`, `Command::MissionSignal`); 10 since prompt 093
/// (`Command::AttackObject`, new combat rules and machine guns in
/// `SimConfig`); 11 since prompt 108 (`arrival` and `ride` of the mission
/// action `spawn`); 12 since prompt 176 (`FireMode::ReturnFire`,
/// `Command::SetFireLimits`, `[discipline]` tactics rules); 13 since
/// prompt 181 (mission `level`, `resources_pct`, trigger `open`, condition
/// `fewer`, action `hopeless`; prompt 180 appended the conditions
/// `stance` … `air`, which old files of version 13 never contain); 14
/// since prompt 182 (`SimConfig::variant`; prompt 189 added mission
/// `rating`, objective `optional`/`timer`/`progress` and the condition
/// `reach` in the same release).
pub const FORMAT_VERSION: u16 = 14;

/// Version of the game that writes replays and saved games.
pub const GAME_VERSION: &str = env!("CARGO_PKG_VERSION");

const HEADER_LEN: usize = MAGIC.len() + 2;

/// The commands of one tick, in the order the simulation applied them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayFrame {
    pub tick: u64,
    pub commands: Vec<PlayerCommand>,
}

/// A player of a recorded match, for replay lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: PlayerId,
    /// Faction id (`de`, `su`, `us`), empty if unknown.
    pub faction: String,
    /// Who played: `human`, `ai:<difficulty>`, or empty if not set.
    pub kind: String,
}

/// What a replay list shows, and the compatibility check (prompt 038).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayInfo {
    /// [`GAME_VERSION`] of the build that wrote it.
    pub game_version: String,
    /// [`crate::snapshot::rules_hash`] of that build.
    pub rules_hash: u64,
    /// Local date and time of writing, set by the caller (the simulation
    /// knows no clock); may be empty.
    pub created: String,
    pub map: String,
    pub tick_hz: u32,
    /// Number of simulated ticks.
    pub ticks: u64,
    pub players: Vec<PlayerInfo>,
    /// Whether the match was decided, and the winner (`None`: draw).
    pub decided: bool,
    pub winner: Option<PlayerId>,
    /// Played over the network (LAN, online).
    pub multiplayer: bool,
}

/// A recorded match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replay {
    /// First, so [`Replay::read_info`] can read it alone.
    pub info: ReplayInfo,
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
    /// Written by a build with other simulation rules (prompt 038).
    RulesChanged {
        game_version: String,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a Mud and Steel replay"),
            Self::UnsupportedVersion(v) if *v < FORMAT_VERSION => write!(
                f,
                "replay format version {v} is from an older game version and cannot be played (expected {FORMAT_VERSION})"
            ),
            Self::UnsupportedVersion(v) => write!(
                f,
                "replay format version {v} is from a newer game version (expected {FORMAT_VERSION})"
            ),
            Self::RulesChanged { game_version } => write!(
                f,
                "replay was recorded with other game rules (version {game_version}); this build ({GAME_VERSION}) cannot play it back"
            ),
            Self::Decode(e) => write!(f, "malformed replay: {e}"),
            Self::Config(e) => write!(f, "invalid replay config: {e}"),
            Self::BadFrameOrder { tick } => write!(f, "replay frame for tick {tick} out of order"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// The body after a checked header.
fn body(bytes: &[u8]) -> Result<&[u8], ReplayError> {
    if bytes.len() < HEADER_LEN || bytes[..MAGIC.len()] != MAGIC {
        return Err(ReplayError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FORMAT_VERSION {
        return Err(ReplayError::UnsupportedVersion(version));
    }
    Ok(&bytes[HEADER_LEN..])
}

impl ReplayInfo {
    /// Whether this build can play the match back (same rules hash).
    pub fn check_rules(&self) -> Result<(), ReplayError> {
        if self.rules_hash == crate::snapshot::rules_hash() {
            Ok(())
        } else {
            Err(ReplayError::RulesChanged {
                game_version: self.game_version.clone(),
            })
        }
    }

    /// Info for a match of `config` that has run to `sim`.
    pub fn of(
        sim: &Sim,
        config: &SimConfig,
        players: &[PlayerId],
        kinds: &[(PlayerId, String)],
    ) -> Self {
        let outcome = sim.outcome();
        Self {
            game_version: GAME_VERSION.into(),
            rules_hash: crate::snapshot::rules_hash(),
            created: String::new(),
            map: config.map.name.clone(),
            tick_hz: config.tick_hz,
            ticks: sim.tick(),
            players: config
                .players()
                .into_iter()
                .map(|id| PlayerInfo {
                    id,
                    faction: sim.faction(id).unwrap_or_default().into(),
                    kind: kinds
                        .iter()
                        .find(|(p, _)| *p == id)
                        .map(|(_, k)| k.clone())
                        .unwrap_or_default(),
                })
                .collect(),
            decided: outcome.is_some(),
            winner: outcome.and_then(|o| o.winner),
            multiplayer: kinds
                .iter()
                .filter(|(p, k)| k == "human" && players.contains(p))
                .count()
                > 1,
        }
    }
}

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

    /// Decodes a replay file and checks that this build can play it back
    /// (same rules hash).
    pub fn decode(bytes: &[u8]) -> Result<Self, ReplayError> {
        let replay: Self = postcard::from_bytes(body(bytes)?).map_err(ReplayError::Decode)?;
        replay.info.check_rules()?;
        Ok(replay)
    }

    /// Only the info at the start of a replay file (for lists), without the
    /// rules check: a list also shows replays it cannot play.
    pub fn read_info(bytes: &[u8]) -> Result<ReplayInfo, ReplayError> {
        postcard::take_from_bytes(body(bytes)?)
            .map(|(info, _)| info)
            .map_err(ReplayError::Decode)
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
    /// Who plays which player, for [`PlayerInfo::kind`].
    kinds: Vec<(PlayerId, String)>,
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
            kinds: Vec::new(),
        }
    }

    /// Continues a recorded match (a loaded game, prompt 038): the next
    /// recorded tick is `replay.ticks`.
    pub fn resume(replay: &Replay) -> Self {
        Self {
            seed: replay.seed,
            config: replay.config.clone(),
            players: replay.players.clone(),
            ticks: replay.ticks,
            frames: replay.frames.clone(),
            kinds: replay
                .info
                .players
                .iter()
                .map(|p| (p.id, p.kind.clone()))
                .collect(),
        }
    }

    /// Records who plays `player` (`human`, `ai:<difficulty>`).
    pub fn set_kind(&mut self, player: PlayerId, kind: &str) {
        self.kinds.retain(|(p, _)| *p != player);
        self.kinds.push((player, kind.into()));
        self.kinds.sort();
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
            info: ReplayInfo::of(sim, &self.config, &self.players, &self.kinds),
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

    #[test]
    fn info_is_readable_alone() {
        let mut replay = demo_replay(200);
        replay.info.created = "2026-11-15 22:00".into();
        let info = Replay::read_info(&replay.encode()).unwrap();
        assert_eq!(info, replay.info);
        assert_eq!(info.map, "test_small");
        assert_eq!(info.ticks, 200);
        assert_eq!(info.game_version, GAME_VERSION);
        assert_eq!(info.players.len(), 2);
    }

    #[test]
    fn old_and_foreign_replays_are_rejected() {
        let bytes = demo_replay(10).encode();
        // A replay of the format before prompt 038.
        let mut old = bytes.clone();
        old[4..6].copy_from_slice(&5u16.to_le_bytes());
        let error = Replay::decode(&old).unwrap_err();
        assert!(matches!(error, ReplayError::UnsupportedVersion(5)));
        assert!(error.to_string().contains("older game version"), "{error}");
        assert!(Replay::read_info(&old).is_err());
        // Other simulation rules: readable for a list, but not playable.
        let mut other = demo_replay(10);
        other.info.rules_hash ^= 1;
        other.info.game_version = "0.0.9".into();
        let bytes = other.encode();
        assert_eq!(Replay::read_info(&bytes).unwrap().game_version, "0.0.9");
        let error = Replay::decode(&bytes).unwrap_err();
        assert!(matches!(error, ReplayError::RulesChanged { .. }));
        assert!(error.to_string().contains("other game rules"), "{error}");
    }
}
