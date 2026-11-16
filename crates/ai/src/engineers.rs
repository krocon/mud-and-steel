//! The computer player's engineers (prompt 035): with a base it builds one
//! engineer squad once its vehicle works stand; engineers lay anti-tank mines at chokepoints on the own
//! half of the map (the own end of bridges, else a point on the way to the
//! enemy) and clear hostile mines the player knows near its army. Normal
//! and Hard only. Ordinary commands only (`Produce`, `Lay`, `Clear`), and
//! only what the [`PlayerView`] shows (the map is public).

use crate::{Ai, Difficulty, OwnUnit, PlayerView, Role, centroid, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, LayKind, Map, Vec2};

/// Most mine sections the AI lays in a match.
const MAX_SECTIONS: u32 = 2;
/// Hostile mines within this distance (m) of the army or the home are
/// cleared.
const CLEAR_NEAR_M: i32 = 150;
/// A clearing order is repeated after this many ticks at the earliest.
const CLEAR_TICKS: u64 = 1200;
/// A production order for an engineer squad is repeated after this many
/// ticks at the earliest.
const PRODUCE_TICKS: u64 = 600;
/// Mines are laid this far (m) beyond the end of a bridge.
const BRIDGE_OFFSET_M: i32 = 16;

/// Engineer jobs of a computer player.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct EngineerState {
    /// Where mines go, best first.
    chokepoints: Vec<Vec2>,
    /// Sections ordered so far.
    laid: u32,
    /// Tick of the last clearing order and its point.
    cleared: Option<(u64, Vec2)>,
    /// Tick of the last production order for engineers.
    produced: Option<u64>,
}

impl EngineerState {
    /// Chokepoints from the public map: the own end of every intact bridge
    /// that is nearer to the own home than to the enemy's (nearest to the
    /// line between both homes first), else the point at 30 % of the way
    /// to the enemy.
    pub(crate) fn new(map: &Map, home: Vec2, enemy: Vec2) -> Self {
        let mut points: Vec<(Fixed, Vec2)> = map
            .bridges()
            .iter()
            .filter(|b| b.intact)
            .filter_map(|b| {
                let start = b.center - b.forward.scale(b.half_length);
                let end = b.center + b.forward.scale(b.half_length);
                let (near, outward) = if dist(start, home) <= dist(end, home) {
                    (start, -Fixed::ONE)
                } else {
                    (end, Fixed::ONE)
                };
                let point = near + b.forward.scale(outward * Fixed::from_num(BRIDGE_OFFSET_M));
                (dist(point, home) < dist(point, enemy) && map.contains(point))
                    .then(|| (line_distance(home, enemy, point), point))
            })
            .collect();
        points.sort_by_key(|&(d, p)| (d, p.x, p.y));
        let mut chokepoints: Vec<Vec2> = points.into_iter().map(|(_, p)| p).collect();
        let way = dist(home, enemy);
        if way > Fixed::ZERO {
            // `toward(enemy, home, -x)` lies x from home towards the enemy.
            chokepoints.push(toward(enemy, home, -(way * 3 / 10)));
            chokepoints.push(toward(enemy, home, -(way * 4 / 10)));
        }
        Self {
            chokepoints,
            ..Self::default()
        }
    }
}

/// Distance of `p` from the line through `a` and `b`.
fn line_distance(a: Vec2, b: Vec2, p: Vec2) -> Fixed {
    let d = b - a;
    let len = d.length();
    if len == Fixed::ZERO {
        return dist(a, p);
    }
    let r = p - a;
    ((r.x * d.y - r.y * d.x) / len).abs()
}

impl Ai {
    /// Production, mine laying and clearing of the engineers.
    pub(crate) fn command_engineers(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !matches!(self.difficulty, Difficulty::Normal | Difficulty::Hard) {
            return;
        }
        let engineers: Vec<OwnUnit> = view
            .own
            .iter()
            .filter(|u| {
                u.alive()
                    && u.carrier.is_none()
                    && self.type_info(u.unit_type).role == Role::Engineer
            })
            .copied()
            .collect();
        if engineers.is_empty() {
            self.produce_engineers(view, out);
            return;
        }
        let units: Vec<sim::UnitId> = engineers.iter().map(|u| u.id).collect();
        let tick = view.tick;
        // Own sites under way: the engineers are busy.
        let building = view
            .works
            .iter()
            .any(|w| w.owner == self.player && !w.built);
        // Hostile mines near the army or the home are cleared first.
        let army = centroid(
            view.own
                .iter()
                .filter(|u| {
                    u.alive()
                        && matches!(
                            self.type_info(u.unit_type).role,
                            Role::Armor | Role::Infantry
                        )
                })
                .map(|u| u.position),
        )
        .unwrap_or(self.home);
        let near = Fixed::from_num(CLEAR_NEAR_M);
        let center = centroid(engineers.iter().map(|u| u.position)).unwrap_or(self.home);
        let hostile = view
            .mines
            .iter()
            .filter(|m| !m.friendly)
            .filter(|m| dist(m.position, army) < near || dist(m.position, self.home) < near)
            .min_by_key(|m| (dist(m.position, center), m.id))
            .map(|m| m.position);
        let clearing = self
            .engineers
            .cleared
            .is_some_and(|(since, _)| tick < since + CLEAR_TICKS);
        if let Some(target) = hostile {
            if !clearing {
                self.engineers.cleared = Some((tick, target));
                out.push(Command::Clear { units, target });
            }
            return;
        }
        if building || clearing || self.engineers.laid >= MAX_SECTIONS {
            return;
        }
        let Some(&point) = self.engineers.chokepoints.get(self.engineers.laid as usize) else {
            return;
        };
        let cost =
            sim::WorksRules::builtin().map_or([0, 60, 0], |r| r.rule(sim::WorkKind::MinesAt).cost);
        if view.account.iter().zip(cost).any(|(&have, c)| have < c) {
            return;
        }
        self.engineers.laid += 1;
        let point = self.clamp_to_map(point);
        out.push(Command::Lay {
            units,
            kind: LayKind::MinesAt,
            from: point,
            to: point,
        });
    }

    /// With a base: one engineer squad from the first completed barracks.
    fn produce_engineers(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !Self::has_base(view)
            || self
                .engineers
                .produced
                .is_some_and(|t| view.tick < t + PRODUCE_TICKS)
        {
            return;
        }
        let engineer = (0..self.names.len() as u32).find(|&t| {
            self.type_info(t).role == Role::Engineer
                && view
                    .own
                    .first()
                    .and_then(|u| sim::economy::faction_of(self.name_of(u.unit_type)))
                    .is_some_and(|f| sim::economy::faction_of(self.name_of(t)) == Some(f))
        });
        let Some(unit_type) = engineer else {
            return;
        };
        let Some(barracks) = view
            .structures
            .iter()
            .find(|s| s.completed() && self.name_of(s.unit_type) == "base_barracks")
        else {
            return;
        };
        // Not before the vehicle works stand: the base comes first.
        if !view
            .structures
            .iter()
            .any(|s| s.completed() && self.name_of(s.unit_type) == "base_factory")
        {
            return;
        }
        if barracks.queue.contains(&unit_type) {
            return;
        }
        let Some(cost) = self.costs.get(unit_type as usize).copied().flatten() else {
            return;
        };
        if view.account.iter().zip(cost).any(|(&have, c)| have < c) {
            return;
        }
        self.engineers.produced = Some(view.tick);
        out.push(Command::Produce {
            building: barracks.unit,
            unit_type,
        });
    }
}
