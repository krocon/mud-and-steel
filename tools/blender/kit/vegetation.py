"""Vegetation generators (prompt 007, part A): trees, bushes, grass, rocks, wood.

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

from .env import (CARD_BROADLEAF, CARD_FLOWERS, CARD_GRASS, CARD_NEEDLE, CARD_PALM, CARD_SHRUB,
                  CARD_TWIGS, env_part, rng)

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


@env_part("bush")
def bush(model, name, p, box):
    """Shrub: a few stems and a dome of crossed card clusters.

    Parameters: `seed`; `clusters` (default 40); `stems` (default 5);
    `pattern` (card pattern, default shrub); `center` (crown center height
    as a fraction of the height, default 0.55).
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
    for i in range(p.get("clusters", 40)):
        q = _ellipsoid_point(r, Vector((0, 0, cz)), box.hw * 0.75, box.hl * 0.75, (H - cz) * 0.75, shell=0.4)
        q.z = max(q.z, size * 0.45)
        _cluster(mb, q, size, r, pattern, 100 + i, cards=2)


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


@env_part("rock")
def rock(model, name, p, box):
    """Boulder: noisy ellipsoid, flattened where it sits in the ground.

    Parameters: `seed`; `rough` (noise amount, default 0.18); `kind`
    (Stone kind, default 0 fieldstone); `seg` (default 10); `rings` (default 7).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    mb.style(kind=p.get("kind", 0))
    mb.blob((0, 0, box.height * 0.35), (box.hw, box.hl, box.height * 0.65), "Stone", seg=p.get("seg", 10),
            rings=p.get("rings", 7), rng=r, rough=p.get("rough", 0.18), flat_bottom=0.0, smooth=False)


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
