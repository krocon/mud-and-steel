"""Coast defences (prompt 202): gun casemate, heavy gun in a ring emplacement,
concrete coast battery, fire control station.

Environment generators like those of `parts_neutral` (`@env_part`), for the
neutral catalog entries `fort_casemate_gun`, `fort_coast_gun`,
`fort_battery_concrete` and `fort_fire_control`. The front (embrasure,
field of fire) faces +Y. The guns are scenery: fixed parts of the mesh, no
weapon of the simulation.

Materials: concrete is `Stone` kind 2, earth `Ground` kind 0 / spoil kind 1,
sandbags `Canvas`, the guns `Paint` kind 0 with `Metal` kind 3 (dark)
barrels, dark openings `Metal` kind 3.
"""

import math

from mathutils import Vector

from .env import env_part


def _gun(mb, base, elevation_deg, barrel=4.74, shield=True, scale=1.0, heading_deg=0.0):
    """Heavy gun on a pedestal mount: pedestal, cradle, barrel, optional
    shield. `base` is the foot of the pedestal, `barrel` the tube length
    (4.74 m: the 90 mm L/53 of the catalog source). The barrel points to
    +Y turned by `heading_deg` (positive to +X)."""
    bx, by, bz = base
    s = scale
    h = math.radians(heading_deg)
    fwd = Vector((math.sin(h), math.cos(h), 0.0))
    right = Vector((math.cos(h), -math.sin(h), 0.0))
    up = Vector((0.0, 0.0, 1.0))
    o = Vector((bx, by, bz))
    mb.style(kind=0)
    mb.cylinder(o, o + up * 0.25 * s, 0.75 * s, seg=10, mat="Paint", smooth=False)      # base ring
    mb.cylinder(o + up * 0.25 * s, o + up * 1.25 * s, 0.38 * s, 0.30 * s, seg=8, mat="Paint", smooth=False)
    tr = o + up * 1.45 * s                                                                # trunnion
    for k in (-1, 1):                                                                     # cheeks
        mb.oriented_box(tr + right * k * 0.36 * s - up * 0.05 * s, fwd, up, (0.85 * s, 0.55 * s, 0.10 * s),
                        "Paint")
    e = math.radians(elevation_deg)
    d = fwd * math.cos(e) + up * math.sin(e)
    n = up * math.cos(e) - fwd * math.sin(e)
    mb.oriented_box(tr + d * 0.25 * s, d, n, (1.9 * s, 0.34 * s, 0.40 * s), "Paint")      # cradle
    mb.oriented_box(tr - d * 0.95 * s, d, n, (0.55 * s, 0.42 * s, 0.46 * s), "Paint")     # breech
    mb.style(kind=3)
    b0 = tr - d * 0.6 * s
    mb.cylinder(b0, b0 + d * barrel * 0.45, 0.115 * s, 0.095 * s, seg=8, mat="Metal")
    mb.cylinder(b0 + d * barrel * 0.45, b0 + d * barrel, 0.095 * s, 0.07 * s, seg=8, mat="Metal")
    if shield:
        mb.style(kind=0)
        c = tr + fwd * 0.85 * s + up * 0.20 * s
        for k in (-1, 1):
            mb.oriented_box(c + right * k * 0.75 * s, right, up, (1.05 * s, 1.75 * s, 0.03), "Paint")
        mb.oriented_box(c + up * 0.62 * s, right, up, (0.50 * s, 0.50 * s, 0.03), "Paint")
        mb.oriented_box(c - up * 0.62 * s, right, up, (0.50 * s, 0.50 * s, 0.03), "Paint")
        for k in (-1, 1):                                                                 # angled wings
            w = (right * 0.82 - fwd * 0.57 * k) if k == 1 else (right * 0.82 + fwd * 0.57)
            mb.oriented_box(c + right * k * 1.55 * s - fwd * 0.20 * s, w, up, (0.70 * s, 1.75 * s, 0.03), "Paint")


def _two_sided(mb, pts, mat):
    """A face seen from both sides (earth banks: their winding depends on
    the side of the model they lie on)."""
    mb.face(mb.verts(pts), mat)
    mb.face(mb.verts(pts[::-1]), mat)


