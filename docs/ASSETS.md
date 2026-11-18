# Asset conventions

Binding rules for every model in the game. All Blender prompts (004–008) build on
this document. Sizes and other real values come from the unit catalog
([UNIT_CATALOG.md](UNIT_CATALOG.md)); sources and legal rules are in
[SOURCES.md](SOURCES.md).

## Pipeline

```
data/catalog/<faction>.toml                 main dimensions (length, width, height ...)
assets-src/blueprints/<faction>/<id>.toml   build plan: parts and their parameters
        │  just unit <faction> <id>   (tools/blender/build_unit.py + tools/blender/kit/)
assets-src/blender/<faction>/<id>.blend     generated source file (Git LFS)
godot/assets/models/<faction>/<id>.glb      exported model (Git LFS)
        │  just import   (Godot import, assets/import/unit_post_import.gd)
Godot scene with the unit_camo shader       palettes and markings applied at runtime
```

- `<faction>` is `de`, `su`, `us` or `neutral`; `<id>` is the catalog id
  (`de_pz4_h`, `env_tree_birch`). One model per catalog entry; camouflage variants
  come from the material system, not from separate models.
- Vehicles, guns and aircraft are **generated** from blueprints by the part kit
  (next section); the .blend is an output, not edited by hand. Environment
  objects (prompt 007) are generated too, with their own builder (section
  "Environment objects"). Hand-made models are exported with
  `just export-model` (`tools/export_blender.py`, `-- --output <glb>` for the
  faction folder).
- When a model is exported, its catalog entry gets `model = "assets/models/<faction>/<id>.glb"`.
- Binary files (`.blend`, `.glb`, textures) are tracked by Git LFS (`.gitattributes`).
- `tools/create_placeholder_tank.py` generates the old placeholder tank from
  primitives. `tools/blender/scale_reference.py` builds the 1.80 m reference figure and
  scale bar (`assets-src/blender/reference/scale_reference.blend`) for line-up renders.

## Part kit (prompt 004)

`tools/blender/kit/` is a Python package that runs only inside Blender
(`blender -b`, no running Blender needed). It turns a blueprint into a finished
model with names, hierarchy, sockets, material slots and baked vertex colors.

| Module | Content |
|---|---|
| `core` | `MeshBuilder` (box, profile extrusion with sloped sides, loft, cylinder, lathe, gear, plate), `Model` (objects with pivots, sockets, `finish()`) |
| `running_gear` | tracks around wheels; `leaf_spring_bogie` (Panzer IV), `christie` (T-34), `vvss_bogie` (Sherman), `halftrack_rear`; prompt 008: `torsion_bar` (single or twin wheels on swing arms, optional return rollers: StuG III, T-70, IS-2, M26, Wespe), `interleaved` (Schachtellaufwerk in 2 or 3 layers: Tiger I, Luchs) |
| `wheels` | road wheels, idlers, sprockets, rollers, tire wheels, `wheel_axle` (trucks, cars, carriages) |
| `hull` | `plate_hull` (lower hull + superstructure from side profiles, sloped side plates, fenders, optional cast look), `deck_box`, `hull_hatch` |
| `turret` | `plate_turret` (plan + side slopes; `open_top` for M10, M8, BA-64), `cast_turret` (superellipse rings), `cupola` (`parent = "Hull"` on casemates), `turret_hatch`, `turret_bustle_box` |
| `weapons` | `tank_gun` (mantlet types incl. `saukopf` and `ball`, barrel segments, muzzle brake types, `x` offset for casemate guns), `mg` (coax, ball mount, pintle), `gun_shield` |
| `attachments` | `skirts`, `turret_skirt`, `spare_tracks`, `tool`, `jerrycans`, `headlight`, `antenna`, `exhaust`, `tow_hook`, `box_prop`, `cylinder_prop`, `marking`, `socket` |
| `artillery` | `split_trail_carriage`, `field_gun` (`parent` Mount and a "fit" barrel segment for AA guns) |
| `trucks` | `ladder_frame`, `engine_hood`, `truck_cab`, `cargo_bed`, `truck_fenders` and the `wheeled` assembly (trucks, cars; prompt 006) |
| `aircraft` | `fuselage` (loft of cross-sections, glazed nose), `wing` (taper, sweep, dihedral, gull `kink`, ailerons), `tail`, `propeller`, `canopy`, `landing_gear` (extended/retracted, `fixed`, `spats`, `skid`); prompt 008: `nacelle`, `wing_struts`, `upper_wing` (biplanes), `fixed_guns`, `flex_mg`, `gun_turret`, `bomb_load`, `rocket_rails`, `bomb_bay`, `shadow_proxy` |
| `vehicles` | `section_hull` (hull lofted from cross-sections, optional open compartment), `fender`, `plate` (doors, lids on sloped faces), `pintle_mg` (MG on a post, optional shield); assembly `halftrack` (prompt 006) |
| `casemate` | prompt 008: `casemate` (closed or open superstructure from a plan with slopes, optional `rear_deck` for an engine deck that drops behind the roof, `wall_shell` helper) and the assembly `casemate` (tank destroyers, assault guns, SPGs: no turret, `Gun` on `Hull`) |
| `armored_car` | prompt 008: assembly `armored_car` (wheels, `section_hull`, optional turret, gun, MGs) |
| `antiair` | prompt 008: `cruciform_platform`, `aa_mount` (traversing `Mount` with seats, optional shield), `crew_sockets` (any gun); assembly `aa_gun` |
| `mortars` | prompt 008: `baseplate`, `mortar_tube` (elevation fitted to the catalog height), `mortar_bipod` (fitted to length and width); assembly `mortar` |
| `early_tanks` | prompt 063, for the vehicles of the reference folder (docs/VEHICLE_REFS.md): `small_wheel_bogie` (small or large single wheels in bogies of 2-4 with leaf or coil springs, optional outer girder: T-26, Pz.Kpfw. I, 35(t), 38(t), T-27, T-37A, R35, H39, Lorraine, Elefant), `hvss_bogie` (M4A3E8), `sub_turret` (secondary `GunTurret_NN` with a fixed gun and/or MGs on the hull or on the turret: T-35, M3 Lee cupola), `sponson_gun` (fixed hull gun fitted to the catalog length, `Socket_Gun_NN`: M3 Lee), `amphibious_floats`, `screw_propeller` (T-37A, T-38, T-40); open superstructures of the captured SPGs use `casemate` with `open = true` |
| `forts` | prompt 008, env generators for faction fortifications: `tobruk`, `log_bunker` (DZOT, US field bunker), `minefield` (built by `build_env.py`) |
| `parts_de`, `parts_su`, `parts_us` | faction-specific parts, prefixed `de.`, `su.`, `us.` |
| `assemblies` | whole units: `tank`, `towed_gun`, `aircraft` (`halftrack` lives in `vehicles`, `wheeled` in `trucks`, the prompt 008 assemblies in their modules) |
| `bake` | vertex colors (see "Material system") |
| `render` | turntable and line-up renders (Eevee) |

Every part documents its parameters in its docstring. Unknown parameters are
errors, so typos in a blueprint fail the build. `just kit-test` builds demo
units that use every part (also those no real unit uses yet) and fails if a
part breaks or lacks a docstring.

**Catalog values are never repeated in a blueprint.** The assembly reads length,
hull length, width and height from the catalog and fits the free values:

- the hull profiles are scaled along Y to the catalog hull length (an error if
  the profile is more than 8 % off, so the drawing must be roughly right),
- tracks end at the catalog width, fenders reach it,
- the turret height (or the shield top of a towed gun) is fitted to the catalog
  height; the generator measures the finished model and corrects in a second pass,
- the barrel length (tanks) or trail length (towed guns) is fitted to the catalog
  overall length.

