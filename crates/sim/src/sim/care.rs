//! Wounds, buddy aid, medics and bailed-out crews (prompt 049). Rules:
//! `crate::care` (`data/care.toml`); docs/INFANTRY.md "Wounds and buddy
//! aid".
//!
//! - **Wounds:** [`Sim::update_care`] compares every soldier's hit points
//!   with those it saw last tick. A man who lost some and lives is wounded:
//!   heavily with `heavy_pct` (always when he was wounded already), else
//!   lightly. Lightly wounded men dress the wound themselves (no firing
//!   meanwhile) and limp; heavily wounded men lie, neither move nor fire,
//!   are hit like prone men and die after `bleed_ms` unless a medic reaches
//!   them. Treated men limp for `limp_ms` and keep the bandage. The roll
//!   uses its own generator seeded from the shared one's state, so battles
//!   keep their random sequence.
//! - **Buddy aid:** a comrade drags a heavily wounded man to his cover spot
//!   while the squad stands, and carries him along (the squad slows down)
//!   while it moves; a comrade goes over to a man knocked down by a blast
//!   (prompt 048) and pulls him up when the knock-down time is over.
//! - **Medics:** an idle medic team goes to the nearest untreated heavily
//!   wounded man of its side, stops the bleeding, carries him on a
//!   stretcher to its post (teams of two or more) and treats him; he then
//!   limps back to his squad.
//! - **Crews:** survivors of a destroyed vehicle bail out beside the wreck
//!   as a squad of their faction's tank crew type. Crews do not keep a
//!   player in the match.
//!
//! Order within a tick (see [`Sim::step`]): [`Sim::care_speed`] right after
//! the stances (it caps the squad speed), [`Sim::update_care`] after the
//! economy, when all hits and destructions of the tick are known.

use super::infantry::Soldier;
use super::{Route, Sim, Unit, stop, transport};
use crate::care::CareRules;
use crate::catalog::Category;
use crate::combat::{CombatEvent, FireMode};
use crate::command::UnitId;
use crate::data::UnitType;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Wound state of a soldier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum WoundState {
    #[default]
    Fit,
    /// Dressing his wound (`left` ticks), then limping.
    Light,
    /// Down; dies when `left` runs out unless `stable`.
    Heavy,
    /// Bandaged; limps while `left` runs.
    Treated,
    /// Never there: a crew member who did not get out of the vehicle.
    Gone,
}

/// What a soldier does for a comrade of his squad.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Task {
    #[default]
    None,
    /// Drags or carries the heavily wounded soldier with this index.
    Aid(u16),
    /// Pulls up the knocked-down soldier with this index.
    HelpUp(u16),
}

/// Care state of one soldier (part of `Soldier`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Wound {
    pub(super) state: WoundState,
    /// Ticks left of the state (see [`WoundState`]).
    pub(super) left: u32,
    /// Heavily wounded and the bleeding stopped (a medic reached him).
    pub(super) stable: bool,
    /// Hit points seen at the last update (0: not seen yet, or dead).
    pub(super) seen_hp: u32,
    pub(super) task: Task,
    /// The care update places him (lying, dragged, carried, helping);
    /// the formation update leaves him alone.
    pub(super) held: bool,
    /// Being dragged by a comrade right now (presentation).
    pub(super) dragged: bool,
}

impl Wound {
    /// Whether he may fire: not wounded beyond a bandage and not busy
    /// with a comrade.
    pub(super) fn can_fire(&self) -> bool {
        matches!(self.state, WoundState::Fit | WoundState::Treated) && self.task == Task::None
    }

    /// Lying: hit like a prone soldier.
    pub(super) fn lying(&self) -> bool {
        self.state == WoundState::Heavy
    }

    fn limping(&self) -> bool {
        self.state == WoundState::Light || (self.state == WoundState::Treated && self.left > 0)
    }

    /// Able to help: not wounded beyond a bandage, not busy.
    fn able(&self) -> bool {
        matches!(self.state, WoundState::Fit | WoundState::Treated) && self.task == Task::None
    }
}

/// Wound state of a soldier for the presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WoundView {
    #[default]
    Fit,
    /// Lightly wounded, dressing the wound.
    Bandaging,
    /// Heavily wounded, lying (`stable`: the bleeding is stopped).
    Down { stable: bool },
    /// Treated, limping, bandage visible.
    Limping,
    /// Treated and recovered, bandage visible.
    Bandaged,
    /// Not there (a crew member who did not get out).
    Gone,
}

