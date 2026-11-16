//! The computer player's bridge-laying tank (prompt 055, Normal and Hard):
//! when deep water lies between its home and the enemy and no bridge stands
//! near the crossing (or the bridges there were blown), it builds one
//! bridge-laying tank in the vehicle works (if its faction has one) and
//! lays the bridge at the narrowest point it finds near the line to the
//! enemy; the army's paths use it like any bridge. Ordinary commands only
//! (`Produce`, `LayBridge`), and only what the public map and the
//! [`PlayerView`] show.

use crate::{Ai, Difficulty, PlayerView, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, LayerState, Map, TerrainRules, Vec2};

/// Bridges within this distance (m) of the crossing count as a way over.
const BRIDGE_NEAR_M: i32 = 150;
/// Points tried along the river on each side of the line to the enemy,
/// this far apart (m).
const ALONG_STEP_M: i32 = 40;
const ALONG_STEPS: i32 = 5;
/// A `LayBridge` order that left the tank carrying its bridge after this
/// many ticks found no gap there: the next point is tried.
const TRY_TICKS: u64 = 60;
/// A production order is repeated after this many ticks at the earliest.
const PRODUCE_TICKS: u64 = 600;
/// Bridge-laying tanks built in a match at most.
const MAX_BUILT: u32 = 2;

/// What the computer player knows and did about the river crossing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LayerPlan {
    /// Points in deep water near the line to the enemy, best first.
    candidates: Vec<Vec2>,
    /// Map bridges near the crossing (indices into `Map::bridges`).
    bridges: Vec<usize>,
    /// Next candidate to try, and when the last one was ordered.
    tried: usize,
    ordered: Option<u64>,
    produced: Option<u64>,
    built: u32,
}

impl LayerPlan {
    /// The first stretch of deep water on the line from `home` to `enemy`
    /// and points along that river on both sides of the line.
    pub(crate) fn new(map: &Map, home: Vec2, enemy: Vec2) -> Self {
        let Ok(rules) = TerrainRules::builtin() else {
            return Self::default();
        };
        let deep = |p: Vec2| {
            map.contains(p)
                && rules
                    .types
                    .get(usize::from(map.ground_terrain_at(p)))
                    .is_some_and(|t| !t.passable && t.name.contains("water"))
        };
        let way = dist(home, enemy);
        let steps = (way / 4).to_num::<i32>();
        // `toward(enemy, home, -x)` lies x from home towards the enemy.
        let at = |k: i32| toward(enemy, home, -Fixed::from_num(4 * k));
        let Some(first) = (1..steps).find(|&k| deep(at(k))) else {
            return Self::default();
        };
        let last = (first..steps).find(|&k| !deep(at(k))).unwrap_or(steps);
        let center = at((first + last) / 2);
        let d = enemy - home;
        let across = if way > Fixed::ZERO {
            Vec2::new(-d.y / way, d.x / way)
        } else {
            Vec2::ZERO
        };
        let mut candidates = vec![center];
        for k in 1..=ALONG_STEPS {
            for sign in [1, -1] {
                let p = center + across.scale(Fixed::from_num(sign * k * ALONG_STEP_M));
                // Follow the river: the deep water nearest to the point on
                // the parallel line.
                if let Some(q) = (0..12)
                    .flat_map(|s| [s, -s])
                    .map(|s| p + Vec2::new(d.x / way, d.y / way).scale(Fixed::from_num(4 * s)))
                    .find(|&q| deep(q))
                {
                    candidates.push(q);
                }
            }
        }
        let near = Fixed::from_num(BRIDGE_NEAR_M);
        let bridges = map
            .bridges()
            .iter()
            .enumerate()
            .filter(|(_, b)| dist(b.center, center) <= near)
            .map(|(i, _)| i)
            .collect();
        Self {
            candidates,
            bridges,
            ..Self::default()
        }
    }
}

