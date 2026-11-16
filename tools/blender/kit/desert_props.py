"""Desert village walls, props and landmarks (prompt 059): courtyard walls,
awnings, jars, well, cart, market stall, village mosque, police post.

North African village 1940-43 (Libya, Tunisia): plastered mud brick in
sand, ochre and whitewash tones (Plaster kinds 0 lime, 1 adobe, 2 whitewash,
3 ochre), weathered wood, muted cloth (`Canvas`), sand drifts (Ground kind
3). Nothing modern, no writing, signs, symbols, insignia or flags.

Unglazed clay (jars) uses Plaster kind 3 (ochre): it reads as terracotta
without the tile rows of the `Roof` slot.

The two buildings (`mosque`, `police_post`) are modular like the houses of
`buildings.py` (`<V>_Walls`, `_Roof`, `_Floor`, `_Tower`, `_Debris`) and
reuse its `House` helper; round arches are added as spandrel infills above
rectangular openings (`_DHouse`).
"""

import math

import bmesh
from mathutils import Vector

from .buildings import House, Opening, _blast, _spread  # noqa: F401
from .core import _basis
from .env import CARD_PALM, env_part, rng

# -- shared helpers ------------------------------------------------------------


def _sheet(mb, fn, nx, ny, off, mat, skip=()):
    """Cloth slab from a parametric surface `fn(u, v) -> (x, y, z)` (u, v in
    0..1) with a thickness offset vector `off` (both sides are visible, the
    env shader culls back faces). Cells (i, j) in `skip` are torn out."""
    off = Vector(off)
    top = [[mb.bm.verts.new(Vector(fn(i / nx, j / ny))) for j in range(ny + 1)] for i in range(nx + 1)]
    bot = [[mb.bm.verts.new(v.co + off) for v in row] for row in top]
    center = sum((v.co for row in top for v in row), Vector()) / ((nx + 1) * (ny + 1))
    up = -off.normalized()
    faces = []
    for i in range(nx):
        for j in range(ny):
            if (i, j) in skip:
                continue
            q = [top[i][j], top[i + 1][j], top[i + 1][j + 1], top[i][j + 1]]
            f = mb.face(q, mat, smooth=True)
            f.normal_update()
            if f.normal.dot(up) < 0:
                f.normal_flip()
            g = mb.face([bot[i][j], bot[i + 1][j], bot[i + 1][j + 1], bot[i][j + 1]], mat, smooth=True)
            g.normal_update()
            if g.normal.dot(up) > 0:
                g.normal_flip()
            faces += [f, g]
    # rim along the outer border
    border = ([(i, 0, i + 1, 0) for i in range(nx)] + [(nx, j, nx, j + 1) for j in range(ny)]
              + [(i, ny, i + 1, ny) for i in range(nx)] + [(0, j, 0, j + 1) for j in range(ny)])
    for i0, j0, i1, j1 in border:
        ci, cj = min(i0, i1, nx - 1), min(j0, j1, ny - 1)
        if (ci, cj) in skip:
            continue
        f = mb.face([top[i0][j0], top[i1][j1], bot[i1][j1], bot[i0][j0]], mat)
        f.normal_update()
        mid = (top[i0][j0].co + top[i1][j1].co) / 2
        if f.normal.dot(mid - center) < 0:
            f.normal_flip()
        faces.append(f)
    return faces


def _pole(mb, r, base, top, rad, mat="Wood", seg=5, crook=0.04):
    """Slightly crooked round pole from `base` to `top`."""
    a, b = Vector(base), Vector(top)
    m = (a + b) / 2 + Vector((r.uniform(-crook, crook), r.uniform(-crook, crook), 0))
    mb.tube([a, m, b], [rad * 1.1, rad, rad * 0.9], seg, mat, cap_start=False)


def _rope(mb, a, b, rad=0.012, mat="Canvas"):
    mb.cylinder(a, b, rad, seg=3, mat=mat, smooth=False)


def _annulus(mb, c, axis, R, r_in, w, seg, mat):
    """Flat ring around `axis` (wheel rims, pulleys, well coping)."""
    c, d = Vector(c), Vector(axis).normalized()
    u, v = _basis(d)

    def ring(rad, s):
        return mb.verts([c + d * (s * w / 2) + (u * math.cos(2 * math.pi * k / seg)
                                                + v * math.sin(2 * math.pi * k / seg)) * rad for k in range(seg)])

    o0, o1, i0, i1 = ring(R, -1), ring(R, 1), ring(r_in, -1), ring(r_in, 1)
    faces = []
    for k in range(seg):
        j = (k + 1) % seg
        faces += [mb.face([o0[k], o0[j], o1[j], o1[k]], mat), mb.face([i0[k], i1[k], i1[j], i0[j]], mat),
                  mb.face([o0[k], i0[k], i0[j], o0[j]], mat), mb.face([o1[k], o1[j], i1[j], i1[k]], mat)]
    bmesh.ops.recalc_face_normals(mb.bm, faces=faces)
    return faces


def _jar(mb, base, H, R, axis=(0, 0, 1), seg=8, kind=3, mat="Plaster"):
    """Unglazed storage jar (amphora-like belly, short neck, rolled lip)."""
    mb.style(kind=kind)
    prof = [(0.0, R * 0.5), (H * 0.14, R * 0.86), (H * 0.45, R), (H * 0.74, R * 0.78),
            (H * 0.88, R * 0.38), (H, R * 0.46), (H * 0.97, R * 0.33)]
    mb.revolve(base, axis, prof, seg=seg, mat=mat, smooth=True)


def _patch(mb, r, side, y, z, w, h, x_face):
    """Spalled plaster patch on the face x = side * x_face: a ragged plate of
    bare mud render (Plaster kind 1) with the brick (Stone kind 3) showing
    in its middle."""
    for scale, mat, kind, out in ((1.0, "Plaster", 1, 0.006), (0.55, "Stone", 3, 0.012)):
        pts = []
        n = 7
        for k in range(n):
            a = 2 * math.pi * k / n + r.uniform(-0.2, 0.2)
            rr = r.uniform(0.6, 1.0) * scale
            pts.append((side * (x_face + out), y + math.cos(a) * w / 2 * rr, z + math.sin(a) * h / 2 * rr))
        mb.style(kind=kind)
        mb.extrude(pts, (-side * 0.03, 0, 0), mat)


def _wall_body(mb, ys, heights, hb, ht, mat="Plaster"):
    """Loft of a battered wall cross-section with a rounded mud coping."""
    rings = []
    for y, h in zip(ys, heights):
        ch = min(0.12, h * 0.15)
        rings.append([(-hb, y, 0), (hb, y, 0), (ht, y, h - ch), (ht * 0.5, y, h), (-ht * 0.5, y, h),
                      (-ht, y, h - ch)])
    mb.loft(rings, mat, True, True)


def _drift(mb, r, side, y0, y1, x_in, x_out, hmax, n=6):
    """Sand drift against a wall face: wedge loft along Y, high in the middle."""
    mb.style(kind=3)
    rings = []
    for k in range(n + 1):
        t = k / n
        y = y0 + (y1 - y0) * t
        hh = max(0.02, hmax * math.sin(math.pi * t) ** 0.7 * r.uniform(0.8, 1.1))
        xo = x_in + (x_out - x_in) * max(0.35, math.sin(math.pi * t) ** 0.5)
        rings.append([(side * x_in, y, 0), (side * xo, y, 0), (side * (x_in + (xo - x_in) * 0.35), y, hh * 0.5),
                      (side * x_in, y, hh)])
    mb.loft(rings, "Ground", True, True, smooth=True)


# -- A) courtyard wall -----------------------------------------------------------


