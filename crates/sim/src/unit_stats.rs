//! Game values of a unit type as one card (prompt 065): what a unit can do
//! in the game, computed from its catalog entry and the rule files by the
//! same functions the simulation uses. The unit page (`cli unit-stats`) and
//! the game (`SimHost.unit_stats`) only display the card.
//!
//! Pure functions: nothing here reads or changes a running [`crate::Sim`],
//! so the state hash and the golden hash do not depend on this module. No
//! new rules either: a value the simulation has no rule for is left out.
//!
//! All numbers are integers with the unit in the key (`_x10` = tenths).
//! "game" values are what the simulation computes with, "real" values the
//! historical ones behind them (`docs/SCALE.md`). Base values: dry ground,
//! no experience bonus, no weather.
//!
//! The five ratings (0–100) at the end are display values with fixed
//! reference points; the AI and the simulation never read them. Their
//! formulas and reference values are the block `RATINGS` below.

use crate::catalog::{Catalog, Entry, Faction, Group};
use crate::combat::{CombatRules, CombatType, WeaponType};
use crate::destruction::DestructionRules;
use crate::economy::EconomyRules;
use crate::indirect::IndirectRules;
use crate::logistics::LogisticsType;
use crate::math::Fixed;
use crate::scale::Scale;
use crate::sim::SimConfig;
use crate::terrain::TerrainRules;
use crate::vision::VisionRules;
use crate::weather::{Ground, WeatherRules};
use crate::{UnitType, objects::sin_cos_deg};
use std::fmt::Write as _;

// --- Reference values ---------------------------------------------------------

/// Real distances in meters at which penetration is listed.
pub const DISTANCES_M: [u32; 4] = [100, 500, 1000, 1500];
/// Real distance in meters of the "can it knock out …" comparison.
pub const DUEL_DISTANCE_M: u32 = 500;

/// Comparison weapons for "how vulnerable is this unit": `(catalog id, part
/// of the weapon name, label)`. Fixed catalog weapons, so the numbers follow
/// the catalog and the rules.
pub const REFERENCE_WEAPONS: [(&str, &str, &str); 7] = [
    ("de_inf_rifle_squad", "Kar98k", "rifle"),
    ("de_inf_mg42_team", "MG 42", "machine_gun"),
    ("us_m5a1", "37 mm", "gun_37mm"),
    ("us_57mm_m1", "57 mm", "gun_57mm"),
    ("de_pak40", "7.5 cm", "gun_75mm"),
    ("de_tiger1_e", "8.8 cm", "gun_88mm"),
    ("de_inf_at", "Panzerschreck", "rocket_launcher"),
];

/// Comparison targets for "what can this unit knock out": a light, a medium
/// and a heavy tank of every faction. A unit is compared with the tanks of
/// the other factions.
pub const REFERENCE_TARGETS: [&str; 9] = [
    "de_pz2_l",
    "de_pz4_h",
    "de_tiger1_e",
    "su_t70m",
    "su_t34_76_1943",
    "su_is2",
    "us_m5a1",
    "us_m4a3_75",
    "us_m26",
];

/// Comparison objects of `data/destruction.toml`.
pub const REFERENCE_OBJECTS: [&str; 4] = [
    "env_tree_deciduous",
    "env_wall_stone",
    "env_house_farm",
    "env_house_farm_brick",
];

/// Reference values of the five ratings. Every rating is
/// `min(100, value × 100 / reference)`, rounded down; see [`ratings`].
pub struct RatingRules {
    /// Speed: mean of road and cross-country game speed in tenths of m/s
    /// that scores 100 (30 m/s in the game = 54 km/h real).
    pub speed_mps_x10: i64,
    /// Protection of armored units: effective armor in mm (front 50 %, side
    /// 30 %, rear 20 %, hull and turret averaged) that scores 100.
    pub armor_mm: i64,
    /// Protection of unarmored vehicles and guns (fixed score).
    pub unarmored: i64,
    /// Protection of infantry: points per soldier and the cap.
    pub per_soldier: i64,
    pub infantry_cap: i64,
    /// Against vehicles: penetration in mm at 500 m real (or at the
    /// weapon's range if shorter) that scores 100.
    pub penetration_mm: i64,
    /// Share of the score a weapon keeps at point-blank reach; it rises
    /// linearly to 100 % at 500 m real range.
    pub short_range_pct: i64,
    /// Against infantry: soldiers put out of action per game minute, in
    /// tenths, that score 100 (every round hitting).
    pub soldiers_per_min_x10: i64,
    /// Against structures: hit points of damage per game minute that score 100.
    pub structure_damage_per_min: i64,
}

/// The reference values in use.
pub const RATINGS: RatingRules = RatingRules {
    speed_mps_x10: 300,
    armor_mm: 150,
    unarmored: 10,
    per_soldier: 2,
    infantry_cap: 20,
    penetration_mm: 200,
    short_range_pct: 50,
    soldiers_per_min_x10: 6000,
    structure_damage_per_min: 20_000,
};

// --- Value tree -----------------------------------------------------------------

/// A card: integers, flags, texts, lists and maps with a fixed key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Text(String),
    List(Vec<Self>),
    Map(Vec<(String, Self)>),
}

impl Value {
    /// Entry `key` of a map.
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Follows `path` through nested maps.
    pub fn at(&self, path: &[&str]) -> Option<&Self> {
        path.iter().try_fold(self, |v, key| v.get(key))
    }

