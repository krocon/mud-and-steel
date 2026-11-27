//! Air support and anti-aircraft fire (prompt 033): calls, flight along a
//! fixed track, strafing, bombs, flak, crashes and reconnaissance. Rules
//! and public types: `crate::air`.
//!
//! Order within a tick (see [`Sim::step`]): calls are commands
//! ([`Sim::order_air`]); [`Sim::update_air`] runs after the shells (the
//! unit grid holds this tick's positions): cooldowns, flight, who sees
//! which aircraft, flak, strafing, bomb releases, falling bombs and
//! crashes; then the viewsheds of reconnaissance aircraft, before the
//! vision update.
//!
//! Aircraft are no units: victory, pathfinding, separation, production and
//! the computer player's unit groups never see them.

use super::{Sim, within_world};
use crate::air::{
    AA_WEAPON, AIR_SHOOTER, AaWeapon, AirBlock, AirEvent, AirGun, AirMission, AirOption, AirRules,
    AirType, AircraftSnapshot, BombSnapshot, SortieSnapshot,
};
use crate::combat::{
    CombatEvent, Face, FireMode, HitKind, Vec3, WeaponType, fixed_sqrt, sin_cos_rad,
};
use crate::command::{PlayerId, UnitId};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::vision::Viewshed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A gun of an aircraft type with the rules applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GunRt {
    caliber_mm_x10: u32,
    /// Roof penetration (mm) at the muzzle.
    pen_mm: Fixed,
    vehicle_damage: u32,
    infantry_damage: u32,
    /// Rounds of all its barrels during one burst.
    rounds: u32,
}

/// An aircraft type with the rules applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct TypeRt {
    /// Meters per tick.
    speed: Fixed,
    guns: Vec<GunRt>,
}

/// An anti-aircraft weapon of a unit type with the rules applied.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct AaRt {
    weapon: AaWeapon,
    reload_ticks: u32,
}

/// A called mission: the track of its aircraft.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Sortie {
    id: u32,
    owner: PlayerId,
    mission: AirMission,
    target: Vec2,
    entry: Vec2,
    exit: Vec2,
    /// Distance from the entry to the target along the track.
    target_s: Fixed,
}

/// The circle of a `recon_loiter` aircraft: it reaches the circle
/// tangentially at `start` along its track (the center on its left), flies
/// `length` meters around it and leaves from `exit` along `exit_dir`.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Loiter {
    center: Vec2,
    radius: Fixed,
    /// Direction of the approach (heading at the start of the circle).
    dir: Vec2,
    start: Fixed,
    length: Fixed,
    exit: Vec2,
    exit_dir: Vec2,
}

impl Loiter {
    /// Ground position and heading `d` meters into the circle.
    fn at(center: Vec2, radius: Fixed, dir: Vec2, d: Fixed) -> (Vec2, Vec2) {
        let left = Vec2::new(-dir.y, dir.x);
        let (sin, cos) = sin_cos(d / radius.max(Fixed::ONE));
        let pos = center + (dir.scale(sin) - left.scale(cos)).scale(radius);
        (pos, left.scale(sin) + dir.scale(cos))
    }
}

/// `(sin, cos)` of any angle in radians: reduced to a quarter turn around
/// a multiple of pi / 2, then [`sin_cos_rad`].
fn sin_cos(x: Fixed) -> (Fixed, Fixed) {
    let quarter = Fixed::PI / 2;
    let k = (x / quarter).round();
    let (sin, cos) = sin_cos_rad(x - k * quarter);
    match k.to_num::<i64>().rem_euclid(4) {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    }
}

/// A shot-down aircraft falling.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Crash {
    left: u32,
    total: u32,
    /// Height over the ground when it was shot down.
    from_m: Fixed,
}

/// An aircraft in the air.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Plane {
    id: u32,
    sortie: u32,
    owner: PlayerId,
    mission: AirMission,
    /// Index into [`Air::types`].
    kind: u16,
    hp: u32,
    /// Start of its own track (the sortie's entry shifted sideways) and
    /// direction.
    origin: Vec2,
    dir: Vec2,
    /// Distance flown from `origin` (negative: still waiting behind it).
    s: Fixed,
    /// Length of the track.
    length: Fixed,
    /// The attack zone `[lo, hi]` along the track (attack altitude), and
    /// the ground height over which altitudes count.
    zone: (Fixed, Fixed),
    base_m: Fixed,
    position: Vec3,
    /// Meters per tick.
    velocity: Vec3,
    crash: Option<Crash>,
    /// Bombs still to drop: `(release s, impact s)`, ascending.
    releases: Vec<(Fixed, Fixed)>,
    /// Ticks until the next burst of its guns.
    burst: u32,
    firing: bool,
    /// Viewshed of a reconnaissance aircraft.
    shed: Option<Viewshed>,
    /// Circle of a `recon_loiter` aircraft.
    loiter: Option<Loiter>,
    /// Bit `s`: player slot `s` sees it.
    seen: u64,
}

impl Plane {
    const fn alive(&self) -> bool {
        self.hp > 0 && self.crash.is_none()
    }

    /// Ground position and heading at `s` along its track.
    fn track_at(&self, s: Fixed) -> (Vec2, Vec2) {
        match &self.loiter {
            Some(l) if s >= l.start => {
                let d = s - l.start;
                if d < l.length {
                    Loiter::at(l.center, l.radius, l.dir, d)
                } else {
                    (l.exit + l.exit_dir.scale(d - l.length), l.exit_dir)
                }
            }
            _ => (self.origin + self.dir.scale(s), self.dir),
        }
    }
}

/// A falling bomb.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bomb {
    id: u32,
    owner: PlayerId,
    origin: Vec3,
    target: Vec3,
    total: u32,
    elapsed: u32,
    kg: u32,
}

impl Bomb {
    fn at(&self, k: u32) -> Vec3 {
        let s = Fixed::from_num(k) / Fixed::from_num(self.total.max(1));
        let lerp = |a: Fixed, b: Fixed| a + (b - a) * s;
        Vec3::new(
            lerp(self.origin.x, self.target.x),
            lerp(self.origin.y, self.target.y),
            // Free fall: the height drops with the square of the time.
            self.origin.z - (self.origin.z - self.target.z) * s * s,
        )
    }
}

/// Air support of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Air {
    rules: AirRules,
    types: Vec<AirType>,
    rt: Vec<TypeRt>,
    /// Anti-aircraft weapons per unit type.
    aa: Vec<Vec<AaRt>>,
    /// Sight range (m) per unit type: who sees an aircraft.
    sight: Vec<Fixed>,
    /// Players in slot order, their start centers and cooldowns (ticks).
    players: Vec<PlayerId>,
    homes: Vec<Vec2>,
    cooldowns: Vec<[u32; AirMission::ALL.len()]>,
    sorties: Vec<Sortie>,
    planes: Vec<Plane>,
    bombs: Vec<Bomb>,
    /// Ticks until each AA weapon of a unit fires again (only while one
    /// reloads).
    reloads: BTreeMap<UnitId, Vec<u32>>,
    next_sortie: u32,
    next_plane: u32,
    /// Viewshed radius of reconnaissance aircraft (vision cells).
    pub(super) recon_radius: i32,
    tick_hz: u32,
    /// Events of the last tick (not state: derived, not hashed).
    pub(super) events: Vec<AirEvent>,
}

