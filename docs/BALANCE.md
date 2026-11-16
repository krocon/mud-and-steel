# Balance

Prompt 045. How the balance of the three factions is measured, what was
measured, and which rule values were changed because of it. The unit
values are real (catalog, [SOURCES.md](SOURCES.md)) and never tuned; only
game rules in `data/*.toml` and scenario data are. Every value changed
here was **[provisional]** until the user confirmed it; the user confirmed
the changes 1–8 on 2026-11-16 and asked for two more (rate of fire in the
price, defeat without base), see "Decisions of the user (2026-11-16)".

## Tools

| Tool | What it measures |
|---|---|
| Result screen and `user://stats/*.json` | Every finished match: losses, kills and kill value per unit type, units produced, supply points taken and lost, account, income, points and living units every 30 s of game time (`sim::stats::MatchStats`). |
| `just balance` (`cli balance`) | AI tournaments over faction pairings × maps × modes with the same statistics, summed: wins per pairing, win rate per faction, kills per loss per unit type, the economy per faction at the end of base matches. Every pairing is played in both orders (each faction half the games as player 0). |
| `just duel` (`cli duel`) | One unit against another on flat, empty ground, both standing and firing at will, over distances and angles (where A stands relative to B's hull). Shows what the catalog values do after the scale and combat rules, without AI decisions. |
| `just ai-tournament` | One pairing of difficulties (prompt 024); now with the statistics too. |

```bash
just balance --modes battle --games 10                              # every map, de-su, de-us, su-us, hard vs hard
just balance --modes base --maps test_small,twin_meadow,skirmish     # base building
just balance --pairs de-de,su-su,us-us --modes battle                 # mirror matches (A / B wins)
just duel --a de_pz4_h --b su_t34_76_1943 --distances 100,200,300 --angles 0,90,180
```

The statistics only observe the simulation (they are not part of its
state, the state hash, snapshots or save games); a loaded game counts from
the moment it was loaded. Kills come from the simulation's `Destroyed`
events (the unit that fired the last hit); deaths without such an event
(heavy strike, fire, mines) are losses without a killer.

**Noise:** 10 games per row (5 per order) give a win rate ±15 % or so;
single rows swing a lot from map to map. Read the per-faction sums
(100–200 games) for decisions, the rows for outliers.

## Findings before tuning

Measured on the code of prompt 051 plus the statistics (seeds from 1, Hard
against Hard, Apple M1 Max, release).

### 1. Base matches on 1–2 km maps stay undecided (ammunition bottleneck)

`skirmish` (1.6 km), 20 minutes: every game of every pairing ran out of
time (de–su 10 of 10; prompt 051 measured the same for all pairings).
One traced game (Hard against Normal, de–su): fuel 1367 and manpower 1045
piled up unused, ammunition stayed between 40 and 290 for the whole match.

### 2. The Soviet side in battle mode

Battle mode, 10 maps, 3 pairings, 10 games each (Hard vs Hard):

| Faction | games | won | win rate |
|---|---|---|---|
| de | 200 | 143 | **71 %** |
| su | 200 | 50 | **25 %** |
| us | 200 | 82 | 41 % |

Rows de–su: 70–100 % for Germany on 9 of 10 maps. Kills per loss of the
tanks over all games: Panzer IV **1.95**, M4A3 1.05, T-34 **0.71**.

The duels show why (20 runs per cell, both standing):

| Duel | 100 m | 200 m | 300 m |
|---|---|---|---|
| Panzer IV (A) vs T-34, A in front of the T-34 | A 100 % | A 85 % | A 100 % (T-34 out of range) |
| T-34 (A) vs Panzer IV, A beside it (90°) | B 100 % | B 90 % | B 100 % |
| T-34 (A) vs Panzer IV, A behind it (180°) | A 100 % | A 95 % | B 100 % |
| M4A3 (A) vs Panzer IV, front | 95 % undecided | B 95 % | B 100 % |
| M4A3 (A) vs T-34, front | B 45 % / A 25 % | B 55 % / A 45 % | out of range |

From the catalog: KwK 40 `range_effective_m` 1500 and 10 rounds/min,
F-34 1000 and 5 rounds/min, M3 75 mm 1000 and 10 rounds/min; after the
direct-fire factor 1/4 the Panzer IV reaches 375 m, the T-34 and the M4A3
250 m. A Panzer IV fires twice as fast as a T-34 and hits it 125 m before
the T-34 can answer; the T-34 wins only from behind.

Hard against the passive enemy (the target of prompt 051), as the Soviet
side against passive Germans, 10 games each: `twin_meadow` 0/10,
`river_land` 0/10, `town_small` 3/10, `test_small` 10/10 (as the German
side against passive Soviets: 10, 10, 9, 10 of 10).

**Conclusion:** this is a rule and scenario problem, not an AI problem.
Battle mode gives every faction the same number of tanks, but under the
game's rules a Panzer IV is worth far more than a T-34 (and somewhat more
than an M4A3). On small open maps no flank lies outside the Panzer IV's
reach, and a defender that never moves and outranges the attacker by 125 m
cannot be beaten by any approach the AI could take; driving into it is a
loss for any AI. The price formula agrees with neither: a T-34 costs 253
(fuel 123, ammunition 106, manpower 24), a Panzer IV 233, although the
T-34 is the weaker tank in the game.

