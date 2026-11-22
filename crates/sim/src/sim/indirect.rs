//! Indirect fire and smoke (prompt 032): orders of mortars, howitzers and
//! self-propelled guns, shells in a ballistic arc, fragmentation, craters
//! and destruction at the point of impact, and smoke clouds that drift with
//! the wind and block sight in the vision grid. Rules and public types:
//! `crate::indirect`.
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::fire_indirect`] after
//! the direct fire (orders, moving into range, setup, new shells),
//! [`Sim::fly_shells`] after the projectiles (impacts), and
//! [`Sim::update_smoke`] before the vision update (drift, expiry, smoke
//! cells; changed cells queue the viewsheds that contain them).
//!
//! Indirect orders live here, not in the unit's direct-fire attack order:
//! a mortar squad keeps firing its small arms at will while its mortar
//! follows the order.

use super::{Route, Sim, stop};
use crate::combat::{CombatEvent, Face, FireMode, HitKind, Vec3};
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::indirect::{
    INDIRECT_WEAPON, IndirectRules, IndirectWeapon, ShellSnapshot, SmokeSnapshot, flight,
};
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// An indirect-fire order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Order {
    /// Fire at a unit the player sees, until it is destroyed or lost.
    Unit(UnitId),
    /// Fire at a ground point until stopped.
    Ground(Vec2),
    /// Fire `salvos` more rounds at random points within `radius`.
    Barrage {
        target: Vec2,
        radius: Fixed,
        salvos: u32,
        smoke: bool,
    },
}

/// Indirect-fire state of one unit.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Gun {
    order: Option<Order>,
    /// Ticks until the next round.
    reload: u32,
    /// Ticks of setting up left (after moving).
    setup: u32,
    /// The unit drives into range of its order's target.
    closing: bool,
    /// Rockets of the current salvo still to launch (prompt 062; 0: no
    /// salvo under way).
    #[serde(default)]
    ripple: u32,
}

/// The indirect weapon of a unit type with the rules applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GunType {
    weapon: IndirectWeapon,
    range: Fixed,
    min_range: Fixed,
    /// Ticks after a round, or after the last rocket of a salvo.
    reload_ticks: u32,
    setup_ticks: u32,
    elevation_deg: u32,
    /// Rounds per salvo (prompt 062: the tubes of a rocket launcher, else
    /// 1) and ticks between two rounds of a salvo.
    #[serde(default)]
    salvo: u32,
    #[serde(default)]
    ripple_ticks: u32,
    /// Dispersion in tenths of a milliradian (0: the global value).
    #[serde(default)]
    mrad_x10: u32,
}

/// What every unit type looks like to a shell.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Body {
    infantry: bool,
    half_length: Fixed,
    half_width: Fixed,
    thinnest_mm: u32,
}

/// A shell in flight: a parabola from `origin` to `target` over `total`
/// ticks with its apex `apex` above the straight line.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Shell {
    id: u32,
    shooter: UnitId,
    owner: PlayerId,
    origin: Vec3,
    target: Vec3,
    apex: Fixed,
    total: u32,
    elapsed: u32,
    caliber_mm_x10: u32,
    smoke: bool,
}

impl Shell {
    /// Position after `k` of `total` ticks.
    fn at(&self, k: u32) -> Vec3 {
        let s = Fixed::from_num(k) / Fixed::from_num(self.total.max(1));
        let lerp = |a: Fixed, b: Fixed| a + (b - a) * s;
        let arc = self.apex * 4 * s * (Fixed::ONE - s);
        Vec3::new(
            lerp(self.origin.x, self.target.x),
            lerp(self.origin.y, self.target.y),
            lerp(self.origin.z, self.target.z) + arc,
        )
    }

    fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.id);
        h.write_u32(self.shooter.0);
        h.write_u8(self.owner.0);
        for v in [
            self.origin.x,
            self.origin.y,
            self.origin.z,
            self.target.x,
            self.target.y,
            self.target.z,
            self.apex,
        ] {
            h.write_i64(v.to_bits());
        }
        h.write_u32(self.total);
        h.write_u32(self.elapsed);
        h.write_u32(self.caliber_mm_x10);
        h.write_u8(u8::from(self.smoke));
    }
}

/// A smoke cloud.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Cloud {
    id: u32,
    center: Vec2,
    radius: Fixed,
    born: u64,
    expires: u64,
    /// Dark smoke of a fire (prompt 041), not of a smoke shell.
    #[serde(default)]
    dark: bool,
}

/// Indirect fire and smoke of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Artillery {
    rules: IndirectRules,
    /// Per unit type.
    types: Vec<Option<GunType>>,
    bodies: Vec<Body>,
    /// Units that have an indirect weapon, by id (created when first
    /// needed).
    guns: BTreeMap<UnitId, Gun>,
    shells: Vec<Shell>,
    clouds: Vec<Cloud>,
    next_cloud: u32,
    /// Vision cells under smoke, ascending.
    smoked: Vec<u32>,
    /// Drift of the clouds per tick (meters).
    wind: Vec2,
    smoke_ticks: u64,
    smoke_height_mm: i32,
}

impl Artillery {
    pub(super) fn new(
        rules: IndirectRules,
        unit_types: &[crate::data::UnitType],
        tick_hz: u32,
    ) -> Self {
        let hz = u64::from(tick_hz.max(1));
        let ticks = |ms: u32, (num, den): (u32, u32)| {
            let (num, den) = if num == 0 || den == 0 {
                (1, 1)
            } else {
                (u64::from(num), u64::from(den))
            };
            // ms × den / num real → game ms, then ticks (rounded).
            u32::try_from((u64::from(ms) * den * hz + 500 * num) / (1000 * num)).unwrap_or(u32::MAX)
        };
        let types = unit_types
            .iter()
            .map(|t| {
                let w = t.combat.indirect.clone()?;
                let c = rules.category(t.combat.category);
                let interval = (w.interval_s * Fixed::from_num(hz)).round();
                let interval = interval.to_num::<u32>().max(1);
                // Prompt 062: a salvo weapon launches its tubes `ripple`
                // apart; the catalog rate is the sustained one, so the
                // reload fills the rest of tubes × interval.
                let (salvo, ripple, reload) = if c.ripple_ms > 0 && w.tubes > 1 {
                    let ripple = ticks(c.ripple_ms, w.time_ratio).max(1);
                    let cycle = interval.saturating_mul(w.tubes);
                    let reload = cycle.saturating_sub(ripple * (w.tubes - 1)).max(ripple);
                    (w.tubes, ripple, reload)
                } else {
                    (1, 0, interval)
                };
                Some(GunType {
                    range: w.range_m,
                    min_range: w.range_m * Fixed::from_num(c.min_range_pct) / 100,
                    reload_ticks: reload,
                    setup_ticks: ticks(c.setup_ms, w.time_ratio),
                    elevation_deg: c.elevation_deg,
                    salvo,
                    ripple_ticks: ripple,
                    mrad_x10: c.mrad_x10,
                    weapon: w,
                })
            })
            .collect();
        let bodies = unit_types
            .iter()
            .map(|t| {
                let c = &t.combat;
                let dim = |v: Fixed| {
                    if v > Fixed::ZERO { v } else { t.footprint_m }
                };
                Body {
                    infantry: c.is_infantry(),
                    half_length: dim(c.length_m) / 2,
                    half_width: dim(c.width_m) / 2,
                    thinnest_mm: u32::from(c.armor.thinnest()),
                }
            })
            .collect();
        let time = unit_types
            .first()
            .map_or((2, 1), |t| t.combat.time_ratio)
            .max((1, 1));
        let s = &rules.smoke;
        // Real mm/s × time factor → game m per tick.
        let drift = |mm: i32| {
            Fixed::from_num(i64::from(mm) * i64::from(time.0))
                / Fixed::from_num(i64::from(time.1.max(1)) * 1000 * i64::try_from(hz).unwrap_or(20))
        };
        let wind = Vec2::new(drift(s.wind_x_mm_per_s), drift(s.wind_y_mm_per_s));
        let smoke_ticks = u64::from(ticks(s.lifetime_ms, time)).max(1);
        let smoke_height_mm = i32::try_from(s.height_mm).unwrap_or(i32::MAX);
        Self {
            rules,
            types,
            bodies,
            guns: BTreeMap::new(),
            shells: Vec::new(),
            clouds: Vec::new(),
            next_cloud: 0,
            smoked: Vec::new(),
            wind,
            smoke_ticks,
            smoke_height_mm,
        }
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
        h.write_u64(self.guns.len() as u64);
        for (id, g) in &self.guns {
            h.write_u32(id.0);
            match g.order {
                None => h.write_u8(0),
                Some(Order::Unit(t)) => {
                    h.write_u8(1);
                    h.write_u32(t.0);
                }
                Some(Order::Ground(p)) => {
                    h.write_u8(2);
                    h.write_i64(p.x.to_bits());
                    h.write_i64(p.y.to_bits());
                }
                Some(Order::Barrage {
                    target,
                    radius,
                    salvos,
                    smoke,
                }) => {
                    h.write_u8(3);
                    h.write_i64(target.x.to_bits());
                    h.write_i64(target.y.to_bits());
                    h.write_i64(radius.to_bits());
                    h.write_u32(salvos);
                    h.write_u8(u8::from(smoke));
                }
            }
            h.write_u32(g.reload);
            h.write_u32(g.setup);
            h.write_u8(u8::from(g.closing));
            h.write_u32(g.ripple);
        }
        h.write_u64(self.shells.len() as u64);
        for s in &self.shells {
            s.hash_into(h);
        }
        h.write_u32(self.next_cloud);
        h.write_u64(self.clouds.len() as u64);
        for c in &self.clouds {
            h.write_u32(c.id);
            h.write_i64(c.center.x.to_bits());
            h.write_i64(c.center.y.to_bits());
            h.write_i64(c.radius.to_bits());
            h.write_u64(c.born);
            h.write_u64(c.expires);
            // Shell smoke hashes as before fire smoke existed.
            if c.dark {
                h.write_u8(1);
            }
        }
        h.write_u64(self.smoked.len() as u64);
        for &c in &self.smoked {
            h.write_u32(c);
        }
    }

    /// Shells in flight for the snapshot.
    pub(super) fn shell_snapshots(&self, tick_hz: u32, out: &mut Vec<ShellSnapshot>) {
        out.clear();
        let hz = Fixed::from_num(tick_hz);
        out.extend(self.shells.iter().map(|s| {
            let a = s.at(s.elapsed);
            let b = s.at(s.elapsed + 1);
            ShellSnapshot {
                id: s.id,
                shooter: s.shooter,
                owner: s.owner,
                position: a,
                velocity: Vec3::new((b.x - a.x) * hz, (b.y - a.y) * hz, (b.z - a.z) * hz),
                target: s.target,
                ticks_left: s.total.saturating_sub(s.elapsed),
                caliber_mm_x10: s.caliber_mm_x10,
                smoke: s.smoke,
            }
        }));
    }

    /// Smoke clouds for the snapshot.
    pub(super) fn smoke_snapshots(&self, tick: u64, out: &mut Vec<SmokeSnapshot>) {
        out.clear();
        out.extend(self.clouds.iter().map(|c| SmokeSnapshot {
            id: c.id,
            center: c.center,
            radius: c.radius,
            age_ticks: u32::try_from(tick.saturating_sub(c.born)).unwrap_or(u32::MAX),
            ticks_left: u32::try_from(c.expires.saturating_sub(tick)).unwrap_or(u32::MAX),
            dark: c.dark,
        }));
    }

    /// Drift of the smoke clouds per tick (prompt 041: fire smoke starts
    /// downwind of the fire).
    pub(super) const fn wind(&self) -> Vec2 {
        self.wind
    }

    /// Scales the wind by the weather (prompt 040), in percent.
    pub(super) fn scale_wind(&mut self, pct: u32) {
        if pct != 100 {
            self.wind = self.wind.scale(Fixed::from_num(pct) / 100);
        }
    }

    /// Fragmentation radius of a shell of `caliber_mm_x10` (prompt 041:
    /// exploding dumps).
    pub(super) fn blast_radius(&self, caliber_mm_x10: u32) -> Fixed {
        self.rules.blast_radius(caliber_mm_x10)
    }

    /// Puts the dark smoke cloud of a fire (prompt 041) at `center`, living
    /// `ticks` from `tick`; the oldest cloud goes if there are too many.
    /// Returns the cloud's id.
    pub(super) fn add_fire_cloud(
        &mut self,
        tick: u64,
        center: Vec2,
        radius: Fixed,
        ticks: u64,
    ) -> u32 {
        if self.clouds.len() >= self.rules.smoke.max_clouds as usize {
            self.clouds.remove(0);
        }
        let id = self.next_cloud;
        self.next_cloud = self.next_cloud.wrapping_add(1);
        self.clouds.push(Cloud {
            id,
            center,
            radius,
            born: tick,
            expires: tick + ticks.max(1),
            dark: true,
        });
        id
    }
}

/// Triangular random value in `-1..=1` (as for direct fire).
fn triangular(rng: &mut crate::rng::Rng) -> Fixed {
    let a = rng.range_i32(-1000, 1000);
    let b = rng.range_i32(-1000, 1000);
    Fixed::from_num(a + b) / 2000
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

impl Sim {
    /// Units of `player` among `ids` that have an indirect weapon.
    fn gun_indices(&self, player: PlayerId, ids: &[UnitId]) -> Vec<usize> {
        let mut out = self.owned_indices(player, ids);
        out.retain(|&i| self.artillery.types[self.units[i].unit_type as usize].is_some());
        out
    }

    /// Gives the units of `player` with an indirect weapon the order;
    /// returns their indices (the other units take a direct-fire order).
    pub(super) fn order_indirect(
        &mut self,
        player: PlayerId,
        ids: &[UnitId],
        order: Order,
    ) -> Vec<usize> {
        let guns = self.gun_indices(player, ids);
        for &i in &guns {
            let id = self.units[i].id;
            let closing = self.artillery.guns.get(&id).is_some_and(|g| g.closing);
            if closing {
                stop(&mut self.units[i]);
            }
            let gun = self.artillery.guns.entry(id).or_default();
            gun.order = Some(order);
            gun.closing = false;
        }
        guns
    }

    /// Whether unit `i` has an indirect weapon (prompt 175).
    pub(super) fn artillery_type_of(&self, i: usize) -> bool {
        self.artillery.types[self.units[i].unit_type as usize].is_some()
    }

    /// Whether unit `id` has an indirect fire order (prompt 175,
    /// [`Sim::activity`]).
    pub(super) fn has_fire_mission(&self, id: UnitId) -> bool {
        self.artillery
            .guns
            .get(&id)
            .is_some_and(|g| g.order.is_some())
    }

    /// A barrage order (`Command::Barrage`).
    pub(super) fn order_barrage(
        &mut self,
        player: PlayerId,
        ids: &[UnitId],
        target: Vec2,
        (radius_m, salvos, smoke): (u16, u16, bool),
    ) {
        if !super::within_world(target) || salvos == 0 {
            self.reject(player, super::RejectReason::OutsideMap, None);
            return;
        }
        if self.gun_indices(player, ids).is_empty() {
            self.reject(player, super::RejectReason::NoArtillery, None);
            return;
        }
        let rules = &self.artillery.rules.barrage;
        let radius = Fixed::from_num(u32::from(radius_m).min(rules.max_radius_m));
        let salvos = u32::from(salvos).min(rules.max_salvos);
        let mut ids = ids.to_vec();
        if smoke {
            // Only units with smoke ammunition fire smoke.
            ids.retain(|id| {
                self.units
                    .binary_search_by_key(id, |u| u.id)
                    .ok()
                    .and_then(|i| self.artillery.types[self.units[i].unit_type as usize].as_ref())
                    .is_some_and(|t| t.weapon.smoke)
            });
        }
        self.order_indirect(
            player,
            &ids,
            Order::Barrage {
                target,
                radius,
                salvos,
                smoke,
            },
        );
    }

    /// Drops the indirect order of unit `i` (stop, move orders).
    pub(super) fn cancel_indirect(&mut self, i: usize) {
        if let Some(gun) = self.artillery.guns.get_mut(&self.units[i].id) {
            gun.order = None;
            gun.closing = false;
        }
    }

    /// Whether unit `i` has an indirect fire order (waypoint chains wait
    /// for it, prompt 031).
    pub(super) fn has_indirect_order(&self, i: usize) -> bool {
        self.artillery
            .guns
            .get(&self.units[i].id)
            .is_some_and(|g| g.order.is_some())
    }

    /// Ends the order of unit `i`; a unit driving into range stops.
    fn end_order(&mut self, i: usize) {
        let id = self.units[i].id;
        if let Some(gun) = self.artillery.guns.get_mut(&id) {
            gun.order = None;
            if gun.closing {
                gun.closing = false;
                stop(&mut self.units[i]);
            }
        }
    }

    /// Fragmentation radius in meters of a shell of `caliber_mm_x10`
    /// (`[blast]` in `data/indirect.toml`); the impact markers (prompt 084)
    /// size themselves by it.
    pub fn shell_blast_radius(&self, caliber_mm_x10: u32) -> Fixed {
        self.artillery.blast_radius(caliber_mm_x10)
    }

    /// Minimum and maximum range (game m) and smoke ammunition of a unit's
    /// indirect weapon (`None`: it has none).
    pub fn indirect_weapon(&self, id: UnitId) -> Option<(Fixed, Fixed, bool)> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        let t = self.artillery.types[self.units[i].unit_type as usize].as_ref()?;
        Some((t.min_range, t.range, t.weapon.smoke))
    }

    /// Whether the vision cell of `p` is under smoke.
    pub fn smoke_at(&self, p: Vec2) -> bool {
        let grid = &self.vision.grid;
        grid.smoke_at(grid.index_of(p))
    }

    /// Orders, moving into range, setup and firing of every unit with an
    /// indirect weapon.
    pub(super) fn fire_indirect(&mut self) {
        for i in 0..self.units.len() {
            let ty = self.units[i].unit_type as usize;
            let Some(gt) = self.artillery.types[ty].as_ref() else {
                continue;
            };
            let (range, min_range, setup_ticks, full_reload) =
                (gt.range, gt.min_range, gt.setup_ticks, gt.reload_ticks);
            let id = self.units[i].id;
            if !self.units[i].alive() {
                self.artillery.guns.remove(&id);
                continue;
            }
            // Towed, limbering or unlimbering counts as moving (prompt 034).
            let blocked = self.transport_blocks_fire(i);
            let moving = self.units[i].route != Route::None || blocked;
            let gun = self.artillery.guns.entry(id).or_default();
            gun.reload = gun.reload.saturating_sub(1);
            if moving {
                gun.setup = setup_ticks;
                // A salvo broken off by moving: the rest is lost, the
                // launcher reloads as a whole (prompt 062).
                if gun.ripple > 0 {
                    gun.ripple = 0;
                    gun.reload = gun.reload.max(full_reload);
                }
            } else {
                gun.setup = gun.setup.saturating_sub(1);
            }
            let order = gun.order;
            // Squads inside a building cannot fire their mortar; a heavy
            // mortar or howitzer without a fit crew neither (prompt 177).
            if super::infantry::inside(&self.units[i]) || blocked || self.crew_silenced(i) {
                continue;
            }
            let point = match order {
                Some(Order::Unit(t)) => {
                    let target = self
                        .units
                        .binary_search_by_key(&t, |u| u.id)
                        .ok()
                        .filter(|&j| self.units[j].alive() && self.owner_sees(i, j));
                    match target {
                        Some(j) => Some(self.units[j].position),
                        None => {
                            // Destroyed or out of the player's sight.
                            self.end_order(i);
                            continue;
                        }
                    }
                }
                Some(Order::Ground(p) | Order::Barrage { target: p, .. }) => Some(p),
                None => None,
            };
            let point = match point {
                Some(p) => {
                    if !self.close_in(i, p, range) {
                        continue;
                    }
                    p
                }
                None => {
                    let gun = &self.artillery.guns[&id];
                    if gun.reload > 0
                        || gun.setup > 0
                        || moving
                        || self.units[i].combat.fire_mode != FireMode::FireAtWill
                    {
                        continue;
                    }
                    let Some(j) = self.indirect_target(i, min_range, range) else {
                        continue;
                    };
                    self.units[j].position
                }
            };
            let gun = &self.artillery.guns[&id];
            if gun.reload > 0 || gun.setup > 0 || self.units[i].route != Route::None {
                continue;
            }
            let distance = (point - self.units[i].position).length();
            if distance < min_range || distance > range {
                continue;
            }
            // A salvo costs one salvo of the stock, paid with its first
            // round (prompt 062; a single round is a salvo of one).
            let starting = gun.ripple == 0;
            if starting && !self.has_ammo(i) {
                continue;
            }
            self.fire_shell(i, point, order);
            if starting {
                self.spend_salvo(i, true);
            }
        }
    }

    /// Drives unit `i` into range of `point`; returns whether it is in
    /// range (and stands).
    fn close_in(&mut self, i: usize, point: Vec2, range: Fixed) -> bool {
        let id = self.units[i].id;
        let distance = (point - self.units[i].position).length();
        let closing = self.artillery.guns[&id].closing;
        let wanted = range * Fixed::from_num(self.combat_rules.fire.attack_range_pct) / 100;
        if distance <= range && (!closing || distance <= wanted) {
            if closing {
                stop(&mut self.units[i]);
                if let Some(g) = self.artillery.guns.get_mut(&id) {
                    g.closing = false;
                }
            }
            return true;
        }
        let unit = &mut self.units[i];
        if !closing && unit.speed_offroad > Fixed::ZERO {
            let request = self.paths.request(
                (&self.map, &self.terrain),
                unit.position,
                point,
                unit.unit_type as usize,
                vec![(unit.id, Vec2::ZERO)],
            );
            unit.target = Some(point);
            unit.route = Route::Waiting(request);
            unit.replans = 0;
            if let Some(g) = self.artillery.guns.get_mut(&id) {
                g.closing = true;
            }
        }
        false
    }

    /// The automatic target of unit `i`'s indirect weapon, if it has one
    /// (prompt 047: mortar teams halt for it on attack-move).
    pub(super) fn indirect_auto_target(&self, i: usize) -> Option<usize> {
        let t = self.artillery.types[self.units[i].unit_type as usize].as_ref()?;
        self.indirect_target(i, t.min_range, t.range)
    }

    /// Automatic target of an indirect weapon: an enemy the unit itself
    /// sees in range that high explosive hurts (infantry and open
    /// vehicles), the nearest first, then the lowest id. Mortar teams also
    /// take enemies their player has spotted (prompt 047,
    /// [`Sim::indirect_spots`]).
    fn indirect_target(&self, i: usize, min_range: Fixed, range: Fixed) -> Option<usize> {
        let me = &self.units[i];
        let open = self.artillery.rules.blast.open_armor_mm;
        let r = Vec2::new(range, range);
        let mut near = Vec::new();
        self.unit_grid
            .query(me.position - r, me.position + r, &mut near);
        let mut best: Option<((i64, UnitId), usize)> = None;
        let caliber = self.artillery.types[me.unit_type as usize]
            .as_ref()
            .map_or(0, |t| t.weapon.caliber_mm_x10);
        let danger = self.artillery.rules.blast_radius(caliber) * 2;
        for &j in &near {
            let j = j as usize;
            let other = &self.units[j];
            if self.teams.allied(other.owner, me.owner) || !other.alive() {
                continue;
            }
            let body = &self.artillery.bodies[other.unit_type as usize];
            if !body.infantry && body.thinnest_mm > open {
                continue;
            }
            let d = (other.position - me.position).length();
            if d < min_range || d > range || !self.indirect_spots(i, j) {
                continue;
            }
            // Prompt 188: not too far and no single men (0 = no limit).
            let t = &self.combat_rules.targeting;
            if t.indirect_auto_range_pct > 0
                && d * 100 > range * Fixed::from_num(t.indirect_auto_range_pct)
            {
                continue;
            }
            if t.indirect_min_soldiers > 0
                && other
                    .squad
                    .as_ref()
                    .is_some_and(|s| s.alive_count() < t.indirect_min_soldiers)
            {
                continue;
            }
            // No automatic fire near own troops (fragments hurt them too).
            if self.friend_near(me.owner, other.position, danger) {
                continue;
            }
            let key = (d.to_bits(), other.id);
            if best.is_none_or(|(b, _)| key < b) {
                best = Some((key, j));
            }
        }
        best.map(|(_, j)| j)
    }

    /// Whether a living unit of `owner` or an ally stands within `radius`
    /// of `p`.
    fn friend_near(&self, owner: PlayerId, p: Vec2, radius: Fixed) -> bool {
        let r = Vec2::new(radius, radius);
        let mut near = Vec::new();
        self.unit_grid.query(p - r, p + r, &mut near);
        near.iter().any(|&k| {
            let u = &self.units[k as usize];
            self.teams.allied(u.owner, owner) && u.alive() && (u.position - p).length() <= radius
        })
    }

    /// Fires one shell of unit `i` at `point` (a barrage picks a point
    /// within its radius), with dispersion.
    fn fire_shell(&mut self, i: usize, point: Vec2, order: Option<Order>) {
        let unit = &self.units[i];
        let (id, owner) = (unit.id, unit.owner);
        let gt = self.artillery.types[unit.unit_type as usize]
            .as_ref()
            .expect("checked");
        let (reload, elevation, caliber) =
            (gt.reload_ticks, gt.elevation_deg, gt.weapon.caliber_mm_x10);
        let (salvo, ripple_ticks, mrad_x10) = (gt.salvo, gt.ripple_ticks, gt.mrad_x10);
        let real_per_game = gt.weapon.real_per_game();
        let weapon = gt.weapon.clone();
        let mut aim = point;
        let mut smoke = false;
        let mut blind = false;
        match order {
            Some(Order::Barrage {
                radius, smoke: s, ..
            }) => {
                smoke = s;
                // Uniform in the circle: rejection sampling on the square.
                if radius > Fixed::ZERO {
                    for _ in 0..8 {
                        let dx = Fixed::from_num(self.rng.range_i32(-1000, 1000)) / 1000;
                        let dy = Fixed::from_num(self.rng.range_i32(-1000, 1000)) / 1000;
                        if dx * dx + dy * dy <= Fixed::ONE {
                            aim = point + Vec2::new(dx * radius, dy * radius);
                            break;
                        }
                    }
                }
                blind = !self.player_sees_point(owner, aim);
            }
            Some(Order::Ground(_)) => blind = !self.player_sees_point(owner, aim),
            _ => {}
        }
        let unit = &self.units[i];
        let ty = &self.types[unit.unit_type as usize];
        let muzzle = Vec3::new(
            unit.position.x,
            unit.position.y,
            unit.height
                + ty.height * Fixed::from_num(self.combat_rules.fire.muzzle_height_pct) / 100,
        );
        let delta = aim - unit.position;
        let distance = delta.length();
        let d = &self.artillery.rules.dispersion;
        let mrad_x10 = if mrad_x10 > 0 { mrad_x10 } else { d.mrad_x10 };
        let mut sigma = (distance * real_per_game * Fixed::from_num(mrad_x10) / 10_000)
            .max(Fixed::from_num(d.min_mm) / 1000);
        if blind {
            sigma = sigma * Fixed::from_num(d.blind_pct) / 100;
        }
        let ex = triangular(&mut self.rng) * sigma;
        let ey = triangular(&mut self.rng) * sigma;
        let mut impact = aim + Vec2::new(ex, ey);
        if !self.map.contains(impact) {
            impact = aim;
        }
        let target = Vec3::new(impact.x, impact.y, self.map.height_at(impact));
        let travel = (impact - muzzle.ground()).length();
        let f = flight(
            &weapon,
            travel,
            elevation,
            self.artillery.rules.ballistics.gravity_mm_per_s2,
        );
        let total = (f.seconds * Fixed::from_num(self.tick_hz))
            .round()
            .to_num::<u32>()
            .max(1);
        let shell = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1);
        self.artillery.shells.push(Shell {
            id: shell,
            shooter: id,
            owner,
            origin: muzzle,
            target,
            apex: f.apex_m,
            total,
            elapsed: 0,
            caliber_mm_x10: caliber,
            smoke,
        });
        self.events.push(CombatEvent::Shot {
            unit: id,
            weapon: INDIRECT_WEAPON,
            projectile: shell,
            muzzle,
            caliber_mm_x10: caliber,
        });
        if distance > Fixed::ZERO {
            self.units[i].combat.aim = Vec2::new(delta.x / distance, delta.y / distance);
        }
        let gun = self.artillery.guns.entry(id).or_default();
        // Prompt 062: the first round of a salvo leaves `salvo - 1` to
        // follow `ripple_ticks` apart; after the last one the full reload.
        if salvo > 1 {
            gun.ripple = if gun.ripple == 0 {
                salvo - 1
            } else {
                gun.ripple - 1
            };
        }
        if gun.ripple > 0 {
            gun.reload = ripple_ticks;
            return;
        }
        gun.reload = reload;
        // Fewer men load more slowly (prompt 177).
        let pct = self.crew_reload_pct(i);
        let gun = self.artillery.guns.entry(id).or_default();
        gun.reload = gun.reload * pct / 100;
        if let Some(Order::Barrage { salvos, .. }) = &mut gun.order {
            *salvos = salvos.saturating_sub(1);
            if *salvos == 0 {
                gun.order = None;
            }
        }
    }

    /// Ticks until the next indirect round of unit `id` (0: none due).
    pub(super) fn indirect_reload(&self, id: UnitId) -> u32 {
        self.artillery.guns.get(&id).map_or(0, |g| g.reload)
    }

    /// Hull of unit type `t` (prompt 035, mines): half length and half
    /// width (m), thinnest armor (mm), and whether it is infantry.
    pub(super) fn hull(&self, t: usize) -> (Fixed, Fixed, u32, bool) {
        self.artillery
            .bodies
            .get(t)
            .map_or((Fixed::ZERO, Fixed::ZERO, 0, true), |b| {
                (b.half_length, b.half_width, b.thinnest_mm, b.infantry)
            })
    }

    /// Fragmentation rules (prompt 033: strafing and bombs use them too).
    pub(super) const fn indirect_blast(&self) -> &crate::indirect::BlastRules {
        &self.artillery.rules.blast
    }

    /// A high-explosive charge of `caliber_mm_x10` explodes at `at`, coming
    /// from `origin` (prompt 033: bombs and crashing aircraft; prompt 041:
    /// exploding dumps): the same direct hit, object, crater and fragment
    /// rules as a shell.
    pub(super) fn detonate(
        &mut self,
        id: u32,
        shooter: UnitId,
        owner: PlayerId,
        origin: Vec3,
        at: Vec3,
        caliber_mm_x10: u32,
    ) {
        let shell = Shell {
            id,
            shooter,
            owner,
            origin,
            target: at,
            apex: Fixed::ZERO,
            total: 1,
            elapsed: 1,
            caliber_mm_x10,
            smoke: false,
        };
        let mut destroyed: Vec<(usize, UnitId, Vec3)> = Vec::new();
        self.explode(&shell, at, &mut destroyed);
        for (j, by, at) in destroyed {
            if !self.units[j].alive() {
                self.destroy(j, by, at);
            }
        }
    }

    /// Puts a high-explosive round from `origin` to `target` in the air
    /// that arrives after `total` ticks over an arc of `apex` meters
    /// (prompt 054: the rockets of a heavy strike). It flies, is drawn and
    /// explodes like a shell; the shot event is the launch.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn launch_shell(
        &mut self,
        shooter: UnitId,
        owner: PlayerId,
        origin: Vec3,
        target: Vec3,
        apex: Fixed,
        total: u32,
        caliber_mm_x10: u32,
    ) {
        let id = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1);
        self.artillery.shells.push(Shell {
            id,
            shooter,
            owner,
            origin,
            target,
            apex,
            total: total.max(1),
            elapsed: 0,
            caliber_mm_x10,
            smoke: false,
        });
        self.events.push(CombatEvent::Shot {
            unit: shooter,
            weapon: INDIRECT_WEAPON,
            projectile: id,
            muzzle: origin,
            caliber_mm_x10,
        });
    }

    /// Moves every shell one tick; shells that arrive explode (or release
    /// their smoke).
    pub(super) fn fly_shells(&mut self) {
        let shells = std::mem::take(&mut self.artillery.shells);
        let mut destroyed: Vec<(usize, UnitId, Vec3)> = Vec::new();
        for mut shell in shells {
            shell.elapsed += 1;
            if shell.elapsed < shell.total {
                self.artillery.shells.push(shell);
                continue;
            }
            let p = shell.target.ground();
            let at = Vec3::new(p.x, p.y, self.map.height_at(p));
            if shell.smoke {
                self.release_smoke(&shell, at);
            } else {
                self.explode(&shell, at, &mut destroyed);
            }
        }
        for (j, by, at) in destroyed {
            if self.units[j].alive() {
                continue;
            }
            self.destroy(j, by, at);
        }
    }

    /// A smoke shell landed: a new cloud (the oldest goes if there are too
    /// many).
    fn release_smoke(&mut self, shell: &Shell, at: Vec3) {
        let a = &mut self.artillery;
        let radius = a.rules.smoke_radius(shell.caliber_mm_x10);
        if a.clouds.len() >= a.rules.smoke.max_clouds as usize {
            a.clouds.remove(0);
        }
        let id = a.next_cloud;
        a.next_cloud = a.next_cloud.wrapping_add(1);
        a.clouds.push(Cloud {
            id,
            center: at.ground(),
            radius,
            born: self.tick,
            expires: self.tick + a.smoke_ticks,
            dark: false,
        });
        self.events.push(CombatEvent::Smoke {
            projectile: shell.id,
            shooter: shell.shooter,
            at,
            radius,
        });
    }

    /// A high-explosive shell landed at `at`: a direct hit on a vehicle,
    /// an object or the ground, then fragments in the blast radius.
    fn explode(&mut self, shell: &Shell, at: Vec3, destroyed: &mut Vec<(usize, UnitId, Vec3)>) {
        let p = at.ground();
        let caliber = shell.caliber_mm_x10;
        let blast = self.artillery.rules.blast;
        let radius = self.artillery.rules.blast_radius(caliber);
        let vehicle_damage = {
            let c = u64::from(caliber);
            u32::try_from(c * c * u64::from(self.combat_rules.damage.vehicle_pct_per_mm2) / 10_000)
                .unwrap_or(u32::MAX)
                .max(1)
        };
        let reach = radius + self.unit_grid.max_reach;
        let mut near = Vec::new();
        self.unit_grid.query_sorted(
            p - Vec2::new(reach, reach),
            p + Vec2::new(reach, reach),
            &mut near,
        );
        // Direct hit: the point lies in the hull outline of a vehicle.
        let direct = near.iter().map(|&j| j as usize).find(|&j| {
            let u = &self.units[j];
            let b = &self.artillery.bodies[u.unit_type as usize];
            if b.infantry || !u.alive() {
                return false;
            }
            let rel = p - u.position;
            let right = Vec2::new(-u.facing.y, u.facing.x);
            dot(rel, u.facing).abs() <= b.half_length && dot(rel, right).abs() <= b.half_width
        });
        let mut kind = HitKind::Ground;
        let (mut face, mut penetrated, mut total) = (None, false, 0u32);
        let mut damage = |sim: &mut Self, j: usize, hp: u32, soldier: Option<usize>| -> u32 {
            let unit = &mut sim.units[j];
            if hp == 0 || !unit.alive() {
                return 0;
            }
            match (&mut unit.squad, soldier) {
                (Some(squad), Some(k)) => {
                    let s = &mut squad.soldiers[k];
                    let taken = hp.min(s.hp);
                    s.hp -= taken;
                    unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
                    if unit.hp == 0 {
                        destroyed.push((j, shell.shooter, at));
                    }
                    taken
                }
                _ => {
                    let taken = hp.min(unit.hp);
                    unit.hp -= taken;
                    if unit.hp == 0 {
                        destroyed.push((j, shell.shooter, at));
                    }
                    taken
                }
            }
        };
        if let Some(j) = direct {
            let b = self.artillery.bodies[self.units[j].unit_type as usize];
            let pen = u64::from(caliber) * u64::from(blast.direct_hit_pen_pct) / 1000;
            let roof = u64::from(b.thinnest_mm) * u64::from(blast.roof_pct) / 100;
            kind = HitKind::Unit(self.units[j].id);
            face = Some(Face::Top);
            if pen >= roof {
                penetrated = true;
                total += damage(self, j, vehicle_damage, None);
            }
        } else {
            let top = Vec3::new(at.x, at.y, at.z + Fixed::ONE / 2);
            let height_of = |o: u32| self.destruction.hit_height(&self.map, o);
            let object = self.objects.hit(&self.map, top, at, height_of, |_| false);
            if let Some((_, o)) = object {
                kind = HitKind::Object(o);
                if let Some((j, k)) = self.building_hit(o, caliber) {
                    kind = HitKind::Unit(self.units[j].id);
                    face = Some(Face::Top);
                    total += damage(self, j, blast.soldier_damage, Some(k));
                } else {
                    let from = (p - shell.origin.ground()).length();
                    let dir = if from > Fixed::ZERO {
                        let d = p - shell.origin.ground();
                        Vec2::new(d.x / from, d.y / from)
                    } else {
                        Vec2::new(Fixed::ZERO, Fixed::ONE)
                    };
                    total += self.destruction.hit(
                        &self.map,
                        o,
                        (caliber, vehicle_damage),
                        dir,
                        at,
                        shell.shooter,
                    );
                }
            } else {
                self.ground_hit(p, caliber);
            }
        }
        // Big blasts fell and strip the trees around (prompt 085).
        let map = std::sync::Arc::clone(&self.map);
        self.destruction.blast(&map, caliber, at, shell.shooter);
        // Who the blast knocks down (prompt 048), before fragments strike.
        let knockdown = self.infantry.rules.knockdown;
        let thrown = self.blast_reach(p, knockdown.radius_mm(caliber), knockdown.throw_mm(caliber));
        // Fragments. Rolls in unit and soldier order (deterministic).
        let stand = self.infantry.rules.stance.stand.height_mm.max(1);
        let lying = self.infantry.rules.stance.prone.height_mm;
        for &j in &near {
            let j = j as usize;
            if Some(j) == direct || !self.units[j].alive() {
                continue;
            }
            let unit = &self.units[j];
            let b = self.artillery.bodies[unit.unit_type as usize];
            if let Some(squad) = &unit.squad {
                if squad.garrison.is_some() {
                    continue;
                }
                let height = self.infantry.rules.stance.values(squad.stance).height_mm;
                let mut close = false;
                let mut hits: Vec<usize> = Vec::new();
                let soldiers: Vec<(usize, Fixed, bool, u32)> = squad
                    .soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive())
                    .map(|(k, s)| {
                        (
                            k,
                            (s.position - p).length(),
                            s.at_spot() && s.cover.is_some(),
                            if s.knocked > 0 { lying } else { height },
                        )
                    })
                    .collect();
                for (k, d, covered, height) in soldiers {
                    if d >= radius {
                        continue;
                    }
                    close = true;
                    // Chance in hundredths of a percent.
                    let mut chance = Fixed::from_num(blast.infantry_pct * 100) * (radius - d)
                        / radius
                        * Fixed::from_num(height)
                        / Fixed::from_num(stand);
                    if covered {
                        chance = chance * Fixed::from_num(blast.cover_pct) / 100;
                    }
                    let chance = chance.to_num::<u32>();
                    if self.rng.below(10_000) < chance && !self.crater_catches(j, k) {
                        hits.push(k);
                    }
                }
                for k in hits {
                    total += damage(self, j, blast.soldier_damage, Some(k));
                }
                if close {
                    let ticks = self.infantry.types[self.units[j].unit_type as usize]
                        .as_ref()
                        .map_or(0, |t| t.suppressed_ticks);
                    let ticks = self
                        .experience
                        .suppression_ticks(self.units[j].veterancy, ticks);
                    if let Some(s) = &mut self.units[j].squad {
                        s.suppressed = s.suppressed.max(ticks);
                    }
                    self.suppress_blast(j);
                }
            } else if b.thinnest_mm <= blast.open_armor_mm && unit.crew.full == 0 {
                let d = (unit.position - p).length();
                if d < radius {
                    let hp = Fixed::from_num(
                        u64::from(caliber) * u64::from(blast.open_damage_per_mm) / 10,
                    ) * (radius - d)
                        / radius;
                    total += damage(self, j, hp.to_num::<u32>(), None);
                }
            }
        }
        self.events.push(CombatEvent::Hit {
            projectile: shell.id,
            shooter: shell.shooter,
            at,
            kind,
            face,
            turret: false,
            penetrated,
            damage: total,
            caliber_mm_x10: caliber,
        });
        self.knock_down(&thrown, p);
        // Prompt 177: fragments strike the crews of guns, not the guns.
        self.crew_fragments(p, radius, blast.infantry_pct, blast.soldier_damage);
    }

    /// Drifts the smoke clouds, dissolves old ones and puts the smoke into
    /// the vision grid; viewsheds that contain changed cells are queued.
    pub(super) fn update_smoke(&mut self) {
        let tick = self.tick;
        let a = &mut self.artillery;
        if a.clouds.is_empty() && a.smoked.is_empty() {
            return;
        }
        let wind = a.wind;
        a.clouds.retain(|c| c.expires > tick);
        for c in &mut a.clouds {
            c.center = c.center + wind;
        }
        let grid = &self.vision.grid;
        let mut cells: Vec<u32> = Vec::new();
        for c in &a.clouds {
            let r = Vec2::new(c.radius, c.radius);
            let (c0, r0) = grid.cell_of(c.center - r);
            let (c1, r1) = grid.cell_of(c.center + r);
            for row in r0..=r1 {
                for col in c0..=c1 {
                    let (col, row) = (col as u32, row as u32);
                    if (grid.center(col, row) - c.center).length() <= c.radius {
                        cells.push(row * grid.columns + col);
                    }
                }
            }
        }
        cells.sort_unstable();
        cells.dedup();
        if cells == a.smoked {
            return;
        }
        let old = std::mem::replace(&mut a.smoked, cells);
        let new = &a.smoked;
        let height = a.smoke_height_mm;
        let columns = grid.columns;
        let mut changed: Vec<(u32, bool)> = Vec::new();
        let (mut x, mut y) = (0, 0);
        while x < old.len() || y < new.len() {
            match (old.get(x), new.get(y)) {
                (Some(&o), Some(&n)) if o == n => {
                    x += 1;
                    y += 1;
                }
                (Some(&o), Some(&n)) if o < n => {
                    changed.push((o, false));
                    x += 1;
                }
                (Some(&o), None) => {
                    changed.push((o, false));
                    x += 1;
                }
                (_, Some(&n)) => {
                    changed.push((n, true));
                    y += 1;
                }
                (None, None) => break,
            }
        }
        let grid = std::sync::Arc::make_mut(&mut self.vision.grid);
        let (mut c0, mut r0, mut c1, mut r1) = (u32::MAX, u32::MAX, 0, 0);
        for &(cell, on) in &changed {
            grid.set_smoke(cell, on.then_some(height));
            let (c, r) = (cell % columns, cell / columns);
            (c0, r0, c1, r1) = (c0.min(c), r0.min(r), c1.max(c), r1.max(r));
        }
        if !changed.is_empty() {
            self.requeue_viewsheds((c0, r0, c1, r1));
        }
    }
}
