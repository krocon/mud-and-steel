//! Keeping the viewsheds of all units and the vision of every player up to
//! date (prompt 018). The sight computation itself: `crate::vision`.
//!
//! A unit's viewshed is computed again when it enters another vision cell.
//! Such units wait in a FIFO queue; every tick works through it until the
//! ray-step budget of the rules is spent (at least one unit per tick), so
//! the cost of a tick stays bounded however many units move. Destroyed units
//! lose their viewshed at once. Afterwards every unit gets the set of
//! players that see it.

mod ambush;

use super::Sim;
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
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
    /// Per unit type: sight range in cells without the downhill bonus
    /// (`radius` includes it, prompt 060).
    #[serde(default)]
    base: Vec<i32>,
    /// Ray steps computed in the last tick (benchmark).
    pub(super) last_steps: u32,
    /// Per player slot (prompt 037): bit `t` = slot `t` is an ally (or the
    /// slot itself), whose units this player always sees.
    allies: Vec<u64>,
    /// Per player slot: bit `t` = this player sees what slot `t` sees
    /// (itself, plus its allies with shared vision).
    shares: Vec<u64>,
    /// Infantry ambush (prompt 056).
    #[serde(default)]
    pub(super) ambush: ambush::Ambush,
}

/// Most players a match can have (one bit each in [`Vision::seen`]).
pub const MAX_PLAYERS: usize = 64;