/// What a soldier does or what is done with him, for the presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AidView {
    #[default]
    None,
    /// Walking over to, or (`pulling`) dragging, comrade `soldier`.
    Drag { soldier: u16, pulling: bool },
    /// Dragged by a comrade.
    Dragged,
    /// Pulling up knocked-down comrade `soldier`.
    HelpUp { soldier: u16 },
    /// A medic treating soldier `soldier` of unit `unit`.
    Treat { unit: UnitId, soldier: u16 },
    /// A stretcher bearer carrying soldier `soldier` of unit `unit`
    /// (`front`: the front end).
    Stretcher {
        unit: UnitId,
        soldier: u16,
        front: bool,
    },
    /// Lying on a stretcher.
    OnStretcher,
}

/// Something care did, for the event feed and tests (not state).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CareEvent {
    Wounded {
        unit: UnitId,
        soldier: u16,
        heavy: bool,
    },
    /// A heavily wounded man died for lack of help.
    DiedOfWounds { unit: UnitId, soldier: u16 },
    /// A medic team finished treating a man.
    Treated {
        medic: UnitId,
        unit: UnitId,
        soldier: u16,
    },
    /// Survivors of destroyed vehicle `vehicle` bailed out as squad `crew`.
    BailedOut {
        vehicle: UnitId,
        crew: UnitId,
        men: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum JobPhase {
    /// Walking to the casualty.
    Going,
    /// Carrying him on the stretcher to the post.
    Carrying,
    /// Treating him (`left` ticks).
    Treating,
}

/// A medic team's work on one casualty.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Job {
    medic: UnitId,
    patient: UnitId,
    soldier: u16,
    phase: JobPhase,
    left: u32,
    /// Where the team stood when it set out (collection point).
    post: Vec2,
    /// Path requests made in this phase.
    tries: u8,
}

/// Care state of a match.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Care {
    pub(super) rules: CareRules,
    /// Rule durations in ticks (infantry time factor applied).
    bandage_ticks: u32,
    bleed_ticks: u32,
    limp_ticks: u32,
    treat_ticks: u32,
    /// Per unit type: a medic team.
    medic: Vec<bool>,
    /// Per unit type: a bailed-out crew squad.
    crew: Vec<bool>,
    /// Per unit type: the crew squad type its crew bails out as.
    crew_type: Vec<Option<u32>>,
    /// Sorted by medic id.
    jobs: Vec<Job>,
    /// Something happened: the state enters the hash from then on, so
    /// matches without wounds keep their hashes.
    active: bool,
    /// Events of the last update (not state, not hashed).
    #[serde(skip)]
    events: Vec<CareEvent>,
}

/// Ticks of `ms` real milliseconds at `tick_hz` with time factor
/// `(num, den)` (as in `sim/infantry.rs`).
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

fn pct(v: Fixed, p: u32) -> Fixed {
    v * Fixed::from_num(p) / 100
}

/// `from` moved towards `to` by at most `step`.
fn toward(from: Vec2, to: Vec2, step: Fixed) -> Vec2 {
    let d = to - from;
    let len = d.length();
    if len <= step || len == Fixed::ZERO {
        to
    } else {
        from + Vec2::new(d.x / len, d.y / len).scale(step)
    }
}

impl Care {
    pub(super) fn new(rules: CareRules, unit_types: &[UnitType], tick_hz: u32) -> Self {
        let ratio = unit_types
            .iter()
            .find(|t| t.combat.is_infantry())
            .map_or((2, 1), |t| t.combat.time_ratio);
        let crew: Vec<bool> = unit_types.iter().map(crate::care::is_crew).collect();
        let crew_type = unit_types
            .iter()
            .map(|t| {
                if !crate::care::bails_out(t) {
                    return None;
                }
                let id = crate::care::crew_id(crate::economy::faction_of(&t.name)?);
                (0u32..)
                    .zip(unit_types)
                    .find(|(_, c)| c.name == id && c.combat.is_infantry())
                    .map(|(k, _)| k)
            })
            .collect();
        Self {
            rules,
            bandage_ticks: ticks(rules.wound.bandage_ms, tick_hz, ratio),
            bleed_ticks: ticks(rules.wound.bleed_ms, tick_hz, ratio).max(1),
            limp_ticks: ticks(rules.wound.limp_ms, tick_hz, ratio),
            treat_ticks: ticks(rules.medic.treat_ms, tick_hz, ratio),
            medic: unit_types
                .iter()
                .map(|t| t.combat.category == Some(Category::InfMedic))
                .collect(),
            crew,
            crew_type,
            jobs: Vec::new(),
            active: false,
            events: Vec::new(),
        }
    }

    /// Whether unit `u` is a bailed-out crew squad.
    pub(super) fn is_crew(&self, u: &Unit) -> bool {
        self.crew.get(u.unit_type as usize).copied() == Some(true)
    }

    fn job_of(&self, medic: UnitId) -> Option<usize> {
        self.jobs.iter().position(|j| j.medic == medic)
    }

    /// The job that holds soldier `k` of unit `unit` (on the stretcher or
    /// under treatment), if any.
    fn holding(&self, unit: UnitId, k: usize) -> Option<&Job> {
        self.jobs.iter().find(|j| {
            j.patient == unit && usize::from(j.soldier) == k && j.phase != JobPhase::Going
        })
    }

