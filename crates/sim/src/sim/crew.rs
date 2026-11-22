//! Gun crews (prompt 177): the men serving a towed gun or heavy mortar
//! (catalog `crew`) as a count with hits of their own. Rules: `[gun_crew]`
//! in `data/combat.toml`, suppression as for squads (`[suppression]` in
//! `data/tactics.toml`).
//!
//! - Small rounds (below `small_caliber_mm_x10`) that hit the gun hit a man
//!   with `hit_pct` percent and never the gun; a shield stops them within
//!   `shield_half_deg` of the front ([`Sim::crew_hit`]).
//! - Fragments strike each man like an infantryman in the open, shield or
//!   not ([`Sim::crew_fragments`]).
//! - Near misses and hits suppress the crew (rounds the shield stops half);
//!   a pinned crew loads like a squad fires: with `pinned_fire_pct`.
//! - Fewer men reload more slowly ([`Sim::crew_reload_pct`]); without men
//!   the gun is abandoned and does not fire ([`Sim::crew_silenced`]).
//!
//! State: [`Crew`] per unit, hashed only for crewed guns.

use super::Sim;
use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::objects::sin_cos_deg;
use serde::{Deserialize, Serialize};

/// What the presentation shows of a gun crew (prompt 193): read only,
/// nothing of it goes back into the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrewView {
    /// Men left.
    pub men: u16,
    /// Men at full strength.
    pub full: u16,
    /// The crew is pinned down (it loads more slowly).
    pub pinned: bool,
    /// Ticks until the gun's next round (direct or indirect fire; 0: loaded).
    pub reload_ticks: u32,
}

/// The crew of a gun.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Crew {
    /// Men at full strength; 0: no crew (not a gun).
    pub(super) full: u16,
    /// Men left.
    pub(super) men: u16,
    /// Damage taken towards the next man lost.
    wounds: u32,
    /// Suppression in milli points.
    suppression: u32,
    /// Ticks since the last round near it.
    quiet: u32,
    pinned: bool,
}

impl Crew {
    pub(super) const fn new(full: u16) -> Self {
        Self {
            full,
            men: full,
            wounds: 0,
            suppression: 0,
            quiet: 0,
            pinned: false,
        }
    }

    pub(super) fn hash_into(self, h: &mut StableHasher) {
        if self.full == 0 {
            return;
        }
        h.write_u8(0xc7);
        for v in [
            u32::from(self.full),
            u32::from(self.men),
            self.wounds,
            self.suppression,
            self.quiet,
        ] {
            h.write_u32(v);
        }
        h.write_u8(u8::from(self.pinned));
    }

    /// A crewed gun whose men are all gone.
    pub(super) const fn abandoned(self) -> bool {
        self.full > 0 && self.men == 0
    }
}

impl Sim {
    /// Whether unit `i` has a crew that can be hit.
    fn crewed(&self, i: usize) -> bool {
        let c = self.units[i].crew;
        c.full > 0 && c.men > 0 && self.combat_rules.gun_crew.small_caliber_mm_x10 > 0
    }

    /// Whether gun `i` cannot fire because its crew is gone.
    pub(super) fn crew_silenced(&self, i: usize) -> bool {
        self.units[i].crew.abandoned()
    }

    /// Whether the crew of gun `i` is pinned down (it loads more slowly).
    pub(super) fn crew_pinned(&self, i: usize) -> bool {
        let c = self.units[i].crew;
        c.full > 0 && c.men > 0 && c.pinned
    }

    /// Reload time of gun `i` in percent of the normal one.
    pub(super) fn crew_reload_pct(&self, i: usize) -> u32 {
        let c = self.units[i].crew;
        if c.full == 0 {
            return 100;
        }
        let table = self.combat_rules.gun_crew.reload_pct;
        let quarters = (u32::from(c.men) * 4).div_ceil(u32::from(c.full));
        let by_men = table[(quarters.clamp(1, 4) - 1) as usize].max(100);
        // Pinned: like a squad that fires with `pinned_fire_pct` of its men.
        let fire_pct = self.tactics.rules.suppression.pinned_fire_pct.max(1);
        if c.pinned {
            by_men * 100 / fire_pct.min(100)
        } else {
            by_men
        }
    }

    /// Crew strength of unit `id` (men left, full); `None` for units
    /// without a crew.
    pub fn crew_of(&self, id: UnitId) -> Option<(u16, u16)> {
        let c = self.unit(id)?.crew;
        (c.full > 0).then_some((c.men, c.full))
    }

    /// The crew of unit `id` for the presentation (prompt 193); `None` for
    /// units without a crew.
    pub fn crew_view(&self, id: UnitId) -> Option<CrewView> {
        let u = self.unit(id)?;
        let c = u.crew;
        if c.full == 0 {
            return None;
        }
        let direct = u.combat.reload.iter().copied().max().unwrap_or(0);
        Some(CrewView {
            men: c.men,
            full: c.full,
            pinned: c.men > 0 && c.pinned,
            reload_ticks: direct.max(self.indirect_reload(id)),
        })
    }