impl Air {
    pub(super) fn new(rules: AirRules, config: &super::SimConfig) -> Self {
        let catalog = crate::economy::builtin_catalog();
        let scale = crate::scale::Scale::builtin().ok();
        let combat = &config.combat;
        let hz = config.tick_hz.max(1);
        let types = match (catalog, &scale) {
            (Some(c), Some(s)) => AirType::all(c, &rules, s),
            _ => Vec::new(),
        };
        // Prompt 117: aircraft of factions the match does not keep are
        // left out (they come last, so the kinds of the others stay).
        let types: Vec<AirType> = types
            .into_iter()
            .filter(|t| crate::economy::keeps_faction(&t.faction, &config.unit_types))
            .collect();
        let rt = types
            .iter()
            .map(|t| TypeRt {
                speed: t.speed_m_per_s / Fixed::from_num(hz),
                guns: t
                    .guns
                    .iter()
                    .map(|g| gun_rt(g, combat, rules.strafe.burst_ticks, hz))
                    .collect(),
            })
            .collect();
        let aa = config
            .unit_types
            .iter()
            .map(|t| {
                let (Some(c), Some(s)) = (catalog, &scale) else {
                    return Vec::new();
                };
                let Some(entry) = c.get(&t.name) else {
                    return Vec::new();
                };
                AaWeapon::from_catalog(entry, &rules, s)
                    .into_iter()
                    .map(|w| {
                        let mut ms = 60_000 / w.rate_rpm.max(1);
                        if w.rate_rpm >= combat.fire.automatic_rpm {
                            ms = ms * 100 / combat.fire.automatic_sustained_pct.max(1);
                        }
                        let seconds = s.game_duration(entry.category, ms);
                        let ticks = (seconds * Fixed::from_num(hz)).round().to_num::<u32>();
                        AaRt {
                            weapon: w,
                            reload_ticks: ticks.max(1),
                        }
                    })
                    .collect()
            })
            .collect();
        let sight = config
            .unit_types
            .iter()
            .map(|t| config.vision.range_m(t.combat.category))
            .collect();
        let players = config.players();
        let homes = players
            .iter()
            .map(|&p| {
                let own: Vec<Vec2> = config
                    .spawns
                    .iter()
                    .filter(|s| s.owner == p)
                    .map(|s| s.position)
                    .collect();
                let n = Fixed::from_num(own.len().max(1));
                own.iter()
                    .fold(Vec2::ZERO, |sum, &q| sum + Vec2::new(q.x / n, q.y / n))
            })
            .collect();
        let (num, den) = config.vision.sight_ratio;
        let recon_m = Fixed::from_num(rules.recon.sight_real_m) * Fixed::from_num(num)
            / Fixed::from_num(den.max(1));
        let recon_radius = (recon_m / config.vision.cell_m()).to_num::<i32>().max(1);
        Self {
            types,
            rt,
            aa,
            sight,
            cooldowns: vec![[0; AirMission::ALL.len()]; players.len()],
            players,
            homes,
            sorties: Vec::new(),
            planes: Vec::new(),
            bombs: Vec::new(),
            reloads: BTreeMap::new(),
            next_sortie: 0,
            next_plane: 0,
            recon_radius,
            tick_hz: hz,
            events: Vec::new(),
            rules,
        }
    }

    fn slot(&self, player: PlayerId) -> Option<usize> {
        self.players.binary_search(&player).ok()
    }

    /// The aircraft type `faction` flies on `mission`.
    fn kind_of(&self, faction: Option<&str>, mission: AirMission) -> Option<u16> {
        let faction = faction?;
        self.types
            .iter()
            .position(|t| t.faction == faction && t.mission == mission)
            .and_then(|i| u16::try_from(i).ok())
    }

    fn cooldown_ticks(&self, mission: AirMission) -> u32 {
        self.rules
            .mission(mission)
            .cooldown_s
            .saturating_mul(self.tick_hz)
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher, paratroopers: &[u32]) {
        self.rules.hash_into(h);
        // Prompt 117: only the paratrooper squads the match has.
        for &t in paratroopers {
            h.write_u32(t);
        }
        h.write_u64(self.types.len() as u64);
        for t in &self.types {
            h.write_bytes(t.name.as_bytes());
            h.write_u8(t.mission as u8);
        }
        for c in &self.cooldowns {
            for &v in c {
                h.write_u32(v);
            }
        }
        h.write_u32(self.next_sortie);
        h.write_u32(self.next_plane);
        h.write_u64(self.sorties.len() as u64);
        for s in &self.sorties {
            h.write_u32(s.id);
            h.write_u8(s.owner.0);
            h.write_u8(s.mission as u8);
            for v in [s.target.x, s.target.y, s.entry.x, s.entry.y, s.target_s] {
                h.write_i64(v.to_bits());
            }
        }
        h.write_u64(self.planes.len() as u64);
        for p in &self.planes {
            h.write_u32(p.id);
            h.write_u32(p.sortie);
            h.write_u8(p.owner.0);
            h.write_u32(u32::from(p.kind));
            h.write_u32(p.hp);
            for v in [
                p.origin.x,
                p.origin.y,
                p.dir.x,
                p.dir.y,
                p.s,
                p.length,
                p.zone.0,
                p.zone.1,
                p.base_m,
                p.position.x,
                p.position.y,
                p.position.z,
            ] {
                h.write_i64(v.to_bits());
            }
            match p.crash {
                None => h.write_u8(0),
                Some(c) => {
                    h.write_u8(1);
                    h.write_u32(c.left);
                    h.write_u32(c.total);
                    h.write_i64(c.from_m.to_bits());
                }
            }
            h.write_u64(p.releases.len() as u64);
            for &(a, b) in &p.releases {
                h.write_i64(a.to_bits());
                h.write_i64(b.to_bits());
            }
            h.write_u32(p.burst);
            h.write_u64(p.seen);
            match &p.loiter {
                None => h.write_u8(0),
                Some(l) => {
                    h.write_u8(1);
                    for v in [
                        l.center.x,
                        l.center.y,
                        l.radius,
                        l.dir.x,
                        l.dir.y,
                        l.start,
                        l.length,
                        l.exit.x,
                        l.exit.y,
                        l.exit_dir.x,
                        l.exit_dir.y,
                    ] {
                        h.write_i64(v.to_bits());
                    }
                }
            }
            match &p.shed {
                None => h.write_u8(0),
                Some(s) => {
                    h.write_u8(1);
                    h.write_u32(s.anchor);
                    h.write_u32(s.eye_mm.cast_unsigned());
                    h.write_u32(s.count());
                }
            }
        }
        h.write_u64(self.bombs.len() as u64);
        for b in &self.bombs {
            h.write_u32(b.id);
            h.write_u8(b.owner.0);
            for v in [
                b.origin.x, b.origin.y, b.origin.z, b.target.x, b.target.y, b.target.z,
            ] {
                h.write_i64(v.to_bits());
            }
            h.write_u32(b.total);
            h.write_u32(b.elapsed);
            h.write_u32(b.kg);
        }
        h.write_u64(self.reloads.len() as u64);
        for (id, r) in &self.reloads {
            h.write_u32(id.0);
            for &v in r {
                h.write_u32(v);
            }
        }
    }
}

