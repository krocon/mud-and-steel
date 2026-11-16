# Determinism

Multiplayer uses deterministic lockstep: every peer runs the same simulation and only
commands travel over the network. If two peers ever compute a single different bit,
their games diverge forever (a *desync*). This document lists what `crates/sim` may and
may not do, and why.

## Rules for `crates/sim`

| Forbidden                                        | Use instead                                   | Why |
|--------------------------------------------------|-----------------------------------------------|-----|
| `f32` / `f64` arithmetic                         | `sim::Fixed` (Q32.32) and `sim::Vec2`         | Float results can differ between x86 and ARM, compilers, optimization levels and fused multiply-add. |
| `HashMap`, `HashSet`, iterating them             | `Vec` (sorted), `BTreeMap`, `BTreeSet`        | Iteration order depends on a random seed. |
| `DefaultHasher`, `RandomState`                   | `sim::StableHasher` (FNV-1a)                  | Output is not specified and may change between Rust versions. |
| `rand`, OS entropy, `getrandom`                  | `sim::Rng` (SplitMix64), seeded from config   | Peers must draw identical numbers. |
| `Instant`, `SystemTime`, any clock               | the tick counter (`Sim::tick`)                | Wall-clock time differs per machine. |
| Threads, `rayon`, async with unordered results   | sequential code                               | Scheduling order is not deterministic. |
| Godot types or calls                             | plain Rust data                               | Godot physics, navigation and randomness are not deterministic. |
| Parsing floats from data files                   | integers with explicit units (`speed_mm_per_s`) | Keeps floats out entirely. |
| Image decoders with color/gamma conversion       | `png` with `Transformations::IDENTITY`, raw samples | PNG is lossless, so the raw samples are fully specified; conversions are not. |

Additional rules:

- Commands within one tick are applied in a defined order (player id ascending, then
  issue order). Units are stored sorted by id.
- The issuing player of a command comes only from the lockstep frame
  (`PlayerCommand`), never from the command itself. Commands for units of other
  players are ignored identically on every peer.
