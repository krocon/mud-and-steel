//! `SimHost` functions for the HUD (prompt 031): waypoint chains and
//! patrols, the unit panel, and the minimap (terrain image and unit dots).
//! Presentation only, apart from the two orders.

use crate::{SimHost, fixed_to_f32, ground_to_godot, meters_to_millimeters, to_unit_ids};
use godot::prelude::*;
use sim::vision::Sight;
use sim::{Command, Fixed, RouteView, UnitId, Vec2};

/// Floats per minimap dot in the MultiMesh buffer: a 2D transform (8) and
/// a color (4).
const DOT_FLOATS: usize = 12;

fn target_of(x: f32, z: f32) -> Option<Vec2> {
    Some(Vec2::from_millimeters(
        meters_to_millimeters(x)?,
        meters_to_millimeters(z)?,
    ))
}

/// `0xRRGGBB` as linear-free sRGB bytes.
const fn rgb(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xff) as f32,
        ((c >> 8) & 0xff) as f32,
        (c & 0xff) as f32,
    ]
}

const FOREST: [f32; 3] = rgb(0x2c_44_22);
const SOLID: [f32; 3] = rgb(0x6a_5e_52);
const ROAD: [f32; 3] = rgb(0x9a_92_80);

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

#[godot_api(secondary)]
impl SimHost {
    /// Appends the ground point `(x, z)` as a waypoint for the units
    /// (Shift + right click). Idle units start at once.
    #[func]
    fn queue_move(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(target)) = (units.is_empty(), target_of(x, z)) {
            self.pending.push(Command::QueueMove { units, target });
        }
    }

    /// The units drive to `(x, z)` and then back and forth between their
    /// start and there.
    #[func]
    fn patrol_units(&mut self, unit_ids: PackedInt64Array, x: f32, z: f32) {
        let units = to_unit_ids(&unit_ids);
        if let (false, Some(target)) = (units.is_empty(), target_of(x, z)) {
            self.pending.push(Command::Patrol { units, target });
        }
    }

    /// Where unit `id` is going, on the ground: the remaining waypoints of
    /// its route (or its target while the path is planned), then its queued
    /// waypoints. Empty for idle or unknown units.
    #[func]
    fn unit_waypoints(&self, id: i64) -> PackedVector3Array {
        let mut out = PackedVector3Array::new();
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return out;
        };
        let id = UnitId(id);
        let map = game.map();
        let mut push = |p: Vec2| out.push(ground_to_godot(p, map.height_at(p)));
        match game.sim.route_of(id) {
            Some(RouteView::Following(points)) => points.iter().for_each(|&p| push(p)),
            Some(RouteView::Waiting) => {
                if let Some(u) = self.current.units.iter().find(|u| u.id == id)
                    && let Some(t) = u.target
                {
                    push(t);
                }
            }
            _ => {}
        }
        if let Some((queued, _)) = game.sim.queued_waypoints(id) {
            queued.iter().for_each(|&p| push(p));
        }
        out
    }

    /// Number of queued waypoints of unit `id` (the route's own waypoints
    /// not counted).
    #[func]
    fn unit_queue_length(&self, id: i64) -> i64 {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return 0;
        };
        game.sim
            .queued_waypoints(UnitId(id))
            .map_or(0, |(q, _)| q.len() as i64)
    }

    /// Whether unit `id` patrols.
    #[func]
    fn unit_patrols(&self, id: i64) -> bool {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return false;
        };
        game.sim
            .queued_waypoints(UnitId(id))
            .is_some_and(|(_, patrol)| patrol)
    }

    /// What the unit panel shows of unit `id` (empty if it is not in the
    /// snapshot): `name`, `type_id`, `category`, `hp`, `max_hp`,
    /// `soldiers` / `squad_size` (squads), `armor_hull` and `armor_turret`
    /// (front, side, rear in mm; the turret empty without turret armor),
    /// `weapons` (array of `name`, `count`, `caliber_mm`, `range_m`).
    #[func]
    fn unit_info(&self, id: i64) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return d;
        };
        let Some(unit) = self.current.units.iter().find(|u| u.id.0 == id) else {
            return d;
        };
        let Some(ty) = game.config.unit_types.get(unit.unit_type as usize) else {
            return d;
        };
        d.set("name", crate::economy_funcs::display_name(&ty.name));
        d.set("type_id", ty.name.as_str());
        d.set(
            "category",
            ty.combat.category.map_or("", sim::catalog::Category::code),
        );
        d.set("hp", i64::from(unit.hp));
        d.set("max_hp", i64::from(unit.max_hp));
        // Gun crews (prompt 177).
        if let Some((men, full)) = game.sim.crew_of(unit.id) {
            d.set("crew", i64::from(men));
            d.set("crew_full", i64::from(full));
        }
        if ty.combat.squad_size > 0 {
            // Riders have no soldiers in the snapshot (prompt 109).
            let riding = unit.carrier.and_then(|_| game.sim.squad_alive(unit.id));
            let alive = match riding {
                Some(n) => n as usize,
                None => self
                    .current
                    .soldiers
                    .iter()
                    .filter(|s| s.unit == unit.id && s.hp > 0)
                    .count(),
            };
            d.set("soldiers", alive as i64);
            d.set("squad_size", i64::from(ty.combat.squad_size));
        }
        let faces = |f: [u16; 3]| {
            f.iter()
                .map(|&mm| i64::from(mm))
                .collect::<PackedInt64Array>()
        };
        let armor = ty.combat.armor;
        d.set("armor_hull", &faces(armor.hull));
        d.set(
            "armor_turret",
            &armor.turret.map_or_else(PackedInt64Array::new, faces),
        );
        let mut weapons = Array::<VarDictionary>::new();
        for w in &ty.combat.weapons {
            let mut wd = VarDictionary::new();
            wd.set("name", w.name.as_str());
            wd.set("count", i64::from(w.count));
            wd.set("caliber_mm", w.caliber_mm_x10 as f32 / 10.0);
            wd.set("range_m", fixed_to_f32(w.range_m));
            weapons.push(&wd);
        }
        d.set("weapons", &weapons);
        d
    }

    /// The ground at `(x, z)` for the ground tooltip (prompt 100), as the
    /// local player may know it (`Sim::ground_info`); empty outside the
    /// map. Keys: `terrain` (id), `terrain_name`, `road` (road speed
    /// basis), `speed_pct`, `gears` (dictionary gear name → percent on soft
    /// ground now), `passable_small`, `passable_large`, `grade_pct`,
    /// `slope_pct` (-1: too steep), `sight` ("clear", "foliage", "solid"),
    /// `smoke`, `object` (catalog id or ""), `ruin`, `hard_cover`,
    /// `object_cover_pct`, `building`, `work` (work name or ""),
    /// `ground_cover_pct`.
    #[func]
    fn ground_info(&self, x: f32, z: f32) -> VarDictionary {
        let mut d = VarDictionary::new();
        let (Some(game), Some(x), Some(z)) = (
            &self.game,
            crate::meters_to_millimeters(x),
            crate::meters_to_millimeters(z),
        ) else {
            return d;
        };
        let player = game.session.local_player();
        let Some(info) = game
            .sim
            .ground_info(player, sim::Vec2::from_millimeters(x, z))
        else {
            return d;
        };
        d.set("terrain", i64::from(info.terrain));
        d.set("terrain_name", info.terrain_name.as_str());
        d.set("road", info.road);
        d.set("speed_pct", i64::from(info.speed_pct));
        let mut gears = VarDictionary::new();
        for (name, pct) in &info.gears {
            gears.set(name.as_str(), i64::from(*pct));
        }
        d.set("gears", &gears);
        d.set("passable_small", info.passable_small);
        d.set("passable_large", info.passable_large);
        d.set("grade_pct", i64::from(info.grade_pct));
        d.set("slope_pct", info.slope_pct.map_or(-1, i64::from));
        d.set(
            "sight",
            match info.sight {
                sim::vision::Sight::Clear => "clear",
                sim::vision::Sight::Foliage => "foliage",
                sim::vision::Sight::Solid => "solid",
            },
        );
        d.set("smoke", info.smoke);
        d.set("object", info.object.as_deref().unwrap_or(""));
        d.set("ruin", info.ruin);
        d.set("hard_cover", info.hard_cover);
        d.set("object_cover_pct", i64::from(info.object_cover_pct));
        d.set("building", info.building);
        d.set("work", info.work.map_or("", sim::works::WorkKind::name));
        d.set("ground_cover_pct", i64::from(info.ground_cover_pct));
        d
    }

    /// Running gear of unit `id` (`foot`, `wheeled`, `halftrack`,
    /// `tracked`; "" without one), prompt 100.
    #[func]
    fn unit_gear(&self, id: i64) -> GString {
        let (Some(game), Ok(id)) = (&self.game, u32::try_from(id)) else {
            return GString::new();
        };
        game.sim
            .unit_gear(sim::UnitId(id))
            .map_or_else(GString::new, |g| GString::from(g.as_str()))
    }

    /// The minimap's ground: one RGB8 pixel per vision cell
    /// (`vision_grid_size`), row by row from the north-west corner, like
    /// `vision_image`. Terrain color with hill shading, roads (any sample
    /// of the cell), forest and solid obstacles from the vision grid.
    #[func]
    fn minimap_image(&self) -> PackedByteArray {
        let Some(game) = &self.game else {
            return PackedByteArray::new();
        };
        let map = game.map();
        let grid = game.sim.vision_grid();
        let types = &game.config.terrain.types;
        let colors: Vec<[f32; 3]> = types.iter().map(|t| rgb(t.debug_color)).collect();
        let road: Vec<bool> = types
            .iter()
            .map(|t| t.speed == sim::terrain::SpeedBasis::Road)
            .collect();
        let (columns, rows) = (grid.columns, grid.rows);
        let cell_m = fixed_to_f32(grid.cell);
        // Samples checked per axis within a cell for roads (about 2 m apart).
        let sub = (cell_m / (map.cell_size_mm() as f32 / 1000.0))
            .round()
            .max(1.0) as i64;
        let height = |c: i64, r: i64| {
            let c = c.clamp(0, i64::from(columns) - 1) as u32;
            let r = r.clamp(0, i64::from(rows) - 1) as u32;
            grid.ground_mm(r * columns + c) as f32 / 1000.0
        };
        let mut out = Vec::with_capacity((columns * rows * 3) as usize);
        for r in 0..rows {
            for c in 0..columns {
                let index = (r * columns + c) as usize;
                let center = grid.center(c, r);
                let id = usize::from(map.terrain_at(center));
                let mut color = colors.get(id).copied().unwrap_or([128.0; 3]);
                let corner = center - Vec2::new(grid.cell / 2, grid.cell / 2);
                let has_road = (0..sub).any(|a| {
                    (0..sub).any(|b| {
                        let step = grid.cell / sub;
                        let p = corner
                            + Vec2::new(
                                step * Fixed::from_num(a) + step / 2,
                                step * Fixed::from_num(b) + step / 2,
                            );
                        map.contains(p) && road.get(usize::from(map.terrain_at(p))) == Some(&true)
                    })
                });
                if has_road {
                    color = ROAD;
                }
                match grid.blocker_at(index) {
                    Sight::Foliage => color = mix(color, FOREST, 0.75),
                    Sight::Solid => color = mix(color, SOLID, 0.8),
                    Sight::Clear => {}
                }
                // Light from the north-west, slopes over two cells.
                let (ci, ri) = (i64::from(c), i64::from(r));
                let dx = height(ci + 1, ri) - height(ci - 1, ri);
                let dz = height(ci, ri + 1) - height(ci, ri - 1);
                let shade = (1.0 - (dx + dz) / (2.0 * cell_m) * 2.5).clamp(0.55, 1.35);
                out.extend(color.map(|v| (v * shade).clamp(0.0, 255.0) as u8));
            }
        }
        PackedByteArray::from(out)
    }

    /// The MultiMesh buffer of the minimap's unit dots (2D transforms with
    /// colors, 12 floats per dot) for the living units of the snapshot:
    /// map point `(x, z)` at `(x - origin.x, z - origin.y) * scale` pixels,
    /// dots of `dot_px` pixels in `colors[owner]` (the last color for
    /// owners beyond the list).
    #[func]
    fn minimap_dots(
        &self,
        origin: Vector2,
        scale: f32,
        dot_px: f32,
        colors: PackedColorArray,
    ) -> PackedFloat32Array {
        let mut out = Vec::with_capacity(self.current.units.len() * DOT_FLOATS);
        let colors = colors.as_slice();
        for u in self.current.units.iter().filter(|u| u.alive()) {
            let x = (fixed_to_f32(u.position.x) - origin.x) * scale;
            let y = (fixed_to_f32(u.position.y) - origin.y) * scale;
            let color = colors
                .get(usize::from(u.owner.0))
                .or(colors.last())
                .copied()
                .unwrap_or(Color::WHITE);
            out.extend_from_slice(&[
                dot_px, 0.0, 0.0, x, 0.0, dot_px, 0.0, y, color.r, color.g, color.b, color.a,
            ]);
        }
        PackedFloat32Array::from(out)
    }
}