### 3. Faction win rates in base matches

Base mode, 20 minutes, 10 games per row (Hard vs Hard):

| Map | de – su | de – us | su – us |
|---|---|---|---|
| test_small | 10 / 90 / 0 | 100 / 0 / 0 | 100 / 0 / 0 |
| twin_meadow | 20 / 70 / 10 | 50 / 20 / 30 | 100 / 0 / 0 |
| skirmish | 0 / 0 / 100 | (not measured: undecided in prompt 051) | (same) |

(X wins / Y wins / undecided, in percent.) Per faction on `test_small`:
de 55 %, **su 95 %**, **us 0 %**.

The statistics show two causes:

- **Start vehicles.** The skirmish starts every player with a "halftrack":
  Germany the Sd.Kfz. 251/1 (two MG 42), the Soviet Union the M3A1 scout
  car (M2HB .50 and M1919 .30), the United States the M3 halftrack (one
  M1919). Over 20 games on `test_small` the scout car destroyed 138 units
  (15 per loss), the 251/1 71, the M3 13. In a duel the scout car kills a
  rifle squad at 150 m in 10–15 s and at 250 m in 40–70 % of the runs; the
  rifles cannot answer beyond 125 m.
- **Manpower.** The US squads are larger (rifle squad and engineers 12
  men, 72 manpower against 54), and the AI keeps the price of its next
  building in reserve: US players sat at 10–60 manpower for most of the
  match with 500–900 fuel and ammunition unused, their vehicle works idle.
  In the traced game the US built engineers and flak, one M4A3 in 20
  games.

## Changes (confirmed 2026-11-16)

Each change is one rule or scenario value, with the measurement that led
to it. (Measured in steps; "E1", "E2", "E3" are the economy variants
tried.)

| # | File | Value | Before | After | Why |
|---|---|---|---|---|---|
| 1 | `data/economy.toml` `[[building]] base_hq` | income per min | 40 / 70 / 80 | 40 / **100** / **90** | Ammunition was the bottleneck of every economy (finding 1); manpower the second one for the US (finding 3). |
| 2 | `data/economy.toml` `base_depot` | income per min | 40 / 50 / 30 | **30** / **70** / **40** | Same; fuel piled up in every measured match, so it pays a little less. |
| 3 | `data/economy.toml` `[supply]` | income per min | 60 / 60 / 30 | **50** / **90** / **40** | Same shift; a point is still worth more than a depot (balancing principle 3). |
| 4 | `data/economy.toml` `[account]` | start | 600 / 600 / 400 | **500** / 600 / **500** | Barracks, depot and vehicle works cost 450 manpower; with 400 the first vehicle waited for manpower in every match. |
| 5 | `data/logistics.toml` `[resupply]` | cost_pct | 50 | **25** | A full refill of a Panzer IV cost 51 ammunition, half a new tank; resupply ate the ammunition income of armies in long fights. |
| 6 | `data/scenarios/skirmish.toml` | start vehicle | 251/1, M3A1 scout car, M3 | **Opel Blitz, ZiS-5, CCKW** (unarmed trucks) | Finding 3: the armament of the three "halftracks" differed by a factor of 3–10 in kills and decided matches on small maps. Trucks carry the infantry as before (prompt 034) and are the same for every faction. |
| 7 | `data/scenarios/demo.toml` `[faction_count_pct]` (new) | tanks per faction | 8 / 8 / 8 | de 8, **su 10** (125 %), **us 9** (112 %) | Finding 2: battle armies of equal strength instead of equal numbers. Measured: su 150 % made the Soviets win 70 % against Germany, 125 % about even. Applies when factions are chosen (the skirmish setup, LAN, tournaments); the plain demo (golden hash, tests) keeps 8 against 8. |
| 8 | `data/economy.toml` `[unit_cost]` (new) | range factor of the ammunition price | – | `range_ref_m = 250`, 50–150 % | Balancing principle 1 ("a stronger unit is never cheaper") was broken: a T-34 (250 m, 5 rounds/min) cost 253, a Panzer IV (375 m, 10 rounds/min) 233; the Panzerschreck squad (37 m) 152, the PTRD squad 26. Now Panzer IV 280, StuG III 269, Tiger 461, Pak 40 186, Panzerschreck squad 87, bazooka squad 52; T-34 and M4A3 unchanged (250 m); heavy flak and long guns cost more. Indirect weapons keep their price. |

Rejected after measuring:

- **E2: ammunition only** (changes 1–3 for ammunition, 5): the end
  accounts of `skirmish` became balanced (no resource piles up any more),
  but every game still ran out of time, and the US still lost every base
  match on `test_small`.
