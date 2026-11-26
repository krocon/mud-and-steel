//! `SimHost` functions for rivers, bridge-laying tanks, camouflage,
//! paratroopers and dragged air calls (prompt 055). The simulation decides
//! everything; Godot draws (`scenes/world/crossing_view.gd`) and picks the
//! orders (`scenes/selection/crossing_orders.gd`, `hud/air_bar.gd`).

use crate::helpers::{
    collect_tick_events, flat_to_godot as flat, ground_point, vec3_to_godot as to_godot,
};
use crate::{SimHost, fixed_to_f32, ground_to_godot, to_unit_ids};
use godot::prelude::*;
use sim::{AirMission, Command, CrossingEvent, LayerState, Sim, UnitId};

/// Events kept for Godot at most (if it stops taking them).
const MAX_PENDING_EVENTS: usize = 4096;

/// Appends the crossing events of the latest tick.
pub(crate) fn collect_events(pending: &mut Vec<(u64, CrossingEvent)>, sim: &Sim) {
    collect_tick_events(
        pending,
        sim.tick(),
        sim.crossing_events(),
        MAX_PENDING_EVENTS,
    );
}

#[godot_api(secondary)]
impl SimHost {
    /// Bridge-laying tanks among `unit_ids` lay their bridge across the gap
    /// nearest to `(x, z)`. Returns whether one of them can span a gap
    /// there (the order goes out either way).
    #[func]
    fn lay_bridge(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) -> bool {
        let units = to_unit_ids(&unit_ids);
        let Some(target) = ground_point(x, z) else {
            return false;
        };
        if units.is_empty() {
            return false;
        }
        let found = self.game.as_ref().is_some_and(|g| {
            units.iter().any(|&id| {
                g.sim
                    .type_of_unit(id)
                    .is_some_and(|t| g.sim.find_gap(t, target).is_some())
            })
        });
        self.pending.push(Command::LayBridge { units, target });
        found
    }

