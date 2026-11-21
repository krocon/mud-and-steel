# Playtests

Prompt 045. How a playtest with the user runs, what to look at, and the form
for the feedback. The measured balance (AI tournaments, duels) is in
[BALANCE.md](BALANCE.md); the rules being tested are in
[GAME_DESIGN.md](GAME_DESIGN.md). Every rule there stays **[provisional]**
until the user confirms it after playing; this document collects what is
needed for that decision.

## Before the session

1. `just check` and `just selftest` are green on the build that is played.
2. `just run` starts the game; note the commit (`git log --oneline -1`).
3. The statistics of every finished match are written to
   `user://stats/<date-time>.json` (macOS:
   `~/Library/Application Support/Godot/app_userdata/Mud and Steel/stats/`),
   the replays to `user://replays/`. Keep both for every match of the
   session; the file names are in the log and on the result screen.
4. Pick the matches of the session from the list below (about 20–30
   minutes each, 2–3 per session).

## Matches

| # | Mode | Map | Player | Opponent | What it tests |
|---|---|---|---|---|---|
| 1 | Battle | test_small | Germany | Normal | First contact, controls, range of tanks |
| 2 | Battle | twin_meadow | Soviet Union | Hard | The weaker side in battle mode (finding 2 of prompt 045) |
| 3 | Base | test_small | any | Normal | Build order, resources, the first 10 minutes |
| 4 | Base | skirmish | Soviet Union | Hard | Supply points, ammunition, waves, end game |
| 5 | Base | farms or river_land | United States | Hard | Larger map, transport, artillery |
| 6 | Base, 2v2 | twin_meadow | any + Normal ally | 2 × Normal | Teams, shared vision |

Play every match to its end or to 30 minutes. If a match is still open
after 30 minutes, note why (who was stronger, what was missing).

## Checklist during the match

Tick what applies, note the minute of the match for anything odd.

**Economy**

- [ ] The first unit or building could be bought within the first minute.
- [ ] Waiting for a resource happened (which one, how long?).
- [ ] A resource piled up unused (which one, how much at the end?).
- [ ] Supply points were worth fighting for (taken / lost / ignored).
- [ ] Resupply (ammunition, fuel) was understood and affordable.
- [ ] Prices felt fair between the factions (e.g. AT squads, tanks).

**Combat**

- [ ] Tanks fought at believable distances; the range advantage of a
      faction was visible and understandable.
- [ ] Infantry was useful (cover, AT squads, MGs) and not just fodder.
- [ ] Artillery, mortars and air support were worth their price.
- [ ] Fights ended: no endless stand-off of two lines.
- [ ] Losses were readable (what killed my unit, why?).

**Opponent (AI)**

- [ ] The opponent attacked with sensible groups, not one by one.
- [ ] The opponent defended its base and its supply points.
- [ ] The difficulty felt right (Normal: winnable, Hard: hard).
- [ ] The opponent surrendered at a sensible moment (or did not when it
      should have).

**Pace and length**

- [ ] The match was decided within 20–30 minutes.
- [ ] Production and build times felt right (too slow / too fast).
- [ ] The time factor 2 (speeds, reloads) felt right.

**Result screen**

- [ ] Victory / defeat, reason and match time are correct.
- [ ] Losses, kills, supply points and the best unit types match what
      happened; the resource curves show the bottleneck.

## Feedback form

Copy this block per match into a note (or into the prompt that follows the
playtest) and fill it in. Scores 1 (bad) – 5 (good).

```text
Date / commit:
Match # / mode / map / my faction / opponent and difficulty:
Result / match time / statistics file:

Fun overall (1-5):
Pace (1 too slow - 3 right - 5 too fast):
Economy (1-5) - bottleneck resource, unused resource:
Faction balance (1-5) - stronger faction, why:
Unit that felt too strong:
Unit that felt too weak / useless:
Opponent (1-5) - too easy / right / too hard:
Clearest moment of frustration (minute, what happened):
Best moment:

Rules I confirm (GAME_DESIGN.md sections):
Rules I want changed (section, what, why):
```