    /// Integer at `path`.
    pub fn int(&self, path: &[&str]) -> Option<i64> {
        match self.at(path) {
            Some(Self::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Elements of a list (empty for anything else).
    pub fn items(&self) -> &[Self] {
        match self {
            Self::List(items) => items,
            _ => &[],
        }
    }

    /// JSON text: two spaces per level, keys in card order.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out, 0);
        out.push('\n');
        out
    }

    fn write_json(&self, out: &mut String, depth: usize) {
        let pad = |out: &mut String, depth: usize| out.push_str(&"  ".repeat(depth));
        match self {
            Self::Int(v) => {
                let _ = write!(out, "{v}");
            }
            Self::Bool(v) => {
                let _ = write!(out, "{v}");
            }
            Self::Text(text) => write_json_string(out, text),
            Self::List(items) if items.iter().all(|i| matches!(i, Self::Int(_))) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write_json(out, depth);
                }
                out.push(']');
            }
            Self::List(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    pad(out, depth + 1);
                    item.write_json(out, depth + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                pad(out, depth);
                out.push(']');
            }
            Self::Map(entries) => {
                out.push_str("{\n");
                for (i, (key, value)) in entries.iter().enumerate() {
                    pad(out, depth + 1);
                    write_json_string(out, key);
                    out.push_str(": ");
                    value.write_json(out, depth + 1);
                    out.push_str(if i + 1 < entries.len() { ",\n" } else { "\n" });
                }
                pad(out, depth);
                out.push('}');
            }
        }
    }
}

fn write_json_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Builds a map in insertion order; `None` values are left out.
#[derive(Default)]
struct MapBuilder(Vec<(String, Value)>);

impl MapBuilder {
    fn int(&mut self, key: &str, value: impl Into<i64>) -> &mut Self {
        self.0.push((key.to_owned(), Value::Int(value.into())));
        self
    }

    fn opt(&mut self, key: &str, value: Option<impl Into<i64>>) -> &mut Self {
        if let Some(v) = value {
            self.int(key, v);
        }
        self
    }

    fn flag(&mut self, key: &str, value: bool) -> &mut Self {
        self.0.push((key.to_owned(), Value::Bool(value)));
        self
    }

    fn text(&mut self, key: &str, value: &str) -> &mut Self {
        self.0.push((key.to_owned(), Value::Text(value.to_owned())));
        self
    }

    fn put(&mut self, key: &str, value: Value) -> &mut Self {
        self.0.push((key.to_owned(), value));
        self
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn build(&mut self) -> Value {
        Value::Map(std::mem::take(&mut self.0))
    }
}

fn ints(values: impl IntoIterator<Item = i64>) -> Value {
    Value::List(values.into_iter().map(Value::Int).collect())
}

fn whole(value: Fixed) -> i64 {
    value.round().to_num::<i64>()
}

fn tenths(value: Fixed) -> i64 {
    (value * 10).round().to_num::<i64>()
}

// --- Rules ------------------------------------------------------------------------

/// Every rule set the cards are computed from: the files compiled into the
/// simulation.
pub struct StatRules {
    pub catalog: Catalog,
    pub scale: Scale,
    pub combat: CombatRules,
    pub destruction: DestructionRules,
    pub indirect: IndirectRules,
    pub vision: VisionRules,
    pub terrain: TerrainRules,
    pub weather: WeatherRules,
    pub economy: EconomyRules,
    pub tick_hz: u32,
}

/// One face of a target as the armor check of the simulation sees it.
#[derive(Clone, Copy)]
struct Plate {
    key: &'static str,
    /// Effective thickness in mm against a round flying straight at it.
    effective: Fixed,
}

impl StatRules {
    /// The rules compiled into the simulation.
    pub fn builtin() -> Result<Self, String> {
        let text = |e: &dyn std::fmt::Display| e.to_string();
        Ok(Self {
            catalog: Catalog::builtin().map_err(|e| text(&e))?,
            scale: Scale::builtin().map_err(|e| text(&e))?,
            combat: CombatRules::builtin().map_err(|e| text(&e))?,
            destruction: DestructionRules::builtin().map_err(|e| text(&e))?,
            indirect: IndirectRules::builtin().map_err(|e| text(&e))?,
            vision: VisionRules::builtin().map_err(|e| text(&e))?,
            terrain: TerrainRules::builtin().map_err(|e| text(&e))?,
            weather: WeatherRules::builtin().map_err(|e| text(&e))?,
            economy: EconomyRules::builtin().map_err(|e| text(&e))?,
            tick_hz: SimConfig::DEFAULT_TICK_HZ,
        })
    }

    /// The cards of every catalog entry: a map from id to card, in catalog
    /// order.
    pub fn all_cards(&self) -> Value {
        Value::Map(
            self.catalog
                .entries
                .iter()
                .map(|e| (e.id.clone(), self.card(e)))
                .collect(),
        )
    }

    /// The card of the catalog entry `id`.
    pub fn card_of(&self, id: &str) -> Option<Value> {
        self.catalog.get(id).map(|e| self.card(e))
    }

    /// The combat values of an entry as the simulation uses them (placed
    /// fortifications get the armor of their economy rule).
    fn combat_type(&self, e: &Entry) -> CombatType {
        let mut combat = CombatType::from_catalog(e, &self.scale);
        if let Some(rule) = self
            .economy
            .fortifications
            .iter()
            .find(|f| f.ids.contains(&e.id))
            && rule.armor_mm > 0
        {
            combat.armor = crate::combat::Armor::uniform(rule.armor_mm);
        }
        combat
    }

