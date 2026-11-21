//! Fire discipline and unarmed vehicles under fire (prompt 176). Rules:
//! `[discipline]` in `data/tactics.toml` (`crate::tactics::DisciplineRules`).
//!
//! - **Fire modes** (`FireMode`): fire at will, return fire (automatic
//!   targets only among units that fired at this one: hits on it and rounds
//!   landing within `return_near_mm`, remembered `return_remember_ms` after
//!   the last round, [`Sim::note_fire`]), hold fire.
//! - **Limits** (`Command::SetFireLimits`, [`FireLimits`]): close range only
//!   (automatic targets within `close_range_pct` of the longest range) and
//!   vehicles only (no automatic infantry targets, also for the machine
//!   guns). Attack orders are never limited, and a target once taken is
//!   kept while it stays in sight and range.
//! - **Unarmed vehicles** without an order that are hit or nearly hit drive
//!   `retreat_mm` away from where the fire came from ([`Sim::unarmed_under_fire`]).
//!
//! State: the limits (in `CombatState`), the remembered attackers and the
//! retreat cooldown per unit; all hashed, written only when not empty so a
//! match that uses none of it hashes as before.

use super::{Sim, stop};
use crate::combat::FireMode;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};

/// Limits of automatic fire (prompt 176).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireLimits {
    /// Only targets within `close_range_pct` of the longest range.
    pub close_only: bool,
    /// No automatic infantry targets.
    pub vehicles_only: bool,
}

impl FireLimits {
    pub const fn is_empty(self) -> bool {
        !self.close_only && !self.vehicles_only
    }

    pub(super) fn hash_into(self, h: &mut StableHasher) {
        if !self.is_empty() {
            h.write_u8(0xd0 | u8::from(self.close_only) | (u8::from(self.vehicles_only) << 1));
        }
    }
}

/// `[discipline]` in ticks and game units, built once. Default: off
/// (saved games from before prompt 176).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DisciplineRt {
    near: Fixed,
    remember: u32,
    close_pct: u32,
    retreat: Fixed,
    cooldown: u32,
}

impl Default for DisciplineRt {
    fn default() -> Self {
        Self {
            near: Fixed::ZERO,
            remember: 0,
            close_pct: 100,
            retreat: Fixed::ZERO,
            cooldown: 0,
        }
    }
}

impl DisciplineRt {
    pub(super) fn new(rules: &crate::tactics::DisciplineRules, ticks: impl Fn(u32) -> u32) -> Self {
        Self {
            near: Fixed::from_num(rules.return_near_mm) / 1000,
            remember: ticks(rules.return_remember_ms),
            close_pct: rules.close_range_pct.clamp(1, 100),
            retreat: if rules.unarmed_retreat > 0 {
                Fixed::from_num(rules.retreat_mm) / 1000
            } else {
                Fixed::ZERO
            },
            cooldown: ticks(rules.retreat_cooldown_ms),
        }
    }
}

/// Who fired at a unit lately (return fire) and its retreat cooldown.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct UnderFire {
    /// Shooters with the ticks they are still remembered, sorted by id.
    attackers: Vec<(UnitId, u32)>,
    /// Ticks until an unarmed vehicle may retreat again.
    cooldown: u32,
}

/// Shooters remembered per unit at most.
const MAX_ATTACKERS: usize = 8;

impl UnderFire {
    pub(super) fn is_empty(&self) -> bool {
        self.attackers.is_empty() && self.cooldown == 0
    }

    /// Nobody fired at the unit lately (prompt 187: it keeps the pace).
    pub(super) fn attackers_empty(&self) -> bool {
        self.attackers.is_empty()
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        if self.is_empty() {
            return;
        }
        h.write_u8(0xd1);
        h.write_u64(self.attackers.len() as u64);
        for &(id, left) in &self.attackers {
            h.write_u32(id.0);
            h.write_u32(left);
        }
        h.write_u32(self.cooldown);
    }

    fn remember(&mut self, shooter: UnitId, ticks: u32) {
        match self.attackers.binary_search_by_key(&shooter, |a| a.0) {
            Ok(k) => self.attackers[k].1 = ticks,
            Err(k) => {
                if self.attackers.len() >= MAX_ATTACKERS {
                    // Forget the one remembered shortest.
                    if let Some(oldest) = (0..self.attackers.len())
                        .min_by_key(|&k| (self.attackers[k].1, self.attackers[k].0))
                    {
                        self.attackers.remove(oldest);
                    }
                    let k = self
                        .attackers
                        .binary_search_by_key(&shooter, |a| a.0)
                        .unwrap_or_else(|k| k);
                    self.attackers.insert(k, (shooter, ticks));
                } else {
                    self.attackers.insert(k, (shooter, ticks));
                }
            }
        }
    }

