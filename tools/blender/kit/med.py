"""Mediterranean village pieces (prompt 120): field-stone walls, terrace
retaining walls and the coastal watch tower.

The houses, row houses, farmhouse, barn, church and public building use the
generic generators of `buildings.py` (`house`, `church`) with the prompt 120
parameters (shutters, shop front, outside stair, campanile) and a
`[surface]` table for plaster colors and monk-and-nun tiles.

Walls run along Y like every boundary (a segment spans y -L/2..L/2 and
chains every catalog length).
"""

import math

from .buildings import House, Opening, _blast
from .env import env_part, rng
from .parts_neutral import _stones_face
from .rocks import _embedded


def _coping(mb, r, y0, y1, z, width):
    """Flat cap stones on top of a wall from y0 to y1."""
    y = y0
    while y < y1 - 0.05:
        w = min(r.uniform(0.22, 0.38), y1 - y)
        mb.oriented_box((r.uniform(-0.02, 0.02), y + w / 2, z), (1, 0, 0), (0, 0, 1),
                        (width * r.uniform(1.0, 1.15), r.uniform(0.09, 0.14), w * 0.94), "Stone")
        y += w


def _wall_run(mb, r, y0, y1, H, hb, ht, top=None, coping=True):
    """Battered dry-stone core from y0 to y1 with face stones on both
    sides; `top` (y -> height) makes a broken top line."""
    prof = [(-hb, 0), (hb, 0), (ht, H * 0.9), (-ht, H * 0.9)]
    if top is None:
        mb.extrude([(x, y0, z) for x, z in prof], (0, y1 - y0, 0), "Stone")
    else:
        # Columns of 0.5 m, each cut to its own height.
        n = max(1, int((y1 - y0) / 0.5 + 0.5))
        for i in range(n):
            ya, yb = y0 + (y1 - y0) * i / n, y0 + (y1 - y0) * (i + 1) / n
            h = top((ya + yb) / 2)
            if h < 0.1:
                continue
            k = h / (H * 0.9)
            w = hb + (ht - hb) * k
            pr = [(-hb, 0), (hb, 0), (w, h), (-w, h)]
            mb.extrude([(x, ya, z) for x, z in pr], (0, yb - ya, 0), "Stone")

    def xz(z):
        return hb + (ht - hb) * (z / (H * 0.9)) - 0.03

    face_h = H * 0.88 if top is None else max(top(y) for y in (y0, (y0 + y1) / 2, y1)) * 0.9
    if top is None:
        _stones_face(mb, r, y0, y1, lambda z: xz(min(z, H * 0.85)), face_h, 1)
        _stones_face(mb, r, y0, y1, lambda z: xz(min(z, H * 0.85)), face_h, -1)
    if coping and top is None:
        _coping(mb, r, y0, y1, H * 0.95, ht * 2.0)


def _loose_stones(mb, r, n, x_range, y_range):
    for _ in range(n):
        s = r.uniform(0.18, 0.35)
        mb.oriented_box((r.uniform(*x_range), r.uniform(*y_range), s * 0.35),
                        (math.cos(r.uniform(0, 3.1)), math.sin(r.uniform(0, 3.1)), 0), (0, 0, 1),
                        (s, s * 0.8, s * 0.7), "Stone")


