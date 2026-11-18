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

Details and enforcement: [DETERMINISM.md](DETERMINISM.md). Budgets, the overall
benchmark and measurements: [PERFORMANCE.md](PERFORMANCE.md).

## Layers

```
Godot 4 (presentation: 3D scene, UI, audio)        godot/
        ▲▼  commands in, snapshots out
godot-rust bridge (GDExtension, thin)              crates/bridge
        ▲▼
Rust workspace:  sim  |  net  |  steam  |  cli      crates/sim, crates/net, crates/steam, crates/cli
        ▲▼
Transport: lobby protocol (net::lobby) over ENet on the LAN (net::lan) or
           Steam Networking Messages online (crates/steam)
```

| Crate / folder  | Responsibility | Depends on |
|-----------------|----------------|------------|
| `crates/sim`    | Game rules and state. `Sim::new(config, map, seed)`, `step(&[PlayerCommand])`, `state_hash()`, `snapshot()`. Players and unit ownership, group commands with formation, maps (height field, terrain, objects, bridges), pathfinding (`sim::path`), combat (`sim::combat`), destruction of map objects (`sim::destruction`), infantry squads with soldiers, stances, cover and buildings (`sim::infantry`, prompt 022), fixed-point math, RNG, stable hash, unit catalog, scale, terrain rules and scenarios (compiled in, like the map `test_small`). | `fixed`, `serde`, `toml`, `png` |
| `crates/net`    | Lockstep protocol: `CommandFrame`, `HashReport`, `LockstepSession` (input delay, also changed at run time, desync detection), `Transport` trait, `LoopbackTransport` for tests, `LocalTransport` for single player. Replays (`net::replay`), snapshots and the rules hash (`net::snapshot`), saved games (`net::save`, prompt 038). Network games: `net::lobby::LobbyNode` (lobby protocol, then the match transport) over a `Link`; LAN games (`net::lan`): ENet link, discovery by UDP broadcast; `MemoryLink` for tests. | `sim`, `postcard`, `miniz_oxide`, `rusty_enet`, `if-addrs` |
| `crates/steam`  | Online over Steam (prompt 026, [ONLINE.md](ONLINE.md)): the Steam client (`Steam`, app id `steam::APP_ID`), `SteamLink` over Steam Networking Messages, `OnlineLobby` (create, list, join, invite, rich presence). Optional: only the bridge (feature `steam`, default) and the CLI (feature `steam`) use it. | `net`, `steamworks` |
| `crates/cli`    | Headless runner: `cli run --ticks N --seed S [--record FILE]` prints the state hash, `cli replay FILE` re-simulates a replay and checks its end hash, `cli save-check FILE` checks a saved game, `cli save-bench` measures loading (prompt 038), `cli gen-map` generates test maps, `cli map-info DIR` measures a map, `cli path-bench` benchmarks pathfinding, `cli map-check DIR` validates a map like the editor, `cli lan-host` / `cli lan-join` play the demo as headless LAN peers, `cli steam-host` / `cli steam-join` over Steam (feature `steam`). | `sim`, `net`, `clap`, `png`, (`steam`) |
| `crates/ai`     | Computer players (prompt 024): `Ai::think(&PlayerView) -> Vec<Command>`, the fog-of-war `PlayerView`, difficulties, `game::Game` for headless AI matches. Runs on one machine as a hosted player of the lockstep session, not inside the simulation. See [AI.md](AI.md). | `sim`, `net` |
| `crates/editor` | Map editing (prompt 028): `EditMap` with integer brushes, undo per stroke, saving in the map format and validation through the simulation's loader and path finder. A tool like `cli gen-map`, not game rules. | `sim`, `png`, `toml` |
| `crates/bridge` | GDExtension `cdylib`. `SimHost` node: runs ticks from a time accumulator through a `LockstepSession`, queues commands, exposes the last two snapshots and the replay; plays single player over `LocalTransport`, a LAN match (`SimHost.start_lan`) or an online match (`SimHost.start_online`). `LanLobby` drives the LAN lobby, `SteamLobby` the online lobby (always registered; without the `steam` feature it reports that). `MapEditor` hosts the map editor. `ReplayPlayer` holds an opened replay and its snapshots (prompt 038). The only float conversion. | `sim`, `net`, `editor`, `godot`, (`steam`) |
| `godot/`        | Scenes, GDScript UI logic, assets. No C#. | bridge (via `.gdextension`) |
| `data/vehicles/` | Vehicle database (prompt 064): technical data of the armored vehicles of the reference material as JSON, far more fields than the catalog (armor of every assembly and face with angle, traverse, engine, gearbox, ground pressure, fording, production). A reference work next to the catalog, never input of a rule: `crates/sim` does not read it, so it cannot change the golden hash. Compiled into the bridge (`SimHost.vehicle_record(type_id)`, `crates/bridge/src/vehicle_funcs.rs`) and read by the unit page. Written by `just vehicle-db`, checked against the catalog by `just vehicle-db-check`. See `data/vehicles/SCHEMA.md`. | – |
| `data/`         | Unit catalog (`catalog/*.toml`, real values), `scale.toml` (conversion to game values), `terrain.toml` (terrain types, slope rules), `combat.toml` (combat rules), `infantry.toml` (stances, cover, buildings), scenarios (`scenarios/*.toml`, start setups) and maps (`maps/<name>/`), integers only. See [SCALE.md](SCALE.md) and [MAPS.md](MAPS.md). | – |

## Players, commands and formation

Every unit has an owner (`sim::PlayerId`). A `sim::Command` names a list of units
but never its issuer:

