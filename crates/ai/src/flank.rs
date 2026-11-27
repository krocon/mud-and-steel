//! The computer player's answer to fire from the flank (prompt 110). The
//! simulation already turns every standing vehicle and gun towards hits
//! (`[flank]` in data/tactics.toml); here the AI decides when a position
//! is lost: an own gun or standing armored vehicle that loses hit points
//! to fire from the side or rear (more than 60° off its hull front, the
//! direction the simulation remembers) and cannot answer it (no enemy seen
//! within its range in that direction) or is badly hurt
//! ([`LOST_PCT`]) leaves:
//!
//! - a vehicle with smoke rounds lays smoke between itself and the fire;
//! - other armored vehicles back off [`WITHDRAW_M`] away from the fire, and
//!   their group leaves them alone for [`BUSY_TICKS`];
//! - a gun is towed [`RELOCATE_M`] back by the nearest free tractor (see
//!   `transport.rs`) and set up facing the fire; without a tractor it stays
//!   and keeps turning (guns have no crew that leaves them in the
//!   simulation).
//!
//! Each unit reacts at most once per [`COOLDOWN_TICKS`]: not every shot
//! makes the AI regroup. Ordinary commands only (`MoveTo`, `Barrage`,
//! `Tow`).

use crate::{Ai, OwnUnit, PlayerView, Role};
use serde::{Deserialize, Serialize};
use sim::{Command, Fixed, UnitId, Vec2};
use std::collections::BTreeMap;

/// Fire from more than 60° off the hull front counts as from the flank
/// (cosine in percent).
const FLANK_COS_PCT: i32 = 50;
/// An enemy within about 45° of the remembered direction is the shooter
/// (cosine in percent).
const SECTOR_COS_PCT: i32 = 70;
/// Below this share of its hit points (percent) the position is lost even
/// if the unit can answer.
const LOST_PCT: u32 = 40;
/// A unit reacts at most once in this many ticks (60 s).
const COOLDOWN_TICKS: u64 = 1200;
/// Groups leave a withdrawing vehicle alone this long (20 s).
const BUSY_TICKS: u64 = 400;
/// How far a vehicle backs off (m).
const WITHDRAW_M: i32 = 40;
/// How far a gun is moved back (m).
const RELOCATE_M: i32 = 40;
/// Smoke lands this far towards the fire (m), within the weapon's ranges.
const SMOKE_M: i32 = 40;
const SMOKE_RADIUS_M: u16 = 15;
const SMOKE_SALVOS: u16 = 2;

/// What the AI remembers about flanked units.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct FlankState {
    /// Hit points at the last decision, per own gun and armored vehicle.
    hp: BTreeMap<UnitId, u32>,
    /// Tick of the last reaction per unit.
    last: BTreeMap<UnitId, u64>,
    /// Withdrawing vehicles: id -> tick until which groups leave them alone.
    busy: BTreeMap<UnitId, u64>,
}

impl FlankState {
    /// The unit withdraws from a flanked position (groups leave it alone).
    pub(crate) fn busy(&self, id: UnitId, tick: u64) -> bool {
        self.busy.get(&id).is_some_and(|&until| tick < until)
    }
}

fn dot(a: Vec2, b: Vec2) -> Fixed {
    a.x * b.x + a.y * b.y
}

fn pct(v: i32) -> Fixed {
    Fixed::from_num(v) / 100
}

impl Ai {
    /// Guns and standing armored vehicles under fire from the flank.
    pub(crate) fn think_flank(&mut self, view: &PlayerView, out: &mut Vec<Command>) {
        let tick = view.tick;
        self.flank.busy.retain(|_, until| tick < *until);
        self.flank.last.retain(|_, at| tick < *at + COOLDOWN_TICKS);
        let mut hp = BTreeMap::new();
        for u in view.own.iter().filter(|u| u.alive() && u.carrier.is_none()) {
            let role = self.type_info(u.unit_type).role;
            if !matches!(role, Role::Gun | Role::Armor) {
                continue;
            }
            hp.insert(u.id, u.hp);
            let hurt = self
                .flank
                .hp
                .get(&u.id)
                .is_some_and(|&before| u.hp < before);
            let Some(dir) = u.alert else {
                continue;
            };
            if !hurt
                || u.target.is_some()
                || u.towing.is_some()
                || dot(u.facing, dir) >= pct(FLANK_COS_PCT)
                || self.flank.last.contains_key(&u.id)
            {
                continue;
            }
            let lost = u.hp * 100 < u.max_hp * LOST_PCT;
            if !lost && self.can_answer(view, u, dir) {
                continue;
            }
            if self.leave_position(view, u, role, dir, out) {
                self.flank.last.insert(u.id, tick);
            }
        }
        self.flank.hp = hp;
    }

    /// Whether `u` sees an enemy within its range in direction `dir`.
    fn can_answer(&self, view: &PlayerView, u: &OwnUnit, dir: Vec2) -> bool {
        let range = self.type_info(u.unit_type).range;
        view.enemies.iter().filter(|e| e.alive()).any(|e| {
            let d = e.position - u.position;
            let length = d.length();
            self.type_info(e.unit_type).role != Role::Structure
                && length > Fixed::ZERO
                && length <= range
                && dot(d, dir) >= length * pct(SECTOR_COS_PCT)
        })
    }

