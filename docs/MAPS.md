# Maps

The map is simulation data. Heights and terrain types decide speed and line of
sight (prompt 018, `data/vision.toml`), and craters change them during a match
(prompt 021, [Craters](#craters)), so they live in `crates/sim`,
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
water_flow_deg = 180         # direction rivers flow towards (0 = north); presentation only (optional)
weather = "rain"             # default weather of matches (optional, prompt 040): clear | overcast | rain | snow | fog
daytime = "night"            # default time of day (optional): morning | day | evening | night

[[start]]                    # start positions per player: the scenario's units stand here
player = 0
team = 0                     # optional (prompt 037): the lobby's suggestion; not in the hash
position_m = [0, 60]

[[marker]]                   # named points for tools and tests
name = "hill_top"
position_m = [-104, -142]
```

Markers whose name starts with `supply` are **supply points** (prompt 023,
[GAME_DESIGN.md](GAME_DESIGN.md#resources-provisional)): units standing there
take them and they pay resources to the holder. `cli gen-map` writes three:
`supply_village`, `supply_hill` and `supply_ford` at the village square, the
landmark hill and the ford; the map editor keeps them but cannot place them
yet.

Teams (prompt 037, [TEAMS.md](TEAMS.md)): a map takes as many players as it
has starts; `team` suggests the teams for the lobby. `cli gen-map --players
4|6` (`just gen-map NAME SIZE SEED --players 4`) writes starts for two teams
on the start meadow and more supply points there (`supply_meadow_*`).
`twin_meadow` (1 km, 4 starts) is the 2-against-2 map, `test_large` has 6
starts; `test_small` keeps its two.

Since prompt 028 the scenario places every player's units at that player's
`[[start]]`: all groups of the player move by the offset between the start and
the center of the player's first group (`sim::scenario`, layout unchanged).
Maps without a start for a player keep the scenario's positions; the test maps
have their starts at the demo's centers.

### Start variants (prompt 182)

A map may have further start layouts besides its `[[start]]` entries (the
"standard" layout):

```toml
[[variant]]
name = "west_east"                 # id: lowercase letters, digits, _ (not standard, random)
title = "West against east"        # shown in the lobby (optional, default: the name)
supply = ["supply_square", "supply_well"]  # optional: only these supply points count

[[variant.start]]                  # the same players as the standard starts
player = 0
team = 0
position_m = [-330, 90]
```

- The single-player setup, the LAN lobby and the Steam lobby offer
  **Start layout**: Standard, Random and the variants (only on maps that
  have some). Random in a lobby is picked by the host from the match seed
  (`Map::choose_variant`, the same on every peer); in single player the menu
  picks one when the match starts. The match records the chosen layout
  (`SimConfig::variant`: name and supply points), so replays and saved
  games keep it.
- It is an input of the scenario, no hidden state: the host places the units
  at the variant's starts (`sim::scenario::build_in_variant` with
  `Map::with_variant`), the simulation only reads which supply points count.
  Starts are no part of the map hash, so the map hash stays; the golden hash
  is unchanged (the demo uses the standard layout).
- Lobby teams and team sizes follow the standard starts (a variant places the
  same players; its `team` entries are kept for the editor).
- The computer players look for enemy homes at the variant's starts.
- Check, `just map-check` and the editor check every variant like the
  standard layout (messages begin with "start variant '<name>':");
  `just map-reach` lists every variant's starts and their path lengths to the
  variant's supply points.

Second layouts (measured in prompt 182, `just map-reach`: every start in the
main area for vehicles and infantry):

| Map | Variant | Starts | Supply points |
|---|---|---|---|
| skirmish | `east_west` | west (-600, 100) against east (520, -420), the river behind the east | all |
| farms | `west_east` | west against east, two per side | all |
| desert_village | `west_east` | west against east, two per side | all three |
| twin_meadow | `river_sides` | west of the village and the meadow against east of the river | meadows, village, hill (not the ford) |
| winter_village | `west_east` | west against east, two per side | square and trench hill only |

`just ai-tournament --map <map> --variant <name> --a hard --b normal
--games 40` (1 against 1: players 0 and 1; hard plays player 0), wins of
hard / normal / timeouts, standard → variant: skirmish 5/65/12 % →
7/42/20 %; farms 27/35/15 → 47/5/19 (the west start favors player 0);
desert_village 42/47/4 → 40/40/8; twin_meadow 37/62/0 → 52/30/7 (the
river between the sides makes games longer); winter_village 35/0/26 →
42/0/23 (normal never wins there in either layout). The pairing is not
mirrored, so the numbers mix strength and side; desert_village is the only
layout pair that is even.

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
| 6 | shallow_water | 30 % cross-country | yes (fords) |
| 7 | deep_water | – | no (bridges only) |
| 8 | tall_grass | 100 % cross-country (like grass) | yes |
| 9 | field_grain | 100 % cross-country (like grass) | yes |
| 10 | field_green | 100 % cross-country (like grass) | yes |
| 11 | field_ploughed | 100 % cross-country (like grass) | yes |

`tall_grass` (prompt 073) is dense, knee- to chest-high grass. In the rules it
is grass (same speed, softness in rain and snow, no cover or concealment yet);
the presentation draws it as tall tufts, herbs and shrubs with a dark ground
under them and a far look of its own (see
[ARCHITECTURE.md](ARCHITECTURE.md#tall-grass-prompt-073)). Maps on which tests or
measurement series get it only where it changes nothing: `river_land` (banks)
and `farms` (fallow patches) were painted by `cli gen-land`, and the AI
tournament gives the same games there before and after; `tall_grass_test`
(`cli gen-map --tall-grass`, 47 %) is the test map for pictures and
measurements. Paint it in the map editor with the terrain brush (the
palette shows a swatch of every terrain's color).

Fields (prompt 088): `field_grain` (ripe grain), `field_green` (beet, clover,
young crops in rows) and `field_ploughed` (ploughed soil). In the rules they
are grass for now (speed, softness in rain and snow, no cover or
concealment); slower wheels on ploughed soil and infantry hidden in grain are
open. The presentation counts them as grass in the ground blend and draws
each field with rows, tramlines or furrows along its long axis (computed per
connected field by the bridge, not stored in the map; see
[ARCHITECTURE.md](ARCHITECTURE.md)); under snow the rows show faintly. Paint
them with the terrain brush of the map editor.

Slope (also in `terrain.toml`): uphill the speed drops by 2 % per percent of grade
(rise over the next meter), down to 20 %; grades above 60 % (31°) cannot be climbed,
and since prompt 014 not driven down either (units used to drop off abutments).
Downhill has no other effect yet. These are game rules, not historical values; per-vehicle
climbing ability, tracked versus wheeled ground pressure and cover follow later.

In the game the **ground tooltip** (prompt 100, `hud/ground_tooltip.gd`,
`Sim::ground_info`) shows these rules for the point under the mouse: terrain
speed and road basis, the soft-ground factor per running gear of the
weather, passability per size class from the path grid, the slope factor,
and from the other rule files concealment (vision cell: `solid` and
`foliage` objects of `data/vision.toml`, smoke) and cover (objects that
block tanks stop bullets, `[[cover.object]]` of `data/infantry.toml`, dug
positions, craters). Terrain types themselves carry neither concealment nor
cover: tall grass and grain hide nobody, and woods slow vehicles only by
their trunks (path grid), not by a terrain speed.

Units drive around impassable terrain, too steep grades and the map edge on paths
planned on a 4 m grid (2 × 2 samples per cell, see
[ARCHITECTURE.md](ARCHITECTURE.md#pathfinding)). Passages need to be about 4 m wide
for infantry and about 8 m for tanks (one aligned cell, plus the 2 m margin tanks
keep from impassable ground); design fords accordingly. Bridges need less: see
[Bridges](#bridges).

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

### Water flow

`water_flow_deg` (optional, default 180 = south, `0..360`, compass degrees)
tells the water renderer which way rivers flow; along a river the flow follows
the banks, this value only picks the sign. Like `biome` it is presentation
only: not in the map hash, no rule reads it (`sim::Map::water_flow_deg()`).

### Biome

`biome` chooses the look of the map (prompt 015): lighting, haze, ground colors,
camouflage palettes and environment looks. It is optional (default `forest`, the
temperate biome); other values than `forest`, `desert` and `winter` fail to load.
`sim::Map::biome()` keeps it for the bridge (`SimHost.map_biome()`), but it is
not part of the map hash or the state hash and no game rule reads it, so the same
height and terrain data can be played in every biome. `cli gen-map --biome <id>`
writes it; in Godot `-- --biome=<id>` overrides it. Details:
[ARCHITECTURE.md](ARCHITECTURE.md#lighting-atmosphere-and-biomes-presentation-only).

### Weather and time of day

`weather` and `daytime` (optional, prompt 040; default `clear` by `day`) are
the defaults of matches on the map; the setup screen may choose others
(`Map default` keeps them), `-- --weather=<id> --daytime=<id>` overrides them
for direct starts, and LAN and online matches use them as they are. Unlike
`biome` they are game rules: the match carries its choice in
`SimConfig::weather` (replays, saved games, the state hash), and rain soaks
the ground, which slows wheels more than tracks off the roads. They are not
part of the map hash: the same map can be played in every weather. The map
editor sets them in its Save dialog and writes them only when they are not
`clear` / `day`. Rules: [ARCHITECTURE.md](ARCHITECTURE.md#weather-mud-and-time-of-day).

### Objects

Trees, buildings, hedges, walls and props that matter for the game (they block
movement and, per `data/vision.toml`, line of sight; since prompt 020 they can be
destroyed, see [ARCHITECTURE.md](ARCHITECTURE.md#destruction)) are **map objects**
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
by default the catalog size). Variants may override both, and their height (`height_mm`, for sight and
projectiles; rocks, prompt 074). These are game rules:

| Objects | Blocks | Shape |
|---|---|---|
| Trees (deciduous, conifer, birch, palm) | large | trunk circle 0.3–0.5 m |
| Hedges, dry-stone walls, large rocks, hedgehogs | large | catalog box / circle |
| Houses, barns, church (intact and damaged), bunker | all | catalog box |
| Ruins of houses, barns and the church (variant `Ruin`, prompt 020) | large | catalog box |
| Barbed wire | small | catalog box |
| Fences, bushes, hay, props, stumps, logs, poles, medium/small rocks, trenches | none | – |
| Rock outcrops, rock ridge pieces (prompt 074) | all | box per variant, 0.4 m larger than the model's foot; own `height_mm` per variant |
| Rock slabs (prompt 074) | none | box per variant (the model) |
| Apartment blocks `env_block_desert`, `env_block_desert_long`, `env_block_town` (prompt 078) | all (ruin: large) | catalog box (24 × 12, 40 × 13, 30 × 13 m) |

Woods, hedgerows and walls are closed to tanks and open to infantry; tanks use
gaps and tracks. Since prompt 093 medium and heavy tanks break through mud and
dry-stone walls, since prompt 106 they push trees over by trunk thickness
(young birch and spruce from 18 t; young oak, mature birch and palms from
40 t; mature oak, beech, spruce and pine never), but never trees in a dense
stand (6 other trees within 10 m: the woods of the generated maps). A map
that wants a tree row as a barrier for tanks plants mature trees or plants it
dense enough; a wood should keep 6.5 m spacing or less.

`just map-reach <map>` reports, below the usual numbers, the ground a
Panzer IV H (25 t) and a Tiger I (57 t) open by breaking walls and trees
(magenta in the `--png` picture). All 26 maps on 2026-11-21 (prompt 106):
the Panzer IV gains 0.0-1.8 points of the land (`desert_village` 3.0: its
mud walls), the Tiger 0.2-3.0 (`desert_village` 4.2, palms and walls). Trees join no
start, supply point or marker to the main part that was cut off before:
outside it stay `harbor_city` (harbor, beach_east: in the water, as in the
plain numbers) and `rock_hills` (bridge_wood: the wooden bridge carries
less than these tanks; the plain numbers ignore bridge loads); the police
post and fort of `desert_village`, separate in the plain numbers, join
through their mud walls (prompt 093). In woods the Tiger connects some open spots between the trunks
(edge trees and thinner spots), so the largest part grows by up to 6
points (`river_fields` 83.6 → 89.6 %, `river_fork` 85.7 → 91.1 %,
`island_chain` 72.1 → 76.0 %); at 40 m of detour per felled cell a path
through a wood stays far dearer than the way round. No map loses a tree
row that was meant as a barrier (the generators plant rows 9 m apart or
more, which tanks drove between before). The missions use tanks below
40 t and rely on no tree row; the tutorial's "wood" stays a wood. Loading resolves every line against the rules; unknown kinds or
variants, objects outside the map, bad numbers and headings are errors with the line
number. The kinds a map uses and their resolved shapes become part of the map hash,
so changing a rule changes the hash of every map using that kind (old replays then
report the different map instead of desyncing).

**Damage stages** (prompt 020, `data/destruction.toml`): a placed `Damaged`
variant starts damaged, a `Ruin` starts destroyed and acts as a ruin (blocks
tanks only, stops projectiles up to 40 % of its height, blocks no sight); the
`blocks` of the `Ruin` variant in `data/objects.toml` is overridden by the
destruction rules (it stays in the map hash). During a match objects lose hit
points to gunfire and change their stage; the map itself never changes.

Bridges are written first, then everything else.

`objects.csv` is plain text so it can be edited in any tool and diffed, and lives in
Git LFS like the PNGs (`test_large`: 98 000 lines, 4.2 MB). A compact binary format
would only pay off far beyond that; parsing takes about 50 ms.

Hidden minefields (prompt 035): `fort_mine_at` and `fort_mine_ap` (variant
`Mine`, blocks nothing) become a 20 m × 20 m section of neutral, hidden
mines each when a match starts (see ARCHITECTURE.md "Field works"); matches
do not draw them, the editor does. `twin_meadow` has one of each in the
middle.

Strongpoints (prompt 052): a marker named `strongpoint_<name>` (in
`map.toml`, `[[marker]]`) becomes a neutral trench strongpoint in the
first tick of a match: a zigzag fire trench ring (16 m), four MG nests,
communication trenches to two dugouts and a wire ring (30 m) with two
gaps (ARCHITECTURE.md "Positions"). Put it on a supply point marker
(`supply_*`) to fortify the flag. The map editor places and removes them
with the "Strongpoint" tool. [provisional]

### Bridges

Bridges (prompt 014) are map objects whose kind has a **deck** rule in
`data/objects.toml`: roadway width, load limit and the terrain whose speed
applies on the deck.

| Kind | Variants (length) | Roadway | Load limit |
|---|---|---|---|
| `env_bridge_wood` (single-lane timber) | `Span8` (8 m), `Span12` (12 m), `Span16` (16 m) | 4.1 m | 16 t |
| `env_bridge_stone` (stone arch) | `Arch1`, `Arch2` (20 m), `Arch3` (30 m) | 5.1 m | 100 t |

The load limits are game rules: the timber bridge carries trucks, half-tracks,
armored cars and light tanks, but no Panzer IV, T-34 or Sherman. The catalog
mass (`mass_kg`, `sim::UnitType::mass_kg`) decides; infantry has none.

- **Deck:** a box along the object's heading, as long as the variant and as
  wide as the roadway. Its height runs linearly between the ground heights at
  both ends of the axis (computed when the objects are loaded), so it always
  meets the banks; the map must bring the banks (ramps) up to the height the
  bridge should have. `Map::height_at` and `Map::terrain_at` report the deck
  (never below the ground) wherever a point lies on an intact deck;
  `ground_height_at` / `ground_terrain_at` give the raw values.
- **Railings:** units get on or off a deck only over its ends, or sideways
  where the ground is within 0.5 m of the deck (abutments)
  (`Map::deck_step_ok`); paths enter and leave deck cells only along the
  axis.
- **Path grid:** a path cell whose center lies on an intact deck is open for
  every unit the bridge carries (no steepness, no margin, the deck terrain's
  cost) and closed for heavier ones. Water under a deck does not count for the
  2 m margin of tanks. Loading checks that the cells along the axis (except
  the last half cell at each end) have their centers on the deck: the axis
  must run along the 4 m path grid, close to a line of cell centers
  (`x` or `y` = `-half + 1 + 4 k` m on 2 m maps), otherwise the bridge is
  rejected with its position. Diagonal bridges are not supported.
- **Placing one:** span the water plus about 4 m of bank on each side (so the
  cells beyond the deck are clear of the tank margin), heading a multiple of
  90°, axis on a cell center line, banks raised to the wanted deck height
  with ramps of at most ~12 % (the generator does all of this).
- **Destruction** (prepared, prompt 020): `Sim::set_bridge_intact(index,
  false)` removes the deck (the map is copied once, copy-on-write), the path
  grid around it is rebuilt and units whose route uses it plan again; `true`
  restores it. The state of every bridge is part of the state hash.

The renderer places the model so that its deck (glTF extra `deck_height_m`)
lies at the simulation's deck height in the middle of the bridge.

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
| Objects (prompts 013, 014) | 98 462 (95 619 blocking, 36 kinds, 4 bridges); `objects.csv` 4.2 MB, 2.4 MB in memory, parse + resolve + hash 52 ms |
| `height_at` | 35–40 ns per lookup (random points; 2 lookups per unit and tick) |
| Godot start incl. map and debug mesh | 1.2 s (debug bridge, headless) |

`test_small` (500 m): 63 001 samples, 0.06 MB files, 0.3 MB memory, 1.4 ms to load;
1 444 objects including 2 bridges (2.7 ms).

If 8 km maps ever come: 80 MB with this layout. Then store heights as `u16` + base and
step in memory too (half), or as tiles; prompt 016 covers large maps.

## Generator

`cli gen-map` (`crates/cli/src/gen_map.rs`) builds a map from a seed:

- **Ground:** three octaves of value noise (3 m, 1 m, 0.15 m amplitude).
- **Hills:** smooth bumps `h · (1 − d²/r²)²` with a steepest grade of about
  `1.54 · h / r`, kept below ~35 %. One landmark hill north-west of the meadow
  (marker `hill_top`); more hills per area on larger maps, away from the meadow and
  the river.
- **River bed:** a wobbly line roughly north–south in the east. Gentle banks
  down to 3 m below the ground over 40 m, then the water: shallow water down
  to 0.5 m below the waterline (0.15 m below the banks), a deep channel down
  to 1.8 m in the middle (none in fords); sand banks and mud patches beside
  it (prompt 014; before, the bed was nearly flat 3.5 m down).
- **Bridges** (prompt 014, own random stream): one crossing per started
  kilometer of river, at least two, spread along the river and away from the
  ford of the east track, alternating stone and timber. At each crossing the
  river runs straight for ±30 m and narrows (timber: 7 m of water, stone:
  11 m, on wide rivers 22 m); the bridge axis runs east–west on a path cell
  center line; the shortest variant spanning the water plus 4 m on each side
  is used (timber falls back to stone if none fits). Embankments raise the
  banks to the deck (timber 1.5 m, stone 3 m above the bank) with 12 % ramps
  and 50 % sides; a field track leads from the meadow over the bridge to the
  east edge. Markers `bridge_stone`, `bridge_wood` and `ford`.
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
  - **Stones in the river** (small and medium rocks, blocking nobody), groups
    every 7 m along the river with a 45 % chance, not within 25 m of bridges
    and fords; they make foam (prompt 014). Placed last, so all other objects
    stay as before.
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
just gen-map skirmish 2400 21 1200   # the default match map (starts 1.2 km apart)
just gen-mountains             # mountain_pass (prompt 060, --relief mountains)
```

**Start distance.** By default both starts lie 60 m north and south of the
center, inside the central meadow; the test maps keep that, because tests,
benchmarks and the demo battle are built on it. `--start-distance-m D`
(fourth argument of `just gen-map`) puts the starts `D/2` north and south
of the center instead and flattens a meadow of their own around each (at most
100 m radius, at its natural height); hills, villages and objects keep clear
of it. `skirmish` (2.4 km, starts 1.2 km apart) is preselected in the setup
screens and lobbies, so the first contact takes a few minutes instead of
opening fire at once.

Regenerating `test_small` changes the golden hash: that is a rule change in the sense
of [DETERMINISM.md](DETERMINISM.md) and needs to be named in the commit. A test
generates a small map twice and compares `objects.csv`; another checks that every
variant in `data/objects.toml` exists in the model's blueprint.

### Relief profiles and mountains (prompt 060)

`--relief gentle|hilly|mountains` (default `gentle`) picks the terrain
profile (`crates/cli/src/gen_map/relief.rs`). The layout (meadows, river,
tracks, villages, bridges) and the objects' random stream stay the same;
`gentle` gives exactly the files of before (the test maps regenerate
bit-identically, checked).

- **hilly:** ground noise and hills 1.7 times as high (steepest hill grade
  just under the 60 % limit).
- **mountains:** ridged noise (crests where value noise crosses zero, four
  octaves 700/300/120/50 m, squared so foothills stay gentle) under a
  large-scale massif noise, up to 600 m at the very top (`mountain_pass`:
  about 330 m above the pass). Every track, road and the river runs in a
  valley: within 22 m of a track (the river: its width plus 50 m) the
  ground stays at the level of the central meadow, and the mountains rise
  to full height over 220 m. Where a track crosses the range it is a
  **pass**. Starts far from the center (`--start-distance-m`) lie in
  **basins** 30 m below the passes (flat 210 m around the start, rising
  over 230 m). **Cliff bands** where a cliff noise is high: the mountain
  height is quantized into 24 m benches with risers over the last fifth of
  each step (rock walls, impassable). A **serpentine** climbs from the
  west rim of the central meadow: six legs of 100 m, 26 m apart, joined by
  hairpins (radius 13 m), an 11 % grade, a 12 m flat bed (gravel road 6 m
  wide) with 120 % cuttings and embankments (rock walls) beside it, cut
  into a mountainside 30 m above its ramp; at the top a plateau of 24 m
  radius (marker `lookout`, supply point `supply_lookout`, about 85 m above
  the pass). Mud only on slopes under 20 %. Objects: woods above 25 m are
  conifer stands, no trees above 200 m (tree line) or on slopes over 60 %,
  boulders on scree slopes (35–60 %).

`mountain_pass` (`just gen-mountains`: 2 km, seed 60, starts 1.2 km apart,
four starts for 2 or 4 players, forest): two basins north and south with
the starts, the central meadow on the pass between them, the river in a
valley in the east with two bridges and a ford, the serpentine to the
lookout in the west. 42 % of the map is under 20 % grade, 32 % steeper than
the 60 % limit. All supply points are reachable (map-check), and
`ai-tournament --map mountain_pass` plays it (1v1 and 2v2).

Measurements (`just map-info mountain_pass`, release, machine under load
of other sessions): 1 001 × 1 001 samples, heights −32 … 329 m, height step
6 mm (16-bit PNG; 500 m of relief would need 8 mm, still finer than any
rule needs), files 1.6 MB, 5.0 MB in memory, decode + validate + hash
19 ms, 14 818 objects. Path bench (200 single orders, ±1800 m): 212
requests, 46 000 expansions each, planning ticks 5.4 ms mean / 8.2 ms p99
(best of three runs; `test_large` in the same session 7.6 ms mean). Vision
bench (500 units): 250 000 ray steps per tick, 7.3 ms mean per tick
(`test_large`: 208 000, 7.7 ms). Terrain bench: near 123 FPS, middle 125,
overview 102, river 55 FPS (`test_large` in the same session 106 / 105 /
86 / 47 FPS; the machine was busy, so absolute rates are low).

**Rules on slopes** (unchanged, decided here): one grade limit (60 %) and
one uphill penalty for every unit; infantry slows uphill like vehicles.
Per-vehicle climbing ability and wheels versus tracks are not modeled: the
catalog has no sourced climbing values, and a grade limit per vehicle would
need one passability and component grid per limit [provisional]. Slopes
give cover through the line of sight (reverse slopes, crests). New is the
downhill sight bonus of heights
([ARCHITECTURE.md](ARCHITECTURE.md#viewsheds)).

**Presentation:** on maps with more than 120–220 m of relief
(`terrain.gd`, from the chunk height ranges) the terrain shader keeps grass
on steeper slopes (bare earth from about 42 %, rock from about 62 %), draws
strata on rock walls, scree between earth and rock, and snow above 80 % of
the height range (45 % in winter, none in the desert) on all but the
steepest rock [provisional]. The camera already keeps its focus on the
ground and stays above hillsides. `SHOT_SET=mountains just terrain-shots
<dir> mountain_pass` records views of the pass, the serpentine, the
lookout, a basin and the range.

Open: fog in valleys, contour lines on the minimap, a serpentine on other
than the west side or more than one, rivers that follow valley floors
downhill (the river still runs north–south through the range).

### Grown rock (prompt 074)

`gen-map --rocks` places grown rock in a random stream of its own (the other
objects stay as they are), last, on open ground only (`Placer::open`: away
from ways, water, starts, villages; the whole box): ridges of overlapping
ridge pieces along the contour where the grade (|dx| + |dy|) exceeds 0.48,
sometimes a second piece beside, with conifers and bushes between them;
outcrops on slopes of grade 0.2–0.48 with bushes and stones at the foot;
slab fields (6–13 slabs) on gentle ground where a 120 m noise is high.
`just gen-rock-hills` writes **`rock_hills`** (1200 m, seed 74, desert,
mountains, about 260 ridge pieces, 85 outcrops, 230 slabs). No test,
tournament, balance or benchmark series uses it; the maps of those series
got no rocks.

### Small town (prompt 041)

`cli gen-town` (`crates/cli/src/gen_town.rs`, `just gen-town`) writes
`data/maps/town_small`: 600 m, gentle bumps flattened towards the town, a
paved main street (east–west) and cross street (north–south, start to
start, 8 m wide) meeting at a 48 m market square; the town hall and the
church at the square, two warehouses behind it, terraces of four town houses
(12 m frontage, 3 m from the street, doors to the street, 8 m alleys)
along both streets, gardens with hay ricks and firewood behind them, six
farms (farmhouse, barn, hay) on the outskirts with dirt tracks, a conifer
wood in the west split by the main street (a firebreak for fire, prompt
041), broadleaf and birch woods in the east, two orchards, and a supply
dump (two ammunition stacks, two fuel dumps, 14 m apart) in front of each
start. Starts at (0, ±250); markers `town_square`, `west_wood`,
`dump_south`, `dump_north` and the supply points `supply_square`,
`supply_west`, `supply_east`. Objects are placed without overlaps and off
the streets. No real place names. A test generates it and checks its
buildings, dumps and woods; `just map-check town_small` passes.

### Desert village (prompt 059)

`cli gen-village` (`crates/cli/src/gen_village.rs`, `just gen-village`)
writes `data/maps/desert_village`: 800 m, biome `desert`, four starts in
two teams (team 0 south at (∓90, 330), team 1 north; a two-player match
uses players 0 and 1, diagonally opposite). Low dunes with loose sand on
the crests, flat ground in the village, at the oasis and the post. A
graded piste (road) runs north–south through the village square (48 m,
packed earth) with the mosque on its west side, four shop rows with
market stalls and a well; about 130 walled courtyard compounds fill the
village (radius 165 m), placed by the editor's village brush. A field
track leads west to an **oasis** (a pond of shallow water in a 2.4 m
hollow, sand banks, a dense palm grove, bushes, a well and a gardener's
yard) and east to the walled **police post** (fort). Palms stand in the
yards and lanes, thorn bushes and rocks in the open. Markers `village`,
`oasis`, `police_post` and the supply points `supply_square`,
`supply_well`, `supply_fort`. A test generates it twice (same files) and
checks starts, compounds, landmarks and palms; `just map-check
desert_village` passes. No real place names.

**Compounds** (`data/compounds.toml`, `editor::compound`): a compound is a
group of ordinary map objects – a house with its walled yard, gate and yard
props, or a landmark (mosque with a low wall, police post with its yard,
shop row with stalls). The yard wall is laid out from the yard size in
3 m segments with corner piers and one gate segment; pieces inside the
compound's buildings are left out (the house wall closes the yard).
Stamps turn in steps of 90°. The simulation never sees a compound, so
compounds are not a rule and not hashed. The **village brush** packs the
compounds marked `village` into its circle in rows: 2–4 m alleys between
the yards of a row (infantry; mud walls stop vehicles, not infantry) and
8 m lanes between the rows (tanks), random compounds and turns (gates
mostly to the lanes), houses 10 % damaged and 5 % ruined, 8 % of the wall
segments broken; never on tracks, roads or water or over existing objects;
deterministic from the editor's seed.

### Apartment block quarter (prompt 078)

`cli gen-blocks` (`crates/cli/src/gen_blocks.rs`, `just gen-blocks`)
writes `data/maps/city_blocks`: 400 m, biome `desert`, flat in the
quarter. Two east-west streets (y = ±44 m) and a north-south cross street
(x = 0), all 16 m wide (four 4 m path cells: two tanks pass each other),
packed earth (`dirt_track`). Eleven blocks face the streets with their
entrance side: in the middle band long blocks and blocks back to back
with a walled courtyard between them (mud-brick walls with 6 m gates, two
low houses, a well, palms), a terrace of two tenements (`env_block_town`,
gable to gable) with two blocks behind them, and an outer row on each
side; gaps of at least 12 m between blocks lead into the courtyards.
Starts at (0, ±165), markers `blocks` (quarter center) and `town_square`.
A test generates it and checks with the path grid that tanks reach both
ends of both streets and every courtyard and infantry every courtyard;
`just map-check city_blocks` passes. No tests, AI tournaments or
benchmarks use this map. Pictures: `just block-shots`, `just
block-fight`.

**For map generators (prompts 079, 081):** catalog ids and sizes
(length along the heading × width, the entrance side is the model's +X):

| id | Box (m) | Height | Entrances | Notes |
|---|---|---|---|---|
| `env_block_desert` | 24 × 12 | 14.2 m | one, +X, canopy 1.2 m | flat roof; the canopy and a 0.3 m apron are inside the box |
| `env_block_desert_long` | 40 × 13 | 17.1 m | two, +X, canopies | balconies on −X inside the box |
| `env_block_town` | 30 × 13 | 20.0 m | two doors and a carriage passage (middle, 3 m) on +X, the passage runs through to −X | blind gables at ±Y: stand in rows gable to gable (centers one length apart) |

- **Rotation:** heading `h` turns the entrance side (+X) to the map
  direction `(cos h, sin h)` (`MapObject::axes`, right axis): 0 = east,
  90 = south (+y), 180 = west, 270 = north. Put blocks parallel to the
  streets, entrance to the street, the box 1-4 m from the street edge.
- **Variants:** `Intact`, `Damaged`, `Ruin` (the destruction stages; a
  placed `Damaged` starts damaged, `Ruin` destroyed). Same box for all.
- **Paths:** the box blocks everybody, so leave gaps of at least 8 m (two
  path cells) for infantry and 12 m for tanks between blocks to reach
  courtyards; the carriage passage of `env_block_town` is a picture, not
  a way through (the box is closed).
- **Occupation:** capacity 24 / 36 / 36 soldiers (`data/infantry.toml`),
  so a block holds two to four squads.
- **Budget:** 3 000-4 800 triangles per block and about 20 draw calls; a
  city of 100 blocks adds about 0.4 M triangles in view (see
  ARCHITECTURE.md, measurements).

### Large city (prompt 079)

`cli gen-city` (`crates/cli/src/gen_city.rs`, `just gen-city`, options
`--seed`, `--biome`, `--name`, `--out`) writes `data/maps/city_large`:
3 km × 3 km, biome `winter` with default weather `snow` (`--biome forest`
or `desert` gives the same layout without snow drifts, with a few mud
patches instead), four starts in two teams (team 0 south at
(∓250, 1100) local, team 1 north; a two-player match uses players 0 and
1, diagonally opposite, 2.26 km apart). No real place names.

**Symmetry.** Street network, squares, supply points, strongpoints,
dumps and starts are point-symmetric about `CENTER` = (1, 1) m, the
center of the path cell next to the map center (path cell center lines
lie at `-1499 + 4 k` m on this map), so every street axis lies on a path
cell center line in both halves. All coordinates below are local, around
that point. Building slots are mirrored too (the same footprint turned
by 180°); building kinds of the two square landmarks (church south, town
hall north), damage stages (drawn separately per half, the same count),
courtyard contents, houses, gardens and woods differ.

| Zone | Extent | Content |
|---|---|---|
| Center | the four cells around the square | paved square 80 × 60 m where the two main streets cross; church (south-west) and town hall (north-east) facing it; the largest blocks (40 m long) along the main streets |
| Block quarter | to 360 m (Chebyshev) | street grid of 120 m: main streets on the lines 0 and ±240 (16 m wide), side streets on ±120 and ±360 (12 m); each 106 m cell is a ring (one block per side, open corners, a small block in the courtyard), rows (three rows of blocks about 25-30 m apart), an angle (an L of two blocks, two single ones, one in the middle) or a terrace (three tenements gable to gable, two opposite, one between), with trees, a shed, firewood, crates and now and then a stone wall in the courtyard; town houses with gardens in the corner cells; per half a park (trees on a 13 m grid, a cross of paths) and a warehouse quarter (four warehouses, an ammunition and a fuel dump, crates and barrels); trees every 16 m along the main streets |
| Suburb | to 660 m, houses within 780 m of the center | 12 m streets: the grid lines continued, a square ring at 520 m and a chamfered ring at 660 m; farm, brick, stone and log houses on plots with gardens (firewood or hay, bare fruit trees, a wooden fence), houses facing the block quarter's edge streets, orchards between the streets |
| Land | to the edge | gentle hills and value noise (relief about 14 m; flat within 780 m of the center, blending in over 240 m), snow drifts (terrain `snow`) in the open, the two main streets as 10 m country roads to all four edges, the avenues ±240 as 8 m roads to the bypass, an 8 m bypass ring at 950 m (corners cut at 700 m), field tracks with hedgerows, three farmsteads per half, haystacks, woods mostly beyond 1 km, single trees |

**Routes.** Three independent north-south routes through the city (the
main streets −240, 0, +240, each 16 m and straight from the suburb ring to
the suburb ring) plus the bypass west and east; the same east-west. A
test checks that tanks reach the north start from the south start along
each of the five corridors alone (everything else in the middle band
closed), that every grid crossing is reachable for tanks and every cell
of the block quarter for infantry. Blocks stand 4 m behind the street
edge, so opposite fronts are 20 m (side streets) or 24 m (main streets)
apart; within a cell blocks keep 20-30 m and corner openings of at least
12 m.

**Supply points** (8): `supply_square_south` and `supply_square_north`
both on the square (holding the square pays twice: the most valuable
place without a new rule), and per half `supply_warehouses_*` (the
warehouse yard), `supply_crossing_*` (avenue × inner suburb ring) and
`supply_farm_*` (a farmstead between suburb and bypass). Markers for
tools and shots: `town_square`, `park_*`, `warehouses_*`, `crossing_*`,
`farm_*`, `strongpoint_*`, `block_edge_*`, `suburb_*`, `main_street_*`,
`side_street_*`, `start_*`, `bypass_west`, `bypass_east`, `fire_start`
(a ring cell for `just fire-bench 40 6000 city_large`).

**Starts:** 60 m flat and free around each start, a field track to the
bypass and one joining the two starts of a team; a supply dump (two
ammunition stacks, two fuel dumps, 14 m apart) 85 m in front of each,
as on `town_small`.

**War damage:** about every eighth city building (blocks, church, town
hall, warehouses) starts `Damaged` and every twentieth as a `Ruin`, the
same number in both halves; houses 10-15 % damaged or ruined. A trench
strongpoint (`strongpoint_south` / `_north`, prompt 052) on open ground
between suburb and bypass is the ring-shaped earthwork. Craters cannot
be a starting state of a map (they are made by shells in a match).

A test generates the map twice (bit-identical files) and checks the
counts (at least 100 blocks and 300 houses, one church, one town hall,
eight warehouses, eight supply points), the symmetry of the street
network, starts and supply points, equal damage per half and the routes.
`just map-check city_large` passes. Pictures: `SHOT_SET=land just
terrain-shots <dir> city_large` adds zone, start and street views to the
supply point views (`city_compare` is the view of the block quarter with
the suburb behind it). Measurements: [PERFORMANCE.md](PERFORMANCE.md#large-city-prompt-079).

Open: a summer version (`--biome forest`, a second run), a desert city of
the flat-roofed blocks, a river with bridges, a station and tracks, a
factory site, new buildings (school, cinema, water tower), street
fighting as a rule (barricades, sewers, fighting inside a house, storeys),
a shot of this map in the demo video, a picture preview of maps in the
setup screen (no map has one).

### Maps from real geodata (prompt 081)

`tools/geo_import.py` (Python 3, standard library only) builds a map in
the format above from an OpenStreetMap extract and a free elevation model.
`just gen-geo harbor_city` writes `data/maps/harbor_city/` (and
`just gen-geo brick_quarter` the second site, prompt 112);
`just gen-geo harbor_city --report DIR` also writes pictures of the stages
(`geodata.png` the OSM data before simplification, `stage1.png` heights
and terrain, `stage1_grid.png` with 100 m lines, starts and markers,
`stage2.png` with buildings and trees). `just gen-geo-check` (part of
`just check` and CI) generates the map twice in two processes, compares
the files byte for byte and their content with the committed map. One run
takes about a minute.

**A site** is a folder `assets-src/geo/<site>/` with three files:

| File | Content |
|---|---|
| `site.json` | the description: sources (bounding box, Overpass query, URLs, retrieval dates), frame (center, bearing of the map's x axis, size), elevation filters, coast, road classes and widths, areas, buildings (catalog classes, districts with estimated storeys, limits), green, starts with their pads, markers |
| `osm.json.gz` | the OSM extract reduced by `geo_import.py extract`: geometry in 1e-7 degrees and only the tags the importer reads (`building`, `building:levels`, `height`, `highway`, `tunnel`, `bridge`, `layer`, `covered`, `oneway`, `area`, `natural`, `landuse`, `leisure`, `man_made`, `floating`, `water`, `intermittent`, `junction`, `start_date` (since prompt 112), and `amenity` for places of worship and town halls). Names, addresses, house numbers, ids and every other attribute are dropped when reading. |
| `dem.json.gz` | a crop of the elevation model in centimeters with its georeference |

Both data files are small (`harbor_city`: 1.4 MB and 80 KB;
`brick_quarter`: 2.1 MB and 46 KB) and live in Git LFS, so the
map can be regenerated without a network. To fetch them again (or for a
new site): run the Overpass query of `site.json` once against
`osm_endpoint` (any HTTP client, no personal data in the request), download
the Copernicus DEM tile of `dem_url`, then
`python3 -I tools/geo_import.py extract assets-src/geo/<site>/site.json --osm RAW.json --dem RAW.tif`
(several answers and neighbouring tiles may be given: `--osm A.json B.json --dem T1.tif T2.tif`).
Keep the raw downloads outside the repository (`build/`).

**Determinism:** floats only with `+ - * /` and `sqrt` (exactly rounded),
own series for sine, cosine and arctangent, a fixed random stream
(`xorshift64*`), fixed iteration orders, integer output. PNGs are
compressed with zlib; the check compares their decompressed content, so a
different zlib build cannot fail it.

**Pipeline** (stage numbers of prompt 081):

1. *Frame:* an equirectangular projection around the center with the
   WGS 84 meters per degree, turned so the map's +x axis points to
   `x_axis_bearing_deg` (`harbor_city`: 37°, along the coast, sea in the
   south); 2 m samples.
2. *Elevation:* the Copernicus DEM is a surface model (roofs, tree tops).
   Per DEM pixel the mean height of the OSM buildings over its ground is
   subtracted (`height`, else `building:levels` × 3.2 m + 1 m, else
   15 m), then a 3 × 3 median and a grey opening (5 × 5 pixels, keeping
   the surface within 3 m of the opened ground) remove what is left of
   buildings and trees; bilinear to 2 m and two box blurs of 18 m.
3. *Coast:* coastline ways as walls, flood fill from `sea_seeds_m`; solid
   piers, breakwaters and quays (areas, not floating) added as land; water
   and land narrower than 8 m removed (morphological opening: moles,
   jetties, slots), basins cut off by that closed, a majority pass smooths
   the edge. Sea bed by rings from the shore: -0.8 m next to land, 0.35 m
   deeper per ring, at most -6 m; terrain `deep_water`, a seam of
   `shallow_water` (3 rings) and `sand` only at beaches; land next to the
   water at least 1.2 m (quays: an edge), all land at least 1 m.
4. *Roads:* the carrying network: motorway to residential and living
   streets (8–14 m by class, `road`) and tracks (6 m, `dirt_track`);
   tunnels, covered ways and ways below ground, steps, foot, cycle and
   pedestrian ways, service roads (drives, parking aisles) and the race
   circuit lines are dropped; a bridge that crosses another road is dropped
   (the lower road stays); dead ends under 25 m and fragments under 80 m
   go; lines simplified (Douglas-Peucker, 4 m). On the ground: every
   sample takes the height of the nearest center line (which follows the
   blurred ground along the road), so the bed is level across; parallel
   carriageways and junctions are closed into one surface (gaps up to
   12 m); the bed is smoothed over road samples only; banks of 40 % beside
   it reach 14 m, and where the slope is steeper the last 4 m blend back
   into it. Pedestrian squares are painted `road` without levelling.
5. *Pads* (starts): flat at their mean height within 34 m, blended over
   30 m.
6. *Areas:* only areas of at least 1 500 m²: beaches `sand`, scrub and
   heath `tall_grass`.
7. *Buildings:* footprints (without `roof`, `construction` and
   underground) are rasterized; touching ones are merged into blocks
   (8-connected), sports pitches and stadium fields stay free. Blocks
   smaller than the first step of `min_area_steps_m2` are dropped; the
   step rises until at most `max_buildings` stand. Per block: the main
   direction from the length-weighted edge directions (modulo 90°), the
   height from the tallest tagged footprint or the district's estimate
   (`levels` for small and large blocks), a class by height (tower ≥ 24 m,
   block ≥ 14 m, town ≥ 9 m, villa) or by type (church, public building,
   industrial or port ground), then rows across the block (8 m apart) filled
   along its length with the class's catalog buildings (largest first). A
   box must cover the block to 45 %, keep 4 m (one path cell) off roads,
   so no building narrows a road for tanks, stay off water, other
   buildings and reserved places (80 m around starts, 12 m around supply
   points) and stand on at most 10 m of fall; otherwise it is shifted
   across (±2, ±4 m), taken smaller or the row moves on. 2 % of the
   buildings are ruins, 6 % damaged.
8. *Building pads:* where the corners of a house differ by more than
   0.6 m its ground (box + 0.5 m) is levelled at the center height,
   without cut or fill around it; a pad that would cut a way (the open path
   cells within 12 m must stay connected as before, on the heights as
   stored) is left out. Then every building that cuts off a part of the
   ground that was connected before (at least 500 path cells) is removed.
   The check emulates the simulation's path grid (`sim::path`: 4 m cells,
   |dx| + |dy| over the height patches, 60 %), so it agrees with
   `map-check`.
9. *Paving:* town ground within 6 m of the buildings becomes `road`
   (districts marked `garden` keep their grass).
10. *Green:* woods 8 m apart (pines, oaks), parks 14 m (palms,
    broadleaf, pines, bushes), scrub bushes and pines 18 m, tree rows a
    tree every 9 m (palms in the lower town), hedges along gardens and
    parks in runs of 30 m with 9 m gaps; trunks keep 1.5 m off roads,
    buildings and water.

Not in the game and therefore dropped: tunnels, bridges and roads above
roads, steps and lanes under 8 m (vehicles; infantry walks the open
ground), floating pontoons, ships, cranes; real building shapes (catalog
models with fixed sizes stand in), landmarks and modern buildings (towers,
stadium stand, halls). See prompt 081, section 11, for what would be
needed.

**Reachability** of any map: `just map-reach <name> [picture.png]` (`cli
map-reach`) uses the simulation's path grid and prints the land open to
vehicles, to infantry only and to nobody (too steep or edges, buildings),
the connected parts, the part every start and marker lies in, and the
path lengths of a tank and a rifle squad from every start to every supply
point; the picture has one pixel per 4 m cell.

**A second site:** copy `site.json`, set the bounding box, query and
frame, fetch and extract the sources, set seeds and districts, then adjust
starts, pads and markers with the `--report` pictures (`stage1_grid.png`)
and `just map-reach` until `just map-check` passes. Values in `site.json`
are guide values of the site, not rules.

#### `harbor_city` ("Hafenstadt")

3.6 km × 2.4 km (1 801 × 1 201 samples), biome `forest` (there is no
Mediterranean biome), heights -6 … 673 m (11 mm steps). The terrain,
coast, harbors and the road network follow today's place named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-prompt-081) (the name is
not used in the game): the rock with the old town between two harbors,
the flat reclaimed land in the west, the town rising up the slope to the
hinterland. Four starts in two teams at the ends of the coastal strip
(team 0 west at (-1470, 540) and (-1290, 800), team 1 east at
(1560, 720) and (1320, 470)), each levelled 34 m around with 80 m kept
free of buildings and trees.

What the simplification kept (`just gen-geo harbor_city`, stage pictures
with `--report`):

| Data | OSM in the map | In the game |
|---|---|---|
| Roads | 235.5 km of ways | 82.3 km of 878 roads and tracks (2 835 instead of 8 233 points); dropped: 85.5 km foot, cycle and pedestrian ways, 34.6 km service roads, 19.8 km in tunnels or covered, 10.2 km of steps, 1.0 km race circuit lines, 0.4 km bridges over roads, 0.7 km short dead ends, 0.2 km fragments; parallel carriageways and junctions closed (7.3 ha) |
| Coast | 643 coastline points | 7 271 coast samples (2 m); narrow water and land removed, net 0.2 ha more land |
| Buildings | 5 862 footprints | 2 842 blocks after merging; 397 of at least 700 m² got 1 455 buildings (659 shifted, 564 taken smaller, 48 removed to keep ways open, 21 blocks without room); 1 224 levelled pads, 61 left on the slope |
| Areas | 291 land use, leisure and natural areas of the kinds read | 146 of at least 1 500 m² (51 parks and gardens, 47 woods, 17 sports grounds, 12 scrub, 12 port or industrial, 7 beaches) and 13 squares; 76.6 ha of town ground paved |
| Green | – | 9 633 trees, 500 palms, 3 741 bushes, 1 354 hedge pieces |

Buildings per district (`site.json`, `districts`; blocks / buildings /
removed): rock 18 / 183 / 0, west flats 50 / 163 / 0, harbor 18 / 66 / 0,
east shore 77 / 348 / 0, slope town 189 / 474 / 5, hinterland 45 / 221 /
43. Kinds: 534 town houses, 331 tenement blocks, 120 + 56 flat-roofed
blocks, 271 + 62 flat houses, 36 stone and 27 brick houses, 10 churches,
6 town halls, 2 warehouses.

Reachability (`just map-reach harbor_city`): of 7.0 km² of land 56 % is
open to vehicles and infantry, 2.7 % to infantry only (woods, hedges),
33.7 % to nobody (steeper than 60 %, quay edges) and 7.6 % is covered by
buildings; 82 % of the open ground hangs together, the rest are terraces
and walled-in plots. Every start and supply point lies in that main part
for both classes.

Supply points in pairs, balanced by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | West | East |
|---|---|---|---|
| `supply_harbor_west` / `supply_harbor_east` | quay on each side of the main harbor | 1 282 / 1 164 | 1 265 / 1 223 |
| `supply_rock` / `supply_upper_east` | square on the rock / upper town east | 1 610 / 1 328 | 1 439 / 1 401 |
| `supply_west_hill` / `supply_square_east` | hill above the west flats / square in the east town | 1 075 / 901 | 1 127 / 1 116 |
| `supply_hinterland` | hinterland above the town (contested) | 2 996 / 2 675 | 2 498 / 2 370 |

Every point lies where vehicles get to. The hinterland is reached over
the east side (about 500 m shorter for vehicles, 300 m for infantry);
over all seven points the east is about 630 m shorter by vehicle and the
west about 40 m shorter on foot [provisional]. There is a vehicle route
between the sides along the coast and through the town. Markers for tools
and shots: `harbor`, `rock`, `west_flats`, `square_east`, `beach_east`,
`upper_west`, `upper_east`, `west_hill`, `hinterland`, `start_west`,
`start_east`, `blocks` (for `just objects-bench harbor_city
--blocks-bench`). Pictures: `SHOT_SET=land just terrain-shots <dir>
harbor_city` adds comparison views from the sea, the quarters, houses on
the slope and a harbor street to the supply point views. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#harbor-city-prompt-081).

Open: a Mediterranean biome (pines, cypresses, light houses, retaining
walls, terraces), buildings shaped from footprints, landmark models,
tunnels and stairs, ships, other sites.

#### Options of the second site (prompt 112)

`brick_quarter` needed a few site options; all of them are off unless a
`site.json` sets them, so `harbor_city` regenerates bit for bit as before
(`just gen-geo-check`).

| Option | Effect |
|---|---|
| `extract --osm A.json B.json`, `--dem T1.tif T2.tif` | several Overpass answers (elements read once) and neighbouring DEM tiles (a mosaic in the pixel frame of the first; the site crosses 74° W) |
| pier multipolygons | relations tagged `man_made=pier`/`quay`/`breakwater` (not floating) are land like pier ways (`osm_query_extra` fetches them) |
| `elevation.clamp_m` | the ground per DEM pixel is held within [lo, hi] m before the blur: in a dense city the surface model is unreliable (tall towers over 30 m pixels, subtracted too deep) |
| `areas.skip_raised`, `areas.built_before_year` | areas on a `layer` above the ground (a park on a viaduct or a pier, roof gardens) and areas with an OSM `start_date` of that year or later are not read |
| `buildings.max_tagged_height_m`, `buildings.built_before_year` | a footprint taller than this, or with a later `start_date`, counts as untagged: the district's estimate stands there (the town of 1944) |
| `buildings.layout: "perimeter"` | street walls instead of rows across the block (below) |
| district `paved`, `skip_blocks` | the district's land is paved (`road`) without parks, woods or pitches; its OSM blocks get no buildings |
| `sheds` | lines of warehouses (`from_m`, `to_m`, `kind`, `gap_m`), length along the line, kept off roads (4 m) and 1 m off water and buildings, shifted across up to 9 m |
| `green.courtyard_spacing_m` | trees (oak, beech, birch) and bushes in the courtyards of perimeter blocks, 4 m off the houses |

**Perimeter blocks:** per merged block the main direction gives a frame
(long axis u, short axis v); in 2 m slices the block's real edge is known
on all four sides. Rows of catalog buildings walk along each side (both
long sides, then the two ends), every house at the innermost edge of its
stretch, so skewed and triangular blocks are followed in steps, with the
street facade (+X) outwards; the class comes from the footprint 5 m inside
the edge (its height after the 1944 rules, church, public building,
industry), else from the block, and the class's models are tried from a
random first one so rows mix. Houses stand 1 m apart (`building_gap_m`);
after every 90 m of row a passage of 8 m lets infantry into the courtyard
(`passage_every_m`, `passage_m`); a house that does not fit at the edge is
moved inwards by up to 4 m. Roads keep 4 m (one path cell) as before.

#### `brick_quarter` ("Backsteinviertel")

3.2 km × 2.2 km (1 601 × 1 101 samples), biome `forest`, heights -6 …
17.7 m (1 mm steps). The quarter of the place named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-brick-quarter-prompt-112)
as it stood around 1944, simplified like `harbor_city`: the frame is
turned to 29° so the avenues run across the picture (and along the 4 m
path grid); the river's shore runs at about 24° to them, so the water is a
wedge in the north-west corner (0.94 km², widest in the south) with the
quays and piers of the southern half; north of about the middle the frame
ends on land. The old irregular street net in the west and middle, the
grid in the north and east. Four starts in two teams at the ends: team 0
south at (-1450, -60) (quay) and (-1420, 600), team 1 north at
(1222, -884) (a college close) and (1508, 503) (a square), each levelled
34 m around, 80 m kept free of buildings and trees.

Heights: the surface model is a mess in the dense city (towers of the
2010s over 30 m pixels; subtracting today's building heights leaves pits
down to -39 m, not subtracting them leaves humps of 45 m). The ground is
the roof-subtracted model opened over 9 × 9 pixels, held within 2–18 m
and blurred over 50 m three times: a gentle rise from the river (2 m) to
the east (about 17 m), no street steeper than a few percent
[provisional].

The state of 1944: footprints taller than 45 m or with a `start_date`
from 1946 on get the district's estimate (the NYC footprint years were
not used, see SOURCES.md); areas on viaducts and piers (the park on the
elevated freight line, a park pier of 2021, roof gardens) and areas made
later are dropped; the waterfront district is paved quay with sheds along
the bulkhead and on three piers instead of today's riverside park; today's
footprints there (later buildings) are not used. The elevated highway
over the shore street and the freight line on stilts existed in 1944 but
cannot be built (no objects over roads); the elevated railways over the
avenues were gone by 1940.

What the simplification kept (`just gen-geo brick_quarter`, stage
pictures with `--report`):

| Data | OSM in the map | In the game |
|---|---|---|
| Roads | 392.8 km of ways | 123.5 km of 1 357 roads (2 794 instead of 7 304 points), 8–20 m wide (side streets 10 m, avenues 18 m, the shore street 20 m); dropped: 253.7 km foot, cycle and pedestrian ways (sidewalks), 7.7 km service roads, 7.2 km in tunnels, 0.6 km steps, 0.1 km under construction; carriageways and junctions closed (2.0 ha) |
| Coast | 118 coastline points (plus pier multipolygons) | 2 506 coast samples (2 m); piers added as land (8.8 ha), narrow piers and slots removed |
| Buildings | 8 218 footprints | 770 blocks after merging; 652 of at least 400 m² got 5 574 buildings in street walls (885 moved inwards, 11 blocks without room, none removed to keep ways open), plus 54 sheds on the quays and piers; 842 levelled pads, 25 left out |
| Areas | 2 677 land use, leisure and natural areas of the kinds read (after dropping 102 raised and 32 later ones) | 129 of at least 1 500 m² (102 parks, 21 pitches, 4 industrial, 2 woods) and 128 squares; 18.6 ha of waterfront and 301 ha of town ground paved |
| Green | – | 769 trees (parks, courtyards), 436 bushes, 498 hedge pieces |

Buildings per district (`site.json`, `districts`; blocks / buildings):
waterfront 4 / 0 (+ 54 sheds), market 14 / 143, SoHo 190 / 1 398, Chelsea
122 / 1 345, east of Fifth 167 / 1 177, West Village 102 / 872, Village
53 / 639. Kinds: 2 519 town houses, 2 262 tenement blocks, 717 brick
houses, 59 warehouses, 39 churches, 32 town halls.

Reachability (`just map-reach brick_quarter`): of 6.10 km² of land
(0.94 km² water) 66.9 % is open to vehicles and infantry, 0.7 % to
infantry only (park hedges, courtyard trees), 0.4 % to nobody (quay
edges) and 32.0 % is covered by buildings; 98.6 % of the open ground
hangs together (the passages lead into the courtyards; the rest are
closed small yards). Every start and supply point lies in that main part
for both classes.

Supply points in pairs, balanced by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | South | North |
|---|---|---|---|
| `supply_square` | the big square at the end of the avenue (contested) | 1 639 / 1 532 | 1 509 / 1 501 |
| `supply_pier_south` / `supply_market_north` | the pier with three sheds / the market quarter at the north end of the quay | 1 026 / 1 012 | 966 / 900 |
| `supply_triangle_south` / `supply_triangle_north` | two triangular places of the old street net | 1 448 / 1 401 | 1 620 / 1 500 |
| `supply_east_south` / `supply_east_north` | a crossing south of the old net / the square at the east edge | 876 / 868 | 809 / 736 |

Over the four rows the north is 85 m shorter by vehicle and 176 m on foot
[provisional]; every point lies where vehicles get to. Vehicle routes between the sides: the shore street (west), the
avenues through the middle and the broad diagonal and the avenues in the
east, three separate axes. Markers for tools and shots: `square`,
`waterfront`, `start_south`, `start_north`, `blocks` (for `just
objects-bench brick_quarter --blocks-bench`), `west_quarter`,
`east_quarter`, `north_quarter`, `south_quarter`. Pictures: `SHOT_SET=land
just terrain-shots <dir> brick_quarter` adds the quarters, street walls
from the game camera's height and a street at 35 m. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#brick-quarter-prompt-112).

Open: own models (brownstone row house, tenement with fire escapes,
cast-iron loft, quay warehouse, the arch on the square, churches);
buildings shaped from footprints; the elevated highway and the freight
line on stilts; hiding buildings in front of the camera (see the
observation in PERFORMANCE.md); the 1944 piers and sheds where today's
data have none (most of the shore north of the frame's middle lies
outside it); other parts of the island (towers over 100 m).

### Landscape maps (prompt 056)

`cli gen-land` (`crates/cli/src/gen_land.rs`, `just gen-land [map]`,
default all three) writes three 1 km maps for two or four players (two
teams; a two-player match uses players 0 and 1, diagonally opposite). No
real place names. Supply points stand on places of the map, and the
objects there are generated with them: the point is the village square,
the farmstead or the bridgehead you see.

| Map | Look | Starts | Supply points (places) |
|---|---|---|---|
| `winter_village` | biome `winter`, default weather `snow`; plowed main road north–south, a village of ~50 farm, brick, stone and log houses along two streets with gardens (woodpiles, hay, bare fruit trees, fences), the church at the square, snowy conifer woods with birches, snow drifts (terrain `snow`, 60 % speed) in the open, no mud (frozen) | team 0 south, team 1 north | `supply_square` (village square), `supply_trench` on a hill east with the trench strongpoint `strongpoint_hill` (prompt 052), `supply_farmstead` west |
| `farms` | rolling farmland, five farmsteads (farmhouse, barn, log shed, hay ricks and bales, fence, orchard; cottages and a second barn outside), field tracks with hedgerows and gaps for tanks, haystacks in the fields, small woods | team 0 south, team 1 north | `supply_farm_center`, `_west`, `_east`, `_north`, `_south` (the farmyards) |
| `river_land` | default weather `overcast`; a river north–south between the teams, a stone bridge in the middle with a hamlet and a damaged church at both ends, timber bridges north and south (16 t: no medium tanks), fords near the north and south edges, birches and young oaks on the banks, a farmstead on each side | team 0 west, team 1 east | `supply_bridgehead` (stone bridge), `supply_bridge_north`, `supply_bridge_south` (timber bridges; the marker sits on the deck, so units on both banks within 25 m count), `supply_farm_west`, `supply_farm_east` |

The ground is bumps and value noise as in `gen_town`, flattened at the
places and the starts (60 m around each start, kept free of objects);
the river and the bridges follow `gen_map` (narrowed water at the
crossings, embankments with 12 % ramps, axes on path cell center lines);
objects are placed without overlaps and off roads, tracks and water (a
16 m bucket grid keeps that fast). Field track headings for hedges use a
polynomial arctangent instead of `atan2`, so files are bit-identical on
every platform (a test generates every map twice). All three pass `just
map-check`; markers `village`, `church`, `farm_*`, `ford_*`,
`bridge_stone`, `trench_hill` serve tools and shots.

Tall grass (prompt 073) is painted after the objects (they do not move):
a frayed band 10–32 m wide along the banks of `river_land` (1.0 % of the
map) and a few fallow patches on `farms` (2.9 %), only on grass outside
woods (8 m from a tree), away from starts, supply points, roads and field
tracks; `winter_village` has none. `cli gen-map --tall-grass` paints large
stands over the meadows instead (map `tall_grass_test`).

`just gen-land ambush_meadow` (prompt 082, only when named; seed 82) writes
the ground of the video scene `data/trailer/ambush.toml` with the same
building blocks: a meadow of tall grass (all of it but a few patches of
short grass and a few single trees) in the south-east, dense wood in the
north (edge at y ≈ -120 m, so units placed there face south: units start
facing the map's center) and along the whole west flank (edge at x ≈ -140 m,
machine-gun range from the line of march along x = 0). The wood is trees
on a 5.2 m grid with a 12 m fringe of birches and young trees on a 5.4 m
grid, so nearly every vision cell in it is forest (two trees per 8 m cell).
A forest track leads from a clearing (player 1's start) to the edge and
stays free of trunks for 9 m to both sides; player 0 starts in the
south-east, off the line of march. It has no supply points and no
buildings: a ground for scenes, not a match map. Markers `wood_edge`,
`clearing`, `meadow`. It passes `just map-check`.

`SHOT_SET=land just terrain-shots <dir> <map>` records the whole map, a
few felled trees and every supply point (prompt 056); `SHOT_SET=tall_land`
the tall grass of `river_land` and `farms` (prompt 073).

Measurements: [prompt 056](../prompts/done/056-karten-basis-stufe2.md#ergebnis).

### Maps from picture sketches (prompt 088)

`cli gen-sketch <name>` (`crates/cli/src/gen_sketch.rs`, `just gen-sketch
[name]`, without a name every map of `gen_sketch::ALL`) writes
`data/maps/<name>/` from the sketch `data/map_sketches/<name>.toml`. A
sketch is the layout a template picture gives, traced by hand and
simplified to what the game's building blocks need; the pictures themselves
are not in the repository. Lines and areas are in meters (x east, y south,
origin in the map center):

| Table | Content |
|---|---|
| top level | `name`, `size_m` (multiples of 4 m, at most 4 km), `biome`, `weather`, `seed`, `water_flow_deg`; `base` = another sketch this one only changes (biome, weather, seed, size) |
| `[[start]]` | player, team, position (60 m kept flat and free) |
| `[[supply]]`, `[[marker]]` | supply point markers (`supply_*`, kept free and flat) and other markers |
| `[[river]]` | center line and width; banks cut 3 m into a 40 m valley, deep water in the middle, shallow at the edges, sand on the banks |
| `[[island]]`, `[[lake]]` | islands in rivers and lakes (circle); lakes and sea (polygon, `deep` or `shallow`) |
| `[[crossing]]` | `stone` or `wood` bridge or `ford` at a point of a river, `axis` `ns` or `we`: the axis is moved onto a path cell center line, the water narrowed to what the longest bridge of the kind spans (stone 21 m, timber 7 m), a straight approach and embankments with 12 % ramps are made, and the shortest fitting variant is placed (as `gen_map`) |
| `[[road]]` | `road` or `track` with width and points |
| `[[ring_road]]`, `[[square]]` | a ring track (a rondel) and paved squares |
| `[[wood]]`, `[[scatter]]` | woods (`mixed`, `deciduous`, `conifer`, `birch`, `palm`, `grove`: palms with bushes between them, `rock`: rock outcrops and boulders between conifers; trees on a jittered grid, 6 m off ways, none on fields or water) and scattered single trees in loose clumps; the biome picks the species (winter: conifers, birches and bare oaks; desert: palms) |
| `[[field]]` | `grain`, `green` or `ploughed` polygons, with a 2.5 m strip of grass inside the outline |
| `[[patchwork]]` | a patchwork of fields: plots of about `size_m` (each side varied by up to a third) in a grid turned along `axis`, 6 m of grass between them, kinds drawn from `kinds`, a share `meadow` left as grass; plots whose middle lies outside the polygon are left out (own random stream) |
| `[[row]]`, `[[ring]]`, `[[farm]]`, `[[building]]` | rows of huts or barracks across a line, rings of huts on an ellipse (walls, fences and hedges as touching segments: a walled height), farmsteads (`gen_land`), single buildings (catalog kinds and variants) |
| `[[hill]]` | bumps added to the ground noise |
| `[[rail]]` | a railway line: a 7 m ballast bed (terrain `dirt_track`, open to everybody) on an embankment (`embankment_m`, default 0.8 m) with telegraph poles beside it; only presentation, no rails or trains yet. A railway bridge is a `stone` crossing on the line |
| `[[paint]]` | any terrain of `terrain.toml` inside a polygon; with `noise_m` and `cover` only in blotches (two octaves of noise, denser along ways, thinning out towards the outline): trampled sand between ruins |
| `sea`, `[[land]]` | `sea = true`: everything outside the land polygons is sea, its depth growing with the distance to the coast (shallow within 10 m); land falls to one coast height within 60 m of the coast (a flat sea level) with a sand strip |
| `[[bay]]` | sea maps: water inside the land that belongs to the sea (lagoons, inlets), at the same level; polygons may overlap, their outer outline is the coast |
| `[[causeway]]`, `[[shoal]]` | strips along a line with a width: a causeway is land across the sea, cut across its whole width under the bridge of a crossing on it (the gap the bridge spans); a shoal keeps the sea shallow (a ford for tanks) |
| `[[town]]` | street building: inside the polygon, buildings of `kinds` line both sides of every road and track of the sketch, long side along the street, `setback_m` (default 3 m) behind its edge, `gap_m` (default 4 m) apart on average, a share `damaged` damaged or ruined; back lots are filled with a `[[village]]` over the same polygon |
| `[[village]]` | `count` loose buildings of `kinds` inside a polygon, turned along the nearest way within 40 m, a share `damaged` damaged or ruined, and `walls` short runs of broken mud-brick walls |

The generator shares the object placer, woods and farmsteads with
`gen_land` (its `Objects` now also takes maps that are not square;
`gen_land`, `gen_city` and their maps stay bit for bit the same). Like every
generator it uses only basic arithmetic and `sqrt`; a test generates every
sketch map twice and compares the files.

| Map | Sketch | Size | Biome, weather | Starts | Supply points |
|---|---|---|---|---|---|
| `river_fields` | broad river west–east with an island, main road north–south over a stone bridge, timber bridges east and west, a ford in the west; barracks rows and a paved square in the north, hut camps in rings, a rondel square with houses and many fields in the south; woods in the north, east and south | 1.6 × 1.6 km | forest, clear | team 0 south (players 0, 2), team 1 north (1, 3) | stone bridge, timber bridge east, north farm, paved square, rondel, camp |
| `river_fields_winter` | the same sketch (`base`) | 1.6 × 1.6 km | winter, snow | as above | as above |
| `river_fork` | a river from the north and one from the east meet in the west and flow on south; a shot-up village of loose houses (three quarters damaged or ruined), broken walls and trampled sand in the angle and south of it, many branching tracks; nine bridges (three over the north river, four over the east river, a road and a railway bridge over the river after the fork); a railway across the south; woods north-west, north-east and south | 1.6 × 1.6 km | forest, clear | team 0 east (player 0) and south-west (2), team 1 north (1) and west (3); 1v1 meets over the east river in the village | north village, west bridges, east bridge, south village, railway, east track |
| `island_chain` | islands and peninsulas in the sea, one road from the north-east to the south-west over four stone bridges on 30 m causeways; two long shoals (from the north-east cape to the wooded island, and between the west woods and the south-west land) are the second way for tanks; farms, five fields, woods, ponds on the wooded island; islands at the edges without access | 1.6 × 1.6 km | forest, clear | player 0 south-west, player 1 north-east (1v1 only) | east islet, west islet, crossroads, field farm, wooded island |
| `desert_lagoon` | desert by the sea in the north-east; a strait leads into a branching lagoon (bays) with four dams: stone bridges at the mouth and at the junction in the middle, timber bridges over the west channel and the east branch, a dam without bridge at the west junction; a land neck in the south; winding tracks, scattered desert houses, a small town in the east, palm and bush groves north-west and south | 1.6 × 1.6 km | desert, clear | team 0 east (players 0, 2, 4), team 1 north (1, 3, 5) | north village, town, inner land, south houses, west houses, neck |
| `winter_town` | a small town of about 770 buildings on both banks of a river from north to south, two stone bridges and a timber bridge in the south, a square with church and town hall, halls, a quarter in the north-west, outlying farms east and west, conifer woods all around, country roads to every edge | 1.6 × 1.6 km | winter, snow | team 0 west (players 0 north-west, 2 west), team 1 east (1, 3) | square, north-west quarter, east crossroads, south town, east road, south bridge |
| `river_town` | a small town of about 880 buildings in a river bend: the river winds from the north through a loop and the town to a junction and on to the south-east, an arm from a lake with an island in the west joins it, a second lake with an island in the north-east; eight bridges (five stone); quarters on the far banks, warehouse quarters west and east, a walled church on a height in the north-east (the picture's castle), two rock heights, fields at the edges, much wood; motorway, roundabouts, sports and golf grounds became country roads and meadows | 3.0 × 2.0 km | forest, clear | team 0 west (players 0 north-west, 2 south-west), team 1 east (1 north-east, 3 south-east) | town square, north quarter, west gate, warehouses west and east, south and south-east village, castle hill |
| `valley_fields` | a river in great bends across the map, an arm from the north and a branch through rock hills with rapids to the south-east; nine bridges (six stone); in the west a town of halls and street blocks around a station yard (railway to the south-west), villages in the middle and east between eight patchworks and about forty single fields, rock heights in the east and south-west, a walled church on a height | 3.0 × 2.0 km | forest, clear | team 0 west (players 0 south-west, 2 north-west), team 1 east (1 south-east, 3 north-east) | station, town square, north-west fields, central village, north-east village, south village, castle hill, rapids bridge |
| `river_valley` | a cut-out of a valley: a broad river in an arc from north to south between wooded hills (three summits up to 28 m, two with a walled stone-house stand-in for a lookout tower), a town on the west bank with a street grid, an old core at the south bridge, a suburb ring, halls; a gravel pit with a lake and an island in the north-west; a trunk road and a railway over two stone bridges in the middle, a small harbour basin and a stream in the east; patchworks of fields; simplified so that it is a landscape of this kind, not a town plan | 3.6 × 2.0 km | forest, clear | team 0 west (players 0, 2), team 1 east (1, 3) | old town, town centre, harbour, halls, east village, south bank, gravel pit, east lookout |
| `river_city` | a city of about 3000 buildings: street quarters in turned grids (traced as clipped grid lines), avenues radiating from a central place with church and town hall, a river in a great loop around the centre with eight bridges (six stone), a park with ponds and a rondel in the north-east, a canal and a park in the south, a railway with a station yard across the south, a quarry with halls in the west, fields at the edges; lighter than the picture | 3.6 × 2.4 km | forest, clear | team 0 west (players 0, 2), team 1 east (1, 3) | central place, station, park rondel, loop village, quarry, north quarter, south-east quarter, south park |

## Map editor (prompt 028)

Maps can be built in the game: main menu → **Map editor** (or
`just map-editor`, `-- --edit=<name>` opens `data/maps/<name>/`). It is a mode
of the game, not a Godot editor plugin: it reuses the terrain renderer, the
object and water rendering, the camera and ground picking unchanged, works in
every build, and its saving and checking run through the same Rust code that
loads maps for the simulation (reasons and data flow:
[ARCHITECTURE.md](ARCHITECTURE.md#map-editor)).

| Tool | Effect |
|---|---|
| Raise / Lower | ground up or down at the center, smooth falloff `(1 − d²/r²)²` to the rim; hold the button (3 m/s at full strength) |
| Smooth | towards the mean of the 3 × 3 samples around |
| Flatten | towards the height where the stroke began |
| Paint terrain | any id of `terrain.toml` inside the circle (roads, tracks, mud, fords …) |
| River | bed down to 0.6–3 m (strength) below the height where the stroke began, parabolic profile: deep water in the channel, shallow water beside it, a sand bank at the rim; only digs, never fills; existing water is never made shallower |
| Vegetation / objects | scatters a preset (deciduous, conifer, birch, palms, bushes, rocks, hay, rock slabs, rock outcrops, rock ridge) at a density per hectare, random headings, keeps a minimum spacing, never on tracks, roads or water; box-shaped objects keep their whole box off them, and nothing lands inside a blocking box. Rock presets (prompt 074): slabs in loose fields, outcrops with bushes and stones, ridge pieces close together (they overlap) with conifers and bushes between them (a second scatter in the same dab) |
| Place object | any kind and variant of `objects.toml` with a heading; bridges snap to a multiple of 90° and onto a center line of the 4 m path grid |
| Erase objects | removes the objects in the circle |
| Start player 1 / 2 | sets the start positions (whole meters) |
| Ridge / Valley | raise or lower like Raise/Lower, but with a linear falloff: dragged along a line it builds a sharp crest or a V-shaped valley (prompt 060) |
| Terrace | pulls the ground onto multiples of 5 m: benches with steep risers, cliff bands |
| Road on slope | a road piece from dab to dab, painted with the chosen terrain (road or field track, otherwise road): the bed (half the radius) follows the ground one radius ahead, but its level changes by at most 12 % [provisional] per meter driven; beyond the bed 50 % cuttings and embankments |
| Place compound | a compound of `data/compounds.toml` (house with walled yard, gate and props; mosque, police post, shop row) at the point, turned by the heading in 90° steps (prompt 059) |
| Village (desert) | packs courtyard compounds into the circle with alleys and lanes (prompt 059, above) |

Every stroke (mouse down to up) is one undo step (**Ctrl+Z**, **Ctrl+Y** or
**Ctrl+Shift+Z**; up to 100 steps). `[` / `]` change the radius, **Ctrl+S**
saves. **New** creates a flat grass map of 256 m to 4 km in a biome, **Open**
loads any map of `data/maps/`, **Save** writes `data/maps/<name>/` (names:
lowercase letters, digits, `_`, `-`); `test_small` and `test_large` cannot be
overwritten (the golden hash and the benchmarks depend on them; regenerate them
with `just test-maps`). A saved map is listed in the single-player menu at once.

Saving writes exactly the format above: the height encoding comes from the
height range like in the generator (base one meter below the lowest point,
the smallest step that covers the range); if that needs steps coarser than
1 mm (more than 65 m of relief), the editor takes over the rounded heights, so
editor and file agree. `generator` and the markers of a loaded map are kept.

**Check** (also on every save, and `just map-check <name>` / `cli map-check
DIR` for any map folder) loads the files the way a match does (`Map::load`,
`add_objects`) and reports:

- errors from loading (e.g. a bridge off the path grid),
- missing start positions for players 1 and 2 (ids 0 and 1),
- units of the scenario that cannot stand at their start (water, too steep,
  objects, map edge),
- whether the tanks of every faction can drive from one start to the other
  (an error) and whether infantry can (a warning), with the path finder of
  the simulation (`PathGrid::find_path`),
- whether every supply point (markers `supply*`) can be reached from
  player 0's start, within 20 m (prompt 060): an error if no unit type
  gets there, a warning listing the types that do not,
- the map hash a match on it will use,
- the same for every start variant (prompt 182).

**Start variants** (prompt 182): the start tools edit the layout chosen under
**Layout** (shown with them): `standard` or a variant; **New** adds a variant
`variant_<n>` with a copy of the standard starts and edits it, **Remove**
drops the edited one. Saved as `[[variant]]` in `map.toml` (rename and
`supply` by hand for now).

The editing code is the crate `crates/editor` (`EditMap`, brushes, undo,
files, validation): integers only like the simulation, so the same edit
script gives byte-identical files (tested). A test builds a map with every
tool, saves it, loads it on two peers and plays the demo scenario on it in
lockstep over loopback with identical hashes every tick
(`crates/editor/tests/build_and_play.rs`); `just editor-selftest` does the
same through the editor scene and the single-player menu.

Measurements (release build, Apple M1 Max):

| | `test_small` (500 m) | `test_large` (4 km) |
|---|---|---|
| Open (read + decode + objects) | 6 ms | 184 ms |
| Raise / smooth dab, radius 25 m | 3 / 6 µs | 3 / 7 µs |
| Objects brush dab, radius 25 m, 300/ha | < 0.1 ms | 8 ms (spacing check over all objects) |
| Undo + redo of a stroke | < 0.1 ms | 21 ms |
| Rebuild the display map after a stroke | 1.5 ms | 88 ms (plus the object and water nodes) |
| Encode the files (PNG compression "high") | 8 ms | 1.3 s |
| Check | 16 ms | 0.53 s |

In Godot every dab also re-reads the changed rectangle for the terrain and the
height ranges of all chunks (whole map); after every stroke the objects,
decoration and water are rebuilt. Measured in the editor scene (debug bridge,
headless): opening 0.3 s / 1.9 s, a raise dab of 20 m with terrain redraw
0.6 ms / 12 ms, the end of a stroke 25 ms / 0.53 s. Smooth on maps up to
1 km; on 4 km maps a stroke ends with a noticeable pause (rebuilding only
the touched object chunks would remove it).

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

Water surfaces (prompt 014) are drawn by `godot/scenes/world/water.gd` from
`water_mesh`; `water_levels`, `water_level_at`, `is_water_at`, `bridges`,
`ground_height_at` and `water_flow_deg` serve the terrain shader, tools and the
self-test (see [ARCHITECTURE.md](ARCHITECTURE.md#water-presentation-only)).