    fn knows(&self, shooter: UnitId) -> bool {
        self.attackers
            .binary_search_by_key(&shooter, |a| a.0)
            .is_ok()
    }
}

impl Sim {
    /// Whether unit `i` may take unit `j` at `distance` as an automatic
    /// target of a weapon whose longest reach is `range` (fire mode and
    /// limits).
    pub(super) fn may_engage(&self, i: usize, j: usize, distance: Fixed, range: Fixed) -> bool {
        let unit = &self.units[i];
        let combat = &unit.combat;
        let mode_ok = match combat.fire_mode {
            FireMode::FireAtWill => true,
            FireMode::HoldFire => false,
            FireMode::ReturnFire => unit.under_fire.knows(self.units[j].id),
        };
        let limits = combat.limits;
        let rt = &self.tactics.discipline;
        mode_ok
            && (!limits.vehicles_only || self.units[j].squad.is_none())
            && (!limits.close_only || distance * 100 <= range * Fixed::from_num(rt.close_pct))
    }

    /// A round of `shooter` landed at `at` (or hit unit `hit`): units in
    /// return-fire mode within `return_near_mm` (and the hit one) remember
    /// the shooter.
    pub(super) fn note_fire(&mut self, at: Vec2, shooter: UnitId, hit: Option<usize>) {
        let rt = self.tactics.discipline;
        if rt.remember == 0
            || !self
                .units
                .iter()
                .any(|u| u.combat.fire_mode == FireMode::ReturnFire)
        {
            return;
        }
        let Ok(k) = self.units.binary_search_by_key(&shooter, |u| u.id) else {
            return;
        };
        let owner = self.units[k].owner;
        let reach = rt.near + self.unit_grid.max_reach;
        let r = Vec2::new(reach, reach);
        let mut near = Vec::new();
        self.unit_grid.query_sorted(at - r, at + r, &mut near);
        if let Some(j) = hit
            && !near.contains(&(j as u32))
        {
            near.push(j as u32);
        }
        for j in near {
            let j = j as usize;
            let unit = &self.units[j];
            if unit.combat.fire_mode != FireMode::ReturnFire
                || !unit.alive()
                || self.teams.allied(unit.owner, owner)
            {
                continue;
            }
            if Some(j) == hit || (unit.position - at).length() <= rt.near {
                self.units[j].under_fire.remember(shooter, rt.remember);
            }
        }
    }

    /// Counts down remembered attackers and retreat cooldowns.
    pub(super) fn decay_under_fire(&mut self) {
        for unit in &mut self.units {
            let u = &mut unit.under_fire;
            if u.is_empty() {
                continue;
            }
            u.cooldown = u.cooldown.saturating_sub(1);
            for a in &mut u.attackers {
                a.1 = a.1.saturating_sub(1);
            }
            u.attackers.retain(|a| a.1 > 0);
        }
    }

    /// Unit `j` was hit or nearly hit by a round flying along `-back`: an
    /// unarmed vehicle standing without an order drives away from where it
    /// came from.
    pub(super) fn unarmed_under_fire(&mut self, j: usize, back: Vec2) {
        let rt = self.tactics.discipline;
        if rt.retreat <= Fixed::ZERO {
            return;
        }
        let unit = &self.units[j];
        let ty = &self.types[unit.unit_type as usize];
        if !unit.alive()
            || ty.infantry
            || !ty.weapons.is_empty()
            || unit.squad.is_some()
            || unit.link.is_some()
            || unit.speed_offroad <= Fixed::ZERO
            || unit.target.is_some()
            || !unit.orders.is_empty()
            || unit.under_fire.cooldown > 0
            || self.transport_task_name(unit.id).is_some()
            || self.economy.structure_index(unit.id).is_some()
        {
            return;
        }
        let Some(towards) = crate::formation::normalized(back) else {
            return;
        };
        let goal = unit.position - towards.scale(rt.retreat);
        let goal = Vec2::new(
            goal.x.clamp(
                -Fixed::from_num(super::WORLD_LIMIT_M),
                Fixed::from_num(super::WORLD_LIMIT_M),
            ),
            goal.y.clamp(
                -Fixed::from_num(super::WORLD_LIMIT_M),
                Fixed::from_num(super::WORLD_LIMIT_M),
            ),
        );
        self.units[j].under_fire.cooldown = rt.cooldown.max(1);
        stop(&mut self.units[j]);
        self.request_group_path(&[(j, goal)], goal);
    }

    /// `SetFireLimits`.
    pub(super) fn order_fire_limits(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        limits: FireLimits,
    ) {
        for i in self.owned_indices(player, units) {
            self.units[i].combat.limits = limits;
            // The current automatic target is chosen again by the new rule.
            self.units[i].combat.auto_target = None;
            self.units[i].combat.mg_targets.clear();
        }
    }
}