impl Vision {
    /// `eye_m` and `categories` per unit type.
    pub(super) fn new(
        rules: &VisionRules,
        map: &crate::map::Map,
        players: Vec<PlayerId>,
        teams: &crate::teams::Teams,
        types: &[(i32, Option<crate::catalog::Category>)],
        units: usize,
    ) -> Self {
        let mask_of = |with: &dyn Fn(PlayerId) -> bool| {
            players
                .iter()
                .enumerate()
                .filter(|&(_, &p)| with(p))
                .fold(0u64, |m, (t, _)| m | (1 << t))
        };
        let allies: Vec<u64> = players
            .iter()
            .map(|&p| mask_of(&|q| teams.allied(p, q)))
            .collect();
        let shares: Vec<u64> = players
            .iter()
            .map(|&p| mask_of(&|q| p == q || (teams.shared_vision && teams.allied(p, q))))
            .collect();
        let base: Vec<i32> = types.iter().map(|&(_, c)| rules.range_cells(c)).collect();
        // Ray tables also reach as far as the largest downhill bonus
        // (prompt 060).
        let radius: Vec<i32> = base.iter().map(|&r| rules.height.reach_cells(r)).collect();
        let tables: Vec<i32> = base.iter().chain(&radius).copied().collect();
        let grid = VisionGrid::new(map, rules, &tables);
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
            base,
            last_steps: 0,
            allies,
            shares,
            ambush: ambush::Ambush::default(),
        }
    }

    /// Whether the player in `slot` sees vision cell `cell`, with the eyes
    /// of the allies it shares vision with.
    fn slot_sees(&self, slot: usize, cell: usize) -> bool {
        let share = self.shares[slot];
        self.per_player
            .iter()
            .enumerate()
            .any(|(t, pv)| share & (1 << t) != 0 && pv.visible(cell))
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

    /// Who sees unit `j` (bit per player slot) and, per slot, its allies
    /// and the slots whose eyes it uses: for camouflage (prompt 055).
    pub(super) fn seen_by(&self, j: usize) -> u64 {
        self.seen.get(j).copied().unwrap_or(0)
    }

    pub(super) fn set_seen_by(&mut self, j: usize, mask: u64) {
        if let Some(m) = self.seen.get_mut(j) {
            *m = mask;
        }
    }

    /// `(slot, allies, shares)` of `player`.
    pub(super) fn slot_masks(&self, player: PlayerId) -> Option<(usize, u64, u64)> {
        let s = self.slot(player)?;
        Some((s, self.allies[s], self.shares[s]))
    }

    /// Players in slot order.
    pub(super) fn slot_players(&self) -> &[PlayerId] {
        &self.players
    }

    /// Adds a viewshed that belongs to no unit (a reconnaissance aircraft,
    /// prompt 033) to the vision of `player`.
    pub(super) fn add_air_shed(&mut self, player: PlayerId, shed: &Viewshed) {
        if let Some(s) = self.slot(player) {
            self.per_player[s].add(&self.grid, shed);
        }
    }

    /// Takes unit `i`'s viewshed out of the vision of `owner` (prompt
    /// 190: before the unit changes hands); the next update computes it
    /// for the new owner.
    pub(super) fn release_shed(&mut self, i: usize, owner: PlayerId) {
        if let (Some(shed), Some(s)) = (
            self.sheds.get_mut(i).and_then(Option::take),
            self.slot(owner),
        ) {
            self.per_player[s].remove(&self.grid, &shed);
        }
    }

    /// Removes a viewshed added by [`Self::add_air_shed`].
    pub(super) fn remove_air_shed(&mut self, player: PlayerId, shed: &Viewshed) {
        if let Some(s) = self.slot(player) {
            self.per_player[s].remove(&self.grid, shed);
        }
    }

    /// Camouflage (prompt 052): unit `i` of `owner` stays visible only to
    /// the owner's allies and to the players who share the vision of one
    /// of `spotters`.
    pub(super) fn restrict(&mut self, i: usize, owner: PlayerId, spotters: &[PlayerId]) {
        let Some(o) = self.slot(owner) else {
            return;
        };
        let spotted = spotters
            .iter()
            .filter_map(|&p| self.slot(p))
            .fold(0u64, |m, s| m | (1 << s));
        let keep = (0..self.players.len()).fold(0u64, |m, s| {
            if self.allies[s] & (1 << o) != 0 || self.shares[s] & spotted != 0 {
                m | (1 << s)
            } else {
                m
            }
        });
        self.seen[i] &= keep;
    }

    /// Lets `player`, and everyone who shares its vision, see unit `i`
    /// until the next vision update (prompt 040: muzzle flashes at night).
    pub(super) fn reveal(&mut self, i: usize, player: PlayerId) {
        if let Some(s) = self.slot(player) {
            for t in 0..self.players.len() {
                if self.shares[t] & (1 << s) != 0 {
                    self.seen[i] |= 1 << t;
                }
            }
        }
    }

    /// Makes sure viewsheds of `radius` cells can be computed (prompt 033:
    /// reconnaissance aircraft).
    pub(super) fn ensure_radius(&mut self, radius: i32) {
        if !self.grid.has_table(radius) {
            Arc::make_mut(&mut self.grid).add_table(radius);
        }
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
        self.ambush.hash_into(h, &self.rules.ambush);
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
            let base = v.base.get(t).copied().unwrap_or(v.radius[t]);
            let radius = grid.downhill_radius(anchor, eye, base, v.radius[t]);
            let slot = v.slot(unit.owner).expect("owners are players");
            let (shed, steps) = if let Some(mut old) = v.sheds[i].take() {
                v.per_player[slot].remove(&grid, &old);
                let steps = grid.viewshed_downhill_into(anchor, eye, radius, base, &mut old);
                (old, steps)
            } else {
                grid.viewshed_downhill(anchor, eye, radius, base)
            };
            v.per_player[slot].add(&grid, &shed);
            v.sheds[i] = Some(shed);
            spent = spent.saturating_add(steps);
            done += 1;
        }
        v.last_steps = spent;
        let mut windows: Vec<usize> = Vec::new();
        for (i, unit) in self.units.iter().enumerate() {
            v.seen[i] = if super::transport::aboard(unit) {
                // Passengers are hidden in their vehicle (prompt 034): only
                // their owner and the owner's allies (prompt 037) see them.
                v.slot(unit.owner).map_or(0, |owner| {
                    (0..v.players.len()).fold(1u64 << owner, |mask, s| {
                        if v.allies[s] & (1 << owner) != 0 {
                            mask | (1 << s)
                        } else {
                            mask
                        }
                    })
                })
            } else if unit.alive() {
                let cell = grid.index_of(unit.position) as usize;
                // Squads in a building are seen at their windows (prompt 022).
                windows.clear();
                if let Some(squad) = &unit.squad
                    && squad.garrison.is_some()
                {
                    windows.extend(
                        squad
                            .soldiers
                            .iter()
                            .filter(|x| x.alive())
                            .map(|x| grid.index_of(x.position) as usize),
                    );
                }
                // Allies always see each other (prompt 037); with shared
                // vision a player sees what their allies see.
                let owner = v.slot(unit.owner).expect("owners are players");
                let visible: u64 = v
                    .per_player
                    .iter()
                    .enumerate()
                    .filter(|(_, pv)| pv.visible(cell) || windows.iter().any(|&c| pv.visible(c)))
                    .fold(0, |m, (t, _)| m | (1 << t));
                (0..v.players.len()).fold(0u64, |mask, s| {
                    if v.allies[s] & (1 << owner) != 0 || v.shares[s] & visible != 0 {
                        mask | (1 << s)
                    } else {
                        mask
                    }
                })
            } else {
                u64::MAX
            };
        }
        // Camouflage nets (prompt 052) hide units from distant observers.
        self.restrict_camouflaged();
        // Infantry in woods and buildings (prompt 056).
        self.update_ambush();
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
            .is_some_and(|s| self.vision.slot_sees(s, cell))
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

    /// Sight range of unit `unit` in game meters on level ground, as the
    /// vision update uses it (weather already applied; the downhill bonus
    /// of prompt 060 comes on top). `None` for unknown units.
    pub fn sight_range(&self, unit: UnitId) -> Option<Fixed> {
        let j = self.units.binary_search_by_key(&unit, |u| u.id).ok()?;
        let t = self.units[j].unit_type as usize;
        let v = &self.vision;
        let cells = v.base.get(t).or_else(|| v.radius.get(t)).copied()?;
        Some(Fixed::from_num(cells) * v.rules.cell_m())
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
                let v = &self.vision;
                let shared: Vec<&PlayerVision> = v
                    .per_player
                    .iter()
                    .enumerate()
                    .filter(|&(t, _)| v.shares[s] & (1 << t) != 0)
                    .map(|(_, pv)| pv)
                    .collect();
                if let [pv] = shared[..] {
                    out.extend((0..cells).map(|i| pv.state(i)));
                } else {
                    // Shared vision (prompt 037): the best state of the team.
                    out.extend((0..cells).map(|i| {
                        shared
                            .iter()
                            .map(|pv| pv.state(i))
                            .max_by_key(|&state| match state {
                                CellState::Unexplored => 0,
                                CellState::Explored => 1,
                                CellState::Visible => 2,
                            })
                            .unwrap_or(CellState::Unexplored)
                    }));
                }
            }
            None => out.resize(cells, CellState::Unexplored),
        }
    }

    /// Changes whenever a cell of `player` changes its state.
    pub fn vision_version(&self, player: PlayerId) -> u64 {
        let v = &self.vision;
        v.slot(player).map_or(0, |s| {
            v.per_player
                .iter()
                .enumerate()
                .filter(|&(t, _)| v.shares[s] & (1 << t) != 0)
                .fold(0u64, |sum, (_, pv)| sum.wrapping_add(pv.version))
        })
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
