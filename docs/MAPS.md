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
biome = "forest"             # forest | desert | winter | mediterranean; presentation only (optional)
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
temperate biome); other values than `forest`, `desert`, `winter` and
`mediterranean` (prompt 120) fail to load. `look` (prompt 122, optional)
lays a variant over the biome's look: `vivid` (the colorful `archipelago`)
loads `godot/assets/biomes/<biome>_vivid.tres` instead of the biome's
preset where one exists (only `forest_vivid` so far); `mud` (prompt
124, `trench_line`) loads `forest_mud.tres`: muted earthy colors, browner
grass, a grey hazy sky, ground that always looks wet (`wet_ground`) and
brown water; `mesa` (prompt 126, `mesa_desert`) loads `desert_mesa.tres`:
the desert with the mountain look of prompt 060 at full strength (strata
on the cliffs, grey scree at their foot) and a lower sun (38° instead of
58°) whose shadows show the cliffs; `rock` (prompt 128, `kessel_lake`)
loads `forest_rock.tres`: the plain forest with the mountain look at full
strength (grey rock walls with strata, scree) at any relief and without a
snow line (the mountain look of prompt 060 starts only above 120 m of
relief and brings snow above 80 % of it); like the biome it
is checked by the simulation (`sim::Map::look()`, other values fail), not
part of any hash and read by no rule; `-- --look=<id>` (`plain`: none)
overrides it.
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

### Prepared ground (prompt 124)

A map may bring field works and shell craters that the simulation makes
before the first tick, with the rules of works and craters made during a
match (`sim::prepared`, `crate::sim::prepared`): so a prepared trench
gives the cover, firing bays, parapet and look of a dug one, a prepared
crater the crater cover and the mud of a shell crater. `map.toml`:

```toml
[[field_work]]                 # neutral, finished, dug before tick 0
kind = "trench"                # trench | comm_trench | wire | mg_nest | dugout
points_m = [[-880, -136], [-866, -123], [-846, -123]]   # a line; points for mg_nest / dugout

[[crater]]                     # craters of one caliber
caliber_mm = 150               # size from data/craters.toml
water = true                   # presentation: rain water in the bowl
at_m = [[-512, 34], [-498, 40]]

[[puddle]]                     # presentation only: wet hollows
radius_m = 3
at_m = [[-1010, -120]]
```

- **Works**: each segment of a line is cut into pieces like an engineer's
  line (`[positions]` zigzag for fire trenches) and added as a finished
  neutral work (`NEUTRAL_OWNER`, like a strongpoint); wire is a finished
  obstacle (infantry is blocked, vehicles flatten it, engineers clear
  it). Dug works go to `map_dig_pct` (75 %) of the dig depth and parapet
  (`data/works.toml`): the full 0.5 m cut with its parapet lies just at the
  60 % grade limit of a path cell and closed the trench lines for
  everybody on sloped ground. Their cover is that of every dug sample.
- **Craters** are planned like ground hits on the loaded ground; their
  samples enter the running crater checksum but they do not count as
  shell craters of the match and leave no crater events.
- Everything is applied to the ground first, then the path and vision
  grids are brought up to date once over the whole area, and the wire's
  path cells are marked once for all of it (5 700 craters, 2 840 trench
  and 2 530 wire pieces of `trench_line`: `Sim::new` takes 0.16 s in all).
- Works and craters are part of the map hash (only maps with some; every
  other map keeps its hash); the water flags and puddles are not.
- Tools that read the ground use the map as a match starts on it:
  `map-reach` (with the match's path grid, so the wire counts) and
  `map-sight` (`prepared_ground` in `crates/cli/src/main.rs`).
- The map editor keeps the tables as loaded; drawing them is open (a line
  tool for trenches is a follow-up).
- The presentation (`world/puddles.gd`) drapes a crater skin (dark wet
  bowl, rim of thrown-up earth) over each prepared crater: at 2 m between
  height samples a shell hole hardly shows otherwise. Water stands in the
  craters with `water` and in the puddles; markers `smoke_<n>` get a smoke
  column, markers `mist_<n>` a low sheet of mist (330 m around).

### Bridges

Bridges (prompt 014) are map objects whose kind has a **deck** rule in
`data/objects.toml`: roadway width, load limit and the terrain whose speed
applies on the deck.

| Kind | Variants (length) | Roadway | Load limit |
|---|---|---|---|
| `env_bridge_wood` (single-lane timber) | `Span8` (8 m), `Span12` (12 m), `Span16` (16 m) | 4.1 m | 16 t |
| `env_bridge_stone` (stone arch) | `Arch1`, `Arch2` (20 m), `Arch3` (30 m) | 5.1 m | 100 t |
| `env_bridge_steel_arch` (steel through arch, prompt 115) | `Arch244` (244 m, Nijmegen), `Arch120` (120 m, Arnhem) | 12 m | 100 t |
| `env_bridge_steel_approach` (girders on piers, prompt 115) | `Span60` (60 m), `Span30` (30 m) | 12 m | 100 t |

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

#### Long bridges (prompt 115)

**Stone chains (prompt 122):** the same rules carry long masonry bridges:
`env_bridge_stone` `ArchHigh` (one arch of 16 m span between 2 m
abutments, 20 m long, deck 8 m above the model base) laid end to end; the
abutments of two pieces form the pier between their arches. The sketch
generator places them for `[[crossing]] kind = "stone_long"` (below); the
deck lies 6 m above the banks, so 3–5 m of arch stand above the water.
Like the short stone bridges they hide what is behind them and fall only
to engineers (no `[shelling]`).

A big river is crossed by several pieces **end to end**: approach spans,
the main arch, approach spans (the Waal bridge at Nijmegen: 244 m arch
in 604 m). Rules (`crate::bridges::Bridge::link_chains`):

- **One roadway:** pieces of the map whose ends meet (within 0.25 m) on
  parallel axes form a chain. Its deck height runs linearly between the
  ground at the two **outer** ends of the whole chain, so the deck lies
  as high as the dams that lead up to it (10-20 m over the water) and is
  continuous across the joints. A single bridge keeps its own ends as
  before. The heights are fixed when the map loads: a destroyed piece
  does not change the others.
- **Hanging joints:** an end of a piece that does not meet the ground
  (more than 0.5 m apart) hangs (`Bridge::hangs`). Units and paths leave
  a deck over a hanging end only onto the next piece; when that piece is
  down, the end is a dead end (the path grid treats the step like a
  railing, movement like a cliff), and units plan to the nearest point
  they still reach. Engineers demolish a piece from beyond one of its
  ends that they can stand on (the ground or the next piece).
- **Ramps:** the map brings the ground at both outer ends up to the deck
  height: earth dams of at most about 12 % with an **abutment** of about
  8 m of ground at deck height under the deck end (otherwise the cell at
  the end is too steep). The importer builds them (below).
- **No underpass:** under a deck nothing is passable, also under the
  approach spans on piers over land (the cell is the deck, the ground 18 m
  below is unreachable). Streets under the approaches and across the
  dams end there; traffic goes round the dam ends. [provisional, see
  prompt 115: two levels per path cell would be needed]
- **Sight:** steel bridges are `see_through`: sight lines pass under the
  deck over the ground below (the vision grid keeps that ground as the
  horizon), while units on the deck stand, see and are seen at deck
  height. A prone soldier on the deck has the railing as cover against
  rounds from the side (`railing_cover_pct`, like a low wall).
- **Destruction:** every piece falls on its own: engineers (prompt 055)
  or heavy rounds and explosions on its deck (`[shelling]` in
  `data/crossings.toml`: from 10 cm, caliber² × the vehicle factor as in
  prompt 020; a 60 m span takes about 16 hits of 15 cm). What stands on
  the piece falls with it (prompt 055). The model shows the collapsed
  variant `<Variant>Down` (broken span, debris in the water).
- **Diagonal rivers:** bridges stay along the path grid axes. A map from
  geodata turns its frame (`frame.x_axis_bearing_deg` of `site.json`) so
  that the bridge runs along map x or y; the river may then cross the map
  at any angle.

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

### Mediterranean village (prompt 120)

**Objects.** Plants: `env_tree_stone_pine` (`StonePine`, `StonePineYoung`,
`StonePineLeaning`), `env_tree_cypress` (`Cypress`, `CypressShort`),
`env_tree_olive` (`Olive`, `OliveOld`, `OliveYoung`) and
`env_bush_macchia` (`Holm`, `Strawberry`, `Broom`). Trees follow the tree
rules (block vehicles at the trunk, foliage for sight, hit points, fire:
pine and cypress like conifers, olive like broadleaf trees; tanks push
over the young pine, the short cypress and the young olive, data/destruction.toml);
macchia blocks nobody and gives 15 % cover like `env_bush`. Buildings
(Intact, Damaged, Ruin; block everybody, ruins vehicles only):
`env_house_med_small`, `env_house_med_two`, `env_house_med_row` (middle
of a row, 6 m frontage, shop), `env_house_med_row_corner` (end or corner,
four storeys, shop), `env_house_med_casale` (outside stair),
`env_barn_med`, `env_church_med` (campanile), `env_tower_coast`,
`env_town_hall_med`; walls `env_wall_field` (`Segment`, `Low`, `Broken`,
`Gate`, `End`, `Corner`; vehicles stop, infantry climbs; heavy tanks push
through like the dry-stone wall) and `env_terrace_wall` (`Segment`,
`End`, `Corner`: a 1.5 m retaining wall with its soil uphill on the model's
+X; blocks vehicles, not destroyed; the terrain stays the map's). Hit
points, capacities and exposures: data files, [provisional].

**Compounds.** `data/compounds.toml` knows building families: `[wall]` is
the desert family (default), `[walls.mediterranean]` builds yards of
`env_wall_field`; a compound names its `family`. Mediterranean
compounds: `med_house_yard`, `med_two_yard`, `med_row`, `med_row_short`
(village brush), `med_casale` (farmstead with barn, olives and
cypresses), `med_church_yard`, `med_town_hall`, `med_tower`. The editor
has the tool "Village (mediterranean)" (`EditMap::village_of`), the
object group "Mediterranean village" and the brush presets "Pine wood",
"Olive grove", "Cypresses" and "Macchia".

**Importer.** With `map.biome = "mediterranean"` `tools/geo_import.py`
swaps the building classes of the site for the Mediterranean family
(`MED_BUILDINGS`: town house → corner row house, stone and brick house →
village house, farmhouse → casale, church → church with campanile, town
hall → town hall; blocks and warehouses stay), plants stone pines, olives
and cypresses instead of oak and spruce (palms in parks stay), macchia
instead of hazel, cypresses along tree rows, field-stone garden walls
instead of hedges, olive groves in rows (8 m grid, `olive_spacing_m`) on
`landuse=orchard` and vine rows (terrain `field_green`) on
`landuse=vineyard`. Other biomes regenerate bit for bit as before.

**Test map.** `cli gen-borgo` (`just gen-borgo`) writes
`data/maps/med_village`: 600 m, biome `mediterranean`, two starts (west,
east), a square with church, town hall and rows of houses, about 36
village compounds, a farmstead, a hill in the east with four rings of
terrace walls, olives on the steps and the watch tower on top, a stone
pine wood, cypresses along the lane, macchia, a vineyard and a stubble
field; markers `village`, `church`, `terraces`, `farm` and the supply
points `supply_square`, `supply_hill`, `supply_farm`. Pictures: `just
med-shots <prefix>`.

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
the format above from an OpenStreetMap extract and a free elevation model
(or an official terrain model in UTM tiles, prompt 114).
`just gen-geo harbor_city` writes `data/maps/harbor_city/` (and
`just gen-geo brick_quarter` the second site, prompt 112,
`just gen-geo coastal_heights` the third, prompt 113,
`just gen-geo dune_coast` the fourth, prompt 114,
`just gen-geo river_crossing` the fifth, prompt 116,
`just gen-geo rhine_bridge` the sixth, prompt 118,
`just gen-geo landing_gulf` the seventh, prompt 121,
`just gen-geo castle_rock` the eighth, prompt 127);
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

3.6 km × 2.4 km (1 801 × 1 201 samples), biome `mediterranean` since
prompt 120 (before `forest`), heights -6 … 673 m (11 mm steps). The terrain,
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

**Prompt 120: Mediterranean.** `site.json` names the biome
`mediterranean`; the importer then builds the Mediterranean family and
plants (see "Mediterranean village (prompt 120)"): 1 498 buildings (696
corner row houses, 14 row houses, 84 village houses, 256 casali, 7
churches with campanile, 4 town halls, 435 blocks and 2 warehouses as
before), 5 534 stone pines, 4 220 olives (2 354 of them in the 13
orchards as olive groves), 1 896 cypresses, 313 palms, 3 534 macchia
shrubs and 1 349 field-stone garden walls instead of hedges. The
smallest built block rose from 700 to 1 000 m² (the new houses fill the
blocks more densely, the 1 600 building cap). Reachability afterwards: 56.9 %
open to vehicles and infantry (before 56.0), 2.8 % infantry only, 6.7 %
covered by buildings (7.6); largest part 82.0 % (81.7); every start and
supply point stays in it, path lengths change by at most 180 m
(`supply_harbor_east` from start 0: 2 303 → 2 485 m by vehicle). AI
tournament `--a hard --b normal --games 12 --max-ticks 30000`: before
33 % / 8 % wins, 7 timeouts; after 33 % / 58 %, 1 timeout (a small series:
the new trees and houses change sight and paths). Pictures before/after:
`just harbor-shots <prefix>`. No vineyards in the site's data.

Open: buildings shaped from footprints, landmark models, terrace walls
on the slopes from the importer, tunnels and stairs, ships, other sites.

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

#### Options of the third site (prompt 113)

`coastal_heights` needed a few more site options; as before all are off
unless a `site.json` sets them (`harbor_city` and `brick_quarter`
regenerate bit for bit, `just gen-geo-check`).

| Option | Effect |
|---|---|
| `sources.extra_tags` | tags `extract` keeps besides the usual ones (here `historic`, `military`, to know batteries and bunkers) |
| `coast.lakes_min_m2` | closed `natural=water` areas of at least this size (not intermittent) are water too, with the sea bed rings of the coast (a lagoon behind a beach) |
| `roads.skip_polygons_m` | ways with more than half their points inside one of these polygons are dropped (streets built after 1944) |
| `roads.drop_bridges_over_water` | bridges with at least 30 % of their line over water are dropped (a strait the game cannot span; the road ends at the shore) |
| `roads.extra` | tracks or ramps of the site (`pts_m`, `width_m`, `terrain`), added after the simplification (none needed here) |
| `buildings.skip_tags` | footprints with one of these `[key, value]` pairs are not read (batteries, bunkers, ruins of later works) |
| `fortifications` | field works at given places (`works`: `at_deg`, `facing_deg` compass, `source`): a `fort_bunker_mg` at the nearest free spot (rings up to 15 m), `trench_sections` trench pieces `trench_ahead_m` in front of it across the facing, a sandbag position on each side; nothing on roads, water, buildings or reserved places |
| `green.scrub_spacing_m: 0` | scrub and heath stay tall grass without trees or bushes |

**Sight from the heights:** `just map-sight <name> [picture.png]` (`cli
map-sight`) computes, with the simulation's viewshed (`VisionGrid::sight_from`:
ground, solid objects, forest, the downhill bonus of prompt 060), what an
observer 2 m above the ground sees from every supply point and every gun
position (markers `height_*`): the area, the part of it 20 m or more below
the eye, the cells of that part on the side towards each team's enemy (the
half-plane in front of the point, seen from the mean of the own starts)
and the markers in sight; then, per start, the gun positions a vehicle
(the tank size class, which trucks with a towed gun share) reaches and
that look down into a valley towards the enemy (at least 100 such cells,
0.64 ha). The picture has one pixel per 8 m vision cell.

#### `coastal_heights` ("Küstenhöhen")

6.0 km × 3.4 km (3 001 × 1 701 samples, 25.5 MB in memory), biome
`forest`, heights −12 … 334 m (6 mm steps). The headlands of the place
named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-coastal-heights-prompt-113)
around 1944, simplified like `harbor_city`: grass ridges up to 334 m (the
north edge; the south ridge 260 m, the north ridge 245–287 m, the ridge
above the east battery 270 m), the broad valley from the lagoon in the
west to the east end of the valley, a cliff coast with a few beaches along
the strait in the south and the ocean in the west, the fort in the east in
a bowl behind the east ridge, a town in the north-east corner. The frame
is turned to 78° so the coast runs along the bottom of the picture (view
from the water); a strip of 0.25–0.7 km of water in the south and west.
Four starts in two teams: team 0 west at (−2620, −410) (the beach fort)
and (−2140, −180) (north shore of the lagoon), team 1 east at (2710, 300)
(the parade ground of the east fort) and (2760, 540) (the bay shore),
each levelled 34 m around with 80 m kept free.

Heights: the surface model with a light opening (3 × 3 pixels; few
buildings), blurred over 14 m twice instead of 18 m, so ridges, saddles
and valleys stay sharp; the tops are still rounded, so an observer on a
summit sees little of the flanks below (a convex dome hides them, like a
real hilltop without a forward crest). The gun positions (`height_*`)
therefore lie on the forward crests, 30–130 m from the summits, where the
slope drops away towards the valley [provisional]. The sea bed falls from
−1.5 m at the shore by 0.8 m per 2 m to −12 m; cliffs stay as they are
(too steep for anyone); beaches are sand. The lagoon is water (`lakes`).

The state of 1944: the batteries' footprints are not built as houses
(34 footprints skipped: `historic=fort`, bunkers, ruins); instead 15
places of coast batteries inside the frame get a pillbox, three trench
sections and two sandbag positions each, facing the sea (41 trench
sections, 30 sandbag positions). The post-war family housing in the
valley (24 blocks, 7 streets) and the 1950s missile site are left out;
the forts' buildings stand as catalog houses (stone and brick houses,
warehouses; no barracks object exists for maps). The bridge over the
strait is not built: its ways over water are dropped (0.5 km), the
highway ends at the shore of the north anchorage, and the viaduct over
the fort's road is dropped as a bridge over a road. The two tunnels
(single-lane tunnel between the east fort and the valley, highway
tunnel) are left out like every tunnel; vehicles cross between the
sides over the ridge roads (the coast road along the south ridge and the
road from the valley over the saddle up to it).