- **E3: more manpower** (headquarters 110, start 600): no clear gain over
  E1, the US still 5 % on `test_small`.
- **A range cap for small arms** (rifle-caliber and heavy machine guns at
  most 1000 m real): no effect in the duels (the scout car's kills come
  from its two MGs inside 250 m, where sight ends anyway); reverted.
- **Soviet tanks 150 %** in battle mode: Soviets 70 % against Germany.

## Results after tuning

Measured with all changes (Hard vs Hard, 10 games per row, seeds from 1).

### Battle mode (10 maps × 3 pairings)

| Faction | before | after |
|---|---|---|
| de | 71 % (143 of 200) | **48 %** (97 of 200) |
| su | 25 % (50 of 200) | **40 %** (80 of 200) |
| us | 41 % (82 of 200) | **48 %** (96 of 200) |

| Map | de – su before | after | de – us before | after | su – us before | after |
|---|---|---|---|---|---|---|
| desert_village | 70 / 20 / 10 | 10 / 80 / 10 | 50 / 30 / 20 | 20 / 40 / 40 | 40 / 60 / 0 | 40 / 50 / 10 |
| farms | 90 / 0 / 10 | 100 / 0 / 0 | 30 / 70 / 0 | 30 / 70 / 0 | 30 / 70 / 0 | 30 / 70 / 0 |
| mountain_pass | 80 / 10 / 10 | 0 / 70 / 30 | 30 / 60 / 10 | 30 / 40 / 30 | 30 / 50 / 20 | 60 / 30 / 10 |
| river_land | 70 / 10 / 20 | 50 / 40 / 10 | 30 / 50 / 20 | 10 / 60 / 30 | 60 / 10 / 30 | 20 / 70 / 10 |
| skirmish | 100 / 0 / 0 | 80 / 20 / 0 | 70 / 20 / 10 | 90 / 10 / 0 | 40 / 50 / 10 | 60 / 40 / 0 |
| test_large | 100 / 0 / 0 | 50 / 50 / 0 | 80 / 20 / 0 | 30 / 70 / 0 | 40 / 60 / 0 | 50 / 50 / 0 |
| test_small | 100 / 0 / 0 | 50 / 50 / 0 | 50 / 50 / 0 | 0 / 100 / 0 | 40 / 60 / 0 | 50 / 50 / 0 |
| town_small | 100 / 0 / 0 | 30 / 40 / 30 | 100 / 0 / 0 | 90 / 0 / 10 | 70 / 10 / 20 | 40 / 50 / 10 |
| twin_meadow | 100 / 0 / 0 | 100 / 0 / 0 | 90 / 10 / 0 | 90 / 10 / 0 | 10 / 90 / 0 | 30 / 70 / 0 |
| winter_village | 30 / 40 / 30 | 40 / 40 / 20 | 60 / 10 / 30 | 70 / 20 / 10 | 60 / 40 / 0 | 30 / 60 / 10 |

(X wins / Y wins / undecided in percent, 10 games each.) Tank kills per
loss after: Panzer IV 1.53 (before 1.95), M4A3 1.21 (1.05), T-34 0.80
(0.71).

Mirror matches (battle, A / B wins of two Hard AIs, the same faction on
both sides; measures start positions and noise): test_small de 60/40, su
40/60, us 50/50; skirmish 70/30, 40/60, 40/60; farms 50/40, 30/70, 60/40;
twin_meadow 50/50, 70/30, 30/70. No mirror row above 70 %, the mean 50 %.

**Exceptions.** The acceptance criterion (no faction above 60 % at the
same AI level) holds for the sums per faction. Single map rows still go
above 60 % in both directions (10 games, about ±15 % noise, and the maps
differ): Germany on the open `twin_meadow` (starts 170 m apart) and
`farms`, where the whole fight happens inside the Panzer IV's 375 m; the
United States on `test_small` and the Soviet Union on `desert_village` and
`mountain_pass`, where cover and short sight lines take the range
advantage away. A rule answer for the open maps would be the weapon range
itself (decision 3 of PLAYTEST.md).

**Hard against the passive enemy as the Soviet side** (finding 2, 10
games each, with 10 T-34, measured before change 8 of the prices): `twin_meadow` 0/10 (before 0/10), `river_land`
1/10 (0/10), `town_small` 5/10 (3/10), `test_small` 10/10 (10/10). The
target of 90 % is not reachable: a passive German line that outranges the
attacker by 125 m on open ground beats any attack, also with 25 % more
tanks. This is an exception by the rules, not a task for the AI.

### Base mode (20 minutes)

| Map | de – su before | after | de – us before | after | su – us before | after |
|---|---|---|---|---|---|---|
| test_small | 10 / 90 / 0 | 90 / 0 / 10 | 100 / 0 / 0 | 30 / 70 / 0 | 100 / 0 / 0 | 0 / 80 / 20 |
| twin_meadow | 20 / 70 / 10 | 0 / 50 / 50 | 50 / 20 / 30 | 60 / 10 / 30 | 100 / 0 / 0 | 80 / 0 / 20 |
| skirmish | 0 / 0 / 100 | 0 / 0 / 100 | – | 0 / 0 / 100 | – | 0 / 0 / 100 |

