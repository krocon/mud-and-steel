"""Grown rock (prompt 074): angular blocks, block groups, ridge pieces, slabs.

The building block is `rock_block`: a convex hull of jittered points in a box
whose upper corners are chopped off by random planes (fracture faces), with
the near-coplanar hull triangles merged and the sharp edges bevelled (edge
wear in vertex color G). Its foot reaches `embed` meters below the ground,
so it sits in a slope without a gap; the bottom cap is left out (never
seen). Blocks stand on rectangular cells, so a group covers its rectangle
at the ground (the blocking box of `data/objects.toml`), and leave dark
joints between them (the AO bake darkens the clefts).

Generators (blueprint `part`):

- `rock`: one loose stone (env_rock; the deco stones and river stones).
- `rock_slab`: a flat jointed sheet, split by cracks into slabs.
- `rock_outcrop`: one to four blocks, split, leaning or stacked in layers.
- `rock_massif`: a ridge piece: a row (or two) of large blocks, high in the
  middle and low at the ends so pieces overlap into a ridge, with blocks
  stacked on top; builds a `<Variant>_Far` mesh (unbevelled, no chips).

All shapes come from the variant seed (`rng`), so a rebuild gives the same
model. Material slot `Rock` (env.gdshader slot 16, world-space pattern).
"""

import math

import bmesh
from mathutils import Matrix, Vector

from .env import env_part, rng

SLOT = "Rock"
# Merge hull triangles closer than this to coplanar into one face.
DISSOLVE_DEG = 9.0
# Bevel only edges whose faces meet at more than this angle.
BEVEL_MIN_DEG = 24.0


def _hull_points(tmp, points):
    """Convex hull of `points` in the bmesh `tmp` (cleared first)."""
    tmp.clear()
    verts = [tmp.verts.new(p) for p in points]
    bmesh.ops.convex_hull(tmp, input=verts, use_existing_faces=False)
    # convex_hull leaves interior points as loose verts
    loose = [v for v in tmp.verts if not v.link_faces]
    if loose:
        bmesh.ops.delete(tmp, geom=loose, context="VERTS")


def _clip(points, plane_co, plane_no):
    """Points of the convex hull of `points` on the inner side of a plane."""
    tmp = bmesh.new()
    _hull_points(tmp, points)
    bmesh.ops.bisect_plane(tmp, geom=list(tmp.verts) + list(tmp.edges) + list(tmp.faces),
                           plane_co=plane_co, plane_no=plane_no, clear_outer=True, dist=1e-5)
    out = [v.co.copy() for v in tmp.verts]
    tmp.free()
    return out