    /// Real meters per game meter of direct fire.
    fn real_per_game(combat: &CombatType) -> Fixed {
        match combat.range_ratio {
            (0, _) | (_, 0) => Fixed::ONE,
            (num, den) => Fixed::from_num(den) / Fixed::from_num(num),
        }
    }

    /// Shots per game minute of a weapon (all barrels), in tenths.
    fn shots_per_min_x10(&self, weapon: &WeaponType) -> i64 {
        let ticks = i64::from(self.combat.reload_ticks(weapon, self.tick_hz));
        i64::from(self.tick_hz) * 600 * i64::from(weapon.count) / ticks.max(1)
    }

    /// The plates of a target and their effective thickness, as in the
    /// armor check of a hit (`Sim::damage_unit`) for a round flying straight
    /// at the face: the hull front counts its slope, limited by `min_cos_pct`.
    fn plates(&self, combat: &CombatType) -> Vec<Plate> {
        let armor = &combat.armor;
        let min_cos = Fixed::from_num(self.combat.penetration.min_cos_pct) / 100;
        let front_cos = sin_cos_deg(armor.hull_front_deg).1.max(min_cos);
        let mm = |v: u16| Fixed::from_num(v);
        let mut plates = vec![
            Plate {
                key: "hull_front",
                effective: mm(armor.hull[0]) / front_cos,
            },
            Plate {
                key: "hull_side",
                effective: mm(armor.hull[1]),
            },
            Plate {
                key: "hull_rear",
                effective: mm(armor.hull[2]),
            },
        ];
        if let Some(turret) = armor.turret {
            for (key, v) in ["turret_front", "turret_side", "turret_rear"]
                .into_iter()
                .zip(turret)
            {
                plates.push(Plate {
                    key,
                    effective: mm(v),
                });
            }
        }
        plates
    }

