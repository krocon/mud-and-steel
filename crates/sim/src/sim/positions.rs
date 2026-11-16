//! Positions tier 2 (prompt 052) on top of the field works of prompt 035.
//! Rules: `crate::positions` (`[positions]` of `data/works.toml`).
//!
//! - **Zigzag trenches** (`super::works` cuts them): every second corner
//!   beside the line; corners and walls of finished trenches and MG nests
//!   are firing positions for standing squads ([`Sim::trench_spots`]).
//! - **Parapets** stop direct fire only from afar and from low angles
//!   ([`Sim::trench_exposed`]); MG nests and dugouts cover better
//!   ([`Sim::class_cover_pct`]).
//! - **Vehicle pits:** a vehicle standing in one for `enter_s` is dug in:
//!   only its turret can be hit ([`Sim::below_parapet`],
//!   [`Sim::dug_in_aim`]); it leaves the pit by driving off.
//! - **Camouflage nets:** standing units under a net that did not fire
//!   lately are seen only by enemy ground units nearby
//!   ([`Sim::restrict_camouflaged`], after the vision update); the works
//!   under a net likewise ([`Sim::work_seen`]).
//! - **Bunkers** at trench edges become the faction's fortification.
//! - **Strongpoints:** map markers `strongpoint_<name>` become neutral,
//!   finished works in the first tick.
//!
//! The state ([`Positions`]) is part of `Works`: saved games keep it, and
//! it enters the state hash once a match used anything of this prompt.

use super::combat::TypeCombat;
use super::craters::nearest_sample;
use super::works::{Work, mm, nearest_on};
use super::{Route, Sim};
use crate::combat::{CombatEvent, FireMode, Vec3};
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::infantry::Stance;
use crate::math::{Fixed, Vec2};
use crate::positions::{NEUTRAL_OWNER, PositionRules, strongpoint_layout};
use crate::works::WorkKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Sample classes of [`Positions::classes`].
const NEST: u8 = 1;
const DUGOUT: u8 = 2;

/// State of prompt 052.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Positions {
    /// Something of this prompt was used (only then is the state hashed).
    pub(super) used: bool,
    /// Map samples of MG nests and dugouts and their class.
    pub(super) classes: BTreeMap<u32, u8>,
    /// Vehicles standing at a finished pit: (pit work id, ticks so far).
    pits: BTreeMap<UnitId, (u32, u32)>,
    /// Units under a camouflage net: the last tick they fired.
    fired: BTreeMap<UnitId, u64>,
    /// Dug works a shell struck (the presentation tears their parapet).
    #[serde(default)]
    damaged: BTreeSet<u32>,
}

impl Positions {
    pub(super) fn hash_into(&self, h: &mut StableHasher, rules: &PositionRules) {
        if !self.used {
            return;
        }
        h.write_u8(0x52);
        rules.hash_into(h);
        h.write_u64(self.classes.len() as u64);
        for (s, c) in &self.classes {
            h.write_u32(*s);
            h.write_u8(*c);
        }
        h.write_u64(self.pits.len() as u64);
        for (id, (pit, ticks)) in &self.pits {
            h.write_u32(id.0);
            h.write_u32(*pit);
            h.write_u32(*ticks);
        }
        h.write_u64(self.fired.len() as u64);
        for (id, tick) in &self.fired {
            h.write_u32(id.0);
            h.write_u64(*tick);
        }
        h.write_u64(self.damaged.len() as u64);
        for id in &self.damaged {
            h.write_u32(*id);
        }
    }
}

/// Offset of zigzag corner `k` of `n` pieces in units of the amplitude:
/// 0 at both ends and at every second corner, else alternately +1 and -1,
/// counted from the middle so the middle of the line stays on it.
pub(super) fn zigzag_side(k: u32, n: u32) -> Fixed {
    if k == 0 || k >= n {
        return Fixed::ZERO;
    }
    match (i64::from(k) - i64::from(n / 2)).rem_euclid(4) {
        1 => Fixed::ONE,
        3 => -Fixed::ONE,
        _ => Fixed::ZERO,
    }
}

/// The sample class a finished dig of `kind` leaves.
pub(super) const fn sample_class(kind: WorkKind) -> Option<u8> {
    match kind {
        WorkKind::MgNest => Some(NEST),
        WorkKind::Dugout => Some(DUGOUT),
        _ => None,
    }
}