def rock_block(mb, x0, y0, x1, y1, height, r, embed, inset=0.25, taper=0.22, cuts=3,
               bevel=0.06, lean=(0.0, 0.0), yaw=0.0, kind=0, z0=0.0, points=None, chop=(0.2, 0.75)):
    """One angular block on the cell `x0..x1, y0..y1` (meters), `height` above
    `z0` (0 = the ground), foot down to `-embed`.

    `inset`: how far (m) the points near the ground stay inside the cell;
    `taper`: how much narrower the top is (fraction of the cell); `cuts`:
    fracture planes over the upper corners; `bevel`: chamfer width (m), 0 =
    sharp; `lean` (dx, dy): shear of the top in meters; `yaw` (radians)
    about the cell center; `chop`: range of the fracture planes between the
    highest foot point (0) and the top corner (1), higher = smaller chips.
    Returns the faces (empty if degenerate).
    """
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    hx, hy = (x1 - x0) / 2, (y1 - y0) / 2
    m = min(hx, hy)
    pts = []
    # stacked blocks reach a little into the block below
    foot = z0 - embed if z0 <= 0.0 else z0 - min(0.3, height * 0.2)
    # Foot: the cell corners exactly (the footprint).
    for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
        pts.append(Vector((sx * hx, sy * hy, foot)))
    n_ring = points or 5
    # (height fraction, inset in m, share of the lean)
    rings = ((0.12, inset, 0.1), (0.5, inset + taper * m * 0.5, 0.5), (0.86, inset + taper * m, 0.85))
    for frac, ins, shear in rings:
        for k in range(n_ring):
            a = 2 * math.pi * (k + r.uniform(-0.3, 0.3)) / n_ring
            dx, dy = math.cos(a), math.sin(a)
            s = min(hx / max(abs(dx), 1e-6), hy / max(abs(dy), 1e-6))
            d = Vector((dx * s, dy * s, 0.0))
            pull = min(ins * r.uniform(0.2, 1.0), m * 0.6)
            q = d * max(0.0, 1.0 - pull / max(d.length, 1e-6))
            z = z0 + height * (frac + r.uniform(-0.1, 0.1))
            pts.append(Vector((q.x + lean[0] * shear, q.y + lean[1] * shear, z)))
    for _ in range(2):
        pts.append(Vector((r.uniform(-0.45, 0.45) * hx * (1 - taper) + lean[0],
                           r.uniform(-0.45, 0.45) * hy * (1 - taper) + lean[1],
                           z0 + height * r.uniform(0.85, 1.0))))
    # Fracture planes chop the upper corners and flanks, never the foot.
    low = [p for p in pts if p.z < z0 + height * 0.22]
    for _ in range(cuts):
        a = r.uniform(0, 2 * math.pi)
        e = math.radians(r.uniform(8, 70))
        d = Vector((math.cos(a) * math.cos(e), math.sin(a) * math.cos(e), math.sin(e)))
        m_low = max(p.dot(d) for p in low)
        m_high = max(p.dot(d) for p in pts)
        if m_high - m_low < 0.05:
            continue
        off = m_low + (m_high - m_low) * r.uniform(*chop)
        clipped = _clip(pts, d * off, d)
        if len(clipped) >= 4:
            pts = clipped
    # Yaw about the cell center, then move onto the cell.
    rot = Matrix.Rotation(yaw, 3, "Z")
    pts = [rot @ p + Vector((cx, cy, 0.0)) for p in pts]
    return _emit(mb, pts, bevel, foot, kind)


def _emit(mb, pts, bevel, foot, kind):
    """Hull of `pts` into the MeshBuilder: merged faces, bevelled edges, no bottom."""
    tmp = bmesh.new()
    _hull_points(tmp, pts)
    if not tmp.faces:
        tmp.free()
        return []
    bmesh.ops.dissolve_limit(tmp, angle_limit=math.radians(DISSOLVE_DEG), use_dissolve_boundaries=False,
                             verts=list(tmp.verts), edges=list(tmp.edges))
    bmesh.ops.recalc_face_normals(tmp, faces=list(tmp.faces))
    wear_layer = tmp.faces.layers.int.new("wear")
    if bevel > 0.0:
        edges = [e for e in tmp.edges if len(e.link_faces) == 2
                 and e.calc_face_angle(0.0) > math.radians(BEVEL_MIN_DEG)
                 and min(v.co.z for v in e.verts) > foot + 0.05]
        if edges:
            res = bmesh.ops.bevel(tmp, geom=edges, offset=bevel, offset_type="OFFSET", segments=1,
                                  profile=0.5, affect="EDGES", clamp_overlap=True)
            for f in res["faces"]:
                f[wear_layer] = 1
    # Drop the bottom cap (all corners at the foot, facing down).
    bottom = [f for f in tmp.faces if f.normal.z < -0.7 and max(v.co.z for v in f.verts) < foot + 0.02]
    if bottom:
        bmesh.ops.delete(tmp, geom=bottom, context="FACES_ONLY")
    mb.style(kind=kind)
    vmap = {}
    faces = []
    for f in tmp.faces:
        vs = []
        for v in f.verts:
            if v not in vmap:
                vmap[v] = mb.bm.verts.new(v.co)
            vs.append(vmap[v])
        try:
            faces.append(mb.face(vs, SLOT, smooth=False, wear=bool(f[wear_layer])))
        except ValueError:
            pass  # duplicate face
    tmp.free()
    return faces


def _embedded(model, name, embed):
    """Records the foot depth: build_env fits the variant above the ground
    only, validate_env expects the lowest point at -embed (extras)."""
    model.part_props.setdefault(name, {})["env_embed_m"] = float(embed)
    far = f"{name}_Far"
    if far in model.parts:
        model.part_props.setdefault(far, {})["env_embed_m"] = float(embed)


