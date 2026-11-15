//! Deterministic lockstep networking, independent of the transport.
//!
//! Every peer runs the same [`sim::Sim`]. Peers only exchange
//! [`CommandFrame`]s (the commands of one player for one tick) and periodic
//! [`HashReport`]s to detect desyncs. A tick is simulated once the frames of
//! all players for that tick are known.
//!
//! Local input is scheduled `input_delay` ticks ahead, so the frame has time to
//! reach the other peers before it is needed.

mod protocol;
mod session;
mod transport;

pub use protocol::{CommandFrame, HashReport, Message, PlayerId, decode, encode};
pub use session::{Desync, LockstepConfig, LockstepSession, NetError};
pub use transport::{LoopbackTransport, Transport, TransportError};
