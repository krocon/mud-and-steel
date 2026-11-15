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

## Scale, catalog and assets

Sizes are 1:1 in real meters; time runs at factor 2. The catalog holds real values
only, converted solely via `data/scale.toml` (`docs/SCALE.md`). Every catalog value
needs a source (`docs/SOURCES.md`: no data from other games, no swastika/SS runes, no
Red Cross). Model conventions and budgets: `docs/ASSETS.md`.

## Commands

All commands live in the `justfile` (`just` lists them). Prefer them over raw commands.

```bash
just check                 # fmt --check, clippy -D warnings, tests (same as CI)
just sim 1000 42           # headless run, prints state hash
just build                 # GDExtension for Godot
just import                # headless Godot import (after clone / new assets)
just run                   # build + start the game in the main menu (--skip-menu: test match)
just selftest              # build + headless test: menu, settings, matches, result
just lan / just lan-selftest  # LAN lobby / two headless instances over localhost
just online                # online lobby over Steam (optional; docs/ONLINE.md)
just map-editor / just editor-selftest  # map editor / its headless test
just map-check <name>      # validate data/maps/<name>/ like the editor
just ai-tournament         # AI against AI over many seeds, statistics (docs/AI.md)
just bench / just bench-view  # overall benchmark: simulation / frame rate (docs/PERFORMANCE.md)
just catalog-doc           # regenerate docs/UNIT_CATALOG.md after catalog changes
just export / just export-selftest  # exported .pck and its self-test; release builds: docs/RELEASE.md
just export-model assets-src/blender/X.blend
just model-tank            # regenerate placeholder tank
```

## Layout

```
crates/sim/      deterministic game rules (Fixed, Vec2, Rng, StableHasher, Sim, scenario)
crates/net/      lockstep protocol, Transport trait, lobby protocol, LAN (ENet)
crates/steam/    online over Steam (steamworks-rs), optional; app id in one place
crates/cli/      headless runner
crates/bridge/   godot-rust GDExtension (SimHost node, MapEditor node)
crates/ai/       computer players: fog-of-war PlayerView, difficulties, headless AI games
crates/editor/   map editor core: brushes, undo, saving, validation (no Godot)
godot/           Godot project: scenes/, ui/, assets/, mud_and_steel.gdextension
assets-src/blender/  .blend sources (Git LFS)
tools/           Blender scripts, Godot import helper
data/            unit catalog (catalog/*.toml, real values) and scale.toml
docs/            ARCHITECTURE.md, DETERMINISM.md, VERSIONS.md, SCALE.md, ASSETS.md,
                 SOURCES.md, INFANTRY.md, LAN.md, ONLINE.md, AI.md, GAME_DESIGN.md,
                 PERFORMANCE.md, RELEASE.md,
                 UNIT_CATALOG.md (generated)
prompts/         task prompts (roadmap in prompts/README.md); finished ones: git mv to prompts/done/
```