Part dimensions that are not in the catalog (road wheel diameter, plan of the
turret, plate angles) are modeling estimates written into the blueprint with
a comment on where they come from.

### Blueprint format

`assets-src/blueprints/<faction>/<id>.toml`, meters, model space (Z up, front +Y,
origin on the ground at the footprint center):

```toml
unit = "de_pz4_h"          # catalog id, must match the file name
assembly = "tank"          # tank | towed_gun | aircraft | wheeled | halftrack | casemate | armored_car | aa_gun | mortar

[layout]                   # assembly-level values
turret_y = 0.28            # turret ring center
turret_ring_diameter = 1.68

[running_gear]
part = "leaf_spring_bogie" # every table names its part ...
track_width = 0.40         # ... and the part's parameters
road_wheels = [1.66, 1.16, 0.71, 0.21, -0.21, -0.71, -1.16, -1.66]
sprocket = { y = 2.42, z = 0.74, diameter = 0.68, teeth = 20 }

[hull]
part = "plate_hull"
lower = [[-2.82, 0.42], [2.30, 0.42], ...]   # side profile [y, z]

[turret]
part = "plate_turret"
height = "fit"             # derived from the catalog height

[cupola]                   # optional
[gun]                      # barrel segment length "fit" = catalog length

[[part]]                   # any number of extra parts, built in order
part = "skirts"
y = [-2.35, 2.05]

[[mg]]                     # machine guns, after the gun
part = "mg"
mount = "coax"

[validate]                 # what the catalog dimension leaves out, with reason
exclude_width = ["Skirt_*"]      # catalog width is without skirts
exclude_height = ["Antenna_*"]
```

Build order of a tank: layout, running gear, hull, turret, cupola, `[[part]]`,
gun, `[[mg]]`. Towed guns: gun, carriage, shield, `[[part]]`. Aircraft:
fuselage, wing, tail, canopy, landing gear, `[[propeller]]`, `[[part]]`.
Halftracks: running gear (`halftrack_rear`), `[front_axle]` (`wheel_axle`,
wheels numbered after the road wheels), hull (`section_hull`), `[[part]]`,
`[[mg]]`; `[layout] track_outer_x` sets the track position, and the model is
moved so that the origin lies at the center of the ground contact (example:
`assets-src/blueprints/de/de_sdkfz251_1.toml`).

`section_hull` sections give, per Y station, three heights with half-widths
(bottom, chine, top): lower plates flaring out and upper plates leaning in
(Sd.Kfz. 251) or vertical sides (equal half-widths). `open = [y_front,
y_rear]` cuts an open-topped compartment with walls and floor between two
stations; Y and the top heights are fitted to the catalog like `plate_hull`.

fuselage, wing, tail, canopy, landing gear, `[[propeller]]`, `[[part]]`. Wheeled
vehicles: `[[axle]]`, frame, hood, cab, body, fenders, `[[part]]` (body rear end,
body width and cab roof are fitted to the catalog length, width and height).

Prompt 008 assemblies (each docstring has the details; `just kit-test` builds a
demo of each, `just kit-test-render` renders them):

