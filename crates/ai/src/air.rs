//! The computer player's air support (prompt 033, Normal and Hard): calls
//! ground attacks and bombers on the largest visible enemy group,
//! reconnaissance on the enemy's base when it has seen nobody for a long
//! time, and builds light flak in its base once enemy aircraft appeared.
//! Ordinary commands only.

use crate::{Ai, Difficulty, PlayerView, Role, SeenUnit, artillery::largest_group, dist};
use serde::{Deserialize, Serialize};
use sim::{AirMission, Command, Fixed, Vec2};

/// A group needs this many units to be worth an air attack.
const MIN_GROUP: usize = 3;
/// A group of this many is worth the bombers.
const BOMBER_GROUP: usize = 5;
/// No air attack lands within this distance of own units (m): bomb
/// fragments, dispersion and the strafing strip.
const SAFE_M: i32 = 150;
/// Ticks between two calls of the computer player.
const CALL_TICKS: u64 = 400;
/// Ticks without any enemy in sight before a reconnaissance flight.
const QUIET_TICKS: u64 = 1200;

/// What the computer player remembers about the air war.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct AirState {
    last_call: Option<u64>,
    /// Tick in which an enemy was last seen.
    last_enemy: u64,
    /// Enemy aircraft were seen: build flak.
    threatened: bool,
}

impl Ai {
    pub(crate) fn think_air(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        if !matches!(self.difficulty, Difficulty::Normal | Difficulty::Hard) {
            return;
        }
        if view.enemy_aircraft > 0 {
            self.air.threatened = true;
        }
        self.build_flak(view, out);
        let enemies: Vec<&SeenUnit> = view
            .enemies
            .iter()
            .filter(|e| e.alive() && self.type_info(e.unit_type).role != Role::Structure)
            .collect();
        if !enemies.is_empty() {
            self.air.last_enemy = view.tick;
        }
        if self
            .air
            .last_call
            .is_some_and(|t| view.tick < t + CALL_TICKS)
        {
            return;
        }
        let own: Vec<Vec2> = view
            .own
            .iter()
            .filter(|u| u.alive())
            .map(|u| u.position)
            // Allies (prompt 037) are friends for the bombs too.
            .chain(view.allies.iter().filter(|a| a.alive()).map(|a| a.position))
            .collect();
        let call = if let Some((members, target)) = self.air_target(&enemies, &own) {
            let wanted: &[AirMission] = if members >= BOMBER_GROUP {
                &[AirMission::Bomb, AirMission::Attack, AirMission::Strafe]
            } else {
                &[AirMission::Attack, AirMission::Strafe]
            };
            wanted
                .iter()
                .find(|&&m| self.can_call(view, m))
                .map(|&m| (m, target))
        } else if view.tick >= self.air.last_enemy + QUIET_TICKS
            && self.can_call(view, AirMission::Recon)
        {
            let target = self
                .memory
                .values()
                .max_by_key(|s| s.tick)
                .map_or(self.enemy_home, |s| s.position);
            Some((AirMission::Recon, self.clamp_to_map(target)))
        } else {
            None
        };
        if let Some((mission, target)) = call {
            self.air.last_call = Some(view.tick);
            if mission == AirMission::Recon {
                // Do not fly again at once if nobody shows up.
                self.air.last_enemy = view.tick;
            }
            out.push(Command::CallAir { mission, target });
        }
    }

    /// The largest visible enemy group (members, center) away from own
    /// units.
    fn air_target(&self, enemies: &[&SeenUnit], own: &[Vec2]) -> Option<(usize, Vec2)> {
        let center = largest_group(enemies, own, SAFE_M)?;
        let radius = Fixed::from_num(crate::artillery::CLUSTER_M * 2);
        let members = enemies
            .iter()
            .filter(|e| dist(e.position, center) <= radius)
            .count();
        (members >= MIN_GROUP).then_some((members, center))
    }

    /// Whether `mission` can be called now, keeping a reserve for the base.
    fn can_call(&self, view: &PlayerView, mission: AirMission) -> bool {
        let Some(option) = view.air.iter().find(|o| o.mission == mission) else {
            return false;
        };
        if option.blocked.is_some() {
            return false;
        }
        // With a base, the economy comes first: only spend what is left
        // after the price of the call twice.
        !Ai::has_base(view)
            || view
                .account
                .iter()
                .zip(option.cost)
                .all(|(&have, c)| have >= c.saturating_mul(2))
    }

    /// Light flak in the base once enemy aircraft were seen (one per
    /// player).
    fn build_flak(&self, view: &PlayerView, out: &mut Vec<Command>) {
        if !self.air.threatened || !Ai::has_base(view) {
            return;
        }
        let Some(flak) = self.light_flak(view) else {
            return;
        };
        if view.structures.iter().any(|s| s.unit_type == flak) {
            return;
        }
        if let Some(&(_, position)) = view.sites.iter().find(|(t, _)| *t == flak) {
            out.push(Command::Build {
                unit_type: flak,
                position,
            });
        }
    }

    /// The faction's light anti-aircraft gun (a fortification of the
    /// economy rules).
    fn light_flak(&self, view: &PlayerView) -> Option<u32> {
        let faction = view
            .own
            .iter()
            .find_map(|u| sim::economy::faction_of(self.name_of(u.unit_type)))?;
        let catalog = sim::economy::builtin_catalog()?;
        let rules = sim::EconomyRules::builtin().ok()?;
        let id = rules
            .fortifications
            .iter()
            .flat_map(|f| &f.ids)
            .filter(|id| sim::economy::faction_of(id) == Some(faction))
            .find(|id| {
                catalog
                    .get(id)
                    .is_some_and(|e| e.category == sim::catalog::Category::GunAaLight)
            })?;
        self.names
            .iter()
            .position(|n| n == id)
            .and_then(|i| u32::try_from(i).ok())
    }
}