- Results must not depend on the order of ids inside a command: unit lists are
  sorted and deduplicated before use, and formation slots are assigned by position
  (see [ARCHITECTURE.md](ARCHITECTURE.md#players-commands-and-formation)).
- Values from the network are not trusted to be in range: move targets beyond
  `sim::WORLD_LIMIT_M` are ignored, so a forged command cannot cause an overflow
  (which would behave differently in debug and release builds).
- Integer overflow must not depend on build mode. Use explicit `wrapping_*`,
  `checked_*` or `saturating_*` where overflow is possible.
- The algorithms of `Rng`, `StableHasher` and `Sim::state_hash` are part of the
  protocol. Changing them changes the golden hash and breaks replays.
- Map data is integer: heights in millimeters (`i32`), terrain ids (`u8`), map
  objects (positions in millimeters, headings in whole degrees, shapes rotated
  with a fixed integer sine table, `sim::objects::sin_cos_deg`). Bilinear
  interpolation runs in `Fixed` on millimeters and divides by 1000 once. The state
  hash contains the map's content hash, so peers on different map versions desync
  immediately instead of drifting apart. See [MAPS.md](MAPS.md).
- Single player runs through `net::LockstepSession` as well, so there is one code
  path for applying commands.
- Pathfinding (see [ARCHITECTURE.md](ARCHITECTURE.md#pathfinding)) is integer-only:
  `u16` cell costs, `u32` path costs, octile heuristic. The A* open list is ordered by
  the full key `(f, h, cell)`, so equal costs never depend on the heap's internal
  order; neighbors are visited in a fixed order and only strictly better costs
  replace a node. Requests run in FIFO order with a fixed node budget per tick
  (`sim::path::NODE_BUDGET_PER_TICK`, a rule constant like the tick rate), so the
  tick in which a path arrives is the same on every peer. Separation is computed
  from the positions before the movement of the tick. Routes, the queue and the
  grid version are part of the state hash.
- Combat (see [ARCHITECTURE.md](ARCHITECTURE.md#combat)) is `Fixed` and integer
  only: angles as unit vectors turned by small-angle Taylor series, square
  roots with `isqrt`, penetration `v^1.5` as `v · sqrt(v)`. Random numbers
  (dispersion, penetration rolls) come from the simulation's `Rng` in a fixed
  order: units by id, weapons in catalog order, barrels in order, projectiles
  by id. Events are derived and not hashed; projectiles, hit points and
  combat state are.
- Vision (see [ARCHITECTURE.md](ARCHITECTURE.md#vision-and-fog-of-war)) is
  integer only: heights in mm, horizon slopes compared by cross-multiplying
  (`i64`), precomputed ray tables with symmetric integer rounding. Viewsheds
  are updated in FIFO order with a fixed ray-step budget per tick
  (`data/vision.toml`, part of the rules), so a late update lands in the
  same tick everywhere. Vision rules, viewshed anchors, the queue and who
  sees which unit are in the state hash; the per-cell counts follow from
  the viewsheds. The fog the bridge draws is per player and not part of
  any hash.
- `sim` is built with `opt-level = 3` in the dev profile too. Optimization does not
  change integer results; overflow checks stay on in debug builds.

## Enforcement

- `crates/sim/Cargo.toml` denies `clippy::float_arithmetic`.
- `crates/sim/clippy.toml` disallows `HashMap`, `HashSet`, `DefaultHasher`,
  `RandomState`, `Instant`, `SystemTime` and `std::thread::spawn`.
- `crates/sim/tests/determinism.rs` contains a golden-hash test: the demo scenario
  (two players with eight tanks each, scripted group commands, seed 42, 1000 ticks,
  on the built-in map `test_small`) must produce `0xfe523a472761c97c`. CI runs it on
  macOS (ARM64), Linux and Windows (x86_64). The value was recorded on macOS ARM64.
  History: `0x075d8c59dfe3d336` (prompt 001/003, four tanks, single-unit commands),
  `0x8e8b2c6ed986ef2c` (prompt 009: owners, group commands with formation, `Stop`,
  owner and footprint in the hash), `0x13dfd6119523d352` (prompt 010: map with
  heights and terrain, terrain- and slope-dependent speed, unit height, map hash and
  terrain rules in the hash), changed in prompt 012 (movement along A* paths, path
  queue, routes and grid version in the hash, `test_small` with wider fords),
  `0xc56989d73505efba` (prompt 012), changed in prompt 013 (map objects on
  `test_small` block path cells by size class; objects and their resolved rules
  in the map hash), `0x9d8c67c292c66be5` (prompt 013), changed in prompt 017
  (combat: hit points, facing, turrets, projectiles, combat rules and state in
  the hash; the demo tanks fight), prompts 027 and 014, changed in prompt 018
  (vision: targets and fire need the unit's own line of sight, attack orders
  follow the last known position, vision rules and state in the hash; replay
  `FORMAT_VERSION` 5), prompts 020, 022 and 024 (destruction, infantry),
  `0xc9ba61182de57872` (prompt 023: economy rules, unit prices, accounts,
  structures, production queues and supply points in the hash; `test_small`
  has three supply point markers; the demo battle is unchanged), prompt 021
  (craters), `0xfe523a472761c97c` (merge of 023 with 021: both states in the
  hash; the demo battle as in 021, decided at tick 480).
  Regenerating `test_small` changes it as well.
- `crates/sim/tests/economy.rs` has a second golden hash: the base-building
  skirmish with a scripted production chain (build, produce, rally points,
  the produced units fighting), seed 42, 5000 ticks. `Sim::spawn_unit` adds
  units during the match with increasing ids, so the unit list stays sorted.
- `crates/net/tests/economy.rs` plays a production chain on two peers in
  lockstep: each peer decides only its own player's orders from its own
  copy of the simulation; the per-tick hashes, accounts and queues agree and
  the replay plays back to the same end.
- `crates/sim/tests/combat.rs` plays duels with catalog units: a Panzer IV
  destroys a T-34 at the tick derived from catalog and rules, armor faces and
  angles, a ridge stopping rounds, hold fire, attack orders, infantry, and a
  battle that is identical for the same seed.
- `crates/net/tests/loopback.rs` plays the demo between two peers in lockstep with
  group commands, compares the hash of every tick, checks that the result equals the
  scripted single-process run, and checks that a divergence is detected through the
  hash exchange.
- `crates/net/tests/loopback.rs` also changes both peers' input delays independently
  during the match and checks that they stay in sync with the scripted run.
- `crates/net/tests/lan.rs` plays the demo over real UDP on localhost (ENet, two and
  three peers, the host relaying); `crates/cli/tests/lan.rs` does the same with two
  separate `cli` processes that find each other with the LAN search, and checks that a
  provoked desync is reported by both. `just lan-selftest` plays a LAN match between
  two headless Godot instances.
- `crates/net/tests/replay.rs` records a lockstep match, encodes and decodes it, and
  re-simulates it to the same end hash; a tampered command is reported.
- `crates/net/tests/snapshot.rs` and `crates/ai/tests/save.rs` (prompt 038): restored
  snapshots and loaded games continue tick for tick like the original.

## Replays

A replay (`net::Replay`) is the seed, the resolved `SimConfig`, the merged commands of
every tick and the end hash. Playback is just `Sim::new` plus `step` with the recorded
commands, so any rule above that is broken shows up as an end-hash mismatch in
`just replay`. A replay is tied to the simulation rules: it carries the rules hash of
the build that wrote it (prompt 038), and a build with other rules rejects it up front
(`ReplayError::RulesChanged`) instead of reporting a mismatch at the end. Bump
`net::replay::FORMAT_VERSION` only when the file layout changes.

## Snapshots and the rules hash

Prompt 038. A snapshot (`sim::snapshot`, `net::snapshot`) saves every field of `Sim`
through serde, the map as changes against the loaded map. Rules for state:

- **Every new state type derives `Serialize` and `Deserialize`.** `Sim` derives both,
  so the compiler refuses a field that does not. Do not `#[serde(skip)]` state: a
  skipped field comes back as its default and the restored simulation diverges. Only
  the map is skipped, because `Sim::restore` puts it back.
- Derived data (caches, grids, indices) is saved as well. If one of them must not be
  saved, rebuild it in `Sim::restore` and prove it with the round-trip test.
- No `skip_serializing_if`, `flatten` or `untagged` in state types: postcard is not
  self-describing.
- `crates/net/tests/snapshot.rs` restores the demo at several ticks (with craters)
  and steps original and restored copy side by side: the state hash must agree every
  tick; `crates/net/tests/snapshot_indirect.rs` does the same with mortar shells in
  flight and smoke clouds drifting (prompt 032). `crates/ai/tests/save.rs` saves matches between computer players (battle and
  base building), loads and continues them: identical to the never-saved match, and the
  replay re-simulated from tick 0 ends in the snapshot's state.

`net::snapshot::rules_hash()` hashes the snapshot bytes after a fixed reference run of
the demo. It changes with every rule change (the state differs) and every new state
field (the bytes differ), so replays and saved games of other builds are rejected
without a version constant to maintain. It also changes with catalog edits of demo
units **[provisional]**, see ARCHITECTURE.md, "Saved games and replay viewer".

If the golden hash changes, either a rule above was broken or the simulation rules
changed on purpose. Only in the latter case update the constant, and say so in the
commit message.

## Why fixed-point with the `fixed` crate

`fixed::types::I32F32` (32 integer, 32 fractional bits) is used as `sim::Fixed`:

- Integer arithmetic only, so results are identical on every platform.
- Range ±2.1 million km with nanometer resolution: no realistic overflow on a map.
- Multiplication and division are exact via 128-bit intermediates.
- Well tested, `no_std`, with `serde` support for network messages.

Square roots are computed with `u128::isqrt` on the raw bits (`Vec2::length`), which is
exact integer math and does not depend on the crate's implementation.

## Floats outside the simulation

Floats are fine in the bridge and in Godot, because they only affect what is drawn.
The bridge converts in two places:

- **Out:** `Fixed` to `f32` for positions in snapshots.
- **In:** mouse targets in meters are rounded to whole millimeters and then turned into
  `Fixed`. This happens only on the machine that issues the command; the resulting
  integer values are what travels through lockstep, so every peer sees the same input.