def _layout_cells(box, cells):
    """Cells given in box fractions (fx0, fy0, fx1, fy1) -> meters."""
    out = []
    for fx0, fy0, fx1, fy1, *rest in cells:
        out.append((-box.hw + fx0 * box.width, -box.hl + fy0 * box.length,
                    -box.hw + fx1 * box.width, -box.hl + fy1 * box.length, *rest))
    return out


def _gap(cell, g):
    x0, y0, x1, y1, *rest = cell
    return (x0 + g, y0 + g, x1 - g, y1 - g, *rest)


def _chips(mb, box, r, count, size, embed, kind=1):
    """Loose stones at the foot, inside the box (scree)."""
    for _ in range(count):
        s = size * r.uniform(0.5, 1.0)
        side = r.randrange(4)
        t = r.uniform(-0.8, 0.8)
        if side < 2:
            x = t * (box.hw - s)
            y = (box.hl - s * 0.6) * (1 if side else -1)
        else:
            x = (box.hw - s * 0.6) * (1 if side == 3 else -1)
            y = t * (box.hl - s)
        rock_block(mb, x - s / 2, y - s / 2, x + s / 2, y + s / 2, s * r.uniform(0.35, 0.6), r, embed,
                   inset=s * 0.15, taper=0.3, cuts=1, bevel=0.0, yaw=r.uniform(0, math.pi), kind=kind, points=4)


