# Vehicle database schema

`de.json`, `su.json`, `us.json`: technical data of the armored vehicles from the
reference folder `docs/.vorlagen/fahrzeuge/` (not in the repository), one entry per
catalog id, sorted by id. `index.json` maps every reference folder to its id.

**Role.** The catalog (`data/catalog/*.toml`) stays the only input of the simulation.
This database is a reference work next to it: more fields, only vehicles of the
reference folder, never input of a game rule. If a rule needs a value from here, the
value moves into the catalog as its own field with a source. The unit page
(`web/units/`) and the game (`SimHost.vehicle_record`) only display it.

**Do not edit the faction files by hand.** `just vehicle-db` writes them from the
reference folder (`tools/vehicle_refs.py`; two runs give the same files) and prints a
report: entries per faction, warnings, values kept as text only. `just vehicle-db-check`
(part of `just check` and CI, needs no reference folder) checks the committed files:
schema (unknown fields are errors), ids against `index.json` and the catalog, and the
values both hold against each other.

## Rules for values

- **Integers, unit in the field name** as in the catalog: `_mm`, `_kg`, `_kmh_x10`
  (tenths of km/h), `_deg`, `_deg_x10`, `_cc`, `_l`, `_s`.
- `n.a.`, `-` and empty values of the source mean: the field is absent. Nothing is
  written as `0` for unknown; `turning_radius_mm = 0` means the vehicle turns on the spot.
- **`text`:** every section may hold `"text": { "<field>": "<original text>" }` for
  values the number alone does not express (ranges, "or", footnote marks, missing
  units). The number next to it is then the first clear value of the text, or absent
  if there is none. Nothing is guessed and nothing dropped.
- Ground pressure: the source gives psi; `ground_pressure_kpa_x10` is converted
  (1 psi = 6.894757 kPa).
- **Armor angles are measured from the horizontal** as in the source: `90` is a vertical
  plate, `0` a flat roof. The catalog's `armor_hull_front_deg` is measured from the
  vertical (`90 - deg`).

## Entry

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Catalog id (or the proposed id of a vehicle that is not in the catalog yet). |
| `name` | string | Name in the source. |
| `type` | string | Vehicle type as the source calls it (`Heavy Tank`). |
| `family` | string | Family in the reference folder (`Tiger`). |
| `source` | object | `name`, `url`, `fetched` (date), `references` (literature codes of the source). |
| `general` | object | See below. |
| `firepower` | object | See below. |
| `mobility` | object | See below. |
| `armor` | object | See below. |
| `notes` | list of strings | Notes of the source, if any. |

### `general`

`designation`, `other_designations`, `manufacturer`, `production_period`, `radio`
(strings); `production_quantity`, `crew`, `length_mm` (overall, gun forward),
`length_hull_mm` (only where the source gives both lengths; otherwise hull length is
`length_mm - barrel_overhang_mm`), `width_mm`, `width_skirts_mm`, `height_mm`,
`barrel_overhang_mm`, `mass_kg` (combat weight).

### `firepower`

`main_gun` (name as in the source), `main_gun_caliber_mm_x10`,
`main_gun_length_cal_x10` (L/56 = `560`), `main_ammo`, `traverse_deg` (total: `360`
for a turret, left plus right for a casemate), `traverse_drive` (`manual`, `power`,
...), `elevation_min_deg_x10`, `elevation_max_deg_x10`, `traverse_time_360_s`, `sight`,
`secondary` (text of the source), `secondary_ammo`.

### `mobility`

`engine`, `engine_type` (text, e.g. `V12, 21.4 liters`), `displacement_cc`, `power_hp`,
`power_rpm`, `power_to_weight_hp_per_t_x10`, `gearbox` (text), `gears_forward`,
`gears_reverse`, `fuel`, `fuel_capacity_l`, `consumption_road_l_per_100km`,
`consumption_offroad_l_per_100km`, `range_road_km`, `range_offroad_km`,
`speed_road_kmh_x10`, `speed_offroad_kmh_x10`, `speed_water_kmh_x10`, `track_width_mm`,
`track_ground_contact_mm`, `track_links` (per track), `ground_pressure_psi_x10`,
`ground_pressure_kpa_x10`, `ground_clearance_mm`, `turning_radius_mm`, `gradient_deg`,
`vertical_obstacle_mm`, `fording_mm`, `trench_crossing_mm`.

### `armor`

One object per assembly: `hull`, `hull_upper`, `hull_lower`, `superstructure`,
`turret`, `main_turret`, `upper_turret`, `lower_turret`, `gun_turrets`, `mg_turrets`
(multi-turret tanks), `mantlet`, `gun_shield`. Each has the faces `front`, `side`,
`rear`, `top_bottom`; a face is absent if the source has no value. A face holds

| Field | Meaning |
|---|---|
| `mm` | Thickness. Layered plates (`50+30mm`) are added up. |
| `mm_min`, `mm_max` | Instead of `mm` where the source gives a range (`100-110mm`). |
| `deg` | Angle from the horizontal, only where the source gives one angle. |
| `text` | Original text if it says more than the numbers (`100mm @Saukopf`, `open`). |

An assembly the source describes only in words is `{ "text": "..." }`.

## `index.json`

`"<NATION>/<family>/<variant>": { "id": "...", "status": "...", "reason": "...", "chassis": "..." }`
for every folder of the reference material. `status`:

| Status | Meaning |
|---|---|
| `existing` | The catalog had this id before prompt 063 (with a model). The check compares the values both hold. |
| `new` | Catalog entry added by prompt 063 (`tier = 3`, model built by its group session). Compared like `existing`. |
| `skipped` | Not taken over (prototype, small series, merged into a neighbor, no drawing and no relative with one); `reason` says why. No database entry. |

`chassis` (optional) names the id of the nearest relative (shared chassis or family);
a vehicle without a drawing is built from that relative's drawing. The family of every
folder belongs to one of the nine model groups of prompt 063 (`GROUPS` in
`tools/vehicle_refs.py`).

`docs/VEHICLE_REFS.md` is the same list as a table, generated by `just vehicle-db`,
followed by the hand-written section "Model checks" (kept when the table is written
again).

## Check against the catalog

For `existing` and `new` entries the check compares length, hull length, width, height, mass,
crew, road and off-road speed, road range, power, hull and turret armor (front, side,
rear; thickest plate, a casemate's superstructure counts as turret), front plate
angle, caliber and ammunition of the main gun. A difference beyond the tolerance (5 %
for dimensions, mass and power, 10–25 % for speeds, range, armor and ammunition, 10°
for the angle; see `comparisons` in `tools/vehicle_refs.py`) is an error unless the
catalog lists the field in `uncertain` (weapon values: `"weapon"`) and names the
difference in `note`. The catalog wins: its values have their own, higher-ranking
sources (`docs/SOURCES.md`).
