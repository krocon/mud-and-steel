# Balance

Prompt 045. How the balance of the three factions is measured, what was
measured, and which rule values were changed because of it. The unit
values are real (catalog, [SOURCES.md](SOURCES.md)) and never tuned; only
game rules in `data/*.toml` and scenario data are. Every value changed
here is **[provisional]** until the user confirms it after playtests
([PLAYTEST.md](PLAYTEST.md)); nothing in [GAME_DESIGN.md](GAME_DESIGN.md)
is marked confirmed by this prompt.

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

## Changes [provisional]

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
not defeated), not the economy any more; see PLAYTEST.md decision 6.
SKIRMISH_LONG

### Duels after the changes

The duels measure catalog values under the combat rules; no combat rule
was changed, so they are the same as before (see above).

### Time

A full battle round (10 maps × 3 pairings × 10 games) takes about 9
minutes, a base round on `test_small`, `twin_meadow` and `skirmish` about
25 minutes (Apple M1 Max, release, all cores; several rounds ran at the
same time, so these are upper bounds).

## Golden hash

The golden hash of `crates/sim/tests/determinism.rs` changed to
`0x164a_535e_3316_b34e`: the economy rules (income, start account, the
range factor of prices) and the logistics rules (resupply cost) are part
of the state hash and of the demo's accounts. The demo keeps 8 against 8
tanks (no faction is chosen there, so `[faction_count_pct]` does not
apply).

## Open, for the user

See [PLAYTEST.md](PLAYTEST.md) "Decisions waiting for the user". In short:
whether battle mode should give equal numbers (historical tanks, unequal
strength) or equal strength (more Soviet and US tanks, as now); whether
weapon ranges should come from the catalog's effective ranges as they are
(the Panzer IV's 375 m against 250 m decides most tank fights); whether
rate of fire and range should enter the unit price; whether base matches
should end when headquarters and production are lost; the start vehicle.
