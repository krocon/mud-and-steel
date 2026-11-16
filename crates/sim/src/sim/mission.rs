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

use super::{Sim, alive_units, formation_slots, formation_spacing};
use crate::command::{Command, PlayerCommand, PlayerId, UnitId};
use crate::formation::Formation;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::mission::{Action, Condition, MAX_SIGNAL_LEN, Mission, ObjectiveState};
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
            })
            .collect()
    }

    /// The latest message of the mission and the tick it appeared.
    pub fn mission_message(&self) -> Option<(&str, u64)> {
        self.mission
            .as_deref()?
            .message
            .as_ref()
            .map(|(text, tick)| (text.as_str(), *tick))
    }

    /// The latest camera request of the mission and its tick.
    pub fn mission_camera(&self) -> Option<(Vec2, u64)> {
        self.mission.as_deref()?.camera
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
        self.mission
            .as_deref()
            .is_none_or(|m| m.def.elimination || m.def.player == player)
    }

    /// Visits every trigger once (end of the tick, see the module docs).
    pub(super) fn update_mission(&mut self) {
        let Some(mut m) = self.mission.take() else {
            return;
        };
        let tick = self.tick;
        for t in 0..m.def.triggers.len() {
            if m.triggers[t] == TriggerState::Waiting
                && m.def.triggers[t].after.iter().all(|&a| m.fired(a))
            {
                m.triggers[t] = TriggerState::Armed;
            }
            if m.triggers[t] == TriggerState::Armed && self.condition_met(&m, t) {
                m.triggers[t] = TriggerState::Due(tick + u64::from(m.def.triggers[t].delay_ticks));
            }
            if let TriggerState::Due(at) = m.triggers[t]
                && tick >= at
            {
                m.triggers[t] = TriggerState::Fired(tick);
                let actions = m.def.triggers[t].actions.clone();
                for action in &actions {
                    self.run_action(&mut m, action);
                }
            }
        }
        m.signals.clear();
        self.mission = Some(m);
    }

    fn condition_met(&self, m: &MissionRt, t: usize) -> bool {
        match &m.def.triggers[t].condition {
            Condition::Always => true,
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
            Condition::Eliminated { player } => alive_units(&self.units, *player) == 0,
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

    fn run_action(&mut self, m: &mut MissionRt, action: &Action) {
        match action {
            Action::Text { text } => m.message = Some((text.clone(), self.tick)),
            Action::Objective { index, state } => m.objectives[usize::from(*index)] = *state,
            Action::Spawn {
                group,
                owner,
                unit_type,
                count,
                at,
                fire_mode,
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
                }
            }
            Action::Camera { at } => m.camera = Some((*at, self.tick)),
            Action::Order {
                group,
                target,
                engage,
            } => {
                // One command per owner, like an order of that player.
                let ids = m.groups[usize::from(*group)].clone();
                let mut owners: Vec<PlayerId> = ids
                    .iter()
                    .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
                    .map(|i| self.units[i].owner)
                    .collect();
                owners.sort_unstable();
                owners.dedup();
                for owner in owners {
                    self.apply(&PlayerCommand::new(
                        owner,
                        Command::FormationMove {
                            units: ids.clone(),
                            target: *target,
                            formation: Formation::Auto,
                            facing: None,
                            engage: *engage,
                        },
                    ));
                }
            }
            Action::FireMode { group, mode } => {
                let ids = m.groups[usize::from(*group)].clone();
                let mut owners: Vec<PlayerId> = ids
                    .iter()
                    .filter_map(|&id| self.units.binary_search_by_key(&id, |u| u.id).ok())
                    .map(|i| self.units[i].owner)
                    .collect();
                owners.sort_unstable();
                owners.dedup();
                for owner in owners {
                    self.apply(&PlayerCommand::new(
                        owner,
                        Command::SetFireMode {
                            units: ids.clone(),
                            mode: *mode,
                        },
                    ));
                }
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
