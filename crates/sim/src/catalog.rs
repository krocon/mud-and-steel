//! The unit catalog (`data/catalog/*.toml`): real-world data of every unit,
//! gun, fortification, aircraft and environment object.
//!
//! All values are integers in the unit named by the field suffix (`_mm`,
//! `_kg`, `_kmh_x10` = tenths of km/h, ...), so parsing never touches floating
//! point. Values are real (historical) values; the conversion to game values
//! happens only in [`crate::scale`]. The field reference is in
//! `data/catalog/SCHEMA.md`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The catalog files compiled into the simulation, as `(file name, contents)`.
///
/// Compiling them in guarantees that all peers use identical data.
pub const BUILTIN_FILES: [(&str, &str); 4] = [
    ("de.toml", include_str!("../../../data/catalog/de.toml")),
    ("su.toml", include_str!("../../../data/catalog/su.toml")),
    ("us.toml", include_str!("../../../data/catalog/us.toml")),
    (
        "neutral.toml",
        include_str!("../../../data/catalog/neutral.toml"),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Faction {
    De,
    Su,
    Us,
    Neutral,
}

impl Faction {
    pub const fn code(self) -> &'static str {
        match self {
            Self::De => "de",
            Self::Su => "su",
            Self::Us => "us",
            Self::Neutral => "neutral",
        }
    }
}

/// Coarse grouping of [`Category`] values; decides which fields are required.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Infantry,
    Vehicle,
    Gun,
    Fortification,
    Aircraft,
    Environment,
}

impl Group {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Infantry => "Infantry",
            Self::Vehicle => "Vehicles",
            Self::Gun => "Guns",
            Self::Fortification => "Fortifications",
            Self::Aircraft => "Aircraft",
            Self::Environment => "Environment",
        }
    }
}

macro_rules! categories {
    ($($group:ident => [$($variant:ident = $code:literal),* $(,)?]),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum Category {
            $($(#[serde(rename = $code)] $variant,)*)*
        }

        impl Category {
            /// Every category, in declaration order.
            pub const ALL: &[Category] = &[$($(Self::$variant,)*)*];

            pub const fn group(self) -> Group {
                match self {
                    $($(Self::$variant => Group::$group,)*)*
                }
            }

            pub const fn code(self) -> &'static str {
                match self {
                    $($(Self::$variant => $code,)*)*
                }
            }
        }
    };
}

categories! {
    Infantry => [
        InfRifle = "inf_rifle",
        InfLmg = "inf_lmg",
        InfHmg = "inf_hmg",
        InfMortarLight = "inf_mortar_light",
        InfMortarMedium = "inf_mortar_medium",
        InfRecon = "inf_recon",
        InfSniper = "inf_sniper",
        InfMedic = "inf_medic",
        InfEngineer = "inf_engineer",
        InfAt = "inf_at",
        InfFlamethrower = "inf_flamethrower",
        InfCommand = "inf_command",
        // Prompt 049: survivors of a destroyed vehicle; never built.
        InfCrew = "inf_crew",
    ],
    Vehicle => [
        TankLight = "tank_light",
        TankMedium = "tank_medium",
        TankHeavy = "tank_heavy",
        TankDestroyer = "tank_destroyer",
        AssaultGun = "assault_gun",
        Halftrack = "halftrack",
        Truck = "truck",
        ReconVehicle = "recon_vehicle",
        SelfPropelled = "self_propelled",
        Bridgelayer = "bridgelayer",
        // Prompt 062: rocket launchers on a vehicle (salvo fire).
        RocketLauncher = "rocket_launcher",
    ],
    Gun => [
        GunAt = "gun_at",
        GunAaLight = "gun_aa_light",
        GunAaHeavy = "gun_aa_heavy",
        GunHowitzer = "gun_howitzer",
        MortarHeavy = "mortar_heavy",
    ],
    Fortification => [
        FortBunker = "fort_bunker",
        FortTrench = "fort_trench",
        FortSandbag = "fort_sandbag",
        FortObstacle = "fort_obstacle",
        FortWire = "fort_wire",
        FortMinefieldAt = "fort_minefield_at",
        FortMinefieldAp = "fort_minefield_ap",
    ],
    Aircraft => [
        AircraftFighter = "aircraft_fighter",
        AircraftAttack = "aircraft_attack",
        AircraftBomber = "aircraft_bomber",
        AircraftRecon = "aircraft_recon",
        AircraftTransport = "aircraft_transport",
    ],
    Environment => [
        EnvTree = "env_tree",
        EnvBush = "env_bush",
        EnvGrass = "env_grass",
        EnvRock = "env_rock",
        EnvBuilding = "env_building",
        EnvBridge = "env_bridge",
        EnvFence = "env_fence",
        EnvPole = "env_pole",
        EnvProp = "env_prop",
    ],
}

/// One weapon of a catalog entry.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weapon {
    pub name: String,
    #[serde(default = "one")]
    pub count: u16,
    /// Caliber in tenths of a millimeter (7.92 mm = 79). Absent for e.g.
    /// flamethrowers.
    pub caliber_mm_x10: Option<u32>,
    #[serde(default)]
    pub ammo: String,
    /// Rounds per minute: cyclic for automatic weapons, practical otherwise.
    pub rate_rpm: Option<u32>,
    pub muzzle_velocity_mps: Option<u32>,
    /// Real effective range in meters (before range compression): the
    /// maximum or sight value of direct-fire weapons, the maximum range of
    /// indirect ones (which the simulation uses for them).
    pub range_effective_m: Option<u32>,
    /// Real typical combat distance in meters (prompt 067): the usual
    /// distance at which the weapon was used with effect against its main
    /// target. The simulation's direct fire uses only this value. Absent
    /// for the indirect weapon of mortars, howitzers and self-propelled guns.
    pub range_typical_m: Option<u32>,
    pub ammo_carried: Option<u32>,
    /// Real effective range against aircraft (slant range / ceiling) in
    /// meters, for anti-aircraft guns (prompt 033).
    pub range_aa_m: Option<u32>,
    /// The weapon sits on an anti-aircraft mount (MGs, prompt 033).
    #[serde(default)]
    pub aa: bool,
    /// Nominal mass of one bomb in kilograms (aircraft, prompt 033).
    pub bomb_kg: Option<u32>,
}

