"""Railway, mine and landmark buildings (prompt 202, group E): a train shed
over the tracks, a loading ramp, a pit headframe, a spoil heap, a piece of
mine tramway with tubs, and free-standing monuments (a round tower on a
hill, an observatory with a dome, a column). No real building is copied:
generic shapes with the main sizes of their kind, no inscriptions.

Damage (`damage` 1 / 2) is cut the cheap way for these shapes: everything
above a broken line is removed (bmesh bisect) and rubble lies at the foot.
Length runs along Y, width along X, as everywhere in the kit.
"""

import math

import bmesh
from mathutils import Vector

from .env import env_part, rng


def _since(mb, first):
    mb.bm.faces.ensure_lookup_table()
    return list(mb.bm.faces)[first:]


def _cut(mb, first, z, tilt=(0.0, 0.0)):
    """Removes the geometry built since face index `first` above the plane
    through (0, 0, z) whose normal leans by `tilt` (x, y): a broken top."""
    faces = _since(mb, first)
    if not faces:
        return
    verts = list({v for f in faces for v in f.verts})
    edges = list({e for f in faces for e in f.edges})
    n = Vector((tilt[0], tilt[1], 1.0)).normalized()
    bmesh.ops.bisect_plane(mb.bm, geom=verts + edges + faces, plane_co=(0, 0, z), plane_no=n, clear_outer=True)


def _rubble(mb, r, cx, cy, rx, ry, h, n=4, mat="Stone"):
    mb.style(kind=0)
    for _ in range(n):
        x, y = cx + r.uniform(-rx, rx), cy + r.uniform(-ry, ry)
        s = r.uniform(0.5, 1.0)
        mb.blob((x, y, h * s * 0.25), (rx * 0.35 * s + 0.4, ry * 0.35 * s + 0.4, h * s * 0.55), mat, seg=6, rings=2,
                rng=r, rough=0.3, smooth=False)


def _rib(mb, pts, size, mat="Metal"):
    """A beam along a polyline (thin oriented boxes)."""
    for a, b in zip(pts, pts[1:]):
        a, b = Vector(a), Vector(b)
        d = b - a
        side = Vector((0, 1, 0)) if abs(d.normalized().y) < 0.9 else Vector((1, 0, 0))
        mb.oriented_box((a + b) / 2, d, side, (d.length + size * 0.5, size, size), mat)


