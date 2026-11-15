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
- `sim` is built with `opt-level = 3` in the dev profile too. Optimization does not
  change integer results; overflow checks stay on in debug builds.

## Enforcement

- `crates/sim/Cargo.toml` denies `clippy::float_arithmetic`.
- `crates/sim/clippy.toml` disallows `HashMap`, `HashSet`, `DefaultHasher`,
  `RandomState`, `Instant`, `SystemTime` and `std::thread::spawn`.
- `crates/sim/tests/determinism.rs` contains a golden-hash test: the demo scenario
  (two players with eight tanks each, scripted group commands, seed 42, 1000 ticks,
  on the built-in map `test_small`) must produce `0x9d8c67c292c66be5`. CI runs it on
  macOS (ARM64), Linux and Windows (x86_64). The value was recorded on macOS ARM64.
  History: `0x075d8c59dfe3d336` (prompt 001/003, four tanks, single-unit commands),
  `0x8e8b2c6ed986ef2c` (prompt 009: owners, group commands with formation, `Stop`,
  owner and footprint in the hash), `0x13dfd6119523d352` (prompt 010: map with
  heights and terrain, terrain- and slope-dependent speed, unit height, map hash and
  terrain rules in the hash), changed in prompt 012 (movement along A* paths, path
  queue, routes and grid version in the hash, `test_small` with wider fords),
  `0xc56989d73505efba` (prompt 012), changed in prompt 013 (map objects on
  `test_small` block path cells by size class; objects and their resolved rules
  in the map hash).
  Regenerating `test_small` changes it as well.
- `crates/net/tests/loopback.rs` plays the demo between two peers in lockstep with
  group commands, compares the hash of every tick, checks that the result equals the
  scripted single-process run, and checks that a divergence is detected through the
  hash exchange.
- `crates/net/tests/replay.rs` records a lockstep match, encodes and decodes it, and
  re-simulates it to the same end hash; a tampered command is reported.

## Replays

A replay (`net::Replay`) is the seed, the resolved `SimConfig`, the merged commands of
every tick and the end hash. Playback is just `Sim::new` plus `step` with the recorded
commands, so any rule above that is broken shows up as an end-hash mismatch in
`just replay`. A replay is tied to the simulation rules: after an intentional rule
change (new golden hash), older replays report a mismatch. That is expected; bump
`net::replay::FORMAT_VERSION` only when the file layout changes.

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