    /// Debug and self-test (prompt 193): gun `id` loses `lose` men and is
    /// pinned down if `pin`, as if under fire. Not a player command.
    pub fn debug_gun_crew(&mut self, id: UnitId, lose: u16, pin: bool) {
        let Ok(j) = self.units.binary_search_by_key(&id, |u| u.id) else {
            return;
        };
        if self.units[j].crew.full == 0 {
            return;
        }
        if pin {
            let (_, _, pinned_at, ..) = self.tactics.crew_suppression(0);
            self.suppress_crew(j, pinned_at * 2);
        }
        if lose > 0 {
            let soldier = self.combat_rules.damage.soldier_hp.max(1);
            self.wound_crew(j, soldier.saturating_mul(u32::from(lose)));
        }
    }

    /// A round of `caliber` flying along `dir` hit gun `j`: returns whether
    /// it went to the crew (then the gun takes no damage). A small round
    /// within the shield's arc strikes steel; otherwise it hits a man with
    /// `hit_pct`, who takes `damage`.
    pub(super) fn crew_hit(
        &mut self,
        j: usize,
        caliber_mm_x10: u32,
        dir: Vec2,
        damage: u32,
    ) -> bool {
        let rules = self.combat_rules.gun_crew;
        if !self.crewed(j) || caliber_mm_x10 >= rules.small_caliber_mm_x10 {
            return false;
        }
        let ty = self.unit_types[self.units[j].unit_type as usize]
            .combat
            .gun_shield;
        // The shield sits on the gun and turns with it (aim direction).
        let facing = self.units[j].combat.aim;
        let length = dir.length();
        // The round comes from the front when it flies against the facing.
        let from_front = length > Fixed::ZERO
            && -(dir.x * facing.x + dir.y * facing.y) / length
                >= sin_cos_deg(u16::try_from(rules.shield_half_deg).unwrap_or(90)).1;
        let (_, hit_points, ..) = self.tactics.crew_suppression(caliber_mm_x10);
        if ty && from_front {
            // The shield takes it; the men duck behind it.
            self.suppress_crew(j, hit_points / 2);
            return true;
        }
        self.suppress_crew(j, hit_points);
        if self.rng.below(100) < rules.hit_pct {
            self.wound_crew(j, damage);
        }
        true
    }

    /// Fragments of a blast at `at` with `radius` and `infantry_pct` (the
    /// infantry chance at the center): each man of every crewed gun within
    /// the radius is struck with the chance falling off with distance.
    pub(super) fn crew_fragments(
        &mut self,
        at: Vec2,
        radius: Fixed,
        infantry_pct: u32,
        damage: u32,
    ) {
        if radius <= Fixed::ZERO {
            return;
        }
        for j in 0..self.units.len() {
            if !self.crewed(j) || !self.units[j].alive() {
                continue;
            }
            let d = (self.units[j].position - at).length();
            if d >= radius {
                continue;
            }
            let chance =
                (Fixed::from_num(infantry_pct * 100) * (radius - d) / radius).to_num::<u32>();
            let (_, hit_points, ..) = self.tactics.crew_suppression(0);
            self.suppress_crew(j, hit_points);
            for _ in 0..self.units[j].crew.men {
                if self.rng.below(10_000) < chance {
                    self.wound_crew(j, damage);
                }
            }
        }
    }

    /// A round of another player landed within the near-miss radius of
    /// crewed guns around `at`.
    pub(super) fn crew_near_miss(
        &mut self,
        at: Vec2,
        radius: Fixed,
        owner: crate::PlayerId,
        caliber_mm_x10: u32,
    ) {
        let (near_points, ..) = self.tactics.crew_suppression(caliber_mm_x10);
        for j in 0..self.units.len() {
            let u = &self.units[j];
            if !self.crewed(j) || !u.alive() || self.teams.allied(u.owner, owner) {
                continue;
            }
            if (u.position - at).length() <= radius {
                self.suppress_crew(j, near_points);
            }
        }
    }

    fn suppress_crew(&mut self, j: usize, points: u32) {
        let (_, _, pinned_at, ..) = self.tactics.crew_suppression(0);
        let c = &mut self.units[j].crew;
        c.suppression = c.suppression.saturating_add(points).min(pinned_at * 2);
        c.quiet = 0;
        if c.suppression >= pinned_at {
            c.pinned = true;
        }
    }

    fn wound_crew(&mut self, j: usize, damage: u32) {
        let soldier = self.combat_rules.damage.soldier_hp.max(1);
        let c = &mut self.units[j].crew;
        c.wounds = c.wounds.saturating_add(damage);
        while c.wounds >= soldier && c.men > 0 {
            c.wounds -= soldier;
            c.men -= 1;
        }
        if c.men == 0 {
            c.wounds = 0;
            c.pinned = false;
            let unit = &mut self.units[j];
            super::stop(unit);
            unit.combat.attack = None;
            unit.combat.auto_target = None;
        }
    }

