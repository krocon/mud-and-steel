//! `SimHost` functions for base building (prompt 023): resources, build
//! options and placement checks for the preview, production queues, rally
//! points and supply points. Godot only draws what comes out of here; every
//! order goes through the lockstep session like any other command.

use crate::helpers::ground_point as point;
use crate::{SimHost, fixed_to_f32};
use godot::prelude::*;
use sim::{BuyOption, Command, PlayerId, Sim, UnitId};

/// The economy rules and catalog are compiled in; names come from there.
pub(crate) fn display_name(sim_type: &str) -> String {
    if let Ok(rules) = sim::EconomyRules::builtin_ref()
        && let Some(b) = rules.buildings.iter().find(|b| b.id == sim_type)
    {
        return crate::language_funcs::translated("NAME_", &b.id, &b.name);
    }
    sim::economy::builtin_catalog()
        .and_then(|c| c.get(sim_type))
        .map_or_else(
            || sim_type.to_owned(),
            |e| crate::language_funcs::entry_name(e).to_owned(),
        )
}

impl SimHost {
    fn local(&self) -> PlayerId {
        self.game
            .as_ref()
            .map_or(PlayerId(0), |g| g.session.local_player())
    }

    fn option_dict(sim: &Sim, types: &[sim::UnitType], o: &BuyOption) -> VarDictionary {
        let id = types
            .get(o.unit_type as usize)
            .map_or("", |t| t.name.as_str());
        let [fuel, ammo, manpower] = o.cost.resources;
        vdict! {
            "type" => i64::from(o.unit_type),
            "id" => id,
            "name" => display_name(id).as_str(),
            "fuel" => i64::from(fuel),
            "ammo" => i64::from(ammo),
            "manpower" => i64::from(manpower),
            "time" => f64::from(o.cost.time_ms) / 1000.0,
            "blocked" => o.blocked.map_or("", |e| e.name()),
            "structure" => sim.is_structure(o.unit_type),
        }
    }
}

#[godot_api(secondary)]
impl SimHost {
    /// Resources of the local player: `fuel`, `ammo`, `manpower`, `income`
    /// (per minute, `[fuel, ammo, manpower]`), `faction` and `enabled`
    /// (the match has base building for this player).
    #[func]
    fn economy_state(&self) -> VarDictionary {
        let Some(game) = &self.game else {
            return vdict! { "enabled" => false };
        };
        let me = self.local();
        let [fuel, ammo, manpower] = game.sim.account(me).unwrap_or_default();
        let income: PackedInt64Array = game.sim.income(me).iter().map(|&v| i64::from(v)).collect();
        let enabled = !game.sim.build_options(me).is_empty();
        vdict! {
            "enabled" => enabled,
            "fuel" => i64::from(fuel),
            "ammo" => i64::from(ammo),
            "manpower" => i64::from(manpower),
            "income" => &income,
            "faction" => game.sim.faction(me).unwrap_or(""),
        }
    }

    /// What the local player can build: dictionaries with `type`, `id`,
    /// `name`, `fuel`, `ammo`, `manpower`, `time` (s) and `blocked` (why it
    /// cannot be placed now, empty if it can).
    #[func]
    fn build_options(&self) -> VarArray {
        let Some(game) = &self.game else {
            return VarArray::new();
        };
        let types = &game.config.unit_types;
        game.sim
            .build_options(self.local())
            .iter()
            .map(|o| Self::option_dict(&game.sim, types, o).to_variant())
            .collect()
    }

    /// Why the local player cannot place `unit_type` at `(x, z)`; empty if
    /// it can (the same check the simulation runs for the order).
    #[func]
    fn placement_check(&self, unit_type: i64, x: f32, z: f32) -> GString {
        let (Some(game), Ok(t), Some(p)) = (&self.game, u32::try_from(unit_type), point(x, z))
        else {
            return GString::from("invalid");
        };
        match game.sim.placement(self.local(), t, p) {
            Ok(()) => GString::new(),
            Err(e) => GString::from(e.name()),
        }
    }

    /// Footprint of a structure type: `x` = width, `y` = length (along its
    /// facing), meters; zero for other types.
    #[func]
    fn structure_size(&self, unit_type: i64) -> Vector2 {
        let size = self
            .game
            .as_ref()
            .zip(u32::try_from(unit_type).ok())
            .and_then(|(g, t)| g.sim.structure_size(t));
        size.map_or(Vector2::ZERO, |(l, w)| {
            Vector2::new(fixed_to_f32(w), fixed_to_f32(l))
        })
    }

    /// Queues the construction of `unit_type` at `(x, z)` (meters).
    #[func]
    fn place_building(&mut self, unit_type: i64, x: f32, z: f32) {
        if let (Ok(unit_type), Some(position)) = (u32::try_from(unit_type), point(x, z)) {
            self.pending.push(Command::Build {
                unit_type,
                position,
            });
        }
    }