@env_part("wall_mud")
def wall_mud(model, name, p, box):
    """Courtyard wall of plastered mud brick, running along Y (a segment spans
    y -L/2..L/2 and chains every catalog length).

    Parameters: `seed`; `piece` ("segment", "broken", "gate", "pier"; default
    "segment"; "pier" is the end / corner pier filling the box); `kind`
    (Plaster kind, default 0 lime); `thick` (wall thickness at the foot,
    default 0.32); `patches` (spalled plaster patches showing brick, default
    3); `drift` (height of the sand drift at the foot, default 0.35, 0 =
    none).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "segment")
    kind = p.get("kind", 0)
    thick = p.get("thick", 0.32)
    patches = p.get("patches", 3)
    drift = p.get("drift", 0.35)
    H, hl, hw = box.height, box.hl, box.hw

    if piece == "pier":
        mb.style(kind=kind)
        mb.box((0, 0, (H - 0.16) / 2), (box.width, box.length, H - 0.16), "Plaster", bevel=0.03)
        mb.box((0, 0, H - 0.09), (box.width * 0.82, box.length * 0.82, 0.18), "Plaster", bevel=0.05)
        for _ in range(patches):
            side = r.choice((-1, 1))
            _patch(mb, r, side, r.uniform(-hl * 0.3, hl * 0.3), r.uniform(0.5, H - 0.7), 0.3, 0.45, hw)
        return

    if piece == "gate":
        pier_l = 0.5
        mb.style(kind=kind)
        for s in (-1, 1):
            y = s * (hl - pier_l / 2)
            mb.box((0, y, (H - 0.16) / 2), (box.width, pier_l, H - 0.16), "Plaster", bevel=0.03)
            mb.box((0, y, H - 0.09), (box.width * 0.82, pier_l * 0.8, 0.18), "Plaster", bevel=0.05)
            _patch(mb, r, r.choice((-1, 1)), y + r.uniform(-0.05, 0.05), r.uniform(0.6, 1.6), 0.3, 0.45, hw)
        span = box.length - 2 * pier_l
        lintel_z = 2.15
        t = min(0.42, box.width * 0.6)
        # lintel beam and mud infill above it
        mb.style(kind=2)
        mb.box((0, 0, lintel_z + 0.11), (t + 0.06, span + 0.3, 0.22), "Wood")
        mb.style(kind=kind)
        top = H - 0.2
        mb.box((0, 0, (lintel_z + 0.22 + top) / 2), (t, span, top - lintel_z - 0.22), "Plaster")
        mb.box((0, 0, top + 0.04), (t * 0.75, span, 0.08), "Plaster", bevel=0.03)
        # closed double plank door
        leaf = span / 2 - 0.01
        door_h = lintel_z - 0.02
        for s in (-1, 1):
            yc = s * (leaf / 2 + 0.005)
            n = 4
            for k in range(n):
                yy = yc - leaf / 2 + leaf * (k + 0.5) / n
                mb.style(kind=0 if (k + (s > 0)) % 2 else 2)
                mb.box((0, yy, door_h / 2 + 0.01), (0.06, leaf / n - 0.012, door_h - r.uniform(0.0, 0.03)), "Wood")
            mb.style(kind=2)
            for zz in (0.35, door_h * 0.5, door_h - 0.3):
                mb.box((0.045, yc, zz), (0.03, leaf * 0.94, 0.12), "Wood")
                mb.box((-0.045, yc, zz), (0.03, leaf * 0.94, 0.12), "Wood")
        # threshold stone and a little sand against the piers
        mb.style(kind=0)
        mb.box((0, 0, 0.025), (t, span, 0.05), "Stone")
        return

    hb, ht = thick / 2, thick / 2 - 0.025
    broken = piece == "broken"
    n = 13 if broken else 7
    ys = [-hl + box.length * k / (n - 1) for k in range(n)]
    low = p.get("low", 0.75)
    hs = []
    for k, y in enumerate(ys):
        t = k / (n - 1)
        h = H * (1.0 - r.uniform(0.0, 0.03))
        if broken:
            if k == 0:
                h = H
            elif t < 0.22:
                h = H * (1.0 - r.uniform(0.0, 0.03))
            elif t < 0.4:
                h = H + (low - H) * (t - 0.22) / 0.18 + r.uniform(-0.12, 0.12)
            elif t < 0.75:
                h = low + r.uniform(-0.15, 0.12)
            else:
                h = low + (H * 0.72 - low) * (t - 0.75) / 0.25 + r.uniform(-0.1, 0.1)
        hs.append(h)
    if not broken:
        hs[0] = H
    mb.style(kind=kind)
    _wall_body(mb, ys, hs, hb, ht)

    def x_face(z):
        return hb + (ht - hb) * min(z / H, 1.0)

    # spalled plaster patches showing the brick
    for i in range(patches):
        side = 1 if i % 2 == 0 else -1
        y = r.uniform(-hl + 0.4, hl - 0.4)
        hmax = min(h for yy, h in zip(ys, hs) if abs(yy - y) < box.length / (n - 1) + 0.01)
        z = r.uniform(0.5, max(0.55, hmax - 0.45))
        w, hh = r.uniform(0.5, 0.9), r.uniform(0.35, 0.6)
        _patch(mb, r, side, y, z, w, hh, x_face(z))
    if broken:
        # exposed brick courses on the broken top, rubble mound at the foot
        mb.style(kind=3)
        for k in range(3, n - 2):
            if r.random() < 0.6:
                mb.oriented_box((r.uniform(-0.05, 0.05), ys[k], hs[k] - 0.02), (r.uniform(-0.3, 0.3), 1, 0),
                                (0, 0, 1), (0.24, ht * 1.5, 0.08), "Stone")
        for k in range(4):
            y = r.uniform(-0.3, hl - 0.4)
            mb.style(kind=kind if k % 2 else 3)
            mb.blob((0, y, 0), (hw * 0.98, r.uniform(0.25, 0.45), r.uniform(0.2, 0.35)),
                    "Plaster" if k % 2 else "Stone", seg=7, rings=3, rng=r, rough=0.25, flat_bottom=0.0,
                    smooth=False)
        mb.style(kind=3)
        for k in range(5):
            y = r.uniform(-0.6, hl - 0.2)
            side = r.choice((-1, 1))
            mb.oriented_box((side * r.uniform(hb, hw - 0.08), y, 0.06), (r.uniform(-1, 1), r.uniform(-1, 1), 0.2),
                            (0, 0, 1), (0.22, 0.11, 0.08), "Stone")
    if drift > 0:
        _drift(mb, r, 1, -hl * 0.85, hl * 0.95, hb - 0.03, hw, drift)
        _drift(mb, r, -1, -hl * 0.3, hl * 0.35, hb - 0.03, hw, drift * 0.5, n=4)


# -- B) awning -------------------------------------------------------------------


def _frond(mb, base, d, side, length, droop, width=0.35):
    """Palm frond lying on a roof and hanging over the edge (3-segment card strip)."""
    base, d, side = Vector(base), Vector(d).normalized(), Vector(side).normalized()
    prev = None
    segs = 3
    u0, u1 = CARD_PALM + 0.001, CARD_PALM + 0.999
    for k in range(segs + 1):
        t = k / segs
        pos = base + d * length * t - Vector((0, 0, droop * max(0.0, t - 0.45) ** 2 / 0.3025))
        w = width * math.sin(math.pi * (0.2 + 0.8 * t)) + 0.05
        cur = (pos - side * w, pos + side * w, t)
        if prev:
            pa, pb, pt = prev
            mb.quad_uv([pa, pb, cur[1], cur[0]], [(u0, pt), (u1, pt), (u1, t), (u0, t)], "Foliage")
        prev = cur


@env_part("awning")
def awning(model, name, p, box):
    """Shade awning: woven cloth on crooked wooden poles, or a palm-frond
    shelter.

    Parameters: `seed`; `style` ("cloth" or "palm", default "cloth");
    `poles` (poles per long side along Y, default 2); `sag` (cloth sag in m,
    default 0.12); `slope` (front edge lower than the back, m, default 0.25).
    The front is +X. Guy ropes run from the corner poles to pegs at the box
    corners.
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    style = p.get("style", "cloth")
    n = p.get("poles", 2)
    sag = p.get("sag", 0.12)
    slope = p.get("slope", 0.25)
    H, hw, hl = box.height, box.hw, box.hl
    inset = 0.3
    px = hw - inset
    py = hl - inset
    ys = [-py + 2 * py * k / (n - 1) for k in range(n)]

    def top_z(x):
        return H - 0.08 - slope * (x + px) / (2 * px)

    if style == "palm":
        # rough poles, two long beams, joists, frond thatch, fronds over the edges
        mb.style(kind=0)
        for s in (-1, 1):
            for y in ys:
                _pole(mb, r, (s * px, y, 0), (s * px, y, top_z(s * px) - 0.3), 0.08, crook=0.06)
            mb.style(kind=2)
            z = top_z(s * px) - 0.3
            mb.tube([(s * px, -hl + 0.1, z), (s * px, 0, z + 0.03), (s * px, hl - 0.1, z)], [0.09, 0.1, 0.085], 5,
                    "Wood")
            mb.style(kind=0)
        mb.style(kind=0)
        nj = max(3, int(box.length / 0.8))
        for k in range(nj):
            y = -hl + 0.3 + (box.length - 0.6) * k / (nj - 1)
            za, zb = top_z(-px) - 0.17, top_z(px) - 0.17
            mb.cylinder((-hw + 0.25, y, za + 0.02), (hw - 0.25, y, zb + 0.02), 0.05, seg=4, mat="Wood", smooth=False)
        # thatch: bundles of laid fronds running front to back, slightly uneven
        ns = max(6, int(box.length / 0.3))
        xa, xb = -hw + 0.12, hw - 0.12
        for k in range(ns):
            y = -hl + 0.15 + (box.length - 0.3) * (k + 0.5) / ns + r.uniform(-0.03, 0.03)
            dz = r.uniform(0.0, 0.05)
            xa_, xb_ = xa + r.uniform(0, 0.12), xb - r.uniform(0, 0.12)
            a3 = Vector((xa_, y, top_z(xa_) - 0.1 + dz))
            b3 = Vector((xb_, y + r.uniform(-0.06, 0.06), top_z(xb_) - 0.1 + dz))
            mb.style(kind=2)
            mb.oriented_box((a3 + b3) / 2, b3 - a3, (0, 1, r.uniform(-0.15, 0.15)),
                            ((b3 - a3).length, (box.length - 0.3) / ns * 1.25, 0.06), "Roof")
        # dry fronds lying on top and hanging over the front and back edges
        # (rigid: they are part of the roof, not a plant)
        mb.style(kind=2, wind=False)
        nf = max(4, int(box.length / 0.55))
        for s in (-1, 1):
            for k in range(nf):
                y = -hl + 0.3 + (box.length - 0.6) * (k + r.uniform(0.3, 0.7)) / nf
                x = s * (hw - 0.55)
                mb.style(phase=k + (10 if s > 0 else 30))
                _frond(mb, (x, y, top_z(x) - 0.02), (s, r.uniform(-0.3, 0.3), 0), (0, 1, 0),
                       r.uniform(0.6, 0.75), r.uniform(0.3, 0.5), width=0.2)
        for k in range(5):
            x = r.uniform(-hw + 0.6, hw - 1.4)
            y = r.uniform(-hl + 0.4, hl - 0.4)
            mb.style(phase=50 + k)
            _frond(mb, (x, y, top_z(x) - 0.02), (1, r.uniform(-0.4, 0.4), -slope / (2 * px)), (0, 1, 0),
                   r.uniform(0.8, 1.0), 0.0, width=0.2)
        return

    # cloth on poles
    mb.style(kind=0)
    for s in (-1, 1):
        for y in ys:
            _pole(mb, r, (s * px, y, 0), (s * px, y, top_z(s * px) + 0.06), 0.045)
            mb.style(kind=0)
            _rope(mb, (s * px, y, top_z(s * px) - 0.05), (s * px, y, top_z(s * px) + 0.0), 0.06)
    # guy ropes from the corner poles to pegs at the box corners
    for sx in (-1, 1):
        for sy in (-1, 1):
            a = (sx * px, sy * py, top_z(sx * px) + 0.02)
            b = (sx * (hw - 0.03), sy * (hl - 0.03), 0.02)
            _rope(mb, a, b, 0.01)
            mb.style(kind=2)
            mb.cylinder((b[0], b[1], 0.0), (b[0], b[1], 0.12), 0.025, seg=4, mat="Wood", smooth=False)
            mb.style(kind=0)
    cx, cy = px + 0.08, py + 0.1
    nx_, ny_ = 6, max(6, 3 * (n - 1) + 2)

    def cloth(u, v):
        x = -cx + 2 * cx * u
        y = -cy + 2 * cy * v
        # sag between the poles along Y and across X
        vv = (v * (n - 1)) % 1.0
        dz = sag * math.sin(math.pi * u) * 0.6 + sag * math.sin(math.pi * vv) * (0.4 + 0.6 * math.sin(math.pi * u))
        return (x, y, top_z(x) - dz)

    mb.style(kind=0)
    _sheet(mb, cloth, nx_, ny_, (0, 0, -0.02), "Canvas")

    # valance hanging down at the front edge
    def val(u, v):
        x, y, z = cloth(1.0, v)
        return (x + 0.01 * u, y, z - 0.28 * u + 0.01)

    _sheet(mb, val, 1, ny_, (-0.02, 0, 0), "Canvas")