    /// Largest real distance in meters (steps of 10 m, at most the weapon's
    /// range) at which `weapon` penetrates `effective` mm without the random
    /// spread, or `None` if it never does.
    fn penetration_range_m(
        &self,
        weapon: &WeaponType,
        real_range_m: i64,
        effective: Fixed,
    ) -> Option<i64> {
        let pierces = |m: i64| self.combat.penetration_at(weapon, Fixed::from_num(m)) >= effective;
        if !pierces(0) {
            return None;
        }
        let (mut low, mut high) = (0, real_range_m.max(0) / 10);
        while low < high {
            let mid = (low + high + 1) / 2;
            if pierces(mid * 10) {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        Some(low * 10)
    }

    fn real_range_m(weapon: &WeaponType, combat: &CombatType) -> i64 {
        whole(weapon.range_m * Self::real_per_game(combat))
    }

    /// Hits of `damage` until `hp` hit points are gone.
    fn hits(hp: u32, damage: u32) -> i64 {
        i64::from(hp.div_ceil(damage.max(1)))
    }

    // --- Sections -----------------------------------------------------------

    fn movement(&self, e: &Entry, unit: Option<&UnitType>) -> Option<Value> {
        let unit = unit?;
        let mut m = MapBuilder::default();
        m.int("road_game_mps_x10", tenths(unit.road_speed_m_per_s));
        m.int("offroad_game_mps_x10", tenths(unit.speed_m_per_s));
        match e.category.group() {
            Group::Aircraft => {
                m.opt("cruise_real_kmh_x10", e.speed_cruise_kmh_x10);
                m.opt("max_real_kmh_x10", e.speed_max_kmh_x10);
            }
            _ => {
                m.opt("road_real_kmh_x10", e.speed_road_kmh_x10);
                m.opt("offroad_real_kmh_x10", e.speed_offroad_kmh_x10);
                m.opt("water_real_kmh_x10", e.speed_water_kmh_x10);
            }
        }
        let ground = matches!(
            e.category.group(),
            Group::Vehicle | Group::Infantry | Group::Gun
        );
        if e.category.group() == Group::Vehicle {
            let factor = self.scale.time_factor_for(e.category);
            let game = |real: u32| i64::from(real) * i64::from(factor.num) / i64::from(factor.den);
            m.int(
                "hull_turn_game_deg_per_s",
                game(self.combat.aim.hull_deg_per_s),
            );
            match e.gun_traverse_deg {
                Some(deg) => m.int("gun_traverse_deg", deg),
                None if !unit.combat.weapons.is_empty() => m.int(
                    "turret_turn_game_deg_per_s",
                    game(self.combat.turret_deg_per_s(Some(e.category))),
                ),
                None => &mut m,
            };
        }
        if ground {
            // Slowest speed on soaked and on snowed-in ground of any terrain.
            if let Some(gear) = self.weather.gear_of(unit) {
                let soft = self.weather.softness(&self.terrain);
                let worst = |kind: Ground| {
                    soft.iter()
                        .map(|&s| self.weather.speed_pct(gear, kind, 100, s))
                        .min()
                        .unwrap_or(100)
                };
                m.int("mud_speed_pct", worst(Ground::Wet));
                m.int("snow_speed_pct", worst(Ground::Snow));
            }
            let slope = &self.terrain.slope;
            m.int("max_grade_pct", slope.max_grade_pct);
            if let Some(f) = slope.factor(Fixed::from_num(slope.max_grade_pct) / 100) {
                m.int("steepest_grade_speed_pct", whole(f * 100));
            }
        }
        if e.category.group() == Group::Vehicle {
            let crushed: Vec<Value> = self
                .destruction
                .classes
                .iter()
                .filter(|c| c.crush_min_mass_kg > 0 && unit.mass_kg >= c.crush_min_mass_kg)
                .flat_map(|c| c.ids.iter().map(|id| Value::Text(id.clone())))
                .collect();
            m.put("crushes", Value::List(crushed));
        }
        Some(m.build())
    }

    fn protection(&self, e: &Entry, combat: &CombatType, mobile: bool) -> Option<Value> {
        let group = e.category.group();
        if !mobile && group != Group::Fortification && group != Group::Gun {
            return None;
        }
        if matches!(group, Group::Aircraft | Group::Environment) {
            return None;
        }
        let mut m = MapBuilder::default();
        let hp = self
            .economy
            .fortifications
            .iter()
            .find(|f| f.ids.contains(&e.id))
            .map(|f| f.hp)
            .filter(|&hp| hp > 0)
            .unwrap_or_else(|| self.combat.max_hp(combat));
        m.int("hp", hp);
        let armored = combat.armor.hull[0] > 0;
        if combat.is_infantry() {
            m.int("soldiers", combat.squad_size);
            m.int("soldier_hp", self.combat.damage.soldier_hp);
        } else if armored {
            let a = &combat.armor;
            m.put("armor_hull_mm", ints(a.hull.map(i64::from)));
            if let Some(t) = a.turret {
                m.put("armor_turret_mm", ints(t.map(i64::from)));
            }
            m.int("hull_front_slope_deg", a.hull_front_deg);
            let plates = self.plates(combat);
            m.int("hull_front_effective_mm", whole(plates[0].effective));
        } else {
            m.flag("unarmored", true);
        }
        if !combat.is_infantry() {
            // Shell fragments (data/indirect.toml): only thin-skinned targets.
            let open = combat.armor.thinnest()
                <= u16::try_from(self.indirect.blast.open_armor_mm).unwrap_or(u16::MAX);
            m.flag("hurt_by_fragments", open);
        }
        // Against the comparison weapons.
        let mut versus = Vec::new();
        for (id, part, label) in REFERENCE_WEAPONS {
            let Some(shooter) = self.catalog.get(id) else {
                continue;
            };
            let shooter = CombatType::from_catalog(shooter, &self.scale);
            let Some(weapon) = shooter.weapons.iter().find(|w| w.name.contains(part)) else {
                continue;
            };
            let range = Self::real_range_m(weapon, &shooter);
            let mut v = MapBuilder::default();
            v.text("weapon", label).text("name", &weapon.name);
            v.int("range_real_m", range);
            if combat.is_infantry() {
                let damage = self.combat.infantry_damage(weapon);
                v.int(
                    "hits_per_soldier",
                    Self::hits(self.combat.damage.soldier_hp, damage),
                );
            } else if armored {
                let damage = self.combat.vehicle_damage(weapon);
                v.int("hits_to_destroy", Self::hits(hp, damage));
                for plate in self.plates(combat) {
                    // -1: never penetrates this face.
                    let reach = self.penetration_range_m(weapon, range, plate.effective);
                    v.int(&format!("{}_real_m", plate.key), reach.unwrap_or(-1));
                }
            } else {
                let damage = self.combat.vehicle_damage(weapon);
                v.int("hits_to_destroy", Self::hits(hp, damage));
            }
            versus.push(v.build());
        }
        m.put("versus", Value::List(versus));
        Some(m.build())
    }

    fn weapons(&self, combat: &CombatType) -> Vec<Value> {
        let min_caliber = self.destruction.damage.min_caliber_mm * 10;
        combat
            .weapons
            .iter()
            .map(|w| {
                let range = Self::real_range_m(w, combat);
                let mut m = MapBuilder::default();
                m.text("name", &w.name);
                m.int("count", w.count);
                m.int("caliber_mm_x10", w.caliber_mm_x10);
                m.int("range_game_m", whole(w.range_m));
                m.int("range_real_m", range);
                m.int("shots_per_game_min_x10", self.shots_per_min_x10(w));
                m.flag("automatic", w.rate_rpm >= self.combat.fire.automatic_rpm);
                m.flag("shaped_charge", self.combat.is_heat(w));
                m.put(
                    "penetration_mm",
                    ints(
                        DISTANCES_M
                            .map(|d| whole(self.combat.penetration_at(w, Fixed::from_num(d)))),
                    ),
                );
                m.int("vehicle_damage", self.combat.vehicle_damage(w));
                m.int(
                    "hits_to_destroy_vehicle",
                    Self::hits(self.combat.damage.vehicle_hp, self.combat.vehicle_damage(w)),
                );
                m.int("infantry_damage", self.combat.infantry_damage(w));
                m.flag("damages_objects", w.caliber_mm_x10 >= min_caliber);
                m.build()
            })
            .collect()
    }

    fn indirect(&self, e: &Entry, combat: &CombatType) -> Option<Value> {
        let w = combat.indirect.as_ref()?;
        let rules = self.indirect.category(Some(e.category));
        let ticks = (w.interval_s * Fixed::from_num(self.tick_hz))
            .round()
            .to_num::<i64>()
            .max(1);
        let mut m = MapBuilder::default();
        m.text("name", &w.name);
        m.int("caliber_mm_x10", w.caliber_mm_x10);
        m.int("range_game_m", whole(w.range_m));
        m.int("range_real_m", w.real_range_m);
        m.int(
            "min_range_game_m",
            whole(w.range_m * Fixed::from_num(rules.min_range_pct) / 100),
        );
        let factor = self.scale.time_factor_for(e.category);
        m.int(
            "setup_game_s_x10",
            i64::from(rules.setup_ms) * i64::from(factor.den) / (i64::from(factor.num) * 100),
        );
        m.int(
            "shots_per_game_min_x10",
            i64::from(self.tick_hz) * 600 / ticks,
        );
        m.int(
            "blast_radius_game_m_x10",
            tenths(self.indirect.blast_radius(w.caliber_mm_x10)),
        );
        m.int("soldier_damage", self.indirect.blast.soldier_damage);
        m.int("direct_hit_damage", self.shell_damage(w.caliber_mm_x10));
        m.int(
            "direct_hit_roof_mm",
            i64::from(w.caliber_mm_x10) * i64::from(self.indirect.blast.direct_hit_pen_pct) / 1000,
        );
        m.flag("smoke", w.smoke);
        Some(m.build())
    }

    /// Damage of a shell of `caliber_mm_x10` on a vehicle or an object: the
    /// formula of a penetrating gun round (`CombatRules::vehicle_damage`).
    fn shell_damage(&self, caliber_mm_x10: u32) -> u32 {
        self.combat.vehicle_damage(&WeaponType {
            name: String::new(),
            count: 1,
            caliber_mm_x10,
            rate_rpm: 1,
            muzzle_velocity_mps: 1,
            range_m: Fixed::ZERO,
            interval_s: Fixed::ONE,
            speed_m_per_s: Fixed::ONE,
        })
    }

    /// The direct weapon that penetrates most at the duel distance (or at
    /// its own range if that is shorter), with that penetration in mm.
    fn best_gun<'a>(&self, combat: &'a CombatType) -> Option<(&'a WeaponType, Fixed, i64)> {
        combat
            .weapons
            .iter()
            .map(|w| {
                let range = Self::real_range_m(w, combat);
                let at = range.min(i64::from(DUEL_DISTANCE_M));
                (w, self.combat.penetration_at(w, Fixed::from_num(at)), range)
            })
            .max_by_key(|&(_, pen, range)| (pen, range))
    }

    fn against_vehicles(&self, e: &Entry, combat: &CombatType) -> Option<Value> {
        let (weapon, pen, range) = self.best_gun(combat)?;
        // Small arms do not belong in a tank duel table.
        if weapon.caliber_mm_x10 < self.destruction.damage.min_caliber_mm * 10 {
            return None;
        }
        let at = range.min(i64::from(DUEL_DISTANCE_M));
        let mut m = MapBuilder::default();
        m.text("weapon", &weapon.name);
        m.int("distance_real_m", at);
        m.int("penetration_mm", whole(pen));
        m.int(
            "hits_to_destroy",
            Self::hits(
                self.combat.damage.vehicle_hp,
                self.combat.vehicle_damage(weapon),
            ),
        );
        let mut targets = Vec::new();
        for id in REFERENCE_TARGETS {
            let Some(target) = self.catalog.get(id) else {
                continue;
            };
            if target.faction == e.faction && e.faction != Faction::Neutral {
                continue;
            }
            let plates = self.plates(&self.combat_type(target));
            let mut t = MapBuilder::default();
            t.text("id", id);
            for plate in plates.iter().filter(|p| p.key.starts_with("hull")) {
                t.flag(plate.key, pen >= plate.effective);
                let reach = self.penetration_range_m(weapon, range, plate.effective);
                t.int(&format!("{}_real_m", plate.key), reach.unwrap_or(-1));
            }
            targets.push(t.build());
        }
        m.put("targets", Value::List(targets));
        Some(m.build())
    }

    /// Soldiers put out of action per game minute, in tenths, if every round
    /// hits: each direct weapon firing all the time (a round that does less
    /// damage than a soldier has hit points needs several), plus the
    /// indirect weapon wounding one soldier per shell.
    fn soldiers_per_min_x10(&self, combat: &CombatType) -> i64 {
        let soldier_hp = self.combat.damage.soldier_hp;
        let direct: i64 = combat
            .weapons
            .iter()
            .map(|w| {
                self.shots_per_min_x10(w) / Self::hits(soldier_hp, self.combat.infantry_damage(w))
            })
            .sum();
        let indirect = combat.indirect.as_ref().map_or(0, |w| {
            let ticks = (w.interval_s * Fixed::from_num(self.tick_hz))
                .round()
                .to_num::<i64>()
                .max(1);
            let hits = Self::hits(soldier_hp, self.indirect.blast.soldier_damage);
            i64::from(self.tick_hz) * 600 / (ticks * hits)
        });
        direct + indirect
    }

    fn against_infantry(&self, combat: &CombatType) -> Option<Value> {
        if combat.weapons.is_empty() && combat.indirect.is_none() {
            return None;
        }
        let mut m = MapBuilder::default();
        m.int("soldier_hp", self.combat.damage.soldier_hp);
        if let Some(best) = combat
            .weapons
            .iter()
            .map(|w| self.combat.infantry_damage(w))
            .max()
        {
            m.int("damage_per_hit", best);
            m.int(
                "hits_per_soldier",
                Self::hits(self.combat.damage.soldier_hp, best),
            );
        }
        let automatic = |w: &&WeaponType| w.rate_rpm >= self.combat.fire.automatic_rpm;
        let guns: i64 = combat
            .weapons
            .iter()
            .filter(automatic)
            .map(|w| i64::from(w.count))
            .sum();
        if guns > 0 {
            m.int("automatic_weapons", guns);
            let shots: i64 = combat
                .weapons
                .iter()
                .filter(automatic)
                .map(|w| self.shots_per_min_x10(w))
                .sum();
            m.int("automatic_shots_per_game_min_x10", shots);
        }
        if let Some(w) = &combat.indirect {
            m.int(
                "blast_radius_game_m_x10",
                tenths(self.indirect.blast_radius(w.caliber_mm_x10)),
            );
        }
        m.int(
            "soldiers_per_game_min_x10",
            self.soldiers_per_min_x10(combat),
        );
        Some(m.build())
    }

    /// Damage per hit and per game minute against map objects: the strongest
    /// weapon of at least the minimum caliber.
    fn structure_damage(&self, combat: &CombatType) -> Option<(u32, i64)> {
        let min_caliber = self.destruction.damage.min_caliber_mm * 10;
        let direct = combat
            .weapons
            .iter()
            .filter(|w| w.caliber_mm_x10 >= min_caliber)
            .map(|w| {
                let d = self.combat.vehicle_damage(w);
                (d, i64::from(d) * self.shots_per_min_x10(w) / 10)
            });
        let indirect = combat
            .indirect
            .iter()
            .filter(|w| w.caliber_mm_x10 >= min_caliber)
            .map(|w| {
                let ticks = (w.interval_s * Fixed::from_num(self.tick_hz))
                    .round()
                    .to_num::<i64>()
                    .max(1);
                let d = self.shell_damage(w.caliber_mm_x10);
                (d, i64::from(d) * i64::from(self.tick_hz) * 60 / ticks)
            });
        direct
            .chain(indirect)
            .max_by_key(|&(d, per_min)| (per_min, d))
    }

    fn against_structures(&self, combat: &CombatType) -> Option<Value> {
        if combat.weapons.is_empty() && combat.indirect.is_none() {
            return None;
        }
        let mut m = MapBuilder::default();
        m.int("min_caliber_mm", self.destruction.damage.min_caliber_mm);
        let damage = self.structure_damage(combat);
        m.flag("effective", damage.is_some());
        if let Some((per_hit, per_min)) = damage {
            m.int("damage_per_hit", per_hit);
            m.int("damage_per_game_min", per_min);
            let mut objects = Vec::new();
            for id in REFERENCE_OBJECTS {
                let Some(class) = self.destruction.class_of(id) else {
                    continue;
                };
                let class = &self.destruction.classes[class];
                let mut o = MapBuilder::default();
                o.text("id", id).int("hp", class.hp);
                o.int("hits", Self::hits(class.hp, per_hit));
                objects.push(o.build());
            }
            m.put("objects", Value::List(objects));
        }
        Some(m.build())
    }

    fn other(&self, e: &Entry, combat: &CombatType, mobile: bool) -> Option<Value> {
        let group = e.category.group();
        let mut m = MapBuilder::default();
        if group != Group::Environment && (mobile || group == Group::Fortification) {
            m.int("sight_game_m", whole(self.vision.range_m(Some(e.category))));
            let (num, den) = (self.scale.sight_range.num, self.scale.sight_range.den);
            m.int(
                "sight_real_m",
                whole(
                    self.vision.range_m(Some(e.category)) * Fixed::from_num(den)
                        / Fixed::from_num(num.max(1)),
                ),
            );
        }
        if mobile {
            let stock = LogisticsType::from_catalog(e, &self.scale);
            if stock.salvos > 0 {
                m.int("ammo_salvos", stock.salvos);
                let ticks = combat.indirect.as_ref().map_or_else(
                    || {
                        combat
                            .weapons
                            .first()
                            .map(|w| i64::from(self.combat.reload_ticks(w, self.tick_hz)))
                    },
                    |w| {
                        Some(
                            (w.interval_s * Fixed::from_num(self.tick_hz))
                                .round()
                                .to_num::<i64>()
                                .max(1),
                        )
                    },
                );
                if let Some(ticks) = ticks {
                    // Game minutes of continuous fire, in tenths.
                    m.int(
                        "ammo_game_min_x10",
                        i64::from(stock.salvos) * ticks * 10 / (i64::from(self.tick_hz) * 60),
                    );
                }
            }
            if stock.fuel_m > 0 {
                m.int("fuel_game_m", stock.fuel_m);
                m.opt("fuel_real_km", e.range_road_km);
            }
        }
        m.opt("crew", e.crew);
        m.opt("squad_size", e.squad_size);
        m.opt("passengers", e.passengers);
        let cost = if self.economy.structure(&e.id).is_some() {
            self.economy.structure_cost(&e.id, &self.catalog, combat)
        } else if matches!(group, Group::Infantry | Group::Vehicle | Group::Gun) {
            Some(self.economy.unit_cost(e, combat))
        } else {
            None
        };
        if let Some(cost) = cost {
            m.put("cost", ints(cost.resources.map(i64::from)));
            m.int("build_game_s_x10", i64::from(cost.time_ms) / 100);
        }
        (!m.is_empty()).then(|| m.build())
    }

    /// A map object: what it takes to destroy or flatten it.
    fn object(&self, e: &Entry) -> Option<Value> {
        let class = &self.destruction.classes[self.destruction.class_of(&e.id)?];
        let mut m = MapBuilder::default();
        m.int("hp", class.hp);
        m.int("min_caliber_mm", self.destruction.damage.min_caliber_mm);
        if class.crush_min_mass_kg > 0 {
            m.int("crush_min_mass_kg", class.crush_min_mass_kg);
        }
        Some(m.build())
    }

    /// The five ratings, each 0–100 (see [`RATINGS`]); a rating that does
    /// not apply to the unit is left out.
    fn ratings(&self, e: &Entry, unit: Option<&UnitType>, combat: &CombatType) -> Option<Value> {
        let r = &RATINGS;
        let score = |value: i64, reference: i64| (value * 100 / reference.max(1)).clamp(0, 100);
        let group = e.category.group();
        let mut m = MapBuilder::default();
        if let Some(unit) = unit {
            let mean = (tenths(unit.road_speed_m_per_s) + tenths(unit.speed_m_per_s)) / 2;
            m.int("speed", score(mean, r.speed_mps_x10));
        }
        if matches!(
            group,
            Group::Infantry | Group::Vehicle | Group::Gun | Group::Fortification
        ) {
            let protection = if combat.is_infantry() {
                (i64::from(combat.squad_size) * r.per_soldier).min(r.infantry_cap)
            } else if combat.armor.hull[0] > 0 {
                let plates = self.plates(combat);
                let face = |i: usize| {
                    // Hull and turret averaged where there is a turret.
                    let hull = plates[i].effective;
                    plates.get(i + 3).map_or(hull, |t| (hull + t.effective) / 2)
                };
                let weighted = (face(0) * 50 + face(1) * 30 + face(2) * 20) / 100;
                r.unarmored + score(whole(weighted), r.armor_mm) * (100 - r.unarmored) / 100
            } else {
                r.unarmored
            };
            m.int("protection", protection);
        }
        if !combat.weapons.is_empty() || combat.indirect.is_some() {
            let vehicles = self.best_gun(combat).map_or(0, |(_, pen, range)| {
                let reach = range.min(i64::from(DUEL_DISTANCE_M)) * (100 - r.short_range_pct)
                    / i64::from(DUEL_DISTANCE_M)
                    + r.short_range_pct;
                score(whole(pen), r.penetration_mm) * reach / 100
            });
            m.int("anti_vehicle", vehicles);
            m.int(
                "anti_infantry",
                score(self.soldiers_per_min_x10(combat), r.soldiers_per_min_x10),
            );
            m.int(
                "anti_structure",
                self.structure_damage(combat)
                    .map_or(0, |(_, per_min)| score(per_min, r.structure_damage_per_min)),
            );
        }
        (!m.is_empty()).then(|| m.build())
    }

    /// The card of one catalog entry.
    pub fn card(&self, e: &Entry) -> Value {
        let unit = UnitType::from_catalog(e, &self.scale);
        let combat = self.combat_type(e);
        let group = e.category.group();
        let mobile = unit.is_some();
        let mut m = MapBuilder::default();
        m.text("id", &e.id);
        m.text("category", e.category.code());
        if let Some(v) = self.ratings(e, unit.as_ref(), &combat) {
            m.put("ratings", v);
        }
        if let Some(v) = self.movement(e, unit.as_ref()) {
            m.put("movement", v);
        }
        if let Some(v) = self.protection(e, &combat, mobile) {
            m.put("protection", v);
        }
        // Aircraft attack through the air-support rules, not as units.
        if group != Group::Aircraft && group != Group::Environment {
            let weapons = self.weapons(&combat);
            if !weapons.is_empty() {
                m.put("weapons", Value::List(weapons));
            }
            if let Some(v) = self.indirect(e, &combat) {
                m.put("indirect", v);
            }
            if let Some(v) = self.against_vehicles(e, &combat) {
                m.put("against_vehicles", v);
            }
            if let Some(v) = self.against_infantry(&combat) {
                m.put("against_infantry", v);
            }
            if let Some(v) = self.against_structures(&combat) {
                m.put("against_structures", v);
            }
        }
        if let Some(v) = self.other(e, &combat, mobile) {
            m.put("other", v);
        }
        if let Some(v) = self.object(e) {
            m.put("object", v);
        }
        m.build()
    }
}

/// What the cards were computed with, for the page: the distances and the
/// comparison lists.
pub fn reference() -> Value {
    let mut m = MapBuilder::default();
    m.put("distances_real_m", ints(DISTANCES_M.map(i64::from)));
    m.int("duel_distance_real_m", DUEL_DISTANCE_M);
    m.put(
        "weapons",
        Value::List(
            REFERENCE_WEAPONS
                .iter()
                .map(|(id, _, label)| {
                    let mut w = MapBuilder::default();
                    w.text("weapon", label).text("from", id);
                    w.build()
                })
                .collect(),
        ),
    );
    let texts =
        |ids: &[&str]| Value::List(ids.iter().map(|id| Value::Text((*id).to_owned())).collect());
    m.put("targets", texts(&REFERENCE_TARGETS));
    m.put("objects", texts(&REFERENCE_OBJECTS));
    m.build()
}

/// The whole output of `cli unit-stats`: `reference` and `units`.
pub fn document(rules: &StatRules) -> Value {
    let mut m = MapBuilder::default();
    m.put("reference", reference());
    m.put("units", rules.all_cards());
    m.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> StatRules {
        StatRules::builtin().expect("builtin rules")
    }

    fn versus<'a>(card: &'a Value, weapon: &str) -> &'a Value {
        card.at(&["protection", "versus"])
            .expect("versus")
            .items()
            .iter()
            .find(|v| v.get("weapon") == Some(&Value::Text(weapon.to_owned())))
            .expect("comparison weapon")
    }

