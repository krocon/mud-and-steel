"""Scenery of camps, tracks and shores (prompt 202, group E): a parked truck,
a parked field gun, plank road pieces, rock needles, ford poles, a rowing
boat and log steps for a climb. None of them is a unit: generic shapes, no
markings. The fronts point to +Y.
"""

import math

from mathutils import Matrix, Vector

from .env import env_part, rng


def _wheel(mb, x, y, radius, width, seg=8):
    mb.style(kind=0)
    mb.cylinder((x - width / 2, y, radius), (x + width / 2, y, radius), radius, seg=seg, mat="Rubber")
    s = 1 if x > 0 else -1
    mb.cylinder((x + s * width / 2, y, radius), (x + s * (width / 2 + 0.02), y, radius), radius * 0.55, seg=6,
                mat="Paint")


@env_part("truck_parked")
def truck_parked(model, name, p, box):
    """Parked 3-ton truck with a bonnet, a closed cab and a cargo bed.

    Parameters: `tilt` (canvas cover on the bed, default true; without it
    the bows stand bare over the bed); `load` (crates on the open bed,
    default 0); `seed`.
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    wheel_r = 0.45
    frame_z = 0.78
    bed_y0, bed_y1 = -hl, hl * 0.12
    cab_y0, cab_y1 = bed_y1 + 0.08, bed_y1 + 1.35
    front_axle = hl - 1.0
    rear_axle = -hl + 1.55
    for s in (-1, 1):
        _wheel(mb, s * (hw - 0.14), front_axle, wheel_r, 0.24)
        _wheel(mb, s * (hw - 0.26), rear_axle, wheel_r, 0.5)
    mb.style(kind=3)
    mb.box((0, 0, frame_z - 0.12), (0.9, 2 * hl - 0.5, 0.2), "Metal")
    mb.box((0, front_axle, wheel_r), (2 * hw - 0.4, 0.12, 0.12), "Metal")
    mb.box((0, rear_axle, wheel_r), (2 * hw - 0.6, 0.14, 0.14), "Metal")
    mb.box((0, hl - 0.06, frame_z - 0.1), (2 * hw - 0.3, 0.1, 0.16), "Metal")  # bumper
    # bonnet, radiator, mudguards
    mb.style(kind=0)
    bon_y0, bon_y1 = cab_y1, hl - 0.18
    mb.loft([[(-0.62, bon_y0, frame_z), (0.62, bon_y0, frame_z), (0.62, bon_y0, 1.62), (0.45, bon_y0, 1.72),
              (-0.45, bon_y0, 1.72), (-0.62, bon_y0, 1.62)],
             [(-0.5, bon_y1, frame_z), (0.5, bon_y1, frame_z), (0.5, bon_y1, 1.5), (0.36, bon_y1, 1.6),
              (-0.36, bon_y1, 1.6), (-0.5, bon_y1, 1.5)]], "Paint")
    mb.style(kind=3)
    mb.box((0, bon_y1 + 0.02, 1.2), (0.8, 0.04, 0.6), "Metal")
    mb.style(kind=0)
    for s in (-1, 1):
        x = s * (hw - 0.2)
        mb.loft([[(x - 0.2, front_axle - 0.75, 0.7), (x + 0.2, front_axle - 0.75, 0.7), (x + 0.2, front_axle - 0.75, 0.76),
                  (x - 0.2, front_axle - 0.75, 0.76)],
                 [(x - 0.2, front_axle - 0.3, 1.0), (x + 0.2, front_axle - 0.3, 1.0), (x + 0.2, front_axle - 0.3, 1.06),
                  (x - 0.2, front_axle - 0.3, 1.06)],
                 [(x - 0.2, front_axle + 0.3, 1.0), (x + 0.2, front_axle + 0.3, 1.0), (x + 0.2, front_axle + 0.3, 1.06),
                  (x - 0.2, front_axle + 0.3, 1.06)],
                 [(x - 0.2, front_axle + 0.7, 0.72), (x + 0.2, front_axle + 0.7, 0.72), (x + 0.2, front_axle + 0.7, 0.78),
                  (x - 0.2, front_axle + 0.7, 0.78)]], "Paint")
        mb.cylinder((s * 0.42, bon_y1 + 0.02, 1.25), (s * 0.42, bon_y1 + 0.12, 1.25), 0.11, seg=8, mat="Glass")
    # cab: lower body, glazed upper part, roof
    cab_w = hw - 0.12
    mb.box((0, (cab_y0 + cab_y1) / 2, (frame_z + 1.5) / 2), (2 * cab_w, cab_y1 - cab_y0, 1.5 - frame_z), "Paint")
    top = min(2.15, H - 0.05)
    mb.loft([[(-cab_w, cab_y0, 1.5), (cab_w, cab_y0, 1.5), (cab_w, cab_y1, 1.5), (-cab_w, cab_y1, 1.5)],
             [(-cab_w + 0.06, cab_y0, top), (cab_w - 0.06, cab_y0, top), (cab_w - 0.06, cab_y1 - 0.18, top),
              (-cab_w + 0.06, cab_y1 - 0.18, top)]], "Paint")
    mb.box((0, cab_y1 - 0.07, (1.5 + top) / 2), (2 * cab_w - 0.3, 0.05, top - 1.5 - 0.16), "Glass")
    for s in (-1, 1):
        mb.box((s * (cab_w - 0.01), (cab_y0 + cab_y1) / 2 - 0.05, (1.5 + top) / 2), (0.05, cab_y1 - cab_y0 - 0.5,
                                                                                    top - 1.5 - 0.18), "Glass")
    # cargo bed: floor, boards, bows or tilt
    bed_z = 1.12
    bw = hw - 0.03
    mb.style(kind=0)
    mb.box((0, (bed_y0 + bed_y1) / 2, bed_z), (2 * bw, bed_y1 - bed_y0, 0.08), "Wood")
    side = 0.5
    for s in (-1, 1):
        mb.box((s * bw, (bed_y0 + bed_y1) / 2, bed_z + side / 2), (0.05, bed_y1 - bed_y0, side), "Wood")
    for y in (bed_y0 + 0.025, bed_y1 - 0.025):
        mb.box((0, y, bed_z + side / 2), (2 * bw, 0.05, side), "Wood")
    arch_z = H
    eave = bed_z + side + 0.75

    def arch(y):
        return [(-bw, y, bed_z + side), (bw, y, bed_z + side), (bw, y, eave), (bw * 0.55, y, arch_z),
                (-bw * 0.55, y, arch_z), (-bw, y, eave)]

    if p.get("tilt", True):
        mb.style(kind=1)
        mb.loft([arch(bed_y0), arch(bed_y1)], "Canvas")
    else:
        mb.style(kind=3)
        for k in range(3):
            y = bed_y0 + 0.1 + (bed_y1 - bed_y0 - 0.2) * k / 2
            pts = arch(y)
            for a, b in zip(pts[1:], pts[2:] + pts[:1]):
                a, b = Vector(a), Vector(b)
                mb.oriented_box((a + b) / 2, b - a, (0, 1, 0), ((b - a).length, 0.04, 0.04), "Metal")
        mb.style(kind=0)
        for k in range(p.get("load", 0)):
            sx, sy = r.uniform(0.5, 0.8), r.uniform(0.6, 1.0)
            mb.box((r.uniform(-bw + 0.5, bw - 0.5), r.uniform(bed_y0 + 0.6, bed_y1 - 0.6), bed_z + 0.04 + 0.22),
                   (sx, sy, 0.44), "Wood")


@env_part("gun_parked")
def gun_parked(model, name, p, box):
    """Field gun set down in a position: two spoked wheels, a shield, the
    barrel level to +Y, one box trail with a spade on the ground.

    Parameters: `cover` (a tarpaulin over breech and shield, default
    false).
    """
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    wheel_r = 0.6
    axle_y = hl * 0.1
    bore_z = 1.02
    mb.style(kind=0)
    for s in (-1, 1):
        x = s * (hw - 0.08)
        mb.style(kind=0)
        mb.cylinder((x - 0.07, axle_y, wheel_r), (x + 0.07, axle_y, wheel_r), wheel_r, seg=12, mat="Rubber")
        mb.cylinder((x + s * 0.07, axle_y, wheel_r), (x + s * 0.09, axle_y, wheel_r), wheel_r * 0.78, seg=10, mat="Paint")
    mb.style(kind=3)
    mb.box((0, axle_y, wheel_r), (2 * hw - 0.2, 0.12, 0.12), "Metal")
    mb.style(kind=0)
    # trail from the axle back to the spade
    a = Vector((0, axle_y, wheel_r + 0.05))
    b = Vector((0, -hl + 0.1, 0.16))
    mb.oriented_box((a + b) / 2, b - a, (1, 0, 0), ((b - a).length, 0.26, 0.2), "Paint")
    mb.box((0, -hl + 0.06, 0.2), (0.6, 0.06, 0.32), "Paint")  # spade dug in: kept off z = 0
    # cradle, barrel, breech
    mb.box((0, axle_y + 0.5, bore_z - 0.13), (0.24, 1.5, 0.16), "Paint")
    muzzle = hl
    mb.cylinder((0, axle_y - 0.55, bore_z), (0, axle_y + 0.3, bore_z), 0.11, seg=8, mat="Paint")
    mb.cylinder((0, axle_y + 0.3, bore_z), (0, muzzle, bore_z), 0.07, 0.055, seg=8, mat="Paint")
    mb.box((0, axle_y - 0.6, bore_z), (0.26, 0.24, 0.26), "Paint")
    # shield: two plates with the gap for the barrel, the top bent back
    top = H
    for s in (-1, 1):
        mb.box((s * (hw * 0.5 + 0.06), axle_y + 0.32, (0.5 + top - 0.2) / 2), (hw - 0.3, 0.02, top - 0.7), "Paint")
        pa = Vector((s * (hw * 0.5 + 0.06), axle_y + 0.32, top - 0.2))
        pb = Vector((s * (hw * 0.5 + 0.06), axle_y + 0.18, top))
        mb.oriented_box((pa + pb) / 2, pb - pa, (1, 0, 0), ((pb - pa).length, hw - 0.3, 0.02), "Paint")
    mb.box((0, axle_y + 0.32, bore_z + 0.32), (0.3, 0.02, 0.22), "Paint")
    if p.get("cover", False):
        mb.style(kind=1)
        y0, y1 = axle_y - 0.9, axle_y + 0.5
        w = hw - 0.12
        mb.loft([[(-w * 0.5, y0, 0.35), (w * 0.5, y0, 0.35), (w * 0.4, y0, bore_z + 0.25), (-w * 0.4, y0, bore_z + 0.25)],
                 [(-w, axle_y, 0.5), (w, axle_y, 0.5), (w * 0.9, axle_y + 0.2, top + 0.02),
                  (-w * 0.9, axle_y + 0.2, top + 0.02)],
                 [(-w, y1, 0.45), (w, y1, 0.45), (w * 0.9, y1 - 0.1, top - 0.1), (-w * 0.9, y1 - 0.1, top - 0.1)]],
                "Canvas")


@env_part("plank_road")
def plank_road(model, name, p, box):
    """Plank road over mud: planks across (X) on two stringers along Y.

    Parameters: `seed`; `plank` (plank width along the road, default
    0.28); `missing` (share of planks gone, default 0); `askew` (how far
    planks lie askew, radians, default 0.03).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    pw = p.get("plank", 0.28)
    n = max(1, int(box.length / (pw + 0.03)))
    step = box.length / n
    th = H * 0.45
    mb.style(kind=2)
    for s in (-1, 1):
        mb.box((s * hw * 0.62, 0, (H - th) / 2), (0.16, box.length, H - th), "Wood")
    for i in range(n):
        if r.random() < p.get("missing", 0.0):
            continue
        y = -hl + step * (i + 0.5)
        a = r.uniform(-1, 1) * p.get("askew", 0.03)
        mb.style(kind=r.choice((0, 0, 2, 1)))
        length = 2 * hw * r.uniform(0.93, 1.0)
        mb.oriented_box((r.uniform(-0.04, 0.04), y, H - th / 2), (math.cos(a), math.sin(a), 0), (-math.sin(a), math.cos(a), 0),
                        (length, pw, th), "Wood")