# -- C) jars -----------------------------------------------------------------------


@env_part("jars")
def jars(model, name, p, box):
    """Unglazed clay storage jars (Plaster kind 3 ochre / 1 adobe).

    Parameters: `seed`; `arrangement` ("group": 5 jars of different sizes,
    one lying, and a goatskin water skin; "single"; "large": a big storage
    jar standing in a ring of stones; default "group").
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    arr = p.get("arrangement", "group")
    H, hw, hl = box.height, box.hw, box.hl
    if arr == "single":
        R = min(hw, hl)
        _jar(mb, (0, 0, 0), H, R / 1.0, seg=10)
        return
    if arr == "large":
        R = min(hw, hl) * 0.93
        mb.style(kind=0)
        ns = 9
        for k in range(ns):
            a = 2 * math.pi * k / ns + r.uniform(-0.1, 0.1)
            c = (math.cos(a) * (R * 0.62), math.sin(a) * (R * 0.62), 0)
            mb.blob(c, (0.13, 0.11, 0.1), "Stone", seg=5, rings=3, rng=r, rough=0.25, flat_bottom=0.0, smooth=False)
        _jar(mb, (0, 0, 0.06), H - 0.06, R, seg=12)
        return
    # group: (x, y, height, radius, kind)
    spec = [(-0.22, -0.4, 0.9, 0.3, 3), (0.28, -0.5, 0.7, 0.24, 1), (-0.28, 0.2, 0.6, 0.21, 3),
            (0.32, 0.0, 0.45, 0.17, 3), (0.05, -0.05, 0.38, 0.14, 1)]
    for x, y, h, R, k in spec:
        _jar(mb, (x + r.uniform(-0.02, 0.02), y + r.uniform(-0.02, 0.02), 0), h, R, kind=k)
    # one lying jar, mouth towards +Y
    R, h = 0.2, 0.62
    d = Vector((0.25, 1.0, 0.0)).normalized()
    base = Vector((0.22, 0.25, R * 0.97)) - d * 0.0
    _jar(mb, base, h, R, axis=d, kind=3)
    # goatskin water skin (dark leather, Wood kind 2) with a tied neck
    mb.style(kind=2)
    c = Vector((-0.3, 0.6, 0.0))
    mb.blob(c, (0.24, 0.17, 0.16), "Wood", seg=7, rings=4, rng=r, rough=0.12, flat_bottom=0.0)
    mb.cylinder(c + Vector((0.12, 0.1, 0.12)), c + Vector((0.2, 0.22, 0.2)), 0.04, seg=5, mat="Wood")
    _rope(mb, c + Vector((0.18, 0.19, 0.18)), c + Vector((0.2, 0.22, 0.2)), 0.05)
    for k in range(2):
        a = 2.2 + k * 1.6
        mb.cylinder(c + Vector((math.cos(a) * 0.18, math.sin(a) * 0.12, 0.06)),
                    c + Vector((math.cos(a) * 0.3, math.sin(a) * 0.2, 0.04)), 0.03, seg=4, mat="Wood")


# -- D) well -------------------------------------------------------------------------


@env_part("well_desert")
def well_desert(model, name, p, box):
    """Draw well: round plastered masonry curb on a stone apron, two wooden
    posts along Y with a cross beam, a wooden pulley, rope and a leather
    bucket.

    Parameters: `seed`; `curb` (curb diameter, default 1.6); `curb_h`
    (default 0.8); `kind` (curb Plaster kind, default 0 lime).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H, hw, hl = box.height, box.hw, box.hl
    D = p.get("curb", 1.6)
    ch = p.get("curb_h", 0.8)
    kind = p.get("kind", 0)
    R = D / 2
    seg = 14
    # stone apron
    mb.style(kind=0)
    mb.cylinder((0, 0, 0), (0, 0, 0.06), hw, seg=seg, mat="Stone", smooth=False, cap0=False)
    # plastered curb: outer wall, top, inner wall, dark water at the bottom
    mb.style(kind=kind)
    rings = []
    for rad, z in ((R - 0.04, 0.05), (R - 0.04, ch - 0.1), (R * 0.74, ch - 0.1), (R * 0.74, 0.3)):
        rings.append([(math.cos(2 * math.pi * k / seg) * rad, math.sin(2 * math.pi * k / seg) * rad, z)
                      for k in range(seg)])
    mb.loft(rings, "Plaster", cap_start=False, cap_end=True, cap_mat="Glass")
    # coping of dressed stones
    mb.style(kind=1)
    _annulus(mb, (0, 0, ch - 0.05), (0, 0, 1), R, R * 0.72, 0.1, seg, "Stone")
    for i in range(3):
        a = r.uniform(0, 2 * math.pi)
        z = r.uniform(0.25, ch - 0.3)
        c = Vector((math.cos(a), math.sin(a), 0)) * (R - 0.035)
        mb.style(kind=0)
        mb.oriented_box((c.x, c.y, z), (-math.sin(a), math.cos(a), 0), (0, 0, 1),
                        (r.uniform(0.25, 0.4), r.uniform(0.15, 0.25), 0.03), "Stone")
    # posts with stone packing, cross beam
    post_y = hl - 0.16
    top = H - 0.18
    for s in (-1, 1):
        mb.style(kind=0)
        _pole(mb, r, (0, s * post_y, 0), (0, s * post_y, top), 0.09, crook=0.03)
        mb.style(kind=0)
        for k in range(3):
            a = 2 * math.pi * k / 3 + r.uniform(0, 1)
            mb.blob((math.cos(a) * 0.12, s * post_y + math.sin(a) * 0.1, 0), (0.09, 0.08, 0.08), "Stone",
                    seg=5, rings=3, rng=r, rough=0.2, flat_bottom=0.0, smooth=False)
    mb.style(kind=2)
    mb.box((0, 0, top + 0.09), (0.18, box.length, 0.18), "Wood", bevel=0.02)
    # pulley in the XZ plane, hanging in a fork from the beam
    pz = top - 0.3
    pr = 0.17
    mb.style(kind=2)
    _annulus(mb, (0, 0, pz), (0, 1, 0), pr, pr * 0.45, 0.07, 10, "Wood")
    mb.cylinder((0, -0.07, pz), (0, 0.07, pz), 0.035, seg=5, mat="Wood")
    for s in (-1, 1):
        mb.box((0, s * 0.06, (pz + top) / 2), (0.06, 0.025, top - pz + 0.05), "Wood")
    # rope: over the wheel down to the bucket, other end tied to a post
    mb.style(kind=0)
    bz = ch + 0.55
    _rope(mb, (pr, 0, pz), (pr * 0.9, 0, bz + 0.38))
    _rope(mb, (-pr, 0, pz), (-0.05, -post_y + 0.09, 1.25))
    # bucket (leather on a wooden hoop) with a rope bail
    mb.style(kind=2)
    bx = pr * 0.9
    mb.cylinder((bx, 0, bz), (bx, 0, bz + 0.3), 0.13, 0.16, seg=8, mat="Wood")
    mb.style(kind=0)
    _annulus(mb, (bx, 0, bz + 0.29), (0, 0, 1), 0.17, 0.14, 0.04, 8, "Wood")
    for s in (-1, 1):
        _rope(mb, (bx + s * 0.15, 0, bz + 0.3), (bx, 0, bz + 0.38), 0.01)


