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
| `just bench-view` | Frame rate in Godot with the same scenario and a Hard AI as enemy: camera 40 / 150 / 600 m over the own army standing, then the army advances and the same distances in the battle. FPS, frame time (mean, p99, worst), the simulation's part of the frame (`SimHost.process_usec`), render CPU, draw calls, primitives and the script time of the big presentation systems. `--quality=medium`, `--perf-quick` (standing only). Screenshots in `user://perf-bench/` |
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

**HUD** (prompt 031: minimap, unit panel, command bar, events, waypoint
lines; budget 1 ms per frame): the bench prints it as "hud"; the battle views
have a dozen units selected. Measured with five other sessions building and
testing on the same machine (frame rates of that run are not comparable to
the table above): standing 0.24–0.27 ms, battle 0.70–0.83 ms per frame. The
minimap dots are one MultiMesh buffer filled by the bridge; the ground image
is built once (`test_large`: 500 × 500 cells).


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
