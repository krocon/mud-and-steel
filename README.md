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

- Rust (stable, installed via rustup; `rust-toolchain.toml` selects it)
- Godot 4.7.x (`godot` on `PATH`)
- Git LFS (`git lfs install` once per machine)
- Blender 5.x (only for regenerating assets)
- [just](https://github.com/casey/just) as command runner
  (`brew install just`, `winget install Casey.Just`, or `cargo install just`)

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
| `just export-model <file.blend>` | exports a model to `godot/assets/models/`       |
| `just model-tank`       | regenerates and exports the placeholder tank             |

Tool paths can be overridden with `GODOT=...` and `BLENDER=...`.

## First start

```bash
just import
just run
```

You command eight Panzer IV (player 0) facing eight T-34 (player 1, red tint, not
selectable). Left click or drag a box to select, double click selects all visible tanks
of the type, Ctrl+1…9 stores a group and 1…9 recalls it (twice centers the camera).
Right-click on the ground sends the selection into a formation there, H stops it.
The camera moves with WASD/arrows, the mouse wheel, middle drag, Q/E and N.

On exit the game writes a replay to `user://replays/` (macOS:
`~/Library/Application Support/Godot/app_userdata/Mud and Steel/replays/`);
`just replay <file>` re-simulates it and checks the end hash.

`just sim` prints the final state hash of the scripted demo. Two runs with the same
arguments print the same hash on every platform. The underlying commands are in the
[justfile](justfile).
