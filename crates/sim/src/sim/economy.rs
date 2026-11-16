//! Accounts, construction sites, production queues and supply points
//! (prompt 023). The rules: `crate::economy`; the design:
//! `docs/GAME_DESIGN.md`.
//!
//! Buildings, construction sites and fortifications are ordinary units
//! that cannot move (speed 0), so combat, vision, destruction and victory
//! conditions treat them like any other unit. This module keeps what only
//! the economy needs: which units are structures, how far they are built,
//! what they produce, and every player's resources.

use super::{NewUnit, Sim, Unit, new_unit};
use crate::combat::FireMode;
use crate::command::{PlayerId, UnitId};
use crate::economy::{Cost, EconomyRules, Resources, StructureRule, faction_of};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::path::SizeClass;
use serde::{Deserialize, Serialize};

/// Account values are kept in this many parts of a resource unit, so an
/// income per minute pays exactly every second.
const PARTS: u64 = 60_000;

/// A faction id from [`faction_of`]. An alias, so that serde's derive does
/// not see the `&'static str` (it would borrow it and bind `'de: 'static`).
type StaticFaction = Option<&'static str>;

/// What the economy knows about a unit type of the match.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct TypeEco {
    /// The type is a structure (base building or fortification).
    structure: bool,
    /// Base building index in the rules (`None` for fortifications).
    building: Option<usize>,
    /// Price and time to build or produce it (`None`: cannot be bought).
    cost: Option<Cost>,
    /// Hit points of a structure from the rules (0 = combat rules).
    hp: u32,
    /// Build radius it gives when completed (m).
    radius_m: u32,
    income: Resources,
    max_count: u32,
    /// Type indices of the structures it needs.
    requires: Vec<u32>,
    /// Production list name (base buildings).
    produces: String,
    /// Faction of a catalog type (`de`, `su`, `us`).
    #[serde(with = "static_faction")]
    faction: StaticFaction,
    /// Half length (along the facing) and half width in meters.
    half: (Fixed, Fixed),
}

/// One order of a production queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Order {
    unit_type: u32,
    /// Ticks still needed.
    left: u32,
    total: u32,
    paid: Resources,
}

/// A unit that is a structure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Structure {
    unit: UnitId,
    owner: PlayerId,
    unit_type: u32,
    /// Ticks until completed; 0 = completed.
    build_left: u32,
    build_total: u32,
    queue: Vec<Order>,
    rally: Vec2,
}

/// A supply point from a map marker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SupplyPoint {
    name: String,
    position: Vec2,
    owner: Option<PlayerId>,
    /// Who is taking it, and for how many ticks so far.
    taker: Option<PlayerId>,
    progress: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Account {
    player: PlayerId,
    /// In [`PARTS`] per resource unit.
    parts: [u64; 3],
    /// Faction of the player (from their first catalog unit).
    #[serde(with = "static_faction")]
    faction: StaticFaction,
}

/// Economy state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Economy {
    rules: EconomyRules,
    pub(super) types: Vec<TypeEco>,
    accounts: Vec<Account>,
    /// By unit id.
    structures: Vec<Structure>,
    supply: Vec<SupplyPoint>,
    tick_hz: u32,
}

/// Why a building cannot be placed (prompt 023).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The type is no structure this player may build.
    NotBuildable,
    /// A required building is missing (or not completed).
    Requires,
    /// The player has as many of this type as allowed.
    Limit,
    /// The player has as many construction sites as allowed.
    Sites,
    NotEnoughResources,
    /// Not within the build radius of an own completed building.
    OutOfRange,
    /// Outside the map.
    OutsideMap,
    /// Water, forest, objects, wrecks or other buildings in the way.
    Blocked,
    TooSteep,
    /// A unit stands in the way.
    Occupied,
}

impl PlaceError {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotBuildable => "not buildable",
            Self::Requires => "requires another building",
            Self::Limit => "limit reached",
            Self::Sites => "too many construction sites",
            Self::NotEnoughResources => "not enough resources",
            Self::OutOfRange => "outside the build radius",
            Self::OutsideMap => "outside the map",
            Self::Blocked => "blocked",
            Self::TooSteep => "too steep",
            Self::Occupied => "units in the way",
        }
    }
}

/// A structure as the presentation and the AI see it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureView {
    pub unit: UnitId,
    pub owner: PlayerId,
    pub unit_type: u32,
    /// Build progress in per mille (1000 = completed).
    pub built_pml: u32,
    /// Unit types in the queue, the one being made first.
    pub queue: Vec<u32>,
    /// Progress of the first order in per mille.
    pub progress_pml: u32,
    pub rally: Vec2,
}

impl StructureView {
    pub const fn completed(&self) -> bool {
        self.built_pml >= 1000
    }
}

