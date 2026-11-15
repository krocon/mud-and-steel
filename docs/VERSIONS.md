# Pinned versions

Determined on 2026-11-15 from crates.io, the godot-rust book and the GitHub/Codeberg
releases.

| Component     | Version              | Pinned in                                  | Notes |
|---------------|----------------------|--------------------------------------------|-------|
| Rust          | 1.100.0              | `rust-toolchain.toml`                      | Exact pin with rustfmt and clippy, so local builds and CI lint the same way. MSRV 1.94 (`rust-version`, required by godot-rust 0.5). |
| Godot         | 4.7.2-stable         | `godot/mud_and_steel.gdextension` (`compatibility_minimum = 4.7`), `project.godot` | Latest stable Godot supported by godot-rust 0.5.5. |
| godot-rust    | `godot = "=0.5.5"`   | `Cargo.toml` (workspace dependencies)      | Feature `api-4-7`. Exact pin. |
| Blender       | 5.3.0                | –                                          | Used for `tools/*.py`; export via the bundled glTF exporter. |
| GodotSteam    | 4.24 (GDExtension)   | – (not integrated yet)                     | GDExtension build supports Godot 4.4+, Steamworks SDK 1.65. |
| Git LFS       | 3.8.1                | `.gitattributes`                           | |

Other crates (`fixed`, `serde`, `postcard`, `toml`, `clap`, `rusty_enet`, `if-addrs`) are
pinned by `Cargo.lock`. `rusty_enet` 0.4 is ENet 1.3.18 transpiled to Rust (no C
toolchain needed); see [LAN.md](LAN.md) for why it was chosen over Godot's ENet.

## Update policy

- **Godot and godot-rust are only updated together.** godot-rust's `api-4-x` feature
  must match the Godot minor version, and `compatibility_minimum` in the
  `.gdextension` file must be raised with it.
- After a Rust update, run `cargo test --workspace` and check that the golden-hash test
  still passes. A failure means something became non-deterministic.
- Blender updates can change exported files; re-export and check the result in Godot.

## Known issues

- Godot 4.7.2 crashes at exit when a fresh project (no `.godot/`) discovers a
  class-registering GDExtension during the first headless import
  ([godotengine/godot#111645](https://github.com/godotengine/godot/issues/111645),
  fix pending in [godotengine/godot#123658](https://github.com/godotengine/godot/pull/123658)).
  `tools/godot-import.sh` works around it by seeding `.godot/extension_list.cfg`.
