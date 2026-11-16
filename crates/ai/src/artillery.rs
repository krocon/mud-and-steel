//! The computer player's artillery (prompt 032, Hard only): mortar teams
//! and self-propelled guns follow the army at a distance, fire barrages on
//! the largest visible enemy group in range, and lay smoke between the army
//! and the enemy when the enemy comes into sight. Ordinary commands only.

use crate::{Ai, OwnUnit, PlayerView, Role, SeenUnit, centroid, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, UnitId, Vec2};

/// Enemies within this distance of each other count as one group (m).
pub(crate) const CLUSTER_M: i32 = 25;
/// No barrage lands within this distance of own troops (m): blast radius
/// plus dispersion of a heavy shell.
const SAFE_M: i32 = 60;
/// Radius and salvos of a barrage.
const BARRAGE_RADIUS_M: u16 = 15;
const BARRAGE_SALVOS: u16 = 3;
/// Salvos of a smoke screen.
const SMOKE_SALVOS: u16 = 2;
/// Ticks between two barrages, and between two smoke screens.
const BARRAGE_TICKS: u64 = 200;
const SMOKE_TICKS: u64 = 800;
/// The screen lies this far in front of the enemy, towards the army (m).
const SMOKE_AHEAD_M: i32 = 40;
/// The artillery keeps this far behind the army's center (m).
const BEHIND_M: i32 = 80;
/// Distance at which the artillery counts as in place (m).
const PLACED_M: i32 = 40;

/// When the artillery fired last.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct FireState {
    barrage_tick: Option<u64>,
    smoke_tick: Option<u64>,
    /// Tick of the last time no enemy was in sight.
    quiet_tick: u64,
    /// Where the artillery was last sent.
    moved_to: Option<Vec2>,
}

impl Ai {
    pub(crate) fn command_artillery(
        &mut self,
        units: &[OwnUnit],
        view: &PlayerView,
        out: &mut Vec<Command>,
    ) {
        let enemies: Vec<&SeenUnit> = view
            .enemies
            .iter()
            .filter(|e| e.alive() && self.type_info(e.unit_type).role != Role::Structure)
            .collect();
        let army: Vec<Vec2> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .filter(|u| {
                matches!(
                    self.type_info(u.unit_type).role,
                    Role::Armor | Role::Infantry
                )
            })
            .map(|u| u.position)
            .collect();
        let army_center = centroid(army.iter().copied());
        if enemies.is_empty() {
            self.fire.quiet_tick = view.tick;
            self.follow_army(units, army_center, view, out);
            return;
        }
        let enemy_center = centroid(enemies.iter().map(|e| e.position)).unwrap_or_default();
        // Smoke first: the enemy just came into sight and the army moves up.
        let smoke_due = self
            .fire
            .smoke_tick
            .is_none_or(|t| view.tick >= t + SMOKE_TICKS)
            && (view.tick <= self.fire.quiet_tick + SMOKE_TICKS / 2
                || self.assault_starting(view.tick));
        let mut busy: Vec<UnitId> = Vec::new();
        if smoke_due && let Some(army_center) = army_center {
            let screen = toward(army_center, enemy_center, -Fixed::from_num(SMOKE_AHEAD_M));
            let screen = self.clamp_to_map(screen);
            let smokers = self.in_range(units, screen, true);
            if !smokers.is_empty() {
                out.push(Command::Barrage {
                    units: smokers.clone(),
                    target: screen,
                    radius_m: BARRAGE_RADIUS_M,
                    salvos: SMOKE_SALVOS,
                    smoke: true,
                });
                self.fire.smoke_tick = Some(view.tick);
                busy = smokers;
            }
        }
        let barrage_due = self
            .fire
            .barrage_tick
            .is_none_or(|t| view.tick >= t + BARRAGE_TICKS);
        let own: Vec<Vec2> = view
            .own
            .iter()
            .filter(|u| u.alive() && self.type_info(u.unit_type).role != Role::Artillery)
            .map(|u| u.position)
            // Allies (prompt 037) are friends for the fragments too.
            .chain(view.allies.iter().filter(|a| a.alive()).map(|a| a.position))
            .collect();
        if barrage_due && let Some(target) = largest_group(&enemies, &own, SAFE_M) {
            let mut gunners = self.in_range(units, target, false);
            gunners.retain(|id| !busy.contains(id));
            if !gunners.is_empty() {
                out.push(Command::Barrage {
                    units: gunners,
                    target,
                    radius_m: BARRAGE_RADIUS_M,
                    salvos: BARRAGE_SALVOS,
                    smoke: false,
                });
                self.fire.barrage_tick = Some(view.tick);
                return;
            }
        }
        if busy.is_empty() {
            self.follow_army(units, army_center, view, out);
        }
    }

