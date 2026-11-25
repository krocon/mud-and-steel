"""Neutral props, boundaries and field fortifications (prompt 007).

Props: crates, drums, wood pile, hay, telegraph pole. Boundaries: wooden
fence, dry-stone wall, hedge, each with segment, end and corner pieces
(segments run along Y; a corner piece joins a run along Y with one along X).
Fortifications: barbed wire, Czech hedgehog, sandbag position, MG pillbox,
trench pieces (sunken: z = 0 is the terrain surface, the trench reaches down
to -depth; prompt 010/021 cuts the matching depression), mines and minefield
signs. Military objects use the `Paint` slot where they are painted, so the
material system can recolor them.
"""

import math

from mathutils import Vector

from .env import CARD_SHRUB, CARD_SOLID, env_part, rng

# -- props ---------------------------------------------------------------------


def _crate(mb, cx, cy, z0, L, W, H, along_x=False):
    """Plank crate with end cleats and rope handles, footprint L (Y) x W (X)."""
    if along_x:
        L, W = W, L
    mb.style(kind=0)
    mb.box((cx, cy, z0 + H / 2), (W * 0.96, L * 0.96, H * 0.96), "Wood")
    # lid boards and two cleats per end
    for i in range(3):
        x = cx + (i - 1) * W / 3
        mb.box((x, cy, z0 + H - 0.01), (W / 3 - 0.01, L, 0.025), "Wood")
    for s in (-1, 1):
        if not along_x:
            mb.box((cx, cy + s * (L / 2 - 0.01), z0 + H / 2), (W, 0.03, H * 0.9), "Wood")
            mb.style(kind=0)
            mb.box((cx, cy + s * (L / 2 + 0.005), z0 + H * 0.6), (W * 0.35, 0.02, 0.03), "Canvas")
        else:
            mb.box((cx + s * (W / 2 - 0.01), cy, z0 + H / 2), (0.03, L, H * 0.9), "Wood")
            mb.box((cx + s * (W / 2 + 0.005), cy, z0 + H * 0.6), (0.02, L * 0.35, 0.03), "Canvas")


@env_part("crate")
def crate(model, name, p, box):
    """Wooden supply crate, or a stack of them.

    Parameters: `stack` (0 = single crate, 1 = two side by side with a third
    on top; default 0). For a stack the box holds two crates side by side.
    """
    mb = model.mb(name)
    if not p.get("stack", 0):
        _crate(mb, 0, 0, 0, box.length, box.width, box.height)
        return
    h = box.height / 2
    w = box.width / 2
    _crate(mb, -w / 2, 0, 0, box.length, w, h)
    _crate(mb, w / 2, 0, 0, box.length * 0.98, w, h)
    _crate(mb, 0, 0, h, box.length * 0.9, w, h)


def _drum(mb, base, axis, R, H, kind=0):
    a = Vector(base)
    d = Vector(axis).normalized()
    mb.style(kind=kind)
    mb.cylinder(a, a + d * H, R * 0.985, seg=12, mat="Paint")
    for t in (0.0, 1.0 / 3, 2.0 / 3, 1.0):
        c = a + d * (H * t)
        h = 0.012 if 0 < t < 1 else 0.02
        c0 = c - d * h if t == 1.0 else c
        mb.cylinder(c0, c0 + d * h, R, seg=12, mat="Paint", smooth=True, cap0=False, cap1=False)
    mb.style(kind=0)
    mb.cylinder(a + d * H, a + d * (H + 0.008), 0.03, seg=6, mat="Metal")


@env_part("drum")
def drum(model, name, p, box):
    """200 l steel drum: `pose` = "standing" (default), "lying" (axis along
    Y) or "group" (three standing drums). `kind` = Paint kind (default 0)."""
    mb = model.mb(name)
    pose = p.get("pose", "standing")
    kind = p.get("kind", 0)
    if pose == "standing":
        R = min(box.hw, box.hl)
        _drum(mb, (0, 0, 0), (0, 0, 1), R, box.height - 0.008, kind)
    elif pose == "lying":
        R = box.height / 2
        _drum(mb, (0, -box.hl + 0.004, R), (0, 1, 0), R, box.length - 0.008, kind)
    else:
        R = 0.292
        for (x, y) in ((-R, -R * 0.6), (R, -R * 0.6), (0, R * 1.1)):
            _drum(mb, (x, y, 0), (0, 0, 1), R, 0.868, kind)


