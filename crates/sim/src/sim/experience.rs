//! Experience and ranks (prompt 050). Rules: `crate::experience`.
//!
//! - Every unit (vehicle, gun, building, whole squad) has a [`Veterancy`]:
//!   experience points, kills and vehicle kills.
//! - [`Sim::update_experience`] reads the combat events of the tick once
//!   everything has fired and burned: damaging hits on enemy units give the
//!   shooter points in proportion to the damage (`damage_xp` for a whole
//!   unit), the hit that destroys an enemy `kill_xp` extra, a unit that
//!   took a damaging enemy hit and still lives `survived_hit_xp`, and once
//!   per game second every suppressed living squad `suppressed_xp_per_s`.
//!   Hits by mines and aircraft have no shooter unit and give nobody points;
//!   blast damage of shells and bombs to units around the impact counts
//!   only through the kill.
//! - The rank follows from the points (`ExperienceRules::rank_of`); its
//!   bonuses scale the dispersion of every shot and the reload time
//!   (`sim/combat.rs`) and how long close fire suppresses a squad
//!   (`sim/infantry.rs`, `sim/indirect.rs`, `sim/air.rs`).
//!
//! Order within a tick (see [`Sim::step`]): after the economy and supply,
//! when all of the tick's combat events are known.

use super::Sim;
use crate::combat::{CombatEvent, HitKind};
use crate::command::UnitId;
use crate::experience::ExperienceRules;
use crate::hash::StableHasher;
use serde::{Deserialize, Serialize};

/// Experience of one unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Veterancy {
    /// Experience points in hundredths (small hits add up exactly).
    pub(super) centi_xp: u32,
    /// Enemy units destroyed (the last hit).
    pub(super) kills: u16,
    /// Of those, enemy vehicles and guns (no squads, no buildings): the
    /// rings painted on the barrel.
    pub(super) vehicle_kills: u16,
}

/// Experience state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Experience {
    pub(super) rules: ExperienceRules,
    /// Units promoted during the last tick and their new rank (derived
    /// like the events: not hashed).
    promoted: Vec<(UnitId, u8)>,
    /// Hit points of every unit (by index) at the end of the last tick:
    /// the damage a target really lost this tick, without overkill.
    /// Derived from the hit points, so not hashed; units produced since
    /// count with their full hit points.
    last_hp: Vec<u32>,
}

/// Experience of a unit for the presentation and the AI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExperienceView {
    pub xp: u32,
    /// Rank index (0 = the first rank of `data/experience.toml`).
    pub rank: u8,
    pub kills: u16,
    pub vehicle_kills: u16,
}

fn scaled(value: u32, pct: u32) -> u32 {
    let v = (u64::from(value) * u64::from(pct) + 50) / 100;
    u32::try_from(v)
        .unwrap_or(u32::MAX)
        .max(u32::from(value > 0))
}

impl Experience {
    pub(super) const fn new(rules: ExperienceRules) -> Self {
        Self {
            rules,
            promoted: Vec::new(),
            last_hp: Vec::new(),
        }
    }

    pub(super) fn rank_of(&self, v: Veterancy) -> u8 {
        self.rules.rank_of(v.centi_xp / 100)
    }

    /// Dispersion of a shot in percent of the recruit's.
    pub(super) fn dispersion_pct(&self, v: Veterancy) -> u32 {
        self.rules.rank(self.rank_of(v)).dispersion_pct
    }

    /// Reload time in ticks after a salvo (at least one tick).
    pub(super) fn reload_ticks(&self, v: Veterancy, ticks: u32) -> u32 {
        scaled(ticks, self.rules.rank(self.rank_of(v)).reload_pct)
    }

    /// Ticks a near miss keeps a squad suppressed.
    pub(super) fn suppression_ticks(&self, v: Veterancy, ticks: u32) -> u32 {
        scaled(ticks, self.rules.rank(self.rank_of(v)).suppression_pct)
    }

    pub(super) fn view(&self, v: Veterancy) -> ExperienceView {
        ExperienceView {
            xp: v.centi_xp / 100,
            rank: self.rank_of(v),
            kills: v.kills,
            vehicle_kills: v.vehicle_kills,
        }
    }
}

