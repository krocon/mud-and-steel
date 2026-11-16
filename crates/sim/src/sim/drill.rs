//! Orders and formations, stage 2 (prompt 046): formation moves with a
//! facing, attack-move and securing an area. Shapes: `crate::formation`.
//!
//! - [`Command::FormationMove`] lays the group out in a formation around
//!   the target, turned to the facing (default: the direction of travel),
//!   and sends it there on one group path like `MoveTo`. `Formation::Auto`
//!   is chosen here from the ground at the target ([`Sim::auto_formation`]).
//! - Attack-move ([`Engage`]): a unit with an `Advance` task halts while it
//!   has an automatic target (an enemy in range that it sees and can hurt),
//!   and drives on to its slot (own path) once the target is gone. Hold
//!   fire means it never halts.
//! - [`Command::Secure`]: the group takes up the formation around the
//!   center; every unit then attacks enemies its player sees inside the
//!   circle, drops the chase once the target leaves the circle, and returns
//!   to its slot.
//! - Facing: after arriving (and when back in its slot when securing) a
//!   unit turns its hull to the facing of the order, so tanks show their
//!   front armor; squads turn their formation.
//!
//! State hash: only units with a task or a facing to turn to are written,
//! after all other state, so matches without these orders keep their hash.

use super::combat::{Attack, rotate_towards};
use super::infantry::inside;
use super::{Route, Sim, formation_spacing, stop, within_world};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::formation::{Engage, Formation, assign, formation_points, normalized, right_of};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::path::SizeClass;
use crate::terrain::SpeedBasis;
use serde::{Deserialize, Serialize};

/// Radius of a secured area in meters: commands outside are clamped.
pub const SECURE_RADIUS_M: (u16, u16) = (10, 300);

/// Auto formation: narrower free ground across the facing than this many
/// slots at the target means a column ([provisional]).
const NARROW_SLOTS: i32 = 3;
/// Auto formation: from this share (percent) of cells around the target
/// that objects block for tanks (trees, hedges, walls), the ground counts
/// as woods ([provisional]).
const WOODS_PCT: usize = 30;
/// Auto formation: from this share (percent) of road along the way (and the
/// target on a road), the group moves as a column ([provisional]).
const ROAD_PCT: usize = 50;
/// Points sampled along the way for the road share.
const ROUTE_SAMPLES: i32 = 16;
/// Side length (points) of the square sampled around the target for woods.
const AREA_SAMPLES: i32 = 7;

/// What a unit does beyond its route (prompt 046).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Drill {
    /// Hull direction to turn to once the unit stands (unit length).
    facing: Option<Vec2>,
    task: Option<Task>,
}

impl Drill {
    pub(super) fn is_empty(&self) -> bool {
        self.facing.is_none() && self.task.is_none()
    }

    /// The unit attack-moves or secures an area.
    pub(super) const fn busy(&self) -> bool {
        self.task.is_some()
    }

