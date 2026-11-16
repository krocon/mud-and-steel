//! Player commands: the only way to change the simulation from outside.

use crate::combat::FireMode;
use crate::infantry::StanceOrder;
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
    /// Stance of infantry squads (prompt 022); ignored by vehicles.
    SetStance {
        units: Vec<UnitId>,
        stance: StanceOrder,
    },
    /// Infantry squads walk to the building (index into
    /// [`crate::Map::objects`]) and occupy it if there is room and no enemy
    /// inside (prompt 022). Ignored for vehicles and objects that are no
    /// buildings.
    Garrison { units: Vec<UnitId>, building: u32 },
    /// Places a construction site of the structure type `unit_type` (index
    /// into `SimConfig::unit_types`) at `position` and pays for it (prompt
    /// 023). Ignored where `Sim::placement` says no.
    Build { unit_type: u32, position: Vec2 },
    /// Queues a unit of `unit_type` in the own completed `building` and pays
    /// for it (prompt 023).
    Produce { building: UnitId, unit_type: u32 },
    /// Removes the last order of `building`'s queue with a full refund.
    CancelProduction { building: UnitId },
    /// Produced units of `building` drive to `point`.
    SetRallyPoint { building: UnitId, point: Vec2 },
    /// Units with an indirect weapon (prompt 032) fire `salvos` rounds
    /// each at random points within `radius_m` of `target` (a barrage),
    /// smoke shells if `smoke` (units without smoke ammunition ignore it).
    /// Salvos and radius are capped by `data/indirect.toml`.
    Barrage {
        units: Vec<UnitId>,
        target: Vec2,
        radius_m: u16,
        salvos: u16,
        smoke: bool,
    },
    /// Appends a waypoint: `units` drive to a formation around `target`
    /// once they are done with their current order (prompt 031). Like
    /// `MoveTo` if all of them are idle.
    QueueMove { units: Vec<UnitId>, target: Vec2 },
    /// `units` drive to `target`, then back and forth between their start
    /// and the target (prompt 031).
    Patrol { units: Vec<UnitId>, target: Vec2 },
    /// Marks `target` on the map of the issuer's allies (prompt 037). Has
    /// no effect on the simulation; it travels through the lockstep session
    /// (and into the replay) so that every ally's game can show it.
    Ping { target: Vec2 },
}

impl Command {
    /// The units the command is for.
    pub fn units(&self) -> &[UnitId] {
        match self {
            Self::MoveTo { units, .. }
            | Self::Stop { units }
            | Self::Attack { units, .. }
            | Self::AttackGround { units, .. }
            | Self::SetFireMode { units, .. }
            | Self::SetStance { units, .. }
            | Self::Garrison { units, .. }
            | Self::Barrage { units, .. }
            | Self::QueueMove { units, .. }
            | Self::Patrol { units, .. } => units,
            Self::Surrender
            | Self::Build { .. }
            | Self::Produce { .. }
            | Self::CancelProduction { .. }
            | Self::SetRallyPoint { .. }
            | Self::Ping { .. } => &[],
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
