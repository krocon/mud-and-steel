# Scale

How real-world values become game values. The catalog (`data/catalog/`) stores only
real, historical values. The conversion happens in exactly one place:
`data/scale.toml`, applied by `crates/sim/src/scale.rs`. Every factor is a fraction
of integers, so the simulation applies it exactly in fixed point.

## Decisions

| Quantity | Rule | Status |
|---|---|---|
| Sizes | **1:1** in real meters, infantry included (reference figure 1.80 m) | fixed |
| Time | **factor 2**: speeds × 2, durations ÷ 2; overridable per category | fixed |
| Direct-fire range | real × **1/4** | confirmed 2026-11-15 |
| Sight range | real × **1/4** | confirmed 2026-11-15 |
| Indirect-fire range | real × **1/8** | confirmed 2026-11-15 |
| Aircraft speed | real × **1/2** (instead of the time factor) | confirmed 2026-11-15 |

### Sizes 1:1

Models, collision footprints and distances on the map use real meters. Nothing is
enlarged for readability: a Panzer IV is 7 m long, a soldier 1.80 m tall. Readability
comes from selection markers, icons and outlines later. 1:1 keeps all models
consistent with each other and lets catalog dimensions feed directly into
blueprints (prompt 004).

### Time factor 2

Real speeds would make movement on a multi-kilometer map tediously slow, so game
time runs twice as fast as real time:

```
game_speed  = real_speed  × time_factor         (time_factor = 2/1)
game_time   = real_time   ÷ time_factor         (reload, build, setup, ...)
```

`speed_m_per_s = speed_kmh_x10 × num / (36 × den)`. Per-category overrides
(`[[time.override]]` in `scale.toml`) let balancing change single categories (for
example infantry) without touching the catalog.

Since prompt 010 the terrain type picks the speed: road speed on roads, cross-country
speed elsewhere, each scaled by a percentage and by the uphill grade
(`data/terrain.toml`, see [MAPS.md](MAPS.md)).

### Range compression

With 1:1 sizes, real combat ranges are far too long for the screen: a PaK 40 hit
targets at 1.5 km, a field howitzer at over 10 km. Ranges are therefore compressed,
by different factors per kind:

- **Direct fire × 1/4.** Rifles (real ~400 m) reach ~100 m, half a screen at medium
  zoom; anti-tank guns (real ~1 500 m) ~375 m, about two screens at medium zoom and
  still within the overview. Duels stay visible, and range differences between weapons
  are kept in proportion.
- **Sight × 1/4**, the same as direct fire, so a unit can see what it can shoot. Real
  ~2 km in open terrain becomes ~500 m; terrain and fog of war (prompt 018) reduce it.
- **Indirect fire × 1/8.** Artillery should cover a large part of the map but not all
  of it: the leFH 18 (real 10.7 km) reaches ~1.3 km, mortars (real 2–6 km) 250–750 m.
  Stronger compression than direct fire, because otherwise every gun reaches every
  point of a 2–4 km map.

Formula: `game_range = real_range × num / den` (`Scale::direct_fire_range_m`,
`sight_range_m`, `indirect_fire_range_m`).

### Aircraft

Aircraft do not use the time factor. Doubled real speeds (Fw 190: ~1 300 km/h) would
cross a 4 km map in about 10 s. Instead:

- **Speed × 1/2** of the real value: a fighter at ~650 km/h real flies ~90 m/s and
  crosses a 4 km map in ~45 s, enough to see and react to it.
- Aircraft are not placed on the map. They **enter from the map edge**, fly their
  mission (strafing run, bombing, reconnaissance pass) and leave again.
- They do not take part in ground pathfinding; flight altitude is visual only until a
  later prompt needs it.
- Sizes stay 1:1 (B-25J: 20.6 m wingspan).

## What this means on the map

Camera assumption: prompt 002 is in progress in parallel; its specification has a
perspective camera with a vertical field of view of ~30°, distance 15–600 m. At
16:9 the visible width at the focus point is about `0.95 × distance`. "Medium zoom"
here means a distance of **200 m ≈ 190 m visible width**; the overview (600 m) shows
about 570 m. Check these numbers once the camera is merged.

### Travel times

Values from the catalog at time factor 2 (aircraft: × 1/2 of maximum speed).
Computed from the catalog on 2026-11-15; recompute after catalog changes.

