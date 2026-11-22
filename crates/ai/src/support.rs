//! Support vehicles from the second line (prompt 195): the Sturmtiger, the
//! assault howitzers and the self-propelled guns the AI does not lead as
//! artillery never advance alone and never lead.
//!
//! Which units: [`crate::TypeInfo::support`] — an assault gun whose main
//! gun has at least 100 mm (Sturmpanzer IV, Sturmtiger, StuIG 33B, SU-122,
//! M4A3 (105): the `assault_howitzer` kind of prompt 062) and an indirect-fire
//! vehicle that is not commanded as artillery on this difficulty (Wespe,
//! Priest, Calliope … on Easy and Normal). A rule on the unit's kind, not on
//! one id. Not on [`crate::Difficulty::HardOld`].
//!
//! Per support vehicle, every think:
//!
//! - **Escort:** at least [`MIN_ESCORTS`] effective own armor or infantry
//!   units within [`ESCORT_M`] (a squad and a tank, or two squads).
//! - **Second line:** its distance to the objective (the nearest enemy in
//!   sight, else the nearest remembered one, else where the escorts drive,
//!   else the enemy's home) stays at
//!   least [`BEHIND_M`] more than that of the foremost escort; farther
//!   forward it backs off, more than [`LAG_M`] behind it moves up.
//! - **Alone:** without escort it drives back to the gathering point (base)
//!   or the own start and waits there until escorts come by.
//! - **Targets:** direct-fire support vehicles fire only in return
//!   (`FireMode::ReturnFire`, the simulation's `[targeting]` picks among
//!   those that fire at it) and get `Attack` orders only on targets worth a
//!   heavy round within their range from where they stand: buildings and
//!   fortifications, towed guns (anti-tank guns), squads in a house, and
//!   groups of at least [`CLUSTER_N`] enemies within [`CLUSTER_M`]. Not lone
//!   squads, not trucks, halftracks or scout cars. Indirect support keeps
//!   firing at will (the simulation's indirect rules already spare lone
//!   soldiers, `indirect_min_soldiers`).
//! - **Reload:** after a shot (its ammunition went down) it backs off
//!   [`BACK_M`] more until the main gun is loaded again.
//!
//! [provisional] all distances. Game meters: the Sturmtiger's 38 cm reaches
//! 62.5 m in the game (500 m typical × 1/8), so the 80–150 m (real) of the
//! prompt would put it out of range; 12–40 m behind the lead is two to six
//! of its lengths.

use crate::{Ai, OwnUnit, PlayerView, Role, SeenUnit, dist};
use serde::{Deserialize, Serialize};
use sim::{Command, FireMode, Fixed, UnitId, Vec2};
use std::collections::BTreeMap;

/// Escorts count within this distance of the support vehicle (m).
const ESCORT_M: i32 = 80;
/// Escorts needed to advance.
const MIN_ESCORTS: usize = 2;
/// At least this much farther from the objective than the foremost escort
/// (m).
const BEHIND_M: i32 = 12;
/// Moves up when more than this behind the foremost escort (m).
const LAG_M: i32 = 40;
/// Backs off this much more while reloading (m).
const BACK_M: i32 = 15;
/// A move is repeated only when its goal moved this far (m).
const RESEND_M: i32 = 12;
/// Alone, it waits once this close to the rear point (m).
const REAR_NEAR_M: i32 = 25;
/// A group of enemies: this many within this distance of each other (m).
const CLUSTER_N: usize = 3;
const CLUSTER_M: i32 = 25;
/// An enemy squad this close to a house is taken to be in it (m).
const HOUSE_NEAR_M: i32 = 15;
/// Out of range of a target worth it, it drives to this share of its range
/// from it (percent), if that stays behind the lead.
const APPROACH_PCT: i64 = 90;
/// ... and for that shot it may stand this little behind the lead (m).
const SHOT_BEHIND_M: i32 = 5;
/// Without enemies in sight or memory the last one counts this long
/// (ticks, 60 s).
const FRONT_KEEP_TICKS: u64 = 1200;
/// Escorts driving to points farther than this show where the army heads
/// before anything was seen (m).
const GOAL_M: i32 = 30;
/// Targets within this share of the range from where it stands (percent):
/// an attack order never makes it drive forward.
const FIRE_RANGE_PCT: i64 = 95;

