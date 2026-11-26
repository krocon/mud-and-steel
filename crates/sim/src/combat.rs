//! Combat data: rules (`data/combat.toml`), weapons and armor of unit types
//! (from the catalog), events and projectiles in snapshots. The combat
//! update itself lives in `sim/combat.rs`. See docs/ARCHITECTURE.md "Combat".

use crate::catalog::{Category, Entry, Group};
use crate::command::UnitId;
use crate::hash::StableHasher;
use crate::math::{Fixed, Vec2};
use crate::scale::Scale;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The combat rules compiled into the simulation.
pub const BUILTIN_COMBAT: &str = include_str!("../../../data/combat.toml");

/// A point in space: `x`, `y` on the ground plane (as [`Vec2`]), `z` the
/// height in meters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
}

impl Vec3 {
    pub const fn new(x: Fixed, y: Fixed, z: Fixed) -> Self {
        Self { x, y, z }
    }

    pub const fn ground(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Euclidean length with an integer square root (exact floor).
    pub fn length(self) -> Fixed {
        let sq = |v: Fixed| {
            let b = i128::from(v.to_bits()).unsigned_abs();
            b * b
        };
        // Three squares below 2^126 each fit into u128.
        let root = (sq(self.x) + sq(self.y) + sq(self.z)).isqrt();
        Fixed::from_bits(i64::try_from(root).unwrap_or(i64::MAX))
    }
}

/// Square root of a non-negative `Fixed` (exact floor; 0 for negatives).
pub fn fixed_sqrt(value: Fixed) -> Fixed {
    let bits = u128::try_from(value.to_bits()).unwrap_or(0);
    Fixed::from_bits(i64::try_from((bits << 32).isqrt()).unwrap_or(i64::MAX))
}

/// `(sin, cos)` of a small angle in radians (Taylor series to the 7th / 8th
/// order: exact to well below 1e-6 up to 1 rad).
pub fn sin_cos_rad(x: Fixed) -> (Fixed, Fixed) {
    let x2 = x * x;
    let sin = x * (Fixed::ONE - x2 / 6 * (Fixed::ONE - x2 / 20 * (Fixed::ONE - x2 / 42)));
    let cos = Fixed::ONE
        - x2 / 2 * (Fixed::ONE - x2 / 12 * (Fixed::ONE - x2 / 30 * (Fixed::ONE - x2 / 56)));
    (sin, cos)
}

/// Radians of `deg` degrees.
pub fn deg_to_rad(deg: Fixed) -> Fixed {
    deg * Fixed::PI / 180
}

/// Whether units fire on their own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FireMode {
    /// Pick targets automatically (default).
    #[default]
    FireAtWill,
    /// Fire only at targets of an attack order.
    HoldFire,
    /// Fire automatically only at units that fired at this one (prompt
    /// 176), and at targets of an attack order.
    ReturnFire,
}

impl FireMode {
    /// The next mode of the fire key: fire at will, return fire, hold fire.
    pub const fn next(self) -> Self {
        match self {
            Self::FireAtWill => Self::ReturnFire,
            Self::ReturnFire => Self::HoldFire,
            Self::HoldFire => Self::FireAtWill,
        }
    }

    /// 0 fire at will, 1 hold fire, 2 return fire (bridge, hash).
    pub const fn code(self) -> u8 {
        match self {
            Self::FireAtWill => 0,
            Self::HoldFire => 1,
            Self::ReturnFire => 2,
        }
    }

    pub const fn from_code(code: i64) -> Self {
        match code {
            1 => Self::HoldFire,
            2 => Self::ReturnFire,
            _ => Self::FireAtWill,
        }
    }
}