@env_part("woodpile")
def woodpile(model, name, p, box):
    """Stacked firewood between end posts: logs along X (depth), stack along Y.

    Parameters: `seed`; `log_diameter` (default 0.24 m).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    d = p.get("log_diameter", 0.24)
    rows = max(1, int(box.height / (d * 0.9)))
    cols = max(1, int((box.length - 0.2) / d))
    for j in range(rows):
        off = d / 2 if j % 2 else 0.0
        for i in range(cols - (1 if j % 2 else 0)):
            y = -box.hl + 0.1 + d / 2 + i * d + off
            z = d / 2 + j * d * 0.87
            rr = d / 2 * r.uniform(0.85, 1.0)
            mb.style(kind=0)
            mb.cylinder((-box.hw, y, z), (box.hw, y, z), rr, seg=5, mat="Bark", cap_mat="Wood",
                        phase=r.uniform(0, 1.2))
    mb.style(kind=0)
    for s in (-1, 1):
        mb.box((0, s * (box.hl - 0.05), box.height / 2), (0.1, 0.1, box.height), "Wood")


@env_part("haystack")
def haystack(model, name, p, box):
    """Loose haystack: `shape` = "rick" (long loaf with a ridge, default) or
    "round" (cone-topped stack). Hay uses the Roof slot, kind thatch."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    mb.style(kind=2)
    if p.get("shape", "rick") == "round":
        mb.blob((0, 0, box.height * 0.2), (box.hw * 1.05, box.hl * 1.05, box.height * 0.8), "Roof", seg=12,
                rings=8, rng=r, rough=0.06, flat_bottom=0.0, smooth=True)
        mb.cylinder((0, 0, box.height * 0.8), (0, 0, box.height * 1.02), 0.04, seg=4, mat="Wood")
    else:
        # broad loaf: ellipsoid centered low and cut by the ground
        mb.blob((0, 0, box.height * 0.15), (box.hw * 1.05, box.hl, box.height * 0.82), "Roof", seg=12, rings=8,
                rng=r, rough=0.05, flat_bottom=0.0, smooth=True)
        mb.style(kind=2)
        mb.extrude_x([(-box.hl * 0.8, box.height * 0.75), (box.hl * 0.8, box.height * 0.75),
                      (box.hl * 0.6, box.height), (-box.hl * 0.6, box.height)], -box.hw * 0.35, box.hw * 0.35,
                     "Roof", smooth=True)


@env_part("hay_bale")
def hay_bale(model, name, p, box):
    """Small rectangular (two-string) hay bale; `stack` = 0 single, 1 = a
    stack of six (3 / 2 / 1). The box holds the whole stack."""
    mb = model.mb(name)

    def bale(cx, cy, cz, L, W, H):
        mb.style(kind=2)
        mb.box((cx, cy, cz + H / 2), (W, L, H), "Roof", bevel=0.03)
        for s in (-1, 1):
            mb.style(kind=0)
            mb.box((cx + s * W * 0.25, cy, cz + H / 2), (0.012, L + 0.008, H + 0.008), "Canvas")

    if not p.get("stack", 0):
        bale(0, 0, 0, box.length, box.width, box.height)
        return
    w, h = box.width / 3, box.height / 3
    for row, n in enumerate((3, 2, 1)):
        for i in range(n):
            x = (i - (n - 1) / 2) * w
            bale(x, 0, row * h, box.length * (1 - 0.03 * row), w * 0.98, h * 0.99)


@env_part("telegraph_pole")
def telegraph_pole(model, name, p, box):
    """Wooden pole with cross-arm, pins and glass insulators.

    Parameters: `insulators` (default 4). Adds `Socket_Wire_NN` at the
    insulator tops for the wires (prompt 013), cross-arm along X.
    """
    mb = model.mb(name)
    H = box.height
    rb = min(box.hl, 0.125)
    mb.style(kind=2)
    mb.cylinder((0, 0, 0), (0, 0, H - 0.12), rb, rb * 0.72, seg=8, mat="Wood", cap0=False)
    mb.blob((0, 0, H - 0.12), (rb * 0.72, rb * 0.72, 0.06), "Wood", seg=8, rings=3)
    arm_z = H - 0.45
    arm_x = rb * 0.72 + 0.05
    mb.box((arm_x, 0, arm_z), (0.09, box.length, 0.11), "Wood")
    n = p.get("insulators", 4)
    for i in range(n):
        y = -box.hl * 0.9 + box.length * 0.9 * i / max(n - 1, 1)
        if abs(y) < 0.15:
            y = 0.15 * (1 if y >= 0 else -1)
        x = arm_x
        mb.style(kind=0)
        mb.cylinder((x, y, arm_z + 0.055), (x, y, arm_z + 0.16), 0.012, seg=4, mat="Metal")
        mb.cylinder((x, y, arm_z + 0.12), (x, y, arm_z + 0.24), 0.04, 0.03, seg=6, mat="Glass")
        model.socket(model.next_index("Socket_Wire_"), name, (x, y, arm_z + 0.22))
    # diagonal brace
    mb.style(kind=2)
    mb.cylinder((rb * 0.72, 0, arm_z - 0.6), (arm_x, 0.35, arm_z - 0.05), 0.02, seg=4, mat="Metal")


# -- boundaries ---------------------------------------------------------------------


