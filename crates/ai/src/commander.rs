//! The commander (prompt 051, Normal and Hard): leads the army as one force
//! against an enemy front instead of letting every unit drive into range on
//! its own.
//!
//! - **Stage:** with a front known (fresh sightings, `intel.rs`) and the
//!   army still out of its reach, the army gathers on a staging point just
//!   outside the front's weapon range, on the axis that shows the enemy's
//!   flank (hull directions seen), avoids ground where own units were lost
//!   and axes that failed before. The way there runs round the front, not
//!   through its range.
//! - **Assault:** gathered (or after a while), the army drives in a line
//!   formation with attack-move to its own range, so every gun arrives at
//!   once; then the fight is the focus fire of prompt 024.
//! - **Withdraw:** clearly outmatched in the fight (strength below a share
//!   of the front's after own losses), the army pulls back out of reach and
//!   attacks again over another axis.
//! - Infantry follows the armor (behind it) and fights enemy infantry.
//!
//! Only commands (`FormationMove`, `MoveTo`, `Attack` through the group
//! logic) and only what the player's units saw.

use crate::intel::Front;
use crate::{Ai, GroupState, OwnUnit, PlayerView, Role, SeenUnit, centroid, dist};
use serde::{Deserialize, Serialize};
use sim::{Command, Engage, Fixed, Formation, UnitId, Vec2};

/// The staging point lies this far outside the front's weapon range (m).
const STAGE_MARGIN_M: i32 = 70;
/// The army waits at the staging point at most this long (ticks).
const STAGE_WAIT_TICKS: u64 = 600;
/// A stage (way there plus waiting) is given up after this long: assault.
const STAGE_MAX_TICKS: u64 = 3600;
/// The assault drives to this share of the own range from the front.
const ASSAULT_RANGE_PCT: i64 = 60;
/// The way round the front: at most this many degrees per leg.
const ARC_STEP_DEG: i32 = 45;
/// Withdraw this far beyond the front's reach (m).
const WITHDRAW_M: i32 = 150;
/// A withdrawal ends after this long at the latest (ticks).
const WITHDRAW_TICKS: u64 = 900;
/// Followers (infantry behind armor) keep this far behind the lead (m).
const FOLLOW_M: i32 = 50;
/// Armor closes in to this share of its range before it fires (percent).
const CLOSE_PCT: i64 = 45;
/// ... but only when it stands farther than this share of its range.
const CLOSE_FROM_PCT: i64 = 60;
/// Armor that outranges the enemy fights from this share of its range.
const STANDOFF_PCT: i64 = 85;
/// With no anti-tank weapon seen, armor closes in this far (m).
const OVERRUN_M: i32 = 60;
/// An assault that arrived without contact creeps this much closer after
/// this long (m, ticks).
const CREEP_M: i32 = 60;
const CREEP_TICKS: u64 = 200;
/// The front is sampled every `WATCH_TICKS`; closer by `ADVANCE_M` means
/// it advances, for `ADVANCING_TICKS`.
const WATCH_TICKS: u64 = 60;
const ADVANCE_M: i32 = 12;
const ADVANCING_TICKS: u64 = 300;
/// An assault without contact for this long hands over to the group
/// logic for `PUSH_TICKS`.
const ASSAULT_MAX_TICKS: u64 = 1200;
const PUSH_TICKS: u64 = 2400;
/// An enemy seen moving counts as maneuvering for this long (ticks).
const ACTIVE_TICKS: u64 = 2400;
/// A fight is reviewed after this long (ticks).
const REVIEW_TICKS: u64 = 300;
/// Below this share (percent) of damage dealt to damage taken the army
/// moves round to the enemy's flank.
const EXCHANGE_PCT: u64 = 50;
/// A front within this distance of the last target is the same one (m).
const SAME_FRONT_M: i32 = 250;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Phase {
    /// No front known: the group logic advances to the objective.
    #[default]
    Free,
    /// Way to the staging point and waiting there.
    Stage,
    /// Line attack from the staging point.
    Assault,
    /// Enemies in range: focus fire (group logic).
    Fight,
    /// Pulling back out of reach after a lost fight.
    Withdraw,
    /// The enemy advances: waiting for it in line.
    Hold,
}

