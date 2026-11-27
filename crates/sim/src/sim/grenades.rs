//! Infantry fragmentation and smoke hand grenades (prompt 232).
//!
//! Rules live in `data/infantry.toml` (`[grenades]`, `crate::infantry::GrenadeRules`).
//!
//! - Combat infantry squads whose category is listed in `grenades.categories`
//!   start with full frag and smoke stock lazily (no per-squad entry is stored
//!   until a squad receives an order, throws a grenade, or enters cooldown).
//! - On [`Command::ThrowGrenade`] or [`Command::ThrowSmokeGrenade`], the
//!   simulation picks **one** suitable squad from `units` (nearest ready squad
//!   without a pending throw order, else nearest with stock and shortest
//!   cooldown) so a single click never wastes every selected squad's stock at
//!   once. Shift+click or repeated orders pick the next ready squad.
//! - If `target` is outside `range_mm`, the chosen squad leaves any occupied
//!   building and advances toward `target` until within range, then stops,
//!   throws via its nearest living soldier (setting `soldier_flags::THROWING`),
//!   and resumes normal combat/cover.
//! - Flying grenades follow a high parabolic arc; smoke grenades release a
//!   vision-blocking smoke cloud (`sim/indirect.rs`) on landing, while frag
//!   grenades pause for `fuse_ms` after landing and then detonate with high
//!   lethality against infantry in the open, trenches, cover and buildings,
//!   bunkers and open-topped vehicles (ineffective against closed armor).
//! - Spent grenades are automatically replenished by allied supply trucks and
//!   ammunition depots in `sim/logistics.rs`.

use super::{RejectReason, Route, Sim, stop, within_world};
use crate::combat::{CombatEvent, Face, HitKind, Vec3};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::infantry::GrenadeRules;
use crate::math::{Fixed, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Duration of the soldier throw animation flag in real ms (~1.0 s real = 0.5 s game).
const THROW_ANIM_MS: u32 = 1000;

/// Pending grenade throw while a squad moves into range (or waits for cooldown).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PendingThrow {
    target: Vec2,
    smoke: bool,
}

/// Per-squad grenade state (only stored once modified from full idle stock).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SquadGrenades {
    frag: u8,
    smoke: u8,
    cooldown: u32,
    order: Option<PendingThrow>,
    /// `(soldier_index, ticks_left)` for the throw animation clip.
    anim: Option<(u16, u32)>,
}

/// A hand grenade currently in flight or sitting on the ground during its fuse.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct FlyingGrenade {
    id: u32,
    thrower: UnitId,
    owner: PlayerId,
    origin: Vec3,
    target: Vec3,
    apex: Fixed,
    flight_ticks: u32,
    fuse_ticks: u32,
    elapsed: u32,
    smoke: bool,
}

impl FlyingGrenade {
    fn pos_at(&self, elapsed: u32) -> Vec3 {
        if elapsed >= self.flight_ticks {
            return self.target;
        }
        let s = Fixed::from_num(elapsed) / Fixed::from_num(self.flight_ticks.max(1));
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
        h.write_u32(self.thrower.0);
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
        h.write_u32(self.flight_ticks);
        h.write_u32(self.fuse_ticks);
        h.write_u32(self.elapsed);
        h.write_u8(u8::from(self.smoke));
    }
}

/// Snapshot of a flying or fusing grenade for presentation (`artillery_view.gd`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrenadeSnapshot {
    pub id: u32,
    pub position: Vec3,
    pub previous: Vec3,
    pub smoke: bool,
}

/// Grenade subsystem state on [`Sim`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct Grenades {
    squads: BTreeMap<UnitId, SquadGrenades>,
    flying: Vec<FlyingGrenade>,
}