/// A supply point as the presentation and the AI see it (public).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyView {
    pub name: String,
    pub position: Vec2,
    pub owner: Option<PlayerId>,
    /// Who is taking it and how far (per mille of one step).
    pub taker: Option<PlayerId>,
    pub progress_pml: u32,
}

/// Something a player can build or produce, with its price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuyOption {
    pub unit_type: u32,
    pub cost: Cost,
    /// `None` if it can be bought now (placement aside), else why not.
    pub blocked: Option<PlaceError>,
}

impl Economy {
    pub(super) fn new(
        rules: EconomyRules,
        config: &super::SimConfig,
        map: &crate::map::Map,
        players: &[PlayerId],
    ) -> Self {
        let catalog = crate::economy::builtin_catalog();
        let types: Vec<TypeEco> = config
            .unit_types
            .iter()
            .map(|t| {
                let entry = catalog.and_then(|c| c.get(&t.name));
                let half_l = if t.combat.length_m > Fixed::ZERO {
                    t.combat.length_m / 2
                } else {
                    t.footprint_m / 2
                };
                let half_w = if t.combat.width_m > Fixed::ZERO {
                    t.combat.width_m / 2
                } else {
                    t.footprint_m / 2
                };
                let mut eco = TypeEco {
                    faction: faction_of(&t.name),
                    half: (half_l, half_w),
                    ..TypeEco::default()
                };
                match rules.structure(&t.name) {
                    Some(StructureRule::Building(b)) => {
                        eco.structure = true;
                        eco.building = rules.buildings.iter().position(|x| x.id == b.id);
                        eco.cost = Some(Cost {
                            resources: b.cost,
                            time_ms: b.build_s.saturating_mul(1000),
                        });
                        eco.hp = b.hp;
                        eco.radius_m = b.build_radius_m;
                        eco.income = b.income_per_min;
                        eco.max_count = b.max_count;
                        eco.produces.clone_from(&b.produces);
                    }
                    Some(StructureRule::Fortification(f)) => {
                        eco.structure = true;
                        eco.hp = f.hp;
                        eco.cost =
                            catalog.and_then(|c| rules.structure_cost(&t.name, c, &t.combat));
                    }
                    None => {
                        eco.cost = entry.map(|e| rules.unit_cost(e, &t.combat));
                    }
                }
                eco
            })
            .collect();
        // Requirements as type indices (needs all types).
        let index_of = |name: &str| {
            config
                .unit_types
                .iter()
                .position(|t| t.name == name)
                .map(|i| i as u32)
        };
        let mut types = types;
        for (t, eco) in config.unit_types.iter().zip(&mut types) {
            let requires: &[String] = match rules.structure(&t.name) {
                Some(StructureRule::Building(b)) => &b.requires,
                Some(StructureRule::Fortification(f)) => &f.requires,
                None => &[],
            };
            // A requirement missing from the match makes the type unbuildable.
            eco.requires = requires
                .iter()
                .map(|r| index_of(r).unwrap_or(u32::MAX))
                .collect();
        }
        let accounts = players
            .iter()
            .map(|&player| Account {
                player,
                parts: rules.account.start.map(|v| u64::from(v) * PARTS),
                faction: config
                    .spawns
                    .iter()
                    .filter(|s| s.owner == player)
                    .find_map(|s| faction_of(&s.unit_type)),
            })
            .collect();
        let supply = map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with(&rules.supply.marker_prefix))
            .map(|m| SupplyPoint {
                name: m.name.clone(),
                position: m.position,
                owner: None,
                taker: None,
                progress: 0,
            })
            .collect();
        Self {
            rules,
            types,
            accounts,
            structures: Vec::new(),
            supply,
            tick_hz: config.tick_hz,
        }
    }

    fn ticks(&self, ms: u32) -> u32 {
        u32::try_from(u64::from(ms) * u64::from(self.tick_hz) / 1000)
            .unwrap_or(u32::MAX)
            .max(1)
    }

    fn account(&self, player: PlayerId) -> Option<&Account> {
        self.accounts.iter().find(|a| a.player == player)
    }

    fn account_mut(&mut self, player: PlayerId) -> Option<&mut Account> {
        self.accounts.iter_mut().find(|a| a.player == player)
    }

    fn can_pay(&self, player: PlayerId, cost: Resources) -> bool {
        self.account(player).is_some_and(|a| {
            a.parts
                .iter()
                .zip(cost)
                .all(|(&have, c)| have >= u64::from(c) * PARTS)
        })
    }

    fn pay(&mut self, player: PlayerId, cost: Resources) -> bool {
        if !self.can_pay(player, cost) {
            return false;
        }
        if let Some(a) = self.account_mut(player) {
            for (have, c) in a.parts.iter_mut().zip(cost) {
                *have -= u64::from(c) * PARTS;
            }
        }
        true
    }

    fn refund(&mut self, player: PlayerId, amount: Resources) {
        if let Some(a) = self.account_mut(player) {
            for (have, c) in a.parts.iter_mut().zip(amount) {
                *have = have.saturating_add(u64::from(c) * PARTS);
            }
        }
    }

    fn structure_index(&self, unit: UnitId) -> Option<usize> {
        self.structures.binary_search_by_key(&unit, |s| s.unit).ok()
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0xEC);
        self.rules.hash_into(h);
        h.write_u64(self.types.len() as u64);
        for t in &self.types {
            match t.cost {
                None => h.write_u8(0),
                Some(c) => {
                    h.write_u8(1);
                    for v in c.resources {
                        h.write_u32(v);
                    }
                    h.write_u32(c.time_ms);
                }
            }
        }
        for a in &self.accounts {
            h.write_u8(a.player.0);
            for v in a.parts {
                h.write_u64(v);
            }
        }
        h.write_u64(self.structures.len() as u64);
        for s in &self.structures {
            h.write_u32(s.unit.0);
            h.write_u32(s.build_left);
            h.write_u32(s.build_total);
            h.write_i64(s.rally.x.to_bits());
            h.write_i64(s.rally.y.to_bits());
            h.write_u64(s.queue.len() as u64);
            for o in &s.queue {
                h.write_u32(o.unit_type);
                h.write_u32(o.left);
                h.write_u32(o.total);
            }
        }
        for p in &self.supply {
            h.write_u8(p.owner.map_or(0xFF, |o| o.0));
            h.write_u8(p.taker.map_or(0xFF, |o| o.0));
            h.write_u32(p.progress);
        }
    }
}

