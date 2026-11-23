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

After every change to a model (units, soldiers, buildings, environment objects),
re-render its thumbnails in `godot/assets/thumbnails/<id>.png`:
`just thumbnails --only=<id>,<id>` (needs a window; without `--only` it renders only the
buildables). Every catalog entry with a model needs one, also new and non-buildable
ones (the unit page shows them). A changed soldier affects every squad and crew
picture that shows him.

The website serves `web/units/` as its root and needs copies of everything the unit
page loads. After changes to the catalog, models, palettes or thumbnails run
`just units-web-sync`; it also names catalog entries that still lack a thumbnail.

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
just building-preview <id> --ref=<picture>  # build + preview sheet of a building beside a reference picture (docs/ASSETS.md)
just map-check <name>      # validate data/maps/<name>/ like the editor
just map-reach <name>      # land open to vehicles / infantry, connected parts, start-to-supply paths
just map-sight <name>      # sight from supply points and gun positions (height_* markers), viewshed of the sim
just stuck-audit           # groups drive across every map (bridges, starts, supply points): stuck units per map
just battle valley_fields/all_units  # battle with attack waves (data/battles/, docs/MISSIONS.md)
just gen-geo harbor_city   # map from OpenStreetMap + elevation model (also brick_quarter, coastal_heights; tools/geo_import.py, docs/MAPS.md)
just gen-city              # regenerate the large city map data/maps/city_large (docs/MAPS.md)
just gen-sketch [name]     # map from its picture sketch data/map_sketches/<name>.toml (docs/MAPS.md)
just ai-tournament         # AI against AI over many seeds, statistics (docs/AI.md)
just bench / just bench-view  # overall benchmark: simulation / frame rate (docs/PERFORMANCE.md)
just catalog-doc           # regenerate docs/UNIT_CATALOG.md after catalog changes
just vehicle-db / just vehicle-db-check  # write data/vehicles/*.json from the reference folder / check it against the catalog
just units-web             # unit browser in the web browser: pictures, 3D models, values (web/units/)
just sounds-web            # every sound playable in the browser, with when the game plays it (web/sounds/, local only)
just trailer-shots         # record the demo video's shots to build/trailer/ (--only=<shot>, --stills; docs/ARCHITECTURE.md)
just trailer-scout / just trailer-cut  # scout a trailer scene over seeds / cut recorded shots `<prefix>_*` into <prefix>_final.mp4
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
web/units/       unit browser (static page; reads data/catalog and godot/assets directly)
web/sounds/      sound page (local only; remarks in usage.toml, checked by just check)
assets-src/blender/  .blend sources (Git LFS)
tools/           Blender scripts, Godot import helper
data/            unit catalog (catalog/*.toml, real values) and scale.toml
data/vehicles/   vehicle database (JSON, generated; reference work, never rule input; SCHEMA.md)
docs/            ARCHITECTURE.md, DETERMINISM.md, VERSIONS.md, SCALE.md, ASSETS.md,
                 SOURCES.md, INFANTRY.md, LAN.md, ONLINE.md, AI.md, GAME_DESIGN.md,
                 PERFORMANCE.md, RELEASE.md, CONTROLS.md (player controls), TEAMS.md,
                 AUDIO_LIBRARY.md (free sound candidates), MISSIONS.md (mission scripts),
                 UNIT_CATALOG.md (generated), VEHICLE_REFS.md (generated: reference folders and ids)
prompts/         task prompts (roadmap in prompts/README.md); finished ones: git mv to prompts/done/
```