| Faction (test_small + twin_meadow, 40 games each) | before | after |
|---|---|---|
| de | 45 % | 45 % |
| su | **90 %** | 33 % |
| us | **5 %** | 40 % |

Over both maps no faction is above 60 % any more, but single maps still
swing hard: on `test_small` the Soviet Union wins none of 20 games, on
`twin_meadow` the United States one of 20. The statistics name the
causes, and they are AI decisions, not prices:

- **AT squads are useless in the open** (all factions, kills per loss
  0.00–0.23 over 650 squads); in the duels an AT squad loses to a tank
  from 50 m every time (the tank's MGs and HE kill the two men before the
  first rocket lands). The counter-production of the AI (prompt 051)
  answers enemy armor with AT squads, so the side that sees more armor
  wastes its manpower: on `test_small` the Soviets built 98 AT squads and
  74 MG teams but only 37 T-34. AT squads would need ambushes from cover
  (prompt 047 tactics) to be worth their price.
- **Economy:** ammunition is no longer the bottleneck. On `skirmish` the
  end accounts after 20 minutes are fuel 110–400, ammunition 150–930,
  manpower 500–1700, with income of 150 / 330 / 205 per minute: the armies
  are at the unit limit (80) and manpower piles up.

**`skirmish` stays undecided in 20 minutes** in every pairing, before and
after: both armies reach the unit limit, rebuild what they lose, and
neither AI pushes through to the other base (1.6 km apart). This is the
AI's end game and the victory condition (a player with any building is
not defeated), not the economy any more; see PLAYTEST.md decision 6
(implemented since, see "Decisions of the user").
With 30 minutes instead of 20 (de–su, 10 games, `--base-ticks 36000`)
every game is still open.

### Duels after the changes

The duels measure catalog values under the combat rules; no combat rule
was changed, so they are the same as before (see above).

### Difficulty matrix (`just ai-tournament`)

The default matrix (demo armies 8 against 8, `test_small`, 40 games, no
faction choice) after the changes: Hard–Normal 55/45, Hard–Easy 87/12,
Hard–Passive 100/0, Normal–Easy 55/45, Normal–Passive 100/0,
Easy–Passive 97/0 (one timeout); tournament hash `0xc83fb9773681200e`.

### Time

A full battle round (10 maps × 3 pairings × 10 games) takes about 9
minutes, a base round on `test_small`, `twin_meadow` and `skirmish` about
25 minutes (Apple M1 Max, release, all cores; several rounds ran at the
same time, so these are upper bounds).

## Decisions of the user (2026-11-16)

The user answered the six questions of [PLAYTEST.md](PLAYTEST.md):
changes 1–8 above are **confirmed** (economy values, equal strength in
battle mode, range factor of prices, unarmed start vehicle); weapon ranges
(decision 3) are handled by prompt 067. Two rules were added on the
user's request:

| # | File | Value | Before | After | Why |
|---|---|---|---|---|---|
| 9 | `data/economy.toml` `[unit_cost]` | rate factor of the ammunition price | – | `rate_ref_rpm = 10`, 75–150 % | Decision 4: rate of fire enters the price. A gun that fires twice as fast deals about twice the damage in the same time (reload follows the catalog rate). The reference is the common 75 mm tank gun (KwK 40, M3: 10 rounds/min). |
| 10 | `data/economy.toml` `[defeat]` (new) | defeat without base | a player with any unit left plays on | `without_base = true`, `base_buildings = ["base_hq", "base_barracks", "base_factory"]` | Decision 6: a player of a base match without completed headquarters and production is defeated (`DefeatReason::BaseLost`). |

### Prices with the rate factor

Ammunition price (total in brackets), `just economy-table`:

| Unit | rate (rounds/min) | before (range factor only) | after |
|---|---|---|---|
| Panzer IV H | 10 | 150 (280) | 150 (280) |
| M4A3 (75) | 10 | 103 (259) | 103 (259) |
| T-34/76 (1943) | 5 | 106 (253) | **82 (229)** |
| StuG III G | 8 | 150 (269) | 122 (241) |
| Tiger I | 6 | 203 (461) | **154 (412)** |
| IS-2 | 2 | 382 (590) | **287 (495)** |
| SU-85 | 6 | 190 (332) | 144 (286) |
| M10 | 8 | 155 (303) | 126 (274) |
| M26 | 8 | 212 (409) | 172 (369) |
| Pak 40 | 14 | 150 (186) | **206 (242)** |
| ZiS-3 | 15 | 106 (148) | 155 (197) |
| 57 mm M1 | 15 | 64 (106) | 91 (133) |
| Rifle squad de / su / us | 40 (rifles) | 14 / 14 / 15 (68 / 68 / 87) | 16 / 15 / 18 (70 / 69 / 90) |
| AT squad de / su / us | 4 / 8 / 4 | 75 / 12 / 40 (87 / 24 / 52) | 58 / 11 / 33 (70 / 23 / 45) |