@env_part("rock")
def rock(model, name, p, box):
    """Loose stone (env_rock), the small sibling of the outcrops.

    Parameters: `seed`; `embed` (m below ground, default 0.15 x height);
    `bevel` (m, default 0.04; 0 for the many small deco stones); `cuts`
    (default 3); `points` (per ring, default 6).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    embed = p.get("embed", box.height * 0.15)
    rock_block(mb, -box.hw, -box.hl, box.hw, box.hl, box.height, r, embed,
               inset=min(box.hw, box.hl) * 0.35, taper=0.3, cuts=p.get("cuts", 3), bevel=p.get("bevel", 0.04),
               points=p.get("points", 6), yaw=0.0)
    _embedded(model, name, embed)


def _area(poly):
    return 0.5 * abs(sum(p[0] * q[1] - q[0] * p[1] for p, q in zip(poly, poly[1:] + poly[:1])))


def _shrink(poly, g):
    """Moves every corner `g` meters towards the centroid (joint width)."""
    cx = sum(p[0] for p in poly) / len(poly)
    cy = sum(p[1] for p in poly) / len(poly)
    out = []
    for x, y in poly:
        d = math.hypot(x - cx, y - cy)
        k = max(0.0, 1.0 - g / max(d, 1e-6))
        out.append((cx + (x - cx) * k, cy + (y - cy) * k))
    return out


def _slab_piece(mb, poly, top, tilt, embed, chamfer, kind=0):
    """One slab: top face (tilted plane), chamfer ring, vertical sides down to -embed."""
    cx = sum(p[0] for p in poly) / len(poly)
    cy = sum(p[1] for p in poly) / len(poly)

    def z_at(x, y):
        return top + tilt[0] * (x - cx) + tilt[1] * (y - cy)

    inner = _shrink(poly, chamfer)
    t = mb.verts([(x, y, z_at(x, y)) for x, y in inner])
    o = mb.verts([(x, y, z_at(x, y) - chamfer * 0.8) for x, y in poly])
    b = mb.verts([(x, y, -embed) for x, y in poly])
    mb.style(kind=kind)
    mb.face(t, SLOT)
    n = len(poly)
    for i in range(n):
        j = (i + 1) % n
        mb.face([o[i], o[j], t[j], t[i]], SLOT, wear=True)
        mb.face([b[i], b[j], o[j], o[i]], SLOT)


def _keep_half(poly, a, b, c):
    """Part of a convex polygon with a x + b y <= c."""
    out = []
    n = len(poly)
    for i in range(n):
        p, q = poly[i], poly[(i + 1) % n]
        sp, sq = a * p[0] + b * p[1] - c, a * q[0] + b * q[1] - c
        if sp <= 0:
            out.append(p)
        if sp * sq < 0:
            t = sp / (sp - sq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out if len(out) >= 3 else []


def _outline(box, r, sides=12, rough=0.12):
    """Convex, roundish outline filling the box (a noisy superellipse)."""
    out = []
    for k in range(sides):
        a = 2 * math.pi * (k + r.uniform(-0.25, 0.25)) / sides
        c, s_ = math.cos(a), math.sin(a)
        e = 0.5  # superellipse exponent 2/e = 4: nearly a rectangle with round corners
        x = math.copysign(abs(c) ** e, c) * box.hw * (1 - r.uniform(0, rough))
        y = math.copysign(abs(s_) ** e, s_) * box.hl * (1 - r.uniform(0, rough))
        out.append((x, y))
    # keep it convex: hull of the points (gift wrapping on few points)
    pts = sorted(set(out))
    if len(pts) < 3:
        return out

    def cross(o, a_, b_):
        return (a_[0] - o[0]) * (b_[1] - o[1]) - (a_[1] - o[1]) * (b_[0] - o[0])

    lower, upper = [], []
    for q in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], q) <= 0:
            lower.pop()
        lower.append(q)
    for q in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], q) <= 0:
            upper.pop()
        upper.append(q)
    return lower[:-1] + upper[:-1]


@env_part("rock_slab")
def rock_slab(model, name, p, box):
    """Flat jointed rock sheet (env_rock_slab): a roundish outline cut into
    `pieces` slabs (Voronoi cells of random seeds) with joints between
    them; pieces at the rim are left out now and then, so the outline is
    ragged. Each slab is a little tilted and at its own height, chamfered
    on top; never more than the box height above the ground.

    Parameters: `seed`; `pieces` (default 5); `joint` (m, default 0.08);
    `embed` (m, default 0.25); `sunk` (fraction of slabs that lie lower,
    default 0.4); `drop` (chance to leave out a rim piece, default 0.35);
    `rough` (outline noise, default 0.2).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    embed = p.get("embed", 0.25)
    outline = _outline(box, r, rough=p.get("rough", 0.2))
    n = max(1, p.get("pieces", 5))
    seeds = [(r.uniform(-0.8, 0.8) * box.hw, r.uniform(-0.8, 0.8) * box.hl) for _ in range(n)]
    joint = p.get("joint", 0.08)
    sunk = p.get("sunk", 0.4)
    drop = p.get("drop", 0.35)
    h = box.height
    kept = 0
    for i, si in enumerate(seeds):
        poly = outline
        for j, sj in enumerate(seeds):
            if i == j or not poly:
                continue
            a, b = sj[0] - si[0], sj[1] - si[1]
            mx, my = (si[0] + sj[0]) / 2, (si[1] + sj[1]) / 2
            poly = _keep_half(poly, a, b, a * mx + b * my)
        if not poly:
            continue
        rim = max(abs(si[0]) / box.hw, abs(si[1]) / box.hl) > 0.55
        roll = r.random()
        if rim and roll < drop and kept + (n - i) > 2:
            continue
        poly = _shrink(poly, joint / 2)
        if _area(poly) < 0.05:
            continue
        top = h * (r.uniform(0.3, 0.55) if r.random() < sunk else r.uniform(0.75, 1.0))
        tilt = (r.uniform(-0.03, 0.03), r.uniform(-0.03, 0.03))
        _slab_piece(mb, poly, top, tilt, embed, min(0.07, h * 0.3), kind=0)
        kept += 1
    _embedded(model, name, embed)


# Outcrop layouts: cells in box fractions (x0, y0, x1, y1, height fraction).
OUTCROPS = {
    "single": [(0.0, 0.0, 1.0, 1.0, 1.0)],
    "split": [(0.0, 0.0, 0.52, 1.0, 1.0), (0.52, 0.0, 1.0, 0.85, 0.82)],
    "pair": [(0.0, 0.0, 1.0, 0.55, 1.0), (0.1, 0.55, 0.95, 1.0, 0.7)],
    "leaning": [(0.0, 0.0, 0.55, 0.62, 1.0), (0.48, 0.05, 1.0, 0.7, 0.86), (0.15, 0.62, 0.8, 1.0, 0.5)],
    "group": [(0.0, 0.0, 0.48, 0.52, 0.8), (0.48, 0.0, 1.0, 0.45, 1.0),
              (0.0, 0.52, 0.55, 1.0, 0.62), (0.55, 0.45, 1.0, 1.0, 0.85)],
}