| Assembly | Build order | Fitted to the catalog | Example |
|---|---|---|---|
| `casemate` | layout, running gear, hull, `[casemate]`, cupola, `[[part]]`, gun (model space trunnion, parent Hull), `[[mg]]` | casemate roof (or open wall top) = height, barrel = length | `de/de_stug3_g.toml` |
| `bridgelayer` | layout (`fender_overhang`: catalog width is the bridge's), running gear, hull, `[[part]]` (the bridge: parts with `parent = "Bridge"`) | catalog length, width and height include the bridge | `su/su_valentine_bridgelayer.toml` |
| `armored_car` | `[[axle]]`, hull (`section_hull`, `height = "keep"` with a turret), turret, cupola, `[[part]]`, gun, `[[mg]]` | hull length, turret top = height, barrel = length | `su/su_ba64b.toml` |
| `aa_gun` | `[carriage]` (cruciform_platform), `[[axle]]`, `[mount]` (aa_mount), gun (field_gun on Mount), `[[part]]` | side arms = width, barrel = length (from the rear end) | `us/us_bofors_m1.toml` |
| `mortar` | baseplate, tube, bipod, `[[axle]]` (transport wheels), `[[part]]` | elevation = height, bipod feet = length and width | `us/us_m2_42in.toml` |
| `aircraft` | (as above) plus `[[part]]` with nacelles, struts, guns, stores, bomb bay, `shadow_proxy` last | wingspan (`span = "fit"`) | `us/us_l4.toml` |

Aircraft are built standing on their landing gear with the gear extended; the
root extras `gear_retractable` says whether the game hides `Gear_*` in flight.
Faction fortifications (`fort_bunker`, `fort_minefield_*` of de/su/us) are
environment objects: `assembly = "env"` blueprints in the faction folder, built
by `just env <id>` with the `forts` generators (examples
`de/de_bunker_tobruk.toml`, `de/de_minefield_at.toml`). Catalog entries with
height 0 (minefields) give the marker height as `height_m` in the variant.

### Generator, checks and previews

| Command | Does |
|---|---|
| `just unit <faction> <id>` | builds one unit: `.blend`, `.glb` and `build/renders/<id>_turntable.png` (4 views) |
| `just units` | builds every unit with a blueprint |
| `just validate-assets` | checks every built model (below); exit code 1 on failure |
| `just lineup` | renders all models in a row with the 1.80 m figure: `build/renders/lineup.png` |
| `just kit-test` | smoke test of all parts |
| `just ref-compare <id>` | prompt 063: side, front and top view of a vehicle model at a fixed scale (80 px/m) with dimension lines, a table model / data.json / catalog and the drawings of its reference folder: `build/renders/compare/<id>.png` and `.md` (never committed; without a model only table and drawings) |
| `just import` | Godot import after new or rebuilt models (runs the import script) |
| `just gallery` / `just gallery-check` | model gallery in Godot / its headless self-test |

`build/` is ignored; set `RENDER_DIR` to render elsewhere.

`tools/blender/validate.py` checks per model: .blend and .glb exist and were
built from the current blueprint and catalog entry (source hash on the root);
root empty named after the id at the origin; scale 1 everywhere, no rotation on
meshes; names and parents from the table below; required parts and sockets per
category (tanks: tracks, wheels, sprocket, idler, gun, `Socket_Muzzle` on the
muzzle, exhaust, hatch, fire and marking sockets; towed guns: carriage, wheels,
`Socket_Tow`, one `Socket_Crew_NN` per crew member); one `MG_NN` per machine gun
in the catalog; only the slot materials; baked `Color` attribute; triangle budget;
length, width and height within ±2 % of the catalog; lowest point at z = 0 and
the ground contact centered on the origin; the .glb has the same objects, bounds
and vertex colors.

**LOD:** Godot's automatic mesh LODs (`meshes/generate_lods`, on by default) are
used. The budgets above keep LOD0 small already, and at medium zoom a tank covers
60–100 px, where the automatic LODs remove most triangles. No own LOD meshes for
now. At run time the static parts of a vehicle are merged per unit type into
one mesh for the hull, one for the turret and one for the `Gun` (it recoils
and raises its barrel since prompt 077; `godot/scenes/world/unit_merge.gd`,
prompt 016): a tank was 30–50 draw calls per pass. Whether a gun has a
muzzle brake is read from the width of its muzzle in the mesh (prompt 077). Parts keep their names and
transforms as empty nodes, so sockets and the turret still work; tools that
inspect models (gallery, validation) see the unmerged scene.

### New unit in 5 steps

1. **Catalog:** make sure the entry in `data/catalog/<faction>.toml` has length,
   width and height (and `length_hull_mm` for vehicles with a gun) with sources.
2. **Blueprint:** copy the closest blueprint to
   `assets-src/blueprints/<faction>/<id>.toml`, set `unit`, pick parts, draw the
   profiles and plan in meters. Missing a part? Add it to the kit (faction parts
   to `kit/parts_<faction>.py`), additively, with a docstring.
3. **Build:** `just unit <faction> <id>` and look at
   `build/renders/<id>_turntable.png`; iterate on the blueprint.
4. **Check:** `just validate-assets` until it is green; add `[validate]`
   exclusions only where the catalog dimension really leaves a part out, and say
   why in a comment. Set `model = "assets/models/<faction>/<id>.glb"` in the catalog.
5. **Look in Godot:** `just import`, then `just gallery`: check the model in all
   three camouflages and the markings; `just lineup` for the size comparison.
   Commit blueprint, .blend, .glb, .glb.import and the catalog change.

## Space, scale and orientation

- **1 Blender unit = 1 m, real size (1:1).** No enlargement for readability, not even
  for infantry (reference figure 1.80 m). Readability comes later from markers and icons.
- **Z up** in Blender (Y up after glTF export).
- **Front faces +Y** in Blender, which becomes −Z in Godot.
- **Origin** on the ground, at the center of the footprint (tracks/wheels/feet touch
  Z = 0). Aircraft: origin on the ground under the center of gravity, model standing
  on its landing gear.
- **Rotation and scale applied** (`Ctrl+A`) on every object. Object scale is 1.

## Hierarchy and names

Parts that move separately are child objects with these names (English, PascalCase,
side `L`/`R`, two-digit index). The root object is named after the catalog id.

| Part | Name | Parent | Pivot |
|---|---|---|---|
| Hull / body | `Hull` | root | origin |
| Turret | `Turret` | `Hull` | turret ring center, rotates around Z |
| Carried bridge (bridge-laying tanks, prompt 055) | `Bridge` | `Hull` | origin; the game folds it out and hides it, it is not merged |
| Gun / barrel | `Gun` | `Turret` (or `Hull` for casemates, `Carriage` for towed guns) | trunnion, elevates around X |
| Towed gun carriage | `Carriage` | root | origin |
| Gun shield | `Shield` | `Carriage` | – |
| Wheels / road wheels | `Wheel_L_01`, `Wheel_R_01`, … | `Hull` | wheel center, rotates around X |
| Sprocket / idler | `Sprocket_L`, `Idler_L`, … | `Hull` | wheel center |
| Track | `Track_L`, `Track_R` | `Hull` | – (scrolling texture) |
| Return roller | `Roller_L_01`, … | `Hull` | roller center |
| Commander's cupola | `Cupola` | `Turret` | cupola base |
| Side skirts | `Skirt_L`, `Skirt_R` | `Hull` | – |
| Turret skirt | `TurretSkirt` | `Turret` | – |
| Antenna | `Antenna_01`, … | `Hull` or `Turret` | antenna base |
| Machine gun on mount | `MG_01`, … | `Turret` or `Hull` | mount pivot |
| Hatch | `Hatch_01`, … | `Hull`, `Turret` or `Cupola` | hinge |
| Propeller | `Propeller`, `Propeller_L`/`_R` | root | hub, rotates around Y |
| Control surfaces | `Aileron_L`, `Elevator`, `Rudder` | root | hinge |
| Landing gear | `Gear_L`, `Gear_R`, `Gear_Tail` (`Gear_Nose`) | root | – |
| AA gun upper carriage | `Mount` | `Carriage` | traverse axis, rotates around Z |
| Aircraft gun turret; sub-turret of a multi-turret tank (prompt 063) | `GunTurret_01`, … (with `MG_NN` children) | `Hull` (`Turret` for a cupola turret on the main turret) | ring center, rotates around Z |
| External bomb / rocket | `Bomb_01`, `Rocket_01`, … | `Hull` | store center (hidden on release) |
| Bomb bay door | `BayDoor_L`, `BayDoor_R` | `Hull` | outer hinge, opens around Y |
| Shadow proxy | `ShadowProxy` | root | flat planform, shadow-only in Godot |

**Attachment points** are empties (no geometry), parented to the moving part they
follow:

| Empty | Use |
|---|---|
| `Socket_Muzzle`, `Socket_Muzzle_02` | muzzle flash, tracer origin (on the barrel tip, +Y forward) |
| `Socket_Exhaust_01` … | exhaust smoke |
| `Socket_Hatch_01` … | commander/crew figure in hatch |
| `Socket_Crew_01` … | crew positions of guns and open vehicles |
| `Socket_Passenger_01` … | seats of transport vehicles |
| `Socket_Tow` | tow hook (trucks, guns) |
| `Socket_Fire_01` … | burning / damage effects |
| `Socket_Marking_Turret_L` … | decal positions for national and tactical markings; local +Z (Godot +Y) is the surface normal, +Y (Godot −Z) is up; glTF extras `marking` = `national` or `number`, `marking_size` in m |
| `Socket_Muzzle_02` … | muzzles of machine guns: `MG_01` has `Socket_Muzzle_02`, `MG_02` has `Socket_Muzzle_03` |
| `Socket_Gun_01` … | muzzles of fixed aircraft guns (`Socket_Muzzle` sits at their mean position); muzzle of a hull sponson gun (M3 Lee, prompt 063) |
| `Socket_Bomb_01` … | bomb release points (external stores and bomb bay stations) |
| `Socket_Rocket_01` … | rocket launch points (rocket tails) |

## Polygon and texture budgets (LOD0)

The camera (prompt 002) looks down from 15 m to 600 m with a narrow field of view.
At medium zoom a 7 m tank covers roughly 60–100 px on a 1080p screen, and the scene
must hold hundreds of units plus dense vegetation. More geometry than below is
invisible in play and costs frame time. Lower LODs come from Godot's automatic mesh
LOD on import; own LOD meshes only if that proves insufficient (prompt 004).

| Category | Triangles LOD0 | Textures |
|---|---|---|
| Soldier incl. equipment | ≤ 3 000 | none: procedural `soldier.gdshader` (prompt 042, [INFANTRY.md](INFANTRY.md)) |
| Small arm, small part | ≤ 800 | none (soldier weapons: `soldier.gdshader`) |
| Truck, recon vehicle | ≤ 5 000 | 1024² |
| Tank, halftrack, SPG | ≤ 8 000 | 2048² `Paint`, 1024² others |
| Towed gun | ≤ 3 000 | 1024² |
| Fighter, attack aircraft, recon aircraft | ≤ 6 000 | 2048² |
| Bomber | ≤ 10 000 | 2048² |
| Fortification element | 500–2 000 | tileable trim sheets 1024² |
| Tree / bush / grass clump / rock | ≤ 2 000 / 500 / 50 / 300 | atlas 1024² with alpha |
| House / church | ≤ 3 000 / 5 000 | trim sheet 2048² per biome |
| Terrain ground layer | – (chunked grid, prompt 011) | 1024² per layer in two texture arrays (albedo + height, normal + roughness + AO), tile 3–8 m, i.e. 128–340 px/m |

Target texel density: about 256 px/m for vehicles, 512 px/m for soldiers. Use
tileable textures and trim sheets for buildings and fortifications so that biome
variants are cheap.

## Material slots

Every model uses only these material names, so the camouflage system (below)
can swap them per biome (forest, desert, winter) and faction:

| Slot | Use |
|---|---|
| `Paint` | painted surfaces; receives the camouflage scheme |
| `Metal` | bare metal: tracks, barrel ends, tools |
| `Rubber` | tires, rubber rims |
| `Wood` | wood: crates, stocks, buildings, bridges |
| `Canvas` | tarpaulins, covers, tents, sandbags |
| `Glass` | vision blocks, windows, canopies |
| `Skin` | exposed skin of soldiers |
| `Cloth` | uniforms and webbing; receives the uniform scheme |
| `Leather` | boots, belts, leather pouches and holsters (soldiers) |
| `Insignia` | plain fixed-color markings worn by soldiers, e.g. the white medic armband |

Environment objects may additionally use `Bark`, `Foliage`, `Stone`, `Plaster`,
`Ground` and `Roof` (roof tiles, slate, thatch and hay). New slots are added here
first. Their material kinds and shaders are described under "Environment objects".

### Material system (prompt 004)

- **In Blender** faces carry only slot materials and a baked corner color
  attribute `Color` (exported as glTF `COLOR_0`, `tools/blender/kit/bake.py`):
  R = ambient occlusion (ray traced against the whole model and the ground),
  G = edge wear mask (bevel strips of plate edges), B = height above ground / 4 m,
  A = 0 on track link surfaces (scrolling link pattern, UV v = links).
- **In Godot** one shader, `godot/assets/shaders/unit_camo.gdshader`, with one
  `ShaderMaterial` per slot (`godot/assets/materials/unit/<slot>.tres`, uniform
  `slot`). The import script `godot/assets/import/unit_post_import.gd` (default
  for scene imports in `project.godot`, acts only on kit models) swaps the slot
  materials for these on import.
- **Palettes** are `CamoPalette` resources per faction × biome:
  `godot/assets/materials/camo/<faction>_<biome>.tres` (`forest`, `desert`,
  `winter`): base coat, two camouflage colors, pattern (plain, soft spray,
  hard-edged, ambush), pattern size and coverage, whitewash, snow on top faces,
  mud color and amount from below, dust, edge wear. `UnitCamo.apply(model,
  palette)` writes them as **instance uniforms**, so all units share the same
  eight materials.
- **Markings** are Godot `Decal` nodes that `UnitCamo.add_markings(model,
  faction, number)` adds at the `Socket_Marking_*` nodes. The textures
  (Balkenkreuz, red star, white star, tactical numbers) are drawn at runtime by
  `UnitMarkings`; there are no image files.
- Colors are sRGB approximations of the historical paints named in each
  palette's `note` (e.g. Dunkelgelb RAL 7028, Olivgrün RAL 6003, Rotbraun
  RAL 8017, Soviet 4BO, US Olive Drab).