@env_part("train_shed")
def train_shed(model, name, p, box):
    """Train shed: a segmental iron roof on two rows of cast-iron columns
    over two tracks and a middle platform, glazed along the crown, open
    ends with an arched rib. Parameters: `seed`; `damage`; `bays` (column
    bays along the length, default 8); `eave` (height of the column tops,
    default 0.55 x height)."""
    r = rng(p.get("seed", 1))
    dmg = p.get("damage", 0)
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    eave = H * p.get("eave", 0.55)
    bays = p.get("bays", 8)
    first = len(mb.bm.faces)
    # platform between the tracks
    mb.style(kind=1)
    mb.box((0, 0, 0.45), (hw * 0.5, box.length - 2, 0.9), "Stone")
    # columns and longitudinal girders
    for s in (-1, 1):
        x = s * (hw - 0.35)
        for i in range(bays + 1):
            y = -hl + 0.3 + (box.length - 0.6) * i / bays
            mb.style(kind=3)
            mb.cylinder((x, y, 0), (x, y, eave), 0.2, 0.16, seg=6, mat="Metal")
        mb.style(kind=3)
        mb.box((x, 0, eave + 0.3), (0.3, box.length, 0.6), "Metal")
    # roof: segmental arch along X, n panels; the middle ones glazed
    n = 10

    def arch(k):
        x = -hw + 2 * hw * k / n
        return x, eave + 0.6 + (H - eave - 0.6) * (1 - (x / hw) ** 2)

    gone = set()
    if dmg == 1:
        gone = {(k, j) for k in range(n) for j in range(bays) if r.random() < 0.18}
    for j in range(bays):
        y0 = -hl + box.length * j / bays
        y1 = -hl + box.length * (j + 1) / bays
        for k in range(n):
            if (k, j) in gone:
                continue
            (xa, za), (xb, zb) = arch(k), arch(k + 1)
            glass = 3 <= k <= 6
            if glass and dmg >= 1:
                continue
            mat = "Glass" if glass else "Roof"
            mb.style(kind=1 if not glass else 0)
            pts = [(xa, y0, za), (xb, y0, zb), (xb, y1, zb), (xa, y1, za)]
            mb.face(mb.verts(pts), mat)  # underside (seen from the platform)
            mb.face(list(reversed(mb.verts([(x, y, z + 0.04) for x, y, z in pts]))), mat)
    # arched ribs at every column pair and the ends
    for i in range(bays + 1):
        y = -hl + 0.3 + (box.length - 0.6) * i / bays
        mb.style(kind=3)
        _rib(mb, [(arch(k)[0] * 0.985, y, arch(k)[1] - 0.15) for k in range(n + 1)], 0.22)
    # end screens: a glazed tympanum on the +Y end only
    if dmg == 0:
        mb.style(kind=0)
        pts = [Vector((arch(k)[0], hl - 0.02, arch(k)[1] - 0.3)) for k in range(n + 1)]
        top = mb.verts(pts)
        low = mb.verts([(x, hl - 0.02, H * 0.86 if abs(x) < hw * 0.5 else z) for x, _y, z in pts])
        for a in range(n):
            if 2 <= a <= 7:
                mb.face([low[a], low[a + 1], top[a + 1], top[a]], "Glass")
    if dmg >= 2:
        _cut(mb, first, eave * 0.55, (0.08, 0.03))
        _rubble(mb, r, 0, 0, hw * 0.6, hl * 0.7, 2.0, n=6, mat="Metal")
    elif dmg == 1:
        _rubble(mb, r, hw * 0.3, -hl * 0.3, hw * 0.3, hl * 0.3, 1.0, n=3, mat="Metal")