@env_part("rock_outcrop")
def rock_outcrop(model, name, p, box):
    """Group of angular blocks (env_rock_outcrop) on the cells of `layout`
    (single, split, pair, leaning, group), with joints between them.
    `layers` > 1 builds every block as a stack of layers (bedding planes)
    instead. Loose chips lie at the foot.

    Parameters: `seed`; `layout` (default single); `layers` (default 1);
    `joint` (m, default 0.25); `embed` (m, default 0.8); `bevel` (m,
    default 0.09); `cuts` (per block, default 5); `chips` (default 4);
    `lean` (m of shear of leaning blocks, default 0.5).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    embed = p.get("embed", 0.8)
    bevel = p.get("bevel", 0.09)
    joint = p.get("joint", 0.25)
    layers = p.get("layers", 1)
    lean = p.get("lean", 0.5)
    layout = p.get("layout", "single")
    if layout not in OUTCROPS:
        raise KeyError(f"rock_outcrop: unknown layout {layout!r}; known: {', '.join(OUTCROPS)}")
    cells = _layout_cells(box, OUTCROPS[layout])
    for k, cell in enumerate(cells):
        x0, y0, x1, y1, hf = _gap(cell, joint / 2 if len(cells) > 1 else 0.0)
        h = box.height * hf * r.uniform(0.92, 1.0)
        # leaning: blocks tip towards the middle of the group
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        lx = -math.copysign(lean, mx) if layout == "leaning" and abs(mx) > 0.2 else 0.0
        ly = -math.copysign(lean * 0.5, my) if layout == "leaning" and abs(my) > 0.2 else 0.0
        if layers <= 1:
            rock_block(mb, x0, y0, x1, y1, h, r, embed, inset=min(x1 - x0, y1 - y0) * 0.08, taper=0.2,
                       cuts=p.get("cuts", 5), bevel=bevel, lean=(lx, ly), yaw=r.uniform(-0.06, 0.06))
            continue
        # bedding: each layer narrower and shifted, joints in between
        z = 0.0
        lh = h / layers
        bx0, by0, bx1, by1 = x0, y0, x1, y1
        for i in range(layers):
            top = lh * r.uniform(0.85, 1.1) if i < layers - 1 else h - z
            rock_block(mb, bx0, by0, bx1, by1, max(top - joint * 0.3, 0.3), r, embed, inset=0.12, taper=0.08,
                       cuts=1 if i < layers - 1 else 2, bevel=bevel, z0=z, points=6)
            z += top
            w, d = bx1 - bx0, by1 - by0
            ox, oy = r.uniform(-0.1, 0.1) * w, r.uniform(-0.1, 0.1) * d
            bx0, bx1 = bx0 + w * 0.1 + ox, bx1 - w * 0.1 + ox
            by0, by1 = by0 + d * 0.08 + oy, by1 - d * 0.08 + oy
    _chips(mb, box, r, p.get("chips", 4), min(box.width, box.length) * 0.12, embed)
    _embedded(model, name, embed)


@env_part("rock_massif")
def rock_massif(model, name, p, box):
    """Piece of a rock ridge (env_rock_massif): `cols` blocks along the
    length in `rows` rows (1 or 2), high in the middle and low at the ends
    (pieces overlap into a ridge), some with a smaller block stacked on
    top, clefts between them. Also builds `<Variant>_Far` (the same cells
    and heights with fewer points and cuts, no bevel, no stacked blocks or
    chips).

    Parameters: `seed`; `cols` (default 4); `rows` (default 1); `joint`
    (m, default 0.15); `embed` (m, default 1.5); `bevel` (m, default 0.14);
    `stack` (chance of a block on top, default 0.6); `flank` (chance of a
    lower block beside the main one, one row only, default 0.7); `low`
    (height of the end blocks, fraction, default 0.45); `chips` (default
    6); `cuts` (default 4).
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    far = model.mb(f"{name}_Far")
    embed = p.get("embed", 1.5)
    bevel = p.get("bevel", 0.14)
    joint = p.get("joint", 0.15)
    cols, rows = p.get("cols", 4), p.get("rows", 1)
    low = p.get("low", 0.45)
    # column borders with jitter
    edges = [0.0] + sorted(r.uniform((i + 0.75) / cols, (i + 1.25) / cols) for i in range(cols - 1)) + [1.0]
    for i in range(cols):
        t = (i + 0.5) / cols
        hf = low + (1.0 - low) * math.sin(math.pi * t) ** 0.7
        for j in range(rows):
            fy0, fy1 = edges[i], edges[i + 1]
            fx0, fx1 = j / rows, (j + 1) / rows
            if rows > 1:
                # the rows are staggered, the second one lower
                fx0 += r.uniform(-0.05, 0.05) if j else 0.0
                hf_row = hf * (r.uniform(0.6, 0.85) if j else 1.0)
            else:
                hf_row = hf
            cell = (-box.hw + fx0 * box.width, -box.hl + fy0 * box.length,
                    -box.hw + fx1 * box.width, -box.hl + fy1 * box.length)
            x0, y0, x1, y1 = _gap(cell, joint / 2)
            h = box.height * hf_row * r.uniform(0.85, 1.0)
            yaw = r.uniform(-0.05, 0.05)
            cuts = p.get("cuts", 4)
            blocks = [(x0, y0, x1, y1, h)]
            if rows == 1 and r.random() < p.get("flank", 0.7):
                # the main block narrower, a lower flank block beside it
                share = r.uniform(0.6, 0.8)
                split = x0 + (x1 - x0) * (share if r.random() < 0.5 else 1 - share)
                left_main = split - x0 > x1 - split
                main = (x0, y0, split - joint / 2, y1) if left_main else (split + joint / 2, y0, x1, y1)
                flank = (split + joint / 2, y0, x1, y1) if left_main else (x0, y0, split - joint / 2, y1)
                fy = (flank[3] - flank[1]) * r.uniform(0.0, 0.25)
                flank = (flank[0], flank[1] + fy * r.random(), flank[2], flank[3] - fy * r.random())
                blocks = [(*main, h), (*flank, h * r.uniform(0.35, 0.6))]
            for k, (bx0, by0, bx1, by1, bh) in enumerate(blocks):
                state = r.getstate()
                rock_block(mb, bx0, by0, bx1, by1, bh, r, embed, inset=min(bx1 - bx0, by1 - by0) * 0.04,
                           taper=0.06, cuts=cuts, bevel=bevel, yaw=yaw, chop=(0.45, 0.85))
                after = r.getstate()
                r.setstate(state)
                rock_block(far, bx0, by0, bx1, by1, bh, r, embed, inset=min(bx1 - bx0, by1 - by0) * 0.04,
                           taper=0.06, cuts=2 if k == 0 else 1, bevel=0.0, yaw=yaw, points=4, chop=(0.45, 0.85))
                r.setstate(after)
            x0, y0, x1, y1, h = blocks[0]
            if r.random() < p.get("stack", 0.6) and h > 2.5:
                w, d = x1 - x0, y1 - y0
                sx0 = x0 + w * r.uniform(0.1, 0.3)
                sy0 = y0 + d * r.uniform(0.1, 0.3)
                sx1 = x1 - w * r.uniform(0.1, 0.3)
                sy1 = y1 - d * r.uniform(0.1, 0.3)
                zs = h * r.uniform(0.7, 0.82)
                sh = min(box.height - zs, h * r.uniform(0.25, 0.45))
                if sh > 0.6:
                    rock_block(mb, sx0, sy0, sx1, sy1, sh, r, embed, inset=0.15, taper=0.25, cuts=2,
                               bevel=bevel * 0.7, z0=zs, yaw=r.uniform(-0.2, 0.2), points=5)
    _chips(mb, box, r, p.get("chips", 6), min(box.width, box.length) * 0.08, embed)
    _embedded(model, name, embed)