impl Grenades {
    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        if self.squads.is_empty() && self.flying.is_empty() {
            return;
        }
        h.write_u8(0x32);
        h.write_u64(self.squads.len() as u64);
        for (id, s) in &self.squads {
            h.write_u32(id.0);
            h.write_u8(s.frag);
            h.write_u8(s.smoke);
            h.write_u32(s.cooldown);
            match s.order {
                None => h.write_u8(0),
                Some(o) => {
                    h.write_u8(1);
                    h.write_i64(o.target.x.to_bits());
                    h.write_i64(o.target.y.to_bits());
                    h.write_u8(u8::from(o.smoke));
                }
            }
            match s.anim {
                None => h.write_u8(0),
                Some((k, left)) => {
                    h.write_u8(1);
                    h.write_u32(u32::from(k));
                    h.write_u32(left);
                }
            }
        }
        h.write_u64(self.flying.len() as u64);
        for g in &self.flying {
            g.hash_into(h);
        }
    }
}

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

fn ticks(ms: u32, tick_hz: u32, (num, den): (u32, u32)) -> u32 {
    let (num, den) = if num == 0 || den == 0 {
        (1, 1)
    } else {
        (u64::from(num), u64::from(den))
    };
    let t = u64::from(ms) * u64::from(tick_hz) * den / (1000 * num);
    u32::try_from(t).unwrap_or(u32::MAX)
}

impl Sim {
    fn grenade_rules(&self) -> &GrenadeRules {
        &self.infantry.rules.grenades
    }

    fn grenade_time_ratio(&self) -> (u32, u32) {
        self.unit_types
            .iter()
            .find(|t| t.combat.is_infantry())
            .map_or((2, 1), |t| t.combat.time_ratio)
    }

    fn grenade_ticks(&self, ms: u32) -> u32 {
        ticks(ms, self.tick_hz, self.grenade_time_ratio())
    }

    /// Whether unit index `i` is an infantry squad whose category carries grenades.
    fn unit_carries_grenades(&self, i: usize) -> bool {
        let u = &self.units[i];
        u.squad.is_some()
            && self
                .grenade_rules()
                .carries(self.unit_types[u.unit_type as usize].combat.category)
    }

    /// Grenade stock and cooldown of unit `id`:
    /// `(frag, max_frag, smoke, max_smoke, cooldown_ticks, cooldown_total_ticks)`.
    pub fn grenade_state(&self, id: UnitId) -> Option<(u8, u8, u8, u8, u32, u32)> {
        let g = self.grenade_rules();
        if g.enabled == 0 {
            return None;
        }
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        if !self.units[i].alive() || !self.unit_carries_grenades(i) {
            return None;
        }
        let max_frag = u8::try_from(g.frag_stock).unwrap_or(u8::MAX);
        let max_smoke = u8::try_from(g.smoke_stock).unwrap_or(u8::MAX);
        let total = self.grenade_ticks(g.cooldown_ms);
        let (frag, smoke, cd) = match self.grenades.squads.get(&id) {
            Some(s) => (s.frag, s.smoke, s.cooldown),
            None => (max_frag, max_smoke, 0),
        };
        Some((frag, max_frag, smoke, max_smoke, cd, total))
    }

    /// Global grenade throw range, frag radius and smoke radius in meters.
    pub fn grenade_params(&self) -> (Fixed, Fixed, Fixed) {
        let g = self.grenade_rules();
        (mm(g.range_mm), mm(g.frag_radius_mm), mm(g.smoke_radius_mm))
    }

    /// Active flying or fusing grenades for presentation.
    pub fn flying_grenades(&self) -> Vec<GrenadeSnapshot> {
        self.grenades
            .flying
            .iter()
            .map(|g| {
                let prev = g.elapsed.saturating_sub(1);
                GrenadeSnapshot {
                    id: g.id,
                    position: g.pos_at(g.elapsed),
                    previous: g.pos_at(prev),
                    smoke: g.smoke,
                }
            })
            .collect()
    }

    /// Whether unit `id` currently has a pending grenade throw order (not idle).
    pub(super) fn has_grenade_order(&self, id: UnitId) -> bool {
        self.grenades
            .squads
            .get(&id)
            .is_some_and(|s| s.order.is_some())
    }

    /// Whether soldier `k` of squad `id` is currently playing the throw animation.
    pub(super) fn soldier_throwing(&self, id: UnitId, k: usize) -> bool {
        self.grenades
            .squads
            .get(&id)
            .and_then(|s| s.anim)
            .is_some_and(|(idx, left)| usize::from(idx) == k && left > 0)
    }