Rates are those of the main gun (catalog). Machine guns and rifles reach
the upper bound of 150 %, but their caliber term is small (a rifle squad pays 1–3 more). The lower
bound is 75 %, not 50 %: with 50 % the T-34 cost 205 and the Tiger's
ammunition price fell below the Panzer IV's (balancing principle 1, "a
stronger unit is never cheaper", checked by `economy::tests`).

**Equal price ≈ equal strength.** The duels did not change (no combat
rule changed; tables above): the Panzer IV beats the T-34 from the front
at every distance and the M4A3 at 200 m and more; the T-34 wins only from
behind. Battle mode measured equal strength at 8 Panzer IV, 10 T-34, 9
M4A3 (change 7). What these armies cost:

| Army | before | after |
|---|---|---|
| 8 Panzer IV | 2240 | 2240 |
| 10 T-34 | 2530 (+13 %) | **2290 (+2 %)** |
| 9 M4A3 | 2331 (+4 %) | 2331 (+4 %) |

So equal strength now costs the same within 4 % for all three main tanks
(`crates/sim/tests/prices.rs` checks 5 %). Before, the Soviet player paid
13 % more for the same strength.

### Base mode with both rules (20 minutes)

`just balance --modes base --maps test_small,twin_meadow,skirmish --games
10` (Hard vs Hard, seeds from 1; before = after the changes 1–8):

| Map | de – su before | after | de – us before | after | su – us before | after |
|---|---|---|---|---|---|---|
| test_small | 90 / 0 / 10 | 100 / 0 / 0 | 30 / 70 / 0 | 30 / 60 / 10 | 0 / 80 / 20 | 10 / 80 / 10 |
| twin_meadow | 0 / 50 / 50 | 0 / 40 / 60 | 60 / 10 / 30 | 50 / 30 / 20 | 80 / 0 / 20 | 80 / 0 / 20 |
| skirmish | 0 / 0 / 100 | 0 / 0 / 100 | 0 / 0 / 100 | 0 / 0 / 100 | 0 / 0 / 100 | 10 / 0 / 90 |

| Faction (test_small + twin_meadow, 40 games each) | before | after |
|---|---|---|
| de | 45 % | 45 % |
| su | 33 % | 33 % |
| us | 40 % | 43 % |

Undecided games: 43 of 90 before, 41 of 90 after. The defeat rule ends the
matches that got to a base: 49 headquarters fell in these 90 games, and
every decided match now ends when the base falls instead of when the last
squad is hunted down (mean decided time 360–490 s on the small maps). It
does not decide `skirmish`: the AI armies (1.6 km apart) still do not
reach the other base in 20 minutes; one of 30 games got there (after 19
minutes). With 30 minutes (`--base-ticks 36000`, de–su, 10 games) every
game is still open, as before the rule.

`just ai-tournament --mode base --a hard --b normal --games 20
--max-ticks 24000` (`test_small`): with the rule 11 / 9 wins, no
timeout, mean 453 s; the same with `without_base = false` 10 / 7 wins
and 3 timeouts.

The per-faction sums did not move: the rate factor makes T-34 and Tiger
cheaper, but the Soviet base AI spends most of its manpower on AT squads
and MG teams (689 AT squads, kills per loss 0.06), see "Open".

## Typical combat distances (prompt 067)

