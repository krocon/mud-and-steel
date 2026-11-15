//! Deterministic lockstep networking, independent of the transport.
//!
//! Every peer runs the same [`sim::Sim`]. Peers only exchange
//! [`CommandFrame`]s (the commands of one player for one tick) and periodic
//! [`HashReport`]s to detect desyncs. A tick is simulated once the frames of
//! all players for that tick are known.
//!
//! Local input is scheduled `input_delay` ticks ahead, so the frame has time to
//! reach the other peers before it is needed.
//!
//! Single player uses the same path with one player and [`LocalTransport`];
//! Network games use a [`lobby::LobbyNode`]: over ENet on the LAN
//! ([`lan::LanNode`]), over Steam online (`crates/steam`).
//! [`Replay`] records the merged commands of every tick for later playback.

pub mod lan;
pub mod lobby;
mod protocol;
pub mod replay;
mod session;
pub mod snapshot;
mod transport;

pub use protocol::{CommandFrame, HashReport, Message, PlayerId, decode, encode};
pub use replay::{Replay, ReplayError, ReplayRecorder};
pub use session::{
    Desync, LockstepConfig, LockstepSession, MAX_INPUT_DELAY, NetError, recommended_input_delay,
};
pub use transport::{LocalTransport, LoopbackTransport, Transport, TransportError};
