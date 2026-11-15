"""Faction parts of `de` for prompt 008, session `guns`: AA guns, howitzers, heavy mortars and fortifications (env generators with @env_part).

Owned by that session only (one module per parallel session, so the three
de sessions never edit the same file). Part names carry the faction prefix
like `parts_de.py`: `@part("de.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("de_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401


@part("de.mortar_buffers")
def mortar_buffers(model, p, ctx):
    """Twin recoil buffers of the 12 cm GrW 42 (as on the 120-PM-38): two
    spring cylinders beside the tube that run from the bipod collar clamp up
    along the tube. Built after `mortar_tube` (uses its collar and
    elevation).

    Parameters: `length` (along the tube, default 0.42), `radius` (default
    0.035), `offset` (distance of each buffer axis from the tube axis,
    sideways, default 0.11), `parent` (default Gun).
    """
    import math

    from mathutils import Vector

    L = p.get("length", 0.42)
    r = p.get("radius", 0.035)
    off = p.get("offset", 0.11)
    a = math.radians(ctx["tube_elevation_deg"])
    d = Vector((0.0, math.cos(a), math.sin(a)))
    c = Vector(ctx["tube_collar"])
    mb = model.mb(p.get("parent", "Gun"))
    for k in (-1, 1):
        p0 = c + Vector((k * off, 0.0, 0.0)) - d * 0.05
        p1 = p0 + d * L
        mb.cylinder(tuple(p0), tuple(p1), r, seg=8, mat="Paint")
        mb.cylinder(tuple(p0), tuple(c + d * 0.0 + Vector((k * off * 0.4, 0.0, 0.0))), r * 0.5, seg=6, mat="Metal")
        mb.cylinder(tuple(p1 - d * 0.02), tuple(c + d * L + Vector((k * off * 0.4, 0.0, 0.0))), r * 0.5, seg=6,
                    mat="Metal")


@part("de.trailer_platform")
def trailer_platform(model, p, ctx):
    """Gun platform that stays on its single-axle trailer, with an A-frame
    drawbar to the rear (2 cm Flak 38 on the Sd.Ah. 51, fired from the
    trailer). Use as `[carriage]` of the `aa_gun` assembly: the drawbar is
    built before the barrel is fitted, so the catalog length runs from the
    towing eye to the muzzle.

    Parameters: `platform` (table for `cruciform_platform`, e.g. three
    arms for a triangular base), `drawbar_from` [x, y, z] (right hinge on the
    platform frame; the left one is mirrored), `drawbar_to` [y, z] (towing
    eye), `drawbar_radius` (default 0.035), `stand` (true: support leg from
    the towing eye to the ground, default true). Adds `Carriage` and
    `Socket_Tow` at the towing eye.
    """
    from .registry import build_part

    build_part("cruciform_platform", model, p.req("platform"), ctx, where="[carriage].platform")
    x, y0, z0 = p.req("drawbar_from")
    y1, z1 = p.req("drawbar_to")
    r = p.get("drawbar_radius", 0.035)
    car = model.mb("Carriage")
    for k in (-1, 1):
        car.cylinder((k * x, y0, z0), (0.0, y1 + 0.12, z1), r, seg=8, mat="Paint")
    car.cylinder((0.0, y1 + 0.14, z1), (0.0, y1 + 0.02, z1), r * 0.8, seg=8, mat="Metal")
    car.cylinder((0.0, y1, z1 - 0.025), (0.0, y1, z1 + 0.025), 0.06, seg=10, mat="Metal")  # towing eye
    if p.get("stand", True):
        car.cylinder((0.0, y1 + 0.18, z1), (0.0, y1 + 0.18, 0.04), r * 0.8, seg=6, mat="Metal")
        car.box((0.0, y1 + 0.18, 0.02), (0.12, 0.12, 0.04), "Metal")
    model.socket("Socket_Tow", "Carriage", (0.0, y1, z1))