impl Ai {
    /// Production and orders of the bridge-laying tank.
    pub(crate) fn command_layers(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !matches!(self.difficulty, Difficulty::Normal | Difficulty::Hard)
            || self.layers.candidates.is_empty()
        {
            return;
        }
        // A standing bridge near the crossing is enough.
        if self
            .layers
            .bridges
            .iter()
            .any(|&b| view.bridges_intact.get(b).copied().unwrap_or(false))
        {
            return;
        }
        let alive: Vec<&sim::LayerView> = view
            .layers
            .iter()
            .filter(|l| view.own.iter().any(|u| u.id == l.unit && u.alive()))
            .collect();
        if alive.iter().any(|l| matches!(l.state, LayerState::Laid(_))) {
            return;
        }
        let Some(layer) = alive.iter().find(|l| l.state != LayerState::Lost) else {
            self.produce_layer(view, out);
            return;
        };
        if layer.state != LayerState::Carrying {
            return;
        }
        let tick = view.tick;
        if self.layers.ordered.is_some_and(|t| tick < t + TRY_TICKS) {
            return;
        }
        if self.layers.ordered.is_some() {
            self.layers.tried += 1;
        }
        let Some(&target) = self
            .layers
            .candidates
            .get(self.layers.tried % self.layers.candidates.len())
        else {
            return;
        };
        self.layers.ordered = Some(tick);
        out.push(Command::LayBridge {
            units: vec![layer.unit],
            target: self.clamp_to_map(target),
        });
    }

    /// With a completed vehicle works: one bridge-laying tank of the
    /// faction, if it has one.
    fn produce_layer(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if self.layers.built >= MAX_BUILT
            || self
                .layers
                .produced
                .is_some_and(|t| view.tick < t + PRODUCE_TICKS)
        {
            return;
        }
        let faction = view
            .own
            .first()
            .and_then(|u| sim::economy::faction_of(self.name_of(u.unit_type)));
        let Some(unit_type) = (0..self.names.len() as u32).find(|&t| {
            self.is_layer[t as usize] && sim::economy::faction_of(self.name_of(t)) == faction
        }) else {
            return;
        };
        let Some(works) = view
            .structures
            .iter()
            .find(|s| s.completed() && self.name_of(s.unit_type) == "base_factory")
        else {
            return;
        };
        if works.queue.contains(&unit_type) {
            return;
        }
        let Some(cost) = self.costs.get(unit_type as usize).copied().flatten() else {
            return;
        };
        if view.account.iter().zip(cost).any(|(&have, c)| have < c) {
            return;
        }
        self.layers.produced = Some(view.tick);
        self.layers.built += 1;
        self.layers.tried = 0;
        self.layers.ordered = None;
        out.push(Command::Produce {
            building: works.unit,
            unit_type,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::map::MapLayout;

    #[test]
    fn finds_the_river_between_the_homes() {
        let (sx, sy) = (101u32, 101u32);
        let mut heights = Vec::new();
        let mut terrain = Vec::new();
        for _ in 0..sy {
            for i in 0..sx {
                let x = 2 * i as i32 - 100;
                let water = (10..=14).contains(&x);
                heights.push(if water { -2000 } else { 0 });
                terrain.push(if water { 7 } else { 0 });
            }
        }
        let layout = MapLayout {
            name: "ai_river".into(),
            samples_x: sx,
            samples_y: sy,
            cell_size_mm: 2000,
            starts: Vec::new(),
            markers: Vec::new(),
        };
        let map = Map::from_rasters(layout, heights, terrain).unwrap();
        let plan = LayerPlan::new(&map, Vec2::from_meters(-80, 0), Vec2::from_meters(80, 0));
        assert!(!plan.candidates.is_empty());
        let c = plan.candidates[0];
        assert!(
            c.x >= Fixed::from_num(9) && c.x <= Fixed::from_num(15),
            "{c:?}"
        );
        assert!(plan.candidates.len() > 3, "points along the river");
        let dry = LayerPlan::new(&map, Vec2::from_meters(-80, 0), Vec2::from_meters(-10, 0));
        assert!(dry.candidates.is_empty());
    }
}