# -- E) cart -------------------------------------------------------------------------


@env_part("cart_wood")
def cart_wood(model, name, p, box):
    """Two-wheeled wooden cart, unhitched: shafts forward (+Y) resting on a
    forked prop.

    Parameters: `seed`; `wheel` (wheel diameter, default 1.2); `spokes`
    (default 10); `bed` (bed length, default 2.1).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H, hw, hl = box.height, box.hw, box.hl
    D = p.get("wheel", 1.2)
    R = D / 2
    spokes = p.get("spokes", 10)
    bed_l = p.get("bed", 2.1)
    by0, by1 = -hl, -hl + bed_l
    ay = (by0 + by1) / 2
    bx = hw - 0.3
    floor_z = R + 0.14
    # frame beams continuing into the shafts
    mb.style(kind=0)
    tip_y = hl - 0.05
    tip_z = 0.5
    for s in (-1, 1):
        a = Vector((s * (bx - 0.08), by0 + 0.02, floor_z - 0.06))
        b = Vector((s * (bx - 0.08), by1, floor_z - 0.06))
        c = Vector((s * 0.32, tip_y, tip_z))
        mb.oriented_box((a + b) / 2, b - a, (0, 0, 1), ((b - a).length, 0.09, 0.1), "Wood")
        mb.oriented_box((b + c) / 2, c - b, (0, 0, 1), ((c - b).length + 0.05, 0.08, 0.08), "Wood")
    # floor, side boards on stakes, end boards
    mb.style(kind=0)
    mb.box((0, (by0 + by1) / 2, floor_z + 0.025), (2 * bx, bed_l, 0.05), "Wood")
    for s in (-1, 1):
        for z in (floor_z + 0.18, floor_z + 0.38):
            mb.style(kind=r.choice((0, 0, 2)))
            mb.box((s * (bx - 0.02), (by0 + by1) / 2, z), (0.04, bed_l, 0.15), "Wood")
        mb.style(kind=2)
        for k in range(3):
            y = by0 + 0.08 + (bed_l - 0.16) * k / 2
            mb.box((s * (bx + 0.02), y, (floor_z + H) / 2), (0.05, 0.06, H - floor_z), "Wood")
    mb.style(kind=0)
    for y in (by0 + 0.02, by1 - 0.02):
        mb.box((0, y, floor_z + 0.18), (2 * bx - 0.08, 0.04, 0.3), "Wood")
    # axle
    mb.style(kind=2)
    mb.box((0, ay, R), (2 * hw - 0.1, 0.09, 0.09), "Wood")
    mb.box((0, ay, (R + floor_z) / 2), (2 * bx - 0.2, 0.12, floor_z - R - 0.02), "Wood")
    # wheels
    for s in (-1, 1):
        x = s * (hw - 0.09)
        mb.style(kind=0)
        _annulus(mb, (x, ay, R), (1, 0, 0), R, R - 0.07, 0.08, 16, "Wood")
        mb.style(kind=2)
        mb.cylinder((s * (hw - 0.17), ay, R), (s * hw, ay, R), 0.1, 0.08, seg=8, mat="Wood")
        for k in range(spokes // 2):
            a = math.pi * k / (spokes // 2) + 0.2
            mb.style(kind=0)
            mb.oriented_box((x, ay, R), (0, math.cos(a), math.sin(a)), (1, 0, 0), (2 * R - 0.08, 0.04, 0.05),
                            "Wood")
    # cross bar between the shaft tips and a forked prop under it
    mb.style(kind=2)
    py = tip_y - 0.6
    pz = tip_z + (floor_z - 0.06 - tip_z) * 0.3 / (tip_y - by1)
    mb.box((0, py, pz - 0.06), (0.75, 0.06, 0.06), "Wood")
    _pole(mb, r, (0.0, py + 0.12, 0), (0, py, pz - 0.09), 0.04, crook=0.02)
    _pole(mb, r, (0.1, py - 0.1, 0), (0, py, pz - 0.09), 0.035, crook=0.02)


# -- F) market stall -------------------------------------------------------------


@env_part("market_stall")
def market_stall(model, name, p, box):
    """Market stall: wooden counter under a sloping cloth roof on four poles,
    customer side +X.

    Parameters: `seed`; `empty` (default false: bare counter, cloth partly
    torn and hanging); `goods` (baskets on the counter, default 3).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    empty = p.get("empty", False)
    goods = p.get("goods", 3)
    H, hw, hl = box.height, box.hw, box.hl
    px, py = hw - 0.15, hl - 0.12

    def top_z(x):
        return H - 0.05 - 0.38 * (x + px) / (2 * px)

    mb.style(kind=0)
    for sx in (-1, 1):
        for sy in (-1, 1):
            _pole(mb, r, (sx * px, sy * py, 0), (sx * px, sy * py, top_z(sx * px) + 0.04), 0.045, crook=0.02)
    # ridge poles carrying the cloth
    mb.style(kind=2)
    for sx in (-1, 1):
        z = top_z(sx * px) - 0.03
        mb.cylinder((sx * px, -hl + 0.05, z), (sx * px, hl - 0.05, z), 0.035, seg=4, mat="Wood", smooth=False)
    # counter
    cx0, cx1 = 0.05, hw - 0.08
    ctop = 0.85
    mb.style(kind=0)
    mb.box(((cx0 + cx1) / 2, 0, ctop - 0.025), (cx1 - cx0, 2 * py - 0.2, 0.05), "Wood")
    mb.style(kind=2)
    mb.box((cx1 - 0.015, 0, (ctop - 0.05) / 2 + 0.05), (0.03, 2 * py - 0.24, ctop - 0.1), "Wood")
    for sy in (-1, 1):
        mb.box((cx0 + 0.04, sy * (py - 0.16), (ctop - 0.05) / 2), (0.06, 0.06, ctop - 0.05), "Wood")
        mb.box(((cx0 + cx1) / 2, sy * (py - 0.13), (ctop - 0.05) / 2), (cx1 - cx0, 0.03, ctop - 0.05), "Wood")
    # cloth roof
    cx, cy = px + 0.13, py + 0.1
    skip = set()
    nx_, ny_ = 4, 6
    if empty:
        # ragged tear along the front edge and a hole further back
        nx_, ny_ = 7, 10
        depth = [0, 1, 2, 3, 2, 1, 0, 0, 0, 0]
        skip = {(nx_ - 1 - k, j) for j in range(ny_) for k in range(depth[j])}
        skip |= {(1, 7), (2, 7), (2, 8)}

    def cloth(u, v):
        x = -cx + 2 * cx * u
        y = -cy + 2 * cy * v
        dz = 0.07 * math.sin(math.pi * u) * math.sin(math.pi * v) + 0.03 * math.sin(math.pi * v * 3) ** 2
        return (x, y, top_z(x) - dz + 0.02)

    mb.style(kind=0)
    _sheet(mb, cloth, nx_, ny_, (0, 0, -0.02), "Canvas", skip=skip)

    def val(u, v):
        x, y, z = cloth(1.0, v)
        return (x + 0.005, y, z - 0.22 * u)

    if empty:
        # front valance mostly gone, a torn flap hangs down at one corner
        def flap(u, v):
            x, y, z = cloth(1.0 - 3 / nx_, 0.32 + 0.12 * v)
            return (x + 0.1 * u, y + 0.05 * u * v, z - 0.75 * u)

        _sheet(mb, flap, 2, 1, (-0.02, 0, 0), "Canvas")

        def val2(u, v):
            x, y, z = cloth(1.0, 0.72 + 0.28 * v)
            return (x + 0.005, y, z - 0.2 * u)

        _sheet(mb, val2, 1, 2, (-0.02, 0, 0), "Canvas")
        # an empty crate behind the counter
        mb.style(kind=2)
        mb.box((-0.45, 0.6, 0.18), (0.45, 0.6, 0.36), "Wood")
        return
    _sheet(mb, val, 1, ny_, (-0.02, 0, 0), "Canvas")
    # baskets with goods, small jars
    for k in range(goods):
        y = -py + 0.45 + (2 * py - 0.9) * k / max(1, goods - 1)
        x = (cx0 + cx1) / 2 + r.uniform(-0.05, 0.05)
        mb.style(kind=2)
        mb.revolve((x, y, ctop), (0, 0, 1), [(0.0, 0.17), (0.13, 0.23)], seg=8, mat="Roof")
        fill = [("Plaster", 3), ("Wood", 2), ("Plaster", 1)][k % 3]
        mb.style(kind=fill[1])
        mb.blob((x, y, ctop + 0.12), (0.2, 0.2, 0.08), fill[0], seg=7, rings=3, rng=r, rough=0.15)
    _jar(mb, ((cx0 + cx1) / 2 + 0.1, -py + 0.95 if goods < 3 else 0.2, ctop), 0.35, 0.12, seg=5, kind=3)
    # sacks and a jar on the ground beside the counter
    mb.style(kind=0)
    for k in range(2):
        mb.blob((r.uniform(-0.6, -0.3), -py + 0.35 + 0.45 * k, 0), (0.22, 0.2, 0.3), "Canvas", seg=5, rings=3, rng=r,
                rough=0.1, flat_bottom=0.0)
    _jar(mb, (-0.5, py - 0.35, 0), 0.6, 0.2, seg=6, kind=1)


