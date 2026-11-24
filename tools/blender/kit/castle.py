"""Castle pieces (prompt 127): curtain wall with towers, gatehouse and an
artillery bastion (half-moon battery) for a castle on a rock, built from
several map objects so the usual rules apply (walls give cover and fall to
heavy fire, infantry occupies the gatehouse and the bastion).

The great stone ranges and the small chapel use the generic generators of
`buildings.py` (`house`, `church`) with their own parameters.

No real building is copied; sizes are judgement values after the sources
named in data/catalog/neutral.toml. Walls run along Y like every boundary.
"""

import math

from .buildings import House, Opening, _blast
from .env import env_part, rng


def _merlons(mb, x0, x1, y0, y1, z, size=0.9, gap=0.8, skip=None):
    """Merlons along the rectangle edge from (x0, y0) to (x1, y1) on top of
    a parapet at height z."""
    length = math.hypot(x1 - x0, y1 - y0)
    n = max(1, int(length / (size + gap)))
    for k in range(n):
        if skip is not None and k in skip:
            continue
        f = (k + 0.5) / n
        cx, cy = x0 + (x1 - x0) * f, y0 + (y1 - y0) * f
        sx = abs(x1 - x0) / n * 0.55 if x1 != x0 else 0.5
        sy = abs(y1 - y0) / n * 0.55 if y1 != y0 else 0.5
        mb.box((cx, cy, z + 0.45), (max(sx, 0.5), max(sy, 0.5), 0.9), "Stone")


@env_part("curtain_wall")
def curtain_wall(model, name, p, box):
    """Curtain wall of rubble masonry: a battered core with a wall walk and
    a crenellated parapet on both faces. `piece` = "segment" (box length)
    or "tower" (a square flanking tower, higher than the wall, with a flat
    crenellated top). `seed`; `damage` 1 knocks out merlons and bites a
    breach into the top."""
    r = rng(p.get("seed", 1))
    mb = model.mb(name)
    H = box.height
    hw, hl = box.hw, box.hl
    damage = p.get("damage", 0)
    mb.style(kind=1)
    if p.get("piece", "segment") == "tower":
        # Battered foot, square shaft, corbelled parapet.
        mb.prism([(-hw, -hl), (hw, -hl), (hw, hl), (-hw, hl)], 0.0, 1.5, "Stone")
        s = 0.25
        mb.prism([(-hw + s, -hl + s), (hw - s, -hl + s), (hw - s, hl - s), (-hw + s, hl - s)], 1.5, H - 1.4, "Stone")
        mb.prism([(-hw, -hl), (hw, -hl), (hw, hl), (-hw, hl)], H - 1.4, H - 0.9, "Stone", bevel=0.03)
        for (ax, ay, bx, by) in ((-hw, -hl, hw, -hl), (hw, -hl, hw, hl), (hw, hl, -hw, hl), (-hw, hl, -hw, -hl)):
            _merlons(mb, ax * 0.94, bx * 0.94, ay * 0.94, by * 0.94, H - 0.9, skip={1} if damage else None)
        # Arrow slits.
        mb.style(kind=0)
        for z in (3.5, 6.5):
            for x, y, ax in ((hw - s + 0.02, 0.0, "x"), (-hw + s - 0.02, 0.0, "x"),
                             (0.0, hl - s + 0.02, "y"), (0.0, -hl + s - 0.02, "y")):
                size = (0.04, 0.18, 1.0) if ax == "x" else (0.18, 0.04, 1.0)
                mb.box((x, y, z), size, "Glass")
        return
    walk = H - 1.6
    # Core: battered faces, the wall walk on top.
    prof = [(-hw, 0.0), (hw, 0.0), (hw - 0.35, walk), (-hw + 0.35, walk)]
    mb.extrude([(x, -hl, z) for x, z in prof], (0.0, 2.0 * hl, 0.0), "Stone")
    # A string course and the plinth.
    mb.box((0.0, 0.0, 0.4), (2.0 * hw + 0.1, 2.0 * hl, 0.8), "Stone")
    mb.box((0.0, 0.0, walk - 0.15), (2.0 * hw - 0.55, 2.0 * hl, 0.3), "Stone", bevel=0.03)
    # Parapets on both faces with merlons.
    for side in (-1, 1):
        x = side * (hw - 0.55)
        if damage:
            # A bite out of the top and fallen merlons.
            mb.box((x, -hl * 0.55, walk + 0.35), (0.5, hl * 0.9, 0.7), "Stone")
            mb.box((x, hl * 0.6, walk + 0.35), (0.5, hl * 0.8, 0.7), "Stone")
            _merlons(mb, x, x, -hl, hl, walk + 0.7, skip={1, 2, 3})
        else:
            mb.box((x, 0.0, walk + 0.35), (0.5, 2.0 * hl, 0.7), "Stone")
            _merlons(mb, x, x, -hl, hl, walk + 0.7)
    # Face stones: a few projecting blocks so the wall does not read flat.
    mb.style(kind=0)
    for side in (-1, 1):
        for _ in range(int(hl * 3)):
            y = r.uniform(-hl + 0.4, hl - 0.4)
            z = r.uniform(1.0, walk - 1.0)
            x = side * (hw - 0.35 * z / walk + 0.02)
            mb.box((x, y, z), (0.08, r.uniform(0.5, 1.1), r.uniform(0.3, 0.55)), "Stone")
    if damage:
        model.part_props.setdefault(name, {})["damage"] = "damaged"


