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
| Ground vehicle speed | real × **1/1** (`[speed] vehicle`, instead of the time factor) | prompt 097, 2026-11-20 |
| Direct-fire range | real **typical combat distance** (`range_typical_m`) × **1/8**, at least **15 m** | typical distances decided 2026-11-16 (prompt 067); factor 1/8 since prompt 097 (2026-11-20; 1/4 before) |
| Sight range | real × **1/8** | prompt 097 (1/4 before) |
| Anti-aircraft range | real × **1/4** (`range.anti_aircraft`) | prompt 097 (was the direct-fire factor) |
| Indirect-fire range | real × **1/8** | confirmed 2026-11-15 |
| Aircraft speed | real × **1/2** (instead of the time factor) | confirmed 2026-11-15 |
| Fuel range | real road range × **1/20** (prompt 036) | provisional |

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

### Vehicle speed and range after a reference video (prompt 097)

The user found that tanks drove too fast and fired too far. The reference
was a short video of another WW2 real-time strategy game; it served **only
for the feel** (speed and distances relative to the vehicles' size and to
the screen). No values, names or looks were taken from it, and the catalog
did not change: only the conversion factors did. Measured in the video (in
own vehicle lengths per second, L/s, which needs no camera angle) and in
the game:

| | Video | Game before | Factor | Game now |
|---|---|---|---|---|
| Tank off road | 0.4–0.65 L/s | Panzer IV 1.5, Sherman 2.2, T-34 3.4 | 2.3–5 × | 0.75, 1.1, 1.7 |
| Truck off road | 1.0–1.1 L/s | ZiS-5 2.3 | 2 × | 1.15 |
| Tank duel | 6–11 tank lengths seen fighting | 75 mm reach 30 lengths (175 m) | – | 15 lengths (87.5 m) |