# -- G / H) buildings ------------------------------------------------------------------


class _DHouse(House):
    """House with round-arched openings (`o.arch = True`) and barred windows
    (kind "barred")."""

    def wall(self, axis, coord, side, u0, u1, z0, z1, openings, t, top=None, frame=False):
        # `House.wall` handles one row of openings; stacked rows (storeys,
        # tower windows) are built as horizontal bands split between them
        rows = []
        for o in sorted(openings, key=lambda o: o.z0):
            if rows and o.z0 < rows[-1][1]:
                rows[-1][1] = max(rows[-1][1], o.z1)
                rows[-1][2].append(o)
            else:
                rows.append([o.z0, o.z1, [o]])
        cuts = [z0] + [(a[1] + b[0]) / 2 for a, b in zip(rows, rows[1:])] + [z1]
        for i, (za, zb) in enumerate(zip(cuts, cuts[1:])):
            super().wall(axis, coord, side, u0, u1, za, zb, rows[i][2] if rows else [], t, top, frame)
        mb = self.walls
        for o in openings:
            if top is not None and min((z for u, z in top if abs(u - o.u) < 0.9), default=z1) < o.z1 + 0.05:
                continue
            if getattr(o, "arch", False) and o.kind != "hole":
                self._arch(axis, coord, side, o, t)
            if o.kind == "barred" and self.damage < 2:
                self._bars(axis, coord, side, o, t)

    def slab(self, x0, x1, y0, y1, z, t=0.2, partial=None):
        """Floors of beaten earth / stone flags instead of planks."""
        self.floor.style(kind=1)
        self.floor.box(((x0 + x1) / 2, (y0 + y1) / 2, z + t / 2), (x1 - x0, y1 - y0, t), "Stone")

    def _P(self, axis, c, u, z):
        return (c, u, z) if axis == "x" else (u, c, z)

    def _arch(self, axis, coord, side, o, t):
        mb = self.walls
        mb.style(kind=self.wall_kind)
        rad = o.w / 2
        zs = o.z1 - rad
        d = (-side * t, 0, 0) if axis == "x" else (0, -side * t, 0)
        n = 5
        for half in (-1, 1):
            corner = (o.u + half * rad, o.z1)
            a0, a1 = (math.pi, math.pi / 2) if half < 0 else (0.0, math.pi / 2)
            pts = [corner] + [(o.u + rad * math.cos(a0 + (a1 - a0) * k / n), zs + rad * math.sin(a0 + (a1 - a0) * k / n))
                              for k in range(n + 1)]
            mb.extrude([self._P(axis, coord, u, z) for u, z in pts], d, self.wall_mat)

    def _bars(self, axis, coord, side, o, t):
        mb = self.walls
        mb.style(kind=0)
        g = coord - side * t * 0.6
        hz = o.z1 - o.z0
        zc = (o.z0 + o.z1) / 2
        mb.box(self._P(axis, g, o.u, zc), (0.03, o.w, hz) if axis == "x" else (o.w, 0.03, hz), "Glass")
        mb.style(kind=3)
        nb = max(2, int(o.w / 0.16))
        c = coord - side * 0.07
        for k in range(nb):
            u = o.lo + o.w * (k + 0.5) / nb
            mb.box(self._P(axis, c, u, zc), (0.03, 0.03, hz), "Metal")


def _arch(o):
    o.arch = True
    return o


def _walls4(h, x0, x1, y0, y1, z0, z1, ops, t, ruin_from=None):
    """Four walls of a rectangle like `House.rect_walls`; with `ruin_from`
    the jagged ruin top lies between ruin_from and z1."""
    for side, axis, coord, sgn, u0, u1 in (("+x", "x", x1, 1, y0, y1), ("-x", "x", x0, -1, y0, y1),
                                           ("+y", "y", y1, 1, x0 + t, x1 - t), ("-y", "y", y0, -1, x0 + t, x1 - t)):
        top = None
        if ruin_from is not None:
            a, b = (y0, y1) if axis == "x" else (x0, x1)
            top = h.ruin_top(ruin_from, z1, a - 1, b + 1)
        h.wall(axis, coord, sgn, u0, u1, z0, z1, ops.get(side, []), t, top)