/// The army's plan against one front.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct ArmyPlan {
    pub(crate) phase: Phase,
    /// Tick the phase began.
    since: u64,
    /// Center of the front when it was chosen.
    target: Option<Vec2>,
    /// Unit vector from the front towards the side the army attacks from.
    axis: Option<Vec2>,
    /// Axes of attacks that were given up.
    failed: Vec<Vec2>,
    /// Strength of the lead group and score of the known enemies when the
    /// fight (or its last review) began.
    fight_value: u64,
    fight_enemy: u64,
    /// The staging point and the last point the lead was sent to.
    stage: Option<Vec2>,
    sent: Option<(Vec2, u64)>,
    /// Last sample of the front's center, and until when the enemy counts
    /// as advancing.
    watch: Option<(Vec2, u64)>,
    advancing_until: u64,
    /// Until then the group logic leads (after an assault without
    /// contact).
    push_until: u64,
    /// Distance of the staging point from the front.
    reach: Option<Fixed>,
    /// Distance of the assault after creeping closer.
    creep: Option<Fixed>,
}

/// `v` turned by `deg` degrees (multiples of 15 only; counter-clockwise).
fn rotate(v: Vec2, deg: i32) -> Vec2 {
    // cos and sin of 0, 15, …, 90 degrees in thousandths.
    const COS: [i64; 7] = [1000, 966, 866, 707, 500, 259, 0];
    let d = deg.rem_euclid(360);
    let quadrant = d / 90;
    let rest = usize::try_from((d % 90) / 15).unwrap_or(0);
    let (c, s) = (
        Fixed::from_num(COS[rest]) / 1000,
        Fixed::from_num(COS[6 - rest]) / 1000,
    );
    let mut r = Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c);
    for _ in 0..quadrant {
        r = Vec2::new(-r.y, r.x);
    }
    r
}

