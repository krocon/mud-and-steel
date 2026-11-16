//! Air support and anti-aircraft fire (prompt 033): the rules of
//! `data/air.toml`, the aircraft of every faction and mission, the
//! anti-aircraft weapons of a unit type, and the public snapshot and event
//! types. The update itself (calls, flight, strafing, bombs, flak, crashes,
//! reconnaissance): `crate::sim::air`. See docs/ARCHITECTURE.md "Air
//! support and anti-aircraft fire".
//!
//! Aircraft are not units: a player calls a mission on a ground point
//! (`Command::CallAir`), the aircraft fly a fixed straight track from the
//! map edge over the point and out again, and nobody steers them.

use crate::Vec3;
use crate::catalog::{Catalog, Category, Entry, Group};
use crate::command::{PlayerId, UnitId};
use crate::economy::Resources;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The rules compiled into the simulation.
pub const BUILTIN_AIR: &str = include_str!("../../../data/air.toml");

/// `weapon` of the [`crate::CombatEvent::Shot`] events of anti-aircraft
/// fire (indirect fire uses 255).
pub const AA_WEAPON: u8 = 254;

/// The shooter of hits and kills by aircraft (strafing, bombs, crashes):
/// aircraft are no units.
pub const AIR_SHOOTER: UnitId = UnitId(u32::MAX);

/// What a player can call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AirMission {
    /// A reconnaissance pass: reveals the ground along the track.
    #[serde(rename = "recon")]
    Recon,
    /// Fighters strafe the target with their guns.
    #[serde(rename = "strafe")]
    Strafe,
    /// Attack aircraft drop their bombs on the target and strafe it.
    #[serde(rename = "attack")]
    Attack,
    /// Bombers drop sticks of bombs across the target.
    #[serde(rename = "bomb")]
    Bomb,
}

impl AirMission {
    pub const ALL: [Self; 4] = [Self::Recon, Self::Strafe, Self::Attack, Self::Bomb];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Recon => "recon",
            Self::Strafe => "strafe",
            Self::Attack => "attack",
            Self::Bomb => "bomb",
        }
    }

    /// Catalog category of the aircraft flying it.
    pub const fn category(self) -> Category {
        match self {
            Self::Recon => Category::AircraftRecon,
            Self::Strafe => Category::AircraftFighter,
            Self::Attack => Category::AircraftAttack,
            Self::Bomb => Category::AircraftBomber,
        }
    }

    pub const fn index(self) -> usize {
        self as usize
    }

    /// Whether the aircraft of this mission fire their guns at the target.
    pub const fn strafes(self) -> bool {
        matches!(self, Self::Strafe | Self::Attack)
    }

    /// Whether they drop bombs.
    pub const fn bombs(self) -> bool {
        matches!(self, Self::Attack | Self::Bomb)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AirError(pub String);

impl fmt::Display for AirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid air rules: {}", self.0)
    }
}