    /// Orders that replace what a squad does cancel its pending grenade walk.
    pub(super) fn drop_grenade_orders(&mut self, command: &PlayerCommand) {
        if self.grenades.squads.is_empty()
            || !matches!(
                command.command,
                Command::MoveTo { .. }
                    | Command::Stop { .. }
                    | Command::Attack { .. }
                    | Command::AttackGround { .. }
                    | Command::AttackObject { .. }
                    | Command::Garrison { .. }
                    | Command::Patrol { .. }
                    | Command::Embark { .. }
                    | Command::FormationMove { .. }
                    | Command::Secure { .. }
                    | Command::Retreat { .. }
            )
        {
            return;
        }
        for i in self.owned_indices(command.player, command.command.units()) {
            let id = self.units[i].id;
            if let Some(s) = self.grenades.squads.get_mut(&id) {
                s.order = None;
            }
        }
    }

    fn ensure_squad_grenades(&mut self, id: UnitId) -> &mut SquadGrenades {
        let g = self.grenade_rules();
        let max_frag = u8::try_from(g.frag_stock).unwrap_or(u8::MAX);
        let max_smoke = u8::try_from(g.smoke_stock).unwrap_or(u8::MAX);
        self.grenades
            .squads
            .entry(id)
            .or_insert_with(|| SquadGrenades {
                frag: max_frag,
                smoke: max_smoke,
                cooldown: 0,
                order: None,
                anim: None,
            })
    }

    /// Executes [`Command::ThrowGrenade`] or [`Command::ThrowSmokeGrenade`].
    pub(super) fn order_throw_grenade(
        &mut self,
        player: PlayerId,
        units: &[UnitId],
        target: Vec2,
        smoke: bool,
    ) {
        if self.grenade_rules().enabled == 0 {
            return;
        }
        if !within_world(target) || !self.map.contains(target) {
            self.reject(player, RejectReason::OutsideMap, None);
            return;
        }
        let owned = self.owned_indices(player, units);
        let carriers: Vec<usize> = owned
            .iter()
            .copied()
            .filter(|&i| {
                self.unit_carries_grenades(i)
                    && self.units[i].link.is_none()
                    && !self.units[i]
                        .squad
                        .as_ref()
                        .is_some_and(|s| s.tactics.retreating)
            })
            .collect();
        if carriers.is_empty() {
            if !owned.is_empty() {
                self.reject(player, RejectReason::NoSquads, None);
            }
            return;
        }
        let g = self.grenade_rules();
        let max_frag = u8::try_from(g.frag_stock).unwrap_or(u8::MAX);
        let max_smoke = u8::try_from(g.smoke_stock).unwrap_or(u8::MAX);
        let dist2 = |p: Vec2| {
            let d = p - target;
            let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
            x * x + y * y
        };
        // Pick ONE squad: prefer free (no pending throw), ready (cooldown 0),
        // then shortest cooldown, then nearest to target, then lowest id.
        let chosen = carriers
            .into_iter()
            .filter_map(|i| {
                let id = self.units[i].id;
                let (frag, smk, cd, busy) = match self.grenades.squads.get(&id) {
                    Some(s) => (s.frag, s.smoke, s.cooldown, s.order.is_some()),
                    None => (max_frag, max_smoke, 0, false),
                };
                let stock = if smoke { smk } else { frag };
                (stock > 0).then(|| {
                    (
                        u8::from(busy),
                        u8::from(cd > 0),
                        cd,
                        dist2(self.units[i].position),
                        id,
                        i,
                    )
                })
            })
            .min();
        let Some((_, _, _, _, id, i)) = chosen else {
            self.reject(player, RejectReason::NoAmmo, None);
            return;
        };
        let range = mm(self.grenade_rules().range_mm);
        let in_range = (target - self.units[i].position).length() <= range;
        let cd = self.grenades.squads.get(&id).map_or(0, |s| s.cooldown);
        if in_range && cd == 0 {
            self.execute_throw(i, target, smoke);
            return;
        }
        // Out of range (or waiting on cooldown): advance into throw range.
        if !in_range {
            self.leave_building(i);
            self.cancel_enter(i);
            self.cancel_indirect(i);
            self.units[i].combat.attack = None;
            let goal = self.reachable_point(id, target).unwrap_or(target);
            let req = self.paths.request(
                (&self.map, &self.terrain),
                self.units[i].position,
                goal,
                self.units[i].unit_type as usize,
                vec![(id, Vec2::ZERO)],
            );
            self.units[i].target = Some(goal);
            self.units[i].route = Route::Waiting(req);
            self.units[i].replans = 0;
        }
        let s = self.ensure_squad_grenades(id);
        s.order = Some(PendingThrow { target, smoke });
    }

