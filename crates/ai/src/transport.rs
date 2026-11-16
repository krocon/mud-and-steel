//! The computer player's transport (prompt 034): trucks and halftracks
//! carry waiting infantry to the front and unload it short of the enemy,
//! and fetch towed guns and set them up behind the army. Ordinary commands
//! only (`Embark`, `Disembark`, `Tow`, `Unlimber`, `MoveTo`).

use crate::{Ai, OwnUnit, PlayerView, Role, centroid, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, UnitId, Vec2};
use std::collections::BTreeMap;

/// Passengers get off this far short of the enemy (m).
const UNLOAD_M: i32 = 180;
/// Squads farther than this from their objective ride (m).
const RIDE_FROM_M: i32 = 200;
/// A carrier picks up squads within this distance (m).
const PICKUP_M: i32 = 120;
/// A boarding or towing order is given up after this many ticks.
const JOB_TICKS: u64 = 600;
/// Guns are set up this far behind the army's center (m).
const GUN_BEHIND_M: i32 = 60;
/// Guns farther than this from their place are fetched (m).
const FETCH_FROM_M: i32 = 150;
/// Distance at which a vehicle counts as arrived (m).
const ARRIVED_M: i32 = 30;

/// Running transport jobs of a computer player.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct TransportState {
    /// Squads sent to a carrier: squad -> (carrier, tick).
    boarding: BTreeMap<UnitId, (UnitId, u64)>,
    /// Guns a tractor was sent to: gun -> (tractor, tick).
    fetching: BTreeMap<UnitId, (UnitId, u64)>,
}

impl TransportState {
    /// Squads that walk to a carrier (the infantry group leaves them alone).
    pub(crate) fn boarding(&self, id: UnitId, tick: u64) -> bool {
        self.boarding
            .get(&id)
            .is_some_and(|&(_, since)| tick < since + JOB_TICKS)
    }
}

