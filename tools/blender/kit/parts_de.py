"""German faction parts (prefix `de.`). Only this module and German
blueprints change for German-specific details (docs/ASSETS.md, parallel work).
"""

from .registry import part


@part("de.driver_visor")
def driver_visor(model, p, ctx):
    """Fahrersehklappe: armored visor block with vision slit on a front plate.

    Parameters: `position` [x, y, z] (center on the plate face), `size`
    [x, z] (default 0.32 x 0.14), `parent` (default Hull).
    """
    x, y, z = p.req("position")
    sx, sz = p.get("size", [0.32, 0.14])
    mb = model.mb(p.get("parent", "Hull"))
    mb.box((x, y + 0.03, z), (sx, 0.06, sz), "Paint", bevel=0.01)
    mb.box((x, y + 0.061, z + sz * 0.1), (sx * 0.7, 0.004, sz * 0.15), "Glass")


@part("de.notek")
def notek(model, p, ctx):
    """Notek blackout driving light on a short post. Parameters: `position`, `parent`."""
    x, y, z = p.req("position")
    mb = model.mb(p.get("parent", "Hull"))
    mb.cylinder((x, y, z), (x, y, z + 0.12), 0.015, seg=6, mat="Metal")
    mb.box((x, y + 0.02, z + 0.17), (0.12, 0.14, 0.1), "Paint", bevel=0.01)
    mb.box((x, y + 0.092, z + 0.17), (0.08, 0.004, 0.02), "Glass")


@part("de.turret_vision_ports")
def turret_vision_ports(model, p, ctx):
    """Vision ports beside the mantlet of the Panzer IV turret front.

    Parameters: `x` (offset left/right of the gun), `y`, `z` (relative to
    the ring center and turret base).
    """
    x = p.req("x")
    y = ctx["turret_y"] + p.req("y")
    z = ctx["turret_z"] + p.req("z")
    mb = model.mb("Turret")
    for k in (-1, 1):
        mb.box((k * x, y, z), (0.2, 0.05, 0.1), "Paint", bevel=0.008)
        mb.box((k * x, y + 0.026, z), (0.12, 0.004, 0.02), "Glass")