// --- Simulation side ----------------------------------------------------------

impl Sim {
    /// Registers the structures among the starting units: completed, facing
    /// along the y axis (towards the map center), their ground blocked.
    pub(super) fn init_economy(&mut self) {
        for i in 0..self.units.len() {
            let t = self.units[i].unit_type as usize;
            if !self.economy.types[t].structure {
                continue;
            }
            let facing = axis_facing(self.units[i].position);
            let unit = &mut self.units[i];
            unit.facing = facing;
            unit.combat.aim = facing;
            let hp = self.economy.types[t].hp;
            if hp > 0 {
                self.types[t].max_hp = hp;
                self.units[i].hp = hp;
            }
            let rally = self.default_rally(i);
            let unit = &self.units[i];
            self.economy.structures.push(Structure {
                unit: unit.id,
                owner: unit.owner,
                unit_type: unit.unit_type,
                build_left: 0,
                build_total: 0,
                queue: Vec::new(),
                rally,
            });
            self.block_ground(i);
        }
        // Rule hit points of every structure type (also those built later).
        for t in 0..self.types.len() {
            let hp = self.economy.types[t].hp;
            if self.economy.types[t].structure && hp > 0 {
                self.types[t].max_hp = hp;
            }
        }
    }

    /// The axis-aligned footprint of unit `i` (corners).
    fn footprint(&self, unit_type: usize, position: Vec2, facing: Vec2) -> (Vec2, Vec2) {
        let (hl, hw) = self.economy.types[unit_type].half;
        let ext = if facing.x.abs() > facing.y.abs() {
            Vec2::new(hl, hw)
        } else {
            Vec2::new(hw, hl)
        };
        (position - ext, position + ext)
    }

    fn block_ground(&mut self, i: usize) {
        let u = &self.units[i];
        let (a, b) = self.footprint(u.unit_type as usize, u.position, u.facing);
        self.set_path_obstacle(a, b, true);
    }

    fn default_rally(&self, i: usize) -> Vec2 {
        let u = &self.units[i];
        let (hl, _) = self.economy.types[u.unit_type as usize].half;
        u.position
            + u.facing
                .scale(hl + Fixed::from_num(self.economy.rules.production.rally_m))
    }

