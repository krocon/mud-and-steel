//! The commander's situation picture (prompt 051): what the player's units
//! saw, how old that knowledge is, where own units were lost, and the enemy
//! fronts with their strength. Built only from the [`PlayerView`]: sightings
//! of the own units (and allies with shared vision) and the own losses.

use crate::{Ai, PlayerView, Role, Sighting, centroid, dist};
use serde::{Deserialize, Serialize};
use sim::{Fixed, UnitId, Vec2};

/// Knowledge younger than this counts in full (ticks, 30 s).
pub(crate) const FRESH_TICKS: u64 = 600;
/// Sightings of mobile units older than this are no longer used for plans
/// (ticks, 4 min): the unit has most likely moved. Buildings do not age.
pub(crate) const STALE_TICKS: u64 = 4800;
/// Losses count in full for this long, then fade out over the same time.
const LOSS_TICKS: u64 = 2400;
/// A loss makes the ground within this distance dangerous (m).
const LOSS_RADIUS_M: i32 = 120;
/// Losses kept at most (the oldest go first).
const MAX_LOSSES: usize = 64;
/// Sightings within this distance of each other belong to one front (m).
const FRONT_LINK_M: i32 = 160;

/// Where and when an own unit was lost, and what it was worth.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Loss {
    pub(crate) position: Vec2,
    pub(crate) tick: u64,
    pub(crate) value: u32,
}

/// The situation picture besides the sightings (`Ai::memory`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Intel {
    /// Own units alive at the last think and where they stood.
    alive: Vec<(UnitId, Vec2, u32)>,
    pub(crate) losses: Vec<Loss>,
    /// Every enemy unit ever seen and not seen destroyed: type, last
    /// position and tick (composition for counter-production, strength for
    /// surrendering). Unlike the sightings these are never forgotten.
    pub(crate) known: Vec<(UnitId, Sighting)>,
}

/// A cluster of fresh sightings: one front of the enemy.
#[derive(Clone, Debug)]
pub(crate) struct Front {
    pub(crate) center: Vec2,
    /// Strength: value times hit point share times freshness.
    pub(crate) value: u64,
    /// Longest range of a weapon in the front that hurts armor, and of
    /// any weapon (m).
    pub(crate) threat_armor: Fixed,
    pub(crate) threat_any: Fixed,
    /// Mean hull direction of its vehicles (zero if none was seen).
    pub(crate) facing: Vec2,
    /// Mobile units in it (not buildings).
    pub(crate) mobile: usize,
    pub(crate) members: Vec<UnitId>,
}

/// Share (percent) of a sighting's weight after `age` ticks.
pub(crate) const fn freshness(age: u64, structure: bool) -> u64 {
    if structure || age <= FRESH_TICKS {
        100
    } else if age >= STALE_TICKS {
        0
    } else {
        100 - (age - FRESH_TICKS) * 100 / (STALE_TICKS - FRESH_TICKS)
    }
}

impl Ai {
    /// What a unit of `unit_type` is worth: its price (sum of the three
    /// resources), else a rough value by role.
    pub(crate) fn value_of(&self, unit_type: u32) -> u32 {
        if let Some(Some(cost)) = self.costs.get(unit_type as usize) {
            let sum: u32 = cost.iter().sum();
            if sum > 0 {
                return sum;
            }
        }
        match self.type_info(unit_type).role {
            Role::Armor => 600,
            Role::Structure => 300,
            Role::Unarmed => 50,
            _ => 200,
        }
    }