const fn one() -> u16 {
    1
}

/// Where values of an entry come from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// URL, technical manual number or book with page.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Which fields this source covers.
    pub covers: String,
}

/// One catalog entry. See `data/catalog/SCHEMA.md`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub faction: Faction,
    pub category: Category,
    pub name_de: String,
    pub name_en: String,
    #[serde(default)]
    pub designation: String,
    pub year_from: Option<u16>,
    pub year_to: Option<u16>,
    pub tier: u8,
    /// Path of the exported model below `godot/`, empty until it exists.
    #[serde(default)]
    pub model: String,

    pub length_mm: Option<u32>,
    pub length_hull_mm: Option<u32>,
    pub width_mm: Option<u32>,
    pub height_mm: Option<u32>,
    pub wingspan_mm: Option<u32>,
    pub mass_kg: Option<u32>,

    pub crew: Option<u16>,
    /// Troop seats besides the crew (trucks and halftracks, prompt 034).
    pub passengers: Option<u16>,
    pub squad_size: Option<u16>,
    /// Length of the carried bridge (bridge-laying tanks, prompt 055).
    pub bridge_length_mm: Option<u32>,
    /// Load limit of the carried bridge in kilograms (prompt 055).
    pub bridge_load_kg: Option<u32>,

    pub speed_road_kmh_x10: Option<u32>,
    pub speed_offroad_kmh_x10: Option<u32>,
    pub speed_water_kmh_x10: Option<u32>,
    pub range_road_km: Option<u32>,
    pub power_hp: Option<u32>,
    pub speed_max_kmh_x10: Option<u32>,
    pub speed_cruise_kmh_x10: Option<u32>,
    pub ceiling_m: Option<u32>,

    pub armor_hull_front_mm: Option<u16>,
    pub armor_hull_side_mm: Option<u16>,
    pub armor_hull_rear_mm: Option<u16>,
    pub armor_turret_front_mm: Option<u16>,
    pub armor_turret_side_mm: Option<u16>,
    pub armor_turret_rear_mm: Option<u16>,
    /// Slope of the upper front hull plate from vertical, in degrees.
    pub armor_hull_front_deg: Option<u16>,
    /// Total horizontal traverse of the main gun in a casemate (degrees);
    /// absent for vehicles with a rotating turret.
    pub gun_traverse_deg: Option<u16>,

    #[serde(default)]
    pub weapon: Vec<Weapon>,
    #[serde(default)]
    pub source: Vec<Source>,
    /// Names of fields whose values are uncertain (conflicting or estimated).
    #[serde(default)]
    pub uncertain: Vec<String>,
    #[serde(default)]
    pub note: String,
}

