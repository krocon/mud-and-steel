# Performance

Prompt 016. One scenario, two measurements and fixed budgets for large
battles on the 4 km map. Optimizations never change behavior: the golden
hash and the end hash of `cli bench` stay the same.

## Scenario

`data/scenarios/bench_large.toml` on `test_large` (4 × 4 km, 98 000 map
objects): two armies of 125 tanks (Panzer IV H / T-34) and 125 rifle squads
each, 504 units with 2250 soldiers, about 1.3 km apart in the open fields
west of the center; each player also has two trucks at its start position
(the scenario format moves a player's first group there). Both sides are
led by the Hard AI: they gather, drive towards each other, find paths
around woods, see and lose each other, fight, leave wrecks and crush
hedges and bushes.

## Recipes

| Command | Measures |
|---|---|
| `just bench` (`cli bench`) | Simulation: two Hard AIs play the scenario through a lockstep session (as `ai-tournament`); per tick `Sim::step` and each of its phases, the AIs' think, the local player's snapshot. Prints mean / median / p99 / max, the five slowest ticks by phase, ten time windows and the end hash. `--runs 3` takes the minimum per tick over three identical runs; `--ticks`, `--seed`, `--ai`, `--scenario` |
| `just bench-view` | Frame rate in Godot with the same scenario and a Hard AI as enemy: camera 40 / 150 / 600 m over the own army standing, then the army advances and the same distances in the battle. FPS, frame time (mean, p99, worst), the simulation's part of the frame (`SimHost.process_usec`), render CPU, draw calls, primitives and the script time of the big presentation systems. `--quality=medium`, `--perf-quick` (standing only), `--perf-column` (only 30 tanks driving in a column, prompt 104), `--perf-kills` (40 vehicles destroyed within 2 s, prompt 105). Screenshots in `user://perf-bench/` |
| `just path-bench`, `vision-bench`, `infantry-bench` | Single systems of the simulation (see ARCHITECTURE.md, INFANTRY.md) |
| `just fire-bench` | Fire on the town map (prompt 041): 40 units per side, 6000 ticks without fire and with a wood, a terrace and the dumps burning; mean and worst tick (ARCHITECTURE.md, "Fire") |
| `just terrain-bench`, `objects-bench`, `effects-bench`, `infantry-view-bench` | Single systems of the presentation |

`MEADOW_AB=1 just terrain-bench` (prompt 057) measures every terrain view once
per variant of the meadow look (each part off in turn, and the look before)
in one run; see ARCHITECTURE.md, "Meadow".

**Phases** come from `Sim::step_observed(commands, done)`: the simulation
calls `done(Phase)` after each phase of the tick; the caller reads the
clock. The simulation itself never reads a clock (DETERMINISM.md).

**Busy machines:** the simulation is deterministic, so every run computes
exactly the same ticks. The minimum per tick over several runs removes most
of the time other processes take; `cli bench` fails if the runs end with
different hashes. Frame rates cannot be cleaned that way: run
`bench-view` on a quiet machine and compare views of the same run.

## Budgets

Apple M1 Max, the scenario above, release simulation, debug bridge (as
`just run`):

| What | Budget | Why |
|---|---|---|
| Sim tick, mean | ≤ 5 ms | 10 % of the 50 ms tick; at 60 FPS a tick runs in every third frame. |
| Sim tick, p99 | ≤ 10 ms | A tick must not lengthen its frame noticeably. |
| Sim tick, max | ≤ 20 ms | No hitch from single events (wrecks, destruction). |
| AI think per tick (both), mean | ≤ 0.5 ms | Runs in the same frame in single player. |
| Snapshot per tick | ≤ 0.2 ms | |
| Frame rate, quality medium, every view | ≥ 60 FPS | |
| Frame rate, quality high, standing 150 m | ≥ 60 FPS | |
| Frame rate, quality high, battle | ≥ 45 FPS, aim 60 | High has MSAA and full SSAO; 2250 animated soldiers. |
| Ragdolls at once (prompt 048) | ≤ 8, only within 60 m of the camera | 11 physics bodies each; the rest fall with clips. Helmets lying around ≤ 12. [provisional] |

## Simulation

Prompt 186 (surfaces per building, relief in `env.gdshader`; `just
objects-bench city_large --blocks-bench --quality=high`, 2026-11-21, warm
shader cache): 35 m 12.51 → 12.47 ms, 110 m 14.52 → 14.49 ms, 350 m
15.59 → 15.87 ms per frame, draw calls unchanged. The first run after the
shader change had one 784 ms frame at 110 m: the new pipeline compiled
when first seen. Cost in the shader: one derivative bump per pixel of every
env material, one extra noise per clay roof pixel; the spall noise only on
buildings with `spall`.

Prompt 165 (hidden units as silhouettes, `unit_xray.gdshader`; `just
bench-view`, 2026-11-21, same machine, the same run with the setting off as
comparison): the stencil writes in the unit shaders cost nothing (draw calls
as before); the silhouette pass adds up to 1230 draw calls at 600 m (render
CPU +0.0–0.2 ms), the script ("unit markers") 0.32–0.41 → 0.45–0.71 ms per
frame; frame times moved by +0.2–0.6 ms, within the run-to-run noise of
about ±0.5 ms. Checking every unit every frame cost 0.6 ms: now once per
tick or on a change; squads beyond 300 m from the camera get none.

Prompt 164 (own-unit markers, `unit_highlight.gd`): `just bench-view`
reports them as "unit markers": 0.36–0.49 ms per frame with 1125 soldiers
(after warm-up). The markers are one MultiMesh, rebuilt only on a new tick
or when the hover changes (every frame: 1.2 ms).

Prompt 098 (fog of war as a translucent layer, `fog_of_war.gd`: two
screen-space quads, B-spline mask, screen-texture desaturation; mask fade in
`SimHost.vision_mask_fade`): `just bench-view --perf-quick`, 2026-11-20,
standing at 40 / 150 / 600 m: 70.3 / 74.3 / 81.0 → 68.2 / 72.5 / 78.9 FPS,
frame time +0.35 ms (about 2–3 %; the screen texture is already copied for
the water).

Prompt 097 (sight and direct fire 1/8, vehicles at real speed;
`just bench --runs 3`, 2026-11-20, same machine): mean sim tick 5.13 → 2.87 ms,
p99 9.24 → 6.43 ms, max 11.1 → 8.6 ms. Vision (half the sight range, a
quarter of the cells) 2.29 → 0.61 ms mean, combat 0.23 → 0.15 ms; paths
1.67 → 1.28 ms.

`just bench --runs 3` (6000 ticks = 5 min game time, seed 1, minimum per
tick of 3 runs, release, Apple M1 Max, 2026-11-15). "Before" and "after"
are prompt 016 alone (end hash `0x21c325f3290ca7fd` both); "with 021/023"
is the state after merging terrain deformation and base building (new
rules, end hash `0x6603281bb9fb609c`).

| ms per tick | Before mean | p99 | max | After mean | p99 | max | With 021/023 mean | p99 | max |
|---|---|---|---|---|---|---|---|---|---|
| **Sim tick** | 5.21 | 31.7 | 65.4 | **3.96** | **7.2** | **7.9** | **4.66** | **7.7** | **8.5** |
| · commands | 0.00 | 0.01 | 0.03 | 0.00 | 0.01 | 0.03 | 0.00 | 0.01 | 0.04 |
| · paths (A\*, budget) | 2.23 | 4.65 | 6.5 | 2.20 | 4.53 | 5.3 | 2.46 | 4.29 | 5.0 |
| · combat (targets, aim, fire) | 0.21 | 0.56 | 1.0 | 0.18 | 0.55 | 1.1 | 0.17 | 0.57 | 1.1 |
| · movement (separation) | 0.46 | 0.86 | 0.93 | 0.26 | 0.42 | 0.47 | 0.26 | 0.39 | 0.43 |
| · squads | 0.18 | 0.23 | 0.27 | 0.18 | 0.23 | 0.28 | 0.18 | 0.22 | 0.27 |
| · projectiles (and crushing) | 0.63 | 24.6 | 58.5 | 0.03 | 0.31 | 0.69 | 0.03 | 0.31 | 0.79 |
| · destruction (and craters) | 0.09 | 0.00 | 27.1 | 0.00 | 0.00 | 0.66 | 0.00 | 0.02 | 1.0 |
| · economy | – | – | – | – | – | – | 0.00 | 0.00 | 0.00 |
| · vision (budget) | 1.41 | 2.86 | 3.0 | 1.10 | 2.19 | 2.3 | 1.56 | 2.73 | 2.9 |
| AI think (both) | 0.09 | 0.30 | 0.47 | 0.06 | 0.26 | 0.46 | 0.06 | 0.26 | 0.45 |
| Snapshot | 0.04 | 0.05 | 0.05 | 0.02 | 0.03 | 0.03 | 0.02 | 0.03 | 0.03 |

Vision costs more with craters: every crater changes the ground of vision
cells and queues the viewsheds that cover them. `just crater-bench` with
16 stacked craters per tick (the case that closes path cells): 22 / 24 ms
per tick with the full relabeling, 6.0 / 6.7 ms with the local one, same
state hash.

Salvos into a wood (prompt 085, `just salvo-bench <map>`: the densest 64 m
square of trees, 200 moving units, release, Apple M1 Max). Big blasts fell
and strip the trees around them; felled trees rebuild path and vision cells,
bare ones vision cells only, overlapping areas joined per tick:

| Map | Salvo | Felled / bare | Ticks before | Worst tick |
|---|---|---|---|---|
| `test_large` | 16 × 13.2 cm, 2 per tick | 24 / 73 | max 5.6 ms | 11.4 ms |
| `test_large` | 18 × 15 cm, 6 per tick | 69 / 70 | max 5.4 ms | 14.3 ms (70 trees in the tick) |
| `city_large` | 16 × 13.2 cm, 2 per tick | 17 / 83 | max 9.3 ms | 10.2 ms |
| `city_large` | 18 × 15 cm, 6 per tick | 74 / 70 | max 8.8 ms | 11.9 ms (70 trees in the tick) |

Far below the 50 ms tick budget. In Godot at most six changed trees per frame
get leaf, splinter and dust bursts (`effects.gd`), the rest change silently.

Trees in the wind (prompt 086): `just objects-bench` (the densest wood of
`test_large`, quality of the settings, Apple M1 Max) before and after the
new wind function: near 15 m 43.5 / 43.3 ms, middle 120 m 31.9 / 31.8 ms,
overview 600 m 15.1 / 15.1 ms, village 19.4 / 19.4 ms: no change (the
bend, the gust and the flutter are a dozen sines per vertex, also in the
shadow passes). The wood itself is slow in this bench with shadows (23 FPS
near, 88 without them), before and after alike; not part of prompt 086.

Cloud shadows (prompt 087): `just bench-view --quality=<q>` (`bench_large`,
Apple M1 Max) with and without them (`--no-cloud-shadows`), frame time in ms
standing at 40 / 150 / 600 m:

| Quality | With | Without |
|---|---|---|
| high | 13.64 / 13.16 / 12.41 | 13.61 / 13.32 / 12.29 |
| medium | 11.26 / 12.88 / 11.08 | 11.21 / 12.81 / 11.07 |
| low | 10.84 / 10.77 / 10.00 | 10.78 / 10.76 / 9.98 |

At most 0.6 % apart, within the noise of two runs (two texture reads per
pixel in the light function, the same in every shader); the battle phase
varies by ±10 % between runs with the AI's decisions either way. They stay
on at every quality, including low.

Shadow distance in steps (prompt 090): `just bench-view --perf-quick`
(quality high, Apple M1 Max, one run each) with the steps and with the
gliding distance (`--no-shadow-steps`), frame time in ms standing at
40 / 150 / 600 m: 14.69 / 13.77 / 12.66 against 14.48 / 13.73 / 12.48
(1.4 %, 0.3 %, 1.4 % slower: the shadows reach up to 1.6 times as far, so
more objects fall into them: 495 against 416 draw calls at 40 m). Medium
and low were not measured.

All simulation budgets are met. What changed:

1. **Path components** (`PathGrid::relabel_components_in`): every wreck
   (`set_path_obstacle`) and every crushed or destroyed object relabeled the
   connected regions of the whole path grid, about 8 ms per component grid
   and three grids in this match: the spikes of 25–65 ms. Now flood fills
   within 32 m of the changed cells decide which regions the changed cells
   belong to; when the pieces of a region do not meet there, flood fills
   grow from all pieces in turn until at most one still grows (a pocket
   behind a cluster of wrecks costs its own size). Only a merge of two
   regions labels the whole grid. A randomized test compares the partition
   with a full labeling after 400 changes. With 16 m instead of 32 m, wreck
   clusters fell back to the full labeling now and then.
2. **Unit grid** (`sim::spatial::UnitGrid`, 64 m cells, counting sort,
   rebuilt before combat and before projectile flight): separation,
   automatic targets, the line of fire and projectile hits look at the
   units of the cells around their box instead of all units.
3. **Snapshots** are built into reused buffers and filtered while they are
   built (`Sim::snapshot_into`); the AI's view uses the same path.
4. The common case of the bridge-railing check is inlined into A\*.

**Scaling** (1004 units, 4500 soldiers: the scenario with twice the units
per group; 3000 ticks, minimum of 2 runs; no budget):

| ms per tick | Without unit grid | With unit grid |
|---|---|---|
| Sim tick, mean / p99 / max | 9.53 / 12.5 / 13.6 | 6.97 / 9.4 / 10.8 |
| · movement | 2.42 | 0.68 |
| · combat | 0.73 | 0.43 |
| · paths | 3.50 | 3.48 |
| · vision | 2.42 | 1.92 |

Separation was O(n²) and grows fastest; with the grid the tick is dominated
by the budgeted work (paths, vision), which does not grow with the number
of units. Path search runs into its budget of 32 768 expansions
(`NODE_BUDGET_PER_TICK`, about 3.6 ms) in most ticks of the approach;
vision into its 400 000 ray steps in few.


## Presentation

`just bench-view` (1920 × 1080, V-Sync off, Apple M1 Max, Metal, debug
bridge; the local player sees 254 units and 1125 soldiers, the rest is in
the fog). Frames per second, mean frame time in brackets:

Measured after merging prompts 021 and 023 (the "before" column is the
state before prompt 016):

| View | Before, high | After, high | After, medium |
|---|---|---|---|
| standing, 40 m | 13.0 (77 ms) | 92.9 (10.8 ms) | 113.3 (8.8 ms) |
| standing, 150 m | 14.3 (70 ms) | 89.0 (11.2 ms) | 91.1 (11.0 ms) |
| standing, 600 m | 30.5 (33 ms) | 101.2 (9.9 ms) | 109.3 (9.2 ms) |
| battle, 40 m | 10.7 (94 ms) | 75.1 (13.3 ms) | 81.1 (12.3 ms) |
| battle, 150 m | 9.5 (105 ms) | 68.2 (14.7 ms) | 73.4 (13.6 ms) |
| battle, 600 m | 24.3 (41 ms) | 55.7 (18.0 ms) | 61.2 (16.3 ms) |

Draw calls standing at 150 / 600 m: 2 525 / 6 579 before, 679 / 1 109
after. The battle views follow the front and differ a little from run to
run. Medium and high are close: the frame is limited by the CPU (engine,
scripts, the simulation), not by the GPU.

**Budgets:** all met, the battle at 600 m in medium just (61 FPS); its p99
frame (33 ms) comes from frames that run a tick of the debug-build
simulation (about 9 ms against 4 ms in release, every third frame) on top
of hundreds of particle emitters.

What changed, by effect:

1. **Fog lookups** (`fog_of_war.state_at`): effects ask whether a point is
   visible for every vehicle and every shot, and each call fetched the
   whole 250 KB vision image from the bridge: 41 ms per frame with 125
   tanks. It now reads the bytes cached with each upload.
2. **Infantry** (`infantry_view.gd`): squads outside the camera's view are
   skipped, squads beyond 260 m updated every 4th frame, soldiers set up
   without shadows (also those never drawn), soldiers beyond 450 m drawn as
   MultiMesh instances, soldier nodes top-level (a moving squad does not
   drag its skeletons along). See [INFANTRY.md](INFANTRY.md#performance-prompt-022).
3. **Vehicle models** (`unit_merge.gd`): static parts merged per unit type
   into one hull and one turret mesh (since prompt 077 a third for the
   `Gun`, which recoils: +1-2 draw calls per gun vehicle, ~0.5 ms at "high"
   on test_small); emptied leaf parts freed. A Panzer IV
   went from about 40 mesh nodes to 2 meshes (3 with the gun); the shader instance-uniform
   buffer no longer overflows ("Too many instances using shader instance
   variables" with 250 tanks).
4. **Per-frame scripts:** `combat_view` searched every unit's model for
   its turret every frame (hundreds of skeleton nodes per squad); `effects`
   read all unit types every frame and all aim directions per shot;
   `main.gd` looked up nodes by path and moved standing units. All cached
   or skipped.
5. **Bridge:** snapshots only for the two ticks a frame interpolates
   between, built into reused buffers.

Script time per system is printed by the bench; after the changes, in the
battle: units 0.7 ms, infantry 1.6 ms, combat view 0.6 ms, effects
0.8–1.0 ms, terrain chunks 0.9–1.3 ms.

**Instance uniforms** (measured 17.11.2026, prompt 068 side check): with
the 12 821 map object nodes of `test_large` (each with biome-look instance
uniforms) plus units and effects, `just bench-view` printed about 3 200
"Too many instances using shader instance variables" errors with
`buffer_size` 262 144, at commit 59e7826 as well as after prompts 062/063
(not a regression of those). `project.godot` now sets 1 048 576 (16 MB of
GPU memory): no errors, frame rates unchanged. The battle at 600 m measured
38–40 FPS in all three runs (old commit, HEAD, larger buffer), under the
budget of 45 already before prompts 062/063; not investigated further.

**Tall grass** (prompt 073): `TALL_AB=1 just terrain-bench tall_grass_test`
measures the stand against the same ground as plain meadow; on high it
costs 1–3 ms (most at 35 m, where the plants have all their blades), and
the test map stays just under 60 FPS at 80 and 110 m on high, where it
runs at 60–67 FPS as plain meadow; medium and low stay at the 120 FPS
cap. Table and flicker measurements: [ARCHITECTURE.md](ARCHITECTURE.md#tall-grass-prompt-073).

**Camera tilt** (prompt 096): `just bench-view --perf-quick --perf-tilts`
measures the standing views with the tilt of the zoom, tilted as flat
(25°) and as steep (85°) as the camera goes. A flat view shows ground up
to 2.4 camera distances deep instead of 1.4, so more is drawn. Measured
on high with other sessions working on the same machine (the absolute
rates are below the table above; compare the columns of this run only),
window 1280 × 720 interface pixels:

| View | Tilt of the zoom | Flat (25°) | Steep (85°) |
|---|---|---|---|
| standing, 40 m | 68.2 FPS (14.7 ms), 589 draw calls | 57.3 (17.4 ms), 817 | 69.7 (14.4 ms), 506 |
| standing, 150 m | 67.7 (14.8 ms), 913 | 57.1 (17.5 ms), 1 795 | 66.6 (15.0 ms), 909 |
| standing, 600 m | 78.5 (12.7 ms), 1 660 | 58.9 (17.0 ms), 5 873 | 81.2 (12.3 ms), 1 474 |

The flattest view costs 3–4 ms per frame (16–25 % of the frame rate; at
600 m it has 3.5 times the draw calls), the steepest costs nothing. The
limit stays at 25°: the flat view is the player's choice and one key away
from the normal one. Not measured: the battle views tilted flat, and a
machine without other load.

**Depth of field by view angle** (prompt 183): `just bench-view --perf-quick
--perf-tilts --perf-distances=30,40 --perf-dof=subtle` (`--perf-dof=` sets the
mode for the run, not saved), on high, window 1280 × 720, before and after the
rule (near blur off when flat, far blur from a screen row):

| View | Before | After |
|---|---|---|
| standing, 30 m, tilt of the zoom | 69.8 FPS (14.33 ms) | 71.4 (14.00 ms) |
| standing, 40 m, tilt of the zoom | 75.7 (13.20 ms) | 76.1 (13.14 ms) |
| standing, 30 m, flat (25°) | 70.4 (14.21 ms) | 69.8 (14.33 ms) |
| standing, 40 m, flat (25°) | 58.9 (16.98 ms) | 59.5 (16.79 ms) |
| standing, 30 m, steep (85°) | 83.5 (11.98 ms) | 81.5 (12.26 ms) |
| standing, 40 m, steep (85°) | 78.7 (12.70 ms) | 77.6 (12.88 ms) |

No difference beyond the noise of a run (±2 FPS both ways; the steep view
runs the same code as before). Switching the near blur off when flat saves
nothing measurable on this machine.

**HUD** (prompt 031: minimap, unit panel, command bar, events, waypoint
lines; budget 1 ms per frame): the bench prints it as "hud"; the battle views
have a dozen units selected. Measured with five other sessions building and
testing on the same machine (frame rates of that run are not comparable to
the table above): standing 0.24–0.27 ms, battle 0.70–0.83 ms per frame. The
minimap dots are one MultiMesh buffer filled by the bridge; the ground image
is built once (`test_large`: 500 × 500 cells).

Far camera distances (prompt 103, 20.11.2026, `just bench-view
--perf-distances=600,1000,1500`, 1280 × 720, M1 Max): standing 76 / 68 /
59 FPS (draw calls 1 455 / 5 048 / 8 720), battle 57 / 46 / 40 FPS (render
CPU 2.5 / 4.1 / 5.5 ms). A plain zoom limit beyond 600 m would cost 20-30 %
of the frame rate, so the game keeps 600 m and the overview (up to 1200 m)
turns shadows and decoration off.

Range circles (prompt 099, 20.11.2026): with the bench's dozen selected
units only the active one gets circles. Battle views 0.95–1.12 ms "hud" with
circles, 0.83–1.04 ms with both circle settings at `never` (same session):
about 0.1 ms for the circles. The target preview costs nothing while no
right button is held.

Vehicle tracks and ground effects (prompt 104, 20.11.2026, `just bench-view
--perf-column`: 30 tanks drive in a column across `test_large`, followed at
60 and 150 m, high, 1280 × 720, M1 Max): before 70.3 / 75.9 FPS (frame
14.22 / 13.18 ms), after 68.1 / 72.7 FPS (14.68 / 13.75 ms), about 3–4 %.
Scripts: tracks 0.15 ms (stamping every other frame; the bench adds them to
"effects", 0.84 ms against 0.57 ms before, the rest is the dust and clod
emitters by ground), the remaining 0.3 ms are the GPU's: three more samples
of the track raster in the terrain shader and the raster pass (a 2048²
window, drawn every second frame while stamps come in, once a second for
the fading). Memory: the raster 16 MB at high (RGBA8), the stamp ring 2 MB.
Graphics quality: `fx_track_texels` 0 turns tracks off; low draws 1024 ×
0.5 m and no clods, snow dust or spray (`fx_ground_fx`).

Kill sequences and state marks (prompt 105, 21.11.2026, `just bench-view
--perf-kills`: 40 own vehicles of `bench_large` destroyed within 2 s for the
presentation, measured at 150 m while the sequences run and at 60 m while
the wrecks burn, high, 1280 × 720, M1 Max): before (one explosion per kill)
51.7 / 47.4 FPS (frame 19.33 / 21.09 ms), after 49.2 / 47.7 FPS (20.32 /
20.95 ms; two more runs 48.2 / 44.7 and 47.6 / 46.5 before the core flash
and the fire stage's fireball were dropped). Scripts: effects 0.89 → 1.00
ms, HUD (state marks) 0.38 → 0.48 ms at 150 m; the rest is GPU overdraw of
the extra bursts (flame jet, smoke puffs, flares) over the 2–5 s of a
sequence. The battle views of `--perf-battle` before: 62.1 / 57.9 / 47.3
FPS at 40 / 150 / 600 m (the armies did not meet within the 150 s timeout).


## Large city (prompt 079)

`city_large` (3 km, 59 699 objects: 132 apartment blocks, 16 other city
buildings, 1 292 houses, 48 333 trees) measured on 17.11.2026, Apple M1
Max, release simulation, debug bridge. The machine was busy (load 6–9),
so `test_large` was measured again in the same session for comparison;
both are slower than the tables above.

| What | Command | city_large | test_large (same session) | Budget |
|---|---|---|---|---|
| Sim tick, big battle | `just bench --scenario data/scenarios/bench_city.toml --runs 2` (504 units, the armies of `bench_large` 1.5 km apart in front of the city, 6000 ticks) | mean 6.54, p99 11.5, max 15.5 ms (paths 2.10, vision 2.60, projectiles 1.04 ms) | `just bench --runs 2`: mean 5.62, p99 9.1, max 11.0 ms (paths 1.83, vision 2.91, projectiles 0.04 ms) | 5 / 10 / 20 ms |
| AI think per tick | same | 0.30 ms mean | 0.30 ms | 0.5 ms |
| Pathfinding | `just path-bench city_large` with `--range-m 1400` (200 single orders across the map) / `600` (inside the city) | planning ticks 7.16 / 7.22 ms mean, p99 9.9 / 9.2 ms; 35 100 / 3 760 expansions per request | planning ticks 5.27–5.29 ms mean, p99 7.5 ms; 113 000 expansions per request | – |
| Vision | `just vision-bench city_large` (500 units) | 9.16 ms mean (6.82 with one-cell sight), 293 000 ray steps per tick | 6.27 ms (4.45), 208 000 | ray steps ≤ 400 000 |
| Fire | `just fire-bench 40 6000 city_large` (a burning ring cell: 5 blocks and the courtyard) | 19 fires started, 25 burnt out, farthest fire 51 m from the cell center (the cell reaches 50 m: the streets stop it); tick mean 2.11 ms, max 13.4 ms | `town_small`: 2.02 ms, max 5.4 ms | 50 ms |
| Frame rate over the block quarter | `just objects-bench city_large --blocks-bench --quality=…` at 35 / 110 / 350 m | high 98 / 85 / 78 FPS, medium 161 / 150 / 134, low 256 / 235 / 183; 260 / 704 / 3 676 draw calls, 0.6–1.4 M triangles | – | ≥ 60 FPS |
| Frame rate with the battle | `godot … -- --scenario=bench_city.toml --ai=hard --perf-bench` (standing 40 / 150 / 600 m, battle 40 / 150 / 600 m) | medium 52 / 38 / 67, battle 49 / 47 / 30 FPS; high 51 / 37 / 59, battle 43 / 42 / 26 FPS | `bench_large` medium: 48 / 48 / 51, battle 40 / 37 / 30 FPS (window not drawable for part of the run) | ≥ 60 FPS medium |
| Loading and files | `just map-info city_large` | 3.68 MB files (height.png 1.1 MB, objects.csv 2.4 MB), 11.3 MB in memory, decode 37 ms, objects 34 ms | 6.8 MB files (objects.csv 3.9 MB), 20 MB, 69 + 52 ms (table in MAPS.md) | – |

Reading the numbers:

- **Simulation:** the city costs about 16 % more per tick than
  `test_large` in the same session: the path search expands nodes more
  slowly in the dense grid (the budget per tick is the same) and the
  projectile phase (which also runs fire, works and crossings) costs
  1 ms instead of 0.04 ms. Both maps miss the mean budget of 5 ms on the
  busy machine today; the city is not the cause. The phase difference of
  the projectile step was not investigated further (open).
- **Vision:** the many sight-blocking objects (blocks, houses, woods)
  make each ray step dearer; the ray budget per tick holds.
- **Frame rate:** over the block quarter every quality is well above
  60 FPS. With 504 units in view no view reaches 60 FPS today on either
  map; the battle runs at the rates of `test_large`, the standing army at
  150 m is 20 % slower (it stands in front of a wood: 3 000 draw calls,
  infantry script 5 ms). Measuring on a quiet machine is open.
- **Fire:** streets of 12 and 16 m are firebreaks (road terrain), so a
  burning cell stays a burning cell.

## Harbor city (prompt 081)

`harbor_city` (3.6 km × 2.4 km from real geodata, 16 683 objects: 1 455
buildings, 10 133 trees and palms, 3 741 bushes, 1 354 hedge pieces)
measured on 17.11.2026, Apple M1 Max, release simulation, debug bridge,
machine otherwise idle.

| What | Command | harbor_city | test_large (same session) | Budget |
|---|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_harbor.toml --runs 2` (254 units, half the armies of `bench_large` on the levelled ground at the four starts, 3 km apart; the town leaves no room for the full armies; 18 spawn points lie on blocked cells) | mean 4.62, p99 7.98, max 10.7 ms (paths 1.70, vision 2.49, projectiles 0.06 ms) | `just bench --runs 2` (504 units): mean 5.63, p99 9.1, max 10.7 ms (paths 1.83, vision 2.92) | 5 / 10 / 20 ms |
| AI think per tick | same | 0.10 ms mean, 1.3 ms max | 0.30 ms | 0.5 ms |
| Pathfinding | `just path-bench harbor_city` (200 single orders around the map center, ±1800 m / `--range-m 600`) | planning ticks 5.61 / 5.26 ms mean, p99 7.8 / 7.5 ms; 64 900 / 22 300 expansions per request; 14 / 16 of 271 / 274 requests fail (units spawned on the steep hillside at the center) | – | – |
| Vision | `just vision-bench harbor_city` (500 units) | 6.25 ms mean (4.17 with one-cell sight), 236 000 ray steps per tick | 6.38 ms (4.52), 208 000 | ray steps ≤ 400 000 |
| Frame rate over the town | `just objects-bench harbor_city --blocks-bench --quality=…` at 35 / 110 / 350 m | high 96 / 112 / 106 FPS, medium 135 / 161 / 151, low 192 / 226 / 203; 241 / 435 / 2 060 draw calls, 1.4–2.6 M triangles | – | ≥ 60 FPS |
| Frame rate with the battle | `godot … -- --scenario=bench_harbor.toml --ai=hard --perf-bench --quality=…` (standing 40 / 150 / 600 m, battle 40 / 150 / 600 m) | medium 72 / 67 / 124, battle 114 / 102 / 69 FPS; high 71 / 65 / 86, battle 89 / 93 / 71 FPS | – | ≥ 60 FPS |
| Loading and files | `just map-info harbor_city` | 2.16 MB files (height.png 1.42 MB, terrain.png 0.07 MB, objects.csv 0.68 MB), 10.8 MB in memory, decode 26 ms, objects 23 ms | 6.8 MB, 20 MB (table in MAPS.md) | – |
| Generating | `just gen-geo harbor_city` / `just gen-geo-check` | about 55 s (Python, one core) / about 55 s (two runs in parallel) | – | – |

Reading the numbers: every view stays above 60 FPS in every quality
measured; the standing armies at 150 m are the slowest view (65 FPS, the
infantry stands in front of a wood and the town). The simulation stays
within its budget with half the armies; the path search expands about
half as many nodes per long request as on `test_large` (the map is
narrower), and a third of the land is closed by grade, which the search
never enters.

## Maps from picture sketches (prompt 088)

The ten sketch maps, measured on 18.11.2026, Apple M1 Max, debug bridge,
quality high: `just objects-bench <map>` (densest wood chunk at 15 m and
120 m, overview at 600 m, densest village at 150 m). Battle tournaments:
`ai-tournament --map <map> --a hard --b hard --games 16 --mode battle`, both
faction orders, `--max-ticks 12000` on the 3 km maps (the armies need
longer to meet).

| Map | Size | Buildings / objects | Files | Wood 15 m | Wood 120 m | Overview | Village | AI µs/tick (mean) |
|---|---|---|---|---|---|---|---|---|
| `river_fields` | 1.6 km | 55 / 24 000 | 1.5 MB | 30 | – | 82 | 66 (middle) | – |
| `river_fork` | 1.6 km | 92 / 20 100 | 1.3 MB | 25 | 31 | 59 | 112 | 15 |
| `island_chain` | 1.6 km | 36 / 18 500 | 1.2 MB | 25 | 31 | 68 | 92 | 12 |
| `desert_lagoon` | 1.6 km | 66 / 15 700 | 1.0 MB | 108 | 75 | 136 | 89 | 101 (3v3) |
| `winter_town` | 1.6 km | 770 / 21 800 | 1.4 MB | 24 | 28 | 59 | 106 | – |
| `river_town` | 3.0 × 2.0 km | 879 / 45 600 | 3.0 MB | 23 | 23 | 55 | 98 | – |
| `valley_fields` | 3.0 × 2.0 km | 931 / 42 400 | 2.9 MB | 28 | 32 | 84 | 96 | – |
| `river_valley` | 3.6 × 2.0 km | 699 / 58 700 | 3.9 MB | 28 | 25 | 41 | 104 | – |
| `river_city` | 3.6 × 2.4 km | 3 042 / 36 000 | 3.1 MB | 25 | 32 | 61 | 105 | – |

Reading the numbers: towns and villages stay far above 60 FPS, the city
included (3 000 buildings, 105 FPS over its densest quarter). The woods are
the slow views, as on `river_fields` and `test_large` in stage 1 (about
30 FPS at 15 m with shadows, 73–82 FPS without): the tree shadows, not the
sketch maps. The overview of `river_valley` (41 FPS) looks at 58 000 objects,
most of them trees. Loading `river_city`: decode 45 ms, objects 28 ms.
Open: tick time with a large battle on each map (no bench scenario for the
sketch maps yet).

## Decisions

| Question | Decision | Why |
|---|---|---|
| Target scenario | 504 units (250 tanks, 250 squads with 2250 soldiers) | A big battle for this game; the AI leads every role as one group. 1004 units are measured too (scaling) without a budget. |
| Path components after changes | Local flood fill around the changed cells, splits by interleaved flood fills, full labeling only for merges | Labels are compared for equality only and not hashed: the same partition is enough (randomized test against the full labeling). |
| Unit grid | 64 m uniform grid, rebuilt before combat and before projectile flight, not state | Counting sort of 500 units costs microseconds; every query keeps the exact test of the old loop; candidates in index order where the order matters. |
| Path and vision budgets | Unchanged (rule constants) | Changing them changes behavior and the golden hash; only the work per node may get faster. |
| Vehicle models | Static parts merged per unit type (hull, turret and gun, one surface per material), automatic LODs | 30–50 parts per tank were 30–50 draw calls per pass and overflowed the shader instance-uniform buffer with 250 tanks. Only the turret and the gun (recoil, raised barrels: prompt 077) move. |
| Infantry far away | Squad culling and throttling in GDScript, no vertex animation textures yet | Removes most of the script and skeleton work without a baking pipeline. |
| CI | Ubuntu job `bench`, `continue-on-error`, table in the job summary | Shared runners are too noisy for hard limits. |

## Open

- **Path search** is now the largest part of a tick (about 3.6 ms whenever
  its budget is used up). Without changing the order of expansions (part
  of the rules) the remaining levers are memory layout (cost and deck
  arrays in one cache line, a smaller node record) or, as a rule change
  with a new golden hash, hierarchical search for long requests.
- **Vision** updates are budgeted; they could be cheaper with coarser
  rays for far cells (a rule change).
- **Far soldiers** show their rest pose; vertex animation textures would
  let them walk (INFANTRY.md).
- **Map objects** at 600 m: about 1 000 draw calls come from tree chunks
  (far meshes per 128 m chunk and variant, prompt 013); merging far trees
  into larger chunks would cut them for weaker GPUs.
- **Measurements** were taken while other sessions kept the machine busy
  (load 60–100 on 10 cores). Simulation numbers are the minimum of three
  runs and stable; frame rates vary by ±15 % between runs of the same
  build. Weaker GPUs and Windows/Linux have not been measured.
- **Godot's process-time monitor** (`TIME_PROCESS`) reported more time
  than whole frames took and is not used; the bench measures the systems
  itself.