impl Ai {
    /// Carriers, tractors and guns.
    pub(crate) fn command_transport(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        let tick = view.tick;
        self.transport
            .boarding
            .retain(|_, &mut (_, since)| tick < since + JOB_TICKS);
        self.transport
            .fetching
            .retain(|_, &mut (_, since)| tick < since + JOB_TICKS);
        let own: Vec<OwnUnit> = view.own.iter().filter(|u| u.alive()).copied().collect();
        let types = self.types.clone();
        let role = |u: &OwnUnit| {
            types
                .get(u.unit_type as usize)
                .map_or(Role::Unarmed, |t| t.role)
        };
        // Squads aboard are done boarding.
        for u in own.iter().filter(|u| u.carrier.is_some()) {
            self.transport.boarding.remove(&u.id);
        }
        let objective = self.ride_objective(view);
        let enemies: Vec<Vec2> = view
            .enemies
            .iter()
            .filter(|e| e.alive() && self.type_info(e.unit_type).role != Role::Structure)
            .map(|e| e.position)
            .collect();
        // With a base, loaded carriers leave with a wave (prompt 023).
        let launched = !Self::has_base(view)
            || self.eco.launched.contains(&Role::Infantry)
            || self.eco.launched.contains(&Role::Armor);
        let place = self.gun_place(view);
        let towed: Vec<UnitId> = own.iter().filter_map(|u| u.towing).collect();
        for carrier in own.iter().filter(|u| role(u) == Role::Transport) {
            let info = self.type_info(carrier.unit_type);
            // A tractor with a gun brings it to its place and unlimbers.
            if carrier.towing.is_some() {
                if dist(carrier.position, place) <= Fixed::from_num(ARRIVED_M) {
                    if carrier.target.is_none() {
                        out.push(Command::Unlimber {
                            units: vec![carrier.id],
                        });
                    }
                } else {
                    self.drive(carrier, place, out);
                }
                continue;
            }
            if self
                .transport
                .fetching
                .values()
                .any(|&(t, _)| t == carrier.id)
            {
                continue;
            }
            if carrier.passengers > 0 {
                let waiting = self
                    .transport
                    .boarding
                    .values()
                    .any(|&(c, _)| c == carrier.id);
                if waiting || !launched {
                    continue;
                }
                let near_enemy = enemies
                    .iter()
                    .any(|&e| dist(e, carrier.position) <= Fixed::from_num(UNLOAD_M));
                let stop = toward(
                    carrier.position,
                    objective,
                    -Fixed::from_num(UNLOAD_M).min(dist(carrier.position, objective)),
                );
                if near_enemy || dist(carrier.position, stop) <= Fixed::from_num(ARRIVED_M) {
                    out.push(Command::Disembark {
                        units: vec![carrier.id],
                    });
                    if carrier.target.is_some() {
                        out.push(Command::Stop {
                            units: vec![carrier.id],
                        });
                    }
                } else {
                    self.drive(carrier, stop, out);
                }
                continue;
            }
            // Empty: fetch a gun, else pick up waiting squads.
            if info.tows
                && let Some(gun) = own
                    .iter()
                    .filter(|g| role(g) == Role::Gun && g.carrier.is_none())
                    .filter(|g| !self.transport.fetching.contains_key(&g.id))
                    .filter(|g| !towed.contains(&g.id))
                    .filter(|g| dist(g.position, place) > Fixed::from_num(FETCH_FROM_M))
                    .filter(|g| self.can_pull(carrier.unit_type, g.unit_type))
                    .min_by_key(|g| (dist(g.position, carrier.position), g.id))
            {
                out.push(Command::Tow {
                    units: vec![carrier.id],
                    gun: gun.id,
                });
                self.transport.fetching.insert(gun.id, (carrier.id, tick));
                continue;
            }
            if info.seats == 0 {
                continue;
            }
            let mut free = info.seats;
            let mut riders = Vec::new();
            let mut squads: Vec<&OwnUnit> = own
                .iter()
                .filter(|s| role(s) == Role::Infantry && s.carrier.is_none() && s.target.is_none())
                .filter(|s| !self.transport.boarding.contains_key(&s.id))
                .filter(|s| dist(s.position, carrier.position) <= Fixed::from_num(PICKUP_M))
                .filter(|s| dist(s.position, objective) > Fixed::from_num(RIDE_FROM_M))
                .collect();
            squads.sort_by_key(|s| (dist(s.position, carrier.position), s.id));
            for s in squads {
                let size = u32::from(self.squad_size(s.unit_type));
                if size == 0 || size > free {
                    continue;
                }
                free -= size;
                riders.push(s.id);
            }
            if !riders.is_empty() {
                for &id in &riders {
                    self.transport.boarding.insert(id, (carrier.id, tick));
                }
                out.push(Command::Embark {
                    units: riders,
                    carrier: carrier.id,
                });
            } else if carrier.target.is_none()
                && dist(carrier.position, place) > Fixed::from_num(ARRIVED_M)
                && enemies
                    .iter()
                    .any(|&e| dist(e, carrier.position) <= Fixed::from_num(2 * UNLOAD_M))
            {
                // An empty carrier does not wait under fire where it
                // unloaded: back behind the army (prompt 047, where squads
                // fight longer from cover and the enemy shoots at it).
                self.drive(carrier, place, out);
            }
        }
    }

    /// Where passengers are going: the freshest enemy sighting, else the
    /// enemy's start.
    fn ride_objective(&self, view: &PlayerView) -> Vec2 {
        let seen = centroid(
            view.enemies
                .iter()
                .filter(|e| e.alive() && self.type_info(e.unit_type).role != Role::Structure)
                .map(|e| e.position),
        );
        seen.or_else(|| {
            self.memory
                .values()
                .max_by_key(|s| s.tick)
                .map(|s| s.position)
        })
        .unwrap_or(self.enemy_home)
    }

    /// Where the guns stand: the gathering point of a base, else behind the
    /// army's center (towards the own start).
    fn gun_place(&self, view: &PlayerView) -> Vec2 {
        if Self::has_base(view) {
            return self.gather_point(view);
        }
        let army = centroid(
            view.own
                .iter()
                .filter(|u| u.alive() && u.carrier.is_none())
                .filter(|u| {
                    matches!(
                        self.type_info(u.unit_type).role,
                        Role::Armor | Role::Infantry
                    )
                })
                .map(|u| u.position),
        )
        .unwrap_or(self.home);
        self.clamp_to_map(toward(self.enemy_home, army, Fixed::from_num(GUN_BEHIND_M)))
    }

    /// Sends a vehicle to `target` unless it already drives there.
    fn drive(&self, unit: &OwnUnit, target: Vec2, out: &mut Vec<Command>) {
        let target = self.clamp_to_map(target);
        if unit
            .target
            .is_some_and(|t| dist(t, target) <= Fixed::from_num(ARRIVED_M))
        {
            return;
        }
        out.push(Command::MoveTo {
            units: vec![unit.id],
            target,
        });
    }
}
