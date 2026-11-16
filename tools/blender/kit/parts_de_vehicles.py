"""Faction parts of `de` for prompt 008, session `vehicles`: tanks, tank destroyers, assault guns, SPGs, armored cars and trucks.

Owned by that session only (one module per parallel session, so the three
de sessions never edit the same file). Part names carry the faction prefix
like `parts_de.py`: `@part("de.<name>")`, every part with a docstring
listing its parameters; environment generators use `@env_part("de_<name>")`
from `.env`. Registered in `kit/__init__.py` (_MODULES) and loaded by
build_unit.py, build_env.py and kit_test.py.
"""

import math

from .registry import part


@part("de.smoke_dischargers")
def smoke_dischargers(model, p, ctx):
    """Nebelkerzenwurfgerät: row of smoke grenade launcher cups on both turret
    sides (Pz.Kpfw. II Ausf. L, Tiger I early/mid production), angled forward
    and outward.

    Parameters: `x` (distance of the cups from the center line), `y`, `z`
    (first cup, relative to the turret ring center and turret base), `count`
    per side (default 3), `step` (spacing along -Y, default 0.11), `length`
    (cup length, default 0.22), `radius` (default 0.05), `parent` (default
    Turret; with Hull, `x`/`y`/`z` are model space).
    """
    parent = p.get("parent", "Turret")
    x = p.req("x")
    if parent == "Turret":
        y0 = ctx["turret_y"] + p.req("y")
        z = ctx["turret_z"] + p.req("z")
    else:
        y0 = p.req("y")
        z = p.req("z")
    n = p.get("count", 3)
    step = p.get("step", 0.11)
    L = p.get("length", 0.22)
    r = p.get("radius", 0.05)
    mb = model.mb(parent)
    for side in (-1, 1):
        mb.box((side * (x - 0.01), y0 - step * (n - 1) / 2, z - r), (0.04, step * n, 0.04), "Paint")
        for i in range(n):
            y = y0 - step * i
            a = (side * x, y, z)
            # 45 degrees up, slightly forward and outward
            b = (side * (x + L * 0.25), y + L * 0.45, z + L * 0.85)
            mb.cylinder(a, b, r, seg=8, mat="Metal")


@part("de.grenade_screen")
def grenade_screen(model, p, ctx):
    """Folding wire-mesh grenade screen (Sd.Kfz. 221/222 'Drahtgitteraufsatz')
    over an open turret: two halves forming a shallow tent, each hinged at
    its outer rim so it folds outward. The halves are the objects `Hatch_NN`
    (children of the turret, pivot on the hinge line), with a
    `Socket_Hatch_NN` for the commander under the left half.

    Parameters: `half_width` (rim half-width across X), `y` [rear, front]
    (relative to the turret ring center), `z` (rim height above the turret
    base; default the turret height), `ridge` (height of the center ridge
    above the rim, default 0.12), `bars` (cross bars per half, default 5),
    `gap` (open slot at the ridge for the guns, default 0.0).
    Built from thin Metal bars only, so the open turret stays visible from
    above.
    """
    hw = p.req("half_width")
    yr, yf = p.req("y")
    ty = ctx["turret_y"]
    y0, y1 = ty + yr, ty + yf
    z = ctx["turret_z"] + p.get("z", ctx["turret_height"])
    ridge = p.get("ridge", 0.12)
    nb = p.get("bars", 5)
    gap = p.get("gap", 0.0)
    t = 0.018
    for side in (-1, 1):
        xo = side * hw
        xr = side * gap / 2
        name = model.next_index("Hatch_")
        mb = model.mb(name, parent="Turret", pivot=(xo, (y0 + y1) / 2, z))
        # frame: outer rim (hinge), ridge bar, front and rear edges
        mb.box((xo, (y0 + y1) / 2, z), (t, y1 - y0, t), "Metal")
        mb.box((xr, (y0 + y1) / 2, z + ridge), (t, y1 - y0, t), "Metal")
        for y in (y0, y1):
            mb.cylinder((xo, y, z), (xr, y, z + ridge), 0.012, seg=4, mat="Metal")
        # cross bars and mesh lines (wire-mesh look)
        for i in range(1, nb + 1):
            y = y0 + (y1 - y0) * i / (nb + 1)
            mb.cylinder((xo, y, z), (xr, y, z + ridge), 0.006, seg=4, mat="Metal")
        for k in (0.33, 0.66):
            x = xo + (xr - xo) * k
            mb.box((x, (y0 + y1) / 2, z + ridge * k), (0.006, y1 - y0, 0.006), "Metal")
    model.socket(model.next_index("Socket_Hatch_"), "Turret", (-hw * 0.4, (y0 + y1) / 2, z - 0.05))


