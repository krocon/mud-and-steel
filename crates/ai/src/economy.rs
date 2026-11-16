//! The computer player's economy (prompt 023): a fixed build order, filled
//! production queues and attack waves. Like the rest of the AI it only reads
//! its [`PlayerView`] and answers with ordinary commands.

use crate::{Ai, Difficulty, PlayerView, centroid, dist, toward};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, Vec2};

/// What the AI builds, in this order (base building ids, or the MG bunker
/// of the player's faction). The AT gun position of prompt 023 is gone:
/// AT guns are produced and towed since prompt 034.
const BUILD_ORDER: [Step; 5] = [
    Step::Building("base_barracks"),
    Step::Building("base_depot"),
    Step::Building("base_factory"),
    Step::Building("base_depot"),
    Step::Bunker,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Building(&'static str),
    /// The faction's MG bunker.
    Bunker,
}

/// Production mixes per difficulty: indices into the production list of
/// the building (`data/economy.toml`), repeated in order.
fn mix(difficulty: Difficulty, vehicles: bool) -> &'static [usize] {
    match (difficulty, vehicles) {
        (Difficulty::Easy, false) => &[0, 0, 1],
        (Difficulty::Easy, true) => &[1, 2],
        // Hard also builds mortar teams (4, prompt 032). Not the
        // self-propelled gun (vehicle 5): saving up for it starved the tank
        // production and lost base matches against Normal.
        // Normal adds an AT gun (7, prompt 034), towed by its halftrack
        // (1).
        (Difficulty::Hard, false) => &[0, 3, 1, 4, 0],
        (Difficulty::Hard, true) => &[2, 3, 2, 4],
        (_, false) => &[0, 1, 0, 3],
        (_, true) => &[2, 2, 3, 1, 7],
    }
}

/// Units an attack wave needs before it leaves the base.
pub(crate) const fn wave_size(difficulty: Difficulty) -> usize {
    match difficulty {
        Difficulty::Easy => 8,
        Difficulty::Hard => 5,
        _ => 6,
    }
}

/// Economy state of a computer player.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct EcoState {
    /// Next entry of the mix per producing building kind (infantry,
    /// vehicles).
    next: [usize; 2],
    /// Rally points already set, by building.
    rallied: Vec<sim::UnitId>,
    /// Per role: the wave has left the base (until it shrinks to half).
    pub(crate) launched: Vec<crate::Role>,
}

impl Ai {
    /// Whether the player plays with a base (owns a structure).
    pub(crate) fn has_base(view: &PlayerView) -> bool {
        !view.structures.is_empty()
    }

    /// Where the army gathers: in front of the headquarters, towards the
    /// enemy.
    pub(crate) fn gather_point(&self, view: &PlayerView) -> Vec2 {
        let base = view
            .structures
            .first()
            .and_then(|s| view.own.iter().find(|u| u.id == s.unit))
            .map_or(self.home, |u| u.position);
        self.clamp_to_map(toward(self.enemy_home, base, -Fixed::from_num(45)))
    }

    /// Construction, production and rally points.
    pub(crate) fn think_economy(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !Self::has_base(view) {
            return;
        }
        // Resources kept for the next building of the build order, so
        // production does not starve the base.
        let reserve = self.build(view, out).unwrap_or_default();
        let gather = self.gather_point(view);
        for s in view.structures.iter().filter(|s| s.completed()) {
            let name = self.name_of(s.unit_type);
            let kind = match name {
                "base_barracks" => 0,
                "base_factory" => 1,
                _ => continue,
            };
            if !self.eco.rallied.contains(&s.unit) {
                self.eco.rallied.push(s.unit);
                out.push(Command::SetRallyPoint {
                    building: s.unit,
                    point: gather,
                });
            }
            let wanted = if self.difficulty == Difficulty::Hard {
                2
            } else {
                1
            };
            if s.queue.len() >= wanted {
                continue;
            }
            let options = self.production_list(kind == 1, view);
            if options.is_empty() {
                continue;
            }
            let order = mix(self.difficulty, kind == 1);
            let index = order[self.eco.next[kind] % order.len()].min(options.len() - 1);
            let unit_type = options[index];
            let Some(cost) = self.costs.get(unit_type as usize).copied().flatten() else {
                continue;
            };
            // Save up for the next unit of the mix rather than skipping it.
            let affordable = view
                .account
                .iter()
                .zip(cost)
                .zip(reserve)
                .all(|((&have, c), r)| have >= c.saturating_add(r));
            if affordable {
                self.eco.next[kind] += 1;
                out.push(Command::Produce {
                    building: s.unit,
                    unit_type,
                });
            }
        }
    }

