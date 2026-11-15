//! Player commands: the only way to change the simulation from outside.

use crate::math::Vec2;
use serde::{Deserialize, Serialize};

/// Stable identifier of a unit. Assigned by the simulation in spawn order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnitId(pub u32);

/// A command issued by a player (human or AI) for one tick.
///
/// Commands are exchanged over the network in lockstep, so the enum must stay
/// serializable and its meaning must not depend on local state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Drive `unit` to `target` in a straight line.
    MoveTo { unit: UnitId, target: Vec2 },
}