@env_part("loading_ramp")
def loading_ramp(model, name, p, box):
    """Loading ramp of a goods yard: a platform of masonry with a timber
    edge at wagon floor height, a slope down at the -Y end, a few crates
    and a crane post. Parameters: `seed`; `crates` (default 5)."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    top = 1.1
    slope = min(10.0, box.length * 0.3)
    y0 = -hl + slope
    mb.style(kind=0)
    mb.box((0, (y0 + hl) / 2, top / 2), (2 * hw, hl - y0, top), "Stone")
    mb.loft([[(-hw, -hl, 0.0), (hw, -hl, 0.0), (hw, -hl, 0.02), (-hw, -hl, 0.02)],
             [(-hw, y0, 0.0), (hw, y0, 0.0), (hw, y0, top), (-hw, y0, top)]], "Stone")
    mb.style(kind=2)
    mb.box((hw - 0.12, (y0 + hl) / 2, top + 0.06), (0.24, hl - y0, 0.12), "Wood")
    for k in range(p.get("crates", 5)):
        mb.style(kind=r.choice((0, 1)))
        s = r.uniform(0.7, 1.1)
        mb.box((r.uniform(-hw + 0.8, hw - 1.2), r.uniform(y0 + 1, hl - 1), top + 0.12 + s * 0.4),
               (s, s * r.uniform(0.9, 1.4), s * 0.8), "Wood")
    # crane post with a jib
    mb.style(kind=0)
    cy = hl - 2.0
    mb.cylinder((-hw + 0.6, cy, top), (-hw + 0.6, cy, H), 0.14, seg=6, mat="Wood")
    _rib(mb, [(-hw + 0.6, cy, H - 0.2), (hw - 0.4, cy, H - 0.9)], 0.14, "Wood")
    _rib(mb, [(-hw + 0.6, cy, H - 1.8), (0.4, cy, H - 0.6)], 0.1, "Wood")


@env_part("headframe")
def headframe(model, name, p, box):
    """Steel headframe over a pit shaft: four lattice legs over the shaft
    (at +Y), two raking back legs to the winding side (-Y), a head with
    two sheave wheels, a landing deck at bank level and a low shaft house.
    Parameters: `seed`; `damage`."""
    r = rng(p.get("seed", 1))
    dmg = p.get("damage", 0)
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    first = len(mb.bm.faces)
    head = H - 2.2
    sy = hl - 3.2  # shaft centre
    ft, tt = 2.6, 1.3  # half spread of the legs at the foot and at the head
    legs = []
    for sx in (-1, 1):
        for dy in (-1, 1):
            legs.append(((sx * ft * 1.25, sy + dy * ft, 0.0), (sx * tt, sy + dy * tt, head)))
    mb.style(kind=0)
    for a, b in legs:
        _rib(mb, [a, b], 0.32, "Paint")
    # horizontal frames and diagonal bracing between the legs
    levels = 6
    for i in range(1, levels + 1):
        t = i / levels
        ring = [Vector(a).lerp(Vector(b), t) for a, b in legs]
        order = (0, 1, 3, 2, 0)
        _rib(mb, [ring[k] for k in order], 0.16, "Paint")
        lo = [Vector(a).lerp(Vector(b), (i - 1) / levels) for a, b in legs]
        for k0, k1 in ((0, 1), (2, 3), (0, 2), (1, 3)):
            _rib(mb, [lo[k0], ring[k1]], 0.1, "Paint")
    # raking back legs (they take the pull of the ropes to the engine house)
    for sx in (-1, 1):
        _rib(mb, [(sx * hw * 0.8, -hl + 0.4, 0.0), (sx * tt, sy - tt, head - 0.6)], 0.36, "Paint")
        for t in (0.35, 0.65):
            a = Vector((sx * hw * 0.8, -hl + 0.4, 0.0)).lerp(Vector((sx * tt, sy - tt, head - 0.6)), t)
            b = Vector(legs[0 if sx < 0 else 2][0]).lerp(Vector(legs[0 if sx < 0 else 2][1]), t * 0.9)
            _rib(mb, [a, b], 0.12, "Paint")
    # head: platform, two sheave wheels side by side, railing
    mb.style(kind=3)
    mb.box((0, sy, head + 0.15), (2 * tt + 0.8, 2 * tt + 0.8, 0.3), "Metal")
    for sx in (-1, 1):
        x = sx * 0.6
        mb.cylinder((x - 0.08, sy, head + 1.1), (x + 0.08, sy, head + 1.1), 1.1, seg=12, mat="Metal", cap0=False,
                    cap1=False)
        mb.cylinder((x - 0.05, sy, head + 1.1), (x + 0.05, sy, head + 1.1), 0.25, seg=6, mat="Metal")
    mb.style(kind=0)
    _rib(mb, [(-tt - 0.4, sy - tt - 0.4, head + 1.2), (tt + 0.4, sy - tt - 0.4, head + 1.2),
              (tt + 0.4, sy + tt + 0.4, head + 1.2), (-tt - 0.4, sy + tt + 0.4, head + 1.2),
              (-tt - 0.4, sy - tt - 0.4, head + 1.2)], 0.06, "Metal")
    # ropes down the back legs
    mb.style(kind=1)
    for sx in (-1, 1):
        _rib(mb, [(sx * 0.6, sy - 1.0, head + 1.6), (sx * 0.6, -hl + 0.2, 4.0)], 0.05, "Metal")
    # bank deck and shaft house
    mb.style(kind=2)
    mb.box((0, sy, 3.2), (2 * ft * 1.25 + 1.0, 2 * ft + 1.0, 0.3), "Wood")
    mb.style(kind=3)
    mb.box((0, sy, 1.55), (2 * ft * 1.25 - 0.4, 2 * ft - 0.4, 3.1), "Stone")
    mb.style(kind=1)
    mb.box((0, sy, 6.2), (2 * ft + 0.6, 2 * ft + 0.2, 2.8), "Metal")
    mb.loft([[(-ft - 0.4, sy - ft - 0.2, 7.6), (ft + 0.4, sy - ft - 0.2, 7.6), (ft + 0.4, sy + ft + 0.2, 7.6),
              (-ft - 0.4, sy + ft + 0.2, 7.6)],
             [(-0.3, sy - ft - 0.2, 8.6), (0.3, sy - ft - 0.2, 8.6), (0.3, sy + ft + 0.2, 8.6),
              (-0.3, sy + ft + 0.2, 8.6)]], "Roof")
    if dmg >= 2:
        _cut(mb, first, H * 0.3, (0.25, -0.1))
        _rubble(mb, r, 0, sy - 2, hw * 0.6, 3.0, 2.2, n=5, mat="Metal")
    elif dmg == 1:
        _cut(mb, first, H * 0.78, (-0.35, 0.12))
        _rubble(mb, r, hw * 0.4, -hl * 0.3, 2.0, 2.5, 1.2, n=3, mat="Metal")


@env_part("spoil_heap")
def spoil_heap(model, name, p, box):
    """Spoil heap of a pit: a long cone of dark waste rock with a flat
    tipping crest and a tub track up its -Y flank. Parameters: `seed`;
    `track` (default true)."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    mb.style(kind=1)
    rings = []
    for z, f in ((0.0, 1.0), (H * 0.35, 0.72), (H * 0.7, 0.42), (H, 0.2)):
        ring = []
        for i in range(12):
            a = 2 * math.pi * i / 12
            j = 1.0 + (r.uniform(-0.07, 0.07) if z < H else 0.0)
            ring.append((math.cos(a) * hw * f * j, math.sin(a) * hl * f * j, z))
        rings.append(ring)
    mb.loft(rings, "Rock", cap_start=False)
    if p.get("track", True):
        mb.style(kind=2)
        a, b = Vector((0, -hl - 1.0, 0.05)), Vector((0, -hl * 0.2, H - 0.05))
        d = (b - a).normalized()
        n_sl = int((b - a).length / 2.5)
        for i in range(n_sl):
            c = a.lerp(b, (i + 0.5) / n_sl)
            mb.oriented_box(c, (1, 0, 0), d, (1.4, 0.18, 0.12), "Wood")
        mb.style(kind=1)
        for sx in (-0.3, 0.3):
            mb.oriented_box((a + b) / 2 + Vector((sx, 0, 0.1)), d, (1, 0, 0), ((b - a).length, 0.06, 0.1), "Metal")


