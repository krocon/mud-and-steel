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


@part("de.flat_muzzle_brake")
def flat_muzzle_brake(model, p, ctx):
    """Flat double-baffle muzzle brake (7.5 cm PaK 40): wider than high, two
    box-shaped chambers. Built over the last `length` of the barrel so that
    it ends at the muzzle (`gun_tip_y`); use `muzzle_brake = "none"` on the
    gun and leave the barrel long enough.

    Parameters: `length` (default 0.30), `width` (default 0.24), `height`
    (default 0.13), `parent` (default Gun).
    """
    L = p.get("length", 0.30)
    w = p.get("width", 0.24)
    h = p.get("height", 0.13)
    tip = ctx["gun_tip_y"]
    z = ctx["gun_z"]
    mb = model.mb(p.get("parent", "Gun"))
    # rear chamber, waist, front chamber (baffle openings show as the gaps)
    mb.box((0.0, tip - L * 0.80, z), (w, L * 0.36, h), "Metal", bevel=0.012)
    mb.box((0.0, tip - L * 0.52, z), (w * 0.45, L * 0.22, h * 0.75), "Metal")
    mb.box((0.0, tip - L * 0.21, z), (w * 0.96, L * 0.38, h * 0.95), "Metal", bevel=0.012)
    mb.box((0.0, tip - L * 0.01, z), (w * 0.5, L * 0.02, h * 0.6), "Metal")
