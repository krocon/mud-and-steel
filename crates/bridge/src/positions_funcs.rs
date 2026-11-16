//! `SimHost` functions for positions tier 2 (prompt 052): dug-in vehicles,
//! camouflaged own units, works a shell struck, the strongpoint layout and
//! a debug strongpoint. The orders go through `order_works`
//! (`works_funcs.rs`); Godot draws in `scenes/world/works_view.gd` and
//! `scenes/world/trench_builder.gd`.

use crate::{SimHost, ground_to_godot, meters_to_millimeters};
use godot::prelude::*;
use sim::Vec2;

fn ground_point(x: f32, z: f32) -> Option<Vec2> {
    Some(Vec2::from_millimeters(
        meters_to_millimeters(x)?,
        meters_to_millimeters(z)?,
    ))
}

#[godot_api(secondary)]
impl SimHost {
    /// Ids of the vehicles dug in in a vehicle pit (visible or not: the
    /// presentation shows only units it draws).
    #[func]
    fn dug_in_units(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim.dug_in().iter().map(|id| i64::from(id.0)).collect()
        })
    }

    /// Ids of the local team's units that are camouflaged under a net now.
    #[func]
    fn camouflaged_units(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim
                .camouflaged_units(g.session.local_player())
                .iter()
                .map(|id| i64::from(id.0))
                .collect()
        })
    }

    /// Ids of dug works (trenches, nests, dugouts, pits) a shell struck.
    #[func]
    fn damaged_works(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim
                .damaged_works()
                .iter()
                .map(|&id| i64::from(id))
                .collect()
        })
    }

    /// The pieces of a strongpoint around `(x, z)`: `{kind, from, to}`
    /// (Vector3 on the ground) per piece, before it is cut into zigzags.
    #[func]
    fn strongpoint_layout(&self, x: f32, z: f32) -> Array<VarDictionary> {
        let (Some(game), Some(center)) = (&self.game, ground_point(x, z)) else {
            return Array::new();
        };
        let map = game.sim.map();
        game.sim
            .strongpoint_preview(center)
            .iter()
            .map(|p| {
                let mut d = VarDictionary::new();
                d.set("kind", p.kind.name());
                d.set("from", ground_to_godot(p.from, map.height_at(p.from)));
                d.set("to", ground_to_godot(p.to, map.height_at(p.to)));
                d
            })
            .collect()
    }

    /// Debug, self-test and screenshots: builds a neutral strongpoint
    /// around `(x, z)` at once, like a map marker. Only in matches without
    /// other human players (it does not go over the network). Returns the
    /// number of works placed.
    #[func]
    fn debug_strongpoint(&mut self, x: f32, z: f32) -> i64 {
        let Some(game) = self.game.as_mut() else {
            return 0;
        };
        if game.networked {
            godot_warn!("debug strongpoints are not possible in network matches");
            return 0;
        }
        let Some(p) = ground_point(x, z) else {
            return 0;
        };
        i64::try_from(game.sim.place_strongpoint(p)).unwrap_or(0)
    }
}
