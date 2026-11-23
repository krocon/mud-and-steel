//! Rivers, bridges and vehicle camouflage (prompt 055): collapsing decks,
//! swimming and drowning, bridge-laying tanks, camouflage with branches and
//! nets, and paratroopers coming down. Rules and public types:
//! `crate::crossings`.
//!
//! Order within a tick (see [`Sim::step`]): swimmers move before the other
//! units ([`Sim::update_swimmers`], orders do not reach them); after the
//! air phase [`Sim::update_crossings`] advances the bridge-laying tanks,
//! the camouflage timers and the paratroopers; after the vision update
//! [`Sim::hide_camouflaged`] takes camouflaged vehicles out of the enemy's
//! sight where no enemy is close enough. Collapsing decks come from the
//! engineers' demolition (`crate::sim::works`, [`Sim::collapse_bridge`]).

use super::{Sim, stop, transport, within_world};
use crate::Vec3;
use crate::bridges::LaidBridge;
use crate::catalog::{Category, Group};
use crate::combat::{CombatEvent, FireMode};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::crossings::{
    CrossingEvent, CrossingRules, Gap, LayerState, LayerView, ParachuteView, SwimmerView,
    WATER_SHOOTER,
};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::terrain::TerrainRules;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A bridge-laying tank gives up driving to its gap after this many path
/// requests without arriving.
const MAX_TRIES: u32 = 5;
/// Ticks between two path requests of a bridge-laying tank.
const RETRY_TICKS: u32 = 40;
/// Ticks between two searches for a bank by a squad that found none.
const BANK_SEARCH_TICKS: u32 = 20;

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

/// The bridge of a bridge-laying tank type, from the catalog.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct LayerType {
    span_mm: u32,
    load_kg: u32,
    half_width: Fixed,
}

/// Work of a bridge-laying tank.
#[derive(Clone, Debug, Serialize, Deserialize)]
enum Job {
    Lay {
        gap: Gap,
        left: u32,
        wait: u32,
        tries: u32,
    },
    PickUp {
        left: u32,
        wait: u32,
        tries: u32,
    },
}

/// The bridge of one bridge-laying tank.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
struct Layer {
    /// Its slot in `Map::bridges` once it laid the bridge for the first
    /// time.
    bridge: Option<u32>,
    laid: bool,
    /// The laid bridge was destroyed.
    lost: bool,
}

/// A squad in deep water.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Swim {
    bank: Option<Vec2>,
    /// Ticks every soldier keeps afloat (by soldier index).
    breath: Vec<u32>,
    /// Ticks until the next search for a bank (without one).
    search: u32,
}

/// Camouflage of a vehicle or gun.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Camo {
    /// Ticks of standing still until the branches are up.
    setup: u32,
    /// Ticks it stood still and did not fire.
    still: u32,
    last: Vec2,
}

/// Paratroopers of one squad under their canopies.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Para {
    id: u32,
    owner: PlayerId,
    unit_type: u32,
    from: Vec3,
    land: Vec2,
    facing: Vec2,
    total: u32,
    elapsed: u32,
    soldiers: u16,
}

impl Para {
    fn at(&self) -> Vec3 {
        let s = Fixed::from_num(self.elapsed) / Fixed::from_num(self.total.max(1));
        let lerp = |a: Fixed, b: Fixed| a + (b - a) * s;
        Vec3::new(
            lerp(self.from.x, self.land.x),
            lerp(self.from.y, self.land.y),
            self.from.z,
        )
    }
}

/// Rivers, bridges and camouflage of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Crossings {
    rules: CrossingRules,
    /// Per unit type: its bridge, for bridge-laying tanks.
    layer_types: Vec<Option<LayerType>>,
    /// Per unit type: catalog category and squad size.
    kinds: Vec<Option<(Category, u16)>>,
    /// Per unit type: sight range (m).
    sight: Vec<Fixed>,
    /// Terrain id of laid decks.
    deck_terrain: u8,
    /// Per terrain id: deep water (impassable water).
    deep: Vec<bool>,
    /// Steepest grade (%) units climb (`data/terrain.toml`).
    max_grade_pct: u32,
    jobs: BTreeMap<UnitId, Job>,
    layers: BTreeMap<UnitId, Layer>,
    swimmers: BTreeMap<UnitId, Swim>,
    camo: BTreeMap<UnitId, Camo>,
    paras: Vec<Para>,
    next_para: u32,
    /// Hit points lost by bridge pieces (index into `Map::bridges`, prompt
    /// 115).
    #[serde(default)]
    bridge_damage: BTreeMap<u32, u32>,
    /// Pieces shot down in this tick, collapsing in
    /// [`Sim::update_crossings`] (not in the middle of the projectile
    /// loop), with who brought them down.
    #[serde(default)]
    falling: Vec<(u32, UnitId)>,
    tick_hz: u32,
    /// One of the rules acted: rules and state enter the state hash.
    active: bool,
    /// Events of the last tick (not state: derived, not hashed).
    #[serde(skip)]
    pub(super) events: Vec<CrossingEvent>,
}

impl Crossings {
    pub(super) fn new(
        rules: CrossingRules,
        config: &super::SimConfig,
        terrain: &TerrainRules,
    ) -> Self {
        let catalog = crate::economy::builtin_catalog();
        let entry = |name: &str| catalog.and_then(|c| c.get(name));
        let layer_types = config
            .unit_types
            .iter()
            .map(|t| {
                let e = entry(&t.name)?;
                (e.category == Category::Bridgelayer).then_some(())?;
                Some(LayerType {
                    span_mm: e.bridge_length_mm?,
                    load_kg: e.bridge_load_kg?,
                    half_width: mm(e.width_mm.unwrap_or(3000)) / 2,
                })
            })
            .collect();
        let kinds = config
            .unit_types
            .iter()
            .map(|t| entry(&t.name).map(|e| (e.category, e.squad_size.unwrap_or(0))))
            .collect();
        let sight = config
            .unit_types
            .iter()
            .map(|t| config.vision.range_m(t.combat.category))
            .collect();
        let deck_terrain = terrain
            .types
            .iter()
            .position(|t| t.name == "road")
            .and_then(|i| u8::try_from(i).ok())
            .unwrap_or(0);
        Self {
            rules,
            layer_types,
            kinds,
            sight,
            deck_terrain,
            deep: terrain
                .types
                .iter()
                .map(|t| !t.passable && t.name.contains("water"))
                .collect(),
            max_grade_pct: terrain.slope.max_grade_pct,
            jobs: BTreeMap::new(),
            layers: BTreeMap::new(),
            swimmers: BTreeMap::new(),
            camo: BTreeMap::new(),
            paras: Vec::new(),
            next_para: 0,
            bridge_damage: BTreeMap::new(),
            falling: Vec::new(),
            tick_hz: config.tick_hz.max(1),
            active: false,
            events: Vec::new(),
        }
    }

