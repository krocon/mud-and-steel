"""Vegetation generators (prompt 007, part A): trees, bushes, grass, wood (rocks: rocks.py).

Each generator builds one variant into the target box (see `env.py`). Crowns
are clusters of crossed alpha cards (env_foliage.gdshader draws the leaves
procedurally, no textures). Trees can also build a `<Variant>_Far` mesh: a
handful of large crown cards plus a low trunk (<= 200 triangles) for distant
trees, switched by Godot visibility ranges (docs/ASSETS.md).

Plant faces carry the wind flag; the env bake turns it into wind weights
(trunk base 0, branch tips and leaves up to 1) and a phase per cluster.
"""

import math

from mathutils import Matrix, Vector

from .env import (CARD_BROADLEAF, CARD_FLOWERS, CARD_GRASS, CARD_NEEDLE, CARD_OLIVE, CARD_PALM, CARD_PINE_TUFT,
                  CARD_SHRUB, CARD_SUCCULENT, CARD_TWIGS, CARD_VINE, env_part, rng)

GOLDEN = math.pi * (3 - math.sqrt(5))
CARD_CROWN = 8  # far crown silhouette, leaves in object space
CARD_CROWN_CONIFER = 9


def _horiz(a):
    return Vector((math.cos(a), math.sin(a), 0.0))


def _rot(v, axis, ang):
    return Matrix.Rotation(ang, 3, axis) @ v


def _cluster(mb, center, size, r, pattern, phase, cards=3, droop=0.0):
    """2-3 crossed cards around `center` with random orientation."""
    a = r.uniform(0, math.pi)
    tilt = r.uniform(-0.35, 0.35)
    mb.style(phase=phase, wind=True)
    right = _horiz(a)
    up = Vector((0, 0, 1)) + _horiz(a + math.pi / 2) * tilt - Vector((0, 0, droop))
    s = size * r.uniform(0.8, 1.2)
    mb.card(center, right, up, s, s * r.uniform(0.85, 1.1), pattern)
    if cards >= 2:
        right2 = _horiz(a + math.pi / 2)
        up2 = Vector((0, 0, 1)) + _horiz(a) * tilt
        mb.card(center, right2, up2, s * 0.95, s, pattern)
    if cards >= 3:
        up3 = _horiz(a + math.pi / 2) + Vector((0, 0, r.uniform(0.15, 0.45)))
        mb.card(center + Vector((0, 0, s * 0.1)), right, up3, s, s * 0.9, pattern)


def _trunk(mb, base_r, top, r, seg=7, kind=0, lean=0.0, flare=1.5):
    """Slightly irregular trunk from the ground to `top` (a Vector)."""
    mb.style(kind=kind, phase=0, wind=True)
    n = 5
    a = r.uniform(0, 2 * math.pi)
    pts, radii = [], []
    for i in range(n):
        t = i / (n - 1)
        off = _horiz(a) * lean * t * t + _horiz(a + 1.7) * r.uniform(-0.04, 0.04) * top.z * 0.03
        pts.append(Vector((top.x * t, top.y * t, top.z * t)) + off * (1 if i else 0))
        radii.append(base_r * (flare if i == 0 else 1.0 - 0.55 * t))
    mb.tube(pts, radii, seg, "Bark", cap_start=False, cap_end=True)
    return pts, radii


def _branch(mb, start, direction, length, r0, r, seg=5, sag=0.15, phase=1, kind=0, tip_r=0.15):
    d = direction.normalized()
    mid = start + d * (length * 0.5) + Vector((0, 0, length * 0.06))
    end = start + d * length - Vector((0, 0, length * sag))
    mb.style(kind=kind, phase=phase, wind=True)
    mb.tube([start, mid, end], [r0, r0 * 0.6, r0 * tip_r], seg, "Bark", cap_start=False)
    return end


def _ellipsoid_point(r, c, rx, ry, rz, shell=0.55, up_bias=0.2):
    while True:
        d = Vector((r.uniform(-1, 1), r.uniform(-1, 1), r.uniform(-1, 1) + up_bias))
        if 0.05 < d.length <= 1.0 + up_bias:
            break
    d.normalize()
    f = shell + (1 - shell) * math.sqrt(r.random())
    return c + Vector((d.x * rx, d.y * ry, d.z * rz)) * f


def _far_crown(model, name, box, crown_base, trunk_r, kind, pattern=CARD_CROWN, layers=2):
    """Distant version: low trunk and crossed crown cards (~30 triangles)."""
    mb = model.mb(f"{name}_Far")
    mb.style(kind=kind, wind=True, phase=0)
    mb.cylinder((0, 0, 0), (0, 0, crown_base + (box.height - crown_base) * 0.4), trunk_r, trunk_r * 0.6,
                seg=4, mat="Bark", cap0=False)
    cz = (crown_base + box.height) / 2
    h = box.height - crown_base
    w = max(box.width, box.length)
    for i in range(3):
        a = i * math.pi / 3
        mb.style(phase=i + 1)
        mb.card((0, 0, cz), _horiz(a), (0, 0, 1), w, h, pattern)
    for j in range(layers):
        z = crown_base + h * (0.35 + 0.35 * j)
        mb.style(phase=10 + j)
        mb.card((0, 0, z), (1, 0, 0), (0, 1, 0.08), w * (0.9 - 0.2 * j), w * (0.9 - 0.2 * j), pattern)