    /// Ticks grenade cooldowns, animations and pending auto-advance throws.
    pub(super) fn update_grenades(&mut self) {
        if self.grenade_rules().enabled == 0 || self.grenades.squads.is_empty() {
            return;
        }
        let range = mm(self.grenade_rules().range_mm);
        let ids: Vec<UnitId> = self.grenades.squads.keys().copied().collect();
        let mut to_throw: Vec<(usize, Vec2, bool)> = Vec::new();
        for id in ids {
            let Ok(i) = self.units.binary_search_by_key(&id, |u| u.id) else {
                self.grenades.squads.remove(&id);
                continue;
            };
            if !self.units[i].alive() {
                self.grenades.squads.remove(&id);
                continue;
            }
            let retreating = self.units[i]
                .squad
                .as_ref()
                .is_some_and(|s| s.tactics.retreating);
            let aboard = self.units[i].link.is_some();
            let pos = self.units[i].position;
            let route_idle = self.units[i].route == Route::None;
            let Some(st) = self.grenades.squads.get_mut(&id) else {
                continue;
            };
            st.cooldown = st.cooldown.saturating_sub(1);
            if let Some((idx, left)) = st.anim {
                if left <= 1 {
                    st.anim = None;
                } else {
                    st.anim = Some((idx, left - 1));
                }
            }
            if retreating || aboard {
                st.order = None;
                continue;
            }
            let Some(ord) = st.order else {
                continue;
            };
            let stock = if ord.smoke { st.smoke } else { st.frag };
            if stock == 0 {
                st.order = None;
                continue;
            }
            let dist = (ord.target - pos).length();
            if dist <= range {
                if st.cooldown == 0 {
                    st.order = None;
                    to_throw.push((i, ord.target, ord.smoke));
                } else {
                    stop(&mut self.units[i]);
                }
            } else if route_idle {
                // Path stopped short and could not reach throw range: cancel.
                st.order = None;
            }
        }
        for (i, target, smoke) in to_throw {
            stop(&mut self.units[i]);
            self.execute_throw(i, target, smoke);
        }
    }