    fn ticks(&self, s: u32) -> u32 {
        s.saturating_mul(self.tick_hz)
    }

    /// Whether unit type `t` may put up camouflage (vehicles and guns).
    fn camouflages(&self, t: usize) -> bool {
        self.kinds
            .get(t)
            .copied()
            .flatten()
            .is_some_and(|(c, _)| matches!(c.group(), Group::Vehicle | Group::Gun))
    }

    /// Nothing for matches in which none of the rules acted, so their state
    /// hash is unchanged.
    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        if !self.active {
            return;
        }
        self.rules.hash_into(h);
        h.write_u64(self.jobs.len() as u64);
        for (id, job) in &self.jobs {
            h.write_u32(id.0);
            match job {
                Job::Lay {
                    gap,
                    left,
                    wait,
                    tries,
                } => {
                    h.write_u8(0);
                    for v in [gap.start.x, gap.start.y, gap.end.x, gap.end.y] {
                        h.write_i64(v.to_bits());
                    }
                    for v in [gap.width_m, *left, *wait, *tries] {
                        h.write_u32(v);
                    }
                }
                Job::PickUp { left, wait, tries } => {
                    h.write_u8(1);
                    for v in [*left, *wait, *tries] {
                        h.write_u32(v);
                    }
                }
            }
        }
        h.write_u64(self.layers.len() as u64);
        for (id, l) in &self.layers {
            h.write_u32(id.0);
            h.write_u32(l.bridge.unwrap_or(u32::MAX));
            h.write_u8(u8::from(l.laid) | (u8::from(l.lost) << 1));
        }
        h.write_u64(self.swimmers.len() as u64);
        for (id, s) in &self.swimmers {
            h.write_u32(id.0);
            match s.bank {
                None => h.write_u8(0),
                Some(b) => {
                    h.write_u8(1);
                    h.write_i64(b.x.to_bits());
                    h.write_i64(b.y.to_bits());
                }
            }
            h.write_u32(s.search);
            h.write_u64(s.breath.len() as u64);
            for &b in &s.breath {
                h.write_u32(b);
            }
        }
        h.write_u64(self.camo.len() as u64);
        for (id, c) in &self.camo {
            h.write_u32(id.0);
            h.write_u32(c.setup);
            h.write_u32(c.still);
            h.write_i64(c.last.x.to_bits());
            h.write_i64(c.last.y.to_bits());
        }
        h.write_u32(self.next_para);
        h.write_u64(self.paras.len() as u64);
        for p in &self.paras {
            h.write_u32(p.id);
            h.write_u8(p.owner.0);
            h.write_u32(p.unit_type);
            for v in [p.from.x, p.from.y, p.from.z, p.land.x, p.land.y] {
                h.write_i64(v.to_bits());
            }
            h.write_u32(p.total);
            h.write_u32(p.elapsed);
        }
        // Prompt 115: only when a bridge took hits, so older hashes stay.
        if !self.bridge_damage.is_empty() || !self.falling.is_empty() {
            h.write_u64(self.bridge_damage.len() as u64);
            for (&b, &d) in &self.bridge_damage {
                h.write_u32(b);
                h.write_u32(d);
            }
            h.write_u64(self.falling.len() as u64);
            for &(b, by) in &self.falling {
                h.write_u32(b);
                h.write_u32(by.0);
            }
        }
    }
}

/// Ranks candidate gaps: distance to the target, length, axis, cell.
type GapKey = (Fixed, Fixed, u8, i32, i32);

/// Triangular random value in `[-1, 1]`.
fn triangular(rng: &mut crate::rng::Rng) -> Fixed {
    let a = rng.range_i32(-500, 500);
    let b = rng.range_i32(-500, 500);
    Fixed::from_num(a + b) / 1000
}

