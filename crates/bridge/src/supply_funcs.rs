//! `SimHost` functions for supply and repair (prompt 036): stocks of every
//! unit for the unit panel, supply and repair radii of the local player's
//! sources, and the building orders `Repair` and `Sell`. The simulation
//! decides everything; Godot shows it (`scenes/hud/unit_panel.gd`,
//! `scenes/hud/supply_rings.gd`, `scenes/world/base_hud.gd`).

use crate::SimHost;
use godot::prelude::*;
use sim::{Command, PlayerId, UnitId};

fn unit_id(id: i64) -> Option<UnitId> {
    u32::try_from(id).ok().map(UnitId)
}

#[godot_api(secondary)]
impl SimHost {
    /// Ammunition of every unit in percent, -1 without a stock (order of
    /// `unit_ids`).
    #[func]
    fn unit_ammo(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| u.stock.ammo_pct.map_or(-1, i32::from))
            .collect()
    }

    /// Fuel of every unit in percent, -1 without a stock.
    #[func]
    fn unit_fuel(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| u.stock.fuel_pct.map_or(-1, i32::from))
            .collect()
    }

    /// Whether every unit is a vehicle too damaged to move.
    #[func]
    fn unit_immobile(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.stock.immobile))
            .collect()
    }

    /// Ammunition and fuel shares (percent) at which the HUD warns:
    /// `[ammo, fuel]`.
    #[func]
    fn supply_low_pct(&self) -> PackedInt32Array {
        let Some(game) = &self.game else {
            return PackedInt32Array::new();
        };
        let rules = game.sim.logistics_rules();
        [rules.ammo.low_pct, rules.fuel.low_pct]
            .into_iter()
            .map(|v| i32::try_from(v).unwrap_or(0))
            .collect()
    }

    /// Supply and repair radius in meters of unit `id` if it is a working
    /// source of the local player or an ally: `[supply_m, repair_m]`
    /// (0 where it is none); empty otherwise.
    #[func]
    fn supply_radius(&self, id: i64) -> PackedFloat32Array {
        let (Some(game), Some(id)) = (&self.game, unit_id(id)) else {
            return PackedFloat32Array::new();
        };
        let local = PlayerId(u8::try_from(self.local_player()).unwrap_or(0));
        game.sim
            .supply_sources(local)
            .into_iter()
            .find(|s| s.unit == id)
            .map_or_else(PackedFloat32Array::new, |s| {
                [s.supply_m as f32, s.repair_m as f32].into_iter().collect()
            })
    }

    /// Starts or stops repairing the own building `id`.
    #[func]
    fn repair_building(&mut self, id: i64) {
        if let Some(building) = unit_id(id) {
            self.pending.push(Command::Repair { building });
        }
    }

    /// Sells the own building `id` or cancels the construction site.
    #[func]
    fn sell_building(&mut self, id: i64) {
        if let Some(building) = unit_id(id) {
            self.pending.push(Command::Sell { building });
        }
    }

    /// Whether building `id` is being repaired.
    #[func]
    fn building_repairing(&self, id: i64) -> bool {
        match (&self.game, unit_id(id)) {
            (Some(game), Some(id)) => game.sim.building_repairing(id),
            _ => false,
        }
    }
}
