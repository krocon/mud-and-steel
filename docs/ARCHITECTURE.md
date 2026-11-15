# Architecture

## Principles

Multiplayer runs on **deterministic lockstep**: all machines simulate the same game and
only exchange commands. This dictates the rules for the whole project:

1. **The simulation is the single source of truth.** It lives in pure Rust
   (`crates/sim`), without Godot, and runs at a fixed tick (20 Hz).
2. **Godot only presents.** Godot physics, `NavigationServer` and Godot randomness are
   off-limits for anything game-relevant, because they are not deterministic.
3. **No floating point in the simulation.** Windows x86 and Apple Silicon must compute
   bit-identical results: fixed-point arithmetic and an own seeded RNG. Conversion to
   `f32` happens only in the bridge.
4. **No hidden nondeterminism in the simulation:** no system time, no iteration over
   `HashMap`/`HashSet`, no threads with undefined ordering.
5. **AI is a normal player** that feeds commands into the same simulation.

Details and enforcement: [DETERMINISM.md](DETERMINISM.md).

## Layers

```
Godot 4 (presentation: 3D scene, UI, audio)        godot/
        ▲▼  commands in, snapshots out
godot-rust bridge (GDExtension, thin)              crates/bridge
        ▲▼
Rust workspace:  sim  |  net  |  cli                crates/sim, crates/net, crates/cli
        ▲▼
Transport (later): Steam online, ENet on LAN
```

| Crate / folder  | Responsibility | Depends on |
|-----------------|----------------|------------|
| `crates/sim`    | Game rules and state. `Sim::new(config, map, seed)`, `step(&[PlayerCommand])`, `state_hash()`, `snapshot()`. Players and unit ownership, group commands with formation, maps (height field, terrain), pathfinding (`sim::path`), fixed-point math, RNG, stable hash, unit catalog, scale, terrain rules and scenarios (compiled in, like the map `test_small`). | `fixed`, `serde`, `toml`, `png` |
| `crates/net`    | Lockstep protocol: `CommandFrame`, `HashReport`, `LockstepSession` (input delay, desync detection), `Transport` trait, `LoopbackTransport` for tests, `LocalTransport` for single player. Replays (`net::replay`). | `sim`, `postcard` |
| `crates/cli`    | Headless runner: `cli run --ticks N --seed S [--record FILE]` prints the state hash, `cli replay FILE` re-simulates a replay and checks its end hash, `cli gen-map` generates test maps, `cli map-info DIR` measures a map, `cli path-bench` benchmarks pathfinding. | `sim`, `net`, `clap`, `png` |
| `crates/bridge` | GDExtension `cdylib`. `SimHost` node: runs ticks from a time accumulator through a `LockstepSession`, queues commands, exposes the last two snapshots and the replay. The only float conversion. | `sim`, `net`, `godot` |
| `godot/`        | Scenes, GDScript UI logic, assets. No C#. | bridge (via `.gdextension`) |
| `data/`         | Unit catalog (`catalog/*.toml`, real values), `scale.toml` (conversion to game values), `terrain.toml` (terrain types, slope rules), scenarios (`scenarios/*.toml`, start setups) and maps (`maps/<name>/`), integers only. See [SCALE.md](SCALE.md) and [MAPS.md](MAPS.md). | – |

## Players, commands and formation

Every unit has an owner (`sim::PlayerId`). A `sim::Command` names a list of units
but never its issuer:

| Command | Effect |
|---|---|
| `MoveTo { units, target }` | The units drive along one planned path to a grid formation around `target` (see [Pathfinding](#pathfinding)). |
| `Stop { units }` | The units stop where they are. |

The simulation consumes `PlayerCommand { player, command }`. Only the lockstep layer
builds these, from the sender of the frame, so a peer cannot command another player's
units by naming them: units that do not exist or belong to someone else are dropped
from the list, duplicates too. Move targets beyond ±1000 km are ignored, which keeps
all position arithmetic far from `Fixed` overflow even for forged commands.

**Why a unit list instead of one command per unit:** one order of the player stays
one command. The formation needs the whole group at once (slot count, spacing, slot
assignment), the frames stay small, and later commands (attack, follow) have the same
shape. **Why the player is not in the command:** it cannot be forged and needs no
check; the frame already carries the sender.

**Formation:** a square grid with `ceil(sqrt(n))` columns, every row (also a shorter
last one) centered on the target, rows along +y. The spacing is 1.5 × the largest
footprint of the group (`UnitType::footprint_m` = larger of catalog length and width,
1 m for entries without dimensions), so a Panzer IV group stands 10.5 m apart. The
grid is axis-aligned. Units are assigned to slots in the order of their current
position (y, then x, then id), slots in the same order, which avoids crossing paths
in the common cases and makes the result independent of the order of ids in the
command. Facing-aware formations (and a slot assignment along the direction of
travel) are still open; see [Pathfinding](#pathfinding).

**Scenarios** (`data/scenarios/*.toml`) name a map and place groups of catalog units
per player in a grid. The compiled-in demo (`sim::scenario::demo_config`) has eight
Panzer IV H (player 0) against eight T-34/76 (player 1), 120 m apart, on the built-in
map `test_small`. Unit ids follow file order.

## Map and movement

The map belongs to the simulation: a height field (integer millimeters on a 2 m grid,
bilinear in `Fixed`) and a terrain raster (one byte per sample), loaded from
`data/maps/<name>/` by the caller and handed to `sim::Map::load` as bytes. Format,
generator and the reasons behind them: [MAPS.md](MAPS.md).

- `SimConfig` references the map by name and content hash (`MapRef`); `Sim::new` gets
  the `Arc<Map>` and rejects a different one. Replays store only the reference.
- Units stand on the ground: their height is part of the snapshot and of the state hash.
- Speed per tick = catalog speed (road or cross-country, chosen by the terrain type)
  × terrain percentage × slope factor (uphill grade over the next meter), never more
  than the road speed. Rules: `data/terrain.toml`.
- Units follow paths around impassable terrain (deep water), too steep grades and the
  map edge ([Pathfinding](#pathfinding)). They never enter impassable terrain or leave
  the map; if the way to the next waypoint is blocked anyway (separation pushed them
  off), they slide along the obstacle or plan again.

## Pathfinding

`crates/sim/src/path.rs` (grid, A*, smoothing, request queue) and the movement in
`crates/sim/src/sim.rs`. Everything is integer or `Fixed`, sequential, and part of
the state hash where it is state (routes, queue, grid version).

### Path grid

The map is covered by **path cells of 4 m**, each exactly 2 × 2 map samples (on a
map with 2 m cells; generally `4000 / cell_size_mm` samples per axis). A sample owns
the 2 m square around it (`Map::terrain_at` uses the nearest sample), so a path cell
covers exactly the squares of its four samples, and the grid starts half a sample
before the map edge. Per cell:

- **Flags:** impassable terrain in its own samples; too steep (largest
  `|dx| + |dy|` height difference of the height patches it overlaps above
  `max_grade_pct`, an upper bound of the gradient, so the movement rule never meets
  a grade the grid let through); obstacle ([`set_obstacle`](#changing-obstacles));
  margin (impassable terrain or the map edge within one sample around the cell).
- **Size classes:** `Small` (footprint up to 4 m: infantry) is blocked by the first
  three flags, `Large` (tanks, trucks) also by the margin, so a tank's center stays
  at least 2 m from deep water. Passages a unit class can use: about 4 m (one
  aligned cell) for infantry, about 8 m for tanks, plus up to one cell of alignment.
- **Cost per unit type**, `u16`, 0 = blocked: the mean crossing time of the four
  samples relative to the type's fastest terrain (terrain speed exactly as the
  movement uses it: road or cross-country basis, percentage, capped at the road
  speed), times a slope factor (mean of uphill `1 / factor` and downhill 1, from
  `terrain.toml`). The cheapest cell costs `MIN_CELL_COST` = 100. A Panzer IV (38 vs
  16 km/h) pays 100 on roads and about 240 on grass; infantry (5 vs 4 km/h) 100
  and 125.
- **Components** per class (4-neighborhood flood fill): a target in another
  component (a lake, behind a river without a ford) is moved to the nearest cell
  of the start's component before the search, so every search succeeds.

`test_large` (1000 × 1000 cells, two unit types) builds in about 170 ms (release;
part of `Sim::new`), `test_small` in 3 ms. Memory: 13 bytes per cell (flags, slope, two classes), 2 bytes per cell and unit
type, 12 bytes per cell of search workspace: 29 MB for `test_large`.

### Search

- **A\*** over the 8 neighbors without corner cutting. Step cost = mean of both
  cells' costs × 10 (straight) or 14 (diagonal), integer.
- **Heuristic:** octile distance × `MIN_CELL_COST`, admissible and consistent, so
  paths are optimal and closed nodes are never reopened.
- **Tie-breaking:** the open list is a `BinaryHeap` of `(f, h, cell)`: a total order
  (prefers nodes nearer the goal, then the lower cell index), so the pop order never
  depends on how the heap arranges equal costs. Neighbors are visited in a fixed
  order and only strictly better costs replace a node.
- **Smoothing:** greedy line of sight over the raw cells (at most 32 cells ahead).
  A shortcut is taken only if every touched cell is passable and the straight
  line (mean cost of the touched cells × length) is not more expensive than the
  skipped part of the raw path measured the same way. Units stay on roads and
  off mud that the search avoided. The line walk is an exact integer grid
  traversal; passing exactly through a cell corner needs both side cells.

### Request queue and budget

Path requests go into a FIFO queue. Every tick, after the commands and before
movement, the queue gets **32 768 node expansions** (`NODE_BUDGET_PER_TICK`);
starting a request costs one. A search that runs out of budget continues next
tick (one search is active at a time, with a reused workspace). Units **wait**
(they keep their target, `RouteView::Waiting`) until their path is there. Requests
nobody waits for any more (new order, `Stop`) are dropped without searching.

### Groups and formation

`MoveTo` assigns formation slots as before, then queues **one request for the
group**: from the unit nearest to the group's center, with the profile of the
largest unit (its class fits everybody). When the path is found, every member
follows it **shifted by its formation offset** (slot minus target), checked by line
of sight in its own class's grid:

- a shifted waypoint that cannot be reached in a straight line is replaced by the
  unshifted one (the group squeezes through a gap);
- a slot in blocked terrain moves to the nearest passable cell visible from the
  group's end point;
- waypoints of the group path within 8 m of its start are skipped by members that
  see the next one (they only re-form the group);
- only if all of that fails does the member get its own request.

On open ground a group needs exactly one search; in the benchmark below, 25 group
orders needed 42 requests including re-plans.

### Movement and separation

Each tick, a unit drives towards its current waypoint at the terrain speed (as
before). **Separation**, computed from the positions at the start of the movement
phase (order-independent):

- **push** away from every unit closer than half of both footprints, linear up to one
  unit length at full overlap;
- **swerve** sideways from units ahead within twice that distance and less than it
  off the own line, away from the side they are on (straight ahead: to the left),
  so two units meeting head-on pass each other.

The push deflects the step but never makes it longer than the speed. It is ignored
within one footprint of the final waypoint, so a parked unit cannot keep another
from arriving, and pushed steps must stay on cells passable for the unit's class.

Rules that keep units from getting stuck: intermediate waypoints count as reached
within 1 m; a unit that has not come closer to its waypoint for 2 s (40 ticks)
skips it, or stops on the last one; if neither the pushed, the direct nor a
sliding step (x or y part only) is possible, it plans a new path from where it is,
at most 3 times until it reaches the next waypoint.

### Changing obstacles

`Sim::set_path_obstacle(a, b, blocked)` marks the cells of a rectangle as blocked
(or frees them), recomputes flags, costs and components of that area and its
neighbors (components for the whole map),
restarts the running search and re-plans every unit whose remaining route touches
the changed cells. It is meant for game systems (buildings, wrecks, destruction in
prompt 020), not for player commands; nothing calls it yet.

### Decisions

| Question | Decision | Why |
|---|---|---|
| A* or flow fields | A* per group | Orders go to groups of a few units with individual targets; a flow field costs a full-map Dijkstra (1 M cells) per target and only pays off for many units with one target. A* with a node budget has bounded cost per tick and easy determinism. Flow fields can be added later for mass orders. |
| Cell size | 4 m = 2 × 2 samples | 1 M nodes and 12 MB of search state on 4 km instead of 4 M nodes and 48 MB at 2 m; about as wide as a tank. Exact sample ownership (no samples shared between cells) makes blocking exact: the first version with shared edge samples closed every ford of the test maps. |
| Size classes | Small without margin, Large with one sample (2 m) margin | A margin of a whole 4 m cell would need 12 m wide passages for tanks (bridges, prompt 014); one sample keeps tanks off the water's edge and passes 8 m. |
| Heuristic | Admissible (octile × cheapest cell) | Measured: weighting the heuristic towards grass cost (4 × fewer expansions) loses roads that are not on the straight line, which is the whole point of road preference. Optimal paths, slower search; the budget pays for it. Switch: `HEURISTIC_WEIGHT_PCT`. |
| Budget | 32 768 expansions per tick | About 4 ms per tick in release (M1 Max): 8 % of a tick. A group order across `test_large` (177 000 expansions on average) takes about 6 ticks, short orders one. |
| Unreachable targets | Nearest cell of the start's component | The unit gets as close as possible instead of stopping at the first obstacle, and no search ever explores a whole region in vain. |
| Separation | Push + swerve, no avoidance planning | Simple, deterministic, enough to pass each other; crowding is handled by the stall and re-plan rules. Proper local avoidance (RVO) is much more code. |
| Debug builds | `sim` with `opt-level = 3` in the dev profile | The Godot bridge and the tests run debug builds, and unoptimized A* would eat the frame; overflow checks and debug assertions stay on, so integer behavior is unchanged. |

### Benchmark

`just path-bench` (`cli path-bench`): 200 units on `test_large` (half Panzer IV H,
half rifle squads, 10 m grid around the center), every order to a random point
within ±1800 m, all in tick 0. Apple M1 Max, release build, other sessions busy:

| Orders | Requests | Expansions per request | All paths after | Time per tick while planning (mean / p99 / max) | All arrived |
|---|---|---|---|---|---|
| 200 × 1 unit | 200 | 136 000 | 834 ticks (42 s) | 4.1 / 5.4 / 5.8 ms | 1449 s |
| 25 × 8 units | 42 | 177 000 | 219 ticks (11 s) | 4.0 / 5.6 / 5.8 ms | 1433 s |

Ticks without path work take 0.02–0.15 ms (200 units, separation is O(n²)). The
debug build (`opt-level = 3`) is about 25 % slower. 200 single orders to random
points across a 4 km map at once are the worst case; the latency until the last
path is there grows linearly with the queue. If that matters later: hierarchical
search (clusters with precomputed portals) or a weighted heuristic for long
requests. On `test_small`, two group orders across the river take 2 ticks.

### Open

- Formation facing and slot assignment along the direction of travel (slots are
  assigned by y, then x, as in prompt 009: diagonal moves make members cross
  while the group re-forms).
- Speed matching within a group (members drive at their own terrain speed).
- Obstacles are cell-based and not widened by the margin for large units.

## Data flow (single player, current state)

Single player runs through the same lockstep path as multiplayer:

1. The player selects units and right-clicks the ground. `selection/selection.gd`
   turns the click into a ground point with `scenes/camera/ground_pick.gd`, which
   intersects the mouse ray with the height field (`SimHost.ray_pick`: ray marching
   plus bisection, no Godot physics), and calls `SimHost.move_units(ids, x, z)`.
2. `SimHost` rounds the target to millimeters and queues `Command::MoveTo`.
3. Every frame, `SimHost::process` adds the frame time to an accumulator and runs as many
   fixed 20 Hz ticks as are due (at most 5 per frame). Per tick it submits the queued
   commands to its `LockstepSession` (one player, `LocalTransport`), which schedules
   them `input_delay` ticks ahead (exported property, default 1 tick = 50 ms). The
   session returns the merged `PlayerCommand`s of the current tick; they are recorded
   for the replay and stepped.
4. After each tick, the previous snapshot is kept and a new one is taken.
5. `main.gd` reads both snapshots and `interpolation_alpha()` and draws every unit
   interpolated between them. A sim position `(x, y)` at ground height `h` maps to
   Godot `(x, h, y)`.

`scenes/debug/path_debug.gd` shows the pathfinding: **F4** the cost grid of the
selected unit's type (red blocked, green cheap, yellow to purple expensive), **F5**
the remaining paths of all units (orange while a path is planned);
`-- --path-debug` (or `just run-paths`) starts with both. **F3** opens the general
debug menu (`scenes/debug/debug_menu.gd`).

The terrain is drawn by `scenes/terrain/terrain.gd`; see "Terrain rendering" below.

Player 1 of the demo issues no commands yet; the AI (prompt 024) will join the session
as another player with its own frames.

## Terrain rendering (presentation only)

The simulation owns heights and terrain types; Godot only draws them. Prompt 011
replaced the coarse debug mesh with chunked, LOD-switched grids and a splatting
shader.

### Data path

```
sim::Map (i32 mm heights, u8 terrain ids, 2 m grid)
   │  read-only, Arc shared with the Sim
   ▼
bridge: SimHost.terrain_layers(i0, j0, w, h)          crates/bridge/src/terrain_layers.rs
   │  floats, per rectangle of samples:
   │    height (f32 m) · normal (RG8) · splat0/splat1 (RGBA8 weights of ids 0–7,
   │    3×3 tent filter) · track (RGBA8: direction along field tracks from the
   │    smoothed structure tensor of the track mask, as a doubled angle; mask)
   ▼
terrain.gd: one Image + ImageTexture per raster, one texel per map sample
   │  (test_large: 2001² texels: 16 MB heights + 56 MB other rasters on the GPU,
   │   the same again as Images on the CPU for region updates)
   ▼
terrain.gdshader: vertex reads heights (texelFetch), fragment blends materials
```

`SimHost.terrain_chunk_ranges(cells)` gives the height range of every chunk for
culling. `SimHost.debug_dent(x, z, radius, depth)` adds a presentation-only offset
(debug crater); the simulation, unit heights and mouse picking keep the original
heights until prompt 021 lets the simulation change the map.

### Chunks and LOD

- Chunks of 64 × 64 cells (128 m); `test_large` has 32 × 32. All chunks of a LOD
  level share one flat grid mesh (64, 32, 16, 8 or 4 quads per side, i.e. every
  1st to 16th sample) and are drawn as instances of one `MultiMesh` per level:
  five draw calls for the whole terrain, plus the shadow passes.
- Every frame in which the camera moved, `terrain.gd` gives each chunk a level from
  the distance between camera and chunk box (full detail up to 140 m, one level
  less per doubling of the distance), drops chunks outside the camera frustum
  (grown by 40 m so hills just off screen still cast shadows) and rewrites the
  instance buffers (about 1.2 ms in GDScript on `test_large`).
- Every instance carries the strides of its four neighbors (`INSTANCE_CUSTOM`). An
  edge next to a coarser chunk is snapped onto the coarser edge in the vertex
  shader, so no cracks appear between levels.
- Vertices sit exactly on map samples, so the drawn ground meets the simulation's
  heights at every sample (checked in the self-test). Between samples the mesh
  interpolates by triangles and the simulation bilinearly; the difference is a few
  centimeters on the gentle test maps. Normals come from `normal_tex` per pixel, so
  lighting does not change with the mesh LOD.

### Splatting shader

`assets/materials/terrain.gdshader` blends ten material layers from two texture
arrays (`assets/textures/terrain/`, generated by `just terrain-textures`):
terrain ids 0–7 map to the layer with the same index (grass, field track, road
gravel, sand, mud, snow, river pebbles, river silt), plus rock and bare earth as
purely optical layers.

- **Weights:** the splat rasters (soft at borders), read through a slightly warped
  coordinate so borders do not follow the 2 m grid; bare earth on slopes above
  about 20–34 % grade and in a few worn patches, rock above about 45 %.
- **Height blending:** each layer's height (albedo alpha) shifts its weight, so
  grass grows into the edges of a track instead of fading.
- **Against tiling:** two samples per layer (one rotated and scaled, and much larger
  in the distance), blended by noise; low-frequency macro noise varies brightness
  and dry/lush tint across hundreds of meters.
- **Field tracks:** the track layer is sampled in a frame turned along the track
  direction from the `track` raster, so ruts follow curves.
- **Water:** river beds are darkened and made glossy as a stand-in until prompt 014
  draws water surfaces.

The map ends in a vertical cut (`terrain_edge.gdshader`, soil layers) down to 12 m
below the lowest point, like a diorama block.

### Region updates (craters, prompt 021)

`terrain.heights_changed(rect)` re-reads a rectangle of samples (grown by 8 samples
for normals and filters) through `terrain_layers`, blits it into the images,
uploads the textures and refreshes the chunk height ranges and, at the map border,
the edge cut. No mesh is rebuilt: chunks only reference the textures. The debug
menu (F3) has "Test crater at focus", which goes through the same path.

### Why our own renderer and not Terrain3D (phase 0)

Terrain3D 1.1 (MIT, Godot 4.5–4.8, macOS/Linux/Windows, actively maintained) would
fit technically: clipmap LOD, splatting, runtime height edits through its data API.
We still chose our own solution:

| Criterion | Terrain3D | Own solution |
|---|---|---|
| Effort | Plugin installation, importing sim data into its regions and control maps, learning its shader to change the look | About 600 lines (shader, GDScript, Rust rasters); the map size is fixed and known, no streaming needed |
| Craters at runtime | Possible (`set_height`, `update_maps`) | One rectangle re-read and uploaded, same path as the start |
| Control over the look | Control map holds two textures per pixel (base and overlay); ruts along tracks would need a custom shader anyway | Any number of layers per pixel, track-aligned ruts, own anti-tiling |
| Dependencies | A second binary GDExtension whose version must follow Godot (next to godot-rust) | None beyond Godot |
| Tools we do not need | Sculpting, painting, collision (Godot physics) | – |

What we give up: Terrain3D's instancer for vegetation (prompt 013 will use
`MultiMesh` directly) and its editor tools (the map editor of prompt 028 edits sim
data anyway).

### Measurements (`test_large`, 1920 × 1080, Apple M1 Max, V-Sync off)

`just terrain-bench`, slow pan in each view, debug bridge:

| View | FPS | Draw calls (total / terrain only) | Triangles | Chunks per LOD 0–4 |
|---|---|---|---|---|
| Near (15 m) | ~270 | 30 | 0.31 M | 5 / 2 / 0 / 0 / 0 |
| Middle (120 m) | ~265 | 30 | 0.38 M | 6 / 5 / 0 / 0 / 0 |
| Overview (600 m) | ~235 (with 16 tanks) | 849 / 30 | 0.45 M / 0.12 M | 0 / 0 / 8 / 26 / 0 |

The frame rate barely changes between views: the terrain is far from limiting.
The target (60 FPS in the overview at 1080p) is met with a wide margin. The 849
draw calls in the overview come from the tank models (many parts times shadow
cascades), not from the terrain. Building the terrain at game start takes about
0.65 s on `test_large` (the bridge is compiled with `opt-level = 2` even in debug
builds for this).

`just terrain-shots <dir>` records fixed views (near, middle, overview, hill, river,
village, map edge, LOD tint, test craters) with `--write-movie` for image
comparisons. Godot skips rendering while its window cannot draw (covered, other
Space); benchmark and shots then render with `RenderingServer.force_draw` and the
benchmark reports how many frames that affected.

## Selection (presentation only)

Selection is local UI state in `godot/scenes/selection/selection.gd`; only the
resulting orders reach the simulation. The bridge exposes per-unit packed arrays in
the same order as `unit_ids()`: `unit_owners()`, `unit_types()`, `unit_footprints()`,
`unit_moving()`, plus `local_player()`. Only units of the local player are selectable.

| Input | Effect |
|---|---|
| Left click | Select the own unit under the cursor; on empty ground clear the selection |
| Shift + left click | Add the unit (or remove it if already selected) |
| Left drag | Selection box (own units whose hull center is inside) |
| Double click | All visible own units of the same type |
| Ctrl+1…9 (Cmd on macOS) | Store the selection as a group |
| 1…9 | Recall the group; twice within 0.4 s also centers the camera on it |
| Right click | Move the selection to the ground point (green marker for 1 s) |
| H | Stop the selection |

Picking projects unit positions to the screen (`Camera3D.unproject_position`); the pick
radius is half the footprint on screen, at least 14 px. Selected units show a green
ring; units of other players carry a red tint overlay.

## Replays

The game writes the match to `user://replays/<date-time>.msreplay` when the main scene
exits (macOS: `~/Library/Application Support/Godot/app_userdata/Mud and Steel/replays/`).
`just replay <file>` re-simulates it headless and reports whether the end hash matches.
`just record <file>` writes a replay of the scripted demo without Godot.

Format (`crates/net/src/replay.rs`):

```
"MSRP"  4 bytes magic
u16 LE  format version (currently 2)
postcard body: seed, resolved SimConfig, lockstep players, tick count,
               frames (tick + merged PlayerCommands, only non-empty ticks), end hash
```

Decisions:

- **Header outside postcard.** postcard is not self-describing, so a reader must know
  the version before decoding. A fixed magic and version let old builds reject new
  files with a clear message instead of garbage.
- **Resolved config instead of a scenario name.** The replay carries the unit types as
  the simulation used them (already converted from the catalog). Catalog or scenario
  edits do not invalidate old replays; changes of the simulation rules do, and then
  playback reports the end-hash mismatch instead of silently showing a different game.
  Terrain rules are part of the config, too.
- **Map by reference.** The map (20 MB for 4 km) is not copied into the replay; the
  config names it with its content hash. `just replay` loads `data/maps/<name>/` (or
  the built-in `test_small`) and refuses a map whose hash differs. Version 2 (prompt
  010) added map reference and terrain rules.
- **Merged commands per tick, sparse.** The replay stores exactly what `Sim::step` got
  (player-tagged, in apply order), so playback needs no lockstep session and does not
  depend on input delay. Ticks without commands are omitted; the tick count marks the
  end. An hour of play with a few orders per second stays well below a megabyte.
- **Only the end hash.** It proves the whole run; intermediate hashes to locate a
  divergence can be added with a version bump.

## Camera rig (presentation only)

`godot/scenes/camera/rts_camera.tscn` is a `Node3D` that sits on a focus point on the
ground, with a perspective `Camera3D` (30° field of view) as its child. The camera is
placed from four values: focus, distance (15–600 m), pitch and yaw.

- **Pitch follows the zoom:** 45° above the horizon at the closest distance, 65° at the
  farthest, blended over a logarithmic zoom level (optional `Curve`, else smoothstep).
- **Zoom** (mouse wheel, trackpad pinch) is smoothed in log space and keeps the ground
  point under the cursor fixed. **Pan** (WASD, arrows, window edge) scales with the
  distance; middle-mouse drag grabs the ground. **Q/E** turn in 45° steps, **N** turns
  back to north (−Z).
- The focus is clamped to `bounds`, set by `main.gd` from `SimHost.map_size()`, and
  sits on the ground height (`GroundPick.height_at`).
- Near/far planes and the sun's shadow distance scale with the camera distance.
- All bindings are `camera_*` actions in the input map in `project.godot`.
- `ground_pick.gd` holds the screen-to-ground math used by the camera and by mouse
  commands. Once `main.gd` sets `GroundPick.height_source` to the SimHost, rays hit
  the simulation's height field (`SimHost.ray_pick`); without it, the plane y = 0.

Perspective was chosen over orthographic: depth and height read better, the view gets
the slightly perspective look of the style references, and zoom works by moving the
camera, which makes pitch changes and distance-based shadows and LOD natural. The small
field of view keeps distortion low.

## Data flow (multiplayer, planned)

Each peer runs the per-tick protocol of `net::LockstepSession` (single player already
does, with one player):

1. Submit local commands for tick `t + input_delay` and send them to all peers (an empty
   frame is also sent).
2. Receive frames of other players.
3. Once the frames of all players for tick `t` are present, merge them in player order,
   step the simulation, and every `hash_interval` ticks send the state hash.
4. A mismatching hash is reported as a `Desync`.

`crates/net/tests/loopback.rs` plays the demo scenario this way between two peers,
each controlling its own eight tanks with group commands (and trying to command the
other's), and compares the hashes of every tick. AI players will produce commands
through the same path.

## Transport (planned, not built yet)

The `Transport` trait only sends and receives bytes, so the protocol does not know how
packets travel.

- **Online:** Steam networking through [GodotSteam](https://godotsteam.com/)
  (GDExtension). Lobbies, matchmaking and NAT traversal come from Steam. Requires a
  Steam App ID.
- **LAN:** ENet over UDP. Games are discovered with a UDP broadcast on the local
  network; joining then uses a direct ENet connection.

Both must deliver reliably and in order per peer, as `Transport` requires.

## Asset pipeline

Folders, scale, orientation, part names, budgets and material slots:
[ASSETS.md](ASSETS.md).