def _slabs(h, x0, x1, y0, y1, z, t=0.25, tile=3.0, holes=0.0, kind=None):
    """Flat roof slab in tiles of about `tile` m; a share `holes` is missing."""
    if h.roof is None:
        return
    nx, ny = max(1, round((x1 - x0) / tile)), max(1, round((y1 - y0) / tile))
    h.roof.style(kind=h.wall_kind if kind is None else kind)
    for i in range(nx):
        for j in range(ny):
            if holes and h.r.random() < holes:
                continue
            xa, xb = x0 + (x1 - x0) * i / nx, x0 + (x1 - x0) * (i + 1) / nx
            ya, yb = y0 + (y1 - y0) * j / ny, y0 + (y1 - y0) * (j + 1) / ny
            h.roof.box(((xa + xb) / 2, (ya + yb) / 2, z - t / 2), (xb - xa, yb - ya, t), h.wall_mat)


def _coping(mb, x0, x1, y0, y1, z, t, over=0.05, hgt=0.1, mat="Plaster"):
    """Coping band on top of four walls of thickness t."""
    w = t + 2 * over
    mb.box(((x0 + x1) / 2, y0 + t / 2, z + hgt / 2), (x1 - x0 + 2 * over, w, hgt), mat)
    mb.box(((x0 + x1) / 2, y1 - t / 2, z + hgt / 2), (x1 - x0 + 2 * over, w, hgt), mat)
    mb.box((x0 + t / 2, (y0 + y1) / 2, z + hgt / 2), (w, y1 - y0 - 2 * t, hgt), mat)
    mb.box((x1 - t / 2, (y0 + y1) / 2, z + hgt / 2), (w, y1 - y0 - 2 * t, hgt), mat)


def _merlons(mb, r, axis, coord, side, u0, u1, z, t, hm, w=0.55, pitch=1.15, missing=0.0, mat="Plaster"):
    """Crenellation along a wall top: merlons at both ends and in between."""
    n = max(1, round((u1 - u0 - w) / pitch))
    c = coord - side * t / 2
    for k in range(n + 1):
        if missing and 0 < k < n and r.random() < missing:
            continue
        u = u0 + w / 2 + (u1 - u0 - w) * k / n
        center = (c, u, z + hm / 2) if axis == "x" else (u, c, z + hm / 2)
        size = (t, w, hm) if axis == "x" else (w, t, hm)
        mb.box(center, size, mat)


def _dome(mb, c, rad, rise, seg=16, rings=5, mat="Plaster", hole=None, thick=0.12):
    """Low dome (elliptic profile) over a circle; `hole` = set of (ring, seg)
    cells left open (cracked dome, then with an inner shell)."""
    c = Vector(c)
    hole = hole or set()

    def pt(i, j, s=1.0):
        a = (math.pi / 2) * i / rings
        rr = (rad - (0 if s == 1.0 else thick)) * math.cos(a)
        return c + Vector((rr * math.cos(2 * math.pi * j / seg), rr * math.sin(2 * math.pi * j / seg),
                           (rise - (0 if s == 1.0 else thick)) * math.sin(a)))

    shells = [1.0] + ([0.0] if hole else [])
    for s in shells:
        rows = [mb.verts([pt(i, j, s) for j in range(seg)]) for i in range(rings)]
        apex = mb.verts([c + Vector((0, 0, rise - (0 if s == 1.0 else thick)))])[0]
        for i in range(rings):
            for j in range(seg):
                if (i, j) in hole:
                    continue
                k = (j + 1) % seg
                vs = [rows[i][j], rows[i][k], rows[i + 1][k], rows[i + 1][j]] if i < rings - 1 else \
                    [rows[i][j], rows[i][k], apex]
                f = mb.face(vs, mat, smooth=True)
                f.normal_update()
                ctr = f.calc_center_median()
                out = (ctr - c).dot(f.normal) > 0
                if out != (s == 1.0):
                    f.normal_flip()