| Command | Effect |
|---|---|
| `MoveTo { units, target }` | The units drive along one planned path to a grid formation around `target` (see [Pathfinding](#pathfinding)). |
| `Stop { units }` | The units stop where they are (and drop an attack order). |
| `Attack { units, target }` | The units drive into range of the unit `target` and fire at it until it is destroyed ([Combat](#combat)). Own targets are ignored. |
| `AttackGround { units, target }` | The units drive into range of the ground point and fire at it. |
| `SetFireMode { units, mode }` | `FireAtWill` (default: pick targets automatically) or `HoldFire` (fire only at attack targets). |
| `Surrender` | The issuing player gives up and is defeated ([Victory conditions](#victory-conditions-and-game-flow)). |
| `SetStance { units, stance }` | Infantry squads stand, kneel, go prone or choose automatically (prompt 022, [INFANTRY.md](INFANTRY.md#behavior-prompt-022)). |
| `Garrison { units, building }` | Infantry squads walk to the building (map object index) and occupy it if there is room and no enemy inside; a `MoveTo` leaves it. |
| `Build { unit_type, position }` | Places and pays a construction site of a structure type ([Economy](#economy-base-building-and-production), prompt 023). |
| `Produce { building, unit_type }` | Queues and pays a unit in an own completed producing building. |
| `CancelProduction { building }` | Removes the last order of the queue with a full refund. |
| `SetRallyPoint { building, point }` | Units of the building drive there when they are done. |
| `QueueMove { units, target }` | Appends a waypoint: a formation slot around `target` per unit, driven to after the current order ([Waypoint chains](#waypoint-chains-and-patrols), prompt 031). |
| `Patrol { units, target }` | Like `MoveTo`, then back and forth between the start and the target. |
| `CallAir { mission, target }` | Calls air support of the player's faction on the ground point ([Air support](#air-support-and-anti-aircraft-fire), prompt 033). |
| `FormationMove { units, target, formation, facing, engage }` | Drives the group to a formation around `target`, turned to `facing` (default: direction of travel); `engage` picks who halts to fight on the way (attack-move) ([Formations](#formations-attack-move-and-securing), prompt 046). |
| `Secure { units, center, radius_m, formation, facing }` | Takes up the formation around `center` and holds the circle: attack enemies inside, return to the slots. |
| `MissionSignal { name }` | Reports what only the presentation knows (camera moved, units selected) to the mission script; ignored without a mission and from other players ([MISSIONS.md](MISSIONS.md), prompt 039). |

Movement commands (`MoveTo`, `Attack`, `AttackGround`, `Garrison`,
`QueueMove`, `Patrol`) ignore units that cannot move (buildings).

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
command. Facing-aware formations with a slot assignment along the direction of
travel came with prompt 046 (`FormationMove`, [below](#formations-attack-move-and-securing));
`MoveTo` keeps the grid.

**Scenarios** (`data/scenarios/*.toml`) name a map and place groups of catalog units
per player in a grid. The compiled-in demo (`sim::scenario::demo_config`) has eight
Panzer IV H and two rifle squads (player 0) against eight T-34/76 and two rifle
squads (player 1), the tanks 120 m apart, on the built-in map `test_small`. Unit ids
follow file order (the squads, appended in prompt 022, have ids 16–19).

### Waypoint chains and patrols

Prompt 031, `crates/sim/src/sim/orders.rs`. Every unit has a queue of move
targets (formation slots), at most `MAX_QUEUED` (64).

- `QueueMove` assigns slots around the waypoint exactly like `MoveTo` (grid,
  units in the order of their position) and appends each unit's slot. If all
  units of the order are idle and have empty queues, it is a plain `MoveTo`
  (one group path).
- Right after the commands of a tick, every living unit without target,
  route and attack order takes the next waypoint and plans its **own** path
  there (a group does not share paths along the chain: its members reach a
  waypoint in different ticks). A finished attack order therefore continues
  the chain.
- `Patrol` is a `MoveTo` to the target; the queue then holds the start and
  the slot at the target and cycles (a taken waypoint goes back to the end).
- `MoveTo`, `Stop`, `Attack`, `AttackGround`, `Garrison`, `Barrage`
  (prompt 032) and `Patrol` drop the queues of their units; destroyed units
  lose theirs. A unit with an indirect fire order is not idle: the chain
  waits until the barrage is over.
- **State hash:** only units with a queue (or a patrol) are written, after all
  other state (unit id, patrol flag, waypoints). Matches without chains keep
  their hash, so the golden hash did not change.
- `Sim::queued_waypoints(id)` serves the presentation (`SimHost.unit_waypoints`,
  `unit_queue_length`, `unit_patrols`).

| Question | Decision | Why |
|---|---|---|
| New command or `queue` flag | Two new variants at the end of `Command` | `MoveTo` and old replays stay unchanged (postcard numbers variants in order). |
| Paths along the chain | One per unit and leg | A shared group path would need the members to wait for each other; short legs cost little search budget. |
| Formation choice, facing by right-drag | Done in prompt 046 as new commands | `MoveTo` keeps its grid, so the golden hash did not change. |

### Formations, attack-move and securing

Prompt 046, `crates/sim/src/formation.rs` (shapes) and
`crates/sim/src/sim/drill.rs` (orders); decisions with reasons in
`prompts/done/046-befehle-formationen.md`.

- **Shapes** (`Formation`): line (at most 8 per rank), wedge (point and two
  ranks, a platoon of 5; more follow in ranks of 5), column (two files from 9
  units), broad wedge (inverted), dispersed (staggered, twice the gap). Laid
  out in units of the spacing (1.5 × largest footprint, as `MoveTo`) across
  and along the facing, then centered: the mean of the slots is the target.
  Slots are filled rank by rank from the front; the units most forward along
  the facing take the front rank, within a rank from left to right.
- **Auto** is resolved once, when the command is applied: free width across
  the facing at the target below 3 slots (for the group's most restrictive
  size class) → column; 30 % or more of the area around the target blocked
  for tanks by objects → dispersed; target on a road and half the way on
  roads → column; else wedge (with vehicles) or line (infantry only).
  [provisional] thresholds.
- **Attack-move** (`engage`: `Never`, `Infantry` = right-click default,
  `All`): a unit with the task halts while it has an automatic target (an
  enemy in range it sees and can hurt) and then plans its own path to its
  slot again. Hold fire never halts. Runs right after `aim_and_fire`.
  Prompt 047: squads of two fire teams bound instead of halting, and mortar
  teams halt only for targets of their mortar (INFANTRY.md "Tactics").
- **Securing**: every `retarget_ticks` a unit without a target attacks the
  nearest enemy its player sees inside the circle that it can hurt (an
  ordinary attack order, so search and sight rules apply); the attack is
  dropped once the target leaves the circle, and the unit returns to its
  slot.
- **Facing:** a standing unit without a fight turns its hull to the facing
  of the order (turret speed rules: `hull_step`); squads turn their soldier
  formation with it.
- Other orders drop task and facing; a waypoint (`QueueMove`) only ends
  securing, and a queue continues after an attack-move (`idle` waits for it).
- **State:** `Unit::drill` (serde, so in saved games); hashed after all other
  state only for units that have a task or a facing, so the golden hash did
  not change. `Sim::drill_view` and `Sim::auto_formation_for` serve the HUD
  (`SimHost.unit_drill`, `auto_formation`).

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
- **Bridges** (prompt 014, [MAPS.md](MAPS.md#bridges)): on an intact deck,
  `Map::height_at` and `Map::terrain_at` report the deck, so units drive over
  bridges with the same movement code. They get on or off a deck only over its
  ends or where the ground beside it is about as high (railings), and grades
  steeper than `max_grade_pct` stop them downhill as well as uphill; no single
  step (also a pushed or sliding one) may be steeper than that, so separation
  cannot push a unit onto an abutment wall or drop it into the river. The path
  grid matches: steps between a deck cell and a cell beside it (across the
  axis) are not allowed in A*, line-of-sight tests and components, except out
  of a blocked start cell.

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
  margin (impassable terrain or the map edge within one sample around the cell);
  map object small / large (a [map object](MAPS.md#objects) whose ground shape
  overlaps the open cell square blocks that class: circles by the nearest point,
  rotated boxes by a separating-axis test, in `Fixed` with an integer sine table).
- **Decks** (prompt 014): a cell whose center lies on the deck of an intact
  [bridge](MAPS.md#bridges) is open whatever lies below (terrain, steepness and
  margin flags cleared), costs the deck terrain and the deck's grade, and is
  closed (cost 0) for unit types heavier than the bridge's load limit
  (`UnitType::mass_kg`). Samples under a deck do not count as impassable for
  the margin of the neighbor cells. Only cells within two cells of a deck are
  checked, so maps without bridges build as fast as before.
- **Size classes:** `Small` (footprint up to 4 m: infantry) is blocked by the first
  three flags and objects that block small units, `Large` (tanks, trucks) also by
  the margin and objects that block large units (trees, hedges, walls), so a tank's
  center stays at least 2 m from deep water and out of woods. Passages a unit class can use: about 4 m (one
  aligned cell) for infantry, about 8 m for tanks, plus up to one cell of alignment.
- **Cost per unit type**, `u16`, 0 = blocked: the mean crossing time of the four
  samples relative to the type's fastest terrain (terrain speed exactly as the
  movement uses it: road or cross-country basis, percentage, capped at the road
  speed), times a slope factor (mean of uphill `1 / factor` and downhill 1, from
  `terrain.toml`). The cheapest cell costs `MIN_CELL_COST` = 100. A Panzer IV (38 vs
  16 km/h) pays 100 on roads and about 240 on grass; infantry (5 vs 4 km/h) 100
  and 125.
- **Components** per class and load tier (4-neighborhood flood fill over the
  cells a profile may enter): a target in another component (a lake, behind a
  river without a ford, behind a bridge too weak for the unit) is moved to the
  nearest cell of the start's component before the search, so every search
  succeeds. The load tier of a unit type is the number of distinct bridge load
  limits of the map below its mass; types with the same class and tier share
  one component grid (on maps without bridges: one per class, as before).

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
most restrictive unit (size class, then load tier, then size), which fits
everybody: a group with a Panzer IV avoids timber bridges. When the path is found, every member
follows it **shifted by its formation offset** (slot minus target), checked by line
of sight in its own class's grid:

- a shifted waypoint that cannot be reached in a straight line (checked with the
  member's own profile, bridges included) is replaced by the unshifted one (the
  group squeezes through a gap or over a bridge);
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
neighbors (components locally, within 32 m of the change; a region that
breaks off is split by flood fills from all its pieces; only a merge of two
regions relabels the whole map, prompt 016),
restarts the running search and re-plans every unit whose remaining route touches
the changed cells. It is meant for game systems (buildings, wrecks, destruction in
prompt 020), not for player commands. Wrecks use it; destroyed map objects
re-rasterize their cells instead ([Destruction](#destruction)).
Craters rebuild the cells under them and re-plan routes only when
passability changed ([Terrain deformation](#terrain-deformation)).

`Sim::set_bridge_intact(bridge, intact)` (prompt 014) works the same way for a
bridge: the map is copied once (copy-on-write through `Arc::make_mut`; the
copy has the deck removed), the cells within two cells of the deck are
rebuilt, and units whose route touches them plan again.

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
| Bridges in the grid | Deck cells by their center, load limit per unit type, components per (class, load tier) | One lane of cells along the axis is exact and cheap; a separate grid per unit type would multiply memory, while tiers give only as many component grids as there are distinct load limits. Diagonal bridges would need corner cutting on decks; the generator builds them along the grid. |
| Debug builds | `sim` with `opt-level = 3` in the dev profile | The Godot bridge and the tests run debug builds, and unoptimized A* would eat the frame; overflow checks and debug assertions stay on, so integer behavior is unchanged. |

### Benchmark

`just path-bench` (`cli path-bench`): 200 units on `test_large` (half Panzer IV H,
half rifle squads, 10 m grid around the center), every order to a random point
within ±1800 m, all in tick 0. Apple M1 Max, release build, other sessions busy:

| Orders | Requests | Expansions per request | All paths after | Time per tick while planning (mean / p99 / max) | All arrived |
|---|---|---|---|---|---|
| 200 × 1 unit | 200 | 136 000 | 834 ticks (42 s) | 4.1 / 5.4 / 5.8 ms | 1449 s |
| 25 × 8 units | 42 | 177 000 | 219 ticks (11 s) | 4.0 / 5.6 / 5.8 ms | 1433 s |

Ticks without path work took 0.02–0.15 ms (200 units, separation was O(n²); since
prompt 016 it asks the unit grid, see [PERFORMANCE.md](PERFORMANCE.md)). The
debug build (`opt-level = 3`) is about 25 % slower. 200 single orders to random
points across a 4 km map at once are the worst case; the latency until the last
path is there grows linearly with the queue. If that matters later: hierarchical
search (clusters with precomputed portals) or a weighted heuristic for long
requests. On `test_small`, two group orders across the river take 2 ticks.

**With map objects** (prompt 013: 98 000 objects on `test_large`, woods on about
20 % of the map closed to tanks), same benchmark:

| Orders | Requests | Expansions per request | All paths after | Time per tick while planning (mean / p99 / max) |
|---|---|---|---|---|
| 200 × 1 unit | 200 | 124 000 | 755 ticks (38 s) | 4.0 / 5.3 / 5.9 ms |
| 25 × 8 units | 94 | 165 000 | 473 ticks (24 s) | 3.8 / 5.1 / 6.4 ms |

Building the grid including the object rasterization takes 172 ms as before.
Groups need more requests (94 instead of 42): members whose shifted path runs into
a wood plan their own. Targets inside woods snap to the nearest cell reachable
from the start; `nearest_cell` visits only the cells on each ring (it used to scan
the whole square per ring, which cost up to 15 ms for targets deep in a wood).

### Open

- Formation facing and slot assignment along the direction of travel (slots are
  assigned by y, then x, as in prompt 009: diagonal moves make members cross
  while the group re-forms).
- Speed matching within a group (members drive at their own terrain speed).
- Obstacles and map objects are cell-based and not widened by the margin for
  large units: a tank whose center passes a free cell next to a tree may overlap
  the trunk by up to half its width.

## Combat

Prompt 017. Rules and data types in `crates/sim/src/combat.rs`, the update
in `crates/sim/src/sim/combat.rs`. Everything is `Fixed` or integer; the
simulation's `Rng` supplies dispersion and penetration rolls in a fixed order
(units by id, weapons in catalog order, projectiles by id).

### Data

- **Unit values come from the catalog**, converted by `data/scale.toml`
  (`CombatType::from_catalog`, part of `UnitType`): direct-fire weapons with
  caliber, rate of fire, muzzle velocity and range (range × 1/4, interval
  between rounds ÷ time factor, projectile speed × time factor), armor per
  face (hull and turret front / side / rear, slope of the upper front plate),
  hull dimensions, squad size, and `gun_traverse_deg` for casemate vehicles.
  Weapons without all four values (grenades, pistols without a rate, MGs
  without a range, mines, bombs) are not used. Mortars, howitzers and
  self-propelled guns get no weapons yet (indirect fire is a later prompt).
  Missing armor faces fall back (rear → side → front, turret side → hull
  side); trucks have none.
- **Rules** are in `data/combat.toml` (`CombatRules`, part of `SimConfig`
  and the state hash): turret and hull traverse per category, retargeting
  interval, sustained rate of automatic weapons, dispersion, gravity,
  penetration formula, damage and hit points.

### Per tick

After commands and path planning, before movement:

1. **Target.** An attack order wins. Otherwise (fire at will) every
   `retarget_ticks` (10) or when the target is gone, a unit picks the enemy
   with the smallest key (main weapon can penetrate it, it can hurt me,
   distance, id), among enemies some weapon can hurt at that distance
   (against their thinnest plate). An order to attack makes the unit drive
   (single path) until the target is within 90 % of its longest range, and
   re-plans when the target moved away.
2. **Aim.** The turret (`aim`, a unit vector) turns towards the target at
   the category's rate × time factor (Panzer IV: 20°/s real, 2° per tick);
   infantry turns at once. Casemate vehicles aim within their traverse and,
   standing still, turn the hull at `hull_deg_per_s`. A weapon fires when the
   turret points at the target, its reload is done, the target is in range,
   it can hurt the target, and no living unit of the same player stands in
   the line of fire.
3. **Fire.** One projectile per barrel (`count`), from the muzzle (half a
   hull length ahead, at 75 % of the unit height) at the target's center.
   The climb is solved for the aim point under gravity (9.807 m/s² × time
   factor²), then side and height get a triangular random error of up to
   2 mrad real, × 4 for the compressed range (× 3 while moving).
4. **Fly** (after movement). Each projectile moves one tick along its
   velocity (75 m per tick for the KwK 40). The segment is tested against the
   oriented hit boxes of all units except the shooter (hull length × width
   × height from the catalog; for infantry squads every living soldier's
   0.6 m box at the height of the squad's stance, prompt 022; wrecks too,
   dead squads not), map objects that block tanks (trees, buildings, walls,
   hedges) up to their catalog height, and the ground (samples at most 1 m
   apart, refined by bisection). The earliest hit counts. Projectiles vanish
   after 150 % of their weapon's range or at the map edge.
5. **Armor.** The face of the box the segment enters is the hit face (front,
   side, rear, top); hits in the upper 35 % of the height strike the turret if
   the catalog lists turret armor. Effective thickness = plate / (cos of the
   angle between flight path and face normal × cos of the plate slope), the
   cosine at least 0.25. Penetration of kinetic rounds = caliber × (v₀ /
   520 m/s)^1.5, −20 % per km of real distance (game distance × 4), ± 10 %
   random; rounds below 300 m/s are shaped charges with 1.6 × caliber at
   any distance.
6. **Damage.** A penetrating hit on a vehicle costs caliber² × 0.1 points
   (7.5 cm: 562 of 1000), a hit on infantry caliber × 8 (rifle 63, a squad of
   nine has 900). At 0 the unit is **destroyed**: it stops, drops its orders,
   takes no commands, does not fire or move, and stays in the unit list
   (stable ids). A vehicle becomes a wreck that blocks its path cells
   (`set_path_obstacle`) and projectiles; dead infantry leaves nothing.
   Infantry (prompt 022): a hit strikes one soldier (`caliber ×
   infantry_per_mm` of his `soldier_hp`); soft cover and buildings may
   catch it first ([INFANTRY.md](INFANTRY.md#behavior-prompt-022)).

Ranges are the catalog's typical combat distances × 1/4 since prompt 067
([SCALE.md](SCALE.md#typical-combat-distances-prompt-067)).

| Gun (catalog) | Muzzle penetration | At its range limit (game / real) |
|---|---|---|
| 7.5 cm KwK 40 L/48 (Panzer IV H, 750 m/s) | 130 mm | 112 mm (175 m / 700 m) |
| 8.8 cm KwK 36 L/56 (Tiger I, 773 m/s) | 159 mm | 127 mm (250 m / 1000 m) |
| 76.2 mm F-34 (T-34, 662 m/s) | 109 mm | 94 mm (175 m / 700 m) |
| Bazooka M9 (HEAT, 81 m/s) | 96 mm | 96 mm (25 m / 100 m) |

### Snapshot and queries

`UnitSnapshot` carries hit points (and maximum), hull direction (`facing`),
turret direction (`aim`) and fire mode; `Snapshot::projectiles` the
projectiles in flight; `Snapshot::events` the `CombatEvent`s of the tick:
`Shot` (unit, weapon, muzzle, caliber), `Hit` (point, unit / ground / object,
face, turret, penetrated, damage) and `Destroyed` (unit, by whom).
`Sim::alive_units(player)` counts living units for victory conditions
(prompt 027). Events are derived, not state, and not hashed.

The bridge collects the events of all ticks of a frame
(`SimHost.take_combat_events`, dictionaries); `scenes/world/combat_view.gd`
reads them (signal `combat_event` for effects and sound), draws a tracer per
projectile, health bars over damaged and selected units, darkens wrecks,
leaves fallen soldiers on the field (prompt 022: `world/infantry_view.gd`
draws the soldiers) and turns each model's `Turret` node by the simulation's
aim. Hulls follow the simulation's `facing`.

### Decisions

| Question | Decision | Why |
|---|---|---|
| Damage model | Hit points; a hit penetrates or not; vehicles lose caliber² × factor, infantry caliber × factor | Simple and traceable from catalog values: a 7.5 cm needs two penetrations, MGs cannot hurt tanks. Modules, crew losses and fires can refine it later. |
| Penetration | caliber × (v / 520)^1.5, −20 %/km | The catalog has caliber and muzzle velocity but no penetration tables; the formula meets published values of the KwK 36 and KwK 40 and needs only `isqrt`. No values from other games. |
| Armor by angle | Entry face of the oriented hit box, angle to its normal | Front, flank, rear and obliquity come from one geometric test, without sector rules. |
| Targets | One target per unit, fixed sort key, every 10 ticks | Deterministic, cheap (O(n²) every 0.5 s), no random choice. |
| Projectiles | Real objects with ballistic flight, tested per tick segment | Hills, trees and wrecks stop rounds by themselves; effects get flight paths. Units near a flight segment come from the unit grid (prompt 016, [PERFORMANCE.md](PERFORMANCE.md)). |
| Friendly fire | Possible, but units do not fire through friends | Rounds that miss can hit anything; a second rank does not shoot into the first. |
| Automatic weapons | 25 % of the cyclic rate | The catalog lists the cyclic rate; the practical rate of MGs is a quarter to a sixth of it. |
| Turret traverse | Per category in `combat.toml` | The catalog has no traverse speeds. |
| Casemates | Catalog field `gun_traverse_deg` | A historical fact (StuG III G: ±10°) with sources, unlike a category rule (the M10 is a turreted tank destroyer). |

### Open

- Guns (`gun_*`) are pushed by their crew or towed since prompt 034
  ([Transport and towing](#transport-and-towing)); flak is placed as
  fortifications (prompt 033).
  Aircraft: [Air support](#air-support-and-anti-aircraft-fire). Damaged
  modules, ammunition limits.
  Indirect fire, HE fragments and smoke: see
  [Indirect fire and smoke](#indirect-fire-and-smoke) (prompt 032). Line of sight for targets: see
  [Vision and fog of war](#vision-and-fog-of-war).
- Target leading: rounds fly at the target's position at the moment of the
  shot; at 75 m per tick this matters only for fast targets far away.

## Unit stats (display only)

`crates/sim/src/unit_stats.rs` (prompt 065) turns a catalog entry and the rule files
into one card of game values, with the rule functions the simulation itself uses. It
is a set of pure functions outside the simulation state: no rule reads it, the state
hash and the golden hash do not depend on it. `cli unit-stats` prints all cards as
JSON (the unit page shows them), `SimHost.unit_stats(type_id)` returns one as a
dictionary (prompt 066; how the game shows them: "Unit values in the interface"
in the HUD section). There is no second calculation anywhere: JavaScript and
GDScript only format.

All numbers are integers with the unit in the key; `game` values are what the
simulation computes with, `real` values the historical ones. Base values: dry ground,
no experience bonus, no weather.

| Card section | Values | From |
|---|---|---|
| `movement` | game speeds on road and cross-country | `UnitType::from_catalog` (catalog speed × `data/scale.toml` time factor) |
| | hull and turret turn rate, gun traverse | `data/combat.toml` `[aim]`, catalog `gun_traverse_deg` |
| | speed in mud and deep snow | `WeatherRules::speed_pct` at full softness, slowest terrain (`data/weather.toml`) |
| | steepest grade and the speed there | `data/terrain.toml` `[slope]` |
| | what the vehicle flattens | `crush_min_mass_kg` of `data/destruction.toml` against catalog `mass_kg` |
| `protection` | hit points | `CombatRules::max_hp`, fortifications from `data/economy.toml` |
| | armor per face, effective hull front | catalog armor as `combat::Armor`; front slope and `min_cos_pct` as in the armor check of a hit |
| | `versus`: reach of each comparison weapon per face, hits to destroy | `CombatRules::penetration_at` (largest real distance, 10 m steps, that still penetrates; no random spread, straight impact), `vehicle_damage`, `infantry_damage` |
| | hurt by shell fragments | `open_armor_mm` of `data/indirect.toml` |
| `weapons` | game and real range, shots per game minute | `CombatType::from_catalog`, `CombatRules::reload_ticks` |
| | penetration at 100/500/1000/1500 m real, damage | `penetration_at`, `vehicle_damage`, `infantry_damage` |
| `against_vehicles` | strongest gun at 500 m real, which reference tank it knocks out from front, side and rear | as `versus`, against the hulls of the other factions' reference tanks |
| `against_infantry` | damage per hit, automatic weapons, soldiers per game minute | `infantry_damage`, `reload_ticks`, `soldier_hp`; shells: one soldier per shell |
| `against_structures` | hits until reference objects collapse, or "no effect" | hit points and `min_caliber_mm` of `data/destruction.toml`, `vehicle_damage` |
| `indirect` | range, minimum range, setup, blast radius, direct hit | `IndirectWeapon::from_catalog`, `data/indirect.toml` |
| `other` | sight, ammunition (salvos, minutes of fire), fuel, seats, cost, build time | `VisionRules::range_m`, `LogisticsType::from_catalog`, `EconomyRules::unit_cost` / `structure_cost` |
| `object` | hit points, minimum caliber, crush mass of map objects | `data/destruction.toml` |

Comparison weapons, targets and objects are the constants `REFERENCE_WEAPONS`
(rifle, machine gun, 3.7 cm, 5.7 cm, 7.5 cm, 8.8 cm, Panzerschreck: catalog weapons),
`REFERENCE_TARGETS` (a light, medium and heavy tank per faction) and
`REFERENCE_OBJECTS`. Only weapons the simulation fires appear: a catalog weapon
without caliber, rate, muzzle velocity and range (many vehicle machine guns) is not
a game weapon and so not on the card. Aircraft attack through the air-support rules
and have no weapon sections.

**Ratings.** Five display numbers 0–100 with fixed reference values (the block
`RATINGS`), so they do not shift when units are added. Each is
`min(100, value × 100 / reference)`:

| Rating | Value | Reference (= 100) |
|---|---|---|
| `speed` | mean of road and cross-country game speed | 30 m/s in the game (54 km/h real) |
| `protection` | armored: 10 + 90 % of effective armor (front 50 %, side 30 %, rear 20 %, hull and turret averaged) | 150 mm |
| | unarmored vehicles, guns: 10; infantry: 2 per soldier, at most 20 | – |
| `anti_vehicle` | best penetration at 500 m real (or at the weapon's range if shorter), times 50–100 % for a range of 0–500 m | 200 mm |
| `anti_infantry` | soldiers put out of action per game minute if every round hits | 600 |
| `anti_structure` | damage per game minute of the strongest weapon of at least the minimum caliber | 20 000 |

They are a display, not a rule: the AI and the simulation never read them.

## Vision and fog of war

Prompt 018. Every player sees only what their units see; hills, forests,
hedges, walls and buildings block sight. Vision is a game rule (combat
targets depend on it), so it lives in `crates/sim` (`vision.rs` for the grid
and the sight computation, `sim/vision.rs` for keeping it up to date). Godot
only draws the result.

### Rules and grid

`data/vision.toml` (built in, `SimConfig::vision`, in the state hash):
cell size (8 m), target height (2 m), forest rules, the ray-step budget per
tick, sight ranges in **real** meters per category (2 km, recon and snipers
2.8 km, trucks 1.6 km; compressed by `range.sight` = 1/4 of
`data/scale.toml` to 500 / 700 / 400 m) and which object ids are `solid`
(houses, barns, church, walls, hedges) or `foliage` (trees). The catalog has
no sight values; these are rules like the turret traverse in `combat.toml`.

`VisionGrid` (built once per match): per 8 m cell the ground height at its
center (mm), the top of solid obstacles (catalog height) and forest (cells
with at least two trees; top = tallest tree).

### Viewsheds

A unit sees from its cell at eye height = its ground height + its catalog
height (infantry: `combat.toml` infantry height). Rays run from the eye to
every cell on the edge of the square with the sight radius (precomputed
offset tables per radius: at every step along the ray's major axis the
nearest cell; together the rays visit every cell of the sight circle). Along
a ray two horizons are kept, as slopes `num/den` compared by
cross-multiplication (integers only, no division):

- **Ground and solid obstacles** raise the horizon at once.
- **Forest:** a sight line may cross `foliage_depth_cells` (1) forest cells
  whose canopy lies above it. The ray keeps the two steepest canopy slopes;
  the second one is the forest horizon. One sees and is seen 16 m deep into
  a forest, the same in both directions; from a hill one sees over the
  canopy; a single tree hides nothing.

A cell is visible when the point 2 m above its ground is not below either
horizon. The result is a bit field over the sight square (≈ 2 KB per unit).

**Seeing downhill** (prompt 060, `[height]` in `data/vision.toml`
[provisional]): beyond its sight range a unit also sees cells whose ground
lies more than 20 m below its eye, 5 % of the range farther per 10 m of
further drop, at most 25 % (500 m → 625 m). The line of sight already lets
heights look over cover; this adds the wider view over a valley, and only
downwards (the valley does not see the height farther). Such a unit uses a
ray table of the longer reach, but only when some ground within that reach
is low enough: the grid keeps the lowest ground per block of 8 × 8 cells
(updated with craters), so units on flat ground keep their usual table and
cost. Beyond the sight range a ray stops as soon as its horizon rules out
every further cell. `test_small` has no 20 m drops within sight, so the
demo is unchanged; on `mountain_pass` vision-bench takes about 18 % more ray
steps than before (250 000 instead of 212 000 per tick, 500 units).

### Updates and budget

A viewshed is computed again only when its unit enters another cell;
destroyed units lose theirs at once. Units to update wait in a FIFO queue;
every tick (after projectiles) works through it until
`budget_steps_per_tick` (400 000) ray steps are spent, at least one unit per
tick. The budget is a rule constant, so a late update happens in the same
tick on every peer. All viewsheds are computed in `Sim::new`.

Every player has a reference count per cell (how many own viewsheds contain
it) and an explored bit. Cell states: never seen, explored, visible. After
the update every unit gets a bit mask of the players that see it (its cell
is visible to them; own units always, wrecks for everyone). State hash:
vision rules, per unit the viewshed anchor, eye height, radius and cell
count, the queue, the masks and the number of explored cells per player.

### Combat

- Automatic targets are chosen only among enemies in the unit's **own**
  viewshed, and an automatic target out of it is dropped. No more rounds
  into a hill at a target only a comrade sees.
- An attack order fires only with the unit's own sight; without it, the
  unit drives towards the target until it sees it. When the player loses
  sight of the target, the unit drives to where it was last seen and drops
  the order there if the target is still hidden. Orders against units the
  player has never seen in this order are dropped. Ground attacks are
  unchanged.

### Ambush (prompt 056) [provisional]

`crates/sim/src/sim/vision/ambush.rs`, rules in `data/vision.toml`
`[ambush]`: a squad that stands (no route, not aboard) in a forest cell of
the vision grid (`Sight::Foliage`) or is garrisoned in a building, and has
not fired for `reveal_s` (20 s), is seen only by its team and by players
with a hostile unit within `spot_real_m` (200 m real, 50 m game) that has
it in sight. It runs at the end of the vision update, after the camouflage
nets of prompt 052, whose pattern it follows (`Vision::restrict`). The
squad's own shots (`CombatEvent::Shot`) reveal it for `reveal_s`; standing
silent hides it again. Since targets are chosen among seen units, a hidden
squad draws no fire from afar and opens fire at the range it chooses (hold
fire until the enemy is close). The rule and the last shots are hashed
only once a squad has been hidden (`Ambush::used`), so matches without
infantry in cover keep their hashes (the golden hash did not change).
`spot_real_m = 0` turns it off.

### Snapshot, bridge and Godot

`Sim::snapshot_for(player)` leaves out enemy units the player does not see
(own units and wrecks stay); projectiles and events stay unfiltered,
because muzzle flashes and tracers can be seen from afar.
`Sim::visible_to`, `vision_states`, `vision_version` and `vision_grid` serve
the presentation. The bridge (`vision_funcs.rs`) gives Godot the local
player's snapshot and the cell states as an L8 image (0 / 128 / 255),
uploaded only when the version changes.

`scenes/world/fog_of_war.gd` draws the fog as a screen-space overlay: a quad
in front of the camera reads the depth buffer, reconstructs every pixel's
world position and darkens it by its cell (never seen: nearly black,
explored: dimmed). It covers terrain, MultiMesh vegetation, objects and water
without changing their materials; linear filtering and five taps give soft
edges. Tracers in the fog are dimmed with the ground behind them. `main.gd`
hides the nodes of units that left the snapshot. F3 → "Fog of war" reveals
everything (presentation only: `SimHost.set_reveal_all`).

**Map hacks:** in lockstep every peer simulates the full state, so a
modified client can always show everything. Filtering happens only in the
presentation; it keeps honest players honest, nothing more. Preventing it
would need a server that holds the state, which lockstep does not have.

### Measurements (`just vision-bench`, Apple M1 Max, release)

500 units of two players (half Panzer IV, half rifle squads) on `test_large`
(500 × 500 vision cells), all driving to random targets in groups of ten,
600 ticks, hold fire:

| | mean | median | p99 | max |
|---|---|---|---|---|
| with vision | 6.03 ms | 6.04 ms | 8.73 ms | 36.9 ms |
| one-cell sight (baseline) | 4.46 ms | 4.40 ms | 5.93 ms | 9.3 ms |

About 1.6 ms per tick for vision; on average 213 000 ray steps per tick
(≈ 7.5 ns each), the budget (400 000) was reached in few ticks and the queue
held at most 8 units. `Sim::new` computes all 500 viewsheds in about 90 ms.
The maximum tick contains the path planning of the first orders.

### Decisions

| Question | Decision | Why |
|---|---|---|
| Resolution | 8 m cells | 62 cells radius at 500 m, ≈ 31 000 ray steps per viewshed; 4 m would cost four times as much. About a tank length; enough for crests and forest edges. |
| Update | On cell change, FIFO with a ray-step budget | Standing units cost nothing; the cost per tick is bounded like path planning; under load vision lags a little instead of a tick taking too long. |
| Sight line | Rays to the square's edge, integer horizon slopes | Standard viewshed method; covers every cell; no division, no floats. |
| Target height | One value (2 m) | One grid per player instead of a sight line per pair; between standing infantry and a tank. |
| Forest | Count of canopy cells above the line (≤ 1) | Symmetric (who sees in is seen out), looks over the canopy from above, single trees hide nothing. A first "delayed horizon" model was asymmetric. |
| Hedges, walls, houses | Solid up to their catalog height | Hedges are only one cell thick but should hide (bocage). |
| Combat | Own line of sight for targets and fire; attack orders follow the last known position | No rounds into hills; behavior reveals nothing about hidden units. |
| Revealing shooters | Not a rule | Keeps the simulation simple; tracers show where fire comes from. |
| Presentation | Screen-space overlay from the depth buffer | One shader for everything, no changes to terrain, vegetation or water materials. |

### Open

- Wrecks do not block sight; destroyed map objects rebuild their cells and
  queue the viewsheds ([Destruction](#destruction)); craters update the
  ground height of their cells and queue the viewsheds
  ([Terrain deformation](#terrain-deformation)).
- Units on bridge decks: target cells use the ground height below the deck.
- Last known positions of enemy units are not drawn (ghost markers).
- Smoke (prompt 019 is presentation only) does not block sight yet.

## Destruction

Prompt 020. Trees, hedges, walls, fences, barbed wire and buildings can be
damaged and destroyed; that changes paths, sight and cover. Rules and public
types in `crates/sim/src/destruction.rs`, the state and its effects in
`crates/sim/src/sim/destruction.rs`; Godot only swaps model variants and
plays effects.

### Rules

`data/destruction.toml` (built in, loaded by `Sim::new` with
`DestructionRules::builtin`, in the state hash; not part of `SimConfig`,
like `data/objects.toml` it belongs to the map objects): per group of
catalog ids the hit points, the share at which an object counts as damaged,
what remains when it is destroyed (`ruin`, `fallen`, `gone`, `flat`), what a
ruin blocks and how high it is, the vehicle mass that crushes it, and the
model variants of the damaged and destroyed stages. Rounds below 20 mm do no
damage. Bunkers, hedgehogs, rocks, bridges and props are in no class and
cannot be destroyed.

| Objects | Hit points | Damaged at | Destroyed |
|---|---|---|---|
| Trees | 400 | 40 %: bare (prompt 085): no forest, stop projectiles up to half the height, block paths as before | fall in the flight direction (away from a blast); block nothing, no forest |
| Hedges / dry-stone walls | 600 / 1200 | – | gone (a gap) |
| Fences / barbed wire | – | – | flattened by vehicles from 2.5 t / 8 t |
| Mud house, barn, farmhouse (brick, stone) | 1800, 2000, 2500 (3000, 3500) | 50 % | ruin: blocks tanks only, stops projectiles up to 40 % of the height |
| Church | 6000 | 50 % | ruin up to 30 % (the tower falls) |
| Apartment blocks (prompt 078): block / long block, tenement | 6000 / 9000 | 50 % | ruin up to 33 / 27 / 30 % (one to two storeys) |

A 7.5 cm round (562 points, the vehicle damage formula of `combat.toml`)
fells a tree, three bring down a wall segment, a farmhouse is damaged after
three and a ruin after five. A 5 cm round (250) strips a tree it hits, two
3.7 cm rounds do too.

**Blast on trees (prompt 085).** `[blast]` in `data/destruction.toml`: every
explosion that goes through `explode` (shells, rockets of launchers and the
heavy strike, bombs at their converted caliber, exploding dumps) of at least
10 cm damages the objects of classes with `blast = true` (trees so far)
around the impact point: 6 × (caliber − 50 mm) hit points at the point,
falling linearly to 0 at 0.18 m per mm of caliber (at most 30 m). A 10.5 cm
shell strips trees within 5 m and fells none; a 13.2 cm rocket fells within
4 m and strips within 12 m; 15 cm fells within 9 m and strips within 16 m.
Blasts do not add up (an object keeps at most what an intact one has left
after the strongest blast; direct hits still add), so a salvo leaves a lane
of felled trees among bare ones instead of clearing a wood. Felled trees fall
away from the impact point (heading of impact → tree from the sine table).
Mortars, light guns and tank guns (below 10 cm) change trees only by direct
hits. The objects are found through the object index (`Destruction::query`),
in index order. A bare tree (`damaged_clear`, `damaged_height_pct = 50`)
counts for no forest cell, so the vision cells of its area are rebuilt
(`pending_sight`, no path rebuild); paths stay blocked as by a standing
tree. Fire: a bare tree can still catch fire; a felled one cannot.

### State and per tick

The map stays as loaded. Per object the simulation knows hit points, stage
(`Intact`, `Damaged`, `Destroyed`) and fall direction (whole degrees); only
objects that differ from their start are stored (`BTreeMap` by index, in
the state hash), so 100 000 objects cost nothing until they are hit. The map
variant gives the start: `Damaged` and `Ruin` variants start damaged or
destroyed (a placed ruin acts as a ruin from `Sim::new` on). How an object
acts follows from its stage (`effective`: blocking class, sight, height):

1. After the movement, vehicles at least as heavy as `crush_min_mass_kg`
   flatten crushable objects whose shape overlaps their footprint square,
   falling in their driving direction.
2. In the projectile flight, a hit on an object costs its hit points the
   vehicle damage of the round (`HitKind::Object`, `damage` in the event).
   Destroyed trees, gaps and flattened objects no longer stop projectiles;
   ruins stop them up to their height. A round whose first segment starts
   inside a ruin ignores that ruin, so infantry in a ruin can fire out.
3. `apply_destruction` (before the vision update): for every object
   destroyed in this tick, the object flags of its path cells are
   re-rasterized from all objects there (`PathGrid::rasterize_objects_in`),
   flags and costs rebuilt (`rebuild_cells`), the components labeled once
   for all of them, and units whose route touches the cells plan again; the
   vision cells are rebuilt (`VisionGrid::rebuild_objects`, copy-on-write of
   the shared grid) and every viewshed that covers them is queued. Objects
   that only changed their sight (bare trees) rebuild vision cells only.
   Overlapping areas are joined into their bounding box first (a salvo into
   a wood rebuilds a few areas, not hundreds).

Events (`Sim::object_events`: object, new stage, remains, fall direction,
point, cause `Hit(shooter)` or `Crushed(vehicle)`) are derived, not hashed.
For prompt 022 (cover, occupying buildings): `object_state`,
`object_stage`, `object_hp`, `object_blocks` (blocking class now),
`object_remains`, `changed_objects`.

**Cost** (`test_large`, 4 km, 98 000 objects, release, Apple M1 Max): the
object index adds a few milliseconds to `Sim::new` (198 ms in all); a tick in
which objects are destroyed takes about 8 ms, almost all of it the one flood
fill of the component grids over the whole map (ticks without destruction
are unchanged). Several objects in one tick cost the same flood fill.
A salvo into the densest wood (`just salvo-bench <map>`, 200 moving units,
prompt 085): 16 × 13.2 cm fells 24 and strips 73 trees on `test_large` (17 /
83 on `city_large`), worst tick 11.4 ms (10.2); 18 × 15 cm, six per tick,
fells 69 and strips 70 (74 / 70), worst tick 14.3 ms (11.9); ticks without
impacts 2–4 ms. Far below the 50 ms budget.

### Bridge and Godot

The bridge collects the events of every tick (`SimHost.take_object_events`)
and gives the state of changed objects (`changed_objects`, `object_state`),
the variant to show and where an object is drawn (`object_instance`: kind,
chunk, slot of `object_groups`). `scenes/world/destruction_view.gd` reads
the events; `map_objects.gd` collapses the object's MultiMesh instance and
puts a single node in its place: the `Damaged` or `Ruin` variant, a tree
that tips over in 1.6 s next to an `env_tree_stump`, a flattened fence or
wire; walls and hedges just vanish. A bare tree (prompt 085) is the leafless
winter oak (`Bare`) for grown oaks and beeches; young oaks, conifers, birches
and palms keep their model with the instance uniform `env_bare = 1`, which
folds every foliage card to a point in `env_foliage.gdshader` (no new models,
no extra draw calls; on winter maps the oaks are bare already and drop
nothing). A bare tree that falls later falls bare. `effects.gd` (`object_event`) throws
debris, dust clouds and a scorch mark for a collapsing building, splinters
and dust where a tree's crown lands, debris for walls and a puff for crushed
fences, only at visible points. A stripped tree drops leaves, needles or
frond bits (`leaves`, 4 s, tinted per tree type) and a few twigs; it makes
no sound of its own (the blast is heard). At most six trees per frame get
bursts, the rest change silently (`TREE_EFFECTS_PER_FRAME`). Objects change in the fog too (the stage is
not secret, only the effects are hidden).

### Decisions

| Question | Decision | Why |
|---|---|---|
| Where the state lives | Next to the unchanged map, only deviations | The map and its hash stay the loaded map; a copy-on-write of a 4 km map per hit would be expensive, and a map's kinds often lack the ruin variant. |
| Rules outside `SimConfig` | Built in like `data/objects.toml`, hashed | Bound to the map objects; keeps `SimConfig` (shared by every caller) unchanged. The state hash reports differing rules. |
| Damage | Direct hits, caliber² like vehicles, from 20 mm | Consistent with prompt 017; rifles do not fell trees. Splash comes with indirect fire. |
| Blast on trees (prompt 085) | From 10 cm, linear falloff, the strongest blast decides; only classes with `blast = true` | Heavy charges thin a wood visibly; mortars and guns must not clear it shot by shot; adding up let one salvo clear a whole wood (105 of ~110 trees). Houses, walls and hedges can opt in later with the same flag. |
| Bare trees and sight (prompt 085) | No forest at all; projectiles up to half the height; paths unchanged | A stripped crown hides nothing in an 8 m cell; counting half would need weights in the forest count. The trunk and main branches still stop rounds, and the standing trunks still keep tanks out. |
| Bare trees in Godot (prompt 085) | Winter oak model for oaks and beeches, foliage folded away in the shader for the rest | Uses what exists: no new models, thumbnails or draw calls. |
| Ruins and sight | Ruins block no sight | At 8 m vision cells a ruin would blind the infantry inside it; rubble has gaps. Cover against fire comes from the projectile height. |
| Felled trees | Block nothing | Tanks drive over trunks; "the forest thins" follows from the forest count of the vision cells. |
| Fall direction | Flight direction in whole degrees from the sine table | Deterministic without floats or randomness. |
| Rebuilds | Collected per tick, components once | Several hits per tick cost one flood fill of the component grids. |
| Presentation | Hide the MultiMesh instance, add a single node | No rebuild of object groups; a few hundred destroyed objects cost little. |

### Open

- Bridges cannot be shot down yet (`Sim::set_bridge_intact` exists; what
  happens to units on the deck is open); blast damage to houses, walls,
  hedges, fences and bridges (only trees take it, prompt 085); rubble
  as an obstacle of its own, fires in buildings.
- Trees (prompt 085): splintered stumps as an obstacle of their own; tree
  bursts (shells exploding in the crowns and showering the infantry below);
  regrowth and seasons; the radii are game values without a source. Craters: [Terrain
  deformation](#terrain-deformation).
- Ruins stop rounds below their height; soldiers inside are hit through
  the rubble with an exposure chance by stance, and occupants of a
  collapsing building take losses (prompt 022,
  [INFANTRY.md](INFANTRY.md#buildings)). Nothing protects from above.
- Fallen trees and ruins do not change the decoration (grass under a felled
  crown stays).

## Economy: base building and production

Prompt 023. The game rules and their reasons: [GAME_DESIGN.md](GAME_DESIGN.md).
Rules in `crates/sim/src/economy.rs` (`data/economy.toml`, compiled in like
the destruction rules, part of the state hash, not of `SimConfig`), the state
in `crates/sim/src/sim/economy.rs`.

**Buildings are units.** Base buildings (`base_*`, sizes, hit points and
armor from the rules: `UnitType::structure`), fortifications from the
catalog (`UnitType::fixed_from_catalog`) and construction sites are units with
speed 0. Combat (hit boxes, armor, automatic targets, bunkers and gun
positions firing their catalog weapons), vision, wrecks that block their path
cells, victory conditions and the computer player's view need no special
case. The economy keeps per unit type whether it is a structure, its price,
build radius, income and requirements, and per structure (by unit id) the
build progress, the production queue and the rally point. A structure faces
along the y axis towards the map center, so its footprint is an axis-aligned
rectangle; its path cells are blocked with `Sim::set_path_obstacle` when it is
placed.

**Scenarios.** `economy = true` in a scenario appends the base buildings, the
fortifications and the production lists of the players' factions to
`SimConfig::unit_types` (`EconomyRules::unit_types`); groups with
`role = "building"` place buildings. `start_distance_pct` moves every
player's groups outwards on the line from the map center through their start
(`data/scenarios/skirmish.toml`: 70 % of half the map size), 5 % closer at a
time while a group would stand within 20 m of a blocking map object or on
impassable terrain. A player's
faction is the prefix of their first catalog unit.

**Per tick** (`Sim::update_economy`, after destruction, before vision):

1. Structures whose unit was destroyed are dropped with their queue.
2. Construction sites gain hit points linearly (from 10 % of the maximum) and
   hold fire until completed.
3. Supply points (map markers `supply*`): units (no buildings) within the
   radius take a point while no other player has units there; an enemy point
   is neutralized first.
4. After every full second: income of completed buildings and held points.
   Accounts are kept in 1/60 000 of a resource unit, so an income per minute
   pays exactly.
5. Production: the first order of every completed producing building counts
   down (not while its owner is at the unit limit); a finished unit appears at
   the first free spot beside the building (rings outwards, 15° steps from
   its front, passable for the unit's path profile) and drives to its slot of
   a 3 × 3 formation around the rally point. Units are added during the match
   by `Sim::spawn_unit`: ids keep increasing, the vision arrays grow.

**Placement** (`Sim::placement`, the same check for the command and for the
preview): a structure type the player's faction may build, requirements
completed, count and construction site limits, resources, within the build
radius of an own completed building, footprint inside the map, every path cell
under it passable for large units (no water, map objects, wrecks or other
buildings), height difference over the footprint at most `max_grade_pct` of
its larger side, no living unit within `clearance_m`. `Sim::find_site` tries
rings around a point in a fixed order (the computer player).

**Queries** for the bridge and the AI: `account`, `income`, `faction`,
`structures` (`StructureView`), `supply_points`, `build_options`,
`production_options` (`BuyOption` with the reason it is blocked), `cost_of`,
`structure_size`, `is_structure`, `unit_type_index`, `unit_position`.

**Bridge and Godot.** `crates/bridge/src/economy_funcs.rs` turns these into
dictionaries and queues the commands; `SimHost.mode` picks the battle or the
base-building scenario for `start_match`, `SimHost.speed` runs more ticks per
frame (self-tests). `scenes/world/base_view.gd` gives the `base_*` buildings
their models (the headquarters a borrowed stone house of prompt 007; since
prompt 056 the vehicle works `env_steel_hall`, the barracks `env_barracks`
and the depot `env_depot`, built by `tools/blender/kit/base_buildings.py`;
all with the `Damaged` and `Ruin` variants of prompt 020), lets a site's model grow with its progress, and draws supply
points as flags in the colour of their holder with a ring that fills while
they are taken. `scenes/world/base_hud.gd`: resource bar, build menu with
prices (greyed out with the reason), placement preview (a box of the
building's size, green or red with the reason from `placement_check`),
production bar of the selected building with its queue and a cancel button,
right click sets the rally point. Box selections that also catch units leave
buildings out.

**Cost** (`test_large`, release, Apple M1 Max): the skirmish has 26 unit
types instead of 4, one path cost profile each, so `Sim::new` takes 0.86 s
instead of 0.27 s (a performance item for prompt 016: profiles with equal
speeds could share their costs). A tick of the demo is unchanged; placing a
building costs one relabeling of the component grids, like a destroyed
object.

### Decisions

| Question | Decision | Why |
|---|---|---|
| What a building is | A unit with speed 0 | Combat, vision, destruction, victory and the AI view work unchanged; a separate entity type would need its own targeting, hit tests and fog filtering. |
| Who builds | The player, by command, within a build radius | No builder units to path, protect or manage for the AI; one command per building. |
| Rules location | Built in and hashed, not in `SimConfig` | Like the destruction rules: keeps `SimConfig` and the replay layout unchanged for the parallel prompts; peers with other rules desync at once. |
| Unit prices | Formula from catalog values | Covers all 133 entries without a hand-made table; stronger units are never cheaper. |
| Supply points | Map markers | Maps decide where the fighting is; the generator places three, the editor keeps them. |
| Consumption | None yet | See GAME_DESIGN.md; the account model allows supply trucks later. |

### Open

- Supply points cannot be placed in the map editor yet (markers are kept).
- Path profiles per unit type grow with the economy types (see Cost).
- The LAN lobby has no mode choice; LAN matches play the battle.

## Terrain deformation

Prompt 021. Ground hits of heavy rounds leave craters that change the
simulation's map: heights and terrain types. Movement, ballistics, paths,
sight and the presentation all read the same samples. Rules and public
types in `crates/sim/src/craters.rs`, the state and its effects in
`crates/sim/src/sim/craters.rs`; Godot only redraws.

### Rules

`data/craters.toml` (built in, loaded by `Sim::new` with
`CraterRules::builtin`, in the state hash; like `data/destruction.toml` not
part of `SimConfig`). Game rules, not catalog values:

| Rule | Value |
|---|---|
| Crater from | 45 mm caliber (ground hits; smaller rounds only throw dust) |
| Bowl radius | 12 mm per mm of caliber, at least 2 m (one raster step) |
| Bowl depth | 5 mm per mm of caliber (7.5 cm: 375 mm, 15 cm: 750 mm) |
| Rim | 25 % of the depth high at the bowl's edge, falling to 0 half a radius further out |
| Limits per sample | at most 1.5 m below and 0.4 m above the loaded map |
| Bowl terrain | `mud` (40 % speed); water types keep theirs and take no craters, nor do bridge decks |
| Per tick | at most 16 craters; further ground hits in that tick leave none |
| Cover | from 250 mm below the loaded height: 15 / 35 / 50 % of hits caught (stand / kneel / prone) |

The shape on the 2 m raster, all integer: for every sample within the rim
radius, with `d²` its squared distance to the impact in mm²,
`-depth + (depth + rim) × d² / r²` inside the bowl and
`rim × (outer - d) / (outer - r)` on the rim (`d` the integer square root).
A 7.5 cm crater on a sample lowers it by 375 mm, raises its four neighbors
by 93 mm and the diagonal ones by 15 mm; one between two samples lowers
both by 258 mm.

### State and per tick

- **The map is changed in place.** The first crater copies it once
  (copy-on-write through `Arc::make_mut`, like a destroyed bridge); after
  that only the simulation holds it. The bridge reads the map through
  `Sim::map()` and keeps no second `Arc`, or every crater would copy it
  again. The map's content hash stays the loaded one (its identity for
  `SimConfig::map` and replays).
- **Deviation raster:** one `i16` per sample (allocated with the first
  crater, 8 MB on `test_large`) holds height minus loaded height. It clamps
  repeated hits, marks craters for infantry and lets the decoration skip
  them. Memory stays constant however many craters there are.
- **Order:** `fly_projectiles` collects ground hits (`HitKind::Ground`) of
  heavy rounds; `apply_craters` runs after `apply_destruction` and before
  the vision update. Per crater: samples changed (planned on the unchanged
  map, then applied in order), then for all craters of the tick:
  - path cells under the craters: flags and costs rebuilt
    (`PathGrid::rebuild_cells`). Components are labeled again and routes
    through the cells re-planned **only if passability changed** (a crater
    field steeper than 60 %, or the reverse); mud only raises the cost;
  - vision cells whose center height changed get the new ground
    (`VisionGrid::ground_changes` / `set_ground`, copy-on-write of the
    shared grid) and the viewsheds that contain them are queued; obstacle
    tops (absolute heights) stay;
  - units and soldiers standing there get the new ground height; standing
    squads nearby choose their cover spots again.
- **Hash:** the rules, the number of craters, queued craters and a running
  checksum (`StableHasher` over the previous checksum, every crater's
  point and caliber and every changed sample's index, height and terrain).
  Hashing 4 M samples per tick would be far too slow; the checksum sees
  every change in order.
- **Events** (`Sim::crater_events`: point, caliber, radius, depth, sample
  rectangle) are derived, not hashed. `Sim::queue_crater(point, caliber)`
  queues a crater like a ground hit, for game systems and tools, never for
  player commands. `deformation_mm(i, j)`, `crater_count`,
  `has_craters`, `crater_rules` serve tests and the presentation.

### Infantry cover

Crater samples at least 250 mm deep are **cover spots** like the spots
behind objects ([INFANTRY.md](INFANTRY.md#cover)): a standing squad assigns
its soldiers to the nearest free spot within reach, object or crater. A
hit on a soldier of a standing squad whose ground lies that deep is caught
by the rim with the chance of his stance, from every direction (simulation
`Rng`); the round becomes a ground hit there. `SoldierSnapshot::in_cover`
holds at crater spots as well.

### Bridge and Godot

The bridge collects the crater events of every tick
(`SimHost.take_crater_events`: position on the new ground, radius, depth,
caliber, sample rectangle), `scatter_deco` leaves deformed samples empty,
and `debug_crater(x, z, caliber_mm)` queues a simulation crater (refused
in network matches; a single-player replay with debug craters no longer
verifies). `scenes/world/crater_view.gd`:

- collects the sample rectangles (overlapping ones merged) and redraws the
  terrain with one `terrain.regions_changed` (one texture upload). Godot
  uploads whole textures, so uploads are spaced at least 0.1 s apart and at
  least eight times the cost of the last upload (at most 0.5 s): on large
  maps 100 craters cost a few uploads, not 100. Units already stand on the
  new ground meanwhile;
- has `map_objects.craters_changed` rebuild the decoration squares over the
  craters (six per frame at most); grass, bushes and stones vanish from the
  crater and its rim.

The impact itself is the ground-hit effect of prompt 019 (earth fountain,
dust, scorch decal); the mud splat of the bowl and the shading of the
changed normals make the crater visible.

### Measurements

`just crater-bench` (`cli crater-bench`): 200 units of two players
(half Panzer IV, half rifle squads) driving across the middle of
`test_large`, then 100 craters of 7.5 cm at random points within ±200 m,
release build, Apple M1 Max:

| Craters | Ticks before | First crater tick (map copy) | Ticks with craters (mean / max) | Path requests |
|---|---|---|---|---|
| 5 per tick (1 s) | 2.2 ms | 3.3 ms | 3.1 / 3.2 ms | 0 |
| 16 per tick (0.35 s) | 2.3 ms | 3.3 ms | 3.2 / 3.2 ms | 0 |
| 16 per tick within ±10 m (crater field steeper than 60 %) | 3.4 ms | 3.3 ms | 27.8 / 39.1 ms | 102 |

Spread craters cost about 1 ms per tick (path cells, vision cells and the
viewshed queue); no route has to be planned again. Stacked craters that
close cells cost a flood fill of the components per tick (about 8 ms on
`test_large`) and re-planning of every route through them, still within
the 50 ms tick. Since prompt 016 the components are relabeled locally
(`relabel_components_in`): the stacked case takes 6.0 / 6.7 ms per tick
instead of 22 / 24 ms (same run, same state hash `0x47f2f2f2b80a01e6`).

`just crater-view-bench [map]` (`tools/crater_bench.gd`, `-- --effects-craters`):
1920 × 1080, V-Sync off, quality high, forest, debug bridge, camera 150 m
above the middle between both sides, every unit holding fire; 100
simulation craters of 7.5 cm (`debug_crater`) within 2 s on an area of
120 m × 120 m, each with the ground-hit effect of prompt 019 (Apple M1 Max):

| Map | Idle | 100 craters in 2 s (worst frame) | 4 s after | Terrain uploads (last) | Decoration squares rebuilt |
|---|---|---|---|---|---|
| `test_small` | 77 FPS | 71 FPS (21 ms) | 64 FPS | 20 (0.7 ms) | 460 |
| `test_large` | 114 FPS | 61 FPS (45 ms) | 96 FPS | 20 (6.2 ms) | 524 |
| `test_large`, craters without effects | 113 FPS | 74 FPS (45 ms) | 102 FPS | 20 (6.1 ms) | 558 |
| `test_large`, without decoration rebuilds | 114 FPS | 64 FPS (36 ms) | 96 FPS | 20 (6.4 ms) | 0 |

On `test_large` one upload of all terrain textures (2001² samples, 72 MB)
costs about 6 ms; recomputing the height ranges of all 1024 chunks took
another 11 ms per upload until `terrain.regions_changed` updated only the
chunks under the craters (`SimHost.terrain_chunk_ranges_in`). The craters
themselves (simulation in the debug bridge, uploads, decoration) cost about
4.7 ms per frame while they fall, the impact effects about 2.3 ms; the
frame rate stays at 60 FPS or above. On `test_small` (`after` lower than
`during`: smoke and 100 scorch decals stay) the craters hardly matter. The
worst frames contain the first crater (the map copy in the debug build)
and an upload together with decoration rebuilds.

### Decisions

| Question | Decision | Why |
|---|---|---|
| Where the heights live | In the simulation's `Map`, changed in place after one copy, plus an `i16` deviation raster | Movement, ballistics, sight and drawing read the same samples without special cases; memory stays constant. An overlay of crater records would grow and need lookups everywhere. |
| State hash | Running checksum over craters and changed samples, plus the count | Hashing the raster every tick is too slow; the checksum sees every change in order. |
| New terrain type for churned earth | No, mud (id 4) | The splatting shader has exactly eight weights (two RGBA rasters); mud already slows units. The deviation raster keeps decoration out. |
| Radius on the 2 m raster | At least one raster step | Smaller craters would often touch no sample. Real 7.5 cm craters are smaller; the bowl is shallow enough not to block anything alone. |
| Re-planning | Only when passability changed | Every crater changes costs; re-planning every route through every crater would be expensive and make units hesitate. |
| Cover | Spots plus an all-round chance by stance | At 2 m samples the geometry alone cannot hide a soldier in a 40 cm dip; the spot and roll mechanism of prompt 022 is reused. |
| Limit | 16 craters per tick | Bounded cost per tick like the path and vision budgets; a barrage spreads over ticks. |
| Terrain upload | Collected, spaced by its cost | Godot 4 has no partial texture update; one upload for many craters. |

### Open

- Indirect fire (mortars, howitzers) and HE splash: today only direct-fire
  rounds from 45 mm that miss or attack the ground make craters; larger
  calibers would dig larger craters with the same rules.
- Map objects standing in a crater keep their height (a tree may float a
  few decimeters above a bowl); vision obstacle tops do not move.
- Water surfaces (prompt 014) are not rebuilt when a crater lowers a bank.
- Craters do not fill again; the deviation limit keeps them bounded.
- The map editor (prompt 028) shows no craters (the editor has none).

## Indirect fire and smoke

Prompt 032. Mortars (`inf_mortar_*`, `mortar_heavy`), howitzers
(`gun_howitzer`), self-propelled guns (`self_propelled`) and rocket
launchers (`rocket_launcher`, prompt 062, below) fire their first
catalog weapon indirectly; their other catalog weapons (the crew's small
arms) are ordinary direct weapons. Rules: `data/indirect.toml`
(`sim::IndirectRules`, loaded by `Sim::new` like the crater rules, in the
state hash, not part of `SimConfig`). Code: `crates/sim/src/indirect.rs`
(rules, `IndirectWeapon`, ballistics, snapshot types) and
`crates/sim/src/sim/indirect.rs` (orders, shells, impacts, smoke).

### Data

`CombatType::indirect` (`IndirectWeapon`): caliber, rate, muzzle velocity
(if the catalog has none: `sqrt(range · g)`, the least velocity that reaches
the catalog range in vacuum), range × `range.indirect_fire` (1/8), interval
÷ time factor, and whether the catalog ammunition names smoke or white
phosphorus. Per category the rules add a minimum range (share of the
range), a setup time after moving and a preferred elevation (mortars 60°,
guns 25°). Unit prices count the indirect weapon like a direct one.

### Orders

| Order | Indirect units | Direct units |
|---|---|---|
| `Attack { target }` | fire at the target while the player sees it (any own observer, prompt 018); the order ends when it is lost or destroyed | unchanged |
| `AttackGround { target }` | fire at the point until `Stop`; blind fire (the player does not see the cell) has `blind_pct` dispersion | unchanged |
| `Barrage { target, radius_m, salvos, smoke }` | `salvos` rounds each at uniform random points in the circle; smoke only with smoke ammunition (others ignore it) | ignored |
| none, fire at will | the nearest enemy the unit itself sees in range that HE hurts (infantry, open vehicles) | unchanged |

Indirect orders live in their own state (`Gun`: order, reload, setup,
closing in), not in the direct-fire attack order, so a mortar squad fires
its rifles at will while the mortar follows the order. Out of range a unit
drives up to 90 % of its range; inside the minimum range it holds fire.
Indirect weapons fire only standing and set up (`setup_ms` after every
move; towed guns also not while towed, limbered or unlimbered, prompt 034). `MoveTo` and `Stop` cancel
the order.

### Per tick

1. `fire_indirect` (after the direct fire): orders, closing in, setup,
   automatic targets, new shells. A shell's impact point is the aim point
   plus a triangular error per axis of `mrad_x10` of the real distance (at
   least `min_mm`; × `blind_pct` when blind). Its flight
   (`indirect::flight`) uses the real distance: the charge is chosen so the
   shell arrives at the preferred elevation (`v² = R·g / sin 2θ`), or full
   charge at the matching angle; `t = 2·v·sin θ / g` ÷ time factor. The arc
   is a parabola between muzzle and impact point with the real apex × the
   range factor. No hit test in flight: shells fly over walls and hills.
   `Shot` events (weapon `INDIRECT_WEAPON` = 255) share the projectile id
   counter.
2. `fly_shells` (after the projectiles): at the impact point, in order: a
   direct hit on a vehicle (point in its hull outline; HE penetrates
   `direct_hit_pen_pct` of the caliber against a roof of `roof_pct` of the
   thinnest plate), else an object under the point (an occupied building
   loses an occupant like a hit from a gun, prompt 022; otherwise object
   damage, prompt 020), else the ground (crater, prompt 021). Then
   fragments within `radius_mm_per_mm` × caliber: every soldier outside a
   building rolls `infantry_pct` × (1 − d/R) × stance height ÷ standing
   height × `cover_pct` at a cover spot, then the crater rim (021); open
   vehicles (thinnest plate ≤ `open_armor_mm`) lose
   `open_damage_per_mm` × caliber × (1 − d/R). Squads in the radius are
   suppressed. One `Hit` event per shell with the total damage. Smoke
   shells instead make a cloud and a `Smoke` event.
3. `update_smoke` (before the vision update): clouds drift by the wind of
   the rules, dissolve after `lifetime_ms`, and the oldest goes beyond
   `max_clouds`. Vision cells whose center lies in a cloud get a smoke top
   (`VisionGrid::set_smoke`, ground + `height_mm`): sight lines below it
   end there and targets in a smoke cell are hidden unless next to the eye;
   an observer above the smoke looks over it. Only changed cells are
   written, and the viewsheds that contain them are queued (budgeted like
   every viewshed update).

Clouds, shells, every unit's indirect state and the smoked cells are in the
state hash.

### Snapshot, bridge and Godot

`Snapshot::shells` (position, velocity, impact point, ticks left, owner,
caliber, smoke) and `Snapshot::smoke` (center, radius, age, ticks left).
`SimHost.barrage`, `unit_indirect`, `shell_*`, `shell_blast_radius`, `smoke_*`;
`take_combat_events` adds `type = "smoke"`. `world/artillery_view.gd` draws
shells, an impact ring for the local player's shells and clouds of billboard
puffs that drift with the simulation and fade out; its signal `shell_incoming`
is meant for the whistle (prompt 029). Muzzle flashes and explosions come from
the usual `shot` and `hit` events. Keys: G barrage, T smoke (`selection.gd`).

Impact rings (prompt 084) are a hint seen in passing, not a signal: a ring
appears only in the last 1.5 s of the flight (with the whistle), fades in
over 0.5 s and out over 0.4 s after the impact, without pulsing. It is a
0.25 m line at 30–35 % opacity, lit and without emission (it dims at dusk
and night), pale khaki in the standard palette and the own color of the
palettes for color vision deficiencies (`relation_colors.gd`). Its radius is
0.4 × the fragmentation radius of `[blast]` (`SimHost.shell_blast_radius`,
the core where the hit chance is still 60 % of point blank; 8.4 m at
10.5 cm), never more than the old caliber × 0.1. Rings of the same kind
closer than 0.6 radius are drawn as one, so a salvo on one point is one
ring and not a carpet. Enemy shells get none (sight and whistle suffice).
`markers_enabled = false` hides them for pictures without interface
(`trailer_shots.gd`); `survey_markers_*` shots show them
(`build/marker_shots/`). The area ring of a strike (`strike_view.gd`) stays:
one thin ring per strike that also warns the target side.

### Decisions

| Topic | Decision | Why |
|---|---|---|
| Rules file | `data/indirect.toml`, outside `SimConfig` | The catalog has no minimum ranges, setup times, blast radii or clouds; keeps the replay/config format unchanged. |
| Flight | Real ballistics at real distance, scaled apex, no collision in flight | Real flight times with the time factor; shells cross walls by design. |
| Dispersion and blast in real meters | Sizes are 1:1 | A 16 m blast radius is a size, not a range. |
| Armor | Direct hits only, roof = half the thinnest plate | The catalog has no roof armor. |
| Smoke in vision | A height per cell, not just a flag | Observers on hills see over smoke; same ray code as walls. |

### Rocket launchers (prompt 062)

Category `rocket_launcher` (vehicles: 15 cm Panzerwerfer 42, BM-13-16 on the
ZiS-6, T34 Calliope on the M4A3) fires indirectly like the categories above,
with two category values of `data/indirect.toml` that are 0 elsewhere:

- `ripple_ms`: the first weapon's catalog `count` (tubes or rails) is one
  salvo. A fire order launches the first rocket, the rest follow
  `ripple_ms` (real, time factor applies) apart, then the launcher reloads
  as a whole. The catalog `rate_rpm` of a launcher is the sustained rate
  (salvo plus reloading), so the reload after the last rocket is
  count × 60 s / rate − (count − 1) × ripple (Panzerwerfer 42: 10 rockets,
  5 ticks apart, then 1455 ticks). `Gun::ripple` counts the rockets still to
  come (hashed). A salvo broken off by moving is lost (full reload).
- `mrad_x10`: the category's own dispersion (8 mrad instead of 3).

Every rocket is a shell of its caliber (fragments, craters, fires,
knockdown: no new damage rule). A `Barrage` counts whole salvos, each rocket
at its own random point in the circle. The stock of prompt 036 already
counts a salvo as `count` rounds; a salvo pays one salvo of the stock with
its first rocket. The presentation shows each launch at the model's
`Socket_Rocket_NN`, one rail after the other, with the rocket salvo
effects of prompt 076 (flare, back blast, flame, standing smoke trail; see
"Rocket salvo (prompt 076)" under Effects) and the sound `shot_rocket`.

The Sturmpanzer IV (prompt 062) needs no new rule: its 15 cm StuH 43
(240 m/s, below `heat_below_velocity_mps`) counts as a shaped charge
(240 mm) and a hit costs 2250 points (caliber² × 0.1), two hits destroy a
bunker (2500) or a farmhouse.

### Open

- Provisional values (marked in `data/indirect.toml`): minimum ranges,
  setup times, elevations, dispersion, blast and smoke figures, the fixed
  wind (scaled by the weather since prompt 040).
- Towed guns are fielded since prompt 034 (see "Transport and towing");
  the Soviet catalog entries name no smoke rounds, and tanks have no smoke
  dischargers in the catalog (the 75 mm M89 smoke round of US tank guns is
  not used for direct fire).
- Shells do not hit aircraft or anything in flight; HE from 10 cm fells
  and strips trees around the impact point (prompt 085), other objects take
  only the direct hit; no counter-battery,
  sound ranging or ammunition limits.

## Fire

Prompt 041. Map objects of a fire class burn: woods, hedges, hay, firewood,
buildings and supply dumps; destroyed vehicles burn as wrecks. Rules:
`data/fire.toml` (`sim::FireRules`, loaded by `Sim::new` like the crater and
indirect-fire rules, in the state hash, not part of `SimConfig`). Code:
`crates/sim/src/fire.rs` (rules, public types) and
`crates/sim/src/sim/fire.rs` (the update).

### Rules

Per class (catalog ids): `hit_pct` (a hit sets it on fire), `catch_pct` (a
burning neighbor does), `reach_mm` (how far its flames reach: the gap
between the two outlines), `burn_ms`, `burn_hp_pct` (share of its intact
hit points the fire destroys: timber burns down to a ruin, masonry keeps its
walls and ends damaged), `smoke_radius_mm`, and for dumps `blast_caliber_mm`
with `fuse_ms`. Every class id must have hit points in
`data/destruction.toml`: fire damages objects only through them (cause
`ObjectCause::Fire`), so ruins, felled trees, path and vision cells follow
exactly as for shell hits (prompt 020). Hay, firewood and dumps got
destruction classes for that (they block nothing, so no round hits them).

### Per tick

`update_fire` runs after `fly_shells` and before `apply_destruction`:

1. **Ignition** from this tick's combat events: a `Hit` on an object with
   at least `min_caliber_mm` rolls the object's `hit_pct`; a ground hit of
   at least `burst_min_caliber_mm` rolls it for every burning-capable object
   whose outline lies within `burst_radius_mm` (artillery into a wood rarely
   hits a trunk); a `Destroyed` vehicle (not infantry) becomes a burning
   wreck for `wreck.burn_ms`. Destroyed and burnt-out objects never ignite.
2. **Dumps** whose fuse ran out explode: `Sim::detonate` fires the blast of
   an HE shell of `blast_caliber_mm` at the dump (fragments, open vehicles,
   crater; prompt 032 rules), the dump is destroyed (charred remains) and
   burns on; objects within half the blast radius roll `catch_pct`
   (chain reactions between dumps).
3. **Interval step**, once per `spread.interval_ms` for every fire, on a
   tick chosen by its object index or wreck unit id, so a large fire spreads
   its work over the ticks: damage in proportion to the burn time passed;
   burnt-out fires (into `burnt`); spreading: one roll per burning-capable
   neighbor within reach (destruction buckets, ascending index) unless a
   `firebreak` terrain (road, water) lies on one of three points of the gap;
   soldiers in a burning building lose `occupants.damage_per_s`; smoke:
   every `smoke.interval_ms` a dark cloud (prompt 032 clouds with
   `dark = true`: drift, sight blocking) next to the fire on the downwind
   side, at most `smoke.max_clouds` fire clouds.
4. Ignition from the events steps 2 and 3 caused (wrecks of vehicles the
   blasts destroyed).

Rolls use the match's random generator in index order and only when there
is something to roll for. Burning objects (who started them, start, end,
damage dealt, fuse, next smoke), wrecks, burnt-out objects and fire clouds
are fields of `Sim`: in the state hash and in the save snapshot (prompt
038; `crates/net/tests/fire_snapshot.rs` restores a burning wood with a
running fuse). `Sim::ignite_object` sets an object on fire directly (tests,
tools; a state change like a command).

### Bridge and Godot

`SimHost.fires` (position, flame radius), `fire_keys` (object index, or
`-1 - unit id` for wrecks), `fire_life`, `burnt_objects`, `smoke_dark`;
object states carry `burnt` and the cause `"fire"`. `world/fire_view.gd`
draws flames sized to the object (the `fire` particles of prompt 019), a
dark smoke column and a flickering light within the quality budget
(`fx_fires` × 3 fires with particles, `fx_fire_lights` lights; the light
carries at night); flames die down in the last fifth. Burnt-out objects are
drawn charred: `map_objects.set_charred` sets the instance uniform
`env_char` of the environment shaders (`charred()` in
`env_common.gdshaderinc`); an object destroyed by fire is sooty at once.
Fire clouds are drawn dark by `artillery_view.gd`; wrecks keep their flames
from `effects.gd`.

### Measurements (`just fire-bench`, Apple M1 Max, release)

`town_small`, 40 units per side driving into the town, 6000 ticks (5 min):
without fire 0.09 ms per tick (worst 3.8 ms); with every tenth conifer of
the west wood, a terrace of town houses and all four dumps set on fire (81
fires, up to 673 burning at once, 665 burnt out) 1.7 ms per tick, worst
19.8 ms (budget 50 ms). Before the interval steps were spread over the
ticks the worst tick was 41 ms.

### Decisions

| Topic | Decision | Why |
|---|---|---|
| Damage | Through the hit points of prompt 020 | Ruins, felled trees, paths and sight are solved there. |
| Spread | One roll per neighbor and interval, reach as a gap between outlines, firebreak terrain | Deterministic, cheap with the destruction buckets; a road stops a fire. |
| Explosions | The blast of an HE shell (prompt 032) | No second damage model. |
| Smoke | Clouds of prompt 032, downwind of the fire, capped | Sight blocking for free; the burning object stays visible. |
| Wind | Not in the spread yet | Weather comes with prompt 040. |

### Open

- Provisional values (marked in `data/fire.toml`): every chance, reach,
  burn time, the occupant damage, smoke and the dump blasts.
- Flamethrowers do not ignite anything: they have no simulation weapon yet.
- Wind does not bias the spread; fires do not jump across roads even in a
  gale; infantry is not set on fire and does not avoid fires; burning
  buildings can still be entered; the AI ignores fires.
- Burning objects are not extinguished; wrecks burn as long as the
  presentation's flames, independent of fuel or ammunition on board.

## Air support and anti-aircraft fire

Prompt 033. Players call aircraft on a ground point; nobody steers them.
Rules: `data/air.toml` (`sim::AirRules`, loaded by `Sim::new` like the
indirect-fire rules, in the state hash, not part of `SimConfig`). Code:
`crates/sim/src/air.rs` (rules, aircraft and AA weapons from the catalog,
snapshot and event types) and `crates/sim/src/sim/air.rs` (calls, flight,
flak, strafing, bombs, crashes, reconnaissance).

### Aircraft are no units

`Command::CallAir { mission, target }` (appended at the end of `Command`)
starts a **sortie**: `recon`, `strafe`, `attack` or `bomb`. The aircraft
come from the player's faction (`Sim::faction`, from the first catalog
unit): per faction and mission the first catalog entry of the mission's
category (`aircraft_recon`, `_fighter`, `_attack`, `_bomber`). They live
in their own state (`Air`), not in the unit list, so victory conditions,
pathfinding, separation, production limits, selection and the computer
player's unit groups never see them. The call is ignored unless the
player's faction has the aircraft, the mission's cooldown is over and the
account pays the price (`Economy::pay`; accounts exist in every match).

**Track.** A straight line through the target, in the direction from the
caller's start (center of their spawns) to the target; the aircraft appear
`edge_margin_m` (300 m) beyond the map edge behind the target and leave as
far beyond the opposite edge. Several aircraft fly side by side
(`lateral_m`) or one behind the other (`trail_m`). Speed: the catalog's
cruise speed (else maximum) × `aircraft.speed` (1/2, `data/scale.toml`).
Altitude in game meters above the target's ground: `altitude_m` while
cruising, `attack_m` in the attack zone (guns firing, bombs released),
linear over `dive_m` in between, never less than 20 m above the ground
below. Positions outside the map are fine (no ground queries there).

**Who sees an aircraft.** Its owner, and every player with a living unit
whose sight range (`data/vision.toml`, horizontal) reaches it.
`Sim::snapshot_for` leaves out the others and every enemy sortie.

### Per tick

`Sim::update_air` runs after the shells (the unit grid holds this tick's
positions), before the vision update:

1. Cooldowns and flak reloads count down.
2. **Flight** along the track; aircraft beyond the end leave.
3. **Spotting** (above).
4. **Flak.** Every unit with AA weapons that fires at will (construction
   sites hold fire) fires each reloaded AA weapon at the nearest enemy
   aircraft its player sees within range (3D distance) and above the
   class's `min_altitude_m`: AA guns (`gun_aa_light`, `gun_aa_heavy`) with
   their catalog `range_aa_m`, MGs with `aa = true` with `range_real_m` of
   the rules; × `range.direct_fire` (1/4), capped at `max_range_m`. One
   round per barrel at the rate of `data/combat.toml` (automatic weapons
   25 % of the cyclic rate, ÷ time factor). Chance: `hit_pct` × (1 − d/2R) ×
   wingspan / 12 m; slow aircraft are in range longer and so are hit more
   often. A hit costs `damage_per_mm` × caliber. The gun turns towards the
   aircraft (`aim`). Events: a `Shot` with weapon `AA_WEAPON` (254) for
   muzzle flash and sound, an `AirEvent::Flak` with the burst point.
5. **Shot down** (0 hit points): the aircraft flies on burning and sinks to
   the ground over `fall_ms`, drops no more bombs, then explodes like a
   shell of `crash.caliber_mm_x10` (crater, fragments) and leaves an
   `AirEvent::Crashed` for the wreck; outside the map it just vanishes.
6. **Strafing** (fighters, attack aircraft): from `before_m` before the
   target to `after_m` behind it, a burst every `burst_ticks`. The rounds
   land in a strip `width_m` wide, `lead_m` ahead of the aircraft, as long
   as the distance flown during the burst. Soldiers in the strip (not in
   buildings) roll `soldier_pct` × stance height share (× `cover_pct` at a
   cover spot, craters catch like fragments) and lose the caliber's infantry
   damage; their squads are suppressed. Vehicles roll `vehicle_pct` per
   round: open vehicles (thinnest plate ≤ `open_armor_mm` of
   `data/indirect.toml`) take `open_damage_per_mm` × caliber, armored ones
   the caliber's vehicle damage if the muzzle penetration (combat formula,
   no distance loss) beats the roof (`roof_pct` of the thinnest plate).
   Rear guns of two-seaters do not strafe. Three ground `Hit` events per
   burst for dust, one per damaged unit.
7. **Bombs** (attack aircraft, bombers): `count` bombs of `bomb_kg` per
   aircraft, impact points along the own track around the target,
   `stick_m` apart, released one fall time `sqrt(2h/g)` early; triangular
   error `error_attack_m` / `error_bomb_m` per axis. A bomb explodes through
   the shell code of prompt 032 (`Sim::detonate`: direct hit, object,
   crater, fragments, one `Hit` event) with the caliber
   `caliber_mm_per_cbrt_kg` × ∛kg (SC 250: 155 mm, 31 m fragments), then
   armored vehicles within `near_miss_dm_per_cbrt_kg` / 10 × ∛kg take up to
   `near_miss_damage`, falling linearly. Hits and kills name the shooter
   `AIR_SHOOTER` (`UnitId(u32::MAX)`).
8. **Reconnaissance**: a recon aircraft over the map holds a viewshed from
   its altitude (radius `recon.sight_real_m` × `range.sight`, a ray table
   added by `Vision::ensure_radius`), recomputed when it enters another
   vision cell and added to its player's vision like a unit's
   (`Vision::add_air_shed`); it is removed when the aircraft leaves, falls
   or is shot down. The unit masks of the vision update that follows see
   it.

Cooldowns, sorties, aircraft (position, hit points, crash, remaining
releases, burst timer, who sees it, viewshed summary), bombs and flak
reloads are in the state hash; the whole `Air` state derives
`Serialize`/`Deserialize` and is part of the snapshot of prompt 038
(`crates/net/tests/air_snapshot.rs` saves during an air attack).

### Snapshot, bridge and Godot

`Snapshot::aircraft` (position, velocity, hit points, crashing, firing),
`Snapshot::bombs`, `Snapshot::sorties` (own only when filtered: entry,
exit, target, ticks to target) and `Sim::air_events` (called, flak, strafe,
released, shot down, crashed; not state). Queries: `Sim::air_options`
(price, cooldown, why blocked), `air_types`, `aa_range`.
`crates/bridge/src/air_funcs.rs`: `call_air`, `air_options`, `aircraft_*`,
`bomb_*`, `air_sorties`, `unit_aa_range`, `take_air_events`.

`scenes/world/air_view.gd` draws the aircraft with their catalog models
(gear retracted if the model's extras say so, `ShadowProxy` shadow-only,
enemy tint, propellers turning), a soft shadow patch on the ground along
the sun direction (directional shadows do not reach 30–500 m up), smoke
trails below half hit points and while falling, flak bursts (heavy: black
puffs, light: flash, puff and tracer, MGs: tracers), strafing tracers,
falling bombs (external `Bomb_*` nodes disappear on release), burning
wrecks of crashed aircraft, and the own sorties (dashed route from the
entry, ring and time on the target). New enemy aircraft post "Enemy
aircraft!" (at most every 15 s) and play the alarm notice. Explosions,
dust and craters come from the ordinary `hit` and crater events.
`scenes/hud/air_bar.gd`: the call bar above the command bar (price or
cooldown per mission, the aircraft in the tooltip, Shift + 1…4, left click
on the map calls, right click or Esc cancels).

### Decisions

The full table is in `prompts/done/033-luftwaffe-flak.md`. In short:

| Question | Decision | Why |
|---|---|---|
| Called support or squadrons | Called support with cooldown and price | Aircraft cross a map in 20–60 s; no airfields, fuel or air combat to model; one command, no units. |
| Hit model of flak | Chance per round from range and wingspan, damage per caliber | Rounds through the sky with fuzes and lead would cost much and decide little; uses only catalog values. |
| Strafing | A strip ahead of the aircraft, rolls per soldier and per round | The projectile system needs a shooter in the unit list; the strip gives the known effect (infantry and soft vehicles suffer, tanks hardly). |
| Bombs | Shells of an equivalent caliber plus near misses | One explosion rule for shells, bombs and crashes; the catalog keeps the real bomb mass. |
| Rules location | `data/air.toml`, outside `SimConfig` | Like 032: no replay/config format change. |

### Open

- Provisional values (marked in `data/air.toml`): prices, cooldowns,
  altitudes, formation, strafing and bomb figures, flak chances, ranges and
  caps, aircraft hit points.
- No air combat (fighters do not intercept), no airfields, no rockets
  (HVAR, RS-82), no smoke or supply drops; one call per mission at a time.
- Flak shoots only at aircraft, never at bombs; shells (032) and direct
  fire do not hit aircraft; aircraft guns do not hit map objects.
- Wrecks of crashed aircraft are drawn only, they block nothing. Engine,
  flak, strafing and crash sounds since prompt 053 (`air_view.air_event`,
  see [Audio](#stage-2-acoustics-and-voices-prompt-053)).
- The approach direction follows the caller's start; players cannot pick
  it.

## Heavy strike

Prompt 054. Rules: `data/strike.toml` (`sim::StrikeRules`), state and update:
`crates/sim/src/sim/strike.rs`, public types: `crates/sim/src/strike.rs`.
Design and numbers: GAME_DESIGN.md "Heavy strike".

- **Call:** `Command::HeavyStrike { target }` (the last variant of the
  enum). It needs a completed, unsold rocket battery position of the player
  (`building`, a base building of `data/economy.toml`), the unlock time
  (`unlock_s` after the start), no cooldown, no other strike of the player
  under way, the faction's rocket caliber and the price, which is paid at
  once. `Sim::strike_option` says why not (`StrikeBlock`).
- **Warning:** `warning_s`. The strike is public (`Sim::strikes`,
  `StrikeEvent::Called`): every player's game shows the smoke and plays the
  siren. After every vision update `Sim::reveal_strikes` shows the battery
  position to every player who is not allied with the owner
  (`Vision::reveal`, like muzzle flashes at night), so the enemy can find
  and destroy it. A position that is destroyed or sold before the first
  rocket leaves calls the strike off (`StrikeEvent::Cancelled`); one lost
  during the salvo fires no more rockets.
- **Salvo:** `Sim::update_strikes` runs after the indirect fire. Every
  `ripple_ms` a rocket leaves the position (`Sim::launch_shell` in
  `indirect.rs`): an ordinary high-explosive shell of the faction's caliber
  aimed at a uniform random point within `radius_m` (rejection sampling with
  the simulation's generator, like a barrage), flying distance /
  `speed_m_per_s` (at least `min_flight_ms`) over an arc of `apex_pct` of the
  distance. From then on it is a shell: `fly_shells` moves it, and on impact
  the usual direct hit, object, crater, fire, fragment and knockdown rules
  apply. The shooter of its hits is the battery position.
- **Hash:** the strike state (rules, cooldowns, strikes, next id) is hashed
  only once a strike was called, so matches without strikes keep their
  hashes. It is saved with the rest of `Sim` (prompt 038); a copy taken in
  the warning time continues identically (`tests/strike.rs`).
- **Events:** `Sim::strike_events` also carries
  `StrikeEvent::AmmoExplosion` (prompt 048's ammunition explosion of a
  destroyed vehicle), so the presentation can blow the turret off.
- **Bridge:** `SimHost.call_strike`, `strike_option`, `strikes`,
  `take_strike_events` (`crates/bridge/src/strike_funcs.rs`).
- **Presentation:** `world/strike_view.gd` (marker smoke, area ring, siren,
  countdown, enemy battery marker, call bar, launch frames for the
  borrowed ammunition dump model of the position: three racks side by
  side with a `Socket_Rocket_NN` per rail, turned towards the target while
  a strike is under way), `world/rocket_effects.gd` (launch, flame and
  smoke trail of every rocket, prompt 076) and `world/blast_effects.gd`
  (see "Effects", big blasts).
- **Debug:** `Sim::debug_rocket` / `SimHost.debug_rocket(shooter, x, z,
  caliber_mm)` launch one rocket like a strike rocket from any unit now
  (tools only: salvo pictures and the effects bench; refused in network
  matches; never called by commands or the AI, so hashes of real matches
  are unchanged).

## Transport and towing

Prompt 034. Squads ride in trucks and halftracks and on tanks (tank
desant); trucks and halftracks tow guns. Rules: `data/transport.toml`
(`sim::TransportRules`, loaded by `Sim::new` like the indirect-fire rules,
in the state hash, not part of `SimConfig`). Code:
`crates/sim/src/transport.rs` (rules, `TransportType` from the catalog) and
`crates/sim/src/sim/transport.rs` (orders, tasks, following, losses).
Decisions and provisional values: `prompts/done/034-transport-schleppen.md`.

### Data

- **Seats:** catalog field `passengers` (trucks, halftracks); tanks carry
  `riders` of the rules (light 6, medium and heavy 10). Seats count living
  soldiers; a squad mounts only as a whole.
- **Towing:** a carrier category with `tows` pulls a gun if its catalog
  `power_hp ≥ min_hp_per_tonne × (its mass + gun mass)` (8 hp/t). Towing
  sets the tractor's speed share to `mass / (mass + gun mass)`.
- **Guns** (`Group::Gun`) are mobile now: `UnitType::from_catalog` gives
  them the rules' `manhandle_kmh_x10` × time factor (Pak 2 km/h real,
  howitzer 1 km/h). They fire only standing, not while towed, being
  limbered or unlimbered; indirect guns then need their setup time too.
- Times (`embark_ms`, `disembark_ms`, `limber_ms`, `unlimber_ms`) are real
  and divided by the time factor.

### Orders and tasks

| Command | Effect |
|---|---|
| `Embark { units, carrier }` | Squads walk behind the own vehicle (planning again when it moves), then mount after `embark_ms` while both stand |
| `Disembark { units }` | Listed vehicles unload everybody, listed passengers get off; `disembark_ms` while the vehicle stands, then behind it (sides or front if blocked) |
| `Tow { units, gun }` | The nearest listed vehicle that can pull the gun and tows nothing drives to it, limbers it (`limber_ms`) and hitches it |
| `Unlimber { units }` | Towed guns of listed vehicles (or listed guns) are unhitched after `unlimber_ms` while the tractor stands, facing the way the tractor drove; a hitch under way is called off |

A unit has at most one task (`Transport::tasks`, by unit id). Move, attack,
stop, garrison, barrage, patrol, embark and tow orders call off walking to a
vehicle or driving to a gun, not a dismount or an unhitch under way.

### Passengers and towed guns

The links live on the unit (`Unit::link`: `Aboard(vehicle)` or
`Towed(tractor)`); ids, per-index arrays and the unit list stay as they are.

| | Passenger (`Aboard`) | Towed gun (`Towed`) |
|---|---|---|
| Position | the vehicle's center, soldiers too | trails the hook: `hook + dir × (gap + length/2)`, `dir` from the hook towards the gun (a trailer), muzzle away from the tractor |
| Unit grid (hits, separation, targets) | not listed: cannot be hit or chosen | listed (it can be hit), no separation |
| Vision | seen only by its owner (`seen` mask), keeps its own viewshed | normal |
| Orders | only `Disembark` (`owned_indices` skips passengers) | no moves (`mobile_indices`); fire mode and `Unlimber` |
| Fire | none | none |
| Squads | `update_squads` skips it, no soldiers in the snapshot | – |

Order within a tick: `update_transport` after the movement (tasks, then
`carry_along`), `transport_losses` after projectiles and shells:

- every Hit event on a loaded carrier (and hit points lost without one:
  shell fragments) kills one random passenger with `hit_pct` (damaging
  rounds) or `bounce_pct` (rounds without damage: tank riders under fire);
- a destroyed carrier kills every passenger soldier with `destroyed_pct`;
  the survivors jump off at once; a destroyed tractor drops its gun, which
  is not ready until its unlimber time is over;
- links of destroyed passengers and guns are dropped.

Tasks, links and the hit points of loaded carriers are in the state hash
(after the order queues) and, as fields of `Sim`, in saved games (prompt
038; `crates/net/tests/snapshot_transport.rs`). Tests:
`crates/sim/tests/transport.rs`.

### Snapshot, bridge and Godot

`UnitSnapshot::carrier`, `passengers`, `towing`, `loading`; `Sim::seats`,
`can_tow`, `transport_load`, `is_towed_gun_type`. `SimHost.embark_units`,
`disembark_units`, `tow_gun`, `unlimber_units`, `unit_carriers`,
`unit_passengers`, `unit_towing`, `unit_loading`, `unit_seats`, `can_tow`,
`is_towed_gun` (`crates/bridge/src/transport_funcs.rs`).
`selection/transport_orders.gd` turns right clicks and the keys J and O
into orders; `world/transport_view.gd` hides riding squads and seats one
rifleman figure (pose `kneel`) per passenger at the model's
`Socket_Passenger_NN` empties, or in two rows over the rear half of the
model's bounding box for tanks (they have no rider sockets). Towed guns
need nothing extra: the simulation places them at the hook (catalog sizes
1:1, `Socket_Tow` exists on every vehicle and gun model).

### Decisions

See the table in `prompts/done/034-transport-schleppen.md`: seats from the
catalog, towing from engine power, rules file outside `SimConfig`, the AT
gun position of prompt 023 replaced by produced towed guns, passengers as
linked units with the same id.

### Open

- Provisional values in `data/transport.toml` (times, losses, rider
  counts, towing threshold); the seats of the Opel Blitz and GMC CCKW are
  estimates.
- Rider sockets on the tank models (the kit has none; Godot spreads riders
  over the deck); the `mount` / `dismount` clips of prompt 042 are not
  played yet (figures appear seated, soldiers appear behind the vehicle).
  Not checked in a rendered frame (headless tests only).
- Passengers do not fire from open vehicles; the gun crew is not modelled
  as passengers (it rides with the gun); bridges check the tractor's mass
  only; no tractors for the 88 mm and 90 mm AA guns in the catalog; no
  path profile for a tractor with a gun.

## Weather, mud and time of day

Prompt 040. A match has one weather (`clear`, `overcast`, `rain`, `snow`,
`fog`) and one time of day (`morning`, `day`, `evening`, `night`) for its
whole length: `SimConfig::weather` (`sim::WeatherSetup`), by default the
map's (`map.toml`, [MAPS.md](MAPS.md#weather-and-time-of-day)), chosen in the
setup screen (`SimHost.weather` / `daytime`). The rules are
`data/weather.toml` (`sim::WeatherRules`, compiled in, loaded by `Sim::new`
like fire and craters); the state `crate::sim::weather::WeatherState`.

**Clear weather by day is neutral.** Every factor is 100 % there (the parser
refuses rules that change the default), the ground is dry, and the weather
state is only fed into the state hash when the setup is not the default:
matches without weather keep their hashes (the golden hash did not change).

### Rules

| What | How | Values ([provisional]) |
|---|---|---|
| Sight | every sight range × weather × time of day; applied once in `Sim::new` through the sight factor of the vision rules (`SimConfig::effective_vision`), so unit viewsheds, recon aircraft (033) and the AI (024) follow | rain 70, snow 60, fog 35, overcast 90; morning/evening 80, night 45 % |
| Soft ground | one softness per match (0..100 %, kind wet or snow); speed = terrain speed × (100 % − loss(gear, kind) × softness(terrain, kind) × level), at least 8 % | losses wet/snow: wheeled 80/60, half-track 50/35, tracked 35/25, foot 30/40 %; terrain wet/snow: dirt track 100/80, grass and tall grass 70/100, road 0/30, mud 100/60, sand 20/100, snow 40/0 % |
| Rain | the ground starts half soaked and gets 10 % softer every 120 game seconds up to 100 % (after ten minutes) | snow: 100 % from the start; fog: 20 % wet; clear/overcast: dry |
| Running gear | catalog category (`[[ground.gear]]`), single ids override (M3A1 Scout Car: wheeled); towed guns roll on wheels; aircraft and buildings are not slowed | |
| Muzzle flash | at night a unit that fires a direct-fire weapon is seen for 2 game seconds by every player with a unit within 2000 m real (500 m game), without a line of sight | |
| Wind | scales the smoke drift of `data/indirect.toml` | rain 150, snow 120, overcast 130, fog 20 % |
| Fire (041) | catch chance × `fire_catch_pct` (wet wood) × (1 + 0.5 · cos(wind, way to the neighbor) · wind strength) | rain 40, snow 30, fog 70, overcast 90 % |

Wheels in mud "get stuck" as a crawl (a truck on a soaked mud field drives at
8 % of its terrain speed), not by a recovery mechanic; roads stay fast in the
rain, so convoys keep to them.

### Per tick

- Movement: `move_unit` scales the terrain speed by the unit's soft-ground
  factor of the terrain it stands on (`PathGrid::ground_pct`).
- Path costs: every profile of the path grid has the soft-ground factors per
  terrain id (`PathGrid::set_ground_pct`); `terrain_times` scales the terrain
  speeds with them, so the cell costs follow and A* prefers roads in the
  rain. At the start all costs are computed at once; after a rain step
  `Sim::update_weather` (destruction phase) recomputes 8 path rows per
  tick (`PathGrid::recost_rows`; a 4 km map takes 63 ticks, at most 1.6 ms each). Passability does
  not change, so components stay; routes already planned are kept (new paths
  use the new costs), which avoids flooding the path queue.
- Muzzle flashes: `update_weather` notes the shooters of this tick's `Shot`
  events; after the vision update `Sim::reveal_flashes` sets their `seen`
  bits for the nearby players (`Vision::reveal`), so they can be fired at in
  the next tick.

### Snapshot, bridge and Godot

`WeatherState` (level, next step, the row band being recomputed, flashes) and
the soft-ground factors in the path grid are fields of `Sim`: saved games
(038) restore them exactly (`crates/net/tests/weather_snapshot.rs`). Replay
format 8 and protocol 4 carry the new `SimConfig` field.

Bridge: `SimHost.weather` / `daytime` (choice of the next single-player
start, empty = the map's), `weather_info()` (weather, time of day, ground
kind and softness, sight percent, wind), `weather_names()`,
`daytime_names()`; map editor `set_map_weather` / `map_weather`.

Godot (`world/weather_look.gd`, presentation only): `WeatherLook.adjust`
turns the biome preset into the weather's look (cloud cover dims the sun
and greys the sky, rain and fog thicken the haze, snow adds snowfall and
snow cover, morning and evening bring a low warm sun from the east or west,
night a weak cold moonlight with a blue sky); `atmosphere.gd` applies it
(`set_weather`). Rain streaks fall in a box around the camera focus (count
from the snow budget of the quality level); the terrain shader's `wetness`
uniform darkens the ground, makes it glossy and fills puddles on flat soft
ground, following the soaking of the match; the ambience (030) plays rain.
Fires (041) and muzzle flashes light up the night through their existing
lights.

### Measurements

See the result of prompt 040 in `prompts/done/040-wetter-schlamm-tageszeit.md`.

### Decisions

- One weather per match, no weather change during it; the ground changes.
- Softness per terrain type, not per cell (craters already make mud cells).
- Running gear from the catalog category, no new catalog field (the values
  are game rules, not historical data).
- Routes are not replanned when the ground softens.

### Open

- Weather changing during a match; flares at night; per-cell wetness (low
  ground wetter), tire tracks in the mud (decals), splashes.
- LAN and online lobbies offer no weather choice (they use the map's).
- Aircraft fly in any weather and at night.
- The look is checked in a few rendered frames only (rain by day, clear
  night); the 2 ms budget is measured on the CPU (the rain moves on the GPU).

## Supply and repair

Prompt 036; the game rules are in [GAME_DESIGN.md](GAME_DESIGN.md#supply-and-repair-provisional),
the numbers in `data/logistics.toml` (`sim::LogisticsRules`, loaded by
`Sim::new`, hashed; not part of `SimConfig`, like `data/transport.toml`).

- **Stocks:** every unit carries a `Stock` (ammunition and fuel in
  millionths of a full load, `sim/logistics.rs`). What a full load is comes
  from the catalog per unit type (`sim::LogisticsType::from_catalog`): the
  salvos of the main weapon (`ammo_carried / count`, else a default per
  group) and the road range converted by `[range] fuel` of
  `data/scale.toml` (`Scale::fuel_range_m`). Types without a stock keep it
  full; buildings and fortifications have none.
- **Consumption:** `aim_and_fire` skips every weapon while the stock is
  short of a salvo and charges a salvo when weapon 0 fired (direct main
  weapons); `fire_indirect` does the same per shell. `move_units` charges
  fuel by the distance moved. `halt_stranded` (before the movement) stops
  vehicles without fuel or at or below `immobile_pct` of their hit points
  and clears their queued waypoints and patrols.
- **Resupply and repair:** `update_logistics` runs once per game second
  after the economy. Sources are units whose type name (a completed base
  building) or category matches a `[[supplier]]` or `[[repairer]]`;
  receivers of the source's player and its allies (`Teams::allied`) within
  the radius gain a share of a full load, paid by their own player through
  `Economy::pay_share` (exact to a part of a resource unit). Repair adds hit
  points to vehicles and towed guns; the best repairer in reach counts.
- **Buildings:** `Command::Repair` toggles a repair order (a sorted list of
  unit ids); each second the building gains hit points while its player
  pays. `Command::Sell` goes through `Sim::remove_structure`
  (`sim/economy.rs`): refund, queue refund, hit points 0, ground freed with
  `set_path_obstacle(.., false)`. Sold ids are kept sorted and left out of
  every snapshot, so the presentation drops the node like a unit out of
  sight; no `Destroyed` event is sent.
- **State and saves:** stocks (per unit), repair orders and sold ids are in
  the state hash (`hash_logistics`) and, as serde fields of `Sim`, in saved
  games and replay snapshots (prompt 038); the rules hash of the snapshot
  header rejects older saves.
- **Presentation:** `UnitSnapshot::stock` (`StockView`: percent or `None`,
  immobilized) and `Sim::supply_sources` reach Godot through
  `bridge/src/supply_funcs.rs` (`unit_ammo`, `unit_fuel`, `unit_immobile`,
  `supply_radius`, `repair_building`, `sell_building`). The unit panel
  shows the bars and warnings, `hud/supply_rings.gd` the radii of selected
  sources, `world/base_hud.gd` the Repair and Sell buttons.

## Experience

Prompt 050; the game rules are in [GAME_DESIGN.md](GAME_DESIGN.md#experience-and-ranks-provisional),
the numbers in `data/experience.toml` (`sim::ExperienceRules`, loaded by
`Sim::new`, hashed; not part of `SimConfig`, like `data/logistics.toml`).

- **State:** every unit has a `Veterancy` (`sim/experience.rs`):
  experience in hundredths of a point (small hits add up exactly), kills
  and vehicle kills. The rank is derived from the points, never stored.
- **Gain:** `update_experience` runs after `update_logistics`, when the
  tick's `CombatEvent`s are complete. `Hit` events of kind `Unit` with
  damage between enemies (`Teams::allied`) are grouped per target; the
  target's real loss this tick (its hit points at the end of the last tick,
  kept in `Experience::last_hp`, minus now; no overkill) is shared out by
  each shooter's damage. `Destroyed` gives the kill; survivors of a damaging
  hit and (once per game second) suppressed squads get their points.
  Shooters that are not units (`MINE_SHOOTER`, `AIR_SHOOTER`) are skipped.
- **Bonuses:** `fire` and `fire_squad` (`sim/combat.rs`) scale the spread
  by the rank's `dispersion_pct` and the reload of the fired weapon by
  `reload_pct` (rounded, at least one tick); `near_miss`, shell blasts and
  air attacks scale the suppression time by `suppression_pct`. Indirect
  fire reload and anti-aircraft fire are unchanged.
- **State and saves:** rules and every unit's points and kills are in the
  state hash (`hash_experience`) and, as serde fields, in saved games; the
  promotions of the last tick and `last_hp` are derived and not hashed.
- **Queries:** `UnitSnapshot::experience` (`ExperienceView`: points, rank,
  kills), `Sim::unit_experience`, `Sim::promotions` (last tick),
  `Sim::experience_rules`; `Sim::set_unit_experience` for tests and the
  debug function `SimHost.debug_set_experience` (no command: not in
  replays). The AI reads the rank through `OwnUnit::rank`.
- **Presentation** (`bridge/src/experience_funcs.rs`): `unit_ranks`,
  `unit_vehicle_kills`, `unit_experience(id)`, `rank_names`. The unit panel
  draws the insignia (`hud/rank_badge.gd`), `world/veterancy_view.gd` the
  kill rings under the model's `Gun` node (gun origin to `Socket_Muzzle`)
  and the promotion messages, `infantry_view.gd` re-dresses soldiers when
  their squad's rank changes (`SoldierLook.apply(.., rank)`).

## Wounds and care

Prompt 049; rules in `data/care.toml` (`sim::CareRules`, loaded by `Sim::new`,
hashed once anything happened; not part of `SimConfig`), behavior in
`sim/care.rs`, the game rules in [INFANTRY.md](INFANTRY.md#wounds-and-buddy-aid-prompt-049).

- **One hook for all damage:** `update_care` runs after `update_experience`
  and compares every soldier's hit points with `Wound::seen_hp`, so direct
  fire, fragments, aircraft, fire and mines wound alike without changes at
  the damage sites. It also reads the tick's `Destroyed` events for crews
  that bail out (`spawn_unit` with the faction's crew type).
- **Movement:** soldiers the care update places (`Wound::held`: lying,
  dragged, on a stretcher, helping) are skipped by `follow_places`;
  `care_speed` runs right after `update_stances` and caps a squad's
  `speed_pct` (limping, carrying, stretcher).
- **Combat:** `fire_squad` skips men who may not fire (`Wound::can_fire`),
  `unit_hit` gives lying men the prone box, `aim_point` picks a man who is
  up (`Squad::aim_soldier`) and aims low at lying men.
- **Victory:** `alive_units` leaves bailed-out crews out.

## Field works

Prompt 035. Engineer squads (catalog category `inf_engineer`) lay
minefields, barbed wire and anti-tank obstacles, dig foxholes and trenches,
clear mines and obstacles, and demolish or repair bridges. Rules:
`data/works.toml` (`sim::WorksRules`, loaded by `Sim::new` like the
transport rules, in the state hash, not part of `SimConfig`). Code:
`crates/sim/src/works.rs` (rules, public kinds and views) and
`crates/sim/src/sim/works.rs` (orders, sites, jobs, mines, obstacles,
digging, bridges). Decisions and provisional values:
`prompts/done/035-pioniere-minen-stellungen.md`. Prompt 052 extends this
(lines of obstacles, vehicle pits, camouflage nets, trench strongpoints):
new kinds are appended to `WorkKind`, new pieces go through the same sites
and jobs.

### Orders, sites and jobs

| Command | Effect |
|---|---|
| `Lay { units, kind, from, to }` | AT or AP mine sections, wire or hedgehogs along the line (a point if `from == to`) |
| `Dig { units, kind, from, to }` | A foxhole at `from` or a trench along the line |
| `Clear { units, target }` | Clear the known mines, wire and hedgehogs within `clear_radius_mm` |
| `DemolishBridge { units, bridge }` / `RepairBridge { units, bridge }` | Blow up an intact bridge (index into `Map::bridges`) / rebuild a destroyed one |

Only engineer squads among `units` take these orders. A line is cut into
**pieces** (`piece_mm`: wire and trench 4 m, a hedgehog every 3 m; mines in
sections whose size and count come from the catalog entry
`<faction>_minefield_at|ap`, 20 m × 20 m with 20 AT or 40–50 AP mines; a
point order faces the engineers' side). Every piece is a **site** paid for
when it is placed (`cost`, the account of prompt 023; pieces the player
cannot pay are dropped, at most `max_sites` open sites per player) and
built in `build_s` game seconds by one squad; every squad within `reach_mm`
adds one tick of work per tick. All squads of an order work the sites in
order, starting at the end nearest to them. A site nobody works on or walks
to for `abandon_s` is given up (refunded if no work was done).

Each engineer has at most one **job** (`Works::jobs`, by unit id): build a
list of sites, clear around a point, or work on a bridge. Move, attack,
stop, garrison, patrol, waypoint and embark orders end it. Squads walk to
their work with ordinary path requests (a few retries if no path is found).

### Mines

Mines are not units: `Works::mines` (by id) with kind, owner (`None`:
placed with the map) and a bit mask of the players who found one, plus a
bucket index (16 m) for the trigger checks. Every tick (after the unit grid
is rebuilt behind the movement):

- a **vehicle** whose hull (the blast body of prompt 032) covers a mine and
  whose catalog mass reaches the trigger (`at_trigger_kg` 150,
  `ap_trigger_kg` 5) sets it off; a **soldier** (counted as `soldier_kg`
  100) within `ap_reach_mm` of an AP mine sets it off. Mines never go off
  under units of their owner's team (prompt 037).
- **Anti-tank:** vehicles with armor up to `open_armor_mm`
  (`data/indirect.toml`) are destroyed; others lose `at_damage_pct` of
  their hit points and their track: speed 0 (no move orders; turret and
  weapons work) until the crew fitted it after `track_repair_s`; a crater
  of `at_crater_caliber_mm_x10`. **Anti-personnel:** the soldier falls,
  then the blast of `ap_caliber_mm_x10` (fragments, a vehicle above it, the
  crater) through `Sim::detonate`. Combat events name
  `sim::works::MINE_SHOOTER` as the shooter.
- An explosion shows the victim's player the mines within `reveal_mm`;
  engineer squads find hostile mines within `detect_mm` (every 10 ticks) and
  the whole clear radius when they start clearing. Knowledge counts for the
  whole team.

**Visibility:** mines never appear in a `Snapshot`. `Sim::mines_for(player)`
returns the mines the player's team knows (own, allied, found); `PlayerView`
(prompt 024) gets exactly these, so the enemy's view never contains hidden
mines. `Sim::all_mines` is for tests and tools.

**Avoiding known minefields per player:** the path grid stays shared. Every
search gets a sorted list of cells to avoid when it starts
(`Pathfinder::run_avoiding`, kept in the resumable search): the cells of
the hostile mines the requesting unit's team knows and their neighbors (a
hull reaches over the edge of the cell its center drives through). A* pays
`AVOID_FACTOR` (10) times the step cost there, and the smoothing compares
shortcuts with the same weights, so routes go round known fields but a unit
inside one still finds its way out. When a team learns of mines, units
whose route touches them plan again. Formation offsets of a group are not
checked against the list (members may brush the edge of a field).

### Obstacles and digging

- **Wire** blocks small units, **hedgehogs** large ones: two path flags
  (`BLOCKED_WORK_SMALL`, `BLOCKED_WORK_LARGE`, the last free bits of the
  cell flags) set by `PathGrid::mark_works_in` from every finished
  obstacle overlapping the changed cells; components and routes are updated
  like for destroyed objects. Moving vehicles of at least `crush.wire_kg`
  flatten the wire piece they drive over.
- **Foxholes and trenches** lower the height field through the crater
  machinery of prompt 021 (`Sim::deform`, `CraterPlan`): `steps` steps of
  `depth_mm` with a parapet of `parapet_mm` on the neighboring samples; the
  dug samples turn to mud. Path cells, vision cells, unit heights, the
  `MapDelta` of saved games and the terrain in Godot (crater events with
  caliber 0, no sound) follow. On the 2 m raster a sample may lie only
  600 mm below its neighbors before the cell is too steep for infantry, so
  holes are 0.5 m deep [provisional]; prompt 052 cuts real trenches. Dug
  samples (`Works::dug`) give a standing squad cover like a crater, with the
  stance values of `[dig]` (60/75/85 %) instead of the crater's.
- **Bridges:** engineers work at the nearer end of the deck; demolishing
  calls `Sim::set_bridge_intact(false)` and detonates a charge of
  `demolish_caliber_mm_x10` on the deck, repairing restores the deck. The
  price is paid when the work is done (the job waits for the money).

### Map editor minefields

Objects `fort_mine_at` / `fort_mine_ap` (variant `Mine`, `data/objects.toml`)
are placed like other objects in the editor (group "Fortifications"). When
a match starts, each becomes a hidden section of **neutral** mines
(`map_at_count` / `map_ap_count`, `section_mm`, the object's heading), laid
out from a hash of the object (not the match's random numbers). Matches do
not draw these objects (`map_objects.show_minefields`). `twin_meadow` has
an AT and an AP section in the middle (the LAN self-test drives into them).

### State, snapshot, bridge and Godot

Sites, mines, jobs, thrown tracks, dug samples and pending dig steps are in
the state hash (last) and, as fields of `Sim`, in saved games
(`crates/net/tests/snapshot_works.rs`). Views: `Sim::mines_for`,
`works_for` (finished works of everybody like map objects, sites of the own
team), `tracks_off`, `is_engineer_type`, `engineer_job`, `place_minefield`
(tools, debug). `SimHost.order_works`, `clear_works`, `order_bridge_work`,
`is_engineer`, `engineer_job`, `known_mines`, `field_works`, `tracks_off`,
`bridges_intact`, `bridge_objects`, `work_cost`, `debug_minefield`
(offline only) in `crates/bridge/src/works_funcs.rs`.
`world/works_view.gd` marks the known mines (own and allied yellow, found
hostile ones red), draws finished wire and hedgehogs with the models of
prompt 007, own sites as translucent outlines with dust while engineers
dig, a thrown track beside immobilized vehicles, and hides demolished
bridges in the map objects. `selection/engineer_orders.gd` and eight
command bar buttons (shown while engineers are selected) give the orders.
Tests: `crates/sim/tests/works.rs`, `tools/works_self_test.gd` (a debug
minefield in front of an own tank), the editor self-test (a minefield
placed, hidden mines in the match), the LAN self-test (tanks drive into
the minefields of `twin_meadow` without a desync).

### Decisions

See the table in `prompts/done/035-pioniere-minen-stellungen.md`: own
simulation layer instead of runtime map objects, pieces per line, avoid
lists instead of a per-player grid, mines never under the own team, digging
through the height field, editor minefields neutral.

### Open

- Provisional values in `data/works.toml` (costs, times, damage, trigger
  masses, cover, depth, avoid factor).
- Trenches are shallow (2 m raster) and look like ditches; prompt 052 makes
  real trenches, lines of obstacles, vehicle pits and camouflage nets.
- Mines cannot be destroyed by artillery; engineers clear without risk;
  no lanes through own fields; wire sites have no model (an outline); the
  editor shows a minefield as a single mine model.
- Thrown tracks are repaired by the crew only (prompt 036 adds repairs);
  not checked in a rendered frame (headless tests only).

### Positions (prompt 052)

Positions tier 2 build on the field works: new kinds appended to `WorkKind`
(and `LayKind` / `DigKind`, no new `Command` variants), rules in
`[positions]` of `data/works.toml` (`sim::PositionRules`, part of
`WorksRules`), code in `crates/sim/src/positions.rs` (rules, strongpoint
layout) and `crates/sim/src/sim/positions.rs` (the update). Decisions:
`prompts/done/052-stellungen-stufe2.md`.

| Order | Kind | Effect |
|---|---|---|
| `Dig` | `Trench` | now a **zigzag**: every second corner `zigzag_mm` (1.5 m) beside the line, the ends and the middle on it |
| `Dig` | `CommTrench` | a straight communication trench (4 m pieces) |
| `Dig` | `VehiclePit` | a pit (`pit_radius_mm`); a vehicle of the team standing `enter_s` within `pit_reach_mm` is **dug in** |
| `Dig` | `MgNest` / `Dugout` | a dug nest / dugout at a trench edge (`edge_mm` of a finished trench of the team or a strongpoint) |
| `Lay` | `CamoNet` | a camouflage net over a point (`net_radius_mm`) |
| `Lay` | `Bunker` | at a trench edge; `to` gives the facing; finished, it becomes the faction's fortification of `positions.bunkers` as a completed structure |
| `Lay` | `Teeth` | concrete teeth along a line, one per `piece_mm` (1.5 m): block large units like hedgehogs, never crushed, 30 s to clear |

- **Firing positions:** `Sim::trench_spots` gives standing squads the
  corners of finished fire trenches and MG nests and points every
  `spot_spacing_mm` along the walls, `wall_mm` towards the squad's facing;
  `choose_spots` (prompt 022) takes them like crater spots.
- **Parapets:** a direct-fire round at a soldier on a sample engineers dug
  is caught by the rim (prompt 021/035 cover roll) only when its shooter is
  farther than `close_mm` and it falls flatter than `steep_pct`
  (`Sim::trench_exposed`); MG nest and dugout samples cover with
  `nest_pct` / `dugout_pct` instead of `[dig]` (also against fragments,
  so dugouts protect from artillery).
- **Dug-in vehicles:** hits below the turret line of the hit zones of prompt
  017 (`turret_height_pct`) strike the parapet (a ground hit); shooters aim
  at the turret. Driving off ends it at once; digging in again takes
  `enter_s`. `Sim::dug_in`.
- **Camouflage nets:** a standing unit of the net owner's team within the
  net that has not fired for `fire_reveal_s` is seen by enemies only when one
  of their ground units within `spot_mm` sees it (`Sim::restrict_camouflaged`
  after the vision update, `Vision::restrict`); reconnaissance aircraft
  (prompt 033) never reveal it. `works_for` hides finished enemy works under
  a net (and the net) the same way. Shots of units under a net are noted from
  the combat events.
- **Strongpoints:** map markers named `strongpoint_<name>` (prefix
  `strongpoint_prefix`) become neutral, finished works in the first tick
  (`crate::positions::strongpoint_layout`): a zigzag fire trench ring of
  `ring_sides` sides at `ring_mm`, an MG nest at every second corner, two
  communication trenches to two dugouts beside the center and a wire ring
  at `wire_mm` with two gaps. Owner `NEUTRAL_OWNER` (no player): everybody
  sees and uses them. `Sim::place_strongpoint` (tools, debug),
  `SimHost.debug_strongpoint`.
- **Dig steps** are applied one after the other within a tick (each plan
  relative to the ground the previous one left), so pieces that share
  samples at their joints do not dig them twice.
- **State** (`Positions` in `Works`, `#[serde(default)]`: dug-in vehicles,
  last shots under nets, nest/dugout samples, works a shell struck) is in
  saved games (`crates/net/tests/snapshot_positions.rs`) and enters the
  state hash with the `[positions]` rules only once a match used anything of
  this prompt (a positions kind, a fire trench, a strongpoint marker), so
  the golden hash did not change.
- **Bridge and Godot:** `positions_funcs.rs` (`dug_in_units`,
  `camouflaged_units`, `damaged_works`, `strongpoint_layout`,
  `debug_strongpoint`, `debug_work`); `work_cost` also returns the piece
  length. `world/trench_builder.gd` builds the dug works as procedural
  meshes (see "Presentation" in `prompts/done/052-*.md`): duckboards, plank
  or wattle revetment with posts, irregular earth parapets on both sides,
  sandbags at the zigzag corners, a trodden path behind fire trenches, torn
  parapets and scattered bags where a shell struck; mitered joints. The
  height field keeps the 0.5 m cut (2 m raster); the meshes make the trench
  look about 1.1 m deep. Camouflage nets (`fort_camo_net`) and teeth
  (`fort_dragon_teeth`) are catalog models; dug-in vehicles get an earth
  collar. `engineer_orders.gd` gives the new orders; while a line is drawn,
  `works_view.gd` shows it with length, pieces and the total price.
  Screenshots: `just position-shots build/positions`.
- **Tests:** `crates/sim/tests/positions.rs`,
  `crates/net/tests/snapshot_positions.rs`,
  `crates/ai/tests/ai.rs::ai_digs_a_position_after_its_mines`,
  `tools/positions_self_test.gd` (self-test part 17).

Open: firing positions do not prefer corners over walls (nearest spot
wins); the zigzag is cut into the 2 m height raster (corners round off);
no exit delay when a dug-in vehicle drives off; strongpoints are neutral
and all-round (no front); bunkers in strongpoints are map objects
(`fort_bunker_mg`, garrisoned) placed separately; the trench meshes are
not cut into the terrain (no hole in the ground mesh).

## Bridges, bridge-laying tanks and camouflage

Prompt 055. Rules: `data/crossings.toml` (`sim::CrossingRules`, loaded by
`Sim::new` like the works rules, not part of `SimConfig`). Code:
`crates/sim/src/crossings.rs` (rules, views, events) and
`crates/sim/src/sim/crossings.rs` (update). Paratroopers and dragged air
calls extend the air support (`crates/sim/src/sim/air.rs`). Decisions and
provisional values: `prompts/done/055-bruecken-pioniertechnik-luftlande.md`.
Rules and state enter the state hash only once one of the rules acted, so
matches without them keep their hashes.

| Command | Effect |
|---|---|
| `LayBridge { units, target }` | Bridge-laying tanks drive to the gap nearest to `target` and lay their bridge across it |
| `PickUpBridge { units }` | They drive to their laid bridge and take it up once nobody is on it |
| `SetCamouflage { units, on }` | Vehicles and guns put up (or take off) branches and nets |
| `CallAirLine { mission, from, to }` | `CallAir` along the dragged line: bombers lay a carpet, transports drop along it |

- **Collapsing decks:** the engineers' demolition calls
  `Sim::collapse_bridge`: every unit whose center stands on the deck (within
  `deck_reach_mm` of its height) falls with it. Vehicles and guns are
  destroyed (passengers by the rules of prompt 034), squads fall into the
  water (`fall_loss_pct` per soldier). Event `CrossingEvent::Collapsed`,
  `Fell` per unit.
- **Swimming and drowning:** before the movement, `update_swimmers` finds
  every squad whose center lies in deep water (impassable terrain whose name
  contains "water") and moves it at `speed_mm_per_s` straight to the nearest
  path cell it may stand on (`search_m`); its orders are dropped while it
  swims. Every soldier gets a breath of `breath_min_s`..`breath_max_s` (sim
  RNG) when he falls in and drowns when it runs out in deep water. The squad
  is ashore when its center reached that cell and nobody is in the water.
  Applies to paratroopers landing in a river too.
- **Bridge-laying tanks** (catalog category `bridgelayer`, span and load from
  `bridge_length_mm` / `bridge_load_kg`): `Sim::find_gap` searches the gap
  nearest to the target (within `search_m`): a straight line along a
  path-grid axis over ground the tank cannot stand on (impassable terrain or
  steeper than the terrain's `max_grade_pct`), whose wet part plus twice
  `bearing_mm` fits the bridge, with cells the tank can reach before and
  after it and both ends within `max_grade_pct`. The deck covers the bank
  cells (abutments), so cells next to the water get no margin from it. The
  tank drives to `reach_m` before the nearer end, faces the gap and lays
  the bridge in `lay_s`: `Map::put_laid_bridge` (copy-on-write like a
  destroyed bridge) adds a bridge without map object (`Bridge::laid_by`,
  `object = u32::MAX`, deck terrain "road"), `PathGrid::sync_bridge` takes
  over its cells and railing direction (a new load limit adds a load tier
  and labels every component grid again), units plan again. Every tank has
  one slot in `Map::bridges` and lays there again after `PickUpBridge`
  (`pickup_s`, waits while units stand on the deck). Engineers may blow a
  laid bridge up (the tank has lost it) but not repair it.
- **Camouflage:** `Crossings::camo` per unit: `apply_s` of standing to put
  it up; the unit then counts how long it stood still and did not fire.
  After `settle_s`, `hide_camouflaged` (right after the vision update)
  clears the bit of every hostile player from the unit's seen mask unless a
  unit of that player (or of an ally whose eyes it shares) is within
  `seen_pct` of its sight range and has the target's cell in its viewshed.
  Allies always see it. Targeting, snapshots and the AI's view follow the
  mask.
- **Paratroopers:** `AirMission::Airborne` (`data/air.toml`: mission
  `airborne`, `[airborne]`): transport aircraft (`aircraft_transport`, seats
  = catalog `passengers`) fly like the others and release their sticks
  where bombers release bombs (no lead). A stick is a `Para`: the
  faction's first unit type of `airborne.category` in the match, sinking at
  `descent_mm_per_s` from the release altitude to a landing point scattered
  by `scatter_m` (triangular). On landing it becomes a squad
  (`Sim::spawn_unit`; next to roofs and walls if it would land on them).
  `Sim::parachutes(player)` lists the sticks in the air (to the player's
  team, or over ground the player sees).
- **Dragged air calls:** `order_air_line` aims the track along the line
  through its middle; bombers spread their bombs evenly over at most
  `bombs.carpet_max_m`, transports drop one stick at each point of the line.

### State, snapshot, bridge and Godot

Jobs, bridge slots, swimmers with their breath, camouflage timers and
sticks in the air are fields of `Sim` (saved games); laid bridges are part
of the map and saved in `MapDelta::laid` (index and deck), restored before
the intact flags (`crates/net/tests/snapshot_crossings.rs`). Queries:
`crossing_events`, `find_gap`, `layers`, `layer_gap`, `swimmers`,
`parachutes`, `camouflage_of`, `camouflage_hides`, `is_bridgelayer_type`,
`can_camouflage_type`, `type_of_unit`. `crates/bridge/src/crossing_funcs.rs`:
`lay_bridge`, `pick_up_bridge`, `bridge_gap`, `is_bridgelayer`,
`bridge_layers`, `bridge_decks`, `set_camouflage`, `can_camouflage`,
`camouflage_state`, `vehicle_camouflage_units`, `swimming_units`, `parachutes`,
`call_air_line`, `air_carpet_max_m`, `take_crossing_events`. `SimHost.bridges`
(prompt 014) leaves laid bridges out. `world/crossing_view.gd` breaks a
collapsing deck into tumbling pieces with splashes, swings the carried
bridge (`Bridge` object of the model, not merged by `unit_merge.gd`) open
while it is laid, hides it while laid and draws the laid treadway deck,
puts branches or a net on camouflaged units, draws the sticks under their
canopies and lifts swimming soldiers to the water surface
(`infantry_view.gd` asks it). `selection/crossing_orders.gd` and three
command bar buttons (Lay bridge, Take up, Camouflage); the air bar has a
fifth mission and aims by dragging. Tests: `crates/sim/tests/crossings.rs`,
`crates/ai/tests/bridgelayer.rs`, `tools/crossing_self_test.gd`.

### Open

- Provisional values in `data/crossings.toml` and the airborne rules in
  `data/air.toml`; catalog values of the bridge-laying tanks and transports
  from memory (marked uncertain).
- Shells and bombs do not destroy bridges; paratroopers under their
  canopies cannot be shot; swimmers still fire; no dedicated paratrooper
  squads (the rifle squad jumps) and no gliders; the US has no bridge-laying
  tank.
- Laid bridges only along the 4 m path grid axes; ditches are found by
  slope only; the laid deck is drawn as a treadway, not as the unfolded kit
  model.

## Victory conditions and game flow

Prompt 027. `crates/sim/src/outcome.rs` decides the match; Godot only shows it.

- Every player that owns a unit at the start is in the **standings**. A
  player is **defeated** when they send `Surrender` or have no living units
  (buildings included, prompt 023) left at the end of a tick (`Sim::alive_units`: units not destroyed by
  [combat](#combat); wrecks stay in the unit list but do not count).
- **Base lost** (decision 6 of prompt 045, `DefeatReason::BaseLost`): a
  player who has had a completed building of `[defeat] base_buildings`
  (`data/economy.toml`: headquarters, barracks, vehicle works) is defeated
  at the end of the tick in which none of them is left completed and
  alive. The economy remembers these players (`Economy::base_players`, in
  the hash and in snapshots) when a starting building is registered or a
  site is completed; `Economy::base_lost` checks them in the standings
  update. Battle armies never have such a building; mission sides that
  cannot be eliminated are exempt as from elimination.
- The match is **decided** as soon as at most one player is undefeated: that
  player wins, or it is a draw when the last players fall in the same tick.
  The result (`Sim::outcome`, with the tick) never changes afterwards; the
  simulation keeps running so the field stays visible. A match that starts
  with a single player is never decided. With teams (prompt 037,
  [TEAMS.md](TEAMS.md)) the match is decided once every undefeated player is
  an ally of the others.
- The standings enter the state hash only once someone is defeated. Before
  that they hold nothing beyond the player list, so undecided matches hash
  as before. The demo battle is decided at tick 515 since the rifle squads of
  prompt 022 (player 1 loses its last unit; before: tick 645 and player 0
  lost, 525 before the vision of prompt 018, 405 before the map of prompt
  014), which is why prompt 027 changed the golden hash.
- `Surrender` is appended to the `Command` enum, so older replays stay
  readable. No debug command removes units: the self-test wins by real
  combat.
- **Factions:** the demo scenario's `[faction_units]` table names one unit per
  faction (and `[faction_infantry]` one for groups with `role = "infantry"`,
  prompt 022); `parse_scenario_with_factions` gives every group of a player the
  unit of the faction they picked (positions unchanged). `SimHost.start_match(
  seed, map_dir, factions)` starts it.

**Scenes.** The project starts in `scenes/menu/main_menu.tscn`:

```
main menu ── Single player ─ setup (map, season, factions, opponent, mode) ┐
          ├─ LAN game ────── lan/lan_lobby.tscn (host, search, join) ────┤
          ├─ Online (Steam)  online/steam_lobby.tscn (host, list, code) ─┤
          ├─ Settings        (user://settings.cfg)                       │
          └─ Quit                                                        ▼
                                  main.tscn: loading screen → match → result → main menu
```

- `menu/match_setup.gd` hands the setup to `main.tscn` (static, like
  `lan/lan_match.gd`). `main.gd` builds a match from the menu or the lobby in
  steps (simulation and path grid, terrain, atmosphere, map objects) and
  waits a frame after each behind `menu/loading_screen.gd`; the simulation and
  the HUD are disabled until it is done. `test_large` loads in about 1.6 s
  (debug bridge, M1 Max). Direct starts (`--map`, `--trailer`, benchmarks,
  `--skip-menu`) skip the menu and load in one frame as before.
- `menu/game_menu.gd`: **Esc** opens the pause menu. In single player it
  pauses the scene tree (simulation, camera, units); multiplayer cannot pause
  a lockstep match for one player, so the menu only overlays it and offers
  Surrender and Leave. The result screen appears as soon as
  `SimHost.match_result()` is decided: victory, defeat or draw, why, match
  time, units left per player; "Back to main menu" (the replay is written as
  before).
- `menu/settings.gd` (static, loaded by every entry scene; language,
  accessibility and display since prompt 043): graphics quality
  (`--quality` overrides it), keyboard bindings of the camera actions (a key
  taken by another action swaps), volumes per audio bus and mute without
  focus (prompt 029, see [Audio](#audio-presentation-only)), pan and zoom
  sensitivity as factors on the camera's tuned speeds, edge scrolling.
  `menu/settings_panel.gd` is the screen, used by the menu and the pause menu.
- Look (`menu/menu_theme.gd`): dark field grey and olive with sand as the
  accent, Godot's default font, plain frames; factions appear by name only,
  without insignia. UI texts are translation keys, English and German
  (prompt 043, see [Translations](#translations-accessibility-and-display-presentation-only)).

**Self-test.** `just selftest` starts in the menu (`menu/flow_self_test.gd`,
on the tree root so it survives scene changes): settings changed, saved and
loaded (own file `user://settings-selftest.cfg`), LAN lobby opened and
closed, match 1 from the setup screen with the loading screen and all match
tests of `self_test.gd` (including combat), pause, surrender → defeat, back
to the menu; match 2 as United States against Germany in winter: everyone
starts with hold fire, the own tanks attack one enemy after the other until
the last is destroyed → victory by elimination, back to the menu; match 3 against the hard
computer opponent (prompt 024) while the own tanks hold fire → defeat by elimination;
match 4 (prompt 023) in the base-building mode against the normal opponent at eight
times the speed: a barracks placed through the preview, completed, selected, a unit
produced from its production bar, the opponent builds too, surrender. Matches 1–3 play
the battle mode.
Matches 1 and 2 choose the passive opponent, which gives no orders. `just lan-selftest` ends with the client
surrendering: the host must show victory, the client defeat (since prompt 037 a 2 vs 2 with computer players, see
[TEAMS.md](TEAMS.md)); the flow self-test also starts a 2 vs 2 from the setup screen.

## Translations, accessibility and display (presentation only)

Prompt 043. The game is English and German; more languages need one column
per table and one PO file per mission. Nothing here reaches the simulation:
the golden hash is unchanged.

**Translation tables.** Every interface text is a key (`MENU_SKIRMISH`,
`HUD_TIP_STOP`: upper case, area prefix, at least one underscore) in a
Godot translation table `godot/locale/<area>.csv` with the columns
`keys,en,de`:

| Table | Prefixes | Texts |
|---|---|---|
| `common.csv` | `COMMON_` | words several screens share: Back, Save, factions, difficulties, resources |
| `menu.csv` | `MENU_`, `MATCH_`, `RESULT_`, `FILES_`, `CREDITS_`, `LOADING_`, `NET_`, `REPLAY_`, `CRASH_` | main menu, setup, pause and result, saves and replays, credits, loading, network status |
| `settings.csv` | `SET_`, `SUB_` | settings screen, key labels (`SET_KEY_<ACTION>`), subtitles |
| `lobby.csv` | `LOBBY_` | LAN and Steam lobbies |
| `hud.csv` | `HUD_` | HUD, picker, orders, base HUD, air bar, strike, events, mission panel |
| `editor.csv` | `EDITOR_` | map editor |
| `names.csv` | `NAME_`, `RANK_` | building names (`data/economy.toml`) and ranks (`data/experience.toml`) |
| `units.csv` | `UNIT_` | ratings, tile tooltip, unit card, unit reference, names of categories and vehicle database fields (prompt 066) |

Godot imports each table into `<area>.<lang>.translation` next to it (not
committed, `*.translation` is ignored); `just build` runs the import when a
table is new or changed (`tools/ensure-imported.sh`). `menu/language.gd`
registers every translation in `res://locale/` (no list in
`project.godot`, so a new table needs no other change) and switches the
locale.

**In code.** A static text assigns the key: `button.text = "MENU_SKIRMISH"`;
a `Control` translates it when drawn and follows a language change. A text
built at run time uses `tr("KEY") % value` or, with several values German
may reorder, `tr("KEY").format({"name": ...})`; screens with such texts
rebuild them on `NOTIFICATION_TRANSLATION_CHANGED` or in their periodic
refresh. Static functions use `Language.t("KEY")`. Codes from the bridge
(blocked reasons, air missions, biomes, defeat reasons) become keys with
`Language.code_text("HUD_BLOCKED_", code)`, which falls back to the code.
Developer tools (`scenes/tools/`, `scenes/debug/`), self-tests, logs and
errors meant for developers stay English.

**Adding a text.** Add the key with both texts to the table of its area,
use the key in code, `just l10n-check`. German style: concise game German,
the player is addressed with "du", buttons as infinitive or noun; the
glossary of the first pass is in `prompts/done/043-sprache-barrierefreiheit.md`.

**Names from data.** Catalog entries carry `name_de`/`name_en`;
`SimHost.set_language` (called by `language.gd`) makes every name the
bridge hands out (`unit_info`, buy options, `catalog_card`, the air
bar's aircraft) come in the interface language
(`crates/bridge/src/language_funcs.rs`). Buildings and ranks are translated
in the bridge through the keys `NAME_<ID>` and `RANK_<NAME>` of
`names.csv`, falling back to the English name of the data file. Weapon,
map and player names are proper names and stay as they are.

**Mission texts.** Mission files stay English; `<id>.<lang>.po` next to the
file translates them, `msgid` being the English text
(`sim::mission::mission_texts`: title, briefing, objectives, messages).
`just mission-po` writes these files (new texts get an empty `msgstr`,
existing translations are kept); `missions.gd` (`Missions.text`) loads them
for the missions screen and the mission panel. A text without translation
shows in English. Details in [MISSIONS.md](MISSIONS.md).

**Checks.**

- `just l10n-check` (`tools/l10n_check.py`, also run by `just selftest`):
  tables have both texts, unique keys and the same placeholders per
  language; every key literal in the game scripts exists; no hard-coded
  interface text: a string literal that reads like prose (or any word on a
  line that sets `text`, `tooltip_text`, `add_item` …) is an error unless
  the line is logging, a node or file name, or ends with `# l10n-ignore`.
  `cli mission-po --check`: every mission text has a German translation and
  the PO files are up to date.
- `just selftest` (`menu/l10n_self_test.gd` in the flow test): every menu
  screen and settings tab in German without a text that holds a key without
  translation; a catalog name and a mission title in German; every new
  setting saved and read back from the file; during match 1 the HUD and the
  pause menu in German and a subtitle for a warning. The other tests run in
  English (`Language.resolve`: any `--…self-test` argument means English
  unless the settings say otherwise; `--language=<code>` always wins).
- `just l10n-shots [de] [build/l10n]` (needs a window): screenshots of every
  menu screen, settings tab, the match HUD (tanks, a squad, the build menu),
  the pause menu and the result, to check by eye that nothing overflows;
  `--ui-scale=1.5` as extra argument for a second set.

**Settings** (`menu/settings.gd`, tabs Graphics and Interface):

| Key | Values | Effect |
|---|---|---|
| `language` | `auto` (system language if offered, else English), `en`, `de` | `TranslationServer` locale, names in the bridge |
| `ui_scale` | 0.75–1.5 | `content_scale_factor` of the root window: all 2D (menus, HUD) |
| `font_size` | 0.85, 1.0, 1.15, 1.3 | factor on the menu theme's font size and every font size override (`MenuTheme.font`) |
| `color_palette` | `standard`, `red_green`, `blue_yellow` | own/ally/enemy colors (`hud/relation_colors.gd`): minimap, health bars, base markers, mines, strike areas, model tints |
| `subtitles` | off by default | a line for warnings (UI notices: under attack, unit lost or ready, building complete, victory, defeat) and battle callouts (`hud/subtitles.gd`) |
| `screen_shake` | on by default | camera shake of explosions (`effects.gd`) |
| `window_mode` | `windowed`, `fullscreen` (borderless), `exclusive` | `DisplayServer.window_set_mode` |
| `resolution` | "" (as started), 1280×720 … 3840×2160 | window size in window mode, centered, at most the screen |
| `vsync`, `fps_limit` | on/off; 0 (none), 30, 60, 120, 144, 240 | VSync mode, `Engine.max_fps` |
| `render_scale`, `upscaler` | 0.5–1.0; `bilinear`, `fsr1`, `fsr2` | `scaling_3d_scale`/`scaling_3d_mode` of the root viewport; the interface stays sharp |

Display values apply only when they changed since the last time (benches
that switch VSync off keep it), never headless; the first call of a run
leaves a plain start untouched.

**Keys.** Every order of the command bar, the camera, the air calls
(Shift+1…5), the rocket strike (Shift+6) and the unit card (Shift+I,
prompt 066) are rebindable, with Shift,
Ctrl, Alt or Meta (a binding is a physical keycode with modifier masks;
older files with plain codes stay valid). Air calls, the strike and the
unit card are
input actions added at run time (`Settings.RUNTIME_ACTIONS`). Fixed
[provisional]: control groups (1…9, Ctrl+1…9), the picker's digit keys
(Alt+digit) and the map editor's shortcuts.

**Decisions** (with reasons in the prompt file):

- Keys instead of English source texts: a missing translation shows as a
  key and the self-test finds it; English wording can change without
  touching code.
- CSV per area instead of one file or PO: both languages side by side for
  a solo developer; parallel prompts add rows to different files.
- Mission texts as PO next to the mission instead of `_de` fields in the
  TOML: the simulation and the mission format stay unchanged, and PO is
  the format translation tools know.
- [provisional] German addresses the player with "du".
- [provisional] Palettes from the Okabe-Ito set; the standard palette keeps
  each view's tuned shades.
- [provisional] Subtitles off by default; callout lines show even while no
  voice files ship (prompt 053), so the setting is useful now.

**Open:** right-to-left and CJK languages (font with those glyphs);
translating validation messages from the Rust crates (editor validation,
lobby and network messages, file errors stay English); scaling the 3D
labels over units with the UI scale.

## Missions and tutorial

Prompt 039, reference in [MISSIONS.md](MISSIONS.md). A mission is a TOML
script next to its map (`data/maps/<map>/missions/<id>.toml`, places in
`mission_points.toml`). `sim::mission::parse_mission` resolves it into
`SimConfig::mission`; the simulation visits its triggers at the end of
every tick (after vision, before the standings) and runs the actions
(units appear, orders as commands of their owner, objectives, messages,
camera requests, victory or defeat with `DefeatReason::Mission`). So a
mission is deterministic and part of replays, saved games and the state
hash (only when there is one: the golden hash is unchanged). What only the
presentation knows arrives as `Command::MissionSignal`. Godot shows the
panel (`hud/mission_hud.gd`) and keeps the progress in `user://`.

## Map editor

Prompt 028 adds an in-game map editor (`godot/scenes/editor/`, main menu →
"Map editor"); its use is described in [MAPS.md](MAPS.md#map-editor-prompt-028).

**Mode of the game, not a Godot editor plugin.** The editor needs the terrain
renderer with its rectangle updates, the object, decoration and water
rendering, the biome look, the RTS camera and picking without physics. All of
them exist for the game; a plugin would need `@tool` versions of these scripts
and a GDExtension running inside the Godot editor. In the game the editor
works in every build, calls the Rust map code directly and is tested headless
like the rest (`just editor-selftest`). The price is an own small tool panel
instead of the inspector.

```
editor_panel.gd (buttons, sliders)   map_editor.gd (strokes, redraw)
                     │                    │  begin_stroke / raise / paint / … / end_stroke, undo, save, validate
                     ▼                    ▼
              MapEditor (crates/bridge/src/map_editor.rs: meters → millimeters)
                     │
              editor::EditMap (crates/editor: rasters, objects, metadata, history)
                     │  files() / save_dir()            validate_files()
                     ▼                                         ▼
     data/maps/<name>/ (format of MAPS.md)   sim::Map::load + add_objects, scenario, PathGrid
```

- **Data.** `EditMap` keeps the map in file form: heights in millimeters,
  terrain ids, objects as `objects.csv` lines, starts and markers in whole
  meters. Brushes work in integers (millimeters, falloff in 1/1024, strength
  in per mille, object scatter from `sim::Rng` with a fixed editor seed), so
  an edit script gives the same files on every machine. The crate denies
  float arithmetic like `sim`.
- **Undo.** `begin_stroke`/`end_stroke` bracket a mouse stroke. Before a brush
  changes samples the stroke records their old values in one growing
  rectangle; objects and metadata are copied once per stroke when first
  touched. `end_stroke` stores old and new state of what changed (nothing
  for strokes without effect); undo and redo paste them back.
- **Drawing.** `MapEditor` offers the read functions of `SimHost` that the
  renderers use (`grid_samples`, `terrain_layers`, `terrain_chunk_ranges`,
  `height_at`, `ray_pick`, `water_levels`, `water_mesh`, `object_kinds`,
  `object_groups`, `scatter_deco`, …); `terrain.gd`, `map_objects.gd` and
  `water.gd` take either host (typed `Object`). Heights and terrain come
  from the live rasters, so `terrain.heights_changed(rect)` shows every dab.
  Objects, decoration and water need a `sim::Map`; it is rebuilt from the
  edited data when a stroke ends (or after undo/redo), and the scene then
  rebuilds the object and water nodes and `terrain.refresh_water()`.
- **Saving and checking.** Saving encodes the PNGs and TOML like
  `cli gen-map`. The check encodes the files and loads them with exactly the
  calls a match uses, so what is checked is what will be played: start
  positions, room for the scenario's units, paths between the starts for
  every faction's tanks and infantry, `Sim::new`. The match uses the map's
  start positions (`sim::scenario`, see MAPS.md), so a map built in the
  editor plays from the starts set there.
- **Determinism.** Nothing of the editor runs during a match. A saved map is
  an ordinary map folder; peers load it independently and compare its
  content hash as for every map (`MapRef`).

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

Player 1 is the computer opponent chosen in the setup (prompt 024): an `ai::Ai`
hosted in the same session (`LockstepSession::host_player`), whose frames are
submitted right after the local one; see [AI.md](AI.md).

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
(terrain tests and benchmarks); real craters change the simulation's map
([Terrain deformation](#terrain-deformation)), which the bridge always reads
through the simulation.

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
- **Against tiling:** two samples per layer, blended by noise: the tile itself and a
  rotated one of 1.37 tiles, which gives way to one of 5.5 tiles between 40 and 250 m
  camera distance (cross-faded, `B_SCALE_NEAR`/`B_SCALE_FAR`); low-frequency macro
  noise varies brightness and dry/lush tint across hundreds of meters.
- **Rule (prompt 071):** every texture coordinate of the ground depends on the world
  position only; camera distance and pixel footprint only steer weights. Until
  prompt 071 the rotated sample grew continuously with the camera distance of each
  pixel: whenever the camera moved, that texture slid over the ground (by up to
  `|position from the map centre| × 0.011` meters per meter of distance change) and
  the change of scale across the screen sheared it into 1 px streaks on meadow and
  road, at every camera distance between 40 and 250 m.
- **Measuring it:** `just ground-motion [dir] [--png]` records the trailer shots
  `column_high` (farms) and `survey_ground` (test_large, near a corner) and measures
  fixed windows of pure ground with `tools/ground_flicker.py`: how far the fine
  texture slides against the ground (px/frame) and how well consecutive frames match
  per wavelength band once the ground motion is undone. `--png` records lossless
  frames; the MJPEG of the default recording dominates the 2–4 px band.

  | Window (lossless, frames 120–149 / 150–179) | Slide before → after | Match 4–8 px | Match 2–4 px |
  |---|---|---|---|
  | `column_high` meadow | 4.73 → 0.19 px/frame | 0.60 → 0.96 | 0.33 → 0.76 |
  | `column_high` road | 1.22 → 0.30 px/frame | 0.59 → 0.90 | 0.05 → 0.52 |
  | `survey_ground` meadow (corner of 4 km) | 0.11 → 0.11 px/frame | 0.91 → 0.99 | 0.69 → 0.91 |

  Near a corner the old sample changed so fast that it turned into per-pixel noise
  instead of a sliding pattern, which shows in the 2–4 px band, not in the slide.
  What is left on the road at 2–4 px is probably gravel detail at the pixel limit,
  resampled differently at every sub-pixel position (not measured further; a
  candidate is a small mip bias on the road layer).
  Cost: two texture reads more per visible layer, only between 40 and 250 m;
  `just terrain-bench` middle (120 m) 107.5 → 103.3 FPS, the other views and
  `just bench-view` (high, same session, old against new) unchanged within noise.
- **Field tracks:** the track layer is sampled in a frame turned along the track
  direction from the `track` raster, so ruts follow curves.
- **Water:** river beds are darkened and made glossy as a stand-in until prompt 014
  draws water surfaces.

The map ends in a vertical cut (`terrain_edge.gdshader`, soil layers) down to 12 m
below the lowest point, like a diorama block.

### Meadow (prompt 057)

The grass layer was one olive texture with 0.7 m crossed-card clumps on top,
which read as boxes from the high camera. Now the meadow is built from three
cheap parts that share one palette (`assets/shaders/meadow_palette.gdshaderinc`:
lush green, dry straw, damp dark green, all procedural from `terrain_macro.png`
and hashes, no new textures):

| Part | Where | What |
|---|---|---|
| Ground (`assets/materials/terrain_meadow.gdshaderinc`, one call in `terrain.gdshader` after the layer blend and before wet ground and snow) | Grass share of every pixel, `meadow_strength` per biome | Color in four scales: dry/lush areas (macro noise, hundreds of meters), clouds of 29/47 m and blotches of 3.7/6.1 m (two turned samples each, so the overview shows no repeat), speckles of 0.3 m. The grass texture only keeps its light/dark pattern (contrast 1.5). Near the camera (pixel < 2.5 cm) one layer of blade strokes in 3.6 cm cells and clumps of 16 cm with dark gaps; at middle zoom (pixel 1.5–8 cm) tufts of 45 cm. Bare soil patches, frayed soil at the edges of field tracks, grass pressed flat along tracks. Daisies and dandelions as 1.6–3.5 cm dots in loose groups (pixel < 6 cm, `flower_density` per biome). Everything antialiased by the pixel footprint and skipped where a pixel is too large to show it. |
| Tufts (`scenes/world/meadow_tufts.gd`, `assets/shaders/meadow_tuft.gdshader`) | Decoration layer "grass" of `map_objects.gd` in the forest biome, 0.55 m lattice, up to 55 m from the camera | Procedural tufts of 14–18 single blades as opaque triangles (no alpha test, no card outlines), normals mostly up so blades are lit like the ground, color from the palette at the tuft (per instance in the vertex shader), dark roots, light tips, wind. Short and wide tufts have straight blades (one triangle); tall tufts are curved (three) and cast shadows. Desert and winter keep the clumps of prompt 013 with their biome look. |
| Light (`atmosphere.gd`, `forest.tres`) | Whole scene | Forest sun at 37° instead of 42° (longer shadows) **[provisional]**, shadow blur 2 (softer edges on soft filters), round leaf blobs in the cheap foliage pattern of shadow passes (the square cells cast checkerboard shadows onto the meadow). Tilt-shift depth of field at close zoom (see "Light and atmosphere"). |

Why not shell texturing or GPU particles for the grass: shells draw the
terrain several times (8–16 passes over the ground near the camera) and still
show layers at the grazing edge of the view; particles would need their own
placement. The decoration lattice of prompt 013 already scatters by terrain
type, wood density, map objects and craters, so the tufts reuse it with new
meshes.

Open: footprints and drag marks of soldiers (a presentation trail texture;
prompt 040 brings tracks into the same shader), mowing stripes (pastures of
the period were grazed, not mown), crater ejecta on grass, seasons (the biome
is the only season so far).

**Flicker while the camera moves (prompt 068).** The streaks users saw on the
meadow while panning came from the ground texture sliding with the camera
distance (fixed in prompt 071, see "Splatting shader"). The near range was
measured afterwards: `just ground-motion --near --png` records the trailer
shots `meadow_near_15/35/55` (open meadow with tufts on `test_large`, pans of
about 7 px/frame at the game's pitch, `haze` = distance so depth of field and
shadow range are as in the game) and measures a window of pure meadow with
`tools/ground_flicker.py`. `--off=<parts>` (`--trailer-off=` of
`trailer_shots.gd`, analysis only) turns parts off: `tufts`, `ground`
(procedural meadow), `shadows`, `wind`, `dof`, `ssao`, `msaa`; `fxaa` and
`taa` turn those on. Match of consecutive frames after undoing the ground
motion (lossless, frames 60–119; slide 0.15–0.17 px/frame everywhere, i.e. the
texture sticks to the ground):

| Variant | 15 m 4–8 / 2–4 px | 35 m 4–8 / 2–4 px | 55 m 4–8 / 2–4 px |
|---|---|---|---|
| as in the game (high: MSAA 4×) | 0.973 / 0.815 | 0.955 / 0.716 | 0.961 / 0.775 |
| no wind | 0.973 / 0.816 | 0.955 / 0.715 | 0.961 / 0.775 |
| no tufts | 0.976 / 0.826 | 0.964 / 0.742 | 0.963 / 0.780 |
| no procedural meadow | 0.979 / 0.834 | 0.936 / 0.678 | 0.936 / 0.728 |
| no shadows | 0.974 / 0.816 | 0.956 / 0.718 | 0.962 / 0.779 |
| no depth of field | 0.973 / 0.815 | 0.955 / 0.716 | 0.961 / 0.775 |
| no MSAA | 0.964 / 0.784 | 0.922 / 0.633 | 0.959 / 0.770 |
| plus FXAA | 0.960 / 0.741 | 0.942 / 0.663 | 0.945 / 0.723 |
| plus TAA | 0.990 / 0.947 | 0.989 / 0.933 | 0.988 / 0.931 |
| blade strokes faded out at 1.8 instead of 2.5 cm pixels (not kept) | 0.974 / 0.817 | 0.956 / 0.722 | 0.961 / 0.775 |

No single part causes what is left: nothing slides, wind, shadows and depth
of field change nothing, the tufts cost a little at 35 m, and the procedural
meadow is calmer than the bare grass texture under it. The rest is detail at
the pixel limit (grass texture, speckles, blade strokes, flowers) sampled at a
new sub-pixel position every frame; only temporal antialiasing removes it.
TAA as a default is a larger decision (softer stills, ghosting behind moving
units, its own cost) and was not made; the tuft lattice is already jittered by
a whole cell and anchored to the world (`objects.rs::scatter`), and the
ground's texture arrays are mipmapped with anisotropic filtering.

`SHOT_SET=meadow just terrain-shots <dir>` records the meadow at 15, 35, 110
and 350 m near the village and two open views, without fog of war and HUD.
`MEADOW_AB=1 just terrain-bench` measures every view once per variant with
one part of the look turned off (`no_ground`, `no_tufts`, `no_dof`,
`old_sun`, `old_blur`, `none` = the look before), in one run so a busy
machine affects all variants alike.

### Tall grass (prompt 073)

Terrain type `tall_grass` (id 8, `data/terrain.toml`): dense, knee- to
chest-high growth like in the tropics, after a screenshot of another game
(not in the repository; nothing taken from it, everything is generated). In
the rules it is grass (same speed, softness, no cover or concealment yet);
the presentation draws it in three parts:

| Part | Where | What |
|---|---|---|
| Plants (`scenes/world/tall_grass.gd`, `assets/shaders/tall_grass.gdshader`) | Decoration layers "tall_grass" (0.44 m lattice, density 0.97, frayed edges) and "tall_shrubs" (2.2 m lattice, 0.3, 25 m patch noise at full strength, so shrubs stand in loose groups) of `map_objects.gd`, 16 m squares, up to the range of the quality level | Single blades, leaves and fronds as opaque triangles like the meadow tufts (no cards, no alpha test). Color from almost black roots through a patch color (fresh, yellowish and dark patches of 6–23 m) to sunlit yellow-green tips; strong AO at the roots. Gust bands, sway and flutter in the biome's wind. |
| Ground (`assets/materials/terrain_tall_grass.gdshaderinc`, one call in `terrain.gdshader` after the meadow) | Mask in the A channel of the track raster (`terrain_layers.rs`; tall grass counts as grass in the eight splat weights) | Near: the meadow below in deep shade, so no light soil shows between the plants. Beyond the plants' range: the stand itself, darker and more saturated than the meadow: tufts of 0.53/0.62 m in two turned layers with radiating blade strokes over dark gaps, clumps of a few meters, the gusts of the plants, a shadow seam just outside the stand. Every texture coordinate depends on the world position only. |
| Tracks (`scenes/world/grass_trails.gd`) | Maps with tall grass | A presentation-only raster over the map (1 m per texel, 2 m above 2 km): a 2D SubViewport that is never cleared, into which vehicles stamp their footprint while they drive (R: pressed flat) and scorch marks of blasts their size (G: burnt). Plants lie flat and pale there and stay so; burnt ones are black stubble. Read by plants and ground, never read back, nothing reaches the simulation. |

Plants:

| Variant | Size | Triangles | What |
|---|---|---|---|
| Tall tuft (50 %) | 1.0–1.55 m × 0.8–1.15 | 110 | 22 blades of 5 cm in three segments, steep, bending over towards the tip, the outer ones more; the tuft leans a little to one side |
| Low tuft (30 %) | 0.65–1.1 m | 100 | 20 blades of 4.2 cm |
| Herb (14 %) | 0.6–1.0 m | 33 | a stem with ten broad leaves |
| Rosette (6 %) | fronds 0.7–1.05 m | 77 | eleven arching fronds, widest in the middle |
| Shrubs (layer "tall_shrubs", two variants) | 1.4–2.0 m | 480–610 | woody stems, a crown of 140–180 olive leaves, a third of them yellow-green on top, seed heads above |

- **Edge:** the Rust scatter (`scatter_deco`, `fray` 1.1 m) interpolates
  the density between the four samples around a point moved by noise, so
  the edge follows no grid line, and plants get shorter towards it (40 %
  at the very edge). The ground mask is warped by noise too.
- **Distance:** all blades up to `near_range`, then fewer by rank (UV2.y)
  down to 35 % at `switch`, the rest widened; from `switch` on the squares
  draw a far mesh with just those blades (a third of the vertices; a square
  counts by its center, so the swap shows nothing). No blade is drawn
  thinner than about half a pixel. From 0.75 of the range the plants sink
  into the ground while the ground's stand look fades in (0.7–0.97 of the
  range).
- **Quality** (`tall_grass.gd` `QUALITY`): range 72 / 65 / 55 m, full
  blades up to 26 / 28 / 20 m, switch 46 / 45 / 35 m, density 1.0 / 0.9 /
  0.75 (high / medium / low; high is held back by its MSAA 4× and SSAO); shrubs cast shadows on high and medium, the
  tufts never (in each other's shadow the stand turned black, and every
  blade would be drawn once per cascade).
- **Biomes** (`tall_grass.gd` `LOOKS`): forest lush green; desert straw,
  60 % of the plants, 0.8 of the height; winter brown, 30 %, 0.7 of the
  height, broken blades hanging over, snow on the upper parts, and on the
  ground the snow cover is 45 % thinner over tall grass so stalks show.
- **Readability:** standing soldiers (1.8 m) and vehicles stand out of the
  grass; prone soldiers nearly vanish, their selection rings and bars do
  not. Plants do not give way around units **[provisional]**: vehicles
  flatten their tracks, which was enough in the pictures.

Height from the picture: the tanks there (about 2.7 m high, 6–7 m long)
give about 15 px/m in the middle of the picture and about 18 px/m in the
lower half (perspective); blades there are 20–35 px long, i.e. 1.5–2 m of
grass, the shrubs' crowns 2–3 m across. The prompt estimated 0.8–1.2 m;
uncertainty ±40 % (a small, compressed picture, perspective, bent blades,
probably exaggerated plants). Chosen: 0.65–1.55 m grass, shrubs to 2 m.

Tools: `SHOT_SET=tall just terrain-shots <dir> tall_grass_test` (15, 35, 80,
110, 350 m, the reference view, edges to meadow, field track, road, water
and wood, a pressed track at 35 and 110 m; `--biome=` and `--daytime=`
pass through), `SHOT_SET=tall_land` (river_land, farms),
`TALL_AB=1 just terrain-bench tall_grass_test` (every view as tall grass,
as its ground look only and as plain meadow), `just ground-motion --tall`
(pans `tall_near_15/35/55/110`; `--off=tall,tall_ground` turn the parts off).
`cli gen-map --tall-grass` paints large stands (test map
`tall_grass_test`); `cli gen-land` paints the banks of river_land and
fallow patches on farms.

Measurements (`tall_grass_test`, 1920 × 1080, Apple M1 Max, V-Sync off,
debug bridge, other sessions on the machine: load 4–6).
`TALL_AB=1 just terrain-bench tall_grass_test`, slow pan over the stand,
each view as tall grass, as its ground look only, and the same ground as
plain meadow, in one run (frames per second, triangles):

| View | High: tall / ground only / meadow | Medium: tall / meadow | Low: tall / meadow |
|---|---|---|---|
| 15 m | 96.6 (1.13 M) / 93.7 / 109.1 (0.32 M) | 119.9 (1.09 M) / 120.0 | 120.0 (0.73 M) / 120.0 |
| 35 m | 67.9 (2.10 M) / 77.2 / 89.5 (0.34 M) | 114.7 (1.97 M) / 119.8 | 119.9 (1.09 M) / 120.0 |
| 80 m | 59.8 (1.08 M) / 61.6 / 67.1 (0.29 M) | 108.3 (0.81 M) / 120.0 | 120.1 (0.20 M) / 119.9 |
| 110 m | 57.0 (0.30 M) / 57.1 / 60.4 (0.30 M) | 110.2 / 119.6 | 120.0 / 119.9 |
| 350 m | 74.7 / 70.7 / 74.9 | 114.8 / 115.7 | 120.0 / 120.1 |

Medium and low run at the frame cap of 120. On high the test map is
already at 60–67 FPS as plain meadow at 80–110 m (3 700 trees on 800 m,
MSAA 4×, SSAO); tall grass costs 1.3 ms at 15 m, 3.0 ms at 35 m, 1.8 ms at
80 m and 1.0 ms at 110 m (there almost all of it the ground look), so high
stays just under 60 FPS at 80 and 110 m **[open]**. At 350 m tall grass costs
nothing measurable. Decoration instances: up to 290 000 at 35 m on high (meadow
56 000), in 16 m squares (316 draw calls against 160). Before the far
meshes (all blades everywhere, range 100 m) the same views drew 3–5 M
triangles at 48–53 FPS.

Flicker, `just ground-motion --tall --png` against `--near` (lossless,
frames 60–119, match of consecutive frames after undoing the ground
motion; meadow values as in prompt 068, measured again the same day):

| Pan | Slide | Match 4–8 px tall / meadow | Match 2–4 px tall / meadow |
|---|---|---|---|
| 15 m | 0.21 px/frame | 0.941 / 0.973 | 0.653 / 0.815 |
| 35 m | 0.11 | 0.919 / 0.955 | 0.630 / 0.716 |
| 55 m | 0.13 | 0.934 / 0.961 | 0.644 / 0.775 |
| 110 m (ground look; no wind) | 0.14 | 0.990 / – | 0.836 / – (0.502 with blade strokes down to one pixel, with wind) |

Nothing slides. Without wind the near values rise only a little (0.685 /
0.645 / 0.652 at 2–4 px), and a wider minimum blade width changes nothing
(the near blades are 4–5 cm, several pixels): what is left is the plants'
own parallax (1–1.5 m above the ground, which the measurement does not
undo) and the edges of blades at the pixel limit. The ground look first
drew its blade strokes down to one pixel and matched 0.50 at 110 m; its
strokes now give way to their coverage below two pixels (0.84, calmer
than the meadow).

Open: concealment of infantry in tall grass (ambush rule, `data/vision.toml`
`[ambush]`; balance and AI), slower infantry or wheels in it, a tropical
biome (light, palms, bamboo, houses), reeds and grain fields with the same
technique, plants giving way around soldiers, burning tall grass as a rule
(the fire below is drawn only; the simulation's fire burns objects), the
tracks raster is lost with a saved game (presentation only).

#### Grass fire (prompt 082, presentation only)

`scenes/world/grass_fire.gd`, created by `main.gd` on maps with tall grass.
Nothing of it reaches the simulation: a burning meadow hurts nobody, blocks
no sight and changes no path. [provisional] all values; not benchmarked.

- **Start:** `effects.gd` reports every scorch mark (`_decal`) to
  `grass_fire.ignite(at, size)`. Marks of at least 4 m (132 mm rockets,
  bombs, heavy shells, exploding vehicles; an 8.8 cm shell leaves 3.3 m) set
  the tall grass under them on fire.
- **Spread:** a raster of 4 m cells over the map. A burning cell schedules
  its eight neighbours that are tall grass (`SimHost.terrain_at`, asked once
  per cell): they catch fire after the way there at 1.4 m per game second,
  up to 55 % faster downwind and slower upwind, times a random 0.7–1.4, with
  a 90 % chance per burning neighbour. A cell burns 9 s. The front stops
  where the tall grass ends (short grass, tracks, roads, water, woods).
  Time is the simulation's tick, so the fire pauses with the game and is as
  far after a fast run-up of the trailer tool as after a slow one; its
  random generator has a fixed seed.
- **Burnt ground:** every cell that catches fire stamps a circle (off
  center, 2.6–4.2 m) into channel G of the track raster (`grass_trails.gd`),
  which plants and ground already draw as black stubble.
- **Flames and smoke:** `fire` and `smoke_column` emitters of the effect
  library (4 + 2 / 10 + 5 / 18 + 10 for low / medium / high) that jump to
  another burning cell every frame; their particles stay in the world, so
  a few systems cover the whole front. Flames are scaled 1.9, deeper orange
  and with less glow than a wreck's fire; smoke is scaled 2.6. Emitters thin
  out while few cells burn and skip cells the player does not see.
- **Sound:** every half second `grass_fire.gd` tells `battle_sound.gd`
  (`set_grass_fires`) where the fire burns nearest the listener and at a
  second place at least 60 m from there; both get a `fire_large` loop that
  follows them, ahead of single wrecks in the six fire voices. Level by the
  number of burning cells: 0 dB over the loop's own at 60 cells, at most
  +14 dB (the sample is a quiet one, judged by ear in the video).
- **Trailer scout:** prints burning cells and the area burning or burnt.

Open: a real rule (damage, sight, spreading into woods and to objects),
embers glowing on the burnt ground, light at night, saving the fire with
a saved game (it is lost like the tracks).

### Region updates (craters, prompt 021)

`terrain.heights_changed(rect)` re-reads a rectangle of samples (grown by 8 samples
for normals and filters) through `terrain_layers`, blits it into the images,
uploads the textures and refreshes the chunk height ranges and, at the map border,
the edge cut. No mesh is rebuilt: chunks only reference the textures.
`terrain.regions_changed(rects)` does the same for several rectangles with one
upload and updates only the height ranges of the chunks under them
(`SimHost.terrain_chunk_ranges_in`); `world/crater_view.gd` uses it for the craters of the simulation
([Terrain deformation](#terrain-deformation)). The debug menu (F3) "Test crater
at focus" makes a simulation crater (a presentation-only dent in network
matches).

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

## Map objects and decoration (presentation only)

The simulation owns the [map objects](MAPS.md#objects) (trees, buildings, hedges,
walls, props) and blocks path cells with them; Godot draws them and adds
decoration that exists only on screen. `godot/scenes/world/map_objects.gd`, data
from `crates/bridge/src/objects.rs`.

### Objects

- `SimHost.object_groups(128)` returns, per kind (catalog id + variant) and 128 m
  chunk, a `MultiMesh` buffer (12 floats per instance): position on the ground
  (trees sunk 0.15 m, boxes at the lowest ground under their corners), heading,
  and a presentation-only size jitter for trees (±12 %), bushes and rocks.
- One `MultiMeshInstance3D` per group and mesh part of the variant (buildings:
  `_Walls`, `_Roof`, `_Floor`, `_Debris` share the buffer). `test_large`: 98 000
  objects in 13 200 nodes, built in 0.25 s.
- **Distance:** trees switch to their `_Far` mesh (crossed cards, prompt 007) at
  220 m by visibility ranges per chunk (fade); hedges vanish beyond 1.4 km, walls
  and rocks beyond 0.7–0.9 km, fences, logs and small props sooner; buildings and
  trees never.
- **Shadows** only from trees, buildings, hedges, walls, hay, rocks and poles.
  Near trees cast their shadow through a shadows-only copy of the far mesh, and
  the foliage shader (`env_foliage.gdshader`) uses a cheap one-hash leaf pattern
  in directional shadow passes (orthographic projection) and beyond 70 m instead
  of its 3 × 3 leaf-cell search. Over a wood at 15 m the full pattern in four
  shadow cascades held the frame at 33 FPS; with both measures 160–175 FPS.
- **Biome:** `EnvLook` instance uniforms per node, from the map's biome (prompt 015,
  `MapObjects.set_biome`). Every node with instance uniforms takes slots of Godot's global
  shader uniform buffer; `project.godot` raises
  `rendering/limits/global_shader_variables/buffer_size` to 262 144 (the default
  65 536 overflowed at ~10 000 nodes and turned foliage black).
- **Wind** comes from the env shaders (per-instance phase from the instance
  position, which `MODEL_MATRIX` includes for MultiMesh instances).
- **Variety (prompt 056):** `env.gdshader` derives a seed from each
  object's position (`v_seed`, the instance origin on a 2 m grid) and picks
  per house the paint of lime-plastered and whitewashed walls (seven
  colours, a few stay bare), the roof covering (red, brown or weathered clay
  tiles, about a fifth dark cement tiles; slate blue-grey to black), the
  brick tone and the boards' weathering. Deterministic and presentation
  only: the same house always looks the same, nothing new is stored.
  Corrugated steel (Metal kinds rust and galvanized) gets ribs.
- **Roof patina (prompt 075):** roofs are weathered per house
  (`env_roof.gdshaderinc`, [ASSETS.md](ASSETS.md#roof-patina-prompt-075)):
  an age from `v_seed`, per covering its own patina (clay tile, slate,
  thatch, planks, corrugated sheet, flat plaster roofs), all from the
  position on the model, so nothing wanders and the same house always
  looks the same. Barns are thatched and the depot roof is galvanized
  sheet since then. Measured on an Apple M1 Max at 1920 x 1080:
  `just objects-bench` line "village" (150 m) 51.5 → 50.5 FPS back to back
  (19.43 → 19.80 ms, about 2 %). Flicker (`just ground-motion --roofs
  --png`, windows on town house roofs, frame-to-frame match in the
  2-4 px band, 1 = stable): 15 m 0.61 → 0.78, 35 m 0.48 → 0.81, 110 m
  0.16 → 0.64; the old rows (which ran down the slope) slid against the
  roof at 20-57 px/frame, the new ones stick (0.3-0.6). Open: chimney
  streaks (the shader does not know where a chimney is; stage 2 would
  bake ridge, eave and chimney into the models), the ridge distance is
  only right for roofs with the ridge above the model origin, and moss on
  haystacks (same Roof kind as thatch).
- **Snow on fallen objects (prompt 056):** snow cover is computed from the
  model normal (objects only turn about the vertical), so a falling tree
  keeps its snow on what was its top; needle branches carry snow on all
  but their undersides. Fallen and flattened objects get `env_fallen = 1`
  (no wind sway).
- **Loose straw (prompt 056):** `_build_straw` scatters 22 flattened, dry
  grass clumps in a ring of 1.6–5.5 m around every haystack (one
  MultiMesh, 180 m view distance, seeded by the stack's position).
- **Winter:** grass clumps are thinned to 8 %, reeds and stones to 40 %
  (snow covers them).
- **Roof snow (prompt 080):** on winter maps roofs carry a blanket with a
  clear edge that stays along ridge and eave and has slid off in between
  (`env_snow.gdshaderinc`, details in [ASSETS.md](ASSETS.md#roof-snow-prompt-080)).
  Shader only, no snow geometry (stage 2 of the prompt, a `_Snow` part per
  building, was not needed: the edge faces of the roof slabs already read
  as the blanket's front). The eave distance comes from the mesh bounds
  (`env_roof_box`, set in `_apply_look`). Measured on `winter_village`
  (M1 Max, 1920 x 1080, `just objects-bench winter_village --roofs-bench`,
  over the house nearest the village): 35 m 61.7 -> 61.3 FPS, 110 m
  68.1 -> 67.5, 350 m 81.0 -> 80.3 (within 1 %). Flicker
  (`just ground-motion --snow-roofs`, falling snow hidden, frame match per
  band 16-32 / 8-16 / 4-8 / 2-4 px, 1 = stable): 15 m 0.91 / 0.40 / 0.05 /
  0.27 -> 0.997 / 0.99 / 0.94 / 0.51, 35 m 0.98 / 0.88 / 0.39 / 0.08 ->
  0.995 / 0.99 / 0.96 / 0.58, 110 m 0.97 / 0.94 / 0.75 / 0.35 -> 0.97 /
  0.95 / 0.76 / 0.32 (MJPEG; the old pale blotchy roofs gave the bands
  little to hold on to). Views: `just snow-shots` (village at 15-350 m,
  two houses like the reference, every roof kind, twenty equal houses,
  damage; day, low sun, night, falling snow; amounts 0.3-1.0). Forest and
  desert unchanged (`env_snow` = 0 skips it). Open: slid-off snow heaps at
  the foot of the walls (would need a snow-heap decoration model), snow on
  roofs that grows, thaws or falls off in the game, icicles, chimney
  smoke and thawed rings, snow drifts at fences and walls, snow on vehicles
  and soldiers; the eave distance of a mesh with several roofs (church
  nave and choir) is taken from the largest; the sun side assumes the
  winter sun (south-east).

### Decoration

Grass and flower clumps (every 0.7 m lattice point, 100 m view distance), bushes
(8 m, 500 m, many in woods), small stones (5 m, 160 m) and stumps (14 m, in woods)
are scattered by `SimHost.scatter_deco` in Rust per 64 m square: a hash of the
lattice point and a fixed seed gives jitter, rotation, scale and variant; the
chance comes from a density per terrain type, a 25 m patch noise and the wood
density (trunks within about 12 m), and samples covered by buildings, walls,
hedges or props (grown by 1 m) get nothing. Only squares within the view distance
of the camera exist; up to six are built per frame and far ones are freed. About
70 000 grass instances around the camera at the closest zoom. Tall grass
(prompt 073, see "Tall grass") has two layers of its own in 16 m squares
(a 16 m square costs a quarter of the per-frame budget of a 64 m one, its
denser lattice twice that), with frayed edges (`fray`: density
interpolated at a noise-moved point, plants shorter at the edge) and the
range of the quality level; the other layers have no density on tall
grass.

### Measurements (`test_large`, 1920 × 1080, Apple M1 Max, V-Sync off)

`just objects-bench`: over the densest wood (400 trees per 128 m chunk), slow pan;
debug bridge; other sessions busy on the machine, so ±15 %:

| View | FPS | Draw calls | Triangles | Decoration |
|---|---|---|---|---|
| Near (15 m), wood edge | 157 | 175 | 0.47 M | 66 000 |
| Near without object shadows | 264 | 120 | 0.45 M | 66 000 |
| Middle (120 m) | 137 | 585 | 0.52 M | 76 000 (mostly culled) |
| Overview (600 m) | 195 | 2 665 | 0.69 M | 185 |
| Overview without objects | 276 | 30 | 0.11 M | 0 |
| Village (150 m) | 261 | 404 | 0.36 M | 33 000 |

`just terrain-bench` (meadow, as in prompt 011) with objects: 260 / 264 / 202 FPS
near / middle / overview. The draw calls in the overview come from the far meshes
of about 300 visible chunks × tree variants × shadow cascades; merging far trees
into larger chunks would cut them if weaker GPUs need it (prompt 016).
`just objects-shots <dir>` records woods, village, meadow and a hedgerow.

### Grown rock (prompt 074)

Three rock kinds (models: [ASSETS.md](ASSETS.md#grown-rock-prompt-074),
rules: [MAPS.md](MAPS.md#objects)): flat **slabs** (`env_rock_slab`, only a
picture), **outcrops** of one to four blocks (`env_rock_outcrop`) and **ridge
pieces** (`env_rock_massif`, 12–25 m), which overlap into ridges. Outcrops and
ridge pieces block every unit and are solid for sight up to their variant
height; as tank-blocking objects with a height they stop projectiles and are
cover spots like walls, with no new cover rule.

- **Sitting in the ground.** The models reach `env_embed_m` (0.25–1.5 m)
  below z = 0. Outcrops and ridge pieces stand at the lowest ground under
  their box like houses, so the uphill side sinks into the slope. Slabs
  are sheared onto the plane through the ground at their corners
  (`push_sheared` in `crates/bridge/src/objects.rs`), so they neither float
  nor sink on slopes. New rock kinds get no size jitter (the box must
  match), slabs ±8 %.
- **Seam.** `DecoRasters` keeps decoration off the rock's foot only (solid
  rocks' boxes are 0.4 m larger than the foot, the raster shrinks them by
  that) and marks a 3 m seam around every rock; layers with `rock` add
  density there on grass, sand, mud, snow and tall grass: `rock_scree`
  (small `env_rock` stones, 1.3 m lattice), `rock_tufts` (tall grass
  clumps) and more bushes. So the foot has scree, tufts and bushes, as in
  the reference pictures; the ground under the rock stays bare.
- **Trees in ridges** come from placement (editor preset, `gen-map
  --rocks`), not from the model.
- **View ranges and shadows:** ridge pieces unlimited, outcrops 1 600 m,
  slabs 600 m without shadow; ridge pieces switch to their far mesh at
  220 m like trees (its shadow stands in for the near one).
- **Shader:** slot `Rock` of `env.gdshader` in world space, so neighbouring
  blocks and overlapping pieces continue each other's pattern; base color of
  the terrain's rock layer per biome.

Measurements (Apple M1 Max, 1920 × 1080, debug bridge, Blender and the
Godot editor open on the machine, so ±15 %): `just objects-bench` on
`test_large` (no rocks there; the new decoration layers find nothing),
before / after: near 22.7 / 22.7 FPS, middle 30.7 / 30.9, overview 66.1 /
66.9, village 51.7 / 52.0, draw calls unchanged (262 / 649 / 1 919 / 797).
`just objects-bench rock_hills --rocks-bench` over the densest ridge field
(22 pieces in a 128 m chunk): 35 m 94 FPS (237 draw calls, 0.50 M
triangles, 63 000 decoration), 110 m 80 FPS (369, 0.49 M), 350 m 119 FPS
(795, 0.20 M). Triangles per variant: slabs ≤ 219, outcrops ≤ 536,
ridge pieces ≤ 1 444 near and ≤ 300 far. `just rock-shots [dir]` saves
slabs, outcrops and ridge pieces at 15, 35, 110 and 350 m, the ridge piece
on the steepest slope and three views like the reference pictures, in all
three biomes (`--daytime=evening` for low sun, `--biomes=`).

Open points: the terrain shader does not draw stonier ground around rocks
(the seam decoration stands in for it; a raster from the objects would be
the next step); ridge fields are sparser than in the reference pictures
(denser placement or wider pieces); the "steepest slope" shot of
`rock-shots` can land in a wood; thumbnails show the embedded foot.
Not part of prompt 074: cliffs as terrain, climbable rocks, ricochets and
splinters, rolling or blasted boulders.

### Apartment blocks (prompt 078)

Three blocks of four to six storeys (`env_block_desert`,
`env_block_desert_long`, `env_block_town`, models: ASSETS.md "Apartment
blocks"; map placement: MAPS.md "Apartment block quarter"). They are
buildings like the town house:

- **Rules** (all [provisional]): `objects.toml` blocks everybody with the
  catalog box, the ruin vehicles only; `vision.toml` solid up to the
  catalog height (14.2-20 m); `destruction.toml` hit points scaled from
  the town house (3 500 for about 1 700 m³) by built volume and capped at
  9 000 (block 6 000; sixteen 7.5 cm hits for the larger two), ruins one
  to two storeys high (27-33 %); `infantry.toml` capacity 24 / 36 / 36
  (about three town houses: one man per 10-15 m² would put 80-150 men in
  a block; Pavlov's House in Stalingrad, four storeys, was held by about
  25-30), exposure like the town hall; `fire.toml` masonry (floors and
  roof burn out). The hashed rule tables changed, so did the golden
  hashes; with the old data the old values come out. Tests: a block
  hides units behind it (`vision.rs`), four squads fill the long block
  and a fifth stays out, a shelled block goes Damaged → Ruin and the
  collapse costs occupants (`infantry.rs`). Editor groups: town and
  desert. The AI does not occupy buildings (no change).
- **Camera:** at the closest zoom (15 m along the view ray, 45° pitch)
  the camera is about 11 m up, below the roofs of blocks, the church and
  the town hall (17 m); before, it could end up inside them. The camera
  now stays `building_clearance` (1.5 m) above the top of any standing
  building or solid rock within 2 m of it (`SimHost.object_top_at`,
  `GroundPick.object_top_at`, a scan of the map's boxes that block
  everybody; not measurable in `just objects-bench`). It rises over a
  block rather than pushing in.
- **Units behind blocks:** health bars and the occupation bar are drawn
  on the HUD and stay visible; selection rings had depth testing and
  vanished behind houses: they now have a second, translucent pass
  without depth test (`unit.tscn`), so a selected unit behind a block
  shows its ring through it. Open (proposal): a silhouette of selected
  units through buildings (a stencil or depth-test-off pass of the unit
  model in a flat color); rotating the camera is no substitute.
- **Shadows:** a 17 m block at low sun casts a shadow over a whole street
  (60-70 m); the directional shadow reaches 2.5 × the camera distance
  (37 m at the closest zoom, 275 m at 110 m), which covers the visible
  ground at every zoom; `just block-shots` at low sun (`evening`) shows
  no torn shadow edges at 15-350 m at the default level and at `low`
  (`--quality=low`); flicker was judged on still pictures only.
- **Fog of war, fire, snow:** the blocks use the env shader like every
  house: fog, charring and snow on roofs, parapets, sills and balconies
  work unchanged; flat roofs get the flat-roof patina of prompt 075.
- **Map objects:** `map_objects.gd` draws `env_block*` with unlimited
  range and shadows. The town hall and the warehouse fell to the default
  class (300 m, no shadow) before; they are now listed like the houses.
- **Observed, not changed:** occupants of a shelled building die faster
  than `heavy_exposure_pct` alone suggests, because the tanks' machine
  guns firing at the same point add many light rounds with
  `exposure_pct` each (`just block-fight`: a squad of nine was gone
  before the block was a ruin). The charring of line-up houses in
  `just roof-shots` / `just block-shots` (instance uniform `env_char` set
  on hand-placed nodes) shows almost no effect; fires in a match char
  through the same uniform, not checked here. Both open.

**Measurements** (Apple M1 Max, 1920 × 1080, V-Sync off, frame rate
capped at 120): `just objects-bench test_large` before and after the
prompt back to back: identical (near 22.6 / 22.6 FPS, middle 30.6 / 30.7,
overview 65.9 / 65.9, village 51.3 / 51.3; the object_top_at scan does not
show). Over the quarter (`just objects-bench city_blocks --blocks-bench
--quality=<level>`, 11 blocks, 128 objects):

| Quality | 35 m | 110 m | 350 m |
|---|---|---|---|
| low | 120 FPS, 190 draw calls, 0.32 M tris | 120, 322, 0.29 M | 120, 610, 0.30 M |
| medium | 120, 273, 0.48 M | 120, 456, 0.44 M | 120, 839, 0.41 M |
| high | 113.5 (8.8 ms), 273, 0.48 M | 107.3 (9.3 ms), 456, 0.44 M | 120, 839, 0.41 M |

Per block: 2 600-4 800 triangles per damage level, 4 mesh parts with 3-6
surfaces each.

**Open:** storeys as a rule (soldiers in single storeys and on the roof,
storey choice, fighting inside, farther sight from upper floors), damage
per storey instead of three stages for the whole house, skyscrapers,
factories, stations, a block quarter in the town generator (prompt 079
builds `city_large`), wall patina beyond the window stains.

## Water (presentation only)

Prompt 014. The simulation knows water only as terrain types on a river bed;
`crates/bridge/src/water.rs` derives the surface, `godot/scenes/world/water.gd`
and `assets/materials/water.gdshader` draw it.

```
sim::Map (heights, terrain ids 6/7, bridges, rocks, water_flow_deg)
   │  WaterField::new (floats, once per map, cached in the bridge)
   ▼
per sample: level · shore distance · flow velocity · depth
   │  level: bank heights (land next to water, not under decks, at most
   │         0.9 m above the bed next to it) carried to the water by
   │         nearest bank (chamfer distance), 8 × 3×3 smoothing, 8 cm down
   │  shore: chamfer distance to land or a stone in the water (m)
   │  flow:  perpendicular to the shore-distance gradient, turned towards
   │         water_flow_deg, smoothed; 0.9 m/s in the middle of deep water,
   │         slower near banks and in shallows, around stones
   ▼
SimHost.water_mesh(): one vertex per water sample and its ring of land
   samples, two triangles per touched 2 m cell; UV = flow, UV2 = (shore, depth)
   ▼
water.gd: one MeshInstance3D, ShaderMaterial, no shadow casting
SimHost.water_levels(): level raster → terrain shader (wet band)
```

**Shader** (`water.gdshader`, transparent pass):

- **Flow map:** two samples of a noise normal map shifted along the flow, half
  a cycle (2.4 s) apart, blended so neither jumps when its phase wraps; plus
  a slow large ripple layer. Normal maps and the foam pattern are
  `NoiseTexture2D` resources (seamless `FastNoiseLite`), so there are no
  texture files.
- **Foam:** near banks and stones (shore distance), in very shallow water and
  where the screen depth shows the water getting thin, with a cellular pattern
  that moves with the flow; streaks on fast water.
- **Color:** the opaque scene behind (screen texture, offset by the normals
  unless that would pick up something in front of the water) comes in as
  emission, so it is not lit twice, weakened per channel over the view
  distance through the water (screen depth; blue is absorbed fastest, like
  humic river water); the water's own color (biome: shallow, deep, foam,
  clarity) is the lit albedo.
- **Reflection:** Godot's sky radiance and sun specular on a smooth surface
  (Fresnel included). No screen-space reflections: they are an effect of
  the whole environment (every surface pays for them, and they streak at the
  screen edges); planar reflections would render the scene twice. From the
  high RTS camera, water mirrors almost only the sky.

**Banks:** the terrain shader darkens ground up to 0.75 m above the waterline
and makes it glossy (wet band), and darkens the bed under water a little; the
water tints the rest. **Reeds** (`env_grass_clump` variant `Reeds`) grow in
a band from 1.5 m in the water to 1.5 m on land, bushes 3–10 m from the water
(`map_objects.gd` decoration layers with a `shore` band, from the shore
distance of `WaterField`). Stones in the river are map objects (generator).

**Bridges** are drawn like other map objects; `object_groups` places them at
the simulation's deck height and `map_objects.gd` lowers each model by its
`deck_height_m`.

**Measurements** (`test_large`, Apple M1 Max, 1920 × 1080, debug bridge;
the machine was heavily loaded by other sessions, load average about 60, so
absolute frame rates are about half of those in the sections above):

| Quantity | Value |
|---|---|
| `WaterField` + mesh, `test_large` | 64 ms at game start; 56 776 vertices, 104 594 triangles (`test_small`: 2 ms, 3 558 triangles) |
| River view at the stone bridge (90 m) | 52.5 FPS with water, 54.6 FPS without: the water costs about 0.7 ms per frame |
| Same run: meadow near / middle / overview | 118 / 101 / 100 FPS |

The river view is the slowest view because of the dense woods and the
decoration along the banks, not the water.

Debug menu (F3): "Water" shows or hides the surface. `just terrain-shots` has
views of the stone bridge (two distances), the timber bridge and the ford;
`just terrain-bench` measures a river view with and without the water.

## Lighting, atmosphere and biomes (presentation only)

Prompt 015. `godot/scenes/world/atmosphere.gd` (created by `main.gd`) sets up
the sun, the `WorldEnvironment`, a vignette, the terrain's biome colors and the
weather from a **biome preset** and a **graphics quality level**.

### Biome

```
data/maps/<name>/map.toml   biome = "forest" | "desert" | "winter"   (default forest)
   │  sim::Map::biome()  (validated on load, not in the map hash or state hash)
   ▼
SimHost.map_biome()  ──  `-- --biome=<id>` overrides it
   ▼
atmosphere.gd: load res://assets/biomes/<id>.tres (BiomePreset)
   ├─ sun, sky, ambient light, haze, tonemapping and grading, vignette
   ├─ terrain.gd apply_biome(): layer sources and tints, snow cover, map edge
   ├─ env materials (bark, foliage): wind strength, direction, speed
   └─ snowfall particles (winter)
main.gd: UnitCamo.palette(faction, preset.camo_biome) for every unit model;
         map_objects.gd: EnvLook look of preset.env_biome on map objects and decoration
```

`forest` is the temperate biome (meadows, fields, forest). The names are the
same as those of the camouflage palettes and `EnvLook`, so there is no mapping
table. The simulation only checks the value; no rule reads it. Debug menu (F3)
and `main.set_biome(id)` switch the biome at runtime (units and map objects are
repainted, decoration is rebuilt).

**Ground per biome.** The map's terrain types stay what they are; the terrain
shader draws every material layer with the texture layer named in
`layer_source` and multiplies it by `layer_tint`. Desert: grass and snow are
drawn as pale sand, mud as dried earth, tracks and gravel lighter and
yellower. Winter: grass and sand are drawn as snow, `snow_cover` adds patchy
snow on flat ground (thinner on roads and tracks, none on water), tracks stay
dark and rutted. Forest is the identity. No extra textures are needed.

### Light and atmosphere

| Element | Setting | Why |
|---|---|---|
| Sun | Direction from elevation and bearing per biome (forest 37° since prompt 057 **[provisional]**, before 42°; desert 58°, winter 24°), from the south-east, so shadows fall to the north-west (upper left when looking north); no angular size, PCF filter by quality, shadow blur 2 (prompt 057) | Readable shadows like the references; side light shows relief; longer, softer shadows darken the meadow. |
| Shadow cascades | 4 (low: 2), distance 1.8 × camera distance, splits at 45 / 58 / 75 % of it, blended | The ground is seen from about 0.8 to 1.5 camera distances; the default splits spent half the cascades on the air in front of the camera. |
| Sky | `ProceduralSkyMaterial` per biome | No downloads; the sky is almost never on screen at 45–65° pitch, it serves as ambient light and haze color. Blue-ish shadow fill comes from it. |
| Tonemapping | AgX, exposure and white per biome | Pleasant highlight roll-off on sand and snow without clipping. |
| Grading | Contrast, saturation and a `GradientTexture1D` per-channel tone curve (cool shadows, warm highlights) | Editable in the `.tres`, no LUT image files. |
| Haze | Depth fog from 0.65–0.7 to 2.0–2.2 camera distances, curve 1.3, at most 35–45 %, aerial perspective | Foreground stays clear at every zoom, the upper screen edge lies in light haze. Exponential fog would grey the whole view at 600 m. |
| SSAO | Radius 2.5 % of the camera distance (0.6–10 m) | Contact shadows under vehicles up close, soft shading of hollows and ditches far away. |
| SSIL, SDFGI | Off | SSIL measured 0.5–0.7 ms on high with hardly a visible change on open ground; SDFGI costs too much on a 4 km map and leaks light. |
| Depth of field | Tilt-shift, setting "Depth of field" with three modes (`DOF_MODES` in `atmosphere.gd`), off on quality low. **Subtle** (prompt 057, the default **[provisional]**): sharp from 0.86 to 1.1 camera distances along the view, blurred over 0.22 distances beyond, amount 0.09, fading out between 30 and 60 m camera distance; box bokeh, very low quality. **Miniature**: sharp from 0.95 to 1.05, blurred over 0.2 distances beyond, amount 0.22, fading out between 150 and 350 m; hexagon bokeh, medium quality; saturation × 1.18 and contrast × 1.05, fading with the blur **[provisional values]**. **Off**: none. | Subtle makes close views look like a photo of a model and hides the far edge of the decoration range. A far blur alone (tried in prompt 015) smeared the upper edge at every zoom; the narrow band around the focus only shows near the ground. Miniature is the full model-railway look as an option: the 45–65° view down already is the angle of such photos, and the depth-based blur keeps a band across the middle of the screen sharp. It is not the default because units near the upper and lower screen edges are harder to make out. |
| Vignette | Canvas shader below the HUD, 25 % | Frames the view lightly. |
| Snowfall | `GPUParticles3D` box around the camera focus, scaled with the camera distance (winter, medium and high) | Weather prepared; flakes stay visible at every zoom. |

### Graphics quality

`-- --quality=low|medium|high` (default high) or the debug menu (F3);
settings in `scenes/world/graphics_quality.gd`. A settings menu follows with
prompt 027.

| Level | Shadow atlas | Cascades | Shadow filter | SSAO | Glow | AA | Snow particles |
|---|---|---|---|---|---|---|---|
| low | 2048² | 2 | hard | off | off | FXAA | off |
| medium | 4096² | 4 | soft low | low, half size | on | FXAA | 6 000 × snowfall |
| high | 8192² | 4 | soft medium | medium, full size | on | MSAA 4× | 16 000 × snowfall |

Measured with `just terrain-bench test_large --quality=<level>` (1920 × 1080,
Apple M1 Max, V-Sync off, debug bridge, forest; "terrain" = overview without
units; measured before the map objects of prompt 013 were merged):

| Level | Near (15 m) | Middle (120 m) | Overview (600 m) | Terrain only |
|---|---|---|---|---|
| low | 279 FPS | 272 FPS | 248 FPS | 260 FPS |
| medium | 239 FPS | 231 FPS | 244 FPS | 253 FPS |
| high | 172 FPS | 164 FPS | 164 FPS | 183 FPS |
| high, winter (snowfall) | 186 FPS | 144 FPS | 165 FPS | 178 FPS |
| high + SSIL (not used) | 164 FPS | 151 FPS | 147 FPS | – |

Every level stays far above the 60 FPS target. Most of the cost of "high" is
MSAA 4× and the full-size SSAO.

### Tools

`just terrain-shots <dir> test_large --biome=desert` records the fixed views
in a biome, `just terrain-bench test_large --quality=low` measures a level.
The model gallery (`just gallery`) lights models with the sun color, sky,
ambient light and grading of the selected biome's preset.

## Effects (presentation only)

Prompt 019. `godot/scenes/world/effects.gd` (created by `main.gd` after the
combat view) turns the combat events and the snapshot into smoke, fire and
dust. Particle systems are built in `effect_library.gd`, their textures are
generated from noise at start-up in `effect_textures.gd` (no image files).
Nothing here reaches the simulation; Godot randomness is fine because no
effect is read back. No Rust code was needed: everything comes from
`SimHost.take_combat_events` (via `combat_view.combat_event`),
`unit_aim_yaws`, `unit_moving`, `unit_footprints`, `terrain_at` and
`height_at`.

| Trigger | Effect |
|---|---|
| `shot`, caliber ≥ 20 mm | The muzzle blast of prompt 077 (below) at the main gun's `Socket_Muzzle` along the real barrel (fallback: the simulation's muzzle and aim): core, fire tongue, glowing gas, powder smoke, side blasts of muzzle brakes, ground dust by terrain, a warm light; the gun recoils and the hull rocks; the tracer starts at the socket |
| `shot`, small arms | A small flash at the nearest machine-gun socket (`Socket_Muzzle_NN`) |
| `shot` of indirect fire from a unit whose model has `Socket_Rocket_NN` (vehicle launchers, the strike's battery position) | The rocket salvo of prompt 076 (`rocket_effects.gd`, below): the next rail's socket in turn, a flare, back blast and ground dust, the position's light and haze; the shell is drawn as a rocket with flame and smoke trail |
| `hit` on the ground | From 37 mm (prompt 072): a core flash, a fan of dark earth streaks, clods, a cloud of earth lumps that rises and greys (tinted by terrain and biome), sparks, a light flash and a scorch decal; below 37 mm clods and a dust column; water: spray; small arms: a dust puff |
| `hit` on a unit | Sparks and a light flash; a penetration adds a small burst of fire and smoke |
| `hit` on an object | Splinters and dust |
| `destroyed`, vehicle | A ray burst, a cream fireball of lumps that turns orange, dark red and black (2.4 s), black smoke growing out of it from 0.9 s, glowing embers, sparks, debris, ground dust ring, a big light flash, a large scorch decal, camera shake (prompt 072); then the wreck **burns** at its `Socket_Fire_NN` (fire, a dense black smoke column drifting with the biome's wind, flickering light) for 60 s, smolders for 90 s and goes out |
| `destroyed`, infantry | Dust only |
| Vehicle moving | Dust trail behind the hull, color and strength by the terrain under it in the current biome (tracks and sand strong, grass weak, roads hardly, mud dark, snow powder, shallow water spray), exhaust smoke at `Socket_Exhaust_NN` (thin when standing) |

Tracers stay in `combat_view.gd` (prompt 017). The ground color follows the
terrain shader's biome mapping (desert draws grass as sand, winter grass and
sand as snow), so dust matches the ground it rises from.

### Drawing

Every particle is a camera-facing quad drawn by one small spatial shader
(render-mode variants: lit, unshaded, additive; since prompt 072 also
billow, fireball and streak): texture × particle color × a per-instance
`tint` (`set_instance_shader_parameter`, the ground color), so one pool
serves every terrain. Lit puffs get a rounded fake normal and read as
volumes in the sun; fire, flashes and sparks are unshaded and push HDR
values into the glow. Soft particles (fade where a quad cuts into the
ground) read the depth texture. An effect is scaled by its emitter's node
scale (caliber / 75 mm, vehicle size); particles inherit it.

**Billows** (prompt 072): `EffectTextures.billow()` is a generated 2 × 2
atlas of cauliflower clusters (6 to 9 lumps each); per pixel it stores the
lump normal (R, G), a shade (B) and the density (A). The particle's random
animation offset (`CUSTOM.z`) picks the cell. The "billow" shader turns the
normal with the quad and lets the real sun and lights shade it (sunlit tops,
dark undersides; at night only fire and flashes light it); the "fireball"
shader is unshaded and darkens the creases as the particle ages. **Streaks**
(`EffectTextures.streak()`, `particle_flag_align_y`) turn the quad so it
points along the particle's velocity: the earth fan of a ground hit.
**Layers:** an explosion is a list of bursts with delays in
`effects.gd` `EXPLOSION_LAYERS` (`ground_hit`, `vehicle`), started by
`effects.layers(profile, ground_point, scale, ground_color)`; delayed layers
wait in `_pending`. Reuse it for new explosion kinds.

### Budget and LOD

Burst effects come from **pools** of `GPUParticles3D` per kind; each burst
restarts the next emitter of its pool, so when the pool is exhausted the
oldest burst of that kind is cut short. Values per graphics quality, keys
`fx_*` in `graphics_quality.gd`:

| Level | Particles | Pools | Pooled emitters / particles | Light flashes | Scorch decals | Burning wrecks (with light) | Soft particles | Dust and exhaust within |
|---|---|---|---|---|---|---|---|---|
| low | × 0.4 | × 0.35 | 272 / 1 841 | 2 | 32 | 4 (0) | off | 150 m |
| medium | × 0.7 | × 0.65 | 496 / 5 876 | 6 | 96 | 10 (3) | off | 250 m |
| high | × 1.0 | × 1.0 | 768 / 12 952 | 12 | 192 | 24 (8) | on | 400 m |

More burning wrecks than the budget: the oldest ones only smolder. A quality
or biome change rebuilds the pools (the biome sets the wind of the smoke).
**LOD:** a burst that would be smaller than 3 px on screen (distance to the
camera, zoom, 30° lens) is skipped, one smaller than 60 px gets fewer
particles (`amount_ratio`, at least 35 %). Small details (embers, sparks,
the core flash: `DETAIL_KINDS`) start only for explosions of at least 50 px
(prompt 072). Decals fade out beyond 450 m.

**Camera shake** offsets the `Camera3D` (`h_offset`, `v_offset`), not the
rig, so it never touches the focus or picking state; strength falls off with
the distance to the camera focus and decays in about half a second.

### Fog of war

`effects.visible_at: Callable` (default: always true) decides whether a world
point is visible. New effects only start at visible points; burning wrecks
are checked twice a second and vehicle effects every frame and hidden while
not visible. Prompt 018 sets it to its visibility query; nothing else
changes.

### Test scene and measurements

`just effects-shots <dir>` runs `scenes/tools/effects_bench.gd` on the demo
map (every unit holding fire) and saves screenshots of every effect at fixed
frames after its trigger (`--fixed-fps 30`): muzzle, small arms, ground
hits, water, ricochet, penetration, explosion, burning wreck, scorch marks,
dust trails, a barrage of 50 explosions, then dust and impacts in desert and
winter. `just effects-bench` measures 50 explosions at once (a new barrage
every 2 s, 6 s per view) in every quality (1920 × 1080, Apple M1 Max, V-Sync
off, debug bridge, forest, test_small with its map objects):

| Level | Idle, 150 m | 50 explosions, 150 m | 50 explosions, 60 m | + 24 burning wrecks |
|---|---|---|---|---|
| low | 254 FPS | 216 FPS | 205 FPS | 191 FPS |
| medium | 166 FPS | 136 FPS | 138 FPS | 129 FPS |
| high | 87 FPS | 79 FPS | 80 FPS | 72 FPS |

Prompt 072 (17.11.2026, same machine, other load on it; low and medium were
capped at 120 FPS in these runs), before → after the new explosions,
measured back to back:

| Level | Idle, 150 m | 50 explosions, 150 m | 50 explosions, 60 m | + 24 burning wrecks |
|---|---|---|---|---|
| low | 120 → 120 FPS (cap) | 120 → 120 | 120 → 120 | 120 → 120 |
| medium | 120 → 120 FPS (cap) | 116 → 114 | 120 → 118 | 113 → 109 |
| high | 64–69 → 66–68 FPS | 56–60 → 57–59 | 72–75 → 65 | 53–61 → 55–59 |

The idle frame on "high" got slower since prompt 019 (meadow, map objects:
68 FPS instead of 87), so "high" no longer stays above 60 FPS with 50
explosions at 150 m, neither before nor after prompt 072. The new
explosions cost about 0.4 ms more at 150 m and 2 ms more at 60 m (larger,
longer fireball and smoke). [open] The self-test
checks that the combat test produced muzzle, impact and explosion effects
and a burning wreck.

### Decisions

| Question | Decision | Why |
|---|---|---|
| Where effects live | GDScript in `scenes/world/`, no Rust change | All data is in the snapshot and event stream; the golden hash and lockstep stay untouched. |
| Particles | `GPUParticles3D` + `ParticleProcessMaterial`, pooled, `restart()` per burst | Proven by the snowfall (015), runs on the GPU, no node creation per shot. |
| One shader with instance tint | Instead of `StandardMaterial3D` per color | One pool per kind serves every terrain and biome; soft particles and the rounded normal are a few lines. |
| Textures | Generated from noise at start-up | No downloads or licenses; a few hundred KB of memory. |
| Muzzle position | Nearest `Socket_Muzzle*` to the simulation's muzzle | No mapping from weapon index to socket needed; falls back to the simulation's point. |
| Debris | Dark sprites, not meshes | A mesh draw pass with a `StandardMaterial3D` leaked its shader at exit (Godot 4.7.2); sprites look the same from above. |
| Decals | `Decal` ring buffer per quality, 2.5 m deep box | Optical; since prompt 021 a scorch mark lies in the simulation's crater of the same hit. |

### Open

- Craters (prompt 021) are terrain now; the impact of a crater-making hit
  still uses the ground-hit effect (earth fountain, dust, scorch decal),
  there is no extra crater decal.
- Sound hooks into the same events since prompt 029 (see [Audio](#audio-presentation-only)).
- Smoke does not block line of sight; smoke shells and screens are game
  rules for later prompts.

### Big blasts (prompt 054)

`world/blast_effects.gd` adds to every hit of at least 110 mm (12.2 cm
howitzers and up, bombs, the rockets of the heavy strike, exploding dumps)
and to a tank's ammunition explosion:

- a **shock-wave dome**: a white hemisphere that grows to 0.075 m per mm of
  caliber in 0.32 s, slowing down, with a bright rim and a nearly clear
  middle, fading (the condensation of the shock front); a flat **pressure
  ring** to twice that radius in 0.55 s; a **dust ring** thrown outwards
  along the ground in the ground's colour (12 pooled domes and rings, 4–8
  dust rings by quality);
- a grey-beige dome of dust (`dust_cloud`, ~16 m within 0.5 s at 15 cm,
  prompt 072), a small fireball and debris on top of the ordinary impact of
  `effects.gd`; the shock dome and ring are fainter since prompt 072;
- a **flash light**: an omni light of 30–66 m range for 0.32 s that lights
  up the ground and the buildings around; at most 1 / 2 / 4 at once (low /
  medium / high, the oldest is reused); at night 3× as strong and 1.45× as
  far, at dawn and dusk 1.7× (prompt 040);
- more camera shake.

A tank whose ammunition explodes (`StrikeEvent::AmmoExplosion`) also
throws its turret: the `Turret` node leaves the model, flies up 10–15 m/s
with a spin, lands and stays (6 in the air, 16 lying at most); it gets the
wreck tint and no longer follows the aim. Pictures: `just blast-shots`
(fixed views by day and by night into `build/explosions/`) and `just
strike-shots` (a real strike on the base skirmish; fails if smoke, siren,
countdown or launch are missing). [provisional] sizes, colours, threshold.

### Explosions after a video reference (prompt 072)

A short promotional video of another game served as a reference for
**timing and look only**: measured times, sizes, colours and motion, no
pictures, textures or sounds (it stays outside the repository; analysis
pictures in `build/explosion-ref/`). Textures are still generated
(`effect_textures.gd`: billow atlas, streak, rays).

**Tool:** `just explosion-series [dir]` (`tools/explosion-series.sh`)
triggers every explosion kind once on flat ground from 60 m at 45°
(`blast_shots.gd --explosion-series`, 30 fps fixed), saves pictures at the
series times (every frame of the first second, then four per second to
10 s) and builds tile sheets with `tools/explosion_series.py` (numpy +
ffmpeg; also video sheets, colour means). If the reference video is in
`.temp/`, the video's sheet of the matching explosion goes above the
game's: `<dir>/compare_<kind>.png`. Crops show the same meters as the
video crop (`SERIES_KINDS`).

**Scale in the video:** soldiers (1.8 m) give 0.010 to 0.022 m per pixel of
the 2160 × 3840 frame, ±25 % (one desert shot without soldiers ±40 %). The
camera moves and zooms in some shots, and cuts every 3 to 6 s, so smoke
lifetimes are lower bounds.

Profiles (times from the first frame of the explosion; video at 30 fps,
partly doubled frames, so ±33–67 ms). Game values for scale 1 unless
noted; the 15 cm reference ground hit is scale 2.

| Phase | Video (target) | Game (reached) |
|---|---|---|
| Flash | 1 frame (≤ 67 ms); the whole frame 15–35 % brighter; white-yellow core 3–5 m | core 0.15 s, 3.5 m × 0.7; light flash 0.12 s (ground hit), 0.4 s (vehicle) |
| Ground hit, earth fan (15 cm) | ~25 dark streaks, 11 m wide and 12–16 m high at 0.3–0.4 s, lumpy columns by 0.6 s | 30 streaks; fan ~10–11 m wide, ~11 m high at 0.4 s (scale 2), down by 1.3 s |
| Ground hit, cloud (15 cm) | ≥ 12 m wide at 1.0 s; rises ~7 m/s first, then slows; brown (110,84,66) at 0.3 s, (79,60,47) at 1.5 s, grey (155,146,134) at 3.5 s; dense at 4 s (cut) | ~12 m wide at 1 s (scale 2); brown, grey-beige from ~3 s, thinning from 4 s, gone ~7–8 s |
| Vehicle, ray burst | 0–0.3 s, pale, ~11 m across | 0–0.4 s, 10 m quads |
| Vehicle, fireball | cream lumps, largest at ~1.0 s: ~10 m wide, ~12 m high (1.4 vehicle lengths); colours (179,147,110) 0.2 s, (231,184,130) 0.5 s, (235,208,184) 1.0 s, (143,89,48) 1.5 s, (108,46,23) 1.8 s, (34,15,5) 2.2 s | cream to 1.0 s, orange 1.5 s, dark red 1.8 s, black 2.2 s, gone 2.4 s; ~9–10 m wide at 1 s |
| Vehicle, smoke | black column from ~2.2 s (25,18,8), opaque ≥ 3.4 s (cut); glowing spots 1.7–2.5 s | dark billows from 0.9 s (hidden in the fireball), a dark grey-black column 2.5–4 s, less opaque than the video, greying, gone ~9.5 s; embers 1.2–3 s; the wreck's column goes on |
| Big blast (D) | grey-beige dust dome ≥ 16 m within 0.5 s, (181,158,136) → (134,116,102) at 1.7 s; orange flame tongues 0.1–0.6 s; ~20 dark chunks 0.3–0.5 m, > 1 s in the air; a burning patch on the ground | dust dome ~16 m at 0.5 s (15 cm), thinning from ~3 s, gone ~8 s; small fireball; debris sprites; no burning patch (open) |
| Small hit (E) | dark streak fan within 0.1 s, hangs ≥ 1.3 s | streak fan 0.1–1.3 s, small earth cloud |
| Shock wave | no condensation dome visible, only the dust dome | white dome and ring of prompt 054 kept, fainter |
| Debris | no smoke trails seen | no trails |
| Afterburn | black-brown column with glowing spots (31–33 s); a flame on the wreck | burning wreck of prompt 019, now with billows |
| Standing smoke (1–6 s) | a dark brown ball of lumps 6 m wide, 9 m high, nearly unchanged for ≥ 2.2 s | — (column and cloud stand ~4 s before thinning) |
| Shape | cauliflower of lumps 1.2–1.8 m, sunlit tops light, undersides very dark | billow atlas, lumps lit by the sun |
| Camera | no shake visible | shake of prompt 019 kept |

Differences above 20 %: the vehicle fireball's lumps are fewer and softer
than in the video, and its black smoke is less opaque (budget: fewer,
larger billows); the smoke column is shorter-lived than the
video suggests (it would read as a smoke screen: explosion smoke does not
block sight, screens do); the ammunition explosion has no video counterpart
(vehicle × 1.5 plus the big blast's dust dome).

**Time factor:** the video times are screen times and are used as they are;
the game's time factor 2 does not stretch effects.

**Open:** burning patch on the ground after big blasts; smoke trails on
debris; the lumps' inner motion (a flipbook) instead of only rotation and
turbulence; the demo video (`prompts/periodic/070-youtube-demo-video.md`)
still shows the old explosions and needs new recordings. [provisional]
all values.

### Rocket salvo (prompt 076)

Three stills from a promotional video of another game (a salvo of three
truck launchers at dusk in snow; outside the repository) served as a
reference for effect and sequence only, as in prompt 072. Code:
`world/rocket_effects.gd`, owned by `effects.gd` (`effects.rockets`).

**Which shells are rockets.** No simulation change: a `shot` event of
indirect fire (`weapon` 255) from a unit whose model has
`Socket_Rocket_NN` sockets is a rocket. `effects.gd` takes the unit's next
socket (round robin over the sockets in tree order) and calls
`rockets.note(projectile, socket, unit, caliber)`. `artillery_view.gd`
hands every noted shell to `rockets.flight(id, at, velocity)` instead of
drawing a ball; the first call starts the launch effects in the shell's
direction. That covers the vehicle launchers of prompt 062 (sockets from the
Blender kit) and the heavy strike's battery position, whose launch frames
(`strike_view.launchers()`) now carry 18 sockets numbered rail by rail
across the three racks, so the rockets leave one rail after the other over
all racks; `strike_view.aim_launchers` turns the racks towards the target
while a strike is under way. Another launcher needs only the sockets;
`rockets.launch(at, direction, caliber, station)` gives the launch effects
without a shell. The drawn rocket starts at its socket and blends onto the
simulated path within 0.35 s (a fading offset; the simulated path, its end
and the impact are untouched).

**Layers.**

| Layer | How | Values (source) |
|---|---|---|
| Body | `MultiMesh` of thin cylinders along the velocity | 1.41 m × 132 mm, real size (M-13; the reference shows the flame, not the rocket) |
| Flame | additive axial billboard (turns around the flight axis to the camera), a drop with a white-yellow core and an orange rim, HDR into the glow; flickers; shrinks in the last tenth of the burn | 5 m × 1.5 m (judged from the reference), scaled by caliber / 132 mm |
| Halo | additive round billboard at the flame's root | 3.5 m (judged) |
| Burn time | flame and new trail only while the motor burns; the rest of the flight (and the approach with the `shell_incoming` whistle) without flame | 1.0 s game time. The M-13 burns 0.85 s (docs/SOURCES.md), 0.43 s at time factor 2; with that only two flames per salvo were in the air and the trails stayed short, unlike the reference (judged deviation) |
| Smoke trail | one ribbon per rocket: points left in the world every ≥ 5 m while the motor burns (the head follows the rocket), drawn as a camera-facing strip; the shader widens (0.6 m + 1.0 m/s of age), fades (from 35 % of its life) and drifts (wind × 0.7, rise 0.25 m/s) every point by its own age, so the trail stands, is wider and paler behind and never tears into puffs; noise frays the edges; it stays after the impact | colour (0.93, 0.92, 0.9), opacity 0.6; life 4.5 / 7 / 9 s (low / medium / high) |
| Trail light | lit by sun and ambient light with a rounded fake normal tilted to the sky (dark at night where nothing burns); warm emission within ~12 m behind a burning flame, fading 0.3 s after burnout; the position's light lights the trails near the launcher | judged |
| Flare | an additive ball (white-yellow core, orange rim, HDR) at the rail's rear end per launch, 10 m, collapsing with a time constant of 85 ms (a few percent left before the next launch 250 ms later: readability); plus the `core` and `flash` bursts | reference: "as big as the vehicle, strongly overexposed" |
| Back blast | `rocket_blast` burst (new): light smoke thrown back and down out of the rail, hanging 3 s | judged |
| Ground | `blast_dust` 4 m behind the launcher in the ground colour (snow, sand, earth) | tint as for every effect |
| Light | one omni light per firing position (not per rocket), flaring with every launch and collapsing with the flare; 6 energy, 14 m range, × the daytime factors of `blast_effects.gd` (night × 3 and × 1.45 range) | reference: ground lit 10–15 m around |
| Haze | `salvo_smoke` (new, continuous): light haze over the position from the first launch, stops 3 s after the last one and drifts off with the wind (11 s particles); off on "low" | reference: the position stands in haze |
| Shake | a little camera shake per launch | judged |

**Factions.** The same effects for all; only the caliber scales flame,
halo and flare (Wurfgranate 41 150 mm, M-13 132 mm, M8 114 mm,
`data/strike.toml`). No per-faction burn times or trail densities: no
sources were looked up for them.

**Budget per quality** (`graphics_quality.gd`):

| Level | Trails at once (oldest reused) | Trail life | Trail width | Haze | Position lights |
|---|---|---|---|---|---|
| low | 24 | 4.5 s | × 0.7 | off | 1 |
| medium | 48 | 7 s | × 1 | on | 2 |
| high | 64 | 9 s | × 1 | on | 2 |

The worst case, two salvos at once (48 rockets, 48 trails, two positions),
fits medium and high; low keeps the newest 24 trails. Bodies, flames and
halos are three `MultiMesh` draw calls for every rocket; each trail is a
small `ArrayMesh` rebuilt only while its motor burns (≤ 96 points), then
only its instance uniforms change. New pools: `rocket_blast` 26 emitters
at high. Bursts use the usual LOD; flames and trails are not culled (thin
and cheap). Fog of war: launch flare, light, blast, dust and haze start
only at visible points (`effects.visible_at`); bodies, flames and trails
are shells and unfiltered, like the balls before.

**Measurements** (`just effects-bench --salvos`, 17.11.2026, 1920 × 1080,
Apple M1 Max, V-Sync off, debug bridge, forest, test_small; the user's
Godot editor and Blender were open). Two own units get the battery
position's model and fire 24 rockets each at the same time, 250 ms apart,
on open ground (the impacts with their big blasts are part of the load);
"balls" is the same salvo with the rocket effects off (`rockets.enabled`,
the look before prompt 076), measured back to back:

| Level | Idle, 150 m | 2 salvos 150 m: balls → rockets | 2 salvos 60 m: balls → rockets | Trails at once |
|---|---|---|---|---|
| low | 120 FPS (cap) | 119 → 113 FPS | 119 → 119 FPS | 24 |
| medium | 117 FPS | 105 → 100 FPS | 89 → 82 FPS | 48 |
| high | 64 FPS | 57 → 55 FPS | 52 → 51 FPS | 48 |

(an uncapped run of low: 162 → 146 FPS at 150 m, 138 → 130 at 60 m). The
rocket effects cost 0.3–0.5 ms at high and 0.5–1 ms at medium. "High"
stays below 60 FPS with two salvos, with and without the new effects: the
idle frame is already 64 FPS on this machine (see the explosion
measurements above) and the impacts' big blasts cost more than the
rockets. [open] The 50-explosion views of `just effects-bench` are
unchanged by this prompt (high 55 FPS at 150 m, 63 FPS at 60 m, 55 FPS with
24 wrecks; low and medium above 98 FPS). In two full runs the window
stopped being drawable after about five minutes (locked screen at night):
the bench now marks such views as invalid; `--salvos` is short enough.

**Tools.** `just salvo-shots [dir] [--only=<run>,…]` (`blast_shots.gd
--salvo-series`, 30 fps fixed): an own unit of the demo map gets the
battery position's model and fires 24 rockets 250 ms apart through
`SimHost.debug_rocket`; the run `winter_evening` is the reference view
(obliquely from above and behind, firing up and to the right, winter,
dusk), a picture every 0.2 s for 14 s; then the side view over the flight,
the approach at the target, day and night, forest and desert, and the
calibers of the other factions (`build/salvo/salvo_<run>_<ms>.png`).
`just strike-shots` also records the launch at the position of a real
strike and fails if fewer than 20 rockets got the launch effects.

**Open:** a launch sound of its own (hiss or howl): the battery position
plays `shot_howitzer` per rocket (its type has no catalog category), the
vehicle launchers `shot_rocket`; the battery position as real launcher
vehicles (prompt 062 brought them as units, the position still borrows the
ammunition dump); smoke trails for other weapons (Panzerfaust, Bazooka,
aircraft rockets) could use the same ribbon; the reference's density (about
ten rockets in the air, three launchers flaring at once) is not reached:
one position fires 4 rockets a second at 300 m/s, one rack flares at a
time, and the haze over the position is thinner (comparison picture:
`build/salvo/compare_reference.png`, not in the repository); the salvo was
checked in picture series, not in a recording with sound;
the demo video (`prompts/periodic/070-youtube-demo-video.md`) needs a new
recording for a shot with a strike. [provisional] all values.

### Muzzle blast (prompt 077)

A screenshot of another game (two tanks firing over a meadow, outside the
repository) served as a reference for **effect and size only**, as in
prompts 072 and 076: no cut-outs, textures or values; all textures are
generated, sizes are real meters (sources: docs/SOURCES.md "Muzzle blast").
Code: `effects.gd` (`_shot`, `muzzle_flash`, `_muzzle_dust`, guns),
`effect_library.gd` (kinds `muzzle_core`, `muzzle_tongue`, `muzzle_gas`,
`muzzle_dust`, `powder`), `effect_textures.gd` (`tongue()`),
`unit_merge.gd` (gun mesh, muzzle brake), `combat_view.gd` (tracers).

**Layers** (values for scale 1 = 7.5 cm; scale = caliber / 75 mm within
0.25 to 2.2, before 0.4 to 1.6):

| Layer | How | Values (source) |
|---|---|---|
| Core | additive ball at the muzzle, HDR into the glow, drawn over the gas | 2.4 m, 0.07 s (2-4 frames at 60 fps); never culled by LOD: at least 5 px on screen from any distance (it shows who fires) |
| Fire tongue | 5 flame quads that fly out along the barrel and stretch along their flight (`streak_add`: additive, aligned to the velocity), white-yellow to orange | 3 m (40 calibers; the intermediate flash stands at 20-25 calibers, NavWeaps), 0.1 s; × v / 750 m/s of the shell (0.65-1.35: a 7.5 cm L/24 short, a 8.8 cm L/71 long; the shell speed of the simulation / time factor 2) |
| Glowing gas | 9 billows thrown forward and stopped hard (`glow_billow`: unshaded atlas billows), white-hot, then pale; dimmed at dusk (× 0.75) and night (× 0.45) | ~9 m long, ~3 m thick (the reference: 1.5-2 vehicle lengths, one vehicle width), 0.55 s; off on "low" |
| Powder smoke | 10 pale puffs along the barrel (1-7 m), lit, drifting with the wind | 3 s, opacity at most 0.38 (units stay visible through it) |
| Side blasts | muzzle brakes: a tongue (0.35 of the length) and gas (0.45) to each side, slightly forward | off on "low" |
| Ground dust | `muzzle_dust` fanned ahead of a level barrel 1.5 m in front of the muzzle; a raised barrel (> 20°) throws `blast_dust` all round the gun | the ground colour mixed 40 % with light beige; amount by the dust-trail strength of the terrain (sand and tracks much, grass little, roads hardly; snow powder; water 0.8: mist); less from higher muzzles; 2.6 s |
| Light | the shared flash lights, warm, 1.5 m ahead of the muzzle | energy 4, range 9 m × (0.5 + 0.5 scale), 0.08 s; × the daytime factors of `blast_effects.gd` (night × 3, range × 1.45) |
| Tracer | the drawn shell is shifted from the simulation's muzzle to the socket by an offset that fades over 40 m of flight (`SimHost.projectile_ids`, bridge only); its tail never reaches back behind the muzzle | — |
| Shake | as before, × class | 0.08 × scale |

**Classes** (`MUZZLE_CLASSES`, factors on core, tongue, gas, smoke, dust,
light, shake, recoil, rocking):

| Class | When | Look |
|---|---|---|
| autocannon | direct fire, 20 to < 40 mm | every round of a burst: core × 1.4, small tongue, little gas and smoke (0.35), no dust, no shake |
| gun | 40 to < 85 mm | as above |
| heavy | from 85 mm | tongue × 1.1, gas × 1.15, smoke × 1.2, dust × 1.3, light × 1.3 |
| howitzer | indirect fire (`weapon` 255) from a model with a `Gun` | tongue × 0.55, gas × 1.25, smoke × 1.6, dust × 1.6 (all round) |
| mortar | indirect fire from a squad, a model without `Gun` or a barrel modeled steeply (muzzle well above the trunnion: Granatwerfer 42, PM-38) | core × 0.6, no tongue, a puff of gas and smoke |

Rockets (`Socket_Rocket_NN`, prompts 062 and 076) never get a gun's blast:
with the rocket effects off (bench comparison) they keep the small flash of
before. Small arms below 20 mm are unchanged.

**Direction.** The blast points along the real barrel: from the `Gun`
node's origin (the trunnion) to its `Socket_Muzzle` (the socket's -Z if the
socket is not on the gun); without a socket the simulation's projectile
velocity, else the aim yaw. Cannon rounds take the main gun's
`Socket_Muzzle` (within 6 m of the simulation's muzzle), small arms the
machine guns' sockets: before, the nearest socket was taken, and the
simulation's muzzle is often nearer to the bow MG, so the flash of a
Panzer IV's gun came out of its hull MG.

**Raised barrels.** An indirect shot raises the `Gun` to the shell's launch
elevation (its velocity in `shell_velocities`; if the shell shows up only in
a later snapshot, the barrel follows at 12°/s). It stays raised 25 s after
the last round or until the unit moves, then sinks at 12°/s. Models are
built with level barrels; the first round of a laid gun snaps up with the
flash.

**Recoil and rocking.** `unit_merge.gd` now merges the parts of the `Gun`
into one mesh on the `Gun` node (+1-2 draw calls per gun vehicle), so the
gun can move: it runs back 0.005 m per mm of caliber (at most 0.6 m;
judged from real recoil lengths of 0.3-0.6 m) in 0.05 s and forward again in
0.7 s; the mantlet is part of the gun's mesh and moves with it (accepted:
hardly visible from the game camera). The hull (`Model` node) rocks about
the horizontal axis across the shot, muzzle side up first: 0.8° × scale,
1.6 Hz, decaying in 0.35 s. Only the `Gun` and `Model` nodes move; the unit
node, which the simulation, the selection and the turret aim use, stays.
Kill rings (`veterancy_view.gd`) and sockets hang on the gun and move with
it. `effects.recoils` counts recoils (self-test).

**Muzzle brakes.** No new data path: the kit (tools/blender/kit/weapons.py)
builds a brake 1.6-1.9 barrel radii wide (double baffle), 1.7 (single), a
slotted one 1.35 and a thread protector 1.15. `UnitMerge.has_muzzle_brake`
compares the widest vertex within 0.45 m of the muzzle face with the
narrowest ring of vertices 0.1-1.5 m behind it; a ratio above 1.45 is a
brake (once per type, `model.get_meta(&"muzzle_brake")`). Of the 31
models whose blueprint has a double or single baffle brake it finds 29
(Panzer IV F2-J, Tiger, Tiger II, Panther, Elefant, Nashorn, Marder, leFH
18, IS-2, IS-3, M26, M36, ...); it misses StuG III G and Marder III
(Sd.Kfz. 139), and it reports four models without a brake: the two mortars
(mortar class: no side blasts), the Panzerbefehlswagen's dummy gun and the
T-27's MG mount. Slotted brakes and flash hiders (ZiS-3, SU-152, Bofors)
count as none (judged: no visible side blasts).

**Budget.** New pools at "high": `muzzle_core` 24, `muzzle_tongue` 40,
`muzzle_gas` 32, `muzzle_dust` 40 (× `fx_pools`); `powder` keeps 40, so at
40 shots a second the oldest powder smoke is cut short after ~1 s.
`fx_muzzle_detail` (`graphics_quality.gd`): "low" leaves out the glowing gas
and the side blasts. LOD as for every burst, except the core. Fog of war:
every layer starts only at visible points (`visible_at`).

**Tools.** `just muzzle-shots [dir] [--only=<run>,…]`
(`tools/muzzle-series.sh`, `effects_bench.gd --muzzle-series`, 30 fps
fixed): one own unit gets the model of each class in turn and fires; every
frame of the first half second, then four pictures a second to 4 s, tile
sheets around the muzzle (`<dir>/sheet_<run>.png`): the reference view
(8.8 cm, obliquely from above) beside the reference picture if it is in
`.temp/` (`compare_<ms>.png`), 2 cm (a burst of five), 3.7, 7.5, 7.62 cm
without brake, 8.8, 12.2 cm, the 12.2 cm M-30 firing indirectly with a
raised barrel, 7.5 cm at 15, 110 and 350 m, 8.8 cm at dusk and night,
7.5 cm in desert and winter; last a real fight of both sides recorded as
`<dir>/battle.mp4` (every third frame; real shells: tracers, recoil).
`just effects-bench -- --muzzles` measures 40 cannon shots a second (round
robin over every vehicle of the map, 3.7 to 12.2 cm, plus two 2 cm
autocannons at 10 rounds a second) from 150 m and 60 m.

**Measurements** (`just effects-bench --muzzles`, 17.11.2026, 1920 × 1080,
Apple M1 Max, V-Sync off, debug bridge, forest, test_small), before → after,
back to back (two runs each):

| Level | Idle, 150 m | 40 shots/s, 150 m | 40 shots/s, 60 m | Draw calls idle |
|---|---|---|---|---|
| low | 183-193 → 189-192 FPS | 180-184 → 184-187 FPS | 178-184 → 162-179 FPS | 465 → 483 |
| medium | 122-123 → 123 FPS | 115-119 → 115-121 FPS | 132 → 133 FPS | 661 → 687 |
| high | 68 → 65-66 FPS | 65 → 63-64 FPS | 84 → 84 FPS | 661 → 687 |

"High" stays above 60 FPS with 40 shots a second; the idle frame costs
~0.5 ms more since the guns are meshes of their own (+26 draw calls on
test_small), the blasts themselves ~0.1-0.2 ms at 150 m. The full
`just effects-bench` (all views one after the other; units destroyed by the
barrages and salvos differ from run to run, so its later views are noisy)
before → after, high: idle 64 → 67 FPS, 50 explosions at 150 m 57 → 55, at
60 m 64 → 63, + 24 wrecks 55 → 54, 40 shots/s at 150 m 60 → 58 (with the
wrecks of the views before), at 60 m 82 → 79; low and medium stay above 98
FPS. A first full run after the change gave high 54 / 51 / 51 / 44 FPS (draw
calls +100 from more burning wrecks); the repeat matched the run before, so
it was counted as an outlier. "High" with 50 explosions stays below 60 FPS,
as before prompt 077 [open].

**Open:** the mantlet recoils with the barrel (one mesh in the kit; a
`Barrel` node of its own would need new exports); muzzle brakes read from
the geometry miss two models and add two dummy guns (above), slotted
brakes get no side blasts (a `muzzle_brake` property on the socket in the
kit would be exact, but needs every model exported again); the first round of a howitzer snaps its barrel up; models without a
`Gun` (some casemates, aircraft) only get the blast, no recoil; the
reference's dust behind the firing tank (track dust of a moving tank) is
the dust trail of prompt 019, not part of the blast; machine guns and small
arms, ejected cases, smoke from the breech or the open fighting
compartment, grass pressed down by the blast (prompt 073), a sound of
their own (prompt 069) are not part of this prompt; the demo video
(`prompts/periodic/070-youtube-demo-video.md`) still shows the old muzzle
flash and needs new recordings. [provisional] all sizes, times and
classes.

## Audio (presentation only)

Prompt 029. Sound reads the same events and snapshot as the effects and
never reaches the simulation (golden hash unchanged; Godot randomness is
fine). Code in `godot/scenes/audio/`, files and manifest in
`godot/assets/audio/` (conventions: [ASSETS.md](ASSETS.md#audio-prompt-029),
origin: [SOURCES.md](SOURCES.md#sounds)).

| Script | Does |
|---|---|
| `sound_library.gd` | Reads `sounds.json`, loads the files of each class (`ResourceLoader.list_directory`, so exports work), makes loops loop, checks the library |
| `sound_classes.gd` | Catalog facts → class: shots by category, caliber and rate of fire; engines by category and mass; hits by surface, caliber, penetration, water; map objects by stage and remains |
| `battle_sound.gd` | Created by `main.gd` (left out with `-- --no-sound`). Listener, voice pools, one-shots for combat, object and crater events, engine/track/fire loops, match notices |
| `ui_sound.gd` | One node under the tree root for the whole run (installed when the settings are applied): UI voice pool, a click for every `BaseButton`, mute on focus loss |
| `sound_buses.gd` | Creates the buses in code and sets their volumes from the settings |

The bridge adds one read function, `SimHost.unit_type_sound(type)`:
catalog id, category, mass, road speed and the weapons in the order of
the shot event's `weapon` index (caliber, rate of fire). Classes are cached
per unit type and weapon.

### Events and sounds

| Trigger | Class |
|---|---|
| `shot` | `shot_rifle` / `shot_mg` (below 20 mm, rate below / from 300 rpm), `shot_rocket` (anti-tank squads from 20 mm), `shot_mortar` (mortar squads, heavy mortars, "mortar" / "GrW" in the weapon name), `shot_howitzer` (field howitzers, self-propelled guns from 100 mm), `shot_cannon_small` / `_medium` / `_large` (20 / 37 / 75 mm); a weapon with files in `weapon/<slug>/` plays them instead (prompt 053) |
| `hit`, ground | `hit_bullet_ground`, `hit_shell_small` (below 37 mm), `hit_shell` (louder with caliber, ±6 dB), `hit_shell_water` in water |
| `hit`, vehicle | `hit_bullet_armor`, `hit_ricochet` (not penetrated), `hit_penetration` |
| `hit`, infantry / object | shells: as on the ground / `hit_object`; bullets: nothing / `hit_bullet_ground` |
| `destroyed` | vehicles `explosion_vehicle` (by size, from medium-tank size with `explosion_tail`) and a `fire` loop for 60 s, 90 s quieter while smoldering; base buildings `building_collapse`; infantry nothing |
| object event | `building_damage`, `building_collapse` (ruin: buildings, ammunition and fuel dumps), `tree_fall`, `wall_break` (gone: walls, hedges, hay bales, woodpiles), `fence_crush` (flattened by vehicles: wooden fences from 2.5 t, barbed wire from 8 t; `data/destruction.toml`) |
| crater event | `crater` (earth raining down, louder with caliber; the impact itself is the hit; engineers digging are silent) |
| indirect fire (prompt 032) | shot events with `weapon` 255: `shot_mortar` / `shot_howitzer` by category, `shot_rocket` for rocket launchers (prompt 062); `artillery_view.shell_incoming`: `shell_incoming`, a 1.5 s whistle at the impact point (smoke shells 8 dB quieter); `smoke` events: a quiet `hit_shell_small` |
| moving vehicles | `engine_wheel` / `engine_halftrack` / `engine_track_light` / `engine_track_heavy` (from 20 t) plus `tracks` for tracked vehicles; pitch 0.8–1.2 and volume by speed / road speed |
| notices (UI bus) | `ui_order` (move, attack, garrison), `ui_select` (the selection changes and is not empty), `ui_unit_ready` (a new own unit, not a building, after the first 2 s), `ui_build_complete`, `ui_under_attack` (own unit hit or enemy aircraft in sight, every 20 s at most), `ui_unit_lost` (6 s), `ui_victory` / `ui_defeat` (also on a draw); `ui_click` on every button of every screen |

### Hearing

An `AudioListener3D` stands above the camera's ground focus, raised by
12 % of the camera distance (at least 6 m), facing the camera's view
direction on the ground, so left and right match the screen. Attenuation
is inverse distance from `ref_m` up to the cut at `range_m` (manifest);
both grow with the zoom (`hearing_scale` = camera distance / 150 m, 1 to
4): close up you hear the skirmish in front of you, in the overview the
whole battle, quieter. Godot's distance low-pass darkens far sounds.

### Budget

| Group | Voices | New per frame | Classes |
|---|---|---|---|
| small_arms | 12 | 3 | rifle, MG |
| cannon | 10 | 4 | cannons, rockets, mortars, howitzers |
| impact | 14 | 4 | bullet and shell hits, ricochets, penetrations, objects hit |
| explosion | 8 | 3 | shell bursts, vehicle explosions |
| object | 6 | 3 | buildings, trees, walls, fences, craters |
| ui | 6 | – | notices and clicks (`ui_sound.gd`) |
| loops | 6 vehicles × 2, 4 fires | every 0.2 s | engines and tracks of the nearest vehicles, nearest burning wrecks |

A new sound takes a free voice, else the weakest busy one if that is not
stronger: score = priority × 12 + estimated loudness at the listener / 3,
lowered while a sound fades out (age / length). Cannons (priority 3–5)
always win against rifles (1); a near rifle beats a far rifle. Sounds
beyond the hearing range are culled before any work. All players are
created once; the self-test fires 200 events per frame for 30 frames and
checks the pools and the per-frame budget.

### Fog of war

`battle_sound.visible_at` is the effects' visibility query. Sounds at points
the player does not see are **muffled, not silent**: they play on the
`Muffled` bus (send to Effects, low-pass at 700 Hz, −6 dB) 3 dB quieter and
up to 15 % of their distance off their true position. Battle noise beyond
the front is information soldiers had, but it does not give away an exact
position. Enemy vehicles out of sight are not in the snapshot, so their
engines are not heard.

### Buses and settings

`Master` → `Effects` (→ `Muffled`), `UI`, `Music`, `Ambience` (the last two
for prompt 030), created in code by `sound_buses.gd` (no bus layout file).
Settings keys `master_volume` (as before), `effects_volume`, `ui_volume`,
`music_volume`, `ambience_volume` (linear 0–1, 0 mutes the bus) and
`mute_unfocused` (default on: Master is muted while the window has no
focus). The Audio tab of the settings screen has a slider per bus.

### Tools

`just audio [classes]` synthesizes the files (`tools/audio/synth.py`, then
`just import`); `just sound-gallery` plays every class once 40 m in front
of the listener and lists them for replay (`--sound-gallery-once` quits
after the round, exit code 1 on a missing file). The self-test part
`tools/sound_self_test.gd` (in `just selftest`): every class loadable,
loops loop, the earlier combat, destruction and crater tests asked for
their sounds, bus volumes follow the settings, hidden sounds play muffled,
own tanks get engine loops, the voice limit holds under load.

`just sounds-web` serves a local page (`web/sounds/`, `tools/sounds_web.py`, prompt 083)
that plays every audio file of `godot/assets/audio/` with a remark on when the game
plays it (`web/sounds/usage.toml`, taken from the code), its origin and its manifest
values; `just sounds-web --check` (in `just check` and CI) fails when a sound folder has
no remark or a remark no folder. It stays local: no audio file is copied under `web/`.

### Measurements

`just bench-view` (prompt 016 scenario, 1920 × 1080, quality high, debug
bridge, Apple M1 Max) with sound and with `-- --no-sound`, two runs each,
alternating, while other sessions loaded the machine (load average 18–55):

| View | Sound, run 1 / 2 | No sound, run 1 / 2 |
|---|---|---|
| standing, 40 m | 54.9 / 87.9 FPS | 55.1 / 93.0 FPS |
| standing, 150 m | 72.4 / 79.0 FPS | 66.4 / 81.7 FPS |
| standing, 600 m | 79.5 / 92.9 FPS | 99.1 / 99.5 FPS |
| battle, 40 m | 38.7 / 68.8 FPS | 59.3 / 74.8 FPS |
| battle, 150 m | 46.9 / 66.2 FPS | 63.2 / 68.3 FPS |
| battle, 600 m | 41.6 / 56.8 FPS | 47.5 / 55.7 FPS |

The runs differ more between themselves than sound against no sound
(the simulation's share of the frame, untouched by sound, varied by 50 %
between runs). In the calmer second pair sound costs between −8 % and +2 %.
What is stable is the main-thread time of the sound code: 0.11–0.20 ms per
frame for `battle_sound._process` (the "sound" column of the bench) plus
about 0.1 ms for the event handling inside the combat view, together under
2 % of a 60 FPS frame. Mixing runs on Godot's audio thread. The self-test
measures 8 µs per combat event in the sound code (debug build). The 5 %
acceptance criterion needs a run on a quiet machine to confirm.

### Decisions

| Question | Decision | Why |
|---|---|---|
| Origin of the sounds | Synthesized with numpy (`tools/audio/synth.py`), reproducible like the Blender models; a pipeline for files of other origin | No downloads, no licenses, no sounds from other games. [provisional] The synthesized set is a first pass, tuned by numbers (spectrum, level) without listening; recordings (CC0) or generated sounds can replace any class without code changes. |
| Format | WAV everywhere, loops with a `smpl` chunk | [provisional] No usable Vorbis encoder without installing one (ffmpeg's built-in encoder is experimental, stereo-only and not reliably gapless); 16 MB for 97 files. Revisit with music (030). |
| Buses | Created in code | No bus layout file to merge; the same buses in the menu, lobbies, editor and tools. |
| Listener | Above the camera's ground focus, hearing range scaled with zoom | At the camera itself a high view would hear everything equally faint. |
| Voice limit | Fixed pools per group, priority and loudness decide, budget per frame | Hundreds of shots per second neither clip nor cost frame time; nothing is allocated during a battle. |
| Fog | Muffled and slightly displaced, not silent | Battle noise is reconnaissance, an exact position is not. [provisional] |
| Speed of sound | Not simulated in 029; since prompt 053 far sounds wait for their travel time (see below) | 029: a delay could read as lag; 053 adds it with a 2.5 s cap and keeps notices and incoming whistles immediate. |
| Mute without focus | On by default | [provisional] Usual for strategy games; a setting. |
| Rockets | Own class `shot_rocket` for anti-tank squads | Panzerschreck, Panzerfaust and Bazooka have 60–150 mm calibers and would otherwise sound like tank guns. |

### Open

- Listening: the set was made and balanced by measurement only; levels,
  ranges and recipes want a pass by ear (`just sound-gallery`).
- Voices: structure since prompt 053, no files yet. Music and ambience:
  see below (prompt 030).
- Infantry hits and deaths are silent (053 adds only the `wounded` callout).
- Indirect fire (prompt 032) is wired by its interface (weapon 255,
  `shell_incoming`, `smoke` events) as described in its merge notes; it was
  not merged yet when this was written, so its sounds are tested only once
  both are in main.

### Stage 2: acoustics and voices (prompt 053)

| Script | Does |
|---|---|
| `acoustics.gd` | Created by `battle_sound.gd`, driven from its `_process` (its time counts in the bench's "sound" column). Speed of sound queue, supersonic crack, reverb by surroundings, zoom filter, aircraft engines and air events |
| `battle_voices.gd` | Created by `battle_sound.gd`, fed its combat events. Callouts of squads near the camera per faction |

- **Speed of sound.** 343 m/s real × time factor 2 = 686 game m/s.
  `battle_sound.play_far` (shots, hits, explosions, destruction, object and
  crater events, flak, strafing, crashes) waits ground distance to the
  camera focus / 686 s, at most 2.5 s [provisional]; under 0.04 s it plays
  at once. `play_at` stays immediate (notices, incoming whistles, cracks,
  voices). The queue is sorted by due time, at most 192 entries; when full,
  small arms are dropped and stronger sounds push out the weakest. The
  per-frame voice budget applies when a sound leaves the queue.
- **Supersonic crack.** Once per simulation tick (only up to 200 m camera
  distance) every projectile faster than 686 m/s is checked: if its
  closest point to the hearing center (focus + 1.5 m) lies on its next
  tick of flight and within 12 m (below 20 mm) or 25 m (shells),
  `crack_bullet` / `crack_shell` plays there at once, so the crack comes
  before the delayed muzzle report. No projectile ids are needed: each
  stretch of flight is checked exactly once.
- **Reverb by surroundings.** Every 0.5 s a 5 × 5 grid of
  `wood_density_at` and `SimHost.buildings_near` (box-shaped objects from
  3 m high; new read function in `audio_funcs.rs`) within 35 m × zoom (up
  to 70 m) give shares of forest (density / 0.6), village (buildings / 8)
  and open field. One `AudioEffectReverb` on the Effects bus blends three
  presets [provisional, set without listening]: open nearly dry with a
  late reflection, forest dense and dark, village short and bright.
- **Zoom.** A low-pass on the Effects bus goes from 20 kHz at 40 m camera
  distance to 2.2 kHz at 450 m (logarithmic); classes of priority ≤ 1
  (rifles, MGs, bullet hits) lose up to 8 dB in the overview. Both effects
  are reset when the match ends (`SoundBuses.reset_effects`).
- **Blasts by size.** Shell hits get pitch (75 / caliber)^0.2 (0.75–1.2);
  from 100 mm an `explosion_tail` (5 s roll, own group `tail`) follows,
  louder with caliber (bombs included). Vehicle explosions (ammunition and
  fuel) go lower with footprint and get the tail from medium-tank size.
- **Weapon recordings.** `<class>@<key>` names (`sound_library.gd`) play
  the files of `assets/audio/weapon/<key>/` with the settings of
  `<class>`. A shot uses `<class>@<weapon slug>` if that folder has files,
  else its synthesized class (checked once per unit type and weapon).
- **Aircraft.** The three aircraft nearest the listener get
  `engine_aircraft` or, from 18 m wingspan, `engine_bomber`; pitch by
  Doppler (686 / (686 − radial speed), 0.75–1.35), higher while falling.
  `air_view` emits `air_event` for every air event: heavy flak
  `flak_burst` at the burst, light flak `shot_cannon_small` and MG flak
  `shot_mg` at the gun, strafing `shot_mg` at the muzzle, crashes
  `explosion_vehicle` with tail.
- **Fires.** Burning map objects (`SimHost.fires`, keys ≥ 0, prompt 041)
  join the wrecks for the fire loops (now 6): `fire_large` from 8 m, louder
  by size (±6 dB), down to −12 dB as they burn out.
- **Voices.** `voice@<faction>/<callout>` from
  `assets/audio/voice/<faction>/<callout>/`; faction = prefix of the unit
  type id. Callouts and triggers [provisional]: `contact` (a squad fires
  after 20 s without firing), `reloading` (one shot in 12), `take_cover`
  (a shell hits a squad), `wounded` (a bullet hits a squad); only up to
  80 m camera distance, for visible squads within 50 m of the focus, one
  callout per 4 s, per squad per 12 s; group `voice` (3 voices). No files
  ship: without files a callout only counts (`missing`).

| Group | Voices | New per frame | Classes |
|---|---|---|---|
| crack | 4 | 2 | `crack_bullet`, `crack_shell` |
| tail | 4 | 2 | `explosion_tail` |
| voice | 3 | 1 | callouts |
| loops | 3 aircraft, 6 fires | every 0.2 s | `engine_aircraft`, `engine_bomber`, `fire`, `fire_large` |

Self-test (`tools/sound_self_test.gd`): weapon slugs and fallback, a shot
400 m away waits 0.58 s and then plays, a supersonic bullet passing 3 m
away cracks while subsonic and receding ones do not, surrounding shares
sum to 1, the Effects bus holds low-pass and reverb, the overview is
darker than close up, a callout without files only counts.

Measurements (`just bench-view`, Apple M1 Max, debug bridge, load average
80–160 from parallel sessions, one run each): the "sound" column
(`battle_sound._process` including `acoustics.update`) is 0.13–0.14 ms
standing and 0.21–0.27 ms in the battle (029: 0.11–0.20 ms), under 2 % of
a 60 FPS frame. FPS with / without sound: battle 40 m 53.1 / 50.1, 150 m
47.8 / 55.3, 600 m 43.0 / 46.9; standing 79.9 / 77.6, 78.5 / 70.8, 87.4 /
82.5. The spread comes from the machine load (the same views differ by
±10 % in either direction). The self-test stress measures 18–27 µs per
combat event in the sound code under that load (limit 60 µs; 029: 8 µs
on a quieter machine).

### Music and ambience

Prompt 030. Files from `tools/audio/music.py` (`just music`, conventions in
[ASSETS.md](ASSETS.md#music-and-ambience-prompt-030), origin in
[SOURCES.md](SOURCES.md#sounds)).

| Script | Does |
|---|---|
| `music.gd` | One node under the tree root for the whole run (like `ui_sound.gd`), so music plays on across scene changes. Moods `menu`, `calm`, `combat`, `victory`, `defeat`; two players on the `Music` bus cross-fade (4 s between moods, 2 s overlap between pieces). Static `play(mood)`, `stop()`, `current_mood()`. Silent with `-- --no-sound`. |
| `match_music.gd` | Created by `battle_sound.gd`. Calm when the match is loaded (the menu piece plays on behind the loading screen), combat by intensity, victory or defeat from `match_result()`; a draw fades out. |
| `ambience.gd` | Created by `battle_sound.gd`. Wind, insects, leaves, water, rain, birds and rumble on the `Ambience` bus. |

The main menu sets `menu`; the lobbies and the map editor keep whatever
plays.

**Calm and combat.** Every combat event the player sees adds to an
intensity (small-arms shot 0.5, gun shot from 20 mm 1.5, hit from 20 mm 1,
destroyed unit 6) that decays with 10 s. Combat starts from 12 (a few tanks
or squads fighting in sight) and ends below 4, at the earliest 25 s after it
began. Fighting in the fog is heard muffled but does not change the music.
Calm pieces have 20–60 s of silence between them; combat pieces follow one
another.

**Ambience.** Per biome (`atmosphere.preset.id`, followed at runtime) a wind
bed, an insect bed (meadow crickets, desert cicadas, none in winter) and a
bird set. Every 0.5 s a 7 × 7 grid around the listener (radius 40 m ×
`hearing_scale`) reads `terrain_at` (water) and the new bridge read
`SimHost.wood_density_at` (the decoration's wood raster, 8 m cells): water
and leaves are one `AudioStreamPlayer3D` each at the center of their
samples, without distance attenuation, as loud as their share of the grid
(silent below 2 %); the position only gives the direction. Camera distance
mixes near and far: at 60 m and closer the details (insects, leaves, water,
birds) are full and the wind 8 dB lower; at 450 m and beyond the details are
24 dB lower and the wind full; logarithmic in between. Birds call at wooded
samples every few seconds (forest 2–7 s, desert 8–20 s, winter 9–24 s); any
combat event within 250 m × `hearing_scale` of the listener silences them
for 60 s, then they come back over 20 s. A distant rumble every 40–120 s
from a random direction 400 m away. `rain` (0..1) is ready for the weather
of a later prompt: a rain bed, fewer birds, quieter insects.

**Memory.** Music: 7 pieces, 7.4 MB of MP3 in total, decoded while
playing (Godot's `AudioStreamMP3` holds only the compressed data, about
16 KB per second instead of 176 KB as PCM). Ambience: 8 beds and 18
one-shots, 17 MB of WAV on disk, QOA in memory (about a fifth).

**Self-test** (`tools/music_self_test.gd`, in `just selftest`): every mood
has a compressed, non-looping piece; every ambience layer loads and the beds
loop; the match starts calm, one shot does not switch, ten gun shots do,
combat ends only after the minimum time; each biome plays its wind and
insects; the grid hears water and wood of the map; close up the wind is
lower and the insects louder than in the overview; birds are silent right
after combat nearby and sing later; a rumble plays. The flow test checks
in the demo battle of match 2 that the music switches from calm to combat
and ends with the victory piece, and that the menu plays the menu music
again.

| Question | Decision | Why |
|---|---|---|
| Origin | Composed and rendered with numpy (`music.py`) | No downloads, no rights questions, reproducible. [provisional] Made without listening, kept simple and restrained; any piece can be replaced by a file in `assets-src/music/`. |
| Format | MP3 (LAME) for music, WAV loops for ambience | No Vorbis encoder without installing one; MP3 decodes while playing. Pieces are cross-faded, not looped, so MP3's padding at the ends does not matter. Ambience loops need to be seamless: WAV. |
| One node for music | Under the tree root, like `ui_sound.gd` | Music must not break off at scene changes (menu → loading → match). |
| Loading screen | No own piece, the menu piece plays on | Loading takes seconds. |
| Thresholds | Combat from 12, calm below 4 after 25 s, decay 10 s | Hysteresis against switching back and forth. [provisional] |
| Music level | −8 dB under the bus slider | Music sits under the battle. [provisional] |
| Wood | Bridge read of the decoration's wood raster | Trees are map objects, not a terrain type; the raster exists already. Presentation only, no sim change. |
| Birds after combat | Silent 60 s, back over 20 s | [provisional] |

Open:

- Listening pass: the pieces, levels and thresholds were made by numbers.
  The music is deliberately plain (pads, piano-like phrases, low strings,
  timpani); it is a placeholder quality that a composer or user-provided
  files should replace.
- Rain waits for the weather prompt (set `ambience.rain`).
- Ambience in the map editor, and pieces per faction or biome.

## Selection (presentation only)

Selection is local UI state in `godot/scenes/selection/selection.gd`; only the
resulting orders reach the simulation. The bridge exposes per-unit packed arrays in
the same order as `unit_ids()`: `unit_owners()`, `unit_types()`, `unit_footprints()`,
`unit_moving()`, `unit_alive()`, plus `local_player()`. Only living units of the local
player are selectable; destroyed units leave the selection.

| Input | Effect |
|---|---|
| Left click | Select the own unit under the cursor; on empty ground clear the selection |
| Shift + left click | Add the unit (or remove it if already selected) |
| Left drag | Selection box (own units whose hull center is inside) |
| Double click on a unit | All visible own units of the same type |
| Double click on empty ground | Camera glides there; the selection stays (the first click's deselect is undone) |
| C | Follow mode: the camera keeps the selection's center in view; ends on any hand pan or when the selection is empty |
| Ctrl+1…9 (Cmd on macOS) | Store the selection as a group |
| 1…9 | Recall the group; twice within 0.4 s also centers the camera on it |
| Right click | Move the selection to the ground point (green marker for 1 s) |
| Shift + right click | Append a waypoint (`SimHost.queue_move`, prompt 031) |
| Right click on an enemy unit | Attack it |
| Ctrl+right click (Cmd on macOS) | Fire at the ground point |
| H | Stop the selection |
| F | Hold fire / fire at will for the selection |
| M, R, L, P, U | Targeted order (move, attack, fire at ground, patrol, occupy): waits for the next left click (`command_mode`; Shift keeps it and queues moves), right click cancels |

Since prompt 031 the order, stance, follow and event keys are input actions
(`order_*`, `stance_*`, `camera_follow`, `event_jump` in `project.godot`),
rebindable in the settings; control groups stay on the fixed keys 1…9. A
box drag that started on the map ends wherever the button is released, also
over a HUD panel (`selection.gd` takes the release in `_input`). The player's
overview of all inputs is [CONTROLS.md](CONTROLS.md).

Picking projects unit positions to the screen (`Camera3D.unproject_position`); the pick
radius is half the footprint on screen, at least 14 px. Selected units show a green
ring; units of other players carry a red tint overlay.

## HUD (presentation only)

Prompt 031, `godot/scenes/hud/` (created by `main.gd` as `hud`): minimap,
unit panel, command bar, event feed and waypoint lines. Nothing here
decides anything; orders go through `selection.gd` or `SimHost`.

- **Minimap** (`minimap.gd`, bottom right): `SimHost.minimap_image` renders
  the ground once in Rust, one RGB pixel per 8 m vision cell (terrain color,
  hill shading from the vision grid's heights, roads if any sample of the
  cell is a road, forest and solid obstacles from the vision grid). One
  canvas shader multiplies it with the fog texture of `fog_of_war.gd` (the
  same grid; never seen 12 %, explored 45 %). Units are one
  `MultiMeshInstance2D` whose buffer (2D transform + color per dot) comes
  from `SimHost.minimap_dots` in one call, only for the units of the local
  player's snapshot. The camera's view is the screen corners on the ground
  (the focus height where a ray misses); pings mark hits on own units. Left
  click/drag looks there, right click moves the selection (Shift: waypoint).
  Not redrawn after craters.
- **Unit panel** (`unit_panel.gd`, bottom center): `SimHost.unit_info` (name,
  hit points, armor per face, weapons with caliber and game range, soldiers);
  stance, fire mode and queued waypoints from the per-unit arrays; under
  the portrait the five ratings of the unit's type, in its corner the
  button to its unit card, and the bonuses of its rank as a line of their
  own (prompt 066). The
  portrait (`unit_portrait.gd`, [provisional]) is rendered once per unit type
  in its own `SubViewport` from a fresh model (duplicating a unit's model
  fails: its parts are merged at run time); a pre-rendered thumbnail
  (prompt 058) replaces it when there is one. Several units: tiles with health
  bars; click selects one, Shift + click removes it.
- **Command bar** (`command_bar.gd`, bottom left): buttons with the bound key
  for every order; targeted orders set `selection.command_mode`. The build
  menu and the production bar of `world/base_hud.gd` are docked above it
  (`BaseHud.dock_into`, [provisional]); "Build" opens the build menu.
- **Picker** (`picker.gd`, prompt 058, [provisional]): the build menu and the
  production bar show their options as picture tiles: thumbnail on a
  faction-tinted backdrop, name, fuel / ammunition / manpower, build time,
  hotkey (Alt + 1…0 for the open tab), count of queued orders or own
  construction sites and the progress of the first as a bar over the
  picture; blocked tiles are dimmed with the reason. Tabs by kind: base
  buildings, other structures (defense), then by catalog group (infantry,
  vehicles, guns, aircraft); empty tabs are hidden. Everything comes from
  `build_options` / `production_options`, so new buildables appear without
  changes. Every tile shows the five ratings of its type and its tooltip
  is a panel with their numbers (prompt 066, see "Unit values in the
  interface" below); base buildings keep the catalog values of
  `SimHost.catalog_card` (hit points, income, requirements). A right click
  on a tile emits `info_requested` (the unit card). Two rows of tiles
  show, the rest scrolls; in a window lower than 800 interface pixels one
  row, so the docked menus stay inside a window 720 pixels high. The
  thumbnails are PNGs in
  `godot/assets/thumbnails/` (`hud/thumbnail.gd`, docs/ASSETS.md), rendered
  by `just thumbnails` for every id of `SimHost.buildable_ids()`; any own
  structure with production options gets the production bar. When the
  docked panels make the command bar tall, `hud.gd` moves the event feed
  above it, or beside it when there is no room above (prompt 066).
- **Event feed** (`event_feed.gd`, left edge): own unit lost, building
  completed, unit ready, enemy sighted (at most every 15 s), under fire (at
  most every 10 s), from the combat event stream and a poll of the snapshot
  every 0.25 s. Click looks there, `event_jump` (Space) jumps to the newest.
- **Waypoint lines** (`waypoints.gd`): for up to 48 selected units the
  remaining route and the queued waypoints (`SimHost.unit_waypoints`),
  projected to the screen; markers at queued waypoints, patrols in blue.
- **Health bars** (`combat_view.gd`): setting `health_bars` = `damaged`
  (damaged and selected units, default), `selected` or `never`.
- **Cost:** every part adds its `_process`/`_draw` time to `hud.usec`;
  `--perf-bench` prints it per view (column "hud").

### Unit values in the interface

Prompt 066. The game shows the cards of `sim::unit_stats` (see "Unit stats
(display only)") in four places; GDScript only formats, no value is
computed there.

- **`hud/unit_stats.gd`** (static): fetches `SimHost.unit_stats(id)` once
  per type and keeps it (the bridge computes a card on every call;
  `fetches` counts the calls for the self-test), likewise
  `unit_stats_reference`, `vehicle_record` and `catalog_card` (per
  language, for names). `num` writes numbers in the interface language
  (German `1.234,5`, English `1,234.5`). It builds the tooltip's lines: at
  most eight values (speed, armor, main weapon with penetration at 500 m
  and game range, effect against infantry and structures in a word, men and
  seats) and the "good against … / vulnerable to …" line. That line is read
  off the card: the reference tanks of `against_vehicles.targets` grouped
  by their category (all of a class penetrated from the front, or only from
  the side), the ratings (strong from 60), and the first comparison weapon
  of `protection.versus` that gets through the hull front and the side. The
  words: 0 "none", below 25 "weak", below 60 "medium", else "strong".
- **`hud/rating_bars.gd`**: draws the ratings. `STRIP` (tiles, unit
  panel): a symbol per rating with a bar under it; `FULL` (tooltip, card):
  rows with symbol, name, bar and number. One sand tone and a symbol per
  rating, so the display does not depend on the color palette; a rating the
  type lacks is left out, a rating of 0 dims its symbol.
- **`hud/unit_card.gd`**: the unit card, a panel with head (picture or 3D
  model, name, designation, category, faction, service years, price,
  ratings, the rank's bonuses for an own unit) and two tabs: "Game values"
  (every section of the card in the layout of `web/units/stats.js`) and
  "Technical data" (the entry of the vehicle database with its source; a
  note where there is none). Base buildings are no catalog entries and
  show the values of `catalog_card` (which also carries `cost`, `build_s`
  and, for catalog entries, `year_to` since this prompt).
- **In a match** `hud.gd` owns one card: `show_unit_card(type_id, unit)`
  from a tile's right click (`picker.gd` `info_requested`), the unit
  panel's button and the input action `unit_card` (Shift + I, a run-time
  action of `settings.gd` like the air calls; plain I is `order_secure` and
  every letter is taken). It sits top right: right of the command bar,
  above the minimap, above the unit panel when that sits over the minimap
  and enough room is left; on a small window it gets narrower and its pages
  scroll. It takes no keyboard focus (order keys reach `selection.gd`),
  stops only the mouse clicks on itself (`covers`), and `hud.gd` closes it
  on Escape in `_input`, before the pause menu sees the key. Nothing is
  paused and no command is involved: lockstep matches are unaffected.
- **In the main menu** `menu/units_screen.gd` ("Units") lists
  `SimHost.catalog_ids()` (base buildings, then every catalog entry that is
  no environment object) by faction ("General": base buildings and the
  common field fortifications) and kind; entries in no build or production
  menu (`buildable_ids`) follow under their own heading. It shows the same
  card with `with_model`: `menu/unit_model_view.gd`, a `SubViewport` with
  the model of `tools/thumbnail_render.gd` `build_model(id, biome)` (the
  one the thumbnails and the model gallery's looks use: `UnitCamo`,
  `SoldierLook`, `EnvLook`), turned by dragging, zoomed by the wheel, with
  a camouflage choice (forest, desert, winter).
- **Texts** are in `locale/units.csv` (`UNIT_`): ratings, groups, labels,
  value patterns, table heads, notes, and the names built from codes:
  `UNIT_CAT_<CATEGORY>`, `UNIT_FIELD_<DATABASE FIELD>`,
  `UNIT_PART_<ASSEMBLY>`, `UNIT_WEAPON_<COMPARISON WEAPON>`. Texts of the
  vehicle database's source stay as the source wrote them.
- **Decisions.** Ratings on every tile, not only in the tooltip: as a
  symbol with a bar they stay readable at interface scale 1 (checked on
  screenshots); four signal-like steps next to each symbol were tried
  first and read as noise at that size. Technical data in the match too, as
  the second tab, so the first stays short. Shift + I instead of I (taken).
  The unit panel keeps its size (a wider or higher one covered units the
  self-test clicks, and so a player's), so its ratings are a strip under a
  slightly smaller portrait and the button sits in the portrait's corner. No forecast at
  the cursor while aiming (see GAME_DESIGN.md).
- **Checks.** `tools/hud_self_test.gd`: tooltip and card for every
  buildable id in English and German without a missing text, sample
  numbers against `SimHost.unit_stats` (the function behind
  `cli unit-stats`), one fetch per type, the ratings on every tile, right
  click, button, key, Escape, orders while the card is open, its place on
  the screen. `tools/experience_self_test.gd`: the rank line.
  `tools/units_self_test.gd` (flow test): every entry of the reference.
  `just unit-card-shots` (`tools/unit_card_shots.gd`, needs a window):
  screenshots of reference, picker, tooltip, card and unit panel in German
  and English at interface scale 1 and 1.25 and in German at 1280 × 720,
  into `build/unit-cards/`.

The self-test (`tools/hud_self_test.gd`) clicks and drags the minimap,
changes the selection through the unit panel, drives a waypoint chain by
Shift + right click, gives orders through the command bar and jumps to an
event. It sets the headless window to 1600 × 900: at the headless default of
64 × 64 pixels the HUD would cover the whole screen.

## Replays

The game writes the match to `user://replays/<date-time>.msreplay` when the main scene
exits (macOS: `~/Library/Application Support/Godot/app_userdata/Mud and Steel/replays/`);
watching a replay writes nothing. `just replay <file>` re-simulates it headless and
reports whether the end hash matches. `just record <file>` writes a replay of the
scripted demo without Godot.

Format (`crates/net/src/replay.rs`):

```
"MSRP"  4 bytes magic
u16 LE  format version (currently 6)
postcard body: info (game version, rules hash, date, map, tick rate, ticks,
                     players with faction and kind, result, multiplayer),
               seed, resolved SimConfig, lockstep players, tick count,
               frames (tick + merged PlayerCommands, only non-empty ticks), end hash
```

Decisions:

- **Header outside postcard.** postcard is not self-describing, so a reader must know
  the version before decoding. A fixed magic and version let old builds reject new
  files with a clear message instead of garbage.
- **Info first (prompt 038).** `ReplayInfo` is the first field of the body, so the
  replay list reads it with `Replay::read_info` without decoding the commands. It
  carries the game version (for the player) and the rules hash (for the check):
  `Replay::decode` rejects a replay whose rules hash differs from this build's
  (`ReplayError::RulesChanged`) instead of playing back a different game. Version 6
  (prompt 038) added the info; older replays are rejected as "from an older game
  version".
- **Resolved config instead of a scenario name.** The replay carries the unit types as
  the simulation used them (already converted from the catalog). Terrain rules are
  part of the config, too. Note that the rules hash also changes with the catalog
  (see "Saved games and replay viewer").
- **Map by reference.** The map (20 MB for 4 km) is not copied into the replay; the
  config names it with its content hash. `just replay` loads `data/maps/<name>/` (or
  the built-in `test_small`) and refuses a map whose hash differs. Version 2 (prompt
  010) added map reference and terrain rules, version 3 (prompt 017) combat rules,
  combat data of unit types, the start fire mode of spawns and the combat commands,
  version 4 (prompt 014) the unit mass (`UnitType::mass_kg`, bridge load limits).
  Prompt 031 appended `QueueMove` and `Patrol` to `Command` without a version
  change: old files decode as before; an older build fails to decode a file
  that uses them.
- **Merged commands per tick, sparse.** The replay stores exactly what `Sim::step` got
  (player-tagged, in apply order), so playback needs no lockstep session and does not
  depend on input delay. Ticks without commands are omitted; the tick count marks the
  end. An hour of play with a few orders per second stays well below a megabyte.
- **Only the end hash.** It proves the whole run; intermediate hashes to locate a
  divergence can be added with a version bump.

## Saved games and replay viewer

Prompt 038. Single-player matches can be saved and continued; every replay can be
watched in the game.

### Snapshots

A snapshot is the complete state of a `Sim` (`crates/sim/src/snapshot.rs`, encoding in
`crates/net/src/snapshot.rs`):

```
"MSSN"  4 bytes magic
u16 LE  format version (currently 1)
u64 LE  Sim::state_hash at the snapshot
deflate (miniz_oxide, level 1) of postcard(SimSnapshot { map: MapDelta, sim: Sim })
```

- `Sim` and every type it holds derive `Serialize` and `Deserialize`. **New state is
  included automatically**: a field whose type does not derive them does not compile.
  Derived data (path grid, vision rasters, object index, unit grid) is saved, not
  rebuilt: rebuilding would have to reproduce every incremental change exactly (local
  path relabeling after craters, vision budgets).
- The map is skipped (`#[serde(skip)]`) and saved as `MapDelta`: the map reference,
  the height and terrain samples that differ from the loaded map (craters) and the
  intact flag of every bridge. `Sim::restore` applies it to the loaded map.
- `net::snapshot::decode` checks the stored state hash against the restored
  simulation. Tests (`crates/net/tests/snapshot.rs`) restore the demo at several ticks
  (with craters) and step original and copy side by side: every tick has the same hash.
- serde's derive borrows `&'static str` fields; the economy's faction ids use a type
  alias and a `with` module (`sim/economy.rs`, `static_faction`).

### Rules hash

`net::snapshot::rules_hash()` is the hash of the uncompressed snapshot bytes after a
fixed reference run (the demo, seed 7, 400 ticks; ≈ 60 ms once per process). Any rule
change changes the state, any new state field the bytes, so files of a build with other
rules are rejected up front, without a constant to maintain. **[provisional]** The demo
uses catalog units, so catalog edits change it too, and older replays then count as
incompatible although they carry their resolved config. A reference run with a config
built in code would avoid that.

### Saved games

`net::save::SaveGame` (`user://saves/<date-time>.mssave`):

```
"MSSV"  4 bytes magic
u16 LE  format version (currently 1)
postcard body: info (as in replays; ticks = save tick), the replay from tick 0 to the
               save tick, snapshot bytes, pending lockstep frames, input delay,
               computer players' state (postcard of ai::Ai), settings (biome)
```

- **Loading restores the snapshot, it does not re-simulate.** Measured with
  `just save-bench` (release, Apple M1 Max): the large battle (`bench_large`, 504 units,
  two hard AIs, 6000 ticks = 5 min) re-simulates in 32.4 s (5.4 ms per tick, an hour
  ≈ 6½ min); its snapshot writes in 162 ms, restores in 113 ms and has 9.1 MB. The base
  building skirmish on `test_small` (10 min): 0.27 s re-simulation, 8 / 5 ms and 452 KB
  for the snapshot.
- **Exact continuation.** Commands already given but not yet simulated (input delay)
  are kept (`LockstepSession::pending_frames` / `resume`), and so is the state of every
  computer player (`ai::Ai::save_state`). A loaded game plays exactly like the match
  that was never saved (`crates/ai/tests/save.rs`, battle and base building).
- **The replay keeps growing.** The saved replay continues (`ReplayRecorder::resume`),
  so the replay written at the end covers the whole match from tick 0.
  `SaveGame::verify` / `just save-check <file>` re-simulates it and compares it with
  the snapshot.
- Only single player can be saved (also against computer players): the pause menu
  offers "Save game" when `SimHost.can_save()`. The main menu's "Load game" lists the
  saves (date, map, players, time) and hands the chosen one to `main.tscn` through
  `MatchSetup.pending` (`menu/game_files.gd`).

### Replay viewer

Main menu → "Replays" lists `user://replays/` (date, map, players with faction and
human/computer, duration, result); files of an older format or other rules are greyed
out with the reason. "Watch" opens the replay in a `ReplayPlayer` (bridge,
`replay_funcs.rs`) and starts `main.tscn` with `SimHost.start_replay(player, tick,
view)`: `Game::step` then takes the commands from the replay instead of the lockstep
session, the player's commands are dropped, nothing is saved or recorded.

- **Controls** (`replay/replay_bar.gd`): play/pause, 1×/2×/4×/8× (`SimHost.speed`; up
  to 20 ticks per frame while a replay plays), a timeline and the perspective.
- **Snapshots for jumps.** While a replay plays (or is fast-forwarded), a snapshot is
  kept in memory every 600 ticks (30 s). A jump rebuilds `main.tscn` from the nearest
  earlier snapshot and plays the rest headless, because terrain, craters and object
  states are built from the simulation when the scene starts. The open replay and its
  snapshots live in a static of `game_files.gd`, so they survive the rebuild.
- **Perspective.** One player: the presentation's local player (that player's fog of
  war, the others tinted as enemies). "Everything": `reveal_all`. A change rebuilds the
  scene at the current tick like a jump.
- **Multiplayer.** No saving; the replay is written as before (with its info) and can
  be watched like any other.

## Camera rig (presentation only)

`godot/scenes/camera/rts_camera.tscn` is a `Node3D` that sits on a focus point on the
ground, with a perspective `Camera3D` (30° field of view) as its child. The camera is
placed from four values: focus, distance (15–600 m), pitch and yaw.

- **Pitch follows the zoom:** 45° above the horizon at the closest distance, 65° at the
  farthest, blended over a logarithmic zoom level (optional `Curve`, else smoothstep).
- **Zoom** (mouse wheel, trackpad pinch) is smoothed in log space and keeps the ground
  point under the cursor fixed. **Pan** (WASD, arrows, window edge) scales with the
  distance; middle-mouse drag, **Option/Alt + mouse move** and the **trackpad
  two-finger pan** (`InputEventPanGesture`, which macOS sends for trackpad scrolling;
  the map follows the fingers, honoring the system's scroll direction) grab the
  ground. **Q/E** turn in 45° steps, **N** turns back to north (−Z).
- `center_on(point)` glides the focus to a point (double click on the ground, follow
  mode); every hand pan cancels the glide and emits `panned_by_hand`.
- The focus is clamped to `bounds`, set by `main.gd` from `SimHost.map_size()`, and
  sits on the ground height (`GroundPick.height_at`).
- Near/far planes and the sun's shadow distance scale with the camera distance
  (`atmosphere.gd` sets the shadow factor to 1.8; see "Lighting, atmosphere and biomes").
- All bindings are `camera_*` actions in the input map in `project.godot`.
- `ground_pick.gd` holds the screen-to-ground math used by the camera and by mouse
  commands. Once `main.gd` sets `GroundPick.height_source` to the SimHost, rays hit
  the simulation's height field (`SimHost.ray_pick`); without it, the plane y = 0.

Perspective was chosen over orthographic: depth and height read better, the view gets
the slightly perspective look of the style references, and zoom works by moving the
camera, which makes pitch changes and distance-based shadows and LOD natural. The small
field of view keeps distortion low.

## Trailer recordings (presentation only)

`just trailer-shots [--only=<shot>,…] [--stills]` records the shots of the demo video
(prompt 070) as `build/trailer/<shot>.avi`: 1920 × 1080, 60 frames per second, picture
and sound, written by Godot's Movie Maker (`--write-movie`, `--fixed-fps 60`; the
`movie` overrides in `project.godot` set the size).

- **Shots** are a table in `godot/scenes/tools/trailer_shots.gd`: map, scene, seed,
  start time, length, weather and time of day, HUD on or off, and camera keys (look-at
  point or unit, distance, yaw, pitch, field of view), interpolated with Catmull-Rom.
  `main.gd` starts the tool with `-- --trailer=<shot>`.
- **Scenes** are mission scripts in `data/trailer/<scene>.toml` ([MISSIONS.md](MISSIONS.md)):
  groups and actions that issue normal commands (`order`, `fire`, and since prompt
  082 `barrage`, `air` and `camouflage`), nothing else. They are no missions of the menu;
  `SimHost.start_mission_on(seed, script, map_dir)` runs one on a map it does not lie
  next to. The simulation is untouched: who hits whom is decided by the rules, the
  shot only picks seed, moment and camera. Several shots show one scene from other
  angles (same script, seed, weather).
- **Run-up:** the tool runs the simulation at high speed and muted to two seconds
  before the shot's start, then at normal speed. Movie Maker writes from the first
  frame, so the tool prints the first frame of the shot and `tools/trailer-shots.sh`
  cuts the run-up off with ffmpeg (stream copy, exact to the frame). Effects live in
  real time, so what starts in the fast part stays visible too long; a shot after
  long-lived effects (rocket trails, explosion smoke) sets `pre_roll_s` above their
  life (prompt 082).
- **Camera:** the rig stands still (no input, no edge scrolling); the tool places the
  `Camera3D` itself, lower and flatter than the game allows, and hands focus and
  distance to the rig, which haze, depth of field and sound follow. Without `hud` the
  interface is off: HUD layer, health bars, fog of war, relation tint, supply point
  markers, the rings where own shells will land; pause menu and result screen are
  always off.
- **Sound:** fixed bus volumes, set every frame (the game applies the settings again and mutes when its window changes focus), with 14 dB headroom (layered explosions clip otherwise; the edit sets the loudness), no music (a cut would chop it).
- **Finding a shot:** `-- --trailer=<shot> --trailer-scout=<seconds>` (headless, with
  `--fixed-fps 60`) plays the shot's scene without a camera and prints every destroyed
  unit with time and place, shots and hits per five seconds and where the groups
  stand; the `survey_<map>` shots show a map from above. Since prompt 082 it also
  prints every gun shot, every hit on a unit (penetrated or bounced, damage), where
  shells, rockets and bombs come down, blown-off turrets, and the hit points left of
  every unit. `tools/trailer-scout.sh <shot> <seconds> [seed …]` runs it; with seeds
  (`--trailer-seed=<n>` replaces the shot's) it plays the scene once per seed and
  prints one line each with the destroyed vehicles in order and who is left.
- **Scene `ambush` (prompt 082):** `data/trailer/ambush.toml` on the map
  `ambush_meadow` ([MAPS.md](MAPS.md)), seed 14: a Soviet column in tall grass runs
  into two camouflaged Tigers at a wood's edge, fires a rocket salvo and is bombed.
  Seventeen shots `ambush_*`, all with the player's camera, in the order of the edit;
  the script's header lists the times. `just trailer-shots --only=ambush_…` records
  them, `just trailer-cut ambush` builds `build/trailer/ambush_final.mp4` (76.1 s).
  The meadow burns at the end through the grass fire (see "Grass fire").
- **Edit (prompt 082):** `just trailer-cut <prefix>` (`tools/trailer-cut.sh`) joins
  the recorded shots `<prefix>_*` in the order of the shot table into
  `build/trailer/<prefix>_final.mp4`: hard cuts, every shot keeps its frames; the
  sound is faded out and in over 1.5 frames each at every cut (nothing overlaps, so
  picture and sound stay in step), then set to −14 LUFS with two passes of ffmpeg's
  `loudnorm` (true peak ≤ −1.5 dB); H.264 `yuv420p` in video range (the recordings
  are full-range MJPEG), AAC, 60 fps.
- **The player's camera (prompt 082):** a shot with `"game_camera": true` whose keys
  give only focus (`at` or `unit`), `dist` and `yaw` is placed by the camera rig
  itself (`rts_camera.gd` `snap_to`), as in a match: pitch from the zoom level (45°
  near to 65° far), distance within 15–600 m, the game's field of view, clearance
  over ground and roofs, planes and shadow range. The keys are interpolated as
  before, so the view pans, turns and zooms smoothly. Shots that set `pitch`,
  `lift`, `fov` or `haze` (the measuring pans of prompts 068–080) keep the free
  placement.
- **Recording the same again (prompt 082):** the tool seeds Godot's random generator
  with the shot's seed (where a turret lands), and `effect_library.gd` gives every
  particle system a fixed seed when the game runs with `--trailer=` (random as ever
  otherwise). Simulation, camera and timing are the same in every take; pictures were
  identical in a march shot (270 of 270 frames) and differed in 2 of 156 frames of an
  explosion shot (PSNR 81 dB on average). The sound is not bit-identical.

## Data flow (multiplayer)

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

The input delay is per peer: every peer starts with the same delay (frames for the
first ticks are implicitly empty), then follows its measured round trip
(`net::recommended_input_delay`; raised at once, lowered after three checks in a
row). A larger delay submits extra, usually empty frames; a smaller one
pauses local scheduling until the simulation caught up. Frames carry their tick, so
this needs no agreement between peers. `LockstepSession::verified_tick` is the last
tick whose hash every peer confirmed.

A lost connection or a desync halts the match in `SimHost` (the last state stays
visible, the replay can still be saved); a desync also saves the replay at once.

## Transport

The `Transport` trait only sends and receives bytes, so the protocol does not know how
packets travel. It must deliver reliably and in order per sender; it may report the
round trip (`round_trip_ms`) and is closed when a match is dropped.

- **LAN:** `net::lan::LanNode`, ENet over UDP with the pure-Rust port
  [`rusty_enet`](https://crates.io/crates/rusty_enet). The host is the ENet server and
  relays the lockstep packets of every client to the others (star). Games are found
  with a UDP broadcast query. Lobby, protocol, ports and how to test with two
  machines: [LAN.md](LAN.md).
- **Online:** `steam::SteamNode`, the same lobby node over Steam Networking
  Messages (relayed by Steam, NAT traversal included). The Steam lobby finds the
  game and invites friends; its owner is the host. Choices, protocol, build and
  how to test with two accounts: [ONLINE.md](ONLINE.md).

The lobby protocol (`net::lobby::LobbyNode`: hello, welcome, ready, start,
relay through the host) is independent of the network; LAN and Steam only
provide a `net::lobby::Link` (connect and disconnect events, packets on two
channels, round trip).
- **Online (prompt 026):** Steam networking through
  [GodotSteam](https://godotsteam.com/) (GDExtension). Lobbies, matchmaking and NAT
  traversal come from Steam; a `SteamTransport` in the bridge implements the same trait
  and reuses the start message of the LAN lobby. Requires a Steam App ID.

## Asset pipeline

Folders, scale, orientation, part names, budgets and material slots:
[ASSETS.md](ASSETS.md).