    /// Whether `player` may place a structure of `unit_type` at `position`
    /// now. The same check runs for the [`crate::Command::Build`] command
    /// and for the preview of the presentation.
    pub fn placement(
        &self,
        player: PlayerId,
        unit_type: u32,
        position: Vec2,
    ) -> Result<(), PlaceError> {
        self.can_build(player, unit_type)?;
        let t = unit_type as usize;
        let facing = axis_facing(position);
        let (a, b) = self.footprint(t, position, facing);
        if !super::within_world(position) || !self.map.contains(a) || !self.map.contains(b) {
            return Err(PlaceError::OutsideMap);
        }
        let in_range = self.economy.structures.iter().any(|s| {
            let eco = &self.economy.types[s.unit_type as usize];
            s.owner == player
                && s.build_left == 0
                && eco.radius_m > 0
                && self.unit(s.unit).is_some_and(Unit::alive)
                && self
                    .unit(s.unit)
                    .is_some_and(|u| within(u.position, position, eco.radius_m))
        });
        if !in_range {
            return Err(PlaceError::OutOfRange);
        }
        let grid = &self.paths.grid;
        let cells = grid.cells_in(a, b);
        for y in cells.y0..=cells.y1 {
            for x in cells.x0..=cells.x1 {
                if !grid.passable(SizeClass::Large, x, y) {
                    return Err(PlaceError::Blocked);
                }
            }
        }
        let heights = [a, b, Vec2::new(a.x, b.y), Vec2::new(b.x, a.y), position]
            .map(|p| self.map.height_at(p));
        let (lo, hi) = heights
            .iter()
            .fold((heights[0], heights[0]), |(lo, hi), &h| {
                (lo.min(h), hi.max(h))
            });
        let side = (b.x - a.x).max(b.y - a.y);
        let grade = Fixed::from_num(self.economy.rules.construction.max_grade_pct);
        if (hi - lo) * 100 > side * grade {
            return Err(PlaceError::TooSteep);
        }
        let clear = Fixed::from_num(self.economy.rules.construction.clearance_m);
        let blocked = self.units.iter().any(|u| {
            let r = clear + u.spread / 2;
            u.alive()
                && u.position.x >= a.x - r
                && u.position.x <= b.x + r
                && u.position.y >= a.y - r
                && u.position.y <= b.y + r
        });
        if blocked {
            return Err(PlaceError::Occupied);
        }
        Ok(())
    }

    /// The checks of [`Self::placement`] that do not depend on the spot.
    fn can_build(&self, player: PlayerId, unit_type: u32) -> Result<(), PlaceError> {
        let eco = &self.economy;
        let Some(t) = eco.types.get(unit_type as usize) else {
            return Err(PlaceError::NotBuildable);
        };
        let account = eco.account(player).ok_or(PlaceError::NotBuildable)?;
        let faction_ok = t.faction.is_none() || t.faction == account.faction;
        let defeated = self
            .standings()
            .iter()
            .any(|s| s.player == player && s.defeat.is_some());
        let Some(cost) = t.cost.filter(|_| t.structure && faction_ok && !defeated) else {
            return Err(PlaceError::NotBuildable);
        };
        let own = || eco.structures.iter().filter(|s| s.owner == player);
        let alive = |s: &&Structure| self.unit(s.unit).is_some_and(Unit::alive);
        for &r in &t.requires {
            let done = own()
                .filter(alive)
                .any(|s| s.unit_type == r && s.build_left == 0);
            if !done {
                return Err(PlaceError::Requires);
            }
        }
        if t.max_count > 0
            && own()
                .filter(alive)
                .filter(|s| s.unit_type == unit_type)
                .count()
                >= t.max_count as usize
        {
            return Err(PlaceError::Limit);
        }
        let sites = own().filter(alive).filter(|s| s.build_left > 0).count();
        if sites >= eco.rules.construction.max_sites as usize {
            return Err(PlaceError::Sites);
        }
        if !eco.can_pay(player, cost.resources) {
            return Err(PlaceError::NotEnoughResources);
        }
        Ok(())
    }

    fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units
            .binary_search_by_key(&id, |u| u.id)
            .ok()
            .map(|i| &self.units[i])
    }

    /// [`crate::Command::Build`].
    pub(super) fn order_build(&mut self, player: PlayerId, unit_type: u32, position: Vec2) {
        if self.placement(player, unit_type, position).is_err() {
            return;
        }
        let t = unit_type as usize;
        let Some(cost) = self.economy.types[t].cost else {
            return;
        };
        if !self.economy.pay(player, cost.resources) {
            return;
        }
        let facing = axis_facing(position);
        let max_hp = self.types[t].max_hp;
        let start = max_hp * self.economy.rules.construction.start_hp_pct / 100;
        let index = self.spawn_unit(player, unit_type, position, facing, FireMode::HoldFire);
        self.units[index].hp = start.max(1);
        let total = self.economy.ticks(cost.time_ms);
        let rally = self.default_rally(index);
        let unit = &self.units[index];
        self.economy.structures.push(Structure {
            unit: unit.id,
            owner: player,
            unit_type,
            build_left: total,
            build_total: total,
            queue: Vec::new(),
            rally,
        });
        self.block_ground(index);
    }

    /// Adds a unit during the match (production, construction sites). Ids
    /// keep increasing, so the unit list stays sorted. Returns its index.
    pub(super) fn spawn_unit(
        &mut self,
        owner: PlayerId,
        unit_type: u32,
        position: Vec2,
        facing: Vec2,
        fire_mode: FireMode,
    ) -> usize {
        let id = UnitId(self.units.last().map_or(0, |u| u.id.0 + 1));
        let t = unit_type as usize;
        let unit = new_unit(&NewUnit {
            id,
            owner,
            type_index: unit_type,
            unit_type: &self.unit_types[t],
            position,
            facing,
            fire_mode,
            hp: self.types[t].max_hp,
            soldier_hp: self.combat_rules.damage.soldier_hp,
            infantry: &self.infantry,
            map: &self.map,
            tick_hz: self.tick_hz,
        });
        self.units.push(unit);
        self.vision.push_unit();
        self.units.len() - 1
    }

    /// [`crate::Command::Produce`].
    pub(super) fn order_produce(&mut self, player: PlayerId, building: UnitId, unit_type: u32) {
        let Some(s) = self.economy.structure_index(building) else {
            return;
        };
        let st = &self.economy.structures[s];
        if st.owner != player || st.build_left > 0 || !self.unit(building).is_some_and(Unit::alive)
        {
            return;
        }
        if !self.producible(st.unit_type, player).contains(&unit_type)
            || st.queue.len() >= self.economy.rules.production.max_queue as usize
        {
            return;
        }
        let Some(cost) = self.economy.types[unit_type as usize].cost else {
            return;
        };
        if !self.economy.pay(player, cost.resources) {
            return;
        }
        let total = self.economy.ticks(cost.time_ms);
        self.economy.structures[s].queue.push(Order {
            unit_type,
            left: total,
            total,
            paid: cost.resources,
        });
    }

    /// [`crate::Command::CancelProduction`]: the last order, fully refunded.
    pub(super) fn order_cancel(&mut self, player: PlayerId, building: UnitId) {
        let Some(s) = self.economy.structure_index(building) else {
            return;
        };
        if self.economy.structures[s].owner != player {
            return;
        }
        if let Some(order) = self.economy.structures[s].queue.pop() {
            self.economy.refund(player, order.paid);
        }
    }

    /// [`crate::Command::SetRallyPoint`].
    pub(super) fn order_rally(&mut self, player: PlayerId, building: UnitId, point: Vec2) {
        if !super::within_world(point) {
            return;
        }
        if let Some(s) = self.economy.structure_index(building)
            && self.economy.structures[s].owner == player
        {
            self.economy.structures[s].rally = point;
        }
    }

    /// Unit types a structure of `building_type` makes for `player`.
    fn producible(&self, building_type: u32, player: PlayerId) -> Vec<u32> {
        let eco = &self.economy;
        let Some(t) = eco.types.get(building_type as usize) else {
            return Vec::new();
        };
        let Some(faction) = eco.account(player).and_then(|a| a.faction) else {
            return Vec::new();
        };
        eco.rules
            .produced(&t.produces, faction)
            .iter()
            .filter_map(|id| self.unit_types.iter().position(|u| &u.name == id))
            .map(|i| i as u32)
            .collect()
    }

    /// Construction, income, supply points and production of one tick
    /// (after destruction, before vision).
    pub(super) fn update_economy(&mut self) {
        // Destroyed structures lose their queue.
        let units = &self.units;
        self.economy.structures.retain(|s| {
            units
                .binary_search_by_key(&s.unit, |u| u.id)
                .is_ok_and(|i| units[i].alive())
        });
        self.build_sites();
        self.take_supply_points();
        if (self.tick + 1).is_multiple_of(u64::from(self.tick_hz)) {
            self.pay_income();
        }
        self.produce();
    }

    fn build_sites(&mut self) {
        let start_pct = self.economy.rules.construction.start_hp_pct;
        for s in 0..self.economy.structures.len() {
            let st = &self.economy.structures[s];
            let Ok(i) = self.units.binary_search_by_key(&st.unit, |u| u.id) else {
                continue;
            };
            let unit = &mut self.units[i];
            if st.build_left == 0 {
                continue;
            }
            // A site does not fire.
            unit.combat.fire_mode = FireMode::HoldFire;
            unit.combat.auto_target = None;
            let max = u64::from(self.types[st.unit_type as usize].max_hp);
            let gain = max * u64::from(100 - start_pct) / 100;
            let total = u64::from(st.build_total.max(1));
            let done = u64::from(st.build_total - st.build_left);
            let add = gain * (done + 1) / total - gain * done / total;
            unit.hp = u32::try_from((u64::from(unit.hp) + add).min(max)).unwrap_or(u32::MAX);
            let st = &mut self.economy.structures[s];
            st.build_left -= 1;
            if st.build_left == 0 {
                unit.combat.fire_mode = FireMode::FireAtWill;
            }
        }
    }

    fn pay_income(&mut self) {
        let eco = &self.economy;
        let mut income: Vec<(PlayerId, Resources)> = Vec::new();
        let mut add = |player: PlayerId, r: Resources| {
            if let Some(e) = income.iter_mut().find(|(p, _)| *p == player) {
                for (a, b) in e.1.iter_mut().zip(r) {
                    *a = a.saturating_add(b);
                }
            } else {
                income.push((player, r));
            }
        };
        for s in eco.structures.iter().filter(|s| s.build_left == 0) {
            add(s.owner, eco.types[s.unit_type as usize].income);
        }
        for p in &eco.supply {
            if let Some(owner) = p.owner {
                add(owner, eco.rules.supply.income_per_min);
            }
        }
        // An income per minute pays 1/60 every second: PARTS / 60 parts per
        // unit and second.
        for (player, r) in income {
            if let Some(a) = self.economy.account_mut(player) {
                for (have, per_min) in a.parts.iter_mut().zip(r) {
                    *have = have.saturating_add(u64::from(per_min) * (PARTS / 60));
                }
            }
        }
    }

    fn take_supply_points(&mut self) {
        let radius = Fixed::from_num(self.economy.rules.supply.radius_m);
        let steps = self
            .economy
            .ticks(self.economy.rules.supply.capture_s.saturating_mul(1000));
        for k in 0..self.economy.supply.len() {
            let at = self.economy.supply[k].position;
            let mut present: Option<PlayerId> = None;
            let mut contested = false;
            for u in &self.units {
                if !u.alive()
                    || self.economy.types[u.unit_type as usize].structure
                    || !within_fixed(u.position, at, radius)
                {
                    continue;
                }
                // Allies (prompt 037) do not contest each other; the first
                // of them present takes the point.
                match present {
                    None => present = Some(u.owner),
                    Some(p) if self.teams.hostile(p, u.owner) => contested = true,
                    Some(_) => {}
                }
            }
            let p = &mut self.economy.supply[k];
            match present {
                None => {
                    p.taker = None;
                    p.progress = 0;
                }
                Some(_) if contested => {}
                Some(player) if p.owner.is_some_and(|o| self.teams.allied(o, player)) => {
                    p.taker = None;
                    p.progress = 0;
                }
                Some(player) => {
                    if p.taker != Some(player) {
                        p.taker = Some(player);
                        p.progress = 0;
                    }
                    p.progress += 1;
                    if p.progress >= steps {
                        // An enemy point is neutralized first.
                        p.owner = if p.owner.is_some() {
                            None
                        } else {
                            Some(player)
                        };
                        p.progress = 0;
                    }
                }
            }
        }
    }

    fn produce(&mut self) {
        let max_units = self.economy.rules.production.max_units as usize;
        for s in 0..self.economy.structures.len() {
            let st = &self.economy.structures[s];
            if st.build_left > 0 || st.queue.is_empty() {
                continue;
            }
            let owner = st.owner;
            let living = self
                .units
                .iter()
                .filter(|u| {
                    u.owner == owner
                        && u.alive()
                        && !self.economy.types[u.unit_type as usize].structure
                })
                .count();
            if living >= max_units {
                continue;
            }
            let order = &mut self.economy.structures[s].queue[0];
            order.left = order.left.saturating_sub(1);
            if order.left > 0 {
                continue;
            }
            let unit_type = order.unit_type;
            let st = &self.economy.structures[s];
            let (building, rally) = (st.unit, st.rally);
            let Some(spot) = self.exit_spot(building, unit_type) else {
                continue; // Wait until there is room.
            };
            self.economy.structures[s].queue.remove(0);
            let facing = direction(spot.0, rally).unwrap_or(spot.1);
            let index = self.spawn_unit(owner, unit_type, spot.0, facing, FireMode::FireAtWill);
            // Units gather on a 3 × 3 formation around the rally point, by id.
            let unit = &self.units[index];
            let slots = super::formation_slots(rally, 9, super::formation_spacing(unit.spread));
            let slot = slots[(unit.id.0 % 9) as usize];
            if self.map.contains(slot) {
                self.request_group_path(&[(index, slot)], slot);
            }
        }
    }

    /// A free spot beside `building` where a unit of `unit_type` can stand,
    /// and the direction away from the building there. The search order is
    /// fixed: rings outwards, directions from the building's front around.
    fn exit_spot(&self, building: UnitId, unit_type: u32) -> Option<(Vec2, Vec2)> {
        let b = self.unit(building)?;
        let (hl, hw) = self.economy.types[b.unit_type as usize].half;
        let size = self.unit_types[unit_type as usize].footprint_m;
        let base_deg = deg_of(b.facing);
        let grid = &self.paths.grid;
        for ring in 0..3 {
            let r = hl.max(hw) + size + Fixed::from_num(3 + 8 * ring);
            for k in 0..24u16 {
                // 0, +15, -15, +30, ... from the front.
                let step = k.div_ceil(2) * 15;
                let deg = if k % 2 == 1 {
                    base_deg + step
                } else {
                    base_deg + 360 - step
                } % 360;
                let (sin, cos) = crate::objects::sin_cos_deg(deg);
                let dir = Vec2::new(sin, -cos);
                let p = b.position + dir.scale(r);
                if !self.map.contains(p) || !grid.passable_for(unit_type as usize, p) {
                    continue;
                }
                let free = self
                    .units
                    .iter()
                    .all(|u| !u.alive() || !within_fixed(u.position, p, (u.spread + size) / 2));
                if free {
                    return Some((p, dir));
                }
            }
        }
        None
    }

    // --- Queries ----------------------------------------------------------------

    /// Resources of `player` (whole units, rounded down).
    pub fn account(&self, player: PlayerId) -> Option<Resources> {
        self.economy.account(player).map(|a| {
            a.parts
                .map(|p| u32::try_from(p / PARTS).unwrap_or(u32::MAX))
        })
    }

    /// Income of `player` per minute now (completed buildings, held points).
    pub fn income(&self, player: PlayerId) -> Resources {
        let eco = &self.economy;
        let mut total = [0u32; 3];
        let mut add = |r: Resources| {
            for (a, b) in total.iter_mut().zip(r) {
                *a = a.saturating_add(b);
            }
        };
        for s in eco
            .structures
            .iter()
            .filter(|s| s.owner == player && s.build_left == 0)
        {
            add(eco.types[s.unit_type as usize].income);
        }
        for _ in eco.supply.iter().filter(|p| p.owner == Some(player)) {
            add(eco.rules.supply.income_per_min);
        }
        total
    }

    /// Faction of `player` (`de`, `su`, `us`), from their first catalog unit.
    pub fn faction(&self, player: PlayerId) -> Option<&'static str> {
        self.economy.account(player).and_then(|a| a.faction)
    }

    /// Every structure of every player (by unit id). Ownership and build
    /// state are visible on the field; the presentation filters by vision.
    pub fn structures(&self) -> Vec<StructureView> {
        self.economy
            .structures
            .iter()
            .map(|s| StructureView {
                unit: s.unit,
                owner: s.owner,
                unit_type: s.unit_type,
                built_pml: if s.build_left == 0 {
                    1000
                } else {
                    pml(s.build_total - s.build_left, s.build_total)
                },
                queue: s.queue.iter().map(|o| o.unit_type).collect(),
                progress_pml: s
                    .queue
                    .first()
                    .map_or(0, |o| pml(o.total - o.left, o.total)),
                rally: s.rally,
            })
            .collect()
    }

    /// The supply points of the map.
    pub fn supply_points(&self) -> Vec<SupplyView> {
        let steps = self
            .economy
            .ticks(self.economy.rules.supply.capture_s.saturating_mul(1000));
        self.economy
            .supply
            .iter()
            .map(|p| SupplyView {
                name: p.name.clone(),
                position: p.position,
                owner: p.owner,
                taker: p.taker,
                progress_pml: pml(p.progress, steps),
            })
            .collect()
    }

    /// Index of the unit type named `name` in `SimConfig::unit_types`.
    pub fn unit_type_index(&self, name: &str) -> Option<u32> {
        self.unit_types
            .iter()
            .position(|t| t.name == name)
            .map(|i| i as u32)
    }

    /// Position of a unit (`None` if it does not exist).
    pub fn unit_position(&self, id: UnitId) -> Option<Vec2> {
        self.unit(id).map(|u| u.position)
    }

    /// Whether a unit type is a structure (building or fortification).
    pub fn is_structure(&self, unit_type: u32) -> bool {
        self.economy
            .types
            .get(unit_type as usize)
            .is_some_and(|t| t.structure)
    }

    /// Price and time of a unit type (`None`: it cannot be bought).
    pub fn cost_of(&self, unit_type: u32) -> Option<Cost> {
        self.economy.types.get(unit_type as usize)?.cost
    }

    /// Footprint of a structure type as (length along its facing, width) in
    /// meters.
    pub fn structure_size(&self, unit_type: u32) -> Option<(Fixed, Fixed)> {
        let t = self.economy.types.get(unit_type as usize)?;
        t.structure.then_some((t.half.0 * 2, t.half.1 * 2))
    }

    /// The structures `player` can build, with price and whether something
    /// keeps them from building it now.
    pub fn build_options(&self, player: PlayerId) -> Vec<BuyOption> {
        let account = self.economy.account(player);
        (0u32..)
            .zip(&self.economy.types)
            .filter(|(_, t)| {
                t.structure
                    && t.cost.is_some()
                    && (t.faction.is_none() || t.faction == account.and_then(|a| a.faction))
            })
            .map(|(i, t)| BuyOption {
                unit_type: i,
                cost: t.cost.unwrap_or_default(),
                blocked: self.can_build(player, i).err(),
            })
            .collect()
    }

    /// What `building` makes for its owner, with price and whether the
    /// owner can order it now (`NotEnoughResources`, `Limit` for a full
    /// queue, `Sites` while it is a construction site).
    pub fn production_options(&self, building: UnitId) -> Vec<BuyOption> {
        let Some(s) = self.economy.structure_index(building) else {
            return Vec::new();
        };
        let st = &self.economy.structures[s];
        let full = st.queue.len() >= self.economy.rules.production.max_queue as usize;
        self.producible(st.unit_type, st.owner)
            .into_iter()
            .filter_map(|t| {
                let cost = self.economy.types[t as usize].cost?;
                let blocked = if st.build_left > 0 {
                    Some(PlaceError::Sites)
                } else if full {
                    Some(PlaceError::Limit)
                } else if !self.economy.can_pay(st.owner, cost.resources) {
                    Some(PlaceError::NotEnoughResources)
                } else {
                    None
                };
                Some(BuyOption {
                    unit_type: t,
                    cost,
                    blocked,
                })
            })
            .collect()
    }

    /// The first spot (if any) around `near` where `player` may place
    /// `unit_type`, searched on rings of growing radius in a fixed order
    /// (for the computer player, prompt 023).
    pub fn find_site(&self, player: PlayerId, unit_type: u32, near: Vec2) -> Option<Vec2> {
        self.can_build(player, unit_type).ok()?;
        let (hl, hw) = self.economy.types[unit_type as usize].half;
        let step = (hl.max(hw) * 2 + Fixed::from_num(6)).max(Fixed::from_num(8));
        for ring in 1..=8 {
            let r = step * ring;
            let count = 8 * ring as u16;
            for k in 0..count {
                let deg = k * (360 / count);
                let (sin, cos) = crate::objects::sin_cos_deg(deg);
                let p = near + Vec2::new(sin, cos).scale(r);
                let p = Vec2::from_millimeters(to_mm(p.x), to_mm(p.y));
                if self.placement(player, unit_type, p).is_ok() {
                    return Some(p);
                }
            }
        }
        None
    }
}