@part("de.radiator_grille")
def radiator_grille(model, p, ctx):
    """Vertical radiator grille with bars on a flat front (Opel Blitz).

    Parameters: `center` [x, y, z] (grille face), `size` [x, z], `bars`
    (vertical bars, default 9), `parent` (default Hull).
    """
    x, y, z = p.req("center")
    sx, sz = p.req("size")
    n = p.get("bars", 9)
    mb = model.mb(p.get("parent", "Hull"))
    mb.box((x, y - 0.01, z), (sx + 0.04, 0.03, sz + 0.04), "Paint", bevel=0.01)
    mb.box((x, y + 0.006, z), (sx, 0.004, sz), "Metal")
    for i in range(n):
        bx = x - sx / 2 + sx * (i + 0.5) / n
        mb.box((bx, y + 0.012, z), (0.012, 0.012, sz * 0.96), "Paint")


@part("de.round_wings")
def round_wings(model, p, ctx):
    """Curved front wings of a 1930s truck (Opel Blitz): sheet bent in an arc
    around the front wheel, flowing into the running board.

    Parameters: `wheel` [y, z, radius] (front wheel center and tire radius),
    `x` [inner, outer] (wing half-widths), `clearance` (gap above the tire,
    default 0.07), `front_deg` / `rear_deg` (arc ends measured from straight
    up, forward positive; default 70 / -95), `segments` (default 7),
    `running_board` [y_rear, z] (optional board from the wing back to
    y_rear at height z), `thickness` (default 0.012), `parent` (default Hull).
    """
    wy, wz, wr = p.req("wheel")
    xi, xo = sorted(p.req("x"))
    cl = p.get("clearance", 0.07)
    a0 = math.radians(p.get("front_deg", 70.0))
    a1 = math.radians(p.get("rear_deg", -95.0))
    n = p.get("segments", 7)
    t = p.get("thickness", 0.012)
    run = p.get("running_board")
    mb = model.mb(p.get("parent", "Hull"))
    R = wr + cl
    pts = []
    for i in range(n + 1):
        a = a0 + (a1 - a0) * i / n
        pts.append((wy + R * math.sin(a), wz + R * math.cos(a)))
    if run:
        pts.append((pts[-1][0] - 0.08, run[1]))
    for side in (-1, 1):
        x0, x1 = sorted((side * xi, side * xo))
        for (ya, za), (yb, zb) in zip(pts, pts[1:]):
            dy, dz = yb - ya, zb - za
            ln = math.hypot(dy, dz)
            ny, nz = dz / ln * t, -dy / ln * t
            quad = [(ya, za), (yb, zb), (yb + ny, zb + nz), (ya + ny, za + nz)]
            mb.extrude_x(quad, x0, x1, "Paint")
        if run:
            y_rear, z = run
            y_front = pts[-1][0]
            mb.box((side * (xi + xo) / 2, (y_front + y_rear) / 2, z), (xo - xi, y_front - y_rear, 0.02), "Paint")
            mb.box((side * (xi + 0.02), (y_front + y_rear) / 2, z - 0.06), (0.04, y_front - y_rear, 0.1), "Paint")


@part("de.gun_housing")
def gun_housing(model, p, ctx):
    """Cast gun housing bulging out of a sloped front plate, behind the
    'Saukopf' mantlet (Jagdpanzer 38(t)): a drum along +Y with a rounded
    front shoulder; the plate hides its rear part.

    Parameters: `position` [x, y, z] (center of the front face), `length`
    (back into the plate), `radius`, `parent` (default Hull).
    """
    x, y, z = p.req("position")
    length = p.req("length")
    r = p.req("radius")
    mb = model.mb(p.get("parent", "Hull"))
    mb.revolve((x, y - length, z), (0, 1, 0),
               [(0.0, r), (length * 0.72, r), (length * 0.88, r * 0.93), (length * 0.97, r * 0.78), (length, r * 0.55)],
               seg=14, mat="Paint")
