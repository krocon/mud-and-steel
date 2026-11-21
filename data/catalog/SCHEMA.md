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
| `tier` | int | `1` = core unit (prompt 006), `2` = the rest, `3` = taken over from the vehicle references (prompt 063): in the catalog, the gallery and on the unit page, not buildable; the unit reference in the game lists it once it has a model. |
| `model` | string | Path of the exported model, e.g. `assets/models/de/de_pz4_h.glb`. Empty until it exists. |
| `length_mm` | int | Overall length, including the gun pointing forward. |
| `length_hull_mm` | int | Length without the gun (vehicles with a gun). |
| `width_mm`, `height_mm` | int | Overall width and height (with tracks, skirts as fitted; height of towed guns in travel or firing position, say which in `note`). |
| `wingspan_mm` | int | Aircraft. |
| `mass_kg` | int | Combat weight (vehicles, aircraft: loaded/normal takeoff weight; guns: in firing position). |
| `crew` | int | Vehicles, guns, aircraft. For guns (prompt 177) the men serving the gun: small arms and fragments hit them apart from the gun. |
| `gun_shield` | bool | Guns only (prompt 177): a shield covers the crew from the front (`[gun_crew] shield_half_deg` in `data/combat.toml`), not from the side and not against fragments from above. Absent: no shield documented in the sources. |
| `passengers` | int | Troop seats besides the crew (trucks, halftracks; prompt 034; transport aircraft, prompt 055). Vehicles and aircraft only, 1–30; required for `aircraft_transport`. |
| `squad_size` | int | Infantry: soldiers in the team. |
| `bridge_length_mm` | int | `bridgelayer` only (required): length of the carried bridge, 4–30 m (prompt 055). |
| `bridge_load_kg` | int | `bridgelayer` only (required): load limit of the carried bridge (Class 30 = 30 long tons = 30480 kg; prompt 055). |
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
| `range_effective_m` | int | Direct fire: the real maximum effective range or sight value (what a manual or data sheet calls "effective range"); kept for reference, **not used by the simulation**. Indirect fire (the first weapon of mortars, howitzers, self-propelled guns): the maximum range, which the simulation uses. |
| `range_typical_m` | int | Real typical combat distance (prompt 067), see below. Required for every weapon with `range_effective_m` except the indirect weapon; at most `range_effective_m`. The simulation's direct fire uses only this value. |
| `ammo_carried` | int | Rounds carried. |
| `range_aa_m` | int | Anti-aircraft guns: real effective range against aircraft (slant range or effective ceiling) in meters (prompt 033). |
| `aa` | bool | The weapon sits on an anti-aircraft mount (MGs with "AA" in name or note; prompt 033). Default `false`. |
| `bomb_kg` | int | Aircraft: nominal mass of one bomb in kilograms (SC 250 = 250, 500 lb = 227; prompt 033). `count` is the number of bombs. |

### Typical combat distance (`range_typical_m`)

Decision of the user (2026-11-16, prompt 067): ranges in the game are **typical
combat distances**, not maximum ranges.

**Definition.** The usual distance, in real meters, at which the weapon was used
with effect against its main target in ground combat: the distance within which the
bulk of its effective fire or of its hits took place (from combat analyses), not the
farthest distance at which it could hit, penetrate or was sighted. The main target is
infantry for small arms and machine guns, armor for tank, anti-tank and anti-aircraft
guns in the ground role and for anti-tank weapons.

**Rules.**

- Weapons of the same class get the same value, whichever faction built them: the
  distance of a fight depends on terrain, sights and tactics far more than on the
  single gun, and sources document classes ("75 mm guns"), not models. What differs
  between guns of a class (penetration, velocity, rate of fire) is in their own fields.
- A value documented for the weapon or its class by a combat analysis is cited in
  `source` (`covers` starts with `range_typical_m:`). A value taken over from another
  class or estimated is a **class estimate**: the entry lists `weapon` in `uncertain`
  and its `note` gives the reasoning.
- **Indirect weapons** (first weapon of `inf_mortar_*`, `mortar_heavy`,
  `gun_howitzer`, `self_propelled`) have no typical distance: they are fired at a map
  position, and their `range_effective_m` is a maximum range by definition
  (compressed by the indirect factor of `data/scale.toml`).
- **Anti-aircraft fire** keeps `range_aa_m` (the effective slant range or ceiling);
  the `range_typical_m` of an anti-aircraft gun is its distance against ground
  targets.
- **Flamethrowers**: the jet range, equal to `range_effective_m`.
- Never values from other games.