@env_part("fence_wood")
def fence_wood(model, name, p, box):
    """Post-and-rail fence. `piece` = "segment" (post at -Y, rails to +Y;
    segments chain), "end" (closing post) or "corner" (corner post).
    `rails` (default 3), `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "segment")
    post = box.width
    H = box.height
    mb.style(kind=0)
    if piece in ("end", "corner"):
        mb.box((0, 0, H / 2), (post, box.length, H), "Wood", bevel=0.01)
        mb.box((0, 0, H + 0.0), (post * 0.8, box.length * 0.8, 0.001), "Wood")
        return
    y0 = -box.hl
    mb.box((0, y0 + post / 2, H / 2), (post, post, H), "Wood", bevel=0.01)
    rails = p.get("rails", 3)
    for i in range(rails):
        z = H * (0.3 + 0.6 * i / max(rails - 1, 1)) - 0.05
        sag = r.uniform(-0.03, 0.01)
        mb.style(kind=0 if i % 2 else 2)
        mb.extrude_x([(y0 + post, z - 0.045), (box.hl, z - 0.045 + sag), (box.hl, z + 0.045 + sag),
                      (y0 + post, z + 0.045)], post * 0.05, post * 0.45, "Wood")


def _stones_face(mb, r, y0, y1, x_of_z, H, side, kind=0):
    """Irregular stone blocks on one face of a battered wall."""
    z = 0.0
    while z < H - 0.05:
        ch = r.uniform(0.18, 0.3)
        y = y0
        while y < y1 - 0.05:
            w = min(r.uniform(0.3, 0.5), y1 - y)
            zc = z + ch / 2
            x = x_of_z(zc) * side
            mb.style(kind=kind)
            mb.oriented_box((x, y + w / 2, zc), (0, 1, 0), (0, 0, 1),
                            (w * 0.96, ch * 0.92, r.uniform(0.08, 0.14)), "Stone")
            y += w
        z += ch


@env_part("wall_stone")
def wall_stone(model, name, p, box):
    """Dry-stone wall: battered core (top half the base width), face stones
    and coping. `piece` = "segment", "end" (short closing piece with an end
    face) or "corner" (square pier). `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "segment")
    H = box.height
    hb = box.hw
    ht = hb * 0.55
    mb.style(kind=0)
    if piece == "corner":
        mb.prism([(-hb, -box.hl), (hb, -box.hl), (hb, box.hl), (-hb, box.hl)], 0, H * 0.92, "Stone")
        mb.prism([(-hb * 1.02, -box.hl * 1.02), (hb * 1.02, -box.hl * 1.02), (hb * 1.02, box.hl * 1.02),
                  (-hb * 1.02, box.hl * 1.02)], H * 0.92, H, "Stone", bevel=0.03)
        return
    prof = [(-hb, 0), (hb, 0), (ht, H * 0.9), (-ht, H * 0.9)]
    y0, y1 = -box.hl, box.hl
    mb.extrude([(x, y0, z) for x, z in prof], (0, y1 - y0, 0), "Stone")

    def xz(z):
        return hb + (ht - hb) * (z / (H * 0.9)) - 0.03

    _stones_face(mb, r, y0, y1, lambda z: xz(min(z, H * 0.85)), H * 0.88, 1)
    _stones_face(mb, r, y0, y1, lambda z: xz(min(z, H * 0.85)), H * 0.88, -1)
    # coping stones on edge
    y = y0
    while y < y1 - 0.05:
        w = min(r.uniform(0.15, 0.25), y1 - y)
        mb.oriented_box((0, y + w / 2, H * 0.95), (1, 0, 0), (0, 0, 1), (ht * 2.2, H * 0.1, w * 0.9), "Stone")
        y += w


def _hedge_lobe(mb, center, rx, ry, rz, r, seg=8, rings=4):
    """Organic bumpy pillow dome sitting on the ground for hedge cores (prompt 218)."""
    cx, cy, cz = center
    ring_pts = []
    # Bulge smoothly outward from z=0 like a rounded bush pillow, no vertical walls.
    elev_c = rz * 0.26
    for k in range(rings + 1):
        phi = -0.34 + (math.pi * 0.5 + 0.34) * (k / rings)
        cp = math.cos(phi)
        sp = math.sin(phi)
        pts = []
        for s in range(seg):
            a = 2.0 * math.pi * s / seg
            bump = 1.0 + r.uniform(-0.14, 0.14) if 0 < k < rings else (1.0 + r.uniform(-0.08, 0.08))
            px = cx + math.cos(a) * rx * cp * bump
            py = cy + math.sin(a) * ry * cp * bump
            pz = max(0.0, cz + elev_c + (rz - elev_c) * sp * bump)
            pts.append((px, py, pz))
        ring_pts.append(pts)
    mb.loft(ring_pts, "Foliage", cap_start=True, cap_end=True, smooth=True)


@env_part("hedge")
def hedge(model, name, p, box):
    """Field / bocage hedge (prompt 007, 218): lumpy overlapping opaque
    foliage mounds wrapped in dense shrub leaf cards, no rigid box core.
    `piece` = "segment", "end" (rounded end) or "corner". `seed`, `cards`
    (default 44), `lobes` (default 5), `kind` (0 standard, 3 dark bocage)."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    piece = p.get("piece", "segment")
    kind = int(p.get("kind", 0))
    mb.style(kind=kind, wind=True)
    lobes = int(p.get("lobes", 5 if piece == "segment" else (3 if piece == "corner" else 2)))
    # Overlapping organic pillow lobes along Y (or clustered for corner/end).
    for i in range(lobes):
        if piece == "corner":
            a = i * (2.0 * math.pi / max(lobes, 1)) + r.uniform(-0.12, 0.12)
            rad = min(box.hw, box.hl) * 0.22
            cx = math.cos(a) * rad
            cy = math.sin(a) * rad
            rx = box.hw * r.uniform(0.66, 0.76)
            ry = box.hl * r.uniform(0.66, 0.76)
        elif lobes == 1:
            cx, cy = 0.0, 0.0
            rx, ry = box.hw * 0.88, box.hl * 0.88
        else:
            t = i / (lobes - 1) - 0.5
            span_y = box.hl * (0.58 if piece == "end" else 0.78)
            cx = r.uniform(-box.hw * 0.14, box.hw * 0.14)
            cy = t * 2.0 * span_y + r.uniform(-0.08, 0.08)
            rx = box.hw * r.uniform(0.76, 0.92)
            ry = max(box.hl / lobes * 1.45, box.hw * 0.82) * r.uniform(0.9, 1.08)
        rz = H * r.uniform(0.78, 0.94)
        mb.style(phase=i + 1)
        _hedge_lobe(mb, (cx, cy, 0.0), rx, ry, rz, r, seg=8, rings=4)
    # Assign triplanar-like world-scaled UVs in the CARD_SOLID atlas cell so
    # there are no stretched stripes on any lobe face.
    for f in mb.bm.faces:
        if not f[mb.uvflag]:
            f[mb.uvflag] = 1
            nrm = f.normal
            for loop in f.loops:
                co = loop.vert.co
                if abs(nrm.z) > 0.6:
                    u, v = co.x * 0.55, co.y * 0.55
                elif abs(nrm.x) > abs(nrm.y):
                    u, v = co.y * 0.55, co.z * 0.55
                else:
                    u, v = co.x * 0.55, co.z * 0.55
                loop[mb.uv].uv = (CARD_SOLID + (u % 1.0), v % 1.0)
    # Lush outer leaf cards wrapping sides (down to the base) and crown top.
    n = int(p.get("cards", 44))
    for i in range(n):
        side = 1 if i % 2 else -1
        y = r.uniform(-box.hl * 0.86, box.hl * 0.86)
        z = r.uniform(H * 0.15, H * 0.80)
        x = side * box.hw * r.uniform(0.56, 0.82)
        mb.style(phase=10 + i)
        s = H * r.uniform(0.40, 0.58)
        tilt_z = r.uniform(-0.32, 0.32)
        mb.card((x, y, z), (r.uniform(-0.25, 0.25), 1.0, tilt_z), (side * 0.35, r.uniform(-0.2, 0.2), 1.0),
                s * 1.25, s, CARD_SHRUB)
    top_n = max(6, n // 4)
    for i in range(top_n):
        y = r.uniform(-box.hl * 0.82, box.hl * 0.82)
        x = r.uniform(-box.hw * 0.42, box.hw * 0.42)
        z = H * r.uniform(0.76, 0.92)
        mb.style(phase=80 + i)
        s = max(box.width, H * 0.62) * r.uniform(0.72, 0.98)
        mb.card((x, y, z), (r.uniform(-0.4, 0.4), 1.0, r.uniform(-0.2, 0.2)),
                (1.0, r.uniform(-0.3, 0.3), r.uniform(0.15, 0.45)), s, s * 0.9, CARD_SHRUB)


# -- fortifications -----------------------------------------------------------------


def _wire(mb, a, b, thick=0.012):
    a, b = Vector(a), Vector(b)
    mb.cylinder(a, b, thick / 2, seg=3, mat="Metal", cap0=False, cap1=False, smooth=False)


@env_part("barbed_wire")
def barbed_wire(model, name, p, box):
    """Double-apron fence section along Y: center pickets at both ends,
    anchor pickets on both sides, apron and line wires (FM 5-30 type).
    `wires` = line wires on the fence (default 4)."""
    mb = model.mb(name)
    H = box.height
    mb.style(kind=1)
    ys = (-box.hl + 0.03, box.hl - 0.03)
    for y in ys:
        mb.cylinder((0, y, 0), (0, y, H), 0.03, seg=5, mat="Wood")
        for s in (-1, 1):
            mb.cylinder((s * (box.hw - 0.03), y, 0), (s * (box.hw - 0.03), y, 0.25), 0.025, seg=4, mat="Wood")
    mb.style(kind=1)
    n = p.get("wires", 4)
    for i in range(n):
        z = H * (0.15 + 0.85 * i / max(n - 1, 1)) - 0.02
        _wire(mb, (0, ys[0], z), (0, ys[1], z))
    for s in (-1, 1):
        for y in ys:
            _wire(mb, (0, y, H - 0.03), (s * (box.hw - 0.03), y, 0.2))
        for k in range(3):
            t = (k + 1) / 4
            x = s * (box.hw - 0.03) * t
            z = (H - 0.03) * (1 - t) + 0.2 * t
            _wire(mb, (x, ys[0], z), (x, ys[1], z))
        # diagonal apron wires
        _wire(mb, (0, ys[0], H - 0.05), (s * (box.hw - 0.03) * 0.5, ys[1], H * 0.55))


@env_part("hedgehog")
def hedgehog(model, name, p, box):
    """Czech hedgehog: three steel angle beams crossing at their middles.
    `leg` = angle leg in m (L 140/140/13 -> 0.14, default)."""
    mb = model.mb(name)
    leg = p.get("leg", 0.14)
    t = 0.013
    L = 1.8
    c = Vector((0, 0, 1.0))
    mb.style(kind=3)
    # three mutually perpendicular beams with the cube diagonal vertical: it
    # stands on three beam ends
    dirs = []
    for i in range(3):
        a = 2 * math.pi * i / 3
        dirs.append(Vector((math.cos(a) * math.sqrt(2 / 3), math.sin(a) * math.sqrt(2 / 3), math.sqrt(1 / 3))))
    for d in dirs:
        d = d.normalized()
        side = d.cross(Vector((0, 0, 1))).normalized()
        n2 = side.cross(d).normalized()
        mb.oriented_box(c + side * (leg / 2), d, side, (L, leg, t), "Paint")
        mb.oriented_box(c + n2 * (leg / 2), d, n2, (L, leg, t), "Paint")


@env_part("sandbag_position")
def sandbag_position(model, name, p, box):
    """Horseshoe sandbag breastwork, open at the rear (-Y): earth core and
    one visible layer of bags in courses. `courses` (default 8), `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    thick = 0.9
    courses = p.get("courses", 8)
    ch = H / courses
    # plan of the outer face: half ellipse front + straight sides
    outer = []
    for i in range(13):
        a = math.pi * i / 12
        outer.append((math.cos(a) * box.hw, max(0.0, math.sin(a)) * box.hl * 1.0 + 0.0 - box.hl * 0.0))
    # shift so that the plan spans -hl..hl in Y: arc from y=-hl (ends) to y=+hl (front)
    outer = [(x, -box.hl + (y / box.hl) * (2 * box.hl)) for x, y in outer]
    for c in range(courses):
        z0 = c * ch
        shrink = 0.04 * c
        for i in range(len(outer) - 1):
            ax, ay = outer[i]
            bx, by = outer[i + 1]
            seg = Vector((bx - ax, by - ay, 0))
            n = max(1, int(seg.length / 0.48 + 0.5))
            for k in range(n):
                t = (k + 0.5 + (0.5 if c % 2 else 0)) / n
                if t > 1:
                    continue
                pos = Vector((ax, ay, 0)) + seg * t
                inward = Vector((-pos.x, -pos.y - box.hl * 0.2, 0)).normalized()
                pos += inward * (0.12 + shrink)
                mb.style(kind=0)
                mb.oriented_box((pos.x, pos.y, z0 + ch / 2), seg, (0, 0, 1),
                                (seg.length / n * 0.98, ch * 0.95, 0.25 * r.uniform(0.95, 1.05)), "Canvas")
    # earth core between the bag faces
    mb.style(kind=1)
    inner = [(x * (1 - thick / box.hw), -box.hl + (y + box.hl) * (1 - thick / (2 * box.hl)))
             for x, y in outer]
    ring_top = mb.verts([(x * 0.9, -box.hl + (y + box.hl) * 0.93, H * 0.97) for x, y in outer])
    iring = mb.verts([(x, y, 0) for x, y in inner])
    iring_top = mb.verts([(x, y, H * 0.97) for x, y in inner])
    for i in range(len(outer) - 1):
        mb.face([ring_top[i], ring_top[i + 1], iring_top[i + 1], iring_top[i]][::-1], "Ground")
        mb.face([iring[i], iring[i + 1], iring_top[i + 1], iring_top[i]], "Canvas")


