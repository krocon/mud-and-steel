# Mud and Steel

Working title. An isometric, old-school real-time strategy game set in World War II
(inspired by Sudden Strike, Blitzkrieg, Red Alert 2 and Beyond All Reason).

Platforms: macOS, Linux and Windows. Multiplayer (online/LAN, PvP/co-op, cross-platform)
uses deterministic lockstep: every machine runs the same simulation and only commands
are exchanged.

## Stack

| Layer        | Technology                                               |
|--------------|----------------------------------------------------------|
| Simulation   | Pure Rust (`crates/sim`), fixed-point, fixed 20 Hz tick |
| Netcode      | Rust (`crates/net`), transport-agnostic lockstep         |
| Headless run | Rust (`crates/cli`)                                      |
| Bridge       | godot-rust GDExtension (`crates/bridge`)                 |
| Presentation | Godot 4 + GDScript (`godot/`)                            |
| Assets       | Blender → glTF (`assets-src/blender/`, `tools/`)         |

Pinned versions are listed in [docs/VERSIONS.md](docs/VERSIONS.md).
Architecture and determinism rules: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md),
[docs/DETERMINISM.md](docs/DETERMINISM.md).

## Prerequisites

- Rust 1.100.0 (installed via rustup; `rust-toolchain.toml` pins it)
- Godot 4.7.x (`godot` on `PATH`)
- Git LFS (`git lfs install` once per machine)
- Blender 5.x (only for regenerating assets)
- [just](https://github.com/casey/just) as command runner
  (`brew install just`, `winget install Casey.Just`, or `cargo install just`)
- [cargo-nextest](https://nexte.st) as test runner for `just check` and `just test`
  (`brew install cargo-nextest`, or `cargo install cargo-nextest --locked`)

## Commands

Run `just` to list all commands. The most important ones:

| Command                 | What it does                                             |
|-------------------------|----------------------------------------------------------|
| `just check`            | `cargo fmt --check`, clippy and all tests, like CI       |
| `just sim [ticks] [seed]` | headless run, prints the state hash (default 1000, 42) |
| `just import`           | headless Godot import, once after cloning / new assets   |
| `just run`              | builds the bridge and starts the game                    |
| `just editor`           | builds the bridge and opens the Godot editor             |
| `just selftest`         | headless smoke test of the game (exit code 0 = passed)   |
| `just replay <file>`    | plays a replay headless and checks its end hash          |
| `just record <file>`    | writes a replay of the scripted demo (no Godot needed)   |
| `just run-map [name]`   | starts the game on `data/maps/<name>/` (default `test_large`) |
| `just map-info [name]`  | load time, memory and height range of a map              |
| `just gen-map <name> [size_m] [seed]` | generates a test map ([docs/MAPS.md](docs/MAPS.md)) |
| `just path-bench [map] [units] [group]` | pathfinding benchmark, time per tick (default `test_large`, 200, 1) |
| `just run-paths [map]`  | starts the game with the path debug view (F4 costs, F5 paths) |
| `just objects-bench [map]` | frame rate and draw calls over woods and a village (opens a window) |
| `just objects-shots <dir> [map]` | records views of woods, village and hedgerows as PNG frames |
| `just export-model <file.blend>` | exports a model to `godot/assets/models/`       |
| `just model-tank`       | regenerates and exports the placeholder tank             |

Tool paths can be overridden with `GODOT=...` and `BLENDER=...`.

## First start

```bash
just import
just run
```

You command eight Panzer IV (player 0) facing eight T-34 (player 1, red tint, not
selectable) on the 500 m test map `test_small`: a start meadow, a hill to the north-west,
a river in the east and a village square in the south-west, drawn as a debug mesh
colored by terrain type with height contours. Left click or drag a box to select, double click selects all visible tanks
of the type, Ctrl+1…9 stores a group and 1…9 recalls it (twice centers the camera).
Right-click on the ground sends the selection into a formation there, H stops it.
Moving the map: two fingers on the trackpad, Option (Alt) + mouse move, middle drag,
WASD/arrows or the window edge; double click on empty ground centers there, C makes
the camera follow the selection. Zoom with the mouse wheel or pinch, turn with Q/E,
N turns back to north.

On exit the game writes a replay to `user://replays/` (macOS:
`~/Library/Application Support/Godot/app_userdata/Mud and Steel/replays/`);
`just replay <file>` re-simulates it and checks the end hash.

`just sim` prints the final state hash of the scripted demo. Two runs with the same
arguments print the same hash on every platform. The underlying commands are in the
[justfile](justfile).