@env_part("gatehouse")
def gatehouse(model, name, p, box):
    """Gatehouse: a tall rectangular stone block over an arched gate
    passage through its length (Y), two round corner turrets on the field
    side (-Y), a crenellated wall walk on top and a steep slate roof behind
    it. Infantry occupies its storeys (the passage itself is drawn but the
    map object blocks like a house; the gate opening is the gap in the
    curtain wall beside it). Parameters: `seed`, `damage` (0, 1, 2)."""
    h = House(model, name, p, box, "Stone", 1)
    H = box.height
    ruin = h.damage >= 2
    t = 1.2
    x0, x1 = -box.hw + 1.0, box.hw - 1.0
    y0, y1 = -box.hl + 1.6, box.hl
    plinth = 0.6
    z_walk = H - 4.5
    wall_top = z_walk if not ruin else z_walk * 0.55
    h.plinth(x0, x1, y0, y1, plinth)
    gate_w = 3.6
    ops = {
        "-y": [Opening(0.0, gate_w, plinth, plinth + 4.2, "door"),
               Opening(-2.6, 0.4, plinth + 7.5, plinth + 8.7, "window"),
               Opening(2.6, 0.4, plinth + 7.5, plinth + 8.7, "window")],
        "+y": [Opening(0.0, gate_w, plinth, plinth + 4.2, "door"),
               Opening(0.0, 1.0, plinth + 7.0, plinth + 8.6, "window")],
        "+x": [Opening(y, 0.8, plinth + 6.0, plinth + 7.4, "window") for y in (y0 + 3.0, y1 - 3.0)],
        "-x": [Opening(y, 0.8, plinth + 6.0, plinth + 7.4, "window") for y in (y0 + 3.0, y1 - 3.0)],
    }
    if h.damage == 1:
        _blast(h, ops["-y"][1:], 1)
    h.rect_walls(x0, x1, y0, y1, plinth, wall_top, ops, t, ruin=(True if ruin else None))
    if not ruin:
        for z in (plinth + 5.0, plinth + 9.5):
            if z < z_walk - 1.0:
                h.slab(x0 + t, x1 - t, y0 + t, y1 - t, z)
        # The vault of the passage.
        h.walls.style(kind=1)
        h.walls.box((0.0, (y0 + y1) / 2, plinth + 4.6), (gate_w + 0.4, y1 - y0 - 0.2, 0.8), "Stone")
        # Parapet with merlons on corbels, slate roof inside.
        h.roof.style(kind=1)
        h.roof.box((0.0, (y0 + y1) / 2, z_walk + 0.2), (x1 - x0 + 0.6, y1 - y0 + 0.6, 0.4), "Stone")
        for (ax, ay, bx, by) in ((x0, y0, x1, y0), (x1, y0, x1, y1), (x1, y1, x0, y1), (x0, y1, x0, y0)):
            for k in range(max(2, int(math.hypot(bx - ax, by - ay) / 1.7))):
                n = max(2, int(math.hypot(bx - ax, by - ay) / 1.7))
                if h.damage == 1 and k == 1:
                    continue
                f = (k + 0.5) / n
                cx, cy = ax + (bx - ax) * f, ay + (by - ay) * f
                h.roof.box((cx, cy, z_walk + 0.85), (0.9, 0.9, 0.9), "Stone")
        h.roof_kind = 1
        h.gable_roof(x0 + 1.4, x1 - 1.4, y0 + 1.4, y1 - 1.4, z_walk + 0.4, H - 0.1, 1, strips=6)
        # Round turrets at the field corners.
        for sx in (-1, 1):
            cx, cy = sx * (box.hw - 1.6), -box.hl + 1.6
            h.walls.style(kind=1)
            h.walls.cylinder((cx, cy, 0.0), (cx, cy, z_walk + 1.0), 1.6, 1.5, seg=14, mat="Stone")
            h.roof.cylinder((cx, cy, z_walk + 1.0), (cx, cy, z_walk + 4.0), 1.75, 0.05, seg=14, mat="Roof")
    if h.damage == 1:
        h.rubble(x0 + 1.0, x1 - 1.0, y0 - 1.2, y0 - 0.4, 1, height=0.8, size=0.6)
    elif ruin:
        h.rubble(x0 + 0.8, x1 - 0.8, y0 + 0.8, y1 - 0.8, 3, height=1.6)
    h.finish()