/// What the AI remembers per support vehicle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Support {
    /// Ammunition at the last think.
    ammo: Option<u8>,
    /// Loading after a shot until this tick.
    reload_until: u64,
    /// Goal of the last move.
    sent: Option<Vec2>,
    /// Target of the last attack order.
    attacking: Option<UnitId>,
    /// Fire mode set to return fire.
    mode_set: bool,
    /// The last enemy position it kept its distance to, and when.
    front: Option<(Vec2, u64)>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SupportState {
    units: BTreeMap<UnitId, Support>,
}

/// Where a support vehicle should be right now.
enum Plan {
    /// Drive to this point (cancels an attack).
    Move(Vec2),
    /// Fire at this enemy from where it stands.
    Attack(UnitId),
    /// Stay as it is.
    Hold,
}

impl Ai {
    /// The support vehicles of `units` (armor on this difficulty).
    pub(crate) fn is_support(&self, u: &OwnUnit) -> bool {
        self.type_info(u.unit_type).support
    }

    /// Commands the support vehicles `units` (alive, not riding, not busy
    /// elsewhere).
    pub(crate) fn command_support(
        &mut self,
        units: &[OwnUnit],
        view: &PlayerView,
        out: &mut Vec<Command>,
    ) {
        let tick = view.tick;
        let mut state = std::mem::take(&mut self.support);
        state
            .units
            .retain(|id, _| units.iter().any(|u| u.id == *id));
        let escorts: Vec<OwnUnit> = view
            .own
            .iter()
            .filter(|u| {
                let info = self.type_info(u.unit_type);
                matches!(info.role, Role::Armor | Role::Infantry)
                    && !info.support
                    && self.effective(u)
            })
            .copied()
            .collect();
        let enemies: Vec<SeenUnit> = view
            .enemies
            .iter()
            .filter(|e| e.alive() && !self.type_info(e.unit_type).crew)
            .copied()
            .collect();
        let mut return_fire = Vec::new();
        for u in units {
            let info = self.type_info(u.unit_type);
            let s = state.units.entry(u.id).or_default();
            let direct = info.indirect.is_none();
            if direct && !s.mode_set && u.fire_mode == FireMode::FireAtWill {
                s.mode_set = true;
                return_fire.push(u.id);
            }
            // A shot: its ammunition went down.
            if let (Some(before), Some(now)) = (s.ammo, u.stock.ammo_pct)
                && now < before
                && direct
            {
                s.reload_until = tick + u64::from(info.reload_ticks);
                s.attacking = None;
            }
            s.ammo = u.stock.ammo_pct;
            let plan = self.support_plan(u, &escorts, &enemies, view, s);
            match plan {
                Plan::Move(goal) => {
                    let goal = self.clamp_to_map(goal);
                    let resend = s.attacking.is_some()
                        || s.sent
                            .is_none_or(|p| dist(p, goal) > Fixed::from_num(RESEND_M))
                        || (u.target.is_none()
                            && dist(u.position, goal) > Fixed::from_num(RESEND_M));
                    if resend {
                        s.sent = Some(goal);
                        s.attacking = None;
                        out.push(Command::MoveTo {
                            units: vec![u.id],
                            target: goal,
                        });
                    }
                }
                Plan::Attack(target) => {
                    if s.attacking != Some(target) {
                        s.attacking = Some(target);
                        s.sent = None;
                        out.push(Command::Attack {
                            units: vec![u.id],
                            target,
                        });
                    }
                }
                Plan::Hold => {}
            }
        }
        if !return_fire.is_empty() {
            out.push(Command::SetFireMode {
                units: return_fire,
                mode: FireMode::ReturnFire,
            });
        }
        self.support = state;
    }

