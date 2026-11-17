//! Heavy rocket strike (prompt 054): calls, the warning time in which the
//! enemy can destroy the battery position, and the salvo. Rules and public
//! types: `crate::strike`.
//!
//! Order within a tick (see [`Sim::step`]): commands call strikes;
//! [`Sim::update_strikes`] runs after the indirect fire (cooldowns, warning,
//! cancelling, launching rockets as shells that `fly_shells` moves from
//! the next tick on); [`Sim::reveal_strikes`] runs after the vision update
//! and shows every enemy the battery positions of called strikes.

use super::{Sim, within_world};
use crate::combat::Vec3;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::strike::{StrikeBlock, StrikeEvent, StrikeOption, StrikeRules, StrikeView};
use serde::{Deserialize, Serialize};

/// A called strike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Strike {
    id: u32,
    owner: PlayerId,
    battery: UnitId,
    target: Vec2,
    caliber_mm_x10: u32,
    /// Ticks until the first rocket leaves.
    warning_left: u32,
    rockets_left: u32,
    /// Ticks until the next rocket leaves (while firing).
    next_rocket: u32,
}

/// Heavy strikes of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Strikes {
    rules: StrikeRules,
    /// Unit type index of the battery position, if the match has it.
    battery_type: Option<u32>,
    /// Players in slot order and their cooldowns (ticks).
    players: Vec<PlayerId>,
    cooldowns: Vec<u32>,
    active: Vec<Strike>,
    next_id: u32,
    tick_hz: u32,
    /// Events of the last tick (not state: derived, not hashed).
    pub(super) events: Vec<StrikeEvent>,
}

impl Strikes {
    pub(super) fn new(rules: StrikeRules, config: &super::SimConfig) -> Self {
        let players = config.players();
        let battery_type = config
            .unit_types
            .iter()
            .position(|t| t.name == rules.building)
            .and_then(|i| u32::try_from(i).ok());
        Self {
            battery_type,
            cooldowns: vec![0; players.len()],
            players,
            active: Vec::new(),
            next_id: 0,
            tick_hz: config.tick_hz.max(1),
            events: Vec::new(),
            rules,
        }
    }

    fn slot(&self, player: PlayerId) -> Option<usize> {
        self.players.binary_search(&player).ok()
    }

    fn ticks(&self, ms: u32) -> u32 {
        u32::try_from(u64::from(ms) * u64::from(self.tick_hz) / 1000).unwrap_or(u32::MAX)
    }

    fn cooldown_ticks(&self) -> u32 {
        self.rules.cooldown_s.saturating_mul(self.tick_hz)
    }

    fn warning_ticks(&self) -> u32 {
        self.rules.warning_s.saturating_mul(self.tick_hz)
    }

    fn unlock_tick(&self) -> u64 {
        u64::from(self.rules.unlock_s) * u64::from(self.tick_hz)
    }

    /// Whether any strike was ever called: only then is the state part of
    /// the hash, so matches without strikes keep their hashes.
    pub(super) const fn used(&self) -> bool {
        self.next_id > 0
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u32(self.next_id);
        for &c in &self.cooldowns {
            h.write_u32(c);
        }
        h.write_u64(self.active.len() as u64);
        for s in &self.active {
            h.write_u32(s.id);
            h.write_u8(s.owner.0);
            h.write_u32(s.battery.0);
            h.write_i64(s.target.x.to_bits());
            h.write_i64(s.target.y.to_bits());
            h.write_u32(s.caliber_mm_x10);
            h.write_u32(s.warning_left);
            h.write_u32(s.rockets_left);
            h.write_u32(s.next_rocket);
        }
    }
}

impl Sim {
    /// The completed own battery position of `player` that is still
    /// standing (the lowest id if there are several).
    fn battery_of(&self, player: PlayerId) -> Option<UnitId> {
        let t = self.strikes.battery_type?;
        self.units
            .iter()
            .find(|u| {
                u.unit_type == t
                    && u.owner == player
                    && u.alive()
                    && !self.sold(u.id)
                    && self.economy.completed_owner(u.id) == Some(player)
            })
            .map(|u| u.id)
    }