/// Facing of a structure: along the y axis, towards the map center.
fn axis_facing(position: Vec2) -> Vec2 {
    if position.y > Fixed::ZERO {
        Vec2::new(Fixed::ZERO, -Fixed::ONE)
    } else {
        Vec2::new(Fixed::ZERO, Fixed::ONE)
    }
}

/// Compass degrees of a direction (0 = -y, 90 = +x), for the axis facings
/// of structures (exact) and roughly for others.
fn deg_of(d: Vec2) -> u16 {
    if d.y.abs() >= d.x.abs() {
        if d.y <= Fixed::ZERO { 0 } else { 180 }
    } else if d.x > Fixed::ZERO {
        90
    } else {
        270
    }
}

fn direction(from: Vec2, to: Vec2) -> Option<Vec2> {
    let d = to - from;
    let len = d.length();
    (len > Fixed::ZERO).then(|| Vec2::new(d.x / len, d.y / len))
}

fn within(a: Vec2, b: Vec2, radius_m: u32) -> bool {
    within_fixed(a, b, Fixed::from_num(radius_m))
}

fn within_fixed(a: Vec2, b: Vec2, r: Fixed) -> bool {
    let d = a - b;
    d.x.abs() <= r && d.y.abs() <= r && d.length() <= r
}

fn pml(done: u32, total: u32) -> u32 {
    if total == 0 {
        1000
    } else {
        u32::try_from(u64::from(done) * 1000 / u64::from(total)).unwrap_or(1000)
    }
}

fn to_mm(v: Fixed) -> i32 {
    (v * Fixed::from_num(1000)).round().to_num::<i32>()
}

/// Serde for the faction ids (`&'static str` from [`faction_of`]) in
/// snapshots (prompt 038).
mod static_faction {
    use super::faction_of;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[allow(clippy::ref_option)] // the signature serde's `with` expects
    pub fn serialize<S: Serializer>(
        faction: &Option<&'static str>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        faction.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<&'static str>, D::Error> {
        let id: Option<String> = Option::deserialize(d)?;
        match id {
            None => Ok(None),
            Some(id) => faction_of(&format!("{id}_"))
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom(format!("unknown faction '{id}'"))),
        }
    }
}
