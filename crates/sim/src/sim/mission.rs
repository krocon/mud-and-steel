//! Runs the mission script (prompt 039, `crate::mission`) inside the tick.
//!
//! Every trigger is waiting (for the triggers in its `after` list), armed
//! (its condition is checked once per tick), due (condition met, the delay
//! runs) or fired (once; its actions ran). Triggers are visited in file
//! order at the end of each tick, after vision and before the standings, so
//! a victory action decides the match in the tick its condition was met.
//!
//! Signals of the human player (`Command::MissionSignal`) are collected
//! while the commands of the tick are applied and dropped after the visit:
//! only triggers armed in that tick can take them. The presentation asks
//! [`Sim::mission_wanted_signals`] which ones are of use.

use super::{Sim, formation_slots, formation_spacing};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::formation::{Engage, Formation};
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::mission::{Action, Arrival, Condition, MAX_SIGNAL_LEN, Mission, ObjectiveState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum TriggerState {
    Waiting,
    Armed,
    /// Fires at this tick.
    Due(u64),
    /// Fired at this tick.
    Fired(u64),
}

/// The mission of a match and its progress.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MissionRt {
    def: Mission,
    triggers: Vec<TriggerState>,
    objectives: Vec<ObjectiveState>,
    /// Unit ids per group (`Mission::groups`).
    groups: Vec<Vec<UnitId>>,
    /// The latest message and the tick it appeared.
    message: Option<(String, u64)>,
    /// The latest camera request and its tick.
    camera: Option<(Vec2, u64)>,
    /// Signals of the current tick.
    signals: Vec<String>,
    /// Columns that drove in (prompt 108), oldest first.
    arrivals: Vec<ArrivalView>,
    /// The "hardly winnable" warning and its tick (prompt 181). Follows
    /// from the triggers, so it is not part of the state hash.
    hopeless: Option<(String, u64)>,
    /// Units that reached the area of each `reach` trigger while it was
    /// armed, sorted (prompt 189; empty for other triggers).
    #[serde(default)]
    reached: Vec<Vec<UnitId>>,
}

/// A column of reinforcements that started driving in (prompt 108): its
/// owner, entry point, destination and tick. Follows from the triggers, so
/// it is not part of the state hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrivalView {
    pub owner: PlayerId,
    pub entry: Vec2,
    pub to: Vec2,
    pub tick: u64,
}

/// Units of a column that still need their move order: owner, entry,
/// destination, unit ids.
type Pending = Vec<(PlayerId, Vec2, Vec2, Vec<UnitId>)>;

/// Distance between the vehicles of a column: their length plus one more
/// (prompt 108; a road march keeps a gap so that one hit does not stop
/// the next vehicle). Squads keep their own width apart.
pub fn column_spacing(footprint: Fixed) -> Fixed {
    footprint * 2
}

/// The point `d` meters along the polyline `points` and the direction
/// there; past its end the last point.
fn along(points: &[Vec2], d: Fixed) -> (Vec2, Vec2) {
    let mut left = d;
    let mut dir = Vec2::new(Fixed::ZERO, -Fixed::ONE);
    for w in points.windows(2) {
        let step = w[1] - w[0];
        let len = step.length();
        if len == Fixed::ZERO {
            continue;
        }
        dir = Vec2::new(step.x / len, step.y / len);
        if left <= len {
            return (w[0] + dir.scale(left), dir);
        }
        left -= len;
    }
    (points.last().copied().unwrap_or(Vec2::ZERO), dir)
}

impl MissionRt {
    pub(super) fn new(def: &Mission) -> Self {
        let mut groups = vec![Vec::new(); def.groups.len()];
        for g in &def.start_groups {
            groups[usize::from(g.group)].extend((g.first..g.first + g.count).map(UnitId));
        }
        Self {
            triggers: vec![TriggerState::Waiting; def.triggers.len()],
            objectives: def
                .objectives
                .iter()
                .map(|o| {
                    if o.shown {
                        ObjectiveState::Active
                    } else {
                        ObjectiveState::Hidden
                    }
                })
                .collect(),
            groups,
            message: None,
            camera: None,
            signals: Vec::new(),
            arrivals: Vec::new(),
            hopeless: None,
            reached: vec![Vec::new(); def.triggers.len()],
            def: def.clone(),
        }
    }

