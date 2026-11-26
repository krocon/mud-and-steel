//! `SimHost` functions for infantry (prompt 022): soldiers of every squad
//! (positions, hit points, cover), stances, buildings and their orders, and
//! starting a scenario file (benchmarks). The simulation decides everything;
//! Godot only draws it (`scenes/world/infantry_view.gd`).

use crate::{SimHost, ground_to_godot, load_map_dir, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::scenario::{BUILTIN_MAP_NAME, builtin_map, parse_scenario, scenario_map_name};
use sim::{
    BuildingCondition, Catalog, Command, Scale, SoldierSnapshot, Stance, StanceOrder, TerrainRules,
    Vec2,
};
use std::path::Path;
use std::sync::Arc;

fn stance_code(stance: Option<Stance>) -> i32 {
    match stance {
        None => -1,
        Some(Stance::Stand) => 0,
        Some(Stance::Kneel) => 1,
        Some(Stance::Prone) => 2,
    }
}

/// Matches `current` soldiers against `previous` in a single O(n + m) merge
/// (both slices are sorted ascending by `(unit, index)`), returning each
/// soldier's previous position or its current position when newly visible
/// (prompt 209).
fn previous_soldier_positions(
    previous: &[SoldierSnapshot],
    current: &[SoldierSnapshot],
) -> Vec<Vector3> {
    let mut out = Vec::with_capacity(current.len());
    let mut j = 0usize;
    for s in current {
        let key = (s.unit, s.index);
        while j < previous.len() && (previous[j].unit, previous[j].index) < key {
            j += 1;
        }
        let before = if j < previous.len() && (previous[j].unit, previous[j].index) == key {
            &previous[j]
        } else {
            s
        };
        out.push(ground_to_godot(before.position, before.height));
    }
    out
}

#[godot_api(secondary)]
impl SimHost {
    /// Unit id of every soldier of the latest snapshot. All per-soldier
    /// arrays use this order (by unit id, then soldier index).
    #[func]
    fn soldier_units(&self) -> PackedInt64Array {
        self.current
            .soldiers
            .iter()
            .map(|s| i64::from(s.unit.0))
            .collect()
    }

    /// Index of every soldier within its squad (matches the `Member_NN`
    /// nodes of the squad scene, `NN = index + 1`).
    #[func]
    fn soldier_indices(&self) -> PackedInt32Array {
        self.current
            .soldiers
            .iter()
            .map(|s| i32::from(s.index))
            .collect()
    }

    /// Soldier positions of the latest snapshot (feet on the ground).
    #[func]
    fn soldier_positions(&self) -> PackedVector3Array {
        self.current
            .soldiers
            .iter()
            .map(|s| ground_to_godot(s.position, s.height))
            .collect()
    }

    /// Soldier positions of the snapshot before, in the order of
    /// `soldier_units` (the current position where there is none).
    #[func]
    fn soldier_previous_positions(&self) -> PackedVector3Array {
        previous_soldier_positions(&self.previous.soldiers, &self.current.soldiers)
            .into_iter()
            .collect()
    }

    /// Hit points of every soldier (0 = dead).
    #[func]
    fn soldier_hp(&self) -> PackedInt32Array {
        self.current
            .soldiers
            .iter()
            .map(|s| i32::try_from(s.hp).unwrap_or(i32::MAX))
            .collect()
    }

    /// Whether every soldier uses cover (a cover spot or a building).
    #[func]
    fn soldier_in_cover(&self) -> PackedByteArray {
        self.current
            .soldiers
            .iter()
            .map(|s| u8::from(s.in_cover))
            .collect()
    }

    /// Stance of every unit (order of `unit_ids`): -1 vehicle, 0 standing,
    /// 1 kneeling, 2 prone.
    #[func]
    fn unit_stances(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| stance_code(u.stance))
            .collect()
    }

    /// Whether every unit is changing its stance.
    #[func]
    fn unit_changing_stance(&self) -> PackedByteArray {
        self.current
            .units
            .iter()
            .map(|u| u8::from(u.changing_stance))
            .collect()
    }

    /// Building (map object index) every unit is in, -1 if none.
    #[func]
    fn unit_garrisons(&self) -> PackedInt32Array {
        self.current
            .units
            .iter()
            .map(|u| u.garrison.map_or(-1, |g| i32::try_from(g).unwrap_or(-1)))
            .collect()
    }

    /// Sets the stance of the units: -1 automatic, 0 stand, 1 kneel,
    /// 2 prone.
    #[func]
    fn set_stance(&mut self, unit_ids: PackedInt64Array, stance: i64) {
        let units = to_unit_ids(&unit_ids);
        let stance = match stance {
            0 => StanceOrder::Fixed(Stance::Stand),
            1 => StanceOrder::Fixed(Stance::Kneel),
            2 => StanceOrder::Fixed(Stance::Prone),
            _ => StanceOrder::Auto,
        };
        if !units.is_empty() {
            self.pending.push(Command::SetStance { units, stance });
        }
    }

    /// Orders the units to occupy building `building` (map object index).
    #[func]
    fn garrison_units(&mut self, unit_ids: PackedInt64Array, building: i64) {
        let units = to_unit_ids(&unit_ids);
        let Ok(building) = u32::try_from(building) else {
            return;
        };
        if !units.is_empty() {
            self.pending.push(Command::Garrison { units, building });
        }
    }

    /// The building infantry can occupy at the ground point `(x, z)` in
    /// meters (its ground shape contains the point), or -1.
    #[func]
    fn building_at(&self, x: f32, z: f32) -> i64 {
        let Some(game) = &self.game else {
            return -1;
        };
        let (Some(x_mm), Some(z_mm)) = (meters_to_millimeters(x), meters_to_millimeters(z)) else {
            return -1;
        };
        game.sim
            .building_at(Vec2::from_millimeters(x_mm, z_mm))
            .map_or(-1, i64::from)
    }

    /// `capacity`, `occupants`, `owner` (-1 if empty) and `condition`
    /// ("intact", "damaged", "ruin") of building `object`; empty if it is
    /// none.
    #[func]
    fn building_info(&self, object: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Ok(object)) = (&self.game, u32::try_from(object)) else {
            return d;
        };
        let Some(capacity) = game.sim.building_capacity(object) else {
            return d;
        };
        let (owner, occupants) = game.sim.building_occupants(object);
        d.set("capacity", i64::from(capacity));
        d.set("occupants", i64::from(occupants));
        d.set("owner", owner.map_or(-1, |o| i64::from(o.0)));
        d.set(
            "condition",
            match game.sim.building_condition(object) {
                BuildingCondition::Intact => "intact",
                BuildingCondition::Damaged => "damaged",
                BuildingCondition::Ruin => "ruin",
            },
        );
        d
    }

    /// Starts the scenario file `scenario_path` (absolute) in single player:
    /// on its map, built in or from `maps_dir/<map>/`. Used by benchmarks.
    #[func]
    fn start_scenario(&mut self, seed: i64, scenario_path: GString, maps_dir: GString) -> bool {
        let path = scenario_path.to_string();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                godot_error!("cannot read scenario {path}: {e}");
                return false;
            }
        };
        let result = (|| -> Result<_, String> {
            let name = scenario_map_name(&text).map_err(|e| e.to_string())?;
            let map = if name == BUILTIN_MAP_NAME {
                builtin_map()
            } else {
                Arc::new(load_map_dir(&Path::new(&maps_dir.to_string()).join(&name))?)
            };
            let catalog = Catalog::builtin().map_err(|e| e.to_string())?;
            let scale = Scale::builtin().map_err(|e| e.to_string())?;
            let terrain = TerrainRules::builtin().map_err(|e| e.to_string())?;
            let config = parse_scenario(&text, &catalog, &scale, &terrain, &map)
                .map_err(|e| e.to_string())?;
            Ok((map, config))
        })();
        match result {
            Ok((map, config)) => self.start_config(seed, map, config),
            Err(e) => {
                godot_error!("cannot start scenario {path}: {e}");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim::{AidView, Fixed, UnitId, WoundView};

    fn soldier_at(unit: u32, index: u16, x: i32) -> SoldierSnapshot {
        SoldierSnapshot {
            unit: UnitId(unit),
            index,
            position: Vec2::from_meters(x, 0),
            height: Fixed::ZERO,
            hp: 100,
            in_cover: false,
            knocked: 0,
            knock_from: Vec2::ZERO,
            throw_from: Vec2::ZERO,
            throw_tick: 0,
            flight: 0,
            cover: None,
            wound: WoundView::Fit,
            aid: AidView::None,
        }
    }

    #[test]
    fn previous_soldier_positions_matches_by_unit_and_index() {
        let prev = [soldier_at(1, 0, 5), soldier_at(2, 1, 15)];
        let curr = [soldier_at(2, 1, 17), soldier_at(3, 0, 40)];
        let got = previous_soldier_positions(&prev, &curr);
        assert_eq!(
            got,
            vec![Vector3::new(15.0, 0.0, 0.0), Vector3::new(40.0, 0.0, 0.0)]
        );
    }
}
