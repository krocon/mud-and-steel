//! The tactics update (prompt 047): fire teams bounding from cover to
//! cover, suppression, close firefights lying down, blind fire, mortar
//! teams and weapon jams. Rules: `crate::tactics` (`data/tactics.toml`).
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::update_stances`] picks
//! the automatic stance with [`auto_stance`]; aiming and firing
//! (`sim/combat.rs`) skip rushing and jammed soldiers and widen the
//! dispersion of suppressed squads; [`Sim::update_drills`] lets bounding
//! squads advance instead of halting; [`Sim::update_tactics`] then runs the
//! bounds, blind fire and recovery and sets the speed; movement and
//! [`Sim::update_squads`] move the soldiers ([`follow`]).
//!
//! A bound: the squad (one unit) splits into two teams by roster order. The
//! moving team runs to a line `lead` ahead of the squad position while the
//! squad position advances `advance` along its route; the other team holds
//! where it is and fires. Then the squad position halts, the moving team
//! takes cover spots near its line, settles, and the teams swap. The
//! squad position stays between the teams, so every soldier stays within
//! the squad's reach for hit tests.

use super::infantry::Squad;
use super::{Route, Sim};
use crate::combat::FireMode;
use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::infantry::{Stance, StanceOrder};
use crate::math::{Fixed, Vec2};
use crate::tactics::{MAX_SUPPRESSION, TacticsRules};
use serde::{Deserialize, Serialize};

/// Suppression is kept in thousandths of a point.
const MILLI: u32 = 1000;

/// Tactics rules with durations in ticks, built once.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct TacticsRt {
    pub(super) rules: TacticsRules,
    settle: u32,
    timeout: u32,
    overwatch: u32,
    contact: u32,
    recover_delay: u32,
    /// Thousandths of a point recovered per tick.
    recover: u32,
    remember: u32,
    jam_clear: u32,
    /// `[flank]` in ticks (prompt 110; off in older saved games).
    #[serde(default)]
    pub(super) flank: super::alert::FlankRt,
    /// `[discipline]` in ticks (prompt 176; off in older saved games).
    #[serde(default)]
    pub(super) discipline: super::discipline::DisciplineRt,
}

/// Ticks of `ms` real milliseconds at `tick_hz` with the time factor
/// `(num, den)` (as `sim/infantry.rs`).
fn ticks(ms: u32, tick_hz: u32, (num, den): (u32, u32)) -> u32 {
    let (num, den) = if num == 0 || den == 0 {
        (1, 1)
    } else {
        (u64::from(num), u64::from(den))
    };
    let t = u64::from(ms) * u64::from(tick_hz) * den / (1000 * num);
    u32::try_from(t).unwrap_or(u32::MAX)
}

fn mm(v: u32) -> Fixed {
    Fixed::from_num(v) / 1000
}

impl TacticsRt {
    pub(super) fn new(
        rules: TacticsRules,
        unit_types: &[crate::data::UnitType],
        tick_hz: u32,
    ) -> Self {
        // Infantry runs at the infantry time factor (data/scale.toml).
        let ratio = unit_types
            .iter()
            .find(|t| t.combat.is_infantry())
            .map_or((1, 1), |t| t.combat.time_ratio);
        let t = |ms: u32| ticks(ms, tick_hz, ratio);
        let per_second = t(1000).max(1);
        Self {
            settle: t(rules.fire_teams.settle_ms),
            timeout: t(rules.fire_teams.timeout_ms),
            overwatch: t(rules.fire_teams.overwatch_ms),
            contact: t(rules.fire_teams.contact_ms),
            recover_delay: t(rules.suppression.recover_delay_ms),
            recover: rules.suppression.recover_per_s * MILLI / per_second,
            remember: t(rules.blind_fire.remember_ms),
            jam_clear: t(rules.jams.clear_ms),
            // The same time factor; a match without infantry takes that of
            // its first vehicle.
            flank: super::alert::FlankRt::new(&rules.flank, |ms| {
                let general = unit_types
                    .iter()
                    .find(|t| t.combat.is_infantry())
                    .or_else(|| unit_types.first())
                    .map_or((1, 1), |t| t.combat.time_ratio);
                ticks(ms, tick_hz, general)
            }),
            discipline: super::discipline::DisciplineRt::new(&rules.discipline, |ms| {
                let general = unit_types
                    .iter()
                    .find(|t| t.combat.is_infantry())
                    .or_else(|| unit_types.first())
                    .map_or((1, 1), |t| t.combat.time_ratio);
                ticks(ms, tick_hz, general)
            }),
            rules,
        }
    }

    /// Adds `points` of suppression to a squad (soldiers lying down take
    /// `prone_pct` of it: rounds crack over them).
    fn add(&self, squad: &mut Squad, points: u32) {
        let points = if squad.stance == Stance::Prone {
            points * MILLI * self.rules.suppression.prone_pct / 100
        } else {
            points * MILLI
        };
        let t = &mut squad.tactics;
        t.suppression = (t.suppression + points).min(MAX_SUPPRESSION * MILLI);
        t.quiet = 0;
    }

    /// Suppression points of a round landing near a squad.
    pub(super) const fn near_miss_points(&self, caliber_mm_x10: u32) -> u32 {
        let s = &self.rules.suppression;
        if caliber_mm_x10 >= s.heavy_caliber_mm_x10 {
            s.heavy_near_miss
        } else {
            s.near_miss
        }
    }

    /// A round of another player landed near squad `squad`.
    pub(super) fn near_miss(&self, squad: &mut Squad, caliber_mm_x10: u32) {
        self.add(squad, self.near_miss_points(caliber_mm_x10));
    }

    /// Suppression rules in milli points for gun crews (prompt 177): points
    /// of a near miss of `caliber` and of a hit, the pinned and unpinned
    /// thresholds, ticks of quiet before recovery, recovery per tick.
    pub(super) fn crew_suppression(&self, caliber_mm_x10: u32) -> (u32, u32, u32, u32, u32, u32) {
        let s = &self.rules.suppression;
        (
            self.near_miss_points(caliber_mm_x10) * MILLI,
            s.hit * MILLI,
            s.pinned_at * MILLI,
            s.unpinned_at * MILLI,
            self.recover_delay,
            self.recover,
        )
    }

    /// A round hit a soldier of `squad`.
    pub(super) fn hit(&self, squad: &mut Squad) {
        self.add(squad, self.rules.suppression.hit);
    }

    /// A shell or bomb went off within its blast radius of `squad`.
    pub(super) fn blast(&self, squad: &mut Squad) {
        self.add(squad, self.rules.suppression.blast);
    }

    /// Dispersion of squad fire in percent of the normal one: suppression,
    /// and blind fire.
    pub(super) fn dispersion_pct(&self, squad: &Squad, blind: bool) -> u32 {
        let t = &squad.tactics;
        let suppressed = self
            .rules
            .suppression
            .dispersion_factor_pct(t.suppression / MILLI);
        if blind {
            suppressed.max(self.rules.blind_fire.dispersion_pct)
        } else {
            suppressed
        }
    }

    /// Rounds a squad fires instead of `count`: pinned and blind-firing
    /// squads fire with a share of their soldiers.
    pub(super) fn fire_count(&self, squad: &Squad, count: u32, blind: bool) -> u32 {
        let mut pct = 100;
        if squad.tactics.pinned {
            pct = pct.min(self.rules.suppression.pinned_fire_pct);
        }
        if blind {
            pct = pct.min(self.rules.blind_fire.fire_pct);
        }
        if pct == 100 || count == 0 {
            count
        } else {
            (count * pct / 100).max(1)
        }
    }

    /// Whether a fired round jams the soldier's weapon (draws from the
    /// generator only when jams are on).
    pub(super) fn roll_jam(&self, rng: &mut crate::rng::Rng) -> bool {
        let j = &self.rules.jams;
        j.enabled == 1 && j.per_100k > 0 && rng.below(100_000) < j.per_100k
    }

    /// Soldier `k` of `squad` clears a jam.
    pub(super) fn jam(&self, squad: &mut Squad, k: usize) {
        let n = squad.soldiers.len();
        let jammed = &mut squad.tactics.jammed;
        if jammed.len() < n {
            jammed.resize(n, 0);
        }
        jammed[k] = self.jam_clear;
    }

    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        self.rules.hash_into(h);
    }
}

/// A bound in progress (see the module documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Bound {
    /// 1: the squad moves and halts as one; 2: two fire teams.
    teams: u8,
    /// The team that moves (0: base of fire, 1: maneuver).
    moving: u8,
    /// Squad position at the last tick, and how far it moved this bound.
    last: Vec2,
    moved: Fixed,
    /// How far the squad position advances this bound.
    advance: Fixed,
    /// The squad position halted; the moving team takes cover.
    halted: bool,
    /// Ticks halted, and ticks since the moving team arrived.
    wait: u32,
    settled: u32,
}

/// Tactical state of a squad (prompt 047). Part of the save snapshot and
/// the state hash.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Tactics {
    /// Suppression in thousandths of a point (0..=100 000).
    pub(super) suppression: u32,
    /// Ticks since suppression was last added (saturating).
    quiet: u32,
    /// Gone to ground: suppression reached `pinned_at` and did not fall
    /// below `unpinned_at` yet.
    pub(super) pinned: bool,
    /// Ticks of contact left.
    contact: u32,
    /// The target is within close range: fight lying down.
    close: bool,
    pub(super) bound: Option<Bound>,
    /// Target that went out of sight, where it was last seen, ticks left.
    blind: Option<(UnitId, Vec2, u32)>,
    /// The squad fired blindly this tick.
    pub(super) blind_now: bool,
    /// Ticks every soldier still clears a jam (empty: no jams).
    pub(super) jammed: Vec<u32>,
}

impl Tactics {
    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u32(self.suppression);
        h.write_u32(self.quiet);
        h.write_u8(u8::from(self.pinned));
        h.write_u32(self.contact);
        h.write_u8(u8::from(self.close));
        match self.bound {
            None => h.write_u8(0),
            Some(b) => {
                h.write_u8(b.teams);
                h.write_u8(b.moving);
                h.write_i64(b.last.x.to_bits());
                h.write_i64(b.last.y.to_bits());
                h.write_i64(b.moved.to_bits());
                h.write_i64(b.advance.to_bits());
                h.write_u8(u8::from(b.halted));
                h.write_u32(b.wait);
                h.write_u32(b.settled);
            }
        }
        match self.blind {
            None => h.write_u8(0),
            Some((id, p, left)) => {
                h.write_u8(1);
                h.write_u32(id.0);
                h.write_i64(p.x.to_bits());
                h.write_i64(p.y.to_bits());
                h.write_u32(left);
            }
        }
        h.write_u8(u8::from(self.blind_now));
        h.write_u64(self.jammed.len() as u64);
        for &j in &self.jammed {
            h.write_u32(j);
        }
    }

    /// Suppression in points (0..=100).
    pub(super) const fn points(&self) -> u32 {
        self.suppression / MILLI
    }
}

/// Fire team of soldier `k` of a squad of `n` (by roster): the first half
/// is the base of fire (team 0), the rest the maneuver team (team 1).
pub(super) const fn team_of(k: usize, n: usize) -> u8 {
    if k < n / 2 { 0 } else { 1 }
}

/// Place of soldier `k` (of `n`) in his team's line while bounding:
/// (forward, right) relative to the squad, before the lead.
fn line_place(k: usize, n: usize, spacing: Fixed) -> (Fixed, Fixed) {
    let (first, size) = if team_of(k, n) == 0 {
        (0, n / 2)
    } else {
        (n / 2, n - n / 2)
    };
    let slot = i64::try_from(2 * (k - first)).unwrap_or(0) - i64::try_from(size).unwrap_or(0) + 1;
    (Fixed::ZERO, spacing * Fixed::from_num(slot) / 2)
}

/// How a soldier follows the squad (`Sim::follow_places`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Follow {
    /// As without tactics: his formation place, his cover spot when the
    /// squad stands (`standing`).
    Formation { standing: bool },
    /// His team's line at (forward, right) from the squad, or his cover
    /// spot if `spot` and he has one.
    Line {
        forward: Fixed,
        right: Fixed,
        spot: bool,
    },
    /// Stays where he is (the team gives covering fire, or the squad is
    /// pinned down).
    Hold,
}

/// How soldier `k` of `n` of a squad with `bound` follows; `standing`: the
/// squad has no route; `spacing`: the formation spacing.
pub(super) fn follow(
    bound: Option<Bound>,
    pinned: bool,
    (k, n): (usize, usize),
    standing: bool,
    spacing: Fixed,
) -> Follow {
    let Some(b) = bound else {
        return Follow::Formation { standing };
    };
    if pinned {
        return Follow::Hold;
    }
    if b.teams < 2 {
        return Follow::Formation {
            standing: standing || b.halted,
        };
    }
    if team_of(k, n) != b.moving {
        return Follow::Hold;
    }
    let (forward, right) = line_place(k, n, spacing);
    Follow::Line {
        forward: forward + b.advance / 2,
        right,
        spot: b.halted,
    }
}

/// Whether soldier `k` of `squad` may fire: not clearing a jam, not
/// rushing with his team (the moving team fires once it has taken its
/// cover spot).
pub(super) fn may_fire(squad: &Squad, k: usize) -> bool {
    let t = &squad.tactics;
    if t.jammed.get(k).is_some_and(|&j| j > 0) {
        return false;
    }
    match t.bound {
        Some(b) if b.teams == 2 && team_of(k, squad.soldiers.len()) == b.moving => {
            let s = &squad.soldiers[k];
            b.halted && (!s.has_spot() || s.at_spot())
        }
        _ => true,
    }
}

/// Whether a bounding squad's firing soldiers stand still (no dispersion
/// for moving).
pub(super) fn steady(squad: &Squad) -> bool {
    squad
        .tactics
        .bound
        .is_some_and(|b| b.teams == 2 || b.halted)
}

/// The automatic stance with tactics: pinned squads and close firefights
/// lie down, squads in contact move crouched (kneeling). Fixed stances and
/// garrisons are left alone.
pub(super) fn auto_stance(squad: &Squad, moving: bool, wanted: Stance) -> Stance {
    if squad.garrison.is_some() || squad.order != StanceOrder::Auto {
        return wanted;
    }
    let t = &squad.tactics;
    if t.pinned || t.close {
        return Stance::Prone;
    }
    if moving && (t.bound.is_some() || t.contact > 0 || squad.suppressed > 0) {
        return wanted.max(Stance::Kneel);
    }
    wanted
}

/// Tactical state of a squad for presentation and tools.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SquadTactics {
    /// Suppression, 0..=100.
    pub suppression: u32,
    pub pinned: bool,
    /// Fire teams bounding: the team that moves (0 base of fire,
    /// 1 maneuver); single-team squads report 0.
    pub bounding: Option<u8>,
    /// The squad fired blindly at a target out of sight this tick.
    pub blind_fire: bool,
    /// The target is close: the squad fights lying down.
    pub close: bool,
}

/// Per-soldier flags of [`Sim::soldier_tactics`].
pub mod soldier_flags {
    /// Clearing a jammed weapon.
    pub const JAMMED: u8 = 1;
    /// Running with the moving team (not firing).
    pub const RUSHING: u8 = 2;
    /// Member of the maneuver team (team 1).
    pub const MANEUVER: u8 = 4;
}

impl Sim {
    /// Fire teams of unit `i`: 2 when the squad is large enough and both
    /// halves still have two soldiers able to fight, else 1. Mortar teams
    /// (an indirect weapon) never split.
    fn team_count(&self, i: usize) -> u8 {
        let unit = &self.units[i];
        let Some(squad) = &unit.squad else {
            return 1;
        };
        if self.indirect_weapon(unit.id).is_some() {
            return 1;
        }
        let n = squad.soldiers.len();
        let mut living = [0u32; 2];
        for (k, s) in squad.soldiers.iter().enumerate() {
            if s.alive() {
                living[usize::from(team_of(k, n))] += 1;
            }
        }
        let min = self.tactics.rules.fire_teams.min_size;
        if living[0] + living[1] >= min && living[0] >= 2 && living[1] >= 2 {
            2
        } else {
            1
        }
    }

    /// Whether squad `i` on attack-move bounds (two fire teams) instead of
    /// halting to fight (`sim/drill.rs`).
    pub(super) fn bounds_on_advance(&self, i: usize) -> bool {
        self.units[i].squad.is_some()
            && self.units[i].combat.fire_mode != FireMode::HoldFire
            && self.team_count(i) == 2
    }

    /// Whether mortar team `i` on attack-move halts: for a target of its
    /// mortar (one its player has spotted in range; it then sets up and
    /// fires, prompt 032), not for targets of its small arms. `None` for
    /// other units (they halt for any automatic target, prompt 046).
    pub(super) fn mortar_halts(&self, i: usize) -> Option<bool> {
        let unit = &self.units[i];
        if self.tactics.rules.mortar.halt_on_attack_move == 0
            || unit.squad.is_none()
            || self.indirect_weapon(unit.id).is_none()
        {
            return None;
        }
        Some(
            unit.combat.fire_mode != FireMode::HoldFire
                && !self.has_indirect_order(i)
                && self.indirect_auto_target(i).is_some(),
        )
    }

    /// Whether indirect-fire unit `i` may pick unit `j` as automatic
    /// target: mortar teams fire at what their player has spotted, other
    /// guns at what they see themselves (prompt 032).
    pub(super) fn indirect_spots(&self, i: usize, j: usize) -> bool {
        if self.tactics.rules.mortar.spotted_targets == 1 && self.units[i].squad.is_some() {
            self.owner_sees(i, j)
        } else {
            self.sees_unit(i, j)
        }
    }

    /// Bounds, blind fire, recovery and speed of every squad. Runs after
    /// the combat and drill updates, before the movement.
    pub(super) fn update_tactics(&mut self) {
        for i in 0..self.units.len() {
            if self.units[i].squad.is_none() || !self.units[i].alive() {
                continue;
            }
            self.recover(i);
            if self.units[i].link.is_some() {
                if let Some(s) = &mut self.units[i].squad {
                    s.tactics.bound = None;
                    s.tactics.blind = None;
                }
                continue;
            }
            self.track_target(i);
            self.blind_fire_step(i);
            match self.wants_bounds(i) {
                Some(teams) => self.bound_step(i, teams),
                None => {
                    if let Some(s) = &mut self.units[i].squad {
                        s.tactics.bound = None;
                    }
                }
            }
            // Suppression slows the squad down (crawling is slow enough).
            let points = self.units[i]
                .squad
                .as_ref()
                .filter(|s| s.stance != Stance::Prone)
                .map_or(0, |s| s.tactics.points());
            let unit = &mut self.units[i];
            if points > 0 && unit.speed_pct > 0 {
                let pct = self.tactics.rules.suppression.speed_factor_pct(points);
                unit.speed_pct = (unit.speed_pct * pct / 100).max(1);
            }
        }
    }

    /// Recovery from suppression, contact, jams and pinning of squad `i`.
    fn recover(&mut self, i: usize) {
        let rt = &self.tactics;
        let s = &rt.rules.suppression;
        let Some(squad) = &mut self.units[i].squad else {
            return;
        };
        let t = &mut squad.tactics;
        t.quiet = t.quiet.saturating_add(1);
        if t.quiet > rt.recover_delay {
            t.suppression = t.suppression.saturating_sub(rt.recover);
        }
        t.contact = t.contact.saturating_sub(1);
        t.blind_now = false;
        if !t.jammed.is_empty() {
            for j in &mut t.jammed {
                *j = j.saturating_sub(1);
            }
            if t.jammed.iter().all(|&j| j == 0) {
                t.jammed.clear();
            }
        }
        let points = t.suppression / MILLI;
        if points >= s.pinned_at {
            t.pinned = true;
        } else if points < s.unpinned_at {
            t.pinned = false;
        }
    }

    /// Contact, close range and the last seen position of squad `i`'s
    /// automatic target.
    fn track_target(&mut self, i: usize) {
        let unit = &self.units[i];
        let target = unit
            .combat
            .auto_target
            .and_then(|id| self.units.binary_search_by_key(&id, |u| u.id).ok());
        let seen = target.map(|t| (self.units[t].id, self.units[t].position));
        let distance = seen.map(|(_, p)| (p - unit.position).length());
        let close = distance.is_some_and(|d| d <= mm(self.tactics.rules.close.prone_within_mm));
        // Contact: a target within the effective range; farther ones the
        // squad engages while it closes in.
        let effective = self.types[unit.unit_type as usize].max_range
            * Fixed::from_num(self.tactics.rules.fire_teams.contact_range_pct)
            / 100;
        let in_contact = distance.is_some_and(|d| d <= effective);
        let hold = unit.combat.fire_mode == FireMode::HoldFire || unit.combat.attack.is_some();
        let (contact, remember) = (self.tactics.contact, self.tactics.remember);
        let Some(squad) = &mut self.units[i].squad else {
            return;
        };
        let t = &mut squad.tactics;
        t.close = close;
        if hold {
            t.blind = None;
            return;
        }
        if let Some((id, p)) = seen {
            if in_contact {
                t.contact = contact;
            }
            t.blind = Some((id, p, remember));
        }
    }

    /// Blind fire of squad `i` at the point where its target went out of
    /// sight.
    fn blind_fire_step(&mut self, i: usize) {
        let unit = &self.units[i];
        if unit.combat.auto_target.is_some() {
            return;
        }
        let Some((id, point, left)) = unit.squad.as_ref().and_then(|s| s.tactics.blind) else {
            return;
        };
        let alive = self
            .units
            .binary_search_by_key(&id, |u| u.id)
            .ok()
            .is_some_and(|t| self.units[t].alive());
        let squad = self.units[i].squad.as_mut().expect("checked");
        if left == 0 || !alive {
            squad.tactics.blind = None;
            return;
        }
        squad.tactics.blind = Some((id, point, left - 1));
        self.blind_fire(i, point);
    }

    /// Whether squad `i` bounds now, and with how many teams: with fire at
    /// will, no attack order, on its way somewhere, and on attack-move (two
    /// teams) or in contact.
    fn wants_bounds(&self, i: usize) -> Option<u8> {
        let unit = &self.units[i];
        let squad = unit.squad.as_ref()?;
        if squad.garrison.is_some()
            || unit.combat.fire_mode == FireMode::HoldFire
            || unit.combat.attack.is_some()
            || unit.route == Route::None
        {
            return None;
        }
        let teams = self.team_count(i);
        if unit.drill.advancing() {
            // Single teams on attack-move halt to fight (prompt 046).
            return (teams == 2).then_some(2);
        }
        (squad.tactics.contact > 0).then_some(teams)
    }

    /// One tick of the bound of squad `i`.
    fn bound_step(&mut self, i: usize, teams: u8) {
        let position = self.units[i].position;
        let squad = self.units[i].squad.as_ref().expect("checked");
        let current = squad.tactics.bound.filter(|b| b.teams == teams);
        let pinned = squad.tactics.pinned;
        let mut b = match current {
            Some(b) => b,
            None => {
                // The squad halts first: the base of fire takes cover and
                // opens fire (a single team returns fire), then the
                // maneuver team makes the first bound.
                let b = Bound {
                    teams,
                    moving: 0,
                    last: position,
                    moved: Fixed::ZERO,
                    advance: Fixed::ZERO,
                    halted: true,
                    wait: 0,
                    settled: 0,
                };
                self.set_bound(i, b);
                self.bound_spots(i, b);
                b
            }
        };
        let following = matches!(self.units[i].route, Route::Following { .. });
        if pinned || !following {
            // Gone to ground (or waiting for a path): nobody moves on.
            b.last = position;
            if pinned {
                self.units[i].speed_pct = 0;
            }
            self.set_bound(i, b);
            return;
        }
        if !b.halted {
            b.moved += (position - b.last).length();
            b.last = position;
            if b.moved >= b.advance {
                b.halted = true;
                b.wait = 0;
                b.settled = 0;
                self.set_bound(i, b);
                self.bound_spots(i, b);
                self.units[i].speed_pct = 0;
            } else {
                self.set_bound(i, b);
            }
            return;
        }
        self.units[i].speed_pct = 0;
        b.last = position;
        b.wait += 1;
        let done = if teams == 2 {
            if self.team_arrived(i, b) {
                b.settled += 1;
            }
            b.settled >= self.tactics.settle
        } else {
            b.wait >= self.tactics.overwatch
        };
        if (done || b.wait >= self.tactics.timeout) && self.may_bound(i) {
            // The first bound starts from the formation: the holding team
            // stands at the squad position, so it is short.
            let advance = if b.advance == Fixed::ZERO {
                mm(self.tactics.rules.fire_teams.bound_min_mm)
            } else {
                self.next_advance(i)
            };
            b = Bound {
                teams,
                moving: if teams == 2 { 1 - b.moving } else { 0 },
                last: position,
                moved: Fixed::ZERO,
                advance,
                halted: false,
                wait: 0,
                settled: 0,
            };
        }
        self.set_bound(i, b);
    }

    /// Whether squad `i` may start its next bound: always on a plain move
    /// (the player wants it there); on attack-move only without a target or
    /// once its target squad is suppressed (`advance_at`): the base of fire
    /// keeps the enemy's heads down, then the other team moves.
    fn may_bound(&self, i: usize) -> bool {
        let unit = &self.units[i];
        if !unit.drill.advancing() {
            return true;
        }
        let Some(target) = unit
            .combat
            .auto_target
            .and_then(|id| self.units.binary_search_by_key(&id, |u| u.id).ok())
        else {
            return true;
        };
        self.units[target]
            .squad
            .as_ref()
            .is_some_and(|s| s.tactics.points() >= self.tactics.rules.fire_teams.advance_at)
    }

    fn set_bound(&mut self, i: usize, b: Bound) {
        if let Some(s) = &mut self.units[i].squad {
            s.tactics.bound = Some(b);
        }
    }

    /// The squad position halted: the moving team (all soldiers of a
    /// single team) take cover spots near their places.
    fn bound_spots(&mut self, i: usize, b: Bound) {
        let squad = self.units[i].squad.as_ref().expect("checked");
        let n = squad.soldiers.len();
        let spacing = mm(self.infantry.rules.formation.spacing_mm);
        let places: Vec<Option<(Fixed, Fixed)>> = (0..n)
            .map(|k| {
                if b.teams < 2 {
                    Some(self.formation_place(i, k))
                } else if team_of(k, n) == b.moving {
                    let (f, r) = line_place(k, n, spacing);
                    Some((f + b.advance / 2, r))
                } else {
                    None
                }
            })
            .collect();
        self.choose_spots_for(i, &places);
    }

    /// Whether every soldier of the moving team who can move stands at his
    /// cover spot or in his line.
    fn team_arrived(&self, i: usize, b: Bound) -> bool {
        let unit = &self.units[i];
        let squad = unit.squad.as_ref().expect("checked");
        let n = squad.soldiers.len();
        let spacing = mm(self.infantry.rules.formation.spacing_mm);
        let right = Vec2::new(-unit.facing.y, unit.facing.x);
        let near = Fixed::ONE / 2;
        squad.soldiers.iter().enumerate().all(|(k, s)| {
            if !s.alive() || s.knocked > 0 || team_of(k, n) != b.moving {
                return true;
            }
            if s.has_spot() {
                return s.at_spot();
            }
            let (f, r) = line_place(k, n, spacing);
            let place = unit.position + unit.facing.scale(f + b.advance / 2) + right.scale(r);
            (s.position - place).length() < near
        })
    }

    /// How far squad `i` advances in its next bound: between
    /// `bound_min_mm` and `bound_max_mm`, where the moving team (half a
    /// bound ahead of the squad position) finds the most cover; the rest of
    /// the route if that is shorter.
    fn next_advance(&self, i: usize) -> Fixed {
        let f = &self.tactics.rules.fire_teams;
        let (lo, hi, step) = (mm(f.bound_min_mm), mm(f.bound_max_mm), mm(f.sample_mm));
        let unit = &self.units[i];
        let Route::Following {
            waypoints, next, ..
        } = &unit.route
        else {
            return lo;
        };
        let mut points = Vec::with_capacity(waypoints.len() + 1 - *next);
        points.push(unit.position);
        points.extend_from_slice(&waypoints[*next..]);
        let remaining: Fixed = points.windows(2).map(|w| (w[1] - w[0]).length()).sum();
        if remaining <= lo + lo / 2 {
            return remaining + Fixed::ONE;
        }
        let along = |d: Fixed| {
            let mut left = d;
            for w in points.windows(2) {
                let l = (w[1] - w[0]).length();
                if left <= l && l > Fixed::ZERO {
                    return w[0] + (w[1] - w[0]).scale(left / l);
                }
                left -= l;
            }
            *points.last().expect("not empty")
        };
        let radius = mm(f.cover_radius_mm);
        let mut best: Option<(u32, Fixed)> = None;
        let mut d = lo;
        while d <= hi && d + d / 2 <= remaining {
            let score = self.cover_spots_near(along(d + d / 2), radius);
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, d));
            }
            d += step.max(Fixed::ONE);
        }
        match best {
            Some((score, d)) if score > 0 => d,
            _ => (lo + hi) / 2,
        }
    }

    /// Adds `points` of suppression to squad `j` (blasts of shells and
    /// bombs; rounds and hits go through [`TacticsRt::near_miss`] and
    /// [`TacticsRt::hit`]).
    pub(super) fn suppress_blast(&mut self, j: usize) {
        if let Some(s) = &mut self.units[j].squad {
            self.tactics.blast(s);
        }
    }

    /// Tactical state of a squad (`None` for vehicles and unknown ids).
    pub fn squad_tactics(&self, id: UnitId) -> Option<SquadTactics> {
        let index = self.units.binary_search_by_key(&id, |u| u.id).ok()?;
        let t = &self.units[index].squad.as_ref()?.tactics;
        Some(SquadTactics {
            suppression: t.points(),
            pinned: t.pinned,
            bounding: t.bound.map(|b| b.moving),
            blind_fire: t.blind_now,
            close: t.close,
        })
    }

    /// Flags of every soldier of squad `id` ([`soldier_flags`]), by
    /// soldier index; empty for vehicles and unknown ids.
    pub fn soldier_tactics(&self, id: UnitId) -> Vec<u8> {
        let Ok(index) = self.units.binary_search_by_key(&id, |u| u.id) else {
            return Vec::new();
        };
        let Some(squad) = &self.units[index].squad else {
            return Vec::new();
        };
        let n = squad.soldiers.len();
        (0..n)
            .map(|k| {
                let mut f = 0;
                if squad.tactics.jammed.get(k).is_some_and(|&j| j > 0) {
                    f |= soldier_flags::JAMMED;
                }
                if team_of(k, n) == 1 {
                    f |= soldier_flags::MANEUVER;
                }
                if let Some(b) = squad.tactics.bound
                    && b.teams == 2
                    && team_of(k, n) == b.moving
                    && !may_fire(squad, k)
                    && squad.tactics.jammed.get(k).is_none_or(|&j| j == 0)
                {
                    f |= soldier_flags::RUSHING;
                }
                f
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teams_split_the_roster_in_halves() {
        // German rifle squad: leader, MG 1, MG 2, ammunition bearer are
        // the base of fire; the five riflemen maneuver.
        let teams: Vec<u8> = (0..9).map(|k| team_of(k, 9)).collect();
        assert_eq!(teams, [0, 0, 0, 0, 1, 1, 1, 1, 1]);
        let spacing = Fixed::from_num(3);
        // Team lines are centered on the squad.
        assert_eq!(line_place(0, 9, spacing).1, Fixed::from_num(-4.5));
        assert_eq!(line_place(3, 9, spacing).1, Fixed::from_num(4.5));
        assert_eq!(line_place(4, 9, spacing).1, Fixed::from_num(-6));
        assert_eq!(line_place(6, 9, spacing).1, Fixed::ZERO);
    }

    #[test]
    fn ticks_apply_the_time_factor() {
        assert_eq!(ticks(1000, 20, (2, 1)), 10);
        assert_eq!(ticks(5000, 20, (0, 0)), 100);
    }
}
