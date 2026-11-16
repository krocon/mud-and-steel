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


def _clip_half(poly, side, x0):
    """Part of a convex plan polygon with side * x >= x0 (Sutherland-Hodgman)."""
    out = []
    n = len(poly)
    for i in range(n):
        (ax, ay), (bx, by) = poly[i], poly[(i + 1) % n]
        da, db = side * ax - x0, side * bx - x0
        if da >= 0:
            out.append((ax, ay))
        if (da >= 0) != (db >= 0):
            k = da / (da - db)
            out.append((side * x0, ay + (by - ay) * k))
    return out


def _wire(mb, a, b, r=0.005):
    """Thin mesh wire: three-sided, no caps (6 triangles)."""
    mb.cylinder(a, b, r, seg=3, mat="Metal", cap0=False, cap1=False)


@part("de.grenade_screen")
def grenade_screen(model, p, ctx):
    """Folding wire-mesh grenade screen (Sd.Kfz. 221/222 'Drahtgitteraufsatz')
    over an open turret: a shallow mesh crown that follows the turret rim
    (sloped mesh panels all round, flat mesh top), split along the center
    line into two halves hinged at their outer rim so they fold outward. The
    halves are the objects `Hatch_NN` (children of the turret, pivot on the
    hinge line), with a `Socket_Hatch_NN` for the commander under the left
    half. Needs a turret built before it (uses its top plan).

    Parameters: `height` (flat top above the turret rim, default 0.16),
    `inset` (how far the flat top lies inside the rim, default 0.13), `gap`
    (open slot along the center line for the guns, default 0.10), `mesh`
    (wire spacing, default 0.13), `z` (rim height above the turret base;
    default the turret height).
    Built from thin Metal bars only, so the open turret stays visible from
    above.
    """
    from .turret import offset_polygon

    plan = [tuple(v) for v in ctx["turret_top_plan"]]
    ty = ctx["turret_y"]
    z0 = ctx["turret_z"] + p.get("z", ctx["turret_height"])
    h = p.get("height", 0.16)
    inset = p.get("inset", 0.13)
    gap = p.get("gap", 0.10)
    step = p.get("mesh", 0.13)
    z1 = z0 + h
    inner_plan = offset_polygon(plan, inset)

    def lerp(a, b, k):
        return tuple(a[i] + (b[i] - a[i]) * k for i in range(3))

    for side in (-1, 1):
        outer = [(x, ty + y, z0) for x, y in _clip_half(plan, side, gap / 2)]
        inner = [(x, ty + y, z1) for x, y in _clip_half(inner_plan, side, gap / 2)]
        if len(outer) != len(inner) or len(outer) < 3:
            raise ValueError("de.grenade_screen: inset or gap too large for the turret top")
        n = len(outer)
        xo = side * max(abs(v[0]) for v in outer)
        ys = [v[1] for v in outer]
        name = model.next_index("Hatch_")
        mb = model.mb(name, parent="Turret", pivot=(xo, (min(ys) + max(ys)) / 2, z0))
        for i in range(n):
            a, b, c, d = outer[i], outer[(i + 1) % n], inner[(i + 1) % n], inner[i]
            center = abs(abs(a[0]) - gap / 2) < 1e-6 and abs(abs(b[0]) - gap / 2) < 1e-6
            mb.cylinder(d, c, 0.009, seg=4, mat="Metal", cap0=False, cap1=False)  # frame of the flat top
            mb.cylinder(a, d, 0.009, seg=4, mat="Metal", cap0=False, cap1=False)  # corner strut
            if center:
                continue
            mb.cylinder(a, b, 0.011, seg=4, mat="Metal", cap0=False, cap1=False)  # rim (hinge) frame
            _wire(mb, lerp(a, d, 0.5), lerp(b, c, 0.5))
            length = math.dist(a, b)
            k = max(1, round(length / step))
            for j in range(1, k):
                _wire(mb, lerp(a, b, j / k), lerp(d, c, j / k))
        # flat top: wires along Y and X, clipped to the top outline
        xs = [v[0] for v in inner]
        iys = [v[1] for v in inner]

        def cuts(axis, value):
            other = 1 - axis
            hit = []
            for i in range(n):
                a, b = inner[i], inner[(i + 1) % n]
                if (a[axis] - value) * (b[axis] - value) < 0:
                    hit.append(a[other] + (b[other] - a[other]) * (value - a[axis]) / (b[axis] - a[axis]))
            return sorted(hit)

        x = gap / 2 + step
        while x < max(abs(v) for v in xs) - 0.03:
            hit = cuts(0, side * x)
            if len(hit) == 2:
                _wire(mb, (side * x, hit[0], z1), (side * x, hit[1], z1))
            x += step
        y = min(iys) + step * 0.75
        while y < max(iys) - 0.03:
            hit = cuts(1, y)
            if len(hit) == 2:
                _wire(mb, (hit[0], y, z1), (hit[1], y, z1))
            y += step
    model.socket(model.next_index("Socket_Hatch_"), "Turret", (-0.22, ty, z0 - 0.05))


