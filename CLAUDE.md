# CLAUDE.md

Isometric WW2 real-time strategy game. Rust simulation + Godot 4 presentation,
deterministic lockstep multiplayer. Solo developer: simplicity beats elegance.

Talk to the user in German. Code, comments, commit messages and docs are in English.

## Non-negotiable rules

1. The simulation (`crates/sim`) is the single source of truth: pure Rust, no Godot,
   fixed 20 Hz tick.
2. Godot only presents. Never use Godot physics, `NavigationServer` or Godot randomness
   for anything game-relevant.
3. No floats in `crates/sim`: use `sim::Fixed` / `sim::Vec2` and `sim::Rng`. Floats are
   converted only in `crates/bridge`.
4. No hidden nondeterminism in `sim`: no clocks, no `HashMap`/`HashSet` iteration, no
   threads. Lints enforce most of this; see `docs/DETERMINISM.md`.
5. AI is a normal player and only issues `Command`s.

Never weaken or disable a test to make it pass. If the golden hash in
`crates/sim/tests/determinism.rs` changes, find out why; only update it for an
intentional rule change and say so in the commit message.

Ask before installing software, deleting things, or hard-to-reverse decisions.
Never `git push` without explicit approval. Godot and godot-rust are only updated
together (`docs/VERSIONS.md`). Look up godot-rust APIs for the pinned version (0.5.5)
instead of guessing, e.g. in `~/.cargo/registry/src/*/godot-core-0.5.5/`.

## Commands

All commands live in the `justfile` (`just` lists them). Prefer them over raw commands.

```bash
just check                 # fmt --check, clippy -D warnings, tests (same as CI)
just sim 1000 42           # headless run, prints state hash
just build                 # GDExtension for Godot
just import                # headless Godot import (after clone / new assets)
just run                   # build + start the game (opens a window)
just selftest              # build + headless smoke test of the game
just export-model assets-src/blender/X.blend
just model-tank            # regenerate placeholder tank
```

## Layout

```
crates/sim/      deterministic game rules (Fixed, Vec2, Rng, StableHasher, Sim, scenario)
crates/net/      lockstep protocol, Transport trait, LoopbackTransport
crates/cli/      headless runner
crates/bridge/   godot-rust GDExtension (SimHost node)
godot/           Godot project: scenes/, ui/, assets/, mud_and_steel.gdextension
assets-src/blender/  .blend sources (Git LFS)
tools/           Blender scripts, Godot import helper
data/            unit definitions (TOML, integer units)
docs/            ARCHITECTURE.md, DETERMINISM.md, VERSIONS.md
prompts/         task prompts from the user; do not modify
```