    fn support_plan(
        &self,
        u: &OwnUnit,
        escorts: &[OwnUnit],
        enemies: &[SeenUnit],
        view: &PlayerView,
        s: &mut Support,
    ) -> Plan {
        let info = self.type_info(u.unit_type);
        if u.stock.immobile {
            return Plan::Hold;
        }
        let near = Fixed::from_num(ESCORT_M);
        let mine: Vec<&OwnUnit> = escorts
            .iter()
            .filter(|e| dist(e.position, u.position) <= near)
            .collect();
        if mine.len() < MIN_ESCORTS || !self.effective(u) {
            // Alone (or out of ammunition): back to the rear and wait.
            let rear = if Self::has_base(view) {
                self.gather_point(view)
            } else {
                self.home
            };
            if dist(u.position, rear) <= Fixed::from_num(REAR_NEAR_M) {
                return Plan::Hold;
            }
            return Plan::Move(rear);
        }
        let objective = self.support_objective(u, &mine, enemies, s, view.tick);
        let lead = mine
            .iter()
            .map(|e| dist(e.position, objective))
            .min()
            .unwrap_or_default();
        let own = dist(u.position, objective);
        let behind = Fixed::from_num(BEHIND_M);
        // The goal: on its own bearing from the objective, behind the lead.
        let at = |extra: Fixed| {
            let d = u.position - objective;
            let len = d.length();
            if len == Fixed::ZERO {
                return u.position;
            }
            objective + d.scale((lead + extra) / len)
        };
        let reloading = view.tick < s.reload_until && info.indirect.is_none();
        if reloading {
            let goal = at(behind + Fixed::from_num(BACK_M));
            return if own + Fixed::from_num(3) < lead + behind + Fixed::from_num(BACK_M) {
                Plan::Move(goal)
            } else {
                Plan::Hold
            };
        }
        // For a shot it may come up to `SHOT_BEHIND_M` behind the lead.
        let shot = lead + Fixed::from_num(SHOT_BEHIND_M);
        let direct = info.indirect.is_none();
        if direct && own >= shot {
            let reach = info.range * FIRE_RANGE_PCT / 100;
            if let Some(t) = self.support_target(u, enemies, s.attacking, reach) {
                return Plan::Attack(t);
            }
        }
        if own < lead + behind {
            return Plan::Move(at(behind + Fixed::from_num(5)));
        }
        if direct {
            // A target worth it in sight but out of range: closer, as long
            // as it stays behind the lead.
            if let Some(t) = self.support_target(u, enemies, None, Fixed::MAX)
                && let Some(e) = enemies.iter().find(|e| e.id == t)
            {
                let d = u.position - e.position;
                let len = d.length();
                if len > Fixed::ZERO {
                    let goal = e.position + d.scale(info.range * APPROACH_PCT / 100 / len);
                    if dist(goal, objective) >= shot && dist(goal, u.position) > Fixed::from_num(5)
                    {
                        return Plan::Move(goal);
                    }
                }
            }
        }
        if own > lead + Fixed::from_num(LAG_M) {
            return Plan::Move(at(behind + Fixed::from_num(10)));
        }
        Plan::Hold
    }

    /// The point the army heads for, seen from support vehicle `u`: the
    /// nearest enemy in sight, else the nearest remembered one, else for
    /// [`FRONT_KEEP_TICKS`] the last of these (escorts that pull back from
    /// it take the support vehicle along), else where its escorts `mine`
    /// drive to, else the enemy's home.
    fn support_objective(
        &self,
        u: &OwnUnit,
        mine: &[&OwnUnit],
        enemies: &[SeenUnit],
        s: &mut Support,
        tick: u64,
    ) -> Vec2 {
        let enemy = enemies
            .iter()
            .min_by_key(|e| (dist(e.position, u.position), e.id))
            .map(|e| e.position)
            .or_else(|| {
                self.memory
                    .iter()
                    .min_by_key(|(id, m)| (dist(m.position, u.position), **id))
                    .map(|(_, m)| m.position)
            });
        if let Some(p) = enemy {
            s.front = Some((p, tick));
            return p;
        }
        if let Some((p, since)) = s.front
            && tick < since + FRONT_KEEP_TICKS
        {
            return p;
        }
        let far = Fixed::from_num(GOAL_M);
        let goals = mine
            .iter()
            .filter_map(|e| e.target.filter(|&t| dist(t, e.position) > far));
        crate::centroid(goals).unwrap_or(self.enemy_home)
    }

