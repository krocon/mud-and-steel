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

## Build and test

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Run headless

```bash
cargo run -p cli -- run --ticks 1000 --seed 42
```

Prints the final state hash. Two runs with the same arguments print the same hash on
every platform.

## Run the game

```bash
cargo build -p bridge
godot --path godot --headless --import   # once after cloning, builds godot/.godot/
godot --path godot                       # starts the main scene
```

Right-click on the ground to send the tank there. `godot --path godot -e` opens the editor.
