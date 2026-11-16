//! Destruction of map objects (prompt 020): hit points and stages of every
//! object, crushing by vehicles, and keeping the path grid and the vision
//! grid in step with what is left. The rules: `crate::destruction`.
//!
//! The map stays as it was loaded; the state lives here. Only objects that
//! differ from their start (the stage their map variant gives) are stored,
//! so the state hash stays small on maps with 100 000 objects.
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::crush_objects`] after
//! the movement, projectile hits in [`Sim::fly_projectiles`], then
//! [`Sim::apply_destruction`] rebuilds the path grid (components once) and
//! the vision cells of every object destroyed in this tick, re-plans routes
//! through them and queues the viewsheds that contain them.

use super::Sim;
use crate::combat::Vec3;
use crate::command::UnitId;
use crate::data::UnitType;
use crate::destruction::{
    ClassRule, DestructionRules, ObjectCause, ObjectEvent, ObjectStage, ObjectState, Remains,
};
use crate::hash::StableHasher;
use crate::map::Map;
use crate::math::{Fixed, Vec2};
use crate::objects::{Blocks, MapObject, Shape, sin_cos_deg};
use crate::vision::{Sight, VisionRules};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Edge of the buckets of the object index in meters.
const BUCKET_M: i32 = 16;

/// How an object acts in its current stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Effective {
    pub(super) blocks: Blocks,
    pub(super) sight: Sight,
    /// Top above the ground for sight and projectiles, meters.
    pub(super) height: Fixed,
}

/// Destruction state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Destruction {
    rules: DestructionRules,
    /// Per map object kind: index into `rules.classes`.
    kind_class: Vec<Option<usize>>,
    /// Per map object kind: the stage its variant starts in.
    kind_stage: Vec<ObjectStage>,
    /// Per map object kind: how it affects sight (`data/vision.toml`).
    kind_sight: Vec<Sight>,
    /// Objects whose state differs from their start, by index.
    changed: BTreeMap<u32, ObjectState>,
    /// Per unit type: catalog mass (kg).
    masses: Vec<u32>,
    /// Lightest vehicle that crushes anything (`None`: nothing to crush).
    min_crush_mass: Option<u32>,
    /// Objects that block, affect sight or can be destroyed, bucketed.
    origin: Vec2,
    columns: u32,
    rows: u32,
    buckets: Vec<Vec<u32>>,
    /// Ground areas of objects destroyed this tick (not yet applied).
    pending: Vec<(Vec2, Vec2)>,
    /// Events of the last tick (derived, not hashed).
    pub(super) events: Vec<ObjectEvent>,
}

impl Destruction {
    pub(super) fn new(
        rules: DestructionRules,
        map: &Map,
        vision: &VisionRules,
        unit_types: &[UnitType],
    ) -> Self {
        let kinds = map.object_kinds();
        let kind_class: Vec<Option<usize>> = kinds.iter().map(|k| rules.class_of(&k.id)).collect();
        let kind_stage = kinds
            .iter()
            .zip(&kind_class)
            .map(|(k, c)| {
                let Some(class) = c.map(|c| &rules.classes[c]) else {
                    return ObjectStage::Intact;
                };
                if class.ruin_variant.as_deref() == Some(k.variant.as_str()) {
                    ObjectStage::Destroyed
                } else if class.damaged_variant.as_deref() == Some(k.variant.as_str()) {
                    ObjectStage::Damaged
                } else {
                    ObjectStage::Intact
                }
            })
            .collect();
        let kind_sight: Vec<Sight> = kinds.iter().map(|k| vision.object_sight(&k.id)).collect();
        let size = map.size();
        let bucket = Fixed::from_num(BUCKET_M);
        let columns = (size.x / bucket).ceil().to_num::<u32>() + 1;
        let rows = (size.y / bucket).ceil().to_num::<u32>() + 1;
        let min_crush_mass = rules
            .classes
            .iter()
            .map(|c| c.crush_min_mass_kg)
            .filter(|&m| m > 0)
            .min();
        let mut d = Self {
            kind_class,
            kind_stage,
            kind_sight,
            changed: BTreeMap::new(),
            masses: unit_types.iter().map(|t| t.mass_kg).collect(),
            min_crush_mass,
            origin: Vec2::new(-size.x / 2, -size.y / 2),
            columns,
            rows,
            buckets: vec![Vec::new(); (columns * rows) as usize],
            pending: Vec::new(),
            events: Vec::new(),
            rules,
        };
        for (i, object) in (0u32..).zip(map.objects()) {
            let k = usize::from(object.kind);
            let kind = &kinds[k];
            if kind.blocks == Blocks::None
                && d.kind_sight[k] == Sight::Clear
                && d.kind_class[k].is_none()
            {
                continue;
            }
            let extent = object.half_extent(kind.shape);
            let (c0, r0) = d.bucket(object.position - extent);
            let (c1, r1) = d.bucket(object.position + extent);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    d.buckets[(r * columns + c) as usize].push(i);
                }
            }
            // Objects placed as ruins act as ruins from the start: the
            // grids were built from their kind and are corrected once.
            if d.kind_stage[k] == ObjectStage::Destroyed {
                d.pending.push(area(object, kind.shape));
            }
        }
        d
    }

    fn bucket(&self, p: Vec2) -> (u32, u32) {
        let bucket = Fixed::from_num(BUCKET_M);
        let axis = |v: Fixed, o: Fixed, n: u32| {
            ((v - o) / bucket)
                .floor()
                .max(Fixed::ZERO)
                .to_num::<u32>()
                .min(n - 1)
        };
        (
            axis(p.x, self.origin.x, self.columns),
            axis(p.y, self.origin.y, self.rows),
        )
    }

    /// Indices of the indexed objects whose bucket overlaps `low..high`
    /// (ascending, each once).
    pub(super) fn query(&self, low: Vec2, high: Vec2) -> Vec<u32> {
        let (c0, r0) = self.bucket(low);
        let (c1, r1) = self.bucket(high);
        let mut found = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                found.extend(&self.buckets[(r * self.columns + c) as usize]);
            }
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    fn class(&self, map: &Map, i: u32) -> Option<&ClassRule> {
        let k = usize::from(map.objects()[i as usize].kind);
        self.kind_class[k].map(|c| &self.rules.classes[c])
    }

    /// State of object `i` (`i` must be a valid index).
    pub(super) fn state(&self, map: &Map, i: u32) -> ObjectState {
        if let Some(&s) = self.changed.get(&i) {
            return s;
        }
        let k = usize::from(map.objects()[i as usize].kind);
        let stage = self.kind_stage[k];
        let hp = self.kind_class[k].map_or(0, |c| {
            let class = &self.rules.classes[c];
            match stage {
                ObjectStage::Intact => class.hp,
                ObjectStage::Damaged => class.damaged_hp(),
                ObjectStage::Destroyed => 0,
            }
        });
        ObjectState {
            hp,
            stage,
            fall_deg: 0,
        }
    }

    /// How object `i` acts now.
    pub(super) fn effective(&self, map: &Map, i: u32) -> Effective {
        let object = &map.objects()[i as usize];
        let k = usize::from(object.kind);
        let kind = &map.object_kinds()[k];
        let intact = Effective {
            blocks: kind.blocks,
            sight: self.kind_sight[k],
            height: kind.height,
        };
        let Some(class) = self.kind_class[k].map(|c| &self.rules.classes[c]) else {
            return intact;
        };
        if self.state(map, i).stage != ObjectStage::Destroyed {
            return intact;
        }
        match class.remains {
            // Rubble and wall stumps with gaps: they stop projectiles but
            // not sight (at 8 m vision cells a ruin would otherwise blind
            // the infantry inside it).
            Remains::Ruin => Effective {
                blocks: class.ruin_blocks,
                sight: Sight::Clear,
                height: kind.height * Fixed::from_num(class.ruin_height_pct) / 100,
            },
            Remains::Fallen | Remains::Gone | Remains::Flat => Effective {
                blocks: Blocks::None,
                sight: Sight::Clear,
                height: Fixed::ZERO,
            },
        }
    }

    /// Height up to which object `i` stops projectiles, if it does.
    pub(super) fn hit_height(&self, map: &Map, i: u32) -> Option<Fixed> {
        let e = self.effective(map, i);
        (e.blocks.large() && e.height > Fixed::ZERO).then_some(e.height)
    }

    /// Whether object `i` is a ruin now.
    pub(super) fn is_ruin(&self, map: &Map, i: u32) -> bool {
        self.class(map, i)
            .is_some_and(|c| c.remains == Remains::Ruin)
            && self.state(map, i).stage == ObjectStage::Destroyed
    }

    /// A projectile `(caliber_mm_x10, damage it would do to a vehicle)`
    /// hit object `i` at `at`, flying along `direction`. Returns the
    /// damage the object took.
    pub(super) fn hit(
        &mut self,
        map: &Map,
        i: u32,
        (caliber_mm_x10, damage): (u32, u32),
        direction: Vec2,
        at: Vec3,
        shooter: UnitId,
    ) -> u32 {
        if caliber_mm_x10 < self.rules.damage.min_caliber_mm * 10 || damage == 0 {
            return 0;
        }
        let Some(class) = self.class(map, i) else {
            return 0;
        };
        let mut state = self.state(map, i);
        if class.hp == 0 || state.stage == ObjectStage::Destroyed {
            return 0;
        }
        let taken = damage.min(state.hp);
        state.hp -= taken;
        let stage = class.stage_at(state.hp);
        if stage == ObjectStage::Destroyed {
            state.fall_deg = heading_deg(direction);
        }
        self.set(map, i, state, stage, at, ObjectCause::Hit(shooter));
        taken
    }

    /// Fire (prompt 041) takes `damage` hit points of object `i`; the
    /// fire was started by `by`. Returns the damage taken.
    pub(super) fn burn(&mut self, map: &Map, i: u32, damage: u32, at: Vec3, by: UnitId) -> u32 {
        let Some(class) = self.class(map, i) else {
            return 0;
        };
        let mut state = self.state(map, i);
        if class.hp == 0 || state.stage == ObjectStage::Destroyed || damage == 0 {
            return 0;
        }
        let taken = damage.min(state.hp);
        state.hp -= taken;
        let stage = class.stage_at(state.hp);
        self.set(map, i, state, stage, at, ObjectCause::Fire(by));
        taken
    }

    /// Stores the new state of object `i`; on a change of stage an event,
    /// and for destroyed objects the area to rebuild.
    fn set(
        &mut self,
        map: &Map,
        i: u32,
        mut state: ObjectState,
        stage: ObjectStage,
        at: Vec3,
        cause: ObjectCause,
    ) {
        let before = state.stage;
        state.stage = stage;
        self.changed.insert(i, state);
        if stage == before {
            return;
        }
        let object = &map.objects()[i as usize];
        let remains = self.class(map, i).map_or(Remains::Gone, |c| c.remains);
        self.events.push(ObjectEvent {
            object: i,
            stage,
            remains,
            fall_deg: state.fall_deg,
            at,
            cause,
        });
        if stage == ObjectStage::Destroyed {
            let shape = map.object_kinds()[usize::from(object.kind)].shape;
            self.pending.push(area(object, shape));
        }
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u64(self.changed.len() as u64);
        for (&i, s) in &self.changed {
            h.write_u32(i);
            h.write_u32(s.hp);
            h.write_u8(s.stage.code());
            h.write_u32(u32::from(s.fall_deg));
        }
    }
}

/// Axis-aligned ground area of an object.
fn area(object: &MapObject, shape: Shape) -> (Vec2, Vec2) {
    let extent = object.half_extent(shape);
    (object.position - extent, object.position + extent)
}

/// Whole-degree heading (`0..360`, clockwise from north = -y) closest to
/// `v`; 0 for a zero vector.
pub(super) fn heading_deg(v: Vec2) -> u16 {
    let mut best = (Fixed::MIN, 0u16);
    for deg in 0..360u16 {
        let (sin, cos) = sin_cos_deg(deg);
        let along = v.x * sin - v.y * cos;
        if along > best.0 {
            best = (along, deg);
        }
    }
    if v == Vec2::ZERO { 0 } else { best.1 }
}

impl Sim {
    /// Vehicles heavy enough flatten crushable objects (fences, barbed
    /// wire) under their footprint. Also starts the tick's object events,
    /// so it runs before any other destruction step of the tick.
    pub(super) fn crush_objects(&mut self) {
        self.destruction.events.clear();
        let Some(min_mass) = self.destruction.min_crush_mass else {
            return;
        };
        let map = Arc::clone(&self.map);
        for unit in &self.units {
            let mass = self.destruction.masses[unit.unit_type as usize];
            if !unit.alive() || mass < min_mass {
                continue;
            }
            let half = unit.footprint / 2;
            let reach = Vec2::new(half, half);
            for i in self
                .destruction
                .query(unit.position - reach, unit.position + reach)
            {
                let d = &self.destruction;
                let Some(class) = d.class(&map, i) else {
                    continue;
                };
                if class.crush_min_mass_kg == 0 || mass < class.crush_min_mass_kg {
                    continue;
                }
                let state = d.state(&map, i);
                if state.stage == ObjectStage::Destroyed {
                    continue;
                }
                let object = &map.objects()[i as usize];
                let shape = map.object_kinds()[usize::from(object.kind)].shape;
                if !object.overlaps_square(shape, unit.position, half) {
                    continue;
                }
                let at = Vec3::new(unit.position.x, unit.position.y, unit.height);
                let state = ObjectState {
                    hp: 0,
                    stage: state.stage,
                    fall_deg: heading_deg(unit.facing),
                };
                self.destruction.set(
                    &map,
                    i,
                    state,
                    ObjectStage::Destroyed,
                    at,
                    ObjectCause::Crushed(unit.id),
                );
            }
        }
    }

    /// Brings the path grid and the vision grid up to date with the objects
    /// destroyed since the last call: object flags, costs and components of
    /// their cells, routes through them, vision cells and the viewsheds
    /// that contain them.
    pub(super) fn apply_destruction(&mut self) {
        let areas = std::mem::take(&mut self.destruction.pending);
        if areas.is_empty() {
            return;
        }
        let map = Arc::clone(&self.map);
        let d = &self.destruction;
        let objects = |low: Vec2, high: Vec2| {
            d.query(low, high).into_iter().map(|i| {
                let object = map.objects()[i as usize];
                let shape = map.object_kinds()[usize::from(object.kind)].shape;
                (object, shape, d.effective(&map, i))
            })
        };
        let grid = &mut self.paths.grid;
        let half_cell = grid.cell_size() / 2;
        let half = Vec2::new(half_cell, half_cell);
        let mut changed = Vec::with_capacity(areas.len());
        for &(low, high) in &areas {
            let rect = grid.cells_in(low, high);
            let (a, b) = (grid.center(rect.x0, rect.y0), grid.center(rect.x1, rect.y1));
            let list = objects(a - half, b + half).map(|(o, s, e)| (o, s, e.blocks));
            grid.rasterize_objects_in(rect, list);
            changed.push(grid.rebuild_cells(&map, &self.terrain, rect));
        }
        grid.relabel_components_in(&changed);
        let vision = Arc::make_mut(&mut self.vision.grid);
        let half_cell = vision.cell / 2;
        let half = Vec2::new(half_cell, half_cell);
        let mut cells = Vec::with_capacity(areas.len());
        for &(low, high) in &areas {
            let (c0, r0) = vision.cell_of(low);
            let (c1, r1) = vision.cell_of(high);
            let (c0, r0, c1, r1) = (c0 as u32, r0 as u32, c1 as u32, r1 as u32);
            let (a, b) = (vision.center(c0, r0), vision.center(c1, r1));
            let list = objects(a - half, b + half).map(|(o, s, e)| (o, s, e.sight, e.height));
            vision.rebuild_objects((c0, r0, c1, r1), list);
            cells.push((c0, r0, c1, r1));
        }
        for rect in changed {
            self.replan_through(rect);
        }
        for rect in cells {
            self.requeue_viewsheds(rect);
        }
    }

    /// Damage stage, hit points and fall direction of map object `index`
    /// (into [`Map::objects`]); `None` for an unknown index.
    pub fn object_state(&self, index: u32) -> Option<ObjectState> {
        ((index as usize) < self.map.objects().len())
            .then(|| self.destruction.state(&self.map, index))
    }

    /// Damage stage of map object `index` (intact, damaged, destroyed), for
    /// cover and occupying buildings (prompt 022).
    pub fn object_stage(&self, index: u32) -> Option<ObjectStage> {
        self.object_state(index).map(|s| s.stage)
    }

    /// Hit points of map object `index` and of an intact one of its kind
    /// (both 0 for objects that cannot be shot to pieces).
    pub fn object_hp(&self, index: u32) -> Option<(u32, u32)> {
        let state = self.object_state(index)?;
        let max = self.destruction.class(&self.map, index).map_or(0, |c| c.hp);
        Some((state.hp, max))
    }

    /// Which path size classes map object `index` blocks in its current
    /// stage (ruins block less than the intact building).
    pub fn object_blocks(&self, index: u32) -> Option<Blocks> {
        ((index as usize) < self.map.objects().len())
            .then(|| self.destruction.effective(&self.map, index).blocks)
    }

    /// What remains of map object `index` once destroyed (`None` for
    /// objects that cannot be destroyed).
    pub fn object_remains(&self, index: u32) -> Option<Remains> {
        if (index as usize) >= self.map.objects().len() {
            return None;
        }
        self.destruction.class(&self.map, index).map(|c| c.remains)
    }

    /// Every object whose state differs from its start, ascending by index.
    pub fn changed_objects(&self) -> impl Iterator<Item = (u32, ObjectState)> + '_ {
        self.destruction.changed.iter().map(|(&i, &s)| (i, s))
    }

    /// Objects that changed their stage during the last tick, in order.
    pub fn object_events(&self) -> &[ObjectEvent] {
        &self.destruction.events
    }

    /// The destruction rules of this match.
    pub const fn destruction_rules(&self) -> &DestructionRules {
        &self.destruction.rules
    }

    /// Whether map object `index` is a ruin now.
    pub(super) fn object_is_ruin(&self, index: u32) -> bool {
        self.destruction.is_ruin(&self.map, index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_point_like_object_axes() {
        assert_eq!(heading_deg(Vec2::from_meters(0, -1)), 0);
        assert_eq!(heading_deg(Vec2::from_meters(1, 0)), 90);
        assert_eq!(heading_deg(Vec2::from_meters(0, 3)), 180);
        assert_eq!(heading_deg(Vec2::from_meters(-2, 0)), 270);
        assert_eq!(heading_deg(Vec2::from_meters(1, -1)), 45);
        assert_eq!(heading_deg(Vec2::ZERO), 0);
    }
}
