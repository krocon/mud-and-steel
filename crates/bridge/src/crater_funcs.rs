//! `SimHost` functions for craters (prompt 021): the crater events of
//! every tick with the map samples they changed, and a debug crater. Godot
//! rebuilds the terrain textures and the decoration of those samples.

use crate::helpers::collect_tick_events;
use crate::{SimHost, fixed_to_f32, ground_to_godot, meters_to_millimeters};
use godot::prelude::*;
use sim::{CraterEvent, Sim};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 4096;

/// Appends the crater events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, CraterEvent)>, sim: &Sim) {
    collect_tick_events(pending, sim.tick(), sim.crater_events(), MAX_PENDING_EVENTS);
}

#[godot_api(secondary)]
impl SimHost {
    /// Craters made since the last call, in order: `{tick, position
    /// (Vector3 on the new ground), radius, depth (meters, the bowl),
    /// caliber (mm), rect (Rect2i of map samples that may have changed,
    /// for `terrain.heights_changed`)}`.
    #[func]
    fn take_crater_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.crater_events);
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let to_i32 = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
        events
            .into_iter()
            .map(|(tick, e)| {
                let mut d = VarDictionary::new();
                d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
                d.set(
                    "position",
                    ground_to_godot(e.at, game.map().height_at(e.at)),
                );
                d.set("radius", fixed_to_f32(e.radius));
                d.set("depth", fixed_to_f32(e.depth));
                d.set("caliber", e.caliber_mm_x10 as f32 / 10.0);
                let (i0, j0, i1, j1) = e.samples;
                d.set(
                    "rect",
                    Rect2i::new(
                        Vector2i::new(to_i32(i0), to_i32(j0)),
                        Vector2i::new(to_i32(i1 - i0 + 1), to_i32(j1 - j0 + 1)),
                    ),
                );
                d
            })
            .collect()
    }

    /// The prepared craters of the map (prompt 124, presentation): each
    /// `{position (Vector3 on the ground of the match: the bottom of the
    /// crater), radius (m, the bowl), outer (m, where the rim ends), depth
    /// (m, the bowl), water (rain water stands in it)}`.
    #[func]
    fn map_craters(&self) -> Array<VarDictionary> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let map = game.map();
        let rules = game.sim.crater_rules();
        map.prepared()
            .craters
            .iter()
            .map(|c| {
                let size = rules.size(c.caliber_mm.saturating_mul(10));
                let mut d = VarDictionary::new();
                d.set(
                    "position",
                    ground_to_godot(c.position, map.height_at(c.position)),
                );
                d.set("radius", size.radius_mm as f32 / 1000.0);
                d.set("outer", size.outer_mm as f32 / 1000.0);
                d.set("depth", size.depth_mm as f32 / 1000.0);
                d.set("water", c.water);
                d
            })
            .collect()
    }

    /// The puddles of the map's wet hollows (prompt 124, `[[puddle]]`,
    /// presentation only): `{position (Vector3 on the ground), radius
    /// (m)}`.
    #[func]
    fn map_puddles(&self) -> Array<VarDictionary> {
        let Some(game) = self.game.as_ref() else {
            return Array::new();
        };
        let map = game.map();
        map.prepared()
            .puddles
            .iter()
            .map(|p| {
                let mut d = VarDictionary::new();
                d.set(
                    "position",
                    ground_to_godot(p.position, map.height_at(p.position)),
                );
                d.set("radius", p.radius_mm as f32 / 1000.0);
                d
            })
            .collect()
    }

    /// Craters made in this match so far.
    #[func]
    fn crater_count(&self) -> i64 {
        self.game.as_ref().map_or(0, |g| {
            i64::try_from(g.sim.crater_count()).unwrap_or(i64::MAX)
        })
    }

    /// Height of map sample `(i, j)` minus its loaded height in meters
    /// (craters; 0 elsewhere).
    #[func]
    fn deformation_at(&self, i: i64, j: i64) -> f32 {
        let (Some(game), Ok(i), Ok(j)) = (self.game.as_ref(), u32::try_from(i), u32::try_from(j))
        else {
            return 0.0;
        };
        game.sim.deformation_mm(i, j) as f32 / 1000.0
    }

    /// Debug and benchmarks: queues a crater of a round of `caliber_mm`
    /// at the ground point `(x, z)`, made in the next tick like a ground
    /// hit. Only in matches without other human players (the command
    /// does not go over the network); the replay of such a match no
    /// longer verifies. Returns whether it was queued.
    #[func]
    fn debug_crater(&mut self, x: f32, z: f32, caliber_mm: f32) -> bool {
        let Some(game) = self.game.as_mut() else {
            return false;
        };
        let (Some(x), Some(z)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            return false;
        };
        if game.networked {
            godot_warn!("debug craters are not possible in network matches");
            return false;
        }
        let caliber = (caliber_mm * 10.0).round().clamp(0.0, 100_000.0) as u32;
        game.sim
            .queue_crater(sim::Vec2::from_millimeters(x, z), caliber);
        true
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Height ranges of the terrain chunks of `cells` x `cells` cells that
    /// contain a sample of `rect`: `(chunk index, lowest, highest)` per
    /// entry, for updating `terrain_chunk_ranges` after craters without
    /// recomputing the whole map.
    #[func]
    fn terrain_chunk_ranges_in(&self, cells: i64, rect: Rect2i) -> PackedVector3Array {
        let Some(game) = self.game.as_ref() else {
            return PackedVector3Array::new();
        };
        let grid = game.grid();
        let Some(rect) = grid.clip(
            i64::from(rect.position.x),
            i64::from(rect.position.y),
            i64::from(rect.size.x),
            i64::from(rect.size.y),
        ) else {
            return PackedVector3Array::new();
        };
        let cells = u32::try_from(cells.clamp(1, 4096)).unwrap_or(1);
        grid.chunk_ranges_in(cells, rect)
            .into_iter()
            .map(|(k, lo, hi)| Vector3::new(k as f32, lo, hi))
            .collect()
    }
}
