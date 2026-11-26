//! Heavy vehicles run over enemy soldiers (prompt 093, `[overrun]` in
//! `data/combat.toml`).
//!
//! Order within a tick (see [`Sim::step`]): after the movement,
//! [`Sim::plan_overrun`] lists the vehicles that drive fast enough and the
//! soldiers who step out of their lane; [`Sim::update_squads`] moves those
//! soldiers sideways instead of to their places; [`Sim::overrun_soldiers`]
//! then kills the enemy soldiers still under a hull. Vehicles in index
//! order, squads in index order, soldiers in order: no randomness.
//!
//! Nothing here is state: the lists are rebuilt every tick from the
//! movement of the tick.

use super::infantry::inside;
use super::{Sim, transport};
use crate::combat::Vec3;
use crate::infantry::Stance;
use crate::math::{Fixed, Vec2};

/// The lists of one tick.
#[derive(Clone, Debug, Default)]
pub(super) struct Overrun {
    /// Vehicles that moved this tick: unit index and distance.
    pub(super) driven: Vec<(u32, Fixed)>,
    /// Vehicles that run soldiers over this tick with the enemy squads
    /// near them (unit indices), sorted.
    threats: Vec<(u32, u32)>,
    /// Soldiers stepping aside: `(unit index, soldier, direction)`, sorted.
    dodges: Vec<(u32, u16, Vec2)>,
}

impl Overrun {
    /// Direction soldier `k` of unit `i` steps aside to, if he does.
    pub(super) fn dodge(&self, i: usize, k: usize) -> Option<Vec2> {
        if self.dodges.is_empty() {
            return None;
        }
        let key = (u32::try_from(i).ok()?, u16::try_from(k).ok()?);
        self.dodges
            .binary_search_by_key(&key, |&(u, s, _)| (u, s))
            .ok()
            .map(|n| self.dodges[n].2)
    }
}

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

impl Sim {
    /// Meters per tick of the step aside.
    pub(super) fn dodge_step(&self) -> Fixed {
        mm(self.combat_rules.overrun.dodge_speed_mm_per_s) / Fixed::from_num(self.tick_hz.max(1))
    }

    /// Whether a soldier at `p` is in a dug position (foxhole, trench,
    /// dugout, nest): nobody is run over there.
    fn dug_in_at(&self, p: Vec2) -> bool {
        if self.works.dug.is_empty() {
            return false;
        }
        super::craters::nearest_sample(&self.map, p)
            .and_then(|s| u32::try_from(s).ok())
            .is_some_and(|s| self.works.dug.contains(&s))
    }

