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
| `just stuck-audit` | Stuck units (prompt 173): groups drive across every map, stalls and give-ups per map with cause; see [Stuck audit](#stuck-audit-prompt-173) |
| `just fire-bench` | Fire on the town map (prompt 041): 40 units per side, 6000 ticks without fire and with a wood, a terrace and the dumps burning; mean and worst tick (ARCHITECTURE.md, "Fire") |
| `just terrain-bench`, `objects-bench`, `effects-bench`, `infantry-view-bench` | Single systems of the presentation |

`MEADOW_AB=1 just terrain-bench` (prompt 057) measures every terrain view once
per variant of the meadow look (each part off in turn, and the look before)
in one run; see ARCHITECTURE.md, "Meadow".
`SCATTER_AB=1 just terrain-bench` (prompt 191) does the same for the ground
scatter (`scatter`, `no_scatter`, `before_191`, `ssao_strong`) at 45, 110
and 300 m; `just bench-view --no-ground-scatter` starts without it. On high
the scatter costs 0.2–0.4 ms at 45 and 110 m and nothing at 300 m; standing
150 m in `bench-view` 14.59 → 15.05 ms (2026-11-22, M1 Max; details and
the spoiled battle runs in ARCHITECTURE.md, "Meadow").

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


### Gun crews (prompt 193)

`just bench-battery` (`data/scenarios/gun_crews.toml`: a firing battery of
six leFH 18 in the middle of the picture, twelve crewed guns with 80 men
in all, quality high, depth of field "miniature", window 1280 × 720, Apple
M1 Max, debug bridge, 2026-11-22), A/B in one run with the figures on and
off:

| View | crews on | crews off | gun crew script | draw calls on / off |
|---|---|---|---|---|
| battery, 40 m | 67.5 FPS (14.82 ms) | 68.5 FPS (14.61 ms) | 0.55 ms | 2 130 / 681 |
| battery, 150 m | 74.9 FPS (13.36 ms) | 75.8 FPS (13.20 ms) | 0.48 ms | 1 420 / 716 |

About 1.3 % of the frame for 80 figures; the extra draw calls are the
soldiers' body, gear and weapon surfaces (and shadows within 70 m). The
infantry LOD bands apply (every third frame to 260 m, frozen to 450 m,
hidden beyond).

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

### Harbor city in the Mediterranean biome (prompt 120)

`harbor_city` regenerated with the Mediterranean family and plants (18 344
objects: 1 498 buildings, 11 963 trees and palms, 3 534 macchia shrubs,
1 349 field-stone walls), measured on 23.11.2026, Apple M1 Max, debug
bridge, 1280 × 720 window, the forest version of the same session
(copied from git into a temporary map) beside it:

| `just objects-bench harbor_city --blocks-bench` at 35 / 110 / 350 m | Mediterranean | Forest (before) |
|---|---|---|
| high | 73.9 / 78.4 / 83.6 FPS; 371 / 530 / 2 521 draw calls, 2.0–2.9 M triangles | 76.2 / 79.0 / 86.6 FPS; 451 / 696 / 3 248 draw calls |
| medium | 103.9 / 111.1 / 120 FPS (vsync) | – |

Every view stays above 60 FPS on high and medium; the new trees cost
2–3 FPS (more trees with more cards), the houses fewer draw calls than the
town houses.

## Brick quarter (prompt 112)

`brick_quarter` (3.2 km × 2.2 km from real geodata, 7 331 objects: 5 628
buildings in street walls, 769 trees, 436 bushes, 498 hedge pieces)
measured on 23.11.2026, Apple M1 Max, release simulation, debug bridge.
`harbor_city` was measured again in the same session for comparison (the
machine was slower than on 17.11.).

| What | Command | brick_quarter | harbor_city (same session) | Budget |
|---|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_brick.toml --runs 2` (254 units, half the armies of `bench_large` on the cleared ground at the four starts, about 2.8 km apart; 0 spawns on blocked cells) | mean 2.71, p99 6.65, max 8.03 ms (paths 1.81, vision 0.30, movement 0.39 ms); the armies do not meet within the 300 s | – | 5 / 10 / 20 ms |
| AI think per tick | same | 0.08 ms mean, 0.6 ms max | – | 0.5 ms |
| Pathfinding | `just path-bench brick_quarter` (200 single orders, ±1800 m) | planning ticks 5.43 ms mean, p99 6.5 ms; 4 378 expansions per request; 0 of 1 043 requests fail | – | – |
| Vision | `just vision-bench brick_quarter` (500 units) | 3.98 ms mean (3.13 with one-cell sight), 106 000 ray steps per tick (max 238 000) | – | ray steps ≤ 400 000 |
| Frame rate over the quarter | `just objects-bench brick_quarter --blocks-bench --quality=…` at 35 / 110 / 350 m | high 99 / 93 / 86 FPS, medium 120 / 120 / 120, low 120 / 120 / 120 (vsync); 527 / 804 / 4 231 draw calls, 1.0–3.2 M triangles | – | ≥ 60 FPS |
| Frame rate with the battle | `godot … -- --scenario=bench_brick.toml --ai=hard --perf-bench --quality=…` (standing 40 / 150 / 600 m, battle 40 / 150 / 600 m) | high 55 / 60 / 79, battle 95 / 80 / 65 FPS; medium – / 61 / 103, battle 107 / 102 / 69 FPS | high 59 / 62 / 76, battle 71 / 76 / 70 FPS | ≥ 60 FPS |
| Loading and files | `just map-info brick_quarter` | 1.21 MB files (height.png 0.85 MB, terrain.png 0.06 MB, objects.csv 0.30 MB), 8.8 MB in memory, decode 30 ms, objects 22 ms | 2.16 MB | – |
| Generating | `just gen-geo brick_quarter` / `just gen-geo-check brick_quarter` | about 55 s (Python, one core) / about 60 s (two runs in parallel) | about 55 s | – |

Reading the numbers: the quarter itself costs little (5 600 houses of
four kinds, drawn as multimeshes): above the quarter every quality stays
far above 60 FPS, and so does the battle. The only views under 60 FPS are
the standing armies at 40 and 150 m in quality high (55 and 60 FPS,
5 860 and 4 560 draw calls, the infantry script alone 3 ms); `harbor_city`
reached 59 and 62 FPS in the same session, so the 1 116 standing soldiers
are the limit, and the street walls cost about 4 FPS more in the close
view [provisional: the number of buildings was not lowered; measure on a
quiet machine]. The standing medium view at 40 m was not captured.
Observation on hiding buildings: from the game camera (45° at 110 m) the
five-storey tenements hide the street behind them on their far side
only; the fight in the street in front stays visible (picture
`walls_game_*` of `SHOT_SET=land just terrain-shots`), so hiding
buildings is not needed for this quarter (open for taller quarters).

## Coastal heights (prompt 113)

`coastal_heights` (6.0 km × 3.4 km from real geodata, the largest map so
far: 3 001 × 1 701 samples; 5 029 objects: 776 buildings, 3 651 trees,
494 bushes, 22 hedge pieces, 86 field works; 66 % of the map tall grass)
measured on 23.11.2026, Apple M1 Max, release simulation, debug bridge.
The game was open in a second window during the frame rate runs (another
session), so the "high" runs were disturbed (frames forced while the
window was not drawable; `harbor_city` measured in the same state reached
only 20–41 FPS in views that make 96–112 FPS on a quiet machine).

| What | Command | coastal_heights | Budget |
|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_coastal.toml --runs 2` (254 units, half the armies of `bench_large` at the four starts, 5.3 km apart; 39 spawns on blocked cells: the armies stand on the slopes around the starts) | mean 5.04, median 5.18, p99 6.04, max 31.3 ms (paths 3.99 mean, 30.1 max; vision 0.12; movement 0.69 ms); the armies do not meet within the 300 s; 2 882 path requests, 1 897 dropped (replaced by newer orders), 192 M nodes expanded | 5 / 10 / 20 ms |
| AI think per tick | same | 0.07 ms mean, 0.18 ms max | 0.5 ms |
| Pathfinding | `just path-bench coastal_heights` (200 single orders, ±1 800 m) | planning ticks 4.66 ms mean, p99 5.7, max 21.0 ms; 163 000 expansions per request (37 × `brick_quarter`: routes go around the ridges and cliffs); 0 of 201 requests fail; not every unit idle after 24 000 ticks | – |
| Vision | `just vision-bench coastal_heights` (500 units) | 5.06 ms mean (4.78 with one-cell sight), 38 000 ray steps per tick (max 122 000); the grid has 319 000 cells | ray steps ≤ 400 000 |
| Frame rate over the map | `just objects-bench coastal_heights --quality=…` (densest wood 15 m, 120 m, overview 600 m, town 150 m) | medium 72 / 67 / 104 / 103 FPS, low 136 / 127 / 156 / 166; high (disturbed) 31 / 38 / 67 / 71, without shadows 77 / 57; 433 / 924 / 838 / 273 draw calls, 2.6–4.0 M triangles, up to 130 000 grass decorations | ≥ 60 FPS |
| Frame rate with the battle | `godot … -- --scenario=bench_coastal.toml --ai=hard --perf-bench --quality=…` (standing 40 / 150 / 600 m, battle 40 / 150 / 600 m) | medium 58 / 55 / 104, battle 61 / 61 / 70 FPS; high (disturbed, 4 281 forced frames) 56 / 54 / 72, battle 26 / 29 / 33 FPS | ≥ 60 FPS |
| Loading and files | `just map-info coastal_heights` | 4.34 MB files (height.png 4.05 MB, terrain.png 0.07 MB, objects.csv 0.21 MB), 25.5 MB in memory (heights i32 + terrain u8), decode 67 ms, objects 40 ms | – |
| Generating | `just gen-geo coastal_heights` / `just gen-geo-check coastal_heights` | about 45 s (Python, one core) / about 45 s (two runs in parallel) | – |

Reading the numbers: the map costs memory and path search, not objects.
25.5 MB is the 25 MB the prompt allowed (6.0 × 3.4 km instead of 5 × 4 km,
the same area). The simulation stays within its mean budget, but the path
search has single ticks of 20–30 ms when many long requests over the
ridges come at once (over the 20 ms maximum) [provisional: open, the
budget of 32 768 expansions per tick is exceeded inside a single long
search]. In medium every view of the battle stays at or above 60 FPS;
the standing armies at 40 and 150 m are at 55–58 FPS, as on the other
geodata maps (infantry script 3 ms). Over the wood and the tall grass in
high the shadows of the grass decoration cost most; this and every high
value is to be measured again on a quiet machine (open).

## Dune coast (prompt 114)

`dune_coast` (4.6 km × 3.2 km from real geodata, 2 301 × 1 601 samples,
heights from the official 1 m terrain model; 2 580 objects: 260
buildings, about 1 750 trees, 300 bushes, 200 hedge pieces, 59 field
works; 26 % of the map tall grass, 6 % mud flats) measured on
23.11.2026, Apple M1 Max, release simulation, debug bridge, no other game
window open.

| What | Command | dune_coast | Budget |
|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_dune.toml --runs 2` (254 units, half the armies of `bench_large` at the four starts, 4.2 km apart; 3 spawns on blocked cells) | mean 4.15, median 5.08, p99 6.58, max 7.09 ms (paths 3.11 mean, 5.96 max; vision 0.23; movement 0.58 ms); the armies do not meet within the 300 s; 3 965 path requests, 2 071 dropped (replaced by newer orders), 141 M nodes expanded | 5 / 10 / 20 ms |
| AI think per tick | same | 0.07 ms mean, 1.14 ms max | 0.5 ms |
| Pathfinding | `just path-bench dune_coast` (200 single orders, ±1 800 m) | planning ticks 4.71 ms mean, p99 6.0, max 6.5 ms; 126 000 expansions per request (the open island has few walls, but every route is long); 0 of 224 requests fail; not every unit idle after 24 000 ticks | – |
| Vision | `just vision-bench dune_coast` (500 units) | 4.74 ms mean (4.55 with one-cell sight), 20 000 ray steps per tick (max 86 000); the grid has 230 000 cells | ray steps ≤ 400 000 |
| Frame rate over the map | `just objects-bench dune_coast --quality=…` (densest wood 15 m, 120 m, overview 600 m, village 150 m) | medium 66 / 73 / 114 / 114 FPS, low 127 / 131 / 162 / 191; high 30 / 43 / 76 / 81, without shadows 78 / 61; 447 / 853 / 1 408 / 627 draw calls, 2.9–3.8 M triangles, up to 175 000 grass decorations | ≥ 60 FPS |
| Frame rate with the battle | `godot … -- --scenario=bench_dune.toml --ai=hard --perf-bench --quality=…` (standing 40 / 150 / 600 m, battle 40 / 150 / 600 m) | medium 49 / 50 / 104, battle 76 / 72 / 98 FPS; high 46 / 47 / 82, battle 62 / 53 / 75 FPS | ≥ 60 FPS |
| Loading and files | `just map-info dune_coast` | 3.23 MB files, 18.4 MB in memory (heights i32 + terrain u8), decode 57 ms, objects 31 ms; steepest grade 280 % (cliff and dune lee sides) | – |
| Generating | `just gen-geo dune_coast` / `just gen-geo-check dune_coast` | about 45 s (Python, one core) / about 50 s (two runs in parallel); `extract` with 28 DGM1 tiles 31 s | – |

Reading the numbers: the map is smaller in memory than `coastal_heights`
(18.4 against 25.5 MB) and its path search has no spikes (max 6–7 ms per
tick against 31 ms there): the island is open, a long route over heath
and dunes is still found within the per-tick budget. The simulation
stays within budget. In medium the battle views keep 72–98 FPS, but the
standing armies at 40 and 150 m drop to 49–50 FPS (coastal heights 55–58;
infantry scripts 3.2 ms, 6 100 draw calls at 40 m) [provisional: open, as
on the other geodata maps]. In high the shadows of the tall grass on the
heath cost most (near the wood 30 FPS with, 78 without shadows), the same
open point as on `coastal_heights`, here measured on a quiet machine.

## Rhine bridge (prompt 118)

`rhine_bridge` (3.0 km × 3.2 km from real geodata, 1 501 × 1 601
samples, heights from the Dutch terrain model; 15 162 objects: 4 924
buildings and the church, 7 547 trees, 1 779 bushes, 903 hedge pieces, the
8 pieces of the steel bridge) measured on 23.11.2026, Apple M1 Max,
release simulation, no other game window open.

| What | Command | rhine_bridge | Budget |
|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_rhine.toml --runs 2` (254 units, half the armies of `bench_large`, each side in the town north of the river and in the polder south of it; 1 spawn on a blocked cell) | mean 4.33, median 4.26, p99 5.88, max 9.27 ms (paths 3.78 mean, 5.53 max; vision 0.16, max 3.79; movement 0.21 ms); the armies do not meet within the 300 s; 9 332 path requests, 7 194 dropped (replaced by newer orders), 196 M nodes expanded | 5 / 10 / 20 ms |
| AI think per tick | same | 0.08 ms mean, 0.62 ms max | 0.5 ms |
| Pathfinding | `just path-bench rhine_bridge` (200 single orders, ±1 800 m) | planning ticks 4.91 ms mean, p99 5.9, max 6.2 ms; 40 600 expansions per request; 0 of 637 requests fail | – |
| Vision | `just vision-bench rhine_bridge` (500 units) | 5.23 ms mean (4.82 with one-cell sight), 42 000 ray steps per tick (max 146 000); the grid has 150 000 cells | ray steps ≤ 400 000 |
| Loading and files | `just map-info rhine_bridge` | 3.19 MB files, 12.0 MB in memory, decode 47 ms, objects 27 ms; steepest grade 827 % (quay walls, the dams' sides) | – |
| Generating | `just gen-geo rhine_bridge` | about 85 s (Python, one core); `extract` with 25 AHN tiles 24 s | – |

The simulation stays within budget; the slowest ticks (9 ms) are vision
updates when the armies start to see each other at the end of the run.
The frame rate in Godot was not measured [provisional, open: expected
like `river_crossing`, whose town is as dense].

## River crossing (prompt 116)

`river_crossing` (3.6 km × 3.2 km from real geodata, 1 801 × 1 601
samples, heights from the Dutch terrain model; 9 667 objects: 3 944
buildings, 4 372 trees, 853 bushes, 490 hedge pieces, the 8 pieces of the
long steel bridge) measured on 23.11.2026, Apple M1 Max, release
simulation, debug bridge, no other game window open.

| What | Command | river_crossing | Budget |
|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_river.toml --runs 2` (254 units, half the armies of `bench_large`, each side in the town or the meadows and in the polder over the river; 2 spawns on blocked cells) | mean 4.30, median 4.32, p99 5.64, max 6.50 ms (paths 3.76 mean, 5.32 max; vision 0.12; movement 0.24 ms); the armies do not meet within the 300 s; 10 339 path requests, 8 392 dropped (replaced by newer orders), 197 M nodes expanded | 5 / 10 / 20 ms |
| AI think per tick | same | 0.07 ms mean, 0.58 ms max | 0.5 ms |
| Pathfinding | `just path-bench river_crossing` (200 single orders, ±1 800 m) | planning ticks 4.64 ms mean, p99 5.7, max 6.3 ms; 56 600 expansions per request (half of `dune_coast`: the river and the town's blocks close most of the map, the bridge is the only way over); 0 of 300 requests fail (targets in the water end at the bank) | – |
| Vision | `just vision-bench river_crossing` (500 units) | 4.82 ms mean (4.47 with one-cell sight), 36 000 ray steps per tick (max 404 000 in the first tick); the grid has 180 000 cells | ray steps ≤ 400 000 |
| Frame rate over the map | `just objects-bench river_crossing --quality=…` (densest wood 15 m, 120 m, overview 600 m, old town 150 m) | medium 69 / 75 / 120 / 118 FPS; high 30 / 44 / 98 / 85, without shadows 74 / 60; 428 / 796 / 664 / 1 484 draw calls, 2.2–2.6 M triangles, up to 121 000 grass decorations | ≥ 60 FPS |
| Loading and files | `just map-info river_crossing` | 3.25 MB files, 14.4 MB in memory, decode 50 ms, objects 27 ms; steepest grade 607 % (quay walls, the terrace edge) | – |
| Generating | `just gen-geo river_crossing` / `just gen-geo-check river_crossing` | about 50 s (Python, one core) / about 60 s (two runs in parallel); `extract` with 30 AHN tiles 28 s | – |

Reading the numbers: the simulation stays within budget; the bridge is
no bottleneck for the path search (a route over it is a straight run
along the deck). Over the map and the town medium keeps 69–120 FPS; in
high the shadows near the densest wood cost most (30 FPS with, 74
without), the same open point as on `dune_coast` and `coastal_heights`.
A battle view with the bench armies in Godot was not measured
[provisional: as on the other geodata maps, standing armies are expected
near 50 FPS in medium].

## Long bridges (prompt 115)

Path planning over a 364 m steel bridge (60 m approach, 244 m arch, 60 m
approach, 18 m above a 280 m river, earth dams of 12 % at both ends) on
the 1.6 km × 400 m test map of `crates/sim/tests/long_bridges.rs`:
100 units (half IS-2, half rifle squads) on the west bank, each ordered on
its own to a point on the east bank, all in one tick
(`cargo test --release -p sim --test long_bridges bench -- --ignored
--nocapture`), 23.11.2026, Apple M1 Max, release, no game window open.

| Case | Requests | Expansions per request | All paths after | Time per tick while planning (mean / max) |
|---|---|---|---|---|
| Bridge intact | 100, all found | 13 100 | 41 ticks (2.1 s) | 3.19 / 3.49 ms |
| East approach down (targets unreachable) | 100, all to the nearest reachable cell | 6 100 | 19 ticks (1.0 s) | 3.37 / 3.67 ms |

The deck adds no cost of its own: deck cells are ordinary path cells
(three lanes wide on the 12 m roadway), the chain test runs once when the
map loads and the hanging-joint test only for steps between a deck cell and
a cell of another (or no) deck. With the bridge cut the components send
every unreachable target to the hanging end of the arch, so no search
explores the far bank in vain. The planning ticks stay within the node
budget (32 768 expansions, about 3–4 ms).

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

## Landing gulf (prompt 121)

`landing_gulf` (5.0 km × 4.4 km from real geodata, 2 501 × 2 201
samples, Copernicus heights, biome `mediterranean`; 20 935 objects: 538
buildings and the church, 7 412 trees, 12 225 macchia bushes, 331 terrace
and 131 field-stone wall pieces, 253 pieces of wire, the works; 7.5 km²
of water) measured on 23.11.2026, Apple M1 Max, release simulation, no
other game window open.

| What | Command | landing_gulf | Budget |
|---|---|---|---|
| Sim tick, battle | `just bench --scenario data/scenarios/bench_gulf.toml --runs 2` (254 units, half the armies of `bench_large`, the north on the plain, the south at the village and on the coast plain) | mean 3.46, median 4.54, p99 6.10, max 8.53 ms (paths 2.53 mean, 5.82 max; vision 0.31, max 3.22; movement 0.39 ms); 3 334 path requests, 0 failed, 120 M nodes expanded | 5 / 10 / 20 ms |
| AI think per tick | same | 0.08 ms mean, 0.61 ms max | 0.5 ms |
| Pathfinding | `just path-bench landing_gulf` | not usable here: the bench puts its units around the map's center, which is the gulf (all 200 requests fail); the battle bench above plans 3 334 paths without a failure | – |
| Vision | `just vision-bench landing_gulf` (500 units) | 0.68 ms mean, p99 6.7, max 16.8 ms; 1 656 ray steps per tick (max 146 000); the grid has 344 000 cells (half of it water) | ray steps ≤ 400 000 |
| Loading and files | `just map-info landing_gulf` | 4.05 MB files, 27.5 MB in memory, decode 71 ms, objects 49 ms | – |
| Frame rate, high | `just objects-bench landing_gulf` (1280 × 720) | densest wood (119 trees in a chunk) near 60.2, middle 55.7, overview 86.8, bare 102.3, village 71.6 FPS; without shadows near 75.4, middle 61.8 | 60 FPS |
| Frame rate, medium | `just objects-bench landing_gulf --quality=medium` (woods still 8 m apart) | near 67.0, middle 78.6, overview 120, bare 120, village 105.5 FPS | 60 FPS |
| Generating | `just gen-geo landing_gulf` | about 75 s (Python, one core) | – |

The simulation stays within budget. The frame rate on "high" falls
below 60 FPS over the densest stone pine wood at 120 m (55.7 FPS): with
woods 8 m apart (as on the other maps) it was 45.5 near and 50.7 at
120 m, so the site plants its woods 10 m apart; the rest is the shadows
of trees and macchia over the hills (61.8 FPS without them) [open]. The
water of the gulf adds 3.8 M triangles (built once, 153 ms).

## Archipelago (prompt 122)

`archipelago` (3.6 km × 2.4 km sketch map, 1 801 × 1 201 samples,
heights −3 … 149 m in 3 mm steps, look `vivid`; 39 129 objects: 104
buildings, 37 321 trees, rock massifs and outcrops, 31 bridge pieces, 51
pier segments, 105 rapids stones; 2.9 km² of water) measured on
23.11.2026, Apple M1 Max, no other game window open.

| What | Command | archipelago | Budget |
|---|---|---|---|
| Loading and files | `just map-info archipelago` | 3.03 MB files, 10.8 MB in memory, decode + validate 37.5 ms, objects 29.7 ms | – |
| Water and terrain | game start | water 828 616 vertices, 1.63 M triangles, 66 ms; terrain 29 × 19 chunks, 1.1 s; 3 waterfalls | – |
| Frame rate, high | `just objects-bench archipelago` (1280 × 720) | densest wood (390 trees in a chunk, rock massifs and conifers) near 36.4, middle 36.1, overview 56.6, bare 99.3, village 56.5 FPS; without shadows near 60.3, middle 61.3 | 60 FPS |
| Frame rate, medium | `just objects-bench archipelago --quality=medium` | near 69.0, middle 67.4, overview 93.0, bare 120, village 89.2 FPS | 60 FPS |
| Generating | `just gen-sketch archipelago` | about 4 s (release) | – |

The woods are planted 9 m apart (first draft 6.5 m: 64 000 trees, 690 in
the densest chunk, near 30 FPS). On "high" the shadows of the dense
conifer and crag stands keep the wood views under 60 FPS, as on the other
sketch maps (`island_chain` wood 25 FPS) [open]. The vivid look costs
nothing measurable (the same shaders with other values; one `if` per roof
pixel).

## River front (prompt 123)

`river_front` (3.6 km × 2.0 km sketch map, 1 801 × 1 001 samples,
heights −4.6 … 107.8 m in 2 mm steps; 36 174 objects: 308 buildings,
32 852 trees, rock massifs and outcrops, 60 bridge pieces, 216
rapids stones, field works; 0.63 km² of water) measured on 23.11.2026,
Apple M1 Max, no other game window open.

| What | Command | river_front | Budget |
|---|---|---|---|
| Loading and files | `just map-info river_front` | 3.17 MB files, 9.0 MB in memory, decode + validate 34.6 ms, objects 26.1 ms | – |
| Water and terrain | game start | water 190 500 vertices, 374 146 triangles, 37 ms; terrain 29 × 16 chunks, 0.7 s; objects in 9 521 nodes, 0.23 s | – |
| Frame rate, high | `just objects-bench river_front` (1280 × 720) | densest wood (319 trees in a chunk, a corner mountain) near 33.4, middle 37.2, overview 61.3, bare 103.4, middle island 50.4 FPS; without shadows near 62.2, middle 59.5 | 60 FPS |
| Frame rate, medium | `just objects-bench river_front --quality=medium` | near 65.8, middle 67.0, overview 104.3, bare 120, middle island 81.7 FPS | 60 FPS |
| Generating | `just gen-sketch river_front` | about 3.6 s (release) | – |
| Test | `sketch_maps_are_the_same_every_time` (all twelve sketch maps twice, debug) | 113 s on 10 cores | – |

As on the other sketch maps the shadows of the dense conifer stands keep
the wood views under 60 FPS on "high" [open, as `archipelago`]; medium
stays above 60 everywhere.

## River fork front (prompt 125)

`river_fork_front` (3.6 km × 2.0 km sketch map, 1 801 × 1 001 samples;
17 782 objects; prepared ground: 3 669 craters, 11 fire trench lines
(16 km), 15 communication trenches (1.9 km), 12 km of wire, 50 MG nests,
148 dugouts) measured on 24.11.2026, Apple M1 Max, no other game window
open.

| What | Command | river_fork_front | Budget |
|---|---|---|---|
| Loading and files | `just map-info river_fork_front` | 2.49 MB files, 9.0 MB in memory, decode + validate 62.8 ms, objects 22.0 ms | – |
| Prepared ground and grid | `Sim::new` (path-bench "built in") | 0.25 s | – |
| Path search in the crater field | `cli path-bench --map data/maps/river_fork_front --range-m 900` (200 single orders) | 438 requests (404 found, 2 failed and 32 dropped: targets in the rivers), 28 000 nodes per request, planning ticks mean 6.9 ms (p99 8.2, max 10.4), all ticks mean 0.55 ms (p99 6.6) | 32 768 nodes per tick |
| Frame rate, high | `just objects-bench river_fork_front --markers=front_n,battery_n1,supply_fork` (1280 × 720) | densest wood (392 trees in a chunk, the wood in the north-east) near 35.1, middle 39.0, overview 67.0, bare 116.3, village 77.0, north front 76.5, gun position at the wood 38.8, fork 77.2 FPS; without shadows near 69.8, middle 62.5 | 60 FPS |
| Frame rate, medium | the same `--quality=medium` | near 70.1, middle 69.8, overview 109.7, village 113.8, north front 115.7, gun position at the wood 75.0, fork 112.2 FPS | 60 FPS |
| Generating | `just gen-sketch river_fork_front` | about 4 s (release) | – |

The wood views stay under 60 FPS on "high" (shadows of dense stands, as
on every wooded sketch map) [open].

## Trench line (prompt 124)

`trench_line` (2.4 km × 2.0 km sketch map, 1 201 × 1 001 samples;
12 243 objects; prepared ground: 5 692 craters, 2 290 fire trench, 550
communication trench and 2 530 wire pieces, 26 MG nests, 78 dugouts)
measured on 23.11.2026, Apple M1 Max, no other game window open.

| What | Command | trench_line | Budget |
|---|---|---|---|
| Prepared ground | `Sim::new` (path-bench "built in") | 0.16 s with craters, digs and wire; 0.13 s without the works. Before batching the wire (every piece refreshed its cells over every other obstacle) 1.0 s | – |
| Path search | `cli path-bench --map data/maps/trench_line --range-m 900` (200 single orders) | 2 832 requests (vehicles flatten wire, routes through it plan again), planning ticks mean 4.9 ms (p99 5.6), all ticks mean 1.0 ms (p99 5.3); without the wire 380 requests, 0.45 ms; `river_fork` (±750 m) for comparison 281 requests, planning 4.1 ms. Crushing wire tested the box of each piece first (it measured the distance to all 2 500 pieces for every moving vehicle every tick: mean 5.0 ms before) | 32 768 nodes per tick |
| Frame rate, high | `just objects-bench trench_line --markers=front_n,battery_n1,supply_n_trench` (1280 × 720) | densest wood (a rear corner) near 26.8, middle 39.1, overview 82.5, bare 114.7, middle house 78.8, front trench 83.5, trench supply point 76.4 FPS | 60 FPS |
| Frame rate, medium | the same `--quality=medium` | near 62.9, middle 70.9, overview 120, middle house 115.6, front trench 119.6, trench supply point 110.6 FPS | 60 FPS |

`works_view.gd` rebuilt every work and searched the joints between all
pairs of trench pieces every 0.25 s; with thousands of map works that
cost frames of 90 ms. Its joints now go through a half-meter grid of
piece ends, and it skips the rebuild while `SimHost.field_works_signature`
stays the same (worst frame 90 → 13 ms over the trenches). The wood views
stay under 60 FPS on "high" (shadows of the dense stand, as on every
wooded sketch map) [open].

## Boats (prompt 119)

Path grid and searches with a boat type in the match (`dune_coast`,
4.6 × 3.2 km, Apple M1 Max, release, three runs each): building the grid
for a rifle squad alone takes 109–112 ms, with an LCVP as well 222–227 ms
(the water levels over 3.7 million samples and a second profile; only in
matches with boats). A boat path of 4.3 km along the whole coast (121
waypoints) takes 0.43–0.44 ms, under the budget of one tick. The landing
mission (`beach_landing`) plays headless like the other missions.

## Stuck audit (prompt 173)

`just stuck-audit` (`cli stuck-audit`, rules in
[ARCHITECTURE.md](ARCHITECTURE.md#movement-and-separation), settings in
`data/stuck_audit.toml`): five groups drive every route of every map (starts,
supply points, the eight longest bridges both ways); a unit is stuck when it
follows a route and moves less than 1 m in 10 s, or stops more than its width
plus 8 m short of its planned end. 26 maps, 1810 group orders, 5792 units,
about 60 s on 4 threads (Apple M1 Max). Before = the same audit against the
movement of prompt 166 (commit `8aec865`).

| Map | Orders | Stuck units (orders) before | After |
|---|---|---|---|
| ambush_meadow | 10 | 0 (0.0%) | 0 (0.0%) |
| city_blocks | 10 | 0 (0.0%) | 0 (0.0%) |
| city_large | 60 | 19 (20.0%) | 0 (0.0%) |
| desert_lagoon | 100 | 6 (6.0%) | 0 (0.0%) |
| desert_village | 35 | 0 (0.0%) | 0 (0.0%) |
| farms | 45 | 6 (6.7%) | 0 (0.0%) |
| harbor_city | 55 | 14 (12.7%) | 0 (0.0%) |
| island_chain | 65 | 19 (16.9%) | 0 (0.0%) |
| mountain_pass | 70 | 10 (10.0%) | 0 (0.0%) |
| river_city | 140 | 27 (10.7%) | 0 (0.0%) |
| river_fields | 75 | 11 (9.3%) | 0 (0.0%) |
| river_fields_winter | 75 | 10 (9.3%) | 0 (0.0%) |
| river_fork | 125 | 11 (4.0%) | 0 (0.0%) |
| river_land | 75 | 17 (14.7%) | 0 (0.0%) |
| river_town | 140 | 28 (13.6%) | 0 (0.0%) |
| river_valley | 90 | 22 (13.3%) | 0 (0.0%) |
| rock_hills | 50 | 13 (14.0%) | 0 (0.0%) |
| skirmish | 55 | 11 (14.5%) | 0 (0.0%) |
| tall_grass_test | 45 | 6 (8.9%) | 0 (0.0%) |
| test_large | 100 | 5 (4.0%) | 0 (0.0%) |
| test_small | 45 | 10 (17.8%) | 0 (0.0%) |
| town_small | 25 | 3 (8.0%) | 0 (0.0%) |
| twin_meadow | 65 | 8 (9.2%) | 0 (0.0%) |
| valley_fields | 140 | 29 (12.1%) | 0 (0.0%) |
| winter_town | 80 | 11 (8.8%) | 0 (0.0%) |
| winter_village | 35 | 1 (2.9%) | 0 (0.0%) |

Causes before (298 cases, 63 of them 30 s or longer): another unit in the
way 228 (groups converging on a target or meeting at a bridge: units crept
forward by millimeters, pushed back almost as far as they drove, so the 2 s
stall rule never fired; squads pushed each other sideways off the deck's
line), stopping short at a bridgehead 35 (pushed beside a deck, then onto the
steep bank below it: no search could start there), stalls at a bridgehead 13,
water and banks 10, a vehicle sliding along an obstacle by millimeters
(never "blocked", never re-planned). After the fixes of prompt 173 no
stuck order on any map; the "stuck" events of the simulation stayed at 0 in
the route runs and in five AI games (one 11 s stall of an AI Panzer IV on a
slope at twin_meadow's minefield remains, below the 20 s limit). Units lost
or immobilized by twin_meadow's minefield (7) are counted apart.

Also measured with the stall at 2 m in 10 s (the first setting): 15
infantry orders on island_chain and desert_lagoon "stalled" on the steepest
passable slopes, crawling at about 0.15 m/s: slow, not stuck, so the audit
uses 1 m.

`just path-bench` (200 units on `test_large`, Apple M1 Max), before → after
prompt 173: single orders 288 → 284 requests, planning ticks mean 4.6 → 4.6
ms, all ticks 0.46 → 0.46 ms; groups of 8 163 → 161 requests, 4.7 → 4.7 ms,
0.42 → 0.42 ms. `just bench` (one run): sim step mean 2.86 → 3.11 ms, p99
6.3 → 6.1 ms, max 8.8 → 8.4 ms, movement 0.43 → 0.45 ms (the remaining route
length is cached per waypoint; summing it every tick had doubled the
movement phase); the rest is path search: the hard AIs' units move
differently (2152 → 2151 requests, 58 → 77 million expansions). Within the
budgets.

### Driving together (prompt 187)

`just stuck-audit --march` drives the same routes with an attack-move
formation order (`FormationMove`, `Engage::All`, the AI's order) instead of
`MoveTo`, so groups drive together ([ARCHITECTURE.md](ARCHITECTURE.md#movement-and-separation)).
Before = the same code with `[march] enabled = 0` and without the change to
fire-team bounds at bridges.

| Audit | Arrived units | Stuck orders | Gave up | Stalls of 30 s or more |
|---|---|---|---|---|
| `--march`, before | 4225 of 5792 | 1560 (36.1 %) | 311 (squads) | 1 |
| `--march`, after | 4218 of 5792 | 1566 (36.2 %) | 25 (22 squads, 3 Panzer IV) | 16 (squads) |
| plain (`MoveTo`), after | 5783 of 5792 | 2 (0.1 %) | 2 | 0 |

Most "stuck" orders with `--march` are short halts of attack-moving squads
that bound by fire teams (prompt 047) even without contact; the audit counts
those halts as stalls, the simulation does not (`speed_pct == 0`). The give-ups
were squads that halted on bridge decks; they no longer bound there. A first
version of driving together let the front wait for the rear; waiting units
stood on decks and in lanes, and vehicles and squads behind them gave up (81
on island_chain alone). The long stalls left are squads on slopes and in
traffic at bridges. The two plain-move cases (a squad at river_fields, a
towing truck at twin_meadow) are no march orders.

`just ai-tournament --matrix`: win shares unchanged in every pairing (hard
vs normal 47/52 %, hard vs easy 50/50 %, normal vs easy 52/47 %, easy vs
passive 65 % with 14 timeouts), game hashes changed, mean times 56 → 54 s
and 40 → 39 s in two pairings. `just bench`: sim step mean 2.98 → 2.87 ms
(p99 5.9 → 6.3 ms, noise of one run), `just path-bench` unchanged (284
requests, 4.47 → 4.48 ms per planning tick).

## AI think time (prompt 178)

`just ai-tournament --matrix` (Apple M1 Max): AI think per tick about
9–13 µs mean before and after prompt 178 (house search over the map's
enterable buildings only for waiting defenders that see an enemy).

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