impl Sim {
    fn crossing_index(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    /// Whether `p` lies in deep water (no deck above it).
    fn deep_at(&self, p: Vec2) -> bool {
        self.map.contains(p)
            && self
                .crossings
                .deep
                .get(usize::from(self.map.terrain_at(p)))
                .copied()
                .unwrap_or(false)
    }

    // --- Long bridges (prompt 115) -------------------------------------------------

    /// Whether the railing of the deck under soldier `k` of squad `j`
    /// catches a round flying along `d`: he lies (prone squad or down),
    /// stands on an intact deck with railing cover, and the round comes
    /// from the side (more across the deck than along it), like a low wall.
    pub(super) fn railing_catches(&mut self, j: usize, k: usize, d: Vec3) -> bool {
        let unit = &self.units[j];
        let Some(squad) = &unit.squad else {
            return false;
        };
        let Some(soldier) = squad.soldiers.get(k) else {
            return false;
        };
        if squad.stance != crate::infantry::Stance::Prone && !soldier.lying() {
            return false;
        }
        let Some(b) = self.map.deck_at(soldier.position) else {
            return false;
        };
        let reach = mm(self.crossings.rules.collapse.deck_reach_mm);
        if b.railing_cover_pct == 0
            || (soldier.height - b.height_at(soldier.position)).abs() > reach
        {
            return false;
        }
        let g = d.ground();
        let along = (g.x * b.forward.x + g.y * b.forward.y).abs();
        let across = (g.x * b.right.x + g.y * b.right.y).abs();
        if across < along {
            return false;
        }
        let pct = b.railing_cover_pct;
        self.rng.below(100) < pct
    }

    /// A heavy round or explosion of `caliber_mm_x10` struck `at`: if that
    /// lies on an intact deck whose kind can be shot to pieces
    /// (`[shelling]` of `data/crossings.toml`, prompt 115), the piece loses
    /// `damage` hit points; at 0 it collapses like a demolished one, taking
    /// down what stands on it. Only that piece falls.
    pub(super) fn shell_bridge(&mut self, at: Vec3, caliber_mm_x10: u32, damage: u32, by: UnitId) {
        let rules = &self.crossings.rules.shelling;
        if rules.kinds.is_empty() || caliber_mm_x10 < rules.min_caliber_mm_x10 || damage == 0 {
            return;
        }
        let p = at.ground();
        let Some(index) = self
            .map
            .bridges()
            .iter()
            .position(|b| b.intact && b.laid_by.is_none() && b.covers(p))
        else {
            return;
        };
        let b = &self.map.bridges()[index];
        if (at.z - b.height_at(p)).abs() > Fixed::from_num(3) {
            return;
        }
        let o = &self.map.objects()[b.object as usize];
        let kind = &self.map.object_kinds()[usize::from(o.kind)];
        let Some(hp) = rules.hp_of(&kind.id) else {
            return;
        };
        self.crossings.active = true;
        let key = u32::try_from(index).unwrap_or(u32::MAX);
        if self.crossings.falling.iter().any(|&(b, _)| b == key) {
            return;
        }
        let taken = self.crossings.bridge_damage.entry(key).or_insert(0);
        *taken = taken.saturating_add(damage);
        if *taken >= hp {
            self.crossings.bridge_damage.remove(&key);
            self.crossings.falling.push((key, by));
        }
    }

    /// Hit points a bridge piece has left (`None`: it cannot be shot to
    /// pieces or is down).
    pub fn bridge_hp(&self, bridge: usize) -> Option<u32> {
        let b = self.map.bridges().get(bridge)?;
        if !b.intact || b.laid_by.is_some() {
            return None;
        }
        let o = &self.map.objects()[b.object as usize];
        let kind = &self.map.object_kinds()[usize::from(o.kind)];
        let hp = self.crossings.rules.shelling.hp_of(&kind.id)?;
        let key = u32::try_from(bridge).ok()?;
        if self.crossings.falling.iter().any(|&(b, _)| b == key) {
            return Some(0);
        }
        Some(hp.saturating_sub(self.crossings.bridge_damage.get(&key).copied().unwrap_or(0)))
    }

    // --- Collapsing decks -------------------------------------------------------

    /// Destroys bridge `bridge` (demolished by `by`): the deck falls into
    /// the river and takes down every unit standing on it. Vehicles and
    /// guns are lost; squads fall into the water and swim.
    pub(super) fn collapse_bridge(&mut self, bridge: usize, by: UnitId) {
        let Some(b) = self.map.bridges().get(bridge).cloned() else {
            return;
        };
        if !b.intact {
            return;
        }
        let reach = mm(self.crossings.rules.collapse.deck_reach_mm);
        let on: Vec<usize> = (0..self.units.len())
            .filter(|&j| {
                let u = &self.units[j];
                u.alive()
                    && !transport::aboard(u)
                    && b.covers(u.position)
                    && (u.height - b.height_at(u.position)).abs() <= reach
            })
            .collect();
        self.set_bridge_intact(bridge, false);
        let z = (b.height_start + b.height_end) / 2;
        self.crossings.events.push(CrossingEvent::Collapsed {
            bridge: u32::try_from(bridge).unwrap_or(u32::MAX),
            at: Vec3::new(b.center.x, b.center.y, z),
        });
        if !on.is_empty() {
            self.crossings.active = true;
        }
        for j in on {
            let p = self.units[j].position;
            let ground = self.map.height_at(p);
            let at = Vec3::new(p.x, p.y, ground);
            let unit = &mut self.units[j];
            stop(unit);
            unit.combat.attack = None;
            unit.height = ground;
            let id = unit.id;
            self.crossings
                .events
                .push(CrossingEvent::Fell { unit: id, at });
            if self.units[j].squad.is_some() {
                self.fall_in(j, by);
            } else {
                self.units[j].hp = 0;
                self.destroy(j, by, at);
            }
        }
    }

    /// Squad `j` fell into the water: some soldiers are lost at once.
    fn fall_in(&mut self, j: usize, by: UnitId) {
        let pct = self.crossings.rules.swim.fall_loss_pct;
        let (map, rng) = (&self.map, &mut self.rng);
        let unit = &mut self.units[j];
        let Some(squad) = unit.squad.as_mut() else {
            return;
        };
        for s in squad.soldiers.iter_mut().filter(|s| s.alive()) {
            s.height = map.height_at(s.position);
            if rng.below(100) < pct {
                s.hp = 0;
            }
        }
        unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
        if unit.hp == 0 {
            let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
            self.destroy(j, by, at);
        }
    }

    // --- Swimming and drowning ----------------------------------------------------

    /// Squads in deep water swim to the nearest bank; soldiers that stay in
    /// it too long drown. Runs before the movement: orders do not reach
    /// swimmers.
    pub(super) fn update_swimmers(&mut self) {
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.alive()
                || u.squad.is_none()
                || transport::aboard(u)
                || super::infantry::inside(u)
                || self.crossings.swimmers.contains_key(&u.id)
                || !self.deep_at(u.position)
            {
                continue;
            }
            let id = u.id;
            let n = u.squad.as_ref().map_or(0, |s| s.soldiers.len());
            let swim = self.crossings.rules.swim;
            let lo = self.crossings.ticks(swim.breath_min_s);
            let hi = self.crossings.ticks(swim.breath_max_s).max(lo);
            let breath = (0..n).map(|_| lo + self.rng.below(hi - lo + 1)).collect();
            let bank = self.find_bank(i);
            self.crossings.swimmers.insert(
                id,
                Swim {
                    bank,
                    breath,
                    search: BANK_SEARCH_TICKS,
                },
            );
            self.crossings.active = true;
        }
        if self.crossings.swimmers.is_empty() {
            return;
        }
        let speed = mm(self.crossings.rules.swim.speed_mm_per_s) / Fixed::from_num(self.tick_hz);
        let ids: Vec<UnitId> = self.crossings.swimmers.keys().copied().collect();
        for id in ids {
            let Some(mut swim) = self.crossings.swimmers.remove(&id) else {
                continue;
            };
            let Some(i) = self.crossing_index(id) else {
                continue;
            };
            if !self.units[i].alive() || transport::aboard(&self.units[i]) {
                continue;
            }
            if swim.bank.is_none() {
                if swim.search == 0 {
                    swim.bank = self.find_bank(i);
                    swim.search = BANK_SEARCH_TICKS;
                } else {
                    swim.search -= 1;
                }
            }
            {
                let unit = &mut self.units[i];
                stop(unit);
                unit.combat.attack = None;
                if let Some(bank) = swim.bank {
                    let d = bank - unit.position;
                    let length = d.length();
                    if length > Fixed::ZERO {
                        let dir = Vec2::new(d.x / length, d.y / length);
                        unit.position = unit.position + dir.scale(speed.min(length));
                        unit.facing = dir;
                    }
                }
                unit.height = self.map.height_at(unit.position);
            }
            // Who is still in the water, and who goes under.
            let deep: Vec<bool> = self.units[i]
                .squad
                .as_ref()
                .map(|s| {
                    s.soldiers
                        .iter()
                        .map(|x| self.deep_at(x.position))
                        .collect()
                })
                .unwrap_or_default();
            let mut drowned = Vec::new();
            let unit = &mut self.units[i];
            let Some(squad) = unit.squad.as_mut() else {
                continue;
            };
            let mut wet = false;
            for (k, s) in squad.soldiers.iter_mut().enumerate() {
                if !s.alive() || !deep.get(k).copied().unwrap_or(false) {
                    continue;
                }
                let b = swim.breath.get_mut(k).map_or(0, |b| {
                    *b = b.saturating_sub(1);
                    *b
                });
                if b == 0 {
                    s.hp = 0;
                    drowned.push((k, Vec3::new(s.position.x, s.position.y, s.height)));
                } else {
                    wet = true;
                }
            }
            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            let (hp, position, height) = (unit.hp, unit.position, unit.height);
            for (k, at) in drowned {
                self.crossings.events.push(CrossingEvent::Drowned {
                    unit: id,
                    soldier: u16::try_from(k).unwrap_or(u16::MAX),
                    at,
                });
            }
            if hp == 0 {
                self.destroy(i, WATER_SHOOTER, Vec3::new(position.x, position.y, height));
                continue;
            }
            // Ashore once on the bank cell (where orders find a path again)
            // and nobody is in the water any more.
            let landed = swim
                .bank
                .is_none_or(|b| (b - position).length() <= Fixed::ONE / 10);
            if !wet && landed && !self.deep_at(position) {
                self.crossings.events.push(CrossingEvent::Ashore {
                    unit: id,
                    at: position,
                });
                continue;
            }
            self.crossings.swimmers.insert(id, swim);
        }
    }