@env_part("tree_broadleaf")
def tree_broadleaf(model, name, p, box):
    """Deciduous tree (oak, beech, birch): trunk, limbs, card-cluster crown.

    Parameters: `seed`; `crown_base` (fraction of the height where the crown
    starts, default 0.3); `clusters` (default 110); `limbs` (default 6);
    `trunk_radius` (m at breast height, default height x 0.022);
    `card_size` (fraction of the crown radius, default 0.45); `droop`
    (hanging twigs, birch 0.5, default 0); `bark_kind` (0 oak, 1 birch);
    `lean` (m, default 0.4); `pattern` (card pattern, default broadleaf);
    `far` (also build `<name>_Far`, default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    cb = H * p.get("crown_base", 0.3)
    rz = (H - cb) / 2
    c = Vector((0, 0, cb + rz))
    tr = p.get("trunk_radius", H * 0.022)
    kind = p.get("bark_kind", 0)
    droop = p.get("droop", 0.0)
    pattern = p.get("pattern", CARD_BROADLEAF)
    pts, _ = _trunk(mb, tr, Vector((0, 0, c.z + rz * 0.35)), r, kind=kind, lean=p.get("lean", 0.4))
    limbs = p.get("limbs", 6)
    for i in range(limbs):
        t = 0.45 + 0.5 * i / max(limbs - 1, 1)
        start = pts[1].lerp(pts[3], t)
        a = i * GOLDEN * 2 + r.uniform(-0.3, 0.3)
        d = _horiz(a) + Vector((0, 0, r.uniform(0.5, 1.1)))
        _branch(mb, start, d, R * r.uniform(0.6, 0.85), tr * 0.45, r, phase=i + 1, kind=kind, sag=0.05)
    n = p.get("clusters", 110)
    size = R * p.get("card_size", 0.45)
    for i in range(n):
        q = _ellipsoid_point(r, c, R * 0.82, R * 0.82, rz * 0.82)
        q.z = max(q.z, cb + size * 0.3)
        _cluster(mb, q, size, r, pattern, 100 + i, droop=droop)
    if p.get("far", True):
        _far_crown(model, name, box, cb, tr, kind)


@env_part("tree_bare")
def tree_bare(model, name, p, box):
    """Leafless deciduous tree (winter): trunk, three branch levels, twig cards.

    Parameters: `seed`; `crown_base` (default 0.3); `limbs` (default 6);
    `trunk_radius` (default height x 0.022); `far` (default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    cb = H * p.get("crown_base", 0.3)
    tr = p.get("trunk_radius", H * 0.022)
    top = Vector((0, 0, H * 0.72))
    pts, _ = _trunk(mb, tr, top, r, lean=0.3)
    phase = 1
    limbs = p.get("limbs", 6)
    for i in range(limbs):
        t = 0.35 + 0.65 * i / max(limbs - 1, 1)
        start = pts[1].lerp(pts[4], t * 0.9)
        a = i * GOLDEN * 2 + r.uniform(-0.3, 0.3)
        d1 = (_horiz(a) + Vector((0, 0, r.uniform(0.7, 1.3)))).normalized()
        l1 = R * r.uniform(0.55, 0.7)
        e1 = _branch(mb, start, d1, l1, tr * 0.42, r, phase=phase, sag=0.0, tip_r=0.45)
        phase += 1
        for j in range(2):
            a2 = a + (j - 0.5) * r.uniform(0.7, 1.2)
            d2 = (_horiz(a2) + Vector((0, 0, r.uniform(0.6, 1.4)))).normalized()
            s2 = start.lerp(e1, 0.55 + 0.45 * j)
            l2 = R * r.uniform(0.35, 0.5)
            e2 = _branch(mb, s2, d2, l2, tr * 0.18, r, seg=4, phase=phase, sag=0.02, tip_r=0.3)
            phase += 1
            for k in range(2):
                a3 = a2 + (k - 0.5) * 1.4
                d3 = (_horiz(a3) + Vector((0, 0, r.uniform(0.5, 1.5)))).normalized()
                s3 = s2.lerp(e2, 0.5 + 0.5 * k)
                e3 = _branch(mb, s3, d3, R * r.uniform(0.18, 0.28), tr * 0.07, r, seg=3, phase=phase,
                             sag=0.0, tip_r=0.3)
                _cluster(mb, e3, R * 0.35, r, CARD_TWIGS, phase, cards=2)
                phase += 1
    if p.get("far", True):
        _far_crown(model, name, box, cb, tr, 0, pattern=CARD_TWIGS, layers=1)


@env_part("tree_conifer")
def tree_conifer(model, name, p, box):
    """Conifer: `shape = "spruce"` (cone of drooping whorls) or `"pine"`
    (tall bare trunk, flat irregular crown at the top).

    Parameters: `seed`; `shape`; `crown_base` (default spruce 0.35,
    pine 0.65); `whorls` (spruce, default 22); `per_whorl` (default 6);
    `clusters` (pine, default 70); `trunk_radius` (default height x 0.012);
    `far` (default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    shape = p.get("shape", "spruce")
    tr = p.get("trunk_radius", H * 0.012)
    if shape == "spruce":
        cb = H * p.get("crown_base", 0.35)
        mb.style(kind=0, wind=True, phase=0)
        mb.tube([(0, 0, 0), (0, 0, H * 0.3), (0, 0, H * 0.7), (0, 0, H)], [tr * 1.4, tr, tr * 0.55, tr * 0.08],
                7, "Bark", cap_start=False)
        whorls = p.get("whorls", 22)
        per = p.get("per_whorl", 6)
        for i in range(whorls):
            t = i / (whorls - 1)
            z = cb + (H * 0.97 - cb) * t
            rad = R * max(0.12, (1 - t) ** 0.95)
            a0 = r.uniform(0, 2 * math.pi)
            for k in range(per):
                a = a0 + 2 * math.pi * k / per
                d = _horiz(a)
                mid = Vector((0, 0, z)) + d * rad * 0.5 - Vector((0, 0, rad * 0.12))
                side = _horiz(a + math.pi / 2)
                mb.style(phase=i * per + k + 1, wind=True)
                mb.card(mid, d - Vector((0, 0, 0.25)), side + Vector((0, 0, 0.2)), rad, rad * 0.7, CARD_NEEDLE)
                mb.card(mid, d - Vector((0, 0, 0.25)), Vector((0, 0, 1)), rad, rad * 0.45, CARD_NEEDLE)
        # vertical fill cards along the crown
        for k in range(2):
            mb.style(phase=500 + k)
            mb.card((0, 0, cb), _horiz(k * math.pi / 2 + 0.3), (0, 0, 1), R * 0.9, H - cb, CARD_NEEDLE,
                    anchor="bottom")
        pattern = CARD_CROWN_CONIFER
    else:
        cb = H * p.get("crown_base", 0.65)
        rz = (H - cb) / 2
        c = Vector((0, 0, cb + rz))
        pts, _ = _trunk(mb, tr, Vector((0, 0, c.z + rz * 0.4)), r, kind=3, lean=H * 0.03, flare=1.3)
        for i in range(5):
            start = pts[3].lerp(pts[4], i / 4)
            a = i * GOLDEN * 2
            _branch(mb, start, _horiz(a) + Vector((0, 0, 0.6)), R * 0.75, tr * 0.35, r, phase=i + 1, kind=3,
                    sag=0.02)
        for i in range(p.get("clusters", 70)):
            q = _ellipsoid_point(r, c, R * 0.85, R * 0.85, rz * 0.7, up_bias=0.0)
            _cluster(mb, q, R * 0.5, r, CARD_NEEDLE, 100 + i)
        pattern = CARD_CROWN
    if p.get("far", True):
        _far_crown(model, name, box, cb, tr * 1.2, 3 if shape == "pine" else 0, pattern=pattern)


@env_part("tree_palm")
def tree_palm(model, name, p, box):
    """Date palm: curved ringed trunk, crown of drooping frond ribbons.

    Parameters: `seed`; `fronds` (default 18); `dead_fronds` (hanging dry
    fronds, default 5); `curve` (trunk bend in m, default height x 0.06);
    `trunk_radius` (default 0.22); `far` (default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    tr = p.get("trunk_radius", 0.22)
    bend = p.get("curve", H * 0.06)
    a = r.uniform(0, 2 * math.pi)
    top_z = H - R * 0.25
    pts, radii = [], []
    n = 9
    for i in range(n):
        t = i / (n - 1)
        pts.append(_horiz(a) * bend * t * t + Vector((0, 0, top_z * t)))
        radii.append(tr * (1.35 if i == 0 else (1.0 - 0.15 * t) * (1.06 if i % 2 else 1.0)))
    mb.style(kind=2, wind=True, phase=0)
    mb.tube(pts, radii, 8, "Bark", cap_start=False)
    top = pts[-1]
    fronds = p.get("fronds", 18)
    dead = p.get("dead_fronds", 5)
    for i in range(fronds + dead):
        is_dead = i >= fronds
        ang = i * GOLDEN * 2
        length = R * (r.uniform(0.85, 1.05) if not is_dead else 0.7)
        lift = r.uniform(0.2, 0.9) if not is_dead else -1.6
        d = _horiz(ang)
        side = _horiz(ang + math.pi / 2)
        segs = 4
        mb.style(kind=2 if is_dead else 0, phase=10 + i, wind=True)
        prev = None
        for k in range(segs + 1):
            t = k / segs
            # arc: up at the base, drooping towards the tip
            pos = top + d * length * t + Vector((0, 0, length * (lift * t - 0.9 * t * t) * 0.6))
            w = length * 0.22 * math.sin(math.pi * (0.15 + 0.85 * t)) + 0.05
            cur = (pos - side * w, pos + side * w, t)
            if prev:
                pa, pb, pt = prev
                u0, u1 = CARD_PALM + 0.001, CARD_PALM + 0.999
                mb.quad_uv([pa, pb, cur[1], cur[0]], [(u0, pt), (u1, pt), (u1, t), (u0, t)], "Foliage")
            prev = cur
    if p.get("far", True):
        far = model.mb(f"{name}_Far")
        far.style(kind=2, wind=True)
        far.tube([pts[0], pts[4], pts[-1]], [tr * 1.2, tr, tr * 0.9], 4, "Bark", cap_start=False)
        for i in range(4):
            ang = i * math.pi / 2 + 0.4
            far.style(phase=10 + i)
            d = _horiz(ang)
            far.card(top + d * R * 0.5 - Vector((0, 0, R * 0.12)), d - Vector((0, 0, 0.35)),
                     _horiz(ang + math.pi / 2), R, R * 0.45, CARD_PALM)


# -- Mediterranean trees (prompt 120) -------------------------------------------


@env_part("tree_stone_pine")
def tree_stone_pine(model, name, p, box):
    """Stone pine (Pinus pinea): tall bare trunk that forks into a few limbs
    under a flat, wide umbrella crown of needle clusters.

    Parameters: `seed`; `crown_base` (fraction of the height where the
    crown's underside starts, default 0.8); `fork` (fraction of the height
    where the trunk forks, default 0.5); `limbs` (default 4); `clusters`
    (default 95); `lean` (m the trunk leans at the crown, default height x
    0.03); `trunk_radius` (default height x 0.022); `far` (default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    cb = H * p.get("crown_base", 0.8)
    fork_z = H * p.get("fork", 0.5)
    tr = p.get("trunk_radius", H * 0.022)
    lean = p.get("lean", H * 0.03)
    a = r.uniform(0, 2 * math.pi)
    top = _horiz(a) * lean + Vector((0, 0, fork_z))
    mb.style(kind=3, phase=0, wind=True)
    pts = [Vector((0, 0, 0)), _horiz(a) * lean * 0.15 + Vector((0, 0, fork_z * 0.35)),
           _horiz(a) * lean * 0.5 + Vector((0, 0, fork_z * 0.7)), top]
    mb.tube(pts, [tr * 1.3, tr, tr * 0.85, tr * 0.7], 7, "Bark", cap_start=False)
    shift = _horiz(a) * lean * 1.2
    limbs = p.get("limbs", 4)
    ends = []
    for i in range(limbs):
        b = a + i * 2 * math.pi / limbs + r.uniform(-0.4, 0.4)
        end = shift + _horiz(b) * R * r.uniform(0.55, 0.78) + Vector((0, 0, cb + (H - cb) * 0.15))
        mid = top.lerp(end, 0.5) + Vector((0, 0, (H - cb) * 0.1))
        mb.style(kind=3, phase=i + 1, wind=True)
        mb.tube([top, mid, end], [tr * 0.6, tr * 0.45, tr * 0.28], 5, "Bark", cap_start=False)
        ends.append(end)
    # Umbrella (sharpened in prompt 202): a flat underside at `cb`, a low
    # dome above it that is widest at the bottom, built from round needle
    # tufts; twigs run from the limbs out to the rim, so the bare branch
    # fan under the roof shows. Flat cards close the roof seen from above.
    rz = H - cb
    size = R * 0.36
    for i, end in enumerate(ends):
        for s in (-1, 1):
            b = math.atan2(end.y - shift.y, end.x - shift.x) + s * 0.45 + r.uniform(-0.15, 0.15)
            tip = shift + _horiz(b) * R * r.uniform(0.8, 0.92) + Vector((0, 0, cb + rz * 0.25))
            mb.style(kind=3, phase=20 + i, wind=True)
            mb.tube([end, end.lerp(tip, 0.5) + Vector((0, 0, rz * 0.08)), tip], [tr * 0.2, tr * 0.14, tr * 0.06], 3,
                    "Bark", cap_start=False)
    mb.style(kind=1)
    n = p.get("clusters", 95)
    for i in range(n):
        # even over the disc (golden angle), a little denser at the rim
        f = math.sqrt((i + 0.5) / n) ** 0.85
        b = i * GOLDEN + r.uniform(-0.2, 0.2)
        dome = math.sqrt(max(0.0, 1.0 - f * f))
        z = cb + size * 0.3 + (rz - size * 0.75) * dome * r.uniform(0.25, 1.0)
        q = shift + _horiz(b) * (R - size * 0.45) * f + Vector((0, 0, z))
        _cluster(mb, q, size, r, CARD_PINE_TUFT, 100 + i)
    mb.style(kind=1, wind=True)
    for j in range(6):
        b = j * math.pi / 3 + r.uniform(-0.3, 0.3)
        q = shift + _horiz(b) * R * 0.42 + Vector((0, 0, cb + rz * 0.5))
        mb.style(phase=300 + j)
        mb.card(q, _horiz(b + math.pi / 2), _horiz(b) + Vector((0, 0, 0.08)), R * 0.95, R * 0.9, CARD_PINE_TUFT)
    if p.get("far", True):
        _far_crown(model, name, box, cb, tr, 3, layers=2)


@env_part("tree_cypress")
def tree_cypress(model, name, p, box):
    """Italian cypress (Cupressus sempervirens 'Stricta'): a slim column of
    upright needle sprays from the ground to the pointed tip.

    Parameters: `seed`; `clusters` (default 80); `foot` (fraction of the
    height of bare trunk at the foot, default 0.04); `widest` (fraction of
    the height where the column is widest, default 0.3); `far` (default
    true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    foot = H * p.get("foot", 0.04)
    widest = p.get("widest", 0.3)
    tr = H * 0.012
    mb.style(kind=3, wind=True, phase=0)
    mb.tube([(0, 0, 0), (0, 0, H * 0.4), (0, 0, H * 0.85)], [tr * 1.4, tr, tr * 0.4], 6, "Bark", cap_start=False)

    def radius(t):
        # Widest low on the column, tapering to a point at the top.
        if t < widest:
            return R * (0.75 + 0.25 * t / widest)
        return R * max(0.06, ((1 - t) / (1 - widest)) ** 0.8)

    n = p.get("clusters", 80)
    for i in range(n):
        t = (i + r.uniform(0, 1)) / n
        z = foot + (H - foot) * t
        rad = radius(t) * r.uniform(0.45, 0.95)
        b = i * GOLDEN * 2
        q = Vector((0, 0, z)) + _horiz(b) * rad * 0.6
        size = max(radius(t) * 1.1, 0.4)
        mb.style(kind=1, phase=100 + i, wind=True)
        # Upright sprays: crossed cards standing along the column.
        mb.card(q, _horiz(b + math.pi / 2), Vector((0, 0, 1)) + _horiz(b) * 0.15, size, size * 1.6, CARD_NEEDLE)
        mb.card(q, _horiz(b), Vector((0, 0, 1)) - _horiz(b + math.pi / 2) * 0.15, size * 0.8, size * 1.4,
                CARD_NEEDLE)
    # Full-height fill cards close the column.
    for k in range(3):
        b = k * math.pi / 3 + 0.2
        mb.style(kind=1, phase=500 + k)
        mb.card((0, 0, foot), _horiz(b), (0, 0, 1), R * 1.5, H - foot, CARD_NEEDLE, anchor="bottom")
    if p.get("far", True):
        _far_crown(model, name, box, foot, tr * 1.2, 3, pattern=CARD_CROWN_CONIFER, layers=0)


@env_part("tree_olive")
def tree_olive(model, name, p, box):
    """Olive tree (Olea europaea): short, gnarled trunk that splits into two
    or three stems, a loose, open crown of small silver-grey leaves.

    Parameters: `seed`; `stems` (default 3); `crown_base` (default 0.38);
    `clusters` (default 70); `trunk_radius` (default height x 0.05);
    `twist` (gnarl of the stems in m, default 0.25); `far` (default true).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    R = min(box.hw, box.hl)
    cb = H * p.get("crown_base", 0.38)
    tr = p.get("trunk_radius", H * 0.05)
    twist = p.get("twist", 0.25)
    split = cb * 0.55
    mb.style(kind=0, phase=0, wind=True)
    base = [Vector((0, 0, 0)), Vector((r.uniform(-1, 1) * twist * 0.3, r.uniform(-1, 1) * twist * 0.3, split * 0.5)),
            Vector((r.uniform(-1, 1) * twist * 0.4, r.uniform(-1, 1) * twist * 0.4, split))]
    mb.tube(base, [tr * 1.5, tr * 1.1, tr], 8, "Bark", cap_start=False)
    stems = p.get("stems", 3)
    a0 = r.uniform(0, 2 * math.pi)
    tips = []
    for i in range(stems):
        b = a0 + i * 2 * math.pi / stems + r.uniform(-0.3, 0.3)
        out = _horiz(b)
        pts = [base[-1]]
        for k in range(1, 4):
            t = k / 3
            wob = _horiz(b + math.pi / 2) * r.uniform(-1, 1) * twist
            pts.append(base[-1] + out * R * 0.45 * t + Vector((0, 0, (H * 0.62 - split) * t)) + wob * (1 - t))
        mb.style(kind=0, phase=i + 1, wind=True)
        mb.tube(pts, [tr * 0.75, tr * 0.55, tr * 0.35, tr * 0.15], 6, "Bark", cap_start=False)
        tips.append(pts[-1])
        # one side branch per stem
        s0 = pts[1].lerp(pts[2], 0.5)
        _branch(mb, s0, _horiz(b + r.uniform(-0.8, 0.8)) + Vector((0, 0, 0.5)), R * 0.5, tr * 0.25, r,
                phase=10 + i, sag=0.08)
    # Loose crown: clusters in lumps around the stem tips, gaps between.
    rz = (H - cb) / 2
    size = R * 0.36
    n = p.get("clusters", 70)
    for i in range(n):
        tip = tips[i % len(tips)]
        c = Vector((tip.x * 0.8, tip.y * 0.8, cb + rz))
        q = _ellipsoid_point(r, c, R * 0.55, R * 0.55, rz * 0.8, shell=0.3, up_bias=0.1)
        q.z = max(q.z, cb + size * 0.2)
        _cluster(mb, q, size, r, CARD_OLIVE, 100 + i, cards=2, droop=0.15)
    if p.get("far", True):
        _far_crown(model, name, box, cb, tr, 0, pattern=CARD_OLIVE, layers=2)


@env_part("bush")
def bush(model, name, p, box):
    """Shrub: a few stems and a dome of crossed card clusters.

    Parameters: `seed`; `clusters` (default 40); `stems` (default 5);
    `pattern` (card pattern, default shrub); `center` (crown center height
    as a fraction of the height, default 0.55); `leaf_kind` (Foliage kind,
    default 0 leaf; prompt 120: 1 for dark evergreen macchia); `upright`
    (cards stand upright like rods, broom; default false); `lift` (lowest
    cluster center as a fraction of the cluster size, default 0.45; prompt
    202: higher for low, wide scrub so no card reaches the ground).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    cz = H * p.get("center", 0.55)
    mb.style(kind=0, wind=True, phase=0)
    for i in range(p.get("stems", 5)):
        a = i * GOLDEN * 2
        top = Vector((0, 0, cz)) + _horiz(a) * box.hw * 0.35
        mb.tube([Vector((0, 0, 0)) + _horiz(a) * 0.05, top], [0.03, 0.012], 3, "Bark", cap_start=False)
    size = min(box.hw, box.hl) * 0.7
    pattern = p.get("pattern", CARD_SHRUB)
    leaf_kind = p.get("leaf_kind", 0)
    upright = p.get("upright", False)
    for i in range(p.get("clusters", 40)):
        q = _ellipsoid_point(r, Vector((0, 0, cz)), box.hw * 0.75, box.hl * 0.75, (H - cz) * 0.75, shell=0.4)
        q.z = max(q.z, size * p.get("lift", 0.45))
        if upright:
            b = r.uniform(0, math.pi)
            mb.style(kind=leaf_kind, phase=100 + i, wind=True)
            mb.card(q, _horiz(b), Vector((0, 0, 1)) + _horiz(b + math.pi / 2) * 0.2, size * 0.7, size * 1.5, pattern)
            mb.card(q, _horiz(b + math.pi / 2), Vector((0, 0, 1)) - _horiz(b) * 0.2, size * 0.7, size * 1.4,
                    pattern)
            continue
        mb.style(kind=leaf_kind)
        _cluster(mb, q, size, r, pattern, 100 + i, cards=2)


def _solid(mb, first, pattern=CARD_SUCCULENT):
    """Faces created since index `first` become solid foliage (no alpha)."""
    mb.bm.faces.ensure_lookup_table()
    for f in list(mb.bm.faces)[first:]:
        if f[mb.uvflag]:
            continue
        f[mb.uvflag] = 1
        for loop in f.loops:
            co = loop.vert.co
            loop[mb.uv].uv = (pattern + 0.02 + ((co.x + co.y) * 0.7) % 0.96, (co.z * 0.7) % 1.0)


@env_part("prickly_pear")
def prickly_pear(model, name, p, box):
    """Prickly pear (Opuntia ficus-indica, prompt 202): flat oval pads that
    grow out of each other's rims into a broad clump; solid, no wind.

    Parameters: `seed`; `stems` (pads standing on the ground, default 4);
    `pads` (all pads, default 16); `pad` ([width, height] of a full-grown
    pad as fractions of the clump's height, default [0.2, 0.3]: the clump
    is three to four pads high; real pads are smaller, the model keeps
    them readable from the game camera); `high` (pads above each other,
    default 3); `fruit` (red fruits on the rims, default 6).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    pw, ph = p.get("pad", [0.2, 0.3])
    pw, ph = pw * box.height, ph * box.height
    mb.style(kind=0, wind=False, phase=0)
    first = len(mb.bm.faces)
    tips = []  # (top point, yaw, lean axis angle, generation)

    def pad(base, yaw, tilt, scale):
        w, h = pw * scale * r.uniform(0.85, 1.1), ph * scale * r.uniform(0.85, 1.15)
        faces = mb.blob((0, 0, h / 2), (w / 2, 0.07 * w, h / 2), "Foliage", seg=6, rings=3, rng=r, rough=0.06)
        m = (Matrix.Translation(base) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Rotation(tilt, 4, "Y")
             @ Matrix.Rotation(r.uniform(-0.35, 0.35), 4, "X"))
        mb.transform_faces(faces, m)
        return m @ Vector((0, 0, h * 0.92)), m

    stems = p.get("stems", 4)
    R = min(box.hw, box.hl)
    for i in range(stems):
        a = i * 2 * math.pi / stems + r.uniform(-0.5, 0.5)
        base = _horiz(a) * r.uniform(0.25, 0.7) * R
        top, m = pad(base - Vector((0, 0, 0.05)), a + math.pi / 2 + r.uniform(-0.6, 0.6), r.uniform(-0.25, 0.25), 1.25)
        tips.append((top, a + math.pi / 2, 1))
    k = 0
    for j in range(p.get("pads", 16) - stems):
        # the next pad grows on the oldest pad that is not yet three high
        while tips[k % len(tips)][2] > p.get("high", 3):
            k += 1
        top, a, gen = tips[k % len(tips)]
        if j % 2:
            k += 1
        yaw = a + r.uniform(-0.7, 0.7)
        tip, m = pad(top - Vector((0, 0, 0.05)), yaw, r.uniform(0.3, 0.9) * (1 if j % 2 else -1),
                     max(0.65, 1.1 - 0.13 * gen))
        tips.append((tip, yaw, gen + 1))
    _solid(mb, first)
    # fruits: small red-orange knobs on the youngest pads (flower pattern
    # cell would need alpha; kind 3 of the succulent cell is the fruit red)
    mb.style(kind=3, wind=False)
    first = len(mb.bm.faces)
    for i in range(p.get("fruit", 6)):
        top = tips[-1 - i % len(tips)][0]
        mb.blob(top + Vector((r.uniform(-0.06, 0.06), r.uniform(-0.06, 0.06), 0.02)), (0.06, 0.06, 0.085), "Foliage",
                seg=4, rings=2, rng=r, rough=0.0)
    _solid(mb, first)


@env_part("agave")
def agave(model, name, p, box):
    """Agave (Agave americana, prompt 202): a rosette of long, stiff,
    grey-green leaves with a V-shaped cross-section; the inner ones stand
    upright, the outer ones arch down to the ground. Solid, no wind.

    Parameters: `seed`; `leaves` (default 16); `stalk` (height of a dry
    flower stalk in m, 0 = none, default 0).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    n = p.get("leaves", 16)
    R = min(box.hw, box.hl)
    mb.style(kind=1, wind=False, phase=0)
    first = len(mb.bm.faces)
    for i in range(n):
        t = i / max(n - 1, 1)  # 0 = innermost
        a = i * GOLDEN + r.uniform(-0.15, 0.15)
        out = _horiz(a)
        side = _horiz(a + math.pi / 2)
        elev = math.radians(78 - 62 * t + r.uniform(-5, 5))
        length = R * (0.75 + 0.45 * t) * r.uniform(0.9, 1.05)
        wmax = R * 0.17
        droop = 0.15 + 0.75 * t
        rows = []
        steps = 4
        pos = out * 0.06 + Vector((0, 0, 0.05))
        for s in range(steps + 1):
            f = s / steps
            ang = elev - droop * f * f * 1.4
            w = wmax * (0.75 + 0.5 * f) * (1 - f) ** 0.6 if s < steps else 0.0
            if s == 0:
                w = wmax * 0.7
            d = out * math.cos(ang) + Vector((0, 0, math.sin(ang)))
            nrm = out * -math.sin(ang) + Vector((0, 0, math.cos(ang)))
            rows.append((pos.copy(), w, nrm))
            pos = pos + d * (length / steps)
        for (p0, w0, n0), (p1, w1, n1) in zip(rows, rows[1:]):
            l0, r0 = p0 - side * w0 + n0 * w0 * 0.45, p0 + side * w0 + n0 * w0 * 0.45
            if w1 > 0:
                l1, r1 = p1 - side * w1 + n1 * w1 * 0.45, p1 + side * w1 + n1 * w1 * 0.45
                mb.face(mb.verts([l0, p0, p1, l1]), "Foliage", True)
                mb.face(mb.verts([p0, r0, r1, p1]), "Foliage", True)
            else:
                mb.face(mb.verts([l0, p0, p1]), "Foliage", True)
                mb.face(mb.verts([p0, r0, p1]), "Foliage", True)
    _solid(mb, first)
    stalk = p.get("stalk", 0)
    if stalk:
        mb.style(kind=2, wind=False)
        first = len(mb.bm.faces)
        mb.tube([Vector((0, 0, 0.2)), Vector((0.05, 0, stalk * 0.6)), Vector((0.1, 0.05, stalk))], [0.06, 0.045, 0.02], 5,
                "Foliage", cap_start=False)
        for j in range(5):
            z = stalk * (0.62 + 0.08 * j)
            b = j * GOLDEN
            mb.tube([Vector((0.06, 0, z)), Vector((0.06, 0, z + 0.15)) + _horiz(b) * (0.55 - 0.07 * j)], [0.02, 0.012], 3,
                    "Foliage", cap_start=False)
            mb.blob(Vector((0.06, 0, z + 0.2)) + _horiz(b) * (0.55 - 0.07 * j), (0.22, 0.22, 0.09), "Foliage", seg=5,
                    rings=2, rng=r, rough=0.1)
        _solid(mb, first)


@env_part("vine_row")
def vine_row(model, name, p, box):
    """Vines on stakes (prompt 202): a piece of a vineyard row along Y. Each
    vine is a short gnarled trunk at a wooden stake, its shoots a loose
    band of large leaves from knee to head height.

    Parameters: `seed`; `vines` (default 5); `missing` (indices of vines
    left out, default none: a gap in the row, the stake stays); `clusters`
    (leaf clusters per vine, default 6); `trunk` (trunk height as a
    fraction of the height, default 0.4).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    n = p.get("vines", 5)
    H = box.height
    tz = H * p.get("trunk", 0.4)
    step = box.length / n
    missing = set(p.get("missing", []))
    size = min(step * 1.05, (H - tz) * 1.25)
    for i in range(n):
        y = -box.hl + step * (i + 0.5) + r.uniform(-0.08, 0.08)
        x = r.uniform(-0.05, 0.05)
        mb.style(kind=0, phase=0, wind=False)
        mb.tube([Vector((x + 0.07, y, 0)), Vector((x + 0.07, y, H * 0.9))], [0.025, 0.02], 3, "Bark", cap_start=False)
        if i in missing:
            continue
        mb.style(kind=0, phase=i + 1, wind=True)
        a = r.uniform(0, 2 * math.pi)
        mb.tube([Vector((x, y, 0)), Vector((x, y, tz * 0.5)) + _horiz(a) * 0.07, Vector((x, y, tz)) - _horiz(a) * 0.03],
                [0.05, 0.04, 0.03], 4, "Bark", cap_start=False)
        mb.style(kind=0)
        for j in range(p.get("clusters", 6)):
            q = Vector((x + r.uniform(-1, 1) * (box.hw - size * 0.3), y + r.uniform(-0.5, 0.5) * step,
                        tz + (H - tz - size * 0.35) * (j + 0.5) / p.get("clusters", 6) + size * 0.1))
            # three cards: the lying one closes the row seen from above
            _cluster(mb, q, size, r, CARD_VINE, 100 + i * 10 + j, cards=3)


@env_part("grass")
def grass(model, name, p, box):
    """Grass clump: crossed cards standing on the ground.

    Parameters: `seed`; `cards` (default 3); `flowers` (extra flower
    cards, default 0); `spread` (outward tilt, default 0.25).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    n = p.get("cards", 3)
    spread = p.get("spread", 0.25)
    w = max(box.width, box.length)
    for i in range(n):
        a = i * math.pi / n + r.uniform(-0.15, 0.15)
        mb.style(phase=i + 1, wind=True)
        mb.card((0, 0, 0), _horiz(a), Vector((0, 0, 1)) + _horiz(a + math.pi / 2) * spread * (1 if i % 2 else -1),
                w, box.height, CARD_GRASS, anchor="bottom")
    for i in range(p.get("flowers", 0)):
        a = i * math.pi / max(p.get("flowers", 1), 1) + 0.6
        mb.style(phase=20 + i, wind=True)
        mb.card((0, 0, 0), _horiz(a), (0, 0, 1), w * 0.8, box.height, CARD_FLOWERS, anchor="bottom")


@env_part("stump")
def stump(model, name, p, box):
    """Tree stump with root flares and a jagged (or sawn) top.

    Parameters: `seed`; `sawn` (flat cut instead of broken, default false);
    `roots` (default 4).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    rad = min(box.hw, box.hl) * 0.55
    h = box.height
    seg = 9
    mb.style(kind=0)
    ring0, ring1, top = [], [], []
    for k in range(seg):
        a = 2 * math.pi * k / seg
        ring0.append(Vector((math.cos(a) * rad * 1.25, math.sin(a) * rad * 1.25, 0)))
        ring1.append(Vector((math.cos(a) * rad, math.sin(a) * rad, h * 0.55)))
        hh = h if p.get("sawn", False) else h * r.uniform(0.7, 1.0)
        top.append(Vector((math.cos(a) * rad * 0.97, math.sin(a) * rad * 0.97, hh)))
    mb.loft([ring0, ring1, top], "Bark", cap_start=False, cap_end=False, smooth=True)
    cv = mb.verts([(0, 0, h * (0.85 if not p.get("sawn", False) else 1.0))])[0]
    tv = mb.verts(top)
    mb.style(kind=1)
    for k in range(seg):
        mb.face([tv[k], tv[(k + 1) % seg], cv], "Wood")
    mb.style(kind=0)
    for i in range(p.get("roots", 4)):
        a = i * 2 * math.pi / p.get("roots", 4) + r.uniform(-0.3, 0.3)
        d = _horiz(a)
        mb.tube([d * rad * 0.6 + Vector((0, 0, h * 0.25)), d * rad * 1.6 + Vector((0, 0, 0.05)),
                 d * min(box.hw, box.hl) * 0.98 + Vector((0, 0, -0.02))],
                [rad * 0.35, rad * 0.18, rad * 0.06], 4, "Bark", cap_start=False)


@env_part("log")
def log(model, name, p, box):
    """Fallen trunk lying along Y with branch stubs and a broken end.

    Parameters: `seed`; `stubs` (default 4); `seg` (default 8).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    L = box.length
    rad = box.height * 0.4
    seg = p.get("seg", 8)
    mb.style(kind=0)
    pts, radii = [], []
    for i in range(5):
        t = i / 4
        radii.append(rad * (1.0 - 0.4 * t))
        pts.append(Vector((math.sin(t * 2.4) * 0.08 * box.width, -L / 2 + L * t, radii[-1])))
    mb.tube(pts, radii, seg, "Bark", cap_start=True, cap_end=True)
    for i in range(p.get("stubs", 4)):
        t = 0.3 + 0.6 * r.random()
        base = pts[0].lerp(pts[-1], t)
        a = r.uniform(-1.2, 1.2) + (math.pi if i % 2 else 0)
        d = Vector((math.cos(a), 0.3, math.sin(a) * 0.5 + 0.6)).normalized()
        rr = rad * (1.0 - 0.4 * t) * 0.3
        mb.tube([base, base + d * rad * 1.6], [rr, rr * 0.5], 4, "Bark", cap_start=False)