@env_part("hoodoo")
def hoodoo(model, name, p, box):
    """Rock needle (hoodoo): a slim, layered column with a harder cap
    rock; several needles on one foot stand together.

    Parameters: `seed`; `needles` (default 1); `cap` (cap rock, default
    true); `waist` (narrowest radius as a share of the foot, default 0.45).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    n = p.get("needles", 1)
    H = box.height
    R = min(box.hw, box.hl)
    for k in range(n):
        if n == 1:
            base, rr, h = Vector((0, 0, 0)), R * 0.9, H
        else:
            a = k * 2 * math.pi / n + r.uniform(-0.3, 0.3)
            base = Vector((math.cos(a), math.sin(a), 0)) * R * 0.5
            rr, h = R * r.uniform(0.38, 0.5), H * (1.0 if k == 0 else r.uniform(0.5, 0.85))
        rings = 8 if n == 1 else 5
        sides = 7 if n == 1 else 6
        pts, radii = [], []
        lean = Vector((r.uniform(-1, 1), r.uniform(-1, 1), 0)) * h * 0.04
        for i in range(rings + 1):
            t = i / rings
            waist = p.get("waist", 0.45)
            # wide foot, narrow waist at 60 %, layers bulging in and out
            prof = 1.0 - (1.0 - waist) * math.sin(min(1.0, t / 0.6) * math.pi / 2) ** 1.4
            prof *= 1.0 + (0.16 if i % 2 else -0.08) * (0.4 + t)
            if t > 0.85:
                prof *= 0.8
            pts.append(base + lean * t * t + Vector((r.uniform(-1, 1), r.uniform(-1, 1), 0)) * rr * 0.05
                       + Vector((0, 0, h * 0.9 * t - (0.3 if i == 0 else 0.0))))
            radii.append(rr * prof)
        mb.style(kind=0)
        mb.tube(pts, radii, sides, "Rock", cap_start=False, cap_end=True, smooth=False)
        if p.get("cap", True):
            mb.style(kind=0)
            top = pts[-1]
            mb.blob(top + Vector((0, 0, h * 0.04)), (radii[-2] * 1.25, radii[-2] * 1.15, h * 0.07), "Rock", seg=7, rings=3,
                    rng=r, rough=0.12, smooth=False)
    # scree at the foot
    mb.style(kind=1)
    for k in range(5 if n == 1 else 2):
        a = r.uniform(0, 2 * math.pi)
        d = R * r.uniform(0.6, 0.95)
        s = R * r.uniform(0.08, 0.16)
        mb.blob((math.cos(a) * d, math.sin(a) * d, s * 0.3), (s, s * 0.8, s * 0.6), "Rock", seg=5, rings=2, rng=r,
                rough=0.2, smooth=False)


@env_part("ford_pole")
def ford_pole(model, name, p, box):
    """Marker pole of a ford: a peeled stake with a white band and a
    bundle of brushwood tied to its top.

    Parameters: `seed`; `lean` (m the top leans, default 0.05); `count`
    (poles side by side along X, default 1).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    n = p.get("count", 1)
    for k in range(n):
        x = 0.0 if n == 1 else (-box.hw + 0.1 + (box.width - 0.2) * k / (n - 1))
        a = r.uniform(0, 2 * math.pi)
        lean = Vector((math.cos(a), math.sin(a), 0)) * p.get("lean", 0.05) * r.uniform(0.5, 1.5)
        h = H * (1.0 if k == 0 else r.uniform(0.8, 0.95))
        base = Vector((x, 0, -0.3))
        top = Vector((x, 0, h * 0.86)) + lean
        mb.style(kind=1)
        mb.cylinder(base, top, 0.05, 0.035, seg=6, mat="Wood")
        mb.style(kind=2)
        band0 = base.lerp(top, 0.72)
        band1 = base.lerp(top, 0.84)
        mb.cylinder(band0, band1, 0.047, 0.043, seg=6, mat="Paint", cap0=False, cap1=False)
        mb.style(kind=2)
        for j in range(5):
            b = j * 2 * math.pi / 5 + r.uniform(-0.3, 0.3)
            d = Vector((math.cos(b), math.sin(b), 0)) * 0.11
            mb.cylinder(top - Vector((0, 0, 0.05)), top + d + Vector((0, 0, h * 0.14)), 0.018, 0.008, seg=3, mat="Wood",
                        cap0=False)


