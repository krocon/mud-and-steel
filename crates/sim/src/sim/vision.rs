//! Keeping the viewsheds of all units and the vision of every player up to
//! date (prompt 018). The sight computation itself: `crate::vision`.
//!
//! A unit's viewshed is computed again when it enters another vision cell.
//! Such units wait in a FIFO queue; every tick works through it until the
//! ray-step budget of the rules is spent (at least one unit per tick), so
//! the cost of a tick stays bounded however many units move. Destroyed units
//! lose their viewshed at once. Afterwards every unit gets the set of
//! players that see it.

use super::Sim;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::Vec2;
use crate::sim::Snapshot;
use crate::vision::{CellState, PlayerVision, Viewshed, VisionGrid, VisionRules};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Vision state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Vision {
    pub(super) rules: VisionRules,
    pub(super) grid: Arc<VisionGrid>,
    /// Players in slot order (ascending).
    players: Vec<PlayerId>,
    per_player: Vec<PlayerVision>,
    /// Per unit index.
    sheds: Vec<Option<Viewshed>>,
    queued: Vec<bool>,
    queue: VecDeque<u32>,
    /// Per unit index: bit `s` = player slot `s` sees the unit.
    seen: Vec<u64>,
    /// Per unit type: eye height over the ground (mm) and sight radius (cells).
    eye_mm: Vec<i32>,
    radius: Vec<i32>,
    /// Ray steps computed in the last tick (benchmark).
    pub(super) last_steps: u32,
}

/// Most players a match can have (one bit each in [`Vision::seen`]).
pub const MAX_PLAYERS: usize = 64;

impl Vision {
    /// `eye_m` and `categories` per unit type.
    pub(super) fn new(
        rules: &VisionRules,
        map: &crate::map::Map,
        players: Vec<PlayerId>,
        types: &[(i32, Option<crate::catalog::Category>)],
        units: usize,
    ) -> Self {
        let radius: Vec<i32> = types.iter().map(|&(_, c)| rules.range_cells(c)).collect();
        let grid = VisionGrid::new(map, rules, &radius);
        let cells = grid.cells();
        Self {
            rules: rules.clone(),
            grid: Arc::new(grid),
            per_player: players.iter().map(|_| PlayerVision::new(cells)).collect(),
            players,
            sheds: vec![None; units],
            queued: vec![false; units],
            queue: VecDeque::new(),
            seen: vec![0; units],
            eye_mm: types.iter().map(|&(eye, _)| eye).collect(),
            radius,
            last_steps: 0,
        }
    }

    /// Room for one more unit at the end of the unit list (units produced
    /// during the match, prompt 023).
    pub(super) fn push_unit(&mut self) {
        self.sheds.push(None);
        self.queued.push(false);
        self.seen.push(0);
    }

    fn slot(&self, player: PlayerId) -> Option<usize> {
        self.players.binary_search(&player).ok()
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        for shed in &self.sheds {
            match shed {
                None => h.write_u8(0),
                Some(s) => {
                    h.write_u8(1);
                    h.write_u32(s.anchor);
                    h.write_u32(s.eye_mm as u32);
                    h.write_u32(s.radius as u32);
                    h.write_u32(s.count());
                }
            }
        }
        h.write_u64(self.queue.len() as u64);
        for &i in &self.queue {
            h.write_u32(i);
        }
        for &s in &self.seen {
            h.write_u64(s);
        }
        for p in &self.per_player {
            p.hash_into(h);
        }
    }
}

fn to_mm(v: crate::math::Fixed) -> i32 {
    (v * crate::math::Fixed::from_num(1000))
        .round()
        .to_num::<i32>()
}

impl Sim {
    /// Updates viewsheds (within the budget, or all of them if `unlimited`)
    /// and who sees which unit.
    pub(super) fn update_vision(&mut self, unlimited: bool) {
        let v = &mut self.vision;
        let grid = Arc::clone(&v.grid);
        for (i, unit) in self.units.iter_mut().enumerate() {
            if !unit.alive() {
                if let Some(shed) = v.sheds[i].take() {
                    let slot = v.slot(unit.owner).expect("owners are players");
                    v.per_player[slot].remove(&grid, &shed);
                }
                continue;
            }
            let anchor = grid.index_of(unit.position);
            let moved = v.sheds[i].as_ref().map(|s| s.anchor) != Some(anchor);
            if (moved || unit.vision_dirty) && !v.queued[i] {
                v.queued[i] = true;
                v.queue.push_back(i as u32);
            }
            unit.vision_dirty = false;
        }
        let budget = v.rules.grid.budget_steps_per_tick;
        let mut spent = 0u32;
        let mut done = 0u32;
        while let Some(&i) = v.queue.front() {
            if !unlimited && done > 0 && spent >= budget {
                break;
            }
            v.queue.pop_front();
            let i = i as usize;
            v.queued[i] = false;
            let unit = &self.units[i];
            if !unit.alive() {
                continue;
            }
            let t = unit.unit_type as usize;
            let anchor = grid.index_of(unit.position);
            // Observers in a building look out over its roof (prompt 022).
            let eye = to_mm(unit.height).saturating_add(v.eye_mm[t].max(unit.elevation_mm));
            let (shed, steps) = grid.viewshed(anchor, eye, v.radius[t]);
            let slot = v.slot(unit.owner).expect("owners are players");
            if let Some(old) = v.sheds[i].take() {
                v.per_player[slot].remove(&grid, &old);
            }
            v.per_player[slot].add(&grid, &shed);
            v.sheds[i] = Some(shed);
            spent = spent.saturating_add(steps);
            done += 1;
        }
        v.last_steps = spent;
        for (i, unit) in self.units.iter().enumerate() {
            v.seen[i] = if super::transport::aboard(unit) {
                // Passengers are hidden in their vehicle (prompt 034).
                v.slot(unit.owner).map_or(0, |s| 1 << s)
            } else if unit.alive() {
                let cell = grid.index_of(unit.position) as usize;
                // Squads in a building are seen at their windows (prompt 022).
                let windows: Vec<usize> = match &unit.squad {
                    Some(squad) if squad.garrison.is_some() => squad
                        .soldiers
                        .iter()
                        .filter(|x| x.alive())
                        .map(|x| grid.index_of(x.position) as usize)
                        .collect(),
                    _ => Vec::new(),
                };
                let mut mask = 0u64;
                for (s, (&player, pv)) in v.players.iter().zip(&v.per_player).enumerate() {
                    if player == unit.owner
                        || pv.visible(cell)
                        || windows.iter().any(|&c| pv.visible(c))
                    {
                        mask |= 1 << s;
                    }
                }
                mask
            } else {
                u64::MAX
            };
        }
    }