    /// Own artillery units that reach `target` (with smoke ammunition if
    /// `smoke`).
    fn in_range(&self, units: &[OwnUnit], target: Vec2, smoke: bool) -> Vec<UnitId> {
        units
            .iter()
            .filter(|u| {
                self.type_info(u.unit_type)
                    .indirect
                    .is_some_and(|(min, max, has_smoke)| {
                        let d = dist(u.position, target);
                        d >= min && d <= max && (has_smoke || !smoke)
                    })
            })
            .map(|u| u.id)
            .collect()
    }

    /// Keeps the artillery behind the army (towards home), or at home.
    fn follow_army(
        &mut self,
        units: &[OwnUnit],
        army_center: Option<Vec2>,
        view: &PlayerView,
        out: &mut Vec<Command>,
    ) {
        let Some(center) = centroid(units.iter().map(|u| u.position)) else {
            return;
        };
        let place = match army_center {
            Some(army) if dist(army, self.home) > Fixed::from_num(BEHIND_M) => {
                toward(self.enemy_home, army, Fixed::from_num(BEHIND_M))
            }
            _ if Ai::has_base(view) => self.gather_point(view),
            _ => self.home,
        };
        let place = self.clamp_to_map(place);
        let placed = dist(center, place) <= Fixed::from_num(PLACED_M);
        let same = self
            .fire
            .moved_to
            .is_some_and(|p| dist(p, place) <= Fixed::from_num(PLACED_M));
        if placed || same {
            return;
        }
        // Units still firing a barrage keep their place.
        let busy = |u: &&OwnUnit| u.target.is_none() && self.recently_fired(view.tick);
        let ids: Vec<UnitId> = units.iter().filter(|u| !busy(u)).map(|u| u.id).collect();
        if ids.is_empty() {
            return;
        }
        out.push(Command::MoveTo {
            units: ids,
            target: place,
        });
        self.fire.moved_to = Some(place);
    }

    fn recently_fired(&self, tick: u64) -> bool {
        self.fire
            .barrage_tick
            .into_iter()
            .chain(self.fire.smoke_tick)
            .any(|t| tick < t + BARRAGE_TICKS)
    }
}

/// Center of the visible enemy with the most other enemies within
/// [`CLUSTER_M`] (ties: the lower id), with no own unit (`own`) within
/// `safe_m` of it: fragments hurt friends too.
pub(crate) fn largest_group(enemies: &[&SeenUnit], own: &[Vec2], safe_m: i32) -> Option<Vec2> {
    let radius = Fixed::from_num(CLUSTER_M);
    let safe = Fixed::from_num(safe_m);
    let mut best: Option<((usize, std::cmp::Reverse<UnitId>), Vec2)> = None;
    for e in enemies {
        let members: Vec<Vec2> = enemies
            .iter()
            .filter(|o| dist(o.position, e.position) <= radius)
            .map(|o| o.position)
            .collect();
        let center = centroid(members.iter().copied()).unwrap_or(e.position);
        if own.iter().any(|&p| dist(p, center) <= safe) {
            continue;
        }
        let key = (members.len(), std::cmp::Reverse(e.id));
        if best.as_ref().is_none_or(|(b, _)| key > *b) {
            best = Some((key, center));
        }
    }
    best.map(|(_, p)| p)
}
