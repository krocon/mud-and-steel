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
- Integer overflow must not depend on build mode. Use explicit `wrapping_*`,
  `checked_*` or `saturating_*` where overflow is possible.
- The algorithms of `Rng`, `StableHasher` and `Sim::state_hash` are part of the
  protocol. Changing them changes the golden hash and breaks replays.

## Enforcement

- `crates/sim/Cargo.toml` denies `clippy::float_arithmetic`.
- `crates/sim/clippy.toml` disallows `HashMap`, `HashSet`, `DefaultHasher`,
  `RandomState`, `Instant`, `SystemTime` and `std::thread::spawn`.
- `crates/sim/tests/determinism.rs` contains a golden-hash test: the demo scenario
  (seed 42, 1000 ticks) must produce `0x42903d08f29e27a6`. CI runs it on macOS (ARM64),
  Linux and Windows (x86_64). The value was recorded on macOS ARM64.
- `crates/net/tests/loopback.rs` runs two simulations in lockstep and checks that a
  divergence is detected through the hash exchange.

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
