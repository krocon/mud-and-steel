//! The computer player's supply and repair (prompt 036): units with little
//! ammunition or fuel, and mobile vehicles with half their hit points
//! gone, drive to the nearest supplier or repairer (own or allied
//! headquarters, depot, vehicle works, truck) and rejoin their group when
//! they are refilled; damaged buildings are repaired when the account
//! allows. Ordinary commands only (`MoveTo`, `Repair`).

use crate::{Ai, OwnUnit, PlayerView, Role, dist};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, SupplySource, UnitId, Vec2};
use std::collections::BTreeMap;

/// Mobile vehicles at or below this share of their hit points are repaired.
const REPAIR_BELOW_PCT: u64 = 50;
/// Back to the group at this share of ammunition, fuel and hit points.
const DONE_PCT: u8 = 90;
/// A unit on its way is ordered again after this many ticks, and given up
/// after `GIVE_UP_TICKS`.
const REISSUE_TICKS: u64 = 400;
const GIVE_UP_TICKS: u64 = 20 * 180;
/// Buildings below this share of their hit points are repaired.
const BUILDING_REPAIR_BELOW_PCT: u64 = 70;
/// A building repair is only ordered with this much of every resource
/// (production comes first; a full repair costs half the building's price).
const REPAIR_RESERVE: u32 = 200;

/// Units sent to resupply or repair, and buildings under repair.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SupplyState {
    /// Unit -> (tick of the first order, tick of the last order).
    trips: BTreeMap<UnitId, (u64, u64)>,
    /// Buildings the AI ordered repaired (the order toggles).
    repairing: Vec<UnitId>,
}

impl SupplyState {
    /// Units on a supply trip (their group leaves them alone).
    pub(crate) fn busy(&self, id: UnitId) -> bool {
        self.trips.contains_key(&id)
    }
}

/// What unit `u` needs: (supply, repair).
fn needs(u: &OwnUnit, low_ammo: u8, low_fuel: u8) -> (bool, bool) {
    let s = u.stock;
    let supply =
        s.ammo_pct.is_some_and(|a| a <= low_ammo) || s.fuel_pct.is_some_and(|f| f <= low_fuel);
    // Vehicles (they have fuel) that can still drive.
    let repair = s.fuel_pct.is_some()
        && !s.immobile
        && u64::from(u.hp) * 100 <= u64::from(u.max_hp) * REPAIR_BELOW_PCT;
    (supply, repair)
}

/// Whether unit `u` is refilled and repaired enough to rejoin.
fn done(u: &OwnUnit) -> bool {
    let s = u.stock;
    s.ammo_pct.is_none_or(|a| a >= DONE_PCT)
        && s.fuel_pct.is_none_or(|f| f >= DONE_PCT)
        && (s.fuel_pct.is_none()
            || u64::from(u.hp) * 100 >= u64::from(u.max_hp) * u64::from(DONE_PCT))
}

impl Ai {
    /// Supply trips and building repairs.
    pub(crate) fn command_supply(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        let (low_ammo, low_fuel) = self.low_stock;
        let tick = view.tick;
        // Trips end when the unit is gone, refilled, stranded or too long
        // under way.
        let alive = |id: UnitId| view.own.iter().find(|u| u.id == id && u.alive());
        self.supply.trips.retain(|&id, &mut (since, _)| {
            alive(id).is_some_and(|u| {
                !done(u)
                    && !u.stock.immobile
                    && u.stock.fuel_pct != Some(0)
                    && tick < since + GIVE_UP_TICKS
            })
        });
        for u in view.own.iter().filter(|u| u.alive() && u.carrier.is_none()) {
            let role = self.type_info(u.unit_type).role;
            if !matches!(
                role,
                Role::Armor | Role::Infantry | Role::Artillery | Role::Transport
            ) {
                continue;
            }
            let (supply, repair) = needs(u, low_ammo, low_fuel);
            let trip = self.supply.trips.get(&u.id).copied();
            if trip.is_none() && !supply && !repair {
                continue;
            }
            if u.stock.immobile || u.stock.fuel_pct == Some(0) {
                continue;
            }
            let Some((source, radius)) =
                nearest_source(&view.sources, u, supply || trip.is_some(), repair)
            else {
                continue;
            };
            let d = dist(u.position, source);
            let inside = d <= radius;
            match trip {
                None => {
                    self.supply.trips.insert(u.id, (tick, tick));
                    if !inside {
                        out.push(goto(u.id, u.position, source, radius));
                    }
                }
                Some((since, last)) => {
                    if !inside && u.target.is_none() && tick >= last + REISSUE_TICKS {
                        self.supply.trips.insert(u.id, (since, tick));
                        out.push(goto(u.id, u.position, source, radius));
                    }
                }
            }
        }
        self.repair_buildings(view, out);
    }

    /// Orders damaged, completed own buildings repaired while the account
    /// has a reserve.
    fn repair_buildings(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        let hp = |id: UnitId| {
            view.own
                .iter()
                .find(|u| u.id == id)
                .map(|u| (u.hp, u.max_hp))
        };
        self.supply
            .repairing
            .retain(|&id| hp(id).is_some_and(|(hp, max)| hp > 0 && hp < max));
        if view.account.iter().any(|&r| r < REPAIR_RESERVE) {
            return;
        }
        for s in view.structures.iter().filter(|s| s.completed()) {
            let Some((hp, max)) = hp(s.unit) else {
                continue;
            };
            if hp == 0
                || u64::from(hp) * 100 >= u64::from(max) * BUILDING_REPAIR_BELOW_PCT
                || self.supply.repairing.contains(&s.unit)
            {
                continue;
            }
            out.push(Command::Repair { building: s.unit });
            self.supply.repairing.push(s.unit);
        }
    }
}

/// The nearest source for unit `u` that supplies (`supply`) or repairs
/// (`repair`), and the radius to drive into. Units do not count as their
/// own source.
fn nearest_source(
    sources: &[SupplySource],
    u: &OwnUnit,
    supply: bool,
    repair: bool,
) -> Option<(Vec2, Fixed)> {
    sources
        .iter()
        .filter(|s| s.unit != u.id)
        .filter_map(|s| {
            let r = match (supply, repair) {
                (true, true) if s.supply_m > 0 && s.repair_m > 0 => s.supply_m.min(s.repair_m),
                (true, _) if s.supply_m > 0 => s.supply_m,
                (false, true) if s.repair_m > 0 => s.repair_m,
                _ => return None,
            };
            Some((s.position, Fixed::from_num(r), s.unit))
        })
        .min_by_key(|&(p, _, id)| (dist(p, u.position), id))
        .map(|(p, r, _)| (p, r))
}

/// A move order into the radius of a source: half way in from the side
/// the unit comes from.
fn goto(id: UnitId, from: Vec2, source: Vec2, radius: Fixed) -> Command {
    let d = dist(from, source);
    let target = if d > Fixed::ZERO {
        source + (from - source).scale(radius / 2 / d)
    } else {
        source
    };
    Command::MoveTo {
        units: vec![id],
        target,
    }
}