/// Kinds that may only stand at a trench edge.
pub(super) const fn needs_trench(kind: WorkKind) -> bool {
    matches!(kind, WorkKind::MgNest | WorkKind::Dugout | WorkKind::Bunker)
}

const fn is_trench(kind: WorkKind) -> bool {
    matches!(kind, WorkKind::Trench | WorkKind::CommTrench)
}

impl Sim {
    /// Whether a finished trench of `player`'s team (or a neutral one of a
    /// strongpoint) lies within `edge_mm` of `p`.
    pub(super) fn near_trench(&self, player: PlayerId, p: Vec2) -> bool {
        let edge = mm(self.works.rules.positions.edge_mm);
        self.works.works.iter().any(|w| {
            w.built
                && is_trench(w.kind)
                && (w.owner == NEUTRAL_OWNER || self.teams.allied(w.owner, player))
                && (nearest_on(w.from, w.to, p) - p).length() <= edge
        })
    }

    /// The fortification type a bunker of `player` built by unit `builder`
    /// becomes: the first of `positions.bunkers` with the faction's prefix
    /// (the player's faction, else the builder's catalog prefix).
    pub(super) fn bunker_type(&self, player: PlayerId, builder: usize) -> Option<u32> {
        let name = &self.unit_types[self.units[builder].unit_type as usize].name;
        let faction = self
            .faction(player)
            .map(str::to_string)
            .or_else(|| name.split('_').next().map(str::to_string))?;
        let prefix = format!("{faction}_");
        let id = self
            .works
            .rules
            .positions
            .bunkers
            .iter()
            .find(|b| b.starts_with(&prefix))?;
        self.unit_types
            .iter()
            .position(|t| &t.name == id)
            .and_then(|t| u32::try_from(t).ok())
    }

    /// The finished bunker site `k` becomes a completed fortification of
    /// its owner; the site is removed.
    pub(super) fn finish_bunker(&mut self, k: usize) {
        let w = self.works.works.remove(k);
        let builder = self
            .units
            .iter()
            .position(|u| u.owner == w.owner && self.is_engineer_type(u.unit_type));
        let Some(t) = builder.and_then(|b| self.bunker_type(w.owner, b)) else {
            self.economy.refund(w.owner, w.paid);
            return;
        };
        let facing = super::works::direction(w.from, w.to);
        self.add_built_structure(w.owner, t, w.from, facing);
    }

    /// Samples a pit, nest or dugout at `center` lowers (besides the
    /// center's own).
    pub(super) fn dig_area(&self, kind: WorkKind, center: Vec2) -> Vec<usize> {
        let r = &self.works.rules.positions;
        let radius = match kind {
            WorkKind::VehiclePit => r.pit_radius_mm,
            WorkKind::MgNest => r.nest_radius_mm,
            WorkKind::Dugout => r.dugout_radius_mm,
            _ => return Vec::new(),
        };
        let radius = mm(radius);
        let step = Fixed::from_num(self.map.cell_size_mm()) / 1000;
        let n = (radius / step).ceil().to_num::<i32>();
        let mut out = Vec::new();
        for dy in -n..=n {
            for dx in -n..=n {
                let off = Vec2::new(step * Fixed::from_num(dx), step * Fixed::from_num(dy));
                let p = center + off;
                if off.length() <= radius
                    && self.ground_for_works(p)
                    && let Some(s) = nearest_sample(&self.map, p)
                {
                    out.push(s);
                }
            }
        }
        out
    }

    /// Cover of a soldier on an MG nest or dugout sample.
    pub(super) fn class_cover_pct(&self, sample: u32, stance: Stance) -> Option<u32> {
        let class = *self.works.positions.classes.get(&sample)?;
        let r = &self.works.rules.positions;
        let pct = if class == DUGOUT {
            r.dugout_pct
        } else {
            r.nest_pct
        };
        Some(match stance {
            Stance::Stand => pct[0],
            Stance::Kneel => pct[1],
            Stance::Prone => pct[2],
        })
    }

    /// Whether the round of `traveled` meters with velocity `d` hits soldier
    /// `k` of squad `j` over the parapet of a dug position: shot from
    /// close by or falling steeply. Only for samples engineers dug.
    pub(super) fn trench_exposed(&self, j: usize, k: usize, traveled: Fixed, d: Vec3) -> bool {
        if self.works.works.is_empty() {
            return false;
        }
        let Some(squad) = &self.units[j].squad else {
            return false;
        };
        let Some(sample) = nearest_sample(&self.map, squad.soldiers[k].position) else {
            return false;
        };
        if !self.works.dug.contains(&(sample as u32)) {
            return false;
        }
        let r = &self.works.rules.positions;
        let close = traveled < mm(r.close_mm);
        let run = d.ground().length();
        let steep = -d.z * 100 > run * Fixed::from_num(r.steep_pct);
        close || steep
    }

