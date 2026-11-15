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
  `artillery`, `aircraft`   generic parts
- `parts_de`, `parts_su`, `parts_us`   faction-specific parts
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
    "parts_de",
    "parts_su",
    "parts_us",
    "assemblies",
)


def load_parts():
    """Imports all part modules so that the registry is complete."""
    for name in _MODULES:
        importlib.import_module(f"{__name__}.{name}")