What the simplification kept (`just gen-geo coastal_heights`, stage
pictures with `--report`):

| Data | OSM in the map | In the game |
|---|---|---|
| Roads | 136.0 km of ways | 70.8 km of 261 roads and tracks (1 625 instead of 5 626 points); dropped: 47.5 km foot and cycle ways (trails are `path`/`footway`; `track` stays as `dirt_track`), 12.0 km service roads, 2.2 km in tunnels, 1.2 km steps, 1.2 km later streets, 0.5 km bridges over water, 0.1 km a bridge over a road, 0.2 km stubs and fragments; carriageways and junctions closed (3.8 ha) |
| Coast | 1 584 coastline points | 10 590 coast samples (2 m), the lagoon and a pond as water |
| Buildings | 1 270 footprints (34 batteries, bunkers and ruins skipped) | 607 blocks of at least 150 m² got 776 buildings (627 in the town in the corner, 149 in the three forts), 24 blocks of later housing skipped; 700 levelled pads, 34 left out |
| Areas | land use, natural and leisure areas | 2 scrub areas (tall grass, 66 % of the map), 6 woods, 16 beaches, 7 parks, 1 pitch |
| Green | – | 3 651 trees (woods and parks only, none on open ridges), 494 bushes, 22 hedge pieces |
| Works | 17 batteries listed | 15 in the frame: 15 pillboxes, 41 trench sections, 30 sandbag positions |

Reachability (`just map-reach coastal_heights`): of 15.97 km² of land
(4.43 km² water) 75.4 % is open to vehicles and infantry, 0.5 % to
infantry only, 23.0 % to nobody (steeper than 60 %: cliffs, the steep
flanks above the coast and along the highway) and 1.0 % is covered by
buildings and objects; 97.3 % of the open ground hangs together. Every
start, supply point and gun position lies in that main part.

Supply points in pairs, balanced by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | West | East |
|---|---|---|---|
| `supply_valley` | east end of the valley (contested) | 3 233 / 2 857 | 3 136 / 2 772 |
| `supply_south_ridge` | summit of the south ridge (contested height) | 3 610 / 3 290 | 3 527 / 3 376 |
| `supply_north_ridge` / `supply_north_east_hill` | north ridge above the valley / the hill north of the east fort | 1 593 / 1 554 | 2 028 / 1 570 |
| `supply_west_height` / `supply_east_ridge` | height above the west point / ridge above the east battery | 1 393 / 1 339 | 1 196 / 1 061 |

Over the two pairs the east is 230 m longer by vehicle and 260 m shorter
on foot [provisional]; the routes from the east are longer since the
tunnel is missing (everything goes over the ridges).

Gun positions (`height_*`, `just map-sight coastal_heights`; each sees
about 10 ha, 0.6–0.8 % of the land, most of it 20 m or more below; no
point sees much more than its own slopes and the valley below):
`height_north_ridge`, `height_west`, `height_middle_east` and
`height_south_ridge_east` look east, `height_east_ridge`,
`height_south_ridge_west` and `height_middle_west` look west. Towed guns
reach from the west starts six positions with a view into a valley
towards the enemy and from the east starts five (test
`map_sight::tests::coastal_heights_guns_reach_heights_with_sight`). The
supply points on the summits see 2–13 ha.

Markers for tools and shots: `valley`, `south_ridge`, `north_ridge`,
`lagoon`, `fort_west`, `fort_east`, `start_west`, `start_east`.
Pictures: `SHOT_SET=land just terrain-shots <dir> coastal_heights` adds
the coast from the water, the valley, both ridges, the lagoon, the forts
and a gun position at 60 m. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#coastal-heights-prompt-113).

Open: tunnels as map elements; own models for concrete batteries,
casemates, fire control stations and the forts' barracks, coast guns as
fixed playable positions; a dry summer look (yellow grass, coastal scrub)
as a biome; the bridge over the strait and the far shore as scenery;
observers and forward artillery spotting as a rule.

#### A second elevation source: official terrain models in UTM tiles (prompt 114)

The Copernicus DEM (30 m) turns dunes into soft humps. `geo_import.py`
therefore reads an official terrain model with 1 m cells as a second
source, first the **DGM1 of Schleswig-Holstein** (open data, CC BY 4.0;
the other German states publish the same format):

- **Tiles:** 1 km × 1 km in ETRS89 / UTM zone 32N (EPSG:25832) as `x y z`
  text, one point per 1 m pixel center, heights over NHN in meters,
  `-9999` for no data. `extract SITE --osm … --dgm T1.xyz T2.xyz …`
  (instead of `--dem`) reads any number of tiles (`sources.dgm_zone`,
  `sources.dgm_margin_m`).
- **Reduction:** 2 m cells (the map's sample spacing), each the mean of
  its 2 × 2 pixels in centimeters, over the map frame plus the margin
  (100 m); cells farther from the frame stay empty, so `dem.json.gz`
  (format `geo_import dem utm v1`) compresses to 4.2 MB for
  `dune_coast` (2 281 × 2 767 cells) and needs no 16-bit packing.
- **Transformation:** every map sample goes map frame → geographic
  (the exact inverse of the frame's equirectangular projection, so OSM
  and heights share one frame) → UTM (transverse Mercator, Snyder's
  series to the sixth order, only `+ - * /`, `sqrt` and the own sine
  series; against the EPSG Guidance Note 7-2 example 1 cm off). The
  exact transformation runs on a lattice every 16 samples (32 m) and is
  bilinear in between: measured against the exact one at the lattice
  cell centers the error is below 1 mm; one affine map for the whole
  frame would be up to 1.0 m off (`stats.dgm`). The 2 m cells are read
  bilinearly; cells without data count as `elevation.nodata_m`.
- **No subtraction:** the terrain model carries no buildings or trees,
  so the roof subtraction, median and grey opening of the Copernicus path
  are off; one box blur over 6 m (`blur_radius` 1, one pass) only takes
  the 1 cm steps and single pixels out.
- **Water:** the model shows land; over the sea and the tidal flats it
  holds a fill value (about +1.3 m here). Sea and tidal flats come from
  OSM as before.
- **Comparison:** `just gen-geo-compare <site> <copernicus.tif> <x>
  [dir]` (`geo_import.py compare-dem`) draws both reliefs in the same
  colors (`dem_compare.png`, Copernicus above, the terrain model below),
  a profile across the map at `x` (`dem_profile.png`, `.csv`) and prints
  the mean slope and the share of land steeper than 60 % for both.
  Copernicus tiles north of 50° have pixels twice as wide in longitude;
  `extract_dem` reads them now (square tiles as before).

#### Options of the fourth site (prompt 114)

All off unless a `site.json` sets them (`harbor_city`, `brick_quarter`
and `coastal_heights` regenerate bit for bit, `just gen-geo-check`).

| Option | Effect |
|---|---|
| `extract --dgm`, `sources.dgm_zone`, `sources.dgm_margin_m` | the second elevation source above; `generate` picks it by the format of `dem.json.gz` |
| `elevation.blur_radius`, `blur_passes`, `nodata_m`, `lattice_samples` | for the terrain model: box blur, height of cells without data, lattice spacing of the transformation (default 16) |
| `coast.tidal_flat` | water inside OSM `wetland=tidalflat` within `mud_m` of the land becomes mud flat (`mud`, land, from `mud_top_m` at the shore to `mud_edge_m` at its outer edge), the next `shallow_m` stay `shallow_water` (bed at least −0.4 m); widths grow in rounded steps (octagons, not squares) |
| `areas.open_sand` | OSM `natural=sand` and `natural=dune` areas (from `min_area_m2`) are `sand`, over heath and scrub |

#### Long bridges over water (prompt 115)

`site.json` may list `long_bridges` (`river_crossing` does, prompt 116,
and `rhine_bridge`, prompt 118); without the key nothing changes (the other geo
maps regenerate bit for bit). The bridge is given, not detected from OSM:

```json
"long_bridges": [{
  "name": "waal", "axis": "x", "line_m": 497, "start_m": -327, "deck_m": 16.0,
  "pieces": [["env_bridge_steel_approach", "Span30"], ["env_bridge_steel_arch", "Arch244"],
             ["env_bridge_steel_approach", "Span30"]],
  "ramp_grade": 0.12, "crest_half_m": 10, "side_grade": 1.0, "abutment_m": 8, "road_half_m": 5
}]
```

- `axis` "x" or "y" (map axes; turn the frame with `x_axis_bearing_deg`
  so the real bridge runs along one), `line_m` the other coordinate of the
  axis: it must lie on a center line of the 4 m path cells (`-half + 1 +
  4 k`), otherwise the importer stops and names the two nearest lines.
- The pieces follow each other from `start_m` in the positive direction
  (lengths from `BRIDGE_PIECES` in the importer, matching
  `data/objects.toml`) and are written first in `objects.csv`.
- At each outer end an **earth dam** up to `deck_m`: `abutment_m` at
  deck height under the deck end, then falling at `ramp_grade` until it
  meets the ground, crest `crest_half_m` either side of the axis with a
  road (`road_half_m`) on it, sides at `side_grade` (100 %: too steep to
  climb). Where the ground is higher than the dam it stays (the deck then
  rises to it: the deck height comes from the ground at the outer ends).
- OSM ways tagged `bridge` that run mostly inside the bridge corridor are
  not painted (the bridge carries the road). Buildings keep off the dams
  and the ground under the bridge. Roads under the approaches and across
  the dams end there (no underpass): the stats count them
  (`long_bridges[].road_under_or_cut_m2`); route them round the dam ends
  with `roads.extra`.

Tried on a copy of `harbor_city` with a 304 m bridge (Span30, Arch244,
Span30) over the inner harbor (`build/geo115/`, not committed): the map
check passes, tanks reach the deck over the west dam (`map-reach`).

- Prompt 116: `level_high_ends` levels a pad at an outer end whose bank
  is higher than `deck_m` (a town on a terrace, where no dam is built):
  at the ground height of the end, `end_pad_half_m` either side of the
  axis, `end_pad_m` beyond the end and `end_pad_under_m` under the deck
  (kept short: flat ground beside the last metres of the deck let heavy
  tanks try to climb on from the side).

#### `dune_coast` ("Dünenküste")

4.6 km × 3.2 km (2 301 × 1 601 samples, 18.4 MB in memory), biome
`forest` (there is no coast biome), heights −6 … 50.3 m (1 mm steps).
The island of the place named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-dune-coast-prompt-114)
around 1944, simplified like `harbor_city`: the North Sea with a sand
beach along the top of the picture, behind it the cliff and a belt of
dunes 10–30 m high with single tops over 40 m (the highest 50.3 m),
hollows and valleys between them every 50–200 m; the old plateau with
heath, fields and the village in the middle; marsh, meadows and a belt
of mud flats along the tidal flats at the bottom. The frame is turned to
15° so the island runs along the x axis (north to the right). Four
starts in two teams: team 0 south at (−2100, 200) and (−2100, 600)
(fields and heath of the plateau), team 1 north at (2080, 300) and
(1700, 470) (meadow and marsh between the north dunes and the flats),
each levelled 34 m around with 80 m kept free.

**Copernicus against DGM1** (`just gen-geo-compare dune_coast
<tile N54 E008> -486`, profile across the island through the highest
dune, x = −486 m):

![Relief from Copernicus (above) and DGM1 (below)](images/dune_coast/dem_compare.jpg)

![Profile across the island at x = −486 m: Copernicus orange, DGM1 blue; lines every 5 m and every 200 m, y from −1 600 (sea, left) to 1 600 m (tidal flats, right)](images/dune_coast/dem_profile.jpg)

| y (m) | Copernicus (m) | DGM1 (m) |
|---|---|---|
| −920 | 0.9 | 2.3 |
| −880 | 6.3 | 11.6 |
| −840 | 19.7 | 25.9 |
| −760 | 33.0 | 34.6 |
| −680 | 35.1 | 36.5 |
| −640 | 40.3 | 49.1 |
| −600 | 44.8 | 45.0 |
| −560 | 38.6 | 36.5 |
| −480 | 30.8 | 30.7 |
| −400 | 27.6 | 28.6 |

Copernicus rounds the dune belt into a few broad humps: the top is
44.9 m instead of 50.3 m (the dune is 52.5 m by the map), the rise from the
beach to the first dune tops (about 25 m) takes some 120 m instead of
80 m, single dunes and hollows of 50–100 m
vanish. On the land of the map the mean slope is 5.6 % (Copernicus)
against 8.1 % (DGM1); steeper than 60 % are 0.0 % against 0.61 % of
the land: only with the terrain model do the lee sides and the cliff
block vehicles, as the slope rules say. On the plateau both agree within
1–2 m.

What the simplification kept (`just gen-geo dune_coast`, stage pictures
with `--report`):

| Data | OSM / DGM1 in the map | In the game |
|---|---|---|
| Heights | 28 DGM1 tiles (28 M points) | 2 m cells, transformation error < 1 mm, 198 samples without data (at sea) |
| Roads | 96.6 km of ways | 29.5 km of 188 roads and sand tracks (521 instead of 1 007 points); dropped: 46.8 km foot and cycle ways (including the beach crossings), 8.9 km service roads and parking aisles, 10.8 km streets of the neighbouring village's north edge (later, polygon), 0.6 km steps; residential streets as 6 m sand tracks |
| Coast | 90 coastline points | 5 081 coast samples; 93.6 ha of mud flats and 27.6 ha of shallow water inside the tidal flats (OSM `wetland=tidalflat`), deep water beyond |
| Buildings | 846 footprints | 237 blocks in the old village core got 260 buildings (farm, brick and stone houses), 137 blocks outside skipped (holiday houses after 1950); 18 levelled pads |
| Areas | land use, natural and leisure areas | 55 heath and scrub areas (tall grass, 26 % of the map), 12 woods, 8 beaches, 7 parks, 7 pitches; no open sand area large enough (`open_sand`) |
| Green | – | 1 753 trees (woods and parks only, none in the dunes), 307 bushes, 201 hedge pieces |
| Works | none in the frame (one bunker ruin 75 m beyond the north edge) | 10 invented places on dune crests facing the sea: 10 pillboxes, 29 trench sections, 20 sandbag positions |

Reachability (`just map-reach dune_coast`; the picture is the slope
map: red is too steep): of 10.38 km² of land (4.34 km² water) 94.6 % is
open to vehicles and infantry (mud and sand are only slow), 0.7 % to
infantry only (woods), 4.1 % to nobody (the cliff edge, the lee sides of
the dunes, the shore step above the mud flats) and 0.6 % is covered by
buildings and objects; 99.9 % of the open ground hangs together. Every
start and supply point lies in that main part. Besides the main road
along the island, vehicles cross the heath and the marsh along the
flats; the dunes are open to them except at their steep sides.

Supply points in pairs, balanced by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | South | North |
|---|---|---|---|
| `supply_village` | crossroads in the village (contested) | 2 129 / 1 899 | 2 465 / 2 163 |
| `supply_high_dune` | top of the highest dune (contested) | 2 374 / 2 042 | 2 961 / 2 695 |
| `supply_cliff` / `supply_north_dune` | dune above the cliff / dune in the north belt | 1 860 / 1 360 | 1 223 / 1 082 |
| `supply_heath_south` / `supply_heath_north` | heath in the south / heath north of the village | 918 / 795 | 1 584 / 1 455 |
| `supply_flat_south` / `supply_flat_north` | edge of the flats in the south / in the north | 1 808 / 1 591 | 827 / 829 |

Over all points the north is 29 m shorter by vehicle and the south
537 m shorter on foot [provisional]; both contested points lie a little
nearer to the south. The points on the cliff dune and the north dune lie
20–30 m beside their tops on the highest ground vehicles reach (the tops
themselves are cut off by steep sides).

Sight (`just map-sight dune_coast`, sight 250 m in the game): the
highest dune sees 15.2 ha (10.2 ha of it 20 m or more below), the north
dune 15.3 ha, the cliff dune 8.2 ha, the village crossroads 7.5 ha
(houses), open heath and flats 11–19 ha; inside the dune belt the
hollows hide most of the ground.

Markers for tools and shots: `village`, `high_dune`, `north_dune`,
`cliff`, `heath`, `flats`, `start_south`, `start_north`. Pictures:
`SHOT_SET=land just terrain-shots <dir> dune_coast` adds the cliff from
the sea, both dune belts, the village, the heath, the flats and a dune
hollow at 60 m. With the `forest` biome heath and dunes look like
rolling grass (tall grass up close), the beach is light sand, the flats
dark mud; a coast biome is open. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#dune-coast-prompt-114).

Open: a coast biome (marram grass, heath, bright light, North Sea
colours); thatched houses and other buildings of the island as own
models; tides (the flats turning to water and back) as a rule; moving
sand and tracks in the dunes; tidal creeks (neither OSM nor the terrain
model has them here); terrain models of other German states (mostly the
same format).

#### A third elevation source and a river site (prompt 116)

`river_crossing` lies on a river, in the Netherlands. Its options are off
unless a `site.json` sets them (the other geo maps regenerate bit for
bit, `just gen-geo-check`).