    /// Firing positions of finished trenches and MG nests within `half` of
    /// `center` on both axes: every corner, then every `spot_spacing_mm`
    /// along the pieces, `wall_mm` towards `facing` (the front wall).
    pub(super) fn trench_spots(&self, center: Vec2, half: Fixed, facing: Vec2) -> Vec<Vec2> {
        let r = &self.works.rules.positions;
        let wall = facing.scale(mm(r.wall_mm));
        let spacing = mm(r.spot_spacing_mm);
        let inside = |p: Vec2| (p.x - center.x).abs() <= half && (p.y - center.y).abs() <= half;
        let mut corners = Vec::new();
        let mut walls = Vec::new();
        for w in &self.works.works {
            if !w.built || !matches!(w.kind, WorkKind::Trench | WorkKind::MgNest) {
                continue;
            }
            if w.kind == WorkKind::MgNest {
                if inside(w.from) {
                    corners.push(w.from + wall);
                }
                continue;
            }
            let near = nearest_on(w.from, w.to, center);
            if (near.x - center.x).abs() > half || (near.y - center.y).abs() > half {
                continue;
            }
            for p in [w.from, w.to] {
                if inside(p) {
                    corners.push(p + wall);
                }
            }
            let length = (w.to - w.from).length();
            let n = (length / spacing).floor().to_num::<u32>();
            for s in 1..n {
                let p = w.from + (w.to - w.from).scale(Fixed::from_num(s) / Fixed::from_num(n));
                if inside(p) {
                    walls.push(p + wall);
                }
            }
        }
        corners.sort_unstable_by_key(|p| (p.x, p.y));
        corners.dedup();
        corners.extend(walls);
        corners
    }

    /// Hit box height (m above the unit's ground) below which dug-in unit
    /// `j` is hidden by its pit, if it is dug in.
    fn dug_in_floor(&self, j: usize, ty: &TypeCombat) -> Option<Fixed> {
        let unit = &self.units[j];
        let &(_, ticks) = self.works.positions.pits.get(&unit.id)?;
        if ticks < self.works.ticks(self.works.rules.positions.enter_s) {
            return None;
        }
        let pct = self.combat_rules.penetration.turret_height_pct.min(100);
        Some(ty.height * Fixed::from_num(100 - pct) / 100)
    }

    /// Whether a hit at `point` on unit `j` strikes the parapet of its
    /// vehicle pit instead (the hull of a dug-in vehicle).
    pub(super) fn below_parapet(&self, j: usize, point: Vec3) -> bool {
        if self.works.positions.pits.is_empty() {
            return false;
        }
        let ty = &self.types[self.units[j].unit_type as usize];
        self.dug_in_floor(j, ty)
            .is_some_and(|floor| point.z - self.units[j].height < floor)
    }

    /// Where shooters aim at dug-in unit `t`: the middle of the turret.
    pub(super) fn dug_in_aim(&self, t: usize) -> Option<Vec3> {
        if self.works.positions.pits.is_empty() {
            return None;
        }
        let u = &self.units[t];
        let ty = &self.types[u.unit_type as usize];
        let floor = self.dug_in_floor(t, ty)?;
        let z = u.height + floor + (ty.height - floor) / 2;
        Some(Vec3::new(u.position.x, u.position.y, z))
    }

