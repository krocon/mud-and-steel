//! Infantry ambush (prompt 056) [provisional].
//!
//! A squad that stands in a forest cell (`Sight::Foliage` in the vision
//! grid) or is garrisoned in a building, and has not fired for
//! `ambush.reveal_s`, is seen only by its team and by players with a hostile
//! ground unit within `ambush.spot_real_m` (compressed like sight ranges)
//! that has it in sight. Its first shot reveals it (it is then seen like any
//! other unit); standing silent again hides it again. Runs after the vision
//! update, like the camouflage nets of prompt 052, whose pattern it follows.
//!
//! The state (when squads last fired) and the rules are hashed only once a
//! squad has been hidden by this rule, so matches without infantry in cover
//! keep their hashes.

use super::super::{Route, Sim};
use crate::combat::CombatEvent;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::Vec2;
use crate::vision::{AmbushRules, Sight};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Ambush state of a match.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(in crate::sim) struct Ambush {
    /// A squad has been hidden by the rule (only then is the state hashed).
    used: bool,
    /// Squads in cover: the last tick they fired.
    fired: BTreeMap<UnitId, u64>,
}

impl Ambush {
    pub(in crate::sim) fn hash_into(&self, h: &mut StableHasher, rules: &AmbushRules) {
        if !self.used {
            return;
        }
        h.write_u8(0x56);
        h.write_u32(rules.spot_real_m);
        h.write_u32(rules.reveal_s);
        h.write_u64(self.fired.len() as u64);
        for (id, tick) in &self.fired {
            h.write_u32(id.0);
            h.write_u64(*tick);
        }
    }
}

impl Sim {
    /// Whether unit `i` is a squad in cover: standing, alive, not aboard,
    /// garrisoned or in a forest cell.
    fn in_ambush_cover(&self, i: usize) -> bool {
        let u = &self.units[i];
        let Some(squad) = &u.squad else {
            return false;
        };
        if !u.alive() || u.route != Route::None || u.link.is_some() {
            return false;
        }
        if squad.garrison.is_some() {
            return true;
        }
        let grid = &self.vision.grid;
        grid.blocker_at(grid.index_of(u.position) as usize) == Sight::Foliage
    }

    fn ambush_quiet_ticks(&self) -> u64 {
        u64::from(self.vision.rules.ambush.reveal_s) * u64::from(self.tick_hz)
    }

    /// Whether unit `i` is hidden in an ambush now.
    pub(in crate::sim) fn in_ambush(&self, i: usize) -> bool {
        if self.vision.rules.ambush.spot_real_m == 0 || !self.in_ambush_cover(i) {
            return false;
        }
        let quiet = self.ambush_quiet_ticks();
        !self
            .vision
            .ambush
            .fired
            .get(&self.units[i].id)
            .is_some_and(|&t| self.tick < t + quiet)
    }

    /// Notes the shots of squads in cover and hides the silent ones from
    /// distant observers. Runs at the end of the vision update.
    pub(in crate::sim) fn update_ambush(&mut self) {
        if self.vision.rules.ambush.spot_real_m == 0 {
            return;
        }
        let shooters: Vec<UnitId> = self
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Shot { unit, .. } => Some(*unit),
                _ => None,
            })
            .collect();
        for id in shooters {
            if let Ok(i) = self.units.binary_search_by_key(&id, |u| u.id)
                && self.units[i].squad.is_some()
                && self.in_ambush_cover(i)
            {
                self.vision.ambush.fired.insert(id, self.tick);
            }
        }
        let quiet = self.ambush_quiet_ticks();
        let tick = self.tick;
        self.vision.ambush.fired.retain(|_, t| tick < *t + quiet);

        let spot = self.vision.rules.ambush_spot_m();
        let mut near = Vec::new();
        for i in 0..self.units.len() {
            if !self.in_ambush(i) {
                continue;
            }
            self.vision.ambush.used = true;
            let (owner, p) = (self.units[i].owner, self.units[i].position);
            self.unit_grid.query_sorted(
                p - Vec2::new(spot, spot),
                p + Vec2::new(spot, spot),
                &mut near,
            );
            let mut spotters: Vec<PlayerId> = near
                .iter()
                .map(|&k| k as usize)
                .filter(|&k| {
                    let s = &self.units[k];
                    s.alive()
                        && self.teams.hostile(s.owner, owner)
                        && (s.position - p).length() <= spot
                        && self.sees_unit(k, i)
                })
                .map(|k| self.units[k].owner)
                .collect();
            spotters.sort_unstable();
            spotters.dedup();
            self.vision.restrict(i, owner, &spotters);
        }
    }
}
