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
| `crates/sim`    | Game rules and state. `Sim::new(config, seed)`, `step(&[Command])`, `state_hash()`, `snapshot()`. Fixed-point math, RNG, stable hash, unit catalog and scale (compiled in), demo scenario. | `fixed`, `serde`, `toml` |
| `crates/net`    | Lockstep protocol: `CommandFrame`, `HashReport`, `LockstepSession` (input delay, desync detection), `Transport` trait, `LoopbackTransport` for tests. | `sim`, `postcard` |
| `crates/cli`    | Headless runner (`cli run --ticks N --seed S`), prints the state hash. | `sim`, `clap` |
| `crates/bridge` | GDExtension `cdylib`. `SimHost` node: runs ticks from a time accumulator, queues commands, exposes the last two snapshots. The only float conversion. | `sim`, `godot` |
| `godot/`        | Scenes, GDScript UI logic, assets. No C#. | bridge (via `.gdextension`) |
| `data/`         | Unit catalog (`catalog/*.toml`, real values) and `scale.toml` (conversion to game values), integers only. See [SCALE.md](SCALE.md). | – |

## Data flow (single player, current state)

1. The player right-clicks the ground. `main.gd` turns the click into a ground point
   with `scenes/camera/ground_pick.gd` (pure math, no Godot physics) and calls
   `SimHost.move_to(id, x, z)`.
2. `SimHost` rounds the target to millimeters and queues `Command::MoveTo`.
3. Every frame, `SimHost::process` adds the frame time to an accumulator and runs as many
   fixed 20 Hz ticks as are due (at most 5 per frame). The queued commands go into the
   next tick.
4. After each tick, the previous snapshot is kept and a new one is taken.
5. `main.gd` reads both snapshots and `interpolation_alpha()` and draws every unit
   interpolated between them. The sim `(x, y)` plane maps to Godot `(x, 0, y)`.

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

Each peer runs the per-tick protocol of `net::LockstepSession`:

1. Submit local commands for tick `t + input_delay` and send them to all peers (an empty
   frame is also sent).
2. Receive frames of other players.
3. Once the frames of all players for tick `t` are present, merge them in player order,
   step the simulation, and every `hash_interval` ticks send the state hash.
4. A mismatching hash is reported as a `Desync`.

`SimHost` will feed its commands through a `LockstepSession` instead of applying them
directly. AI players produce commands through the same path.

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