@env_part("mosque")
def mosque(model, name, p, box):
    """Small whitewashed village mosque: prayer hall with flat roof, plain
    parapet with corner blocks and a low dome on a drum, arched entrance
    vestibule on +X, small arched windows, a projecting niche on +Y and a
    square minaret at the +X/-Y corner with a gallery and a small domed
    lantern. No writing, symbols or meaningful ornament; plain knobs on the
    domes.

    Parameters: `seed`; `damage` (0 intact, 1 damaged: holes, dome cracked
    open, minaret top broken; 2 ruin: low jagged walls, minaret stump,
    rubble); `kind` (Plaster kind, default 2 whitewash); `hall_h` (top of
    the hall parapet, default 5.2); `minaret` (minaret side, default 3.0).
    """
    kind = p.get("kind", 2)
    h = _DHouse(model, name, p, box, "Plaster", kind)
    d = h.damage
    ruin = d >= 2
    H, hw, hl = box.height, box.hw, box.hl
    hall_h = p.get("hall_h", 5.2)
    ms = p.get("minaret", 3.0)
    plinth = 0.3
    t = 0.6
    go = 0.3  # minaret gallery overhang; the shaft stays inside the box by this much
    x0, x1 = -hw, hw - 2.0
    y0, y1 = -hl + go + ms, hl - 1.0
    xc, yc = (x0 + x1) / 2, (y0 + y1) / 2
    roof_z = hall_h - 0.5
    # floor
    h.plinth(x0, x1, y0, y1, plinth)
    h.plinth(x1, hw, yc - 1.5, yc + 1.5, plinth)
    h.slab(x0 + t, x1 - t, y0 + t, y1 - t, plinth - 0.02, t=0.05)
    # hall walls
    win = (0.7, plinth + 2.0, plinth + 3.3)

    def w(u):
        return _arch(Opening(u, win[0], win[1], win[2], "arch_window"))

    ops = {"+x": [_arch(Opening(yc, 1.4, plinth, plinth + 2.9, "door")), w(yc - 3.5), w(yc + 3.5)],
           "-x": [w(u) for u in _spread(y0 + 0.8, y1 - 0.8, 4, win[0])],
           "+y": [w(x0 + 1.6), w(x1 - 1.6)],
           "-y": [w(x0 + 2.0), w(x0 + 5.0)]}
    if d == 1:
        for o in (ops["-x"][1], ops["+y"][0], ops["-y"][1]):
            o.kind, o.arch = "hole", False
            o.w *= h.r.uniform(1.8, 2.4)
            o.z0 -= h.r.uniform(0.4, 0.9)
            o.z1 += h.r.uniform(0.2, 0.5)
        ops["-x"].append(Opening(ops["-x"][2].u + 1.2, 1.6, plinth + 0.6, plinth + 2.2, "hole"))
    zt = 3.6 if ruin else hall_h
    _walls4(h, x0, x1, y0, y1, plinth, zt, ops, t, ruin_from=plinth if ruin else None)
    # niche projection on +Y, entrance vestibule on +X
    nt = 0.45
    vy0, vy1 = yc - 1.5, yc + 1.5
    vh = 4.2
    vops = {"+x": [_arch(Opening(yc, 1.8, plinth, plinth + 3.2, "open"))]}
    for side, axis, coord, sgn, u0, u1, z1, o in (
            ("+y", "y", hl, 1, xc - 1.0, xc + 1.0, 4.0, []),
            ("+x", "x", xc + 1.0, 1, y1, hl, 4.0, []), ("-x", "x", xc - 1.0, -1, y1, hl, 4.0, []),
            ("+x", "x", hw, 1, vy0, vy1, vh, vops["+x"]),
            ("+y", "y", vy1, 1, x1 - 0.1, hw, vh, []), ("-y", "y", vy0, -1, x1 - 0.1, hw, vh, [])):
        top = h.ruin_top(plinth, min(z1, 3.0), u0 - 1, u1 + 1) if ruin else None
        h.wall(axis, coord, sgn, u0, u1, plinth, z1, o, nt, top)
    if not ruin:
        h.walls.style(kind=kind)
        h.plinth(xc - 1.0, xc + 1.0, y1, hl, plinth)
        # coping, corner blocks, vestibule and niche tops
        _coping(h.walls, x0, x1, y0, y1, hall_h, t)
        for sx in (x0, x1):
            for sy in (y0, y1):
                if d == 1 and sx == x0 and sy == y1:
                    continue
                cx_ = sx + (0.35 if sx == x0 else -0.35)
                cy_ = sy + (0.35 if sy == y0 else -0.35)
                h.walls.box((cx_, cy_, hall_h + 0.25), (0.8, 0.8, 0.5), "Plaster")
                h.walls.box((cx_, cy_, hall_h + 0.55), (0.55, 0.55, 0.12), "Plaster")
        h.walls.box(((x1 + hw) / 2, yc, vh + 0.05), (hw - x1 + 0.1, 3.1, 0.1), "Plaster")
        h.walls.box((xc, (y1 + hl) / 2, 4.05), (2.1, hl - y1 + 0.1, 0.1), "Plaster")
        # roofs: hall (tiles), vestibule, niche; drum and dome
        _slabs(h, x0 + t, x1 - t, y0 + t, y1 - t, roof_z, tile=3.0, holes=0.12 if d == 1 else 0.0)
        _slabs(h, x1, hw - nt, vy0 + nt, vy1 - nt, vh - 0.3, tile=3.0)
        _slabs(h, xc - 1.0 + nt, xc + 1.0 - nt, y1, hl - nt, 3.7, tile=3.0)
        mb = h.roof
        mb.style(kind=kind)
        dr = 2.1
        mb.cylinder((xc, yc, roof_z - 0.05), (xc, yc, roof_z + 0.85), dr, seg=16, mat="Plaster", smooth=False,
                    cap0=False, cap1=d == 1)
        mb.cylinder((xc, yc, roof_z + 0.85), (xc, yc, roof_z + 0.95), dr + 0.08, seg=16, mat="Plaster", smooth=False)
        hole = None
        if d == 1:
            j0 = h.r.randrange(16)
            hole = {(i, (j0 + k) % 16) for i in range(1, 5) for k in range(3 + (i > 2))} | {(4, j) for j in range(16)}
        _dome(mb, (xc, yc, roof_z + 0.95), dr, 1.6, hole=hole)
        if d == 0:
            mb.revolve((xc, yc, roof_z + 0.95 + 1.55), (0, 0, 1), [(0.0, 0.08), (0.12, 0.13), (0.26, 0.05),
                                                                   (0.3, 0.02)], seg=8, mat="Plaster")
        if d == 1:
            # broken roof beams sticking into the dome hole
            mb.style(kind=3)
            mb.oriented_box((xc + 0.3, yc, roof_z + 0.7), (1, 0.3, 0.4), (0, 0, 1), (2.2, 0.14, 0.14), "Wood")
    # minaret
    tower = model.mb(f"{name}_Tower", parent=name)
    walls = h.walls
    h.walls = tower
    tx0, tx1, ty0, ty1 = hw - go - ms, hw - go, -hl + go, -hl + go + ms
    tcx, tcy = (tx0 + tx1) / 2, (ty0 + ty1) / 2
    h.plinth(tx0, tx1, ty0, ty1, plinth)
    shaft = 11.6
    tops = {0: shaft, 1: shaft, 2: 5.0}[d]
    sw = 0.55
    tops_ops = {s: [] for s in ("+x", "-x", "+y", "-y")}
    for z in (3.6, 6.4, 9.2):
        for s, u in (("+x", tcy), ("-y", tcx), ("-x", tcy)):
            tops_ops[s].append(_arch(Opening(u, sw, z, z + 1.2, "arch_window")))
    tops_ops["+x"].append(_arch(Opening(tcy, 0.9, plinth, plinth + 2.2, "door")))
    if d == 1:
        _walls4(h, tx0, tx1, ty0, ty1, plinth, tops, tops_ops, 0.45, ruin_from=9.3)
    else:
        _walls4(h, tx0, tx1, ty0, ty1, plinth, tops, tops_ops, 0.45, ruin_from=2.5 if ruin else None)
    if d == 0:
        tower.style(kind=kind)
        tower.box((tcx, tcy, shaft - 0.15), (ms + 0.2, ms + 0.2, 0.3), "Plaster")
        tower.box((tcx, tcy, shaft + 0.1), (ms + 2 * go, ms + 2 * go, 0.2), "Plaster")
        g = ms / 2 + go - 0.08
        for s in (-1, 1):
            tower.box((tcx + s * g, tcy, shaft + 0.6), (0.16, 2 * g + 0.16, 0.8), "Plaster")
            tower.box((tcx, tcy + s * g, shaft + 0.6), (2 * g - 0.16, 0.16, 0.8), "Plaster")
        # lantern with one arched opening per side, cap slab, small dome and knob
        lw = 1.7
        lz0, lz1 = shaft + 0.2, shaft + 2.9
        lops = {s: [_arch(Opening(tcy if s[1] == "x" else tcx, 0.6, lz0 + 0.5, lz0 + 1.8, "open"))]
                for s in ("+x", "-x", "+y", "-y")}
        _walls4(h, tcx - lw / 2, tcx + lw / 2, tcy - lw / 2, tcy + lw / 2, lz0, lz1, lops, 0.3)
        tower.style(kind=kind)
        tower.box((tcx, tcy, lz1 + 0.08), (lw + 0.3, lw + 0.3, 0.16), "Plaster")
        tower.cylinder((tcx, tcy, lz1 + 0.16), (tcx, tcy, lz1 + 0.4), 0.72, seg=12, mat="Plaster", smooth=False)
        _dome(tower, (tcx, tcy, lz1 + 0.4), 0.72, 0.85, seg=12, rings=4)
        top_z = lz1 + 0.4 + 0.85
        tower.revolve((tcx, tcy, top_z - 0.04), (0, 0, 1), [(0.0, 0.06), (0.1, 0.12), (0.2, 0.11), (H - top_z + 0.04, 0.02)],
                      seg=6, mat="Plaster")
    elif d == 1:
        # half of the gallery slab survives
        tower.style(kind=kind)
        tower.box((tcx - 0.4, tcy, 9.6), (ms - 0.6, ms + 0.4, 0.2), "Plaster")
    h.walls = walls
    # debris
    if d == 1:
        h.rubble(x0 + 0.2, x0 + 0.5, y0 + 2, y1 - 2, 2, mats=("Plaster",), height=0.7, size=0.5)
        h.rubble(tx0 - 0.5, tx0, ty0 + 0.2, ty1, 1, mats=("Plaster",), height=0.6, size=0.5)
        h.rubble(xc - 1, xc + 1, yc - 1, yc + 1, 1, mats=("Plaster",), height=0.8, size=0.8)
    elif ruin:
        h.rubble(x0 + 1.5, x1 - 1.5, y0 + 1.5, y1 - 1.5, 4, mats=("Plaster",), height=1.5)
        h.rubble(tx0 - 0.8, tx1, ty0 + 0.5, ty1 + 0.5, 2, mats=("Plaster",), height=1.6, size=0.8)
        h.rubble(x0 + 0.3, x0 + 0.8, y0, y1, 2, mats=("Plaster",), height=0.8, size=0.6)
    h.finish()
    model.part_props.setdefault(name, {})["door_height_m"] = 2.2