    #[test]
    fn every_catalog_entry_gets_a_card_and_two_runs_agree() {
        let rules = rules();
        let first = document(&rules).to_json();
        assert_eq!(first, document(&rules).to_json());
        for e in &rules.catalog.entries {
            let card = rules.card(e);
            assert_eq!(card.get("id"), Some(&Value::Text(e.id.clone())));
            if let Some(Value::Map(ratings)) = card.get("ratings") {
                for (name, value) in ratings {
                    let Value::Int(v) = value else {
                        panic!("{}: {name}", e.id)
                    };
                    assert!((0..=100).contains(v), "{}: {name} = {v}", e.id);
                }
            }
        }
    }

    #[test]
    fn tiger_front_holds_against_the_75mm_at_500_m() {
        let rules = rules();
        let tiger = rules.card_of("de_tiger1_e").expect("tiger");
        // The same check by hand: 7.5 cm PaK 40 at 500 m against 100 mm at 9°.
        let pak =
            CombatType::from_catalog(rules.catalog.get("de_pak40").expect("pak"), &rules.scale);
        let pen = rules
            .combat
            .penetration_at(&pak.weapons[0], Fixed::from_num(500));
        let effective = tiger
            .int(&["protection", "hull_front_effective_mm"])
            .expect("effective");
        assert!(
            effective >= 100,
            "slope must not thin the plate: {effective}"
        );
        let reach = versus(&tiger, "gun_75mm")
            .int(&["hull_front_real_m"])
            .expect("reach");
        assert_eq!(
            whole(pen) >= effective,
            reach >= 500,
            "pen {pen}, plate {effective}, reach {reach}"
        );
        // Rifles never hurt it.
        assert_eq!(versus(&tiger, "rifle").int(&["hull_side_real_m"]), Some(-1));
    }

