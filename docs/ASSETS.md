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
  (next section); the .blend is an output, not edited by hand. Hand-made models
  (infantry, prompt 005; environment objects, prompt 007) are exported with
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
| `running_gear` | tracks around wheels; `leaf_spring_bogie` (Panzer IV), `christie` (T-34), `vvss_bogie` (Sherman), `halftrack_rear` |
| `wheels` | road wheels, idlers, sprockets, rollers, tire wheels, `wheel_axle` (trucks, cars, carriages) |
| `hull` | `plate_hull` (lower hull + superstructure from side profiles, sloped side plates, fenders, optional cast look), `deck_box`, `hull_hatch` |
| `turret` | `plate_turret` (plan + side slopes), `cast_turret` (superellipse rings), `cupola`, `turret_hatch`, `turret_bustle_box` |
| `weapons` | `tank_gun` (mantlet types, barrel segments, muzzle brake types), `mg` (coax, ball mount, pintle), `gun_shield` |
| `attachments` | `skirts`, `turret_skirt`, `spare_tracks`, `tool`, `jerrycans`, `headlight`, `antenna`, `exhaust`, `tow_hook`, `box_prop`, `cylinder_prop`, `marking`, `socket` |
| `artillery` | `split_trail_carriage`, `field_gun` |
| `aircraft` | `fuselage` (loft of cross-sections), `wing` (taper, sweep, dihedral, ailerons), `tail`, `propeller`, `canopy`, `landing_gear` (extended/retracted) |
| `parts_de`, `parts_su`, `parts_us` | faction-specific parts, prefixed `de.`, `su.`, `us.` |
| `assemblies` | whole units: `tank`, `towed_gun`, `aircraft` |
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
assembly = "tank"          # tank | towed_gun | aircraft

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

### Generator, checks and previews

| Command | Does |
|---|---|
| `just unit <faction> <id>` | builds one unit: `.blend`, `.glb` and `build/renders/<id>_turntable.png` (4 views) |
| `just units` | builds every unit with a blueprint |
| `just validate-assets` | checks every built model (below); exit code 1 on failure |
| `just lineup` | renders all models in a row with the 1.80 m figure: `build/renders/lineup.png` |
| `just kit-test` | smoke test of all parts |
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
now; prompt 016 (performance) measures this together with draw calls.

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

## Polygon and texture budgets (LOD0)

The camera (prompt 002) looks down from 15 m to 600 m with a narrow field of view.
At medium zoom a 7 m tank covers roughly 60–100 px on a 1080p screen, and the scene
must hold hundreds of units plus dense vegetation. More geometry than below is
invisible in play and costs frame time. Lower LODs come from Godot's automatic mesh
LOD on import; own LOD meshes only if that proves insufficient (prompt 004).

| Category | Triangles LOD0 | Textures |
|---|---|---|
| Soldier incl. equipment | ≤ 3 000 | 1024² uniform atlas + 512² equipment |
| Small arm, small part | ≤ 800 | shared weapon atlas 1024² |
| Truck, recon vehicle | ≤ 5 000 | 1024² |
| Tank, halftrack, SPG | ≤ 8 000 | 2048² `Paint`, 1024² others |
| Towed gun | ≤ 3 000 | 1024² |
| Fighter, attack aircraft, recon aircraft | ≤ 6 000 | 2048² |
| Bomber | ≤ 10 000 | 2048² |
| Fortification element | 500–2 000 | tileable trim sheets 1024² |
| Tree / bush / grass clump / rock | ≤ 2 000 / 500 / 50 / 300 | atlas 1024² with alpha |
| House / church | ≤ 3 000 / 5 000 | trim sheet 2048² per biome |

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

Environment objects may additionally use `Bark`, `Foliage`, `Stone`, `Plaster` and
`Ground`. New slots are added here first.

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

Soldiers follow their own layout (prompt 005), described in [INFANTRY.md](INFANTRY.md):
one shared humanoid skeleton (Godot `SkeletonProfileHumanoid` bone names), body and
gear as skinned meshes, the weapon on the extra bone `Weapon` with `Socket_Muzzle`.
Files: `assets-src/blender/<faction>/infantry/<role>.blend` and
`godot/assets/models/<faction>/infantry/<role>.glb`, built by
`tools/blender/build_soldier.py` and checked by `tools/blender/validate_soldier.py`.

## Markings

National markings and numbers are decals at the `Socket_Marking_*` empties, never
painted into the base texture: Balkenkreuz (`de`), red star (`su`), white star (`us`).
Never swastikas or SS runes; medics carry no Red Cross. Details in
[SOURCES.md](SOURCES.md).

## Parallel work

Several sessions build models at the same time (prompts 006–008):

- Each session writes only its own files: `assets-src/blueprints/<faction>/<id>.toml`,
  `assets-src/blender/<faction>/<id>.blend`, `godot/assets/models/<faction>/<id>.glb`
  and the `model` field of its own catalog entries.
- Shared building blocks (part library, materials, scripts in `tools/blender/`) are
  changed additively only: new parts and functions, no renames or behavior changes.
  Changes that break existing parts go through a separate, sequential session.
- The merging session regenerates `docs/UNIT_CATALOG.md` (`just catalog-doc`).
