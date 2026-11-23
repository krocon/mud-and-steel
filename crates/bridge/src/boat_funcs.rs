//! `SimHost` functions for boats (prompt 119): landing on a beach, backing
//! off, the ramp and the cargo for the presentation. The simulation
//! decides everything; Godot draws (`scenes/world/boat_view.gd`) and picks
//! the orders (`scenes/selection/boat_orders.gd`).

use crate::{SimHost, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::{Command, UnitId, Vec2};

#[godot_api(secondary)]
impl SimHost {
    /// Boats among `unit_ids` run onto the beach at the shore point
    /// `(x, z)`. Returns whether a boat is among them (the order goes out
    /// only then).
    #[func]
    fn land_boats(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) -> bool {
        let units: Vec<UnitId> = to_unit_ids(&unit_ids)
            .into_iter()
            .filter(|&id| self.is_boat_unit(id))
            .collect();
        let (Some(x), Some(y)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            return false;
        };
        if units.is_empty() {
            return false;
        }
        let target = Vec2::from_millimeters(x, y);
        self.pending.push(Command::Land { units, target });
        true
    }

    /// Boats among `unit_ids` that lie on a beach back off.
    #[func]
    fn retract_boats(&mut self, unit_ids: PackedInt64Array) {
        let units: Vec<UnitId> = to_unit_ids(&unit_ids)
            .into_iter()
            .filter(|&id| self.is_boat_unit(id))
            .collect();
        if !units.is_empty() {
            self.pending.push(Command::Retract { units });
        }
    }

    /// Whether unit `id` is a boat.
    #[func]
    fn is_boat(&self, id: i64) -> bool {
        u32::try_from(id).is_ok_and(|id| self.is_boat_unit(UnitId(id)))
    }

    /// Phase of boat `id`: "afloat", "landing", "run_in", "ramp",
    /// "beached", "retract"; empty for no boat.
    #[func]
    fn boat_phase(&self, id: i64) -> GString {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return GString::new();
        };
        game.sim
            .boat_phase(UnitId(id))
            .map_or_else(GString::new, GString::from)
    }

    /// Every boat of the match: `unit`, `phase`, `ramp` (0..1, 1 = down),
    /// `soldiers` and `vehicles` aboard, `draft` (m: the water line above
    /// the keel).
    #[func]
    fn boats(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        for b in game.sim.boat_views() {
            let mut d = VarDictionary::new();
            d.set("unit", i64::from(b.unit.0));
            d.set("phase", &GString::from(b.phase));
            d.set("ramp", b.ramp_pct as f32 / 100.0);
            d.set("soldiers", i64::from(b.soldiers));
            d.set("vehicles", i64::from(b.vehicles));
            d.set("draft", b.draft_mm as f32 / 1000.0);
            out.push(&d);
        }
        out
    }

    /// Whether boat `id` floats at `(x, z)` (open water deep enough for
    /// its draft); a right click elsewhere lands it.
    #[func]
    fn boat_can_float(&self, id: i64, x: f32, z: f32) -> bool {
        let (Some(game), Ok(id), Some(x), Some(y)) = (
            &self.game,
            u32::try_from(id),
            meters_to_millimeters(x),
            meters_to_millimeters(z),
        ) else {
            return false;
        };
        let p = Vec2::from_millimeters(x, y);
        game.sim.type_of_unit(UnitId(id)).is_some_and(|t| {
            game.sim.map().contains(p) && game.sim.path_grid().passable_for(t as usize, p)
        })
    }

    /// Vehicles and guns aboard boat `id` (ids), for the unit panel.
    #[func]
    fn boat_cargo(&self, id: i64) -> PackedInt64Array {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return PackedInt64Array::new();
        };
        game.sim
            .boat_cargo(UnitId(id))
            .iter()
            .map(|u| i64::from(u.0))
            .collect()
    }
}

impl SimHost {
    fn is_boat_unit(&self, id: UnitId) -> bool {
        self.game.as_ref().is_some_and(|g| {
            g.sim
                .type_of_unit(id)
                .is_some_and(|t| g.sim.is_boat_type(t))
        })
    }
}
