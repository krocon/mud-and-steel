//! Lockstep session: collects command frames and decides when a tick may run.

use crate::protocol::{CommandFrame, HashReport, Message, PlayerId, decode, encode};
use crate::transport::{Transport, TransportError};
use sim::Command;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockstepConfig {
    /// All players of the match, including the local one.
    pub players: Vec<PlayerId>,
    /// Local commands are scheduled this many ticks in the future.
    /// At 20 Hz, a delay of 2 ticks hides up to 100 ms of latency.
    pub input_delay: u32,
    /// State hashes are exchanged every this many ticks. Must be positive.
    pub hash_interval: u64,
}

/// Two peers computed different states for the same tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Desync {
    pub tick: u64,
    pub player: PlayerId,
    pub local_hash: u64,
    pub remote_hash: u64,
}

#[derive(Debug)]
pub enum NetError {
    Transport(TransportError),
    Decode(postcard::Error),
    /// A message came from a player that is not part of the match.
    UnknownPlayer(PlayerId),
    /// Local input was already submitted while simulating `tick`.
    InputAlreadySubmitted {
        tick: u64,
    },
    /// A remote frame for the same tick arrived twice.
    DuplicateFrame {
        tick: u64,
        player: PlayerId,
    },
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "transport error: {e}"),
            Self::Decode(e) => write!(f, "malformed message: {e}"),
            Self::UnknownPlayer(p) => write!(f, "unknown player {}", p.0),
            Self::InputAlreadySubmitted { tick } => {
                write!(f, "local input for tick {tick} already submitted")
            }
            Self::DuplicateFrame { tick, player } => {
                write!(
                    f,
                    "duplicate frame for tick {tick} from player {}",
                    player.0
                )
            }
        }
    }
}

impl std::error::Error for NetError {}

impl From<TransportError> for NetError {
    fn from(e: TransportError) -> Self {
        Self::Transport(e)
    }
}

/// Lockstep state of one peer.
///
/// Per tick, a peer calls:
/// 1. [`submit_local`](Self::submit_local) once (possibly with no commands),
/// 2. [`receive`](Self::receive),
/// 3. [`next_commands`](Self::next_commands); if it returns `Some`, step the
///    simulation with them and call [`record_hash`](Self::record_hash).
#[derive(Debug)]
pub struct LockstepSession {
    local: PlayerId,
    /// Sorted and deduplicated.
    players: Vec<PlayerId>,
    input_delay: u64,
    hash_interval: u64,
    /// The next tick to simulate.
    next_tick: u64,
    /// Local frames exist for all ticks below this value.
    scheduled_until: u64,
    frames: BTreeMap<u64, BTreeMap<PlayerId, Vec<Command>>>,
    local_hashes: BTreeMap<u64, u64>,
    remote_hashes: BTreeMap<u64, BTreeMap<PlayerId, u64>>,
    desync: Option<Desync>,
}

impl LockstepSession {
    /// # Panics
    /// Panics if `local` is not in `config.players` or `hash_interval` is zero.
    pub fn new(local: PlayerId, config: &LockstepConfig) -> Self {
        let mut players = config.players.clone();
        players.sort_unstable();
        players.dedup();
        assert!(players.contains(&local), "local player must take part");
        assert!(config.hash_interval > 0, "hash_interval must be positive");

        let input_delay = u64::from(config.input_delay);
        // Nobody can have sent input for the first ticks, so they are empty.
        let frames = (0..input_delay)
            .map(|tick| (tick, players.iter().map(|&p| (p, Vec::new())).collect()))
            .collect();

        Self {
            local,
            players,
            input_delay,
            hash_interval: config.hash_interval,
            next_tick: 0,
            scheduled_until: input_delay,
            frames,
            local_hashes: BTreeMap::new(),
            remote_hashes: BTreeMap::new(),
            desync: None,
        }
    }

    pub const fn local_player(&self) -> PlayerId {
        self.local
    }

    /// The next tick to simulate (= number of ticks simulated so far).
    pub const fn next_tick(&self) -> u64 {
        self.next_tick
    }

    /// Whether local input for the current tick is still missing.
    pub const fn needs_local_input(&self) -> bool {
        self.scheduled_until <= self.next_tick + self.input_delay
    }