@env_part("pillbox")
def pillbox(model, name, p, box):
    """Concrete MG pillbox, hexagonal plan (rear wall with entrance, five
    walls with embrasures), roof slab with overhang. `wall` = wall thickness
    (default 0.3 m, bulletproof Type 22 standard)."""
    mb = model.mb(name)
    H = box.height
    R = box.hw
    wall = p.get("wall", 0.3)
    roof_t = 0.3
    mb.style(kind=2)
    pts = [(math.cos(i * math.pi / 3) * R * 0.94, math.sin(i * math.pi / 3) * R * 0.94)
           for i in range(6)]
    # walls as six slabs with an embrasure gap (front five) or a door (rear)
    for i in range(6):
        a = Vector((*pts[i], 0))
        b = Vector((*pts[(i + 1) % 6], 0))
        d = b - a
        mid = (a + b) / 2
        n = Vector((mid.x, mid.y, 0)).normalized()
        L = d.length
        z_emb0, z_emb1 = 1.05, 1.35
        rear = i == 4  # wall facing -Y
        c = mid - n * (wall / 2)
        if rear:
            # door 0.7 m wide, 1.6 m high
            for s in (-1, 1):
                off = d.normalized() * s * (L / 4 + 0.175)
                mb.oriented_box(c + off + Vector((0, 0, (H - roof_t) / 2)), d, (0, 0, 1),
                                (L / 2 - 0.35, H - roof_t, wall), "Stone")
            mb.oriented_box(c + Vector((0, 0, 1.6 + (H - roof_t - 1.6) / 2)), d, (0, 0, 1),
                            (0.7, H - roof_t - 1.6, wall), "Stone")
            continue
        mb.oriented_box(c + Vector((0, 0, z_emb0 / 2)), d, (0, 0, 1), (L, z_emb0, wall), "Stone")
        mb.oriented_box(c + Vector((0, 0, (z_emb1 + H - roof_t) / 2)), d, (0, 0, 1),
                        (L, H - roof_t - z_emb1, wall), "Stone")
        for s in (-1, 1):
            off = d.normalized() * s * (L / 4 + 0.3)
            mb.oriented_box(c + off + Vector((0, 0, (z_emb0 + z_emb1) / 2)), d, (0, 0, 1),
                            (L / 2 - 0.6, z_emb1 - z_emb0, wall), "Stone")
    roof = [(math.cos(i * math.pi / 3) * R, math.sin(i * math.pi / 3) * R)
            for i in range(6)]
    mb.prism(roof, H - roof_t, H, "Stone", bevel=0.04)
    mb.style(kind=0)
    mb.prism([(x * 0.8, y * 0.8) for x, y in pts], 0.0, 0.05, "Ground")