@env_part("rock_cliff")
def rock_cliff(model, name, p, box):
    """Asymmetric modular cliff wall (env_rock_cliff, prompt 243):

    Axis convention:
    - `+Y` runs along the cliff wall (`-box.hl .. +box.hl`).
    - `+X` is the steep valley showcase front (craggy ribs, deep vertical/angled
      clefts, subtle overhangs, leaning foot buttress blocks, scree stones).
    - `-X` is the wider, stepped slope-side back shoulder dipping smoothly into
      the upper terrain tier.
    - Deep foot (`embed = 3.0 m` default) prevents floating edges on steep slopes.

    Builds both `<Variant>` (detail, <= 4000 tris) and `<Variant>_Far` (<= 450 tris).
    Parameters: `seed`, `cols` (4..7), `style` ("straight"|"long"|"high"|"buttress"|
    "corner"|"end"), `embed` (default 3.0), `bevel` (default 0.14), `joint`
    (default 0.38), `cuts` (default 5), `buttresses` (default 3), `chips` (default 8).
    """
    r = rng(p.get("seed", 101))
    mb = model.mb(name)
    far = model.mb(f"{name}_Far")
    embed = float(p.get("embed", 3.0))
    bevel = float(p.get("bevel", 0.14))
    joint = float(p.get("joint", 0.38))
    cols = max(3, int(p.get("cols", 6)))
    cuts = int(p.get("cuts", 5))
    style = str(p.get("style", "straight"))

    # Irregular column divisions along +Y so ribs never repeat like uniform boxes.
    edges = [0.0] + sorted(r.uniform((i + 0.72) / cols, (i + 1.28) / cols) for i in range(cols - 1)) + [1.0]

    # 1. Stepped back shoulder (-X): sits slightly lower/deeper into the upper slope
    #    and seals the rear of the clefts so no daylight leaks through the wall.
    shoulder_n = max(2, cols // 2)
    s_edges = [0.0] + sorted(r.uniform((i + 0.75) / shoulder_n, (i + 1.25) / shoulder_n) for i in range(shoulder_n - 1)) + [1.0]
    for i in range(shoulder_n):
        t = (i + 0.5) / shoulder_n
        if style == "end":
            # Step smoothly down toward +Y end.
            end_f = max(0.32, 1.0 - 0.68 * (t ** 1.25))
        else:
            end_f = 0.76 + 0.24 * math.sin(math.pi * t) ** 0.6
        sy0 = -box.hl + s_edges[i] * box.length + 0.1
        sy1 = -box.hl + s_edges[i + 1] * box.length - 0.1
        sx0 = -box.hw
        sx1 = box.hw * (0.12 if style != "corner" else 0.22)
        sh = box.height * end_f * r.uniform(0.74, 0.88)
        state = r.getstate()
        rock_block(mb, sx0, sy0, sx1, sy1, sh, r, embed,
                   inset=0.22, taper=0.14, cuts=3, bevel=bevel * 0.75,
                   lean=(0.25, r.uniform(-0.2, 0.2)), yaw=r.uniform(-0.04, 0.04),
                   points=5, chop=(0.42, 0.82))
        after = r.getstate()
        r.setstate(state)
        rock_block(far, sx0, sy0, sx1, sy1, sh, r, embed,
                   inset=0.22, taper=0.14, cuts=1, bevel=0.0,
                   lean=(0.25, 0.0), yaw=0.0, points=4, chop=(0.45, 0.85))
        r.setstate(after)

    # 2. Craggy main cliff ribs (+X showcase face): staggered projection and height
    #    with real 0.35-0.85 m vertical/angled clefts between ribs.
    for i in range(cols):
        t = (i + 0.5) / cols
        if style == "end":
            hf = max(0.24, 0.98 - 0.76 * (t ** 1.15))
        elif style == "high":
            hf = 0.62 + 0.38 * (math.sin(math.pi * t) ** 0.55)
        elif style == "corner":
            hf = 0.66 + 0.34 * (math.sin(math.pi * t) ** 0.6)
        else:
            hf = 0.68 + 0.32 * (math.sin(math.pi * t) ** 0.55)

        fy0, fy1 = edges[i], edges[i + 1]
        cleft = joint * r.uniform(0.85, 1.55)
        y0 = -box.hl + fy0 * box.length + cleft * 0.5
        y1 = -box.hl + fy1 * box.length - cleft * 0.5
        if y1 - y0 < 0.8:
            continue

        # Stagger front face projection (+X) for deep shadow ribs & recesses.
        if style == "corner":
            bulge = 0.45 + 0.45 * math.sin(math.pi * t)
        elif style == "end":
            bulge = max(0.20, 0.82 - 0.55 * t)
        else:
            bulge = 0.58 + (0.28 if i % 2 == 0 else -0.06) + r.uniform(-0.08, 0.14)
        x0 = -box.hw * r.uniform(0.32, 0.56)
        x1 = box.hw * min(0.96, bulge)
        h = box.height * hf * r.uniform(0.86, 1.0)
        # Steep craggy face with occasional slight outward overhang (+X lean).
        lean_x = r.uniform(-0.35, 0.55) if style != "end" else r.uniform(-0.4, 0.1)
        lean_y = r.uniform(-0.35, 0.35)
        yaw = r.uniform(-0.09, 0.09)

        state = r.getstate()
        rock_block(mb, x0, y0, x1, y1, h, r, embed,
                   inset=min(x1 - x0, y1 - y0) * 0.05, taper=0.07,
                   cuts=cuts, bevel=bevel, lean=(lean_x, lean_y), yaw=yaw,
                   points=5, chop=(0.40, 0.84))
        after = r.getstate()
        r.setstate(state)
        rock_block(far, x0, y0, x1, y1, h, r, embed,
                   inset=min(x1 - x0, y1 - y0) * 0.05, taper=0.07,
                   cuts=2, bevel=0.0, lean=(lean_x, lean_y), yaw=yaw,
                   points=4, chop=(0.45, 0.85))
        r.setstate(after)

        # Stepped upper crown ledge block on taller ribs (catches moss & light).
        if h > box.height * 0.62 and r.random() < 0.55:
            cw, cd = x1 - x0, y1 - y0
            cx0 = x0 + cw * r.uniform(0.05, 0.25)
            cx1 = x1 - cw * r.uniform(0.18, 0.42)
            cy0 = y0 + cd * r.uniform(0.08, 0.22)
            cy1 = y1 - cd * r.uniform(0.08, 0.22)
            zs = h * r.uniform(0.72, 0.82)
            ch = min(box.height - zs, h * r.uniform(0.20, 0.32))
            if cx1 - cx0 > 0.7 and cy1 - cy0 > 0.7 and ch > 0.5:
                rock_block(mb, cx0, cy0, cx1, cy1, ch, r, embed,
                           inset=0.14, taper=0.18, cuts=2, bevel=bevel * 0.65,
                           z0=zs, yaw=r.uniform(-0.14, 0.14), points=5, chop=(0.45, 0.85))

    # 3. Leaning foot buttress slabs (+X front): massive angled rock blocks
    #    propped against the cliff base (2-5 m wide, leaning back toward -X).
    n_buttress = int(p.get("buttresses", 4 if style == "buttress" else 3))
    for k in range(n_buttress):
        bt = (k + 0.5 + r.uniform(-0.22, 0.22)) / max(n_buttress, 1)
        bt = max(0.10, min(0.90, bt))
        if style == "end":
            bh_scale = max(0.32, 0.88 - 0.55 * bt)
        elif style == "buttress":
            bh_scale = r.uniform(0.45, 0.68)
        else:
            bh_scale = r.uniform(0.30, 0.52)
        bw = r.uniform(2.2, 4.4) if style == "buttress" else r.uniform(1.8, 3.6)
        bd = r.uniform(1.6, 2.8)
        by = -box.hl + bt * box.length
        by0 = max(-box.hl + 0.3, by - bw * 0.5)
        by1 = min(box.hl - 0.3, by + bw * 0.5)
        bx1 = box.hw * r.uniform(0.84, 1.0)
        bx0 = max(0.0, bx1 - bd)
        bh = box.height * bh_scale
        # Lean top back toward the cliff wall (-X, 20-40 deg tilt feel).
        lean_back = -min(bd * 0.65, bh * r.uniform(0.32, 0.55))
        rock_block(mb, bx0, by0, bx1, by1, bh, r, embed,
                   inset=0.16, taper=0.22, cuts=3, bevel=bevel * 0.75,
                   lean=(lean_back, r.uniform(-0.35, 0.35)),
                   yaw=r.uniform(-0.18, 0.18), points=5, chop=(0.35, 0.8))

    # 4. Angular scree stones (kind = 1, darker loose rock) along the +X valley foot.
    n_chips = int(p.get("chips", 8))
    for _ in range(n_chips):
        s = r.uniform(0.65, 1.55)
        y = r.uniform(-box.hl + s * 0.6, box.hl - s * 0.6)
        x = box.hw * r.uniform(0.72, 0.98) - s * 0.25
        rock_block(mb, x - s * 0.5, y - s * 0.5, x + s * 0.5, y + s * 0.5,
                   s * r.uniform(0.35, 0.65), r, embed * 0.5,
                   inset=s * 0.12, taper=0.28, cuts=2, bevel=0.0,
                   yaw=r.uniform(0.0, math.pi), kind=1, points=4)

    _embedded(model, name, embed)
