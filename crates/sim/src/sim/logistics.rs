//! Supply, consumption and repair (prompt 036). Rules and catalog data:
//! `crate::logistics`.
//!
//! - Every unit has a [`Stock`] of ammunition and fuel in millionths of a
//!   full load. Units without a stock of a kind (unarmed units, buildings,
//!   fortifications; fuel: everything but vehicles) keep it full.
//! - A salvo of the main weapon (direct, or a shell of an indirect weapon)
//!   costs `FULL / salvos`; without ammunition the unit fires no weapon.
//!   Driving costs fuel by distance; without fuel a vehicle stands.
//! - Vehicles at or below `immobile_pct` of their hit points cannot move.
//! - Once per game second, units near a supplier of their own player or
//!   an ally regain ammunition and fuel (their player pays), vehicles and
//!   towed guns near a repairer regain hit points (free), buildings under
//!   a repair order regain hit points (paid).
//! - Buildings can be sold, construction sites cancelled
//!   ([`crate::Command::Sell`]): a sold structure is gone (hit points 0,
//!   ground free, no longer in the snapshot).
//!
//! Order within a tick (see [`Sim::step`]): consumption where units fire
//! (combat, indirect fire) and move; [`Sim::update_logistics`] after the
//! economy.

use super::{Sim, Unit, stop, transport};
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::logistics::{FULL, LogisticsRules, LogisticsType};
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Ammunition and fuel of a unit, in millionths of a full load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Stock {
    pub(super) ammo: u32,
    pub(super) fuel: u32,
}

impl Default for Stock {
    fn default() -> Self {
        Self {
            ammo: FULL,
            fuel: FULL,
        }
    }
}

/// What logistics knows about a unit type of the match.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct TypeLog {
    /// Ammunition per salvo of the main weapon (0: no stock).
    salvo_cost: u32,
    /// The main weapon fires directly (else it is the indirect weapon).
    direct_main: bool,
    /// Meters on a full tank (0: no fuel stock).
    fuel_m: u32,
    /// A vehicle: can be immobilized.
    vehicle: bool,
    /// Vehicles and towed guns are repaired.
    repairable: bool,
    /// Supply radius, if a supplier.
    supply_m: Option<u32>,
    /// Repair radius and rate (percent per second), if a repairer.
    repair: Option<(u32, u32)>,
    /// The source is a base building (works only when completed).
    building: bool,
}

/// Logistics state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Logistics {
    pub(super) rules: LogisticsRules,
    types: Vec<TypeLog>,
    /// Buildings under a repair order, by id.
    repairing: Vec<UnitId>,
    /// Sold buildings and cancelled construction sites, by id.
    sold: Vec<UnitId>,
}

/// A supplier or repairer as the presentation and the AI see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SupplySource {
    pub unit: UnitId,
    pub owner: PlayerId,
    pub position: Vec2,
    /// Supply radius in meters (0: no supplier).
    pub supply_m: u32,
    /// Repair radius in meters (0: no repairer).
    pub repair_m: u32,
}

/// Stocks of a unit for the presentation and the AI (percent; `None`:
/// the unit has no such stock).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StockView {
    pub ammo_pct: Option<u8>,
    pub fuel_pct: Option<u8>,
    /// Too damaged to move (prompt 036).
    pub immobile: bool,
}

fn pct(value: u32) -> u8 {
    u8::try_from(u64::from(value) * 100 / u64::from(FULL)).unwrap_or(100)
}

impl Logistics {
    pub(super) fn new(rules: LogisticsRules, unit_types: &[crate::data::UnitType]) -> Self {
        let catalog = crate::economy::builtin_catalog();
        let scale = crate::scale::Scale::builtin().ok();
        let types = unit_types
            .iter()
            .map(|t| {
                let category = t.combat.category;
                let group = category.map(crate::catalog::Category::group);
                let stocks = match (catalog.and_then(|c| c.get(&t.name)), &scale) {
                    (Some(e), Some(s)) if t.mobile() => LogisticsType::from_catalog(e, s),
                    _ => LogisticsType::default(),
                };
                let supplier = rules
                    .suppliers
                    .iter()
                    .find(|s| s.matches(&t.name, category));
                let repairer = rules
                    .repairers
                    .iter()
                    .find(|s| s.matches(&t.name, category));
                TypeLog {
                    salvo_cost: FULL.checked_div(stocks.salvos).map_or(0, |c| c.max(1)),
                    direct_main: t.combat.indirect.is_none(),
                    fuel_m: stocks.fuel_m,
                    vehicle: t.mobile() && group == Some(crate::catalog::Group::Vehicle),
                    repairable: t.mobile()
                        && matches!(
                            group,
                            Some(crate::catalog::Group::Vehicle | crate::catalog::Group::Gun)
                        ),
                    supply_m: supplier.map(|s| s.radius_m),
                    repair: repairer.map(|s| (s.radius_m, s.pct_per_s)),
                    building: supplier.or(repairer).is_some_and(|s| s.building.is_some()),
                }
            })
            .collect();
        Self {
            rules,
            types,
            repairing: Vec::new(),
            sold: Vec::new(),
        }
    }
}