    /// Finds the vehicles that run soldiers over this tick and the
    /// soldiers who step out of their way.
    pub(super) fn plan_overrun(&mut self) {
        let rules = self.combat_rules.overrun;
        let mut plan = std::mem::take(&mut self.overrun);
        plan.threats.clear();
        plan.dodges.clear();
        if rules.min_mass_kg == 0 {
            plan.driven.clear();
            self.overrun = plan;
            return;
        }
        let hz = Fixed::from_num(self.tick_hz.max(1));
        let min_step = mm(rules.min_speed_mm_per_s) / hz;
        let (notice, clear) = (mm(rules.notice_mm), mm(rules.dodge_clear_mm));
        let mut near = Vec::new();
        for &(v, step) in &plan.driven {
            let vehicle = &self.units[v as usize];
            let unit_type = &self.unit_types[vehicle.unit_type as usize];
            if !vehicle.alive()
                || vehicle.squad.is_some()
                || unit_type.mass_kg < rules.min_mass_kg
                || step <= Fixed::ZERO
                || transport::aboard(vehicle)
            {
                continue;
            }
            let fast = step >= min_step;
            let ty = &self.types[vehicle.unit_type as usize];
            let (forward, center) = (vehicle.facing, vehicle.position);
            let right = Vec2::new(-forward.y, forward.x);
            // The way the vehicle covers while a soldier reacts.
            let late = step * Fixed::from_num(rules.reaction_ticks);
            let lane = ty.half_length + notice + Fixed::ONE;
            let reach = lane + self.unit_grid.max_spread;
            let reach = Vec2::new(reach, reach);
            self.unit_grid
                .query(center - reach, center + reach, &mut near);
            for &j in &near {
                let other = &self.units[j as usize];
                let Some(squad) = &other.squad else { continue };
                // The grid's cells are coarse: most candidates are far away.
                let away = other.position - center;
                let far = lane + other.spread;
                if away.x.abs() > far
                    || away.y.abs() > far
                    || !other.alive()
                    || inside(other)
                    || transport::aboard(other)
                {
                    continue;
                }
                let friend = self.teams.allied(other.owner, vehicle.owner);
                if !friend && !fast {
                    continue;
                }
                if !friend {
                    plan.threats.push((v, j));
                }
                // Enemies must see it coming and be on their feet.
                let aware = friend
                    || (squad.stance != Stance::Prone
                        && !squad.tactics.pinned
                        && squad.changing == 0
                        && self.sees_unit(j as usize, v as usize));
                if !aware {
                    continue;
                }
                let ahead_limit = if friend { notice } else { notice - late };
                for (k, s) in squad.soldiers.iter().enumerate() {
                    if !s.alive() || s.knocked > 0 || s.wound.held || s.wound.lying() {
                        continue;
                    }
                    let d = s.position - center;
                    let ahead = d.x * forward.x + d.y * forward.y;
                    let side = d.x * right.x + d.y * right.y;
                    if ahead < -ty.half_length
                        || ahead - ty.half_length > ahead_limit
                        || side.abs() >= ty.half_width + clear
                        || self.dug_in_at(s.position)
                    {
                        continue;
                    }
                    // Away from the middle of the lane; on it, by index.
                    let to_right = side > Fixed::ZERO || (side == Fixed::ZERO && k % 2 == 0);
                    let direction = if to_right { right } else { Vec2::ZERO - right };
                    let key = (j, u16::try_from(k).unwrap_or(u16::MAX));
                    // The first vehicle (lowest index) decides.
                    if !plan.dodges.iter().any(|&(u, n, _)| (u, n) == key) {
                        plan.dodges.push((key.0, key.1, direction));
                    }
                }
            }
        }
        // Vehicles in index order (as they drove), their squads by index.
        plan.threats.sort_unstable();
        plan.dodges.sort_unstable_by_key(|&(u, s, _)| (u, s));
        plan.driven.clear();
        self.overrun = plan;
    }

    /// Kills the enemy soldiers under the hull of the vehicles found by
    /// [`Self::plan_overrun`]. A squad that loses its last soldier counts
    /// as destroyed by the vehicle (kills, experience, statistics).
    pub(super) fn overrun_soldiers(&mut self) {
        if self.overrun.threats.is_empty() {
            return;
        }
        let threats = std::mem::take(&mut self.overrun.threats);
        let mut destroyed = Vec::new();
        for &(v, j) in &threats {
            let vehicle = &self.units[v as usize];
            let ty = &self.types[vehicle.unit_type as usize];
            let (half_length, half_width) = (ty.half_length, ty.half_width);
            let (forward, center, id) = (vehicle.facing, vehicle.position, vehicle.id);
            let right = Vec2::new(-forward.y, forward.x);
            let j = j as usize;
            let other = &self.units[j];
            // Its last man may have died under another vehicle, or the
            // squad entered a building in this tick.
            if !other.alive() || inside(other) {
                continue;
            }
            let squad = other.squad.as_ref().expect("checked");
            let under: Vec<usize> = squad
                .soldiers
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    let d = s.position - center;
                    s.alive()
                        && (d.x * forward.x + d.y * forward.y).abs() <= half_length
                        && (d.x * right.x + d.y * right.y).abs() <= half_width
                        && !self.dug_in_at(s.position)
                })
                .map(|(k, _)| k)
                .collect();
            if under.is_empty() {
                continue;
            }
            let unit = &mut self.units[j];
            let squad = unit.squad.as_mut().expect("checked");
            for k in under {
                squad.soldiers[k].hp = 0;
            }
            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            self.tactics.hit(squad);
            if unit.hp == 0 {
                let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
                destroyed.push((j, id, at));
            }
        }
        self.overrun.threats = threats;
        for (j, by, at) in destroyed {
            self.destroy(j, by, at);
        }
    }
}