def _oriented_box(mb, m, size, z0, z1, mat, offset=(0.0, 0.0)):
    """Box in the local frame of matrix `m`: `size` across local X/Y around
    `offset`, from local z0 to z1."""
    from mathutils import Vector

    hx, hy = size[0] / 2, size[1] / 2
    ox, oy = offset
    v = mb.verts([m @ Vector((ox + sx * hx, oy + sy * hy, z)) for z in (z0, z1) for sy in (-1, 1) for sx in (-1, 1)])
    idx = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return mb._shell([mb.face([v[i] for i in q], mat) for q in idx])


@part("de.flap")
def flap(model, p, ctx):
    """Armored flap or lid lying on a flat or sloped plate: visor flaps
    ('Sehklappen') with a vision slit, engine deck hatches, louvre plates.
    The rough position is moved along the normal onto the parent's surface,
    so positions on sloped plates need not be exact.

    Parameters: `position` [x, y, z] (center), `normal` (surface normal
    pointing out), `size` [width, height] (height runs up the plate; on
    decks toward the front), `thickness` (default 0.02), `slit` (default
    false: Glass vision slit), `bars` (default 0: raised louvre bars across
    the width), `material` (default Paint), `mirror` (default false: also
    builds the copy mirrored to -x), `parent` (default Hull).
    """
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree

    from .core import surface_matrix

    parent = p.get("parent", "Hull")
    pos = Vector(p.req("position"))
    normal = Vector(p.req("normal")).normalized()
    w, h = p.req("size")
    t = p.get("thickness", 0.02)
    mat = p.get("material", "Paint")
    slit = p.get("slit", False)
    bars = p.get("bars", 0)
    mb = model.mb(parent)
    tree = BVHTree.FromBMesh(mb.bm)
    sides = (1, -1) if p.get("mirror", False) else (1,)
    for side in sides:
        c = Vector((side * pos.x, pos.y, pos.z))
        nrm = Vector((side * normal.x, normal.y, normal.z))
        hit = tree.ray_cast(c + nrm * 0.6, -nrm, 1.2)
        if hit[0] is None:
            raise ValueError(f"de.flap: no {parent} surface within 0.6 m along the normal of {tuple(c)}")
        m = surface_matrix(hit[0], nrm)
        _oriented_box(mb, m, (w, h), -0.01, t, mat)
        if slit:
            _oriented_box(mb, m, (w * 0.62, h * 0.16), t, t + 0.005, "Glass", offset=(0.0, h * 0.08))
        for i in range(bars):
            _oriented_box(mb, m, (w * 0.9, h * 0.35 / bars), t, t + 0.012, mat,
                          offset=(0.0, h * ((i + 0.5) / bars - 0.5) * 0.9))


@part("de.spare_wheel")
def spare_wheel(model, p, ctx):
    """Spare wheel carried flat against the hull side under a canvas cover
    (Sd.Kfz. 221/222/223: right side, between the wheels).

    Parameters: `position` [x, y, z] (center of the face toward the hull),
    `diameter`, `width`, `axis` [x, y, z] (pointing away from the hull,
    default [1, 0, 0]), `cover` (default true: Canvas; false: bare Rubber
    tire), `parent` (default Hull).
    """
    pos = tuple(p.req("position"))
    r = p.req("diameter") / 2
    w = p.req("width")
    axis = tuple(p.get("axis", [1.0, 0.0, 0.0]))
    mat = "Canvas" if p.get("cover", True) else "Rubber"
    mb = model.mb(p.get("parent", "Hull"))
    mb.revolve(pos, axis, [(0.0, r * 0.97), (w * 0.70, r), (w * 0.92, r * 0.88), (w, r * 0.60)],
               seg=14, mat=mat, cap_mat=mat)


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