    fn fired(&self, trigger: u16) -> bool {
        matches!(
            self.triggers.get(usize::from(trigger)),
            Some(TriggerState::Fired(_))
        )
    }

    /// Progress in the state hash: trigger states, objectives and groups.
    /// Message and camera follow from the triggers.
    pub(super) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u8(0x39);
        h.write_u64(self.triggers.len() as u64);
        for t in &self.triggers {
            match *t {
                TriggerState::Waiting => h.write_u8(0),
                TriggerState::Armed => h.write_u8(1),
                TriggerState::Due(at) => {
                    h.write_u8(2);
                    h.write_u64(at);
                }
                TriggerState::Fired(at) => {
                    h.write_u8(3);
                    h.write_u64(at);
                }
            }
        }
        for o in &self.objectives {
            h.write_u8(*o as u8);
        }
        for g in &self.groups {
            h.write_u64(g.len() as u64);
            for id in g {
                h.write_u32(id.0);
            }
        }
        // Prompt 189: only once a unit reached a `reach` area, so older
        // missions hash as before.
        if self.reached.iter().any(|r| !r.is_empty()) {
            h.write_u8(0xbd);
            for r in &self.reached {
                h.write_u64(r.len() as u64);
                for id in r {
                    h.write_u32(id.0);
                }
            }
        }
    }
}

/// An objective as the presentation shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveView {
    pub id: String,
    pub text: String,
    pub state: ObjectiveState,
    /// Center and radius of its area.
    pub area: Option<(Vec2, Fixed)>,
    pub unit: Option<String>,
    /// A side objective (prompt 189).
    pub optional: bool,
    /// Ticks until its timer trigger fires, while that one counts down.
    pub timer_ticks: Option<u64>,
    /// How far its progress trigger's count is: (now, needed).
    pub progress: Option<(u32, u32)>,
}

/// The rating of a won mission (prompt 189, `Mission::rating`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RatingView {
    /// Stars, 0..=3 (0 unless the player won).
    pub stars: u8,
    pub in_time: bool,
    pub few_losses: bool,
    pub optional_done: bool,
    /// Own units lost and own units in the match (spawned ones too).
    pub lost: u32,
    pub units: u32,
    /// Optional objectives done and in the mission.
    pub optional: (u32, u32),
}

impl Sim {
    /// The mission of this match (prompt 039), if any.
    pub fn mission(&self) -> Option<&Mission> {
        self.mission.as_deref().map(|m| &m.def)
    }

    /// Objectives in file order with their state.
    pub fn mission_objectives(&self) -> Vec<ObjectiveView> {
        let Some(m) = self.mission.as_deref() else {
            return Vec::new();
        };
        m.def
            .objectives
            .iter()
            .zip(&m.objectives)
            .map(|(o, &state)| ObjectiveView {
                id: o.id.clone(),
                text: o.text.clone(),
                state,
                area: o.area.map(|a| {
                    let a = &m.def.areas[usize::from(a)];
                    (a.center, a.radius)
                }),
                unit: o.unit.clone(),
                optional: o.optional,
                timer_ticks: o.timer.and_then(|t| match m.triggers[usize::from(t)] {
                    TriggerState::Due(at) => Some(at.saturating_sub(self.tick)),
                    _ => None,
                }),
                progress: o
                    .progress
                    .and_then(|t| self.trigger_count(m, usize::from(t))),
            })
            .collect()
    }

