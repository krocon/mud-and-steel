//! Blasts knock soldiers down (prompt 048). Rules: `[knockdown]` in
//! `data/infantry.toml` (`crate::infantry::KnockdownRules`).
//!
//! The simulation decides who falls, where he lies and when he gets up; the
//! presentation (ragdolls in Godot) only shows the body falling there.
//! High-explosive charges ([`Sim::detonate`], shells, bombs, exploding
//! dumps) knock down the soldiers within a radius per caliber; a destroyed
//! gun-armed vehicle blows up its ammunition with a chance and knocks down
//! the soldiers around the wreck. A soldier is thrown away from the blast
//! (at most `throw_mm`, less towards the edge of the radius) and lies there
//! for `down_ms`: he neither moves nor fires and is hit like a prone
//! soldier. Soldiers killed by the same blast are thrown as well, so the
//! bodies lie where the blast put them.

use super::Sim;
use crate::math::{Fixed, Vec2};

/// Soldiers a blast reaches: (unit index, soldier index, new position).
pub(super) type Thrown = Vec<(usize, usize, Vec2)>;

impl Sim {
    /// The living soldiers that a blast of `radius_mm` (before stance and
    /// cover factors) at `p` knocks down, with the point each is thrown to.
    /// Computed before the blast's fragments strike, applied after them by
    /// [`Sim::knock_down`].
    pub(super) fn blast_reach(&self, p: Vec2, radius_mm: u32) -> Thrown {
        let mut out = Vec::new();
        if radius_mm == 0 {
            return out;
        }
        let rules = self.infantry.rules.knockdown;
        let radius = Fixed::from_num(radius_mm) / 1000;
        let throw = Fixed::from_num(rules.throw_mm) / 1000;
        let reach = radius + self.unit_grid.max_reach;
        let mut near = Vec::new();
        self.unit_grid.query_sorted(
            p - Vec2::new(reach, reach),
            p + Vec2::new(reach, reach),
            &mut near,
        );
        let (map, grid, terrain) = (&self.map, &self.paths.grid, &self.terrain);
        for &j in &near {
            let j = j as usize;
            let unit = &self.units[j];
            let Some(squad) = &unit.squad else { continue };
            if !unit.alive() || squad.garrison.is_some() || super::transport::aboard(unit) {
                continue;
            }
            let profile = unit.unit_type as usize;
            let ok = |q: Vec2| {
                map.contains(q)
                    && terrain.types[usize::from(map.terrain_at(q))].passable
                    && grid.passable_for(profile, q)
            };
            let stance_r = radius * Fixed::from_num(rules.stance_pct(squad.stance)) / 100;
            for (k, s) in squad.soldiers.iter().enumerate() {
                if !s.alive() {
                    continue;
                }
                let r = if s.at_spot() {
                    stance_r * Fixed::from_num(rules.cover_pct) / 100
                } else {
                    stance_r
                };
                let rel = s.position - p;
                let d = rel.length();
                if r <= Fixed::ZERO || d >= r {
                    continue;
                }
                // Away from the blast; straight on it, backwards.
                let dir = if d > Fixed::ZERO {
                    Vec2::new(rel.x / d, rel.y / d)
                } else {
                    Vec2::new(-unit.facing.x, -unit.facing.y)
                };
                let to = s.position + dir.scale(throw * (r - d) / r);
                out.push((j, k, if ok(to) { to } else { s.position }));
            }
        }
        out
    }

    /// Knocks the soldiers of `thrown` down (`blast_reach`): they lie at
    /// their new position for the knock-down time. Those who died of the
    /// blast meanwhile are only moved.
    pub(super) fn knock_down(&mut self, thrown: &Thrown, from: Vec2) {
        for &(j, k, to) in thrown {
            let ticks = self.infantry.types[self.units[j].unit_type as usize]
                .as_ref()
                .map_or(0, |t| t.knock_ticks);
            let height = self.map.height_at(to);
            let Some(squad) = &mut self.units[j].squad else {
                continue;
            };
            let s = &mut squad.soldiers[k];
            s.position = to;
            s.height = height;
            s.knock_from = from;
            s.knocked = if s.alive() { ticks } else { 0 };
        }
    }

    /// A destroyed vehicle (unit index `j`, largest gun `gun_mm_x10`) may
    /// blow up its ammunition: soldiers around the wreck are knocked down.
    pub(super) fn ammo_explosion(&mut self, j: usize, gun_mm_x10: u32) {
        let rules = self.infantry.rules.knockdown;
        let unit = &self.units[j];
        let armed = gun_mm_x10 > 0 && gun_mm_x10 >= rules.ammo_min_caliber_mm_x10;
        if unit.squad.is_some() || !armed || rules.ammo_explosion_pct == 0 {
            return;
        }
        // A roll from a generator seeded by the shared one's state and the
        // unit id, without advancing the shared one: battles keep their
        // random sequence (deterministic all the same).
        let seed = self.rng.state() ^ (u64::from(unit.id.0) << 32) ^ 0x9e37_79b9_7f4a_7c15;
        if crate::rng::Rng::new(seed).below(100) >= rules.ammo_explosion_pct {
            return;
        }
        let at = unit.position;
        // The presentation blows the turret off (prompt 054).
        let event = crate::strike::StrikeEvent::AmmoExplosion {
            unit: unit.id,
            at: crate::combat::Vec3::new(at.x, at.y, unit.height),
        };
        self.strikes.events.push(event);
        let thrown = self.blast_reach(at, rules.ammo_radius_mm);
        self.knock_down(&thrown, at);
    }
}