    /// Smoke, withdraw or relocate; returns whether a command was given.
    fn leave_position(
        &mut self,
        view: &PlayerView,
        u: &OwnUnit,
        role: Role,
        dir: Vec2,
        out: &mut Vec<Command>,
    ) -> bool {
        let info = self.type_info(u.unit_type);
        if let Some((min, max, true)) = info.indirect {
            let at = Fixed::from_num(SMOKE_M).max(min).min(max);
            out.push(Command::Barrage {
                units: vec![u.id],
                target: self.clamp_to_map(u.position + dir.scale(at)),
                radius_m: SMOKE_RADIUS_M,
                salvos: SMOKE_SALVOS,
                smoke: true,
            });
            return true;
        }
        let away = |m: i32| self.clamp_to_map(u.position - dir.scale(Fixed::from_num(m)));
        match role {
            Role::Gun => self.tow_away(view, u, away(RELOCATE_M), dir, out),
            _ => {
                let back = away(WITHDRAW_M);
                out.push(Command::MoveTo {
                    units: vec![u.id],
                    target: back,
                });
                self.flank.busy.insert(u.id, view.tick + BUSY_TICKS);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Difficulty;
    use sim::scenario::{builtin_map, demo_config};
    use sim::{FireMode, PlayerId, Sim, Spawn};

    const P0: PlayerId = PlayerId(0);

    /// The demo map with player 0's units `units` (type, meters from its
    /// first start unit) and nothing else of it; returns the AI, a view and
    /// the ids of the added units.
    fn setup(units: &[(&str, i32, i32)]) -> (Ai, PlayerView, Vec<UnitId>) {
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
        let names: Vec<&str> = units.iter().map(|u| u.0).collect();
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
        let ai = Ai::new(P0, Difficulty::Normal, &config, &map, 1);
        let view = PlayerView::new(&sim, P0);
        let ids = (first..first + u32::try_from(units.len()).unwrap())
            .map(UnitId)
            .collect();
        (ai, view, ids)
    }

    fn own(view: &mut PlayerView, id: UnitId) -> &mut OwnUnit {
        view.own.iter_mut().find(|u| u.id == id).unwrap()
    }

    const NORTH: Vec2 = Vec2 {
        x: Fixed::ZERO,
        y: Fixed::from_bits(-(1 << 32)),
    };
    const WEST: Vec2 = Vec2 {
        x: Fixed::from_bits(-(1 << 32)),
        y: Fixed::ZERO,
    };

    /// Hit from `dir` while facing north, between two decisions; the
    /// commands of the second.
    fn hit_twice(ai: &mut Ai, view: &mut PlayerView, id: UnitId, dir: Vec2) -> Vec<Command> {
        view.enemies.clear();
        let u = own(view, id);
        u.facing = NORTH;
        u.alert = Some(dir);
        let mut out = Vec::new();
        ai.think_flank(view, &mut out);
        assert!(out.is_empty());
        let u = own(view, id);
        u.hp = u.max_hp * 7 / 10;
        view.tick += 20;
        ai.think_flank(view, &mut out);
        out
    }

    #[test]
    fn a_tank_hit_from_the_side_by_an_unseen_enemy_backs_off() {
        let (mut ai, mut view, ids) = setup(&[("de_stug3_g", 0, 0)]);
        let stug = ids[0];
        let start = own(&mut view, stug).position;
        let out = hit_twice(&mut ai, &mut view, stug, WEST);
        let [Command::MoveTo { units, target }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(units, &vec![stug]);
        // Away from the fire (east).
        assert!(target.x > start.x + Fixed::from_num(30));
        assert!(ai.flank.busy(stug, view.tick));
        // Not again within the cooldown.
        own(&mut view, stug).hp -= 100;
        view.tick += 20;
        let mut again = Vec::new();
        ai.think_flank(&view, &mut again);
        assert!(again.is_empty());
    }

    #[test]
    fn fire_from_the_front_or_a_seen_shooter_changes_nothing() {
        let (mut ai, mut view, ids) = setup(&[("de_stug3_g", 0, 0)]);
        assert!(hit_twice(&mut ai, &mut view, ids[0], NORTH).is_empty());
        // Hit from the west by an enemy it sees within range: it answers.
        let (mut ai, mut view, ids) = setup(&[("de_stug3_g", 0, 0)]);
        let stug = own(&mut view, ids[0]).position;
        let mut seen = None;
        let config = demo_config();
        let sim = Sim::new(&config, builtin_map(), 1).unwrap();
        for e in PlayerView::new(&sim, PlayerId(1)).own {
            seen = Some(crate::SeenUnit {
                id: e.id,
                owner: PlayerId(1),
                unit_type: e.unit_type,
                position: stug - Vec2::from_meters(40, 0),
                facing: NORTH,
                aim: NORTH,
                hp: e.hp,
                max_hp: e.max_hp,
                garrisoned: false,
            });
            if ai.type_info(e.unit_type).role == Role::Armor {
                break;
            }
        }
        let enemy = seen.unwrap();
        let u = own(&mut view, ids[0]);
        u.facing = NORTH;
        u.alert = Some(WEST);
        let mut out = Vec::new();
        view.enemies = vec![enemy];
        ai.think_flank(&view, &mut out);
        own(&mut view, ids[0]).hp -= 100;
        view.tick += 20;
        ai.think_flank(&view, &mut out);
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn a_flanked_gun_is_towed_away_by_a_nearby_tractor() {
        let (mut ai, mut view, ids) = setup(&[("de_pak40", 0, 0), ("de_opel_blitz", 30, 20)]);
        let (gun, truck) = (ids[0], ids[1]);
        let out = hit_twice(&mut ai, &mut view, gun, WEST);
        assert_eq!(
            out,
            vec![Command::Tow {
                units: vec![truck],
                gun
            }]
        );
        assert!(ai.transport.relocate_of(gun).is_some());
    }

    #[test]
    fn a_flanked_gun_without_a_tractor_stays() {
        let (mut ai, mut view, ids) = setup(&[("de_pak40", 0, 0)]);
        assert!(hit_twice(&mut ai, &mut view, ids[0], WEST).is_empty());
    }
}