fn gun_rt(g: &AirGun, combat: &crate::CombatRules, burst_ticks: u32, hz: u32) -> GunRt {
    let weapon = WeaponType {
        name: String::new(),
        count: g.count,
        caliber_mm_x10: g.caliber_mm_x10,
        rate_rpm: g.rate_rpm,
        muzzle_velocity_mps: g.muzzle_velocity_mps,
        range_m: Fixed::ZERO,
        interval_s: Fixed::ONE,
        speed_m_per_s: Fixed::ZERO,
    };
    let rounds =
        u64::from(g.count) * u64::from(g.rate_rpm) * u64::from(burst_ticks) / (60 * u64::from(hz));
    GunRt {
        caliber_mm_x10: g.caliber_mm_x10,
        pen_mm: if g.muzzle_velocity_mps > 0 {
            combat.muzzle_penetration_mm(&weapon)
        } else {
            Fixed::ZERO
        },
        vehicle_damage: combat.vehicle_damage(&weapon),
        infantry_damage: combat.infantry_damage(&weapon),
        rounds: u32::try_from(rounds).unwrap_or(u32::MAX).max(1),
    }
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

fn triangular(rng: &mut crate::rng::Rng) -> Fixed {
    let a = rng.range_i32(-500, 500);
    let b = rng.range_i32(-500, 500);
    Fixed::from_num(a + b) / 1000
}

/// Distance from `p` along `-dir` (`back`) and `dir` to the edge of the
/// rectangle `±half`.
fn to_edges(p: Vec2, dir: Vec2, half: Vec2) -> (Fixed, Fixed) {
    let tiny = Fixed::from_num(1) / 1000;
    let axis = |v: Fixed, d: Fixed, h: Fixed| -> (Fixed, Fixed) {
        if d.abs() < tiny {
            (Fixed::MAX, Fixed::MAX)
        } else if d > Fixed::ZERO {
            (
                ((v + h) / d).max(Fixed::ZERO),
                ((h - v) / d).max(Fixed::ZERO),
            )
        } else {
            (
                ((h - v) / -d).max(Fixed::ZERO),
                ((v + h) / -d).max(Fixed::ZERO),
            )
        }
    };
    let (bx, fx) = axis(p.x, dir.x, half.x);
    let (by, fy) = axis(p.y, dir.y, half.y);
    (bx.min(by), fx.min(fy))
}

/// Seconds to fall `height` meters.
fn fall_seconds(height: Fixed, gravity_mm_per_s2: u32) -> Fixed {
    let g = Fixed::from_num(gravity_mm_per_s2) / 1000;
    fixed_sqrt(height.max(Fixed::ZERO) * 2 / g)
}

impl Sim {
    /// The air rules of the match (prompt 055: paratroopers).
    pub(super) const fn air_rules(&self) -> &AirRules {
        &self.air.rules
    }

    /// [`crate::Command::CallAirLine`] (prompt 055): `mission` along
    /// `from` → `to`, over its middle.
    pub(super) fn order_air_line(
        &mut self,
        player: PlayerId,
        mission: AirMission,
        (from, to): (Vec2, Vec2),
    ) {
        if !within_world(from) || !within_world(to) {
            return;
        }
        let d = to - from;
        let length = d.length();
        if length < Fixed::ONE {
            self.order_air_on(player, mission, from, None);
            return;
        }
        let dir = Vec2::new(d.x / length, d.y / length);
        let max = Fixed::from_num(self.air.rules.bombs.carpet_max_m);
        let length = length.min(max);
        let mid = Vec2::new((from.x + to.x) / 2, (from.y + to.y) / 2);
        self.order_air_on(player, mission, mid, Some((dir, length)));
    }

    /// Calls `mission` on `target` for `player` if the player's faction
    /// has the aircraft, the mission is ready and the player can pay.
    pub(super) fn order_air(&mut self, player: PlayerId, mission: AirMission, target: Vec2) {
        self.order_air_on(player, mission, target, None);
    }

    /// [`Self::order_air`], optionally along a line: direction and length
    /// (prompt 055).
    fn order_air_on(
        &mut self,
        player: PlayerId,
        mission: AirMission,
        target: Vec2,
        line: Option<(Vec2, Fixed)>,
    ) {
        if !within_world(target) || !self.map.contains(target) {
            self.reject(player, super::RejectReason::OutsideMap, None);
            return;
        }
        if let Some(block) = self.air_block(player, mission) {
            let reason = match block {
                AirBlock::NotEnoughResources => Some(super::RejectReason::CannotAfford),
                AirBlock::NoAircraft | AirBlock::Cooldown => {
                    Some(super::RejectReason::NotAvailable)
                }
                AirBlock::Defeated => None,
            };
            if let Some(reason) = reason {
                self.reject(player, reason, None);
            }
            return;
        }
        let faction = self.faction(player);
        let (Some(slot), Some(kind)) = (self.air.slot(player), self.air.kind_of(faction, mission))
        else {
            return;
        };
        let cost = self.air.rules.mission(mission).cost;
        if !self.economy.pay(player, cost) {
            return;
        }
        self.air.cooldowns[slot][mission.index()] = self.air.cooldown_ticks(mission);
        self.launch(player, slot, mission, kind, target, line);
    }

    /// Why `player` cannot call `mission` now (`None`: it can).
    fn air_block(&self, player: PlayerId, mission: AirMission) -> Option<AirBlock> {
        let Some(slot) = self.air.slot(player) else {
            return Some(AirBlock::Defeated);
        };
        if self.alive_units(player) == 0
            || self
                .standings()
                .iter()
                .any(|s| s.player == player && s.defeat.is_some())
        {
            return Some(AirBlock::Defeated);
        }
        let Some(kind) = self.air.kind_of(self.faction(player), mission) else {
            return Some(AirBlock::NoAircraft);
        };
        // Paratroopers need a squad type of the faction in the match.
        if mission == AirMission::Airborne
            && self
                .para_type(player, self.air.types[usize::from(kind)].seats)
                .is_none()
        {
            return Some(AirBlock::NoAircraft);
        }
        if self.air.cooldowns[slot][mission.index()] > 0 {
            return Some(AirBlock::Cooldown);
        }
        if !self
            .economy
            .can_pay(player, self.air.rules.mission(mission).cost)
        {
            return Some(AirBlock::NotEnoughResources);
        }
        None
    }

    /// Puts the aircraft of a new sortie on their track.
    fn launch(
        &mut self,
        owner: PlayerId,
        slot: usize,
        mission: AirMission,
        kind: u16,
        target: Vec2,
        line: Option<(Vec2, Fixed)>,
    ) {
        // Paratroopers: squads per aircraft instead of bombs (prompt 055).
        let squads = if mission == AirMission::Airborne {
            let seats = self.air.types[usize::from(kind)].seats;
            self.para_type(owner, seats).map_or(0, |(_, _, n)| n)
        } else {
            0
        };
        let air = &mut self.air;
        let rules = air.rules.mission(mission);
        let d = target - air.homes[slot];
        let length = d.length();
        let dir = if let Some((dir, _)) = line {
            dir
        } else if length > Fixed::ONE {
            Vec2::new(d.x / length, d.y / length)
        } else {
            Vec2::new(Fixed::ZERO, -Fixed::ONE)
        };
        let size = self.map.size();
        let half = Vec2::new(size.x / 2, size.y / 2);
        let margin = Fixed::from_num(air.rules.approach.edge_margin_m);
        let (back, ahead) = to_edges(target, dir, half);
        let (back, ahead) = (back + margin, ahead + margin);
        let entry = target - dir.scale(back);
        let exit = target + dir.scale(ahead);
        let id = air.next_sortie;
        air.next_sortie = air.next_sortie.wrapping_add(1);
        air.sorties.push(Sortie {
            id,
            owner,
            mission,
            target,
            entry,
            exit,
            target_s: back,
        });
        air.events.push(AirEvent::Called {
            sortie: id,
            owner,
            mission,
            target,
        });
        let ty = &air.types[usize::from(kind)];
        let speed = air.rt[usize::from(kind)].speed;
        let hz = Fixed::from_num(air.tick_hz);
        let s_rules = air.rules.strafe;
        let b_rules = air.rules.bombs;
        let base_m = self.map.height_at(target);
        let attack_m = Fixed::from_num(rules.attack_m);
        // Bombs: impacts along the own track around the target, released
        // one fall time early.
        let fall_ticks = (fall_seconds(attack_m, b_rules.gravity_mm_per_s2) * hz)
            .round()
            .max(Fixed::ONE);
        let lead = speed * fall_ticks;
        let bombs = ty.bombs.map_or(squads, |(n, _)| u32::from(n));
        let stick = Fixed::from_num(b_rules.stick_m);
        // Paratroopers jump over their landing point.
        let lead = if mission == AirMission::Airborne {
            Fixed::ZERO
        } else {
            lead
        };
        let carpet = line.map_or(Fixed::ZERO, |(_, l)| l);
        let n = rules.aircraft;
        // A carpet (prompt 055): every bomber spreads its bombs over the
        // whole line; every transport drops at its own point of the line.
        let spread = |k: u32, count: u32| {
            if count <= 1 {
                Fixed::ZERO
            } else {
                -carpet / 2 + carpet * Fixed::from_num(k) / Fixed::from_num(count - 1)
            }
        };
        let releases_of = |plane: u32| -> Vec<(Fixed, Fixed)> {
            (0..bombs)
                .map(|k| {
                    let off = if carpet == Fixed::ZERO {
                        stick * Fixed::from_num(2 * i64::from(k) - i64::from(bombs) + 1) / 2
                    } else if mission == AirMission::Airborne {
                        spread(plane * bombs + k, n * bombs)
                    } else {
                        spread(k, bombs)
                    };
                    let impact = back + off;
                    (impact - lead, impact)
                })
                .collect()
        };
        let mut releases: Vec<(Fixed, Fixed)> = (0..n).flat_map(releases_of).collect();
        releases.sort_unstable();
        let gun_lead = Fixed::from_num(s_rules.lead_m);
        let mut zone = (back, back);
        if !air.rt[usize::from(kind)].guns.is_empty() {
            zone = (
                back - Fixed::from_num(s_rules.before_m) - gun_lead,
                back + Fixed::from_num(s_rules.after_m) - gun_lead,
            );
        }
        if let (Some(first), Some(last)) = (releases.first(), releases.last()) {
            zone = (
                zone.0.min(first.0 - Fixed::from_num(50)),
                zone.1.max(last.1),
            );
        }
        let left = Vec2::new(-dir.y, dir.x);
        let max_hp = ty.hp;
        // Circling reconnaissance: radius and meters flown on the circle.
        let circle = (mission == AirMission::ReconLoiter).then(|| {
            (
                Fixed::from_num(rules.loiter_radius_m),
                speed * hz * Fixed::from_num(rules.loiter_s),
            )
        });
        for k in 0..n {
            let side = Fixed::from_num(rules.lateral_m)
                * Fixed::from_num(2 * i64::from(k) - i64::from(n) + 1)
                / 2;
            let mut origin = entry + left.scale(side);
            let mut length = back + ahead;
            let mut loiter = None;
            if let Some((radius, around)) = circle {
                // The straight approach passes the target `radius` to its
                // right: tangent to the circle at `back`.
                origin = origin - left.scale(radius);
                let (exit, exit_dir) = Loiter::at(target, radius, dir, around);
                let out = to_edges(exit, exit_dir, half).1 + margin;
                length = back + around + out;
                loiter = Some(Loiter {
                    center: target,
                    radius,
                    dir,
                    start: back,
                    length: around,
                    exit,
                    exit_dir,
                });
            }
            let s = -Fixed::from_num(u64::from(rules.trail_m) * u64::from(k));
            let plane = air.next_plane;
            air.next_plane = air.next_plane.wrapping_add(1);
            air.planes.push(Plane {
                id: plane,
                sortie: id,
                owner,
                mission,
                kind,
                hp: max_hp,
                origin,
                dir,
                s,
                length,
                zone,
                base_m,
                position: Vec3::new(
                    origin.x,
                    origin.y,
                    base_m + Fixed::from_num(rules.altitude_m),
                ),
                velocity: Vec3::default(),
                crash: None,
                releases: releases_of(k),
                burst: 0,
                firing: false,
                shed: None,
                loiter,
                seen: 0,
            });
        }
    }

    /// Altitude (m above `base_m`) of a plane at `s` along its track.
    fn altitude(&self, p: &Plane) -> Fixed {
        let rules = self.air.rules.mission(p.mission);
        let cruise = Fixed::from_num(rules.altitude_m);
        let attack = Fixed::from_num(rules.attack_m);
        let dive = Fixed::from_num(rules.dive_m);
        let (lo, hi) = p.zone;
        let d = if p.s < lo {
            lo - p.s
        } else if p.s > hi {
            p.s - hi
        } else {
            Fixed::ZERO
        };
        if d == Fixed::ZERO {
            attack
        } else if dive == Fixed::ZERO || d >= dive {
            cruise
        } else {
            attack + (cruise - attack) * d / dive
        }
    }

    /// The air phase of a tick (see the module documentation).
    pub(super) fn update_air(&mut self) {
        for c in &mut self.air.cooldowns {
            for v in c.iter_mut() {
                *v = v.saturating_sub(1);
            }
        }
        self.air.reloads.retain(|_, r| {
            for v in r.iter_mut() {
                *v = v.saturating_sub(1);
            }
            r.iter().any(|&v| v > 0)
        });
        if self.air.planes.is_empty() && self.air.bombs.is_empty() {
            return;
        }
        self.fly_planes();
        self.spot_planes();
        self.fire_flak();
        self.strafe();
        self.release_bombs();
        self.fall_bombs();
        self.update_recon();
        let planes = &self.air.planes;
        self.air
            .sorties
            .retain(|s| planes.iter().any(|p| p.sortie == s.id));
    }

    fn fly_planes(&mut self) {
        let mut planes = std::mem::take(&mut self.air.planes);
        let mut crashed: Vec<Plane> = Vec::new();
        planes.retain_mut(|p| {
            let speed = self.air.rt[usize::from(p.kind)].speed;
            p.s += speed;
            let (ground, _) = p.track_at(p.s);
            let floor = if self.map.contains(ground) {
                self.map.height_at(ground)
            } else {
                p.base_m
            };
            let z = if let Some(c) = &mut p.crash {
                c.left = c.left.saturating_sub(1);
                floor + c.from_m * Fixed::from_num(c.left) / Fixed::from_num(c.total.max(1))
            } else {
                let z = p.base_m + self.altitude(p);
                // Never into a hill.
                z.max(floor + Fixed::from_num(20))
            };
            let at = Vec3::new(ground.x, ground.y, z);
            p.velocity = Vec3::new(
                at.x - p.position.x,
                at.y - p.position.y,
                at.z - p.position.z,
            );
            p.position = at;
            if p.crash.is_some_and(|c| c.left == 0) {
                crashed.push(p.clone());
                return false;
            }
            if p.s > p.length {
                if let Some(shed) = p.shed.take() {
                    self.vision.remove_air_shed(p.owner, &shed);
                }
                return false;
            }
            true
        });
        self.air.planes = planes;
        for p in crashed {
            self.crash(&p);
        }
    }

    /// A falling plane hit the ground.
    fn crash(&mut self, p: &Plane) {
        if let Some(shed) = &p.shed {
            self.vision.remove_air_shed(p.owner, shed);
        }
        let ground = p.position.ground();
        let at = if self.map.contains(ground) {
            Vec3::new(ground.x, ground.y, self.map.surface_height_at(ground))
        } else {
            p.position
        };
        self.air.events.push(AirEvent::Crashed {
            aircraft: p.id,
            owner: p.owner,
            kind: p.kind,
            at,
            facing: p.track_at(p.s).1,
        });
        let caliber = self.air.rules.crash.caliber_mm_x10;
        if caliber > 0 && self.map.contains(ground) {
            let id = self.next_projectile;
            self.next_projectile = self.next_projectile.wrapping_add(1);
            let facing = p.track_at(p.s).1;
            let from = Vec3::new(
                at.x - facing.x * 10,
                at.y - facing.y * 10,
                at.z + Fixed::from_num(10),
            );
            self.detonate(id, AIR_SHOOTER, p.owner, from, at, caliber);
        }
    }

    /// Who sees which plane: its owner, and every player with a living
    /// unit within that unit's sight range (horizontal).
    fn spot_planes(&mut self) {
        let air = &mut self.air;
        for p in &mut air.planes {
            let own = air.players.binary_search(&p.owner).map_or(0, |s| 1u64 << s);
            let mut mask = own;
            let at = p.position.ground();
            for u in &self.units {
                if !u.alive() || self.map.inside_tunnel(u.position).is_some() {
                    continue;
                }
                let Ok(slot) = air.players.binary_search(&u.owner) else {
                    continue;
                };
                let bit = 1u64 << slot;
                if mask & bit != 0 {
                    continue;
                }
                let r = air.sight[u.unit_type as usize];
                let d = at - u.position;
                if d.x.abs() <= r && d.y.abs() <= r && d.length() <= r {
                    mask |= bit;
                }
            }
            // Teams (prompt 037): allies see each other's planes, and with
            // shared vision what any ally sees.
            let teams = &self.teams;
            if !teams.is_free_for_all() {
                let seen = mask;
                for (s, &q) in air.players.iter().enumerate() {
                    let shares = air.players.iter().enumerate().any(|(t, &r)| {
                        seen & (1 << t) != 0 && teams.shared_vision && teams.allied(q, r)
                    });
                    if teams.allied(q, p.owner) || shares {
                        mask |= 1 << s;
                    }
                }
            }
            p.seen = mask;
        }
    }

    /// Anti-aircraft fire of every unit with AA weapons.
    fn fire_flak(&mut self) {
        if !self.air.planes.iter().any(Plane::alive) {
            return;
        }
        let common = self.air.rules.flak_rules;
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            let t = unit.unit_type as usize;
            if !unit.alive()
                || unit.combat.fire_mode == FireMode::HoldFire
                || self.air.aa.get(t).is_none_or(Vec::is_empty)
                // Passengers, towed guns and units inside tunnels do not fire
                // flak (prompts 034, 148).
                || self.transport_blocks_fire(i)
                || self.map.inside_tunnel(unit.position).is_some()
            {
                continue;
            }
            let (id, owner) = (unit.id, unit.owner);
            let Ok(slot) = self.air.players.binary_search(&owner) else {
                continue;
            };
            let eye = Vec3::new(
                unit.position.x,
                unit.position.y,
                unit.height + Fixed::from_num(2),
            );
            for w in 0..self.air.aa[t].len() {
                let aa = self.air.aa[t][w];
                let ready = self.air.reloads.get(&id).is_none_or(|r| r[w] == 0);
                if !ready {
                    continue;
                }
                let class = self.air.rules.flak(aa.weapon.class);
                let min_alt = Fixed::from_num(class.min_altitude_m);
                let range = aa.weapon.range;
                let target = self
                    .air
                    .planes
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        p.alive() && self.teams.hostile(p.owner, owner) && p.seen & (1 << slot) != 0
                    })
                    .filter(|(_, p)| p.position.z - eye.z >= min_alt)
                    .filter_map(|(k, p)| {
                        let d = Vec3::new(
                            p.position.x - eye.x,
                            p.position.y - eye.y,
                            p.position.z - eye.z,
                        );
                        let dx = d.x.abs().max(d.y.abs());
                        if dx > range {
                            return None;
                        }
                        let dist = d.length();
                        (dist <= range).then_some((dist, p.id, k))
                    })
                    .min();
                let Some((dist, _, k)) = target else {
                    continue;
                };
                let reloads = self
                    .air
                    .reloads
                    .entry(id)
                    .or_insert_with(|| vec![0; self.air.aa[t].len()]);
                reloads[w] = aa.reload_ticks;
                let p = &self.air.planes[k];
                let ty = &self.air.types[usize::from(p.kind)];
                // Chance in hundredths of a percent.
                let mut chance =
                    Fixed::from_num(class.hit_pct * 100) * (Fixed::ONE - dist / (range * 2));
                chance = chance * ty.wingspan_m.max(Fixed::ONE)
                    / Fixed::from_num(common.ref_wingspan_m.max(1));
                let chance = chance
                    .max(Fixed::ZERO)
                    .min(Fixed::from_num(10_000))
                    .to_num::<u32>();
                let caliber = aa.weapon.caliber_mm_x10;
                let damage = (caliber * common.damage_per_mm / 10).max(1);
                let mut hits = 0u32;
                for _ in 0..aa.weapon.count {
                    if self.rng.below(10_000) < chance {
                        hits += 1;
                    }
                }
                let spread = if hits > 0 { 3000 } else { 25_000 };
                let off = Vec3::new(
                    Fixed::from_num(self.rng.range_i32(-spread, spread)) / 1000,
                    Fixed::from_num(self.rng.range_i32(-spread, spread)) / 1000,
                    Fixed::from_num(self.rng.range_i32(-spread, spread)) / 1000,
                );
                let plane_at = self.air.planes[k].position;
                let burst = Vec3::new(plane_at.x + off.x, plane_at.y + off.y, plane_at.z + off.z);
                let projectile = self.next_projectile;
                self.next_projectile = self.next_projectile.wrapping_add(1);
                self.events.push(CombatEvent::Shot {
                    unit: id,
                    weapon: AA_WEAPON,
                    projectile,
                    muzzle: eye,
                    caliber_mm_x10: caliber,
                });
                let plane_id = self.air.planes[k].id;
                self.air.events.push(AirEvent::Flak {
                    unit: id,
                    aircraft: plane_id,
                    at: burst,
                    class: aa.weapon.class,
                    hit: hits > 0,
                });
                let to = plane_at.ground() - self.units[i].position;
                let length = to.length();
                if length > Fixed::ZERO {
                    self.units[i].combat.aim = Vec2::new(to.x / length, to.y / length);
                }
                if hits > 0 {
                    self.damage_plane(k, damage.saturating_mul(hits), id);
                }
            }
        }
    }

    fn damage_plane(&mut self, k: usize, damage: u32, by: UnitId) {
        let fall = self.air.rules.crash.fall_ms;
        let hz = self.air.tick_hz;
        let p = &mut self.air.planes[k];
        if !p.alive() {
            return;
        }
        p.hp = p.hp.saturating_sub(damage);
        if p.hp > 0 {
            return;
        }
        let total = u32::try_from(u64::from(fall) * u64::from(hz) / 1000)
            .unwrap_or(u32::MAX)
            .max(1);
        let floor = if self.map.contains(p.position.ground()) {
            self.map.height_at(p.position.ground())
        } else {
            p.base_m
        };
        p.crash = Some(Crash {
            left: total,
            total,
            from_m: (p.position.z - floor).max(Fixed::ZERO),
        });
        p.releases.clear();
        p.firing = false;
        let (id, owner, at) = (p.id, p.owner, p.position);
        if let Some(shed) = p.shed.take() {
            self.vision.remove_air_shed(owner, &shed);
        }
        self.air.events.push(AirEvent::ShotDown {
            aircraft: id,
            owner,
            by,
            at,
        });
    }

    /// Bursts of the guns of strafing aircraft.
    fn strafe(&mut self) {
        let rules = self.air.rules.strafe;
        let lead = Fixed::from_num(rules.lead_m);
        for k in 0..self.air.planes.len() {
            let p = &mut self.air.planes[k];
            p.firing = false;
            if !p.alive() || !p.mission.strafes() {
                continue;
            }
            let rt = &self.air.rt[usize::from(p.kind)];
            if rt.guns.is_empty() {
                continue;
            }
            let back = self
                .air
                .sorties
                .iter()
                .find(|s| s.id == p.sortie)
                .map_or(Fixed::ZERO, |s| s.target_s);
            let lo = back - Fixed::from_num(rules.before_m) - lead;
            let hi = back + Fixed::from_num(rules.after_m) - lead;
            if p.s < lo || p.s > hi {
                continue;
            }
            p.firing = true;
            p.burst = p.burst.saturating_sub(1);
            if p.burst > 0 {
                continue;
            }
            p.burst = rules.burst_ticks;
            let length = rt.speed * Fixed::from_num(rules.burst_ticks);
            let from = p.position.ground() + p.dir.scale(lead);
            let (plane, dir, muzzle) = (p.id, p.dir, p.position);
            self.strafe_burst(k, plane, muzzle, from, dir, length);
        }
    }

    /// One burst: every unit in the strip from `from` along `dir` may be
    /// hit.
    fn strafe_burst(
        &mut self,
        k: usize,
        plane: u32,
        muzzle: Vec3,
        from: Vec2,
        dir: Vec2,
        length: Fixed,
    ) {
        let rules = self.air.rules.strafe;
        let blast = *self.indirect_blast();
        let half_w = Fixed::from_num(rules.width_m) / 2;
        let to = from + dir.scale(length);
        let kind = usize::from(self.air.planes[k].kind);
        let guns = self.air.rt[kind].guns.clone();
        let Some(main) = guns.iter().max_by_key(|g| g.caliber_mm_x10) else {
            return;
        };
        let caliber = main.caliber_mm_x10;
        let at_ground = |sim: &Self, p: Vec2| {
            let z = if sim.map.contains(p) {
                sim.map.surface_height_at(p)
            } else {
                Fixed::ZERO
            };
            Vec3::new(p.x, p.y, z)
        };
        self.air.events.push(AirEvent::Strafe {
            aircraft: plane,
            muzzle,
            from: at_ground(self, from),
            to: at_ground(self, to),
            caliber_mm_x10: caliber,
        });
        let reach = half_w + self.unit_grid.max_reach;
        let low = Vec2::new(from.x.min(to.x) - reach, from.y.min(to.y) - reach);
        let high = Vec2::new(from.x.max(to.x) + reach, from.y.max(to.y) + reach);
        let mut near = Vec::new();
        self.unit_grid.query_sorted(low, high, &mut near);
        let left = Vec2::new(-dir.y, dir.x);
        let inside = |p: Vec2, r: Fixed| {
            let rel = p - from;
            let along = dot(rel, dir);
            along >= -r && along <= length + r && dot(rel, left).abs() <= half_w + r
        };
        let stand = self.infantry.rules.stance.stand.height_mm.max(1);
        let infantry_damage = guns.iter().map(|g| g.infantry_damage).max().unwrap_or(1);
        let mut destroyed: Vec<usize> = Vec::new();
        let mut hit_events: Vec<(usize, u32)> = Vec::new();
        for &j in &near {
            let j = j as usize;
            if !self.units[j].alive() || self.map.inside_tunnel(self.units[j].position).is_some() {
                continue;
            }
            if self.units[j].squad.is_some() {
                let unit = &self.units[j];
                let squad = unit.squad.as_ref().expect("checked");
                if squad.garrison.is_some() {
                    continue;
                }
                let height = self.infantry.rules.stance.values(squad.stance).height_mm;
                let soldiers: Vec<(usize, bool)> = squad
                    .soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.alive() && inside(s.position, Fixed::ZERO))
                    .map(|(n, s)| (n, s.at_spot() && s.cover.is_some()))
                    .collect();
                if soldiers.is_empty() {
                    continue;
                }
                let mut taken = 0u32;
                for (n, covered) in soldiers {
                    let mut chance = rules.soldier_pct * 100 * height / stand;
                    if covered {
                        chance = chance * blast.cover_pct / 100;
                    }
                    if self.rng.below(10_000) < chance && !self.crater_catches(j, n) {
                        let unit = &mut self.units[j];
                        let squad = unit.squad.as_mut().expect("checked");
                        let s = &mut squad.soldiers[n];
                        let d = infantry_damage.min(s.hp);
                        s.hp -= d;
                        taken += d;
                        unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
                    }
                }
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
                if taken > 0 {
                    hit_events.push((j, taken));
                    if !self.units[j].alive() {
                        destroyed.push(j);
                    }
                }
                continue;
            }
            let unit = &self.units[j];
            let footprint = unit.footprint / 2;
            if !inside(unit.position, footprint) {
                continue;
            }
            let thinnest = u32::from(
                self.unit_types[unit.unit_type as usize]
                    .combat
                    .armor
                    .thinnest(),
            );
            let open = thinnest <= blast.open_armor_mm;
            let roof = Fixed::from_num(thinnest * blast.roof_pct) / 100;
            let mut total = 0u32;
            for g in &guns {
                for _ in 0..g.rounds {
                    if self.rng.below(100) >= rules.vehicle_pct {
                        continue;
                    }
                    total = total.saturating_add(if open {
                        g.caliber_mm_x10 * rules.open_damage_per_mm / 10
                    } else if g.pen_mm >= roof {
                        g.vehicle_damage
                    } else {
                        0
                    });
                }
            }
            if total > 0 {
                let unit = &mut self.units[j];
                let d = total.min(unit.hp);
                unit.hp -= d;
                hit_events.push((j, d));
                if unit.hp == 0 {
                    destroyed.push(j);
                }
            }
        }
        // Dust along the strip, then the units hit.
        for n in 0..3 {
            let p = from + dir.scale(length * Fixed::from_num(2 * n + 1) / 6);
            let id = self.next_projectile;
            self.next_projectile = self.next_projectile.wrapping_add(1);
            let at = at_ground(self, p);
            self.events.push(CombatEvent::Hit {
                projectile: id,
                shooter: AIR_SHOOTER,
                at,
                kind: HitKind::Ground,
                face: None,
                turret: false,
                penetrated: false,
                damage: 0,
                caliber_mm_x10: caliber,
            });
        }
        for (j, damage) in hit_events {
            let id = self.next_projectile;
            self.next_projectile = self.next_projectile.wrapping_add(1);
            let u = &self.units[j];
            let at = Vec3::new(u.position.x, u.position.y, u.height + Fixed::ONE);
            self.events.push(CombatEvent::Hit {
                projectile: id,
                shooter: AIR_SHOOTER,
                at,
                kind: HitKind::Unit(u.id),
                face: Some(Face::Top),
                turret: false,
                penetrated: true,
                damage,
                caliber_mm_x10: caliber,
            });
        }
        for j in destroyed {
            let u = &self.units[j];
            let at = Vec3::new(u.position.x, u.position.y, u.height);
            self.destroy(j, AIR_SHOOTER, at);
        }
    }

    /// Bombs leave their aircraft one fall time before the impact point.
    fn release_bombs(&mut self) {
        let rules = self.air.rules.bombs;
        let hz = Fixed::from_num(self.air.tick_hz);
        for k in 0..self.air.planes.len() {
            loop {
                let p = &mut self.air.planes[k];
                if !p.alive() || p.releases.first().is_none_or(|&(s, _)| p.s < s) {
                    break;
                }
                let (_, impact_s) = p.releases.remove(0);
                let (owner, origin, dir, position, mission, plane) =
                    (p.owner, p.origin, p.dir, p.position, p.mission, p.id);
                if mission == AirMission::Airborne {
                    // A stick of paratroopers jumps (prompt 055).
                    self.drop_paratroopers(owner, position, origin + dir.scale(impact_s), dir);
                    continue;
                }
                let kg = self.air.types[usize::from(p.kind)]
                    .bombs
                    .map_or(0, |(_, kg)| kg);
                let error = Fixed::from_num(if mission == AirMission::Bomb {
                    rules.error_bomb_m
                } else {
                    rules.error_attack_m
                });
                let aim = origin + dir.scale(impact_s);
                let ex = triangular(&mut self.rng) * error;
                let ey = triangular(&mut self.rng) * error;
                let mut impact = aim + Vec2::new(ex, ey);
                if !self.map.contains(impact) {
                    impact = aim;
                }
                let z = if self.map.contains(impact) {
                    self.map.surface_height_at(impact)
                } else {
                    Fixed::ZERO
                };
                let fall = fall_seconds(position.z - z, rules.gravity_mm_per_s2);
                let total = (fall * hz).round().to_num::<u32>().max(1);
                let id = self.next_projectile;
                self.next_projectile = self.next_projectile.wrapping_add(1);
                self.air.bombs.push(Bomb {
                    id,
                    owner,
                    origin: position,
                    target: Vec3::new(impact.x, impact.y, z),
                    total,
                    elapsed: 0,
                    kg,
                });
                self.air.events.push(AirEvent::Released {
                    aircraft: plane,
                    bomb: id,
                    at: position,
                });
            }
        }
    }

    /// Moves the falling bombs; those that arrive explode.
    fn fall_bombs(&mut self) {
        let bombs = std::mem::take(&mut self.air.bombs);
        for mut b in bombs {
            b.elapsed += 1;
            if b.elapsed < b.total {
                self.air.bombs.push(b);
                continue;
            }
            let at = b.target;
            if !self.map.contains(at.ground()) {
                continue;
            }
            let caliber = self.air.rules.bomb_caliber(b.kg);
            self.detonate(b.id, AIR_SHOOTER, b.owner, b.origin, at, caliber);
            self.bomb_near_miss(at, b.kg);
        }
    }

    /// Armored vehicles close to a bomb impact (not hit directly) take
    /// damage falling with the distance.
    fn bomb_near_miss(&mut self, at: Vec3, kg: u32) {
        let radius = self.air.rules.near_miss_radius(kg);
        let max = self.air.rules.bombs.near_miss_damage;
        if radius <= Fixed::ZERO || max == 0 {
            return;
        }
        let open_mm = self.indirect_blast().open_armor_mm;
        let p = at.ground();
        let reach = radius + self.unit_grid.max_reach;
        let mut near = Vec::new();
        self.unit_grid.query_sorted(
            p - Vec2::new(reach, reach),
            p + Vec2::new(reach, reach),
            &mut near,
        );
        let mut destroyed = Vec::new();
        for &j in &near {
            let j = j as usize;
            let u = &self.units[j];
            if !u.alive() || u.squad.is_some() || self.map.inside_tunnel(u.position).is_some() {
                continue;
            }
            let ty = &self.unit_types[u.unit_type as usize];
            if u32::from(ty.combat.armor.thinnest()) <= open_mm {
                continue;
            }
            // A direct hit (inside the hull outline) was the shell's.
            let rel = p - u.position;
            let right = Vec2::new(-u.facing.y, u.facing.x);
            let inside = dot(rel, u.facing).abs() <= ty.combat.length_m / 2
                && dot(rel, right).abs() <= ty.combat.width_m / 2;
            let d = rel.length();
            if inside || d >= radius {
                continue;
            }
            let hp = (Fixed::from_num(max) * (radius - d) / radius).to_num::<u32>();
            let unit = &mut self.units[j];
            let taken = hp.min(unit.hp);
            unit.hp -= taken;
            if unit.hp == 0 {
                destroyed.push(j);
            }
        }
        for j in destroyed {
            self.destroy(j, AIR_SHOOTER, at);
        }
    }

    /// Viewsheds of reconnaissance aircraft from their flight altitude.
    fn update_recon(&mut self) {
        let grid = Arc::clone(&self.vision.grid);
        let radius = self.air.recon_radius;
        for k in 0..self.air.planes.len() {
            let p = &self.air.planes[k];
            if !p.mission.reveals() {
                continue;
            }
            let ground = p.position.ground();
            let over_map = p.alive() && self.map.contains(ground);
            let anchor = grid.index_of(ground);
            if over_map && p.shed.as_ref().is_some_and(|s| s.anchor == anchor) {
                continue;
            }
            let owner = p.owner;
            let eye = (p.position.z * Fixed::from_num(1000))
                .round()
                .to_num::<i32>();
            let old = self.air.planes[k].shed.take();
            if let Some(old) = old {
                self.vision.remove_air_shed(owner, &old);
            }
            if over_map {
                let (shed, _) = grid.viewshed(anchor, eye, radius);
                self.vision.add_air_shed(owner, &shed);
                self.air.planes[k].shed = Some(shed);
            }
        }
    }

    // --- Queries ----------------------------------------------------------------

    /// The aircraft types of the match (every faction and mission).
    pub fn air_types(&self) -> &[AirType] {
        &self.air.types
    }

    /// What `player` can call, one entry per mission.
    pub fn air_options(&self, player: PlayerId) -> Vec<AirOption> {
        let slot = self.air.slot(player);
        let faction = self.faction(player);
        AirMission::ALL
            .into_iter()
            .map(|m| AirOption {
                mission: m,
                aircraft: self
                    .air
                    .kind_of(faction, m)
                    .map(|k| self.air.types[usize::from(k)].name.clone()),
                cost: self.air.rules.mission(m).cost,
                cooldown_left: slot.map_or(0, |s| self.air.cooldowns[s][m.index()]),
                cooldown_total: self.air.cooldown_ticks(m),
                blocked: self.air_block(player, m),
            })
            .collect()
    }

    /// Air events of the last tick.
    pub fn air_events(&self) -> &[AirEvent] {
        &self.air.events
    }

    /// Range (m) of the longest anti-aircraft weapon of a unit, if it has
    /// one.
    pub fn aa_range(&self, id: UnitId) -> Option<Fixed> {
        let i = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        self.air.aa[self.units[i].unit_type as usize]
            .iter()
            .map(|a| a.weapon.range)
            .max()
    }

    /// Aircraft, bombs and sorties for the snapshot; with `player`, only
    /// the aircraft that player sees and the player's own sorties.
    pub(super) fn air_snapshots(
        &self,
        player: Option<PlayerId>,
        aircraft: &mut Vec<AircraftSnapshot>,
        bombs: &mut Vec<BombSnapshot>,
        sorties: &mut Vec<SortieSnapshot>,
    ) {
        let air = &self.air;
        let hz = Fixed::from_num(air.tick_hz);
        let bit = player.map(|p| air.slot(p).map_or(0, |s| 1u64 << s));
        aircraft.clear();
        aircraft.extend(
            air.planes
                .iter()
                .filter(|p| bit.is_none_or(|b| p.seen & b != 0))
                .map(|p| AircraftSnapshot {
                    id: p.id,
                    owner: p.owner,
                    mission: p.mission,
                    kind: p.kind,
                    position: p.position,
                    velocity: Vec3::new(p.velocity.x * hz, p.velocity.y * hz, p.velocity.z * hz),
                    hp: p.hp,
                    max_hp: air.types[usize::from(p.kind)].hp,
                    crashing: p.crash.is_some(),
                    firing: p.firing,
                }),
        );
        bombs.clear();
        bombs.extend(air.bombs.iter().map(|b| {
            let now = b.at(b.elapsed);
            let next = b.at(b.elapsed + 1);
            BombSnapshot {
                id: b.id,
                owner: b.owner,
                position: now,
                velocity: Vec3::new(
                    (next.x - now.x) * hz,
                    (next.y - now.y) * hz,
                    (next.z - now.z) * hz,
                ),
                target: b.target,
                ticks_left: b.total - b.elapsed,
            }
        }));
        sorties.clear();
        sorties.extend(
            air.sorties
                .iter()
                .filter(|s| player.is_none_or(|p| p == s.owner))
                .map(|s| {
                    let planes = air.planes.iter().filter(|p| p.sortie == s.id);
                    let ticks = planes
                        .clone()
                        .filter(|p| p.alive())
                        .map(|p| {
                            let speed = air.rt[usize::from(p.kind)].speed.max(Fixed::ONE / 1000);
                            ((s.target_s - p.s).max(Fixed::ZERO) / speed).to_num::<u32>()
                        })
                        .min()
                        .unwrap_or(0);
                    SortieSnapshot {
                        id: s.id,
                        owner: s.owner,
                        mission: s.mission,
                        target: s.target,
                        entry: s.entry,
                        exit: s.exit,
                        ticks_to_target: ticks,
                        aircraft: u16::try_from(planes.filter(|p| p.alive()).count())
                            .unwrap_or(u16::MAX),
                    }
                }),
        );
    }
}