@env_part("rowing_boat")
def rowing_boat(model, name, p, box):
    """Clinker rowing boat: pointed bow (+Y), transom stern, three thwarts,
    two oars laid inside.

    Parameters: `upturned` (keel up on the shore, default false); `heel`
    (degrees the boat lies over on its bilge, default 0); `oars` (default
    true).
    """
    mb = model.mb(name)
    hw, hl, H = box.hw, box.hl, box.height
    first = len(mb.bm.faces)
    # sections stern to bow: (y, half width at the gunwale, keel z, sheer z)
    secs = [(-hl, hw * 0.62, H * 0.22, H * 0.92), (-hl * 0.45, hw * 0.98, H * 0.03, H * 0.84),
            (hl * 0.15, hw, 0.0, H * 0.84), (hl * 0.62, hw * 0.72, H * 0.04, H * 0.9), (hl * 0.99, 0.03, H * 0.3, H)]
    outer, inner = [], []
    for y, w, k, z in secs:
        outer.append([(-w, y, z), (-w * 0.82, y, k + (z - k) * 0.38), (0, y, k), (w * 0.82, y, k + (z - k) * 0.38),
                      (w, y, z)])
        wi = max(w - 0.04, 0.01)
        inner.append([(wi, y, z), (wi * 0.8, y, k + (z - k) * 0.42 + 0.03), (0, y, k + 0.04),
                      (-wi * 0.8, y, k + (z - k) * 0.42 + 0.03), (-wi, y, z)])
    mb.style(kind=2)
    for a, b in zip(outer, outer[1:]):
        va, vb = mb.verts(a), mb.verts(b)
        for i in range(4):
            mb.face([va[i], va[i + 1], vb[i + 1], vb[i]], "Wood", True)
    mb.face(list(reversed(mb.verts(outer[0]))), "Wood")
    mb.style(kind=0)
    for a, b in zip(inner, inner[1:]):
        va, vb = mb.verts(a), mb.verts(b)
        for i in range(4):
            mb.face([va[i], va[i + 1], vb[i + 1], vb[i]], "Wood", True)
    mb.face(list(reversed(mb.verts(inner[0]))), "Wood")
    for y, f in ((-hl * 0.55, 0.9), (0.0, 0.97), (hl * 0.5, 0.78)):
        mb.box((0, y, H * 0.62), (2 * hw * f - 0.08, 0.2, 0.03), "Wood")
    if p.get("oars", True):
        mb.style(kind=1)
        for s in (-1, 1):
            a = Vector((s * hw * 0.3, -hl * 0.7, H * 0.66))
            b = Vector((s * hw * 0.22, hl * 0.6, H * 0.66))
            mb.cylinder(a, b, 0.022, seg=5, mat="Wood")
            mb.oriented_box(a + (a - b).normalized() * 0.25, a - b, (1, 0, 0), (0.6, 0.13, 0.02), "Wood")
    m = Matrix.Identity(4)
    if p.get("upturned", False):
        # keel up, resting on bow and stern (the sheer is higher at the bow)
        m = Matrix.Rotation(math.pi, 4, "Y") @ Matrix.Rotation(-math.asin(0.08 * H / box.length), 4, "X")
    elif p.get("heel", 0):
        m = Matrix.Rotation(math.radians(p.get("heel", 0)), 4, "Y")
    if m != Matrix.Identity(4):
        mb.bm.faces.ensure_lookup_table()
        faces = list(mb.bm.faces)[first:]
        mb.transform_faces(faces, m)
        low = min(v.co.z for f in faces for v in f.verts)
        mb.transform_faces(faces, Matrix.Translation((0, 0, -low)))