    fn claimed(&self, unit: UnitId, k: usize) -> bool {
        self.jobs
            .iter()
            .any(|j| j.patient == unit && usize::from(j.soldier) == k)
    }
}

/// Whether a unit is a medic team that may take a new casualty.
fn idle(unit: &Unit) -> bool {
    unit.alive()
        && unit.route == Route::None
        && unit.target.is_none()
        && unit.orders == super::orders::OrderQueue::default()
        && unit.combat.attack.is_none()
        && unit.link.is_none()
        && unit.squad.as_ref().is_some_and(|s| s.garrison.is_none())
}

impl Sim {
    /// Whether unit `u` is a bailed-out crew squad (not counted for
    /// elimination).
    pub(super) fn is_crew_unit(&self, u: &Unit) -> bool {
        self.care.is_crew(u)
    }

    /// Caps the speed of squads that carry wounded men or limp, and of
    /// medic teams carrying a stretcher. After the stances.
    pub(super) fn care_speed(&mut self) {
        if !self.care.active {
            return;
        }
        let w = self.care.rules.wound;
        let stretcher = self.care.rules.medic.stretcher_speed_pct;
        for unit in &mut self.units {
            let Some(s) = &unit.squad else { continue };
            if !unit.alive() {
                continue;
            }
            let (mut down, mut able, mut limp) = (0u32, 0u32, false);
            for (k, x) in s.soldiers.iter().enumerate() {
                if !x.alive() {
                    continue;
                }
                match x.wound.state {
                    WoundState::Heavy if self.care.holding(unit.id, k).is_none() => down += 1,
                    WoundState::Fit | WoundState::Treated if x.knocked == 0 => able += 1,
                    _ => {}
                }
                limp |= x.wound.limping();
            }
            let mut cap = 100;
            if limp {
                cap = cap.min(w.limp_speed_pct);
            }
            if down > 0 {
                cap = if able == 0 {
                    0
                } else {
                    cap.min(w.carry_speed_pct)
                };
            }
            if self
                .care
                .job_of(unit.id)
                .is_some_and(|j| self.care.jobs[j].phase == JobPhase::Carrying)
            {
                cap = cap.min(stretcher);
            }
            if cap < 100 {
                unit.speed_pct = unit.speed_pct * cap / 100;
            }
        }
    }