    /// Whether the battery `id` of `owner` still stands.
    fn battery_stands(&self, id: UnitId, owner: PlayerId) -> Option<usize> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        let u = &self.units[i];
        (u.alive() && u.owner == owner && !self.sold(id)).then_some(i)
    }

    /// Why `player` cannot call a strike now (`None`: it can).
    fn strike_block(&self, player: PlayerId) -> Option<StrikeBlock> {
        let Some(slot) = self.strikes.slot(player) else {
            return Some(StrikeBlock::Defeated);
        };
        if self.alive_units(player) == 0
            || self
                .standings()
                .iter()
                .any(|s| s.player == player && s.defeat.is_some())
        {
            return Some(StrikeBlock::Defeated);
        }
        if self.strikes.rules.caliber(self.faction(player)).is_none() {
            return Some(StrikeBlock::NoRockets);
        }
        if self.battery_of(player).is_none() {
            return Some(StrikeBlock::NoBattery);
        }
        if self.tick < self.strikes.unlock_tick() {
            return Some(StrikeBlock::Locked);
        }
        if self.strikes.active.iter().any(|s| s.owner == player) {
            return Some(StrikeBlock::Busy);
        }
        if self.strikes.cooldowns[slot] > 0 {
            return Some(StrikeBlock::Cooldown);
        }
        if !self.economy.can_pay(player, self.strikes.rules.cost) {
            return Some(StrikeBlock::NotEnoughResources);
        }
        None
    }

    /// Calls a strike of `player` on `target` if it can.
    pub(super) fn order_strike(&mut self, player: PlayerId, target: Vec2) {
        if !within_world(target) || !self.map.contains(target) {
            return;
        }
        if self.strike_block(player).is_some() {
            return;
        }
        let (Some(slot), Some(battery), Some(caliber)) = (
            self.strikes.slot(player),
            self.battery_of(player),
            self.strikes.rules.caliber(self.faction(player)),
        ) else {
            return;
        };
        if !self.economy.pay(player, self.strikes.rules.cost) {
            return;
        }
        let s = &mut self.strikes;
        s.cooldowns[slot] = s.cooldown_ticks();
        let id = s.next_id;
        s.next_id += 1;
        let warning = s.warning_ticks();
        s.active.push(Strike {
            id,
            owner: player,
            battery,
            target,
            caliber_mm_x10: caliber,
            warning_left: warning,
            rockets_left: s.rules.rockets,
            next_rocket: 0,
        });
        let from = self.unit_position(battery).unwrap_or(target);
        self.strikes.events.push(StrikeEvent::Called {
            strike: id,
            owner: player,
            battery,
            from,
            target,
            radius: Fixed::from_num(self.strikes.rules.radius_m),
            warning_ticks: warning,
        });
    }

    /// Cooldowns, warning time, cancelled strikes and the rockets of this
    /// tick.
    pub(super) fn update_strikes(&mut self) {
        for c in &mut self.strikes.cooldowns {
            *c = c.saturating_sub(1);
        }
        if self.strikes.active.is_empty() {
            return;
        }
        let strikes = std::mem::take(&mut self.strikes.active);
        let mut kept = Vec::with_capacity(strikes.len());
        for mut s in strikes {
            let Some(i) = self.battery_stands(s.battery, s.owner) else {
                // Lost before or during the salvo: the rest is not fired.
                if s.warning_left > 0 || s.rockets_left == self.strikes.rules.rockets {
                    self.strikes.events.push(StrikeEvent::Cancelled {
                        strike: s.id,
                        owner: s.owner,
                        target: s.target,
                    });
                }
                continue;
            };
            if s.warning_left > 0 {
                s.warning_left -= 1;
                kept.push(s);
                continue;
            }
            if s.next_rocket > 0 {
                s.next_rocket -= 1;
            } else {
                if s.rockets_left == self.strikes.rules.rockets {
                    self.strikes.events.push(StrikeEvent::Launched {
                        strike: s.id,
                        owner: s.owner,
                        from: self.units[i].position,
                        target: s.target,
                    });
                }
                self.launch_rocket(i, &s);
                s.rockets_left -= 1;
                s.next_rocket = self.strikes.ticks(self.strikes.rules.ripple_ms);
            }
            if s.rockets_left > 0 {
                kept.push(s);
            }
        }
        self.strikes.active = kept;
    }

    /// One rocket from the battery (unit index `i`) to a random point within
    /// the radius of the strike's target.
    fn launch_rocket(&mut self, i: usize, s: &Strike) {
        let rules = &self.strikes.rules;
        let radius = Fixed::from_num(rules.radius_m);
        let mut aim = s.target;
        // Uniform in the circle: rejection sampling on the square (like a
        // barrage, prompt 032).
        for _ in 0..8 {
            let dx = Fixed::from_num(self.rng.range_i32(-1000, 1000)) / 1000;
            let dy = Fixed::from_num(self.rng.range_i32(-1000, 1000)) / 1000;
            if dx * dx + dy * dy <= Fixed::ONE {
                aim = s.target + Vec2::new(dx * radius, dy * radius);
                break;
            }
        }
        if !self.map.contains(aim) {
            aim = s.target;
        }
        self.fire_rocket(i, s.owner, aim, s.caliber_mm_x10);
    }

    /// Debug and tools only (prompt 076: salvo pictures and the effects
    /// bench): one rocket of `caliber_mm_x10` from unit `shooter` to `aim`
    /// now, flying like a strike rocket. Never called by commands or the
    /// AI; changes the state like a shell would. False if the unit is gone
    /// or the point is off the map.
    pub fn debug_rocket(&mut self, shooter: UnitId, aim: Vec2, caliber_mm_x10: u32) -> bool {
        let Ok(i) = self.units.binary_search_by_key(&shooter, |u| u.id) else {
            return false;
        };
        if !self.units[i].alive() || !self.map.contains(aim) {
            return false;
        }
        let owner = self.units[i].owner;
        self.fire_rocket(i, owner, aim, caliber_mm_x10);
        true
    }

    /// Puts one rocket from unit `i` to `aim` in the air (speed, arc and
    /// minimum flight from the strike rules).
    fn fire_rocket(&mut self, i: usize, owner: PlayerId, aim: Vec2, caliber: u32) {
        let rules = &self.strikes.rules;
        let unit = &self.units[i];
        let origin = Vec3::new(
            unit.position.x,
            unit.position.y,
            unit.height + Fixed::from_num(3) / 2,
        );
        let target = Vec3::new(aim.x, aim.y, self.map.height_at(aim));
        let distance = (aim - unit.position).length();
        let seconds = (distance / Fixed::from_num(rules.speed_m_per_s))
            .max(Fixed::from_num(rules.min_flight_ms) / 1000);
        let total = (seconds * Fixed::from_num(self.strikes.tick_hz))
            .round()
            .to_num::<u32>();
        let apex = distance * Fixed::from_num(rules.apex_pct) / 100;
        let shooter = unit.id;
        self.launch_shell(shooter, owner, origin, target, apex, total, caliber);
    }

    /// After the vision update: every enemy of a strike's owner sees its
    /// battery position until the salvo is fired.
    pub(super) fn reveal_strikes(&mut self) {
        if self.strikes.active.is_empty() {
            return;
        }
        for k in 0..self.strikes.active.len() {
            let s = self.strikes.active[k];
            let Some(i) = self.battery_stands(s.battery, s.owner) else {
                continue;
            };
            for p in 0..self.strikes.players.len() {
                let player = self.strikes.players[p];
                if !self.teams.allied(player, s.owner) {
                    self.vision.reveal(i, player);
                }
            }
        }
    }

    // --- Queries --------------------------------------------------------------

    /// Whether and when `player` can call a strike.
    pub fn strike_option(&self, player: PlayerId) -> StrikeOption {
        let s = &self.strikes;
        StrikeOption {
            cost: s.rules.cost,
            cooldown_left: s.slot(player).map_or(0, |k| s.cooldowns[k]),
            cooldown_total: s.cooldown_ticks(),
            unlock_left: u32::try_from(s.unlock_tick().saturating_sub(self.tick))
                .unwrap_or(u32::MAX),
            battery: self.battery_of(player),
            rockets: s.rules.rockets,
            radius_m: s.rules.radius_m,
            warning_ticks: s.warning_ticks(),
            blocked: self.strike_block(player),
        }
    }

    /// Every called strike that has rockets left (public: smoke, siren and
    /// the revealed position).
    pub fn strikes(&self) -> Vec<StrikeView> {
        let radius = Fixed::from_num(self.strikes.rules.radius_m);
        self.strikes
            .active
            .iter()
            .map(|s| StrikeView {
                id: s.id,
                owner: s.owner,
                battery: s.battery,
                from: self.unit_position(s.battery).unwrap_or(s.target),
                target: s.target,
                radius,
                warning_left: s.warning_left,
                rockets_left: s.rockets_left,
            })
            .collect()
    }

    /// Strike and big-blast events of the last tick.
    pub fn strike_events(&self) -> &[StrikeEvent] {
        &self.strikes.events
    }

    /// The strike rules of the match.
    pub const fn strike_rules(&self) -> &StrikeRules {
        &self.strikes.rules
    }
}