    /// The count of a counting trigger (`area`, `units`, `reach`): how
    /// many it has and how many it needs (prompt 189). A fired trigger
    /// shows the count it needed.
    fn trigger_count(&self, m: &MissionRt, t: usize) -> Option<(u32, u32)> {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        let (now, need) = match &m.def.triggers[t].condition {
            Condition::Area {
                area,
                player,
                count: c,
                unit,
            } => (
                self.matching(
                    *player,
                    unit.as_deref(),
                    Some(&m.def.areas[usize::from(*area)]),
                )
                .count(),
                *c,
            ),
            Condition::Units {
                player,
                unit,
                count: c,
            } => (self.count_units(*player, Some(unit)), *c),
            Condition::Reach { count: c, .. } => (m.reached.get(t).map_or(0, Vec::len), *c),
            _ => return None,
        };
        let now = if matches!(m.triggers[t], TriggerState::Fired(_)) {
            need
        } else {
            count(now).min(need)
        };
        Some((now, need))
    }

    /// Prompt 190: a human of a cooperative mission who gives up hands
    /// every unit to the first other human of the side who has not given
    /// up (the leader first, then the seats in file order); without one
    /// nothing changes. The new owner gets their orders as they are.
    pub(super) fn hand_over_on_surrender(&mut self, player: PlayerId) {
        let Some(m) = self.mission.as_deref() else {
            return;
        };
        let humans: Vec<PlayerId> = std::iter::once(m.def.player)
            .chain(m.def.seats.iter().filter(|s| s.taken).map(|s| s.player))
            .collect();
        if humans.len() < 2 || !humans.contains(&player) {
            return;
        }
        let undefeated = |p: PlayerId| {
            self.standings
                .players()
                .iter()
                .any(|s| s.player == p && s.defeat.is_none())
        };
        let Some(heir) = humans
            .iter()
            .copied()
            .find(|&p| p != player && undefeated(p))
        else {
            return;
        };
        if self.outcome().is_some() {
            return;
        }
        for i in 0..self.units.len() {
            if self.units[i].owner == player {
                // Its viewshed counts for the old owner until now.
                self.vision.release_shed(i, player);
                self.units[i].owner = heir;
                self.units[i].vision_dirty = true;
            }
        }
    }

    /// The rating of the mission once the human player won it (prompt
    /// 189); `None` without a mission or before a victory.
    pub fn mission_rating(&self) -> Option<RatingView> {
        let m = self.mission.as_deref()?;
        let outcome = self.outcome()?;
        let won = outcome
            .winner
            .is_some_and(|w| self.teams.allied(w, m.def.player));
        if !won {
            return None;
        }
        let rating = m.def.rating;
        let own = self.units.iter().filter(|u| u.owner == m.def.player);
        let units = u32::try_from(own.clone().count()).unwrap_or(u32::MAX);
        let lost = u32::try_from(own.filter(|u| !u.alive()).count()).unwrap_or(u32::MAX);
        let optional: Vec<ObjectiveState> = m
            .def
            .objectives
            .iter()
            .zip(&m.objectives)
            .filter(|(o, _)| o.optional)
            .map(|(_, &s)| s)
            .collect();
        let done = optional
            .iter()
            .filter(|&&s| s == ObjectiveState::Done)
            .count();
        let in_time = rating.par_ticks == 0 || outcome.tick <= rating.par_ticks;
        let few_losses = u64::from(lost) * 100 <= u64::from(units) * u64::from(rating.losses_pct);
        let optional_done = done == optional.len();
        Some(RatingView {
            stars: u8::from(in_time) + u8::from(few_losses) + u8::from(optional_done),
            in_time,
            few_losses,
            optional_done,
            lost,
            units,
            optional: (
                u32::try_from(done).unwrap_or(0),
                u32::try_from(optional.len()).unwrap_or(0),
            ),
        })
    }

    /// The latest message of the mission and the tick it appeared.
    pub fn mission_message(&self) -> Option<(&str, u64)> {
        self.mission
            .as_deref()?
            .message
            .as_ref()
            .map(|(text, tick)| (text.as_str(), *tick))
    }