@env_part("wall_field")
def wall_field(model, name, p, box):
    """Mediterranean field-stone wall (muro a secco) along Y.

    Parameters: `seed`; `piece` ("segment", "low", "broken", "gate", "end",
    "corner"; default "segment"). "low" is the segment in a lower box
    (`dims_m`); "broken" has a collapsed middle with stones at its foot;
    "gate" has two pillars and an open farm gate of planks; "end" a short
    piece with an end face; "corner" a square pier.
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "segment")
    H = box.height
    hb = box.hw * 0.8
    ht = hb * 0.6
    y0, y1 = -box.hl, box.hl
    mb.style(kind=0)
    if piece == "corner":
        mb.prism([(-box.hw, -box.hl), (box.hw, -box.hl), (box.hw, box.hl), (-box.hw, box.hl)], 0, H * 0.9, "Stone")
        _coping(mb, r, -box.hl, box.hl, H * 0.95, box.width * 0.95)
        return
    if piece == "gate":
        pl = 0.45
        for s in (-1, 1):
            yc = s * (box.hl - pl / 2)
            mb.box((0, yc, H * 0.5), (box.width * 0.9, pl, H), "Stone", bevel=0.03)
            mb.box((0, yc, H + 0.06), (box.width, pl + 0.08, 0.12), "Stone")
        # Open gate leaf of grey planks, swung to one side.
        mb.style(kind=0)
        span = box.length - 2 * pl
        leaf = span * 0.5
        ya = -box.hl + pl
        for k in range(5):
            z = 0.2 + k * (H * 0.75) / 4
            mb.oriented_box((-box.hw * 0.2 - leaf / 2 * 0.7, ya + leaf / 2 * 0.7, z), (0.7, 0.7, 0), (0, 0, 1),
                            (leaf, 0.06, 0.04), "Wood")
        for k in (0.05, 0.95):
            mb.oriented_box((-box.hw * 0.2 - leaf * k * 0.7, ya + leaf * k * 0.7, H * 0.45), (0.7, 0.7, 0), (0, 0, 1),
                            (0.08, H * 0.9, 0.05), "Wood")
        return
    if piece == "end":
        _wall_run(mb, r, y0, y1, H, hb, ht)
        mb.box((0, y1 - 0.15, H * 0.47), (hb * 2.05, 0.3, H * 0.94), "Stone")
        return
    if piece == "broken":
        gap0, gap1 = r.uniform(-0.6, -0.2), r.uniform(0.3, 0.8)

        def top(y):
            if gap0 < y < gap1:
                return H * r.uniform(0.15, 0.35)
            return H * (0.9 if abs(y) > 1.0 else r.uniform(0.55, 0.8))

        _wall_run(mb, r, y0, y1, H, hb, ht, top=top)
        _coping(mb, r, y0, gap0 - 0.4, H * 0.95, ht * 2.0)
        _coping(mb, r, gap1 + 0.4, y1, H * 0.95, ht * 2.0)
        _loose_stones(mb, r, 10, (-box.hw, box.hw), (gap0 - 0.3, gap1 + 0.3))
        return
    _wall_run(mb, r, y0, y1, H, hb, ht)


@env_part("terrace_wall")
def terrace_wall(model, name, p, box):
    """Retaining wall of a hillside terrace (prompt 120) along Y: a battered
    dry-stone face towards -X (downhill) with its foot reaching below the
    ground (`foot`, m, default 0.8, so it closes on slopes), the terrace
    soil behind it at the top (+X, Ground slot) sloping back down.

    Parameters: `seed`; `piece` ("segment", "end", "corner"); `foot`.
    The terrain stays the map: the object only shows the step.
    """
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    piece = p.get("piece", "segment")
    foot = p.get("foot", 0.8)
    H = box.height
    y0, y1 = -box.hl, box.hl
    xf = -box.hw  # foot of the face
    xt = xf + 0.35  # top of the face (batter)
    back = xf + 0.9
    mb.style(kind=0)
    _embedded(model, name, foot)
    if piece == "corner":
        mb.prism([(-box.hw, -box.hl), (box.hw, -box.hl), (box.hw, box.hl), (-box.hw, box.hl)], -foot, H * 0.95, "Stone")
        _coping(mb, r, -box.hl, box.hl, H, box.width)
        return
    mb.extrude([(xf, y0, -foot), (back, y0, -foot), (back, y0, H), (xt, y0, H)], (0, y1 - y0, 0), "Stone")
    _stones_face(mb, r, y0, y1, lambda z: -(xf + (xt - xf) * max(z, 0.0) / H) - 0.03, H * 0.95, -1)
    _coping(mb, r, y0, y1, H + 0.04, 0.5)
    # Soil of the terrace behind the wall, sloping down to the ground.
    mb.style(kind=0)
    mb.extrude([(back, y0, H - 0.05), (box.hw, y0, -0.3), (box.hw, y0, -foot), (back, y0, -foot)],
               (0, y1 - y0, 0), "Ground")
    if piece == "end":
        # The face turns back into the slope.
        mb.box((xf + 0.5, y1 - 0.25, (H - foot) / 2), (1.0, 0.5, H + foot), "Stone")


@env_part("watch_tower")
def watch_tower(model, name, p, box):
    """Coastal watch tower (torre costiera): a square stone tower with a
    battered base, a raised door on the first floor, small windows and a
    flat top with a crenellated parapet.

    Parameters: `seed`; `damage` (0 intact, 1 damaged, 2 ruin); `batter`
    (height of the sloped base, default 3.5 m).
    """
    h = House(model, name, p, box, "Stone", 0)
    H = box.height
    ruin = h.damage >= 2
    batter = p.get("batter", 3.5)
    x0, x1, y0, y1 = -box.hw + 0.5, box.hw - 0.5, -box.hl + 0.5, box.hl - 0.5
    t = 1.0
    # Sloped base: a frustum from the box to the wall line.
    h.floor.style(kind=0)
    h.floor.extrude([(-box.hw, -box.hl, 0), (box.hw, -box.hl, 0), (box.hw, box.hl, 0), (-box.hw, box.hl, 0)],
                    (0, 0, 0.4), "Stone")
    base = [(-box.hw, -box.hl), (box.hw, -box.hl), (box.hw, box.hl), (-box.hw, box.hl)]
    top = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    vb = h.floor.verts([(x, y, 0.4) for x, y in base])
    vt = h.floor.verts([(x, y, batter) for x, y in top])
    for i in range(4):
        j = (i + 1) % 4
        h.floor.face([vb[i], vb[j], vt[j], vt[i]], "Stone")
    h.floor.face(list(vt), "Stone")
    z_top = H - 1.2
    wall_top = z_top if not ruin else z_top * 0.55
    ops = {
        "+x": [Opening(0, 1.0, batter + 0.6, batter + 2.6, "door")],
        "-x": [Opening(0, 0.5, batter + 4.0, batter + 5.0, "window")],
        "+y": [Opening(0, 0.5, batter + 4.0, batter + 5.0, "window")],
        "-y": [Opening(0, 0.5, batter + 4.0, batter + 5.0, "window")],
    }
    if h.damage == 1:
        _blast(h, ops["-y"], 1)
    h.rect_walls(x0, x1, y0, y1, batter, wall_top, ops, t, ruin=(True if ruin else None))
    if not ruin:
        h.slab(x0 + t, x1 - t, y0 + t, y1 - t, batter + 0.4)
        # Flat top on corbels, parapet with merlons.
        h.roof.style(kind=0)
        h.roof.box((0, 0, z_top + 0.15), (x1 - x0 + 0.5, y1 - y0 + 0.5, 0.3), "Stone")
        for (ax, ay, bx, by) in ((x0, y0, x1, y0), (x1, y0, x1, y1), (x1, y1, x0, y1), (x0, y1, x0, y0)):
            n = 4
            for k in range(n):
                if h.damage == 1 and k == 1:
                    continue
                f = (k + 0.5) / n
                cx, cy = ax + (bx - ax) * f, ay + (by - ay) * f
                sx = abs(bx - ax) / n * 0.55 + 0.4 * (ax == bx)
                sy = abs(by - ay) / n * 0.55 + 0.4 * (ay == by)
                h.roof.box((cx * 1.02, cy * 1.02, z_top + 0.3 + 0.45), (sx, sy, 0.9), "Stone")
        # The wooden ladder to the raised door.
        h.walls.style(kind=0)
        for s in (-0.25, 0.25):
            h.walls.oriented_box((box.hw - 0.25, s, (batter + 0.6) / 2 + 0.2), (-0.12, 0, 1), (0, 1, 0),
                                 (batter + 0.8, 0.06, 0.07), "Wood")
    if h.damage == 1:
        h.rubble(x0 + 1.0, x1 - 1.0, y0 + 0.2, y0 + 0.6, 1, height=0.8, size=0.6)
    elif ruin:
        h.rubble(x0 + 0.5, x1 - 0.5, y0 + 0.5, y1 - 0.5, 2, height=1.5)
    h.finish()
