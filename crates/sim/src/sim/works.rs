//! Field works of engineers (prompt 035): mines, wire, anti-tank
//! obstacles, foxholes and trenches, clearing, demolishing and repairing
//! bridges. Rules and public types: `crate::works`.
//!
//! - **Sites:** an order (`Lay`, `Dig`) is cut into pieces (a mine
//!   section, a wire piece, a hedgehog, a foxhole, a trench piece). Every
//!   piece is a site that is paid for when it is placed and built by the
//!   engineer squads of the order, one after the other; every squad within
//!   reach adds one tick of work per tick. A finished mine section becomes
//!   single mines, finished wire and hedgehogs block path cells (two flags
//!   of the shared `PathGrid`), foxholes and trenches lower the ground step
//!   by step through the crater machinery of prompt 021.
//! - **Mines** are not units and never appear in a snapshot. Each knows its
//!   owner (`None`: placed with the map) and which players found it (a bit
//!   mask); knowledge is shared within a team. Mines never go off under
//!   the owner's team. Searches of a player avoid the known hostile mines
//!   (`PathGrid` stays shared: the cells are passed per search).
//! - **Jobs:** one per engineer squad: build a list of sites, clear around
//!   a point, demolish or repair a bridge. Orders that move the squad
//!   otherwise end the job.
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::update_works`] after the
//! unit grid is rebuilt behind the movement (jobs, sites, mines, wire
//! crushed by vehicles, tracks repaired), [`Sim::apply_digs`] right after
//! the craters (the ground of the dig steps of this tick).

use super::craters::{CraterPlan, nearest_sample, sample_point};
use super::{Route, Sim, stop, transport, within_world};
use crate::combat::{CombatEvent, HitKind, Vec3};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::craters::CraterEvent;
use crate::data::UnitType;
use crate::economy::Resources;
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::Blocks;
use crate::path::CellRect;
use crate::rng::Rng;
use crate::works::{
    MAP_MINEFIELD_AP, MAP_MINEFIELD_AT, MINE_SHOOTER, MineKind, MineView, WorkKind, WorkView,
    WorksRules, catalog_section,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Edge of the buckets of the mine index (m).
const BUCKET_M: i32 = 16;
/// A squad that cannot get a path to its work tries again after this many
/// ticks, at most `MAX_TRIES` times per site.
const RETRY_TICKS: u32 = 40;
const MAX_TRIES: u32 = 5;
/// Engineers look for hostile mines every this many ticks.
const DETECT_INTERVAL: u64 = 10;
/// Radius of a hedgehog (m, `data/objects.toml` `fort_hedgehog`).
const HEDGEHOG_RADIUS_MM: i32 = 800;

/// A site or a finished work.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Work {
    pub(super) id: u32,
    pub(super) kind: WorkKind,
    pub(super) owner: PlayerId,
    pub(super) from: Vec2,
    pub(super) to: Vec2,
    /// Ticks of work done and needed.
    done: u32,
    total: u32,
    pub(super) paid: Resources,
    /// Ticks without an engineer job for it.
    idle: u32,
    /// Engineers worked on it in the last tick.
    worked: bool,
    pub(super) built: bool,
    /// Dig steps applied (foxholes, trenches).
    steps: u32,
    /// Mine positions of a mine section.
    plan: Vec<Vec2>,
}

impl Work {
    fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.id);
        self.kind.hash_into(h);
        h.write_u8(self.owner.0);
        for v in [self.from.x, self.from.y, self.to.x, self.to.y] {
            h.write_i64(v.to_bits());
        }
        for v in [self.done, self.total, self.idle, self.steps] {
            h.write_u32(v);
        }
        for v in self.paid {
            h.write_u32(v);
        }
        h.write_u8(u8::from(self.worked) | (u8::from(self.built) << 1));
        h.write_u64(self.plan.len() as u64);
        for p in &self.plan {
            h.write_i64(p.x.to_bits());
            h.write_i64(p.y.to_bits());
        }
    }
}

/// An armed mine.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Mine {
    id: u32,
    kind: MineKind,
    owner: Option<PlayerId>,
    position: Vec2,
    /// Players who found it (bit = player id).
    known: u64,
}

/// What an engineer squad is doing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Job {
    /// Build these sites in order (the first is the current one).
    Build {
        sites: Vec<u32>,
        wait: u32,
        tries: u32,
    },
    /// Clear around `center`; `left` ticks until the current item is gone.
    Clear {
        center: Vec2,
        left: Option<u32>,
        wait: u32,
        tries: u32,
    },
    /// Demolish (or repair) bridge `bridge`; `left` ticks of work.
    Bridge {
        bridge: u32,
        repair: bool,
        left: u32,
        wait: u32,
        tries: u32,
    },
}

impl Job {
    fn hash_into(&self, h: &mut StableHasher) {
        match self {
            Self::Build { sites, wait, tries } => {
                h.write_u8(0);
                h.write_u64(sites.len() as u64);
                for s in sites {
                    h.write_u32(*s);
                }
                h.write_u32(*wait);
                h.write_u32(*tries);
            }
            Self::Clear {
                center,
                left,
                wait,
                tries,
            } => {
                h.write_u8(1);
                h.write_i64(center.x.to_bits());
                h.write_i64(center.y.to_bits());
                h.write_u32(left.unwrap_or(u32::MAX));
                h.write_u32(*wait);
                h.write_u32(*tries);
            }
            Self::Bridge {
                bridge,
                repair,
                left,
                wait,
                tries,
            } => {
                h.write_u8(2);
                h.write_u32(*bridge);
                h.write_u8(u8::from(*repair));
                h.write_u32(*left);
                h.write_u32(*wait);
                h.write_u32(*tries);
            }
        }
    }
}

/// A vehicle without its track: ticks until the crew has fitted it, and
/// its speeds (per tick) to restore.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct TrackOff {
    left: u32,
    offroad: Fixed,
    road: Fixed,
}

/// Field works of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Works {
    pub(super) rules: WorksRules,
    tick_hz: u32,
    /// Per unit type: does engineer work.
    engineer: Vec<bool>,
    /// Sites and finished works, by id.
    pub(super) works: Vec<Work>,
    next_work: u32,
    /// Armed mines, by id.
    mines: Vec<Mine>,
    next_mine: u32,
    /// Mine ids by bucket.
    buckets: BTreeMap<(i32, i32), Vec<u32>>,
    jobs: BTreeMap<UnitId, Job>,
    track_off: BTreeMap<UnitId, TrackOff>,
    /// Map samples dug by engineers (cover, prompt 022).
    pub(super) dug: BTreeSet<u32>,
    /// Dig steps of this tick: (work id, step), applied after the craters.
    pending_digs: Vec<(u32, u32)>,
    /// Mines per section of each faction (from the catalog): (anti-tank,
    /// anti-personnel), each `(count, length mm, width mm)`.
    sections: BTreeMap<String, [(u32, u32, u32); 2]>,
    /// Positions tier 2 (prompt 052).
    #[serde(default)]
    pub(super) positions: super::positions::Positions,
}

pub(super) fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// The point of segment `a`..`b` nearest to `p`.
pub(super) fn nearest_on(a: Vec2, b: Vec2, p: Vec2) -> Vec2 {
    let d = b - a;
    let len2 = dot(d, d);
    if len2 == Fixed::ZERO {
        return a;
    }
    let t = (dot(p - a, d) / len2).clamp(Fixed::ZERO, Fixed::ONE);
    a + d.scale(t)
}

fn bucket(p: Vec2) -> (i32, i32) {
    let b = Fixed::from_num(BUCKET_M);
    ((p.x / b).floor().to_num(), (p.y / b).floor().to_num())
}

/// Unit vector from `a` to `b`, or +x if they are equal.
pub(super) fn direction(a: Vec2, b: Vec2) -> Vec2 {
    let d = b - a;
    let len = d.length();
    if len > Fixed::ZERO {
        Vec2::new(d.x / len, d.y / len)
    } else {
        Vec2::new(Fixed::ONE, Fixed::ZERO)
    }
}

impl Works {
    pub(super) fn new(rules: WorksRules, unit_types: &[UnitType], map: &Map, tick_hz: u32) -> Self {
        let engineer = unit_types
            .iter()
            .map(|t| rules.is_engineer(t.combat.category))
            .collect();
        let mut sections = BTreeMap::new();
        if let Some(catalog) = crate::economy::builtin_catalog() {
            for faction in ["de", "su", "us"] {
                let at = catalog_section(catalog, faction, MineKind::AntiTank);
                let ap = catalog_section(catalog, faction, MineKind::AntiPersonnel);
                if let (Some(at), Some(ap)) = (at, ap) {
                    sections.insert(faction.to_string(), [at, ap]);
                }
            }
        }
        let mut works = Self {
            rules,
            tick_hz,
            engineer,
            works: Vec::new(),
            next_work: 0,
            mines: Vec::new(),
            next_mine: 0,
            buckets: BTreeMap::new(),
            jobs: BTreeMap::new(),
            track_off: BTreeMap::new(),
            dug: BTreeSet::new(),
            pending_digs: Vec::new(),
            sections,
            positions: super::positions::Positions::default(),
        };
        works.place_map_minefields(map);
        works
    }

    /// Map objects `fort_mine_at` / `fort_mine_ap` (map editor) become
    /// sections of neutral mines. Their layout comes from a hash of the
    /// object, not from the match's random numbers.
    fn place_map_minefields(&mut self, map: &Map) {
        let kinds = map.object_kinds();
        let section = mm(self.rules.mines.section_mm);
        for (index, object) in (0u32..).zip(map.objects()) {
            let kind = match kinds[usize::from(object.kind)].id.as_str() {
                MAP_MINEFIELD_AT => MineKind::AntiTank,
                MAP_MINEFIELD_AP => MineKind::AntiPersonnel,
                _ => continue,
            };
            let count = match kind {
                MineKind::AntiTank => self.rules.mines.map_at_count,
                MineKind::AntiPersonnel => self.rules.mines.map_ap_count,
            };
            let mut h = StableHasher::new();
            h.write_u32(index);
            h.write_i64(object.position.x.to_bits());
            h.write_i64(object.position.y.to_bits());
            let mut rng = Rng::new(h.finish());
            let (forward, right) = object.axes();
            let spots = self.layout(
                &mut rng,
                count,
                (section, section),
                object.position,
                right,
                forward,
            );
            for p in spots {
                if map.contains(p) && map.deck_at(p).is_none() {
                    self.add_mine(kind, None, p);
                }
            }
        }
    }

    /// `count` mine positions on a staggered grid of `width` (along
    /// `along`) by `depth` (along `across`) around `center`, shifted by up
    /// to `jitter_mm`.
    fn layout(
        &self,
        rng: &mut Rng,
        count: u32,
        (width, depth): (Fixed, Fixed),
        center: Vec2,
        along: Vec2,
        across: Vec2,
    ) -> Vec<Vec2> {
        let mut columns = 1u32;
        while columns * columns < count {
            columns += 1;
        }
        let rows = count.div_ceil(columns);
        let jitter = i32::try_from(self.rules.mines.jitter_mm).unwrap_or(0);
        let mut out = Vec::with_capacity(count as usize);
        for r in 0..rows {
            for c in 0..columns {
                if out.len() as u32 >= count {
                    break;
                }
                // Odd rows are shifted by a quarter cell, even rows back.
                let shift = if r % 2 == 0 {
                    -Fixed::from_num(1) / 4
                } else {
                    Fixed::from_num(1) / 4
                };
                let u = (Fixed::from_num(c) + Fixed::ONE / 2 + shift) / Fixed::from_num(columns)
                    - Fixed::ONE / 2;
                let v =
                    (Fixed::from_num(r) + Fixed::ONE / 2) / Fixed::from_num(rows) - Fixed::ONE / 2;
                let jx = Fixed::from_num(rng.range_i32(-jitter, jitter)) / 1000;
                let jy = Fixed::from_num(rng.range_i32(-jitter, jitter)) / 1000;
                out.push(center + along.scale(width * u + jx) + across.scale(depth * v + jy));
            }
        }
        out
    }

    fn add_mine(&mut self, kind: MineKind, owner: Option<PlayerId>, position: Vec2) {
        let id = self.next_mine;
        self.next_mine = self.next_mine.wrapping_add(1);
        self.mines.push(Mine {
            id,
            kind,
            owner,
            position,
            known: 0,
        });
        self.buckets.entry(bucket(position)).or_default().push(id);
    }

    fn remove_mine(&mut self, id: u32) -> Option<Mine> {
        let k = self.mines.binary_search_by_key(&id, |m| m.id).ok()?;
        let mine = self.mines.remove(k);
        let key = bucket(mine.position);
        if let Some(list) = self.buckets.get_mut(&key) {
            list.retain(|&m| m != id);
            if list.is_empty() {
                self.buckets.remove(&key);
            }
        }
        Some(mine)
    }

    fn mine(&self, id: u32) -> Option<&Mine> {
        self.mines
            .binary_search_by_key(&id, |m| m.id)
            .ok()
            .map(|k| &self.mines[k])
    }

    /// Ids of the mines within `radius` of `p` (ascending).
    fn mines_near(&self, p: Vec2, radius: Fixed) -> Vec<u32> {
        let (x0, y0) = bucket(p - Vec2::new(radius, radius));
        let (x1, y1) = bucket(p + Vec2::new(radius, radius));
        let mut out = Vec::new();
        for by in y0..=y1 {
            for bx in x0..=x1 {
                if let Some(list) = self.buckets.get(&(bx, by)) {
                    for &id in list {
                        if let Some(m) = self.mine(id)
                            && (m.position - p).length() <= radius
                        {
                            out.push(id);
                        }
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    fn work_index(&self, id: u32) -> Option<usize> {
        self.works.binary_search_by_key(&id, |w| w.id).ok()
    }

    pub(super) fn ticks(&self, s: u32) -> u32 {
        s.saturating_mul(self.tick_hz).max(1)
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u32(self.next_work);
        h.write_u64(self.works.len() as u64);
        for w in &self.works {
            w.hash_into(h);
        }
        h.write_u32(self.next_mine);
        h.write_u64(self.mines.len() as u64);
        for m in &self.mines {
            h.write_u32(m.id);
            h.write_u8(match m.kind {
                MineKind::AntiTank => 0,
                MineKind::AntiPersonnel => 1,
            });
            h.write_u32(m.owner.map_or(u32::MAX, |o| u32::from(o.0)));
            h.write_i64(m.position.x.to_bits());
            h.write_i64(m.position.y.to_bits());
            h.write_u64(m.known);
        }
        h.write_u64(self.jobs.len() as u64);
        for (id, job) in &self.jobs {
            h.write_u32(id.0);
            job.hash_into(h);
        }
        h.write_u64(self.track_off.len() as u64);
        for (id, t) in &self.track_off {
            h.write_u32(id.0);
            h.write_u32(t.left);
            h.write_i64(t.offroad.to_bits());
            h.write_i64(t.road.to_bits());
        }
        h.write_u64(self.dug.len() as u64);
        for k in &self.dug {
            h.write_u32(*k);
        }
        h.write_u64(self.pending_digs.len() as u64);
        for (w, s) in &self.pending_digs {
            h.write_u32(*w);
            h.write_u32(*s);
        }
        self.positions.hash_into(h, &self.rules.positions);
    }
}

impl Sim {
    fn works_index_of(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    /// Bits of every player allied with `player` (ids below 64).
    fn team_mask(&self, player: PlayerId) -> u64 {
        self.standings
            .players()
            .iter()
            .map(|s| s.player)
            .chain([player])
            .filter(|&p| p.0 < 64 && self.teams.allied(p, player))
            .fold(0, |mask, p| mask | (1u64 << p.0))
    }

    fn mine_friendly(&self, mine: &Mine, player: PlayerId) -> bool {
        mine.owner.is_some_and(|o| self.teams.allied(o, player))
    }

    fn mine_known(&self, mine: &Mine, player: PlayerId, mask: u64) -> bool {
        self.mine_friendly(mine, player) || mine.known & mask != 0
    }

    /// Path cells a search of `player` avoids: the cells of the hostile
    /// mines the player's team knows and their neighbors (sorted,
    /// deduplicated).
    pub(super) fn avoid_cells(&self, player: PlayerId) -> Vec<u32> {
        if self.works.mines.is_empty() {
            return Vec::new();
        }
        let mask = self.team_mask(player);
        let grid = &self.paths.grid;
        let (width, height) = grid.size();
        // The cell of every mine and its neighbors: a hull reaches over the
        // edge of the cell its center drives through.
        let mut cells: Vec<u32> = self
            .works
            .mines
            .iter()
            .filter(|m| !self.mine_friendly(m, player) && m.known & mask != 0)
            .flat_map(|m| {
                let (x, y) = grid.cell_of(m.position);
                let xs = x.saturating_sub(1)..=(x + 1).min(width - 1);
                xs.flat_map(move |cx| {
                    (y.saturating_sub(1)..=(y + 1).min(height - 1)).map(move |cy| (cx, cy))
                })
            })
            .map(|(x, y)| grid.cell_index(x, y))
            .collect();
        cells.sort_unstable();
        cells.dedup();
        cells
    }

    /// [`Self::avoid_cells`] of every player with something to avoid
    /// (empty without mines).
    pub(super) fn avoid_by_player(&self) -> Vec<(PlayerId, Vec<u32>)> {
        if self.works.mines.is_empty() || self.paths.queued() == 0 {
            return Vec::new();
        }
        self.standings
            .players()
            .iter()
            .map(|s| (s.player, self.avoid_cells(s.player)))
            .filter(|(_, cells)| !cells.is_empty())
            .collect()
    }

    /// Indices of the engineer squads of `player` among `units` that can
    /// take an order.
    fn engineer_indices(&self, player: PlayerId, units: &[UnitId]) -> Vec<usize> {
        let mut out = self.mobile_indices(player, units);
        out.retain(|&i| {
            let u = &self.units[i];
            u.squad.is_some()
                && self
                    .works
                    .engineer
                    .get(u.unit_type as usize)
                    .copied()
                    .unwrap_or(false)
        });
        out
    }

    /// Gives every listed squad `job` (they leave buildings and drop what
    /// they did).
    fn assign_job(&mut self, indices: &[usize], job: &Job) {
        for &i in indices {
            self.leave_building(i);
            self.units[i].combat.attack = None;
            self.units[i].combat.attack_moving = false;
            self.cancel_indirect(i);
            self.works.jobs.insert(self.units[i].id, job.clone());
        }
    }

    /// `Lay` and `Dig`: sites along the line, paid piece by piece, built by
    /// the engineers among `units`.
    pub(super) fn order_works(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        kind: WorkKind,
        (from, to): (Vec2, Vec2),
    ) {
        if !within_world(from) || !within_world(to) {
            return;
        }
        let squads = self.engineer_indices(player, units);
        if squads.is_empty() {
            return;
        }
        // Clamp the line and start at the end nearer to the engineers.
        let max = mm(self.works.rules.engineers.max_line_mm);
        let mut to = to;
        if (to - from).length() > max {
            to = from + direction(from, to).scale(max);
        }
        let center = {
            let n = Fixed::from_num(squads.len());
            squads.iter().fold(Vec2::ZERO, |sum, &i| {
                let p = self.units[i].position;
                sum + Vec2::new(p.x / n, p.y / n)
            })
        };
        let (from, to) = if (to - center).length() < (from - center).length() {
            (to, from)
        } else {
            (from, to)
        };
        let mut pieces = self.pieces(player, kind, from, to, center);
        // Prompt 052: nests, dugouts and bunkers only at a trench edge;
        // bunkers need the faction's fortification type.
        if kind.is_positions() {
            pieces.retain(|(a, ..)| {
                !super::positions::needs_trench(kind) || self.near_trench(player, *a)
            });
            if kind == WorkKind::Bunker && self.bunker_type(player, squads[0]).is_none() {
                return;
            }
        }
        let open = self
            .works
            .works
            .iter()
            .filter(|w| w.owner == player && !w.built)
            .count();
        let room = (self.works.rules.engineers.max_sites as usize).saturating_sub(open);
        let rule = *self.works.rules.rule(kind);
        let total = self.works.ticks(rule.build_s);
        let mut ids = Vec::new();
        for (a, b, plan) in pieces.into_iter().take(room) {
            if !self.economy.pay(player, rule.cost) {
                break;
            }
            let id = self.works.next_work;
            self.works.next_work = self.works.next_work.wrapping_add(1);
            self.works.works.push(Work {
                id,
                kind,
                owner: player,
                from: a,
                to: b,
                done: 0,
                total,
                paid: rule.cost,
                idle: 0,
                worked: false,
                built: false,
                steps: 0,
                plan,
            });
            ids.push(id);
        }
        if ids.is_empty() {
            return;
        }
        self.works.positions.used |= kind.is_positions() || kind == WorkKind::Trench;
        self.assign_job(
            &squads,
            &Job::Build {
                sites: ids,
                wait: 0,
                tries: 0,
            },
        );
    }

    /// Adds a finished work at once (map strongpoints, prompt 052): holes
    /// are dug to full depth in this tick, obstacles block.
    pub(super) fn add_finished_work(
        &mut self,
        kind: WorkKind,
        owner: PlayerId,
        from: Vec2,
        to: Vec2,
    ) {
        let id = self.works.next_work;
        self.works.next_work = self.works.next_work.wrapping_add(1);
        let steps = if kind.is_dig() {
            self.works.rules.dig.steps
        } else {
            0
        };
        self.works.works.push(Work {
            id,
            kind,
            owner,
            from,
            to,
            done: 1,
            total: 1,
            paid: [0; 3],
            idle: 0,
            worked: false,
            built: true,
            steps,
            plan: Vec::new(),
        });
        if kind.is_dig() {
            self.works.pending_digs.push((id, steps));
        }
        if kind.is_obstacle() {
            let k = self.works.works.len() - 1;
            let rect = self.work_cells_rect(k);
            self.refresh_work_cells(rect);
        }
    }

    /// Whether field works may stand at `p` (on the map, on passable
    /// ground, not on a bridge deck).
    pub(super) fn ground_for_works(&self, p: Vec2) -> bool {
        self.map.contains(p)
            && self.map.deck_at(p).is_none()
            && self
                .terrain
                .types
                .get(usize::from(self.map.terrain_at(p)))
                .is_some_and(|t| t.passable)
    }

    /// The pieces of an order: `(from, to, mine positions)`, valid ground
    /// only.
    pub(super) fn pieces(
        &mut self,
        player: PlayerId,
        kind: WorkKind,
        from: Vec2,
        to: Vec2,
        engineers: Vec2,
    ) -> Vec<(Vec2, Vec2, Vec<Vec2>)> {
        let length = (to - from).length();
        let dir = direction(from, to);
        let mut out = Vec::new();
        match kind {
            WorkKind::MinesAt | WorkKind::MinesAp => {
                let mine = if kind == WorkKind::MinesAt {
                    MineKind::AntiTank
                } else {
                    MineKind::AntiPersonnel
                };
                let (count, depth, width) = self
                    .faction(player)
                    .and_then(|f| self.works.sections.get(f))
                    .map_or_else(
                        || {
                            let r = &self.works.rules.mines;
                            let n = match mine {
                                MineKind::AntiTank => r.map_at_count,
                                MineKind::AntiPersonnel => r.map_ap_count,
                            };
                            (n, r.section_mm, r.section_mm)
                        },
                        |s| s[usize::from(mine == MineKind::AntiPersonnel)],
                    );
                let (width, depth) = (mm(width), mm(depth));
                // A point order faces the engineers' side.
                let along = if length > Fixed::ZERO {
                    dir
                } else {
                    let towards = direction(engineers, from);
                    Vec2::new(-towards.y, towards.x)
                };
                let across = Vec2::new(-along.y, along.x);
                let n = (length / width).ceil().to_num::<u32>().max(1);
                for k in 0..n {
                    let t = if n == 1 && length == Fixed::ZERO {
                        Fixed::ZERO
                    } else {
                        (Fixed::from_num(k) + Fixed::ONE / 2) * width
                    };
                    let center = from + dir.scale(t.min(length.max(Fixed::ZERO)));
                    if !self.ground_for_works(center) {
                        continue;
                    }
                    let spots = self.works.layout(
                        &mut self.rng,
                        count,
                        (width, depth),
                        center,
                        along,
                        across,
                    );
                    let plan: Vec<Vec2> = spots
                        .into_iter()
                        .filter(|&p| self.ground_for_works(p))
                        .collect();
                    if !plan.is_empty() {
                        let half = along.scale(width / 2);
                        out.push((center - half, center + half, plan));
                    }
                }
            }
            WorkKind::Wire
            | WorkKind::Trench
            | WorkKind::CommTrench
            | WorkKind::Hedgehogs
            | WorkKind::Teeth => {
                let piece = mm(self.works.rules.rule(kind).piece_mm);
                let n = (length / piece).ceil().to_num::<u32>().max(1);
                // Fire trenches zigzag (prompt 052): every second corner
                // lies beside the line, the ends stay on it.
                let zigzag = if kind == WorkKind::Trench {
                    mm(self.works.rules.positions.zigzag_mm)
                } else {
                    Fixed::ZERO
                };
                let across = Vec2::new(-dir.y, dir.x);
                let point = |k: u32| {
                    let on = from + dir.scale(length * Fixed::from_num(k) / Fixed::from_num(n));
                    on + across.scale(zigzag * super::positions::zigzag_side(k, n))
                };
                if matches!(kind, WorkKind::Hedgehogs | WorkKind::Teeth) {
                    let count = if length == Fixed::ZERO { 1 } else { n + 1 };
                    for k in 0..count {
                        let p = point(k);
                        if self.ground_for_works(p) {
                            out.push((p, p, Vec::new()));
                        }
                    }
                } else if length == Fixed::ZERO {
                    if self.ground_for_works(from) {
                        out.push((from, from, Vec::new()));
                    }
                } else {
                    for k in 0..n {
                        let (a, b) = (point(k), point(k + 1));
                        let mid = a + (b - a).scale(Fixed::ONE / 2);
                        if self.ground_for_works(mid) {
                            out.push((a, b, Vec::new()));
                        }
                    }
                }
            }
            WorkKind::Foxhole
            | WorkKind::VehiclePit
            | WorkKind::MgNest
            | WorkKind::Dugout
            | WorkKind::CamoNet => {
                if self.ground_for_works(from) {
                    out.push((from, from, Vec::new()));
                }
            }
            WorkKind::Bunker => {
                // `to` gives the facing; a point order faces away from the
                // engineers.
                let facing = if length > Fixed::ZERO {
                    dir
                } else {
                    direction(engineers, from)
                };
                if self.ground_for_works(from) {
                    out.push((from, from + facing.scale(Fixed::from_num(4)), Vec::new()));
                }
            }
        }
        out
    }

    /// `Clear`: the engineers clear known mines, wire and obstacles around
    /// `target`.
    pub(super) fn order_clear(&mut self, player: PlayerId, units: &[UnitId], target: Vec2) {
        if !within_world(target) {
            return;
        }
        let squads = self.engineer_indices(player, units);
        self.assign_job(
            &squads,
            &Job::Clear {
                center: target,
                left: None,
                wait: 0,
                tries: 0,
            },
        );
    }

    /// `DemolishBridge` / `RepairBridge` of bridge `bridge`.
    pub(super) fn order_bridge(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        bridge: u32,
        repair: bool,
    ) {
        let Some(b) = self.map.bridges().get(bridge as usize) else {
            return;
        };
        let cost = if repair {
            self.works.rules.bridge.repair_cost
        } else {
            self.works.rules.bridge.demolish_cost
        };
        if b.intact == repair || !self.economy.can_pay(player, cost) {
            return;
        }
        let left = self.works.ticks(if repair {
            self.works.rules.bridge.repair_s
        } else {
            self.works.rules.bridge.demolish_s
        });
        let squads = self.engineer_indices(player, units);
        self.assign_job(
            &squads,
            &Job::Bridge {
                bridge,
                repair,
                left,
                wait: 0,
                tries: 0,
            },
        );
    }

    /// Orders that give an engineer something else to do end its job.
    pub(super) fn drop_work_jobs(&mut self, command: &PlayerCommand) {
        if !matches!(
            command.command,
            Command::MoveTo { .. }
                | Command::Stop { .. }
                | Command::Attack { .. }
                | Command::AttackGround { .. }
                | Command::Garrison { .. }
                | Command::Patrol { .. }
                | Command::QueueMove { .. }
                | Command::Embark { .. }
        ) || self.works.jobs.is_empty()
        {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            self.works.jobs.remove(&self.units[i].id);
        }
    }

    /// Walks squad `i` towards `spot`: `Some(true)` once it is within
    /// reach (it stops there), `Some(false)` on the way, `None` when no
    /// path was found after several tries.
    fn walk_to(&mut self, i: usize, spot: Vec2, wait: &mut u32, tries: &mut u32) -> Option<bool> {
        let reach = mm(self.works.rules.engineers.reach_mm);
        let unit = &mut self.units[i];
        if (unit.position - spot).length() <= reach {
            if unit.route != Route::None {
                stop(unit);
            }
            *tries = 0;
            return Some(true);
        }
        if unit.route != Route::None {
            return Some(false);
        }
        if *wait > 0 {
            *wait -= 1;
            return Some(false);
        }
        if *tries >= MAX_TRIES {
            return None;
        }
        *tries += 1;
        *wait = RETRY_TICKS;
        self.request_group_path(&[(i, spot)], spot);
        Some(false)
    }

    /// One tick of field works (see the module documentation).
    pub(super) fn update_works(&mut self) {
        for w in &mut self.works.works {
            w.worked = false;
        }
        self.advance_jobs();
        self.give_up_sites();
        self.trigger_mines();
        if self.tick.is_multiple_of(DETECT_INTERVAL) {
            self.detect_mines();
        }
        self.crush_wire();
        self.fit_tracks();
        self.update_positions();
    }

    fn advance_jobs(&mut self) {
        let ids: Vec<UnitId> = self.works.jobs.keys().copied().collect();
        for id in ids {
            let Some(job) = self.works.jobs.get(&id).cloned() else {
                continue;
            };
            let keep = match self.works_index_of(id) {
                Some(i)
                    if self.units[i].alive()
                        && !transport::aboard(&self.units[i])
                        && !super::infantry::inside(&self.units[i]) =>
                {
                    self.advance_job(i, job)
                }
                _ => None,
            };
            match keep {
                Some(job) => {
                    self.works.jobs.insert(id, job);
                }
                None => {
                    self.works.jobs.remove(&id);
                }
            }
        }
    }

    fn advance_job(&mut self, i: usize, job: Job) -> Option<Job> {
        match job {
            Job::Build {
                mut sites,
                mut wait,
                mut tries,
            } => {
                // Finished or removed sites drop out.
                while let Some(&s) = sites.first() {
                    match self.works.work_index(s) {
                        Some(k) if !self.works.works[k].built => break,
                        _ => {
                            sites.remove(0);
                            tries = 0;
                        }
                    }
                }
                let &s = sites.first()?;
                let k = self.works.work_index(s)?;
                let w = &self.works.works[k];
                let spot = nearest_on(w.from, w.to, self.units[i].position);
                match self.walk_to(i, spot, &mut wait, &mut tries) {
                    None => {
                        // Unreachable: skip this site.
                        sites.remove(0);
                        tries = 0;
                    }
                    Some(true) => self.work_on(k),
                    Some(false) => {}
                }
                (!sites.is_empty()).then_some(Job::Build { sites, wait, tries })
            }
            Job::Clear {
                center,
                left,
                mut wait,
                mut tries,
            } => {
                if !self.walk_to(i, center, &mut wait, &mut tries)? {
                    return Some(Job::Clear {
                        center,
                        left,
                        wait,
                        tries,
                    });
                }
                let owner = self.units[i].owner;
                let at = self.units[i].position;
                let left = match left {
                    None => {
                        // Clearing engineers search the whole area.
                        let radius = mm(self.works.rules.engineers.clear_radius_mm);
                        let ids = self.works.mines_near(center, radius);
                        self.learn_mines(&ids, owner);
                        Some(self.next_clear_item(owner, center, at)?.1)
                    }
                    Some(0) => {
                        if let Some((item, _)) = self.next_clear_item(owner, center, at) {
                            self.clear_item(item);
                        }
                        None
                    }
                    Some(n) => Some(n - 1),
                };
                Some(Job::Clear {
                    center,
                    left,
                    wait,
                    tries,
                })
            }
            Job::Bridge {
                bridge,
                repair,
                left,
                mut wait,
                mut tries,
            } => {
                let b = self.map.bridges().get(bridge as usize)?;
                if b.intact == repair {
                    return None;
                }
                let at = self.units[i].position;
                let start = b.center - b.forward.scale(b.half_length);
                let end = b.center + b.forward.scale(b.half_length);
                let out = Fixed::from_num(3);
                let spot = if (start - at).length() <= (end - at).length() {
                    start - b.forward.scale(out)
                } else {
                    end + b.forward.scale(out)
                };
                if !self.walk_to(i, spot, &mut wait, &mut tries)? {
                    return Some(Job::Bridge {
                        bridge,
                        repair,
                        left,
                        wait,
                        tries,
                    });
                }
                if left > 0 {
                    return Some(Job::Bridge {
                        bridge,
                        repair,
                        left: left - 1,
                        wait,
                        tries,
                    });
                }
                let owner = self.units[i].owner;
                let cost = if repair {
                    self.works.rules.bridge.repair_cost
                } else {
                    self.works.rules.bridge.demolish_cost
                };
                if !self.economy.pay(owner, cost) {
                    // Waits for the money.
                    return Some(Job::Bridge {
                        bridge,
                        repair,
                        left,
                        wait,
                        tries,
                    });
                }
                self.finish_bridge(bridge as usize, repair, self.units[i].id, owner);
                None
            }
        }
    }

    /// One tick of work on site `k`.
    fn work_on(&mut self, k: usize) {
        let w = &mut self.works.works[k];
        w.worked = true;
        w.idle = 0;
        w.done = (w.done + 1).min(w.total);
        if w.kind.is_dig() {
            let steps = self.works.rules.dig.steps;
            let due =
                u32::try_from(u64::from(w.done) * u64::from(steps) / u64::from(w.total.max(1)))
                    .unwrap_or(steps);
            if due > w.steps {
                w.steps = due;
                let id = w.id;
                self.works.pending_digs.push((id, due));
            }
        }
        if self.works.works[k].done >= self.works.works[k].total {
            self.finish_work(k);
        }
    }

    /// A site is done: mines are armed, obstacles block, holes are dug.
    fn finish_work(&mut self, k: usize) {
        let w = &mut self.works.works[k];
        w.built = true;
        match w.kind {
            WorkKind::MinesAt | WorkKind::MinesAp => {
                let kind = if w.kind == WorkKind::MinesAt {
                    MineKind::AntiTank
                } else {
                    MineKind::AntiPersonnel
                };
                let owner = w.owner;
                let plan = std::mem::take(&mut w.plan);
                self.works.works.remove(k);
                for p in plan {
                    self.works.add_mine(kind, Some(owner), p);
                }
            }
            WorkKind::Wire | WorkKind::Hedgehogs | WorkKind::Teeth => {
                let rect = self.work_cells_rect(k);
                self.refresh_work_cells(rect);
            }
            WorkKind::Bunker => self.finish_bunker(k),
            WorkKind::Foxhole
            | WorkKind::Trench
            | WorkKind::CommTrench
            | WorkKind::VehiclePit
            | WorkKind::MgNest
            | WorkKind::Dugout
            | WorkKind::CamoNet => {}
        }
    }

    /// Unfinished sites nobody works on or walks to are given up after a
    /// while (refunded if no work was done).
    fn give_up_sites(&mut self) {
        let attended: BTreeSet<u32> = self
            .works
            .jobs
            .values()
            .filter_map(|j| match j {
                Job::Build { sites, .. } => Some(sites.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        let limit = self.works.ticks(self.works.rules.engineers.abandon_s);
        let mut gone = Vec::new();
        for w in &mut self.works.works {
            if w.built {
                continue;
            }
            if attended.contains(&w.id) {
                w.idle = 0;
            } else {
                w.idle += 1;
                if w.idle >= limit {
                    gone.push(w.id);
                }
            }
        }
        for id in gone {
            if let Some(k) = self.works.work_index(id) {
                let w = self.works.works.remove(k);
                if w.done == 0 {
                    self.economy.refund(w.owner, w.paid);
                }
            }
        }
    }

    /// Path cells of a finished obstacle and the classes it blocks.
    fn cells_of(&self, w: &Work) -> Vec<((u32, u32), Blocks)> {
        let grid = &self.paths.grid;
        let mut out = Vec::new();
        match w.kind {
            WorkKind::Wire => {
                let length = (w.to - w.from).length();
                let n = length.ceil().to_num::<u32>().max(1);
                for k in 0..=n {
                    let p = w.from + (w.to - w.from).scale(Fixed::from_num(k) / Fixed::from_num(n));
                    out.push((grid.cell_of(p), Blocks::Small));
                }
            }
            WorkKind::Hedgehogs | WorkKind::Teeth => {
                let r = Fixed::from_num(HEDGEHOG_RADIUS_MM) / 1000;
                let rect = grid.cells_in(w.from - Vec2::new(r, r), w.from + Vec2::new(r, r));
                for y in rect.y0..=rect.y1 {
                    for x in rect.x0..=rect.x1 {
                        out.push(((x, y), Blocks::Large));
                    }
                }
            }
            _ => {}
        }
        out.sort_unstable_by_key(|&((x, y), _)| (y, x));
        out.dedup();
        out
    }

    fn work_cells_rect(&self, k: usize) -> CellRect {
        let w = &self.works.works[k];
        let r = Fixed::from_num(HEDGEHOG_RADIUS_MM) / 1000;
        let low = Vec2::new(w.from.x.min(w.to.x) - r, w.from.y.min(w.to.y) - r);
        let high = Vec2::new(w.from.x.max(w.to.x) + r, w.from.y.max(w.to.y) + r);
        self.paths.grid.cells_in(low, high)
    }

    /// Marks the obstacle cells of `rect` again from every finished
    /// obstacle and lets routes through them plan again.
    fn refresh_work_cells(&mut self, rect: CellRect) {
        let cells: Vec<((u32, u32), Blocks)> = self
            .works
            .works
            .iter()
            .filter(|w| w.built && w.kind.is_obstacle())
            .flat_map(|w| self.cells_of(w))
            .collect();
        let grid = &mut self.paths.grid;
        grid.mark_works_in(rect, cells);
        let area = grid.rebuild_cells(&self.map, &self.terrain, rect);
        grid.relabel_components_in(&[area]);
        self.replan_through(area);
    }

    /// The nearest thing engineers of `player` can clear within the clear
    /// radius of `center`, and its clear time in ticks.
    fn next_clear_item(
        &self,
        player: PlayerId,
        center: Vec2,
        at: Vec2,
    ) -> Option<(ClearItem, u32)> {
        let radius = mm(self.works.rules.engineers.clear_radius_mm);
        let mask = self.team_mask(player);
        let rules = &self.works.rules;
        let mut best: Option<(Fixed, ClearItem, u32)> = None;
        let mut consider = |d: Fixed, item: ClearItem, ticks: u32| {
            if best.as_ref().is_none_or(|b| d < b.0) {
                best = Some((d, item, ticks));
            }
        };
        for id in self.works.mines_near(center, radius) {
            let Some(m) = self.works.mine(id) else {
                continue;
            };
            if !self.mine_known(m, player, mask) {
                continue;
            }
            let kind = match m.kind {
                MineKind::AntiTank => WorkKind::MinesAt,
                MineKind::AntiPersonnel => WorkKind::MinesAp,
            };
            let clear = rules.rule(kind).clear_s;
            if clear > 0 {
                consider(
                    (m.position - at).length(),
                    ClearItem::Mine(id),
                    self.works.ticks(clear),
                );
            }
        }
        for w in &self.works.works {
            if !w.built || !(w.kind.is_obstacle() || w.kind == WorkKind::CamoNet) {
                continue;
            }
            let p = nearest_on(w.from, w.to, center);
            let clear = rules.rule(w.kind).clear_s;
            if clear > 0 && (p - center).length() <= radius {
                let near = nearest_on(w.from, w.to, at);
                consider(
                    (near - at).length(),
                    ClearItem::Work(w.id),
                    self.works.ticks(clear),
                );
            }
        }
        best.map(|(_, item, ticks)| (item, ticks))
    }

    fn clear_item(&mut self, item: ClearItem) {
        match item {
            ClearItem::Mine(id) => {
                self.works.remove_mine(id);
            }
            ClearItem::Work(id) => self.remove_work(id),
        }
    }

    /// Removes a finished obstacle (cleared or crushed).
    fn remove_work(&mut self, id: u32) {
        let Some(k) = self.works.work_index(id) else {
            return;
        };
        let rect = self.work_cells_rect(k);
        self.works.works.remove(k);
        self.refresh_work_cells(rect);
    }

    /// Mines under units that set them off explode.
    fn trigger_mines(&mut self) {
        if self.works.mines.is_empty() {
            return;
        }
        let rules = self.works.rules.mines;
        let ap_reach = mm(rules.ap_reach_mm);
        // (unit index, soldier, mine id), in unit order.
        let mut hits: Vec<(usize, Option<usize>, u32)> = Vec::new();
        let mut taken: BTreeSet<u32> = BTreeSet::new();
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive()
                || transport::aboard(u)
                || super::infantry::inside(u)
                || self.is_structure(u.unit_type)
            {
                continue;
            }
            let owner = u.owner;
            if let Some(squad) = &u.squad {
                for (k, s) in squad.soldiers.iter().enumerate() {
                    if !s.alive() {
                        continue;
                    }
                    for id in self.works.mines_near(s.position, ap_reach) {
                        let Some(m) = self.works.mine(id) else {
                            continue;
                        };
                        let trigger = match m.kind {
                            MineKind::AntiTank => rules.at_trigger_kg,
                            MineKind::AntiPersonnel => rules.ap_trigger_kg,
                        };
                        if rules.soldier_kg >= trigger
                            && !self.mine_friendly(m, owner)
                            && taken.insert(id)
                        {
                            hits.push((i, Some(k), id));
                        }
                    }
                }
            } else {
                let t = u.unit_type as usize;
                let (hl, hw, _, infantry) = self.hull(t);
                if infantry || hl == Fixed::ZERO {
                    continue;
                }
                let mass = self.unit_types.get(t).map_or(0, |t| t.mass_kg);
                let right = Vec2::new(-u.facing.y, u.facing.x);
                for id in self.works.mines_near(u.position, hl + hw) {
                    let Some(m) = self.works.mine(id) else {
                        continue;
                    };
                    let trigger = match m.kind {
                        MineKind::AntiTank => rules.at_trigger_kg,
                        MineKind::AntiPersonnel => rules.ap_trigger_kg,
                    };
                    let rel = m.position - u.position;
                    if mass >= trigger
                        && dot(rel, u.facing).abs() <= hl
                        && dot(rel, right).abs() <= hw
                        && !self.mine_friendly(m, owner)
                        && taken.insert(id)
                    {
                        hits.push((i, None, id));
                    }
                }
            }
        }
        for (i, soldier, id) in hits {
            if let Some(mine) = self.works.remove_mine(id) {
                self.explode_mine(i, soldier, &mine);
            }
        }
    }

    /// Mine `mine` goes off under unit `i` (soldier `soldier` of a squad).
    fn explode_mine(&mut self, i: usize, soldier: Option<usize>, mine: &Mine) {
        let rules = self.works.rules.mines;
        let p = mine.position;
        let at = Vec3::new(p.x, p.y, self.map.height_at(p));
        let victim = self.units[i].owner;
        let unit_id = self.units[i].id;
        let id = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1);
        let owner = mine.owner.unwrap_or(victim);
        match (mine.kind, soldier) {
            (MineKind::AntiTank, None) => {
                let t = self.units[i].unit_type as usize;
                let (_, _, thinnest, _) = self.hull(t);
                let max_hp = self.types[t].max_hp;
                let open = self.indirect_blast().open_armor_mm;
                let unit = &mut self.units[i];
                let damage = if thinnest <= open {
                    unit.hp
                } else {
                    (u64::from(max_hp) * u64::from(rules.at_damage_pct) / 100)
                        .clamp(1, u64::from(unit.hp)) as u32
                };
                unit.hp -= damage;
                self.events.push(CombatEvent::Hit {
                    projectile: id,
                    shooter: MINE_SHOOTER,
                    at,
                    kind: HitKind::Unit(unit_id),
                    face: None,
                    turret: false,
                    penetrated: true,
                    damage,
                    caliber_mm_x10: rules.at_crater_caliber_mm_x10,
                });
                if self.units[i].alive() {
                    self.throw_track(i);
                } else {
                    self.destroy(i, MINE_SHOOTER, at);
                }
                self.ground_hit(p, rules.at_crater_caliber_mm_x10);
            }
            (kind, _) => {
                // The soldier who stepped on it falls; the blast does the
                // rest (fragments, a vehicle above it, the crater).
                if let Some(k) = soldier
                    && let Some(squad) = &mut self.units[i].squad
                {
                    squad.soldiers[k].hp = 0;
                    let hp = squad.soldiers.iter().map(|s| s.hp).sum();
                    self.units[i].hp = hp;
                    if hp == 0 {
                        self.destroy(i, MINE_SHOOTER, at);
                    }
                }
                let caliber = match kind {
                    MineKind::AntiTank => rules.at_crater_caliber_mm_x10,
                    MineKind::AntiPersonnel => rules.ap_caliber_mm_x10,
                };
                self.detonate(id, MINE_SHOOTER, owner, at, at, caliber);
            }
        }
        self.reveal_mines(p, victim);
    }

    /// The vehicle loses its track: it cannot move until the crew fitted
    /// it again.
    fn throw_track(&mut self, i: usize) {
        let ticks = self.works.ticks(self.works.rules.mines.track_repair_s);
        let unit = &mut self.units[i];
        let id = unit.id;
        if let Some(t) = self.works.track_off.get_mut(&id) {
            t.left = ticks;
            return;
        }
        self.works.track_off.insert(
            id,
            TrackOff {
                left: ticks,
                offroad: unit.speed_offroad,
                road: unit.speed_road,
            },
        );
        unit.speed_offroad = Fixed::ZERO;
        unit.speed_road = Fixed::ZERO;
        unit.combat.attack_moving = false;
        stop(unit);
    }

    /// Crews fit thrown tracks.
    fn fit_tracks(&mut self) {
        if self.works.track_off.is_empty() {
            return;
        }
        let ids: Vec<UnitId> = self.works.track_off.keys().copied().collect();
        for id in ids {
            let Some(i) = self.works_index_of(id) else {
                self.works.track_off.remove(&id);
                continue;
            };
            if !self.units[i].alive() {
                self.works.track_off.remove(&id);
                continue;
            }
            let Some(t) = self.works.track_off.get_mut(&id) else {
                continue;
            };
            t.left = t.left.saturating_sub(1);
            if t.left == 0 {
                let t = *t;
                self.works.track_off.remove(&id);
                self.units[i].speed_offroad = t.offroad;
                self.units[i].speed_road = t.road;
            }
        }
    }

    /// The team of `player` learns the mines within the reveal radius of
    /// `p` (an explosion).
    fn reveal_mines(&mut self, p: Vec2, player: PlayerId) {
        let radius = mm(self.works.rules.mines.reveal_mm);
        let ids = self.works.mines_near(p, radius);
        self.learn_mines(&ids, player);
    }

    /// Player `player` learns the mines `ids`; routes of everybody through
    /// newly known ones are planned again.
    fn learn_mines(&mut self, ids: &[u32], player: PlayerId) {
        if player.0 >= 64 {
            return;
        }
        let mask = self.team_mask(player);
        let mut area: Option<(Vec2, Vec2)> = None;
        for &id in ids {
            let Ok(k) = self.works.mines.binary_search_by_key(&id, |m| m.id) else {
                continue;
            };
            let m = self.works.mines[k];
            if self.mine_known(&m, player, mask) {
                continue;
            }
            self.works.mines[k].known |= 1u64 << player.0;
            let p = m.position;
            area = Some(area.map_or((p, p), |(l, h)| {
                (
                    Vec2::new(l.x.min(p.x), l.y.min(p.y)),
                    Vec2::new(h.x.max(p.x), h.y.max(p.y)),
                )
            }));
        }
        if let Some((low, high)) = area {
            let rect = self.paths.grid.cells_in(low, high);
            self.replan_through(rect);
        }
    }

    /// Engineer squads find the hostile mines near them.
    fn detect_mines(&mut self) {
        if self.works.mines.is_empty() {
            return;
        }
        let radius = mm(self.works.rules.engineers.detect_mm);
        let mut found: Vec<(PlayerId, Vec<u32>)> = Vec::new();
        for u in &self.units {
            if !u.alive()
                || u.squad.is_none()
                || transport::aboard(u)
                || !self
                    .works
                    .engineer
                    .get(u.unit_type as usize)
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }
            let ids = self.works.mines_near(u.position, radius);
            if !ids.is_empty() {
                found.push((u.owner, ids));
            }
        }
        for (player, ids) in found {
            self.learn_mines(&ids, player);
        }
    }

    /// Heavy vehicles flatten the wire they drive over.
    fn crush_wire(&mut self) {
        if !self
            .works
            .works
            .iter()
            .any(|w| w.built && w.kind == WorkKind::Wire)
        {
            return;
        }
        let min_kg = self.works.rules.crush.wire_kg;
        let mut crushed = Vec::new();
        for u in &self.units {
            if !u.alive() || u.squad.is_some() || u.route == Route::None || u.link.is_some() {
                continue;
            }
            let t = u.unit_type as usize;
            if self.unit_types.get(t).is_none_or(|t| t.mass_kg < min_kg) {
                continue;
            }
            let (_, hw, _, _) = self.hull(t);
            for w in &self.works.works {
                if w.built
                    && w.kind == WorkKind::Wire
                    && (nearest_on(w.from, w.to, u.position) - u.position).length() <= hw
                    && !crushed.contains(&w.id)
                {
                    crushed.push(w.id);
                }
            }
        }
        for id in crushed {
            self.remove_work(id);
        }
    }

    /// Demolishes or repairs a bridge.
    fn finish_bridge(&mut self, bridge: usize, repair: bool, by: UnitId, owner: PlayerId) {
        self.set_bridge_intact(bridge, repair);
        if repair {
            return;
        }
        let b = &self.map.bridges()[bridge];
        let z = (b.height_start + b.height_end) / 2;
        let at = Vec3::new(b.center.x, b.center.y, z);
        let id = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1);
        let caliber = self.works.rules.bridge.demolish_caliber_mm_x10;
        self.detonate(id, by, owner, at, at, caliber);
    }

    /// The ground of this tick's dig steps (after the craters).
    pub(super) fn apply_digs(&mut self) {
        if self.works.pending_digs.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.works.pending_digs);
        for (id, step) in pending {
            let Some(k) = self.works.work_index(id) else {
                continue;
            };
            // One after the other: a plan is relative to the ground the
            // previous ones left (pieces share samples at their joints).
            if let Some(plan) = self.dig_plan(k, step) {
                self.deform(vec![plan]);
            }
        }
    }

    /// The samples dig step `step` of work `k` changes.
    fn dig_plan(&mut self, k: usize, step: u32) -> Option<CraterPlan> {
        let w = &self.works.works[k];
        let map = Arc::clone(&self.map);
        let (sx, sy) = map.samples();
        let mut holes: Vec<usize> = Vec::new();
        let length = (w.to - w.from).length();
        let n = (length * 2).ceil().to_num::<u32>().max(1);
        for s in 0..=n {
            let p = w.from + (w.to - w.from).scale(Fixed::from_num(s) / Fixed::from_num(n));
            if self.ground_for_works(p)
                && let Some(k) = nearest_sample(&map, p)
            {
                holes.push(k);
            }
        }
        // Pits, nests and dugouts (prompt 052) lower a round area.
        let (kind, center) = (w.kind, w.from);
        holes.extend(self.dig_area(kind, center));
        holes.sort_unstable();
        holes.dedup();
        if holes.is_empty() {
            return None;
        }
        let class = super::positions::sample_class(kind);
        for &h in &holes {
            self.works.dug.insert(h as u32);
            if let Some(class) = class {
                self.works.positions.classes.insert(h as u32, class);
            }
        }
        let rules = self.works.rules.dig;
        let steps = rules.steps.max(1);
        let depth = -i64::from(rules.depth_mm) * i64::from(step) / i64::from(steps);
        let rise = i64::from(rules.parapet_mm) * i64::from(step) / i64::from(steps);
        let mut samples: Vec<(usize, i64, bool)> = Vec::new();
        let mut rim: Vec<usize> = Vec::new();
        let (mut i0, mut j0, mut i1, mut j1) = (u32::MAX, u32::MAX, 0u32, 0u32);
        for &h in &holes {
            let (i, j) = ((h % sx as usize) as u32, (h / sx as usize) as u32);
            let dh = (depth - i64::from(self.craters.delta(h))).min(0);
            samples.push((h, dh, true));
            for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (Some(ni), Some(nj)) = (i.checked_add_signed(di), j.checked_add_signed(dj))
                else {
                    continue;
                };
                if ni < sx && nj < sy {
                    rim.push(nj as usize * sx as usize + ni as usize);
                }
            }
            (i0, j0) = (i0.min(i.saturating_sub(1)), j0.min(j.saturating_sub(1)));
            (i1, j1) = ((i1.max(i + 1)).min(sx - 1), (j1.max(j + 1)).min(sy - 1));
        }
        rim.sort_unstable();
        rim.dedup();
        for r in rim {
            if self.works.dug.contains(&(r as u32)) {
                continue;
            }
            let (i, j) = ((r % sx as usize) as u32, (r / sx as usize) as u32);
            let p = sample_point(&map, i, j);
            if !self.ground_for_works(p) {
                continue;
            }
            let dh = (rise - i64::from(self.craters.delta(r))).max(0);
            if dh != 0 {
                samples.push((r, dh, false));
            }
        }
        samples.retain(|&(_, dh, bowl)| dh != 0 || bowl);
        let w = &self.works.works[k];
        let mid = w.from + (w.to - w.from).scale(Fixed::ONE / 2);
        let low = sample_point(&map, i0, j0);
        let high = sample_point(&map, i1, j1);
        Some(CraterPlan {
            samples,
            area: (low, high),
            event: CraterEvent {
                at: mid,
                caliber_mm_x10: 0,
                radius: length / 2 + Fixed::ONE,
                depth: Fixed::from_num(-depth) / 1000,
                samples: (i0, j0, i1, j1),
            },
        })
    }

    /// Cover of a soldier on map sample `sample` if engineers dug it.
    pub(super) fn dug_cover_pct(
        &self,
        sample: usize,
        stance: crate::infantry::Stance,
    ) -> Option<u32> {
        let sample = u32::try_from(sample).ok()?;
        if let Some(pct) = self.class_cover_pct(sample, stance) {
            return Some(pct);
        }
        self.works
            .dug
            .contains(&sample)
            .then(|| self.works.rules.dig.pct(stance))
    }

    /// The mines `player`'s team knows: its own and its allies', and the
    /// hostile ones it found. Never the others: the enemy's view (prompt
    /// 024) does not see them.
    pub fn mines_for(&self, player: PlayerId) -> Vec<MineView> {
        let mask = self.team_mask(player);
        self.works
            .mines
            .iter()
            .filter(|m| self.mine_known(m, player, mask))
            .map(|m| MineView {
                id: m.id,
                kind: m.kind,
                position: m.position,
                owner: m.owner,
                friendly: self.mine_friendly(m, player),
            })
            .collect()
    }

    /// Every mine of the match (tools, tests, debug views; not for the
    /// presentation of a player).
    pub fn all_mines(&self) -> Vec<MineView> {
        self.works
            .mines
            .iter()
            .map(|m| MineView {
                id: m.id,
                kind: m.kind,
                position: m.position,
                owner: m.owner,
                friendly: false,
            })
            .collect()
    }

    /// Field works `player` sees: finished obstacles and dug positions of
    /// everybody (like map objects), sites only of the own team.
    pub fn works_for(&self, player: PlayerId) -> Vec<WorkView> {
        self.works
            .works
            .iter()
            .filter(|w| w.built || self.teams.allied(w.owner, player))
            .filter(|w| self.work_seen(w, player))
            .map(|w| WorkView {
                id: w.id,
                kind: w.kind,
                owner: w.owner,
                from: w.from,
                to: w.to,
                built: w.built,
                progress_pml: u32::try_from(u64::from(w.done) * 1000 / u64::from(w.total.max(1)))
                    .unwrap_or(1000),
                active: w.worked,
            })
            .collect()
    }

    /// Vehicles that lost their track to a mine, with the ticks until the
    /// crew has fitted it.
    pub fn tracks_off(&self) -> Vec<(UnitId, u32)> {
        self.works
            .track_off
            .iter()
            .map(|(&id, t)| (id, t.left))
            .collect()
    }

    /// Whether units of `unit_type` do engineer work (prompt 035).
    pub fn is_engineer_type(&self, unit_type: u32) -> bool {
        self.works
            .engineer
            .get(unit_type as usize)
            .copied()
            .unwrap_or(false)
    }

    /// What engineer `id` is doing: `"build"`, `"clear"`, `"bridge"`.
    pub fn engineer_job(&self, id: UnitId) -> Option<&'static str> {
        self.works.jobs.get(&id).map(|j| match j {
            Job::Build { .. } => "build",
            Job::Clear { .. } => "clear",
            Job::Bridge { .. } => "bridge",
        })
    }

    /// Places a section of neutral, hidden mines around `center`, like a
    /// minefield of the map editor. For tools and the debug menu, never for
    /// player commands (a match with it no longer replays). Returns the
    /// number of mines placed.
    pub fn place_minefield(&mut self, center: Vec2, kind: MineKind) -> usize {
        if !within_world(center) {
            return 0;
        }
        let rules = self.works.rules.mines;
        let count = match kind {
            MineKind::AntiTank => rules.map_at_count,
            MineKind::AntiPersonnel => rules.map_ap_count,
        };
        let mut h = StableHasher::new();
        h.write_u32(self.works.next_mine);
        h.write_i64(center.x.to_bits());
        h.write_i64(center.y.to_bits());
        let mut rng = Rng::new(h.finish());
        let section = mm(rules.section_mm);
        let spots = self.works.layout(
            &mut rng,
            count,
            (section, section),
            center,
            Vec2::new(Fixed::ONE, Fixed::ZERO),
            Vec2::new(Fixed::ZERO, Fixed::ONE),
        );
        let mut placed = 0;
        for p in spots {
            if self.ground_for_works(p) {
                self.works.add_mine(kind, None, p);
                placed += 1;
            }
        }
        placed
    }

    /// The field work rules of this match.
    pub const fn works_rules(&self) -> &WorksRules {
        &self.works.rules
    }
}

/// Something engineers clear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClearItem {
    Mine(u32),
    Work(u32),
}
