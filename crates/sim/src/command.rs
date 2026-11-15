//! Player commands: the only way to change the simulation from outside.

use crate::combat::FireMode;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};

/// Stable identifier of a unit. Assigned by the simulation in spawn order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnitId(pub u32);

/// Player slot in a match. Owns units and determines the order in which
/// commands of the same tick are applied (ascending), so it must be identical
/// on all peers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u8);

/// A command issued by a player (human or AI) for one tick.
///
/// Commands are exchanged over the network in lockstep, so the enum must stay
/// serializable and its meaning must not depend on local state. A command
/// never names its issuer: the player comes from the lockstep frame (see
/// [`PlayerCommand`]). Units that do not exist or belong to another player are
/// ignored, as are duplicates in `units`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Drive `units` in a straight line to a grid formation around `target`.
    MoveTo { units: Vec<UnitId>, target: Vec2 },
    /// `units` stop where they are (and drop an attack order).
    Stop { units: Vec<UnitId> },
    /// `units` drive into range of `target` and fire at it until it is
    /// destroyed. Ignored for own targets.
    Attack { units: Vec<UnitId>, target: UnitId },
    /// `units` drive into range of the ground point `target` and fire at it.
    AttackGround { units: Vec<UnitId>, target: Vec2 },
    /// Whether `units` pick targets on their own.
    SetFireMode { units: Vec<UnitId>, mode: FireMode },
    /// The issuing player gives up and is defeated (prompt 027).
    Surrender,
}

impl Command {
    /// The units the command is for.
    pub fn units(&self) -> &[UnitId] {
        match self {
            Self::MoveTo { units, .. }
            | Self::Stop { units }
            | Self::Attack { units, .. }
            | Self::AttackGround { units, .. }
            | Self::SetFireMode { units, .. } => units,
            Self::Surrender => &[],
        }
    }
}

/// A command together with the player that issued it, as the simulation
/// consumes it. Built by the lockstep layer from the frame's sender.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub player: PlayerId,
    pub command: Command,
}

impl PlayerCommand {
    pub const fn new(player: PlayerId, command: Command) -> Self {
        Self { player, command }
    }
}