@env_part("mine_tubs")
def mine_tubs(model, name, p, box):
    """A piece of narrow-gauge mine tramway with a short train of tipping
    tubs. Parameters: `seed`; `tubs` (default 3, 0 for bare track)."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl = box.hw, box.hl
    gauge = 0.3
    mb.style(kind=2)
    n = max(2, int(box.length / 0.8))
    for i in range(n):
        y = -hl + box.length * (i + 0.5) / n
        mb.box((r.uniform(-0.03, 0.03), y, 0.05), (1.2, 0.16, 0.1), "Wood")
    mb.style(kind=1)
    for sx in (-gauge, gauge):
        mb.box((sx, 0, 0.14), (0.05, box.length, 0.08), "Metal")
    for k in range(p.get("tubs", 3)):
        y = -hl + 1.0 + k * 1.6
        if y > hl - 0.7:
            break
        mb.style(kind=r.choice((1, 3)))
        mb.loft([[(-0.42, y - 0.5, 0.45), (0.42, y - 0.5, 0.45), (0.42, y + 0.5, 0.45), (-0.42, y + 0.5, 0.45)],
                 [(-hw + 0.02, y - 0.6, box.height), (hw - 0.02, y - 0.6, box.height), (hw - 0.02, y + 0.6, box.height),
                  (-hw + 0.02, y + 0.6, box.height)]], "Metal", cap_end=False)
        mb.style(kind=0)
        for dy in (-0.3, 0.3):
            for sx in (-1, 1):
                mb.cylinder((sx * (gauge - 0.04), y + dy, 0.32), (sx * (gauge + 0.04), y + dy, 0.32), 0.14, seg=6,
                            mat="Metal")


@env_part("monument_tower")
def monument_tower(model, name, p, box):
    """Monument tower on a hill: a low polygonal base house, a round tower
    in stages narrowing upward, a crenellated gallery and a short top
    stage with a flagstaff. Parameters: `seed`; `damage`; `stages`
    (default 4)."""
    r = rng(p.get("seed", 1))
    dmg = p.get("damage", 0)
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    first = len(mb.bm.faces)
    R0 = min(hw, hl)
    mb.style(kind=1)
    base = [(math.cos(2 * math.pi * i / 5) * R0, math.sin(2 * math.pi * i / 5) * R0) for i in range(5)]
    mb.prism(base, 0.0, 4.5, "Stone")
    mb.prism([(x * 1.04, y * 1.04) for x, y in base], 4.5, 5.1, "Stone")
    stages = p.get("stages", 4)
    z = 5.1
    shaft = H - 2.5 - z
    rad = R0 * 0.55
    for s in range(stages):
        h = shaft * (0.34 if s == 0 else 0.66 / (stages - 1))
        mb.style(kind=1)
        mb.cylinder((0, 0, z), (0, 0, z + h), rad, rad * 0.97, seg=12, mat="Stone", cap0=False)
        mb.cylinder((0, 0, z + h), (0, 0, z + h + 0.35), rad * 1.08, seg=12, mat="Stone", cap0=True)
        # a slit window per stage
        mb.style(kind=0)
        mb.box((0, -rad * 0.98, z + h * 0.55), (0.35, 0.06, min(1.4, h * 0.4)), "Glass")
        z += h + 0.35
        rad *= 0.86
    # gallery with crenels
    mb.style(kind=1)
    for i in range(12):
        if i % 2:
            continue
        a = 2 * math.pi * i / 12
        mb.box((math.cos(a) * rad * 1.15, math.sin(a) * rad * 1.15, z + 0.45), (0.7, 0.7, 0.9), "Stone")
    mb.cylinder((0, 0, z), (0, 0, H - 2.5), rad * 0.6, seg=8, mat="Stone")
    mb.style(kind=0)
    mb.cylinder((0, 0, H - 2.5), (0, 0, H), 0.06, seg=4, mat="Wood")
    if dmg >= 2:
        _cut(mb, first, H * 0.28, (0.2, 0.1))
        _rubble(mb, r, 0, 0, R0 * 0.45, R0 * 0.45, 2.5, n=5)
    elif dmg == 1:
        _cut(mb, first, H * 0.7, (-0.3, 0.2))
        _rubble(mb, r, R0 * 0.35, 0, R0 * 0.35, R0 * 0.35, 1.5, n=3)


@env_part("observatory")
def observatory(model, name, p, box):
    """Small observatory: a classical cross-shaped building with a portico
    of columns at +Y, a drum with a copper dome over the crossing, small
    domes on the corner pavilions. Parameters: `seed`; `damage`."""
    r = rng(p.get("seed", 1))
    dmg = p.get("damage", 0)
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    first = len(mb.bm.faces)
    wall = H * 0.42
    arm = min(hw, hl) * 0.45
    mb.style(kind=1)
    mb.box((0, 0, wall / 2), (2 * hw, 2 * arm, wall), "Stone")
    mb.box((0, 0, wall / 2), (2 * arm, 2 * hl - 2.4, wall), "Stone")
    mb.box((0, 0, wall + 0.25), (2 * hw + 0.3, 2 * arm + 0.3, 0.5), "Stone")
    mb.box((0, 0, wall + 0.25), (2 * arm + 0.3, 2 * hl - 2.1, 0.5), "Stone")
    # portico
    for i in range(4):
        x = -arm * 0.8 + arm * 1.6 * i / 3
        mb.cylinder((x, hl - 0.6, 0.0), (x, hl - 0.6, wall), 0.32, seg=8, mat="Stone")
    mb.loft([[(-arm, hl - 1.2, wall), (arm, hl - 1.2, wall), (arm, hl, wall), (-arm, hl, wall)],
             [(-0.1, hl - 1.2, wall + 2.0), (0.1, hl - 1.2, wall + 2.0), (0.1, hl, wall + 2.0),
              (-0.1, hl, wall + 2.0)]], "Stone")
    # windows
    mb.style(kind=0)
    for s in (-1, 1):
        mb.box((s * hw, 0, wall * 0.5), (0.06, 1.2, wall * 0.45), "Glass")
        mb.box((s * arm, hl * 0.55, wall * 0.5), (0.06, 1.0, wall * 0.4), "Glass")
    # drum and dome
    d0 = arm * 0.85
    mb.style(kind=1)
    mb.cylinder((0, 0, wall + 0.5), (0, 0, wall + 2.4), d0, seg=14, mat="Stone", cap0=False)
    mb.style(kind=0)
    prof = [(0.0, d0 * 1.03)] + [(math.sin(t * math.pi / 2) * (H - wall - 2.4), d0 * 1.03 * math.cos(t * math.pi / 2))
                                 for t in (0.25, 0.5, 0.75, 1.0)]
    mb.revolve((0, 0, wall + 2.4), (0, 0, 1), prof, seg=14, mat="Paint")
    # the slit of the dome
    mb.style(kind=3)
    mb.box((0, d0 * 0.2, wall + 2.4 + (H - wall - 2.4) * 0.7), (0.5, d0 * 1.3, (H - wall - 2.4) * 0.5), "Metal")
    if dmg >= 2:
        _cut(mb, first, wall * 0.55, (0.1, -0.15))
        _rubble(mb, r, 0, 0, hw * 0.6, hl * 0.6, 2.0, n=5)
    elif dmg == 1:
        _cut(mb, first, wall + 1.6, (0.25, 0.0))
        _rubble(mb, r, hw * 0.5, 0, 1.5, 2.0, 1.2, n=3)


@env_part("column_monument")
def column_monument(model, name, p, box):
    """Free-standing column monument: a stepped base, a pedestal, a fluted
    column with a capital and an urn on top (no figure, no inscription).
    Parameters: `seed`; `damage`."""
    r = rng(p.get("seed", 1))
    dmg = p.get("damage", 0)
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    first = len(mb.bm.faces)
    mb.style(kind=1)
    z = 0.0
    for k in range(3):
        f = 1.0 - k * 0.16
        mb.box((0, 0, z + 0.25), (2 * hw * f, 2 * hl * f, 0.5), "Stone")
        z += 0.5
    ped = min(hw, hl) * 0.5
    mb.box((0, 0, z + 1.6), (2 * ped, 2 * ped, 3.2), "Stone")
    mb.box((0, 0, z + 3.3), (2 * ped + 0.3, 2 * ped + 0.3, 0.3), "Stone")
    z += 3.45
    cr = ped * 0.62
    top = H - 2.4
    mb.cylinder((0, 0, z), (0, 0, top - 0.8), cr, cr * 0.86, seg=12, mat="Stone")
    mb.box((0, 0, top - 0.5), (cr * 2.6, cr * 2.6, 0.6), "Stone")
    mb.cylinder((0, 0, top - 0.8), (0, 0, top - 0.2), cr * 0.9, cr * 1.2, seg=12, mat="Stone")
    mb.style(kind=0)
    mb.revolve((0, 0, top - 0.2), (0, 0, 1), [(0, 0.3), (0.5, 0.55), (1.2, 0.6), (1.8, 0.35), (2.2, 0.15), (2.4, 0.02)],
               seg=10, mat="Stone")
    if dmg >= 2:
        _cut(mb, first, 3.0, (0.15, 0.1))
        _rubble(mb, r, 0, 0, hw * 0.4, hl * 0.4, 1.5, n=4)
    elif dmg == 1:
        _cut(mb, first, H * 0.6, (-0.2, 0.15))
        _rubble(mb, r, hw * 0.3, 0, hw * 0.3, hl * 0.3, 1.0, n=3)
