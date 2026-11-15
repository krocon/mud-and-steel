//! Wire messages of the lockstep protocol.

use serde::{Deserialize, Serialize};
use sim::Command;
pub use sim::PlayerId;

/// All commands of one player for one tick. An empty frame is still sent:
/// it tells the other peers that the player has nothing to do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFrame {
    pub tick: u64,
    pub player: PlayerId,
    pub commands: Vec<Command>,
}

/// State hash of a player after simulating `tick` ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashReport {
    pub tick: u64,
    pub player: PlayerId,
    pub hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Frame(CommandFrame),
    Hash(HashReport),
}

/// Serializes a message with postcard (compact, stable wire format).
pub fn encode(message: &Message) -> Vec<u8> {
    postcard::to_allocvec(message).expect("serializing into a Vec cannot fail")
}

pub fn decode(bytes: &[u8]) -> Result<Message, postcard::Error> {
    postcard::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{UnitId, Vec2};

    #[test]
    fn roundtrip() {
        let message = Message::Frame(CommandFrame {
            tick: 7,
            player: PlayerId(1),
            commands: vec![
                Command::MoveTo {
                    units: vec![UnitId(3), UnitId(5)],
                    target: Vec2::from_millimeters(-1500, 2750),
                },
                Command::Stop {
                    units: vec![UnitId(4)],
                },
                Command::Surrender,
            ],
        });
        assert_eq!(decode(&encode(&message)).unwrap(), message);
    }
}