impl std::error::Error for AirError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionRules {
    pub mission: AirMission,
    pub cost: Resources,
    pub cooldown_s: u32,
    pub aircraft: u32,
    pub altitude_m: u32,
    pub attack_m: u32,
    pub dive_m: u32,
    pub lateral_m: u32,
    pub trail_m: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApproachRules {
    pub edge_margin_m: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HpRule {
    pub category: Category,
    pub hp: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrashRules {
    pub fall_ms: u32,
    pub caliber_mm_x10: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StrafeRules {
    pub before_m: u32,
    pub after_m: u32,
    pub burst_ticks: u32,
    pub lead_m: u32,
    pub width_m: u32,
    pub soldier_pct: u32,
    pub vehicle_pct: u32,
    pub open_damage_per_mm: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BombRules {
    pub caliber_mm_per_cbrt_kg: u32,
    pub near_miss_dm_per_cbrt_kg: u32,
    pub near_miss_damage: u32,
    pub stick_m: u32,
    pub error_attack_m: u32,
    pub error_bomb_m: u32,
    pub gravity_mm_per_s2: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReconRules {
    pub sight_real_m: u32,
}

/// Class of an anti-aircraft weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum FlakClass {
    /// Machine guns on an anti-aircraft mount (`aa = true`).
    #[serde(rename = "mg")]
    Mg,
    /// Weapons of `gun_aa_light`.
    #[serde(rename = "light")]
    Light,
    /// Weapons of `gun_aa_heavy`.
    #[serde(rename = "heavy")]
    Heavy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlakRules {
    pub class: FlakClass,
    pub range_real_m: u32,
    pub max_range_m: u32,
    pub hit_pct: u32,
    pub min_altitude_m: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlakCommon {
    pub ref_wingspan_m: u32,
    pub damage_per_mm: u32,
}

/// Rules of `data/air.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AirRules {
    #[serde(rename = "mission")]
    pub missions: Vec<MissionRules>,
    pub approach: ApproachRules,
    pub hp: Vec<HpRule>,
    pub crash: CrashRules,
    pub strafe: StrafeRules,
    pub bombs: BombRules,
    pub recon: ReconRules,
    pub flak: Vec<FlakRules>,
    pub flak_rules: FlakCommon,
}

impl AirRules {
    pub fn parse(text: &str) -> Result<Self, AirError> {
        let rules: Self = toml::from_str(text).map_err(|e| AirError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(AirError(format!("{name} = {value} outside {min}..={max}")))
            }
        };
        for m in AirMission::ALL {
            let count = rules.missions.iter().filter(|r| r.mission == m).count();
            if count != 1 {
                return Err(AirError(format!(
                    "mission '{}' must appear once, not {count} times",
                    m.name()
                )));
            }
        }
        for m in &rules.missions {
            check("mission.cooldown_s", m.cooldown_s, 0, 3600)?;
            check("mission.aircraft", m.aircraft, 1, 8)?;
            check("mission.altitude_m", m.altitude_m, 10, 5000)?;
            check("mission.attack_m", m.attack_m, 10, 5000)?;
            check("mission.dive_m", m.dive_m, 0, 5000)?;
            check("mission.lateral_m", m.lateral_m, 0, 1000)?;
            check("mission.trail_m", m.trail_m, 0, 5000)?;
            for c in m.cost {
                check("mission.cost", c, 0, 100_000)?;
            }
        }
        check(
            "approach.edge_margin_m",
            rules.approach.edge_margin_m,
            0,
            10_000,
        )?;
        for c in [
            Category::AircraftRecon,
            Category::AircraftFighter,
            Category::AircraftAttack,
            Category::AircraftBomber,
        ] {
            if !rules.hp.iter().any(|h| h.category == c) {
                return Err(AirError(format!("no hp for '{}'", c.code())));
            }
        }
        for h in &rules.hp {
            if h.category.group() != Group::Aircraft {
                return Err(AirError(format!("'{}' is no aircraft", h.category.code())));
            }
            check("hp.hp", h.hp, 1, 100_000)?;
        }
        let (c, s, b) = (&rules.crash, &rules.strafe, &rules.bombs);
        check("crash.fall_ms", c.fall_ms, 0, 60_000)?;
        check("crash.caliber_mm_x10", c.caliber_mm_x10, 0, 20_000)?;
        check("strafe.before_m", s.before_m, 0, 5000)?;
        check("strafe.after_m", s.after_m, 0, 5000)?;
        check("strafe.burst_ticks", s.burst_ticks, 1, 1000)?;
        check("strafe.lead_m", s.lead_m, 0, 2000)?;
        check("strafe.width_m", s.width_m, 1, 200)?;
        check("strafe.soldier_pct", s.soldier_pct, 0, 100)?;
        check("strafe.vehicle_pct", s.vehicle_pct, 0, 100)?;
        check("strafe.open_damage_per_mm", s.open_damage_per_mm, 0, 1000)?;
        check(
            "bombs.caliber_mm_per_cbrt_kg",
            b.caliber_mm_per_cbrt_kg,
            1,
            200,
        )?;
        check(
            "bombs.near_miss_dm_per_cbrt_kg",
            b.near_miss_dm_per_cbrt_kg,
            0,
            1000,
        )?;
        check("bombs.near_miss_damage", b.near_miss_damage, 0, 100_000)?;
        check("bombs.stick_m", b.stick_m, 0, 500)?;
        check("bombs.error_attack_m", b.error_attack_m, 0, 1000)?;
        check("bombs.error_bomb_m", b.error_bomb_m, 0, 1000)?;
        check("bombs.gravity_mm_per_s2", b.gravity_mm_per_s2, 1, 100_000)?;
        check("recon.sight_real_m", rules.recon.sight_real_m, 100, 20_000)?;
        for class in [FlakClass::Mg, FlakClass::Light, FlakClass::Heavy] {
            if rules.flak.iter().filter(|f| f.class == class).count() != 1 {
                return Err(AirError(format!("flak class {class:?} must appear once")));
            }
        }
        for f in &rules.flak {
            check("flak.range_real_m", f.range_real_m, 0, 30_000)?;
            check("flak.max_range_m", f.max_range_m, 1, 10_000)?;
            check("flak.hit_pct", f.hit_pct, 0, 100)?;
            check("flak.min_altitude_m", f.min_altitude_m, 0, 5000)?;
        }
        let f = &rules.flak_rules;
        check("flak_rules.ref_wingspan_m", f.ref_wingspan_m, 1, 100)?;
        check("flak_rules.damage_per_mm", f.damage_per_mm, 0, 10_000)?;
        Ok(rules)
    }

    /// The rules from `data/air.toml`, compiled in.
    pub fn builtin() -> Result<Self, AirError> {
        Self::parse(BUILTIN_AIR)
    }

    /// Rules of a mission (every mission has one, see [`Self::parse`]).
    pub fn mission(&self, mission: AirMission) -> &MissionRules {
        self.missions
            .iter()
            .find(|m| m.mission == mission)
            .expect("parse checks every mission")
    }

    /// Rules of a flak class (every class has one).
    pub fn flak(&self, class: FlakClass) -> &FlakRules {
        self.flak
            .iter()
            .find(|f| f.class == class)
            .expect("parse checks every class")
    }

    /// Hit points of an aircraft of `category`.
    pub fn hp(&self, category: Category) -> u32 {
        self.hp
            .iter()
            .find(|h| h.category == category)
            .map_or(1, |h| h.hp)
    }

    /// Caliber (tenths of mm) of the shell whose blast a bomb of `kg`
    /// kilograms makes.
    pub fn bomb_caliber(&self, kg: u32) -> u32 {
        cbrt_x10(kg) * self.bombs.caliber_mm_per_cbrt_kg
    }

    /// Radius (m) within which a bomb of `kg` damages armored vehicles.
    pub fn near_miss_radius(&self, kg: u32) -> Fixed {
        Fixed::from_num(cbrt_x10(kg) * self.bombs.near_miss_dm_per_cbrt_kg) / 100
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        h.write_u64(self.missions.len() as u64);
        for m in &self.missions {
            h.write_bytes(m.mission.name().as_bytes());
            for v in m.cost {
                h.write_u32(v);
            }
            for v in [
                m.cooldown_s,
                m.aircraft,
                m.altitude_m,
                m.attack_m,
                m.dive_m,
                m.lateral_m,
                m.trail_m,
            ] {
                h.write_u32(v);
            }
        }
        h.write_u64(self.hp.len() as u64);
        for x in &self.hp {
            h.write_bytes(x.category.code().as_bytes());
            h.write_u32(x.hp);
        }
        h.write_u64(self.flak.len() as u64);
        for f in &self.flak {
            h.write_u8(f.class as u8);
            for v in [f.range_real_m, f.max_range_m, f.hit_pct, f.min_altitude_m] {
                h.write_u32(v);
            }
        }
        let (a, c, s, b, r, f) = (
            &self.approach,
            &self.crash,
            &self.strafe,
            &self.bombs,
            &self.recon,
            &self.flak_rules,
        );
        for v in [
            a.edge_margin_m,
            c.fall_ms,
            c.caliber_mm_x10,
            s.before_m,
            s.after_m,
            s.burst_ticks,
            s.lead_m,
            s.width_m,
            s.soldier_pct,
            s.vehicle_pct,
            s.open_damage_per_mm,
            b.caliber_mm_per_cbrt_kg,
            b.near_miss_dm_per_cbrt_kg,
            b.near_miss_damage,
            b.stick_m,
            b.error_attack_m,
            b.error_bomb_m,
            b.gravity_mm_per_s2,
            r.sight_real_m,
            f.ref_wingspan_m,
            f.damage_per_mm,
        ] {
            h.write_u32(v);
        }
    }
}

/// Ten times the cube root of `kg` (rounded down): 250 → 62.
pub fn cbrt_x10(kg: u32) -> u32 {
    // (r / 10)³ <= kg  ⇔  r³ <= 1000 · kg
    let target = u64::from(kg) * 1000;
    let mut r = 0u64;
    while (r + 1).pow(3) <= target {
        r += 1;
    }
    u32::try_from(r).unwrap_or(u32::MAX)
}

/// A gun of an aircraft (strafing), from the catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AirGun {
    pub caliber_mm_x10: u32,
    pub count: u16,
    /// Cyclic rate, real (aircraft do not use the time factor).
    pub rate_rpm: u32,
    pub muzzle_velocity_mps: u32,
}

/// An aircraft type: the catalog entry a faction flies on a mission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AirType {
    /// Catalog id.
    pub name: String,
    /// Faction code (`de`, `su`, `us`).
    pub faction: String,
    pub mission: AirMission,
    /// Game meters per second (cruise, else maximum; × aircraft speed
    /// factor).
    pub speed_m_per_s: Fixed,
    pub wingspan_m: Fixed,
    pub length_m: Fixed,
    pub hp: u32,
    /// Guns fired at ground targets (strafing missions only).
    pub guns: Vec<AirGun>,
    /// Bombs per aircraft and nominal mass (bombing missions only).
    pub bombs: Option<(u16, u32)>,
}

impl AirType {
    /// The aircraft of `entry` flying `mission`, `None` if it cannot
    /// (no speed, or nothing to fire on an attack mission).
    pub fn from_catalog(
        entry: &Entry,
        mission: AirMission,
        rules: &AirRules,
        scale: &Scale,
    ) -> Option<Self> {
        let speed = entry
            .speed_cruise_kmh_x10
            .or(entry.speed_max_kmh_x10)
            .filter(|&s| s > 0)?;
        let guns: Vec<AirGun> = if mission.strafes() {
            entry
                .weapon
                .iter()
                // Rear guns of two-seaters point backwards.
                .filter(|w| w.bomb_kg.is_none() && !w.name.to_ascii_lowercase().contains("rear"))
                .filter_map(|w| {
                    Some(AirGun {
                        caliber_mm_x10: w.caliber_mm_x10.filter(|&c| c > 0)?,
                        count: w.count.max(1),
                        rate_rpm: w.rate_rpm.filter(|&r| r > 0)?,
                        muzzle_velocity_mps: w.muzzle_velocity_mps.unwrap_or(0),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let bombs = if mission.bombs() {
            entry
                .weapon
                .iter()
                .find_map(|w| w.bomb_kg.map(|kg| (w.count.max(1), kg)))
        } else {
            None
        };
        if (mission == AirMission::Strafe && guns.is_empty())
            || (mission == AirMission::Bomb && bombs.is_none())
            || (mission == AirMission::Attack && guns.is_empty() && bombs.is_none())
        {
            return None;
        }
        let m = |v: Option<u32>| v.map_or(Fixed::ZERO, |v| Fixed::from_num(v) / 1000);
        Some(Self {
            name: entry.id.clone(),
            faction: entry.faction.code().to_owned(),
            mission,
            speed_m_per_s: scale.game_speed(entry.category, speed),
            wingspan_m: m(entry.wingspan_mm),
            length_m: m(entry.length_mm),
            hp: rules.hp(entry.category),
            guns,
            bombs,
        })
    }

    /// Every aircraft type of the catalog: per faction (catalog order) and
    /// mission the first entry of the mission's category that can fly it.
    pub fn all(catalog: &Catalog, rules: &AirRules, scale: &Scale) -> Vec<Self> {
        let mut factions: Vec<&str> = Vec::new();
        for e in &catalog.entries {
            let code = e.faction.code();
            if e.category.group() == Group::Aircraft && !factions.contains(&code) {
                factions.push(code);
            }
        }
        let mut out = Vec::new();
        for faction in factions {
            for mission in AirMission::ALL {
                if let Some(t) = catalog
                    .entries
                    .iter()
                    .filter(|e| e.faction.code() == faction && e.category == mission.category())
                    .find_map(|e| Self::from_catalog(e, mission, rules, scale))
                {
                    out.push(t);
                }
            }
        }
        out
    }
}

/// An anti-aircraft weapon of a unit type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AaWeapon {
    pub class: FlakClass,
    pub caliber_mm_x10: u32,
    pub count: u16,
    /// Game meters (3D distance to the aircraft).
    pub range: Fixed,
    /// Catalog rate (cyclic for automatic weapons), real.
    pub rate_rpm: u32,
}

impl AaWeapon {
    /// The anti-aircraft weapons of a catalog entry: every weapon of an AA
    /// gun with `range_aa_m`, and every weapon with `aa = true`.
    pub fn from_catalog(entry: &Entry, rules: &AirRules, scale: &Scale) -> Vec<Self> {
        let gun_class = match entry.category {
            Category::GunAaLight => Some(FlakClass::Light),
            Category::GunAaHeavy => Some(FlakClass::Heavy),
            _ => None,
        };
        entry
            .weapon
            .iter()
            .filter_map(|w| {
                let caliber = w.caliber_mm_x10.filter(|&c| c > 0)?;
                let rate = w.rate_rpm.filter(|&r| r > 0)?;
                let (class, real) = match (gun_class, w.range_aa_m) {
                    (Some(class), Some(r)) => (class, r),
                    _ if w.aa => (FlakClass::Mg, rules.flak(FlakClass::Mg).range_real_m),
                    _ => return None,
                };
                let range = scale
                    .direct_fire_range_m(real)
                    .min(Fixed::from_num(rules.flak(class).max_range_m));
                Some(Self {
                    class,
                    caliber_mm_x10: caliber,
                    count: w.count.max(1),
                    range,
                    rate_rpm: rate,
                })
            })
            .collect()
    }
}

// --- Snapshot data --------------------------------------------------------------

/// An aircraft in the air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AircraftSnapshot {
    pub id: u32,
    pub owner: PlayerId,
    pub mission: AirMission,
    /// Index into [`crate::Sim::air_types`].
    pub kind: u16,
    pub position: Vec3,
    /// Meters per second.
    pub velocity: Vec3,
    pub hp: u32,
    pub max_hp: u32,
    /// Shot down, falling.
    pub crashing: bool,
    /// Its guns fire this tick.
    pub firing: bool,
}

/// A falling bomb.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BombSnapshot {
    pub id: u32,
    pub owner: PlayerId,
    pub position: Vec3,
    /// Meters per second.
    pub velocity: Vec3,
    pub target: Vec3,
    pub ticks_left: u32,
}

/// A called mission (for the caller's display).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortieSnapshot {
    pub id: u32,
    pub owner: PlayerId,
    pub mission: AirMission,
    pub target: Vec2,
    /// Where the aircraft enter and leave.
    pub entry: Vec2,
    pub exit: Vec2,
    /// Ticks until the first aircraft passes the target (0 once it did).
    pub ticks_to_target: u32,
    /// Aircraft still in the air.
    pub aircraft: u16,
}

/// Why a mission cannot be called now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirBlock {
    /// The player's faction has no aircraft for it.
    NoAircraft,
    /// Called recently.
    Cooldown,
    NotEnoughResources,
    /// The player is defeated or unknown.
    Defeated,
}

impl AirBlock {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoAircraft => "no aircraft",
            Self::Cooldown => "not ready",
            Self::NotEnoughResources => "not enough resources",
            Self::Defeated => "defeated",
        }
    }
}

/// A mission a player may call (the call bar).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AirOption {
    pub mission: AirMission,
    /// Catalog id of the aircraft, if the faction has one.
    pub aircraft: Option<String>,
    pub cost: Resources,
    /// Ticks until it can be called again, and the full cooldown.
    pub cooldown_left: u32,
    pub cooldown_total: u32,
    /// `None` if it can be called now.
    pub blocked: Option<AirBlock>,
}