    /// The next missing entry of the build order, placed on the spot the
    /// view offers. Returns its price while it is not placed yet.
    fn build(&self, view: &PlayerView, out: &mut Vec<Command>) -> Option<sim::Resources> {
        let mut seen: Vec<u32> = Vec::new();
        for step in BUILD_ORDER {
            let Some(unit_type) = self.step_type(step, view) else {
                continue;
            };
            seen.push(unit_type);
            let wanted = seen.iter().filter(|&&t| t == unit_type).count();
            let have = view
                .structures
                .iter()
                .filter(|s| s.unit_type == unit_type)
                .count();
            if have >= wanted {
                continue;
            }
            if let Some(&(_, position)) = view.sites.iter().find(|(t, _)| *t == unit_type) {
                out.push(Command::Build {
                    unit_type,
                    position,
                });
                return None;
            }
            // One step at a time: later steps wait for this one.
            return self.costs.get(unit_type as usize).copied().flatten();
        }
        None
    }

    fn step_type(&self, step: Step, view: &PlayerView) -> Option<u32> {
        let faction = self.faction(view)?;
        let name = match step {
            Step::Building(name) => name.to_owned(),
            Step::Bunker => self.bunker(faction)?,
        };
        self.type_index(&name)
    }

    /// The faction of the player, from its own catalog units.
    fn faction(&self, view: &PlayerView) -> Option<&'static str> {
        view.own
            .iter()
            .find_map(|u| sim::economy::faction_of(self.name_of(u.unit_type)))
    }

    /// The faction's MG bunker (a fortification of the rules).
    fn bunker(&self, faction: &str) -> Option<String> {
        let rules = sim::EconomyRules::builtin().ok()?;
        rules
            .fortifications
            .iter()
            .flat_map(|f| &f.ids)
            .filter(|id| sim::economy::faction_of(id) == Some(faction))
            .find(|id| id.contains("bunker"))
            .cloned()
    }

    /// Unit types of the faction's infantry or vehicle list, in rule order.
    fn production_list(&self, vehicles: bool, view: &PlayerView) -> Vec<u32> {
        let Some(faction) = self.faction(view) else {
            return Vec::new();
        };
        let Ok(rules) = sim::EconomyRules::builtin() else {
            return Vec::new();
        };
        let list = if vehicles { "vehicles" } else { "infantry" };
        rules
            .produced(list, faction)
            .iter()
            .filter_map(|id| self.type_index(id))
            .collect()
    }

    fn type_index(&self, name: &str) -> Option<u32> {
        self.names.iter().position(|n| n == name).map(|i| i as u32)
    }

    pub(crate) fn name_of(&self, unit_type: u32) -> &str {
        self.names
            .get(unit_type as usize)
            .map_or("", String::as_str)
    }

    /// Where a group without enemies in sight goes while its wave is not
    /// complete: the nearest supply point the player does not hold (if it
    /// is nearer to the own base than to the enemy's), else the gathering
    /// point. `None` once the wave has left.
    pub(crate) fn hold_point(
        &mut self,
        role: crate::Role,
        units: usize,
        center: Vec2,
        view: &PlayerView,
    ) -> Option<Vec2> {
        if !Self::has_base(view) {
            return None;
        }
        let wave = wave_size(self.difficulty);
        let launched = self.eco.launched.contains(&role);
        if launched && units * 2 >= wave {
            return None;
        }
        if units >= wave {
            self.eco.launched.push(role);
            return None;
        }
        self.eco.launched.retain(|&r| r != role);
        let gather = self.gather_point(view);
        let base = centroid(
            view.structures
                .iter()
                .filter_map(|s| view.own.iter().find(|u| u.id == s.unit))
                .map(|u| u.position),
        )
        .unwrap_or(self.home);
        let point = view
            .supply
            .iter()
            .filter(|p| p.owner != Some(view.player))
            .filter(|p| dist(p.position, base) < dist(p.position, self.enemy_home))
            .min_by_key(|p| (dist(p.position, center), p.name.clone()))
            .map(|p| p.position);
        Some(point.unwrap_or(gather))
    }
}