    /// Launches a grenade from the living soldier of squad `i` nearest `target`.
    fn execute_throw(&mut self, i: usize, target: Vec2, smoke: bool) {
        let unit = &self.units[i];
        let (id, owner) = (unit.id, unit.owner);
        let Some(squad) = &unit.squad else {
            return;
        };
        let dist2 = |p: Vec2| {
            let d = p - target;
            let (x, y) = (i128::from(d.x.to_bits()), i128::from(d.y.to_bits()));
            x * x + y * y
        };
        let Some((soldier_idx, origin_2d, origin_h)) = squad
            .soldiers
            .iter()
            .enumerate()
            .filter(|(_, s)| s.alive() && !s.lying())
            .min_by_key(|&(k, s)| (dist2(s.position), k))
            .or_else(|| {
                squad
                    .soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive())
                    .min_by_key(|&(k, s)| (dist2(s.position), k))
            })
            .map(|(k, s)| (k, s.position, s.height))
        else {
            return;
        };
        let delta = target - unit.position;
        let len = delta.length();
        if len > Fixed::ZERO {
            self.units[i].combat.aim = Vec2::new(delta.x / len, delta.y / len);
        }
        let g = self.grenade_rules();
        let cooldown = self.grenade_ticks(g.cooldown_ms);
        let anim_ticks = self.grenade_ticks(THROW_ANIM_MS).max(1);
        let flight_ticks = self.grenade_ticks(g.flight_ms).max(1);
        let fuse_ticks = if smoke {
            0
        } else {
            self.grenade_ticks(g.fuse_ms)
        };
        let apex = mm(g.arc_apex_mm);
        let origin = Vec3::new(
            origin_2d.x,
            origin_2d.y,
            origin_h + Fixed::from_num(14) / 10,
        );
        let landing = Vec3::new(target.x, target.y, self.map.height_at(target));
        let st = self.ensure_squad_grenades(id);
        if smoke {
            if st.smoke == 0 {
                return;
            }
            st.smoke -= 1;
        } else {
            if st.frag == 0 {
                return;
            }
            st.frag -= 1;
        }
        st.cooldown = cooldown;
        st.anim = Some((u16::try_from(soldier_idx).unwrap_or(0), anim_ticks));
        let gid = self.next_projectile;
        self.next_projectile = self.next_projectile.wrapping_add(1);
        self.grenades.flying.push(FlyingGrenade {
            id: gid,
            thrower: id,
            owner,
            origin,
            target: landing,
            apex,
            flight_ticks,
            fuse_ticks,
            elapsed: 0,
            smoke,
        });
    }

    /// Advances all flying and fusing hand grenades by one tick.
    pub(super) fn fly_grenades(&mut self) {
        if self.grenades.flying.is_empty() {
            return;
        }
        let flying = std::mem::take(&mut self.grenades.flying);
        let mut destroyed: Vec<(usize, UnitId, Vec3)> = Vec::new();
        for mut g in flying {
            g.elapsed += 1;
            let total = g.flight_ticks + g.fuse_ticks;
            if g.elapsed < total {
                self.grenades.flying.push(g);
                continue;
            }
            let p = g.target.ground();
            let at = Vec3::new(p.x, p.y, self.map.height_at(p));
            if g.smoke {
                let radius = mm(self.grenade_rules().smoke_radius_mm);
                let life =
                    u64::from(self.grenade_ticks(self.grenade_rules().smoke_lifetime_ms)).max(1);
                self.spawn_smoke_cloud(g.id, g.thrower, at, radius, life);
            } else {
                self.detonate_grenade(&g, at, &mut destroyed);
            }
        }
        for (j, by, at) in destroyed {
            if self.units[j].alive() {
                continue;
            }
            self.destroy(j, by, at);
        }
    }

    /// Resolves a fragmentation hand grenade detonation at `at`.
    fn detonate_grenade(
        &mut self,
        g: &FlyingGrenade,
        at: Vec3,
        destroyed: &mut Vec<(usize, UnitId, Vec3)>,
    ) {
        let p = at.ground();
        let rules = self.grenade_rules().clone();
        let radius = mm(rules.frag_radius_mm);
        let caliber = rules.frag_caliber_mm_x10;
        let open_armor = self.indirect_blast().open_armor_mm;
        let knockdown = self.infantry.rules.knockdown;
        let thrown = self.blast_reach(p, knockdown.radius_mm(caliber), knockdown.throw_mm(caliber));

        let reach = radius + self.unit_grid.max_reach + Fixed::from_num(8);
        let mut near = Vec::new();
        self.unit_grid.query_sorted(
            p - Vec2::new(reach, reach),
            p + Vec2::new(reach, reach),
            &mut near,
        );

        let mut total_dmg = 0u32;
        let mut kind = HitKind::Ground;
        let mut hit_face = None;

        // 1. Check nearby occupied building: grenade thrown into/beside a house
        //    hits occupants with elevated close-quarters exposure.
        let building = self.building_at(p).or_else(|| {
            // Check 4 cardinal offsets within 3 m in case player clicked right next to wall.
            let step = Fixed::from_num(3);
            [
                Vec2::new(step, Fixed::ZERO),
                Vec2::new(-step, Fixed::ZERO),
                Vec2::new(Fixed::ZERO, step),
                Vec2::new(Fixed::ZERO, -step),
            ]
            .into_iter()
            .find_map(|off| self.building_at(p + off))
        });
        if let Some(b_obj) = building {
            kind = HitKind::Object(b_obj);
            let mut hits_left = 4u32;
            for j in 0..self.units.len() {
                if hits_left == 0 || !self.units[j].alive() {
                    continue;
                }
                let in_b = self.units[j]
                    .squad
                    .as_ref()
                    .is_some_and(|s| s.garrison == Some(b_obj));
                if !in_b {
                    continue;
                }
                let soldier_indices: Vec<usize> = self.units[j]
                    .squad
                    .as_ref()
                    .expect("checked")
                    .soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive())
                    .map(|(k, _)| k)
                    .collect();
                let mut hit_any = false;
                for k in soldier_indices {
                    if hits_left == 0 {
                        break;
                    }
                    if self.rng.below(100) < rules.frag_building_exposure_pct {
                        hits_left -= 1;
                        hit_any = true;
                        let u = &mut self.units[j];
                        if let Some(sq) = &mut u.squad {
                            let taken = rules.frag_damage.min(sq.soldiers[k].hp);
                            sq.soldiers[k].hp -= taken;
                            u.hp = sq.soldiers.iter().map(|s| s.hp).sum();
                            total_dmg += taken;
                            kind = HitKind::Unit(u.id);
                            hit_face = Some(Face::Top);
                            if u.hp == 0 {
                                destroyed.push((j, g.thrower, at));
                                break;
                            }
                        }
                    }
                }
                if hit_any && self.units[j].alive() {
                    self.suppress_blast(j);
                }
            }
        }

        // 2. Infantry in the open / trenches / cover, bunkers, open-topped vehicles, gun crews.
        for &j in &near {
            let j = j as usize;
            if !self.units[j].alive() {
                continue;
            }
            if self.units[j].squad.is_some() {
                if self.units[j]
                    .squad
                    .as_ref()
                    .is_some_and(|s| s.garrison.is_some())
                {
                    continue;
                }
                let soldiers: Vec<(usize, Fixed, bool)> = self.units[j]
                    .squad
                    .as_ref()
                    .expect("checked")
                    .soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive())
                    .map(|(k, s)| {
                        (
                            k,
                            (s.position - p).length(),
                            s.at_spot() && s.cover.is_some(),
                        )
                    })
                    .collect();
                let mut close = false;
                let mut struck: Vec<(usize, u32)> = Vec::new();
                for (k, d, covered) in soldiers {
                    if d >= radius {
                        continue;
                    }
                    close = true;
                    // High hit probability inside frag radius; cover objects soften
                    // it slightly (`frag_cover_pct`), trenches/open ground take full brunt.
                    let mut chance = Fixed::from_num(10_000) * (radius - d) / radius;
                    if covered {
                        chance = chance * Fixed::from_num(rules.frag_cover_pct) / 100;
                    }
                    if self.rng.below(10_000) < chance.to_num::<u32>() {
                        let dmg = (Fixed::from_num(rules.frag_damage) * (radius - d) / radius)
                            .to_num::<u32>()
                            .max(rules.frag_damage / 3);
                        struck.push((k, dmg));
                    }
                }
                for (k, dmg) in struck {
                    let u = &mut self.units[j];
                    if let Some(sq) = &mut u.squad {
                        let taken = dmg.min(sq.soldiers[k].hp);
                        sq.soldiers[k].hp -= taken;
                        u.hp = sq.soldiers.iter().map(|s| s.hp).sum();
                        total_dmg += taken;
                        kind = HitKind::Unit(u.id);
                        if u.hp == 0 {
                            destroyed.push((j, g.thrower, at));
                            break;
                        }
                    }
                }
                if close && self.units[j].alive() {
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
                continue;
            }
            // Non-infantry unit: bunkers, gun crews, or open-topped vehicles.
            let ty_idx = self.units[j].unit_type as usize;
            let (hl, hw, thinnest, _) = self.hull(ty_idx);
            let is_bunker = self.unit_types[ty_idx].name.contains("bunker");
            let d = (self.units[j].position - p).length();
            let hit_reach = radius + hl.max(hw);
            if d > hit_reach {
                continue;
            }
            if is_bunker {
                let u = &mut self.units[j];
                let taken = rules.frag_bunker_damage.min(u.hp);
                u.hp -= taken;
                total_dmg += taken;
                kind = HitKind::Unit(u.id);
                hit_face = Some(Face::Top);
                if u.hp == 0 {
                    destroyed.push((j, g.thrower, at));
                }
            } else if self.units[j].crew.full > 0 && d <= radius + Fixed::from_num(2) {
                // Open gun crew: grenade knocks out crewmen.
                let dir = if d > Fixed::ZERO {
                    let v = self.units[j].position - p;
                    Vec2::new(v.x / d, v.y / d)
                } else {
                    Vec2::new(Fixed::ZERO, Fixed::ONE)
                };
                self.crew_hit(j, caliber, dir, rules.frag_damage);
                self.crew_near_miss(p, radius, g.owner, caliber);
                kind = HitKind::Unit(self.units[j].id);
            } else if thinnest <= open_armor && d <= radius {
                let dmg = (Fixed::from_num(rules.frag_damage) * (radius - d) / radius)
                    .to_num::<u32>()
                    .max(1);
                let u = &mut self.units[j];
                let taken = dmg.min(u.hp);
                u.hp -= taken;
                total_dmg += taken;
                kind = HitKind::Unit(u.id);
                if u.hp == 0 {
                    destroyed.push((j, g.thrower, at));
                }
            }
        }

        // Knock down & throw nearby soldiers (like artillery blasts).
        self.knock_down(&thrown, p);
        self.hit_the_dirt(p, mm(self.tactics.rules.drop.blast_mm(caliber)), None);
        self.events.push(CombatEvent::Hit {
            projectile: g.id,
            shooter: g.thrower,
            at,
            kind,
            face: hit_face,
            turret: false,
            penetrated: total_dmg > 0,
            damage: total_dmg,
            caliber_mm_x10: caliber,
        });
    }

    /// Replenishes 1 spent frag and/or 1 spent smoke grenade per resupply cycle
    /// (called once per game second from `sim/logistics.rs` when near an allied
    /// supply source).
    pub(super) fn resupply_grenades(&mut self, i: usize, owner: PlayerId) -> bool {
        let g = self.grenade_rules();
        if g.enabled == 0 || !self.unit_carries_grenades(i) {
            return false;
        }
        let id = self.units[i].id;
        let max_frag = u8::try_from(g.frag_stock).unwrap_or(u8::MAX);
        let max_smoke = u8::try_from(g.smoke_stock).unwrap_or(u8::MAX);
        let Some(st) = self.grenades.squads.get(&id) else {
            return false;
        };
        let need_frag = st.frag < max_frag;
        let need_smoke = st.smoke < max_smoke;
        if !need_frag && !need_smoke {
            return false;
        }
        // Small ammo cost per grenade replenished (2 ammo).
        let cost = [0, 2, 0];
        let mut added = false;
        if need_frag
            && self.economy.pay(owner, cost)
            && let Some(st) = self.grenades.squads.get_mut(&id)
        {
            st.frag = (st.frag + 1).min(max_frag);
            added = true;
        }
        if need_smoke
            && self.economy.pay(owner, cost)
            && let Some(st) = self.grenades.squads.get_mut(&id)
        {
            st.smoke = (st.smoke + 1).min(max_smoke);
            added = true;
        }
        added
    }

    /// Whether unit `i` is missing any grenades (so logistics checks suppliers).
    pub(super) fn wants_grenades(&self, i: usize) -> bool {
        let g = self.grenade_rules();
        if g.enabled == 0 || !self.unit_carries_grenades(i) {
            return false;
        }
        let max_frag = u8::try_from(g.frag_stock).unwrap_or(u8::MAX);
        let max_smoke = u8::try_from(g.smoke_stock).unwrap_or(u8::MAX);
        self.grenades
            .squads
            .get(&self.units[i].id)
            .is_some_and(|s| s.frag < max_frag || s.smoke < max_smoke)
    }
}