    /// Recovery of every crew from suppression (once per tick).
    pub(super) fn recover_crews(&mut self) {
        let (_, _, _, unpinned_at, delay, recover) = self.tactics.crew_suppression(0);
        for unit in &mut self.units {
            let c = &mut unit.crew;
            if c.full == 0 || c.suppression == 0 {
                continue;
            }
            c.quiet = c.quiet.saturating_add(1);
            if c.quiet > delay {
                c.suppression = c.suppression.saturating_sub(recover);
            }
            if c.pinned && c.suppression < unpinned_at {
                c.pinned = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::FireMode;
    use crate::command::PlayerId;
    use crate::{Catalog, Scale, SimConfig, Spawn};

    /// A Pak 40 (unit 0, facing and aiming -y) and an enemy truck.
    fn pak() -> Sim {
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let types =
            crate::unit_types_from_catalog(&catalog, &scale, &["de_pak40", "su_zis5"]).unwrap();
        let map = crate::scenario::builtin_map();
        let spawn = |owner, unit: &str, x, y| Spawn {
            owner,
            unit_type: unit.into(),
            position: Vec2::from_meters(x, y),
            fire_mode: FireMode::HoldFire,
        };
        let config = SimConfig {
            tick_hz: 20,
            unit_types: types,
            spawns: vec![
                spawn(PlayerId(0), "de_pak40", 0, 0),
                spawn(PlayerId(1), "su_zis5", 0, -100),
            ],
            map: map.reference(),
            terrain: crate::TerrainRules::builtin().unwrap(),
            combat: crate::CombatRules::builtin().unwrap(),
            vision: crate::VisionRules::builtin().unwrap(),
            infantry: crate::InfantryRules::builtin().unwrap(),
            teams: crate::Teams::default(),
            weather: crate::WeatherSetup::default(),
            mission: None,
            variant: None,
        };
        let mut sim = Sim::new(&config, map, 3).unwrap();
        sim.units[0].facing = Vec2::new(Fixed::ZERO, -Fixed::ONE);
        sim.units[0].combat.aim = sim.units[0].facing;
        sim
    }

    /// Rifle rounds (`n`, flying along `dir`) on the gun; the men left.
    fn rounds(sim: &mut Sim, dir: Vec2, n: u32) -> u16 {
        for _ in 0..n {
            if sim.units[0].crew.men == 0 {
                break;
            }
            assert!(
                sim.crew_hit(0, 79, dir, 63),
                "a rifle round went to the gun"
            );
        }
        sim.units[0].crew.men
    }

    #[test]
    fn the_shield_covers_the_front_only() {
        let mut sim = pak();
        assert_eq!(sim.crew_of(UnitId(0)), Some((6, 6)));
        // From the front: rounds fly towards +y, against the facing.
        let front = Vec2::new(Fixed::ZERO, Fixed::ONE);
        assert_eq!(
            rounds(&mut sim, front, 60),
            6,
            "the shield let rounds through"
        );
        let mut sim = pak();
        let side = Vec2::new(-Fixed::ONE, Fixed::ZERO);
        assert!(rounds(&mut sim, side, 60) < 6, "no man hit from the side");
        // Heavy rounds go to the gun as before.
        assert!(!sim.crew_hit(0, 750, side, 600));
    }

    #[test]
    fn a_gun_without_crew_is_abandoned_and_silent() {
        let mut sim = pak();
        let side = Vec2::new(-Fixed::ONE, Fixed::ZERO);
        let mut n = 0;
        while sim.units[0].crew.men > 0 && n < 10_000 {
            sim.crew_hit(0, 79, side, 63);
            n += 1;
        }
        assert!(sim.units[0].crew.abandoned());
        assert!(sim.crew_silenced(0));
        assert_eq!(
            sim.fire_status(UnitId(0)).map(|s| s.0),
            Some(crate::FireStatus::Abandoned)
        );
        // Fewer men load more slowly; a full crew at the normal pace.
        let mut fresh = pak();
        assert_eq!(fresh.crew_reload_pct(0), 100);
        fresh.units[0].crew.men = 2;
        assert_eq!(fresh.crew_reload_pct(0), 250);
    }

    #[test]
    fn the_crew_view_follows_the_count() {
        let mut sim = pak();
        let full = sim.crew_view(UnitId(0)).unwrap();
        assert_eq!((full.men, full.full, full.pinned), (6, 6, false));
        assert_eq!(sim.crew_view(UnitId(1)), None, "a truck has no crew");
        sim.debug_gun_crew(UnitId(0), 2, true);
        let hit = sim.crew_view(UnitId(0)).unwrap();
        assert_eq!((hit.men, hit.pinned), (4, true));
        sim.debug_gun_crew(UnitId(0), 9, false);
        let gone = sim.crew_view(UnitId(0)).unwrap();
        assert_eq!((gone.men, gone.pinned), (0, false));
        assert!(sim.crew_silenced(0));
    }
}