    /// Updates losses and the list of known enemies after `remember`.
    pub(crate) fn update_intel(&mut self, view: &PlayerView) {
        let now: Vec<(UnitId, Vec2, u32)> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .map(|u| (u.id, u.position, self.value_of(u.unit_type)))
            .collect();
        for &(id, position, value) in &self.intel.alive {
            if !now.iter().any(|&(n, _, _)| n == id) {
                self.intel.losses.push(Loss {
                    position,
                    tick: view.tick,
                    value,
                });
            }
        }
        if self.intel.losses.len() > MAX_LOSSES {
            let cut = self.intel.losses.len() - MAX_LOSSES;
            self.intel.losses.drain(..cut);
        }
        self.intel.alive = now;
        for e in &view.enemies {
            let pos = self.intel.known.iter().position(|(id, _)| *id == e.id);
            if !e.alive() {
                if let Some(i) = pos {
                    self.intel.known.remove(i);
                }
                continue;
            }
            if let Some(&s) = self.memory.get(&e.id) {
                match pos {
                    Some(i) => self.intel.known[i].1 = s,
                    None => self.intel.known.push((e.id, s)),
                }
            }
        }
    }

    /// Danger at `p` from own losses nearby (value of the units lost,
    /// fading with time) and from fresh sightings of enemy weapons that
    /// reach `p` (value times freshness).
    pub(crate) fn danger_at(&self, p: Vec2, tick: u64, armor: bool) -> u64 {
        let radius = Fixed::from_num(LOSS_RADIUS_M);
        let mut danger = 0u64;
        for l in &self.intel.losses {
            let age = tick.saturating_sub(l.tick);
            if age >= 2 * LOSS_TICKS || dist(l.position, p) > radius {
                continue;
            }
            let pct = if age <= LOSS_TICKS {
                100
            } else {
                (2 * LOSS_TICKS - age) * 100 / LOSS_TICKS
            };
            danger += u64::from(l.value) * pct / 100;
        }
        for s in self.memory.values() {
            let info = self.type_info(s.unit_type);
            let reach = if armor && !info.anti_tank {
                Fixed::ZERO
            } else {
                info.range
            };
            if reach == Fixed::ZERO || dist(s.position, p) > reach {
                continue;
            }
            let fresh = freshness(tick.saturating_sub(s.tick), info.role == Role::Structure);
            danger += u64::from(self.value_of(s.unit_type)) * fresh / 100;
        }
        danger
    }

    /// The enemy fronts: fresh sightings linked within `FRONT_LINK_M`,
    /// strongest first.
    pub(crate) fn fronts(&self, tick: u64) -> Vec<Front> {
        let items: Vec<(UnitId, Sighting)> = self
            .memory
            .iter()
            .filter(|(_, s)| {
                freshness(
                    tick.saturating_sub(s.tick),
                    self.type_info(s.unit_type).role == Role::Structure,
                ) > 0
            })
            .map(|(&id, &s)| (id, s))
            .collect();
        let link = Fixed::from_num(FRONT_LINK_M);
        let mut label: Vec<usize> = (0..items.len()).collect();
        // Single linkage; the lists are short (tens of sightings).
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if dist(items[i].1.position, items[j].1.position) <= link {
                    let (a, b) = (label[i], label[j]);
                    if a != b {
                        for l in &mut label {
                            if *l == b {
                                *l = a;
                            }
                        }
                    }
                }
            }
        }
        let mut fronts: Vec<Front> = Vec::new();
        let mut labels: Vec<usize> = label.clone();
        labels.sort_unstable();
        labels.dedup();
        for l in labels {
            let members: Vec<&(UnitId, Sighting)> = items
                .iter()
                .zip(&label)
                .filter(|&(_, &x)| x == l)
                .map(|(m, _)| m)
                .collect();
            let Some(center) = centroid(members.iter().map(|(_, s)| s.position)) else {
                continue;
            };
            let mut front = Front {
                center,
                value: 0,
                threat_armor: Fixed::ZERO,
                threat_any: Fixed::ZERO,
                facing: Vec2::ZERO,
                mobile: 0,
                members: members.iter().map(|(id, _)| *id).collect(),
            };
            let mut facing = Vec2::ZERO;
            for (_, s) in &members {
                let info = self.type_info(s.unit_type);
                let structure = info.role == Role::Structure;
                let fresh = freshness(tick.saturating_sub(s.tick), structure);
                let hp_pct = if s.max_hp > 0 {
                    u64::from(s.hp) * 100 / u64::from(s.max_hp)
                } else {
                    100
                };
                front.value += u64::from(self.value_of(s.unit_type)) * hp_pct * fresh / 10_000;
                front.threat_any = front.threat_any.max(info.range);
                if info.anti_tank {
                    front.threat_armor = front.threat_armor.max(info.range);
                }
                if !structure {
                    front.mobile += 1;
                }
                if info.role == Role::Armor {
                    facing = facing + s.facing;
                }
            }
            let len = facing.length();
            if len > Fixed::ZERO {
                front.facing = facing.scale(Fixed::ONE / len);
            }
            fronts.push(front);
        }
        fronts.sort_by(|a, b| b.value.cmp(&a.value).then(a.members.cmp(&b.members)));
        fronts
    }

    /// Value of the enemy units known to be alive (seen and not seen
    /// destroyed, however old the sighting), and whether any of them is
    /// armor.
    pub(crate) fn known_enemy(&self, view: &PlayerView) -> (u64, bool) {
        let defeated =
            |owner: sim::PlayerId| view.enemy_players.iter().any(|&(p, d)| p == owner && d);
        let mut value = 0u64;
        let mut armor = false;
        for (_, s) in &self.intel.known {
            if defeated(s.owner) {
                continue;
            }
            let hp_pct = if s.max_hp > 0 {
                u64::from(s.hp) * 100 / u64::from(s.max_hp)
            } else {
                100
            };
            value += u64::from(self.value_of(s.unit_type)) * hp_pct / 100;
            armor |= self.type_info(s.unit_type).role == Role::Armor;
        }
        (value, armor)
    }
}