    /// Wounds, buddy aid, medics and crews; after the economy.
    pub(super) fn update_care(&mut self) {
        self.care.events.clear();
        self.bail_out_crews();
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if u.squad.is_some() && u.alive() && !transport::aboard(u) {
                self.care_squad(i);
            }
        }
        self.update_medics();
    }

    /// New wounds, wound timers and buddy aid of squad `i`.
    fn care_squad(&mut self, i: usize) {
        let w = self.care.rules.wound;
        let (bandage, bleed, limp) = (
            self.care.bandage_ticks,
            self.care.bleed_ticks,
            self.care.limp_ticks,
        );
        let id = self.units[i].id;
        let seed = self.rng.state() ^ (u64::from(id.0) << 32) ^ self.tick ^ 0x4904_9a1d_5eed_0049;
        let mut events = Vec::new();
        let mut died = false;
        let mut knocked = false;
        {
            let squad = self.units[i].squad.as_mut().expect("checked");
            for (k, s) in squad.soldiers.iter_mut().enumerate() {
                let index = u16::try_from(k).unwrap_or(u16::MAX);
                let wound = &mut s.wound;
                if wound.seen_hp == 0 {
                    wound.seen_hp = s.hp;
                } else if s.hp < wound.seen_hp && s.hp > 0 {
                    let was = wound.state;
                    let heavy = was != WoundState::Fit
                        || Rng::new(seed ^ (u64::from(index) << 16)).below(100) < w.heavy_pct;
                    if heavy && was != WoundState::Heavy {
                        wound.state = WoundState::Heavy;
                        wound.left = bleed;
                        wound.stable = false;
                    } else if !heavy {
                        wound.state = WoundState::Light;
                        wound.left = bandage;
                    }
                    events.push(CareEvent::Wounded {
                        unit: id,
                        soldier: index,
                        heavy,
                    });
                }
                wound.seen_hp = s.hp;
                if s.hp == 0 {
                    wound.task = Task::None;
                    wound.held = wound.state == WoundState::Gone;
                    continue;
                }
                match wound.state {
                    WoundState::Light => {
                        wound.left = wound.left.saturating_sub(1);
                        if wound.left == 0 {
                            wound.state = WoundState::Treated;
                            wound.left = limp;
                        }
                    }
                    WoundState::Heavy if !wound.stable => {
                        wound.left = wound.left.saturating_sub(1);
                        if wound.left == 0 {
                            s.hp = 0;
                            wound.seen_hp = 0;
                            wound.task = Task::None;
                            died = true;
                            events.push(CareEvent::DiedOfWounds {
                                unit: id,
                                soldier: index,
                            });
                        }
                    }
                    WoundState::Treated => wound.left = wound.left.saturating_sub(1),
                    _ => {}
                }
                wound.held = s.hp > 0 && wound.state == WoundState::Heavy;
                wound.dragged = false;
                knocked |= s.knocked > 1;
            }
        }
        if knocked {
            self.care.active = true;
        }
        if !events.is_empty() {
            self.care.active = true;
            self.care.events.append(&mut events);
        }
        if died {
            let unit = &mut self.units[i];
            let squad = unit.squad.as_ref().expect("checked");
            unit.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            if unit.hp == 0 {
                let at = crate::combat::Vec3::new(unit.position.x, unit.position.y, unit.height);
                self.destroy(i, id, at);
                return;
            }
        }
        if self.care.active {
            self.buddy_aid(i);
        }
    }

    /// Comrades drag and carry the heavily wounded and pull up the
    /// knocked down.
    fn buddy_aid(&mut self, i: usize) {
        let b = self.care.rules.buddy;
        let unit = &self.units[i];
        let squad = unit.squad.as_ref().expect("checked");
        if squad.garrison.is_some() {
            return;
        }
        let id = unit.id;
        let moving = matches!(unit.route, Route::Following { .. });
        let facing = unit.facing;
        let walk =
            unit.speed_road * Fixed::from_num(self.infantry.rules.formation.catch_up_pct) / 100;
        let walk = walk.max(unit.speed_offroad / 4);
        let drag = pct(walk, b.drag_speed_pct);
        let n = squad.soldiers.len();
        let busy = squad.soldiers.iter().any(|x| {
            x.alive()
                && (x.wound.state == WoundState::Heavy
                    || x.knocked > 1
                    || x.wound.task != Task::None)
        });
        if !busy {
            return;
        }
        // Who is held by a medic (stretcher, treatment) is not ours to move.
        let held_by_medic: Vec<bool> = (0..n).map(|k| self.care.holding(id, k).is_some()).collect();
        let near = |a: Vec2, c: Vec2| (a - c).length() <= Fixed::from_num(4) / 5;
        // Assign helpers.
        let mut soldiers = squad.soldiers.clone();
        for k in 0..n {
            let p = soldiers[k].clone();
            if !p.alive() || held_by_medic[k] {
                continue;
            }
            let index = u16::try_from(k).unwrap_or(u16::MAX);
            let wants = if p.wound.state == WoundState::Heavy {
                let to_cover = p
                    .spot
                    .is_some_and(|s| !p.at_spot() && (s - p.position).length() <= mm(b.drag_mm));
                (moving || to_cover).then_some(Task::Aid(index))
            } else if p.knocked > 1 && p.wound.state != WoundState::Gone {
                Some(Task::HelpUp(index))
            } else {
                None
            };
            let Some(task) = wants else { continue };
            if soldiers.iter().any(|h| h.alive() && h.wound.task == task) {
                continue;
            }
            let reach = match task {
                Task::HelpUp(_) => mm(b.help_mm),
                _ => Fixed::from_num(1000),
            };
            let helper = soldiers
                .iter()
                .enumerate()
                .filter(|&(j, h)| j != k && h.alive() && h.knocked == 0 && h.wound.able())
                .map(|(j, h)| ((h.position - p.position).length(), j))
                .filter(|&(d, _)| d <= reach)
                .min();
            if let Some((_, j)) = helper {
                soldiers[j].wound.task = task;
            }
        }
        // Carry out the tasks.
        let map = &self.map;
        for j in 0..n {
            let task = soldiers[j].wound.task;
            match task {
                Task::None => {}
                Task::HelpUp(k) => {
                    let k = usize::from(k);
                    let p = soldiers[k].clone();
                    if !soldiers[j].alive() || !p.alive() || p.knocked == 0 {
                        soldiers[j].wound.task = Task::None;
                        continue;
                    }
                    soldiers[j].wound.held = true;
                    // Beside him he waits to pull him up; the knock-down
                    // time itself (dazed, prompt 048) stays.
                    if !near(soldiers[j].position, p.position) {
                        let to = toward(soldiers[j].position, p.position, walk);
                        soldiers[j].position = to;
                        soldiers[j].height = map.height_at(to);
                    }
                }
                Task::Aid(k) => {
                    let k = usize::from(k);
                    let p = soldiers[k].clone();
                    if !soldiers[j].alive()
                        || !p.alive()
                        || p.wound.state != WoundState::Heavy
                        || held_by_medic[k]
                    {
                        soldiers[j].wound.task = Task::None;
                        continue;
                    }
                    if !near(soldiers[j].position, p.position) {
                        soldiers[j].wound.held = true;
                        let to = toward(soldiers[j].position, p.position, walk);
                        soldiers[j].position = to;
                        soldiers[j].height = map.height_at(to);
                        continue;
                    }
                    if moving {
                        // Carried along behind his helper, who keeps his
                        // place in the formation.
                        soldiers[j].wound.held = false;
                        let to = soldiers[j].position - facing.scale(Fixed::ONE / 2);
                        let to = if map.contains(to) {
                            to
                        } else {
                            soldiers[j].position
                        };
                        soldiers[k].position = to;
                        soldiers[k].height = map.height_at(to);
                        soldiers[k].wound.dragged = true;
                    } else if let Some(spot) = p.spot.filter(|_| !p.at_spot()) {
                        // Dragged to his cover spot, the helper walking
                        // backwards ahead of him.
                        soldiers[j].wound.held = true;
                        let to = toward(p.position, spot, drag);
                        let d = spot - to;
                        let len = d.length();
                        let ahead = if len > Fixed::ZERO {
                            Vec2::new(d.x / len, d.y / len)
                        } else {
                            facing
                        };
                        soldiers[k].position = to;
                        soldiers[k].height = map.height_at(to);
                        soldiers[k].wound.dragged = true;
                        let h = to + ahead.scale(Fixed::from_num(7) / 10);
                        let h = if map.contains(h) { h } else { to };
                        soldiers[j].position = h;
                        soldiers[j].height = map.height_at(h);
                    } else {
                        soldiers[j].wound.task = Task::None;
                    }
                }
            }
        }
        let squad = self.units[i].squad.as_mut().expect("checked");
        squad.soldiers = soldiers;
    }

    /// Medic teams take casualties, carry them and treat them.
    fn update_medics(&mut self) {
        // Advance the jobs.
        let mut k = 0;
        while k < self.care.jobs.len() {
            if self.advance_medic_job(k) {
                k += 1;
            } else {
                self.care.jobs.remove(k);
            }
        }
        if !self.care.medic.iter().any(|&m| m) {
            return;
        }
        // Idle teams look for casualties.
        let search = mm(self.care.rules.medic.search_mm);
        for i in 0..self.units.len() {
            let medic = &self.units[i];
            if !self.care.medic[medic.unit_type as usize]
                || !idle(medic)
                || self.care.job_of(medic.id).is_some()
            {
                continue;
            }
            let (owner, at) = (medic.owner, medic.position);
            let mut best: Option<(Fixed, usize, usize)> = None;
            for (j, u) in self.units.iter().enumerate() {
                let Some(s) = &u.squad else { continue };
                if !u.alive()
                    || !self.teams.allied(u.owner, owner)
                    || transport::aboard(u)
                    || s.garrison.is_some()
                {
                    continue;
                }
                let d0 = u.position - at;
                if d0.x.abs() > search + u.spread || d0.y.abs() > search + u.spread {
                    continue;
                }
                for (k, x) in s.soldiers.iter().enumerate() {
                    if !x.alive() || x.wound.state != WoundState::Heavy {
                        continue;
                    }
                    let d = (x.position - at).length();
                    if d <= search
                        && best.is_none_or(|b| (d, j, k) < b)
                        && !self.care.claimed(u.id, k)
                    {
                        best = Some((d, j, k));
                    }
                }
            }
            let Some((_, j, k)) = best else { continue };
            let job = Job {
                medic: self.units[i].id,
                patient: self.units[j].id,
                soldier: u16::try_from(k).unwrap_or(u16::MAX),
                phase: JobPhase::Going,
                left: 0,
                post: at,
                tries: 0,
            };
            let target = self.patient_at(&job).unwrap_or(at);
            self.send(i, target);
            self.care.jobs.push(job);
            self.care.jobs.sort_by_key(|j| j.medic);
            self.care.active = true;
        }
    }

    /// Index of unit `id`.
    fn care_index(&self, id: UnitId) -> Option<usize> {
        self.units.binary_search_by_key(&id, |u| u.id).ok()
    }

    /// Where the casualty of `job` lies, if he still needs the team.
    fn patient_at(&self, job: &Job) -> Option<Vec2> {
        let u = &self.units[self.care_index(job.patient)?];
        let s = u.squad.as_ref()?.soldiers.get(usize::from(job.soldier))?;
        (u.alive() && s.alive() && s.wound.state == WoundState::Heavy && !transport::aboard(u))
            .then_some(s.position)
    }

    /// Sends unit `i` to `target` (a path request, as an order would).
    fn send(&mut self, i: usize, target: Vec2) {
        let unit = &mut self.units[i];
        let id = self.paths.request(
            unit.position,
            target,
            unit.unit_type as usize,
            vec![(unit.id, Vec2::ZERO)],
        );
        unit.target = Some(target);
        unit.route = Route::Waiting(id);
        unit.replans = 0;
    }

    /// One tick of job `k`; `false` when it ends.
    fn advance_medic_job(&mut self, k: usize) -> bool {
        let rules = self.care.rules.medic;
        let reach = mm(rules.reach_mm);
        let mut job = self.care.jobs[k];
        let Some(i) = self.care_index(job.medic) else {
            return false;
        };
        let medic = &self.units[i];
        let alive_medic = medic.alive() && !transport::aboard(medic);
        let Some(at) = self.patient_at(&job).filter(|_| alive_medic) else {
            self.release_medics(i);
            return false;
        };
        let able: Vec<usize> = medic
            .squad
            .as_ref()
            .map(|s| {
                s.soldiers
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.alive() && x.knocked == 0 && x.wound.can_fire())
                    .map(|(n, _)| n)
                    .collect()
            })
            .unwrap_or_default();
        if able.is_empty() {
            self.release_medics(i);
            return false;
        }
        let pos = medic.position;
        let following = medic.route != Route::None;
        match job.phase {
            JobPhase::Going => {
                if following {
                    // Redirected by the player, or the casualty was carried
                    // off: give up (an idle team looks again).
                    return !self.units[i]
                        .target
                        .is_some_and(|t| (t - at).length() > reach * 4);
                }
                if (pos - at).length() > reach * 2 {
                    if job.tries >= 2 {
                        return false;
                    }
                    job.tries += 1;
                    self.care.jobs[k] = job;
                    self.send(i, at);
                    return true;
                }
                // Arrived: first aid stops the bleeding.
                self.set_patient(&job, |w| w.stable = true);
                job.tries = 0;
                let stretcher = able.len() >= rules.stretcher_men as usize;
                if stretcher && (job.post - at).length() > reach * 2 {
                    job.phase = JobPhase::Carrying;
                    self.send(i, job.post);
                } else {
                    job.phase = JobPhase::Treating;
                    job.left = self.care.treat_ticks;
                    stop(&mut self.units[i]);
                }
            }
            JobPhase::Carrying => {
                if following && self.units[i].target.is_some_and(|t| t != job.post) {
                    self.release_medics(i);
                    return false;
                }
                if !following {
                    if (pos - job.post).length() > reach * 2 && job.tries < 2 {
                        job.tries += 1;
                        self.care.jobs[k] = job;
                        self.send(i, job.post);
                        self.carry(i, &job, &able);
                        return true;
                    }
                    job.phase = JobPhase::Treating;
                    job.left = self.care.treat_ticks;
                }
                self.carry(i, &job, &able);
            }
            JobPhase::Treating => {
                if following {
                    self.release_medics(i);
                    return false;
                }
                self.treat(i, &job, &able, at);
                job.left = job.left.saturating_sub(1);
                if job.left == 0 {
                    let limp = self.care.limp_ticks;
                    self.set_patient(&job, |w| {
                        w.state = WoundState::Treated;
                        w.left = limp;
                        w.stable = false;
                        w.held = false;
                    });
                    self.care.events.push(CareEvent::Treated {
                        medic: job.medic,
                        unit: job.patient,
                        soldier: job.soldier,
                    });
                    self.release_medics(i);
                    return false;
                }
            }
        }
        self.care.jobs[k] = job;
        true
    }

    /// Changes the wound of the casualty of `job`.
    fn set_patient(&mut self, job: &Job, f: impl FnOnce(&mut Wound)) {
        let Some(j) = self.care_index(job.patient) else {
            return;
        };
        if let Some(s) = self.units[j]
            .squad
            .as_mut()
            .and_then(|s| s.soldiers.get_mut(usize::from(job.soldier)))
        {
            f(&mut s.wound);
        }
    }

    /// Moves the casualty of `job` and soldier `soldier` of his unit.
    fn place_patient(&mut self, job: &Job, to: Vec2) {
        let Some(j) = self.care_index(job.patient) else {
            return;
        };
        let height = self.map.height_at(to);
        if let Some(s) = self.units[j]
            .squad
            .as_mut()
            .and_then(|s| s.soldiers.get_mut(usize::from(job.soldier)))
        {
            s.position = to;
            s.height = height;
            s.wound.held = true;
        }
    }

    fn place_medic(&mut self, i: usize, n: usize, to: Vec2) {
        let height = self.map.height_at(to);
        if let Some(s) = self.units[i]
            .squad
            .as_mut()
            .and_then(|s| s.soldiers.get_mut(n))
        {
            s.position = to;
            s.height = height;
            s.wound.held = true;
        }
    }

    /// The stretcher between the first two able men of team `i`; the
    /// casualty on it.
    fn carry(&mut self, i: usize, job: &Job, able: &[usize]) {
        let (pos, facing) = (self.units[i].position, self.units[i].facing);
        let half = Fixed::from_num(95) / 100;
        self.place_patient(job, pos);
        if let [front, back, ..] = *able {
            let (f, b) = (pos + facing.scale(half), pos - facing.scale(half));
            let f = if self.map.contains(f) { f } else { pos };
            let b = if self.map.contains(b) { b } else { pos };
            self.place_medic(i, front, f);
            self.place_medic(i, back, b);
        }
    }

    /// The team kneels on both sides of the casualty.
    fn treat(&mut self, i: usize, job: &Job, able: &[usize], at: Vec2) {
        self.place_patient(job, at);
        let side = Vec2::new(-self.units[i].facing.y, self.units[i].facing.x)
            .scale(Fixed::from_num(7) / 10);
        for (n, &m) in able.iter().take(2).enumerate() {
            let to = if n == 0 { at + side } else { at - side };
            let to = if self.map.contains(to) { to } else { at };
            self.place_medic(i, m, to);
        }
    }

    /// The soldiers of team `i` go back to their places.
    fn release_medics(&mut self, i: usize) {
        if let Some(s) = self.units[i].squad.as_mut() {
            for x in &mut s.soldiers {
                if x.wound.state != WoundState::Heavy {
                    x.wound.held = false;
                }
            }
        }
    }

    /// Survivors of the vehicles destroyed this tick bail out.
    fn bail_out_crews(&mut self) {
        let destroyed: Vec<UnitId> = self
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Destroyed { unit, .. } => Some(unit),
                _ => None,
            })
            .collect();
        for vehicle in destroyed {
            let Some(j) = self.care_index(vehicle) else {
                continue;
            };
            let unit = &self.units[j];
            let t = unit.unit_type as usize;
            let Some(crew_t) = self.care.crew_type.get(t).copied().flatten() else {
                continue;
            };
            if unit.link.is_some() || unit.squad.is_some() {
                continue;
            }
            let crew = u32::from(self.unit_types[t].transport.crew);
            let size = u32::from(self.unit_types[crew_t as usize].combat.squad_size);
            let mut rng = Rng::new(
                self.rng.state() ^ (u64::from(vehicle.0) << 32) ^ self.tick ^ 0xc4e3_ba11_0049_0049,
            );
            let survive = self.care.rules.crew.survive_pct;
            let men = (0..crew.min(size))
                .filter(|_| rng.below(100) < survive)
                .count();
            if men == 0 {
                continue;
            }
            let (owner, facing) = (unit.owner, unit.facing);
            let back = self.unit_types[t].combat.length_m / 2 + Fixed::from_num(2);
            let Some(at) = self.bail_spot(crew_t as usize, unit.position, facing, back) else {
                continue;
            };
            let index = self.spawn_unit(owner, crew_t, at, facing, FireMode::default());
            let new = &mut self.units[index];
            let squad = new.squad.as_mut().expect("crews are squads");
            for (n, s) in squad.soldiers.iter_mut().enumerate() {
                if n >= men {
                    s.hp = 0;
                    s.wound.state = WoundState::Gone;
                    s.wound.held = true;
                }
                s.wound.seen_hp = s.hp;
            }
            new.hp = squad.soldiers.iter().map(|s| s.hp).sum();
            let crew_id = new.id;
            self.care.active = true;
            self.care.events.push(CareEvent::BailedOut {
                vehicle,
                crew: crew_id,
                men: u16::try_from(men).unwrap_or(u16::MAX),
            });
        }
    }

    /// A passable point for unit type `profile` behind (then beside) a
    /// wreck at `at`.
    fn bail_spot(&self, profile: usize, at: Vec2, facing: Vec2, back: Fixed) -> Option<Vec2> {
        let right = Vec2::new(-facing.y, facing.x);
        let ok = |p: Vec2| {
            self.map.contains(p)
                && self.terrain.types[usize::from(self.map.terrain_at(p))].passable
                && self.paths.grid.passable_for(profile, p)
        };
        let dirs = [
            Vec2::new(-facing.x, -facing.y),
            right,
            Vec2::new(-right.x, -right.y),
            facing,
        ];
        for extra in 0..4 {
            let r = back + Fixed::from_num(extra * 2);
            for d in dirs {
                let p = at + d.scale(r);
                if ok(p) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Care state for the state hash (once anything happened).
    pub(super) fn hash_care(&self, h: &mut StableHasher) {
        if !self.care.active {
            return;
        }
        self.care.rules.hash_into(h);
        for u in &self.units {
            let Some(s) = &u.squad else { continue };
            for x in &s.soldiers {
                let w = &x.wound;
                h.write_u8(w.state as u8);
                h.write_u32(w.left);
                h.write_u8(u8::from(w.stable) | u8::from(w.held) << 1 | u8::from(w.dragged) << 2);
                h.write_u32(w.seen_hp);
                let (kind, of) = match w.task {
                    Task::None => (0, 0),
                    Task::Aid(k) => (1, k),
                    Task::HelpUp(k) => (2, k),
                };
                h.write_u8(kind);
                h.write_u32(u32::from(of));
            }
        }
        h.write_u64(self.care.jobs.len() as u64);
        for j in &self.care.jobs {
            h.write_u32(j.medic.0);
            h.write_u32(j.patient.0);
            h.write_u32(u32::from(j.soldier));
            h.write_u8(j.phase as u8);
            h.write_u32(j.left);
            h.write_i64(j.post.x.to_bits());
            h.write_i64(j.post.y.to_bits());
            h.write_u8(j.tries);
        }
    }

    /// Presentation state of soldier `k` of unit `u`.
    pub(super) fn care_view(&self, u: &Unit, k: usize, s: &Soldier) -> (WoundView, AidView) {
        let w = &s.wound;
        let wound = match w.state {
            WoundState::Fit => WoundView::Fit,
            WoundState::Light => WoundView::Bandaging,
            WoundState::Heavy => WoundView::Down { stable: w.stable },
            WoundState::Treated if w.left > 0 => WoundView::Limping,
            WoundState::Treated => WoundView::Bandaged,
            WoundState::Gone => WoundView::Gone,
        };
        if !self.care.active || !s.alive() {
            return (wound, AidView::None);
        }
        let aid = match w.task {
            Task::Aid(p) => {
                let pulling = u.squad.as_ref().is_some_and(|q| {
                    q.soldiers
                        .get(usize::from(p))
                        .is_some_and(|x| x.wound.dragged)
                });
                AidView::Drag {
                    soldier: p,
                    pulling,
                }
            }
            Task::HelpUp(p) => AidView::HelpUp { soldier: p },
            Task::None if w.dragged => AidView::Dragged,
            Task::None => self.medic_view(u, k, s),
        };
        (wound, aid)
    }

    fn medic_view(&self, u: &Unit, k: usize, s: &Soldier) -> AidView {
        if self.care.jobs.is_empty() {
            return AidView::None;
        }
        if let Some(job) = self.care.holding(u.id, k) {
            return if job.phase == JobPhase::Carrying {
                AidView::OnStretcher
            } else {
                AidView::None
            };
        }
        let Some(job) = self.care.job_of(u.id).map(|j| &self.care.jobs[j]) else {
            return AidView::None;
        };
        if !s.wound.held {
            return AidView::None;
        }
        match job.phase {
            JobPhase::Going => AidView::None,
            JobPhase::Treating => AidView::Treat {
                unit: job.patient,
                soldier: job.soldier,
            },
            JobPhase::Carrying => AidView::Stretcher {
                unit: job.patient,
                soldier: job.soldier,
                front: {
                    let d = s.position - u.position;
                    d.x * u.facing.x + d.y * u.facing.y > Fixed::ZERO
                },
            },
        }
    }

    // --- Public queries (presentation, tests) -------------------------------

    /// Care events of the last tick (wounds, deaths of wounds, treatments,
    /// crews bailing out).
    pub fn care_events(&self) -> &[CareEvent] {
        &self.care.events
    }

    /// The care rules of the match.
    pub const fn care_rules(&self) -> &CareRules {
        &self.care.rules
    }

    /// Living soldiers of unit `id` that are (lightly, heavily) wounded and
    /// not yet treated.
    pub fn unit_wounded(&self, id: UnitId) -> (u32, u32) {
        let Some(i) = self.care_index(id) else {
            return (0, 0);
        };
        let Some(s) = &self.units[i].squad else {
            return (0, 0);
        };
        s.soldiers
            .iter()
            .filter(|x| x.alive())
            .fold((0, 0), |(l, h), x| match x.wound.state {
                WoundState::Light => (l + 1, h),
                WoundState::Heavy => (l, h + 1),
                _ => (l, h),
            })
    }

    /// Whether unit `id` is a crew that bailed out of a destroyed vehicle.
    pub fn is_bailed_crew(&self, id: UnitId) -> bool {
        self.care_index(id)
            .is_some_and(|i| self.is_crew_unit(&self.units[i]))
    }

    /// Sets soldier `soldier` of unit `id` heavily wounded (missions and
    /// tests). Not a player command.
    pub fn wound_soldier(&mut self, id: UnitId, soldier: u16, heavy: bool) {
        let (bandage, bleed) = (self.care.bandage_ticks, self.care.bleed_ticks);
        let Some(i) = self.care_index(id) else { return };
        if let Some(s) = self.units[i]
            .squad
            .as_mut()
            .and_then(|q| q.soldiers.get_mut(usize::from(soldier)))
            .filter(|s| s.alive())
        {
            s.wound.state = if heavy {
                WoundState::Heavy
            } else {
                WoundState::Light
            };
            s.wound.left = if heavy { bleed } else { bandage };
            s.wound.stable = false;
            s.wound.held = heavy;
            self.care.active = true;
        }
    }
}