| Option | Effect |
|---|---|
| `extract --ahn T1.tif …` | the Dutch terrain model **AHN** (DTM 0.5 m, CC0) as GeoTIFF tiles of the PDOK coverage service in RD New (EPSG:28992, float32, deflate, strips): reduced to 2 m cells over the frame plus `sources.dgm_margin_m` (`dem.json.gz` format `geo_import dem rd v1`); the DTM has no data under buildings and over water, those cells are filled ring by ring from their neighbours. `generate` carries every sample to RD on the lattice of prompt 114 (`wgs_to_rd`: the polynomial of Schreutelkamp and Strang van Hees, about 0.25 m off RDNAPTRANS, only `+ - *`) |
| `elevation.offset_m` | added to every height: a river site puts its water level at 0 (here −8 m: the Waal at about NAP +8 m) |
| `coast.sea_seeds_m: []` | no sea: the water comes from OSM water areas (`lakes_min_m2`, the river as a `natural=water` multipolygon) |
| `coast.skip_water_at_m` | water areas whose outer ring's mean point lies within 10 m of one of these points are not read (water dug after 1944) |
| `coast.dry_polygons_m` | inside the polygon water is land and the ground is held at most at `max_m` (a channel, its island and its new dike back to the polder of 1944) |
| `coast.land_strips_m` | a dam across narrow water (`pts_m`, `half_m`, `height_m`) where a road crosses it: the small bridges of the place are not in the game |
| `roads.causeway_max_m`, `causeway_side_m` | a kept road that crosses water no wider than this along its line runs on a dam (`causeway_side_m` beside the road); short bridge ways over water are then kept |
| `roads.minor_skip_polygons_m`, `minor_classes` | minor streets mostly inside the polygons are dropped (a new town on the polder; its main roads stay) |
| `buildings.drop_from_year`, district `drop_postwar` | footprints with an OSM `start_date` from that year on are not read in such districts (the Dutch register gives nearly every building its year) |
| district `layout`, `ruin_share`, `damaged_share` | a district's own layout (`perimeter` in the town, rows in the village) and shares of ruins and damaged houses (the fighting in the old town) |
| `long_bridges[].level_high_ends` | see "Long bridges over water" above |

#### `river_crossing` ("Flussübergang")

3.6 km × 3.2 km (1 801 × 1 601 samples, 14.4 MB in memory), biome
`forest`, heights −6 … 40.2 m over the river (1 mm steps). The town,
river and polder of the place named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-river-crossing-prompt-116)
around September 1944: the frame is turned to 70.5° so the long steel
bridge runs along the map's y axis (up is 19.5° west of north);
the old town on a terrace 15–30 m over the river in the south-west, with
the castle hill and the park at the bridge's south end; the river,
250–400 m wide, bending from the east to the north-west; the floodplain,
the old dike and the polder with the village in the north; low meadows
with an old river arm and a lake in the east. The bridge (x = 9 m, from
y = −24 to 580 m): three 60 m and one 30 m approach spans over the north
floodplain and the river's north part, the 244 m arch over the deep
channel, two 60 m and one 30 m spans over the quay and the bank; deck
14 m over the river at the north end (an earth dam that meets the old
dike) and 18.4 m on the town's terrace at the south end (a levelled pad);
it is the only crossing for vehicles. The railway bridge west of it is
left out (no model; its embankments end at the river), as are the modern
bridge further west and the cycle bridges.

**Stand 1944:** the side channel of 2013–2016 north of the river, its
island and the dike moved inland are polder again; the old dike remains
on the former island and meets the bridge's dam. North of the river and
in the eastern meadows every building of the register from 1946 on is
left out (4 675 footprints): the new town on the north bank goes, the old
village stays (157 houses), with the minor streets of the new quarters
(240 ways, 27.8 km). In the town all footprints stand for the houses of
1944 (rebuilt on the old plan), at most 22 m tall by their tags; the old
town has 12 % ruins and 20 % damaged houses [provisional].

| Data | OSM / AHN in the map | In the game |
|---|---|---|
| Heights | 30 AHN tiles (15 M points of 1 m) | 2 m cells, 986 988 cells (of 5.3 M) filled under buildings and water, lattice error < 1 mm |
| Water | the river as one multipolygon, 59 lakes and ponds from 3 000 m² | 2.02 km² water; the side channel, its inlet and two later ponds left out; a dam across the old river arm where the road crosses it, one causeway |
| Roads | 265.6 km of ways | 82.0 km of 1 043 roads and tracks; dropped: 115 km foot and cycle ways, 23 km service roads, 27.8 km minor streets of the new quarters, 4.6 km tunnels and covered ways, 1.5 km bridges over the river (the modern bridge), 9 bridge ways of the long bridge's corridor |
| Buildings | 9 427 footprints | 796 blocks got 3 944 buildings: old town 1 020, town 2 756, village 157, meadows 11 (perimeter blocks in the town, rows in the village) |
| Green | parks, woods, pitches | 4 372 trees, 853 bushes, 490 hedge pieces |

Four starts on both banks [provisional]: team 0 (west) in the town at
(−1250, 1000) and in the polder north-west at (−1150, −1150), team 1
(east) on the meadows at (1150, 1100) and in the polder north-east at
(900, −1300); each levelled 34 m around and kept 80 m free. In a 1v1
(starts 0 and 1) both sides start on the south bank and meet in the town
and the meadows; with four players each team has a foot on both banks,
so holding the bridge does not lock one side out.

Reachability (`just map-reach river_crossing`): of 9.50 km² of land
(2.02 km² water) 85.4 % is open to vehicles and infantry, 1.4 % to
infantry only (woods), 4.7 % to nobody (terrace edges, the dams' sides,
quays) and 8.6 % is covered by buildings and objects; 98.8 % of the open
ground hangs together, over the bridge. Every start and supply point
lies in that main part.

Supply points, balanced in pairs by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | West | East |
|---|---|---|---|
| `supply_bridgehead_south` | park at the bridge's south end (contested) | 1 541 / 1 500 | 1 547 / 1 271 |
| `supply_bridgehead_north` | the polder at the north end (contested) | 1 595 / 1 526 | 1 686 / 1 388 |
| `supply_castle_hill` / `supply_meadow_east` | castle hill / meadow by the river in the east | 1 277 / 1 219 | 1 344 / 1 144 |
| `supply_west_bank` / `supply_dike_north` | floodplain west of the bridge (where the boats landed) / dike east of the village | 923 / 923 | 1 006 / 911 |

Over the six points the east is 329 m longer by vehicle [provisional].
Markers for tools and shots: `bridge`, `bridgehead_south`,
`bridgehead_north`, `castle_hill`, `old_town`, `village_north`,
`polder`, `meadow_east`, `west_bank`, `dike_north`, `start_west`,
`start_east`, `blocks`. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#river-crossing-prompt-116); the mission
"The great bridge": [MISSIONS.md](MISSIONS.md).

Open: the railway bridge (scenery without a roadway) and the boats of
the crossing; orchards of the polder (not in today's data); the
historical map of 1944 could not be placed exactly over the frame, so
the old dike and the village were judged from today's data (the dike on
the island, the register's years); an underpass under the bridge's
south spans (the quay road ends there, prompt 115); one heavy tank of a
group still turns back beside the deck's south end in `just stuck-audit`
(1 of 100 orders).

#### A historical core (prompt 118)

The quarter at the north end of the bridge of `rhine_bridge` was
destroyed in 1944 and rebuilt on another plan, so today's OpenStreetMap
data do not show it. A site may therefore name a **core**: a polygon in
which the town of 1944 comes from a file traced from a historical map
instead of today's data. Without the `core` key nothing changes (the
other geo maps regenerate bit for bit, `just gen-geo-check`).

| Option | Effect |
|---|---|
| `core.file` | the core file next to `site.json` (`core_1944.json`): `blocks`, `water` and `parks` as rings in degrees × 1e7 (traced), `streets` (`highway` class, `pts_m`), `buildings` (`tags`, `pts_m`) and `landmarks` (`kind`, `variant`, `at_m`, `heading`) in map meters (by hand) |
| `core.polygon_m` | inside, today's footprints, areas and ponds (below `water_keep_m2`, so not the river) are not read; footprints with an `amenity` of `keep_amenity` stay |
| `core.street_block_share`, `street_near_share`, `street_near_m` | today's streets mostly inside are kept only if at most this share of their length lies over the old blocks (and, if set, at least `street_near_share` within `street_near_m` of them): the axes that survived keep their measured lines |
| `core.cleared_polygons_m` | ground rebuilt after the old map's edition (here the bridge ramp of 1935 over the filled moat): today's streets there are not read, the old blocks, ponds and parks there are not traced |
| `core.street_max_m` | the old streets: every sample of the core between old blocks that a closing of the blocks fills (gaps up to `street_max_m`, then a second closing twice as wide for the crossings) becomes road, except ponds, parks and cleared ground; open yards around single buildings stay |
| `core.year`, `block_min_m2` | for the tracer: the Topotijdreis edition and the smallest block |

**Tracing** (`tools/geo_trace_core.py`, a one-time aid with numpy and
ImageMagick, not part of `generate`): the Kadaster's historical
topographic map (Topotijdreis tile service, RD New tiling, level 11 =
1.59 m per pixel) is warped into the map frame (an affine fit of the RD
transformation over the core, below 1 m off) and classified by colour:
built-up blocks are salmon, single buildings red, water grey-blue, parks
green, streets the white ground between blocks with black casings. Blocks
(closed over text and church dots, opened against noise), ponds and parks
become rings (Moore tracing, Douglas-Peucker 1.5–2 m). Street centre lines
were tried as the skeleton of the white ground and given up (casings and
text cut it into pieces); the road surface between the blocks (above)
replaces them. Hand-made parts of the core file stay when the tracer runs
again.

#### `rhine_bridge` ("Rheinbrücke")

3.0 km × 3.2 km (1 501 × 1 601 samples, 12.0 MB in memory), biome
`forest`, heights −6 … 61.5 m over the river (2 mm steps). The town,
river and polder of the place named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-rhine-bridge-prompt-118)
around September 1944: the frame is turned to 121° so the road bridge
of 1935 runs along the map's y axis (up is 31° east of north); the town
north of the river rising to the wooded hills in the north-west (up to
61 m), the old town west of the bridge's north end, works and quarters
east of it, the railway across the middle; the river 140–160 m wide
from the east to the west; the floodplain with the harbour basin in the
west and the polder with its village and dikes in the south. The bridge
(x = 1 m, from y = 305 to 845 m): two 60 m approach spans over the north
bank, the 120 m arch (`Arch120`) over the river, five 60 m spans over the
floodplain to the south; deck 15 m over the river; a short dam at the
north end (the town lies 10–12 m over the river), a 6 % dam at the south
end; it is the only crossing for vehicles.

**The core of 1944** (the semicircle of 640 m around the north end):
107 blocks, two ponds and two parks traced from the topographic map of
the 1930s (the Topotijdreis edition "1944" in this place predates the
bridge); 973 of today's footprints and 335 areas inside are not read;
293 of today's streets (11.8 km, among them the interchange of the modern
bridge) are dropped, 463 kept where the old map shows open ground; 9.8 ha
of old streets painted between the blocks. By hand: the road down the
ramp to the ring street, streets on both sides of the ramp (the filled
moat), three blocks of houses beside the ramp (the defenders' positions;
outlines judged, the old map shows no ramp), the great church of the old
town as a damaged `env_church` [provisional]. Outside the core the town
keeps all footprints as stand-ins (perimeter blocks, at most 22 m tall
by their tags, 2 % ruins, 6 % damaged); south of the river and on the
former fields east of the town ("broek" district) every footprint from
1946 on is left out (8 390), with the minor streets of the post-war
quarters south of the river (222 ways, 26.1 km); 17 lakes and ponds of
later clay pits and parks south of the river are not read
(`skip_water_at_m`).

| Data | OSM / AHN in the map | In the game |
|---|---|---|
| Heights | 25 AHN tiles (25 M points of 1 m) | 2 m cells (2 243 × 2 278), gaps filled under buildings and water, lattice error < 1 mm |
| Water | the river as an OSM multipolygon, 38 lakes and ponds from 3 000 m² | 0.53 km² water |
| Roads | 331 km of ways | 118.9 km of 1 442 roads and tracks; dropped: 135.6 km foot and cycle ways, 32.8 km service roads, 26.1 km minor streets south of the river, 11.8 km in the core, 5.9 km tunnels and covered ways |
| Buildings | 16 322 footprints | 1 138 blocks got 4 924 buildings (town 4 510, broek 225, south 189) and the church |
| Green | parks, woods, pitches | 7 547 trees, 1 779 bushes, 903 hedge pieces |

Four starts on both banks, the solution of `river_crossing`
[provisional]: team 0 (west) in the town at (−1150, −250) and in the
polder at (−1000, 1250), team 1 (east) in the town at (1150, −250) and
in the polder at (1000, 1250); each levelled 34 m around and kept 80 m
free. In a 1v1 (starts 0 and 1) both sides start north of the river
and meet in the town; with four players each team has a foot on both
banks, so holding the bridge does not lock one side out.

Reachability (`just map-reach rhine_bridge`): of 9.07 km² of land (0.53
km² water) 83.3 % is open to vehicles and infantry, 2.2 % to infantry
only (woods), 3.0 % to nobody (steep slopes of the hills, the dams'
sides, quays) and 11.5 % is covered by buildings and objects; 98.7 % of
the open ground hangs together, over the bridge. Every start and supply
point lies in that main part.

Supply points, balanced in pairs by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | West | East |
|---|---|---|---|
| `supply_bridgehead_north` | the yards east of the ramp (contested) | 1 616 / 1 354 | 1 529 / 1 186 |
| `supply_bridgehead_south` | the dam foot south of the bridge (contested) | 1 215 / 1 111 | 1 183 / 1 032 |
| `supply_old_town` / `supply_works_east` | old town west of the ramp / works east of it | 967 / 918 | 1 029 / 869 |
| `supply_dike_west` / `supply_dike_east` | the polder dike | 512 / 512 | 504 / 504 |

Markers for tools and shots: `bridge`, `bridgehead_north`,
`bridgehead_south`, `ramp_west`, `ramp_east`, `old_town`, `church`,
`works_east`, `polder`, `dike_west`, `dike_east`, `start_west`,
`start_east`, `blocks`. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#rhine-bridge-prompt-118); the mission
"Holding the bridge": [MISSIONS.md](MISSIONS.md).

Open: the ramp of 1935 was a long embankment with a viaduct over the
quay roads down to a roundabout; in the game the deck ends over the bank
and the ramp is a street on the ground (no underpass, prompt 115); the
outlines of the houses beside the ramp are judged; the street centre
lines of the core were not traced (the old streets are road surface, only
the axes that survived are levelled); the post-war row houses south of
the river partly stood in 1944 (an aerial photograph shows rows north
of the old village); Kadaster editions after 1935 were not found in the
tile service.

#### Options of the seventh site (prompt 121)

`landing_gulf` is a Mediterranean coast with a sand beach for landing
craft. Its options are off unless a `site.json` sets them (the other geo
maps regenerate bit for bit, `just gen-geo-check`).

| Option | Effect |
|---|---|
| `coast.beach_bed` (`reach_m`, `bed_m`, `shallow_above_m`) | water within `reach_m` of an OSM beach gets a gentle bed of its own (here −0.3 m at the shore, 0.04 m deeper per 2 m, down to −6 m), and `shallow_water` where the bed is at least `shallow_above_m` (−1.0 m): landing craft run aground 10–30 m out (an LCVP draws 0.9 m, an LCM 1.2 m) and the men wade ashore; the rest of the coast keeps `bed_m` (here −2 m at the shore, 0.6 m deeper per ring: rocks stay an edge) |
| `coast.bed_rings` | rings of the sea bed (default 40, here 80: 160 m) |
| `coast.beach_rings` | the beach's rise (`beach_m`) carried this many rings inland (default 8, here 30, 10 %); the sand stays 8 rings wide. Without it the ground behind the beach rose in one step where it is higher than the beach |
| `areas.fields` (`polygons_m`, `parcel_m`, `kinds`, `seed`, `max_grade`) | Mediterranean biome: OSM farmland, meadows, farmyards and the given polygons (ground under later buildings, the airfield) as a patchwork of parcels (`parcel_m` along the map axes), each a field kind of `kinds` (terrain ids: grain stubble, green rows, ploughed, grass) from a fixed random stream; only grass, not steeper than `max_grade`, not in woods, parks, scrub, groves or vineyards |
| `green.walls_from_osm` | Mediterranean biome: OSM `barrier=wall` lines as field-stone wall pieces (`env_wall_field`, 3 m, a few low or broken), `barrier=retaining_wall` as terrace wall pieces (`env_terrace_wall`, 4 m) with their soil side to the higher ground (needs `barrier` in `sources.extra_tags`) |
| `green.terraces` (`lattice_m`, `min_grade`, `step_m`) | Mediterranean biome: terraces in olive groves and vineyards on slopes: on a lattice, where the ground is steeper than `min_grade` and a contour (every `step_m` of height) passes within one lattice step downhill, a terrace wall piece along the contour (the ground stays the map) |
| `fortifications.wire`, `fortifications.mines` | barbed wire along lines (3 m pieces in runs of `run_m` with gaps of `gap_m`, so infantry gets through) and mine sections (`fort_mine_ap`/`fort_mine_at` at points; 20 m × 20 m of neutral mines when a match starts), each with its `source` |

#### `landing_gulf` ("Landung im Golf")

5.0 km × 4.4 km (2 501 × 2 201 samples, 27.5 MB in memory), biome
`mediterranean`, heights −10 … 353 m (6 mm steps). The gulf of the place
named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-landing-gulf-prompt-121)
around June 1944, simplified like `harbor_city`: the frame is not turned
(x east, y south), so the sea lies at the bottom of the picture (the view
from the sea); the gulf, 2.5 km wide at its mouth, reaches 2.6 km into the
land; at its head a sand beach of 1.2 km curves from the village and its
harbour in the south-west to the north-east; behind it the plain, flat
fields 1–35 m high, up to the north edge (where the airfield lies today);
in the south-west the heights of the headland (up to 160 m, the
lighthouse height 130 m) between the gulf and the open sea; in the east
and north-east hills up to 350 m with the coast road, a small cove with a
beach and the point at the gulf's mouth; in the north-west the first
slopes towards the island's mountain. Four starts in two teams:
**hinterland against coast** [provisional]: team 0 north at (−550, −1760)
(fields where the airfield is today) and (−1500, −1200) (the plain), team
1 south at (−1420, 200) (west of the village) and (−2210, 280) (the coast
plain in the west), each levelled 34 m around with 80 m kept free.
West against east over the gulf was not chosen: the east has no level
ground for a start (every 100 m cell of the eastern hills rises more than
10 m within 64 m) and its routes to the beach and the village run over
the hill roads; both teams of the chosen layout start on the plain.