    /// The first detected desync, if any.
    pub const fn desync(&self) -> Option<Desync> {
        self.desync
    }

    /// Schedules local commands for tick `next_tick + input_delay` and sends
    /// them to all peers. Must be called exactly once per simulated tick.
    pub fn submit_local(
        &mut self,
        commands: Vec<Command>,
        transport: &mut impl Transport,
    ) -> Result<u64, NetError> {
        if !self.needs_local_input() {
            return Err(NetError::InputAlreadySubmitted {
                tick: self.next_tick,
            });
        }
        let tick = self.scheduled_until;
        let frame = CommandFrame {
            tick,
            player: self.local,
            commands,
        };
        transport.send(&encode(&Message::Frame(frame.clone())))?;
        self.frames
            .entry(tick)
            .or_default()
            .insert(self.local, frame.commands);
        self.scheduled_until += 1;
        Ok(tick)
    }

    /// Processes all pending messages from the transport.
    pub fn receive(&mut self, transport: &mut impl Transport) -> Result<(), NetError> {
        while let Some(packet) = transport.recv()? {
            match decode(&packet).map_err(NetError::Decode)? {
                Message::Frame(frame) => self.insert_remote_frame(frame)?,
                Message::Hash(report) => self.insert_remote_hash(report)?,
            }
        }
        Ok(())
    }

    fn check_remote(&self, player: PlayerId) -> Result<(), NetError> {
        if player == self.local || self.players.binary_search(&player).is_err() {
            return Err(NetError::UnknownPlayer(player));
        }
        Ok(())
    }

    fn insert_remote_frame(&mut self, frame: CommandFrame) -> Result<(), NetError> {
        self.check_remote(frame.player)?;
        let duplicate = frame.tick < self.next_tick
            || self
                .frames
                .get(&frame.tick)
                .is_some_and(|f| f.contains_key(&frame.player));
        if duplicate {
            return Err(NetError::DuplicateFrame {
                tick: frame.tick,
                player: frame.player,
            });
        }
        self.frames
            .entry(frame.tick)
            .or_default()
            .insert(frame.player, frame.commands);
        Ok(())
    }

    fn insert_remote_hash(&mut self, report: HashReport) -> Result<(), NetError> {
        self.check_remote(report.player)?;
        self.remote_hashes
            .entry(report.tick)
            .or_default()
            .insert(report.player, report.hash);
        self.compare_hashes(report.tick);
        Ok(())
    }

    /// Returns the merged commands for the next tick once all players' frames
    /// are known, and advances to the following tick. Commands are ordered by
    /// player id, then in the order the player issued them.
    pub fn next_commands(&mut self) -> Option<Vec<Command>> {
        let complete = self
            .frames
            .get(&self.next_tick)
            .is_some_and(|f| f.len() == self.players.len());
        if !complete {
            return None;
        }
        let frames = self.frames.remove(&self.next_tick)?;
        self.next_tick += 1;
        // BTreeMap iterates in ascending player order.
        Some(frames.into_values().flatten().collect())
    }

    /// Records the local state hash after `tick` simulated ticks and shares it
    /// on every `hash_interval`-th tick.
    pub fn record_hash(
        &mut self,
        tick: u64,
        hash: u64,
        transport: &mut impl Transport,
    ) -> Result<(), NetError> {
        if !tick.is_multiple_of(self.hash_interval) {
            return Ok(());
        }
        self.local_hashes.insert(tick, hash);
        let report = HashReport {
            tick,
            player: self.local,
            hash,
        };
        transport.send(&encode(&Message::Hash(report)))?;
        self.compare_hashes(tick);
        Ok(())
    }

    fn compare_hashes(&mut self, tick: u64) {
        if self.desync.is_some() {
            return;
        }
        let (Some(&local_hash), Some(remote)) =
            (self.local_hashes.get(&tick), self.remote_hashes.get(&tick))
        else {
            return;
        };
        if let Some((&player, &remote_hash)) = remote.iter().find(|&(_, &h)| h != local_hash) {
            self.desync = Some(Desync {
                tick,
                player,
                local_hash,
                remote_hash,
            });
        }
        // Once every player has reported, the entries are no longer needed.
        if remote.len() + 1 == self.players.len() {
            self.local_hashes.remove(&tick);
            self.remote_hashes.remove(&tick);
        }
    }
}
