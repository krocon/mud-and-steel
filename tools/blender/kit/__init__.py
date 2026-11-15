"""Blender part kit: builds unit models from blueprints (prompt 004).

Runs only inside Blender (`blender -b`). Entry points are
`tools/blender/build_unit.py`, `tools/blender/validate.py` and
`tools/blender/render.py`; see docs/ASSETS.md ("Part kit").

Module overview:
- `core`        mesh builder, objects, sockets, material slots, finalize
- `bake`        vertex-color bake: AO, edge wear, height, track flag
- `catalog`     reads data/catalog/*.toml and blueprints
- `registry`    part and assembly registry (`@part`, `@assembly`)
- `running_gear`, `wheels`, `hull`, `turret`, `weapons`, `attachments`,
  `artillery`, `aircraft`, `vehicles`, `trucks`   generic parts
- `casemate`, `armored_car`, `antiair`, `mortars`   prompt 008: casemate
  vehicles, armored cars, anti-aircraft guns, heavy mortars (parts and
  assemblies); `forts` (faction fortifications) is an env module loaded by
  build_env.py
- `parts_de`, `parts_su`, `parts_us`   faction-specific parts; prompt 008 adds
  `parts_<faction>_{vehicles,guns,air}` (one per parallel session)
- `assemblies`  whole-unit builders (`tank`, `towed_gun`, ...)
"""

import importlib

# Importing the part modules registers their parts. Faction modules are
# imported last so that they can build on the generic parts.
_MODULES = (
    "running_gear",
    "wheels",
    "hull",
    "turret",
    "weapons",
    "attachments",
    "artillery",
    "aircraft",
    "vehicles",
    "trucks",
    "casemate",
    "armored_car",
    "antiair",
    "mortars",
    "parts_de",
    "parts_su",
    "parts_us",
    # prompt 008: one faction module per parallel session (vehicles, guns, air)
    "parts_de_vehicles", "parts_de_guns", "parts_de_air",
    "parts_su_vehicles", "parts_su_guns", "parts_su_air",
    "parts_us_vehicles", "parts_us_guns", "parts_us_air",
    "assemblies",
)


def load_parts():
    """Imports all part modules so that the registry is complete."""
    for name in _MODULES:
        importlib.import_module(f"{__name__}.{name}")