Heights: the surface model with an opening of 5 × 5 pixels and a blur of
14 m twice (as `harbor_city`); the hills keep their shape, cliffs are
rare in the 30 m model. The sea bed: rocks −2 m at the shore, 0.6 m
deeper per 2 m, to −10 m (a step from the land: an edge for everyone);
in front of the beaches the gentle `beach_bed` (see above): shallow water
about 36 m wide, LCVPs ground 10–15 m out, LCMs about 30 m. The beach
rises at 10 % to the ground behind it.

The state of 1944 [provisional, by general knowledge of the place]: the
airfield (runway, aprons, hangars and terminal) is not read, its ground
and the later town around the old village are fields in parcels of 96 m
× 64 m (stubble, green rows, ploughed, grass), the later town's minor
streets are dropped (14.9 km) and its blocks get no houses (394 blocks,
hotels and camp sites with them); the old village (a polygon around the
harbour and the main street, about 600 m × 400 m) has row houses, casali
and a church with a campanile; elsewhere farmhouses stand where OSM has
buildings (the plain and the slopes). Olive groves on the slopes get
terrace walls along the contours (325), OSM's field-stone walls and
retaining walls stand as walls (54 pieces, 6 terrace pieces); the hills'
macchia is tall grass with macchia bushes, pines, olives and cypresses.

What the simplification kept (`just gen-geo landing_gulf`, stage
pictures with `--report`):

| Data | OSM in the map | In the game |
|---|---|---|
| Roads | 109.1 km of ways | 53.9 km of 302 roads and tracks (1 575 instead of 4 006 points); dropped: 28.8 km foot and cycle ways, 14.9 km minor streets of the later town (site polygon), 10.9 km service roads, 0.2 km steps, 0.1 km in tunnels, 0.1 km short dead ends; carriageways and junctions closed (2.4 ha) |
| Coast | 862 coastline points | 6 798 coast samples (2 m); the harbour's jetty as land, floating pontoons dropped |
| Buildings | 1 788 footprints (75 skipped by tag: hotels, hangars, terminal, ruins, roofs) | 852 blocks; 394 of the later town and 39 of the airfield skipped; 538 buildings from blocks of at least 150 m² (144 in the old village, 394 farmhouses); 269 levelled pads |
| Areas | land use, natural and leisure areas | 45 scrub areas (tall grass), 48 woods, 9 orchards (olive groves), 11 vineyards (green rows), 10 beaches, 36 farmland areas plus the two polygons as 162 ha of fields |
| Green | – | 7 412 trees (woods 10 m apart, to keep the frame rate), 13 palms, 12 225 macchia bushes, 77 garden walls, 54 field-stone wall pieces, 331 terrace wall pieces |
| Works | no bunker in OSM | 6 works from the sources (6 pillboxes, 14 trench sections, 10 sandbag positions), 253 pieces of barbed wire on the beach, 2 mine sections |

Buildings per district (`site.json`, `districts`; blocks / buildings):
old village 85 / 144, later town 394 / 0 (skipped), airfield 39 / 0
(skipped), rural 334 / 394. Kinds: 332 casali, 133 corner row houses, 60
two-storey houses, 11 small houses, a barn and the church with campanile.

Reachability (`just map-reach landing_gulf`): of 14.54 km² of land
(7.46 km² water) 86.8 % is open to vehicles and infantry, 1.4 % to
infantry only, 11.1 % to nobody (steeper than 60 %, the rock coast's
edge, the wire) and 0.8 % is covered by buildings and objects; 98.4 % of
the open ground hangs together. Every start and supply point lies in that
main part for both classes; the hill roads carry vehicles over the
eastern hills and the headland.

Supply points in pairs, balanced by path length (vehicles / infantry, m,
from the nearer start of each side):

| Point | Place | North | South |
|---|---|---|---|
| `supply_beach` | behind the beach (contested) | 1 405 / 1 136 | 1 275 / 1 099 |
| `supply_harbor` / `supply_junction` | the square at the harbour / the road junction into the hinterland | 2 061 / 1 790 | 775 / 736 |
| | | 1 130 / 940 | 2 351 / 1 643 |
| `supply_west_height` / `supply_east_height` | the lighthouse height on the headland / the ridge east of the beach | 3 654 / 3 200 | 1 771 / 1 768 |
| | | 2 212 / 1 852 | 3 433 / 2 518 |
| `supply_plain` | the plain west of the beach | 399 / 325 | 1 164 / 1 110 |