(The game values come from the catalog speeds and the factors, which is
what the simulation drives on flat grass; the video gave no road speeds,
turn rates or maximum ranges, its camera is much closer than the game's.)

Decisions:

- **Ground vehicles drive their real speed**: `[speed] vehicle = 1/1`
  replaces the time factor for movement only. A `[[time.override]]` would
  also have halved reloads, set-up and other durations of the category.
  Infantry and guns pushed by their crews keep the time factor; a Panzer IV
  off road (4.4 m/s) still outruns a walking squad (2.2 m/s). Slower than
  real would be hard to justify and would make 4 km maps drag.
- **Direct fire and sight 1/8.** 1/6 was measured too: battle tournaments
  (10 maps × 3 pairings × 10 games, Hard against Hard, both with vehicles at
  1/1) gave de / su / us 51 / 40 / 54 % at 1/6 and 51 / 46 / 49 % at 1/8
  (before the change 55 / 53 / 37 %); 1/8 is the better balanced and the
  closer to the reference. Sight follows with the same factor, so every
  unit still sees farther than it shoots (sight 250 m, longest gun 125 m)
  and recon keeps its lead (350 m).
- **Minimum direct-fire range 15 m** (`direct_fire_min_m`): pure 1/8 would
  leave a Panzerfaust 7.5 m (less than the 12 m at which infantry notices a
  tank coming, `combat.toml [overrun]`) and a pistol 3 m. Penetration and
  dispersion still use the real distance (game distance × 8).
- **Anti-aircraft fire keeps 1/4** (`range.anti_aircraft`): aircraft speeds
  and altitudes do not change.
- **Indirect fire unchanged** (1/8): artillery still covers most of a map.
- **`test_small` starts 60 m apart** (were 120 m), with the demo's groups:
  the demo's tanks start inside the 87.5 m of their guns as before inside
  175 m.
- The AI needed retuning, see [AI.md](AI.md).

| Weapon class | Typical (m, real) | Game at 1/4 (m) | Game now (m) | Tank lengths now |
|---|---|---|---|---|
| Pistols / .45 submachine guns / other submachine guns | 25 / 50 / 100 | 6 / 12 / 25 | 15 / 15 / 15 (minimum) | 2.5 |
| Rifles | 300 | 75 | 37.5 | 6 |
| Sniper rifles | 400 | 100 | 50 | 8 |
| Bipod and vehicle machine guns | 400 | 100 | 50 | 8 |
| Tripod machine guns | 800 | 200 | 100 | 17 |
| Panzerfaust 60 / anti-tank rockets / anti-tank rifle | 60 / 100 / 150 | 15 / 25 / 37 | 15 / 15 / 19 | 2.5–3 |
| 57–76 mm tank and anti-tank guns | 700 | 175 | 87.5 | 15 |
| 85–122 mm high-velocity guns | 1000 | 250 | 125 | 21 |

Consequences: battles last longer (mean decided battle 175 s before, 232 s
now in the tournament); infantry rides trucks that are slower too. If
advancing on 4 km maps becomes too slow, possible remedies are a road bonus
or a faster game speed setting; neither is built in.

To try other values: change `vehicle` under `[speed]` and `direct_fire` /
`sight` under `[range]` in `data/scale.toml` (fractions of whole numbers),
`just run`; `range_ref_m` in `data/economy.toml` should follow the 75 mm
range (700 m × factor).

### Range compression

With 1:1 sizes, real combat ranges are far too long for the screen: a PaK 40 hit
targets at 1.5 km, a field howitzer at over 10 km. Ranges are therefore compressed,
by different factors per kind:

- **Direct fire × 1/8 of the typical combat distance since prompt 097** (1/4
  before; the following describes 1/4, see "Typical combat distances" below
  and the prompt 097 section above). Rifles (typical 300 m) reach 75 m, 0.4 screens at medium
  zoom; the 75 mm class tank and anti-tank guns (700 m) 175 m, about one screen;
  88 mm class guns (1000 m) 250 m. A duel fits on one screen, and range differences
  between weapon classes are kept in proportion.
- **Sight × 1/4** (× 1/8 since prompt 097), the same as direct fire, so a unit can see what it can shoot. Real
  ~2 km in open terrain becomes ~500 m; hills, forests and buildings reduce it (prompt
  018). The catalog has no sight values: `data/vision.toml` sets them as rules in real
  meters per category (2 km; recon and snipers 2.8 km → 700 m; trucks 1.6 km → 400 m).
- **Indirect fire × 1/8.** Artillery should cover a large part of the map but not all
  of it: the leFH 18 (real 10.7 km) reaches ~1.3 km, mortars (real 2–6 km) 250–750 m.
  Stronger compression than direct fire, because otherwise every gun reaches every
  point of a 2–4 km map.

Formula: `game_range = real_range × num / den` (`Scale::direct_fire_range_m`,
`sight_range_m`, `indirect_fire_range_m`).

### Typical combat distances (prompt 067)

Decision of the user (2026-11-16): the game computes direct fire with **typical
combat distances**, not with maximum ranges. Until then the catalog had one value,
`range_effective_m`, and its sources meant different things by it: 1500 m for the
KwK 40 of the Panzer IV, 1000 m for the F-34 of the T-34 and the 75 mm M3 of the
M4A3, 3000 m for an MG 42 on its tripod, 14 860 m (the maximum horizontal range) for
the 8.8 cm Flak 36. In the game the Panzer IV outranged the T-34 by 125 m for no
historical reason.

- The catalog has two values per weapon now: `range_effective_m` (the maximum or
  sight value, kept for reference) and `range_typical_m`, the usual distance at
  which the weapon was used with effect against its main target. Definition, rules
  and the class table: [`data/catalog/SCHEMA.md`](../data/catalog/SCHEMA.md).
- **Direct fire** uses only `range_typical_m × 1/4` (`sim::combat::WeaponType::range_m`).
  Weapons of one class have one value in every faction; what differs between guns
  (penetration, velocity, rate of fire) stays in their own values.
- **Indirect fire** (mortars, howitzers, self-propelled guns) is unchanged: these
  weapons have no typical distance, their `range_effective_m` is a maximum range by
  definition and is compressed × 1/8.
- **Anti-aircraft fire** is unchanged: `range_aa_m` × 1/4 with the caps of
  `data/air.toml`. Against ground targets an anti-aircraft gun uses its
  `range_typical_m` like any gun (8.8 cm Flak 36: 250 m instead of 3715 m).
- **Sight** is unchanged (`data/vision.toml`: 2 km real → 500 m, recon and snipers
  700 m). Every direct-fire weapon now reaches less far than its unit sees (the
  longest: 250 m); before, tripod machine guns and anti-aircraft guns reached beyond
  sight.

**The factor was re-checked** (2026-11-16) with battle tournaments (10 maps × 3
pairings × 10 games, Hard against Hard, [BALANCE.md](BALANCE.md)): with 1/4 the
factions win 51 / 45 / 47 %, with 1/3 (which would keep the game ranges about as
long as before: rifles 100 m, 75 mm guns 233 m) 50 / 39 / 51 %. 1/4 stays: it is
the better balanced of the two, it keeps one factor for sight, direct fire and
anti-aircraft fire, and the shorter ranges fit a fight on one screen.

| Class | Typical (m, real) | Game (m) | Before (m) |
|---|---|---|---|
| Pistols / .45 submachine guns / other submachine guns | 25 / 50 / 100 | 6 / 12 / 25 | 12 / 12 / 25–50 |
| Rifles | 300 | 75 | 114–125 |
| M1 carbine | 200 | 50 | 75 |
| Sniper rifles | 400 | 100 | 150–212 |
| Bipod and vehicle machine guns, BAR | 400 | 100 | 137–342 |
| Tripod machine guns, .50 on a vehicle | 800 | 200 | 250–750 |
| Panzerfaust 60 / anti-tank rockets / anti-tank rifle | 60 / 100 / 150 | 15 / 25 / 37 | 15 / 27–37 / 75 |
| 20 mm cannon / light guns 37–45 mm | 400 / 500 | 100 / 125 | 150 / 200–250 (flak in the ground role up to 1790) |
| 57–76 mm tank and anti-tank guns | 700 | 175 | 250–450 |
| 85–122 mm high-velocity guns | 1000 | 250 | 375–500 (heavy flak up to 4760) |
| 105 mm howitzer of the M4A3 (105) in direct fire | 500 | 125 | 2790 |

- **Fuel × 1/20** (prompt 036, provisional). A full tank lasts the real road range
  times 1/20: Panzer IV 10.5 km, Tiger 9.75 km, Opel Blitz 16 km. Real ranges
  (200–400 km) would never run out on maps of 2–4 km; at 1/20 an army that crosses
  the map two or three times has to refuel (`Scale::fuel_range_m`).

### Aircraft

Aircraft do not use the time factor. Doubled real speeds (Fw 190: ~1 300 km/h) would
cross a 4 km map in about 10 s. Instead:

- **Speed × 1/2** of the real value: a fighter at ~650 km/h real flies ~90 m/s and
  crosses a 4 km map in ~45 s, enough to see and react to it.
- Aircraft are not placed on the map. They **enter from the map edge**, fly their
  mission (strafing run, bombing, reconnaissance pass) and leave again.
- They do not take part in ground pathfinding.
- Sizes stay 1:1 (B-25J: 20.6 m wingspan).

Prompt 033 (air support, [ARCHITECTURE.md](ARCHITECTURE.md#air-support-and-anti-aircraft-fire))
uses the **cruise speed** (else the maximum) × 1/2: Fi 156 18 m/s, Ju 87 D-5
44 m/s, He 111 54 m/s, P-51D 81 m/s, Fw 190 A-8 (no cruise value) 91 m/s.

- **Altitude** is a game rule in game meters (`data/air.toml`, [provisional]):
  cruise 200–500 m, over the target 30 m (fighters strafing), 150 m (ground
  attack), 500 m (bombers). Real attack altitudes (1–4 km) would be far above
  every camera view and every compressed flak range.
- **Flak range** is the catalog's effective anti-aircraft range or ceiling
  (`range_aa_m`) × the direct-fire factor 1/4, capped per class: 2 cm Flak 38
  2200 m → 550 m, 37 mm 61-K 3000 m → 750 m, 40 mm Bofors 3500 m → 875 m,
  heavy flak (8.8 cm 8000 m, 85 mm 10.5 km, 90 mm 13.3 km) capped at 1500 m;
  AA machine guns 1000 m → 250 m. Heavy flak does not fire below 100 m.
- **Rate of fire** of AA weapons: as in combat (automatic weapons 25 % of the
  cyclic rate), durations ÷ time factor 2. Aircraft guns use their real cyclic
  rate (aircraft have no time factor): a P-47's eight .50s fire 53 rounds in
  a 0.5 s burst.
- **Bombs** fall `sqrt(2h/g)` real seconds (no time factor, like the aircraft):
  10 s from 500 m, 5.5 s from 150 m. Blast and near-miss radii are sizes
  (1:1): SC 250 → 31 m fragment radius.
- **Reconnaissance** sees 2000 m real × 1/4 = 500 m around its track.

## What this means on the map

Camera assumption: prompt 002 is in progress in parallel; its specification has a
perspective camera with a vertical field of view of ~30°, distance 15–600 m. At
16:9 the visible width at the focus point is about `0.95 × distance`. "Medium zoom"
here means a distance of **200 m ≈ 190 m visible width**; the overview (600 m) shows
about 570 m. Check these numbers once the camera is merged.

### Travel times

Values from the catalog: infantry at time factor 2, ground vehicles at their
real speed (`[speed] vehicle = 1/1` since prompt 097, twice as fast before),
aircraft × 1/2 of maximum speed. L/s: own hull lengths per second on road /
off road. Recomputed on 2026-11-20 (prompt 097); recompute after catalog
changes.

| Unit | Real road / off (km/h) | Game road / off (m/s) | L/s road / off | 1 km road / off (min:s) | 4 km road / off (min:s) |
|---|---|---|---|---|---|
| Rifle Squad (`us_inf_rifle_squad`) | 5 / 4 | 2.8 / 2.2 | – | 6:00 / 7:30 | 24:00 / 30:00 |
| Panzer IV Ausf. H (`de_pz4_h`) | 38 / 16 | 10.6 / 4.4 | 1.8 / 0.75 | 1:35 / 3:45 | 6:19 / 15:00 |
| T-34/76 Model 1943 (`su_t34_76_1943`) | 54 / 36 | 15.0 / 10.0 | 2.5 / 1.69 | 1:07 / 1:40 | 4:27 / 6:40 |
| Tiger I Ausf. E (`de_tiger1_e`) | 45.4 / 20 | 12.6 / 5.6 | 2.0 / 0.88 | 1:19 / 3:00 | 5:17 / 12:00 |
| M4A3 (75)W Sherman (`us_m4a3_75`) | 42 / 25 | 11.7 / 6.9 | 1.9 / 1.11 | 1:26 / 2:24 | 5:43 / 9:36 |
| Sd.Kfz. 251/1 Ausf. D halftrack (`de_sdkfz251_1`) | 52.5 / 21.2 | 14.6 / 5.9 | 2.5 / 1.02 | 1:09 / 2:50 | 4:34 / 11:19 |
| ZiS-5V (`su_zis5`) | 60 / 25 | 16.7 / 6.9 | 2.8 / 1.15 | 1:00 / 2:24 | 4:00 / 9:36 |
| M8 Greyhound (`us_m8_greyhound`) | 89 / 51 | 24.7 / 14.2 | 5.3 / 3.01 | 0:40 / 1:11 | 2:42 / 4:42 |
| Focke-Wulf Fw 190 A-8 (`de_fw190_a8`, aircraft, max) | 652 | 91 | – | 0:11 | 0:44 |
| B-25J Mitchell (`us_b25j`, aircraft, max) | 438 | 61 | – | 0:16 | 1:06 |

### Combat ranges

Game range = real range × factor; the real range of direct fire is the typical
combat distance (`range_typical_m`), of indirect fire the maximum range. Screen
width: 190 m at medium zoom, 570 m in the overview.

| Weapon | Real (m) | Factor | Game (m) | Medium-zoom screens | Overview screens |
|---|---|---|---|---|---|
| Kar98k (`de_inf_rifle_squad`) | 300 | 1/8 direct | 37.5 | 0.2 | 0.1 |
| MG 42 (`de_inf_mg42_team`) | 400 | 1/8 direct | 50 | 0.3 | 0.1 |
| MG 42 on Lafette 42 (`de_inf_hmg_team`) | 800 | 1/8 direct | 100 | 0.5 | 0.2 |
| 2.36 in Rocket Launcher M9 (Bazooka) (`us_inf_at`) | 100 | 1/8 direct, min 15 m | 15 | 0.1 | 0.0 |
| 7.5 cm PaK 40 L/46 (`de_pak40`) | 700 | 1/8 direct | 87.5 | 0.5 | 0.2 |
| 7.5 cm KwK 40 L/48 (`de_pz4_h`) | 700 | 1/8 direct | 87.5 | 0.5 | 0.2 |
| 76.2 mm F-34 (`su_t34_76_1943`), 75 mm M3 (`us_m4a3_75`) | 700 | 1/8 direct | 87.5 | 0.5 | 0.2 |
| 8.8 cm KwK 36 L/56 (`de_tiger1_e`) | 1000 | 1/8 direct | 125 | 0.7 | 0.2 |
| 60 mm Mortar M2 (mount M2) (`us_inf_mortar_light`) | 1815 | 1/8 indirect | 227 | 1.2 | 0.4 |
| 8 cm s.GrW 34 (`de_inf_mortar_medium`) | 2400 | 1/8 indirect | 300 | 1.6 | 0.5 |
| 10.5 cm leFH 18M L/28 (`de_lefh18`) | 12325 | 1/8 indirect | 1541 | 8.1 | 2.7 |
| 122 mm M-30 L/22.7 (`su_m30`) | 11800 | 1/8 indirect | 1475 | 7.8 | 2.6 |

Combat (prompt 017, [ARCHITECTURE.md](ARCHITECTURE.md#combat)) uses these game
ranges, the interval between rounds ÷ time factor (Panzer IV: 10 rounds per minute
real → one round every 3 s), projectile speeds × time factor (750 m/s → 1500 m/s)
and gravity × time factor². Penetration loss and dispersion work on real distances
(game distance × 8 since prompt 097, × 4 before), so a shot at 87.5 m in the game
is as likely to hit and penetrate as one at 700 m in reality. Combat rules (not catalog values) are in
`data/combat.toml`.

Indirect fire (prompt 032, [ARCHITECTURE.md](ARCHITECTURE.md#indirect-fire-and-smoke))
uses the game ranges of the table and computes the flight at the **real**
distance (game distance × 8): the charge is chosen so the shell arrives at a
preferred elevation (mortars 60°, guns 25°, `data/indirect.toml`), the real
flight time `2·v·sin θ / g` is divided by the time factor, and the apex is
scaled down by 1/8 like the distance. Examples: 8 cm s.GrW 34 at 300 m game
(2.4 km real) about 15 s, at 50 m game 6 s; 10.5 cm leFH 18M at 1540 m game
(12.3 km real) about 17 s. Dispersion uses the real distance too (3 mrad),
while blast and smoke radii are sizes and stay 1:1 (8 cm: 16 m fragments,
12 m smoke). Smoke lasts 90 s real = 45 s game and drifts with the wind ×
time factor.

Transport (prompt 034, [ARCHITECTURE.md](ARCHITECTURE.md#transport-and-towing)):
mounting, dismounting, limbering and unlimbering times in `data/transport.toml`
are real and divided by the time factor (truck 20 s real = 10 s game); a gun
pushed by its crew moves at the rules' real speed × time factor (Pak 2 km/h →
4 km/h). Seats are catalog values (`passengers`); towing compares the catalog's
engine power with the masses of tractor and gun (8 hp per tonne).

Observations for balancing:

- Infantry crossing 4 km takes 30 min of game time. That is slow but fits the
  role of infantry on a large map; trucks and halftracks move it. If it feels too
  slow, a `[[time.override]]` for infantry categories is the intended lever.
- Short-range infantry anti-tank weapons (bazooka 100 m real) ended up at 25 m.
  Since prompt 097 a minimum game range of 15 m applies (`direct_fire_min_m`).
- Artillery covers 1.5 km, a large part of a 2–4 km map, as intended.
- Light mortars: the 5 cm leGrW 36 (520 m real) reaches 65 m, farther than a
  rifle since prompt 097 (37.5 m; 75 m since prompt 067; 125 m before).
- Since prompt 097 sight (250 m) is much shorter than artillery reach (indirect
  fire stays 1/8): guns fire blind at points nobody sees more often, with the
  wider blind dispersion (`blind_pct`). An observer near the target helps.
- Formations, wrecks and buildings are full size while ranges were halved by
  prompt 097: wrecks block lines of fire more often, and a line of eight tanks
  (~85 m) is as wide as a tank gun reaches. The AI was retuned for this
  ([AI.md](AI.md)).
- With typical distances (prompt 067) fights happen at shorter real distances,
  where rounds hit more often (dispersion works on the real distance): a rifle
  squad in the open 75 m from an enemy squad (300 m real) is wiped out within a few
  seconds. Cover, stance and suppression matter more than before.

## Map scale

Maps use the same 1:1 meters: a height field with **2 m** cells and heights in
**millimeters**, up to 4 km × 4 km (`test_large`). Details in [MAPS.md](MAPS.md).

## Changing the scale

1. Change `data/scale.toml` (fractions only). After a change of
   `range.direct_fire`, set `range_ref_m` in `data/economy.toml` to the
   75 mm range (700 m × factor) and check the fixed distances of the AI
   ([AI.md](AI.md)) and of `test_small`'s starts.
2. `just check`. The golden hash changes if the demo unit's speed changes; that is
   an intentional rule change and must be named in the commit message.
3. Update the tables in this file.
