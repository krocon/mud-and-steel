//! `SimHost` functions for transport and towing (prompt 034): the four
//! orders, who rides in or is towed by whom, and what a vehicle can carry
//! or pull. The simulation decides everything; Godot draws passengers and
//! towed guns (`scenes/world/transport_view.gd`) and picks the orders
//! (`scenes/selection/transport_orders.gd`).

use crate::{SimHost, to_unit_ids};
use godot::prelude::*;
use sim::{Command, UnitId};

fn unit_id(id: i64) -> Option<UnitId> {
    u32::try_from(id).ok().map(UnitId)
}

#[godot_api(secondary)]
impl SimHost {
    /// The squads walk to the own vehicle `carrier` and mount it.
    #[func]
    fn embark_units(&mut self, unit_ids: PackedInt64Array, carrier: i64) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(carrier)) = (units.is_empty(), unit_id(carrier)) {
            self.pending.push(Command::Embark { units, carrier });
        }
    }

    /// Passengers of the listed vehicles (or the listed passengers) get
    /// off once their vehicle stands.
    #[func]
    fn disembark_units(&mut self, unit_ids: PackedInt64Array) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::Disembark { units });
        }
    }

    /// The nearest free vehicle of the list that can pull `gun` hitches it.
    #[func]
    fn tow_gun(&mut self, unit_ids: PackedInt64Array, gun: i64) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(gun)) = (units.is_empty(), unit_id(gun)) {
            self.pending.push(Command::Tow { units, gun });
        }
    }

    /// Towed guns of the listed vehicles (or the listed guns) are unhitched.
    #[func]
    fn unlimber_units(&mut self, unit_ids: PackedInt64Array) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::Unlimber { units });
        }
    }

    /// The vehicle every unit rides in or is towed by, -1 if none (order
    /// of `unit_ids`).
    #[func]
    fn unit_carriers(&self) -> PackedInt64Array {
        self.current
            .units
            .iter()
            .map(|u| u.carrier.map_or(-1, |c| i64::from(c.0)))
            .collect()
    }

    /// Soldiers riding in or on every unit.
    #[func]
    fn unit_passengers(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| i32::from(u.passengers))
            .collect()
    }

    /// The gun every unit tows, -1 if none.
    #[func]
    fn unit_towing(&self) -> PackedInt64Array {
        self.current
            .units
            .iter()
            .map(|u| u.towing.map_or(-1, |g| i64::from(g.0)))
            .collect()
    }

    /// Whether every unit is mounting, dismounting, limbering or
    /// unlimbering.
    #[func]
    fn unit_loading(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.loading))
            .collect()
    }

    /// Seats for soldiers of unit `id` (0: it carries nobody).
    #[func]
    fn unit_seats(&self, id: i64) -> i64 {
        match (&self.game, unit_id(id)) {
            (Some(game), Some(id)) => i64::from(game.sim.seats(id)),
            _ => 0,
        }
    }

    /// Whether vehicle `tractor` can pull gun `gun`.
    #[func]
    fn can_tow(&self, tractor: i64, gun: i64) -> bool {
        match (&self.game, unit_id(tractor), unit_id(gun)) {
            (Some(game), Some(t), Some(g)) => game.sim.can_tow(t, g),
            _ => false,
        }
    }

    /// Whether unit `id` is a towed gun.
    #[func]
    fn is_towed_gun(&self, id: i64) -> bool {
        let (Some(game), Some(id)) = (&self.game, unit_id(id)) else {
            return false;
        };
        self.current
            .units
            .iter()
            .find(|u| u.id == id)
            .is_some_and(|u| game.sim.is_towed_gun_type(u.unit_type))
    }
}