## Infantry

Squads (prompt 006) are catalog units made of soldiers: a squad file
`assets-src/blueprints/<faction>/squads/<id>.toml` lists the members (role,
formation position, gallery clip, note), `just squad <faction> <id>`
(`tools/blender/build_squad.py`) writes the Godot scene
`godot/assets/models/<faction>/<id>.tscn` that instances the soldier .glbs, and
the catalog `model` points to that scene. `just validate-squads` (also part of
`just validate-assets`) checks it. Details in [INFANTRY.md](INFANTRY.md).

Soldiers follow their own layout (prompt 005), described in [INFANTRY.md](INFANTRY.md):
one shared humanoid skeleton (Godot `SkeletonProfileHumanoid` bone names), body and
gear as skinned meshes, the weapon on the extra bone `Weapon` with `Socket_Muzzle`.
Files: `assets-src/blender/<faction>/infantry/<role>.blend` and
`godot/assets/models/<faction>/infantry/<role>.glb`, built by
`tools/blender/build_soldier.py` and checked by `tools/blender/validate_soldier.py`.

## Environment objects (prompt 007)

Vegetation, props, buildings, bridges, boundaries and field fortifications: the
catalog faction `neutral` (`data/catalog/neutral.toml`). They are generated from
blueprints like units, but have their own builder, structure and checks.

```
data/catalog/neutral.toml                  main dimensions (real values, sources)
assets-src/blueprints/neutral/<id>.toml    assembly = "env", one [[variant]] per variant
        │  just env [<id> ...]   (tools/blender/build_env.py + kit/env.py, vegetation.py,
        │                         parts_neutral.py, buildings.py,
        │                         parts_dumps.py)
assets-src/blender/neutral/<id>.blend      generated, Git LFS
godot/assets/models/neutral/<id>.glb       exported, Git LFS
        │  just import   (env materials from godot/assets/materials/env/)
EnvLook.apply(model, biome)                biome look as instance uniforms
```

`catalog.all_blueprints()` leaves environment blueprints out, so `just units`,
`validate.py` and `lineup.py` only see units; `just validate-assets` runs
`validate_env.py` as a second step.

### Structure: variants

One .glb per catalog entry, all variants standing at the origin:

```
env_house_farm                 root empty: catalog extras, env_kit_version, env_variants (JSON)
├── Intact                     empty, extras damage = "intact", door_height_m
│   ├── Intact_Walls           walls, timber frame, windows, doors
│   ├── Intact_Roof
│   └── Intact_Floor           plinth, ground floor, upper floor
├── Damaged  (..._Debris)      damage = "damaged": shell holes, roof holes, rubble
└── Ruin     (..._Debris)      damage = "ruin": jagged walls, no roof, partial floor, rubble
env_tree_deciduous
├── Oak                        one mesh = one instanceable variant (Bark + Foliage surfaces)
├── Oak_Far                    distant version (<= 200 triangles)
└── ...
```

- **Single-mesh variants** (vegetation, rocks, props, fences, fortifications)
  are one `MeshInstance3D` each: one mesh resource per variant, ready for
  `MultiMesh` instancing (prompt 013). One surface per material slot, so a
  tree costs two draw calls (Bark, Foliage).
- **Modular variants** (buildings, bridges) are an empty with the parts
  `<Variant>_Walls`, `_Roof`, `_Floor`, `_Debris`, `_Tower` (buildings) or
  `_Deck`, `_Pier`, `_Rail` (bridges). Each part is one mesh resource; prompt
  020 can swap or combine parts per damage level. Buildings have the damage
  levels `Intact`, `Damaged`, `Ruin` as variants already now.
- Boundaries have the variants `Segment` (runs along Y; a segment starts with
  its post/end at -Y and chains every catalog length), `End` and `Corner`.
- Trench pieces (`Straight`, `Corner`, `Tee`, `End`) sit on a 5 m grid and are
  **sunken**: z = 0 is the terrain surface, the cut reaches down to -1.7 m; the
  spoil parapets lie 2 mm above the surface. Variant extras `terrain_cut`,
  `cut_depth_m`, `cut_width_top_m`, `cut_width_bottom_m` describe the
  depression that prompt 010/021 cuts into the terrain.