@env_part("police_post")
def police_post(model, name, p, box):
    """Colonial-era police post / small fort: two-storey plastered block
    around a small courtyard, crenellated parapet, arched gateway with a
    closed wooden gate on +X, small barred windows, flat roof, square
    watchtower at the -X/-Y corner. No insignia, flags or writing.

    Parameters: `seed`; `damage` (0, 1, 2 as for houses); `kind` (Plaster
    kind, default 3 ochre); `tower` (tower side, default 3.5); `parapet`
    (top of the parapet wall below the merlons, default 6.0); `wing`
    (depth of the wings around the courtyard, default 3.5).
    """
    kind = p.get("kind", 3)
    h = _DHouse(model, name, p, box, "Plaster", kind)
    d = h.damage
    ruin = d >= 2
    r = h.r
    H, hw, hl = box.height, box.hw, box.hl
    ts = p.get("tower", 3.5)
    par = p.get("parapet", 6.0)
    wing = p.get("wing", 3.5)
    plinth = 0.3
    t = 0.5
    bx0, bx1, by0, by1 = -hw + 0.5, hw, -hl + 0.5, hl
    cx0, cx1, cy0, cy1 = bx0 + wing, bx1 - wing, by0 + wing, by1 - wing
    roof_z = par - 0.4
    gy = (cy0 + cy1) / 2
    h.plinth(bx0, bx1, by0, by1, plinth)
    h.floor.style(kind=3)
    h.floor.box(((cx0 + cx1) / 2, (cy0 + cy1) / 2, plinth + 0.01), (cx1 - cx0, cy1 - cy0, 0.03), "Ground")

    def bw(u, z, w=0.5, hh=0.6):
        return Opening(u, w, z, z + hh, "barred")

    g1, g2 = plinth + 1.6, plinth + 3.4
    ops = {"+x": [_arch(Opening(gy, 3.0, plinth, plinth + 4.0, "door")), bw(gy - 4.5, g1), bw(gy + 4.5, g1),
                  bw(gy - 5.5, g2, 0.7, 1.0), bw(gy - 3.0, g2, 0.7, 1.0), bw(gy + 3.0, g2, 0.7, 1.0),
                  bw(gy + 5.5, g2, 0.7, 1.0)],
           "-x": [bw(u, g1) for u in _spread(by0 + 3.5, by1, 3, 0.5)]
           + [bw(u, g2, 0.7, 1.0) for u in _spread(by0 + 3.5, by1, 4, 0.7)],
           "+y": [bw(u, g1) for u in _spread(bx0, bx1, 2, 0.5)] + [bw(u, g2, 0.7, 1.0) for u in _spread(bx0, bx1, 3, 0.7)],
           "-y": [bw(u, g1) for u in _spread(bx0 + 3.5, bx1, 1, 0.5)]
           + [bw(u, g2, 0.7, 1.0) for u in _spread(bx0 + 3.5, bx1, 2, 0.7)]}
    if d == 1:
        cand = ops["-x"] + ops["+y"]
        r.shuffle(cand)
        for o in cand[:4]:
            o.kind = "hole"
            o.w = min(o.w * r.uniform(1.8, 2.6), 2.0)
            o.z0 -= r.uniform(0.2, 0.6)
            o.z1 += r.uniform(0.2, 0.5)
        ops["+y"].append(Opening(bx0 + 2.0, 2.2, plinth + 2.2, plinth + 4.6, "hole"))
    zt = 4.6 if ruin else par
    _walls4(h, bx0, bx1, by0, by1, plinth, zt, ops, t, ruin_from=plinth if ruin else None)
    # courtyard walls (wing inner walls), the gate passage is open on the court side
    iops = {"+x": [_arch(Opening(gy, 2.4, plinth, plinth + 3.2, "open"))]
            + [Opening(u, 0.8, plinth + 1.5, plinth + 2.5, "window") for u in (cy0 + 1.2, cy1 - 1.2)],
            "-x": [Opening(gy, 1.0, plinth, plinth + 2.0, "door"), Opening(gy - 2.6, 0.8, g2, g2 + 1.1, "window"),
                   Opening(gy + 2.6, 0.8, g2, g2 + 1.1, "window")],
            "+y": [Opening((cx0 + cx1) / 2, 1.0, plinth, plinth + 2.0, "door")],
            "-y": [Opening((cx0 + cx1) / 2, 0.8, g2, g2 + 1.1, "window")]}
    _walls4(h, cx0, cx1, cy0, cy1, plinth, 4.0 if ruin else par - 0.1, iops, 0.4,
            ruin_from=plinth if ruin else None)
    if not ruin:
        # roofs of the four wings, parapet coping and merlons
        holes = 0.15 if d == 1 else 0.0
        _slabs(h, bx0 + t, cx0 + 0.4, by0 + t, by1 - t, roof_z, holes=holes)
        _slabs(h, cx1 - 0.4, bx1 - t, by0 + t, by1 - t, roof_z, holes=holes)
        _slabs(h, cx0 + 0.4, cx1 - 0.4, cy1 - 0.4, by1 - t, roof_z, holes=holes)
        _slabs(h, cx0 + 0.4, cx1 - 0.4, by0 + t, cy0 + 0.4, roof_z, holes=holes)
        mb = h.walls
        mb.style(kind=kind)
        _coping(mb, bx0, bx1, by0, by1, par, t, over=0.04, hgt=0.08)
        miss = 0.3 if d == 1 else 0.0
        for axis, coord, sgn, u0, u1 in (("x", bx1, 1, by0 - 0.04, by1 + 0.04), ("x", bx0, -1, by0 + 3.0, by1 + 0.04),
                                         ("y", by1, 1, bx0 - 0.04, bx1 + 0.04), ("y", by0, -1, bx0 + 3.0, bx1 + 0.04)):
            _merlons(mb, r, axis, coord, sgn, u0, u1, par + 0.08, t + 0.08, 0.42, missing=miss)
        # cornice band below the parapet
        _coping(mb, bx0, bx1, by0, by1, roof_z - 0.25, t, over=0.08, hgt=0.15)
        # outside stair in the courtyard up to the roof
        mb.style(kind=kind)
        n = 9
        for k in range(n):
            z1 = plinth + (roof_z - plinth) * (k + 1) / n
            y = cy1 - 0.4 - 0.32 * (n - k)
            mb.box((cx0 + 0.4 + 0.45, y + 0.16, (plinth + z1) / 2), (0.9, 0.34, z1 - plinth), "Plaster")
    # watchtower
    tower = model.mb(f"{name}_Tower", parent=name)
    walls = h.walls
    h.walls = tower
    tx0, tx1, ty0, ty1 = -hw + 0.08, -hw + 0.08 + ts, -hl + 0.08, -hl + 0.08 + ts
    tcx, tcy = (tx0 + tx1) / 2, (ty0 + ty1) / 2
    h.plinth(tx0, tx1, ty0, ty1, plinth)
    tz = H - 0.55
    tops = {0: tz, 1: tz, 2: 5.2}[d]
    tops_ops = {"-x": [bw(tcy, z, 0.3, 0.8) for z in (2.0, 4.0, 6.6)], "-y": [bw(tcx, z, 0.3, 0.8) for z in (2.0, 6.6)],
                "+x": [bw(tcy, 6.6, 0.3, 0.8)], "+y": [bw(tcx, 6.6, 0.3, 0.8)]}
    if d == 1:
        tops_ops["-x"].append(Opening(tcy + 0.6, 1.4, 6.0, 7.4, "hole"))
    _walls4(h, tx0, tx1, ty0, ty1, plinth, tops, tops_ops, 0.5,
            ruin_from=(6.8 if d == 1 else (1.5 if ruin else None)))
    if d == 0:
        tower.style(kind=kind)
        tower.box((tcx, tcy, tz - 0.55), (ts - 1.0, ts - 1.0, 0.25), "Plaster")
        _coping(tower, tx0, tx1, ty0, ty1, tz - 0.35, 0.5, over=0.08, hgt=0.18)
        for axis, coord, sgn, u0, u1 in (("x", tx1, 1, ty0, ty1), ("x", tx0, -1, ty0, ty1),
                                         ("y", ty1, 1, tx0 + 0.5, tx1 - 0.5), ("y", ty0, -1, tx0 + 0.5, tx1 - 0.5)):
            if axis == "x":
                _merlons(tower, r, axis, coord, sgn, u0, u1, tz, 0.5, H - tz, w=0.6, pitch=1.0)
            else:
                _merlons(tower, r, axis, coord, sgn, u0 + 0.5, u1 - 0.5, tz, 0.5, H - tz, w=0.5, pitch=1.2)
    h.walls = walls
    if d == 1:
        h.rubble(bx0 - 0.4, bx0, by0 + 4, by1 - 2, 2, mats=("Plaster",), height=0.7, size=0.5)
        h.rubble(bx0 + 1.0, bx0 + 3.0, by1 - 0.3, by1, 1, mats=("Plaster",), height=0.8, size=0.6)
    elif ruin:
        h.rubble(bx0 + 1, cx0, by0 + 3, by1 - 1, 3, mats=("Plaster",), height=1.4)
        h.rubble(cx1, bx1 - 1, by0 + 1, by1 - 1, 2, mats=("Plaster",), height=1.2)
        h.rubble(tx0, tx1 + 1.0, ty0 + 0.5, ty1 + 1.0, 2, mats=("Plaster",), height=1.6, size=0.8)
    h.finish()
    model.part_props.setdefault(name, {})["door_height_m"] = 2.0