    /// The "hardly winnable" warning of the mission and its tick
    /// (prompt 181), if a `hopeless` action ran.
    pub fn mission_hopeless(&self) -> Option<(&str, u64)> {
        self.mission
            .as_deref()?
            .hopeless
            .as_ref()
            .map(|(text, tick)| (text.as_str(), *tick))
    }

    /// The latest camera request of the mission and its tick.
    pub fn mission_camera(&self) -> Option<(Vec2, u64)> {
        self.mission.as_deref()?.camera
    }

    /// Columns of reinforcements that drove in so far, oldest first
    /// (prompt 108).
    pub fn mission_arrivals(&self) -> &[ArrivalView] {
        self.mission
            .as_deref()
            .map_or(&[], |m| m.arrivals.as_slice())
    }

    /// Signals that armed triggers wait for, in trigger order.
    pub fn mission_wanted_signals(&self) -> Vec<&str> {
        let Some(m) = self.mission.as_deref() else {
            return Vec::new();
        };
        m.def
            .triggers
            .iter()
            .zip(&m.triggers)
            .filter(|(_, s)| **s == TriggerState::Armed)
            .filter_map(|(t, _)| match &t.condition {
                Condition::Signal { name } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// What completes objective `objective` (prompt 180): the condition
    /// of the first trigger that sets it done, if any. For hints and the
    /// autopilots of the tests.
    pub fn mission_goal(&self, objective: usize) -> Option<&Condition> {
        let m = self.mission.as_deref()?;
        m.def
            .triggers
            .iter()
            .find(|t| {
                t.actions.iter().any(|a| {
                    matches!(a, Action::Objective { index, state: ObjectiveState::Done }
                        if usize::from(*index) == objective)
                })
            })
            .map(|t| &t.condition)
    }

    /// Triggers fired so far and their number.
    pub fn mission_progress(&self) -> (usize, usize) {
        self.mission.as_deref().map_or((0, 0), |m| {
            (
                m.triggers
                    .iter()
                    .filter(|t| matches!(t, TriggerState::Fired(_)))
                    .count(),
                m.triggers.len(),
            )
        })
    }

    /// Whether unit `id` exists and lives.
    pub fn unit_alive(&self, id: UnitId) -> bool {
        self.units
            .binary_search_by_key(&id, |u| u.id)
            .is_ok_and(|i| self.units[i].alive())
    }

    /// Unit ids of a mission group (living or not).
    pub fn mission_group(&self, name: &str) -> Vec<UnitId> {
        self.mission.as_deref().map_or_else(Vec::new, |m| {
            m.def
                .groups
                .iter()
                .position(|g| g == name)
                .map_or_else(Vec::new, |i| m.groups[i].clone())
        })
    }

    /// [`Command::MissionSignal`]: only the mission's player sends them.
    pub(super) fn mission_signal(&mut self, player: PlayerId, name: &str) {
        if let Some(m) = self.mission.as_deref_mut()
            && m.def.player == player
            && name.len() <= MAX_SIGNAL_LEN
            && !m.signals.iter().any(|s| s == name)
        {
            m.signals.push(name.to_owned());
        }
    }

    /// Whether `player` may be eliminated by losing every unit: always
    /// without a mission and for the mission's player, for enemies only if
    /// the mission says so.
    pub(super) fn eliminable(&self, player: PlayerId) -> bool {
        self.mission.as_deref().is_none_or(|m| {
            m.def.elimination
                    || m.def.player == player
                    // Prompt 190: the humans of a cooperative mission.
                    || m.def.seats.iter().any(|s| s.taken && s.player == player)
        })
    }

    /// Visits every trigger once (end of the tick, see the module docs).
    pub(super) fn update_mission(&mut self) {
        let Some(mut m) = self.mission.take() else {
            return;
        };
        let tick = self.tick;
        if m.reached.len() < m.def.triggers.len() {
            m.reached.resize(m.def.triggers.len(), Vec::new());
        }
        for t in 0..m.def.triggers.len() {
            if m.triggers[t] == TriggerState::Waiting
                && m.def.triggers[t].after.iter().all(|&a| m.fired(a))
            {
                m.triggers[t] = TriggerState::Armed;
            }
            if m.triggers[t] == TriggerState::Armed
                && let Condition::Reach {
                    area, player, unit, ..
                } = &m.def.triggers[t].condition
            {
                // Remember every unit that stood in the area (prompt 189).
                let area = &m.def.areas[usize::from(*area)];
                let inside: Vec<UnitId> = self
                    .matching(*player, unit.as_deref(), Some(area))
                    .map(|u| u.id)
                    .collect();
                for id in inside {
                    if let Err(k) = m.reached[t].binary_search(&id) {
                        m.reached[t].insert(k, id);
                    }
                }
            }
            if m.triggers[t] == TriggerState::Armed && self.condition_met(&m, t) {
                m.triggers[t] = TriggerState::Due(tick + u64::from(m.def.triggers[t].delay_ticks));
            }
            if let TriggerState::Due(at) = m.triggers[t]
                && tick >= at
            {
                m.triggers[t] = TriggerState::Fired(tick);
                let actions = m.def.triggers[t].actions.clone();
                let offsets = self.column_offsets(&actions);
                let mut pending: Pending = Vec::new();
                for (action, offset) in actions.iter().zip(offsets) {
                    let drive = matches!(
                        action,
                        Action::Spawn {
                            arrival: Arrival::Drive { .. },
                            ..
                        }
                    );
                    if !drive {
                        self.send_columns(&mut m, &mut pending);
                    }
                    self.run_action(&mut m, action, offset, &mut pending);
                }
                self.send_columns(&mut m, &mut pending);
            }
        }
        m.signals.clear();
        self.mission = Some(m);
    }

    fn condition_met(&self, m: &MissionRt, t: usize) -> bool {
        let open = m.def.triggers[t].open.iter().all(|&o| {
            matches!(
                m.objectives[usize::from(o)],
                ObjectiveState::Hidden | ObjectiveState::Active
            )
        });
        open && self.condition_holds(m, t)
    }

    /// Counts living units of `player` whose catalog id starts with `unit`.
    fn count_units(&self, player: PlayerId, unit: Option<&str>) -> usize {
        self.matching(player, unit, None).count()
    }

    /// Living units of `player` whose catalog id starts with `unit` (if
    /// given) standing in the area (if given).
    fn matching<'a>(
        &'a self,
        player: PlayerId,
        unit: Option<&'a str>,
        area: Option<&'a crate::mission::Area>,
    ) -> impl Iterator<Item = &'a super::Unit> + 'a {
        self.units.iter().filter(move |u| {
            u.alive()
                && u.owner == player
                && unit.is_none_or(|prefix| {
                    self.unit_types[u.unit_type as usize]
                        .name
                        .starts_with(prefix)
                })
                && area.is_none_or(|a| a.contains(u.position))
        })
    }

    /// Living units of a mission group.
    fn group_alive<'a>(
        &'a self,
        m: &'a MissionRt,
        group: u16,
    ) -> impl Iterator<Item = &'a super::Unit> + 'a {
        m.groups[usize::from(group)]
            .iter()
            .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
            .map(|i| &self.units[i])
            .filter(|u| u.alive())
    }

    fn condition_holds(&self, m: &MissionRt, t: usize) -> bool {
        match &m.def.triggers[t].condition {
            Condition::Fewer {
                player,
                unit,
                count,
            } => self.count_units(*player, unit.as_deref()) < *count as usize,
            Condition::Stance {
                player,
                unit,
                area,
                stance,
                count,
            } => {
                let area = area.map(|a| &m.def.areas[usize::from(a)]);
                self.matching(*player, unit.as_deref(), area)
                    .filter(|u| {
                        u.squad
                            .as_ref()
                            .is_some_and(|s| s.stance == *stance && s.changing == 0)
                    })
                    .count()
                    >= *count as usize
            }
            Condition::Garrison {
                player,
                unit,
                area,
                count,
            } => {
                let area = area.map(|a| &m.def.areas[usize::from(a)]);
                self.matching(*player, unit.as_deref(), area)
                    .filter(|u| u.squad.as_ref().is_some_and(|s| s.garrison.is_some()))
                    .count()
                    >= *count as usize
            }
            Condition::Spotted { group, player } => self
                .group_alive(m, *group)
                .any(|u| self.visible_to(*player, u.id)),
            Condition::Unseen {
                player,
                unit,
                area,
                by,
                count,
            } => {
                let area = area.map(|a| &m.def.areas[usize::from(a)]);
                self.matching(*player, unit.as_deref(), area)
                    .filter(|u| !self.visible_to(*by, u.id))
                    .count()
                    >= *count as usize
            }
            Condition::Pinned { group, pct } => self.group_alive(m, *group).any(|u| {
                self.squad_tactics(u.id)
                    .is_some_and(|t| t.suppression >= *pct)
            }),
            Condition::Smoke { area } => self.smoke_at(m.def.areas[usize::from(*area)].center),
            Condition::Air { player } => self.air_events().iter().any(
                |e| matches!(e, crate::air::AirEvent::Called { owner, .. } if owner == player),
            ),
            Condition::Always => true,
            Condition::Reach { count, .. } => {
                m.reached.get(t).map_or(0, Vec::len) >= *count as usize
            }
            Condition::Area {
                area,
                player,
                count,
                unit,
            } => {
                let area = &m.def.areas[usize::from(*area)];
                let inside = self
                    .units
                    .iter()
                    .filter(|u| {
                        u.alive()
                            && u.owner == *player
                            && area.contains(u.position)
                            && unit.as_deref().is_none_or(|prefix| {
                                self.unit_types[u.unit_type as usize]
                                    .name
                                    .starts_with(prefix)
                            })
                    })
                    .count();
                inside >= *count as usize
            }
            Condition::Destroyed { group } => {
                let ids = &m.groups[usize::from(*group)];
                !ids.is_empty()
                    && ids.iter().all(|&id| {
                        self.units
                            .binary_search_by_key(&id, |u| u.id)
                            .map_or(true, |i| !self.units[i].alive())
                    })
            }
            Condition::Eliminated { player } => self.alive_units(*player) == 0,
            Condition::Captured { point, player } => self
                .supply_points()
                .iter()
                .any(|p| p.name == *point && p.owner == Some(*player)),
            Condition::Signal { name } => m.signals.iter().any(|s| s == name),
            Condition::Units {
                player,
                unit,
                count,
            } => {
                let structures = self.structures();
                let owned = self
                    .units
                    .iter()
                    .filter(|u| {
                        u.alive()
                            && u.owner == *player
                            && self.unit_types[u.unit_type as usize]
                                .name
                                .starts_with(unit.as_str())
                            && structures
                                .iter()
                                .find(|s| s.unit == u.id)
                                .is_none_or(crate::sim::StructureView::completed)
                    })
                    .count();
                owned >= *count as usize
            }
        }
    }

    /// Applies `command` for the units of `group`, once per owner, like an
    /// order of that player.
    fn group_command(
        &mut self,
        m: &MissionRt,
        group: u16,
        command: impl Fn(Vec<UnitId>) -> Command,
    ) {
        let ids = &m.groups[usize::from(group)];
        let mut owners: Vec<PlayerId> = ids
            .iter()
            .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
            .map(|i| self.units[i].owner)
            .collect();
        owners.sort_unstable();
        owners.dedup();
        for owner in owners {
            self.apply(&PlayerCommand::new(owner, command(ids.clone())));
        }
    }

    /// Where each driving spawn of one trigger starts in its column, in
    /// meters from the entry: spawns with the same entry and destination
    /// form one column, the first spawn at its head, the last at the edge.
    fn column_offsets(&self, actions: &[Action]) -> Vec<Fixed> {
        let mut offsets = vec![Fixed::ZERO; actions.len()];
        let mut used: Vec<(Vec2, Vec2, Fixed)> = Vec::new();
        for (k, action) in actions.iter().enumerate().rev() {
            let Action::Spawn {
                unit_type,
                count,
                at,
                arrival: Arrival::Drive { entry },
                ride,
                ..
            } = action
            else {
                continue;
            };
            let length = if ride.is_some() {
                Fixed::ZERO
            } else {
                column_spacing(self.unit_types[*unit_type as usize].footprint_m)
                    * Fixed::from_num(*count)
            };
            match used.iter_mut().find(|u| u.0 == *entry && u.1 == *at) {
                Some(u) => {
                    offsets[k] = u.2;
                    u.2 += length;
                }
                None => used.push((*entry, *at, length)),
            }
        }
        offsets
    }

    /// Gives the columns waiting in `pending` their move order (one per
    /// owner, entry and destination, as a command of the owner) and records
    /// their arrival.
    fn send_columns(&mut self, m: &mut MissionRt, pending: &mut Pending) {
        for (owner, entry, to, units) in pending.drain(..) {
            m.arrivals.push(ArrivalView {
                owner,
                entry,
                to,
                tick: self.tick,
            });
            if units.is_empty() {
                continue;
            }
            self.apply(&PlayerCommand::new(
                owner,
                Command::FormationMove {
                    units,
                    target: to,
                    formation: Formation::Auto,
                    facing: None,
                    engage: Engage::Infantry,
                },
            ));
        }
    }

    /// A driving spawn (prompt 108): the units start one behind the other
    /// along the path from `entry` to `at`, `offset` meters from the entry
    /// for the rearmost, facing along the path. Passengers mount a free
    /// carrier of `ride`; the others wait in `pending` for their order.
    #[allow(clippy::too_many_arguments)]
    fn spawn_column(
        &mut self,
        m: &mut MissionRt,
        (group, owner, unit_type, count): (u16, PlayerId, u32, u32),
        (entry, at): (Vec2, Vec2),
        fire_mode: crate::combat::FireMode,
        ride: Option<u16>,
        offset: Fixed,
        pending: &mut Pending,
    ) {
        let spacing = column_spacing(self.unit_types[unit_type as usize].footprint_m);
        let points = self
            .paths
            .grid
            .find_path(entry, at, unit_type as usize)
            .map(|p| p.waypoints)
            .filter(|w| w.len() >= 2)
            .unwrap_or_else(|| vec![entry, at]);
        let slot = match pending
            .iter()
            .position(|p| p.0 == owner && p.1 == entry && p.2 == at)
        {
            Some(i) => i,
            None => {
                pending.push((owner, entry, at, Vec::new()));
                pending.len() - 1
            }
        };
        for k in 0..count {
            let behind = if ride.is_some() { 0 } else { count - 1 - k };
            let d = offset + spacing * Fixed::from_num(behind) + spacing / 2;
            let (p, facing) = along(&points, d);
            if !self.map.contains(p) {
                continue;
            }
            let i = self.spawn_unit(owner, unit_type, p, facing, fire_mode);
            let id = self.units[i].id;
            m.groups[usize::from(group)].push(id);
            let boarded = ride.is_some_and(|r| self.mount_on_group(m, i, r));
            if !boarded {
                pending[slot].3.push(id);
            }
        }
    }

    /// Squad `i` mounts the first living carrier of `group` (same owner,
    /// not towed or aboard) with enough free seats: vehicles with seats
    /// inside first, then tanks that carry riders on their hull.
    fn mount_on_group(&mut self, m: &MissionRt, i: usize, group: u16) -> bool {
        if self.units[i].squad.is_none() {
            return false;
        }
        let owner = self.units[i].owner;
        for inside in [true, false] {
            for &id in &m.groups[usize::from(group)] {
                let Ok(c) = self.units.binary_search_by_key(&id, |u| u.id) else {
                    continue;
                };
                let v = &self.units[c];
                if c == i
                    || !v.alive()
                    || v.owner != owner
                    || v.link.is_some()
                    || self.seats_inside(c) != inside
                {
                    continue;
                }
                self.board(i, c);
                if self.units[i].link.is_some() {
                    return true;
                }
            }
        }
        false
    }

    fn run_action(
        &mut self,
        m: &mut MissionRt,
        action: &Action,
        offset: Fixed,
        pending: &mut Pending,
    ) {
        match action {
            Action::Spawn {
                group,
                owner,
                unit_type,
                count,
                at,
                fire_mode,
                arrival: Arrival::Drive { entry },
                ride,
            } => self.spawn_column(
                m,
                (*group, *owner, *unit_type, *count),
                (*entry, *at),
                *fire_mode,
                *ride,
                offset,
                pending,
            ),
            Action::Text { text } => m.message = Some((text.clone(), self.tick)),
            Action::Hopeless { text } => {
                // Only while the match runs; once a time.
                if self.standings.outcome().is_none() && m.hopeless.is_none() {
                    m.message = Some((text.clone(), self.tick));
                    m.hopeless = Some((text.clone(), self.tick));
                }
            }
            Action::Objective { index, state } => m.objectives[usize::from(*index)] = *state,
            Action::Spawn {
                group,
                owner,
                unit_type,
                count,
                at,
                fire_mode,
                arrival: Arrival::Appear,
                ride,
            } => {
                let footprint = self.unit_types[*unit_type as usize].footprint_m;
                let slots = formation_slots(*at, *count as usize, formation_spacing(footprint));
                for p in slots {
                    if !self.map.contains(p) {
                        continue;
                    }
                    let to_center = Vec2::ZERO - p;
                    let distance = to_center.length();
                    let facing = if distance > Fixed::ZERO {
                        Vec2::new(to_center.x / distance, to_center.y / distance)
                    } else {
                        Vec2::new(Fixed::ZERO, -Fixed::ONE)
                    };
                    let i = self.spawn_unit(*owner, *unit_type, p, facing, *fire_mode);
                    m.groups[usize::from(*group)].push(self.units[i].id);
                    if let Some(r) = ride {
                        self.mount_on_group(m, i, *r);
                    }
                }
            }
            Action::Camera { at } => m.camera = Some((*at, self.tick)),
            Action::Order {
                group,
                target,
                engage,
            } => self.group_command(m, *group, |units| Command::FormationMove {
                units,
                target: *target,
                formation: Formation::Auto,
                facing: None,
                engage: *engage,
            }),
            Action::FireMode { group, mode } => {
                self.group_command(m, *group, |units| Command::SetFireMode {
                    units,
                    mode: *mode,
                });
            }
            Action::Barrage {
                group,
                target,
                radius_m,
                salvos,
            } => self.group_command(m, *group, |units| Command::Barrage {
                units,
                target: *target,
                radius_m: *radius_m,
                salvos: *salvos,
                smoke: false,
            }),
            Action::Air {
                player,
                mission,
                target,
                to,
            } => {
                let command = match to {
                    Some(to) => Command::CallAirLine {
                        mission: *mission,
                        from: *target,
                        to: *to,
                    },
                    None => Command::CallAir {
                        mission: *mission,
                        target: *target,
                    },
                };
                self.apply(&PlayerCommand::new(*player, command));
            }
            Action::Camouflage { group, on } => {
                self.group_command(m, *group, |units| Command::SetCamouflage { units, on: *on });
            }
            Action::Victory => {
                let player = m.def.player;
                let teams = &self.teams;
                self.standings
                    .decide(self.tick, player, true, |a, b| teams.allied(a, b));
            }
            Action::Defeat => {
                let player = m.def.player;
                let teams = &self.teams;
                self.standings
                    .decide(self.tick, player, false, |a, b| teams.allied(a, b));
            }
        }
    }
}