fn unit_vec(v: Vec2) -> Option<Vec2> {
    let len = v.length();
    (len > Fixed::ZERO).then(|| v.scale(Fixed::ONE / len))
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

/// Angle in degrees (0..180) between two unit vectors, in steps of the
/// cosine table (coarse, enough for choosing axes).
fn angle_deg(a: Vec2, b: Vec2) -> i32 {
    let c = (dot(a, b) * 1000).to_num::<i64>();
    const STEPS: [(i64, i32); 7] = [
        (966, 0),
        (866, 15),
        (707, 30),
        (500, 45),
        (259, 60),
        (0, 75),
        (-259, 90),
    ];
    for (limit, deg) in STEPS {
        if c >= limit {
            return deg;
        }
    }
    if c >= -707 { 120 } else { 180 }
}

impl Ai {
    /// The lead role of the army: armor while there is any, else infantry.
    fn lead_role(&self, view: &PlayerView) -> Role {
        let armor = view
            .own
            .iter()
            .any(|u| self.effective(u) && self.type_info(u.unit_type).role == Role::Armor);
        if armor { Role::Armor } else { Role::Infantry }
    }

    /// A unit that can still fight and move: alive, with ammunition, not
    /// immobilized, not riding.
    pub(crate) fn effective(&self, u: &OwnUnit) -> bool {
        u.alive() && u.stock.ammo_pct != Some(0) && !u.stock.immobile && u.carrier.is_none()
    }

    /// Strength of `units`: value times hit point share.
    fn strength(&self, units: &[OwnUnit]) -> u64 {
        units
            .iter()
            .map(|u| {
                let pct = u64::from(u.hp) * 100 / u64::from(u.max_hp.max(1));
                u64::from(self.value_of(u.unit_type)) * pct / 100
            })
            .sum()
    }

    /// The front the army deals with: the one near the last target, else
    /// the nearest to the army.
    fn current_front(&self, center: Vec2, view: &PlayerView) -> Option<Front> {
        let fronts = self.fronts(view.tick);
        if let Some(t) = self.army.target
            && let Some(f) = fronts
                .iter()
                .filter(|f| dist(f.center, t) <= Fixed::from_num(SAME_FRONT_M))
                .min_by_key(|f| dist(f.center, t))
        {
            return Some(f.clone());
        }
        fronts.into_iter().min_by_key(|f| dist(f.center, center))
    }

    /// Whether a point is open ground the army can stand on.
    pub(crate) fn standable(&self, p: Vec2) -> bool {
        let m = Fixed::from_num(30);
        if p.x.abs() > self.half.x - m || p.y.abs() > self.half.y - m {
            return false;
        }
        self.ground.passable(p)
    }

    /// The axis to attack `front` from (unit vector from the front towards
    /// the staging point): the enemy's flank or rear if its hulls were
    /// seen, little danger at the staging point, a short way round, and
    /// not an axis that failed.
    /// On small maps the staging point may lie closer than `reach` (down
    /// to 90 % of it) if the full distance leaves the map; the returned
    /// distance is the one to use.
    fn choose_axis(&mut self, front: &Front, from: Vec2, reach: Fixed, tick: u64) -> (Vec2, Fixed) {
        let direct = unit_vec(from - front.center).unwrap_or(Vec2::new(Fixed::ZERO, Fixed::ONE));
        let degrees: &[i32] = if self.difficulty == crate::Difficulty::Hard {
            &[0, 30, -30, 60, -60, 90, -90]
        } else {
            &[0, 45, -45]
        };
        let mut best: Option<(i64, Vec2, Fixed)> = None;
        for &deg in degrees {
            let dir = rotate(direct, deg);
            let Some(shrink) =
                (0..2).find(|&k| self.standable(front.center + dir.scale(reach * (10 - k) / 10)))
            else {
                continue;
            };
            let r = reach * (10 - shrink) / 10;
            let stage = front.center + dir.scale(r);
            let mut score: i64 = -15 * shrink;
            // Hulls towards us cost, flanks and rear gain (-100..100).
            if front.facing != Vec2::ZERO {
                score -= (dot(front.facing, dir) * 100).to_num::<i64>();
            }
            score -= i64::try_from(self.danger_at(stage, tick, true) / 10).unwrap_or(i64::MAX);
            score -= i64::from(deg.abs()) / 2;
            if self.army.failed.iter().any(|&f| angle_deg(f, dir) <= 30) {
                score -= 150;
            }
            if best.is_none_or(|(b, _, _)| score > b) {
                best = Some((score, dir, r));
            }
        }
        best.map_or((direct, reach), |(_, d, r)| (d, r))
    }

    /// The range from which the enemy can hurt `role` near `front`: what
    /// was seen there, at least what the enemy was ever seen with, and
    /// before any armor-killer was seen half again the own range.
    fn reach_of(&self, front: &Front, role: Role, range: Fixed) -> Fixed {
        let armor = role == Role::Armor;
        let mut known = Fixed::ZERO;
        for (_, s) in &self.intel.known {
            let info = self.type_info(s.unit_type);
            if info.role != Role::Structure && (!armor || info.anti_tank) {
                known = known.max(info.range);
            }
        }
        if known == Fixed::ZERO && !self.intel.seen_any {
            known = (range * 3 / 2).max(Fixed::from_num(250));
        }
        let seen = if armor {
            front.threat_armor
        } else {
            front.threat_any
        };
        seen.max(known)
    }

    /// With guns that reach clearly farther than the front's anti-tank
    /// weapons, armor fights from this distance instead of closing in.
    fn standoff(&self, front: &Front, range: Fixed) -> Option<Fixed> {
        let threat = self.reach_of(front, Role::Armor, range);
        let standoff = range * STANDOFF_PCT / 100;
        (threat > Fixed::ZERO && standoff > threat + Fixed::from_num(30)).then_some(standoff)
    }

    /// The enemy's home as a front while nothing was seen: the army
    /// stages on its flank instead of driving straight at it.
    fn home_front(&self) -> Option<Front> {
        if self.enemy_home_searched {
            return None;
        }
        Some(Front {
            center: self.enemy_home,
            value: 0,
            threat_armor: Fixed::ZERO,
            threat_any: Fixed::ZERO,
            facing: unit_vec(self.home - self.enemy_home).unwrap_or(Vec2::ZERO),
            mobile: 0,
            members: Vec::new(),
        })
    }

    /// Value of the known enemies, hit points counted (the fight's score).
    fn enemy_score(&self) -> u64 {
        self.intel
            .known
            .iter()
            .map(|(_, s)| {
                let pct = u64::from(s.hp) * 100 / u64::from(s.max_hp.max(1));
                u64::from(self.value_of(s.unit_type)) * pct / 100
            })
            .sum()
    }

    /// Runs the commander for a group of `role`. Returns `true` if it gave
    /// the group its orders this think, `false` to leave it to the group
    /// logic (no front, or fighting).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn command_army(
        &mut self,
        role: Role,
        units: &[OwnUnit],
        center: Vec2,
        range: Fixed,
        near: &[&SeenUnit],
        view: &PlayerView,
        state: &mut GroupState,
        out: &mut Vec<Command>,
    ) -> bool {
        let lead = self.lead_role(view);
        if role != lead {
            return self.follow_lead(units, center, range, near, view, state, out);
        }
        let tick = view.tick;
        let able: Vec<OwnUnit> = units
            .iter()
            .filter(|u| self.effective(u))
            .copied()
            .collect();
        if able.is_empty() {
            return false;
        }
        let units = &able[..];
        let Some(center) = centroid(units.iter().map(|u| u.position)) else {
            return false;
        };
        let found = self.current_front(center, view);
        let virtual_front = found.is_none();
        let Some(front) = found.or_else(|| self.home_front()) else {
            self.army.phase = Phase::Free;
            self.army.target = None;
            self.army.axis = None;
            return false;
        };
        // An assault that found nothing to shoot for long pushes on with
        // the group logic (attack-move to the sightings, buildings too).
        if tick < self.army.push_until {
            return false;
        }
        let threat = self.reach_of(&front, role, range);
        let d = dist(center, front.center);
        // Base matches: the group logic holds the army (and defends) until
        // its wave is complete.

        if Self::has_base(view) && self.wave_waiting(role, units.len(), view) {
            return false;
        }
        if self
            .army
            .target
            .is_none_or(|t| dist(t, front.center) > Fixed::from_num(SAME_FRONT_M))
        {
            self.army.axis = None;
            if !matches!(self.army.phase, Phase::Withdraw) {
                self.army.phase = Phase::Free;
            }
        }
        self.army.target = Some(front.center);
        let advancing = !virtual_front && self.watch_front(&front, center, tick);
        let in_range = near
            .iter()
            .any(|e| dist(e.position, center) <= range + Fixed::from_num(30));
        let ids: Vec<UnitId> = units.iter().map(|u| u.id).collect();

        // Against an enemy that maneuvers (it was seen moving within the
        // last two minutes) the army does not go round it: it receives an
        // advance in line and otherwise advances with the group logic
        // (attack-move). Staged flank attacks are for static defenses.
        let active = self.enemy_active(tick);
        if active
            && !in_range
            && matches!(
                self.army.phase,
                Phase::Free | Phase::Stage | Phase::Assault | Phase::Hold
            )
        {
            if advancing {
                self.army.phase = Phase::Hold;
                let facing = Some(front.center - center);
                self.send(
                    &ids,
                    center,
                    facing,
                    Formation::Line,
                    Engage::All,
                    tick,
                    out,
                );
                return true;
            }
            self.army.phase = Phase::Free;
            self.army.axis = None;
            return false;
        }

        match self.army.phase {
            Phase::Withdraw => {
                let back = threat + Fixed::from_num(WITHDRAW_M);
                if d >= back - Fixed::from_num(40) || tick > self.army.since + WITHDRAW_TICKS {
                    self.army.phase = Phase::Free;
                    self.army.axis = None;
                } else {
                    return true;
                }
            }
            Phase::Fight => {
                if !near.is_empty() || in_range {
                    return self
                        .review_fight(role, units, center, range, near, &front, threat, tick, out);
                }
                // The fight moved out of sight: plan again.
                self.army.phase = Phase::Free;
                self.army.axis = None;
            }
            _ => {}
        }
        // The enemy comes on: receive it in line where the army stands
        // instead of driving into its fire (prompt 051).
        if advancing && !in_range && self.army.phase != Phase::Withdraw {
            self.army.phase = Phase::Hold;
            let facing = Some(front.center - center);
            self.send(
                &ids,
                center,
                facing,
                Formation::Line,
                Engage::All,
                tick,
                out,
            );
            return true;
        }
        if self.army.phase == Phase::Hold {
            self.army.phase = Phase::Free;
            self.army.axis = None;
        }
        if in_range {
            self.start_fight(units, tick);
            if role == Role::Armor && self.close_in(&ids, center, range, near, &front, tick, out) {
                return true;
            }
            return false;
        }
        // Nothing seen at the enemy's home: searched.
        if virtual_front && d <= range {
            self.enemy_home_searched_at(front.center, view);
            self.army.phase = Phase::Free;
            self.army.axis = None;
            return false;
        }
        let standoff = if role == Role::Armor {
            self.standoff(&front, range)
        } else {
            None
        };
        let reach = standoff.unwrap_or(threat + Fixed::from_num(STAGE_MARGIN_M));
        if self.army.axis.is_none() {
            let (axis, r) = self.choose_axis(&front, center, reach, tick);
            self.army.axis = Some(axis);
            self.army.reach = Some(r);
            self.army.phase = Phase::Stage;
            self.army.since = tick;
            self.army.sent = None;
        }
        let axis = self.army.axis.unwrap_or(Vec2::ZERO);
        let reach = self.army.reach.unwrap_or(reach);
        let stage = self.clamp_to_map(front.center + axis.scale(reach));
        self.army.stage = Some(stage);
        let facing = Some(front.center - stage);

        // Inside the front's reach without a target in range: no time to
        // gather, close in.
        let inside = d < threat;
        if self.army.phase == Phase::Stage && !inside {
            let n = i32::try_from(units.len()).unwrap_or(i32::MAX);
            let tolerance = Fixed::from_num(40 + 6 * n);
            let gathered = units.iter().all(|u| dist(u.position, stage) <= tolerance);
            let near_stage = dist(center, stage) <= tolerance;
            let waited = near_stage && tick > self.army.since + STAGE_WAIT_TICKS;
            let too_long = tick > self.army.since + STAGE_MAX_TICKS;
            if !(gathered || waited || too_long) {
                if near_stage {
                    // Arrived: the waiting time counts from here.
                    if self.army.sent.is_some_and(|(p, _)| p != stage) {
                        self.army.since = tick;
                    }
                    self.send(&ids, stage, facing, Formation::Line, Engage::All, tick, out);
                } else {
                    let leg = self.arc_leg(center, front.center, axis, reach);
                    self.send(&ids, leg, None, Formation::Auto, Engage::All, tick, out);
                }
                return true;
            }
            self.army.phase = Phase::Assault;
            self.army.since = tick;
            self.army.sent = None;
        }
        // Assault: in line to the own range; closer step by step while
        // nothing comes into sight there (woods, houses).
        if self.army.phase != Phase::Assault {
            self.army.creep = None;
            self.army.since = tick;
        } else if tick > self.army.since + ASSAULT_MAX_TICKS {
            self.army.push_until = tick + PUSH_TICKS;
            self.army.phase = Phase::Free;
            self.army.axis = None;
            return false;
        }
        self.army.phase = Phase::Assault;
        let mut close = if role == Role::Armor && threat == Fixed::ZERO {
            Fixed::from_num(OVERRUN_M)
        } else {
            standoff.unwrap_or(range * ASSAULT_RANGE_PCT / 100)
        };
        if let Some(c) = self.army.creep {
            close = close.min(c);
        }
        if let Some((goal, sent)) = self.army.sent
            && dist(center, goal) <= Fixed::from_num(40)
            && tick > sent + CREEP_TICKS
        {
            let next = (d - Fixed::from_num(CREEP_M)).max(range * 3 / 10);
            close = close.min(next);
            self.army.creep = Some(close);
        }
        let dir = unit_vec(center - front.center).unwrap_or(axis);
        let goal = self.clamp_to_map(front.center + dir.scale(close));
        self.send(
            &ids,
            goal,
            Some(front.center - goal),
            Formation::Line,
            Engage::All,
            tick,
            out,
        );
        true
    }

    /// The own armed units are at least twice as strong as every enemy unit
    /// known (prompt 051): base matches attack with everything.
    pub(crate) fn superior(&self, view: &PlayerView) -> bool {
        let own: u64 = view
            .own
            .iter()
            .filter(|u| self.effective(u))
            .filter(|u| {
                matches!(
                    self.type_info(u.unit_type).role,
                    Role::Armor | Role::Infantry
                )
            })
            .map(|u| {
                let pct = u64::from(u.hp) * 100 / u64::from(u.max_hp.max(1));
                u64::from(self.value_of(u.unit_type)) * pct / 100
            })
            .sum();
        let (enemy, _) = self.known_enemy(view);
        enemy > 0 && own >= 2 * enemy
    }

    /// The army began an assault just now (the artillery lays smoke).
    pub(crate) fn assault_starting(&self, tick: u64) -> bool {
        self.commander && self.army.phase == Phase::Assault && tick <= self.army.since + 200
    }

    /// The enemy was seen moving within `ACTIVE_TICKS`.
    fn enemy_active(&self, tick: u64) -> bool {
        self.intel
            .moved_tick
            .is_some_and(|t| tick < t + ACTIVE_TICKS)
    }

    /// Samples the front every `WATCH_TICKS`: it advances if it came
    /// `ADVANCE_M` closer to the army since the last sample.
    fn watch_front(&mut self, front: &Front, center: Vec2, tick: u64) -> bool {
        match self.army.watch {
            Some((p, t)) if tick < t + WATCH_TICKS => {
                let _ = p;
            }
            Some((p, _)) => {
                let before = dist(p, center);
                let now = dist(front.center, center);
                if before > now + Fixed::from_num(ADVANCE_M) {
                    self.army.advancing_until = tick + ADVANCING_TICKS;
                }
                self.army.watch = Some((front.center, tick));
            }
            None => self.army.watch = Some((front.center, tick)),
        }
        tick < self.army.advancing_until
    }

    fn start_fight(&mut self, units: &[OwnUnit], tick: u64) {
        self.army.phase = Phase::Fight;
        self.army.since = tick;
        self.army.fight_value = self.strength(units);
        self.army.fight_enemy = self.enemy_score();
        self.army.sent = None;
    }

    /// Every `REVIEW_TICKS` of a fight: clearly outmatched → withdraw;
    /// losing more than inflicting → move round to another side at the
    /// own range (the enemy's flank); else the group logic fights on.
    #[allow(clippy::too_many_arguments)]
    fn review_fight(
        &mut self,
        role: Role,
        units: &[OwnUnit],
        center: Vec2,
        range: Fixed,
        near: &[&SeenUnit],
        front: &Front,
        threat: Fixed,
        tick: u64,
        out: &mut Vec<Command>,
    ) -> bool {
        let ids: Vec<UnitId> = units.iter().map(|u| u.id).collect();
        // A flank move under way.
        if let Some((goal, sent)) = self.army.sent {
            let arrived = dist(center, goal) <= Fixed::from_num(40);
            if !arrived && tick < sent + 3 * REVIEW_TICKS {
                return true;
            }
            self.army.sent = None;
            self.start_fight(units, tick);
            return false;
        }
        if tick < self.army.since + REVIEW_TICKS {
            return false;
        }
        let now = self.strength(units);
        let lost = self.army.fight_value.saturating_sub(now);
        let dealt = self.army.fight_enemy.saturating_sub(self.enemy_score());
        if self.should_withdraw(now, lost, front) && dealt < lost {
            self.withdraw(&ids, center, front, threat, tick, out);
            return true;
        }
        if lost > 0 && dealt * 100 < lost * EXCHANGE_PCT && !self.enemy_active(tick) {
            // Bad exchange: try the enemy's flank at the own range.
            let dir = unit_vec(center - front.center).unwrap_or(Vec2::ZERO);
            self.army.failed.push(dir);
            let reach = range * 85 / 100;
            let (axis, reach) = self.choose_axis(front, center, reach, tick);
            let goal = self.arc_leg(center, front.center, axis, reach);
            out.push(Command::FormationMove {
                units: ids,
                target: goal,
                formation: Formation::Auto,
                facing: Some(front.center - goal),
                engage: Engage::Infantry,
            });
            self.army.axis = Some(axis);
            self.army.sent = Some((goal, tick));
            return true;
        }
        // Even or better: a new window, closer if need be.
        self.start_fight(units, tick);
        role == Role::Armor && self.close_in(&ids, center, range, near, front, tick, out)
    }

    /// Closes in on the enemy before (and between) firing: armor drives
    /// to `CLOSE_PCT` of its range from the nearest enemy armor
    /// (dispersion grows with the distance, rounds at the edge of the
    /// range miss); with no anti-tank weapon seen at the front it drives
    /// to `OVERRUN_M` of the nearest enemy, where its machine guns reach.
    #[allow(clippy::too_many_arguments)]
    fn close_in(
        &mut self,
        ids: &[UnitId],
        center: Vec2,
        range: Fixed,
        near: &[&SeenUnit],
        front: &Front,
        tick: u64,
        out: &mut Vec<Command>,
    ) -> bool {
        let armor: Vec<Vec2> = near
            .iter()
            .filter(|e| self.type_info(e.unit_type).role == Role::Armor)
            .map(|e| e.position)
            .collect();
        // Against a maneuvering or advancing enemy the army fights where it
        // stands: driving up under fire loses the exchange.
        if tick < self.army.advancing_until || self.enemy_active(tick) {
            return false;
        }
        let outranges = self.standoff(front, range).is_some();
        let (targets, close) = if !armor.is_empty() {
            if outranges {
                return false;
            }
            (armor, range * CLOSE_PCT / 100)
        } else if front.threat_armor == Fixed::ZERO {
            (
                near.iter()
                    .filter(|e| self.type_info(e.unit_type).role != Role::Structure)
                    .map(|e| e.position)
                    .collect(),
                Fixed::from_num(OVERRUN_M),
            )
        } else {
            return false;
        };
        let Some(nearest) = targets.iter().copied().min_by_key(|&p| dist(p, center)) else {
            return false;
        };
        if dist(nearest, center) <= close.max(range * CLOSE_FROM_PCT / 100) {
            return false;
        }
        let dir = unit_vec(center - nearest).unwrap_or(Vec2::ZERO);
        let goal = self.clamp_to_map(nearest + dir.scale(close));
        out.push(Command::FormationMove {
            units: ids.to_vec(),
            target: goal,
            formation: Formation::Line,
            facing: Some(nearest - goal),
            engage: Engage::Infantry,
        });
        self.army.sent = Some((goal, tick));
        true
    }

    /// The next point on the way round the front to the staging side: on
    /// the circle of `reach` around the front, at most `ARC_STEP_DEG`
    /// further towards the axis.
    fn arc_leg(&self, center: Vec2, front: Vec2, axis: Vec2, reach: Fixed) -> Vec2 {
        let Some(now) = unit_vec(center - front) else {
            return front + axis.scale(reach);
        };
        if angle_deg(now, axis) <= ARC_STEP_DEG {
            return self.clamp_to_map(front + axis.scale(reach));
        }
        // Turn towards the axis: the sign of the cross product.
        let cross = now.x * axis.y - now.y * axis.x;
        let step = if cross >= Fixed::ZERO {
            ARC_STEP_DEG
        } else {
            -ARC_STEP_DEG
        };
        let mut dir = rotate(now, step);
        // Beyond the circle when the army is still far out.
        let out_r = reach.max(dist(center, front).min(reach * 2));
        let mut p = front + dir.scale(out_r);
        if !self.standable(p) {
            dir = rotate(now, step / 3);
            p = front + dir.scale(out_r);
        }
        self.clamp_to_map(p)
    }

    /// Orders `ids` to `target` unless the same order is running.
    #[allow(clippy::too_many_arguments)]
    fn send(
        &mut self,
        ids: &[UnitId],
        target: Vec2,
        facing: Option<Vec2>,
        formation: Formation,
        engage: Engage,
        tick: u64,
        out: &mut Vec<Command>,
    ) {
        if self.army.sent.is_some_and(|(p, t)| {
            dist(p, target) <= Fixed::from_num(15) && tick < t + 4 * crate::REISSUE_TICKS
        }) {
            return;
        }
        out.push(Command::FormationMove {
            units: ids.to_vec(),
            target,
            formation,
            facing,
            engage,
        });
        self.army.sent = Some((target, tick));
    }

    /// Clearly outmatched: the group lost a good part of its strength in
    /// this fight and is weaker than the front by the difficulty's margin.
    fn should_withdraw(&self, now: u64, lost: u64, front: &Front) -> bool {
        let ratio_pct = if self.difficulty == crate::Difficulty::Hard {
            60
        } else {
            40
        };
        lost * 100 >= self.army.fight_value * 30 && now * 100 < front.value * ratio_pct
    }

    fn withdraw(
        &mut self,
        ids: &[UnitId],
        center: Vec2,
        front: &Front,
        threat: Fixed,
        tick: u64,
        out: &mut Vec<Command>,
    ) {
        let dir = unit_vec(center - front.center).unwrap_or(Vec2::new(Fixed::ZERO, Fixed::ONE));
        if let Some(axis) = self.army.axis {
            self.army.failed.push(axis);
        } else {
            self.army.failed.push(dir);
        }
        let back =
            self.clamp_to_map(front.center + dir.scale(threat + Fixed::from_num(WITHDRAW_M)));
        out.push(Command::MoveTo {
            units: ids.to_vec(),
            target: back,
        });
        self.army.phase = Phase::Withdraw;
        self.army.since = tick;
        self.army.axis = None;
        self.army.sent = Some((back, tick));
    }

    /// A group that is not the lead (infantry behind armor): fights enemies
    /// it can hurt in its range, else keeps behind the lead.
    #[allow(clippy::too_many_arguments)]
    fn follow_lead(
        &self,
        units: &[OwnUnit],
        center: Vec2,
        range: Fixed,
        near: &[&SeenUnit],
        view: &PlayerView,
        state: &mut GroupState,
        out: &mut Vec<Command>,
    ) -> bool {
        if near
            .iter()
            .any(|e| dist(e.position, center) <= range + Fixed::from_num(30))
        {
            return false;
        }
        // Without enemy armor known the infantry hunts on its own.
        let enemy_armor = self.intel.known.iter().any(|(_, s)| {
            self.type_info(s.unit_type).role == Role::Armor
                && !view.enemy_players.iter().any(|&(p, d)| p == s.owner && d)
        });
        if !enemy_armor {
            return false;
        }
        let lead: Vec<Vec2> = view
            .own
            .iter()
            .filter(|u| self.effective(u) && self.type_info(u.unit_type).role == Role::Armor)
            .map(|u| u.position)
            .collect();
        let Some(lead_center) = centroid(lead.iter().copied()) else {
            return false;
        };
        let back_dir = match (self.army.phase, self.army.axis, self.army.target) {
            (Phase::Free, _, _) | (_, None, None) => unit_vec(self.home - lead_center),
            (_, Some(axis), _) => Some(axis),
            (_, None, Some(t)) => unit_vec(lead_center - t),
        }
        .unwrap_or(Vec2::ZERO);
        let point = self.clamp_to_map(lead_center + back_dir.scale(Fixed::from_num(FOLLOW_M)));
        let ids: Vec<UnitId> = units.iter().map(|u| u.id).collect();
        if dist(center, point) > Fixed::from_num(30) {
            let idle = units.iter().all(|u| u.target.is_none());
            let stale = view.tick >= state.issued + crate::REISSUE_TICKS;
            self.move_group(
                state,
                &ids,
                (point, unit_vec(lead_center - point)),
                stale && idle,
                view.tick,
                out,
            );
        }
        true
    }
}