impl Sim {
    fn unit_index(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    /// Experience points from the combat events of this tick.
    pub(super) fn update_experience(&mut self) {
        self.experience.promoted.clear();
        let gain = self.experience.rules.gain;
        let centi = |points: u32| points.saturating_mul(100);
        // (unit index, hundredths of points, kill, vehicle kill).
        let mut awards: Vec<(usize, u32, bool, bool)> = Vec::new();
        // (target, shooter, damage) of damaging enemy hits.
        let mut hits: Vec<(usize, usize, u32)> = Vec::new();
        for event in &self.events {
            match *event {
                CombatEvent::Hit {
                    shooter,
                    kind: HitKind::Unit(target),
                    damage,
                    ..
                } if damage > 0 => {
                    let (Some(i), Some(j)) = (self.unit_index(shooter), self.unit_index(target))
                    else {
                        continue;
                    };
                    if self.teams.allied(self.units[i].owner, self.units[j].owner) {
                        continue;
                    }
                    hits.push((j, i, damage));
                }
                CombatEvent::Destroyed { unit, by, .. } => {
                    let (Some(i), Some(j)) = (self.unit_index(by), self.unit_index(unit)) else {
                        continue;
                    };
                    if self.teams.allied(self.units[i].owner, self.units[j].owner) {
                        continue;
                    }
                    let target = &self.units[j];
                    let vehicle = target.squad.is_none()
                        && self.unit_types[target.unit_type as usize].mobile();
                    awards.push((i, centi(gain.kill_xp), true, vehicle));
                }
                _ => {}
            }
        }
        // Per target: the hit points it really lost (at most the damage of
        // the hits), shared out by the damage of every shooter's hits.
        hits.sort_by_key(|&(j, i, _)| (j, i));
        for group in hits.chunk_by(|a, b| a.0 == b.0) {
            let j = group[0].0;
            let max_hp = self.types[self.units[j].unit_type as usize].max_hp.max(1);
            let before = self.experience.last_hp.get(j).copied().unwrap_or(max_hp);
            let lost = before.saturating_sub(self.units[j].hp);
            let total: u64 = group.iter().map(|&(_, _, d)| u64::from(d)).sum();
            let credited = u64::from(lost).min(total);
            for shooter in group.chunk_by(|a, b| a.1 == b.1) {
                let damage: u64 = shooter.iter().map(|&(_, _, d)| u64::from(d)).sum();
                let points = credited * damage / total.max(1) * u64::from(gain.damage_xp) * 100
                    / u64::from(max_hp);
                let points = u32::try_from(points).unwrap_or(u32::MAX);
                if points > 0 {
                    awards.push((shooter[0].1, points, false, false));
                }
            }
            if self.units[j].alive() {
                awards.push((j, centi(gain.survived_hit_xp), false, false));
            }
        }
        if self.tick_hz > 0 && self.tick.is_multiple_of(u64::from(self.tick_hz)) {
            for (i, unit) in self.units.iter().enumerate() {
                if unit.alive() && unit.squad.as_ref().is_some_and(|s| s.suppressed > 0) {
                    awards.push((i, centi(gain.suppressed_xp_per_s), false, false));
                }
            }
        }
        for (i, points, kill, vehicle) in awards {
            let unit = &mut self.units[i];
            let before = self.experience.rank_of(unit.veterancy);
            let v = &mut unit.veterancy;
            v.centi_xp = v.centi_xp.saturating_add(points);
            if kill {
                v.kills = v.kills.saturating_add(1);
            }
            if vehicle {
                v.vehicle_kills = v.vehicle_kills.saturating_add(1);
            }
            let after = self.experience.rules.rank_of(v.centi_xp / 100);
            if after > before {
                self.experience.promoted.push((unit.id, after));
            }
        }
        self.experience.last_hp.clear();
        self.experience
            .last_hp
            .extend(self.units.iter().map(|u| u.hp));
    }

    pub(super) fn hash_experience(&self, h: &mut StableHasher) {
        self.experience.rules.hash_into(h);
        for u in &self.units {
            h.write_u32(u.veterancy.centi_xp);
            h.write_u32(u32::from(u.veterancy.kills));
            h.write_u32(u32::from(u.veterancy.vehicle_kills));
        }
    }

    // --- Public queries (presentation, AI) -----------------------------------

    /// Experience, rank and kills of unit `id` (`None` if it does not
    /// exist).
    pub fn unit_experience(&self, id: UnitId) -> Option<ExperienceView> {
        let i = self.unit_index(id)?;
        Some(self.experience.view(self.units[i].veterancy))
    }

    /// The experience rules of the match (rank names and thresholds).
    pub const fn experience_rules(&self) -> &ExperienceRules {
        &self.experience.rules
    }

    /// Units promoted during the last tick, with their new rank.
    pub fn promotions(&self) -> &[(UnitId, u8)] {
        &self.experience.promoted
    }

    /// Sets the experience points of unit `id` (scenarios, missions and
    /// tests: veteran units at the start). Not a player command.
    pub fn set_unit_experience(&mut self, id: UnitId, xp: u32) {
        if let Some(i) = self.unit_index(id) {
            self.units[i].veterancy.centi_xp = xp.saturating_mul(100);
        }
    }
}