// --- Rules -----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AimOverride {
    pub category: Category,
    pub turret_deg_per_s: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AimRules {
    pub turret_deg_per_s: u32,
    pub hull_deg_per_s: u32,
    #[serde(default, rename = "category")]
    pub categories: Vec<AimOverride>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireRules {
    pub retarget_ticks: u32,
    pub automatic_rpm: u32,
    pub automatic_sustained_pct: u32,
    pub attack_range_pct: u32,
    pub muzzle_height_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispersionRules {
    pub mrad_x10: u32,
    pub moving_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallisticsRules {
    pub gravity_mm_per_s2: u32,
    pub max_flight_range_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PenetrationRules {
    pub reference_velocity_mps: u32,
    pub loss_pct_per_km: u32,
    pub heat_below_velocity_mps: u32,
    pub heat_pct: u32,
    pub spread_pct: u32,
    pub min_cos_pct: u32,
    pub turret_height_pct: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageRules {
    pub vehicle_hp: u32,
    pub vehicle_pct_per_mm2: u32,
    pub soldier_hp: u32,
    pub infantry_per_mm: u32,
    pub infantry_height_mm: u32,
}

/// All combat rules. Part of [`crate::SimConfig`], so replays carry the
/// rules they were played with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatRules {
    pub aim: AimRules,
    pub fire: FireRules,
    pub dispersion: DispersionRules,
    pub ballistics: BallisticsRules,
    pub penetration: PenetrationRules,
    pub damage: DamageRules,
    /// Heavy vehicles running over enemy soldiers (prompt 093). Absent in
    /// older replays: nobody is run over.
    #[serde(default)]
    pub overrun: OverrunRules,
    /// Secondary machine guns of vehicles (prompt 093). Absent in older
    /// replays: they pick no targets of their own.
    #[serde(default)]
    pub vehicle_mg: VehicleMgRules,
    /// Scores of automatic targets (prompt 188). Absent in older replays:
    /// all weights 0, ties by distance (close to the old order).
    #[serde(default)]
    pub targeting: TargetingRules,
    /// Gun crews (prompt 177). Absent in older replays: no crew is hit.
    #[serde(default)]
    pub gun_crew: GunCrewRules,
}

/// Gun crews (prompt 177, `[gun_crew]` in `data/combat.toml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunCrewRules {
    /// Rounds below this caliber hit the crew instead of the gun; 0 = off.
    pub small_caliber_mm_x10: u32,
    pub hit_pct: u32,
    pub shield_half_deg: u32,
    /// Reload time in percent by the share of men left (up to 25, 50, 75 %
    /// and above).
    pub reload_pct: [u32; 4],
}

/// Scores of automatic targets (prompt 188, `[targeting]` in
/// `data/combat.toml`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetingRules {
    pub effect: u32,
    pub threat: u32,
    pub role: u32,
    pub focus: u32,
    pub focus_max: u32,
    pub stick: u32,
    pub distance_per_10m: u32,
    pub indirect_auto_range_pct: u32,
    pub indirect_min_soldiers: u32,
}

/// Moving heavy vehicles kill the enemy soldiers under their hull (prompt
/// 093). Speeds are game speeds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrunRules {
    /// Lightest vehicle (catalog mass) that runs soldiers over; 0 = none.
    pub min_mass_kg: u32,
    /// Slower vehicles (turning on the spot, creeping) hurt nobody.
    pub min_speed_mm_per_s: u32,
    /// Soldiers notice a vehicle coming at them this far ahead of its nose.
    pub notice_mm: u32,
    /// Ticks before a soldier who noticed it starts to move.
    pub reaction_ticks: u32,
    /// Speed of the step aside.
    pub dodge_speed_mm_per_s: u32,
    /// Soldiers step aside until they are this far beside the hull.
    pub dodge_clear_mm: u32,
}

/// Machine guns of vehicles that pick their own infantry targets beside
/// the main weapon's target (prompt 093).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleMgRules {
    /// Weapons below this caliber count as machine guns; 0 = none.
    pub max_caliber_mm: u32,
    /// Real typical combat distance of a vehicle's secondary machine gun
    /// whose catalog entry has none (converted like every direct-fire
    /// range).
    pub range_real_m: u32,
    /// Real muzzle velocity of such a gun whose catalog entry has none.
    pub velocity_real_mps: u32,
    /// Half the cone around the turret direction a coaxial gun covers.
    pub coax_half_deg: u32,
    /// Half the cone around the hull direction a bow gun covers.
    pub bow_half_deg: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatError(pub String);

impl fmt::Display for CombatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid combat rules: {}", self.0)
    }
}

impl std::error::Error for CombatError {}