def _ring_wall(mb, r_out, r_in, z0, z1, seg, mat, gap=None):
    """Ring wall from sector blocks; `gap` = (center angle, width in rad) is
    left open (entrance)."""
    for i in range(seg):
        a0, a1 = 2 * math.pi * i / seg, 2 * math.pi * (i + 1) / seg
        am = (a0 + a1) / 2
        if gap is not None and abs((am - gap[0] + math.pi) % (2 * math.pi) - math.pi) < gap[1] / 2:
            continue
        poly = [(math.cos(a0) * r_in, math.sin(a0) * r_in), (math.cos(a0) * r_out, math.sin(a0) * r_out),
                (math.cos(a1) * r_out, math.sin(a1) * r_out), (math.cos(a1) * r_in, math.sin(a1) * r_in)]
        mb.prism(poly, z0, z1, mat)


@env_part("gun_casemate")
def gun_casemate(model, name, p, box):
    """Concrete casemate for a field or coast gun: a block with a wide,
    stepped embrasure in the front (+Y), the roof slab overhanging it, wing
    walls beside the embrasure, an entrance at the rear and earth banked
    against both sides. The barrel of the gun looks out of the embrasure.

    Parameters: `wall` (wall and roof thickness, default 2.0 m: the
    construction standard "B new"), `embrasure` [width, height] (default
    [3.6, 1.5]), `gun` (default true), `banks` (earth at the sides, default
    true).
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    wall = p.get("wall", 2.0)
    ew, eh = p.get("embrasure", [3.6, 1.5])
    bank = W * 0.14 if p.get("banks", True) else 0.0
    hw = W / 2 - bank
    hl = L / 2
    front = hl - L * 0.10          # front wall face; the wing walls reach to hl
    ez0 = 0.9
    mb.style(kind=2)
    # rear and side walls, roof
    mb.box((0, -hl + wall / 2, (H - wall) / 2), (hw * 2, wall, H - wall), "Stone", bevel=0.04)
    for k in (-1, 1):
        mb.box((k * (hw - wall / 2), (front - hl) / 2 + wall / 2, (H - wall) / 2),
               (wall, front + hl - wall, H - wall), "Stone", bevel=0.04)
    mb.box((0, (front + 0.5 - hl) / 2, H - wall / 2), (hw * 2 + 0.3, front + 0.5 + hl, wall), "Stone", bevel=0.06)
    # front wall around the embrasure: sill, two cheeks, lintel
    mb.box((0, front - wall / 2, ez0 / 2), (hw * 2 - wall * 2, wall, ez0), "Stone")
    cw = hw - wall - ew / 2
    for k in (-1, 1):
        mb.box((k * (ew / 2 + cw / 2), front - wall / 2, ez0 + eh / 2), (cw, wall, eh), "Stone")
        # stepped embrasure cheek (anti-ricochet steps)
        for i in range(3):
            mb.box((k * (ew / 2 - 0.12 - i * 0.22), front - wall * (0.2 + 0.25 * i), ez0 + eh / 2),
                   (0.22, wall * 0.25, eh), "Stone")
    top = ez0 + eh
    if H - wall > top:
        mb.box((0, front - wall / 2, (top + H - wall) / 2), (hw * 2 - wall * 2, wall, H - wall - top), "Stone")
    # wing walls shielding the embrasure from the flanks
    for k in (-1, 1):
        mb.prism([(k * hw, front), (k * (hw - 0.8), front), (k * (hw + 0.2), hl), (k * (hw + 0.9), hl)][::k],
                 0.0, H * 0.62, "Stone", bevel=0.04)
    # dark interior behind the embrasure and the rear door
    mb.style(kind=3)
    mb.box((0, front - wall - 0.02, ez0 + eh / 2), (ew, 0.04, eh), "Metal")
    mb.box((hw * 0.45, -hl - 0.02, 1.0), (0.9, 0.04, 2.0), "Metal")
    if p.get("gun", True):
        _gun(mb, (0.0, front - wall - 0.3, 0.25), 2.0, barrel=4.74, shield=False, scale=0.95)
    if bank:
        mb.style(kind=0)
        for k in (-1, 1):
            x0, x1 = k * hw, k * W / 2
            ys = (-hl + 0.4, front - 0.6)
            _two_sided(mb, [(x0, ys[0], H * 0.80), (x0, ys[1], H * 0.80), (x1, ys[1] - 1.0, 0.0),
                            (x1, ys[0] + 1.0, 0.0)], "Ground")
            for y, yo in ((ys[0], ys[0] + 1.0), (ys[1], ys[1] - 1.0)):
                _two_sided(mb, [(x0, y, H * 0.80), (x1, yo, 0.0), (x0, y, 0.0)], "Ground")
    mb.style(kind=0)
    mb.prism([(-hw * 0.9, -hl * 0.9), (hw * 0.9, -hl * 0.9), (hw * 0.9, front), (-hw * 0.9, front)], 0.0, 0.04,
             "Ground")


@env_part("gun_ring")
def gun_ring(model, name, p, box):
    """Heavy gun in a round emplacement: a pedestal gun with a shield in
    the middle of a ring wall with an entrance at the rear (-Y).

    Parameters: `wall` "concrete" (low ring of concrete with a platform,
    default) or "sandbags" (courses of bags around an earth core),
    `parapet` (height of the ring wall, default 1.2), `thickness` (default
    0.9), `elevation_deg` (barrel, default 5; 60 for an anti-aircraft
    gun), `heading_deg` (barrel turned from +Y, default 0), `lockers`
    (ready-use ammunition niches in the wall, default 4).
    """
    mb = model.mb(name)
    R = min(box.hl, box.hw)
    par = p.get("parapet", 1.2)
    th = p.get("thickness", 0.9)
    kind = p.get("wall", "concrete")
    gap = (-math.pi / 2, 0.55)
    if kind == "concrete":
        mb.style(kind=2)
        _ring_wall(mb, R, R - th, 0.0, par, 16, "Stone", gap=gap)
        mb.cylinder((0, 0, 0.0), (0, 0, 0.12), R - th, seg=16, mat="Stone", smooth=False)   # platform
        mb.style(kind=3)
        n = p.get("lockers", 4)
        for i in range(n):
            a = math.pi * (0.15 + 0.7 * i / max(n - 1, 1))
            c = Vector((math.cos(a) * (R - th + 0.03), math.sin(a) * (R - th + 0.03), par * 0.45))
            mb.oriented_box(c, (-math.sin(a), math.cos(a), 0), (0, 0, 1), (0.9, par * 0.55, 0.06), "Metal")
    elif kind == "sandbags":
        mb.style(kind=1)
        _ring_wall(mb, R - 0.14, R - th + 0.14, 0.0, par * 0.97, 14, "Ground", gap=gap)
        mb.style(kind=0)
        courses = max(3, int(par / 0.30))
        ch = par / courses
        rr = R - 0.13          # bags on the outer face only (triangle budget)
        n = int(2 * math.pi * rr / 0.92)
        for c in range(courses):
            for i in range(n):
                a = 2 * math.pi * (i + (0.5 if c % 2 else 0.0)) / n
                if abs((a - gap[0] + math.pi) % (2 * math.pi) - math.pi) < gap[1] / 2:
                    continue
                mb.oriented_box((math.cos(a) * rr, math.sin(a) * rr, c * ch + ch / 2),
                                (-math.sin(a), math.cos(a), 0), (0, 0, 1), (0.88, ch * 0.94, 0.26), "Canvas")
        mb.style(kind=0)
        mb.cylinder((0, 0, 0.0), (0, 0, 0.05), R - th, seg=14, mat="Ground", smooth=False)
    else:
        raise ValueError(f"gun_ring: wall must be concrete or sandbags, not {kind!r}")
    _gun(mb, (0.0, 0.0, 0.1), p.get("elevation_deg", 5.0), barrel=4.74, shield=True,
         heading_deg=p.get("heading_deg", 0.0))


@env_part("coast_battery")
def coast_battery(model, name, p, box):
    """Concrete coast battery of the late 19th century type: a long, massive
    parapet towards the sea (+Y), behind it the gun platforms open to the
    rear, traverses with the magazines between and beside them, stairs and
    a loading gallery at the back.

    Parameters: `guns` (number of gun platforms, default 2), `armed`
    (true: a shielded gun on every platform; false: empty platforms of a
    disarmed battery, default true).
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    hl, hw = L / 2, W / 2
    guns = p.get("guns", 2)
    par_t = L * 0.34                  # parapet thickness towards the sea
    plat_z = H * 0.42                 # gun platform level
    mb.style(kind=2)
    # parapet: sloping outer face, vertical inner face
    gl = L * 0.16                     # earth glacis in front of the parapet
    prof = [(hl - gl, 0.0), (hl - gl, H * 0.55), (hl - gl - par_t * 0.40, H), (hl - par_t - gl * 0.3, H),
            (hl - par_t - gl * 0.3, 0.0)]
    a = mb.verts([(-hw, y, z) for y, z in prof])
    b = mb.verts([(hw, y, z) for y, z in prof])
    n = len(prof)
    mb.face(a[::-1], "Stone")
    mb.face(b, "Stone")
    for i in range(n):
        j = (i + 1) % n
        mb.face([a[i], a[j], b[j], b[i]], "Stone")
    # lower storey (magazines) under the platforms, with dark doors to the rear
    y_in = hl - par_t - gl * 0.3
    mb.box((0, (y_in - hl) / 2, plat_z / 2), (W, y_in + hl, plat_z), "Stone", bevel=0.05)
    # traverses: one at each end and one between neighbouring platforms
    n_tr = guns + 1
    tw = W * 0.10
    bay = (W - n_tr * tw) / guns
    for i in range(n_tr):
        x = -hw + tw / 2 + i * (tw + bay)
        mb.box((x, (y_in - hl * 0.55) / 2, (plat_z + H) / 2), (tw, y_in + hl * 0.55, H - plat_z), "Stone", bevel=0.05)
        mb.style(kind=3)
        mb.box((x, -hl - 0.02, plat_z * 0.45), (min(1.4, tw * 0.5), 0.04, plat_z * 0.7), "Metal")
        mb.style(kind=2)
    for i in range(guns):
        x = -hw + tw + bay / 2 + i * (tw + bay)
        if p.get("armed", True):
            _gun(mb, (x, y_in - bay * 0.34, plat_z), 4.0, barrel=4.74, shield=True)
        else:
            mb.style(kind=3)
            mb.cylinder((x, y_in - bay * 0.34, plat_z), (x, y_in - bay * 0.34, plat_z + 0.06), 1.4, seg=12,
                        mat="Metal", smooth=False)
            mb.style(kind=2)
    # earth glacis in front of the parapet, running out at both ends
    mb.style(kind=0)
    yt = hl - gl
    _two_sided(mb, [(-hw, yt, H * 0.5), (hw, yt, H * 0.5), (hw - gl, hl, 0.0), (-hw + gl, hl, 0.0)], "Ground")
    for k in (-1, 1):
        _two_sided(mb, [(k * hw, yt, H * 0.5), (k * (hw - gl), hl, 0.0), (k * hw, yt, 0.0)], "Ground")