/// Every field name an [`Entry`] may list in `uncertain`.
pub const FIELD_NAMES: &[&str] = &[
    "designation",
    "year_from",
    "year_to",
    "length_mm",
    "length_hull_mm",
    "width_mm",
    "height_mm",
    "wingspan_mm",
    "mass_kg",
    "crew",
    "passengers",
    "squad_size",
    "bridge_length_mm",
    "bridge_load_kg",
    "speed_road_kmh_x10",
    "speed_offroad_kmh_x10",
    "speed_water_kmh_x10",
    "range_road_km",
    "power_hp",
    "speed_max_kmh_x10",
    "speed_cruise_kmh_x10",
    "ceiling_m",
    "armor_hull_front_mm",
    "armor_hull_side_mm",
    "armor_hull_rear_mm",
    "armor_turret_front_mm",
    "armor_turret_side_mm",
    "armor_turret_rear_mm",
    "armor_hull_front_deg",
    "gun_traverse_deg",
    "weapon",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogFile {
    #[serde(default)]
    unit: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CatalogError(pub String);

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid catalog: {}", self.0)
    }
}

impl std::error::Error for CatalogError {}

/// The validated catalog. Entries keep file order (files in the given order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    pub entries: Vec<Entry>,
}

impl Catalog {
    /// Parses and validates the catalog compiled into the simulation.
    pub fn builtin() -> Result<Self, CatalogError> {
        Self::parse(&BUILTIN_FILES)
    }