    /// Queues every viewshed that covers a vision cell of `c0..=c1` ×
    /// `r0..=r1` (obstacles there changed, prompt 020).
    pub(super) fn requeue_viewsheds(&mut self, (c0, r0, c1, r1): (u32, u32, u32, u32)) {
        let v = &mut self.vision;
        let columns = v.grid.columns;
        for (i, shed) in v.sheds.iter().enumerate() {
            let Some(shed) = shed else { continue };
            let (c, r) = (
                (shed.anchor % columns) as i64,
                (shed.anchor / columns) as i64,
            );
            let radius = i64::from(shed.radius);
            let overlaps = c + radius >= i64::from(c0)
                && c - radius <= i64::from(c1)
                && r + radius >= i64::from(r0)
                && r - radius <= i64::from(r1);
            if overlaps && !v.queued[i] {
                v.queued[i] = true;
                v.queue.push_back(i as u32);
            }
        }
    }

    /// Whether the viewshed of unit `i` contains the point `p`.
    /// Whether unit `i` sees unit `j`: its cell, or for a squad in a
    /// building one of the windows its soldiers stand at (prompt 022).
    pub(super) fn sees_unit(&self, i: usize, j: usize) -> bool {
        let target = &self.units[j];
        if self.unit_sees(i, target.position) {
            return true;
        }
        target.squad.as_ref().is_some_and(|s| {
            s.garrison.is_some()
                && s.soldiers
                    .iter()
                    .any(|x| x.alive() && self.unit_sees(i, x.position))
        })
    }

    pub(super) fn unit_sees(&self, i: usize, p: Vec2) -> bool {
        let grid = &self.vision.grid;
        let (c, r) = grid.cell_of(p);
        self.vision.sheds[i]
            .as_ref()
            .is_some_and(|s| s.contains(grid, c, r))
    }

    /// Whether `player` sees the vision cell of `p` now (prompt 032:
    /// indirect fire at unseen points is blind).
    pub(super) fn player_sees_point(&self, player: PlayerId, p: Vec2) -> bool {
        let cell = self.vision.grid.index_of(p) as usize;
        self.vision
            .slot(player)
            .is_some_and(|s| self.vision.per_player[s].visible(cell))
    }

    /// Whether the owner of unit `i` sees unit `j`.
    pub(super) fn owner_sees(&self, i: usize, j: usize) -> bool {
        self.vision
            .slot(self.units[i].owner)
            .is_some_and(|s| self.vision.seen[j] & (1 << s) != 0)
    }

    /// Whether `player` sees unit `unit`: own units, wrecks and enemies in
    /// the player's vision. Unknown players see only wrecks.
    pub fn visible_to(&self, player: PlayerId, unit: UnitId) -> bool {
        let Ok(j) = self.units.binary_search_by_key(&unit, |u| u.id) else {
            return false;
        };
        match self.vision.slot(player) {
            Some(s) => self.vision.seen[j] & (1 << s) != 0,
            None => !self.units[j].alive(),
        }
    }

    /// [`Sim::snapshot`] as `player` sees it: enemy units outside the
    /// player's vision are left out. Projectiles and events stay (muzzle
    /// flashes and tracers can be seen from afar).
    pub fn snapshot_for(&self, player: PlayerId) -> Snapshot {
        let mut snapshot = Snapshot::default();
        self.snapshot_into(Some(player), &mut snapshot);
        snapshot
    }

    /// The vision grid of this match (cell size, origin, dimensions).
    pub fn vision_grid(&self) -> &VisionGrid {
        &self.vision.grid
    }

    /// The state of every vision cell for `player`, row by row (all
    /// unexplored for unknown players).
    pub fn vision_states(&self, player: PlayerId, out: &mut Vec<CellState>) {
        out.clear();
        let cells = self.vision.grid.cells();
        match self.vision.slot(player) {
            Some(s) => {
                let pv = &self.vision.per_player[s];
                out.extend((0..cells).map(|i| pv.state(i)));
            }
            None => out.resize(cells, CellState::Unexplored),
        }
    }

    /// Changes whenever a cell of `player` changes its state.
    pub fn vision_version(&self, player: PlayerId) -> u64 {
        self.vision
            .slot(player)
            .map_or(0, |s| self.vision.per_player[s].version)
    }

    /// Units waiting for a new viewshed.
    pub fn vision_queue_len(&self) -> usize {
        self.vision.queue.len()
    }

    /// Ray steps computed in the last tick.
    pub const fn vision_steps(&self) -> u32 {
        self.vision.last_steps
    }
}

/// Eye height of a unit type in millimeters (its hull or soldier height).
pub(super) fn eye_mm(height: crate::math::Fixed) -> i32 {
    to_mm(height)
}