def _trench_arm(mb, r, y0, y1, depth, w_top, w_bot, para_w, para_h, x_axis=False, closed0=False,
                closed1=False, revet=True, gap=None):
    """One straight trench arm between y0 and y1 (or along X)."""

    def P(x, y, z):
        return (y, x, z) if x_axis else (x, y, z)

    def runs(s):
        # `gap` = (ya, yb): opening in the +side wall where a branch joins
        if s > 0 and gap:
            return [(a, b) for a, b in ((y0, gap[0]), (gap[1], y1)) if b - a > 0.01]
        return [(y0, y1)]

    flip = x_axis
    # side walls (sloped), floor, duckboards
    for s in (-1, 1):
        mb.style(kind=0)
        for ya, yb in runs(s):
            q = [mb.verts([P(s * w_top / 2, ya, 0)])[0], mb.verts([P(s * w_top / 2, yb, 0)])[0],
                 mb.verts([P(s * w_bot / 2, yb, -depth)])[0], mb.verts([P(s * w_bot / 2, ya, -depth)])[0]]
            mb.face(q if (s > 0) != flip else q[::-1], "Ground")
    q = [mb.verts([P(-w_bot / 2, y0, -depth)])[0], mb.verts([P(-w_bot / 2, y1, -depth)])[0],
         mb.verts([P(w_bot / 2, y1, -depth)])[0], mb.verts([P(w_bot / 2, y0, -depth)])[0]]
    mb.face(q[::-1] if not flip else q, "Ground")
    mb.style(kind=0)
    n = max(1, int(abs(y1 - y0) / 1.2))
    for i in range(n):
        y = y0 + (y1 - y0) * (i + 0.5) / n
        mb.box(P(0, y, -depth + 0.06), (w_bot * 0.8, abs(y1 - y0) / n * 0.95, 0.04) if not x_axis else
               (abs(y1 - y0) / n * 0.95, w_bot * 0.8, 0.04), "Wood")
        if revet:
            for s in (-1, 1):
                if s > 0 and gap and gap[0] - 0.1 < y < gap[1] + 0.1:
                    continue
                x = s * (w_bot / 2 + (w_top - w_bot) / 4)
                mb.cylinder(P(x, y, -depth), P(s * (w_top / 2 - 0.02), y, 0.05), 0.04, seg=4, mat="Wood")
    # spoil parapets on both sides
    for s in (-1, 1):
        mb.style(kind=1)
        x0 = s * w_top / 2
        x1 = s * (w_top / 2 + para_w)
        # 2 mm above the surface: not part of the cut (sizes are measured at z <= 0)
        prof = [(x0, 0.002), (x1, 0.002), (s * (w_top / 2 + para_w * 0.45), para_h),
                (s * (w_top / 2 + 0.1), para_h * 0.8)]
        for ya, yb in runs(s):
            a = mb.verts([P(x, ya, z) for x, z in prof])
            b = mb.verts([P(x, yb, z) for x, z in prof])
            for i in range(3):
                j = i + 1
                q = [a[i], a[j], b[j], b[i]]
                mb.face(q if (s > 0) == flip else q[::-1], "Ground")
            # end faces of the parapet, facing away from the run
            out = Vector(P(0, -1, 0)) - Vector(P(0, 0, 0))
            for ring, d in ((a, out), (b, -out)):
                f = mb.face([ring[1], ring[2], ring[3], ring[0]], "Ground")
                f.normal_update()
                if f.normal.dot(d) < 0:
                    f.normal_flip()
    for closed, y in ((closed0, y0), (closed1, y1)):
        if closed:
            q = [mb.verts([P(-w_top / 2, y, 0)])[0], mb.verts([P(w_top / 2, y, 0)])[0],
                 mb.verts([P(w_bot / 2, y, -depth)])[0], mb.verts([P(-w_bot / 2, y, -depth)])[0]]
            mb.face(q, "Ground")