| Unit | Real road / off (km/h) | Game road / off (m/s) | 1 km road / off (min:s) | 4 km road / off (min:s) |
|---|---|---|---|---|
| Rifle Squad (`us_inf_rifle_squad`) | 5 / 4 | 2.8 / 2.2 | 6:00 / 7:30 | 24:00 / 30:00 |
| Panzer IV Ausf. H (`de_pz4_h`) | 38 / 16 | 21.1 / 8.9 | 0:47 / 1:52 | 3:09 / 7:30 |
| T-34/76 Model 1943 (`su_t34_76_1943`) | 54 / 36 | 30.0 / 20.0 | 0:33 / 0:50 | 2:13 / 3:20 |
| Tiger I Ausf. E (`de_tiger1_e`) | 45.4 / 20 | 25.2 / 11.1 | 0:40 / 1:30 | 2:39 / 6:00 |
| M4A3 (75)W Sherman (`us_m4a3_75`) | 42 / 25 | 23.3 / 13.9 | 0:43 / 1:12 | 2:51 / 4:48 |
| Sd.Kfz. 251/1 Ausf. D halftrack (`de_sdkfz251_1`) | 52.5 / 21.2 | 29.2 / 11.8 | 0:34 / 1:25 | 2:17 / 5:40 |
| ZiS-5V (`su_zis5`) | 60 / 25 | 33.3 / 13.9 | 0:30 / 1:12 | 2:00 / 4:48 |
| M8 Greyhound (`us_m8_greyhound`) | 89 / 51 | 49.4 / 28.3 | 0:20 / 0:35 | 1:21 / 2:21 |
| Focke-Wulf Fw 190 A-8 (`de_fw190_a8`, aircraft, max) | 652 | 91 | 0:11 | 0:44 |
| B-25J Mitchell (`us_b25j`, aircraft, max) | 438 | 61 | 0:16 | 1:06 |

### Combat ranges

Game range = real range × factor. Screen width: 190 m at medium zoom, 570 m in the overview.

| Weapon | Real (m) | Factor | Game (m) | Medium-zoom screens | Overview screens |
|---|---|---|---|---|---|
| Kar98k (`de_inf_rifle_squad`) | 500 | 1/4 direct | 125 | 0.7 | 0.2 |
| MG 42 (`de_inf_mg42_team`) | 800 | 1/4 direct | 200 | 1.1 | 0.4 |
| 2.36 in Rocket Launcher M9 (Bazooka) (`us_inf_at`) | 110 | 1/4 direct | 28 | 0.1 | 0.0 |
| 7.5 cm PaK 40 L/46 (`de_pak40`) | 1800 | 1/4 direct | 450 | 2.4 | 0.8 |
| 7.5 cm KwK 40 L/48 (`de_pz4_h`) | 1500 | 1/4 direct | 375 | 2.0 | 0.7 |
| 8.8 cm KwK 36 L/56 (`de_tiger1_e`) | 2000 | 1/4 direct | 500 | 2.6 | 0.9 |
| 60 mm Mortar M2 (mount M2) (`us_inf_mortar_light`) | 1815 | 1/8 indirect | 227 | 1.2 | 0.4 |
| 8 cm s.GrW 34 (`de_inf_mortar_medium`) | 2400 | 1/8 indirect | 300 | 1.6 | 0.5 |
| 10.5 cm leFH 18M L/28 (`de_lefh18`) | 12325 | 1/8 indirect | 1541 | 8.1 | 2.7 |
| 122 mm M-30 L/22.7 (`su_m30`) | 11800 | 1/8 indirect | 1475 | 7.8 | 2.6 |

Observations for balancing (to be decided with combat, prompt 017):

- Infantry crossing 4 km takes 30 min of game time. That is slow but fits the
  role of infantry on a large map; trucks and halftracks move it. If it feels too
  slow, a `[[time.override]]` for infantry categories is the intended lever.
- Short-range infantry anti-tank weapons (bazooka 110 m real) end up at ~30 m. They
  may need a minimum game range rather than pure compression.
- Artillery covers 1.5 km, a large part of a 2–4 km map, as intended.

## Map scale

Maps use the same 1:1 meters: a height field with **2 m** cells and heights in
**millimeters**, up to 4 km × 4 km (`test_large`). Details in [MAPS.md](MAPS.md).

## Changing the scale

1. Change `data/scale.toml` (fractions only).
2. `just check`. The golden hash changes if the demo unit's speed changes; that is
   an intentional rule change and must be named in the commit message.
3. Update the tables in this file.
