//! Byte encoding of simulation snapshots (prompt 038) and the rules hash.
//!
//! # Format
//!
//! ```text
//! offset  size  content
//! 0       4     magic "MSSN" (Mud and Steel SNapshot)
//! 4       2     format version, u16 little endian (FORMAT_VERSION)
//! 6       8     Sim::state_hash at the snapshot, u64 little endian
//! 14      ..    deflate (miniz_oxide) of the postcard encoding of
//!               sim::snapshot::SimSnapshot
//! ```
//!
//! [`decode`] checks the stored state hash against the restored simulation,
//! so a snapshot that does not fit the build (other rules, other state
//! layout) is rejected instead of continuing a different game.
//!
//! # Rules hash
//!
//! [`rules_hash`] identifies the simulation rules and the state layout of
//! this build: the hash of the snapshot bytes after a fixed reference run of
//! the demo. Any change of the rules shows up in the state, any new state
//! field in the bytes (postcard writes every field), so replays and saved
//! games of a build with other rules are rejected up front.

use serde::Serialize;
use sim::snapshot::SimSnapshotOwned;
use sim::{Map, Sim, StableHasher};
use std::fmt;
use std::sync::{Arc, OnceLock};

pub const MAGIC: [u8; 4] = *b"MSSN";

/// Version of this byte layout (not of the state: that is the rules hash).
pub const FORMAT_VERSION: u16 = 1;

const HEADER_LEN: usize = MAGIC.len() + 2 + 8;

/// Fast deflate: snapshots are written while the game runs.
const DEFLATE_LEVEL: u8 = 1;

/// Decompressed snapshots larger than this are rejected (garbage input).
const MAX_STATE_BYTES: usize = 1 << 30;

/// Reference run of [`rules_hash`]: the demo battle, seed and ticks.
const RULES_SEED: u64 = 7;
const RULES_TICKS: u64 = 400;

#[derive(Debug)]
pub enum SnapshotError {
    BadMagic,
    UnsupportedVersion(u16),
    Inflate,
    Decode(postcard::Error),
    Restore(sim::snapshot::SnapshotError),
    /// The restored simulation has another state hash than the one saved.
    HashMismatch {
        expected: u64,
        actual: u64,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadMagic => write!(f, "not a Mud and Steel snapshot"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "snapshot format version {v} is not supported (expected {FORMAT_VERSION})"
            ),
            Self::Inflate => write!(f, "snapshot data is damaged"),
            Self::Decode(e) => write!(f, "malformed snapshot: {e}"),
            Self::Restore(e) => write!(f, "{e}"),
            Self::HashMismatch { expected, actual } => write!(
                f,
                "snapshot does not restore the saved state (hash {actual:#018x}, saved {expected:#018x})"
            ),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Postcard bytes of the snapshot, uncompressed.
fn state_bytes(sim: &Sim, loaded: &Map) -> Vec<u8> {
    postcard::to_allocvec(&sim.save_state(loaded)).expect("serializing into a Vec cannot fail")
}

/// Encodes the complete state of `sim`; `loaded` is the map as loaded (the
/// one given to `Sim::new`).
pub fn encode(sim: &Sim, loaded: &Map) -> Vec<u8> {
    let body = miniz_oxide::deflate::compress_to_vec(&state_bytes(sim, loaded), DEFLATE_LEVEL);
    let mut bytes = Vec::with_capacity(HEADER_LEN + body.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&sim.state_hash().to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes
}

/// The state hash stored in the header, without decoding the body.
pub fn stored_hash(bytes: &[u8]) -> Result<u64, SnapshotError> {
    check_header(bytes)?;
    Ok(u64::from_le_bytes(
        bytes[6..HEADER_LEN].try_into().expect("8 bytes"),
    ))
}

fn check_header(bytes: &[u8]) -> Result<(), SnapshotError> {
    if bytes.len() < HEADER_LEN || bytes[..MAGIC.len()] != MAGIC {
        return Err(SnapshotError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != FORMAT_VERSION {
        return Err(SnapshotError::UnsupportedVersion(version));
    }
    Ok(())
}

/// Restores a simulation from [`encode`]d bytes on `loaded` (the map as
/// loaded) and checks its state hash.
pub fn decode(bytes: &[u8], loaded: Arc<Map>) -> Result<Sim, SnapshotError> {
    let expected = stored_hash(bytes)?;
    let raw =
        miniz_oxide::inflate::decompress_to_vec_with_limit(&bytes[HEADER_LEN..], MAX_STATE_BYTES)
            .map_err(|_| SnapshotError::Inflate)?;
    let snapshot: SimSnapshotOwned = postcard::from_bytes(&raw).map_err(SnapshotError::Decode)?;
    let sim = Sim::restore(snapshot, loaded).map_err(SnapshotError::Restore)?;
    let actual = sim.state_hash();
    if actual != expected {
        return Err(SnapshotError::HashMismatch { expected, actual });
    }
    Ok(sim)
}

/// Hash of the simulation rules and state layout of this build (see the
/// module docs). Computed once per process (a short headless demo run).
pub fn rules_hash() -> u64 {
    static HASH: OnceLock<u64> = OnceLock::new();
    *HASH.get_or_init(|| {
        let sim = sim::scenario::run_demo(RULES_SEED, RULES_TICKS);
        let mut h = StableHasher::new();
        h.write_bytes(&state_bytes(&sim, &sim::scenario::builtin_map()));
        h.finish()
    })
}

/// Encodes any value with postcard (for the bridge and the save files).
pub(crate) fn to_bytes(value: &impl Serialize) -> Vec<u8> {
    postcard::to_allocvec(value).expect("serializing into a Vec cannot fail")
}