/// What happened in the air during one tick, for effects, sound and the
/// HUD. Derived, not state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AirEvent {
    /// A player called a mission.
    Called {
        sortie: u32,
        owner: PlayerId,
        mission: AirMission,
        target: Vec2,
    },
    /// An anti-aircraft shot of `unit` burst at `at` near `aircraft`.
    Flak {
        unit: UnitId,
        aircraft: u32,
        at: Vec3,
        class: FlakClass,
        hit: bool,
    },
    /// A burst of an aircraft's guns: rounds land between `from` and `to`.
    Strafe {
        aircraft: u32,
        muzzle: Vec3,
        from: Vec3,
        to: Vec3,
        caliber_mm_x10: u32,
    },
    /// An aircraft released bomb `bomb`.
    Released { aircraft: u32, bomb: u32, at: Vec3 },
    /// An aircraft lost its last hit point to the fire of `by`.
    ShotDown {
        aircraft: u32,
        owner: PlayerId,
        by: UnitId,
        at: Vec3,
    },
    /// A shot-down aircraft hit the ground (or left the map falling).
    Crashed {
        aircraft: u32,
        owner: PlayerId,
        kind: u16,
        at: Vec3,
        facing: Vec2,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = AirRules::builtin().unwrap();
        assert_eq!(rules.mission(AirMission::Bomb).aircraft, 3);
        assert!(AirRules::parse(&BUILTIN_AIR.replace("hit_pct = 3", "hit_pct = 300")).is_err());
        assert!(
            AirRules::parse(&BUILTIN_AIR.replace("mission = \"bomb\"", "mission = \"recon\""))
                .is_err()
        );
    }

    #[test]
    fn cube_roots_and_bomb_blasts() {
        assert_eq!(cbrt_x10(1000), 100);
        assert_eq!(cbrt_x10(250), 62);
        assert_eq!(cbrt_x10(0), 0);
        let rules = AirRules::builtin().unwrap();
        // SC 250: 6.2 × 25 mm = 155 mm.
        assert_eq!(rules.bomb_caliber(250), 1550);
    }

    #[test]
    fn every_faction_flies_every_mission() {
        let catalog = Catalog::builtin().unwrap();
        let rules = AirRules::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let all = AirType::all(&catalog, &rules, &scale);
        for faction in ["de", "su", "us"] {
            for mission in AirMission::ALL {
                assert!(
                    all.iter()
                        .any(|t| t.faction == faction && t.mission == mission),
                    "{faction} {mission:?}: {all:#?}"
                );
            }
        }
        let ju87 = all.iter().find(|t| t.name == "de_ju87_d5").unwrap();
        assert_eq!(ju87.bombs, Some((1, 500)));
        assert!(!ju87.guns.is_empty());
        // Cruise 319 km/h real × 1/2 = 44.3 m/s.
        assert_eq!(ju87.speed_m_per_s.to_num::<i32>(), 44);
        let b25 = all.iter().find(|t| t.name == "us_b25j").unwrap();
        assert!(b25.guns.is_empty(), "bombers do not strafe");
        assert_eq!(b25.bombs, Some((6, 227)));
    }

    #[test]
    fn flak_ranges_come_from_the_catalog() {
        let catalog = Catalog::builtin().unwrap();
        let rules = AirRules::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let aa = |id: &str| AaWeapon::from_catalog(catalog.get(id).unwrap(), &rules, &scale);
        let flak38 = aa("de_flak38");
        assert_eq!(flak38.len(), 1);
        assert_eq!(flak38[0].class, FlakClass::Light);
        // 2200 m real × 1/4.
        assert_eq!(flak38[0].range, Fixed::from_num(550));
        // 8000 m × 1/4 = 2000 m, capped at 1500 m.
        assert_eq!(aa("de_flak36")[0].range, Fixed::from_num(1500));
        let sherman = aa("us_m4a3_75");
        assert_eq!(sherman.len(), 1);
        assert_eq!(sherman[0].class, FlakClass::Mg);
        assert!(aa("de_pz4_h").is_empty());
    }
}
