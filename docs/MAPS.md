# Maps

The map is simulation data. Heights and terrain types decide speed now and line of
sight (prompt 018) and craters (prompt 021) later, so they live in `crates/sim`,
deterministic and integer-only. Godot only reads them for drawing.

## Format

One folder per map under `data/maps/<name>/`:

```
data/maps/test_small/
  map.toml      name, grid, height encoding, start positions, markers
  height.png    16-bit grayscale, one pixel per sample
  terrain.png   8-bit grayscale or 8-bit indexed, value = terrain id
```

`map.toml`:

```toml
name = "test_small"
generator = "cli gen-map --name test_small --size-m 500 --seed 7"   # informational
cell_size_mm = 2000          # distance between samples
samples = [251, 251]         # [x, y] = width and height of both PNGs
height_base_mm = -6000       # height of PNG value 0
height_step_mm = 1           # millimeters per PNG step

[[start]]                    # start positions per player (for scenarios, later menus)
player = 0
position_m = [0, 60]

[[marker]]                   # named points for tools and tests
name = "hill_top"
position_m = [-104, -142]
```

Height of a sample: `height_base_mm + png_value * height_step_mm` millimeters. All
heights must stay within ±10 km.

### Coordinates

The map is centered on the origin. Pixel `(i, j)` of both PNGs is the sample at
`x = -width/2 + i * cell`, `y = -depth/2 + j * cell` (sim coordinates: x east, y
south; Godot `(x, h, y)`). Image columns therefore run east and rows run south, so the
PNG looks like the map seen from above with north up. Width and depth must be whole,
even meters, so the half extent is exact.

### Height between samples

`Map::height_at` interpolates bilinearly between the four surrounding samples. The
interpolation runs in `Fixed` on millimeters and divides by 1000 once at the end:
heights at samples, at cell centers and at simple fractions are exact whenever the
result is representable (tested in `map.rs` with exact `Fixed` values). Points outside
the map use the nearest edge height.

### Terrain

Every sample carries one terrain id; a position uses the nearest sample, so each id
covers the 2 m × 2 m square around its sample. The ids index the types in
`data/terrain.toml`:

| Id | Name | Speed | Passable |
|---|---|---|---|
| 0 | grass | 100 % cross-country | yes |
| 1 | dirt_track | 125 % cross-country (at most road speed) | yes |
| 2 | road | 100 % road | yes |
| 3 | sand | 70 % cross-country | yes |
| 4 | mud | 40 % cross-country | yes |
| 5 | snow | 60 % cross-country | yes |
| 6 | shallow_water | 30 % cross-country | yes |
| 7 | deep_water | – | no |

Slope (also in `terrain.toml`): uphill the speed drops by 2 % per percent of grade
(rise over the next meter), down to 20 %; grades above 60 % (31°) cannot be climbed.
Downhill has no effect yet. These are game rules, not historical values; per-vehicle
climbing ability, tracked versus wheeled ground pressure and cover follow later.

A unit that would enter impassable terrain, climb too steep a grade or leave the map
stops and drops its target. Pathfinding around obstacles comes with prompt 012.

### Loading and identity

The simulation never opens files. Callers (CLI, bridge, tests) read the three files
and call `sim::Map::load(toml, height_png, terrain_png)`. The built-in map
`test_small` is compiled into `sim` (`sim::scenario::builtin_map`), like the catalog,
so tests and the golden hash need no file system.

A map has a content hash (grid, heights, terrain; not the name or `map.toml`
metadata). `SimConfig::map` is a `MapRef { name, hash }`; `Sim::new(config, map,
seed)` rejects a map whose hash differs, and the hash is part of the state hash.
Replays store only the reference, so `just replay` loads `data/maps/<name>/` again.

## Decisions (prompt 010)

