//! Scouting (prompt 051, Normal and Hard): reconnaissance vehicles drive to
//! the part of the map the player knows least about, avoid ground where own
//! units were lost or enemy guns were seen, and turn back once they see the
//! enemy. Their sightings feed the situation picture (`intel.rs`), whose
//! knowledge ages; the knowledge grid below remembers when the player last
//! had eyes on every part of the map.

use crate::{Ai, OwnUnit, PlayerView, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, UnitId, Vec2};

/// Cell size of the knowledge grid (m).
pub(crate) const KNOW_CELL_M: i32 = 200;
/// A scout turns back once an enemy is this close (m).
const SHY_M: i32 = 350;
/// A scout's goal is renewed after this long at the latest (ticks).
const GOAL_TICKS: u64 = 1200;
/// Danger above which a cell is not scouted.
const DANGER_LIMIT: u64 = 400;

/// When the player last saw every cell of the map.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct Knowledge {
    width: i32,
    height: i32,
    half: (i32, i32),
    seen: Vec<u64>,
    /// Current goal of every scout and when it was set.
    goals: Vec<(UnitId, Vec2, u64)>,
}

impl Knowledge {
    pub(crate) fn new(half: Vec2) -> Self {
        let half = (half.x.to_num::<i32>(), half.y.to_num::<i32>());
        let width = (2 * half.0 + KNOW_CELL_M - 1) / KNOW_CELL_M;
        let height = (2 * half.1 + KNOW_CELL_M - 1) / KNOW_CELL_M;
        Self {
            width,
            height,
            half,
            seen: vec![0; usize::try_from(width * height).unwrap_or(0)],
            goals: Vec::new(),
        }
    }

    fn center(&self, i: i32, j: i32) -> Vec2 {
        Vec2::from_meters(
            i * KNOW_CELL_M + KNOW_CELL_M / 2 - self.half.0,
            j * KNOW_CELL_M + KNOW_CELL_M / 2 - self.half.1,
        )
    }

    /// Marks the cells within `sight` of `p` as seen at `tick`.
    pub(crate) fn mark(&mut self, p: Vec2, sight: Fixed, tick: u64) {
        if self.seen.is_empty() {
            return;
        }
        let r = sight.to_num::<i32>() / KNOW_CELL_M;
        let ci = (p.x.to_num::<i32>() + self.half.0).div_euclid(KNOW_CELL_M);
        let cj = (p.y.to_num::<i32>() + self.half.1).div_euclid(KNOW_CELL_M);
        for j in (cj - r).max(0)..=(cj + r).min(self.height - 1) {
            for i in (ci - r).max(0)..=(ci + r).min(self.width - 1) {
                if dist(self.center(i, j), p) <= sight {
                    self.seen[usize::try_from(j * self.width + i).unwrap_or(0)] = tick;
                }
            }
        }
    }
}

impl Ai {
    /// Marks what the own units see now.
    pub(crate) fn update_knowledge(&mut self, view: &PlayerView) {
        if self.knowledge.seen.is_empty() {
            self.knowledge = Knowledge::new(self.half);
        }
        let marks: Vec<(Vec2, Fixed)> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .map(|u| (u.position, self.type_info(u.unit_type).sight))
            .collect();
        for (p, sight) in marks {
            self.knowledge.mark(p, sight, view.tick);
        }
    }

    /// The cell to scout from `from`: the one seen longest ago, nearer to
    /// the enemy's home first, without known danger.
    fn scout_goal(&self, from: Vec2, tick: u64) -> Option<Vec2> {
        let k = &self.knowledge;
        let mut best: Option<(i64, Vec2)> = None;
        for j in 0..k.height {
            for i in 0..k.width {
                let c = k.center(i, j);
                if !self.standable(c) || self.danger_at(c, tick, true) > DANGER_LIMIT {
                    continue;
                }
                let age =
                    tick.saturating_sub(k.seen[usize::try_from(j * k.width + i).unwrap_or(0)]);
                let age = i64::try_from(age.min(1_000_000)).unwrap_or(0);
                let to_enemy = dist(c, self.enemy_home).to_num::<i64>();
                let way = dist(c, from).to_num::<i64>();
                let score = age - to_enemy - way / 2;
                if best.is_none_or(|(b, _)| score > b) {
                    best = Some((score, c));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Commands the reconnaissance vehicles.
    pub(crate) fn command_scouts(
        &mut self,
        units: &[OwnUnit],
        view: &PlayerView,
        out: &mut Vec<Command>,
    ) {
        let tick = view.tick;
        self.knowledge
            .goals
            .retain(|(id, _, _)| units.iter().any(|u| u.id == *id));
        for u in units {
            let shy = Fixed::from_num(SHY_M);
            let threat = view
                .enemies
                .iter()
                .filter(|e| e.alive() && dist(e.position, u.position) <= shy)
                .map(|e| e.position)
                .min_by_key(|&p| dist(p, u.position));
            if let Some(enemy) = threat {
                // Seen: back towards home, the sighting is reported.
                let back = self.clamp_to_map(toward(enemy, u.position, Fixed::from_num(200)));
                if u.target.is_none_or(|t| dist(t, back) > Fixed::from_num(60)) {
                    out.push(Command::MoveTo {
                        units: vec![u.id],
                        target: back,
                    });
                }
                self.knowledge.goals.retain(|(id, _, _)| *id != u.id);
                continue;
            }
            let current = self
                .knowledge
                .goals
                .iter()
                .find(|(id, _, _)| *id == u.id)
                .copied();
            let renew = current.is_none_or(|(_, g, t)| {
                dist(u.position, g) <= Fixed::from_num(60)
                    || tick > t + GOAL_TICKS
                    || u.target.is_none()
            });
            if renew && let Some(goal) = self.scout_goal(u.position, tick) {
                self.knowledge.goals.retain(|(id, _, _)| *id != u.id);
                self.knowledge.goals.push((u.id, goal, tick));
                out.push(Command::MoveTo {
                    units: vec![u.id],
                    target: goal,
                });
            }
        }
    }
}
