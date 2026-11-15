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
  objects.csv   optional: trees, buildings, hedges, walls, props (see "Objects")
```

`map.toml`:

```toml
name = "test_small"
generator = "cli gen-map --name test_small --size-m 500 --seed 7"   # informational
cell_size_mm = 2000          # distance between samples
samples = [251, 251]         # [x, y] = width and height of both PNGs
height_base_mm = -6000       # height of PNG value 0
height_step_mm = 1           # millimeters per PNG step
biome = "forest"             # forest | desert | winter; presentation only (optional)

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

Units drive around impassable terrain, too steep grades and the map edge on paths
planned on a 4 m grid (2 × 2 samples per cell, see
[ARCHITECTURE.md](ARCHITECTURE.md#pathfinding)). Passages need to be about 4 m wide
for infantry and about 8 m for tanks (one aligned cell, plus the 2 m margin tanks
keep from impassable ground); design fords and bridges accordingly.

### Loading and identity

The simulation never opens files. Callers (CLI, bridge, tests) read the three files
and call `sim::Map::load(toml, height_png, terrain_png)`, then
`Map::add_objects(csv)` if the folder has an `objects.csv`. The built-in map
`test_small` is compiled into `sim` (`sim::scenario::builtin_map`), like the catalog,
so tests and the golden hash need no file system.

A map has a content hash (grid, heights, terrain, objects with their resolved rules;
not the name or `map.toml` metadata). `SimConfig::map` is a `MapRef { name, hash }`; `Sim::new(config, map,
seed)` rejects a map whose hash differs, and the hash is part of the state hash.
Replays store only the reference, so `just replay` loads `data/maps/<name>/` again.

### Biome

`biome` chooses the look of the map (prompt 015): lighting, haze, ground colors,
camouflage palettes and environment looks. It is optional (default `forest`, the
temperate biome); other values than `forest`, `desert` and `winter` fail to load.
`sim::Map::biome()` keeps it for the bridge (`SimHost.map_biome()`), but it is
not part of the map hash or the state hash and no game rule reads it, so the same
height and terrain data can be played in every biome. `cli gen-map --biome <id>`
writes it; in Godot `-- --biome=<id>` overrides it. Details:
[ARCHITECTURE.md](ARCHITECTURE.md#lighting-atmosphere-and-biomes-presentation-only).
### Objects

Trees, buildings, hedges, walls and props that matter for the game (they block
movement now, and line of sight, cover and destruction later) are **map objects**
owned by the simulation. Grass, flowers, small bushes and stones are decoration and
exist only in Godot (see [ARCHITECTURE.md](ARCHITECTURE.md#map-objects-and-decoration-presentation-only)).

`objects.csv`, one object per line (`#` starts a comment line):

```
kind,variant,x_mm,y_mm,heading_deg
env_tree_deciduous,Oak,-41250,12034,217
env_house_farm,Damaged,-1093402,1047811,90
env_hedge,Segment,180412,-3311,4
```

- `kind` is a catalog id of `data/catalog/neutral.toml`, `variant` a model variant
  of it (`assets-src/blueprints/neutral/<kind>.toml`).
- `x_mm`, `y_mm`: center of the object in map coordinates (millimeters).
- `heading_deg` (0–359): clockwise seen from above; 0 means the model's +Y axis
  (Blender; its length) points north (−y), 90 east. Models face −Z in Godot, so the
  bridge turns them by −heading about Y.

**Rules** (`data/objects.toml`, compiled into `sim`, `sim::ObjectRules`): per kind
the variants a map may use, the size classes it blocks (`none`, `small` for
infantry, `large` for tanks and trucks, `all`) and its ground shape: a circle
(`radius_mm`, e.g. a trunk) or a box (`length_mm` along the heading × `width_mm`,
by default the catalog size). Variants may override both. These are game rules:

| Objects | Blocks | Shape |
|---|---|---|
| Trees (deciduous, conifer, birch, palm) | large | trunk circle 0.3–0.5 m |
| Hedges, dry-stone walls, large rocks, hedgehogs | large | catalog box / circle |
| Houses, barns, church (every damage level), bunker | all | catalog box |
| Barbed wire | small | catalog box |
| Fences, bushes, hay, props, stumps, logs, poles, medium/small rocks, trenches | none | – |

Woods, hedgerows and walls are closed to tanks and open to infantry; tanks use
gaps and tracks. Loading resolves every line against the rules; unknown kinds or
variants, objects outside the map, bad numbers and headings are errors with the line
number. The kinds a map uses and their resolved shapes become part of the map hash,
so changing a rule changes the hash of every map using that kind (old replays then
report the different map instead of desyncing).

`objects.csv` is plain text so it can be edited in any tool and diffed, and lives in
Git LFS like the PNGs (`test_large`: 98 000 lines, 4.2 MB). A compact binary format
would only pay off far beyond that; parsing takes about 50 ms.

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
| Objects (prompt 013) | 98 375 (96 404 blocking, 32 kinds); `objects.csv` 4.2 MB, 2.4 MB in memory, parse + resolve + hash 52 ms |
| `height_at` | 35–40 ns per lookup (random points; 2 lookups per unit and tick) |
| Godot start incl. map and debug mesh | 1.2 s (debug bridge, headless) |

`test_small` (500 m): 63 001 samples, 0.06 MB files, 0.3 MB memory, 1.4 ms to load;
1 447 objects (2.6 ms).

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
  the river (a ford: shallow instead of deep water within 7 m of the track, wide
  enough for tanks; prompt 012), plus a road from the west edge to the first village.
  Mud patches in the open.
- **Objects** (`crates/cli/src/gen_map/objects.rs`, prompt 013), from a random
  stream of their own, so heights and terrain do not change:
  - **Villages:** farmhouses, brick and stone houses around each square, turned with
    the long side to it, mostly intact (about 12 % damaged, 6 % ruins); a church in
    the first village; behind the houses yards with barns, haystacks, woodpiles,
    crates and barrels, a wall or fence line and often an **orchard** of young trees
    on an 8 m grid.
  - **Road side:** an avenue of oaks and beeches 8 m from the road, telegraph poles.
  - **Hedgerows** 5.5 m beside the field tracks, segments 2.7 m apart (they overlap
    on curves), in runs of 30–160 m with entrances of 9–21 m wide enough for tanks,
    now and then a tree in the hedge.
  - **Woods** from a noise mask (about 20 % of `test_large`): trees on a jittered
    6.5 m grid, stands of oak and beech, spruce and pine, or birch, birches and young
    trees at the edges. Trunks keep 6 m from the edges of tracks, so tracks stay open
    through woods.
  - **Fields:** single trees, groups of haystacks, fallen logs; boulders on the hills.
  - Kept free: the start meadow, water and banks, village squares, the landmark
    hill top and a corridor from the meadow to it (self-test and demo drive there).

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
of [DETERMINISM.md](DETERMINISM.md) and needs to be named in the commit. A test
generates a small map twice and compares `objects.csv`; another checks that every
variant in `data/objects.toml` exists in the model's blueprint.

## Presentation

Since prompt 011, `godot/scenes/terrain/terrain.gd` draws the height field as chunks
with five LOD levels and a splatting shader (data path, LOD, shader and measurements:
[ARCHITECTURE.md](ARCHITECTURE.md#terrain-rendering-presentation-only)). The bridge
exposes `map_size`, `height_at`, `terrain_at`, `height_grid`, `terrain_grid`,
`terrain_colors`, `map_marker`, `ray_pick`, and for the renderer `terrain_layers`,
`terrain_chunk_ranges` and `debug_dent`. Mouse picking marches along the ray in
steps growing from 0.5 m to 20 m and refines the hit by bisection, in floats on the
same grid (presentation only); the order that results is quantized to millimeters
as before. `debug_color` in `terrain.toml` is still used for the PNG palette.

Map objects and decoration are drawn by `godot/scenes/world/map_objects.gd`
(prompt 013): `object_kinds`, `object_count`, `object_groups`, `object_position`
and `scatter_deco` in the bridge.
