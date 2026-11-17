"""Turrets from a plan (top view) and side slopes, cupolas, turret hatches.

The turret object has its pivot at the turret ring center on the hull roof and
rotates around Z. Plans are `[x, y]` points in meters relative to the ring
center (front +Y), counter-clockwise or clockwise.

Height: `height = "fit"` makes the assembly derive the turret height from the
catalog height (minus cupola, unless the cupola is excluded from the height).
"""

import math

from mathutils import Vector

from .registry import part


def offset_polygon(poly, d):
    """Moves each vertex of a convex polygon inward by d (along the bisector)."""
    n = len(poly)
    area = sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n))
    sign = 1 if area > 0 else -1
    out = []
    for i in range(n):
        p0, p1, p2 = Vector(poly[i - 1]), Vector(poly[i]), Vector(poly[(i + 1) % n])
        e0 = (p1 - p0).normalized()
        e1 = (p2 - p1).normalized()
        n0 = Vector((-e0.y, e0.x)) * sign  # inward normal
        n1 = Vector((-e1.y, e1.x)) * sign
        b = (n0 + n1)
        if b.length < 1e-6:
            b = n0
        b.normalize()
        cos_half = max(0.2, b.dot(n0))
        q = p1 + b * (d / cos_half)
        out.append((q.x, q.y))
    return out


def superellipse(length, width, exponent, n, y_shift=0.0, front_scale=1.0):
    """Rounded plan (cast turrets): |x/a|^e + |y/b|^e = 1, front half scaled."""
    pts = []
    for i in range(n):
        t = 2 * math.pi * i / n
        c, s = math.cos(t), math.sin(t)
        x = width / 2 * math.copysign(abs(c) ** (2 / exponent), c)
        y = length / 2 * math.copysign(abs(s) ** (2 / exponent), s)
        if y > 0:
            x *= front_scale
        pts.append((x, y + y_shift))
    return pts


@part("plate_turret")
def plate_turret(model, p, ctx):
    """Welded turret: plan at the base, sides sloped inward by `side_slope_deg`.

    Parameters: `plan` [[x, y], ...], `height` (m or "fit"), `side_slope_deg`,
    `front_slope_deg` (extra inward slope of points with y > 0), `bevel`,
    `cast` (rounded edges), `open_top` (open-topped turret, M10 / M8 /
    BA-64: walls of `wall` thickness, default 0.025 m, no roof), `x`
    (prompt 063: ring center offset sideways, + = right, default 0; the
    `Turret` pivot moves with it; gun, hatches and MGs take model-space x:
    Pz.Kpfw. I).
    Returns `turret_top_z`, `turret_plan`.
    """
    xo = p.get("x", 0.0)
    plan = [(x + xo, y) for x, y in p.req("plan")]
    h = ctx["turret_height"]
    z0 = ctx["turret_z"]
    ty = ctx["turret_y"]
    slope = math.tan(math.radians(p.get("side_slope_deg", 10.0)))
    front = math.tan(math.radians(p.get("front_slope_deg", 0.0)))
    top = offset_polygon(plan, h * slope)
    top = [(x, y - (h * front if y > 0 else 0.0)) for x, y in top]
    p.get("height")
    mb = model.mb("Turret", parent="Hull", pivot=(xo, ty, z0))
    if p.get("cast", False):
        mb.bevel_segments = 3
    rings = [[(x, ty + y, z0) for x, y in plan], [(x, ty + y, z0 + h) for x, y in top]]
    if p.get("open_top", False):
        from .casemate import wall_shell

        p.get("bevel")
        wall_shell(mb, rings, p.get("wall", 0.025), floor=True)
    else:
        p.get("wall")
        mb.loft(rings, "Paint", bevel=p.get("bevel", 0.015), smooth=p.get("cast", False))
    mb.bevel_segments = 1
    # turret ring collar
    ring_r = ctx.get("turret_ring_r", 0.8)
    mb.cylinder((xo, ty, z0 - 0.06), (xo, ty, z0 + 0.01), ring_r, seg=16, mat="Paint", cap0=True, cap1=False)
    return {"turret_top_z": z0 + h, "turret_plan": plan, "turret_top_plan": top}