    /// Bridge-laying tanks among `unit_ids` take their bridge up again.
    #[func]
    fn pick_up_bridge(&mut self, unit_ids: PackedInt64Array) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::PickUpBridge { units });
        }
    }

    /// The deck a bridge-laying tank `id` would lay near `(x, z)`:
    /// `{start: Vector3, end: Vector3, width_m}` (ground heights), or empty.
    #[func]
    fn bridge_gap(&self, id: i64, x: f32, z: f32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Some(p), Ok(id)) = (&self.game, ground_point(x, z), u32::try_from(id))
        else {
            return d;
        };
        let sim = &game.sim;
        let Some(gap) = sim
            .type_of_unit(UnitId(id))
            .and_then(|t| sim.find_gap(t, p))
        else {
            return d;
        };
        d.set(
            "start",
            ground_to_godot(gap.start, sim.map().ground_height_at(gap.start)),
        );
        d.set(
            "end",
            ground_to_godot(gap.end, sim.map().ground_height_at(gap.end)),
        );
        d.set("width_m", i64::from(gap.width_m));
        d
    }

    /// Whether unit `id` is a bridge-laying tank.
    #[func]
    fn is_bridgelayer(&self, id: i64) -> bool {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return false;
        };
        game.sim
            .type_of_unit(UnitId(id))
            .is_some_and(|t| game.sim.is_bridgelayer_type(t))
    }

    /// Every bridge-laying tank of the match: `unit`, `owner`, `state`
    /// ("carrying", "driving", "laying", "laid", "fetching", "picking_up",
    /// "lost"), `progress` (0..1 while laying or picking up), `bridge`
    /// (index into the bridges, -1 if none), `span_m`, `load_t`, and the
    /// gap it drives to (`gap_start`, `gap_end`, if any).
    #[func]
    fn bridge_layers(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let sim = &game.sim;
        for l in sim.layers() {
            let mut d = VarDictionary::new();
            d.set("unit", i64::from(l.unit.0));
            d.set("owner", i64::from(l.owner.0));
            let (state, progress, bridge) = match l.state {
                LayerState::Carrying => ("carrying", 0, -1),
                LayerState::Driving => ("driving", 0, -1),
                LayerState::Laying(p) => ("laying", p, -1),
                LayerState::Laid(b) => ("laid", 1000, i64::from(b)),
                LayerState::Fetching(b) => ("fetching", 1000, i64::from(b)),
                LayerState::PickingUp(b, p) => ("picking_up", 1000 - p, i64::from(b)),
                LayerState::Lost => ("lost", 0, -1),
            };
            d.set("state", state);
            d.set("progress", progress as f32 / 1000.0);
            d.set("bridge", bridge);
            d.set("span_m", l.span_mm as f32 / 1000.0);
            d.set("load_t", l.load_kg as f32 / 1000.0);
            if let Some(gap) = sim.layer_gap(l.unit) {
                d.set(
                    "gap_start",
                    ground_to_godot(gap.start, sim.map().ground_height_at(gap.start)),
                );
                d.set(
                    "gap_end",
                    ground_to_godot(gap.end, sim.map().ground_height_at(gap.end)),
                );
            }
            out.push(&d);
        }
        out
    }

    /// The deck of every bridge (map order, laid bridges last): `bridge`
    /// (index), `unit` (the tank of a laid bridge, -1 for the map's),
    /// `intact`, `start`, `end` (deck ends with deck heights), `width_m`
    /// (roadway).
    #[func]
    fn bridge_decks(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        for (i, b) in game.sim.map().bridges().iter().enumerate() {
            let (start, end) = b.ends();
            let mut d = VarDictionary::new();
            d.set("bridge", i64::try_from(i).unwrap_or(-1));
            d.set("unit", b.laid_by.map_or(-1, |l| i64::from(l.0)));
            d.set("intact", b.intact);
            d.set("start", ground_to_godot(start, b.height_start));
            d.set("end", ground_to_godot(end, b.height_end));
            d.set("width_m", fixed_to_f32(b.half_width * 2));
            out.push(&d);
        }
        out
    }

    /// Vehicles and guns among `unit_ids` put up (`on`) or take off their
    /// camouflage.
    #[func]
    fn set_camouflage(&mut self, unit_ids: PackedInt64Array, on: bool) {
        let units = to_unit_ids(&unit_ids);
        if !units.is_empty() {
            self.pending.push(Command::SetCamouflage { units, on });
        }
    }

    /// Whether unit `id` may put up camouflage.
    #[func]
    fn can_camouflage(&self, id: i64) -> bool {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return false;
        };
        game.sim
            .type_of_unit(UnitId(id))
            .is_some_and(|t| game.sim.can_camouflage_type(t))
    }

    /// Camouflage of unit `id`: 0 none, 1 being put up, 2 up, 3 up and
    /// hiding the unit (it stood still long enough).
    #[func]
    fn camouflage_state(&self, id: i64) -> i64 {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return 0;
        };
        let id = UnitId(id);
        match game.sim.camouflage_of(id) {
            None => 0,
            Some(false) => 1,
            Some(true) if game.sim.camouflage_hides(id) => 3,
            Some(true) => 2,
        }
    }

    /// Ids of the units with branches or nets of their own (being put up
    /// or up; prompt 055, not the placed nets of prompt 052).
    #[func]
    fn vehicle_camouflage_units(&self) -> PackedInt64Array {
        let Some(game) = &self.game else {
            return PackedInt64Array::new();
        };
        self.current
            .units
            .iter()
            .filter(|u| game.sim.camouflage_of(u.id).is_some())
            .map(|u| i64::from(u.id.0))
            .collect()
    }

    /// Ids of the squads in deep water (swimming to the bank).
    #[func]
    fn swimming_units(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim
                .swimmers()
                .iter()
                .map(|s| i64::from(s.unit.0))
                .collect()
        })
    }

    /// Paratroopers in the air the local player sees: `id`, `owner`,
    /// `unit_type`, `position` (center of the stick), `landing`,
    /// `soldiers`, `seconds_left`.
    #[func]
    fn parachutes(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        let Some(game) = &self.game else {
            return out;
        };
        let local = (!self.reveal_all).then(|| game.session.local_player());
        let hz = self.tick_hz.max(1) as f32;
        for p in game.sim.parachutes(local) {
            let mut d = VarDictionary::new();
            d.set("id", i64::from(p.id));
            d.set("owner", i64::from(p.owner.0));
            d.set("unit_type", i64::from(p.unit_type));
            d.set("position", to_godot(p.position));
            d.set("landing", flat(p.landing));
            d.set("soldiers", i64::from(p.soldiers));
            d.set("seconds_left", p.ticks_left as f32 / hz);
            out.push(&d);
        }
        out
    }

    /// Calls the air mission `mission` along the dragged line from
    /// `(from_x, from_z)` to `(to_x, to_z)`: bombers lay a carpet, transports
    /// drop along it (prompt 055).
    #[func]
    fn call_air_line(&mut self, mission: GString, from: Vector2, to: Vector2) {
        let (Some(from), Some(to)) = (ground_point(from.x, from.y), ground_point(to.x, to.y))
        else {
            return;
        };
        let name = mission.to_string();
        let Some(mission) = AirMission::ALL.into_iter().find(|m| m.name() == name) else {
            return;
        };
        self.pending
            .push(Command::CallAirLine { mission, from, to });
    }

    /// Longest bomb carpet (m) a dragged bomb run lays.
    #[func]
    fn air_carpet_max_m(&self) -> f32 {
        sim::AirRules::builtin().map_or(0.0, |r| r.bombs.carpet_max_m as f32)
    }

    /// Crossing events since the last call, one dictionary each: `tick`,
    /// `type` ("collapsed", "fell", "drowned", "ashore", "bridge",
    /// "landed"), `at` (Vector3) and per type `bridge`, `unit`, `laid`,
    /// `water`.
    #[func]
    fn take_crossing_events(&mut self) -> Array<VarDictionary> {
        let events = std::mem::take(&mut self.crossing_events);
        let mut out = Array::new();
        for (tick, event) in events {
            let mut d = VarDictionary::new();
            d.set("tick", i64::try_from(tick).unwrap_or(i64::MAX));
            match event {
                CrossingEvent::Collapsed { bridge, at } => {
                    d.set("type", "collapsed");
                    d.set("bridge", i64::from(bridge));
                    d.set("at", to_godot(at));
                }
                CrossingEvent::Fell { unit, at } => {
                    d.set("type", "fell");
                    d.set("unit", i64::from(unit.0));
                    d.set("at", to_godot(at));
                }
                CrossingEvent::Drowned { unit, soldier, at } => {
                    d.set("type", "drowned");
                    d.set("unit", i64::from(unit.0));
                    d.set("soldier", i64::from(soldier));
                    d.set("at", to_godot(at));
                }
                CrossingEvent::Ashore { unit, at } => {
                    d.set("type", "ashore");
                    d.set("unit", i64::from(unit.0));
                    d.set("at", flat(at));
                }
                CrossingEvent::Bridge { unit, bridge, laid } => {
                    d.set("type", "bridge");
                    d.set("unit", i64::from(unit.0));
                    d.set("bridge", i64::from(bridge));
                    d.set("laid", laid);
                }
                CrossingEvent::Landed { unit, at, water } => {
                    d.set("type", "landed");
                    d.set("unit", i64::from(unit.0));
                    d.set("at", flat(at));
                    d.set("water", water);
                }
            }
            out.push(&d);
        }
        out
    }
}