impl Sim {
    fn log_type(&self, i: usize) -> &TypeLog {
        &self.logistics.types[self.units[i].unit_type as usize]
    }

    /// Whether unit `i` has ammunition for a salvo (or needs none).
    pub(super) fn has_ammo(&self, i: usize) -> bool {
        let cost = self.log_type(i).salvo_cost;
        cost == 0 || self.units[i].stock.ammo >= cost
    }

    /// Unit `i` fired a salvo of its main weapon; `indirect`: a shell of
    /// its indirect weapon.
    pub(super) fn spend_salvo(&mut self, i: usize, indirect: bool) {
        let t = self.log_type(i);
        if t.direct_main == indirect {
            return;
        }
        let cost = t.salvo_cost;
        let stock = &mut self.units[i].stock;
        stock.ammo = stock.ammo.saturating_sub(cost);
    }

    /// Whether unit `i` is a vehicle too damaged to move.
    fn immobile(&self, i: usize) -> bool {
        let u = &self.units[i];
        let max = self.types[u.unit_type as usize].max_hp;
        self.log_type(i).vehicle
            && u.alive()
            && u64::from(u.hp) * 100
                <= u64::from(max) * u64::from(self.logistics.rules.damage.immobile_pct)
    }

    /// Before the movement: vehicles without fuel or too damaged to move
    /// stand (their route is dropped).
    pub(super) fn halt_stranded(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if u.target.is_none() || !u.alive() {
                continue;
            }
            let t = self.log_type(i);
            let dry = t.fuel_m > 0 && u.stock.fuel == 0;
            if dry || self.immobile(i) {
                // Queued waypoints and patrols end too (no new path every
                // tick).
                stop(&mut self.units[i]);
                self.units[i].orders.clear();
            }
        }
    }

    /// After the movement: unit `i` drove `distance` meters.
    pub(super) fn burn_fuel(&mut self, i: usize, distance: Fixed) {
        let fuel_m = self.log_type(i).fuel_m;
        if fuel_m == 0 || distance <= Fixed::ZERO {
            return;
        }
        let used = (distance * Fixed::from_num(FULL) / Fixed::from_num(fuel_m)).ceil();
        let used = used.to_num::<u32>().max(1);
        let stock = &mut self.units[i].stock;
        stock.fuel = stock.fuel.saturating_sub(used);
    }

    /// Resupply, repair and building repair, once per game second (after
    /// the economy).
    pub(super) fn update_logistics(&mut self) {
        if !(self.tick + 1).is_multiple_of(u64::from(self.tick_hz)) {
            return;
        }
        let sources = self.sources(None);
        if !sources.is_empty() {
            self.resupply_and_repair(&sources);
        }
        self.repair_buildings();
    }

    /// Working suppliers and repairers (of `player` and their allies, or of
    /// everybody), by unit id.
    fn sources(&self, player: Option<PlayerId>) -> Vec<SupplySource> {
        let mut out = Vec::new();
        for u in &self.units {
            let t = &self.logistics.types[u.unit_type as usize];
            if (t.supply_m.is_none() && t.repair.is_none()) || !u.alive() || transport::aboard(u) {
                continue;
            }
            if player.is_some_and(|p| !self.teams.allied(p, u.owner)) {
                continue;
            }
            if t.building && self.economy.completed_owner(u.id).is_none() {
                continue;
            }
            out.push(SupplySource {
                unit: u.id,
                owner: u.owner,
                position: u.position,
                supply_m: t.supply_m.unwrap_or(0),
                repair_m: t.repair.map_or(0, |r| r.0),
            });
        }
        out
    }

    /// Mission action `resupply` (prompt 118): supply containers dropped
    /// from the air. Every living unit of `player` within `radius_m` of
    /// `target` that has an ammunition stock and is not riding regains
    /// `pct` percent of a full load, free (the script decides where they
    /// land; containers in the wrong place help nobody or the enemy).
    pub(super) fn drop_supplies(&mut self, player: PlayerId, target: Vec2, radius_m: u16, pct: u8) {
        let reach = Fixed::from_num(radius_m);
        let add = FULL / 100 * u32::from(pct.min(100));
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive()
                || u.owner != player
                || transport::aboard(u)
                || (u.position - target).length() > reach
            {
                continue;
            }
            if self.logistics.types[u.unit_type as usize].salvo_cost == 0 {
                continue;
            }
            let stock = &mut self.units[i].stock;
            stock.ammo = stock.ammo.saturating_add(add).min(FULL);
        }
    }

    fn resupply_and_repair(&mut self, sources: &[SupplySource]) {
        let rules = &self.logistics.rules;
        let gain = FULL / 100 * rules.resupply.pct_per_s;
        let cost_pct = rules.resupply.cost_pct;
        let near = |p: Vec2, s: &SupplySource, r: u32| {
            r > 0 && (p - s.position).length() <= Fixed::from_num(r)
        };
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive() || transport::aboard(u) {
                continue;
            }
            let t = &self.logistics.types[u.unit_type as usize];
            let (owner, position) = (u.owner, u.position);
            let max_hp = self.types[u.unit_type as usize].max_hp;
            let wants_ammo = t.salvo_cost > 0 && u.stock.ammo < FULL;
            let wants_fuel = t.fuel_m > 0 && u.stock.fuel < FULL;
            let wants_repair = t.repairable && u.hp < max_hp;
            if !(wants_ammo || wants_fuel || wants_repair) {
                continue;
            }
            let allied = |s: &SupplySource| self.teams.allied(s.owner, owner);
            if (wants_ammo || wants_fuel)
                && sources
                    .iter()
                    .any(|s| allied(s) && near(position, s, s.supply_m))
            {
                let price = self
                    .economy
                    .price(self.units[i].unit_type as usize)
                    .unwrap_or([0; 3]);
                let stock = self.units[i].stock;
                if wants_ammo {
                    let add = gain.min(FULL - stock.ammo);
                    let bill = [0, price[1] * cost_pct / 100, 0];
                    if self.economy.pay_share(owner, bill, add, FULL) {
                        self.units[i].stock.ammo += add;
                    }
                }
                if wants_fuel {
                    let add = gain.min(FULL - stock.fuel);
                    let bill = [price[0] * cost_pct / 100, 0, 0];
                    if self.economy.pay_share(owner, bill, add, FULL) {
                        self.units[i].stock.fuel += add;
                    }
                }
            }
            if wants_repair {
                let rate = sources
                    .iter()
                    .filter(|s| allied(s) && s.unit != self.units[i].id)
                    .filter_map(|s| {
                        let t = &self.logistics.types[self.unit_type_of(s.unit)?];
                        let (radius, rate) = t.repair?;
                        near(position, s, radius).then_some(rate)
                    })
                    .max();
                if let Some(rate) = rate {
                    let add = (u64::from(max_hp) * u64::from(rate) / 100).max(1);
                    let u = &mut self.units[i];
                    u.hp = u32::try_from((u64::from(u.hp) + add).min(u64::from(max_hp)))
                        .unwrap_or(max_hp);
                }
            }
        }
    }

    fn unit_type_of(&self, id: UnitId) -> Option<usize> {
        self.units
            .binary_search_by_key(&id, |u| u.id)
            .ok()
            .map(|i| self.units[i].unit_type as usize)
    }

    /// Buildings under a repair order regain hit points while their player
    /// pays; the order ends when they are whole or gone.
    fn repair_buildings(&mut self) {
        let b = self.logistics.rules.buildings;
        let mut done = Vec::new();
        for k in 0..self.logistics.repairing.len() {
            let id = self.logistics.repairing[k];
            let Ok(i) = self.units.binary_search_by_key(&id, |u| u.id) else {
                done.push(id);
                continue;
            };
            let t = self.units[i].unit_type as usize;
            let max = self.types[t].max_hp;
            let (owner, hp) = (self.units[i].owner, self.units[i].hp);
            if hp == 0 || hp >= max || self.economy.completed_owner(id).is_none() {
                done.push(id);
                continue;
            }
            let add =
                (u64::from(max) * u64::from(b.repair_pct_per_s) / 100).min(u64::from(max - hp));
            let add = u32::try_from(add).unwrap_or(0).max(1);
            let price = self.economy.price(t).unwrap_or([0; 3]);
            let bill = price.map(|p| p * b.repair_cost_pct / 100);
            if self.economy.pay_share(owner, bill, add, max) {
                self.units[i].hp = (hp + add).min(max);
            }
        }
        self.logistics.repairing.retain(|id| !done.contains(id));
    }

    /// [`crate::Command::Repair`]: starts (or stops) repairing an own
    /// completed, damaged building.
    pub(super) fn order_repair(&mut self, player: PlayerId, building: UnitId) {
        if let Ok(k) = self.logistics.repairing.binary_search(&building) {
            if self.economy.completed_owner(building) == Some(player) {
                self.logistics.repairing.remove(k);
            }
            return;
        }
        let Ok(i) = self.units.binary_search_by_key(&building, |u| u.id) else {
            return;
        };
        let max = self.types[self.units[i].unit_type as usize].max_hp;
        if self.economy.completed_owner(building) != Some(player)
            || !self.units[i].alive()
            || self.units[i].hp >= max
        {
            return;
        }
        let at = self.logistics.repairing.partition_point(|&r| r < building);
        self.logistics.repairing.insert(at, building);
    }

    /// [`crate::Command::Sell`]: sells an own completed building or
    /// cancels an own construction site.
    pub(super) fn order_sell(&mut self, player: PlayerId, building: UnitId) {
        let Ok(i) = self.units.binary_search_by_key(&building, |u| u.id) else {
            return;
        };
        let b = self.logistics.rules.buildings;
        if self.units[i].owner != player || !self.units[i].alive() {
            return;
        }
        if self.remove_structure(i, b.sell_refund_pct, b.cancel_refund_pct) {
            let at = self.logistics.sold.partition_point(|&s| s < building);
            self.logistics.sold.insert(at, building);
            self.logistics.repairing.retain(|&r| r != building);
        }
    }

    /// Whether unit `id` was sold (it is left out of the snapshot).
    pub(super) fn sold(&self, id: UnitId) -> bool {
        self.logistics.sold.binary_search(&id).is_ok()
    }

    /// Stocks of unit `u` in percent.
    pub(super) fn stock_view(&self, u: &Unit) -> StockView {
        let t = &self.logistics.types[u.unit_type as usize];
        let index = self.units.binary_search_by_key(&u.id, |x| x.id).ok();
        StockView {
            ammo_pct: (t.salvo_cost > 0).then(|| pct(u.stock.ammo)),
            fuel_pct: (t.fuel_m > 0).then(|| pct(u.stock.fuel)),
            immobile: index.is_some_and(|i| self.immobile(i)),
        }
    }

    pub(super) fn hash_logistics(&self, h: &mut StableHasher) {
        self.logistics.rules.hash_into(h);
        for u in &self.units {
            h.write_u32(u.stock.ammo);
            h.write_u32(u.stock.fuel);
        }
        for list in [&self.logistics.repairing, &self.logistics.sold] {
            h.write_u64(list.len() as u64);
            for id in list {
                h.write_u32(id.0);
            }
        }
    }

    // --- Public queries (presentation, AI) -----------------------------------

    /// Ammunition, fuel and mobility of unit `id` (`None` if it does not
    /// exist).
    pub fn unit_stock(&self, id: UnitId) -> Option<StockView> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        Some(self.stock_view(&self.units[i]))
    }

    /// Suppliers and repairers that serve `player` (own and allied), by
    /// unit id.
    pub fn supply_sources(&self, player: PlayerId) -> Vec<SupplySource> {
        self.sources(Some(player))
    }

    /// Whether building `id` is under a repair order.
    pub fn building_repairing(&self, id: UnitId) -> bool {
        self.logistics.repairing.binary_search(&id).is_ok()
    }

    /// The supply and repair rules of the match.
    pub const fn logistics_rules(&self) -> &LogisticsRules {
        &self.logistics.rules
    }
}