- Bridges: lowest point (abutment base / water level) at z = 0, extras
  `deck_height_m`, `span_m`, `deck_width_m` for placement over rivers
  (prompt 014). The span is parametric: the generator builds to the variant
  length (`Span8`, `Span12`, `Span16`; `Arch1`..`Arch3`). In the game the
  model is lowered by `deck_height_m` so its deck lies on the simulation's
  deck (`data/objects.toml`, [MAPS.md](MAPS.md#bridges)); the deck rule's
  roadway width matches `deck_width_m`.
- Grass clumps have the variant `Reeds` (prompt 014): six tall, narrow, nearly
  upright grass cards, about 0.45 m wide and 1.7 m high, for the reed
  decoration along banks.
- Telegraph poles have `Socket_Wire_NN` empties at the insulators for wires.

### Blueprint format

```toml
unit = "env_tree_deciduous"   # catalog id = file name
assembly = "env"
sunken = false                # true for trenches (measured below z = 0)

[[variant]]                   # first variant: the catalog size exactly
name = "Oak"                  # node name in Blender and Godot
part = "tree_broadleaf"       # generator (kit/vegetation.py, parts_neutral.py, buildings.py)
seed = 1                      # ... and its parameters (documented in the docstring)

[[variant]]
name = "Beech"
part = "tree_broadleaf"
size = [0.8, 0.8, 1.1]        # box = catalog length, width, height x size
# dims_m = [L, W, H]          # or an absolute box in meters
# fit = false                 # build exactly instead of scaling into the box
# role = "winter"             # free tag, stored as extras env_role
# damage_of = "Intact"        # damaged variant: checked not larger than Intact
# center = "ground"           # center the ground contact (default for trees, bushes)
```

Generators build into the target box; by default `fit` then scales the finished
variant exactly into it (vertex transform, object scale stays 1). Unknown
parameters are errors.

### Materials and shaders

No texture files: all patterns are procedural, so texture budgets are zero and
biome variants cost nothing.

- `godot/assets/shaders/env.gdshader` (opaque) with one material per slot in
  `godot/assets/materials/env/`, and `env_foliage.gdshader` (alpha scissor,
  double sided) for `Foliage`. Shared code: `env_common.gdshaderinc`.
- Vertex colors (`Color`, COLOR_0): R = ambient occlusion; G = wind weight on
  `Bark`/`Foliage`, edge wear elsewhere; B = wind phase on `Bark`/`Foliage`,
  height / 4 m elsewhere; A = material kind, `A = 1 - kind / 4`.
- Kinds per slot (kind 0 first): Bark oak / birch / palm / pine; Stone
  fieldstone / ashlar / concrete / brick; Plaster lime / adobe / whitewash /
  ochre; Roof clay tile / slate / thatch (also hay) / planks (prompt 075:
  Roof patina below); Wood weathered /
  fresh / creosote / charred; Paint field grey (`env_paint`) / warning red /
  white / rust; Metal steel / rust / galvanized / dark; Ground earth / spoil /
  mud / sand; Rock bedrock / scree (prompt 074).
- **Foliage cards** use a virtual atlas: the integer part of U selects the
  pattern (0 broadleaf, 1 needles, 2 palm frond, 3 grass, 4 flowers, 5 shrub,
  6 twigs, 7 solid hedge core, 8 far crown, 9 far conifer), the fraction is the
  position on the card. Card normals point away from the crown center for soft
  crown lighting.
- **Biomes**: `EnvLook.apply(model, biome, paint)` sets the instance uniforms
  `env_foliage`, `env_dry`, `env_snow`, `env_dust`, `env_paint`: green
  forest, dry olive desert with dust, winter with snow on upward faces and on
  needles. The leafless `Bare` oak is the winter variant of deciduous trees.
  Military objects use `Paint`; pass a faction palette base color as `paint`.
- **Wind** is set as material parameters `wind_strength`,
  `wind_direction`, `wind_speed`, `tree_sway`, `leaf_flutter` on the shared
  `bark` and `foliage` materials from the biome preset (prompt 015, 086).
  `wind_offset` (`env_common.gdshaderinc`, prompt 086) has three parts:
  the whole tree bends downwind with the height above its foot ((h / 20 m)²,
  from `VERTEX.y`, so no model needs new data and the far and shadow meshes
  move the same way), swinging slowly about a slight lean and pushed by the
  gust wave shared with the grass (`wind_common.gdshaderinc`); branches
  swing faster by the baked weight (G) and phase (B); leaves flutter
  (foliage only). Bark and foliage call it with the same arguments, so the
  crown stays on the trunk. The whole-tree bend is opt-in per kind
  (`env_bend`, `EnvLook.BEND`: oak 1.0, conifer 0.6, birch 1.35, palm 1.15;
  0 for everything else, so log houses, fences, bushes and hedges keep only
  the branch sway). Phase and tempo differ per tree (position, scale).
  Lying trees (`env_fallen`) are still. Thumbnails and the model gallery
  call `EnvLook.still_trees()` (wind 0), so their pictures stay the same.
- In the game the biome comes from the map (`map.toml`, prompt 015): unit
  palettes and `EnvLook` follow the preset's `camo_biome` and `env_biome`.
- **Charred** (prompt 041): the instance uniform `env_char` (0..1) blackens
  any environment mesh (`charred()` in `env_common.gdshaderinc`); the fire
  presentation sets it on objects that burned. No separate charred models,
  except the `Burnt` remains of the dumps.
- **Town and winter buildings** (prompt 041) are blueprints of the `house`
  generator only: `env_house_town` (three storeys, eaves to the street,
  flush gables, so several stand in a row), `env_town_hall` (ashlar),
  `env_warehouse` (brick, loading doors), `env_house_log` (log walls, plank
  roof). Snow on winter maps comes from the biome look, not from extra
  variants.

- **Desert village** (prompt 059): ten house types from the generator
  `desert_house` (`kit/desert.py`, data-driven room blocks with flat roofs
  and parapets, roof rooms, outside stairs, barrel vaults, arcade,
  balcony; palm-trunk beam ends, drain spouts, shutters and grilles
  instead of glass, spalled plaster showing brick, sand drifts in
  `_Floor` with Ground kind 3), all with the door / courtyard side on +X:
  `env_house_desert_hut`, `_two`, `_long`, `_corner` (L), `_stair`,
  `_roofroom`, `_tall`, `_vault`, `_large`, `_shop`. `kit/desert_props.py`:
  the courtyard wall `env_wall_mud` (`Segment`, `Low`, `Broken`, `Gate`,
  `End`, `Corner`), the props `env_awning` (`Cloth`, `Wide`, `Palm`
  frond shelter), `env_jars`, `env_well_desert`, `env_cart_wood`,
  `env_market_stall`, and the landmarks `env_mosque` (whitewashed hall,
  dome, square minaret as `_Tower`; no writing, symbols or crescent) and
  `env_police_post` (colonial post with crenellations and watchtower; no
  insignia or flags). Mosque and police post have the church's budget
  (5 000). Nothing modern. Compounds of these objects:
  [MAPS.md](MAPS.md#desert-village-prompt-059).
- **Apartment blocks** (prompt 078): generator `apartment_block`
  (`kit/blocks.py`), see "Apartment blocks" below.
- **AO bake per variant** (prompt 059): `bake_env` bakes each variant (a
  child of the root with its parts) on its own; before, all variants
  standing at the origin shaded each other (inner variants came out
  darker). Models built before keep their old bake until rebuilt.

### Apartment blocks (prompt 078)

`env_block_desert` (four storeys, flat roof, one entrance),
`env_block_desert_long` (five storeys, two entrances, balconies; the
block of the reference pictures) and `env_block_town` (tenement: stone
plinth, five storeys, string course, cornice, saddle roof with blind
fire-wall gables, carriage passage), from the generator `apartment_block`
(`kit/blocks.py`, parameters in its docstring). Street side with the
entrances +X, courtyard side with the balconies −X; the box height is the
highest point (stair heads of flat roofs, chimney tops of saddle roofs).

- **Why a new generator:** `house` builds each wall piece as a closed box
  and each window from four boxes (about 80 triangles), and has no
  interiors; `desert_house` is a room-block plan with palm beams and
  shutters. `apartment_block` subclasses the shared `House` base (parts,
  `finish`).
- **Facade from single faces:** a strict grid of bays per side; the outer
  wall face is cut into piers and spandrels (one quad each, equal
  neighbours merged, split once at 1.5 m so the per-vertex AO keeps the
  ground shade at the foot); a window is four reveal quads, one recessed
  sash quad and a two-quad sill, about 15 triangles. An intact wall has
  one inner quad. Only walls cut by damage are built from closed boxes.
- **Sash windows** are one `Glass` quad whose UV is `102 + u, v` (v = 0 at
  the top): `env.gdshader` (`sash_window`) draws the white (some green or
  brown) frame, mullion, transom and glazing bars anti-aliased, dark panes
  with a faint sheen and curtains in some windows. About 12 % of the
  windows have closed shutters (`Wood`).
- **Stains:** `Plaster` cards 1 cm in front of the wall with UV `100 + u`
  (dirt streak under a sill, 60 % of the windows) or `101 + u` (soot
  plume over a burnt-out window). The card repeats the wall's plaster
  pattern (same slot, kind and position), so only the stain shows; 2
  triangles each (`wall_stain`).
- **Roofs:** flat roofs are `Plaster` slabs (top face in 2.5 m cells for
  the AO bake) and get the flat-roof patina of prompt 075; parapet with a
  concrete coping, stair heads over the entrances, black water tanks on
  concrete sleepers, chimneys at the parapets, metal vents, all placed by
  the design seed. The tenement's roof is clay tile (`Roof`) with
  chimneys on the ridge.
- **Damage levels:** `Damaged` tears one corner open over one or two
  storeys: the two walls get a jagged top falling towards the corner, the
  roof slab and the upper floor slabs are bitten off (a broken slab hangs
  into the room), the corner rooms show their floor, the partitions that
  close them and one between them, rubble on the floor and at the foot;
  6-10 windows are burnt out (dark room, soot plume), three blasted into
  larger holes, three shell holes in the piers. `Ruin`: one to two
  storeys of wall stumps whose top steps at the bays (so the window holes
  stay readable) with notches and higher corners, the ground floor, slab
  remnants along the walls and a leaning slab, partition stumps, rubble
  heaps of about 15 % of the height inside and along the walls (spoil,
  fieldstone, brick, plaster). Nothing outside the intact box.
- **Biomes:** `EnvLook` as for every house: the desert dusts the ochre
  plaster, forest and winter fit the tenement; snow lies on roofs,
  parapets, copings, sills and balconies.

### Roof patina (prompt 075)

`godot/assets/shaders/env_roof.gdshaderinc`, included by `env.gdshader`.
Roofs look used: weathered, stained, patched, overgrown. Everything depends
only on the position on the model and the per-house seed `v_seed`, so the
same house always looks the same and nothing wanders with the camera.

- **Roof frame:** `roof_frame()` derives from the model position and normal
  the plane coordinates (`uv.x` along the ridge, `uv.y` down the slope, in
  meters), the distance from the ridge (houses are built with the ridge
  above the model origin, `kit/buildings.py gable_roof`), north / west
  facing, and the pixel footprint `fw`. Stage 1 of the prompt: the models
  carry no ridge, eave or chimney data. The roof snow of prompt 080 uses it.
- **Age per house** (`roof_age`): about 12 % new, two thirds weathered, a
  fifth very old; scales every other layer.
- **Shared layers** (`roof_weather`, `roof_growth`, `roof_desert`): large
  stains (darkened, bleached, damp-green), dark streaks down the slope
  (denser under the ridge), a darker, dirtier eave and a washed-out ridge,
  moss in the joints and at the eave (more on the north side and on old
  roofs), pale lichen dots; in the desert no growth, bleached, sand in the
  joints.
- **Per covering:** clay tile (courses of 17 cm, tiles 21 cm in a half
  bond, wavy rows, slipped tiles, strong tone per tile, groups of repaired
  tiles in fresher or foreign tone, five base colors from bright brick red
  to dark brown and weathered grey brown); slate and the dark cement tiles
  (grey plates, lime and lichen patches, replaced bluish plates); thatch
  (barns since prompt 075, haystacks, hay bales: courses of 35 cm with
  frayed edges, fibres, frayed dark patches, moss cushions, fresh straw to
  silver grey with age); planks (boards down the slope, seams, butt
  joints, newer boards, green on the weather side); corrugated `Metal`
  kinds rust and galvanized (`sheet_patina`: sheets 0.9 x 2.4 m, matte and
  bright sheets, rust at the seams and in runs below the fastener rows; the
  depot roof is galvanized since prompt 075); upward faces of `Plaster`
  (`flat_roof_patina`, flat desert roofs: grime rims and dust from the
  baked occlusion, water stains, fine cracks; wall colors unchanged).
- **Colors** are composed in sRGB and converted to linear albedo at the end
  (`roof_linear`); the old linear values read pale salmon. Judgement values
  for rural Central and Eastern Europe around 1940, [provisional].
- **No flicker:** tile rows and joints are box-filtered (`roof_joint`,
  exact for any pixel footprint) and fade to their mean below about 2.5
  pixels per row; per-tile tones, lichen dots, fibres and cracks fade by
  `roof_fine(fw, size)` before they reach 2 pixels. Stains, streaks, age
  and the eave gradient carry the far view.
- **Order:** patina, then edge wear, dust, roof snow (prompt 080, below)
  and soot; damaged roofs and roof debris get the patina of their house
  (same seed).
- **Checks:** `just roof-shots [dir] [map]` (village at 15-350 m and like
  the reference, all coverings side by side, twenty equal houses, damaged
  and charred houses; forest by day, low sun and night, desert, winter),
  `just ground-motion --roofs --png` (flicker on roof windows of
  `town_small`), `just objects-bench` (line "village").

### Roof snow (prompt 080)

`godot/assets/shaders/env_snow.gdshaderinc`, included by `env.gdshader`.
On winter maps the snow on roofs is a blanket with a clear edge instead of
a color blend: it stays along the ridge and the eave and has slid off in
between. It replaces `snow_cover()` on roof pixels only: the `Roof` slot
(clay, slate and cement tiles, thatch, boards; also haystacks and bales),
the sloped faces of the corrugated `Metal` kinds and the flat roofs
(upward `Plaster` faces). Everything else (walls, sills, chimney tops,
fences, rocks, trees) keeps `snow_cover()`, so a chimney gets its cap from
its top face. Shader only: no snow geometry, the models are unchanged.

- **Blanket or nothing:** `roof_snow()` builds a signed distance `d` (m,
  > 0 in the snow) and thresholds it, smoothed over 1.2 pixel footprints
  (`RoofFrame.fw`), so the edge is sharp near and never flickers far.
- **Where it lies:** `u` runs from the ridge (0, the model origin) to the
  eave (1); the eave distance comes from the mesh bounds, set per mesh as
  the instance uniform `env_roof_box` (`EnvLook.set_roof_box()`, called by
  `EnvLook.apply()` and `map_objects._apply_look()`). Layers: a ridge band
  whose lower edge is jagged (a 2 m wave, tongues hanging down, 20-40 cm
  bites), a broad eave band with a wavy upper edge, lanes where nothing
  slid (ridge and eave joined), floes on the free roof, a filled middle
  with a lot of snow, holes when the snow is thin.
- **How much stays:** `c = hold x pitch x amount / 0.85 x sun x lee x
  house`: covering (thatch 1.35 with soft edges, sheet 0.7 with a narrow
  ridge band and a long free middle, others 1.0), pitch (`n.y` 0.2-0.72:
  steep spires and towers stay almost bare), sunny side (towards the
  winter sun, south-east) up to 22 % less, lee side (along the wind of the
  biome) up to 15 % more, per house and roof side 0.78-1.22 from `v_seed`.
  `env_snow` 0.3 gives a thin, holey blanket, 0.85 (winter) the reference
  look, 1.0 a nearly closed roof.
- **Thickness, faked:** the outward edge faces of the roof slabs (eave,
  verge, ridge cap) turn white under snow and read as the blanket's front;
  only on buildings (mesh bounds over 4.8 m), so bales keep bare sides. A
  dark line (up to 65 %) runs under the blanket's edge on the free roof;
  the normal bends over at the lower edge of the ridge band (downhill) and
  the upper edge of the eave band (uphill).
- **Color:** `snow_cover()`'s white a touch lower (0.86 / 0.88 / 0.92
  linear) so noon sun does not burn out, a faint grain, warmer on the
  sunny side, bluish on the shady side; roughness 0.8, no metal.
- **Flat roofs:** a closed blanket with holes only when the snow is thin.
- **Fire and damage:** no roof snow where `env_char` is above about 0.3;
  holes of damaged roofs have no slab, so no snow; ruins keep only their
  rafters (Wood, `snow_cover()`).
- **Cost:** only on roof pixels and only with `env_snow > 0`; the shadow
  pass is untouched. Checks: `just snow-shots`, `just objects-bench
  winter_village --roofs-bench`, `just ground-motion --snow-roofs`.

### Grown rock (prompt 074)

`kit/rocks.py`, slot `Rock` (env.gdshader slot 16). The building block is
`rock_block`: the convex hull of jittered points in a box (the four foot
corners exactly on the cell corners, three rings tapering upwards, a few top
points), its upper corners and flanks chopped by random fracture planes
that never reach the foot, near-coplanar hull triangles merged, sharp edges
bevelled (edge wear in G), no bottom cap. Blocks stand on rectangular
cells, so a group covers its blocking box at the ground and leaves dark
joints (AO bake) between the blocks. All from the variant seed.

| Generator | Catalog id | Variants | Build |
|---|---|---|---|
| `rock` | `env_rock` | Medium, Small (deco stone, no bevel), Large | one block |
| `rock_slab` | `env_rock_slab` | Field, Long, Broken, Small, Wide, Strip | roundish outline cut into Voronoi slabs with joints, rim pieces sometimes left out, each slab tilted, chamfered, some lower; at most 0.3 m above ground |
| `rock_outcrop` | `env_rock_outcrop` | Group, Block, Split, Pair, Leaning, Layered | blocks on the cells of a layout (single, split, pair, leaning, group of four), `layers` stacks bedding layers; chips at the foot |
| `rock_massif` | `env_rock_massif` | Ridge, Long, Short, High, Double | `cols` blocks along the length in one or two rows, high in the middle, low at the ends (pieces overlap into ridges), a lower flank block beside, stacked blocks on top; `<Variant>_Far` (same cells, no bevel, fewer points) |

**Embedded foot:** the generators record `env_embed_m` (extras); `fit`
then scales only the part above z = 0 to the box height and keeps the foot
at -embed, and `validate_env.py` measures the height above z = 0 and
expects the lowest point at -embed. In the game the foot sinks into slopes
(placement: [ARCHITECTURE.md](ARCHITECTURE.md#grown-rock-prompt-074)).

**Shading:** `rock_albedo()` in `env.gdshader`, in world space (neighbouring
blocks and overlapping pieces continue the pattern): base color of the
terrain's rock layer (`gen_textures.rs` `rock`) times the biome's rock tint
(`layer_tint[8]`), recognized from the look uniforms; tops brighter and
warmer than the flanks by the normal, bedding bands, two crack noises,
dark water streaks down the flanks, lichen and moss on top (not in the
desert), pale lichen spots, a darker foot (height in B); half the usual
desert dust; snow from the shared snow cover. [provisional] colors.

Sizes from the reference pictures' scale (houses about 8 × 5 m, telegraph
poles about 7 m, trucks, soldiers), uncertain by about ±30 %: slabs 2–8 m
wide, outcrops 3.6–7.8 m and 2.5–4.8 m high, ridge pieces 12–25 m and
5–9 m high. `just env-lineup build/renders/env env_rock_slab
env_rock_outcrop env_rock_massif` shows them with the figure and the tank.

### Distance: far meshes instead of impostors

Trees have a `<Variant>_Far` mesh: low trunk plus three crossed and one or two
horizontal crown cards (18–26 triangles) with a far crown pattern.
`EnvLook.set_far_ranges(model, 180)` switches near and far by Godot visibility
ranges with a fade. Reason: Godot's automatic mesh LOD cannot simplify alpha
cards well (it removes whole cards and the crown falls apart), and baked
impostor textures would need texture files and an atlas per biome. A crossed-
card far mesh keeps the biome tint and wind of the shader. At 600 m (the
farthest camera) a 25 m tree still covers about 80 px, so the far mesh is not
needed below ~150–200 m; prompt 016 measures the switch distance.

Prompt 013 places the objects on the maps as MultiMesh instances and switches at
220 m per 128 m chunk. Near trees cast their shadow through a shadows-only copy of
the far mesh, and `env_foliage.gdshader` draws leaf and needle cells with a cheap
one-hash pattern in directional shadow passes and beyond `detail_distance` (70 m):
the 3 × 3 leaf-cell search over a whole wood in four shadow cascades cost most of
the frame ([ARCHITECTURE.md](ARCHITECTURE.md#map-objects-and-decoration-presentation-only)).

### Budgets (per variant)

| Category | Triangles | Note |
|---|---|---|
| Tree | ≤ 2 000 (far ≤ 200) | per variant, not per file |
| Bush / grass clump / rock | ≤ 500 / 50 / 300 | |
| Rock slab / outcrop / ridge piece (prompt 074) | ≤ 300 / 1 500 / 3 000 (ridge far ≤ 300) | `BUDGET_OVERRIDES`, `FAR_BUDGET_OVERRIDES` in `kit/env.py` |
| House / church, mosque, police post | ≤ 3 000 / 5 000 | per damage variant, all parts together |
| Apartment block (prompt 078) | ≤ 6 000 | per damage variant; 100-170 windows of about 15 triangles plus roof furniture and, when damaged, interiors; built: 2 600-4 800 |
| Bridge | ≤ 3 000 | no budget in the table above; same as a house |
| Fence, wall, hedge segment; fortification | ≤ 2 000 | |
| Prop, pole, mine, sign | ≤ 800 | like small parts |
| Supply dump (`env_dump_ammo`, `env_dump_fuel`) | ≤ 1 500 | many crates or drums (prompt 041) |

### Checks and previews

| Command | Does |
|---|---|
| `just env [<id> ...]` | builds environment objects (.blend + .glb) |
| `just validate-env [<id> ...]` | checks them (also part of `just validate-assets`) |
| `just env-lineup [out] [<id> ...]` | per object all variants with the 1.80 m figure; a front and perspective line-up with the Panzer IV; prints measured sizes and door heights |
| `just gallery` | category "Environment", biome switch |

`validate_env.py` checks per object: files exist and are current (source
hash), root and variant structure, part names, far meshes of trees, scale 1 and
no rotation, only env slots, baked `Color`, triangle budget per variant, size
of every variant within ±2 % of its box (damaged variants not larger than the
intact one), lowest point on the ground (sunken: at -depth), ground contact
centered (not for chained segments), the .glb matches the .blend, catalog
`model` field, and that every neutral catalog entry has a blueprint.

## Audio (prompt 029)

```
tools/audio/synth.py                  recipes: noise, partials, envelopes, filters (numpy)
assets-src/audio/<class>/*.wav|ogg|mp3|flac   recordings or generated sounds of other origin (optional)
        │  just audio [classes]   (normalize, trim, fade, mono 44.1 kHz)
godot/assets/audio/<class>/*.wav      game files (Git LFS), imported by `just import`
godot/assets/audio/sounds.json        manifest: class -> group, bus, level, range, priority
```

- **Classes, not units.** A sound class (`shot_cannon_medium`,
  `hit_ricochet`, `engine_track_heavy`, `ui_order` …) is a folder of
  variants; the game plays a random one with a little pitch spread. Units
  get their class from the catalog (category, caliber, rate of fire, mass;
  `scenes/audio/sound_classes.gd`), never one by one.
- **Format.** 16-bit PCM WAV, mono, 44.1 kHz, peak at −1 dBFS; Godot
  imports them QOA-compressed. Loops (`"loop": true`) are built periodic
  (whole cycles, noise shaped over the whole loop) and carry a `smpl` loop
  chunk that Godot reads. [provisional] OGG Vorbis was planned for long
  loops, but the only encoder on the machine (ffmpeg's built-in `vorbis`)
  is experimental, stereo-only and not reliably gapless; our loops are 4–6
  s and small as WAV. Music (prompt 030) may need OGG and libvorbis.
- **One-shots** start at their first sample (no silence before the
  attack: timing in a battle), end with a 30 ms fade, trailing silence
  below −60 dB is cut.
- **Levels** are set in the manifest (`volume_db`), not baked into files,
  so all files stay normalized and a class can be balanced without
  regenerating.
- **Other origins.** Files in `assets-src/audio/<class>/` replace the
  synthesized variants of that class on the next `just audio` and take the
  same steps (prepared as `ext_<name>.wav`; formats other than 16-bit 44.1
  kHz WAV need ffmpeg). Each needs an entry in
  [SOURCES.md](SOURCES.md#sounds): CC0 link, or for AI-generated sounds the
  tool, prompt and a license that allows commercial use. No sounds from
  other games.
- **Private sounds** (prompt 069). Files that must not be published as single files
  live in the private repository `mud-and-steel-sfx` (same layout as
  `assets-src/audio/`), cloned next to this one or named by `MUD_AND_STEEL_SFX`.
  `just audio` prepares them as `priv_<name>.wav` beside the public files; a folder
  with `priv_` files plays only those (`sound_library.gd`). They and their import
  files are gitignored. Without the private repository nothing changes.
- **New class:** add it to `sounds.json` (group, bus, levels, `variants`),
  write a recipe in `synth.py` or put files into `assets-src/audio/<class>/`,
  run `just audio <class>` and `just import`, and map it in
  `sound_classes.gd` or `battle_sound.gd`. Add a line to
  `web/sounds/usage.toml` saying when the game plays it (`just sounds-web --check`
  fails without; the same for a new weapon folder, ambience layer or music mood).
  `just sound-gallery` plays it; the self-test checks that every class has a
  loadable file.

### Weapon recordings and battle voices (prompt 053)

```
assets-src/audio/weapon/<slug>/*.wav|ogg|mp3|flac              recordings of one weapon type
assets-src/audio/voice/<faction>/<callout>/*.wav|ogg|mp3|flac  battle voices
        │  just audio   (or: just audio weapon / just audio voice)
godot/assets/audio/weapon/<slug>/ext_*.wav
godot/assets/audio/voice/<faction>/<callout>/ext_*.wav
```

- **Weapons.** The slug is the catalog weapon name without parentheses,
  lower case, every run of other characters one `_`: `MG 42` → `mg_42`,
  `Kar98k` → `kar98k`, `Browning M2HB (AA)` → `browning_m2hb`.
  `python3 tools/audio/synth.py --weapons` lists every slug of the
  catalog with its file count. A weapon with files plays them at the
  levels, ranges and pitch spread of its class (`shot_mg`,
  `shot_cannon_large` …); without files it keeps the synthesized class.
  One-shots, dry or with a short natural tail: the game adds distance,
  delay and reverb. Several takes per weapon are welcome (random pick).
- **Voices.** Factions `de`, `su`, `us` (prefix of the unit type ids);
  callouts `contact`, `take_cover`, `reloading`, `wounded`. Short (0.4–1.5
  s), dry, mono, one line per file, 2–4 takes and speakers per callout.
  Recorded or provided by the user; nothing ships yet. Tactical calls in
  the faction's language only: no slogans, no ideology, no names of real
  people or units. [provisional] Suggested lines:

  | Callout | de | su | us |
  |---|---|---|---|
  | `contact` | „Feind!“, „Feind gesichtet!“ | „Противник!“ (Protivnik!) | "Contact!", "Enemy spotted!" |
  | `take_cover` | „Deckung!“, „Runter!“ | „Ложись!“ (Lozhis'!) | "Take cover!", "Get down!" |
  | `reloading` | „Lade nach!“ | „Перезаряжаю!“ (Perezaryazhayu!) | "Reloading!" |
  | `wounded` | „Sanitäter!“, „Getroffen!“ | „Санитар!“ (Sanitar!) | "Medic!", "I'm hit!" |

- Every file needs a row in [SOURCES.md](SOURCES.md#sounds) before it is
  committed (own recording: speaker's consent and date).

### Music and ambience (prompt 030)

```
tools/audio/music.py                          composition, instruments, ambience recipes (numpy)
assets-src/music/<mood>_NN.wav|mp3|ogg|flac   pieces of other origin (optional, replace a composed piece)
        │  just music [names]   (then just import)
godot/assets/audio/music/<mood>_NN.mp3        pieces, moods: menu, calm, combat, victory, defeat
godot/assets/audio/ambience/<layer>/*.wav     beds (loops) and one-shots (birds, rumble)
```

- **Music** is stereo MP3 (LAME VBR `-V 5`, about 130 kbit/s, 44.1 kHz),
  1–2.5 min per piece, each mixed to the same loudness (RMS −21 dBFS, peak
  −1 dBFS) with a fade at the end; pieces do not loop, the game
  cross-fades them. Godot keeps only the compressed file in memory and
  decodes while playing. A new piece: a file `<mood>_NN` with a new number
  is enough; `music.gd` finds pieces by the name before the last `_`.
- **Ambience beds** are periodic WAV loops of 20 s with a `smpl` chunk:
  wind stereo 22.05 kHz, insects and rain stereo 32 kHz, leaves and water
  mono 32 kHz (played at a position). Godot imports them QOA-compressed.
  **One-shots** (birds, rumble) are mono 44.1 kHz, trimmed, faded.
- **Levels** are set in code (`music.gd` `MUSIC_DB`, `ambience.gd` per
  layer), the files stay normalized.

## Thumbnails (prompt 058)

Pictures of everything a player can build or produce, for the picker tiles
and the unit panel: `godot/assets/thumbnails/<catalog id>.png` (Git LFS,
with their `.import` files), 240 × 160 pixels, transparent background (the
HUD draws a faction-tinted backdrop behind them).

- **Which:** every id of `SimHost.buildable_ids()`, i.e. base buildings,
  fortifications and production lists of `data/economy.toml` for all
  factions. A new buildable needs a model and then `just thumbnails`.
- **How:** `just thumbnails` (`godot/scenes/tools/thumbnail_render.gd`, needs a
  window, Godot cannot render headless) builds each model like the match
  does (forest camouflage and markings, borrowed houses for base buildings,
  squads with their soldier look), renders it orthographically from the
  front left, 30 degrees above, with a fixed key and fill light, three times
  larger, then crops to the model's pixels and scales it into the picture
  with a 5 % margin, standing on the lower edge. Squads show their first
  five soldiers in a fixed tight group (real formations are too wide for a
  tile). `just thumbnails --only=<id>,<id>` renders just these; `just
  import` runs afterwards. [provisional] angle, light and size.
- **Check:** `just gallery-check` fails if a buildable id has no thumbnail
  or no catalog card.

## Markings

National markings and numbers are decals at the `Socket_Marking_*` empties, never
painted into the base texture: Balkenkreuz (`de`), red star (`su`), white star (`us`).
Never swastikas or SS runes; medics carry no Red Cross. Details in
[SOURCES.md](SOURCES.md).

## Parallel work

Several sessions build models at the same time (prompts 006–008):

- Each session writes only its own files: `assets-src/blueprints/<faction>/<id>.toml`
  (and `squads/<id>.toml`),
  `assets-src/blender/<faction>/<id>.blend`, `godot/assets/models/<faction>/<id>.glb`
  and the `model` field of its own catalog entries.
- Shared building blocks (part library, materials, scripts in `tools/blender/`) are
  changed additively only: new parts and functions, no renames or behavior changes.
  Changes that break existing parts go through a separate, sequential session.
- The merging session regenerates `docs/UNIT_CATALOG.md` (`just catalog-doc`).
