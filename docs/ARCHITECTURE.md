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
| `crates/sim`    | Game rules and state. `Sim::new(config, seed)`, `step(&[PlayerCommand])`, `state_hash()`, `snapshot()`. Players and unit ownership, group commands with formation, fixed-point math, RNG, stable hash, unit catalog, scale and scenarios (compiled in). | `fixed`, `serde`, `toml` |
| `crates/net`    | Lockstep protocol: `CommandFrame`, `HashReport`, `LockstepSession` (input delay, desync detection), `Transport` trait, `LoopbackTransport` for tests, `LocalTransport` for single player. Replays (`net::replay`). | `sim`, `postcard` |
| `crates/cli`    | Headless runner: `cli run --ticks N --seed S [--record FILE]` prints the state hash, `cli replay FILE` re-simulates a replay and checks its end hash. | `sim`, `net`, `clap` |
| `crates/bridge` | GDExtension `cdylib`. `SimHost` node: runs ticks from a time accumulator through a `LockstepSession`, queues commands, exposes the last two snapshots and the replay. The only float conversion. | `sim`, `net`, `godot` |
| `godot/`        | Scenes, GDScript UI logic, assets. No C#. | bridge (via `.gdextension`) |
| `data/`         | Unit catalog (`catalog/*.toml`, real values), `scale.toml` (conversion to game values) and scenarios (`scenarios/*.toml`, start setups), integers only. See [SCALE.md](SCALE.md). | – |

## Players, commands and formation

Every unit has an owner (`sim::PlayerId`). A `sim::Command` names a list of units
but never its issuer:

| Command | Effect |
|---|---|
| `MoveTo { units, target }` | The units drive in a straight line to a grid formation around `target`. |
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
command. Pathfinding around obstacles and facing-aware formations follow with
prompt 012.

**Scenarios** (`data/scenarios/*.toml`) place groups of catalog units per player in a
grid. The compiled-in demo (`sim::scenario::demo_config`) has eight Panzer IV H
(player 0) against eight T-34/76 (player 1), 120 m apart. Unit ids follow file order.

## Data flow (single player, current state)

Single player runs through the same lockstep path as multiplayer:

1. The player selects units and right-clicks the ground. `selection/selection.gd`
   turns the click into a ground point with `scenes/camera/ground_pick.gd` (pure
   math, no Godot physics) and calls `SimHost.move_units(ids, x, z)`.
2. `SimHost` rounds the target to millimeters and queues `Command::MoveTo`.
3. Every frame, `SimHost::process` adds the frame time to an accumulator and runs as many
   fixed 20 Hz ticks as are due (at most 5 per frame). Per tick it submits the queued
   commands to its `LockstepSession` (one player, `LocalTransport`), which schedules
   them `input_delay` ticks ahead (exported property, default 1 tick = 50 ms). The
   session returns the merged `PlayerCommand`s of the current tick; they are recorded
   for the replay and stepped.
4. After each tick, the previous snapshot is kept and a new one is taken.
5. `main.gd` reads both snapshots and `interpolation_alpha()` and draws every unit
   interpolated between them. The sim `(x, y)` plane maps to Godot `(x, 0, y)`.

Player 1 of the demo issues no commands yet; the AI (prompt 024) will join the session
as another player with its own frames.

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
u16 LE  format version (currently 1)
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
- The focus is clamped to `bounds`, set by `main.gd` from `MAP_SIZE` (test ground,
  2 km × 2 km; prompt 010 replaces it with the map size).
- Near/far planes and the sun's shadow distance scale with the camera distance.
- All bindings are `camera_*` actions in the input map in `project.godot`.
- `ground_pick.gd` holds the screen-to-ground math used by the camera and by mouse
  commands. Today it intersects the plane y = 0; prompt 011 swaps in the height field.

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