@env_part("bastion")
def bastion(model, name, p, box):
    """Artillery bastion (half-moon battery): a D-shaped mass of masonry,
    round towards the field (-Y), straight at the back (+Y), with a thick
    parapet and gun embrasures on the platform and a small guardroom at
    the back. Infantry occupies the platform and the casemates under it.
    Parameters: `seed`, `damage` (0, 1, 2)."""
    h = House(model, name, p, box, "Stone", 1)
    H = box.height
    ruin = h.damage >= 2
    hw, hl = box.hw, box.hl
    seg = 20
    # Plan: half ellipse of radius hw x (2 hl - back) in front of the back wall.
    back = hl
    depth = 2.0 * hl - 3.0
    plan = []
    for k in range(seg + 1):
        a = math.pi * k / seg
        plan.append((hw * math.cos(a), back - 3.0 - depth * math.sin(a)))
    plan += [(-hw, back), (hw, back)]
    top = H - 5.4 if not ruin else (H - 5.4) * 0.6
    h.walls.style(kind=1)
    # Battered foot ring, then the mass.
    foot = [(x * 1.03, (y - back) * 1.03 + back) for x, y in plan]
    h.walls.prism(list(reversed(foot)), 0.0, 1.2, "Stone")
    h.walls.prism(list(reversed(plan)), 1.2, top, "Stone")
    # Embrasures and parapet: blocks along the curve with gaps.
    if not ruin:
        h.walls.style(kind=1)
        for k in range(seg):
            if k % 3 == 1:
                continue      # embrasure
            if h.damage == 1 and k in (6, 7, 8):
                continue
            (ax, ay), (bx, by) = plan[k], plan[k + 1]
            mx, my = (ax + bx) / 2, (ay + by) / 2
            dx, dy = bx - ax, by - ay
            L = math.hypot(dx, dy)
            h.walls.oriented_box((mx * 0.97, (my - back) * 0.97 + back, top + 0.6), (dx / L, dy / L, 0.0),
                                 (0.0, 0.0, 1.0), (L * 1.02, 1.2, 1.2), "Stone")
        # A string course under the parapet.
        ring = [(x * 1.01, (y - back) * 1.01 + back) for x, y in plan]
        h.walls.prism(list(reversed(ring)), top - 0.4, top - 0.1, "Stone")
        # Guardroom with a slate roof at the back.
        gx0, gx1, gy0, gy1 = -4.0, 4.0, back - 6.0, back - 0.3
        ops = {"-y": [Opening(0.0, 1.0, top, top + 2.0, "door")],
               "+x": [Opening(back - 3.0, 0.8, top + 0.9, top + 2.0, "window")],
               "-x": [Opening(back - 3.0, 0.8, top + 0.9, top + 2.0, "window")]}
        h.rect_walls(gx0, gx1, gy0, gy1, top, top + 3.0, ops, 0.6)
        h.roof_kind = 1
        h.gable_roof(gx0 - 0.3, gx1 + 0.3, gy0 - 0.2, gy1 + 0.2, top + 2.9, top + 5.0, 1, strips=4)
        # Casemate openings in the face.
        h.walls.style(kind=0)
        for k in (5, 10, 15):
            (ax, ay), (bx, by) = plan[k], plan[k + 1]
            mx, my = (ax + bx) / 2, (ay + by) / 2
            h.walls.box((mx * 1.005, (my - back) * 1.005 + back, top - 3.2), (0.9, 0.9, 0.9), "Glass")
    if h.damage == 1:
        h.rubble(-hw * 0.4, hw * 0.4, -hl + 0.6, -hl + 1.4, 1, height=0.9, size=0.7)
    elif ruin:
        h.rubble(-hw * 0.5, hw * 0.5, -hl * 0.3, hl * 0.6, 3, height=1.8)
    h.finish()