    #[test]
    fn the_88_pierces_the_t34_side_at_its_whole_range() {
        let rules = rules();
        let t34 = rules.card_of("su_t34_76_1943").expect("t34");
        let v = versus(&t34, "gun_88mm");
        assert_eq!(v.int(&["hull_side_real_m"]), v.int(&["range_real_m"]));
        // And the Tiger's own card says it knocks out the T-34 from the side.
        let tiger = rules.card_of("de_tiger1_e").expect("tiger");
        let target = tiger
            .at(&["against_vehicles", "targets"])
            .expect("targets")
            .items()
            .iter()
            .find(|t| t.get("id") == Some(&Value::Text("su_t34_76_1943".into())))
            .expect("t34 target");
        assert_eq!(target.get("hull_side"), Some(&Value::Bool(true)));
        // No tank of its own faction in the list.
        assert!(
            tiger
                .at(&["against_vehicles", "targets"])
                .expect("targets")
                .items()
                .iter()
                .all(|t| {
                    !matches!(t.get("id"), Some(Value::Text(id)) if id.starts_with("de_"))
                })
        );
    }

    #[test]
    fn three_75mm_rounds_bring_down_the_stone_wall() {
        let rules = rules();
        let pz4 = rules.card_of("de_pz4_h").expect("pz4");
        let objects = pz4
            .at(&["against_structures", "objects"])
            .expect("objects")
            .items();
        let hits = |id: &str| {
            objects
                .iter()
                .find(|o| o.get("id") == Some(&Value::Text(id.to_owned())))
                .and_then(|o| o.int(&["hits"]))
        };
        assert_eq!(hits("env_wall_stone"), Some(3));
        assert_eq!(hits("env_tree_deciduous"), Some(1));
        // Rifles are below the minimum caliber.
        let rifles = rules.card_of("de_inf_rifle_squad").expect("rifles");
        assert_eq!(
            rifles.at(&["against_structures", "effective"]),
            Some(&Value::Bool(false))
        );
        assert_eq!(rifles.int(&["ratings", "anti_structure"]), Some(0));
    }

