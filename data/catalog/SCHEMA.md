# Catalog schema

One TOML file per faction (`de.toml`, `su.toml`, `us.toml`) plus `neutral.toml`
for environment objects and faction-neutral fortifications. Each entry is a
`[[unit]]` table. The simulation parses these files strictly
(`crates/sim/src/catalog.rs`): unknown fields are errors.

Rules:

- **Integers only.** The unit is in the field name. `_kmh_x10` means tenths of km/h
  (38 km/h = `380`), `caliber_mm_x10` tenths of a millimeter (7.92 mm = `79`).
- **Real values.** Historical, unscaled values. Conversion to game values happens
  only through `data/scale.toml`.
- **Every value needs a source.** List the sources in `source`, each with what it
  covers. Values that are disputed or estimated go into `uncertain` (field names)
  and get a `note`. Mark uncertain values; never guess silently.
- Omit fields that do not apply (e.g. armor of a truck). Do not write `0` for unknown.
- Validate with `cargo run -p cli -- catalog-check data/catalog/<file>.toml`.
- After any change run `just catalog-doc`; `docs/UNIT_CATALOG.md` is generated.

## Fields

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Unique, snake_case. Faction entries start with `<faction>_` (`de_pz4_h`). |
| `faction` | string | `de`, `su`, `us` or `neutral`; must match the file name. |
| `category` | string | See below. |
| `name_de`, `name_en` | string | Display names. |
| `designation` | string | Historical designation (`Sd.Kfz. 161/2`). Required for faction entries. |
| `year_from`, `year_to` | int | Years in service (production/first use until end of use in WW2 or later). Required for faction entries. |
| `tier` | int | `1` = core unit (prompt 006), `2` = the rest. |
| `model` | string | Path of the exported model, e.g. `assets/models/de/de_pz4_h.glb`. Empty until it exists. |
| `length_mm` | int | Overall length, including the gun pointing forward. |
| `length_hull_mm` | int | Length without the gun (vehicles with a gun). |
| `width_mm`, `height_mm` | int | Overall width and height (with tracks, skirts as fitted; height of towed guns in travel or firing position, say which in `note`). |
| `wingspan_mm` | int | Aircraft. |
| `mass_kg` | int | Combat weight (vehicles, aircraft: loaded/normal takeoff weight; guns: in firing position). |
| `crew` | int | Vehicles, guns, aircraft. |
| `passengers` | int | Troop seats besides the crew (trucks, halftracks; prompt 034). Vehicles only, 1–30. |
| `squad_size` | int | Infantry: soldiers in the team. |
| `speed_road_kmh_x10` | int | Maximum road speed (vehicles); marching speed (infantry, ~50). |
| `speed_offroad_kmh_x10` | int | Cross-country speed (vehicles); infantry cross-country (~40). |
| `speed_water_kmh_x10` | int | Amphibious vehicles only. |
| `range_road_km` | int | Road range on internal fuel. |
| `power_hp` | int | Engine power in metric hp. |
| `speed_max_kmh_x10` | int | Aircraft maximum speed. |
| `speed_cruise_kmh_x10` | int | Aircraft cruise speed. |
| `ceiling_m` | int | Aircraft service ceiling. |
| `armor_hull_front_mm`, `armor_hull_side_mm`, `armor_hull_rear_mm` | int | Hull armor (thickest plate of that face, e.g. upper front). |
| `armor_turret_front_mm`, `armor_turret_side_mm`, `armor_turret_rear_mm` | int | Turret (or casemate/superstructure) armor. Gun shield of towed guns goes into `armor_turret_front_mm`. |
| `armor_hull_front_deg` | int | Upper front plate slope from vertical. |
| `gun_traverse_deg` | int | Vehicles whose main gun sits in a casemate (no rotating turret): total horizontal traverse of the gun (StuG III G: `20` = 10° left and right). Absent means a turret that turns 360°. Used by combat (prompt 017). |
| `weapon` | list | See below. Infantry: all weapons of the team, with `count`. |
| `source` | list | `{ ref = "...", covers = "..." }`; `ref` is a URL, TM number or book with page. |
| `uncertain` | list | Field names (or `"weapon"`) with uncertain values. |
| `note` | string | Free text: assumptions, variant chosen, conflicts between sources. |