    /// Ids of the vehicles that are dug in now.
    pub fn dug_in(&self) -> Vec<UnitId> {
        let enter = self.works.ticks(self.works.rules.positions.enter_s);
        self.works
            .positions
            .pits
            .iter()
            .filter(|(_, (_, t))| *t >= enter)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Whether unit `i` stands under a finished camouflage net of its team.
    fn under_net(&self, i: usize) -> bool {
        let u = &self.units[i];
        let radius = mm(self.works.rules.positions.net_radius_mm);
        self.works.works.iter().any(|w| {
            w.built
                && w.kind == WorkKind::CamoNet
                && self.teams.allied(w.owner, u.owner)
                && (w.from - u.position).length() <= radius
        })
    }

    /// Whether unit `i` is camouflaged now: under a net, standing, and
    /// silent for `fire_reveal_s`.
    pub(super) fn camouflaged(&self, i: usize) -> bool {
        let u = &self.units[i];
        if !u.alive() || u.route != Route::None || u.link.is_some() {
            return false;
        }
        let quiet = self.works.ticks(self.works.rules.positions.fire_reveal_s);
        let fired = self
            .works
            .positions
            .fired
            .get(&u.id)
            .is_some_and(|&t| self.tick < t + u64::from(quiet));
        !fired && self.under_net(i)
    }

    fn has_nets(&self) -> bool {
        self.works
            .works
            .iter()
            .any(|w| w.built && w.kind == WorkKind::CamoNet)
    }

    /// Camouflaged units are seen only by their team and by players with a
    /// ground unit within `spot_mm` that sees them (never from the air).
    /// Runs after the vision update.
    pub(super) fn restrict_camouflaged(&mut self) {
        if !self.works.positions.used || !self.has_nets() {
            return;
        }
        let spot = mm(self.works.rules.positions.spot_mm);
        let mut near = Vec::new();
        for i in 0..self.units.len() {
            if !self.camouflaged(i) {
                continue;
            }
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
                        && self.unit_sees(k, p)
                })
                .map(|k| self.units[k].owner)
                .collect();
            spotters.sort_unstable();
            spotters.dedup();
            self.vision.restrict(i, owner, &spotters);
        }
    }

    /// Whether `player` sees work `w`: everything except finished works
    /// under a hostile camouflage net (and the net itself) without an own
    /// unit within `spot_mm`.
    pub(super) fn work_seen(&self, w: &Work, player: PlayerId) -> bool {
        if !self.works.positions.used
            || w.owner == NEUTRAL_OWNER
            || self.teams.allied(w.owner, player)
        {
            return true;
        }
        let radius = mm(self.works.rules.positions.net_radius_mm);
        let covered = self.works.works.iter().any(|n| {
            n.built
                && n.kind == WorkKind::CamoNet
                && self.teams.allied(n.owner, w.owner)
                && (nearest_on(w.from, w.to, n.from) - n.from).length() <= radius
        });
        if !covered {
            return true;
        }
        let spot = mm(self.works.rules.positions.spot_mm);
        let at = w.from + (w.to - w.from).scale(Fixed::ONE / 2);
        self.units.iter().any(|u| {
            u.alive() && self.teams.allied(u.owner, player) && (u.position - at).length() <= spot
        })
    }

    /// One tick of prompt 052 (end of `update_works`): strongpoints in the
    /// first tick, shots under nets, vehicles in pits.
    pub(super) fn update_positions(&mut self) {
        if self.tick == 0 {
            self.place_strongpoints();
        }
        if !self.works.positions.used {
            return;
        }
        self.note_craters();
        self.note_shots();
        self.update_pits();
    }

    /// Dug works within reach of last tick's shell craters are damaged.
    fn note_craters(&mut self) {
        let hits: Vec<(Vec2, Fixed)> = self
            .crater_events()
            .iter()
            .filter(|e| e.caliber_mm_x10 > 0)
            .map(|e| (e.at, e.radius + Fixed::from_num(2)))
            .collect();
        for (at, reach) in hits {
            let ids: Vec<u32> = self
                .works
                .works
                .iter()
                .filter(|w| w.built && w.kind.is_dig())
                .filter(|w| (nearest_on(w.from, w.to, at) - at).length() <= reach)
                .map(|w| w.id)
                .collect();
            self.works.positions.damaged.extend(ids);
        }
    }

    /// Ids of dug works a shell struck (prompt 052, presentation).
    pub fn damaged_works(&self) -> Vec<u32> {
        self.works.positions.damaged.iter().copied().collect()
    }

    /// Units of `player`'s team that are camouflaged now.
    pub fn camouflaged_units(&self, player: PlayerId) -> Vec<UnitId> {
        if !self.works.positions.used || !self.has_nets() {
            return Vec::new();
        }
        (0..self.units.len())
            .filter(|&i| self.teams.allied(self.units[i].owner, player) && self.camouflaged(i))
            .map(|i| self.units[i].id)
            .collect()
    }

    /// Remembers when units under a camouflage net fired.
    fn note_shots(&mut self) {
        if !self.has_nets() && self.works.positions.fired.is_empty() {
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
            if let Some(i) = self.works_unit_index(id)
                && self.under_net(i)
            {
                self.works.positions.fired.insert(id, self.tick);
            }
        }
        let quiet = u64::from(self.works.ticks(self.works.rules.positions.fire_reveal_s));
        let tick = self.tick;
        self.works.positions.fired.retain(|_, t| tick < *t + quiet);
    }

    fn works_unit_index(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    /// Vehicles standing in a finished pit of their team count the ticks
    /// until they are dug in; driving off (a route) ends it.
    fn update_pits(&mut self) {
        let reach = mm(self.works.rules.positions.pit_reach_mm);
        let pits: Vec<(u32, PlayerId, Vec2)> = self
            .works
            .works
            .iter()
            .filter(|w| w.built && w.kind == WorkKind::VehiclePit)
            .map(|w| (w.id, w.owner, w.from))
            .collect();
        let mut next: BTreeMap<UnitId, (u32, u32)> = BTreeMap::new();
        let mut near = Vec::new();
        for (pit, owner, at) in pits {
            self.unit_grid.query_sorted(
                at - Vec2::new(reach, reach),
                at + Vec2::new(reach, reach),
                &mut near,
            );
            let found = near.iter().map(|&k| k as usize).find(|&k| {
                let u = &self.units[k];
                u.alive()
                    && u.squad.is_none()
                    && u.link.is_none()
                    && u.speed_offroad > Fixed::ZERO
                    && u.route == Route::None
                    && (owner == NEUTRAL_OWNER || self.teams.allied(owner, u.owner))
                    && (u.position - at).length() <= reach
                    && !next.contains_key(&u.id)
            });
            if let Some(k) = found {
                let id = self.units[k].id;
                let ticks = match self.works.positions.pits.get(&id) {
                    Some(&(p, t)) if p == pit => t.saturating_add(1),
                    _ => 1,
                };
                next.insert(id, (pit, ticks));
            }
        }
        self.works.positions.pits = next;
    }

    /// Markers `strongpoint_<name>` become neutral, finished works (first
    /// tick only).
    fn place_strongpoints(&mut self) {
        let prefix = self.works.rules.positions.strongpoint_prefix.clone();
        let centers: Vec<Vec2> = self
            .map
            .markers()
            .iter()
            .filter(|m| m.name.starts_with(&prefix))
            .map(|m| m.position)
            .collect();
        for center in centers {
            self.place_strongpoint(center);
        }
    }

    /// Builds a neutral strongpoint around `center` at once (map markers;
    /// tools and the debug menu). Returns the number of works placed.
    pub fn place_strongpoint(&mut self, center: Vec2) -> usize {
        if !super::within_world(center) {
            return 0;
        }
        let layout = strongpoint_layout(&self.works.rules.positions, center);
        let mut placed = 0;
        for piece in layout {
            let pieces = self.pieces(NEUTRAL_OWNER, piece.kind, piece.from, piece.to, center);
            for (a, b, _) in pieces {
                self.add_finished_work(piece.kind, NEUTRAL_OWNER, a, b);
                placed += 1;
            }
        }
        if placed > 0 {
            self.works.positions.used = true;
        }
        placed
    }

    /// Adds a finished work of `kind` for `owner` at once, like a
    /// strongpoint (tools, screenshots and the debug menu; never for player
    /// commands: a match with it no longer replays). Bunkers are refused.
    pub fn place_finished_work(
        &mut self,
        kind: WorkKind,
        owner: PlayerId,
        from: Vec2,
        to: Vec2,
    ) -> usize {
        if kind == WorkKind::Bunker || kind.is_mines() || !super::within_world(from) {
            return 0;
        }
        let pieces = self.pieces(owner, kind, from, to, from);
        let n = pieces.len();
        for (a, b, _) in pieces {
            self.add_finished_work(kind, owner, a, b);
        }
        self.works.positions.used |= n > 0;
        n
    }

    /// Bunkers engineers built become completed structures.
    pub(super) fn add_built_structure(
        &mut self,
        owner: PlayerId,
        unit_type: u32,
        position: Vec2,
        facing: Vec2,
    ) {
        self.add_completed_structure(owner, unit_type, position, facing, FireMode::FireAtWill);
    }

    /// The strongpoint layout around `center` with this match's rules
    /// (presentation previews).
    pub fn strongpoint_preview(&self, center: Vec2) -> Vec<crate::positions::LayoutPiece> {
        strongpoint_layout(&self.works.rules.positions, center)
    }
}
