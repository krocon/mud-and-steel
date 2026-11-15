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
- Single player runs through `net::LockstepSession` as well, so there is one code
  path for applying commands.

## Enforcement

- `crates/sim/Cargo.toml` denies `clippy::float_arithmetic`.
- `crates/sim/clippy.toml` disallows `HashMap`, `HashSet`, `DefaultHasher`,
  `RandomState`, `Instant`, `SystemTime` and `std::thread::spawn`.
- `crates/sim/tests/determinism.rs` contains a golden-hash test: the demo scenario
  (two players with eight tanks each, scripted group commands, seed 42, 1000 ticks)
  must produce `0x8e8b2c6ed986ef2c`. CI runs it on macOS (ARM64), Linux and Windows
  (x86_64). The value was recorded on macOS ARM64. History: `0x075d8c59dfe3d336`
  (prompt 001/003, four tanks, single-unit commands), changed in prompt 009 (owners,
  group commands with formation, `Stop`, owner and footprint in the hash).
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
