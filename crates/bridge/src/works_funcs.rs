//! `SimHost` functions for field works (prompt 035): the engineer orders
//! (`Lay`, `Dig`, `Clear`, `DemolishBridge`, `RepairBridge`), the mines the local
//! player knows, the field works it sees, vehicles without their track and
//! the state of the bridges. The simulation decides everything; Godot draws
//! (`scenes/world/works_view.gd`) and picks the orders
//! (`scenes/selection/engineer_orders.gd`).

use crate::{SimHost, ground_to_godot, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::{Command, DigKind, LayKind, MineKind, PlayerId, Vec2, WorkKind};

/// A bridge order picks the bridge within this distance (m) of the click.
const BRIDGE_PICK_M: i32 = 40;

fn ground_point(x: f32, z: f32) -> Option<Vec2> {
    Some(Vec2::from_millimeters(
        meters_to_millimeters(x)?,
        meters_to_millimeters(z)?,
    ))
}

#[godot_api(secondary)]
impl SimHost {
    /// Engineers among `unit_ids` lay or dig `kind` ("mines_at",
    /// "mines_ap", "wire", "hedgehogs", "foxhole", "trench") along the line
    /// from `(from_x, from_z)` to `(to_x, to_z)` (a point if equal).
    /// Returns whether an order went out.
    #[func]
    fn order_works(
        &mut self,
        unit_ids: PackedInt64Array,
        kind: GString,
        from: Vector2,
        to: Vector2,
    ) -> bool {
        let units = to_unit_ids(&unit_ids);
        let (Some(from), Some(to)) = (ground_point(from.x, from.y), ground_point(to.x, to.y))
        else {
            return false;
        };
        if units.is_empty() {
            return false;
        }
        let command = match WorkKind::from_name(&kind.to_string()) {
            Some(WorkKind::MinesAt) => lay(units, LayKind::MinesAt, from, to),
            Some(WorkKind::MinesAp) => lay(units, LayKind::MinesAp, from, to),
            Some(WorkKind::Wire) => lay(units, LayKind::Wire, from, to),
            Some(WorkKind::Hedgehogs) => lay(units, LayKind::Hedgehogs, from, to),
            Some(WorkKind::Foxhole) => dig(units, DigKind::Foxhole, from, to),
            Some(WorkKind::Trench) => dig(units, DigKind::Trench, from, to),
            // Prompt 052.
            Some(WorkKind::CommTrench) => dig(units, DigKind::CommTrench, from, to),
            Some(WorkKind::VehiclePit) => dig(units, DigKind::VehiclePit, from, to),
            Some(WorkKind::MgNest) => dig(units, DigKind::MgNest, from, to),
            Some(WorkKind::Dugout) => dig(units, DigKind::Dugout, from, to),
            Some(WorkKind::CamoNet) => lay(units, LayKind::CamoNet, from, to),
            Some(WorkKind::Bunker) => lay(units, LayKind::Bunker, from, to),
            Some(WorkKind::Teeth) => lay(units, LayKind::Teeth, from, to),
            None => return false,
        };
        self.pending.push(command);
        true
    }

    /// Engineers among `unit_ids` clear the known mines, wire and
    /// obstacles around `(x, z)`.
    #[func]
    fn clear_works(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(target)) = (units.is_empty(), ground_point(x, z)) {
            self.pending.push(Command::Clear { units, target });
        }
    }

    /// Engineers among `unit_ids` demolish the intact bridge near `(x, z)`
    /// or repair the destroyed one. Returns "demolish", "repair" or "" (no
    /// bridge near).
    #[func]
    fn order_bridge_work(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) -> GString {
        let units = to_unit_ids(&unit_ids);
        let (Some(game), Some(p)) = (&self.game, ground_point(x, z)) else {
            return GString::new();
        };
        let limit = sim::Fixed::from_num(BRIDGE_PICK_M);
        let nearest = (0u32..)
            .zip(game.sim.map().bridges())
            .map(|(i, b)| (i, b, (b.center - p).length()))
            .filter(|&(_, b, d)| d <= limit + b.half_length)
            .min_by_key(|&(i, _, d)| (d, i));
        let Some((bridge, b, _)) = nearest else {
            return GString::new();
        };
        if units.is_empty() {
            return GString::new();
        }
        let (command, name) = if b.intact {
            (Command::DemolishBridge { units, bridge }, "demolish")
        } else {
            (Command::RepairBridge { units, bridge }, "repair")
        };
        self.pending.push(command);
        GString::from(name)
    }

    /// Whether unit `id` does engineer work.
    #[func]
    fn is_engineer(&self, id: i64) -> bool {
        let Some(game) = &self.game else {
            return false;
        };
        self.current
            .units
            .iter()
            .find(|u| i64::from(u.id.0) == id)
            .is_some_and(|u| game.sim.is_engineer_type(u.unit_type))
    }

    /// What engineer `id` is doing: "build", "clear", "bridge" or "".
    #[func]
    fn engineer_job(&self, id: i64) -> GString {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return GString::new();
        };
        GString::from(game.sim.engineer_job(sim::UnitId(id)).unwrap_or(""))
    }

    /// Mines the local player's team knows (all with the debug reveal):
    /// per mine `{id, position (Vector3 on the ground), anti_tank (bool),
    /// friendly (bool)}`.
    #[func]
    fn known_mines(&self) -> Array<VarDictionary> {
        let Some(game) = &self.game else {
            return Array::new();
        };
        let local = game.session.local_player();
        let mines = if self.reveal_all {
            game.sim.all_mines()
        } else {
            game.sim.mines_for(local)
        };
        let map = game.sim.map();
        mines
            .iter()
            .map(|m| {
                let mut d = VarDictionary::new();
                d.set("id", i64::from(m.id));
                d.set(
                    "position",
                    ground_to_godot(m.position, map.height_at(m.position)),
                );
                d.set("anti_tank", m.kind == MineKind::AntiTank);
                d.set(
                    "friendly",
                    m.owner.is_some_and(|o| game.sim.allied(o, local)),
                );
                d
            })
            .collect()
    }

    /// Field works the local player sees: per work `{id, kind (name),
    /// owner, from, to (Vector3 on the ground), built, progress (0..1),
    /// active (engineers worked on it in the last tick), own (of the
    /// player's team)}`.
    #[func]
    fn field_works(&self) -> Array<VarDictionary> {
        let Some(game) = &self.game else {
            return Array::new();
        };
        let local = game.session.local_player();
        let map = game.sim.map();
        game.sim
            .works_for(local)
            .iter()
            .map(|w| {
                let mut d = VarDictionary::new();
                d.set("id", i64::from(w.id));
                d.set("kind", w.kind.name());
                d.set("owner", i64::from(w.owner.0));
                d.set("from", ground_to_godot(w.from, map.height_at(w.from)));
                d.set("to", ground_to_godot(w.to, map.height_at(w.to)));
                d.set("built", w.built);
                d.set("progress", w.progress_pml as f32 / 1000.0);
                d.set("active", w.active);
                d.set("own", game.sim.allied(w.owner, local));
                d
            })
            .collect()
    }

    /// A number that changes whenever `field_works` or `damaged_works`
    /// would change (prompt 124: `works_view.gd` skips its rebuild while it
    /// stays, maps bring thousands of works).
    #[func]
    fn field_works_signature(&self) -> i64 {
        let Some(game) = &self.game else {
            return 0;
        };
        let local = game.session.local_player();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        for w in game.sim.works_for(local) {
            mix(u64::from(w.id));
            mix(w.kind as u64);
            mix(u64::from(w.built) | u64::from(w.active) << 1);
            mix(u64::from(w.progress_pml));
            mix(w.from.x.to_bits() as u64 ^ (w.to.y.to_bits() as u64).rotate_left(17));
        }
        for id in game.sim.damaged_works() {
            mix(u64::from(id) | 1 << 40);
        }
        h as i64
    }

    /// Ids of the vehicles that lost their track to a mine.
    #[func]
    fn tracks_off(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim
                .tracks_off()
                .iter()
                .map(|&(id, _)| i64::from(id.0))
                .collect()
        })
    }

    /// Whether every bridge of the map is intact (map order).
    #[func]
    fn bridges_intact(&self) -> PackedByteArray {
        self.game.as_ref().map_or_else(PackedByteArray::new, |g| {
            g.sim
                .map()
                .bridges()
                .iter()
                .map(|b| u8::from(b.intact))
                .collect()
        })
    }

    /// Map object index of every bridge (map order).
    #[func]
    fn bridge_objects(&self) -> PackedInt64Array {
        self.game.as_ref().map_or_else(PackedInt64Array::new, |g| {
            g.sim
                .map()
                .bridges()
                .iter()
                .map(|b| i64::from(b.object))
                .collect()
        })
    }

    /// Price of one piece of `kind` (fuel, ammo, manpower), its build
    /// time in seconds and its length along a line in mm (0: one piece per
    /// point; prompt 052).
    #[func]
    fn work_cost(&self, kind: GString) -> PackedInt32Array {
        let (Some(game), Some(kind)) = (&self.game, WorkKind::from_name(&kind.to_string())) else {
            return PackedInt32Array::new();
        };
        let rule = game.sim.works_rules().rule(kind);
        rule.cost
            .iter()
            .chain([&rule.build_s, &rule.piece_mm])
            .map(|&v| i32::try_from(v).unwrap_or(i32::MAX))
            .collect()
    }

    /// Debug and self-test: every mine of the match, as `known_mines`
    /// (also the hidden ones).
    #[func]
    fn all_mines_count(&self) -> i64 {
        self.game
            .as_ref()
            .map_or(0, |g| i64::try_from(g.sim.all_mines().len()).unwrap_or(0))
    }

    /// Debug and self-test: places a hidden section of neutral mines at
    /// the ground point `(x, z)` (anti-tank or anti-personnel). Only in
    /// matches without other human players (it does not go over the
    /// network; the replay of such a match no longer verifies). Returns
    /// the number of mines placed.
    #[func]
    fn debug_minefield(&mut self, x: f32, z: f32, anti_tank: bool) -> i64 {
        let Some(game) = self.game.as_mut() else {
            return 0;
        };
        if game.networked {
            godot_warn!("debug minefields are not possible in network matches");
            return 0;
        }
        let Some(p) = ground_point(x, z) else {
            return 0;
        };
        let kind = if anti_tank {
            MineKind::AntiTank
        } else {
            MineKind::AntiPersonnel
        };
        i64::try_from(game.sim.place_minefield(p, kind)).unwrap_or(0)
    }

    /// Mines player `player` knows (self-test: none of the enemy's).
    #[func]
    fn mines_known_by(&self, player: i64) -> i64 {
        let (Some(game), Ok(p)) = (&self.game, u8::try_from(player)) else {
            return 0;
        };
        i64::try_from(game.sim.mines_for(PlayerId(p)).len()).unwrap_or(0)
    }
}

fn lay(units: Vec<sim::UnitId>, kind: LayKind, from: Vec2, to: Vec2) -> Command {
    Command::Lay {
        units,
        kind,
        from,
        to,
    }
}

fn dig(units: Vec<sim::UnitId>, kind: DigKind, from: Vec2, to: Vec2) -> Command {
    Command::Dig {
        units,
        kind,
        from,
        to,
    }
}