impl CombatRules {
    pub fn parse(text: &str) -> Result<Self, CombatError> {
        let rules: Self = toml::from_str(text).map_err(|e| CombatError(e.to_string()))?;
        let check = |name: &str, value: u32, min: u32, max: u32| {
            if (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(CombatError(format!(
                    "{name} = {value} outside {min}..={max}"
                )))
            }
        };
        let (a, f, d, b, p, g) = (
            &rules.aim,
            &rules.fire,
            &rules.dispersion,
            &rules.ballistics,
            &rules.penetration,
            &rules.damage,
        );
        check("aim.turret_deg_per_s", a.turret_deg_per_s, 1, 360)?;
        check("aim.hull_deg_per_s", a.hull_deg_per_s, 1, 360)?;
        for o in &a.categories {
            check("aim.category.turret_deg_per_s", o.turret_deg_per_s, 1, 360)?;
        }
        check(
            "vehicle_mg.coax_half_deg",
            rules.vehicle_mg.coax_half_deg,
            0,
            180,
        )?;
        check(
            "vehicle_mg.bow_half_deg",
            rules.vehicle_mg.bow_half_deg,
            0,
            180,
        )?;
        check("fire.retarget_ticks", f.retarget_ticks, 1, 1000)?;
        check("fire.automatic_rpm", f.automatic_rpm, 1, 100_000)?;
        check(
            "fire.automatic_sustained_pct",
            f.automatic_sustained_pct,
            1,
            100,
        )?;
        check("fire.attack_range_pct", f.attack_range_pct, 10, 100)?;
        check("fire.muzzle_height_pct", f.muzzle_height_pct, 0, 100)?;
        check("dispersion.mrad_x10", d.mrad_x10, 0, 1000)?;
        check("dispersion.moving_pct", d.moving_pct, 0, 10_000)?;
        check(
            "ballistics.gravity_mm_per_s2",
            b.gravity_mm_per_s2,
            0,
            100_000,
        )?;
        check(
            "ballistics.max_flight_range_pct",
            b.max_flight_range_pct,
            100,
            1000,
        )?;
        check(
            "penetration.reference_velocity_mps",
            p.reference_velocity_mps,
            1,
            10_000,
        )?;
        check("penetration.loss_pct_per_km", p.loss_pct_per_km, 0, 100)?;
        check(
            "penetration.heat_below_velocity_mps",
            p.heat_below_velocity_mps,
            0,
            10_000,
        )?;
        check("penetration.heat_pct", p.heat_pct, 1, 1000)?;
        check("penetration.spread_pct", p.spread_pct, 0, 90)?;
        check("penetration.min_cos_pct", p.min_cos_pct, 1, 100)?;
        check("penetration.turret_height_pct", p.turret_height_pct, 0, 100)?;
        check("damage.vehicle_hp", g.vehicle_hp, 1, 1_000_000)?;
        check(
            "damage.vehicle_pct_per_mm2",
            g.vehicle_pct_per_mm2,
            1,
            10_000,
        )?;
        check("damage.soldier_hp", g.soldier_hp, 1, 100_000)?;
        check("damage.infantry_per_mm", g.infantry_per_mm, 1, 10_000)?;
        check("damage.infantry_height_mm", g.infantry_height_mm, 100, 5000)?;
        let t = &rules.targeting;
        for (name, v) in [
            ("effect", t.effect),
            ("threat", t.threat),
            ("role", t.role),
            ("focus", t.focus),
            ("stick", t.stick),
            ("distance_per_10m", t.distance_per_10m),
        ] {
            check(&format!("targeting.{name}"), v, 0, 10_000)?;
        }
        check("targeting.focus_max", t.focus_max, 0, 16)?;
        check(
            "targeting.indirect_auto_range_pct",
            t.indirect_auto_range_pct,
            0,
            100,
        )?;
        check(
            "targeting.indirect_min_soldiers",
            t.indirect_min_soldiers,
            0,
            100,
        )?;
        let c = &rules.gun_crew;
        check(
            "gun_crew.small_caliber_mm_x10",
            c.small_caliber_mm_x10,
            0,
            1000,
        )?;
        check("gun_crew.hit_pct", c.hit_pct, 0, 100)?;
        check("gun_crew.shield_half_deg", c.shield_half_deg, 0, 180)?;
        for v in c.reload_pct {
            check("gun_crew.reload_pct", v, 100, 1000)?;
        }
        Ok(rules)
    }

    /// The rules from `data/combat.toml`, compiled in.
    /// Borrowed reference to the built-in rules, parsed once.
    pub fn builtin_ref() -> Result<&'static Self, &'static CombatError> {
        static CACHE: std::sync::OnceLock<Result<CombatRules, CombatError>> =
            std::sync::OnceLock::new();
        CACHE.get_or_init(|| Self::parse(BUILTIN_COMBAT)).as_ref()
    }

    pub fn builtin() -> Result<Self, CombatError> {
        Self::builtin_ref().cloned().map_err(Clone::clone)
    }

    /// Turret traverse in real degrees per second for a category.
    pub fn turret_deg_per_s(&self, category: Option<Category>) -> u32 {
        self.aim
            .categories
            .iter()
            .find(|o| Some(o.category) == category)
            .map_or(self.aim.turret_deg_per_s, |o| o.turret_deg_per_s)
    }