/// Cell size of [`Ground`] (m).
const GROUND_CELL_M: i32 = 20;

/// Where vehicles can stand, coarsely, from the public map (terrain types
/// that are passable at the center of every cell): staging points must not
/// lie in a lake.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Ground {
    width: u32,
    height: u32,
    /// Half the map size (m).
    half: (i32, i32),
    passable: Vec<bool>,
}

impl Ground {
    pub(crate) fn new(map: &sim::Map) -> Self {
        let Ok(rules) = sim::TerrainRules::builtin() else {
            return Self::default();
        };
        let size = map.size();
        let (w_m, h_m) = (size.x.to_num::<i32>(), size.y.to_num::<i32>());
        let width = u32::try_from(w_m / GROUND_CELL_M).unwrap_or(0);
        let height = u32::try_from(h_m / GROUND_CELL_M).unwrap_or(0);
        let half = (w_m / 2, h_m / 2);
        let mut passable = Vec::with_capacity((width * height) as usize);
        for j in 0..height {
            for i in 0..width {
                let p = Vec2::from_meters(
                    i32::try_from(i).unwrap_or(0) * GROUND_CELL_M + GROUND_CELL_M / 2 - half.0,
                    i32::try_from(j).unwrap_or(0) * GROUND_CELL_M + GROUND_CELL_M / 2 - half.1,
                );
                let ok = map.contains(p)
                    && rules
                        .types
                        .get(usize::from(map.ground_terrain_at(p)))
                        .is_some_and(|t| t.passable);
                passable.push(ok);
            }
        }
        Self {
            width,
            height,
            half,
            passable,
        }
    }

    /// Whether the cell of `p` is passable (unknown ground counts as
    /// passable).
    pub(crate) fn passable(&self, p: Vec2) -> bool {
        if self.passable.is_empty() {
            return true;
        }
        let i = (p.x.to_num::<i32>() + self.half.0).div_euclid(GROUND_CELL_M);
        let j = (p.y.to_num::<i32>() + self.half.1).div_euclid(GROUND_CELL_M);
        match (u32::try_from(i), u32::try_from(j)) {
            (Ok(i), Ok(j)) if i < self.width && j < self.height => {
                self.passable[(j * self.width + i) as usize]
            }
            _ => false,
        }
    }
}