    /// The enemy in range worth a heavy round: buildings, guns, squads in a
    /// house, groups; the current target while it stays worth it.
    fn support_target(
        &self,
        u: &OwnUnit,
        enemies: &[SeenUnit],
        current: Option<UnitId>,
        reach: Fixed,
    ) -> Option<UnitId> {
        let cluster = Fixed::from_num(CLUSTER_M);
        let house = Fixed::from_num(HOUSE_NEAR_M);
        let worth = |e: &SeenUnit| -> u8 {
            let t = self.type_info(e.unit_type);
            match t.role {
                Role::Structure | Role::Gun => return 4,
                Role::Transport | Role::Scout | Role::Unarmed => return 0,
                Role::Infantry | Role::Engineer
                    if self
                        .houses
                        .iter()
                        .any(|&(_, p)| dist(p, e.position) <= house) =>
                {
                    return 3;
                }
                _ => {}
            }
            let group = enemies
                .iter()
                .filter(|o| dist(o.position, e.position) <= cluster)
                .count();
            if group >= CLUSTER_N { 2 } else { 0 }
        };
        let candidates: Vec<(&SeenUnit, u8)> = enemies
            .iter()
            .filter(|e| dist(e.position, u.position) <= reach)
            .map(|e| (e, worth(e)))
            .filter(|&(_, w)| w > 0)
            .collect();
        if let Some(c) = current
            && candidates.iter().any(|(e, _)| e.id == c)
        {
            return Some(c);
        }
        candidates
            .into_iter()
            .max_by_key(|&(e, w)| {
                (
                    w,
                    std::cmp::Reverse(dist(e.position, u.position)),
                    std::cmp::Reverse(e.id),
                )
            })
            .map(|(e, _)| e.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Difficulty;
    use sim::scenario::{builtin_map, demo_config};
    use sim::{PlayerId, Sim, Spawn};

    const P0: PlayerId = PlayerId(0);

    /// The demo map with player 0's units `units` (type, meters from its
    /// first start unit) and nothing else of it; returns the AI, a view,
    /// the ids of the added units and the type index of a Soviet rifle
    /// squad.
    fn setup(units: &[(&str, i32, i32)]) -> (Ai, PlayerView, Vec<UnitId>, u32) {
        let mut config = demo_config();
        let own = config
            .spawns
            .iter()
            .find(|s| s.owner == P0)
            .unwrap()
            .position;
        config.spawns.retain(|s| s.owner != P0);
        let catalog = sim::Catalog::builtin().unwrap();
        let scale = sim::Scale::builtin().unwrap();
        let mut names: Vec<&str> = units.iter().map(|u| u.0).collect();
        names.push("su_inf_rifle_squad");
        names.dedup();
        let squad = u32::try_from(config.unit_types.len() + names.len() - 1).unwrap();
        config
            .unit_types
            .extend(sim::unit_types_from_catalog(&catalog, &scale, &names).unwrap());
        let first = u32::try_from(config.spawns.len()).unwrap();
        for &(name, x, y) in units {
            config.spawns.push(Spawn {
                owner: P0,
                unit_type: name.into(),
                position: own + Vec2::from_meters(x, y),
                fire_mode: FireMode::FireAtWill,
            });
        }
        let map = builtin_map();
        let sim = Sim::new(&config, map.clone(), 1).unwrap();
        let ai = Ai::new(P0, Difficulty::Hard, &config, &map, 1);
        let view = PlayerView::new(&sim, P0);
        let ids = (first..first + u32::try_from(units.len()).unwrap())
            .map(UnitId)
            .collect();
        (ai, view, ids, squad)
    }

    fn own(view: &mut PlayerView, id: UnitId) -> &mut OwnUnit {
        view.own.iter_mut().find(|u| u.id == id).unwrap()
    }

    /// A Sturmtiger with a Panzer IV and a rifle squad in front of it; an
    /// enemy squad `ahead` meters north of the Sturmtiger.
    fn column(ahead: i32) -> (Ai, PlayerView, [UnitId; 3], SeenUnit) {
        let (ai, mut view, ids, squad) = setup(&[
            ("de_sturmtiger", 0, 0),
            ("de_pz4_h", -8, -25),
            ("de_inf_rifle_squad", 8, -25),
        ]);
        let st = own(&mut view, ids[0]).position;
        let enemy = SeenUnit {
            id: UnitId(9000),
            owner: PlayerId(1),
            unit_type: squad,
            position: st - Vec2::from_meters(0, ahead),
            facing: Vec2::ZERO,
            aim: Vec2::ZERO,
            hp: 100,
            max_hp: 100,
        };
        view.enemies = vec![enemy];
        (ai, view, [ids[0], ids[1], ids[2]], enemy)
    }

    fn support_commands(ai: &mut Ai, view: &PlayerView) -> Vec<Command> {
        let units: Vec<OwnUnit> = view
            .own
            .iter()
            .filter(|u| u.alive() && ai.is_support(u))
            .copied()
            .collect();
        let mut out = Vec::new();
        ai.command_support(&units, view, &mut out);
        out
    }

    #[test]
    fn assault_howitzers_are_support_but_not_on_hard_old() {
        let (ai, view, ids, _) = setup(&[
            ("de_sturmtiger", 0, 0),
            ("de_sturmpanzer4", 10, 0),
            ("de_pz4_h", 20, 0),
            ("de_stug3_g", 30, 0),
        ]);
        let support: Vec<bool> = ids
            .iter()
            .map(|id| ai.is_support(view.own.iter().find(|u| u.id == *id).unwrap()))
            .collect();
        assert_eq!(support, [true, true, false, false]);
        let info = ai.type_info(view.own.iter().find(|u| u.id == ids[0]).unwrap().unit_type);
        // One round a minute, at time factor 2: 30 s.
        assert_eq!(info.reload_ticks, 600);
        let config = demo_config();
        let old = Ai::new(P0, Difficulty::HardOld, &config, &builtin_map(), 1);
        assert!(old.types.iter().all(|t| !t.support));
    }

    #[test]
    fn behind_its_escorts_it_holds_and_fires_only_in_return() {
        let (mut ai, view, ids, _) = column(100);
        let out = support_commands(&mut ai, &view);
        assert_eq!(
            out,
            vec![Command::SetFireMode {
                units: vec![ids[0]],
                mode: FireMode::ReturnFire
            }]
        );
    }

    #[test]
    fn in_front_of_its_escorts_it_backs_off() {
        let (mut ai, mut view, ids, enemy) = column(100);
        // 40 m forward: 15 m in front of the escorts.
        own(&mut view, ids[0]).position = enemy.position + Vec2::from_meters(0, 60);
        let out = support_commands(&mut ai, &view);
        let Some(Command::MoveTo { units, target }) = out.first() else {
            panic!("{out:?}");
        };
        assert_eq!(units, &vec![ids[0]]);
        let lead = dist(own(&mut view, ids[1]).position, enemy.position);
        assert!(dist(*target, enemy.position) >= lead + Fixed::from_num(BEHIND_M));
    }

    #[test]
    fn without_escorts_it_drives_back_and_waits() {
        let (mut ai, mut view, ids, _) = column(100);
        ai.home = own(&mut view, ids[0]).position + Vec2::from_meters(0, 100);
        assert!(
            !support_commands(&mut ai, &view)
                .iter()
                .any(|c| matches!(c, Command::MoveTo { .. }))
        );
        // Both escorts fall: back to the start.
        own(&mut view, ids[1]).hp = 0;
        own(&mut view, ids[2]).hp = 0;
        view.tick += 20;
        let out = support_commands(&mut ai, &view);
        assert_eq!(
            out,
            vec![Command::MoveTo {
                units: vec![ids[0]],
                target: ai.clamp_to_map(ai.home)
            }]
        );
        // There it waits.
        own(&mut view, ids[0]).position = ai.home;
        view.tick += 20;
        assert!(support_commands(&mut ai, &view).is_empty());
    }

    #[test]
    fn a_lone_squad_is_no_target_a_squad_in_a_house_is() {
        // The enemy squad within its range (55 of 62.5 m), the escorts
        // 30 m from it.
        let (mut ai, mut view, ids, enemy) = column(55);
        ai.houses.clear();
        let out = support_commands(&mut ai, &view);
        assert!(
            !out.iter().any(|c| matches!(c, Command::Attack { .. })),
            "{out:?}"
        );
        ai.houses = vec![(0, enemy.position + Vec2::from_meters(5, 0))];
        view.tick += 20;
        let out = support_commands(&mut ai, &view);
        assert_eq!(
            out,
            vec![Command::Attack {
                units: vec![ids[0]],
                target: enemy.id
            }]
        );
        // The shot: it backs off until loaded again.
        own(&mut view, ids[0]).stock.ammo_pct = Some(92);
        view.tick += 20;
        let out = support_commands(&mut ai, &view);
        let Some(Command::MoveTo { target, .. }) = out.first() else {
            panic!("{out:?}");
        };
        let st = own(&mut view, ids[0]).position;
        assert!(dist(*target, enemy.position) > dist(st, enemy.position));
    }
}