@env_part("climb_steps")
def climb_steps(model, name, p, box):
    """A piece of a foot path up a steep slope: log steps held by stakes,
    on one side posts with a hand rope. The path runs along Y.

    Parameters: `seed`; `steps` (default 3); `rope` (default true);
    `ladder` (a ladder of two rails and rungs lying on the slope instead
    of steps, default false); `rise` (m the path climbs over its length
    towards +Y, default 0: the piece is built on the slope and sunk by
    half the rise into the ground, `env_embed_m`, so it sits on a slope
    whose middle is at the object's ground height).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    rise = p.get("rise", 0.0)
    first = len(mb.bm.faces)
    hw, hl, H = box.hw, box.hl, box.height - rise / 2
    n = p.get("steps", 3)
    tread_w = hw * (1.4 if p.get("rope", True) else 1.9)
    x0 = -hw + tread_w / 2 + 0.05
    if p.get("ladder", False):
        mb.style(kind=0)
        for s in (-1, 1):
            mb.box((x0 + s * tread_w * 0.4, 0, 0.1), (0.07, box.length, 0.07), "Wood")
        rungs = max(3, int(box.length / 0.35))
        for i in range(rungs):
            y = -hl + box.length * (i + 0.5) / rungs
            mb.cylinder((x0 - tread_w * 0.42, y, 0.13), (x0 + tread_w * 0.42, y, 0.13), 0.022, seg=5, mat="Wood")
    else:
        for i in range(n):
            y = -hl + box.length * (i + 0.5) / n
            mb.style(kind=r.choice((0, 2)))
            mb.cylinder((x0 - tread_w / 2, y, 0.08), (x0 + tread_w / 2, y + r.uniform(-0.04, 0.04), 0.08), 0.085, seg=6,
                        mat="Wood")
            mb.style(kind=2)
            for s in (-1, 1):
                mb.box((x0 + s * tread_w * 0.38, y - 0.11, 0.1), (0.05, 0.05, 0.36), "Wood")
    if p.get("rope", True):
        x = hw - 0.08
        posts = []
        for y in (-hl + 0.15, hl - 0.15):
            mb.style(kind=2)
            mb.cylinder((x, y, -0.2), (x, y, H), 0.04, 0.035, seg=5, mat="Wood")
            posts.append(Vector((x, y, H - 0.08)))
        mb.style(kind=0)
        mid = (posts[0] + posts[1]) / 2 - Vector((0, 0, 0.12))
        mb.tube([posts[0], mid, posts[1]], [0.014, 0.014, 0.014], 4, "Canvas", cap_start=False, cap_end=False)
    if rise > 0:
        verts = {v for f in list(mb.bm.faces)[first:] for v in f.verts}
        for v in verts:
            v.co.z += rise * ((v.co.y + hl) / box.length - 0.5)
        low = -min(v.co.z for v in verts)
        model.part_props.setdefault(name, {})["env_embed_m"] = float(min(rise / 2, low))