    /// Parses and validates catalog files given as `(file name, contents)`.
    ///
    /// A file named `<faction>.toml` may only contain entries of that faction.
    /// All errors are collected, one per line.
    pub fn parse(files: &[(&str, &str)]) -> Result<Self, CatalogError> {
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for (name, contents) in files {
            match toml::from_str::<CatalogFile>(contents) {
                Ok(file) => {
                    let expected = name.strip_suffix(".toml").unwrap_or(name);
                    for entry in file.unit {
                        if entry.faction.code() != expected {
                            errors.push(format!(
                                "{name}: '{}' has faction '{}'",
                                entry.id,
                                entry.faction.code()
                            ));
                        }
                        entries.push(entry);
                    }
                }
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        }

        let mut ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        ids.sort_unstable();
        for pair in ids.windows(2).filter(|pair| pair[0] == pair[1]) {
            errors.push(format!("duplicate id '{}'", pair[0]));
        }
        for entry in &entries {
            errors.extend(
                validate(entry)
                    .into_iter()
                    .map(|e| format!("{}: {e}", entry.id)),
            );
        }

        if errors.is_empty() {
            Ok(Self { entries })
        } else {
            Err(CatalogError(errors.join("\n")))
        }
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

/// Checks required fields and plausible value ranges of one entry.
fn validate(e: &Entry) -> Vec<String> {
    let mut errors = Vec::new();
    let mut need = |name: &str, value: Option<u32>, min: u32, max: u32| match value {
        None => errors.push(format!("missing {name}")),
        Some(v) if v < min || v > max => {
            errors.push(format!("{name} = {v} outside {min}..={max}"));
        }
        Some(_) => {}
    };
    let group = e.category.group();
    let wide = |v: Option<u16>| v.map(u32::from);

    match group {
        Group::Infantry => {
            need("squad_size", wide(e.squad_size), 1, 15);
            need("speed_road_kmh_x10", e.speed_road_kmh_x10, 30, 80);
            need("speed_offroad_kmh_x10", e.speed_offroad_kmh_x10, 20, 80);
        }
        Group::Vehicle => {
            need("length_mm", e.length_mm, 2_500, 12_000);
            need("width_mm", e.width_mm, 1_500, 4_000);
            // 1.4 m: the T-27A tankette is 1.45 m high; crew 12: the
            // multi-turret T-35 had 11 (prompt 063).
            need("height_mm", e.height_mm, 1_400, 4_000);
            need("mass_kg", e.mass_kg, 1_000, 80_000);
            need("crew", wide(e.crew), 1, 12);
            need("speed_road_kmh_x10", e.speed_road_kmh_x10, 150, 1_200);
            need("speed_offroad_kmh_x10", e.speed_offroad_kmh_x10, 50, 1_000);
            need("range_road_km", e.range_road_km, 50, 1_000);
            // Trucks and rocket launchers on soft trucks (BM-13) may be
            // unarmored.
            if !matches!(e.category, Category::Truck | Category::RocketLauncher) {
                need("armor_hull_front_mm", wide(e.armor_hull_front_mm), 5, 300);
            }
        }
        Group::Gun => {
            need("length_mm", e.length_mm, 500, 12_000);
            need("width_mm", e.width_mm, 300, 5_000);
            need("height_mm", e.height_mm, 300, 4_000);
            need("mass_kg", e.mass_kg, 20, 20_000);
            need("crew", wide(e.crew), 1, 15);
        }
        Group::Fortification => {
            need("length_mm", e.length_mm, 100, 100_000);
            need("width_mm", e.width_mm, 100, 100_000);
            need("height_mm", e.height_mm, 0, 10_000);
        }
        Group::Aircraft => {
            need("length_mm", e.length_mm, 5_000, 25_000);
            need("wingspan_mm", e.wingspan_mm, 8_000, 35_000);
            need("height_mm", e.height_mm, 1_500, 7_000);
            need("mass_kg", e.mass_kg, 300, 40_000);
            need("crew", wide(e.crew), 1, 10);
            need("speed_max_kmh_x10", e.speed_max_kmh_x10, 1_000, 8_000);
            need("ceiling_m", e.ceiling_m, 1_000, 15_000);
        }
        Group::Environment => {
            need("length_mm", e.length_mm, 10, 100_000);
            need("width_mm", e.width_mm, 10, 100_000);
            need("height_mm", e.height_mm, 10, 60_000);
        }
    }

    if let (Some(road), Some(off)) = (e.speed_road_kmh_x10, e.speed_offroad_kmh_x10)
        && off > road
    {
        errors.push(format!(
            "speed_offroad_kmh_x10 {off} > speed_road_kmh_x10 {road}"
        ));
    }
    if let (Some(hull), Some(total)) = (e.length_hull_mm, e.length_mm)
        && hull > total
    {
        errors.push(format!("length_hull_mm {hull} > length_mm {total}"));
    }
    for (name, value) in [
        ("armor_hull_side_mm", e.armor_hull_side_mm),
        ("armor_hull_rear_mm", e.armor_hull_rear_mm),
        ("armor_turret_front_mm", e.armor_turret_front_mm),
        ("armor_turret_side_mm", e.armor_turret_side_mm),
        ("armor_turret_rear_mm", e.armor_turret_rear_mm),
    ] {
        if value.is_some_and(|v| v > 300) {
            errors.push(format!("{name} above 300"));
        }
    }
    if let Some(v) = e.passengers
        && (!matches!(group, Group::Vehicle | Group::Aircraft) || !(1..=30).contains(&v))
    {
        errors.push(format!(
            "passengers = {v}: only vehicles and aircraft, 1..=30"
        ));
    }
    if e.category == Category::AircraftTransport && e.passengers.is_none() {
        errors.push("missing passengers".into());
    }
    if e.category == Category::Bridgelayer {
        need_bridge(
            &mut errors,
            "bridge_length_mm",
            e.bridge_length_mm,
            4_000,
            30_000,
        );
        need_bridge(
            &mut errors,
            "bridge_load_kg",
            e.bridge_load_kg,
            5_000,
            80_000,
        );
    } else if e.bridge_length_mm.is_some() || e.bridge_load_kg.is_some() {
        errors.push("bridge_length_mm / bridge_load_kg: only bridgelayer".into());
    }
    if e.armor_hull_front_deg.is_some_and(|v| v > 80) {
        errors.push("armor_hull_front_deg above 80".into());
    }
    if let Some(v) = e.gun_traverse_deg
        && (group != Group::Vehicle || !(1..=359).contains(&v))
    {
        errors.push(format!(
            "gun_traverse_deg = {v}: only vehicles with a casemate, 1..=359"
        ));
    }

    let is_snake = !e.id.is_empty()
        && e.id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if !is_snake {
        errors.push("id must be snake_case ASCII".into());
    }
    if e.faction != Faction::Neutral && !e.id.starts_with(&format!("{}_", e.faction.code())) {
        errors.push(format!("id must start with '{}_'", e.faction.code()));
    }
    if e.name_de.trim().is_empty() || e.name_en.trim().is_empty() {
        errors.push("missing name_de or name_en".into());
    }
    // Tier 3 (prompt 063): taken over from the vehicle references, not
    // buildable yet.
    if !matches!(e.tier, 1..=3) {
        errors.push(format!("tier {} must be 1, 2 or 3", e.tier));
    }
    if e.faction != Faction::Neutral {
        if e.designation.trim().is_empty() {
            errors.push("missing designation".into());
        }
        match (e.year_from, e.year_to) {
            (Some(from), Some(to)) if (1900..=1945).contains(&from) && from <= to && to <= 1960 => {
            }
            _ => errors.push("year_from/year_to missing or implausible".into()),
        }
    }
    // Medics and liaison/recon aircraft may be unarmed.
    let may_be_unarmed = matches!(
        e.category,
        Category::InfMedic | Category::AircraftRecon | Category::AircraftTransport
    );
    let armed_group = matches!(group, Group::Infantry | Group::Gun | Group::Aircraft);
    let combat_vehicle = matches!(
        e.category,
        Category::TankLight
            | Category::TankMedium
            | Category::TankHeavy
            | Category::TankDestroyer
            | Category::AssaultGun
            | Category::SelfPropelled
            | Category::RocketLauncher
    );
    if ((armed_group && !may_be_unarmed) || combat_vehicle) && e.weapon.is_empty() {
        errors.push("missing weapon".into());
    }
    for w in &e.weapon {
        if w.name.trim().is_empty() || w.count == 0 {
            errors.push("weapon needs a name and count >= 1".into());
        }
        if w.caliber_mm_x10.is_some_and(|c| !(50..=2_000).contains(&c)) {
            errors.push(format!(
                "weapon '{}': caliber_mm_x10 outside 50..=2000",
                w.name
            ));
        }
        if w.range_effective_m.is_some_and(|r| r > 30_000) {
            errors.push(format!(
                "weapon '{}': range_effective_m above 30000",
                w.name
            ));
        }
        // Typical combat distance (prompt 067): every weapon with a range
        // has one, except the indirect weapon (the first weapon of an
        // indirect category), whose range is a maximum range by definition.
        let indirect_main = crate::indirect::is_indirect_category(e.category)
            && e.weapon.first().is_some_and(|first| std::ptr::eq(first, w));
        match (w.range_effective_m, w.range_typical_m) {
            (_, Some(_)) if indirect_main => errors.push(format!(
                "weapon '{}': the indirect weapon has no range_typical_m",
                w.name
            )),
            (Some(_), None) if !indirect_main => errors.push(format!(
                "weapon '{}': range_effective_m without range_typical_m",
                w.name
            )),
            (None, Some(_)) => errors.push(format!(
                "weapon '{}': range_typical_m without range_effective_m",
                w.name
            )),
            (Some(effective), Some(typical)) if typical == 0 || typical > effective => {
                errors.push(format!(
                    "weapon '{}': range_typical_m must be 1..=range_effective_m",
                    w.name
                ));
            }
            _ => {}
        }
        if w.range_aa_m.is_some_and(|r| !(100..=20_000).contains(&r)) {
            errors.push(format!(
                "weapon '{}': range_aa_m outside 100..=20000",
                w.name
            ));
        }
        if w.bomb_kg.is_some_and(|kg| !(1..=5_000).contains(&kg)) {
            errors.push(format!("weapon '{}': bomb_kg outside 1..=5000", w.name));
        }
        if w.bomb_kg.is_some() && group != Group::Aircraft {
            errors.push(format!("weapon '{}': bombs only on aircraft", w.name));
        }
    }
    if e.source.is_empty() {
        errors.push("missing source".into());
    }
    for s in &e.source {
        if s.reference.trim().is_empty() || s.covers.trim().is_empty() {
            errors.push("source needs ref and covers".into());
        }
    }
    for field in &e.uncertain {
        if !FIELD_NAMES.contains(&field.as_str()) {
            errors.push(format!("uncertain lists unknown field '{field}'"));
        }
    }
    errors
}

/// A required field of a bridge-laying tank (prompt 055).
fn need_bridge(errors: &mut Vec<String>, name: &str, value: Option<u32>, min: u32, max: u32) {
    match value {
        None => errors.push(format!("missing {name}")),
        Some(v) if !(min..=max).contains(&v) => {
            errors.push(format!("{name} = {v} outside {min}..={max}"));
        }
        Some(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TANK: &str = r#"
[[unit]]
id = "de_test"
faction = "de"
category = "tank_medium"
name_de = "Test"
name_en = "Test"
designation = "T"
year_from = 1943
year_to = 1945
tier = 2
length_mm = 7000
width_mm = 3000
height_mm = 2600
mass_kg = 25000
crew = 5
speed_road_kmh_x10 = 380
speed_offroad_kmh_x10 = 160
range_road_km = 200
armor_hull_front_mm = 80
weapon = [{ name = "gun", caliber_mm_x10 = 750 }]
source = [{ ref = "book", covers = "all" }]
"#;

    #[test]
    fn valid_entry_parses() {
        let catalog = Catalog::parse(&[("de.toml", TANK)]).unwrap();
        assert_eq!(catalog.get("de_test").unwrap().crew, Some(5));
    }

    #[test]
    fn rejects_wrong_file_duplicates_and_missing_fields() {
        let err = Catalog::parse(&[("su.toml", TANK)]).unwrap_err();
        assert!(err.0.contains("has faction 'de'"), "{err}");

        let err = Catalog::parse(&[("de.toml", TANK), ("de.toml", TANK)]).unwrap_err();
        assert!(err.0.contains("duplicate id 'de_test'"), "{err}");

        let broken = TANK
            .replace("crew = 5\n", "")
            .replace("mass_kg = 25000", "mass_kg = 9");
        let err = Catalog::parse(&[("de.toml", &broken)]).unwrap_err();
        assert!(err.0.contains("missing crew"), "{err}");
        assert!(err.0.contains("mass_kg = 9 outside"), "{err}");
    }

    #[test]
    fn rejects_unknown_fields_and_uncertain_names() {
        let unknown = TANK.replace("tier = 2", "tier = 2\nspeed = 3");
        assert!(Catalog::parse(&[("de.toml", &unknown)]).is_err());
        let bad = TANK.replace("tier = 2", "tier = 2\nuncertain = [\"colour\"]");
        let err = Catalog::parse(&[("de.toml", &bad)]).unwrap_err();
        assert!(err.0.contains("unknown field 'colour'"), "{err}");
    }

    #[test]
    fn tier_3_is_the_last_tier() {
        let three = TANK.replace("tier = 2", "tier = 3");
        assert!(Catalog::parse(&[("de.toml", &three)]).is_ok());
        let four = TANK.replace("tier = 2", "tier = 4");
        let err = Catalog::parse(&[("de.toml", &four)]).unwrap_err();
        assert!(err.0.contains("tier 4 must be 1, 2 or 3"), "{err}");
    }
}