    /// The center of the nearest path cell within reach that unit `i` may
    /// stand on (out of deep water).
    fn find_bank(&self, i: usize) -> Option<Vec2> {
        let u = &self.units[i];
        let profile = u.unit_type as usize;
        let grid = &self.paths.grid;
        let cell = grid.cell_size();
        let reach = Fixed::from_num(self.crossings.rules.swim.search_m);
        let r = (reach / cell).to_num::<i32>() + 1;
        let (cx, cy) = grid.cell_of(u.position);
        let (w, h) = grid.size();
        let mut best: Option<(Fixed, u32, u32)> = None;
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx.cast_signed() + dx, cy.cast_signed() + dy);
                if x < 0 || y < 0 || x >= w.cast_signed() || y >= h.cast_signed() {
                    continue;
                }
                let (x, y) = (x.cast_unsigned(), y.cast_unsigned());
                let c = grid.center(x, y);
                if !grid.passable_for(profile, c) || self.deep_at(c) {
                    continue;
                }
                let d = (c - u.position).length();
                if d > reach {
                    continue;
                }
                if best.is_none_or(|b| (d, y, x) < b) {
                    best = Some((d, y, x));
                }
            }
        }
        best.map(|(_, y, x)| grid.center(x, y))
    }

    // --- Bridge-laying tanks --------------------------------------------------------

    /// [`Command::LayBridge`].
    pub(super) fn order_lay_bridge(&mut self, player: PlayerId, units: &[UnitId], target: Vec2) {
        if !within_world(target) {
            return;
        }
        for i in self.mobile_indices(player, units) {
            let t = self.units[i].unit_type;
            if self
                .crossings
                .layer_types
                .get(t as usize)
                .is_none_or(Option::is_none)
            {
                continue;
            }
            let id = self.units[i].id;
            let layer = self.crossings.layers.get(&id).copied().unwrap_or_default();
            if layer.laid || layer.lost {
                continue;
            }
            let Some(gap) = self.find_gap(t, target) else {
                continue;
            };
            self.units[i].combat.attack = None;
            let left = self.crossings.ticks(self.crossings.rules.layer.lay_s);
            self.crossings.jobs.insert(
                id,
                Job::Lay {
                    gap,
                    left,
                    wait: 0,
                    tries: 0,
                },
            );
            self.crossings.active = true;
        }
    }

    /// [`Command::PickUpBridge`].
    pub(super) fn order_pick_up_bridge(&mut self, player: PlayerId, units: &[UnitId]) {
        for i in self.mobile_indices(player, units) {
            let id = self.units[i].id;
            if !self.crossings.layers.get(&id).is_some_and(|l| l.laid) {
                continue;
            }
            let left = self.crossings.ticks(self.crossings.rules.layer.pickup_s);
            self.crossings.jobs.insert(
                id,
                Job::PickUp {
                    left,
                    wait: 0,
                    tries: 0,
                },
            );
        }
    }

    /// Orders that give a bridge-laying tank something else to do end its
    /// work.
    pub(super) fn drop_crossing_jobs(&mut self, command: &PlayerCommand) {
        if self.crossings.jobs.is_empty()
            || !matches!(
                command.command,
                Command::MoveTo { .. }
                    | Command::Stop { .. }
                    | Command::Attack { .. }
                    | Command::AttackGround { .. }
                    | Command::AttackObject { .. }
                    | Command::Patrol { .. }
                    | Command::QueueMove { .. }
                    | Command::Embark { .. }
                    | Command::Tow { .. }
            )
        {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            self.crossings.jobs.remove(&self.units[i].id);
        }
    }

    /// The gap nearest to `target` (within `layer.search_m`) that a
    /// bridge-laying tank of type `unit_type` can span: a straight line
    /// along a path-grid axis over ground it cannot stand on (deep water,
    /// steep ditches), at most its bridge's length minus the bearing on
    /// both banks, between two cells it can reach. The deck covers the bank
    /// cells on both sides. `None` if there is none (or the type lays no
    /// bridges).
    pub fn find_gap(&self, unit_type: u32, target: Vec2) -> Option<Gap> {
        let lt = (*self.crossings.layer_types.get(unit_type as usize)?)?;
        let rules = self.crossings.rules.layer;
        let grid = &self.paths.grid;
        let map = &self.map;
        let cell = grid.cell_size();
        let (w, h) = grid.size();
        let search = Fixed::from_num(rules.search_m);
        let r = (search / cell).to_num::<i32>() + 1;
        let (cx, cy) = grid.cell_of(target);
        let span = mm(lt.span_mm);
        let bearing = mm(rules.bearing_mm);
        let profile = unit_type as usize;
        let max_cells = ((span / cell).to_num::<i32>() + 2).max(1);
        let grade = Fixed::from_num(self.crossings.max_grade_pct) / 100;
        // Ground a unit can stand on: passable terrain, no steep slope along
        // `dir`, no deck.
        let ground = |p: Vec2, dir: Vec2| {
            if !map.contains(p) || map.deck_at(p).is_some() {
                return false;
            }
            let t = usize::from(map.ground_terrain_at(p));
            if !self.terrain.types.get(t).is_some_and(|t| t.passable) {
                return false;
            }
            let rise = (map.ground_height_at(p + dir) - map.ground_height_at(p - dir)).abs();
            rise <= grade * 2
        };
        let inside =
            |x: i32, y: i32| x >= 0 && y >= 0 && x < w.cast_signed() && y < h.cast_signed();
        let center = |x: i32, y: i32| grid.center(x.cast_unsigned(), y.cast_unsigned());
        let mut best: Option<(GapKey, Gap)> = None;
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx.cast_signed() + dx, cy.cast_signed() + dy);
                if !inside(x, y) {
                    continue;
                }
                let c = center(x, y);
                if (c - target).length() > search + cell {
                    continue;
                }
                for axis in 0u8..2 {
                    let (sx, sy) = if axis == 0 { (1, 0) } else { (0, 1) };
                    let dir = Vec2::new(Fixed::from_num(sx), Fixed::from_num(sy));
                    if ground(c, dir) {
                        continue;
                    }
                    let find = |sign: i32| {
                        (1..=max_cells).find_map(|k| {
                            let (nx, ny) = (x + sign * k * sx, y + sign * k * sy);
                            if !inside(nx, ny) {
                                return Some(None);
                            }
                            ground(center(nx, ny), dir).then_some(Some((nx, ny)))
                        })
                    };
                    let (Some(Some(a)), Some(Some(b))) = (find(-1), find(1)) else {
                        continue;
                    };
                    let (pa, pb) = (center(a.0, a.1), center(b.0, b.1));
                    let length = (pb - pa).length();
                    let steps = length.to_num::<i32>();
                    let wet = (0..=steps)
                        .filter(|&k| !ground(pa + dir.scale(Fixed::from_num(k)), dir))
                        .count();
                    let wet_m = Fixed::from_num(wet);
                    if wet_m + bearing * 2 > span {
                        continue;
                    }
                    // Both ends lead somewhere.
                    let (before, after) = (center(a.0 - sx, a.1 - sy), center(b.0 + sx, b.1 + sy));
                    if !inside(a.0 - sx, a.1 - sy)
                        || !inside(b.0 + sx, b.1 + sy)
                        || !grid.passable_for(profile, before)
                        || !grid.passable_for(profile, after)
                    {
                        continue;
                    }
                    let half = cell / 2;
                    let start = pa - dir.scale(half);
                    let end = pb + dir.scale(half);
                    let deck = length + cell;
                    let rise = (map.ground_height_at(end) - map.ground_height_at(start)).abs();
                    if rise * 100 > deck * Fixed::from_num(rules.max_grade_pct) {
                        continue;
                    }
                    let mid = Vec2::new((start.x + end.x) / 2, (start.y + end.y) / 2);
                    if map
                        .bridges()
                        .iter()
                        .any(|b| b.intact && (b.covers(start) || b.covers(end) || b.covers(mid)))
                    {
                        continue;
                    }
                    let key = ((mid - target).length(), length, axis, a.1, a.0);
                    if best.as_ref().is_none_or(|(k, _)| key < *k) {
                        best = Some((
                            key,
                            Gap {
                                start,
                                end,
                                width_m: u32::try_from(wet).unwrap_or(u32::MAX),
                            },
                        ));
                    }
                }
            }
        }
        best.map(|(_, gap)| gap)
    }

    /// Drives tank `i` to `spot`: `Some(true)` once it is within the reach
    /// of `end` (it stops), `Some(false)` on the way, `None` when no path
    /// was found after several tries.
    fn layer_walk(
        &mut self,
        i: usize,
        spot: Vec2,
        end: Vec2,
        wait: &mut u32,
        tries: &mut u32,
    ) -> Option<bool> {
        let reach = Fixed::from_num(self.crossings.rules.layer.reach_m) + Fixed::from_num(4);
        let unit = &mut self.units[i];
        if (unit.position - end).length() <= reach {
            if unit.route != super::Route::None {
                stop(unit);
            }
            *tries = 0;
            return Some(true);
        }
        if unit.route != super::Route::None {
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

    /// The end of a deck nearer to `p`, and the direction out of the deck
    /// there.
    fn near_end(start: Vec2, end: Vec2, p: Vec2) -> (Vec2, Vec2) {
        let d = end - start;
        let length = d.length().max(Fixed::DELTA);
        let dir = Vec2::new(d.x / length, d.y / length);
        if (start - p).length() <= (end - p).length() {
            (start, Vec2::new(-dir.x, -dir.y))
        } else {
            (end, dir)
        }
    }

    fn advance_layers(&mut self) {
        // A laid bridge that was blown up is lost.
        for l in self.crossings.layers.values_mut() {
            if l.laid
                && l.bridge
                    .and_then(|b| self.map.bridges().get(b as usize))
                    .is_none_or(|b| !b.intact)
            {
                l.laid = false;
                l.lost = true;
            }
        }
        if self.crossings.jobs.is_empty() {
            return;
        }
        let ids: Vec<UnitId> = self.crossings.jobs.keys().copied().collect();
        for id in ids {
            let Some(job) = self.crossings.jobs.remove(&id) else {
                continue;
            };
            let Some(i) = self.crossing_index(id) else {
                continue;
            };
            if !self.units[i].alive() || transport::aboard(&self.units[i]) {
                continue;
            }
            if let Some(job) = self.advance_layer(i, job) {
                self.crossings.jobs.insert(id, job);
            }
        }
    }

    fn advance_layer(&mut self, i: usize, job: Job) -> Option<Job> {
        let id = self.units[i].id;
        let layer = self.crossings.layers.get(&id).copied().unwrap_or_default();
        let reach = Fixed::from_num(self.crossings.rules.layer.reach_m);
        match job {
            Job::Lay {
                gap,
                mut left,
                mut wait,
                mut tries,
            } => {
                if layer.laid || layer.lost {
                    return None;
                }
                let (end, out) = Self::near_end(gap.start, gap.end, self.units[i].position);
                let spot = end + out.scale(reach);
                if !self.layer_walk(i, spot, end, &mut wait, &mut tries)? {
                    return Some(Job::Lay {
                        gap,
                        left,
                        wait,
                        tries,
                    });
                }
                self.units[i].facing = Vec2::new(-out.x, -out.y);
                if left > 0 {
                    left -= 1;
                    return Some(Job::Lay {
                        gap,
                        left,
                        wait,
                        tries,
                    });
                }
                self.lay_bridge(i, &gap);
                None
            }
            Job::PickUp {
                mut left,
                mut wait,
                mut tries,
            } => {
                let b = layer.bridge? as usize;
                if !layer.laid {
                    return None;
                }
                let bridge = self.map.bridges().get(b)?.clone();
                let (start, end) = bridge.ends();
                let (near, out) = Self::near_end(start, end, self.units[i].position);
                let spot = near + out.scale(reach);
                if !self.layer_walk(i, spot, near, &mut wait, &mut tries)? {
                    return Some(Job::PickUp { left, wait, tries });
                }
                self.units[i].facing = Vec2::new(-out.x, -out.y);
                let deck_reach = mm(self.crossings.rules.collapse.deck_reach_mm);
                let busy = self.units.iter().any(|u| {
                    u.alive()
                        && !transport::aboard(u)
                        && bridge.covers(u.position)
                        && (u.height - bridge.height_at(u.position)).abs() <= deck_reach
                });
                if busy {
                    return Some(Job::PickUp { left, wait, tries });
                }
                if left > 0 {
                    left -= 1;
                    return Some(Job::PickUp { left, wait, tries });
                }
                self.set_bridge_intact(b, false);
                if let Some(l) = self.crossings.layers.get_mut(&id) {
                    l.laid = false;
                }
                self.crossings.events.push(CrossingEvent::Bridge {
                    unit: id,
                    bridge: b as u32,
                    laid: false,
                });
                None
            }
        }
    }

    /// Tank `i` lays its bridge across `gap`.
    fn lay_bridge(&mut self, i: usize, gap: &Gap) {
        let id = self.units[i].id;
        let Some(lt) = self.crossings.layer_types[self.units[i].unit_type as usize] else {
            return;
        };
        let d = gap.end - gap.start;
        let length = d.length();
        if length <= Fixed::ZERO {
            return;
        }
        let forward = Vec2::new(d.x / length, d.y / length);
        let center = Vec2::new((gap.start.x + gap.end.x) / 2, (gap.start.y + gap.end.y) / 2);
        // Somebody else bridged it meanwhile.
        if self
            .map
            .bridges()
            .iter()
            .any(|b| b.intact && (b.covers(center) || b.covers(gap.start) || b.covers(gap.end)))
        {
            return;
        }
        let laid = LaidBridge {
            layer: id,
            center,
            forward,
            half_length: length / 2,
            half_width: lt.half_width,
            height_start: self.map.ground_height_at(gap.start),
            height_end: self.map.ground_height_at(gap.end),
            max_load_kg: lt.load_kg,
            terrain: self.crossings.deck_terrain,
        };
        let layer = self.crossings.layers.get(&id).copied().unwrap_or_default();
        let slot = layer
            .bridge
            .map_or(self.map.bridges().len(), |b| b as usize);
        let Some(index) = Arc::make_mut(&mut self.map).put_laid_bridge(slot, &laid) else {
            return;
        };
        self.crossings.layers.insert(
            id,
            super::crossings::Layer {
                bridge: Some(index as u32),
                laid: true,
                lost: false,
            },
        );
        if let Some(rect) = self.paths.grid.sync_bridge(&self.map, index) {
            let changed = self.paths.grid.rebuild(&self.map, &self.terrain, rect);
            self.replan_through(changed);
        }
        self.crossings.events.push(CrossingEvent::Bridge {
            unit: id,
            bridge: index as u32,
            laid: true,
        });
    }

    // --- Camouflage ------------------------------------------------------------------

    /// [`Command::SetCamouflage`].
    pub(super) fn order_camouflage(&mut self, player: PlayerId, units: &[UnitId], on: bool) {
        let setup = self
            .crossings
            .ticks(self.crossings.rules.camouflage.apply_s);
        for i in self.owned_indices(player, units) {
            if !self.crossings.camouflages(self.units[i].unit_type as usize) {
                continue;
            }
            let (id, position) = (self.units[i].id, self.units[i].position);
            if on {
                self.crossings.camo.entry(id).or_insert(Camo {
                    setup,
                    still: 0,
                    last: position,
                });
                self.crossings.active = true;
            } else {
                self.crossings.camo.remove(&id);
            }
        }
    }

    /// Camouflage is put up while the vehicle stands; driving or firing
    /// starts the settling time again.
    fn update_camouflage(&mut self) {
        if self.crossings.camo.is_empty() {
            return;
        }
        let shooters: Vec<UnitId> = self
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Shot { unit, .. } => Some(unit),
                _ => None,
            })
            .collect();
        let ids: Vec<UnitId> = self.crossings.camo.keys().copied().collect();
        for id in ids {
            let alive = self
                .crossing_index(id)
                .map(|i| (self.units[i].alive(), self.units[i].position));
            let Some((true, position)) = alive else {
                self.crossings.camo.remove(&id);
                continue;
            };
            let Some(c) = self.crossings.camo.get_mut(&id) else {
                continue;
            };
            if position == c.last {
                c.setup = c.setup.saturating_sub(1);
                c.still = c.still.saturating_add(1);
            } else {
                c.still = 0;
                c.last = position;
            }
            if shooters.contains(&id) {
                c.still = 0;
            }
        }
    }

    /// Camouflaged vehicles that stand still are seen only by enemy units
    /// within `seen_pct` of their sight range (and in their viewshed).
    pub(super) fn hide_camouflaged(&mut self) {
        if self.crossings.camo.is_empty() {
            return;
        }
        let rules = self.crossings.rules.camouflage;
        let settle = self.crossings.ticks(rules.settle_s);
        let hidden: Vec<usize> = self
            .crossings
            .camo
            .iter()
            .filter(|(_, c)| c.setup == 0 && c.still >= settle)
            .filter_map(|(&id, _)| self.crossing_index(id))
            .collect();
        let players: Vec<PlayerId> = self.vision.slot_players().to_vec();
        for j in hidden {
            let target = &self.units[j];
            if !target.alive() {
                continue;
            }
            let Some((_, allies, _)) = self.vision.slot_masks(target.owner) else {
                continue;
            };
            let mut mask = self.vision.seen_by(j);
            for (s, &p) in players.iter().enumerate() {
                if mask & (1 << s) == 0 || allies & (1 << s) != 0 {
                    continue;
                }
                let Some((_, _, shares)) = self.vision.slot_masks(p) else {
                    continue;
                };
                let at = self.units[j].position;
                let seen = self.units.iter().enumerate().any(|(i, u)| {
                    if !u.alive() || transport::aboard(u) {
                        return false;
                    }
                    let Some((t, _, _)) = self.vision.slot_masks(u.owner) else {
                        return false;
                    };
                    if shares & (1 << t) == 0 {
                        return false;
                    }
                    let range = self.crossings.sight[u.unit_type as usize]
                        * Fixed::from_num(rules.seen_pct)
                        / 100;
                    let d = at - u.position;
                    d.x.abs() <= range
                        && d.y.abs() <= range
                        && d.length() <= range
                        && self.unit_sees(i, at)
                });
                if !seen {
                    mask &= !(1 << s);
                }
            }
            self.vision.set_seen_by(j, mask);
        }
    }

    // --- Paratroopers ------------------------------------------------------------------

    /// The squad type and soldiers per squad `player`'s transports drop,
    /// and squads per aircraft of type `seats`; `None` if the faction has
    /// none in this match.
    pub(super) fn para_type(&self, player: PlayerId, seats: u16) -> Option<(u32, u16, u32)> {
        let faction = self.faction(player)?;
        let rules = self.air_rules().airborne;
        let (t, size) = (0u32..).zip(&self.crossings.kinds).find_map(|(t, k)| {
            let (category, size) = (*k)?;
            (category == rules.category
                && crate::economy::faction_of(&self.unit_types[t as usize].name) == Some(faction))
            .then_some((t, size.max(1)))
        })?;
        let squads = rules.squads_per_aircraft.min(u32::from(seats / size));
        (squads > 0).then_some((t, size, squads))
    }

    /// A stick of paratroopers jumps from `from` over `aim`.
    pub(super) fn drop_paratroopers(&mut self, owner: PlayerId, from: Vec3, aim: Vec2, dir: Vec2) {
        let Some((unit_type, soldiers, _)) = self.para_type(owner, u16::MAX) else {
            return;
        };
        let rules = self.air_rules().airborne;
        let scatter = Fixed::from_num(rules.scatter_m);
        let ex = triangular(&mut self.rng) * scatter;
        let ey = triangular(&mut self.rng) * scatter;
        let mut land = aim + Vec2::new(ex, ey);
        if !self.map.contains(land) {
            land = aim;
        }
        if !self.map.contains(land) {
            return;
        }
        let fall = (from.z - self.map.height_at(land)).max(Fixed::ZERO);
        let seconds = fall / mm(rules.descent_mm_per_s);
        let total = (seconds * Fixed::from_num(self.tick_hz))
            .round()
            .to_num::<u32>()
            .max(1);
        let id = self.crossings.next_para;
        self.crossings.next_para = self.crossings.next_para.wrapping_add(1);
        self.crossings.paras.push(Para {
            id,
            owner,
            unit_type,
            from,
            land,
            facing: dir,
            total,
            elapsed: 0,
            soldiers,
        });
        self.crossings.active = true;
    }

    /// Paratroopers come down; landed sticks become squads.
    fn update_paras(&mut self) {
        if self.crossings.paras.is_empty() {
            return;
        }
        let paras = std::mem::take(&mut self.crossings.paras);
        for mut p in paras {
            p.elapsed += 1;
            if p.elapsed < p.total {
                self.crossings.paras.push(p);
                continue;
            }
            let water = self.deep_at(p.land);
            let profile = p.unit_type as usize;
            let mut at = p.land;
            if !water && !self.paths.grid.passable_for(profile, at) {
                // Roofs, walls, wire: the stick gathers beside them.
                at = self.nearest_standing(profile, at).unwrap_or(at);
            }
            let index = self.spawn_unit(p.owner, p.unit_type, at, p.facing, FireMode::default());
            let id = self.units[index].id;
            self.crossings.events.push(CrossingEvent::Landed {
                unit: id,
                at,
                water,
            });
        }
    }

    /// The center of the nearest path cell (within 24 m) unit type
    /// `profile` may stand on.
    fn nearest_standing(&self, profile: usize, p: Vec2) -> Option<Vec2> {
        let grid = &self.paths.grid;
        let (cx, cy) = grid.cell_of(p);
        let (w, h) = grid.size();
        let mut best: Option<(Fixed, u32, u32)> = None;
        for dy in -6i32..=6 {
            for dx in -6i32..=6 {
                let (x, y) = (cx.cast_signed() + dx, cy.cast_signed() + dy);
                if x < 0 || y < 0 || x >= w.cast_signed() || y >= h.cast_signed() {
                    continue;
                }
                let (x, y) = (x.cast_unsigned(), y.cast_unsigned());
                let c = grid.center(x, y);
                if !grid.passable_for(profile, c) {
                    continue;
                }
                let d = (c - p).length();
                if best.is_none_or(|b| (d, y, x) < b) {
                    best = Some((d, y, x));
                }
            }
        }
        best.map(|(_, y, x)| grid.center(x, y))
    }

    /// Bridge-laying tanks, camouflage timers and paratroopers (after the
    /// air phase).
    pub(super) fn update_crossings(&mut self) {
        // Bridge pieces shot down this tick (prompt 115).
        for (bridge, by) in std::mem::take(&mut self.crossings.falling) {
            self.collapse_bridge(bridge as usize, by);
        }
        self.advance_layers();
        self.update_camouflage();
        self.update_paras();
    }

    // --- Queries -----------------------------------------------------------------------

    /// What happened at rivers and bridges in the last tick.
    pub fn crossing_events(&self) -> &[CrossingEvent] {
        &self.crossings.events
    }

    /// The rules of `data/crossings.toml`.
    pub const fn crossing_rules(&self) -> &CrossingRules {
        &self.crossings.rules
    }

    /// Index into `SimConfig::unit_types` of unit `id`.
    pub fn type_of_unit(&self, id: UnitId) -> Option<u32> {
        self.crossing_index(id).map(|i| self.units[i].unit_type)
    }

    /// Whether unit type `unit_type` lays bridges.
    pub fn is_bridgelayer_type(&self, unit_type: u32) -> bool {
        self.crossings
            .layer_types
            .get(unit_type as usize)
            .is_some_and(Option::is_some)
    }

    /// Whether unit type `unit_type` may put up camouflage.
    pub fn can_camouflage_type(&self, unit_type: u32) -> bool {
        self.crossings.camouflages(unit_type as usize)
    }

    /// Camouflage of a unit: `None` without; `Some(false)` while it is put
    /// up, `Some(true)` once it is up. [`Self::camouflage_hides`] tells
    /// whether it hides the unit now.
    pub fn camouflage_of(&self, id: UnitId) -> Option<bool> {
        self.crossings.camo.get(&id).map(|c| c.setup == 0)
    }

    /// Whether the camouflage of unit `id` hides it now (up and settled).
    pub fn camouflage_hides(&self, id: UnitId) -> bool {
        let settle = self
            .crossings
            .ticks(self.crossings.rules.camouflage.settle_s);
        self.crossings
            .camo
            .get(&id)
            .is_some_and(|c| c.setup == 0 && c.still >= settle)
    }

    /// Every bridge-laying tank of the match (alive or not), by id.
    pub fn layers(&self) -> Vec<LayerView> {
        let lay_total = self
            .crossings
            .ticks(self.crossings.rules.layer.lay_s)
            .max(1);
        let pick_total = self
            .crossings
            .ticks(self.crossings.rules.layer.pickup_s)
            .max(1);
        let pml = |left: u32, total: u32| (total - left.min(total)) * 1000 / total;
        self.units
            .iter()
            .filter_map(|u| {
                let lt = (*self.crossings.layer_types.get(u.unit_type as usize)?)?;
                let layer = self
                    .crossings
                    .layers
                    .get(&u.id)
                    .copied()
                    .unwrap_or_default();
                let state = match (self.crossings.jobs.get(&u.id), layer) {
                    (_, Layer { lost: true, .. }) => LayerState::Lost,
                    (
                        Some(Job::PickUp { left, .. }),
                        Layer {
                            bridge: Some(b),
                            laid: true,
                            ..
                        },
                    ) if *left < pick_total => LayerState::PickingUp(b, pml(*left, pick_total)),
                    (
                        Some(Job::PickUp { .. }),
                        Layer {
                            bridge: Some(b),
                            laid: true,
                            ..
                        },
                    ) => LayerState::Fetching(b),
                    (
                        _,
                        Layer {
                            bridge: Some(b),
                            laid: true,
                            ..
                        },
                    ) => LayerState::Laid(b),
                    (Some(Job::Lay { left, .. }), _) if *left < lay_total => {
                        LayerState::Laying(pml(*left, lay_total))
                    }
                    (Some(Job::Lay { .. }), _) => LayerState::Driving,
                    _ => LayerState::Carrying,
                };
                Some(LayerView {
                    unit: u.id,
                    owner: u.owner,
                    state,
                    span_mm: lt.span_mm,
                    load_kg: lt.load_kg,
                })
            })
            .collect()
    }

    /// The gap a bridge-laying tank is about to span (driving or laying).
    pub fn layer_gap(&self, id: UnitId) -> Option<Gap> {
        match self.crossings.jobs.get(&id)? {
            Job::Lay { gap, .. } => Some(*gap),
            Job::PickUp { .. } => None,
        }
    }

    /// Squads in deep water, by id.
    pub fn swimmers(&self) -> Vec<SwimmerView> {
        self.crossings
            .swimmers
            .iter()
            .map(|(&unit, s)| SwimmerView { unit, bank: s.bank })
            .collect()
    }

    /// Paratroopers in the air; with `player`, only those over ground the
    /// player sees and the player's team's own.
    pub fn parachutes(&self, player: Option<PlayerId>) -> Vec<ParachuteView> {
        self.crossings
            .paras
            .iter()
            .filter(|p| {
                player.is_none_or(|q| {
                    self.teams.allied(q, p.owner) || self.player_sees_point(q, p.land)
                })
            })
            .map(|p| {
                let mut position = p.at();
                let ground = if self.map.contains(p.land) {
                    self.map.height_at(p.land)
                } else {
                    Fixed::ZERO
                };
                let s = Fixed::from_num(p.elapsed) / Fixed::from_num(p.total.max(1));
                position.z = p.from.z + (ground - p.from.z) * s;
                ParachuteView {
                    id: p.id,
                    owner: p.owner,
                    unit_type: p.unit_type,
                    position,
                    landing: p.land,
                    soldiers: p.soldiers,
                    ticks_left: p.total - p.elapsed.min(p.total),
                }
            })
            .collect()
    }
}