    pub(crate) fn hash_into(&self, h: &mut StableHasher) {
        let a = &self.aim;
        h.write_u32(a.turret_deg_per_s);
        h.write_u32(a.hull_deg_per_s);
        h.write_u64(a.categories.len() as u64);
        for o in &a.categories {
            h.write_bytes(o.category.code().as_bytes());
            h.write_u32(o.turret_deg_per_s);
        }
        let f = &self.fire;
        let d = &self.dispersion;
        let b = &self.ballistics;
        let p = &self.penetration;
        let g = &self.damage;
        for v in [
            f.retarget_ticks,
            f.automatic_rpm,
            f.automatic_sustained_pct,
            f.attack_range_pct,
            f.muzzle_height_pct,
            d.mrad_x10,
            d.moving_pct,
            b.gravity_mm_per_s2,
            b.max_flight_range_pct,
            p.reference_velocity_mps,
            p.loss_pct_per_km,
            p.heat_below_velocity_mps,
            p.heat_pct,
            p.spread_pct,
            p.min_cos_pct,
            p.turret_height_pct,
            g.vehicle_hp,
            g.vehicle_pct_per_mm2,
            g.soldier_hp,
            g.infantry_per_mm,
            g.infantry_height_mm,
        ] {
            h.write_u32(v);
        }
        let o = &self.overrun;
        let m = &self.vehicle_mg;
        for v in [
            o.min_mass_kg,
            o.min_speed_mm_per_s,
            o.notice_mm,
            o.reaction_ticks,
            o.dodge_speed_mm_per_s,
            o.dodge_clear_mm,
            m.max_caliber_mm,
            m.range_real_m,
            m.velocity_real_mps,
            m.coax_half_deg,
            m.bow_half_deg,
        ] {
            h.write_u32(v);
        }
        let t = &self.targeting;
        for v in [
            t.effect,
            t.threat,
            t.role,
            t.focus,
            t.focus_max,
            t.stick,
            t.distance_per_10m,
            t.indirect_auto_range_pct,
            t.indirect_min_soldiers,
        ] {
            h.write_u32(v);
        }
        let c = &self.gun_crew;
        for v in [c.small_caliber_mm_x10, c.hit_pct, c.shield_half_deg] {
            h.write_u32(v);
        }
        for v in c.reload_pct {
            h.write_u32(v);
        }
    }
}

// --- Unit data from the catalog ---------------------------------------------

/// A direct-fire weapon, converted by [`Scale`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponType {
    pub name: String,
    /// Identical weapons fired together (one projectile each).
    pub count: u16,
    pub caliber_mm_x10: u32,
    /// Catalog rate of fire (cyclic for automatic weapons), real.
    pub rate_rpm: u32,
    /// Catalog muzzle velocity, real (penetration uses the real value).
    pub muzzle_velocity_mps: u32,
    /// Game range in meters (real effective range × direct-fire factor).
    pub range_m: Fixed,
    /// Game seconds between two rounds at the catalog rate.
    pub interval_s: Fixed,
    /// Game projectile speed in meters per second (real × time factor).
    pub speed_m_per_s: Fixed,
}

/// Armor in millimeters per face; missing catalog faces are filled in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Armor {
    /// Hull front, side, rear.
    pub hull: [u16; 3],
    /// Turret (or casemate) front, side, rear; `None` if the catalog has no
    /// turret armor (hits there use the hull).
    pub turret: Option<[u16; 3]>,
    /// Slope of the upper front plate from vertical, degrees.
    pub hull_front_deg: u16,
}

impl Armor {
    /// The same plate on every face, no turret (buildings, prompt 023).
    pub const fn uniform(mm: u16) -> Self {
        Self {
            hull: [mm; 3],
            turret: None,
            hull_front_deg: 0,
        }
    }

    /// Thinnest plate anywhere.
    pub fn thinnest(&self) -> u16 {
        let turret = self.turret.unwrap_or(self.hull);
        self.hull.iter().chain(&turret).copied().min().unwrap_or(0)
    }

    fn from_catalog(e: &Entry) -> Self {
        // Rear falls back to side, side to front.
        let faces = |front: Option<u16>, side: Option<u16>, rear: Option<u16>| {
            let front = front?;
            let side = side.unwrap_or(front);
            Some([front, side, rear.unwrap_or(side)])
        };
        let hull = faces(
            e.armor_hull_front_mm,
            e.armor_hull_side_mm,
            e.armor_hull_rear_mm,
        )
        .unwrap_or([0; 3]);
        let turret = faces(
            e.armor_turret_front_mm,
            e.armor_turret_side_mm.or(e.armor_hull_side_mm),
            e.armor_turret_rear_mm,
        );
        Self {
            hull,
            turret,
            hull_front_deg: e.armor_hull_front_deg.unwrap_or(0),
        }
    }
}