Over all six points the north is 90 m longer by vehicle and 370 m on
foot [provisional]; every point lies where vehicles get to. There is no
boat in free play (prompt 119: boats only in missions), so no side lands;
the beach keeps the gentle bed for the mission. Markers for tools and
shots: `beach`, `village`, `plain`, `west_headland`, `east_ridge`,
`cove_east`, `east_point`, `gulf` (on the water), `start_north`,
`start_south`. Pictures: `SHOT_SET=land just terrain-shots <dir>
landing_gulf` adds the gulf from the sea, the beach, the village, both
slopes, the plain, the point and a beach view at 60 m. Measurements:
[PERFORMANCE.md](PERFORMANCE.md#landing-gulf-prompt-121). Mission:
[MISSIONS.md](MISSIONS.md#landing-in-the-gulf-prompt-121).

Open: coast guns and casemates as models; a historical map of the
village and the plain in 1944; the flak ship and the jetty's state in
1944; heights only from the 30 m model (no official terrain model of the
region was read); boats in free play.

#### Options of the eighth site (prompt 127)

`castle_rock` is a city on volcanic rocks with cliffs, a valley with a
railway and a small river with stone bridges. Its options are off unless
a `site.json` sets them (the other geo maps regenerate bit for bit,
`just gen-geo-check`). One fix applies to every site but changes none:
Copernicus tiles above 50° have fewer pixels per degree of longitude
(`px_per_deg_lon`); `dem_heights` now uses it for the columns (before only
official terrain models were used that far north).

| Option | Effect |
|---|---|
| `elevation.raw_polygons_m` | inside, the roof-corrected surface model is used without median and opening: those filters shave a narrow rock or summit by 10–20 m (the castle rock lost 20 m) |
| `elevation.lifts` | heights from a source where the 30 m model is too low: a `polygon_m` lift raises the ground inside to at least `top_m` (a rock's plateau) and blends back over `feather_m`; a `peak_m` lift raises a summit (`top_m`, `radius_m`, (1 − d/r)²). After the cliffs the plateau is raised once more inside, so a face drawn along its rim does not dent it |
| `cliffs` (`band_m`, `min_drop_m`, `join_m`, `simplify_m`, `rock_m`, `skip_polygons_m`, `extra_m`) | OSM `natural=cliff` lines (low side to the right, as OSM draws them; the elevation model decides per piece where it clearly disagrees) sharpen the blurred rock faces: the fall within `band_m` on both sides is gathered into a step on the line (a sample at distance s moves towards the band's edge height by 1 − s/band), at least `min_drop_m` or the line's `height` tag; line ends closer than `join_m` are joined, a 5 × 5 median over the band removes spikes where two pieces change side; faces inside `skip_polygons_m` (terraces of a plateau) are left out; samples within `rock_m` of a line are reserved (no buildings, pads or trees) |
| `rails` (`kinds`, `skip_service`, `poles_every_m`, `pole_side_m`) | railway lines of the given kinds (here also lines closed after 1944) as ways of the road class `rail` (in `roads.classes`: a 7 m ballast bed of `dirt_track`, as the sketch generators build `[[rail]]`): tunnels drop out, a railway bridge over a road is dropped (the road stays), a road bridge over the tracks is dropped (the tracks stay), the bed is level across; telegraph poles beside the line |
| `roads.pedestrian_polygons_m` | pedestrian ways mostly inside (streets closed to traffic after 1944: the main street of the old town, a lane of shops) are streets of the class `pedestrian` |
| `roads.keep_bridges_at_m` | bridges whose mean point lies within 20 m of one of these points are kept although they cross a road: main streets over a lane in a deep, narrow cut the model does not show; both lie on the ground and meet like a junction |
| `stone_bridges` (`max_m`, `bank_m`, `max_skew_deg`) | a kept road that crosses water no wider than `max_m`, within `max_skew_deg` of a map axis, gets an `env_bridge_stone` (`Arch1` 20 m or `Arch3` 30 m) along that axis on the nearest line of path cell centres, spanning the water and `bank_m` of dry bank; the water stays under it, the causeways skip it |
| `areas.heath_polygons_m` | park ground inside the polygons is heath (tall grass, scrub without planted trees with `green.scrub_spacing_m: 0`): a bare hill mapped as one big park; its mapped woods stay |
| `sheds[].road_margin_m` | a line of sheds may keep a smaller margin from roads (station halls beside the tracks) |
| `landmarks` (`objects`, `walls`) | a landmark built from several catalog objects (the castle): `objects` (kind, variant, position, heading) and `walls` (kind, `pts_m`, `tower`): pieces of the catalog length end to end along every leg, a tower variant at every inner corner; placed before the town, which keeps off them |

#### `castle_rock` ("Burgfelsen")

3.6 km × 3.4 km (1 801 × 1 701 samples, 15.3 MB in memory), biome
`forest`, heights −3 … 233 m (4 mm steps). The inner city of the place
named in
[SOURCES.md](SOURCES.md#maps-from-real-geodata-the-castle-rock-prompt-127)
around 1944, simplified like `harbor_city`: the frame is turned to 73° so
the grid of the new town runs along the map axes and the long steel
bridge over the valley along y. From west to east: the castle on its rock
(cliffs on three sides, the only way in from the east over the ridge),
the old town on the ridge falling to the east, the valley with the park
and the railway between the old and the new town, the grid of the new
town, the small steep hill with a tower, and in the south-east the big
hill (233 m in the model, 251 m in reality) with crags over a park. In
the north the small river with four stone bridges, beyond it parks and
villas. A fictional fight, no historical battle.

Heights: Copernicus DEM with the buildings subtracted. On the castle rock,
the small hill and the big hill the surface model is used without median
and opening (`raw_polygons_m`); the castle's plateau is raised to 121 m
(the rock's summit is 130 m by the source; the model gives 116 m) and the
OSM cliff lines are sharpened (9.2 km, 234 lines, 92 joins): the rock
falls 40–60 m in a step on three sides, the crags of the big hill 20–30 m.
No summit lift on the big hill: adding the missing 18 m made its upper
flanks steeper than 60 % and cut the summit off [provisional].

The state of 1944: footprints with an OSM `start_date` from 1946 on or
taller than 45 m count as untagged; the parliament of 2004 and its
neighbours stand in an industrial district (the brewery ground of 1944:
warehouses); the shopping centres of today stand as rows of town houses
(the tenements and squares there before); tram lines and stops are not
read (`rail.kinds` without `tram`); streets closed to traffic later are
streets again. 1 % ruins, 3 % damaged (a fictional fight, the city was
hardly damaged).

What the simplification kept (`just gen-geo castle_rock`, stage pictures
with `--report`):

| Data | OSM in the map | In the game |
|---|---|---|
| Roads | 464.4 km of ways (with the railways) | 177.3 km of 1 906 roads, tracks and railway beds (4 813 instead of 14 387 points), 22.5 km of them railway; dropped: 208.8 km foot, cycle and pedestrian ways and closes, 54.6 km service roads, 12.4 km in tunnels or covered (the railway tunnels under the new town and the small hill), 8.9 km steps, 0.6 km bridges over roads and tracks (the lower way stays), 1.3 km stubs and fragments; carriageways and junctions closed (6.0 ha) |
| Water | the small river and 66 ponds and lakes of at least 300 m² | 7.0 ha of water; four stone bridges over the river, no causeway |
| Bridges | the steel bridge over the valley | three `Span60` approach spans (180 m) on x = −507 m, a dam and abutment at its north end; the valley floor and the street under it are closed beneath the deck (no underpass, prompt 115) |
| Rock | 234 cliff lines, 9.2 km | steps on three sides of the castle rock, the crags and smaller faces of the big hill |
| Buildings | 13 449 footprints | 1 198 blocks of at least 400 m² in street walls, 10 052 buildings (5 060 stone houses, 4 471 town houses, 334 tenement blocks, 83 churches, 49 public buildings, 54 warehouses, one station hall), 35 removed to keep ways open; 5 625 levelled pads, 118 left out |
| Castle | footprint of the castle | 51 curtain wall pieces with 9 flanking towers, gatehouse, half-moon bastion, three stone ranges, a chapel (prompt 127 models) on a paved plateau |
| Areas | land use, leisure and natural areas | 291 parks and gardens, 185 woods, 127 pitches, 61 scrub (tall grass), 45 industrial, 57 squares; 321 ha of town ground paved |
| Green | – | 9 633 trees (woods 10 m apart for the frame rate; the big hill bare heath, 206 ha), 3 860 bushes, 5 213 hedge pieces, 236 telegraph poles along the railways |

Buildings per district (`site.json`, `districts`; blocks / buildings):
castle 6 / 0 (the castle objects instead), brewery 10 / 74, old town 81 /
590 (5–7 storeys), new town 176 / 2 181 (3–4), north 364 / 2 537 (2–3),
south and east 561 / 4 669 (3–4).

Reachability (`just map-reach castle_rock`): of 12.17 km² of land 75.4 %
is open to vehicles and infantry, 3.2 % to infantry only (woods, hedges),
5.0 % to nobody (the cliffs, steep flanks, quay edges) and 16.3 % is
covered by buildings; 98.5 % of the vehicles' ground hangs together.
Every start and supply point lies in the main part for both classes. The
castle is open to vehicles only over the esplanade from the ridge; its
rock is closed on the north, west and south. The big hill is open to
vehicles over its grass slopes, not only over the park road: the rules
know no ground for infantry only except objects (woods, hedges, walls)
[provisional, open].

Starts **north against south-east** (decided with `map-reach`): team 0 in
the parks beyond the river at (−1450, −1560) and (−1050, −1600), team 1
in the park at the foot of the big hill at (1350, 640) and (1650, 420),
each levelled 34 m around, 80 m kept free. West against east was far more
uneven (the west start 0.9–1.3 km from castle, old town, station and new
town, the east 2.7–3.3 km).

Supply points (vehicles / infantry, m, from start 0 and start 1):

| Point | Place | North | South-east |
|---|---|---|---|
| `supply_castle` | the castle's inner ward (a dead end) | 2 818 / 2 341 | 3 316 / 3 038 |
| `supply_old_town` | the main street of the old town | 2 608 / 2 449 | 2 732 / 2 454 |
| `supply_station` | the station in the valley | 2 545 / 2 340 | 2 979 / 2 581 |
| `supply_new_town` | the square at the east end of the new town | 2 164 / 1 902 | 3 171 / 2 698 |
| `supply_small_hill` | the small hill | 2 757 / 2 557 | 2 492 / 2 002 |
| `supply_summit` | the summit of the big hill (sight over the town) | 6 106 / 5 000 | 988 / 990 |
| `supply_river_bridge` | the stone bridge over the river in the north | 718 / 649 | 5 557 / 5 125 |
| `supply_palace` | the palace forecourt at the foot of the hill | 3 694 / 3 374 | 1 673 / 1 243 |

Over all eight points the north is 0.5 km longer by vehicle (23.4 against
22.9 km) and 0.5 km on foot (20.6 against 20.1 km) [provisional]. Gun
positions (`height_*`, for `map-sight`): the castle, the small hill, the
summit, the top of the crags. Markers for tools and shots: `castle`,
`castle_approach`, `north_bridge`, `small_hill`, `summit`, `crags`, `station`,
`new_town`, `old_town`, `start_north`, `start_southeast`.

**Sight** (`just map-sight castle_rock`, sight 250 m): the castle sees
2 ha (walls and ranges stand around it; the town below lies beyond the
cliff edge and the walls), the small hill 2–4 ha, the summit 3.5–4.5 ha;
the top of the crags 13.5 ha, 6.3 ha of it 20 m or more below (the old
town's east end and the park): the only gun position that looks into a
valley towards the enemy, and only for team 0 coming from the north.
Pictures `build/castle_rock/final/sight_*.png` (`--only=height_castle`
and so on).

**Ranges** (game range = real range × 1/8; straight distances):

| From | Gun (catalog) | Game | What it reaches |
|---|---|---|---|
| `height_castle` | medium mortars | 300–380 m | the castle and the esplanade only |
| | heavy mortars | 469–756 m | the old town (541 m), the station (720 m), the new town square (752 m) |
| | 15 cm sIG, 152 mm, rocket launchers | 525–1 092 m | the valley and the new town; not the small hill (1 222 m) |
| | light howitzers and field guns | 1 099–1 541 m | the small hill (1 222 m); the river bridge (1 614 m) only from the 15 cm sFH 18 (1 666 m) |
| `height_small_hill` | heavy mortars | 469–756 m | the station (540 m), the new town square (643 m), the palace (653 m) |
| | light howitzers | 1 099–1 541 m | the castle (1 292 m), the river bridge (1 420 m) |
| `height_summit` | heavy mortars | 469–756 m | nothing but the park |
| | 15 cm sIG, rocket launchers | 525–1 092 m | the south-east start (875 m) |
| | light howitzers | 1 099–1 541 m | the palace (1 351 m), the second south-east start (1 225 m); no point of the town |
| `height_crags` | heavy mortars | 469–756 m | the palace (742 m); the summit (757 m) just beyond |
| | light howitzers | 1 099–1 541 m | the old town (1 356 m), the station (1 347 m), the small hill (1 162 m) |

From the castle and the small hill no gun reaches a start (1.7 km or
more); from the summit and the crags the south-east starts lie 0.9–1.3 km
away (sIG, rocket launchers, light howitzers), the north starts 2.9 km or
more.

Checks: `just map-check castle_rock` passes; the test
`castle_rock_plays_and_carries_no_real_names` (`crates/cli/src/map_reach.rs`);
`just stuck-audit --map=castle_rock`: 130 group orders, 416 units, 415
arrived, one rifle squad gave up at the deck of a stone bridge in the
north. AI and frame rate: [AI.md](AI.md#the-castle-rock-prompt-127),
[PERFORMANCE.md](PERFORMANCE.md#castle-rock-prompt-127). Pictures:
`SHOT_SET=land just terrain-shots <dir> castle_rock` adds the castle from
the valley, the ridge and at 90 m, the old town, the valley, the new town,
the small hill, the crags and the big hill.

Open: own models for Scottish stone tenements and the landmarks (the
houses are the game's placeholders, red tile roofs among them); ground
for infantry only (scree, gorse) so the big hill stays closed to
vehicles; an underpass under the steel bridge (the valley is cut there);
the summit 18 m too low; the official LiDAR terrain model of the country
(not used, see SOURCES.md); halls over the station tracks (one hall fits
beside them).

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
| top level | `name`, `size_m` (multiples of 4 m, at most 4 km), `biome`, `look` (prompt 122, written to `map.toml`), `weather`, `seed`, `water_flow_deg`; `base` = another sketch this one only changes (biome, look, weather, seed, size); `mirror = "north"` (prompt 123): the north half is generated and copied mirrored onto the south half, see below; `mirror = "point"` (prompt 124): the north half is turned by 180° onto the south half (point symmetry, below); `mirror = "west"` (prompt 130): the west half is generated and copied mirrored onto the east half (west against east, below) |
| `[[start]]` | player, team, position (60 m kept flat and free) |
| `[[supply]]`, `[[marker]]` | supply point markers (`supply_*`, kept free and flat) and other markers; `flat_m` (prompt 123): kept flat over that radius and free of objects (gun positions) |
| `[[river]]` | center line and width; banks cut 3 m into a 40 m valley, deep water in the middle, shallow at the edges, sand on the banks; `shallow = true` (prompt 124): a brook shallow over its whole width (no channel); `through_hills = true` (prompt 125): the river runs through hills in a valley: its water keeps the level of the plain ground (the ground noise without bumps and hills, as a `[[lake]] kind = "river"`) and the banks fall to it over `valley_m` (default 40 m, wider where the hills are high: a 30 m hill over 140 m stays passable) instead of the water climbing over the hills; rivers without it are unchanged |
| `[[island]]`, `[[lake]]` | islands in rivers, lakes and (prompt 122) the bays of sea maps (circle, or since prompt 123 `polygon_m`); lakes and sea (polygon, `deep` or `shallow`); `kind = "river"` (prompt 123): a broad river drawn as a polygon, its water 3 m below the plain ground (the ground noise without bumps and hills), so hills beside it fall to the water over a steep bank within 45 m instead of lifting it; islands in it slope to the bank level within 25 m of their shore; shoals in it stay shallow |
| `[[crossing]]` | `stone` or `wood` bridge or `ford` at a point of a river, `axis` `ns` or `we`: the axis is moved onto a path cell center line, the water narrowed to what the longest bridge of the kind spans (stone 21 m, timber 7 m), a straight approach and embankments with 12 % ramps are made, and the shortest fitting variant is placed (as `gen_map`). `stone_long` (prompt 122): the water is not narrowed; high stone arches (`env_bridge_stone` `ArchHigh`, 20 m each) are laid end to end over the whole width (water within 150 m of the point along the axis plus 4 m of bank on each side, at least two pieces) and form one deck (see "Long bridges"); 125 m straight approach; `reach_m` (reworked `river_front`): water counts only within that distance of the point along the axis (default 150 m), so a bridge keeps to its own channel where a small island carries the road on to the next bridge (leave at least `rise_m` / 12 % + 10 m of island between the ends of two bridges, or the second deck is measured from the first one's ramp). `rise_m`: deck height above the bank (default 3 m stone, 1.5 m timber, 6 m long stone), ramps 12 % |
| `[[road]]` | `road` or `track` with width and points |
| `[[ring_road]]`, `[[square]]` | a ring track (a rondel) and paved squares |
| `[[wood]]`, `[[scatter]]` | woods (`mixed`, `deciduous`, `conifer`, `birch`, `palm`, `grove`: palms with bushes between them, `rock`: rock outcrops and boulders between conifers, `crag` (prompt 122): rock massifs and outcrops between a few conifers, for mountain tops and islets, `shattered` (prompt 124): a shot-up wood of bare and broken trees, stumps and fallen trunks; trees on a jittered grid, 6 m off ways, none on fields or water) and scattered single trees in loose clumps; the biome picks the species (winter: conifers, birches and bare oaks; desert: palms) |
| `[[field]]` | `grain`, `green` or `ploughed` polygons, with a 2.5 m strip of grass inside the outline |
| `[[patchwork]]` | a patchwork of fields: plots of about `size_m` (each side varied by up to a third) in a grid turned along `axis`, 6 m of grass between them, kinds drawn from `kinds`, a share `meadow` left as grass; plots whose middle lies outside the polygon are left out (own random stream) |
| `[[row]]`, `[[ring]]`, `[[farm]]`, `[[building]]` | rows of huts or barracks across a line, rings of huts on an ellipse (walls, fences and hedges as touching segments: a walled height), farmsteads (`gen_land`), single buildings (catalog kinds and variants) |
| `[[hill]]` | bumps added to the ground noise; `kind = "peak"` (prompt 122): a rock mountain, height × ((1 − d)^1.75 · 0.96 + 0.07 · (1 − d)^0.75 · ridge²), d = distance / radius, ridge from a 70 m ridged noise: pointed top, steep upper flanks over 60 % with crags, gentle foot. Relief over 60 m is stored with coarser height steps (`height_step_mm` = relief / 65 535, 3 mm on `archipelago`); maps under 60 m keep 1 mm |
| `[[rail]]` | a railway line: a 7 m ballast bed (terrain `dirt_track`, open to everybody) on an embankment (`embankment_m`, default 0.8 m) with telegraph poles beside it; only presentation, no rails or trains yet. A railway bridge is a `stone` crossing on the line |
| `[[paint]]` | any terrain of `terrain.toml` inside a polygon; with `noise_m` and `cover` only in blotches (two octaves of noise, denser along ways, thinning out towards the outline): trampled sand between ruins |
| `sea`, `[[land]]` | `sea = true`: everything outside the land polygons is sea, its depth growing with the distance to the coast (shallow within 10 m); land falls to one coast height within 60 m of the coast (a flat sea level) with a sand strip |
| `[[bay]]` | sea maps: water inside the land that belongs to the sea (lagoons, inlets), at the same level; polygons may overlap, their outer outline is the coast |
| `[[causeway]]`, `[[shoal]]` | strips along a line with a width: a causeway is land across the sea, cut across its whole width under the bridge of a crossing on it (the gap the bridge spans); a shoal keeps the sea or a `river` lake shallow (a ford for tanks) |
| `[[works]]` | prompt 123: prepared field works facing `facing_m` (a direction): `battery` = four sandbag gun pits (`fort_sandbag_position`) 15 m apart across the facing and a trench section (`fort_trench_section`) on each flank; `bridgehead` = an MG bunker (`fort_bunker_mg`), three trench sections beside it and a sandbag position. Pieces that do not fit (roads, water, objects) are named in the generator's report |
| `[[town]]` | street building: inside the polygon, buildings of `kinds` line both sides of every road and track of the sketch, long side along the street, `setback_m` (default 3 m) behind its edge, `gap_m` (default 4 m) apart on average, a share `damaged` damaged or ruined; back lots are filled with a `[[village]]` over the same polygon |
| `[[waterfall]]` | prompt 122: `top_m` (the rock edge) and `foot_m` (in the water); the generator writes the markers `waterfall_<n>_top` / `_foot`; Godot (`world/waterfalls.gd`) draws a ribbon of falling water over the ground between them with streaks running down, mist at the foot (`waterfall_mist`) and the river sound of the ambience as a 3D loop. Presentation only: no rule reads the markers, ground and water stay as they are |
| `[[pier]]` | prompt 122: a timber pier from the first point (shore) along the line: `env_pier_wood` segments of 10 m (`Span`), the last one `Boat` (two rowing boats moored beside it) unless `boat = false`. Written beside the object placer (it keeps out of water); the presentation stands piers on the water level (`map_objects.gd` `FLOATING_KINDS`). They block nothing and cannot be destroyed (scenery) |
| `[[rapids]]` | prompt 122: `count` stones (`env_rock` `Small`/`Medium`, block nothing) in the water within `width_m` of the line, not within 16 m of a bridge piece; the water foams around them. Deep water stays deep (no ford) |
| `[[village]]` | `count` loose buildings of `kinds` inside a polygon, turned along the nearest way within 40 m, a share `damaged` damaged or ruined, and `walls` short runs of broken mud-brick walls |
| `[[trench]]` | prompt 124: a trench line, written as prepared field works (see "Prepared ground"). `kind = "fire"`: the line becomes a zigzag trace of teeth `tooth_m` long (stretched to fit the line), each a fire bay pushed `depth_m` towards `facing_m` over the middle of the tooth (out at 20 %, back at 70 % of it); an MG nest in front of every `nest_every`-th bay, a dugout behind every `dugout_every`-th tooth; the simulation adds its own 1.5 m zigzag. `kind = "line"`: a fire trench exactly along the points (hand-drawn large jags, redans, closed cells), with MG nests at `nests_m` and dugouts at `dugouts_m`. `kind = "comm"`: a straight communication trench along the points. Kept free of objects (4.5 m, 3 m) |
| `[[wire]]` | prompt 124: prepared wire along a line, cut open `gap_m` wide (default 16 m) at the points `gaps_m` and wherever a lane crosses |
| `[[lane]]` | prompt 124: a lane (`width_m`, `points_m`) through a crater field: no craters, wire open, no objects, no painted terrain (firm ground through the mud) |
| `[[crater_field]]` | prompt 124: about `density_ha` craters per hectare in the polygon (thinning out over `edge_m` towards the outline), calibers drawn from `calibers` (`[mm, weight]`), a share `water` with water in the bowl; none on lanes, ways or water, within 5 m of trenches, 2 m of wire, 25 m of bridges, near starts and supply points; centers at least 0.75 of the two radii apart (own random stream). Written as prepared craters |
| `[[marsh]]` | prompt 124: `count` puddles of `radius_m` (min, max) in the polygon: a 0.3 m dip of mud with standing water (`[[puddle]]`, presentation); none within 6 m of a way (own random stream) |
| `[[camp]]` | prompt 124: `items` (`kind`, `variant`, `count`) placed loosely in the polygon, turned along the nearest way (dumps, crates, barrels, woodpiles) |
| `[[mesa]]` | prompt 126: a table mountain: `polygon_m` is its foot (rounded by cutting the corners twice, then frayed by `rough_m` of noise, default 8 m, an own noise stream), `height_m` the plateau above the basin. From the foot inwards a scree slope (a fifth of the height, at most 5 m, grade 35–60 %, boulders and a few outcrops on it, `boulders` per 10 m of foot, default 1) and a cliff (180 %, impassable) rise to the plateau; the mesa adds to the ground (the plateau keeps its gentle noise). `[[mesa.tier]]`: a step (`height_m` above the plateau's) or a hollow (below it, with a cliff down and scree at the bottom), on the plateau or on an earlier tier that holds its first point. `ramps_m` (of the mesa and of each tier): points on the outline where a ramp leads from the lower to the higher level: `ramp_grade` (default 20 %, under the 60 % limit; uphill speed 60 %), `ramp_width_m` (default 18 m bed: a tank needs 8 m plus 2 m beside it, also on a slant to the path grid) with a dirt track on it, half cut into the higher level (rock walls 150 %), half heaped onto the lower one (sides 50 %); a 35 m plateau needs 175 m of ramp. A mesa without ramps is scenery (buttes, rock needles). With a `name` the generator writes a marker `ramp_<name>_<k>` (tiers `ramp_<name>_t<i>_<k>`) 12 m beyond the top of every ramp (`map-reach` shows whether the plateau is in the main part; mirrored maps add their images as `…_image`) and prints every ramp's top and foot |
| `[[scatter]] kind = "scrub"` | prompt 126: dry scrub of bushes and stones (desert hollows) |
| `[[mesa]] climbs_m` | prompt 128: points on the outline of a mesa where a climb for infantry leads up: built like a ramp (half cut, half heaped) but `climb_grade` steep (default 38 %: under the 60 % limit also on a slant to the path grid, where the grid adds both gradient components) and `climb_width_m` wide (default 12 m; 16 m on a slant to the grid, so that a staircase of open path cells fits), without a track; two rows of large boulders (`env_rock` `Large`, 3 m apart, 4 m between the rows) across its cut stop vehicles, and its bed is kept free of other objects (woods, scree boulders). Markers `climb_<name>_<k>` (with `_image` on symmetric maps) 12 m beyond the top. A ring of rock is two C-shaped mesas (arc bands) with their ends apart: the gaps are passages at the level of the ground (`kessel_lake`) |
| `[[mesa]] slope_grade` (and on `[[mesa.tier]]`) | prompt 129: a terraced hill instead of a table mountain: the edge of the mesa (or tier) is an even slope of `slope_grade` (under 60 %; keep it at about a third: the path grid adds both gradient components on a slant, √2 × the grade) instead of scree and a cliff; its outline is frayed by the broad noise octave only (`rough_m`), the fine ones would steepen it. `cliffs_m`: sectors as pairs of points, clockwise on the map from the direction of the first to that of the second as seen from `center_m` (default the mean of the outline's points; two equal points: all round), where the top `cliff_m` of the slope (default half the step) is a rock band (180 %, impassable) and the slope below it gentler, the edge keeping its width; blended over 8 m across the bounding rays; debris boulders at the foot of the bands. `ramp_paths_m`: ramps along lines from the foot (on the lower level) to the top (on the higher one) whose bed rises evenly along them, so a ramp winding round a hill keeps a low grade; built like ramps (half cut, half heaped, `ramp_width_m`, a track, kept free of objects), markers `ramp_<name>_p<k>` (tiers `ramp_<name>_t<i>_p<k>`) 12 m beyond the last point; the generator prints each one's length and grade. `stop_vehicles = true`: two rows of large boulders (3 m apart, rows 4 m apart) along the middle of the slope outside its rock bands, ramps and climbs stop vehicles, not infantry (a track across the rows leaves a gap: draw foot ways with a gap there). A pit is a hollow tier with a ramp (`pit_hill`). Without `slope_grade` nothing changes (`cliffs_m`, `cliff_m`, `stop_vehicles` need it) |

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
| `archipelago` ("Inselreich", prompt 122) | eight islands around a ring-shaped middle island with a lagoon and an islet; six long stone bridges of four to six high arches (80–120 m, `stone_long`) and a short stone bridge over the creek west of the lagoon; three rock mountains with snow-capped tops (north 150 m, north-east 115 m, south-west 125 m) and four rock hills, three waterfalls with rapids below them, six villages with churches (the north-east one larger, with a town hall), nine patchworks of grain and green fields, dense conifer woods, sand bays, six timber piers, twenty rock islets; fords link the islands of a side (north-west–west, north-east–east). Look `vivid` | 3.6 × 2.4 km | forest (vivid), clear | team 0 west (players 0 west island, 2 south-west island), team 1 east (1 north-east island, 3 south-east island) | lagoon north and south, north pass, west and east bridgehead, north-west, east, south-west and south-east village, south-west and north-east pass |
| `river_front` ("Flussfront", prompt 123) | an artillery duel over a broad river (below): the river west–east through the middle: two arms round a head of land at each end, 770 m wide in the middle with many wooded islands, rocky banks, rapids and four fords; 44 stone bridges (`stone_long`, 80–160 m, small islands between the bridges of a chain); a town with square, town hall and church on each bank on the straight main road, ring roads joining the bridgeheads, farmsteads, golden and green fields, conifer and mixed woods, a mountain range vehicles can drive in each corner (summits 55–95 m, a track to each main summit), four table mountains with rock walls and ramps at the west arms, hills behind the towns; four gun positions per side on reverse slopes (`battery_*`), observation hills on the banks, MG bunkers and trenches at the bridge ends, stone houses as bridge towers. Strictly mirror-symmetric (`mirror = "north"`) | 3.6 × 2.0 km | forest, clear | team 0 north (players 0 west, 2 east), team 1 south (1, 3) | center island, west and east island, the two town squares, the four observation hills |
| `trench_line` ("Grabenlinie", prompt 124) | a deadlocked trench war in the mud (below): a brook runs diagonally through the map and a road crosses it at a right angle through a ford in the middle; around the ford a round no man's land of shell craters (110 per hectare, 45 % with water) inside a ring track with two timber bridges, a shot-up house on each bank; north and south of the ring a trench system of large jags (front line, a second line jagged the other way, support line, communication trenches, MG nests, dugouts, two rows of wire); west and east of the ring a low rise with two gun positions in sandbag pits and a camp; fords on the flanks, remains of woods at the edges. Point-symmetric (`mirror = "point"`), look `mud` | 2.4 × 2.0 km | forest (mud), clear | team 0 north-east (players 0, 2), team 1 south-west (1, 3) | the ford in the middle, the two ring bridges, the houses in the ring, the gun rises, one point in each trench system |
| `river_fork_front` ("Front an der Flussgabel", prompt 125) | a front across a fork of three rivers, **deliberately not symmetric** (below): the main river from the north-west through the middle to the south-east, a tributary from the north (through the heights, `through_hills`) and one from the south meet in no man's land; two trench systems across the map with front and support lines, communication trenches, two rows of wire with gaps, three lanes of craters (3 700, 65 per hectare between the wire); the north holds wavy heights with woods (crest 30–40 m, front trenches on a gentle forward slope), gun positions on the reverse slope at the edge of the wood and behind the height over the fork, villages and farms between the rivers, a wood in the north-west; the south holds the flat plain with villages up to the front, fields, roads in every direction, gun positions in and beside villages; ten bridges (two north rear, two in each trench system, four south rear) and two fords (south). Look `mud` | 3.6 × 2.0 km | forest (mud), clear | team 0 north-east on the heights (players 0, 2), team 1 south-west behind the villages (1, 3) | the fork, the height over the fork, a bridge in each trench system, a village on each side, the wood in the north-west, the middle village and the hamlet east of the main river (south) |
| `mesa_desert` ("Tafelberge", prompt 126) | a desert of table mountains (below): flat basins with hollows of dry grass and scrub, from which mesas rise with cliffs over narrow scree: edge plateaus in the north-west (split by a lane from the corner), north (with a ridge along the north edge to the east) and east, each with steps and hollows, and their images; middle mesas, a ring of buttes and rock needles around an open centre; 18 ramps per half (`[[mesa]]`: 12 from the basins, 6 to steps and into hollows), tracks between the basins and to the ramps, two abandoned farmsteads with wells per half. Point-symmetric (`mirror = "point"`), look `mesa` | 2.4 × 2.4 km | desert (mesa), clear | team 0 north-west and north-east corner (players 0, 2), team 1 south-east and south-west corner (1, 3); 1v1 north-west against south-east | the open centre, the big butte of each half, the north-east and south-west basin, the hollow on the north-west and south-east plateau |
| `kessel_lake` ("Kesselsee", prompt 128) | an own design accepted by the user (below): a rock ring (the "Kessel", two C-shaped `[[mesa]]` arcs, 22 m) with a passage in the north and the south and a hamlet on the meadow inside, a ring track around it; a lake in the north-west and its image in the south-east with beaches, an islet and a sandbank, two fords (`[[shoal]]`) from the shore over the islet to the beach; mountains in the north-east and south-west corner (up to 123 m, rock and conifers); a street village north and south with fields, farmsteads, small woods; infantry climbs over each arc (`climbs_m`). Point-symmetric (`mirror = "point"`), look `rock` | 2.0 × 2.0 km | forest (rock), clear | team 0 west (players 0 west, 2 south), team 1 east (1 east, 3 north); 1v1 west against east | the hamlet in the Kessel, the village north and south, the ford north-west and south-east |
| `pit_hill` ("Grubenhügel", prompt 129) | an own design accepted by the user, built on relief (below): a terraced hill in the middle (`[[mesa]] slope_grade`: 32 % slope to an upland ring at 35 m, the top at 60 m) with rock bands on half of each slope, boulder rows on the open quarters of the top's slope, two winding ramps (under 9 %), a mining village and two pits (10 m) on the top; a long side ridge (30 m) west and east beyond a dip, a ring track around both; a stepped mountain range along the north and south edge (only the foot drivable, up to 115 m); lake bays north-west and south-east with a peninsula and an islet; a railway along the west and east edge with a station and a loading yard; hamlets at the mountain foot, rocks, small woods. Point-symmetric (`mirror = "point"`), no look (earth shows on the slopes) | 2.0 × 2.0 km | forest, clear | team 0 west (players 0, 2), team 1 east (1, 3); 1v1 player 0 against 1 | the top of the hill, the stations, the hamlets, the ridges, the peninsulas |
| `omega_bend` ("Omegaschleife", prompt 130) | an own design accepted by the user (below): a river from west to east swinging into an almost round loop (an omega) towards the north edge; a peninsula in the loop with a small village and a church, joined to the south by a neck of about 300 m; four long stone bridges (`stone_long`, five arches each): two north–south over the straight reaches, two west–east over the loop sides from the north quarters onto the peninsula; north quarters with a farmstead, a field and woods, reached only over bridges; the wide meadow of the south with hamlets at the bridgeheads and the starts, wood strips west and east of the middle, small woods, fields, a pond in the middle and a small station at the south edge; few houses, much meadow and wood, every way curved. Mirror-symmetric west against east (`mirror = "west"`) | 2.0 × 2.0 km | forest, clear | team 0 west (players 0, 2), team 1 east (1, 3); 1v1 player 0 against 1 | north of the pond, the peninsula village, the station, the loop bridges, the south bridgeheads, the start hamlets |
| `river_city` | a city of about 3000 buildings: street quarters in turned grids (traced as clipped grid lines), avenues radiating from a central place with church and town hall, a river in a great loop around the centre with eight bridges (six stone), a park with ponds and a rondel in the north-east, a canal and a park in the south, a railway with a station yard across the south, a quarry with halls in the west, fields at the edges; lighter than the picture | 3.6 × 2.4 km | forest, clear | team 0 west (players 0, 2), team 1 east (1, 3) | central place, station, park rondel, loop village, quarry, north quarter, south-east quarter, south park |

#### Mirror-symmetric maps (prompt 123)

`mirror = "north"` makes a sketch map strictly mirror-symmetric, north
against south. The mirror axis is the path cell center line next to
y = 0 (y = 1 m on maps whose depth is a multiple of 8 m): it maps height
samples, path cells and the axes of west–east bridges onto themselves.
The generator samples the whole map, copies every sample row north of
the axis mirrored onto the south (before the bridges are measured, so
water across the axis is symmetric, and again after their embankments),
gives every bridge chain of the north half its mirrored copy (a chain
across the axis must already be symmetric, otherwise generating fails)
and keeps only objects that lie wholly north of the axis, each with a
mirrored copy (heading `180° − h`, bridges keep theirs). So the sketch
draws only the north half and what lies on the axis; starts, supply
points and markers are listed for both halves. The light is not
symmetric (sun in the south), which is accepted.

The 8 m vision grid starts at the map edge, so its cells lie 2 m off
the mirror axis: sight is symmetric only to within a cell
(`river_front`: the observation hills see 6.1 / 6.6 ha (west) and
7.9 / 8.7 ha (east) north / south). No axis can match both grids (the
path grid is offset by one sample). The path search is not
mirror-invariant either: between two routes of nearly the same cost it
may pick another one on the mirrored side (in a first draft of
`river_front` 784 against 624 m to an observation hill at 0.4 %
different cost; in the final map all lengths of `just map-reach` agree).
The test `river_front_is_mirror_symmetric`
checks heights, terrain, objects, starts, markers, open path cells and
equal path costs (within 2 %) for both sides.

#### Mirror-symmetric west against east (prompt 130)

`mirror = "west"` is the mirror of prompt 123 turned by 90°: the axis is
the path cell center line next to x = 0 (x = 1 m on maps whose width is a
multiple of 8 m), the **west half** is the source. Every sample column east
of the axis takes the value of its image (before and after the bridges;
the first column, x = −half, has no image and stays), every bridge chain of
the west half gets its mirrored copy (bridges keep their heading; a chain
across the axis must already be symmetric), and objects that lie wholly
west of the axis get their image at (2 − x, y) with heading `360° − h`.
Unlike the other two mirrors an object **centered on the axis and turned
along it** (heading 0° or 180°) is its own image and stays once: a church
or a station hall can stand on the axis (`omega_bend`). Trenches, wire,
craters, puddles and mesa markers follow the same rule as under the other
mirrors (west half, images). So the sketch draws the west half and what
lies on the axis; lines and areas may be written for both halves (the east
ones only shape the drawing), crossings only for the west half (their
images are made). Starts, supply points and markers are listed for both
halves. The test helper `check_symmetry` (`Sym::West`) checks heights,
terrain, objects, starts, markers, open path cells and path costs as for
the other mirrors (`omega_bend_is_mirror_symmetric`). Sketches without
`mirror = "west"` are unchanged (every other sketch map regenerates bit
for bit).

#### `river_front` ("Flussfront", prompt 123)

Built for an **artillery duel over a broad river**: each side has heights
and prepared gun positions on its bank and fires over the river, the
islands and the bridges; whoever wants to cross has to go over bridges
and islands under fire. The sketch follows the template's layout (broad
river with wooded islands, about ten stone bridges with towers at their
ends, a town with a square on each bank on one straight main road, ring
roads, rock mountains in the corners); the template is not in the
repository.

- **River** (reworked after the first version, on the user's request:
  broader, forked, more islands): at both ends two arms of 140 m round a
  head of land on the axis (a farmstead, fields, the road between the two
  end bridges); between the heads 660 m from bank to bank, 770 m in the
  middle. Islands: the center island (supply point, four bridges), the
  west and east island on the axis (485 × 210 m), a bank island before
  each town on the main road (600 × 135 m), four smaller wooded islands,
  fourteen small islands that carry a road from one bridge to the next,
  and rock islets. Fords (`[[shoal]]`, shallow: tanks cross): one over
  each east arm (x = 1260 m) and one from the tip of each head to the
  west / east island; rapids stones at the fords, the tips of the heads
  and the ends.
- **Bridges** (all `stone_long`, prompts 115/122): 44 bridges, 260 pieces,
  none longer than 160 m (the user asked for more bridges after the first
  rework). One over each arm at both ends and one over each west arm
  between the table mountains (deck 6 m above the bank); the main road
  from each bank to its bank island and on to the center island, east of
  it a second bridge from the bank to the bank island; from each bank to
  the west and east island over a small island; from the west and east
  island to the center island over a small island on the axis; from each
  bank over two small islands to each head; the four wooded islands to
  the bank islands (west–east) and to the west / east island
  (north–south). The bridges of a chain have `reach_m`, so
  each spans only its own channel, and a 4 m deck (`rise_m`), so their
  ramps fit on the small islands. Every piece falls to engineers (prompts
  020/055). Stone houses stand at the bank ends as bridge towers (an own
  gatehouse model is open), MG bunkers and trenches (`[[works]]
  bridgehead`) at the bank ends, the fords, the bank islands, the heads
  and the axis ends of the west and east island.
- **Table mountains** (`[[mesa]]` `gate_w`, `gate_e`, 20 m, rock walls):
  two on each bank at the west arms, beside the way from the ring road to
  the arm's second bridge; a 22 % ramp leads up each one from the ring
  road (markers `ramp_gate_*`), from the top they overlook the arm and
  its two bridges. (A high bridge between them is not built: a deck over
  land closes the way beneath it.)
- **Corner mountains**: in each corner a main summit (92–95 m, r = 400 m),
  a lower one on the north / south edge (55–58 m) and one above the arm
  (56–60 m), all `peak`s with flanks under the 60 % limit: vehicles drive
  them slowly (rock only in the far corner and in bands); two tracks lead
  to each main summit (markers `peak_[ns]_[we]`), from the gun position
  track and from the ring road.
- **Gun positions** `battery_n1`–`n4`, `battery_s1`–`s4`: four per side,
  two behind the hills north (south) of the town at x = ±370 m and two
  behind hills at the corner mountains at x = ±1130 m, |y| = 805–822 m;
  flat over 35 m, four sandbag pits and two trench sections each, a track
  from the road and from the starts. From the other bank they are out of
  sight (sight is 250 m, recon 350 m, 1/8 of the real 2 km / 2.8 km) and
  behind the hill from the own bank.
- **Observation hills** (22 m, supply points `supply_[ns]_obs_[we]`):
  bluffs on the banks beside the island bridges; they see down onto the
  near channel, its small islands and bridges (7–9 ha, most of it 20 m
  or more below). The other bank lies 660 m away, the enemy hill 786 m: it is seen from
  the islands, not from the hills.
- **Starts**: two per side behind the ring road between the town and the
  corner mountains (|x| = 720 m, |y| = 790 m), 60 m flat and free.

**Ranges** (game range = real range × 1/8, `[range] indirect_fire` of
`data/scale.toml`; distances from `just map-sight river_front`):

| Gun (catalog) | Real | Game | What it reaches |
|---|---|---|---|
| Light mortars (5 cm, 2-inch, 50-PM, 60 mm) | 460–1 815 m | 58–227 m | from the bank only the near water |
| Medium mortars (8 cm s.GrW 34, 3-inch, 81 mm, 82-BM) | 2 400–3 040 m | 300–380 m | from the bank the near channel and the near islands; from the islands on the axis the other side's bank islands |
| Heavy mortars (4.2-inch, 120-PM, 12 cm GrW 42) | 3 750–6 050 m | 469–756 m | from the observation hills the west / east island (450 m); no enemy hill (786 m); from the gun positions nothing beyond the near bank |
| 15 cm sIG 33 (Grille, Jagdpanzer 38(t)), 152 mm (SU/ISU-152), rocket launchers | 4 200–8 740 m | 525–1 092 m | from the gun positions the near water and islands (center island 900 m) |
| Light howitzers and field guns (75 mm pack, Wespe, M7, 105 mm M2A1, 122 mm M-30, 25-pdr, leFH 18) | 8 790–12 325 m | 1 099–1 541 m | from the gun positions the river, the islands and the far bridgeheads of their sector (M2A1, 1 409 m: every bridge and ford from a position of each side); no enemy position (1 612–1 642 m straight across), no enemy start (1 649 m or more) |
| 15 cm sFH 18 (Hummel) | 13 325 m | 1 666 m | the enemy position straight across (1 612 / 1 642 m) and the nearest enemy start (1 649 / 1 650 m), nothing diagonal |

So counter-battery fire costs movement and risk: from a gun position only
the Hummel reaches the position across; light howitzers have to come
forward to the bank or the islands (from an observation hill the enemy
positions lie 1 218 m away or more). The test `river_front_ranges_and_sight`
(`crates/cli/src/map_sight.rs`) checks this table against the catalog:
positions unseen and seeing nothing of the other side, only the farthest
gun from position to position, starts beyond the light howitzers, every
bridge and ford within the M2A1 from a position of each side, no mortar
from a position onto an enemy supply point, heavy mortars from the hills
onto an island on the axis but not from hill to hill. `just map-sight` prints, for maps with `battery_*` markers, what
each position sees of the other side, who sees it, and for every position
and supply point the distance to the enemy starts, supply points and
positions and to every bridge and ford with the number of indirect
weapons that reach it.

Checks: `just map-check river_front` passes; `just map-reach river_front`:
every start, supply point, gun position, bridge and ford in the main part
for vehicles and infantry (82 % of the land open to vehicles, 11 %
infantry only: the mountains and steep banks); path lengths of both sides
equal. Pictures: `just river-front-shots` (top, oblique, town, middle
bridge, gun position, observation hill, ford, end bridge, mountain; beside
the template if it is there). AI and frame rate:
[AI.md](AI.md#the-river-front-prompt-123),
[PERFORMANCE.md](PERFORMANCE.md#river-front-prompt-123) (both measured on
the first version of the map, before the river was reworked; not
measured again).

#### Point-symmetric maps (prompt 124)

`mirror = "point"` turns the north half by 180° about the path cell
center point next to the origin ((1, 1) on maps whose sides are
multiples of 8 m): it maps height samples and path cells onto each other
(the first row and column have no image). Unlike the mirror of prompt
123 the two halves meet at the center line from different places, so the
generator blends the heights within 24 m of the line with their images
(half and half on the line), then copies every sample south of the line
(and east of the center on it) from its image, before and after the
bridges. Bridges, objects (heading + 180°), trenches, wire, craters and
puddles of the north half get their images; an object exactly on the
center point is its own image and stays once. A line that should run on
across the center line is drawn in the north half up to the point where
the image of its other end begins; one through the center itself runs a
little past it and the copy overwrites the rest (`trench_line`: the
brook, the road and the ring). A hill near the line is listed with its
image, so that the ground of both halves meets. The test
`trench_line_is_point_symmetric` checks heights, terrain, objects,
starts, markers, prepared ground, open path cells (on the dug map) and
equal path costs; it shares its body with the mirror test of
`river_front` (and `mesa_desert_is_point_symmetric`, prompt 126, whose
`needle_*` markers stand on mesas without a ramp and are left out of the
path check).

#### `trench_line` ("Grabenlinie", prompt 124)

Built for a **deadlocked trench war**: two trench systems face each
other across a round, churned no man's land; whoever attacks crosses
craters, wire, mud and a brook under the fire of mortars and of the guns
on the two rises. The sketch follows the landscape of the template (a
ring track around a cratered no man's land, a brook running diagonally
through it, trench systems of large jags with communication trenches and
dugouts, gun pits with tents on low rises at the sides, a shot-up house,
bare trees, timber bridges); the template is not in the repository and
nothing of its interface or style is used. This is the second layout:
the first one (two straight trench lines across the map, brooks on the
flanks) did not resemble the template.

- **Size** 2.4 × 2.0 km. Team 0 holds the north-east, team 1 the
  south-west; the brook divides them.
- **Brook**: 12 m wide with a channel, from the north-west edge through
  the map center to the south-east edge (about 35° to the west–east
  axis), marshy banks with puddles. It is crossed at the **ford in the
  middle**, on the two **timber bridges of the ring** and at a ford on
  each flank (|x| = 780 m).
- **Road** at a right angle to the brook: from the starts in the corners
  through the own trench system, across the ring and the ford. It is the
  lane for vehicles: no craters on it, wire open, firm ground.
- **Ring**: a track of 260 m radius around the ford. Inside it no man's
  land: 110 craters per hectare (105, 150, 210, 280 mm; 45 % with water),
  mud on four fifths of the ground, bare trees, a shot-up house on each
  bank. Outside 28 craters per hectare around the trenches and rises, 4
  elsewhere (4 950 craters in all). Crater centers close for everybody
  (the grade in the bowl), mud slows vehicles to 40 %.
- **Trenches** (per side, `[[trench]] kind = "line"`): the front line 50 m
  outside the ring with jags of 120 m (points 45 m towards the ring, an
  MG nest at every point, a dugout behind every re-entrant), a second
  line 85 m behind it jagged the other way, so that both enclose six
  cells closed by communication trenches; a support line (teeth of 40 m)
  and communication trenches to it and to the lateral track. All lines
  are open at the road. 12 fire trench lines, 28 communication trenches,
  12 MG nests, 36 dugouts on the map; they do not stop vehicles.
- **Wire**: two rows in front of the front line following its jags (open
  at the road and at one re-entrant per wing for infantry), one row on
  each flank of the system.
- **Gun rises**: a rise of 7 m west and east of the ring, 450 m from the
  middle: the east one on the north bank (team 0), the west one on the
  south bank (team 1). Two gun positions each (`battery_n1`, `n2`,
  `battery_s1`, `s2`; `[[works]] battery`, 35 m flat), a supply point and
  a camp with tents and dumps; a track from the ring and from the rear.
- **Behind the lines**: a lateral track to the flank ford, camps, a
  ruined farm as command post, low hills, remains of woods at the edges
  and shattered woods beside the trench systems.
- **Starts**: two per side in the corner behind the own trench system,
  1v1 = players 0 (north-east) and 1 (south-west).
- **Supply points**: the ford in the middle (a marker in the water, taken
  from within 25 m), the two ring bridges, the houses in the ring (one per
  bank), the gun rises, one point in each trench system.
- **Look** `mud` (above); smoke of fires (`smoke_1`–`4`) and two mist
  sheets over no man's land (`mist_1`, `mist_2`).

**Ranges** (game range = real range × 1/8, `[range] indirect_fire` of
`data/scale.toml`; distances from `just map-sight trench_line`):

| Gun (catalog) | Game | What it reaches |
|---|---|---|
| Light mortars (5 cm, 2-inch, 50-PM, 60 mm) | 58–227 m | from the own front the near part of no man's land; the house on the own bank (141 m) only with the 60 mm |
| Medium mortars (8 cm s.GrW 34, 3-inch, 81 mm, 82-BM) | 300–380 m | from the own front the house on the own bank and the brook, not the enemy front (620 m or more); from the ford the ring bridges (264 m) |
| Heavy mortars (4.2-inch, 120-PM, 12 cm GrW 42) | 469–756 m | the longest ones the enemy front from every point of the own front (at most 728 m); from the ford both rises (436–501 m) |
| Light howitzers and field guns (75 mm pack, 105 mm M2A1, 122 mm M-30, 25-pdr, leFH 18) | 1 099–1 541 m | from a gun rise the whole ring, the enemy trench system (supply point 669–676 m), the enemy front and the enemy rise (872–1 000 m): the batteries can duel; no enemy start |
| 15 cm sFH 18 (Hummel) | 1 666 m | from a gun rise everything but the enemy starts (1 686 m or more) |

So the rises are strong but exposed: every howitzer on one reaches the
other, and heavy mortars reach them from the ring. The gun positions see
nothing of the other side and nobody there sees them (sight is 250 m), so
fire needs observers forward. The test `trench_line_ranges_and_sight`
(`crates/cli/src/map_sight.rs`) checks the table against the catalog:
positions unseen, starts beyond every gun on a rise, every light howitzer
from rise to rise, onto the enemy front and the enemy trench point, no
medium mortar from rise to rise, the fronts beyond the medium and within
the heavy mortars, the own house within the medium mortars of the front.

Checks: `just map-check trench_line` passes; `just map-reach
trench_line`: every start, supply point and gun position in the main
part for vehicles and infantry (86 % of the land open to vehicles, 10 %
infantry only, 4 % nobody: the crater bowls and brook banks); path
lengths of both sides equal. `just stuck-audit --map trench_line`: 85
group orders, 272 units, 271 arrived; one rifle squad of a mixed group
stopped 13 m short of a start behind another unit. Pictures: `just
trench-line-shots` (top, oblique, no man's land, front trench, ford,
gun rise, camp, ring bridge in the mud look and as plain forest, beside
the template if it is there). The AI tournament and the frame rate and
path figures in [AI.md](AI.md#the-trench-line-prompt-124) and
[PERFORMANCE.md](PERFORMANCE.md#trench-line-prompt-124) were measured on
the first layout and are not repeated yet.

#### `river_fork_front` ("Front an der Flussgabel", prompt 125)

Built for a **front across a river fork that is deliberately not
symmetric**: the north (team 0) defends wavy heights with woods, the
south (team 1) attacks from a flat plain with villages. The balance comes
from compensation, not from mirroring. The sketch follows the landscape
of the template (three rivers meeting in the middle, two trench belts
across them, hills with a battery at a wood edge in the north-east, a
plain with villages, fields and a battery in a village in the south-west,
craters behind the lines, a wood in the north-west); the template is not
in the repository and none of its names are used.

- **Rivers**: the main river (56 m) runs as one line from the north-west
  through the fork to the south-east (the tributaries end on it, so its
  water decides at the fork), the north tributary (36 m) runs through the
  heights in a valley (`through_hills`, `valley_m = 140`), the south
  tributary (36 m) through the plain. Every river is straight across its
  bridges (axis west–east or north–south).
- **Fronts** (base lines): north front y = −0.06 x − 140, support line
  − 260; south front y = −0.06 x + 140, support line + 260; each front
  in three parts between the rivers (west of the main river, in the angle
  north / south of the fork, east of the tributary / main river), so each
  side holds land on both banks. Fire trenches of 46 m teeth (support
  lines 36 m), MG nests in every third bay, dugouts, 15 communication
  trenches; two rows of wire 32–40 m in front of each front with gaps for
  infantry; lanes at x = −1 000 and 900 m. The front lines lie 250–282 m
  apart (every point within the medium mortars of the enemy front).
- **Heights** (north-east): a row of broad bumps (19 m, 460 m radius) puts
  the north front 4–6 m up a forward slope of at most 8 % and the support
  line 13–16 m up; a row of steeper bumps (18 m, 230 m) behind it makes
  the crest 30–40 m above the plain. Steeper ground under the trenches
  would close their cells: a dug cut plus parapet on a 20 % slope exceeds
  the 60 % grade limit of a path cell (first draft: the hill trench lines
  were closed for everybody).
- **Bridges and fords** (`bridge_*`, `ford_*` markers): north two behind
  its lines (stone over the tributary and the main river) and two in its
  trench system (stone over the main river, timber over the tributary);
  south two in its trench system (timber over the south tributary, stone
  over the main river) and four bridges and two fords behind its lines.
- **Gun positions** (`[[works]] battery`, 35 m flat): `battery_n1` at the
  edge of the wood in the north-east and `battery_n2` behind the crest
  north of the height over the fork (reverse slopes, 915 / 950 m from the
  south front); `battery_s1` in the battery village in the south-west and
  `battery_s2` south of the village by the south tributary (617 / 909 m
  from the north front). Enemy positions lie 1 865 m or more apart.
- **Starts**: north on the low ground behind the crest (players 0 and 2,
  (1 000, −910) and (1 590, −930)), south behind the villages (players
  1 and 3, (−990, 830) and (−1 560, 800)); 1v1 starts 2.6 km apart; every
  enemy position 2 190 m or more away.
- **Supply points**: the fork (`supply_fork`, the tip of land between
  the main river and the south tributary in the middle of no man's land,
  reachable over land from both west fronts), the height over the fork
  (north), a bridge in each trench system, a village on each side of the
  front, the wood in the north-west (north), the middle village and the
  hamlet east of the main river (south).

**Asymmetry and its levers.** North: height (sight over cover, the
downhill bonus from the crest), reverse slope gun positions; but few
tracks, its trenches on the forward slope (seen from below), only two
bridges behind its lines, its supply points spread over three sectors
between the rivers (start 0 to the wood in the north-west 2.7 km).
South: four bridges and two fords behind its lines, roads in every
direction, villages as cover up to the front, supply points nearer its
starts (the front village 0.6 km); but every move is in sight of the
heights. The levers used in the AI tournament (`docs/AI.md`): the
height of the crest (26 → 18 m) and the south starts (y 900 → 830, a
shorter way to the front). Further levers: bridges, fords, supply points,
the position of the trenches.

**Ranges** (game range = real range × 1/8, `[range] indirect_fire` of
`data/scale.toml`; distances from `just map-sight river_fork_front`):

| Gun (catalog) | Game | What it reaches |
|---|---|---|
| Light mortars | 58–227 m | from the own front only no man's land |
| Medium mortars | 300–380 m | from every point of the own front the enemy front (at most 282 m) |
| Heavy mortars, 15 cm sIG 33, 152 mm, rocket launchers | 469–1 092 m | from the fork the trench bridges and the height over the fork (412 m); from the gun positions the enemy front only for the longest of them (617–950 m: from `battery_s1` the 12 cm mortars and up, from the others the sFH 13 and BM-13) |
| Light howitzers and field guns | 1 099–1 541 m | from every gun position the enemy front; no enemy gun position (1 865 m or more), no enemy start (2 190 m or more) |
| 15 cm sFH 18 (Hummel) | 1 666 m | no enemy gun position, no start |

So counter-battery fire needs guns brought forward. The test
`river_fork_front_ranges_and_sight` (`crates/cli/src/map_sight.rs`)
checks: gun positions unseen and seeing nothing of the other side, no gun
from a position onto an enemy position or start, the shortest light
howitzer from every position onto the enemy front, every medium mortar
from front to front. `just map-sight` also shows the sight from the front
trenches (`front_*` markers, since prompt 125) and the crest
(`height_n_crest_*`).

Checks: `just map-check river_fork_front` passes; `just map-reach
river_fork_front`: every start, supply point, gun position, bridge, ford
and lane in the main part for vehicles and infantry (91.5 % of the land
open to vehicles, 5.8 % infantry only, 2.4 % nobody: crater bowls, river
banks, the valley sides of the north tributary). `just stuck-audit --map
river_fork_front`: 145 group orders, 464 units, all arrived. Pictures:
`just river-fork-front-shots` (top, oblique, fork, hill front, plain
village, both gun positions in the north and south, trench bridge, west
lane in the mud look; top, oblique, fork, hill also as plain forest;
beside the template if it is there). AI and frame rate:
[AI.md](AI.md#the-river-fork-front-prompt-125),
[PERFORMANCE.md](PERFORMANCE.md#river-fork-front-prompt-125).

#### `mesa_desert` ("Tafelberge", prompt 126)

Built for **height against width**: the plateaus look far over the basins
and are good positions for guns and observers, but they are reached only
over a few ramps; the basins hold room for tanks but little cover, and
between the mesas lie lanes and narrows. The sketch follows the layout of
the template (a shaded relief of a desert: pale basins with darker
hollows, mesas with stepped tops and hollows, large ragged plateaus at the
edges, a ring of buttes around an open centre; roughly point-symmetric);
the template is not in the repository.

- **Size** 2.4 × 2.4 km, biome `desert`, look `mesa` (above), weather
  clear; point-symmetric about (1, 1): the north half is drawn, the south
  half is its image. Relief 2–52 m (1 mm height steps); a broad 5 m lift
  (`[[hill]]` of 6 km radius) keeps the hollows of the basins above 0 m.
- **Mesas** (per half, `[[mesa]]`): edge plateaus `nw` (35 m, the hollow
  with the sheltered supply point, 15 m) and `w` (35 m, a step of 45 m)
  in the north-west, split by a lane from the corner start; `north`
  (35 m, a step of 45 m and a hollow of 20 m, a ridge along the north edge
  to the east); `e` (30 m, a step of 40 m and a hollow of 15 m); the
  middle mesas `f` (25 m) and `o` (22 m); the centre ring with the big
  butte `i` (25 m, a supply point), the elongated butte `m` (20 m), a
  crescent (15 m), two small buttes (18 and 12 m) and three rock needles
  (10 m). Scree with boulders at every foot (380 boulders and outcrops
  per half).
- **Ramps** (20 %, 18 m bed with a dirt track): every edge plateau two
  from the basins plus one or two to its step and into its hollow, the
  middle mesas and the big and elongated butte one each, the crescent, the
  small buttes and the needles none (12 + 6 per half). The ramps of the
  edge plateaus face the own basins and starts, that of the big butte the
  own side (the elongated butte's points to the centre), so the way onto
  an enemy plateau is long and in sight.
- **Starts** in the corners behind the edge plateaus: team 0 north-west
  (player 0) and north-east (2), team 1 south-east (1) and south-west (3);
  1v1 north-west against south-east (3 km). A first draft had the starts
  in the west and east basins; there the enemy east plateau looked down
  on a start from 808 m (rocket launchers, heavy mortars and the 15 cm sIG
  in reach), so they moved into the corners.
- **Supply points**: the open centre (`supply_center`, its own image),
  the big butte of each half (`supply_[ns]_butte`, one ramp facing the own
  side, 226 m from the centre), the north-east and south-west basin
  (`supply_[ns]_basin`), the hollow on the north-west and south-east edge
  plateau (`supply_[ns]_hollow`, sheltered, one ramp down from the
  plateau, 443 m from the own corner start).
- **Gun positions** `battery_n1`–`n3` (images `battery_s1`–`s3`) at the
  rims of the north, east and north-west plateau, 8 m behind the top of
  the cliff: 40 m further back the plateau itself hides the basin below
  (a 2 m eye over a flat top sees ground 35 m down only from about 700 m
  on, beyond the sight range).
- **Basins**: hollows (`[[hill]]` with negative height, 2.5–4 m) with
  dry tall grass in blotches (dark ground, full speed: room for tanks)
  and dry scrub (`kind = "scrub"`); tracks from the starts through the
  basins to the centre and to every ramp foot; two abandoned farmsteads
  (three ruined desert houses, mud walls, a well) by the tracks in the
  west basin and the north-east basin. Markers `basin_[ns]_west`,
  `basin_[ns]_east` (range targets), `needle_[ns]_k`, `needle_[ns]_l`
  (tops of buttes without a ramp).

**Sight** (`just map-sight mesa_desert`, sight 1/4 of the real one): from
the rims the gun positions see 7.7–12.7 ha, 3–6 ha of it 20 m or more below
(the basins under the cliff, a quarter circle of about 300 m with the
downhill bonus), never more than 2.2 % of the land; a basin sees about
18 ha around it, nothing from above; the big butte and the hollow see
little (3 and 2 ha: their own plateau). `--only=battery_n1` (and so on)
draws one point's sight; `build/mesa_desert/final/sight_*.png`.

**Ranges** (game range = real range × 1/8; distances from `just
map-sight mesa_desert`, the south half the same mirrored):

| From | Gun (catalog) | Game | What it reaches |
|---|---|---|---|
| the centre | medium mortars | 300–380 m | both buttes (226 m) |
| `battery_n1` (north plateau, 41 m) | 120-PM, 12 cm GrW 42, 152 mm, Panzerwerfer, sFH 13, BM-13 | 725–1 092 m | the centre (687 m), the enemy butte (907 m) and the enemy gun position on the east plateau (1 036 m, from 1 075 m guns) |
| | light howitzers and field guns | 1 099–1 541 m | the enemy basins (1 286 / 1 496 m), the enemy basin supply point (1 449 m), the enemy gun position on the north plateau (1 374 m); no enemy start (1 920 m or more) |
| `battery_n2` (east plateau, 37 m) | 12 cm GrW 42 and longer | 756–1 092 m | the centre (743 m), the enemy butte (728 m), the enemy west basin (745 m) |
| | light howitzers | 1 099–1 541 m | the enemy start in the south-east (1 361 m, from the M7), the enemy hollow (1 373 m) and the enemy gun positions (1 036–1 199 m) |
| `battery_n3` (north-west plateau, 41 m) | light howitzers | 1 334–1 541 m | the centre (1 226 m), the enemy butte (1 451 m), the enemy gun position on the east plateau (1 199 m); nothing else |
| any | light mortars (58–227 m) | – | from the rims only the ground at the foot |

No mortar or rocket launcher reaches an enemy start from a plateau (the
nearest is 1 361 m away). The test `mesa_desert_ranges_and_sight`
(`crates/cli/src/map_sight.rs`) checks: every gun position sees 2 ha or
more 20 m below and less than a twentieth of the land, a basin sees
nothing from above, no mortar or rocket launcher reaches an enemy start
from a gun position, light howitzers reach the centre from every gun
position (the shortest from the north and east plateau), the buttes lie
within the medium mortars of the centre. Since prompt 126 `map-sight`
also counts the `basin_*` markers of the other side as range targets.

Checks: `just map-check mesa_desert` passes; `just map-reach
mesa_desert`: every start, supply point, gun position and ramp top in the
main part for vehicles and infantry (89 % of the land open to vehicles,
10 % nobody: the cliffs, ramp sides and the edge strip), the tops of the
buttes without a ramp separate (`needle_*`), path lengths of both sides
equal; the test `mesa_desert_is_point_symmetric`. `just stuck-audit
--map mesa_desert`: 55 group orders, 176 units, all arrived (up the ramps
to the butte and the hollows). Pictures: `just mesa-desert-shots` (top,
oblique, centre ring, north plateau, a ramp, the hollow, the north-east
basin and the east plateau, beside the template if it is there). AI and
frame rate: [AI.md](AI.md#the-mesa-desert-prompt-126),
[PERFORMANCE.md](PERFORMANCE.md#mesa-desert-prompt-126).

#### `kessel_lake` ("Kesselsee", prompt 128)

An **own design** the user accepted (a drawing in `docs/.vorlagen/kesselsee/`,
not in the repository; the idea came from the minimap of a commercial game,
which is not rebuilt: only similar ingredients in an own arrangement).
Built around a **rock basin in the middle**: vehicles get into it only
through two passages, infantry also over two climbs; the lakes give a slow
way round over fords, the mountains a gun position on their foot.

- **Size** 2.0 × 2.0 km, biome `forest`, look `rock` (above), weather
  clear; point-symmetric about (1, 1): the north half counts, the sketch
  writes every line for both halves. Relief from the lake beds to 123 m
  (the corners); a broad 4 m lift keeps the land above 0 m.
- **The Kessel**: two C-shaped mesas (`[[mesa]]`, 22 m, foot from radius
  212 to 290 m around (1, 1), scree with boulders and a 180 % cliff on both
  sides, crags of massifs, outcrops and a few pines on top). The arcs end
  36 m beside the north–south axis: the two **passages** are about 70 m
  wide at the foot and flat; the field track from the ring track runs
  through both and the hamlet (three houses per half) on the meadow
  inside. **Climbs** (`climbs_m`, 38 %, 16 m, boulders across them): over
  the east arc in the north-east (outside and inside) and, as its image,
  over the west arc in the south-west; the ring top is open to infantry
  only (`height_*_rim`, `climb_*`).
- **Ring track** (radius 370 m, a closed `[[road]]`; a `[[ring_road]]`
  would flatten everything inside it), the main roads from the west and
  east start, the branches of the villages and the lake ways join it.
- **Lakes** north-west and south-east (`[[lake]] kind = "river"`): beaches,
  an islet with trees and a sandbank; two **fords** (`[[shoal]]`, 18 m)
  lead from the shore at the ford supply point over the islet to the beach
  in the north (south-east: the image), a slow way along the lake beside
  the lake way (shallow water, 30 % speed).
- **Mountains** north-east and south-west: three rock mountains (`peak`)
  per corner (summit 123 m in the corner, ridges along the edges), rock
  and crag woods above, conifers at the foot; the north and south start
  lie on the gentle foot.
- **Villages** north (crossing at (321, −571)) and south: loose houses
  along the main road and its branches (`[[town]]`, 26 m apart), golden,
  green and ploughed fields east and south-east, a field west; farmsteads
  at (643, −257) and its image; woods by the lake way, groups by the
  starts.
- **Starts**: team 0 west (player 0, (−855, 109)) and south (player 2),
  team 1 east (player 1, (857, −107)) and north (player 3, (321, −891));
  1v1 west against east (1.7 km). **Supply points**: the hamlet in the
  Kessel (`supply_kessel`, its own image), the villages
  (`supply_[ns]_village`), the fords (`supply_[ns]_ford`).

**Sight** (`just map-sight kessel_lake`; picture `--only=height_n_rim,supply_kessel,height_n_foot`):
the hamlet sees 14.7 ha, the inside of the Kessel and two strips out
through the passages, nothing over the ring; the rim (24 m) sees 3.2 ha,
2.6 ha of it 20 m below (the inside of the Kessel); the mountain foot (17 m)
11–13 ha down its slope; villages and fords 17–19 ha around them.

**Ranges** (game range = real range × 1/8, from `just map-sight
kessel_lake`; the other half the same mirrored):

| From | Target, distance | Reached by |
|---|---|---|
| the hamlet (centre) | both villages 547 m; both fords 680 m; the foot gun positions 836 m; the starts 863 m (east / west) and 948 m (north / south) | the villages from the Grille / sIG 33 (588 m) on (19 of 30 weapons); the fords from the heavy mortars (725 m); the foot positions from the Panzerwerfer (862 m); the starts from the sFH 13 (1 075 m) on |
| a start (east / west) | the Kessel 835 m; the enemy village 1 272 m; the enemy ford 1 385 m; the enemy south / north start 1 520 m; the opposite start 1 697 m | the Kessel from Panzerwerfer on (13); the village from light howitzers (9); the near enemy start only with the 25-pdr and leFH 18 (4); the opposite start with none |
| a start (north / south) | the Kessel 921 m; the own-side ford 935 m; the enemy west / east start 1 526 m; the opposite start 1 869 m | sFH 13 and longer (12); 25-pdr, leFH 18 (4); none |
| the mountain foot (`battery_*_foot`, 17 m up) | the Kessel 836 m; the enemy ford 1 088 m; the enemy village 1 381 m; the enemy west / east start 1 592 m | 13; BM-13 and longer (11); M7 and longer (7); only the Hummel (1) |
| a ford | its own islet and beach 208–260 m | every mortar from the medium ones |

No mortar or rocket launcher reaches an enemy start from a start or the
mountain foot; the Kessel is in reach of medium artillery from everywhere.

Checks: `just map-check kessel_lake` passes; `just map-reach kessel_lake`:
every start, supply point, ford end and the foot gun positions in the main
part for vehicles and infantry (89.7 % of the land open to vehicles,
4.5 % to infantry only: the ring tops and woods), the ring tops separate
for vehicles but in the main part for infantry, path lengths of both
sides equal; tests `kessel_lake_is_point_symmetric` and
`kessel_lake_ring_has_two_passages` (with the passages closed no vehicle
gets out of the Kessel, infantry does over the climbs; each passage is
open over 60 m on the ring's center line; a tank drives over both fords).
`just stuck-audit --map kessel_lake`: 45 group orders, 144 units, all
arrived. Pictures: `just kessel-lake-shots` (top, oblique, the Kessel,
the hamlet, a climb, the lake with its fords, the north village, the
mountains; beside the drawing if it is there). AI and frame rate:
[AI.md](AI.md#the-kessel-lake-prompt-128),
[PERFORMANCE.md](PERFORMANCE.md#kessel-lake-prompt-128).

#### `pit_hill` ("Grubenhügel", prompt 129)

An **own design** the user accepted (a drawing in
`docs/.vorlagen/grubenhuegel/`, not in the repository; its colours are
height bands; the idea came from the minimap of a commercial game, which is
not rebuilt). The map lives from its **relief**: valley floor, slopes,
uplands, rock bands and summits differ in sight, speed and ways.

- **Size** 2.0 × 2.0 km, biome `forest` without a look, weather clear;
  point-symmetric about (1, 1) (the north half counts, the sketch writes
  every line for both halves; the numbers come from a small script:
  ellipses, winding ramps). Heights from the lake beds (−0.1 m) to 115 m;
  the valley lies at about 5–8 m (a broad 5 m lift), so the values below
  count from it. The relief stays under 120 m on purpose: from 120 m on the
  terrain shader turns to its mountain look (snow on the summits, grass up
  to 42 % instead of earth from 20 %). Without a look the 32 % slopes show
  bare earth: the slopes read as brown bands like the height bands of the
  drawing; the `rock` look of `kessel_lake` kept them green (stage 1).
- **The pit hill** (`[[mesa]] name = "hill"`): foot 452 × 416 m, an even
  32 % slope to the upland ring at 35 m (343 × 307 m); the top
  (`[[mesa.tier]]`, 60 m) rises from 312 × 282 m at 34 % to 235 × 206 m.
  Rock bands (the top 18 m / 14 m of the slope at 180 %): on the lower slope
  the north and south quarters, on the upper slope the east and west
  quarters; the open quarters of the upper slope carry two rows of large
  boulders (`stop_vehicles`). So vehicles get onto the top only over the
  **two winding ramps** (`ramp_paths_m`, 18 m bed, track): from the
  north-east foot round to the north (398 m, 8.8 %) and on round the top's
  slope to its north-west rim (294 m, 8.5 %), and the image from the
  south-west; they reach the upland ring also over the open lower slopes
  (32 %, 36 % speed uphill). Infantry climbs everywhere except the rock
  bands. On the top a mining village (houses, two halls, a stone tower as a
  stand-in for the headframe, crates and timber) and **two pits** (hollow
  tiers, 64 × 43 m, 10 m deep, scree and a cliff, a 20 % ramp from the east
  / west end); a foot way crosses the top north–south with a gap at the
  boulder rows (a track there would open them to vehicles).
- **Side ridges** west and east: crescents between the ring track and the
  hill, 30 m high, 32 % slopes, the upland (35–50 m wide) along the middle;
  a dip of about 60 m between ridge and hill keeps them two heights. The
  drawing's ridges overlapped the hill's foot; to give them room the **ring
  track** is an ellipse of 750 × 590 m (drawing 636 × 579 m).
- **Mountain ranges** north and south (the drawing's outlines, run on
  250 m beyond the edge so that the slope is measured from the foot inside
  the map): a 40 m mesa whose slope is a rock band all round (`cliffs_m`
  with two equal points) above a 20 % foot, the only part open to vehicles;
  a rock step to 75 m, the summit block to 100 m and a rock peak on it
  (115 m), conifers on the upland, crags on the rock. A way along the foot.
- **Lake bays** north-west and south-east (`[[lake]]`, deep): the
  peninsula from the west edge (y −690 … −470 m, to x −716 m, a 17 m hump)
  hangs on a 55 m wide strip of land along the edge; the islet lies in the
  bay south of it (moved from the drawing, where it lay on land).
- **Railway** along the west edge (`[[rail]]`) to a **loading yard** outside
  the ring track (hall, crates, barrels), a **station** (station building,
  goods shed); the main road from the ring track past them to the far start;
  hamlets at the mountain foot (five houses) with tracks to the ring track
  and the main road; rocks (massifs, outcrops) in the valley and on the
  hill's slope; small woods at the ridge ends, the mountain foot, the lakes,
  the starts; tall grass in blotches on the slopes.
- **Starts**: team 0 west (player 0 (−715, 285), player 2 (−751, 921)),
  team 1 east (the images); 1v1 player 0 against 1. **Supply points**:
  the top (`supply_kuppe`), the stations, the hamlets, the ridge tops, the
  peninsulas (`supply_[ns]_*`). Markers on path cell centres.

**Sight** (`just map-sight pit_hill`; picture
`--only=height_n_rim,height_n_plateau,height_n_ridge_rim,height_n_foot,height_n_valley`).
The game's sight is 250 m (1/8 of 2 km) plus up to 25 % downhill, shorter
than the distances of the drawing, so the staging holds within it: the
middle of the top sees only the top (11.3 ha, its flat 235 m hide every
slope); the rim of the top (66 m) sees 8.6–10.7 ha, 1.7–3.6 ha of it 20 m
lower, down the slopes into the ring valley (the valley marker north of the
hill) and onto the upland ring; the upland ring 6–9 ha; the ridge rim (36 m)
6.7 ha, the dip and the hill's lower slope, not the upland (280 m) and not
the village; the mountain foot (12 m) 10–11 ha of its own valley; the
valley 18 ha. Both halves differ by up to 2 ha (8 m vision cells, as on
the other point-symmetric maps).

**Ranges** (game range = real range × 1/8, from `just map-sight pit_hill`):

| From | Target, distance | Reached by |
|---|---|---|
| the top (centre) | ridges 612 m, hamlets 642 m, mountain foot 717 m, the near starts 743 m, peninsulas 1 022 m, stations 1 139 m, far starts 1 167 m | ridges, hamlets, foot from the heavy mortars (su_pm38, 725 m) on (17 of 30); near starts from the GrW 42 (16); peninsulas from the sFH 13 (12); stations and far starts from the leFH 18/39 (9) |
| a station | the top 1 139 m; the own ridge 1 623 m; enemy points 1 559–2 317 m | the top from 9 weapons; the rest only the Hummel or nobody |
| a hamlet | the top 642 m; the enemy near start 1 387 m | 17; 7 (M7 and longer) |

No mortar or rocket launcher reaches an enemy start from a start, station
or hamlet; the top is in reach of medium artillery from both sides.

Checks: `just map-check pit_hill` passes; `just map-reach pit_hill`: every
start, supply point, the upland markers, both pits and all ramp tops in
the main part for vehicles and infantry (91.5 % of the land for vehicles,
2.2 % infantry only: the top's slope behind the boulders, woods; the
mountain uplands are separate, as meant), path lengths of both sides
equal; tests `pit_hill_is_point_symmetric` and
`pit_hill_top_only_over_ramps` (with the upper ramps closed no vehicle
gets off the top, infantry does; a ray through a lower rock band is closed
even to infantry; a tank drives from every start onto the top, the upland
ring and into both pits; both ramp legs under 12 %), the map in the
generated-twice test. `just stuck-audit --map pit_hill`: 65 group orders,
208 units, all arrived. Pictures: `just pit-hill-shots` (top, oblique, low
from the side, the hill, the top, a ramp, a ridge, the mountains, the lake,
the station; beside the drawing if it is there). AI and frame rate:
[AI.md](AI.md#the-pit-hill-prompt-129),
[PERFORMANCE.md](PERFORMANCE.md#pit-hill-prompt-129).

#### `omega_bend` ("Omegaschleife", prompt 130)

An **own design** the user accepted after two rounds (a drawing in
`docs/.vorlagen/omegaschleife/`, not in the repository; the idea came from
the minimap of a town map of a commercial game, which is not rebuilt): few
houses, much meadow and wood, every way curved, no landmark in the middle.

- **Size** 2.0 × 2.0 km, biome `forest` without a look, weather clear;
  mirror-symmetric west against east about x = 1 m (`mirror = "west"`,
  above; the sketch writes every line for both halves, the numbers come
  from a small script: circle, bends, the drawing's curves). Gentle ground
  (5–10 m: a broad 5 m lift, waves of 2–4 m away from the river, a 3 m
  rise under the peninsula village), the banks 3 m lower over 40 m.
- **River** (`[[river]]`, 85 m wide, one line of points): straight from
  the west edge at y = −179 m, a bend of 112 m radius tangent to the
  **loop** (a circle of 320 m round (0, −594), its top 86 m from the north
  edge), over the north and mirrored back to the east edge. The drawing
  meets the loop with a kink at the foot; the bend replaces it (no spike
  of land). The **neck** between the two bends is 392 m between the centre
  lines, 307 m of land.
- **Bridges**: four long stone bridges (`stone_long`, five high arches,
  100 m, deck 6 m above the banks) over the full width, the water not
  narrowed: north–south over the straight reaches at x = −711 / 713 m,
  west–east over the loop sides at y = −595 m.
- **Peninsula**: a church on the axis and four houses round the cross track
  between the loop bridges, a track from the neck along the axis through
  the village to the church, a small wood on each side of the church.
- **North quarters** (north of the river, outside the loop), reached only
  over the bridges: a farmstead with a grain field, a wood at the west /
  east edge, a small wood, a single house at the bridge; a road from the
  south bridge to the loop bridge, tracks to the edge and past the farm.
- **South**: hamlets of five buildings round a small square at the south
  bridgeheads and of three at the first starts; wood strips west and east
  of the middle (x ±400…540 m) in a darker meadow (tall grass in
  blotches), small woods, a larger wood in each south corner, two grain
  fields per side, scattered trees in the south-west and south-east; a
  deep pond in the middle (230 × 180 m); a small station at the south edge
  (a goods hall over the end of a track on the axis, two wings, a
  forecourt) and a single house on each side. Woods are open stands
  (9 m spacing, frame rate). A way along the axis could not snake and stay
  symmetric: neck → pond and pond → station are each two mirrored bows (a
  lens), so both sides have their own way past the pond.
- **Starts**: team 0 west (player 0 (−855, 605), player 2 (−819, 913)),
  team 1 east (the images); 1v1 player 0 against 1. **Supply points**:
  north of the pond, the peninsula village and the station forecourt on
  the axis; the loop bridges (in the north quarters), the south
  bridgeheads and the start hamlets (`supply_w_*` / `supply_e_*`). Every
  point on a path cell centre.

**Sight** (`just map-sight omega_bend`, pictures `--only=supply_village`,
`--only=supply_pond`, `--only=height_w_woodstrip,height_e_woodstrip`). On
gentle ground the game's 250 m decide: north of the pond 18.3 ha round
the pond (the water and the two lens ways), the peninsula village 15.8 ha
(church and houses shut out parts, the loop banks are at its edge), the
east edge of a wood strip 13.6–14.6 ha towards the middle (the wood itself
hides everything behind it), a loop bridge 17 ha of its quarter and the
bridge, a south bridgehead 11.6 ha; the gun positions see nothing of the
other side.

**Ranges** (game range = real range × 1/8, from `just map-sight
omega_bend`):

| From | Target, distance | Reached by |
|---|---|---|
| a start (`battery_*_start`) | enemy starts 1 679–1 680 m, enemy bridgehead / hamlet / start 1 576–1 648 m, the pond 983 m, the village 1 324 m | the enemy starts: nobody; the enemy hamlet and bridgehead only the Hummel; the pond from the sFH 13 on (12 of 30), the village from the leFH 18/39 (9) |
| behind the wood strip (`battery_*_wood`) | enemy wood position 1 128 m, enemy starts 1 448–1 505 m, enemy bridgehead 1 316 m, the pond 631 m, the village 966 m | the enemy wood position and bridgehead from the light howitzers (9); the enemy starts from the M-30 / 25-pdr on (4–5); the pond from the heavy mortars on (17) |
| the peninsula village | the loop bridges 520 m, the pond 500 m, south bridgeheads 850 m, hamlets 1 251 m | bridges and pond from the 4.2-inch / Calliope on (20–21); bridgeheads from the Panzerwerfer (13) |
| north of the pond | the village 500 m, station 692 m, bridgeheads 713 m, loop bridges 815 m, hamlets 903 m | 21, 17, 17, 13, 12 |

No mortar reaches an enemy start or the enemy half's supply points from a
start; the middle (pond, village) lies within heavy mortars and medium
artillery of both sides.

Checks: `just map-check omega_bend` passes; `just map-reach omega_bend`:
every start, supply point and marker in the main part for vehicles and
infantry (96.3 % of the land for vehicles, 3.0 % infantry only: woods),
path lengths of both sides equal; tests `omega_bend_is_mirror_symmetric`
and `omega_bend_north_only_over_bridges` (with every bridge down nobody
gets from a start into the north quarters, everybody still onto the
peninsula over the neck; with only the south bridges down the peninsula
is reached from the north quarters over the loop bridges), the map in the
generated-twice test. `just stuck-audit --map omega_bend`: 115 group
orders, 368 units, 365 arrived; two Panzer IV of a group gave up beside
the north ramp of the west south bridge (slope of the embankment, as on
`archipelago` [open]). Pictures: `just omega-bend-shots` (top, oblique,
loop, peninsula village, a south bridge, a loop bridge, the neck, the
pond, a wood strip, the bridgehead hamlet, the north quarter, the
station; beside the drawing if it is there). AI and frame rate:
[AI.md](AI.md#the-omega-bend-prompt-130),
[PERFORMANCE.md](PERFORMANCE.md#omega-bend-prompt-130).

## Preview pictures (prompt 201)

The map picker (`godot/scenes/menu/map_picker.gd`) and the setup screens
show a top view of a map in the style of the minimap, built from the map
folder without starting a match (`SimHost.map_preview_build`): terrain color
with hill shading, roads, forest and buildings (from a vision grid with a
2–8 m cell, about 640 px on the longer edge), water, bridge decks; over it
the starts of the chosen start layout, the supply points of that layout, a
scale bar and a north arrow; or a small 3D relief from a coarse height grid.
Pictures are cached in `user://map_previews/<name>.png` (+ `.dat`) and built
again when a map file changes (modification time and size), when
`CACHE_VERSION` in `menu/map_preview.gd` is raised, or after the map editor
saved the map. The picker's "Made from" column reads `generator` in
`map.toml`: `tools/geo_import.py` → real geodata, `cli gen-sketch` → picture
sketch, other `cli gen-*` → generator, none → built by hand (editor maps).
A map may get a description through the translation key
`MAP_<NAME>_DESC` in `godot/locale/menu.csv`. Times and sizes:
[ARCHITECTURE.md](ARCHITECTURE.md) (main menu, "Map picker and previews").

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