    /// The unit attack-moves (prompt 047: squads bound).
    pub(super) const fn advancing(&self) -> bool {
        matches!(self.task, Some(Task::Advance { .. }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum Task {
    /// Attack-move to `slot`; `halted` while fighting on the way.
    Advance { slot: Vec2, halted: bool },
    /// Hold the circle around `center`, standing at `home` facing `facing`.
    Secure {
        center: Vec2,
        radius: Fixed,
        home: Vec2,
        facing: Vec2,
    },
}

/// Task of a unit for presentation (prompt 046).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillView {
    /// Attack-move to the slot; `true` while halted to fight.
    Advance(Vec2, bool),
    /// Securing the circle (center, radius in meters).
    Secure(Vec2, Fixed),
}

/// Whether `command` replaces the drill of its units.
const fn replaces_drill(command: &Command) -> bool {
    matches!(
        command,
        Command::MoveTo { .. }
            | Command::Stop { .. }
            | Command::Attack { .. }
            | Command::AttackGround { .. }
            | Command::Garrison { .. }
            | Command::Barrage { .. }
            | Command::Patrol { .. }
            | Command::Embark { .. }
            | Command::Tow { .. }
            | Command::FormationMove { .. }
            | Command::Secure { .. }
            | Command::Lay { .. }
            | Command::Dig { .. }
            | Command::Clear { .. }
            | Command::DemolishBridge { .. }
            | Command::RepairBridge { .. }
    )
}

impl Sim {
    /// Drops the tasks and facings of the units of `command` if it replaces
    /// their orders; a waypoint (`QueueMove`) ends securing only (an
    /// attack-move finishes first, then the queue goes on).
    pub(super) fn drop_drills(&mut self, command: &PlayerCommand) {
        let queue = matches!(command.command, Command::QueueMove { .. });
        if !queue && !replaces_drill(&command.command) {
            return;
        }
        for index in self.owned_indices(command.player, command.command.units()) {
            let drill = &mut self.units[index].drill;
            if queue {
                if matches!(drill.task, Some(Task::Secure { .. })) {
                    *drill = Drill::default();
                }
            } else {
                *drill = Drill::default();
            }
        }
    }

    /// `FormationMove`: the group in `formation` around `target`, facing
    /// `facing` (default: the direction of travel); `engage` picks the
    /// units that attack-move. Returns the slots with the facing.
    pub(super) fn order_formation_move(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
        formation: Formation,
        facing: Option<Vec2>,
        engage: Engage,
    ) -> Option<(Vec<(usize, Vec2)>, Vec2)> {
        if !within_world(target) {
            return None;
        }
        let group = self.mobile_indices(player, units);
        if group.is_empty() {
            return None;
        }
        let (assigned, forward) = self.formation_slots_for(&group, target, formation, facing);
        for &i in &group {
            self.leave_building(i);
            self.units[i].combat.attack = None;
            self.cancel_indirect(i);
        }
        self.request_group_path(&assigned, target);
        for &(i, slot) in &assigned {
            let fights = match engage {
                Engage::Never => false,
                Engage::Infantry => self.units[i].squad.is_some(),
                Engage::All => true,
            };
            let drill = &mut self.units[i].drill;
            drill.facing = Some(forward);
            drill.task = fights.then_some(Task::Advance {
                slot,
                halted: false,
            });
        }
        Some((assigned, forward))
    }

    /// `Secure`: the group takes up `formation` around `center` and holds
    /// the circle of `radius_m` meters.
    pub(super) fn order_secure(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        center: Vec2,
        radius_m: u16,
        formation: Formation,
        facing: Option<Vec2>,
    ) {
        let Some((assigned, forward)) =
            self.order_formation_move(player, units, center, formation, facing, Engage::Never)
        else {
            return;
        };
        let radius = Fixed::from_num(radius_m.clamp(SECURE_RADIUS_M.0, SECURE_RADIUS_M.1));
        for (i, home) in assigned {
            self.units[i].drill.task = Some(Task::Secure {
                center,
                radius,
                home,
                facing: forward,
            });
        }
    }

    /// Center of the group, facing of the order (given, else the direction
    /// of travel, else the first unit's hull) and slot spacing.
    fn group_frame(
        &self,
        group: &[usize],
        target: Vec2,
        facing: Option<Vec2>,
    ) -> (Vec2, Vec2, Fixed) {
        let count = Fixed::from_num(group.len());
        let center = group.iter().fold(Vec2::ZERO, |sum, &i| {
            let p = self.units[i].position;
            sum + Vec2::new(p.x / count, p.y / count)
        });
        let forward = facing
            .filter(|f| within_world(*f))
            .and_then(normalized)
            .or_else(|| normalized(target - center))
            .unwrap_or(self.units[group[0]].facing);
        let footprint = group
            .iter()
            .map(|&i| self.units[i].spread)
            .max()
            .unwrap_or(Fixed::ZERO);
        (center, forward, formation_spacing(footprint))
    }

    /// Formation slots around `target` for the unit indices `group` and the
    /// facing they use.
    fn formation_slots_for(
        &self,
        group: &[usize],
        target: Vec2,
        formation: Formation,
        facing: Option<Vec2>,
    ) -> (Vec<(usize, Vec2)>, Vec2) {
        let (center, forward, spacing) = self.group_frame(group, target, facing);
        let formation = if formation == Formation::Auto {
            self.auto_formation(group, center, target, forward, spacing)
        } else {
            formation
        };
        let slots = formation_points(formation, target, forward, group.len(), spacing);
        let units: Vec<(usize, Vec2)> =
            group.iter().map(|&i| (i, self.units[i].position)).collect();
        (assign(&units, &slots, center, forward), forward)
    }

    /// The formation `Auto` stands for, from the ground at `target` (and on
    /// the way from `from`), in this order:
    /// 1. free ground across the facing narrower than [`NARROW_SLOTS`]
    ///    slots (for the group's most restrictive size class): column;
    /// 2. at least [`WOODS_PCT`] of the area blocked for tanks by objects:
    ///    dispersed;
    /// 3. target on a road and at least [`ROAD_PCT`] of the way on roads:
    ///    column;
    /// 4. open ground: wedge if the group has vehicles, line for infantry.
    pub(super) fn auto_formation(
        &self,
        group: &[usize],
        from: Vec2,
        target: Vec2,
        forward: Vec2,
        spacing: Fixed,
    ) -> Formation {
        if group.len() <= 1 {
            return Formation::Line;
        }
        let grid = &self.paths.grid;
        let cell = grid.cell_size();
        let class = group
            .iter()
            .map(|&i| grid.class_of(self.units[i].unit_type as usize))
            .max()
            .unwrap_or(SizeClass::Small);
        // 1. Narrow ground: walk across the facing until blocked.
        let wanted = spacing * Fixed::from_num(NARROW_SLOTS) / 2 + cell;
        let right = right_of(forward);
        let free = |side: Fixed| {
            let mut d = Fixed::ZERO;
            while d < wanted {
                let p = target + right.scale(side * (d + cell));
                if !self.map.contains(p) || !grid.passable_at(class, p) {
                    break;
                }
                d += cell;
            }
            d
        };
        let width = if grid.passable_at(class, target) {
            free(Fixed::ONE) + free(-Fixed::ONE) + cell
        } else {
            Fixed::ZERO
        };
        if width < spacing * Fixed::from_num(NARROW_SLOTS) {
            return Formation::Column;
        }
        // 2. Woods: objects that block tanks around the target.
        let half = (spacing * 2).max(Fixed::from_num(20));
        let mut blocked = 0;
        let mut samples = 0;
        for sy in 0..AREA_SAMPLES {
            for sx in 0..AREA_SAMPLES {
                let at = |s: i32| {
                    half * Fixed::from_num(2 * s - (AREA_SAMPLES - 1))
                        / Fixed::from_num(AREA_SAMPLES - 1)
                };
                let p = target + Vec2::new(at(sx), at(sy));
                if !self.map.contains(p) {
                    continue;
                }
                samples += 1;
                let (x, y) = grid.cell_of(p);
                if grid.object_blocked(SizeClass::Large, x, y) {
                    blocked += 1;
                }
            }
        }
        if samples > 0 && blocked * 100 >= samples * WOODS_PCT {
            return Formation::Dispersed;
        }
        // 3. Road.
        let road = |p: Vec2| {
            self.map.contains(p)
                && self.terrain.types[usize::from(self.map.terrain_at(p))].speed == SpeedBasis::Road
        };
        if road(target) {
            let on_road = (1..=ROUTE_SAMPLES)
                .filter(|&k| {
                    let t = Fixed::from_num(k) / Fixed::from_num(ROUTE_SAMPLES);
                    road(from + (target - from).scale(t))
                })
                .count();
            if on_road * 100 >= usize::try_from(ROUTE_SAMPLES).unwrap_or(1) * ROAD_PCT {
                return Formation::Column;
            }
        }
        // 4. Open ground.
        if group.iter().any(|&i| self.units[i].squad.is_none()) {
            Formation::Wedge
        } else {
            Formation::Line
        }
    }

    /// Attack-move halts and resumes, securing units chase and return, and
    /// standing units turn to their facing. Runs right after the combat
    /// update (which picks the automatic targets) and before the movement.
    pub(super) fn update_drills(&mut self) {
        let retarget = self
            .tick
            .is_multiple_of(u64::from(self.combat_rules.fire.retarget_ticks));
        for i in 0..self.units.len() {
            if self.units[i].drill.is_empty() {
                continue;
            }
            if !self.units[i].alive() || self.units[i].link.is_some() {
                self.units[i].drill = Drill::default();
                continue;
            }
            match self.units[i].drill.task {
                Some(Task::Advance { slot, halted }) => self.advance(i, slot, halted),
                Some(Task::Secure {
                    center,
                    radius,
                    home,
                    facing,
                }) => self.secure(i, (center, radius), (home, facing), retarget),
                None => {}
            }
            self.turn_to_facing(i);
        }
    }

    fn advance(&mut self, i: usize, slot: Vec2, halted: bool) {
        // Prompt 047: squads of two fire teams bound on instead of halting;
        // mortar teams halt only for targets of their mortar.
        let bounds = self.bounds_on_advance(i);
        let mortar = self.mortar_halts(i);
        let unit = &mut self.units[i];
        let fighting = mortar.unwrap_or_else(|| {
            unit.combat.auto_target.is_some() && unit.combat.attack.is_none() && !bounds
        });
        let task = if fighting {
            if unit.route != Route::None || unit.target.is_some() {
                stop(unit);
            }
            Some(Task::Advance { slot, halted: true })
        } else if halted {
            let id = self.paths.request(
                unit.position,
                slot,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(slot);
            unit.route = Route::Waiting(id);
            unit.replans = 0;
            Some(Task::Advance {
                slot,
                halted: false,
            })
        } else if unit.route == Route::None && unit.target.is_none() {
            // Arrived (or gave up on the way).
            None
        } else {
            Some(Task::Advance { slot, halted })
        };
        unit.drill.task = task;
    }

    fn secure(&mut self, i: usize, area: (Vec2, Fixed), post: (Vec2, Vec2), retarget: bool) {
        let (center, radius) = area;
        let (home, facing) = post;
        // The chase ends where the circle ends.
        if let Some(attack) = self.units[i].combat.attack {
            let inside_circle = match attack {
                Attack::Unit(id) => self
                    .units
                    .binary_search_by_key(&id, |u| u.id)
                    .ok()
                    .is_some_and(|t| {
                        self.units[t].alive()
                            && (self.units[t].position - center).length() <= radius
                    }),
                Attack::Ground(_) => false,
            };
            if inside_circle {
                return;
            }
            let unit = &mut self.units[i];
            unit.combat.attack = None;
            unit.combat.attack_moving = false;
            stop(unit);
        }
        if !retarget {
            return;
        }
        if self.units[i].combat.auto_target.is_none()
            && let Some(t) = self.secure_target(i, center, radius)
        {
            let id = self.units[t].id;
            self.start_attack(i, Attack::Unit(id));
            return;
        }
        let unit = &mut self.units[i];
        let tolerance = unit.spread.max(Fixed::from_num(4));
        if (home - unit.position).length() > tolerance
            && unit.route == Route::None
            && unit.target.is_none()
        {
            let id = self.paths.request(
                unit.position,
                home,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(home);
            unit.route = Route::Waiting(id);
            unit.replans = 0;
            unit.drill.facing = Some(facing);
        }
    }

    /// The enemy nearest to unit `i` inside the circle that its player sees
    /// and that it can hurt.
    fn secure_target(&self, i: usize, center: Vec2, radius: Fixed) -> Option<usize> {
        let me = &self.units[i];
        let r = Vec2::new(radius, radius);
        let mut near = Vec::new();
        self.unit_grid.query(center - r, center + r, &mut near);
        let mut best: Option<((Fixed, UnitId), usize)> = None;
        for &j in &near {
            let j = j as usize;
            let other = &self.units[j];
            if !other.alive()
                || self.teams.allied(other.owner, me.owner)
                || other.link.is_some()
                || (other.position - center).length() > radius
                || !self.owner_sees(i, j)
                || !self.could_hurt(i, j)
            {
                continue;
            }
            let key = ((other.position - me.position).length(), other.id);
            if best.is_none_or(|(b, _)| key < b) {
                best = Some((key, j));
            }
        }
        best.map(|(_, j)| j)
    }

    /// Turns a standing unit without a fight towards its facing.
    fn turn_to_facing(&mut self, i: usize) {
        let unit = &self.units[i];
        let Some(want) = unit.drill.facing else {
            return;
        };
        let ready = unit.route == Route::None
            && unit.target.is_none()
            && unit.combat.attack.is_none()
            && unit.combat.auto_target.is_none()
            && !inside(unit)
            && match unit.drill.task {
                None => true,
                Some(Task::Advance { .. }) => false,
                Some(Task::Secure { home, .. }) => {
                    (home - unit.position).length() <= unit.spread.max(Fixed::from_num(4))
                }
            };
        if !ready {
            return;
        }
        let step = self.types[unit.unit_type as usize].hull_step;
        let unit = &mut self.units[i];
        let (facing, done) = rotate_towards(unit.facing, want, step);
        unit.facing = facing;
        if done {
            unit.drill.facing = None;
        }
    }

    /// The task of a unit for presentation; `None` without one.
    pub fn drill_view(&self, id: UnitId) -> Option<DrillView> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        match self.units[index].drill.task? {
            Task::Advance { slot, halted } => Some(DrillView::Advance(slot, halted)),
            Task::Secure { center, radius, .. } => Some(DrillView::Secure(center, radius)),
        }
    }

    /// The formation `Auto` would choose for the own mobile `units` moving
    /// to `target` (for the HUD and tests).
    pub fn auto_formation_for(
        &self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
        facing: Option<Vec2>,
    ) -> Option<Formation> {
        let group = self.mobile_indices(player, units);
        if group.is_empty() || !within_world(target) {
            return None;
        }
        let (center, forward, spacing) = self.group_frame(&group, target, facing);
        Some(self.auto_formation(&group, center, target, forward, spacing))
    }

    /// Hashes the drills of the units that have one.
    pub(super) fn hash_drills(&self, h: &mut StableHasher) {
        let write = |h: &mut StableHasher, p: Vec2| {
            h.write_i64(p.x.to_bits());
            h.write_i64(p.y.to_bits());
        };
        for unit in self.units.iter().filter(|u| !u.drill.is_empty()) {
            h.write_u32(unit.id.0);
            match unit.drill.facing {
                None => h.write_u8(0),
                Some(f) => {
                    h.write_u8(1);
                    write(h, f);
                }
            }
            match unit.drill.task {
                None => h.write_u8(0),
                Some(Task::Advance { slot, halted }) => {
                    h.write_u8(1);
                    write(h, slot);
                    h.write_u8(u8::from(halted));
                }
                Some(Task::Secure {
                    center,
                    radius,
                    home,
                    facing,
                }) => {
                    h.write_u8(2);
                    write(h, center);
                    h.write_i64(radius.to_bits());
                    write(h, home);
                    write(h, facing);
                }
            }
        }
    }
}