| Class | Weapons | Typical (m) | Basis |
|---|---|---|---|
| Rifles | Kar98k, Mosin-Nagant, M1 | 300 | ORO-T-160 (Hitchman 1952): 80 % of the effective rifle and LMG fire of the European actions within 200 yd, 90 % within 300 yd |
| Sniper rifles | Kar98k ZF 39, Mosin PU, M1903A4 | 400 | Truppendienst 1967 (German snipers): 65–80 % of the hits below 400 m; class estimate for su, us |
| Bipod and flexible-mount MGs, squad automatic rifle | MG 42, DP-27, BAR M1918A2, DT, M1919A4 on vehicles, ring mounts | 400 | class estimate between ORO-T-160 (300 yd) and TM-E 30-451 (600–800 yd effective as light MG) |
| Tripod MGs, .50 MG on a vehicle | MG 42 on Lafette 42, Maxim, M1917A1, M1919A4 on tripod, M2HB | 800 | class estimate: twice the bipod value; TM-E 30-451 and FM 7-15 give 2,000 yd and more as the limit |
| Submachine guns | MP 40, PPSh-41 / Thompson, M3 | 100 / 50 | class estimate: battle sight setting; .45 ACP weapons their effective range |
| Carbine / pistols | M1 carbine / P 38, TT-33, M1911A1 | 200 / 25 | class estimates |
| Anti-tank rockets / Panzerfaust 60 / anti-tank rifle | Panzerschreck, Bazooka / Panzerfaust 60 / PTRD-41 | 100 / 60 / 150 | class estimates |
| 20 mm cannon / light guns 37–45 mm | KwK 38, Flak 38 / M6, 20-K, 61-K, Bofors | 400 / 500 | class estimates below the 75 mm class |
| Tank and anti-tank guns 57–76 mm | KwK 40, StuK 40, PaK 39, PaK 40, F-34, ZiS-3, 75 mm M3, 3 in M7, 57 mm M1 | 700 | NII-48 (1942–44): German 75 mm guns fire mostly from 100 to 700 m; ORO-T-117: Allied tanks knocked out by gunfire at just under 800 yd on average; class estimate for su, us |
| High-velocity guns 85–122 mm | 8.8 cm KwK 36 and Flak 36, D-5S, 52-K, D-25T, 90 mm M3 and M2 | 1000 | NII-48: German 88 mm guns fire mostly from 600 to 1300 m (at T-34 tanks 400–1100 m); class estimate for su, us |
| Howitzer in direct fire | 105 mm M4 of the M4A3 (105) | 500 | class estimate |

## Categories

| Group | Categories | Required fields |
|---|---|---|
| Infantry (`weapon` optional for `inf_medic`) | `inf_rifle`, `inf_lmg`, `inf_hmg`, `inf_mortar_light`, `inf_mortar_medium`, `inf_recon`, `inf_sniper`, `inf_medic`, `inf_engineer`, `inf_at`, `inf_flamethrower`, `inf_command` | `squad_size`, road and offroad speed, `weapon` |
| Vehicles | `tank_light`, `tank_medium`, `tank_heavy`, `tank_destroyer`, `assault_gun`, `halftrack`, `truck`, `recon_vehicle`, `self_propelled`, `bridgelayer` | length, width, height, mass, crew, road/offroad speed, range; front hull armor (not trucks); `weapon` for combat vehicles; `bridge_length_mm` and `bridge_load_kg` for `bridgelayer` |
| Guns | `gun_at`, `gun_aa_light`, `gun_aa_heavy`, `gun_howitzer`, `mortar_heavy` | length, width, height, mass, crew, `weapon` |
| Fortifications | `fort_bunker`, `fort_trench`, `fort_sandbag`, `fort_obstacle`, `fort_wire`, `fort_minefield_at`, `fort_minefield_ap` | length, width, height (0 allowed for minefields) |
| Aircraft | `aircraft_fighter`, `aircraft_attack`, `aircraft_bomber`, `aircraft_recon`, `aircraft_transport` | length, wingspan, height, mass, crew, max speed, ceiling, `weapon` (optional for `aircraft_recon` and `aircraft_transport`); `passengers` for `aircraft_transport` |
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
    { name = "7.5 cm KwK 40 L/48", caliber_mm_x10 = 750, ammo = "PzGr 39 APCBC, Sprgr 34 HE", muzzle_velocity_mps = 790, range_effective_m = 1500, range_typical_m = 700, ammo_carried = 87 },
    { name = "MG 34", count = 2, caliber_mm_x10 = 79, rate_rpm = 900, ammo_carried = 3150 },
]
source = [
    { ref = "Jentz/Doyle, Panzer Tracts 4-3 (2001)", covers = "dimensions, mass, armor, speeds" },
]
uncertain = ["speed_offroad_kmh_x10"]
note = "Values illustrate the format; the real entry is researched."
```