    /// Every structure: dictionaries with `id`, `owner`, `type`, `type_id`,
    /// `built` (0..1), `queue` (unit types), `progress` (0..1 of the first
    /// order) and `rally` (ground point).
    #[func]
    fn structures(&self) -> VarArray {
        let Some(game) = &self.game else {
            return VarArray::new();
        };
        let types = &game.config.unit_types;
        game.sim
            .structures()
            .iter()
            .map(|s| {
                let queue: PackedInt32Array = s
                    .queue
                    .iter()
                    .map(|&t| i32::try_from(t).unwrap_or(-1))
                    .collect();
                let (x, y) = (fixed_to_f32(s.rally.x), fixed_to_f32(s.rally.y));
                let h = fixed_to_f32(game.map().height_at(s.rally));
                let id = types
                    .get(s.unit_type as usize)
                    .map_or("", |t| t.name.as_str());
                vdict! {
                    "id" => i64::from(s.unit.0),
                    "owner" => i64::from(s.owner.0),
                    "type" => i64::from(s.unit_type),
                    "type_id" => id,
                    "name" => display_name(id).as_str(),
                    "built" => f64::from(s.built_pml) / 1000.0,
                    "queue" => &queue,
                    "progress" => f64::from(s.progress_pml) / 1000.0,
                    "rally" => Vector3::new(x, h, y),
                }
                .to_variant()
            })
            .collect()
    }

    /// What the building `unit_id` produces (same dictionaries as
    /// `build_options`; `blocked` is "not enough resources", "limit
    /// reached" for a full queue or "too many construction sites" while it
    /// is still being built).
    #[func]
    fn production_options(&self, unit_id: i64) -> VarArray {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(unit_id)) else {
            return VarArray::new();
        };
        let types = &game.config.unit_types;
        game.sim
            .production_options(UnitId(id))
            .iter()
            .map(|o| Self::option_dict(&game.sim, types, o).to_variant())
            .collect()
    }

    /// Display name of a unit type index.
    #[func]
    fn unit_type_name(&self, unit_type: i64) -> GString {
        let (Some(game), Ok(i)) = (&self.game, usize::try_from(unit_type)) else {
            return GString::new();
        };
        game.config
            .unit_types
            .get(i)
            .map_or_else(GString::new, |t| {
                GString::from(display_name(&t.name).as_str())
            })
    }

    /// Queues one unit of `unit_type` in the building `unit_id`.
    #[func]
    fn produce(&mut self, unit_id: i64, unit_type: i64) {
        if let (Ok(building), Ok(unit_type)) = (u32::try_from(unit_id), u32::try_from(unit_type)) {
            self.pending.push(Command::Produce {
                building: UnitId(building),
                unit_type,
            });
        }
    }

    /// Cancels the last order of the building `unit_id` (full refund).
    #[func]
    fn cancel_production(&mut self, unit_id: i64) {
        if let Ok(building) = u32::try_from(unit_id) {
            self.pending.push(Command::CancelProduction {
                building: UnitId(building),
            });
        }
    }

    /// Units made by the building `unit_id` drive to `(x, z)`.
    #[func]
    fn set_rally_point(&mut self, unit_id: i64, x: f32, z: f32) {
        if let (Ok(building), Some(point)) = (u32::try_from(unit_id), point(x, z)) {
            self.pending.push(Command::SetRallyPoint {
                building: UnitId(building),
                point,
            });
        }
    }

    /// Supply points: dictionaries with `name`, `position` (ground point),
    /// `owner` and `taker` (player id, -1 for none), `progress` (0..1),
    /// `contested` (hostile players both in the area), `radius` (m, from
    /// `[supply] radius_m`) and `income` (per minute, `[fuel, ammo,
    /// manpower]`). The true state; `supply_view.gd` applies the fog.
    #[func]
    fn supply_points(&self) -> VarArray {
        let Some(game) = &self.game else {
            return VarArray::new();
        };
        let radius = fixed_to_f32(game.sim.supply_radius());
        let income: PackedInt64Array = game
            .sim
            .supply_income()
            .iter()
            .map(|&v| i64::from(v))
            .collect();
        game.sim
            .supply_points()
            .iter()
            .map(|p| {
                let h = fixed_to_f32(game.map().height_at(p.position));
                vdict! {
                    "name" => p.name.as_str(),
                    "position" => Vector3::new(fixed_to_f32(p.position.x), h, fixed_to_f32(p.position.y)),
                    "owner" => p.owner.map_or(-1, |o| i64::from(o.0)),
                    "taker" => p.taker.map_or(-1, |o| i64::from(o.0)),
                    "progress" => f64::from(p.progress_pml) / 1000.0,
                    "contested" => p.contested,
                    "radius" => radius,
                    "income" => &income,
                }
                .to_variant()
            })
            .collect()
    }
}
