# Asset conventions

Binding rules for every model in the game. All Blender prompts (004–008) build on
this document. Sizes and other real values come from the unit catalog
([UNIT_CATALOG.md](UNIT_CATALOG.md)); sources and legal rules are in
[SOURCES.md](SOURCES.md).

## Pipeline

```
assets-src/blueprints/<faction>/<id>.toml   build plan (parts, dimensions from the catalog)
        │  Blender (tools/blender/…, prompt 004)
assets-src/blender/<faction>/<id>.blend     source file (Git LFS)
        │  just export-model  (tools/export_blender.py)
godot/assets/models/<faction>/<id>.glb      exported model (Git LFS)
```

- `<faction>` is `de`, `su`, `us` or `neutral`; `<id>` is the catalog id
  (`de_pz4_h`, `env_tree_birch`). One model per catalog entry; camouflage variants
  come from the material system, not from separate models.
- When a model is exported, its catalog entry gets `model = "assets/models/<faction>/<id>.glb"`.
- Export: `blender -b <file>.blend --python tools/export_blender.py -- --output <glb>`.
  Without `--output` the script writes `godot/assets/models/<name>.glb` (flat); the
  blueprint tooling of prompt 004 passes the faction folder.
- Binary files (`.blend`, `.glb`, textures) are tracked by Git LFS (`.gitattributes`).
- `tools/create_placeholder_tank.py` generates the current placeholder tank from
  primitives. `tools/blender/scale_reference.py` builds the 1.80 m reference figure and
  scale bar (`assets-src/blender/reference/scale_reference.blend`) for line-up renders.

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
| Machine gun on mount | `MG_01`, … | `Turret` or `Hull` | mount pivot |
| Hatch | `Hatch_01`, … | parent part | hinge |
| Propeller | `Propeller`, `Propeller_L`/`_R` | root | hub, rotates around Y |
| Control surfaces | `Aileron_L`, `Elevator`, `Rudder` | root | hinge |
| Landing gear | `Gear_L`, `Gear_R`, `Gear_Tail` | root | – |

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
| `Socket_Marking_Turret_L` … | decal positions for national and tactical markings |

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

Every model uses only these material names, so the camouflage system of prompt 004
can swap them per biome (temperate/forest, desert, winter) and faction:

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