| Question | Decision | Why |
|---|---|---|
| Grid | 2 m cells, samples at the corners (4 km: 2001 × 2001) | Finer than any vehicle (Panzer IV 7 m), so slopes under a tank are smooth; coarser grids (4 m) would blur river banks and tracks to one cell. Finer grids (1 m) quadruple memory (80 MB at 4 km) for detail that the debug mesh and pathfinding do not need. |
| Height values | `i32` millimeters in memory, `u16` steps in the file | Millimeters keep all math integer and exact; `i32` leaves room for craters (021) beyond the file's range. The file maps 65 536 steps onto `base + value * step`: 1 mm steps give 65 m of relief, `test_large` uses 3 mm steps for 129 m. |
| Interpolation | Bilinear in `Fixed`, in millimeters, one division | Simple, continuous, exact at samples; exact test values possible. |
| Terrain | One byte per sample, same grid as heights | One image size for both rasters, simple tooling; 256 types are plenty. |
| File format | Folder with `map.toml` + 16-bit `height.png` + 8-bit `terrain.png` | Every image program and script can edit PNG; the indexed terrain PNG shows terrain colors in any viewer. Compressed (test_large: 2.9 MB instead of 12 MB raw). Metadata stays readable TOML. |
| PNG decoder | `png` crate 0.18 with `Transformations::IDENTITY` | Decoding is integer-only: inflate (`fdeflate`/`miniz_oxide`), unfiltering and CRC use integers; the crate's only floats are optional gamma/chromaticity accessors that the map loader never calls (checked in the source). PNG is lossless, so the raw samples are fully specified and any correct decoder yields the same bytes; the map hash in the state hash would reveal a difference. An own raw format would need no dependency but would lose editing in image programs. |
| Storage | PNGs in Git LFS (`*.png` rule) | CI checks out with `lfs: true`; without LFS the built-in map fails to load with a hint. |

Hard to reverse: the cell size and the millimeter unit (all maps and the golden hash
depend on them). The file encoding (`u16` + base/step) can change later without
touching the simulation.

## Measurements (`test_large`, 4 km × 4 km)

`just map-info test_large`, Apple M1 Max, release build:

| Quantity | Value |
|---|---|
| Samples | 2001 × 2001 = 4 004 001 |
| Files | 2.94 MB (height.png 2.9 MB, terrain.png 0.06 MB) |
| Memory of the loaded map | 20.0 MB (heights 16.0 MB as `i32`, terrain 4.0 MB) |
| Transient while decoding | + 12 MB (raw PNG rows); peak RSS of `map-info` 50 MB including its 16 MB benchmark points |
| Read files | 0.5 ms |
| Decode + validate + hash | 69 ms (debug build: 690 ms) |
| `height_at` | 35–40 ns per lookup (random points; 2 lookups per unit and tick) |
| Godot start incl. map and debug mesh | 1.2 s (debug bridge, headless) |

`test_small` (500 m): 63 001 samples, 0.06 MB files, 0.3 MB memory, 1.4 ms to load.

If 8 km maps ever come: 80 MB with this layout. Then store heights as `u16` + base and
step in memory too (half), or as tiles; prompt 016 covers large maps.

## Generator

`cli gen-map` (`crates/cli/src/gen_map.rs`) builds a map from a seed:

- **Ground:** three octaves of value noise (3 m, 1 m, 0.15 m amplitude).
- **Hills:** smooth bumps `h · (1 − d²/r²)²` with a steepest grade of about
  `1.54 · h / r`, kept below ~35 %. One landmark hill north-west of the meadow
  (marker `hill_top`); more hills per area on larger maps, away from the meadow and
  the river.
- **River bed:** a wobbly line roughly north–south in the east, cut 3.5 m deep with
  gentle banks: deep water in the middle, shallow water, sand banks and mud patches.
- **Start meadow:** flat at height 0 around the origin (radius 0.38 × half size, at
  most 160 m), blended into the surroundings; the demo scenario stands on it.
- **Village squares:** flat road-covered circles at the natural height (marker
  `village`), one per 1.5 km of map size.
- **Field tracks** from the meadow to the edges, the villages, the hill foot and across
  the river (a ford: shallow instead of deep water), plus a road from the west edge to
  the first village. Mud patches in the open.

It uses floats (it is a tool, not the simulation) but only basic arithmetic and `sqrt`,
which IEEE 754 rounds exactly, so the same seed produces the same files everywhere
(checked: regenerating gives bit-identical PNGs).

```bash
just gen-map my_map 1000 3     # data/maps/my_map/, 1 km, seed 3
just test-maps                 # regenerate test_small (500 m, seed 7) and test_large (4 km, seed 11)
just map-info test_large       # measurements as above
just run-map test_large        # play the demo on a map from data/maps/
```

Regenerating `test_small` changes the golden hash: that is a rule change in the sense
of [DETERMINISM.md](DETERMINISM.md) and needs to be named in the commit.

## Presentation

Until prompt 011, `godot/scenes/terrain/terrain_debug.gd` draws the height field as one
mesh of at most 256 × 256 vertices (`test_large`: every 8th sample), colored by
terrain type (`debug_color` in `terrain.toml`) with grid lines every 10/100 m and
contour lines every 1/5 m. The bridge exposes `map_size`, `height_at`, `terrain_at`,
`height_grid`, `terrain_grid`, `terrain_colors`, `map_marker` and `ray_pick`. Mouse
picking marches along the ray in steps growing from 0.5 m to 20 m and refines the hit
by bisection, in floats on the same grid (presentation only); the order that results
is quantized to millimeters as before.