## After the session

1. Put the statistics JSON files next to the notes and compare them with
   the tournament numbers in [BALANCE.md](BALANCE.md) (losses per type,
   end accounts, supply points). Since prompt 179 the result screen's tab
   "Report" names the costliest causes of the losses with a hint; ask the
   tester whether it matches what they felt went wrong.
2. For every rule the user confirms, a later prompt changes its
   **[provisional]** mark in GAME_DESIGN.md to **[confirmed]** with the
   date; only the user can confirm (prompt 045 does not).
3. For every requested change: change one number at a time in `data/*.toml`,
   run `cli balance` and `cli duel` for the affected units, and note the
   reason and the measurement in BALANCE.md and the commit (balancing
   principle 6 of GAME_DESIGN.md).

## Decisions of prompt 045

Collected by prompt 045 (details and measurements in
[BALANCE.md](BALANCE.md), decisions in
`prompts/done/045-balancing-playtest.md`). **The user answered all six on
2026-11-16:** 1, 2, 4, 5 and 6 are confirmed (4 and 6 with the additions
below, both implemented and measured in BALANCE.md); 3 is handled by
prompt 067 (typical combat ranges instead of maximum ranges). No decision
of this list is open any more.

1. **Economy ratios** **[confirmed 2026-11-16]** (GAME_DESIGN.md "Resources"): ammunition and
   manpower income raised, fuel lowered, start account 500/600/500,
   resupply 25 % of the price instead of 50 %, because ammunition was the
   bottleneck of every AI economy and manpower starved the US. Confirm
   after matches 3–5.
2. **Battle mode: equal numbers or equal strength** **[confirmed
   2026-11-16: equal strength]** (GAME_DESIGN.md "Game modes"): with 8 tanks each Germany won 71 % of the AI battles (Panzer
   IV 375 m and 10 rounds/min against T-34 250 m and 5 rounds/min, catalog
   values). Now 8 Panzer IV, 10 T-34, 9 M4A3 when factions are chosen
   (47 / 41 / 49 %). Alternative: equal numbers and accept the historical
   imbalance.
3. **Weapon ranges** **[decided 2026-11-16: typical combat ranges,
   prompt 067]** (SCALE.md): the catalog's `range_effective_m` mixes
   typical and maximum effective ranges; the Panzer IV's 375 m against
   250 m decides most tank fights and makes a passive German line
   unbeatable for Soviet tanks on small maps. A common rule (e.g. one range
   per gun class) would be a scale decision; prompt 067 changes the
   catalog to typical combat ranges. **Done (prompt 067):** every weapon
   has `range_typical_m`, one value per weapon class (75 mm class guns
   700 m real = 175 m in the game for all three factions, rifles 75 m);
   see SCALE.md "Typical combat distances" and BALANCE.md "Typical combat
   distances (prompt 067)". To check in a match: fights start at about
   half the distances of before, and infantry in the open dies faster.
4. **Unit prices** **[confirmed 2026-11-16, rate of fire added]**
   (GAME_DESIGN.md "Unit costs"): the ammunition price of
   direct-fire weapons now scales with their range (reference 250 m,
   50–150 %): Panzer IV 280 instead of 233, Panzerschreck squad 87 instead
   of 152. The user confirmed it and asked for the rate of fire in the
   price too: the same term now also scales with the catalog rate of fire
   (reference 10 rounds/min, 75–150 %): T-34 229 instead of 253, Tiger
   412 instead of 461.
5. **Start vehicle** **[confirmed 2026-11-16]** (GAME_DESIGN.md
   "Buildings"): an unarmed truck for
   every faction instead of the differently armed halftracks.
6. **Victory condition** **[confirmed 2026-11-16, implemented]**
   (GAME_DESIGN.md "Victory"): a player with any building or unit left
   was not defeated, which kept AI matches on large maps open. Now a
   player of a base match without completed headquarters, barracks and
   vehicle works is defeated ("base lost").