@part("cast_turret")
def cast_turret(model, p, ctx):
    """Cast turret: stacked superellipse rings, smooth shaded (Sherman, T-34 obr. 1941).

    Parameters: `length`, `width`, `exponent` (2 = ellipse, 4 = boxy),
    `front_scale` (front half narrower), `y_shift` (plan offset from ring
    center, + = forward), `height` (m or "fit"), `profile` [[z_frac, scale], ...]
    (ring shrink per height fraction, default rounded top), `segments`.
    Returns `turret_top_z`, `turret_plan`.
    """
    L = p.req("length")
    Wd = p.req("width")
    e = p.get("exponent", 2.6)
    fs = p.get("front_scale", 1.0)
    ys = p.get("y_shift", 0.0)
    n = p.get("segments", 20)
    prof = p.get("profile", [[0.0, 1.0], [0.55, 1.0], [0.85, 0.92], [1.0, 0.78]])
    p.get("height")
    h = ctx["turret_height"]
    z0 = ctx["turret_z"]
    ty = ctx["turret_y"]
    mb = model.mb("Turret", parent="Hull", pivot=(0.0, ty, z0))
    base = superellipse(L, Wd, e, n, ys, fs)
    rings = []
    for zf, sc in prof:
        rings.append([(x * sc, ty + ys + (y - ys) * sc, z0 + h * zf) for x, y in base])
    mb.loft(rings, "Paint", smooth=True)
    ring_r = ctx.get("turret_ring_r", 0.8)
    mb.cylinder((0, ty, z0 - 0.06), (0, ty, z0 + 0.01), ring_r, seg=16, mat="Paint", cap0=True, cap1=False)
    return {"turret_top_z": z0 + h, "turret_plan": base, "turret_top_plan": [(x * prof[-1][1], y) for x, y in base]}


@part("cupola")
def cupola(model, p, ctx):
    """Commander's cupola with vision blocks and a hatch lid (own object).

    Parameters: `x`, `y` (relative to ring center), `diameter`, `height`,
    `vision_blocks` (count), `z` (default turret top), `hatch` ("split" or
    "single"), `exclude_from_height` (validation, see blueprint [validate]),
    `parent` (default Turret; Hull for casemate vehicles).
    Adds `Cupola` (child of the parent), `Hatch_NN` and `Socket_Hatch_NN`.
    """
    x = p.req("x")
    y = ctx["turret_y"] + p.req("y")
    r = p.req("diameter") / 2
    h = p.req("height")
    z = p.get("z", ctx["turret_top_z"]) - 0.02
    p.get("exclude_from_height")
    blocks = p.get("vision_blocks", 6)
    tz = ctx["turret_z"]
    parent = p.get("parent", "Turret")
    mb = model.mb("Cupola", parent=parent, pivot=(x, y, z))
    mb.cylinder((x, y, z), (x, y, z + h * 0.75), r, seg=14, mat="Paint", cap0=True, cap1=True)
    for i in range(blocks):
        a = 2 * math.pi * (i + 0.5) / blocks
        cx, cy = x + math.cos(a) * r * 0.97, y + math.sin(a) * r * 0.97
        mb.box((cx, cy, z + h * 0.45), (0.1 + abs(math.sin(a)) * 0.06, 0.1 + abs(math.cos(a)) * 0.06, 0.07), "Glass")
    name = model.next_index("Hatch_")
    hatch_type = p.get("hatch", "single")
    hinge = (x, y - r * 0.8, z + h * 0.75)
    hb = model.mb(name, parent="Cupola", pivot=hinge)
    if hatch_type == "split":
        for k in (-1, 1):
            hb.box((x + k * r * 0.4, y, z + h * 0.85), (r * 0.75, r * 1.5, h * 0.18), "Paint", bevel=0.01)
    else:
        hb.cylinder((x, y, z + h * 0.75), (x, y, z + h), r * 0.82, r1=r * 0.7, seg=12, mat="Paint")
    model.socket(model.next_index("Socket_Hatch_"), parent, (x, y, z + h * 0.75))
    _ = tz
    return {"cupola_top_z": z + h}


@part("turret_hatch")
def turret_hatch(model, p, ctx):
    """Round or square roof hatch on the turret.

    Parameters: `x`, `y` (relative to ring center), `diameter` or `size`
    [x, y], `z` (default turret top), `socket` (adds Socket_Hatch_NN),
    `height` (lid thickness).
    """
    x = p.req("x")
    y = ctx["turret_y"] + p.req("y")
    z = p.get("z", ctx["turret_top_z"])
    t = p.get("height", 0.05)
    name = model.next_index("Hatch_")
    if "diameter" in p:
        r = p.req("diameter") / 2
        hb = model.mb(name, parent="Turret", pivot=(x, y - r, z))
        hb.cylinder((x, y, z - 0.01), (x, y, z + t), r, r1=r * 0.92, seg=12, mat="Paint")
    else:
        sx, sy = p.req("size")
        hb = model.mb(name, parent="Turret", pivot=(x, y - sy / 2, z))
        hb.box((x, y, z + t / 2 - 0.01), (sx, sy, t), "Paint", bevel=0.008)
    if p.get("socket", False):
        model.socket(model.next_index("Socket_Hatch_"), "Turret", (x, y, z))


@part("turret_bustle_box")
def turret_bustle_box(model, p, ctx):
    """Stowage box on the turret rear (Panzer IV 'Gepäckkasten').

    Parameters: `y` (relative to ring center, rear edge of the box), `size`
    [x, y, z], `z` (bottom, relative to turret base).
    """
    sx, sy, sz = p.req("size")
    y = ctx["turret_y"] + p.req("y")
    z = ctx["turret_z"] + p.get("z", 0.25)
    model.mb("Turret").box((0, y + sy / 2, z + sz / 2), (sx, sy, sz), "Paint", bevel=0.02)