/// Everything combat needs to know about a unit type.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatType {
    /// Catalog category (turret traverse rules); `None` in hand-made types.
    pub category: Option<Category>,
    /// Soldiers of an infantry squad; 0 for vehicles.
    pub squad_size: u16,
    /// Men serving a towed gun or heavy mortar (prompt 177: they can be
    /// hit apart from the gun); 0 for everything else.
    #[serde(default)]
    pub gun_crew: u16,
    /// The gun has a shield that covers its crew from the front.
    #[serde(default)]
    pub gun_shield: bool,
    /// Hull length, width and height in meters (0: use the footprint and
    /// the rules' infantry height).
    pub length_m: Fixed,
    pub width_m: Fixed,
    pub height_m: Fixed,
    pub armor: Armor,
    /// Casemate guns: total traverse in degrees; `None` = turret.
    pub gun_traverse_deg: Option<u16>,
    /// Direct-fire weapons in catalog order; the first one is the main weapon.
    pub weapons: Vec<WeaponType>,
    /// Time factor of the category `(num, den)` (gravity, turn rates);
    /// `(0, 0)` counts as 1.
    pub time_ratio: (u32, u32),
    /// Direct-fire range factor `(num, den)` (real distances for
    /// penetration loss, dispersion); `(0, 0)` counts as 1.
    pub range_ratio: (u32, u32),
    /// Indirect weapon of mortars, howitzers and self-propelled guns
    /// (prompt 032).
    #[serde(default)]
    pub indirect: Option<crate::indirect::IndirectWeapon>,
    /// Secondary weapons of vehicles whose catalog entry gives no range
    /// (bow, coaxial and roof machine guns; prompt 093). `range_m` is 0,
    /// and so are the velocities where the catalog has none: the rules
    /// give those below `vehicle_mg.max_caliber_mm` their range (and
    /// velocity) and add them after `weapons` ([`Self::armed_with`]); the
    /// others stay unused.
    #[serde(default)]
    pub machine_guns: Vec<WeaponType>,
}

/// How a secondary machine gun of a vehicle is mounted (prompt 093); it
/// follows from the catalog name of the weapon ([`MgMount::of_name`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MgMount {
    /// Beside the main gun: fires where the turret points.
    Coax,
    /// In the hull front: a cone ahead of the vehicle.
    Bow,
    /// On the roof or a ring: all round.
    Roof,
    /// Several guns in one entry, or a name that says nothing: the turret
    /// cone or the bow cone.
    Mixed,
}

impl MgMount {
    /// The mount a weapon name describes.
    pub fn of_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
        let coax = has(&["coax"]);
        let bow = has(&["bow", "hull", "ball mount"]);
        if has(&["(aa", "roof", "ring mount", "pulpit", "pedestal", "remote"]) {
            Self::Roof
        } else if coax && !bow {
            Self::Coax
        } else if bow && !coax {
            Self::Bow
        } else {
            Self::Mixed
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Coax => "coax",
            Self::Bow => "bow",
            Self::Roof => "roof",
            Self::Mixed => "mixed",
        }
    }
}

impl CombatType {
    pub const fn is_infantry(&self) -> bool {
        self.squad_size > 0
    }

    /// The weapons a unit of this type fires with `rules`, in the order of
    /// the weapon indices of [`CombatEvent::Shot`]: `weapons`, then the
    /// machine guns of `machine_guns` (prompt 093) with their rule range.
    pub fn armed_with(&self, rules: &CombatRules) -> Vec<WeaponType> {
        let mg = rules.vehicle_mg;
        let range = Fixed::from_num(mg.range_real_m) * ratio_of(self.range_ratio);
        let time = ratio_of(self.time_ratio);
        let extra = self
            .machine_guns
            .iter()
            .filter(|w| {
                w.caliber_mm_x10 < mg.max_caliber_mm * 10
                    && range > Fixed::ZERO
                    && (w.muzzle_velocity_mps > 0 || mg.velocity_real_mps > 0)
            })
            .map(|w| {
                let mut w = w.clone();
                w.range_m = range;
                if w.muzzle_velocity_mps == 0 {
                    w.muzzle_velocity_mps = mg.velocity_real_mps;
                    w.speed_m_per_s = Fixed::from_num(mg.velocity_real_mps) * time;
                }
                w
            });
        self.weapons.iter().cloned().chain(extra).collect()
    }

    /// Mount of weapon `index` of [`Self::armed_with`] if it is a secondary
    /// machine gun of a vehicle that picks its own targets (prompt 093).
    pub fn mg_mount(&self, rules: &CombatRules, index: usize) -> Option<MgMount> {
        let weapons = self.armed_with(rules);
        let w = weapons.get(index)?;
        let secondary = index >= self.weapons.len() || index > 0;
        (!self.is_infantry()
            && secondary
            && w.caliber_mm_x10 < rules.vehicle_mg.max_caliber_mm * 10)
            .then(|| MgMount::of_name(&w.name))
    }