@env_part("trench")
def trench(model, name, p, box):
    """Fire trench piece on a 5 m grid, sunk below z = 0 (FM 5-15 profile:
    1.2 m at the top, 0.6 m at the bottom, 1.7 m deep, spoil parapets 0.9 m
    wide). `piece` = "straight" (along Y), "end" (from -Y to the cell
    center, closed), "corner" (-Y to center, then to +X), "t" (along Y with a
    branch to +X). `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "straight")
    depth = 1.7
    w_top, w_bot = 1.2, 0.6
    para_w, para_h = 0.9, 0.35
    half = 2.5
    if piece == "straight":
        _trench_arm(mb, r, -half, half, depth, w_top, w_bot, para_w, para_h)
    elif piece == "end":
        _trench_arm(mb, r, -half, 0.0, depth, w_top, w_bot, para_w, para_h, closed1=True)
    elif piece == "corner":
        _trench_arm(mb, r, -half, w_top / 2, depth, w_top, w_bot, para_w, para_h, closed1=True,
                    gap=(-w_top / 2, w_top / 2))
        _trench_arm(mb, r, w_top / 2, half, depth, w_top, w_bot, para_w, para_h, x_axis=True)
    elif piece == "t":
        _trench_arm(mb, r, -half, half, depth, w_top, w_bot, para_w, para_h, gap=(-w_top / 2, w_top / 2))
        _trench_arm(mb, r, w_top / 2, half, depth, w_top, w_bot, para_w, para_h, x_axis=True)
    else:
        raise ValueError(f"trench: unknown piece {piece!r}")
    model.part_props[name] = {"terrain_cut": "trench", "cut_depth_m": depth, "cut_width_top_m": w_top,
                              "cut_width_bottom_m": w_bot, "trench_piece": piece}


@env_part("mine_at")
def mine_at(model, name, p, box):
    """Anti-tank pressure mine (Teller-type disc with central fuze and
    carrying handle). Lies on the surface (visible for engineers / debug)."""
    mb = model.mb(name)
    R = min(box.hw, box.hl)
    H = box.height
    mb.style(kind=0)
    mb.revolve((0, 0, 0), (0, 0, 1), [(0.0, R * 0.92), (H * 0.15, R), (H * 0.62, R), (H * 0.8, R * 0.7),
                                       (H * 0.8, R * 0.3), (H * 0.82, 0.0)], seg=14, mat="Paint")
    mb.cylinder((0, 0, H * 0.78), (0, 0, H), R * 0.2, seg=8, mat="Paint")
    mb.style(kind=3)
    mb.box((R * 0.8, 0, H * 0.45), (0.04, 0.12, 0.02), "Metal")


@env_part("mine_ap")
def mine_ap(model, name, p, box):
    """Bounding anti-personnel mine (S-mine type): cylinder with fuze rod
    and three prongs, normally buried up to the prongs."""
    mb = model.mb(name)
    R = min(box.hw, box.hl)
    H = box.height
    body = H * 0.8
    mb.style(kind=0)
    mb.cylinder((0, 0, 0), (0, 0, body), R, seg=10, mat="Paint")
    mb.style(kind=3)
    mb.cylinder((0, 0, body), (0, 0, H * 0.92), 0.008, seg=4, mat="Metal")
    for i in range(3):
        a = i * 2 * math.pi / 3
        mb.cylinder((0, 0, H * 0.9), (math.cos(a) * 0.012, math.sin(a) * 0.012, H), 0.002, seg=3, mat="Metal")


@env_part("mine_sign")
def mine_sign(model, name, p, box):
    """Minefield warning marker on a picket, without inscriptions:
    `shape` = "triangle" (point down, red with white border) or "board"
    (white board with a red triangle)."""
    mb = model.mb(name)
    H = box.height
    w = box.width
    mb.style(kind=1)
    mb.box((0, 0, H * 0.4), (0.05, 0.05, H * 0.8), "Wood")
    top = H
    mb.style(kind=2)
    if p.get("shape", "triangle") == "triangle":
        tri = [(-w / 2, top), (w / 2, top), (0, top - w * 0.87)]
        mb.extrude([(x, -0.03, z) for x, z in tri], (0, -0.012, 0), "Paint")
        inner = [(-w * 0.38, top - w * 0.07), (w * 0.38, top - w * 0.07), (0, top - w * 0.73)]
        mb.style(kind=1)
        mb.extrude([(x, -0.042, z) for x, z in inner], (0, -0.004, 0), "Paint")
    else:
        bh = w * 0.6
        mb.box((0, -0.04, top - bh / 2), (w, 0.02, bh), "Paint")
        mb.style(kind=1)
        tri = [(-bh * 0.35, top - bh * 0.15), (bh * 0.35, top - bh * 0.15), (0, top - bh * 0.85)]
        mb.extrude([(x, -0.051, z) for x, z in tri], (0, -0.004, 0), "Paint")


# -- prompt 052: camouflage net, concrete teeth ---------------------------------


@env_part("camo_net")
def camo_net(model, name, p, box):
    """Garnished drape net on poles: a square net (the box footprint) that
    sags between a center pole and four corner poles, with dry and green
    foliage cards as garnish and the net edges pegged down. `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.width / 2, box.length / 2, box.height
    # Sagging net surface: highest over the center pole, edges lower.

    def height(u, v):
        d = max(abs(u), abs(v))
        sag = 0.25 * math.sin(math.pi * abs(u)) * math.sin(math.pi * abs(v))
        return H - 0.9 * d * d - sag + r.uniform(-0.04, 0.04)

    # The net itself is not drawn as a sheet: dense garnish cards make it
    # (gaps between them read as the netting).
    # Garnish: cards lying on the net, alternating green and dry.
    for k in range(320):
        u, v = r.uniform(-0.97, 0.97), r.uniform(-0.97, 0.97)
        z = height(u, v) + 0.03
        a = r.uniform(0, math.pi)
        right = Vector((math.cos(a), math.sin(a), 0))
        up = Vector((-math.sin(a), math.cos(a), r.uniform(0.1, 0.4)))
        mb.style(kind=(0, 3, 0, 2)[k % 4], phase=k, wind=True)
        mb.card((u * hw, v * hl, z), right, up, r.uniform(1.0, 1.6), r.uniform(0.7, 1.2), CARD_SHRUB,
                mat="Foliage")
    mb.style(kind=0, wind=False)
    # Poles: one in the middle, four at the corners (shorter), guy pegs.
    mb.cylinder((0, 0, 0), (0, 0, H), 0.05, seg=6, mat="Wood")
    for sx in (-1, 1):
        for sy in (-1, 1):
            x, y = sx * hw * 0.8, sy * hl * 0.8
            top = height(sx * 0.8, sy * 0.8)
            mb.cylinder((x, y, 0), (x, y, top), 0.04, seg=6, mat="Wood")
            mb.cylinder((sx * hw, sy * hl, 0), (sx * hw * 1.02, sy * hl * 1.02, 0.25), 0.02, seg=4, mat="Wood")


@env_part("dragon_tooth")
def dragon_tooth(model, name, p, box):
    """Concrete anti-tank tooth: a truncated square pyramid on a low footing,
    slightly irregular (cast in place). `seed`."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    hw, hl, H = box.width / 2, box.length / 2, box.height
    foot = 0.12
    mb.style(kind=2)
    mb.box((0, 0, foot / 2), (box.width, box.length, foot), "Stone")
    top_w, top_l = hw * 0.32, hl * 0.32
    lean = (r.uniform(-0.03, 0.03), r.uniform(-0.03, 0.03))
    low = [(-hw * 0.88, -hl * 0.88, foot), (hw * 0.88, -hl * 0.88, foot), (hw * 0.88, hl * 0.88, foot),
           (-hw * 0.88, hl * 0.88, foot)]
    high = [(x * top_w / (hw * 0.88) + lean[0], y * top_l / (hl * 0.88) + lean[1], H) for x, y, _ in low]
    lv = mb.verts(low)
    hv = mb.verts(high)
    for i in range(4):
        j = (i + 1) % 4
        mb.face([lv[i], lv[j], hv[j], hv[i]], "Stone")
    mb.face(hv, "Stone")