Decision 3 of [PLAYTEST.md](PLAYTEST.md) (user, 2026-11-16): weapon ranges
are **typical combat distances**. The catalog got a second range per
weapon, `range_typical_m` (definition and class table in
[`data/catalog/SCHEMA.md`](../data/catalog/SCHEMA.md), scale in
[SCALE.md](SCALE.md#typical-combat-distances-prompt-067)); direct fire uses
only this value × 1/4. This is a catalog and scale change, not a tuned rule:
the values are class values with sources, the same for every faction.

| Weapon | game range before | after |
|---|---|---|
| Panzer IV (KwK 40) | 375 m | 175 m |
| T-34 (F-34), M4A3 (75 mm M3) | 250 m | 175 m |
| Tiger (KwK 36) | 500 m | 250 m |
| Pak 40 / ZiS-3 / 57 mm M1 | 450 / 250 / 250 m | 175 m |
| Rifles (Kar98k, Mosin / M1) | 125 / 114 m | 75 m |
| MG 42, DP-27 on the bipod / BAR | 200 / 137 m | 100 m |
| Tripod machine guns (Lafette 42 / Maxim / M1917A1) | 750 / 250 / 457 m | 200 m |
| 8.8 cm Flak 36 against ground targets | 3715 m | 250 m |

All measurements: Hard against Hard, 10 games per row, seeds from 1;
"before" is `main` before prompt 067 (the state after the decisions of
2026-11-16), measured again on the same day.

### The scale factor

Typical distances are about half the old values, so the direct-fire factor
was checked again. Battle mode, 10 maps × 3 pairings (600 games each; these
two runs without the new velocity factor of the prices, which the AI's
unit values use):

| Direct-fire factor | game ranges (rifle / 75 mm / 88 mm) | de | su | us |
|---|---|---|---|---|
| 1/4 (kept) | 75 / 175 / 250 m | 51 % | 45 % | 47 % |
| 1/3 | 100 / 233 / 333 m | 50 % | 39 % | 51 % |

1/4 stays (SCALE.md).

### Battle mode (10 maps × 3 pairings)

| Faction | before | after |
|---|---|---|
| de | 48 % (97 of 200) | **49 %** (98 of 200) |
| su | 41 % (83 of 200) | **48 %** (96 of 200) |
| us | 47 % (94 of 200) | **46 %** (93 of 200) |

Sum of the distances from 50 %: 14 points before, 7 after. Undecided
games: 26 of 300 before, 13 after. Tank kills per loss: Panzer IV 1.55 →
1.40, M4A3 1.20 → 1.10, T-34 0.80 → **1.02**. The battle armies stay at
8 Panzer IV, 10 T-34, 9 M4A3 (`[faction_count_pct]`): the Panzer IV still
penetrates more and fires twice as fast as the T-34.

| Map | de – su before | after | de – us before | after | su – us before | after |
|---|---|---|---|---|---|---|
| desert_village | 10 / 80 / 10 | 30 / 70 / 0 | 20 / 40 / 40 | 30 / 70 / 0 | 40 / 50 / 10 | 70 / 30 / 0 |
| farms | 100 / 0 / 0 | 40 / 50 / 10 | 30 / 70 / 0 | 60 / 40 / 0 | 30 / 70 / 0 | 50 / 50 / 0 |
| mountain_pass | 0 / 70 / 30 | 60 / 40 / 0 | 30 / 40 / 30 | 60 / 30 / 10 | 70 / 20 / 10 | 60 / 30 / 10 |
| river_land | 50 / 40 / 10 | 80 / 20 / 0 | 10 / 60 / 30 | 60 / 40 / 0 | 20 / 80 / 0 | 40 / 50 / 10 |
| skirmish | 80 / 20 / 0 | 20 / 80 / 0 | 90 / 10 / 0 | 80 / 20 / 0 | 60 / 40 / 0 | 80 / 20 / 0 |
| test_large | 50 / 50 / 0 | 40 / 60 / 0 | 30 / 70 / 0 | 20 / 80 / 0 | 50 / 50 / 0 | 60 / 40 / 0 |
| test_small | 50 / 50 / 0 | 80 / 20 / 0 | 0 / 100 / 0 | 20 / 80 / 0 | 60 / 40 / 0 | 50 / 50 / 0 |
| town_small | 30 / 40 / 30 | 10 / 70 / 20 | 90 / 0 / 10 | 20 / 40 / 40 | 40 / 50 / 10 | 30 / 60 / 10 |
| twin_meadow | 100 / 0 / 0 | 100 / 0 / 0 | 90 / 10 / 0 | 60 / 40 / 0 | 30 / 70 / 0 | 40 / 60 / 0 |
| winter_village | 40 / 40 / 20 | 50 / 40 / 10 | 70 / 20 / 10 | 60 / 40 / 0 | 40 / 50 / 10 | 30 / 60 / 10 |

(X wins / Y wins / undecided in percent.) Rows at 100 % for one side: three
before (farms and twin_meadow de – su, test_small de – us), one after.
**Exception:** `twin_meadow` de – su stays 100 / 0. The armies start 170 m
apart on open ground, inside the range of every tank from the first tick;
the fight is the frontal duel below, which the Panzer IV wins at every
distance. Single rows still swing by ±15 % and more (10 games).

### Duels

20 runs per cell, both standing, A's wins / B's wins / undecided in
percent; "–" = out of range for both.

| Duel | 100 m | 150 m | 200 m | 300 m |
|---|---|---|---|---|
| Panzer IV (A) vs T-34, A in front, before | 100 / 0 / 0 | 100 / 0 / 0 | 85 / 0 / 15 | 100 / 0 / 0 (T-34 out of range) |
| after | 100 / 0 / 0 | 100 / 0 / 0 | – | – |
| T-34 (A) vs Panzer IV, A behind it, before | 100 / 0 / 0 | 100 / 0 / 0 | 95 / 5 / 0 | 0 / 100 / 0 (T-34 out of range) |
| after | 100 / 0 / 0 | 100 / 0 / 0 | – | – |
| M4A3 (A) vs Panzer IV, front, before | 0 / 5 / 95 | 0 / 100 / 0 | 5 / 95 / 0 | 0 / 100 / 0 (M4A3 out of range) |
| after | 0 / 5 / 95 | 0 / 100 / 0 | – | – |
| M4A3 (A) vs T-34, front, before | 25 / 45 / 30 | 35 / 40 / 25 | 45 / 55 / 0 | – |
| after | 25 / 45 / 30 | 35 / 40 / 25 | – | – |
| German (A) vs Soviet rifle squad, before | 75 / 25 / 0 | 100 / 0 / 0 | 90 / 10 / 0 | – |
| after | 95 / 5 / 0 | – | – | – |
| German (A) vs US rifle squad, before | 10 / 90 / 0 | 100 / 0 / 0 | 100 / 0 / 0 | – |
| after | 100 / 0 / 0 | – | – | – |

Inside the common range nothing changed for the tanks (hit chance and
penetration work on the real distance, which is the same at the same game
distance). What is gone is the band in which only one side could fire:
125 m for the Panzer IV against T-34 and M4A3. The Panzer IV still wins
the frontal duel; the T-34 and the M4A3 win from the side and from behind,
now at every distance they can fire from.

Infantry: the German squad now beats the US squad at 100 m (before the US
squad won there): the MG 42 and the BAR reach 100 m, the rifles 75 m, so
at 100 m one MG 42 fights one BAR. At 50 m the German squad beats the
Soviet one (before the reverse): the PPSh-41 reaches 25 m instead of 50 m.
Fights at 75–100 m are 300–400 m in reality, where nearly every aimed
round hits: a squad in the open is wiped out in a few seconds, and who
fires first wins.

### Prices

`range_ref_m` follows the 75 mm class (250 m → 175 m). With typical
distances the Panzer IV would have lost its range surcharge and cost what
a T-34 costs (233 against 229) while it still beats it from the front;
`crates/sim/tests/prices.rs` (equal strength, 8 Panzer IV = 10 T-34 = 9
M4A3, within 5 % of the same price) failed. New rule, **[provisional]**:

| # | File | Value | Before | After | Why |
|---|---|---|---|---|---|
| 11 | `data/economy.toml` `[unit_cost]` | range reference | `range_ref_m = 250` | `175` | The reference is the game range of the 75 mm class guns, which keep 100 %. |
| 12 | `data/economy.toml` `[unit_cost]` (new) | velocity factor of the ammunition price | – | `velocity_ref_mps = 650`, 100–150 %, cubic | Caliber² × velocity³ is the square of the combat rules' muzzle penetration. Before, the longer catalog ranges of the high-velocity guns did this by accident. Lower bound 100 %: shaped charges and howitzers do not get cheaper. |

| Unit | before (total) | after (total) |
|---|---|---|
| Panzer IV H | 150 (280) | 150 (280) |
| M4A3 (75) | 103 (259) | 103 (259) |
| T-34/76 (1943) | 82 (229) | 85 (232) |
| StuG III G / M10 / Pak 40 | 122 / 126 / 206 | unchanged |
| ZiS-3 | 155 (197) | 161 (203) |
| 57 mm M1 | 91 (133) | **131 (173)** |
| Tiger I | 154 (412) | **215 (473)** |
| IS-2 | 287 (495) | **404 (612)** |
| SU-85 | 144 (286) | **201 (343)** |
| M26 | 172 (369) | **238 (435)** |
| 8.8 cm Flak 36 / 85 mm 52-K / 90 mm M2 | 300 / 226 / 313 | **421 / 317 / 440** |
| 2 cm Flak 38 / 37 mm 61-K / 40 mm Bofors | 25 / 61 / 70 | 18 / 46 / 52 |
| Rifle squad de / su / us | 16 / 15 / 18 | 19 / 16 / 23 |
| AT squad de / su / us | 58 / 11 / 33 | 58 / 12 / 33 |

Armies of equal strength: 8 Panzer IV 2240, 10 T-34 2320 (+3.6 %), 9 M4A3
2331 (+4.1 %); the price test holds unchanged. **Deviation:** the guns of
the 88 mm class cost 15–24 % more than before. They reach 250 m against
175 m (143 %) and are fast (150 %); before, range alone was capped at
150 % for them and for the Panzer IV alike, so their longer reach over the
Panzer IV was free. Not measured separately; if heavy tanks become too
rare, `range_max_pct` is the lever.

### Base mode (20 minutes, test_small and twin_meadow)

| Map | de – su before | after | de – us before | after | su – us before | after |
|---|---|---|---|---|---|---|
| test_small | 100 / 0 / 0 | 70 / 10 / 20 | 30 / 60 / 10 | 80 / 0 / 20 | 10 / 80 / 10 | 40 / 50 / 10 |
| twin_meadow | 0 / 40 / 60 | 20 / 20 / 60 | 50 / 30 / 20 | 70 / 0 / 30 | 80 / 0 / 20 | 60 / 10 / 30 |

| Faction (40 games each) | before | after |
|---|---|---|
| de | 45 % (18) | **60 %** (24) |
| su | 32 % (13) | 32 % (13) |
| us | 42 % (17) | **15 %** (6) |

**Base mode got worse for the United States** (the acceptance criterion of
prompt 067 is about battle mode; this is a deviation to decide on).
Germany against the United States went from 8 : 9 wins to 15 : 0 in 20
games. The statistics name three causes:

- **Rifle squads.** Kills per loss of the US rifle squad fell from 1.44 to
  0.51 (German 1.31 → 1.28, Soviet 0.36 → 0.61). Before, eleven M1 rifles
  at 40 rounds/min out-shot every other squad between 100 and 114 m; now
  all rifles reach 75 m and the squads meet at 100 m, where the German
  squad has an MG 42 (1200 rounds/min) and the US squad a BAR (375). The
  squad's armament is historical; the US answer in the game is the
  machine-gun team (tripod M1919A4, 200 m: kills per loss 0.32 → 1.67).
- **Longer matches, more heavy tanks.** Fights start at half the distance,
  so armies need longer to decide a match (mean decided time 360–490 s →
  390–790 s). More matches reach the heavy tanks: 30 Tigers instead of 17,
  with 117 kills for 4 losses; the Tiger reaches 250 m against the 175 m
  of every US gun except the M26's.
- **AT squads.** The US AI answered the armor with 304 bazooka squads
  (before 106; 1 kill, 245 lost): the counter-production of prompt 051
  wastes manpower, as noted under "Open" before; with more enemy armor
  alive it wastes more.

40 games per faction give about ±15 %, so the German 60 % and the Soviet
32 % are within the noise of before; the US 15 % is not.

### Tests and AI

- AI distances are shares of the weapon and sight ranges (AI.md), so
  `crates/ai` needed no change. Easy against Passive as the Soviet side
  takes up to 6740 ticks instead of under 6000 (its squads must close in to
  75 m under the passive tanks' fire); it still wins every match.
- The demo battle (8 Panzer IV against 8 T-34) is decided at tick 1834
  instead of 1013, same winner: the rifle squads start 176 m apart, out of
  each other's range.
- Sight ranges (`data/vision.toml`) are unchanged; every direct-fire weapon
  now reaches less far than its unit sees.

## Golden hash

Prompt 067 changed the golden hash to `0x3ad2_633d_388d_e4f8` (economy
golden hash `0x57e7_5da9_31e6_20dd`): weapon ranges come from
`range_typical_m`, so the demo battle runs differently, and the hashed
price rules changed (range reference, velocity factor). Before:

The golden hash of `crates/sim/tests/determinism.rs` changed to
`0x0b95_b29d_5ff1_d003` with the decisions of 2026-11-16 (the rate factor
and `[defeat]` are hashed economy rules, the hashed prices of the match's
types changed, the players who have had a base are in the economy hash;
the demo battle is unchanged; the economy golden hash is
`0xe007_c7b9_cb19_a7b3`). Before, prompt 045 changed it to
`0x164a_535e_3316_b34e`: the economy rules (income, start account, the
range factor of prices) and the logistics rules (resupply cost) are part
of the state hash and of the demo's accounts. The demo keeps 8 against 8
tanks (no faction is chosen there, so `[faction_count_pct]` does not
apply).

## Open, for the user

The decisions of [PLAYTEST.md](PLAYTEST.md) were answered on 2026-11-16.
From prompt 067 (typical combat distances, see above):

1. **Velocity factor of the prices** (change 12, provisional) and the
   higher prices of the 88 mm class guns (Tiger 473 instead of 412, IS-2
   612 instead of 495): confirm, or cap the range factor at 100 %
   (`range_max_pct`), which gives the heavy guns their old prices.
2. **United States in base mode** (15 % of 40 games, before 42 %): the US
   rifle squad loses to the German and Soviet squads at 75–100 m. Rule
   levers, none decided: the price of the US squad (12 men = 72 manpower),
   the AI's choice of machine-gun teams instead of rifle and AT squads,
   or a second BAR per squad as many units carried in 1944–45 (a catalog
   decision that needs a source).
3. **Class estimates** of `range_typical_m` (machine guns, submachine
   guns, anti-tank rockets, light guns; Soviet and US guns by analogy with
   the German 75 mm and 88 mm data): listed in `uncertain`; better sources
   are welcome (SCHEMA.md).

## Open

- **AI:** AT squads walk into the open and are wasted (kills per loss
  below 0.25 for every faction); the counter-production should only buy
  them with cover or ambush tactics. Lost engineers are rebuilt again and
  again. The end game of base matches on maps of 1.5 km and more (see
  `skirmish`): with the defeat rule a single push to the enemy
  headquarters and works would decide them, but the AI armies do not
  get there.
- **Per-map swings** in base mode (Soviet Union 0 % on `test_small`,
  United States 5 % on `twin_meadow`) need more games per row and a look
  at single replays; the JSON statistics of the tournaments are not
  written to disk yet (only the summed tables).
- More measurements: base mode on the other seven maps, Easy/Normal
  against Hard per faction, team matches.