    /// Converts the combat values of a catalog entry.
    pub fn from_catalog(e: &Entry, scale: &Scale) -> Self {
        let group = e.category.group();
        // Mortars, howitzers and self-propelled guns fire their first
        // weapon indirectly (prompt 032); the others are direct weapons.
        let indirect = crate::indirect::IndirectWeapon::from_catalog(e, scale);
        let skip = usize::from(crate::indirect::is_indirect_category(e.category));
        let factor = scale.time_factor_for(e.category);
        let weapons = {
            e.weapon
                .iter()
                .skip(skip)
                .filter_map(|w| {
                    let (caliber, rate, velocity, range) = (
                        w.caliber_mm_x10?,
                        w.rate_rpm?,
                        w.muzzle_velocity_mps?,
                        // The typical combat distance (prompt 067), not
                        // the maximum or sight value `range_effective_m`.
                        w.range_typical_m?,
                    );
                    (caliber > 0 && rate > 0 && velocity > 0 && range > 0).then(|| WeaponType {
                        name: w.name.clone(),
                        count: w.count.max(1),
                        caliber_mm_x10: caliber,
                        rate_rpm: rate,
                        muzzle_velocity_mps: velocity,
                        range_m: scale.direct_fire_range_m(range),
                        interval_s: scale.game_duration(e.category, 60_000 / rate.max(1)),
                        speed_m_per_s: Fixed::from_num(u64::from(velocity) * u64::from(factor.num))
                            / Fixed::from_num(factor.den),
                    })
                })
                .collect()
        };
        // Secondary weapons of vehicles without a range in the catalog
        // (prompt 093): the rules decide which of them fire, and how far.
        let machine_guns = if group == Group::Vehicle {
            e.weapon
                .iter()
                .skip(skip + 1)
                .filter_map(|w| {
                    let (caliber, rate) = (w.caliber_mm_x10?, w.rate_rpm?);
                    let velocity = w.muzzle_velocity_mps.unwrap_or(0);
                    (w.range_typical_m.is_none() && caliber > 0 && rate > 0).then(|| WeaponType {
                        name: w.name.clone(),
                        count: w.count.max(1),
                        caliber_mm_x10: caliber,
                        rate_rpm: rate,
                        muzzle_velocity_mps: velocity,
                        range_m: Fixed::ZERO,
                        interval_s: scale.game_duration(e.category, 60_000 / rate.max(1)),
                        speed_m_per_s: Fixed::from_num(u64::from(velocity) * u64::from(factor.num))
                            / Fixed::from_num(factor.den),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let m = |v: Option<u32>| v.map_or(Fixed::ZERO, |v| Fixed::from_num(v) / 1000);
        Self {
            category: Some(e.category),
            squad_size: if group == Group::Infantry {
                e.squad_size.unwrap_or(1).max(1)
            } else {
                0
            },
            gun_crew: if group == Group::Gun {
                e.crew.unwrap_or(0)
            } else {
                0
            },
            gun_shield: group == Group::Gun && e.gun_shield.unwrap_or(false),
            length_m: m(e.length_hull_mm.or(e.length_mm)),
            width_m: m(e.width_mm),
            height_m: m(e.height_mm),
            armor: Armor::from_catalog(e),
            gun_traverse_deg: e.gun_traverse_deg,
            weapons,
            time_ratio: (factor.num, factor.den),
            range_ratio: (scale.direct_fire_range.num, scale.direct_fire_range.den),
            indirect,
            machine_guns,
        }
    }
}

/// `(num, den)` as a factor; `(0, 0)` counts as 1.
fn ratio_of((num, den): (u32, u32)) -> Fixed {
    if num == 0 || den == 0 {
        Fixed::ONE
    } else {
        Fixed::from_num(num) / Fixed::from_num(den)
    }
}

// --- Rule formulas ------------------------------------------------------------

impl CombatRules {
    /// Penetration in mm against a vertical plate at the muzzle.
    pub fn muzzle_penetration_mm(&self, weapon: &WeaponType) -> Fixed {
        let caliber = Fixed::from_num(weapon.caliber_mm_x10) / 10;
        let p = &self.penetration;
        if weapon.muzzle_velocity_mps < p.heat_below_velocity_mps {
            return caliber * Fixed::from_num(p.heat_pct) / 100;
        }
        let ratio =
            Fixed::from_num(weapon.muzzle_velocity_mps) / Fixed::from_num(p.reference_velocity_mps);
        caliber * ratio * fixed_sqrt(ratio)
    }

    /// Whether a weapon fires shaped charges (no loss over distance).
    pub const fn is_heat(&self, weapon: &WeaponType) -> bool {
        weapon.muzzle_velocity_mps < self.penetration.heat_below_velocity_mps
    }

    /// Penetration in mm after `real_m` meters of real distance.
    pub fn penetration_at(&self, weapon: &WeaponType, real_m: Fixed) -> Fixed {
        let p0 = self.muzzle_penetration_mm(weapon);
        if self.is_heat(weapon) {
            return p0;
        }
        let loss = real_m * Fixed::from_num(self.penetration.loss_pct_per_km) / 100_000;
        (p0 * (Fixed::ONE - loss)).max(Fixed::ZERO)
    }

    /// Damage of a penetrating hit on a vehicle.
    pub fn vehicle_damage(&self, weapon: &WeaponType) -> u32 {
        let c = u64::from(weapon.caliber_mm_x10);
        let d = c * c * u64::from(self.damage.vehicle_pct_per_mm2) / 10_000;
        u32::try_from(d).unwrap_or(u32::MAX).max(1)
    }

    /// Damage of a hit on infantry.
    pub fn infantry_damage(&self, weapon: &WeaponType) -> u32 {
        let d = u64::from(weapon.caliber_mm_x10) * u64::from(self.damage.infantry_per_mm) / 10;
        u32::try_from(d).unwrap_or(u32::MAX).max(1)
    }

    /// Hit points of a unit type.
    pub fn max_hp(&self, combat: &CombatType) -> u32 {
        if combat.is_infantry() {
            u32::from(combat.squad_size) * self.damage.soldier_hp
        } else {
            self.damage.vehicle_hp
        }
    }

    /// Ticks between two salvos of a weapon (at least one).
    pub fn reload_ticks(&self, weapon: &WeaponType, tick_hz: u32) -> u32 {
        let mut interval = weapon.interval_s;
        if weapon.rate_rpm >= self.fire.automatic_rpm {
            interval = interval * 100 / Fixed::from_num(self.fire.automatic_sustained_pct);
        }
        let ticks = (interval * Fixed::from_num(tick_hz)).round();
        ticks.to_num::<u32>().max(1)
    }
}

// --- Snapshot data --------------------------------------------------------------

/// What a projectile hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitKind {
    /// A unit (alive or wreck).
    Unit(UnitId),
    Ground,
    /// A map object (index into [`crate::Map::objects`]).
    Object(u32),
}

/// Face of a unit that was hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Face {
    Front,
    Side,
    Rear,
    Top,
}

/// Things that happened during one tick, for effects and sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatEvent {
    /// A weapon fired one projectile.
    Shot {
        unit: UnitId,
        /// Index into the unit type's weapons.
        weapon: u8,
        projectile: u32,
        muzzle: Vec3,
        caliber_mm_x10: u32,
    },
    /// A projectile hit something and is gone.
    Hit {
        projectile: u32,
        shooter: UnitId,
        at: Vec3,
        kind: HitKind,
        /// Unit hits only.
        face: Option<Face>,
        turret: bool,
        penetrated: bool,
        damage: u32,
        caliber_mm_x10: u32,
    },
    /// A smoke shell burst at `at` and left a cloud of `radius` meters
    /// (prompt 032).
    Smoke {
        projectile: u32,
        shooter: UnitId,
        at: Vec3,
        radius: Fixed,
    },
    /// A unit lost its last hit point.
    Destroyed {
        unit: UnitId,
        /// The unit that fired the last hit.
        by: UnitId,
        at: Vec3,
    },
}

/// A projectile in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectileSnapshot {
    pub id: u32,
    pub shooter: UnitId,
    pub position: Vec3,
    /// Meters per second.
    pub velocity: Vec3,
    pub caliber_mm_x10: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    fn weapon(id: &str) -> (CombatType, WeaponType) {
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let combat = CombatType::from_catalog(catalog.get(id).unwrap(), &scale);
        let w = combat.weapons[0].clone();
        (combat, w)
    }

    #[test]
    fn builtin_rules_parse_and_bad_values_fail() {
        let rules = CombatRules::builtin().unwrap();
        assert_eq!(rules.damage.vehicle_hp, 1000);
        assert_eq!(rules.turret_deg_per_s(Some(Category::TankHeavy)), 15);
        assert_eq!(rules.turret_deg_per_s(Some(Category::TankMedium)), 20);
        assert!(
            CombatRules::parse(&BUILTIN_COMBAT.replace("spread_pct = 10", "spread_pct = 99"))
                .is_err()
        );
        assert!(
            CombatRules::parse(&BUILTIN_COMBAT.replace("[damage]", "[damage]\ncolour = 1"))
                .is_err()
        );
    }

    #[test]
    fn penetration_matches_published_values() {
        let rules = CombatRules::builtin().unwrap();
        let mm = |id: &str| {
            rules
                .muzzle_penetration_mm(&weapon(id).1)
                .round()
                .to_num::<i32>()
        };
        // 8.8 cm KwK 36 about 160 mm, 7.5 cm KwK 40 L/48 about 130 mm at the
        // muzzle (0°); a 7.92 mm rifle about 11 mm; the bazooka (HEAT) 96 mm.
        assert_eq!(mm("de_tiger1_e"), 159);
        assert_eq!(mm("de_pz4_h"), 130);
        assert_eq!(mm("su_t34_76_1943"), 109);
        assert_eq!(mm("us_inf_at"), 96);
        let (_, kwk) = weapon("de_pz4_h");
        let at_1km = rules.penetration_at(&kwk, Fixed::from_num(1000));
        assert_eq!(at_1km.round().to_num::<i32>(), 104);
        // HEAT keeps its penetration.
        let (_, zook) = weapon("us_inf_at");
        assert_eq!(
            rules.penetration_at(&zook, Fixed::from_num(1000)),
            rules.muzzle_penetration_mm(&zook)
        );
    }

    #[test]
    fn catalog_values_are_scaled() {
        let rules = CombatRules::builtin().unwrap();
        let (pz4, kwk) = weapon("de_pz4_h");
        // 700 m real (typical distance) × 1/8 → 87.5 m; 10 rpm real → 6 s
        // → 3 s game = 60 ticks;
        // 750 m/s × 2.
        assert_eq!(kwk.range_m, Fixed::from_num(700) / Fixed::from_num(8));
        assert_eq!(rules.reload_ticks(&kwk, 20), 60);
        assert_eq!(kwk.speed_m_per_s, Fixed::from_num(1500));
        // MG 34 without range is not used.
        assert_eq!(pz4.weapons.len(), 1);
        assert_eq!(pz4.armor.hull, [80, 30, 20]);
        assert_eq!(pz4.armor.turret, Some([50, 30, 30]));
        assert_eq!(rules.vehicle_damage(&kwk), 562);
        // Infantry: MG 42 is automatic (1200 rpm cyclic, 25 % sustained):
        // 0.05 s × 2 (time) ... 1200 → 300 rpm real → 0.2 s → 0.1 s game.
        let (squad, _) = weapon("de_inf_rifle_squad");
        assert_eq!(squad.squad_size, 9);
        assert_eq!(rules.max_hp(&squad), 900);
        let mg = squad.weapons.iter().find(|w| w.name == "MG 42").unwrap();
        assert_eq!(rules.reload_ticks(mg, 20), 2);
        assert_eq!(rules.infantry_damage(mg), 63);
        // Casemate and indirect fire.
        let (stug, _) = weapon("de_stug3_g");
        assert_eq!(stug.gun_traverse_deg, Some(20));
        let catalog = Catalog::builtin().unwrap();
        let scale = Scale::builtin().unwrap();
        let wespe = CombatType::from_catalog(catalog.get("de_wespe").unwrap(), &scale);
        assert!(wespe.weapons.is_empty());
        assert_eq!(wespe.indirect.unwrap().caliber_mm_x10, 1050);
        // The crew of a mortar keeps its small arms as direct weapons.
        let grw = CombatType::from_catalog(catalog.get("de_inf_mortar_medium").unwrap(), &scale);
        assert_eq!(grw.indirect.unwrap().name, "8 cm s.GrW 34");
        assert!(grw.weapons.iter().all(|w| w.caliber_mm_x10 < 100));
        assert!(!grw.weapons.is_empty());
    }

    #[test]
    fn small_angles_and_roots() {
        let (s, c) = sin_cos_rad(deg_to_rad(Fixed::from_num(30)));
        assert!((s - Fixed::ONE / 2).abs() < Fixed::from_num(1) / 1_000_000);
        assert!((c - fixed_sqrt(Fixed::from_num(3)) / 2).abs() < Fixed::from_num(1) / 1_000_000);
        assert_eq!(fixed_sqrt(Fixed::from_num(16)), Fixed::from_num(4));
        assert_eq!(
            Vec3::new(Fixed::from_num(2), Fixed::from_num(3), Fixed::from_num(6)).length(),
            Fixed::from_num(7)
        );
    }
}