@env_part("fire_control")
def fire_control(model, name, p, box):
    """Fire control / observation station: a small concrete room with a
    long observation slit under an overhanging roof slab towards +Y, half
    sunk, the entrance at the rear, earth banked against the sides.

    Parameters: `slit` (height of the observation slit, default 0.45),
    `wall` (default 0.6).
    """
    mb = model.mb(name)
    L, W, H = box.length, box.width, box.height
    hl, hw = L / 2 - L * 0.08, W / 2 - W * 0.12
    wall = p.get("wall", 0.6)
    slit = p.get("slit", 0.45)
    roof = 0.45
    z1 = H - roof - slit
    mb.style(kind=2)
    mb.box((0, 0, z1 / 2), (hw * 2, hl * 2, z1), "Stone", bevel=0.04)
    for k in (-1, 1):   # slit posts at the corners and the rear wall up to the roof
        mb.box((k * (hw - wall / 2), hl - wall / 2, z1 + slit / 2), (wall, wall, slit), "Stone")
    mb.box((0, -hl + wall / 2, z1 + slit / 2), (hw * 2, wall, slit), "Stone")
    mb.box((0, 0.05, H - roof / 2), (hw * 2 + 0.4, hl * 2 + 0.3, roof), "Stone", bevel=0.05)
    mb.style(kind=3)
    mb.box((0, 0.0, z1 + slit / 2), (hw * 2 - wall * 2, hl * 2 - 0.1, slit * 0.98), "Metal")
    mb.box((0, -hl - 0.02, min(1.0, z1 * 0.5)), (0.8, 0.04, min(1.9, z1 * 0.95)), "Metal")
    mb.style(kind=0)
    for k in (-1, 1):
        x0, x1 = k * hw, k * W / 2
        _two_sided(mb, [(x0, -hl, z1 * 0.9), (x0, hl, z1 * 0.9), (x1, hl, 0.0), (x1, -hl, 0.0)], "Ground")
    _two_sided(mb, [(-hw, hl, z1 * 0.9), (hw, hl, z1 * 0.9), (hw, L / 2, 0.0), (-hw, L / 2, 0.0)], "Ground")