### `weapon` entries

| Field | Type | Meaning |
|---|---|---|
| `name` | string | `7.5 cm KwK 40 L/48`, `MG 42`, `Kar98k`. |
| `count` | int | Default 1. |
| `caliber_mm_x10` | int | Optional (absent for flamethrowers). |
| `ammo` | string | Main ammunition types, e.g. `PzGr 39 APCBC, Sprgr 34 HE`. |
| `rate_rpm` | int | Rounds per minute: cyclic rate for automatic weapons (MGs, SMGs, automatic cannon), practical rate for everything else. Practical rate of automatic weapons goes into `note`. |
| `muzzle_velocity_mps` | int | Of the main AP (or standard) round. |
| `range_effective_m` | int | Real effective range (direct fire) or maximum range (indirect fire). |
| `ammo_carried` | int | Rounds carried. |

## Categories

| Group | Categories | Required fields |
|---|---|---|
| Infantry (`weapon` optional for `inf_medic`) | `inf_rifle`, `inf_lmg`, `inf_hmg`, `inf_mortar_light`, `inf_mortar_medium`, `inf_recon`, `inf_sniper`, `inf_medic`, `inf_engineer`, `inf_at`, `inf_flamethrower`, `inf_command` | `squad_size`, road and offroad speed, `weapon` |
| Vehicles | `tank_light`, `tank_medium`, `tank_heavy`, `tank_destroyer`, `assault_gun`, `halftrack`, `truck`, `recon_vehicle`, `self_propelled` | length, width, height, mass, crew, road/offroad speed, range; front hull armor (not trucks); `weapon` for combat vehicles |
| Guns | `gun_at`, `gun_aa_light`, `gun_aa_heavy`, `gun_howitzer`, `mortar_heavy` | length, width, height, mass, crew, `weapon` |
| Fortifications | `fort_bunker`, `fort_trench`, `fort_sandbag`, `fort_obstacle`, `fort_wire`, `fort_minefield_at`, `fort_minefield_ap` | length, width, height (0 allowed for minefields) |
| Aircraft | `aircraft_fighter`, `aircraft_attack`, `aircraft_bomber`, `aircraft_recon` | length, wingspan, height, mass, crew, max speed, ceiling, `weapon` (optional for `aircraft_recon`) |
| Environment | `env_tree`, `env_bush`, `env_grass`, `env_rock`, `env_building`, `env_bridge`, `env_fence`, `env_pole`, `env_prop` | length, width, height |

`sources` is required for every entry. The validation also checks plausible
ranges (see `validate` in `crates/sim/src/catalog.rs`).

## Example

```toml
[[unit]]
id = "de_pz4_h"
faction = "de"
category = "tank_medium"
name_de = "Panzerkampfwagen IV Ausf. H"
name_en = "Panzer IV Ausf. H"
designation = "Sd.Kfz. 161/2"
year_from = 1943
year_to = 1945
tier = 1
length_mm = 7020
length_hull_mm = 5890
width_mm = 2880
height_mm = 2680
mass_kg = 25000
crew = 5
speed_road_kmh_x10 = 380
speed_offroad_kmh_x10 = 160
range_road_km = 210
power_hp = 300
armor_hull_front_mm = 80
armor_hull_front_deg = 10
weapon = [
    { name = "7.5 cm KwK 40 L/48", caliber_mm_x10 = 750, ammo = "PzGr 39 APCBC, Sprgr 34 HE", muzzle_velocity_mps = 790, range_effective_m = 1500, ammo_carried = 87 },
    { name = "MG 34", count = 2, caliber_mm_x10 = 79, rate_rpm = 900, ammo_carried = 3150 },
]
source = [
    { ref = "Jentz/Doyle, Panzer Tracts 4-3 (2001)", covers = "dimensions, mass, armor, speeds" },
]
uncertain = ["speed_offroad_kmh_x10"]
note = "Values illustrate the format; the real entry is researched."
```