    #[test]
    fn cards_show_only_what_applies() {
        let rules = rules();
        let truck = rules.card_of("de_opel_blitz").expect("truck");
        assert!(truck.get("movement").is_some());
        assert!(truck.get("weapons").is_none() && truck.get("against_vehicles").is_none());
        assert!(truck.int(&["ratings", "anti_vehicle"]).is_none());
        let tree = rules.card_of("env_tree_deciduous").expect("tree");
        assert!(tree.get("ratings").is_none() && tree.get("movement").is_none());
        assert_eq!(tree.int(&["object", "hp"]), Some(400));
        let howitzer = rules.card_of("de_lefh18").expect("howitzer");
        assert!(
            howitzer
                .int(&["indirect", "range_game_m"])
                .is_some_and(|m| m > 500)
        );
        // Game speed is the real speed times the time factor.
        let pz4 = rules.card_of("de_pz4_h").expect("pz4");
        let real = pz4.int(&["movement", "road_real_kmh_x10"]).expect("real");
        let game = pz4.int(&["movement", "road_game_mps_x10"]).expect("game");
        assert_eq!(game, (real * 20 + 18) / 36);
    }

    #[test]
    fn json_is_plain_and_stable() {
        let mut m = MapBuilder::default();
        m.text("name", "8.8 cm \"KwK\"")
            .put("values", ints([1, 2]))
            .flag("ok", true);
        assert_eq!(
            m.build().to_json(),
            "{\n  \"name\": \"8.8 cm \\\"KwK\\\"\",\n  \"values\": [1, 2],\n  \"ok\": true\n}\n"
        );
    }
}
