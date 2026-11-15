"""Faction parts of `us` for prompt 008, session `vehicles`: tanks, tank destroyers, assault guns, SPGs, armored cars and trucks.

Owned by that session only (one module per parallel session, so the three
us sessions never edit the same file). Part names carry the faction prefix
like `parts_us.py`: `@part("us.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("us_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

from .registry import part  # noqa: F401


# --- prompt 008 us vehicles -----------------------------------------------------------


@part("us.cargo_cover")
def cargo_cover(model, p, ctx):
    """Canvas tilt over an open cargo body (GMC CCKW), stretched over bows.

    Parameters: `front_y`, `rear_y` (default: the cargo_bed rear end
    `bed_rear_y`), `width` (default: `bed_width`, else the catalog width),
    `bottom_z` (lower edge of the canvas, default `bed_top_z` - 0.10, i.e.
    overlapping the body sides), `top_z` (m or "fit" = catalog height,
    default), `shoulder` (height of the vertical canvas sides above
    `bottom_z` as a fraction of the total, default 0.55), `bows` (visible
    bow ribs, default 4), `segments` (roof arc segments, default 6),
    `rear_open` (rear flap rolled up: open rear face, default false).
    Builds into `Hull` with the `Canvas` slot.
    A generic `canvas_cover` for all trucks is proposed in the 008 report.
    """
    import math

    yf = p.req("front_y")
    yr = p.get("rear_y", ctx.get("bed_rear_y"))
    if yr is None:
        raise KeyError("us.cargo_cover: rear_y missing (no cargo_bed before it)")
    w = p.get("width", ctx.get("bed_width", ctx["width"]))
    z0 = p.get("bottom_z", ctx.get("bed_top_z", 1.5) - 0.10)
    top = p.get("top_z", "fit")
    zt = ctx["height"] if top == "fit" else top
    shoulder = p.get("shoulder", 0.55)
    nb = p.get("bows", 4)
    seg = p.get("segments", 6)
    hw = w / 2
    zs = z0 + (zt - z0) * shoulder
    rise = zt - zs
    # cross-section (x, z), counter-clockwise seen from the front
    sec = [(hw, z0), (hw, zs)]
    for i in range(1, seg):
        a = math.pi * i / seg
        sec.append((math.cos(a) * hw, zs + math.sin(a) * rise))
    sec += [(-hw, zs), (-hw, z0)]
    mb = model.mb("Hull")
    rings = [[(x, y, z) for x, z in sec] for y in (yf, yr)]
    mb.loft(rings, "Canvas", cap_start=True, cap_end=not p.get("rear_open", False), smooth=False)
    # bows: thin ribs standing slightly proud of the canvas
    for k in range(nb):
        y = yf - 0.15 - (yf - yr - 0.30) * k / max(nb - 1, 1)
        pts = [(x * 1.01, z + (0.008 if z > zs else 0.0)) for x, z in sec]
        for (xa, za), (xb, zb) in zip(pts, pts[1:]):
            cx, cz = (xa + xb) / 2, (za + zb) / 2
            length = math.hypot(xb - xa, zb - za)
            if abs(xb - xa) < 1e-6:
                mb.box((cx, y, cz), (0.02, 0.05, length), "Canvas")
            elif abs(zb - za) < 1e-6:
                mb.box((cx, y, cz), (length, 0.05, 0.02), "Canvas")
            else:
                ang = math.atan2(zb - za, xb - xa)
                nx, nz = -math.sin(ang) * 0.01, math.cos(ang) * 0.01
                ux, uz = math.cos(ang) * length / 2, math.sin(ang) * length / 2
                quad = [(cx - ux - nx, y - 0.025, cz - uz - nz), (cx + ux - nx, y - 0.025, cz + uz - nz),
                        (cx + ux + nx, y - 0.025, cz + uz + nz), (cx - ux + nx, y - 0.025, cz - uz + nz)]
                mb.extrude(quad, (0, 0.05, 0), "Canvas")
